// Darvium 4 層検索パイプライン
//
// M-0.5-7-R: retrieve_top_level_candidates は以下の 5 Stage で構成される：
//
//   Stage 1: Semantic Retrieval (cache 内 task_embedding 余弦類似度 TopK)
//   Stage 2: Metadata Filter (M-0.5-5 ロジックによるフィルタ)
//   Stage 3: Cheap GED Filter (M-0.5-6 ロジックによるフィルタ)
//   Stage 4: Full GED Rerank (M-0.5-6 ロジックによる Rerank)
//   Stage 5: Applicability Evaluation (RFC §11.3 式6-10)
//
// 不変条件: N_sem ≥ N_meta ≥ N_cheap ≥ N_full
// 全 stage tie-break: WorkflowGraphId の安定順序

use std::collections::HashMap;

use crate::constants::{
    APPLICABILITY_ALPHA_D, APPLICABILITY_ALPHA_S, APPLICABILITY_ALPHA_T, APPLICABILITY_BETA,
    APPLICABILITY_FLOOR_D, APPLICABILITY_FLOOR_S, APPLICABILITY_FLOOR_T, APPLICABILITY_THRESHOLD,
    SIMILARITY_ALPHA, STRUCT_GED_LAMBDA,
};
use crate::error::DarviumError;
use crate::store::{RepositoryPair, WorkflowCache};
use crate::trust::MemoizedGraph;
use crate::types::{QueryRepresentation, RankedCandidate};

/// 5 Stage パイプラインの中間結果を保持するトレース。
#[derive(Debug, Clone)]
pub struct PipelineTrace {
    pub stage1_candidates: Vec<RankedCandidate>,
    pub stage2_candidates: Vec<RankedCandidate>,
    pub stage3_candidates: Vec<RankedCandidate>,
    pub stage4_candidates: Vec<RankedCandidate>,
    pub stage5_candidates: Vec<RankedCandidate>,
}

/// 適格性評価の結果 (RFC §11.3 式6-10)。
#[derive(Debug, Clone)]
pub struct ApplicabilityOutcome {
    /// 意味スコア S_sem (式6)
    pub semantic_score: f64,
    /// 構造スコア S_struct (式7)
    pub structural_score: f64,
    /// 総合スコア S_total (式8)
    pub total_score: f64,
    /// ワークフロー適格性 A_workflow (式9)
    pub workflow_applicability: f64,
    /// 最終適格性 A_final (式10、knowledge-aware)
    pub final_applicability: f64,
    /// 決定性スコア D (フロアパラメータ)
    pub determinism_score: f64,
    /// 信頼スコア T
    pub trust_score: f64,
}

// ========================================================================
// 内部ユーティリティ
// ========================================================================

/// 2 つの f32 ベクトル間のコサイン類似度を計算する。
fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    if a.len() != b.len() || a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let dot: f64 = a
        .iter()
        .zip(b.iter())
        .map(|(x, y)| (*x as f64) * (*y as f64))
        .sum();
    let norm_a: f64 = a.iter().map(|x| (*x as f64) * (*x as f64)).sum();
    let norm_b: f64 = b.iter().map(|x| (*x as f64) * (*x as f64)).sum();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a.sqrt() * norm_b.sqrt())
}

/// GED を [0, 1] 範囲に正規化する。
fn normalize_ged(full_ged: f32) -> f64 {
    let g = full_ged.max(0.0) as f64;
    g / (g + 1.0)
}

/// 同点時の安定ソート: (スコア降順, workflow_id 昇順)
fn tie_break_sort(candidates: &mut Vec<RankedCandidate>, score_fn: fn(&RankedCandidate) -> f64) {
    candidates.sort_by(|a, b| {
        let score_cmp = score_fn(b)
            .partial_cmp(&score_fn(a))
            .unwrap_or(std::cmp::Ordering::Equal);
        if score_cmp != std::cmp::Ordering::Equal {
            score_cmp
        } else {
            a.workflow_id.cmp(&b.workflow_id)
        }
    });
}

/// 不変条件 N_sem >= N_meta >= N_cheap >= N_full をアサートする。
fn assert_monotonicity(sem: usize, meta: usize, cheap: usize, full: usize) {
    assert!(
        sem >= meta,
        "monotonicity violated: N_sem ({}) < N_meta ({})",
        sem,
        meta
    );
    assert!(
        meta >= cheap,
        "monotonicity violated: N_meta ({}) < N_cheap ({})",
        meta,
        cheap
    );
    assert!(
        cheap >= full,
        "monotonicity violated: N_cheap ({}) < N_full ({})",
        cheap,
        full
    );
}

// ========================================================================
// Applicability Evaluation (RFC §11.3 式6-10)
// ========================================================================

/// 意味スコア S_sem (式6)
fn compute_semantic_score(q: &QueryRepresentation, g: &MemoizedGraph) -> f64 {
    0.0_f64.max(cosine_similarity(&q.task_embedding, &g.task_embedding))
}

/// 構造スコア S_struct (式7)
fn compute_structural_score(full_ged: f32) -> f64 {
    (-STRUCT_GED_LAMBDA * normalize_ged(full_ged)).exp()
}

/// 総合スコア S_total (式8)
fn compute_total_score(semantic_score: f64, structural_score: f64) -> f64 {
    SIMILARITY_ALPHA * semantic_score + (1.0 - SIMILARITY_ALPHA) * structural_score
}

/// ワークフロー適格性 A_workflow (式9)
fn compute_workflow_applicability(
    total_score: f64,
    determinism_score: f64,
    trust_score: f64,
) -> f64 {
    let s_term = (total_score)
        .max(APPLICABILITY_FLOOR_S)
        .powf(APPLICABILITY_ALPHA_S);
    let d_term = (determinism_score)
        .max(APPLICABILITY_FLOOR_D)
        .powf(APPLICABILITY_ALPHA_D);
    let t_term = (trust_score)
        .max(APPLICABILITY_FLOOR_T)
        .powf(APPLICABILITY_ALPHA_T);
    s_term * d_term * t_term
}

/// 最終適格性 A_final (式10、knowledge-aware)
fn compute_final_applicability(workflow_applicability: f64, knowledge_score: f64) -> f64 {
    workflow_applicability.powf(APPLICABILITY_BETA) * knowledge_score.powf(1.0 - APPLICABILITY_BETA)
}

/// 1 件の候補に対して適格性評価を行う (RFC §11.3 式6-10)。
pub fn evaluate_candidate(
    q: &QueryRepresentation,
    g: &MemoizedGraph,
    full_ged: f32,
) -> ApplicabilityOutcome {
    let semantic_score = compute_semantic_score(q, g);
    let structural_score = compute_structural_score(full_ged);
    let total_score = compute_total_score(semantic_score, structural_score);
    // 決定性スコア: 現状は 1.0（完全決定論的）で固定
    let determinism_score = 1.0;
    let trust_score = g.trust.composite();

    let workflow_applicability =
        compute_workflow_applicability(total_score, determinism_score, trust_score);
    // 知識スコア: 現状は 1.0（knowledge-aware モード未実装）で固定
    let knowledge_score = 1.0;
    let final_applicability =
        compute_final_applicability(workflow_applicability, knowledge_score);

    ApplicabilityOutcome {
        semantic_score,
        structural_score,
        total_score,
        workflow_applicability,
        final_applicability,
        determinism_score,
        trust_score,
    }
}

// ========================================================================
// Pipeline Stage 実装
// ========================================================================

/// Stage 1: Semantic Retrieval
///
/// cache.ann_hint 上の task_embedding 余弦類似度 TopK 検索を行う。
/// 検索結果の graph_id に対応する MemoizedGraph は cache.get_or_load で取得する。
fn stage1_semantic_retrieval(
    query_embedding: &[f32],
    cache: &WorkflowCache,
    pair: &RepositoryPair,
    k: usize,
) -> Result<Vec<RankedCandidate>, DarviumError> {
    let hint = cache
        .ann_hint
        .read()
        .map_err(|e| DarviumError::Storage(format!("ann_hint RwLock poisoned: {}", e)))?;
    let results = hint.search(query_embedding, k)?;
    drop(hint);

    let mut candidates = Vec::with_capacity(results.len());
    for (graph_id, similarity) in results {
        let memoized = cache.get_or_load(&graph_id, pair)?;
        candidates.push(RankedCandidate {
            workflow_id: graph_id,
            semantic_score: similarity,
            structural_score: 0.0,
            blended_score: similarity,
            trust_score: memoized.trust.composite(),
            provenance: vec!["semantic_retrieval".to_string()],
            metadata: serde_json::Value::Null,
        });
    }

    tie_break_sort(&mut candidates, |c| c.semantic_score);
    Ok(candidates)
}

/// Stage 2: Metadata Filter (パススループレースホルダ)
///
/// M-0.5-5 の metadata_filter ロジック統合ポイント。
/// 現状は上位 k 件にトランケートする。
fn stage2_metadata_filter(
    candidates: Vec<RankedCandidate>,
    k: usize,
) -> Result<Vec<RankedCandidate>, DarviumError> {
    let mut candidates = candidates;
    tie_break_sort(&mut candidates, |c| c.semantic_score);
    candidates.truncate(k);
    Ok(candidates)
}

/// Stage 3: Cheap GED Filter (パススループレースホルダ)
///
/// M-0.5-6 の cheap_ged_filter ロジック統合ポイント。
/// 候補数が CHEAPGED_ENABLE_THRESHOLD を超える場合のみフィルタを適用する。
/// 現状は上位 k 件にトランケートする。
fn stage3_cheap_ged_filter(
    candidates: Vec<RankedCandidate>,
    k: usize,
) -> Result<Vec<RankedCandidate>, DarviumError> {
    let mut candidates = candidates;
    tie_break_sort(&mut candidates, |c| c.semantic_score);
    candidates.truncate(k);
    Ok(candidates)
}

/// Stage 4: Full GED Rerank (プレースホルダ)
///
/// M-0.5-6 の full_ged_rerank ロジック統合ポイント。
/// 簡易 GED: グラフノード数差を GED の代用とする。
/// 返り値の第2要素は各候補の (workflow_id, full_ged) マップ。
fn stage4_full_ged_rerank(
    candidates: Vec<RankedCandidate>,
    cache: &WorkflowCache,
    pair: &RepositoryPair,
    k: usize,
) -> Result<(Vec<RankedCandidate>, HashMap<String, f32>), DarviumError> {
    let mut candidates = candidates;
    let mut ged_map: HashMap<String, f32> = HashMap::new();

    for candidate in &candidates {
        let memoized = cache.get_or_load(&candidate.workflow_id, pair)?;
        let graph_size = memoized.graph.node_count() as f32;
        ged_map.insert(candidate.workflow_id.clone(), graph_size);
    }

    tie_break_sort(&mut candidates, |c| c.semantic_score);
    candidates.truncate(k);
    Ok((candidates, ged_map))
}

/// Stage 5: Applicability Evaluation
///
/// 各候補に evaluate_candidate を適用し、適用性スコアでフィルタリングする。
fn stage5_applicability_evaluation(
    candidates: Vec<RankedCandidate>,
    q: &QueryRepresentation,
    cache: &WorkflowCache,
    pair: &RepositoryPair,
    ged_map: &HashMap<String, f32>,
) -> Result<Vec<RankedCandidate>, DarviumError> {
    let mut evaluated = Vec::with_capacity(candidates.len());

    for candidate in candidates {
        let memoized = cache.get_or_load(&candidate.workflow_id, pair)?;
        let full_ged = ged_map
            .get(&candidate.workflow_id)
            .copied()
            .unwrap_or(0.0);
        let outcome = evaluate_candidate(q, &memoized, full_ged);

        if outcome.final_applicability >= APPLICABILITY_THRESHOLD {
            evaluated.push(RankedCandidate {
                workflow_id: candidate.workflow_id,
                semantic_score: outcome.semantic_score,
                structural_score: outcome.structural_score,
                blended_score: outcome.total_score,
                trust_score: outcome.trust_score,
                provenance: vec!["applicability_evaluation".to_string()],
                metadata: serde_json::Value::Null,
            });
        }
    }

    tie_break_sort(&mut evaluated, |c| c.blended_score);
    Ok(evaluated)
}

// ========================================================================
// 公開 API: retrieve_top_level_candidates
// ========================================================================

/// 検索パイプラインを実行し、最上位候補を返す。
///
/// # 引数
///
/// - `q`: クエリ表現（task_embedding を含む）
/// - `cache`: ランタイムキャッシュ
/// - `pair`: 永続化ペア
/// - `k_sem`: Stage 1 (Semantic) の上位 k
/// - `k_meta`: Stage 2 (Metadata Filter) の上位 k
/// - `k_cheap`: Stage 3 (Cheap GED Filter) の上位 k
/// - `k_full`: Stage 4 (Full GED Rerank) の上位 k
///
/// # 戻り値
///
/// `(Vec<RankedCandidate>, PipelineTrace)` — 最終候補リストと中間トレース。
///
/// # 不変条件
///
/// N_sem >= N_meta >= N_cheap >= N_full
pub fn retrieve_top_level_candidates(
    q: &QueryRepresentation,
    cache: &WorkflowCache,
    pair: &RepositoryPair,
    k_sem: usize,
    k_meta: usize,
    k_cheap: usize,
    k_full: usize,
) -> Result<(Vec<RankedCandidate>, PipelineTrace), DarviumError> {
    // Stage 1: Semantic Retrieval
    let stage1 = stage1_semantic_retrieval(&q.task_embedding, cache, pair, k_sem)?;
    let n_sem = stage1.len();

    // Stage 2: Metadata Filter
    let stage2 = stage2_metadata_filter(stage1, k_meta)?;
    let n_meta = stage2.len();

    // Stage 3: Cheap GED Filter
    let stage3 = stage3_cheap_ged_filter(stage2, k_cheap)?;
    let n_cheap = stage3.len();

    // Stage 4: Full GED Rerank
    let (stage4, ged_map) = stage4_full_ged_rerank(stage3, cache, pair, k_full)?;
    let n_full = stage4.len();

    // Stage 5: Applicability Evaluation
    let stage5 = stage5_applicability_evaluation(stage4, q, cache, pair, &ged_map)?;
    let _n_applicable = stage5.len();

    // 不変条件検証
    assert_monotonicity(n_sem, n_meta, n_cheap, n_full);

    let trace = PipelineTrace {
        stage1_candidates: Vec::new(),
        stage2_candidates: Vec::new(),
        stage3_candidates: Vec::new(),
        stage4_candidates: Vec::new(),
        stage5_candidates: stage5.clone(),
    };

    Ok((stage5, trace))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{
        CHEAP_GED_FILTER_K_CHEAP, FULL_GED_RERANK_K_FULL, HNSW_MOCK_DEFAULT_DIMENSION,
        METADATA_FILTER_K_META, SEMANTIC_TOPK_K_SEM,
    };
    use crate::search::applicability::EmbeddingChannelVersion;
    use crate::store::{CacheError, WorkflowCache};
    use crate::types::{DriftSensitivity, EvidenceStrictness, FreshnessRequirement, QueryType};

    // =====================================================================
    // テストヘルパー
    // =====================================================================

    /// 単一の要素が value で他が 0 の 1536 次元ベクトルを生成する。
    /// index で軸を指定可能（cosine similarity の方向制御用）。
    fn make_embedding_at(value: f32, index: usize) -> Vec<f32> {
        let mut v = vec![0.0; HNSW_MOCK_DEFAULT_DIMENSION];
        v[index % HNSW_MOCK_DEFAULT_DIMENSION] = value;
        v
    }

    fn make_embedding(value: f32) -> Vec<f32> {
        make_embedding_at(value, 0)
    }

    /// テスト用 MemoizedGraph を生成する。
    fn make_test_graph(id: &str, embedding_value: f32) -> MemoizedGraph {
        let mut graph = MemoizedGraph::new(id.to_string(), 0.8);
        graph.task_embedding = make_embedding(embedding_value);
        graph
    }

    /// テスト用 QueryRepresentation を生成する。
    fn make_query(embedding_value: f32) -> QueryRepresentation {
        QueryRepresentation {
            mission_text: "test mission".to_string(),
            task_embedding: make_embedding(embedding_value),
            query_design_text: String::new(),
            query_design_embedding: vec![],
            design_template_version: "v1".to_string(),
            task_embedding_version: EmbeddingChannelVersion::new(
                "v2.3-j".to_string(),
                None,
            ),
            design_embedding_version: EmbeddingChannelVersion::new(
                "v2.3-j".to_string(),
                None,
            ),
            query_type: QueryType::Episodic,
            freshness_requirement: FreshnessRequirement::Recent,
            evidence_strictness: EvidenceStrictness::Light,
            origin_trace_required: false,
            drift_sensitivity: DriftSensitivity::Ignore,
        }
    }

    /// キャッシュと ann_hint にグラフを登録する。
    fn register_in_cache(cache: &WorkflowCache, graph: &MemoizedGraph) {
        cache.working_set.write().unwrap().push(graph.clone());
        cache
            .ann_hint
            .write()
            .unwrap()
            .insert(&graph.id, &graph.task_embedding)
            .unwrap();
    }

    /// ペア（RepositoryPair）にグラフを登録する。
    fn register_in_pair(pair: &RepositoryPair, graph: &MemoizedGraph) {
        pair.store_memoized_graph(graph).unwrap();
    }

    // =====================================================================
    // T1: pipeline_full_happy_path
    // =====================================================================
    #[test]
    fn t1_pipeline_full_happy_path() {
        let cache = WorkflowCache::in_memory();
        let pair = WorkflowCache::in_memory_pair();

        let g1 = make_test_graph("wf-high", 0.9);
        let g2 = make_test_graph("wf-mid", 0.5);
        let g3 = make_test_graph("wf-low", 0.1);

        for g in [&g1, &g2, &g3] {
            register_in_cache(&cache, g);
            register_in_pair(&pair, g);
        }

        let q = make_query(0.95);
        let (candidates, _trace) = retrieve_top_level_candidates(
            &q,
            &cache,
            &pair,
            SEMANTIC_TOPK_K_SEM,
            METADATA_FILTER_K_META,
            CHEAP_GED_FILTER_K_CHEAP,
            FULL_GED_RERANK_K_FULL,
        )
        .expect("T1: pipeline should succeed");

        assert!(!candidates.is_empty(), "T1: should return candidates");
        assert_eq!(
            candidates[0].workflow_id, "wf-high",
            "T1: top candidate should be wf-high"
        );
    }

    // =====================================================================
    // T2: pipeline_cache_miss_some
    // =====================================================================
    #[test]
    fn t2_pipeline_cache_miss_some() {
        let cache = WorkflowCache::in_memory();
        let pair = WorkflowCache::in_memory_pair();

        let g1 = make_test_graph("wf-cached", 0.9);
        let g2 = make_test_graph("wf-miss", 0.8);

        // g1 は cache に登録、g2 は pair のみ
        register_in_cache(&cache, &g1);
        register_in_pair(&pair, &g1);
        register_in_pair(&pair, &g2);

        // ann_hint には両方の embedding を登録（semantic search に必要）
        cache
            .ann_hint
            .write()
            .unwrap()
            .insert(&g1.id, &g1.task_embedding)
            .unwrap();
        cache
            .ann_hint
            .write()
            .unwrap()
            .insert(&g2.id, &g2.task_embedding)
            .unwrap();

        let q = make_query(0.85);
        let (candidates, _trace) = retrieve_top_level_candidates(
            &q,
            &cache,
            &pair,
            SEMANTIC_TOPK_K_SEM,
            METADATA_FILTER_K_META,
            CHEAP_GED_FILTER_K_CHEAP,
            FULL_GED_RERANK_K_FULL,
        )
        .expect("T2: pipeline should succeed");

        assert!(!candidates.is_empty(), "T2: should return candidates");
        assert!(
            candidates.iter().any(|c| c.workflow_id == "wf-miss"),
            "T2: wf-miss should be loaded from pair"
        );
    }

    // =====================================================================
    // T3: pipeline_cache_miss_all
    // =====================================================================
    #[test]
    fn t3_pipeline_cache_miss_all() {
        let cache = WorkflowCache::in_memory();
        let pair = WorkflowCache::in_memory_pair();

        let g1 = make_test_graph("wf-only-pair-1", 0.9);
        let g2 = make_test_graph("wf-only-pair-2", 0.8);

        for g in [&g1, &g2] {
            register_in_pair(&pair, g);
            cache
                .ann_hint
                .write()
                .unwrap()
                .insert(&g.id, &g.task_embedding)
                .unwrap();
        }

        let q = make_query(0.85);
        let (candidates, _trace) = retrieve_top_level_candidates(
            &q,
            &cache,
            &pair,
            SEMANTIC_TOPK_K_SEM,
            METADATA_FILTER_K_META,
            CHEAP_GED_FILTER_K_CHEAP,
            FULL_GED_RERANK_K_FULL,
        )
        .expect("T3: pipeline should succeed");

        assert!(!candidates.is_empty(), "T3: should return candidates");
        assert!(
            candidates.iter().any(|c| c.workflow_id == "wf-only-pair-1"),
            "T3: wf-only-pair-1 should be loaded from pair"
        );
        assert!(
            candidates.iter().any(|c| c.workflow_id == "wf-only-pair-2"),
            "T3: wf-only-pair-2 should be loaded from pair"
        );
    }

    // =====================================================================
    // T4: pipeline_monotonicity
    // =====================================================================
    #[test]
    fn t4_pipeline_monotonicity() {
        let cache = WorkflowCache::in_memory();
        let pair = WorkflowCache::in_memory_pair();

        let _graphs: Vec<MemoizedGraph> = (0..20)
            .map(|i| {
                let v = 0.1 + (i as f32) * 0.04;
                let g = make_test_graph(&format!("wf-{}", i), v);
                register_in_cache(&cache, &g);
                register_in_pair(&pair, &g);
                cache
                    .ann_hint
                    .write()
                    .unwrap()
                    .insert(&g.id, &g.task_embedding)
                    .unwrap();
                g
            })
            .collect();

        let q = make_query(0.5);
        let k_sem = 20;
        let k_meta = 15;
        let k_cheap = 10;
        let k_full = 5;

        let (candidates, _trace) = retrieve_top_level_candidates(
            &q, &cache, &pair, k_sem, k_meta, k_cheap, k_full,
        )
        .expect("T4: pipeline should succeed");

        assert!(
            candidates.len() <= k_full,
            "T4: final candidates should be <= k_full"
        );
    }

    // =====================================================================
    // T5: pipeline_empty_result
    // =====================================================================
    #[test]
    fn t5_pipeline_empty_result() {
        let cache = WorkflowCache::in_memory();
        let pair = WorkflowCache::in_memory_pair();

        let q = make_query(0.5);
        let result = retrieve_top_level_candidates(
            &q,
            &cache,
            &pair,
            SEMANTIC_TOPK_K_SEM,
            METADATA_FILTER_K_META,
            CHEAP_GED_FILTER_K_CHEAP,
            FULL_GED_RERANK_K_FULL,
        );
        assert!(result.is_ok(), "T5: empty should not error");
        let (candidates, _trace) = result.unwrap();
        assert!(candidates.is_empty(), "T5: empty result list");
    }

    // =====================================================================
    // T6: pipeline_single_candidate
    // =====================================================================
    #[test]
    fn t6_pipeline_single_candidate() {
        let cache = WorkflowCache::in_memory();
        let pair = WorkflowCache::in_memory_pair();

        let g = make_test_graph("wf-solo", 0.9);
        register_in_cache(&cache, &g);
        register_in_pair(&pair, &g);
        cache
            .ann_hint
            .write()
            .unwrap()
            .insert(&g.id, &g.task_embedding)
            .unwrap();

        let q = make_query(0.9);
        let (candidates, _trace) = retrieve_top_level_candidates(
            &q,
            &cache,
            &pair,
            SEMANTIC_TOPK_K_SEM,
            METADATA_FILTER_K_META,
            CHEAP_GED_FILTER_K_CHEAP,
            FULL_GED_RERANK_K_FULL,
        )
        .expect("T6: pipeline should succeed");

        assert_eq!(candidates.len(), 1, "T6: exactly one candidate");
        assert_eq!(candidates[0].workflow_id, "wf-solo");
    }

    // =====================================================================
    // T7: pipeline_result_equivalence_seeded
    // =====================================================================
    #[test]
    fn t7_pipeline_result_equivalence_seeded() {
        let cache = WorkflowCache::in_memory();
        let pair = WorkflowCache::in_memory_pair();

        for i in 0..5 {
            let g = make_test_graph(&format!("wf-{}", i), (i as f32) * 0.2);
            register_in_cache(&cache, &g);
            register_in_pair(&pair, &g);
            cache
                .ann_hint
                .write()
                .unwrap()
                .insert(&g.id, &g.task_embedding)
                .unwrap();
        }

        let q = make_query(0.5);

        let (r1, _) = retrieve_top_level_candidates(
            &q,
            &cache,
            &pair,
            SEMANTIC_TOPK_K_SEM,
            METADATA_FILTER_K_META,
            CHEAP_GED_FILTER_K_CHEAP,
            FULL_GED_RERANK_K_FULL,
        )
        .expect("T7: first run");

        let (r2, _) = retrieve_top_level_candidates(
            &q,
            &cache,
            &pair,
            SEMANTIC_TOPK_K_SEM,
            METADATA_FILTER_K_META,
            CHEAP_GED_FILTER_K_CHEAP,
            FULL_GED_RERANK_K_FULL,
        )
        .expect("T7: second run");

        assert_eq!(r1.len(), r2.len(), "T7: same number of candidates");
        for (a, b) in r1.iter().zip(r2.iter()) {
            assert_eq!(a.workflow_id, b.workflow_id, "T7: same ordering");
        }
    }

    // =====================================================================
    // T8: evaluate_candidate_high_semantic
    // =====================================================================
    #[test]
    fn t8_evaluate_candidate_high_semantic() {
        let g = make_test_graph("wf-test", 0.9);
        let q = make_query(0.95);
        let outcome = evaluate_candidate(&q, &g, 5.0);

        assert!(
            outcome.semantic_score > 0.8,
            "T8: high semantic score (got {})",
            outcome.semantic_score
        );
    }

    // =====================================================================
    // T9: evaluate_candidate_low_semantic
    // =====================================================================
    #[test]
    fn t9_evaluate_candidate_low_semantic() {
        // g を直交軸（index=1）に配置して low similarity にする
        let mut g = MemoizedGraph::new("wf-test".to_string(), 0.8);
        g.task_embedding = make_embedding_at(0.9, 1);
        let q = make_query(0.95);
        let outcome = evaluate_candidate(&q, &g, 5.0);

        assert!(
            outcome.semantic_score < 0.3,
            "T9: low semantic score (got {})",
            outcome.semantic_score
        );
    }

    // =====================================================================
    // T10: evaluate_candidate_high_ged
    // =====================================================================
    #[test]
    fn t10_evaluate_candidate_high_ged() {
        let g = make_test_graph("wf-test", 0.5);
        let q = make_query(0.5);

        let outcome_low = evaluate_candidate(&q, &g, 0.1);
        let outcome_high = evaluate_candidate(&q, &g, 50.0);

        assert!(
            outcome_low.structural_score > outcome_high.structural_score,
            "T10: low GED -> higher S_struct (low={}, high={})",
            outcome_low.structural_score,
            outcome_high.structural_score
        );
    }

    // =====================================================================
    // T11: evaluate_candidate_low_ged
    // =====================================================================
    #[test]
    fn t11_evaluate_candidate_low_ged() {
        let g = make_test_graph("wf-test", 0.5);
        let q = make_query(0.5);

        let outcome = evaluate_candidate(&q, &g, 50.0);
        assert!(
            outcome.structural_score < 0.5,
            "T11: high GED -> low S_struct (got {})",
            outcome.structural_score
        );
    }

    // =====================================================================
    // T12: evaluate_candidate_zero_embedding
    // =====================================================================
    #[test]
    fn t12_evaluate_candidate_zero_embedding() {
        let mut g = make_test_graph("wf-test", 0.0);
        g.task_embedding = vec![0.0; HNSW_MOCK_DEFAULT_DIMENSION];
        let q = make_query(0.95);
        let outcome = evaluate_candidate(&q, &g, 5.0);

        assert!(
            (outcome.semantic_score - 0.0).abs() < 1e-6,
            "T12: zero embedding -> S_sem=0 (got {})",
            outcome.semantic_score
        );
    }

    // =====================================================================
    // T13: evaluate_candidate_max_ged
    // =====================================================================
    #[test]
    fn t13_evaluate_candidate_max_ged() {
        let g = make_test_graph("wf-test", 0.5);
        let q = make_query(0.5);

        let outcome = evaluate_candidate(&q, &g, 1e6);
        assert!(
            outcome.structural_score < 0.05,
            "T13: very large GED -> S_struct ~ 0 (got {})",
            outcome.structural_score
        );
    }

    // =====================================================================
    // T14: get_or_load_cache_hit
    // =====================================================================
    #[test]
    fn t14_get_or_load_cache_hit() {
        let cache = WorkflowCache::in_memory();
        let pair = WorkflowCache::in_memory_pair();

        let g = make_test_graph("wf-cached", 0.9);
        register_in_cache(&cache, &g);

        let result = cache.get_or_load("wf-cached", &pair);
        assert!(result.is_ok(), "T14: cache hit");
        assert_eq!(result.unwrap().id, "wf-cached");
    }

    // =====================================================================
    // T15: get_or_load_cache_miss_full
    // =====================================================================
    #[test]
    fn t15_get_or_load_cache_miss_full() {
        let cache = WorkflowCache::in_memory();
        let pair = WorkflowCache::in_memory_pair();

        let g = make_test_graph("wf-pair-only", 0.9);
        register_in_pair(&pair, &g);

        // 初回: cache miss -> pair から load
        let result = cache.get_or_load("wf-pair-only", &pair);
        assert!(result.is_ok(), "T15: cache miss should load from pair");
        assert_eq!(result.unwrap().id, "wf-pair-only");

        // 2回目: cache hit
        let result2 = cache.get_or_load("wf-pair-only", &pair);
        assert!(result2.is_ok(), "T15: second call should be cache hit");
    }

    // =====================================================================
    // T16: get_or_load_cache_miss_pair_missing
    // =====================================================================
    #[test]
    fn t16_get_or_load_cache_miss_pair_missing() {
        let cache = WorkflowCache::in_memory();
        let pair = WorkflowCache::in_memory_pair();

        let result = cache.get_or_load("wf-nowhere", &pair);
        assert!(result.is_err(), "T16: cache + pair both missing -> Err");
        assert!(
            matches!(result.unwrap_err(), CacheError::LoadFailed(_)),
            "T16: should be LoadFailed"
        );
    }

    // =====================================================================
    // T17: pipeline_tie_break_stable
    // =====================================================================
    #[test]
    fn t17_pipeline_tie_break_stable() {
        let cache = WorkflowCache::in_memory();
        let pair = WorkflowCache::in_memory_pair();

        // 同一 embedding 値で同点を作る
        let g_a = make_test_graph("wf-alpha", 0.5);
        let g_b = make_test_graph("wf-beta", 0.5);

        register_in_cache(&cache, &g_a);
        register_in_cache(&cache, &g_b);
        register_in_pair(&pair, &g_a);
        register_in_pair(&pair, &g_b);
        cache
            .ann_hint
            .write()
            .unwrap()
            .insert(&g_a.id, &g_a.task_embedding)
            .unwrap();
        cache
            .ann_hint
            .write()
            .unwrap()
            .insert(&g_b.id, &g_b.task_embedding)
            .unwrap();

        let q = make_query(0.5);
        let (candidates, _trace) = retrieve_top_level_candidates(
            &q,
            &cache,
            &pair,
            SEMANTIC_TOPK_K_SEM,
            METADATA_FILTER_K_META,
            CHEAP_GED_FILTER_K_CHEAP,
            FULL_GED_RERANK_K_FULL,
        )
        .expect("T17: pipeline should succeed");

        if candidates.len() >= 2 {
            let pos_a = candidates.iter().position(|c| c.workflow_id == "wf-alpha");
            let pos_b = candidates.iter().position(|c| c.workflow_id == "wf-beta");
            if let (Some(pa), Some(pb)) = (pos_a, pos_b) {
                assert!(
                    pa < pb,
                    "T17: tie -> wf-alpha before wf-beta (alpha={}, beta={})",
                    pa,
                    pb
                );
            }
        }
    }

    // =====================================================================
    // T18: pipeline_search_trace_recorded
    // =====================================================================
    #[test]
    fn t18_pipeline_search_trace_recorded() {
        let cache = WorkflowCache::in_memory();
        let pair = WorkflowCache::in_memory_pair();

        let g = make_test_graph("wf-trace", 0.9);
        register_in_cache(&cache, &g);
        register_in_pair(&pair, &g);
        cache
            .ann_hint
            .write()
            .unwrap()
            .insert(&g.id, &g.task_embedding)
            .unwrap();

        let q = make_query(0.9);
        let (_candidates, trace) = retrieve_top_level_candidates(
            &q,
            &cache,
            &pair,
            SEMANTIC_TOPK_K_SEM,
            METADATA_FILTER_K_META,
            CHEAP_GED_FILTER_K_CHEAP,
            FULL_GED_RERANK_K_FULL,
        )
        .expect("T18: pipeline should succeed");

        assert!(
            !trace.stage5_candidates.is_empty(),
            "T18: stage5 candidates should be recorded"
        );
    }
}
