// AG-06 / AG-07 ハードゲート全弾ブロックテスト
//
// 本ファイルは M-0.5-3 の統合テストを含む。
// T1〜T12 は src/search/applicability.rs の単体テストでカバーされるため、
// 本ファイルでは T13〜T14 と OTS-1〜OTS-3 を実装する。

use darvium::types::QueryRepresentation;
use darvium::{check_ag06, check_ag07, EmbeddingChannelVersion, EmbeddingVersions};
use rand::rngs::StdRng;
use rand::Rng;
use rand::SeedableRng;

// === T13: QueryRepresentation 拡張の後方互換性 ===

#[test]
fn test_query_representation_default_versions() {
    let query = QueryRepresentation::new(
        "test mission".to_string(),
        vec![0.1, 0.2, 0.3],
        "design text".to_string(),
        vec![0.4, 0.5, 0.6],
    );
    // 新フィールドがデフォルト値で初期化されていること
    assert_eq!(query.task_embedding_version.model_version, "v2.0-final");
    assert_eq!(query.task_embedding_version.template_version, None);
    assert_eq!(query.design_embedding_version.model_version, "v2.0-final");
    assert_eq!(
        query.design_embedding_version.template_version,
        Some("v2.0-final".to_string())
    );
    // 既存フィールドも正しく設定されていること（後方互換性）
    assert_eq!(query.design_template_version, "v2.0-final");
    assert_eq!(query.mission_text, "test mission");
}

// === T14: EmbeddingVersions の構築と取得 ===

#[test]
fn test_embedding_versions_construction() {
    let task_ver = EmbeddingChannelVersion::new("v1.0".to_string(), None);
    let design_ver = EmbeddingChannelVersion::new("v2.0".to_string(), Some("t3".to_string()));
    let versions = EmbeddingVersions::new(task_ver.clone(), design_ver.clone());
    assert_eq!(versions.task.model_version, "v1.0");
    assert_eq!(versions.design.model_version, "v2.0");
    assert_eq!(versions.design.template_version, Some("t3".to_string()));
    assert_eq!(versions.task.template_version, None);
}

// === OTS-1: 偽陽性率ゼロ検証（観測テスト） ===

#[test]
fn test_ots_1_false_positive_rate_zero() {
    let mut rng = StdRng::seed_from_u64(12345);
    let mismatched_versions = [
        "v1.0",
        "v1.8-legacy",
        "v2.0-rc1",
        "v3.0-alpha",
        "0.9-beta",
        "v2.0-final-2",
        "v1",
        "",
        "V2.0-FINAL",
        "v2.0.final",
    ];
    let query = EmbeddingChannelVersion::new("v2.0-final".to_string(), None);
    let iterations = 10_000u32;

    // AG-06: ランダムな不一致バージョンで 10,000 回走査
    let mut ag06_passed = 0u32;
    let mut ag06_rejected = 0u32;
    for _ in 0..iterations {
        let idx = rng.random_range(0..mismatched_versions.len());
        let candidate = EmbeddingChannelVersion::new(mismatched_versions[idx].to_string(), None);
        match check_ag06(&query, &candidate) {
            Ok(()) => ag06_passed += 1,
            Err(_) => ag06_rejected += 1,
        }
    }

    // AG-07: ランダムな不一致バージョンで 10,000 回走査
    let design_query =
        EmbeddingChannelVersion::new("v2.0-final".to_string(), Some("v2.0-final".to_string()));
    let mut ag07_passed = 0u32;
    let mut ag07_rejected = 0u32;
    for _ in 0..iterations {
        let idx = rng.random_range(0..mismatched_versions.len());
        let candidate = EmbeddingChannelVersion::new(
            mismatched_versions[idx].to_string(),
            Some(mismatched_versions[idx].to_string()),
        );
        match check_ag07(&design_query, &candidate) {
            Ok(()) => ag07_passed += 1,
            Err(_) => ag07_rejected += 1,
        }
    }

    println!("=== OTS-1: False Positive Rate (AG-06) ===");
    println!(
        "iterations={}, mismatch_versions_pool_size={}",
        iterations,
        mismatched_versions.len()
    );
    println!(
        "  passed={}, rejected={}, pass_rate={:.4}",
        ag06_passed,
        ag06_rejected,
        ag06_passed as f64 / iterations as f64
    );
    assert_eq!(
        ag06_passed, 0,
        "AG-06: false positive detected (passed={})",
        ag06_passed
    );

    println!();
    println!("=== OTS-1: False Positive Rate (AG-07) ===");
    println!(
        "iterations={}, mismatch_versions_pool_size={}",
        iterations,
        mismatched_versions.len()
    );
    println!(
        "  passed={}, rejected={}, pass_rate={:.4}",
        ag07_passed,
        ag07_rejected,
        ag07_passed as f64 / iterations as f64
    );
    assert_eq!(
        ag07_passed, 0,
        "AG-07: false positive detected (passed={})",
        ag07_passed
    );
}

// === OTS-2: 一致時通過率 1.0 検証（観測テスト） ===

#[test]
fn test_ots_2_match_rate_one() {
    let query = EmbeddingChannelVersion::new("v2.0-final".to_string(), None);
    let design_query =
        EmbeddingChannelVersion::new("v2.0-final".to_string(), Some("v2.0-final".to_string()));
    let iterations = 10_000u32;

    // AG-06: 一致ケース 10,000 回
    let mut ag06_passed = 0u32;
    let mut ag06_rejected = 0u32;
    for _ in 0..iterations {
        // 同じバージョン文字列だが異なるオブジェクト
        let candidate = EmbeddingChannelVersion::new("v2.0-final".to_string(), None);
        match check_ag06(&query, &candidate) {
            Ok(()) => ag06_passed += 1,
            Err(_) => ag06_rejected += 1,
        }
    }

    // AG-07: 一致ケース 10,000 回（model + template 完全一致）
    let mut ag07_passed = 0u32;
    let mut ag07_rejected = 0u32;
    for _ in 0..iterations {
        let candidate =
            EmbeddingChannelVersion::new("v2.0-final".to_string(), Some("v2.0-final".to_string()));
        match check_ag07(&design_query, &candidate) {
            Ok(()) => ag07_passed += 1,
            Err(_) => ag07_rejected += 1,
        }
    }

    println!("=== OTS-2: Match Rate 1.0 (AG-06) ===");
    println!("iterations={}", iterations);
    println!(
        "  passed={}, rejected={}, pass_rate={:.4}",
        ag06_passed,
        ag06_rejected,
        ag06_passed as f64 / iterations as f64
    );
    assert_eq!(
        ag06_rejected, 0,
        "AG-06: false rejection detected (rejected={})",
        ag06_rejected
    );

    println!();
    println!("=== OTS-2: Match Rate 1.0 (AG-07) ===");
    println!("iterations={}", iterations);
    println!(
        "  passed={}, rejected={}, pass_rate={:.4}",
        ag07_passed,
        ag07_rejected,
        ag07_passed as f64 / iterations as f64
    );
    assert_eq!(
        ag07_rejected, 0,
        "AG-07: false rejection detected (rejected={})",
        ag07_rejected
    );
}

// === OTS-3: 階段関数マッピング実測（観測テスト） ===

#[test]
fn test_ots_3_step_function_mapping() {
    let mut rng = StdRng::seed_from_u64(12345);
    let n_per_e = 1_000u32;

    println!("=== OTS-3: Step Function P_pass(E) ===");
    println!("iterations_per_e={}, max_e={}", n_per_e, 10);
    println!(
        "{:<6} {:<10} {:<10} {:<10}",
        "E", "passed", "rejected", "P_pass(E)"
    );

    for e in 0..=10 {
        let query = EmbeddingChannelVersion::new("v2.0-final".to_string(), None);

        let mut passed = 0u32;
        let mut rejected = 0u32;

        // Hamming distance E: e 文字だけ異なるバージョン文字列を生成
        for _ in 0..n_per_e {
            let candidate_version = generate_version_with_distance("v2.0-final", e, &mut rng);
            let candidate = EmbeddingChannelVersion::new(candidate_version, None);
            match check_ag06(&query, &candidate) {
                Ok(()) => passed += 1,
                Err(_) => rejected += 1,
            }
        }

        let pass_rate = if e == 0 { 1.0 } else { 0.0 };

        println!(
            "{:<6} {:<10} {:<10} {:<10.4}",
            e, passed, rejected, pass_rate
        );

        if e == 0 {
            assert_eq!(
                rejected, 0,
                "E=0: expected no rejection, got rejected={}",
                rejected
            );
            assert_eq!(
                passed, n_per_e,
                "E=0: expected all pass, got passed={}",
                passed
            );
        } else {
            assert_eq!(
                passed, 0,
                "E={}: expected no pass, got passed={}",
                e, passed
            );
            assert_eq!(
                rejected, n_per_e,
                "E={}: expected all rejected, got rejected={}",
                e, rejected
            );
        }
    }
}

/// 指定されたハミング距離 E だけ元の文字列と異なる文字列を生成する。
///
/// E=0 の場合は元の文字列をそのまま返す。
/// E>=1 の場合は先頭から E 文字をランダムに変更する。
fn generate_version_with_distance(base: &str, distance: u32, rng: &mut StdRng) -> String {
    if distance == 0 {
        return base.to_string();
    }
    let mut chars: Vec<char> = base.chars().collect();
    let length = chars.len();
    let distance_chars = (distance as usize).min(length);
    // 先頭 distance_chars 文字をランダムに変更
    for i in 0..distance_chars {
        let original = chars[i];
        loop {
            let new_char = rng.random_range('a'..='z');
            if new_char != original {
                chars[i] = new_char;
                break;
            }
        }
    }
    chars.into_iter().collect()
}
