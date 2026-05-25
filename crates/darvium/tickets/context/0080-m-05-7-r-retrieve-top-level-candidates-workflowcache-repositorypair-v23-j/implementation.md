# 変更したファイル一覧と実装内容の概要

## 変更ファイル

| ファイル | 種別 | 内容 |
|---------|------|------|
| src/constants.rs | 修正 | 15のパイプライン定数追加 (SEMANTIC_TOPK_K_SEM〜APPLICABILITY_THRESHOLD) |
| src/trust.rs | 修正 | MemoizedGraph に task_embedding: Vec<f32> フィールド追加 |
| src/store/graph_store.rs | 修正 | GraphStore trait に load_embedding / store_workflow_graph_with_id 追加、InMemoryGraphStore/FailingGraphStore に実装 |
| src/store/coordinator.rs | 修正 | DualStoreCoordinator に store_memoized_graph / load_memoized_graph 追加 |
| src/store/workflow_cache.rs | 修正 | get_or_load のキャッシュミス時に RepositoryPair からのレイジーロードを実装 |
| src/search/pipeline.rs | 新規 | 4層検索パイプライン本体 + 18テスト |
| src/search/mod.rs | 修正 | pipeline モジュールの公開 |
| src/lib.rs | 修正 | evaluate_candidate, retrieve_top_level_candidates, ApplicabilityOutcome, PipelineTrace の公開APIエクスポート |

## 公開API

- `evaluate_candidate(query: &QueryRepresentation, graph: &MemoizedGraph, full_ged: f64) -> ApplicabilityOutcome`
  - RFC §11.3 式(6)-(10)の連鎖計算を実行
- `retrieve_top_level_candidates(q: &QueryRepresentation, cache: &WorkflowCache, pair: &RepositoryPair, k_sem: usize, k_meta: usize, k_cheap: usize, k_full: usize) -> Result<(Vec<RankedCandidate>, PipelineTrace), DarviumError>`
  - 5ステージパイプライン（セマンティック検索→メタデータフィルタ→CheapGED→FullGED→Applicability評価）
  - 単調性不変条件: N_sem >= N_meta >= N_cheap >= N_full

## RFC 準拠

- §11.3 式(6) S_sem = max(0, cosine(q, g))
- §11.3 式(7) S_struct = exp(-λ * normalize_ged(full_ged))
- §11.3 式(8) S_total = α * S_sem + (1-α) * S_struct
- §11.3 式(9) A_workflow = max(S,f_S)^αS * max(D,f_D)^αD * max(T,f_T)^αT
- §11.3 式(10) A_final = A_workflow^β * K^(1-β)
