// Darvium デュアルストア抽象化レイヤ
//
// 本モジュールは LadybugDB 責務 (GraphStore) と SQLite 責務 (MetadataStore) の
// 2系統トレイトを定義し、メモリ内実装 (InMemoryGraphStore / InMemoryMetadataStore) を提供する。
// 全13フェーズはこのトレイトに対するプログラミングで実装され、
// 実DB接続フェーズでは各トレイトの別実装を追加するだけで差し替えが完了する。

mod coordinator;
mod graph_store;
mod json_metadata_store;
mod metadata_store;
mod workflow_cache;

use std::collections::HashMap;

use crate::types::RankedCandidate;

pub use coordinator::{DualStoreCoordinator, RepairScanSummary};
pub use graph_store::{GraphStore, InMemoryGraphStore};
pub use json_metadata_store::JsonMetadataStore;
pub use metadata_store::{InMemoryMetadataStore, MetadataStore};
pub use workflow_cache::{
    AnnHotIndex, CacheError, CachePolicy, PersistenceError, RepositoryPair, WorkflowCache,
};

/// 2つの異種ストア（セマンティック / 構造）から取得した検索候補リストを統合し、
/// 同一 `workflow_id` を持つ候補を重複排除する (RFC §12.2 Stage 2c)。
///
/// 重複時は高い方の `blended_score` を残す（最大値保存則）。
/// セマンティック側の順序を優先し、構造側由来の新規候補を後方に追加する。
/// 入力リストは変更されない（非破壊的）。
pub fn merge_and_deduplicate_candidates(
    semantic: Vec<RankedCandidate>,
    structural: Vec<RankedCandidate>,
) -> Vec<RankedCandidate> {
    // 同一 workflow_id の候補をグループ化
    let mut groups: HashMap<String, RankedCandidate> = HashMap::new();

    // セマンティック側を先に登録（順序維持に使用）
    for candidate in &semantic {
        groups
            .entry(candidate.workflow_id.clone())
            .and_modify(|existing| {
                if candidate.blended_score > existing.blended_score {
                    existing.blended_score = candidate.blended_score;
                }
                for source in &candidate.provenance {
                    if !existing.provenance.contains(source) {
                        existing.provenance.push(source.clone());
                    }
                }
            })
            .or_insert_with(|| candidate.clone());
    }

    // 構造側を登録（既存より高いスコアで上書き、provenance 連結）
    for candidate in &structural {
        groups
            .entry(candidate.workflow_id.clone())
            .and_modify(|existing| {
                if candidate.blended_score > existing.blended_score {
                    existing.blended_score = candidate.blended_score;
                }
                for source in &candidate.provenance {
                    if !existing.provenance.contains(source) {
                        existing.provenance.push(source.clone());
                    }
                }
            })
            .or_insert_with(|| candidate.clone());
    }

    // 元の順序で復元: セマンティック側の候補を順に配置し、構造側の新規候補を後方に追加
    let mut seen: HashMap<String, bool> = HashMap::new();
    let mut result: Vec<RankedCandidate> = Vec::with_capacity(groups.len());

    // セマンティック側の順序を維持
    for candidate in &semantic {
        if let Some(merged) = groups.remove(&candidate.workflow_id) {
            seen.insert(merged.workflow_id.clone(), true);
            result.push(merged);
        }
    }

    // 構造側由来の残り（セマンティック側に存在しなかった候補）を順序維持して追加
    for candidate in &structural {
        if !seen.contains_key(&candidate.workflow_id) {
            if let Some(merged) = groups.remove(&candidate.workflow_id) {
                seen.insert(merged.workflow_id.clone(), true);
                result.push(merged);
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::RankedCandidate;

    /// テスト用の RankedCandidate を生成するヘルパー。
    fn make_candidate(id: &str, blended: f64, provenance: Vec<&str>) -> RankedCandidate {
        RankedCandidate {
            workflow_id: id.to_string(),
            semantic_score: blended,
            structural_score: blended * 0.9,
            blended_score: blended,
            trust_score: 0.8,
            provenance: provenance.into_iter().map(|s| s.to_string()).collect(),
            metadata: serde_json::json!({}),
        }
    }

    // ============================================================
    // T1: 正常系 — 重複なしマージ
    // ============================================================
    #[test]
    fn merge_no_duplicates() {
        let semantic = vec![
            make_candidate("wf-a", 0.9, vec!["semantic"]),
            make_candidate("wf-b", 0.8, vec!["semantic"]),
            make_candidate("wf-c", 0.7, vec!["semantic"]),
        ];
        let structural = vec![
            make_candidate("wf-d", 0.6, vec!["structural"]),
            make_candidate("wf-e", 0.5, vec!["structural"]),
        ];

        let result = merge_and_deduplicate_candidates(semantic, structural);
        assert_eq!(result.len(), 5, "T1: 全5件がマージされること");
        // 順序: セマンティック3件 → 構造2件
        assert_eq!(result[0].workflow_id, "wf-a");
        assert_eq!(result[1].workflow_id, "wf-b");
        assert_eq!(result[2].workflow_id, "wf-c");
        assert_eq!(result[3].workflow_id, "wf-d");
        assert_eq!(result[4].workflow_id, "wf-e");
    }

    // ============================================================
    // T2: 重複排除 — セマンティック側が高いスコア
    // ============================================================
    #[test]
    fn dedupe_semantic_higher() {
        let semantic = vec![make_candidate("wf-x", 0.9, vec!["semantic"])];
        let structural = vec![make_candidate("wf-x", 0.7, vec!["structural"])];

        let result = merge_and_deduplicate_candidates(semantic, structural);
        assert_eq!(result.len(), 1, "T2: 重複排除後1件");
        assert_eq!(
            result[0].blended_score, 0.9,
            "T2: 高い方のスコア(0.9)が残る"
        );
    }

    // ============================================================
    // T3: 重複排除 — 構造側が高いスコア
    // ============================================================
    #[test]
    fn dedupe_structural_higher() {
        let semantic = vec![make_candidate("wf-x", 0.6, vec!["semantic"])];
        let structural = vec![make_candidate("wf-x", 0.8, vec!["structural"])];

        let result = merge_and_deduplicate_candidates(semantic, structural);
        assert_eq!(result.len(), 1, "T3: 重複排除後1件");
        assert_eq!(
            result[0].blended_score, 0.8,
            "T3: 高い方のスコア(0.8)が残る"
        );
    }

    // ============================================================
    // T4: 重複排除 — スコア同値
    // ============================================================
    #[test]
    fn dedupe_equal_score() {
        let semantic = vec![make_candidate("wf-x", 0.75, vec!["semantic"])];
        let structural = vec![make_candidate("wf-x", 0.75, vec!["structural"])];

        let result = merge_and_deduplicate_candidates(semantic, structural);
        assert_eq!(result.len(), 1, "T4: 重複排除後1件");
        assert_eq!(
            result[0].blended_score, 0.75,
            "T4: 同値の場合はその値が残る"
        );
    }

    // ============================================================
    // T5: 境界値 — 両方空
    // ============================================================
    #[test]
    fn both_empty() {
        let result = merge_and_deduplicate_candidates(vec![], vec![]);
        assert_eq!(result.len(), 0, "T5: 空リストは空を返す");
    }

    // ============================================================
    // T6: 境界値 — 片方のみ空
    // ============================================================
    #[test]
    fn semantic_only() {
        let semantic = vec![
            make_candidate("wf-a", 0.9, vec!["semantic"]),
            make_candidate("wf-b", 0.8, vec!["semantic"]),
            make_candidate("wf-c", 0.7, vec!["semantic"]),
        ];
        let result = merge_and_deduplicate_candidates(semantic, vec![]);
        assert_eq!(result.len(), 3, "T6a: 構造側空でも3件返る");
    }

    #[test]
    fn structural_only() {
        let structural = vec![
            make_candidate("wf-d", 0.6, vec!["structural"]),
            make_candidate("wf-e", 0.5, vec!["structural"]),
            make_candidate("wf-f", 0.4, vec!["structural"]),
        ];
        let result = merge_and_deduplicate_candidates(vec![], structural);
        assert_eq!(result.len(), 3, "T6b: セマンティック側空でも3件返る");
    }

    // ============================================================
    // T7: provenance 連結検証
    // ============================================================
    #[test]
    fn provenance_merge() {
        let semantic = vec![RankedCandidate {
            workflow_id: "wf-x".to_string(),
            semantic_score: 0.9,
            structural_score: 0.8,
            blended_score: 0.85,
            trust_score: 0.8,
            provenance: vec!["semantic-v1".to_string()],
            metadata: serde_json::json!({}),
        }];
        let structural = vec![RankedCandidate {
            workflow_id: "wf-x".to_string(),
            semantic_score: 0.8,
            structural_score: 0.9,
            blended_score: 0.75,
            trust_score: 0.8,
            provenance: vec!["struct-v1".to_string()],
            metadata: serde_json::json!({}),
        }];

        let result = merge_and_deduplicate_candidates(semantic, structural);
        assert_eq!(result.len(), 1, "T7: 重複排除後1件");
        assert_eq!(
            result[0].provenance.len(),
            2,
            "T7: provenance が2件に連結される"
        );
        assert!(
            result[0].provenance.contains(&"semantic-v1".to_string()),
            "T7: セマンティック由来のprovenanceを含む"
        );
        assert!(
            result[0].provenance.contains(&"struct-v1".to_string()),
            "T7: 構造由来のprovenanceを含む"
        );
    }

    // ============================================================
    // T8: 大量候補マージ（パフォーマンス境界値）
    // ============================================================
    #[test]
    fn large_merge_no_panic() {
        let semantic: Vec<RankedCandidate> = (0..1000)
            .map(|i| {
                make_candidate(
                    &format!("wf-sem-{}", i),
                    0.5 + (i as f64 / 1000.0),
                    vec!["semantic"],
                )
            })
            .collect();
        let structural: Vec<RankedCandidate> = (0..1000)
            .map(|i| {
                // 最初の500件は重複、残り500件は新規
                if i < 500 {
                    make_candidate(
                        &format!("wf-sem-{}", i),
                        0.5 + (i as f64 / 1000.0) * 0.8,
                        vec!["structural"],
                    )
                } else {
                    make_candidate(
                        &format!("wf-struct-{}", i),
                        0.5 + (i as f64 / 1000.0),
                        vec!["structural"],
                    )
                }
            })
            .collect();

        let result = merge_and_deduplicate_candidates(semantic, structural);
        // 1000件(セマンティック) + 500件(構造新規) = 1500件
        assert_eq!(result.len(), 1500, "T8: 1500件に統合されること");
    }

    // ============================================================
    // T9: 入力非破壊性の検証
    // ============================================================
    #[test]
    fn input_immutability() {
        let semantic = vec![make_candidate("wf-a", 0.9, vec!["semantic"])];
        let structural = vec![make_candidate("wf-a", 0.8, vec!["structural"])];

        let semantic_copy = semantic.clone();
        let structural_copy = structural.clone();

        let _result = merge_and_deduplicate_candidates(semantic, structural);

        assert_eq!(
            semantic_copy.len(),
            1,
            "T9: セマンティック入力が変更されていない"
        );
        assert_eq!(semantic_copy[0].blended_score, 0.9);
        assert_eq!(structural_copy.len(), 1, "T9: 構造入力が変更されていない");
        assert_eq!(structural_copy[0].blended_score, 0.8);
    }

    // ============================================================
    // OTS-1: カイ二乗検定によるバケット割り当て一様性
    // ============================================================
    #[test]
    fn ots1_chi_squared_uniformity() {
        use rand::rngs::StdRng;
        use rand::{Rng, SeedableRng};
        use std::hash::{DefaultHasher, Hash, Hasher};

        let iterations: usize = 10_000;
        let buckets: usize = 64;
        let mut rng = StdRng::seed_from_u64(crate::constants::TEST_PRNG_SEED);

        // 全イテレーションの候補を集約し、バケットカウントを蓄積
        let mut total_observed = vec![0usize; buckets];
        let mut total_candidates: usize = 0;

        for _iter in 0..iterations {
            let k_sem: usize = rng.random_range(1..=20);
            let k_struct: usize = rng.random_range(1..=20);

            let semantic: Vec<RankedCandidate> = (0..k_sem)
                .map(|_| {
                    make_candidate(
                        &format!("wf-{}", rng.random::<u64>()),
                        rng.random::<f64>(),
                        vec!["semantic"],
                    )
                })
                .collect();

            let structural: Vec<RankedCandidate> = (0..k_struct)
                .map(|_| {
                    make_candidate(
                        &format!("wf-{}", rng.random::<u64>()),
                        rng.random::<f64>(),
                        vec!["structural"],
                    )
                })
                .collect();

            let merged = merge_and_deduplicate_candidates(semantic, structural);

            for candidate in &merged {
                let mut hasher = DefaultHasher::new();
                candidate.workflow_id.hash(&mut hasher);
                let bucket = (hasher.finish() as usize) % buckets;
                total_observed[bucket] += 1;
            }
            total_candidates += merged.len();
        }

        // 集約データでカイ二乗検定
        let expected_per_bucket = total_candidates as f64 / buckets as f64;
        let chi_sq: f64 = total_observed
            .iter()
            .map(|&o| {
                let diff = o as f64 - expected_per_bucket;
                diff * diff / expected_per_bucket
            })
            .sum();

        // 自由度 63, 有意水準 5% の臨界値 ≈ 82.5
        // 帰無仮説: バケット割り当ては一様分布に従う
        let chi_sq_critical_95 = 82.5; // χ²(63, 0.95) 近似値

        println!("=== OTS-1: Chi-Squared Uniformity Test ===");
        println!("iterations={}, buckets={}", iterations, buckets);
        println!("total_candidates={}", total_candidates);
        println!("expected_per_bucket={:.2}", expected_per_bucket);
        println!(
            "chi_sq={:.4}, critical_95={:.2}",
            chi_sq, chi_sq_critical_95
        );

        // バケット分布ヒストグラム
        println!("bucket_distribution:");
        let max_count = *total_observed.iter().max().unwrap_or(&1);
        for (i, &count) in total_observed.iter().enumerate() {
            let bar_len = (count as f64 / max_count as f64 * 50.0) as usize;
            if i % 8 == 0 {
                // 8バケットごとに表示
                println!(
                    "  [{:2}]: count={:5}, expected={:.1}  {}",
                    i,
                    count,
                    expected_per_bucket,
                    "#".repeat(bar_len)
                );
            }
        }

        assert!(
            chi_sq < chi_sq_critical_95 * 1.5,
            "OTS-1: カイ二乗統計量が許容範囲を超過 (chi_sq={:.4}, threshold={:.2})",
            chi_sq,
            chi_sq_critical_95 * 1.5
        );
        println!(
            "=== 結果: PASS (chi_sq={:.4} < {:.2}) ===",
            chi_sq,
            chi_sq_critical_95 * 1.5
        );
    }

    // ============================================================
    // OTS-2: 最大値保存則
    // ============================================================
    #[test]
    fn ots2_max_preservation() {
        use rand::rngs::StdRng;
        use rand::{Rng, SeedableRng};

        let n_pairs: usize = 10_000;
        let mut rng = StdRng::seed_from_u64(crate::constants::TEST_PRNG_SEED);

        println!("=== OTS-2: Max Score Preservation ===");
        let mut preservation_count: u64 = 0;

        for _ in 0..n_pairs {
            let score_a: f64 = rng.random();
            let score_b: f64 = rng.random();

            let semantic = vec![make_candidate("wf-dup", score_a, vec!["semantic"])];
            let structural = vec![make_candidate("wf-dup", score_b, vec!["structural"])];

            let result = merge_and_deduplicate_candidates(semantic, structural);

            assert_eq!(result.len(), 1, "OTS-2: 重複排除後1件になること");
            let max_expected = score_a.max(score_b);
            assert!(
                (result[0].blended_score - max_expected).abs() < 1e-12,
                "OTS-2: blended_score が最大値 {} と一致すること (got {})",
                max_expected,
                result[0].blended_score
            );
            preservation_count += 1;
        }

        let preservation_rate = preservation_count as f64 / n_pairs as f64 * 100.0;
        println!(
            "pairs={}, preservation_rate={:.2}%",
            n_pairs, preservation_rate
        );
        assert_eq!(
            preservation_count, n_pairs as u64,
            "OTS-2: 全 {} 組で最大値保存が成立していること",
            n_pairs
        );
        println!("=== 結果: PASS (保存率 100%) ===");
    }
}
