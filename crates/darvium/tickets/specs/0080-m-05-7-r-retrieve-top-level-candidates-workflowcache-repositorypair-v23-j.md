---
ticket_id: 80
title: M-0.5-7-R: retrieve_top_level_candidates の WorkflowCache + RepositoryPair 移行（v2.3-j 追従）
slug: m-05-7-r-retrieve-top-level-candidates-workflowcache-repositorypair-v23-j
status: reviewed
created_at: 2026-05-25
updated_at: 2026-05-25
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0080-m-05-7-r-retrieve-top-level-candidates-workflowcache-repositorypair-v23-j/plan.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0080-m-05-7-r-retrieve-top-level-candidates-workflowcache-repositorypair-v23-j/observation-20260525-144228.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0080-m-05-7-r-retrieve-top-level-candidates-workflowcache-repositorypair-v23-j/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0080-m-05-7-r-retrieve-top-level-candidates-workflowcache-repositorypair-v23-j/review.md
---
# M-0.5-7-R: retrieve_top_level_candidates の WorkflowCache + RepositoryPair 移行（v2.3-j 追従）

## Summary

v2.3-j の用語是正により、M-0.5-7 で実装予定だった `retrieve_top_level_candidates` の関数シグネチャを `(q: &QueryRepresentation, repo: &WorkflowRepository)` ではなく `(q: &QueryRepresentation, cache: &WorkflowCache, pair: &RepositoryPair)` で実装する。加えて、全面的なキャッシュファースト設計（cache hit → 即時返却、cache miss → RepositoryPair から lazy load）を採用する。合わせて `evaluate_candidate`（適用性評価）を実装し、4 層検索パイプラインを完成させる。

## Background

v2.3-i まで `WorkflowRepository` が runtime cache と永続化の責務を兼ねていたが、v2.3-j RFC §8 で以下へ分割された：

- **WorkflowCache**: MemoizedGraph 群の runtime cache / in-memory working set
- **RepositoryPair**: SQLite + LadybugDB から構成される永続化ペア（source-of-truth）

✅ M-0.5-7-P（#79）で WorkflowCache / RepositoryPair / CacheError / PersistenceError の型定義基盤は完了済み。WorkflowCache::get_or_load は cache miss 時に `LoadFailed` を返す仮実装であり、本チケットで lazy load の完全実装を行う。

✅ M-0.5-7 の spec には `retrieve_top_level_candidates` が含まれていたが、その関数はまだソースコード上に存在しない（`src/search/` や `src/store/` に実装なし）。本チケットはこの関数を初めて実装する機会であり、同時に新しい責務区分に合わせたアーキテクチャで実装する。

## Scope

1. **`retrieve_top_level_candidates` 関数の新規実装**（`src/search/mod.rs` または新規 `src/search/pipeline.rs`）:
   - シグネチャ: `pub fn retrieve_top_level_candidates(q: &QueryRepresentation, cache: &WorkflowCache, pair: &RepositoryPair, k_sem: usize, k_meta: usize, k_cheap: usize, k_full: usize) -> Result<Vec<RankedCandidate>, DarviumError>`
   - Stage 1（Semantic Retrieval）: `semantic_topk(&q.task_embedding, cache, pair, K_SEM)` — task_embedding 余弦類似度 TopK。cache 内グラフの task_embedding と比較し、不足分は pair から load
   - Stage 2（Metadata Filter）: Stage 1 結果に対して M-0.5-5 の metadata_filter ロジックでフィルタ
   - Stage 3（Cheap GED Filter）: M-0.5-6 の cheap_ged_filter ロジックでフィルタ
   - Stage 4（Full GED Rerank）: M-0.5-6 の full_ged_rerank ロジックで rerank
   - Stage 5（Applicability Evaluation）: 各候補に `evaluate_candidate` を適用
   - 不変条件検証: N_sem ≥ N_meta ≥ N_cheap ≥ N_full（各 stage 通過後の候補数単調減少）

2. **`WorkflowCache::get_or_load` の完全実装**:
   - cache hit → 即時返却（現状維持）
   - cache miss → `RepositoryPair`（`DualStoreCoordinator`）の既存メソッドを利用してグラフデータを load → cache に昇格 → 返却
   - 現状の `Err(CacheError::LoadFailed(...))` スタブを置き換える

3. **`evaluate_candidate` 関数の新規実装**:
   - シグネチャ: `pub fn evaluate_candidate(q: &QueryRepresentation, g: &MemoizedGraph, full_ged: f32) -> ApplicabilityOutcome`
   - 式(6) S_sem = max(0, cosine(task_embedding))
   - 式(7) S_struct = exp(-λ * normalize_ged(full_ged, q, g))
   - 式(8) S_total = α * S_sem + (1-α) * S_struct
   - 式(9) A_workflow = max(S_total, f_S)^αS * max(D, f_D)^αD * max(T, f_T)^αT
   - 式(10) A_final = A_workflow^β * K^(1-β)（knowledge-aware 時）
   - 推奨初期値 α=0.45, λ=4.0, β=0.70
   - `ApplicabilityOutcome` 構造体の新規定義

4. **テスト構築**: `(WorkflowCache::in_memory(), WorkflowCache::in_memory_pair())` を使用

5. **各 stage の中間結果**（候補集合・スコア）を SearchTrace 互換で記録（構造体のフィールドに保持）

6. **全 stage の tie-break**: WorkflowGraphId の安定順序で固定（§12.3C）

## Non-scope

- 実埋め込みモデル（MockHnswIndex で代用）
- `semantic_topk` の本格的なベクトル検索（M-0.5-1 の FakeEmbedding で代用。`DualStoreCoordinator` の既存 `commit_dual_store_update` / `filter_retrieval_eligible` は引き続き利用）
- `metadata_filter` / `cheap_ged_filter` / `full_ged_rerank` のアルゴリズム自体の実装（M-0.5-5 / M-0.5-6 のロジックを利用。本チケットはそれらをパイプラインに統合するオーケストレーションが主責務）
- `evaluate_candidate` における信頼関連パラメータ（f_S, f_D, f_T 等）の詳細較正（較正フェーズは別チケット）
- LRU 等のキャッシュ逐次ポリシー
- 並列実行・非同期アクセス最適化
- エンドツーエンド LLM 結合（M2+）

## Investigation

### 参照観察レポート

過去の観察レポートはなし。本チケットは M-0.5-7-P の直後かつ M-0.5-7 の実質的初回実装であり、新しい型体系でのパイプライン確立が目的。

### ソースコード調査結果

1. **`retrieve_top_level_candidates` は未実装**: `src/` 配下全体を grep したが該当関数は存在しない。M-0.5-7 の spec（`Darvium-Tickets-v2.3.md:455-474`）で定義されているがコード化されていなかった。

2. **既存のパイプライン構成要素**:
   - `merge_and_deduplicate_candidates`（`src/store/mod.rs:32`）: 重複排除ロジックは実装済み・テスト済み
   - `evaluate_candidates`（`src/types.rs:4732`）: M-1-1 の静的閾値評価。M-0.5-7 で定義された `evaluate_candidate`（適用性評価・式6-10）とは別物
   - `check_ag06` / `check_ag07`（`src/search/applicability.rs:86`）: ハードゲート実装済み

3. **`WorkflowCache::get_or_load` のスタブ状態**（`src/store/workflow_cache.rs:182-201`）:
   - cache hit パス: `working_set` からの検索は実装済み
   - cache miss パス: `Err(CacheError::LoadFailed("...M-0.5-7-R..."))` のスタブ。「full RepositoryPair::load is implemented in M-0.5-7-R」のメッセージ通り、本チケットで完全実装が必要
   - `RepositoryPair` の `filter_retrieval_eligible`（`coordinator.rs:201`）は MemoizedGraph の eligibility 判定に利用可能

4. **テスト用コンストラクタ**:
   - `WorkflowCache::in_memory()`（`workflow_cache.rs:166`）: `CachePolicy::Default` + `MockHnswIndex` で初期化
   - `WorkflowCache::in_memory_pair()`（`workflow_cache.rs:171`）: `DualStoreCoordinator::new(InMemoryGraphStore, InMemoryMetadataStore)` で初期化
   - 両方ともテストで使用可能

5. **`MockHnswIndex::search` の実在**（`src/vector_index.rs:89`）: `search(&self, query: &[f32], k: usize) -> Result<Vec<(String, f64)>, DarviumError>` — cache 内グラフの埋め込み類似度検索に利用可能

## Test Plan

| # | テスト名 | 種別 | 内容 |
|---|---------|------|------|
| T1 | `pipeline_full_happy_path` | 正常系 | 全5 stage 通過。cache 内に全グラフが存在する状態での complete pipeline |
| T2 | `pipeline_cache_miss_some` | 正常系 | Stage 1 で一部グラフが cache 未登録 → RepositoryPair から lazy load |
| T3 | `pipeline_cache_miss_all` | 正常系 | 全グラフが cache 未登録 → 全件 RepositoryPair から lazy load |
| T4 | `pipeline_monotonicity` | 不変条件 | N_sem ≥ N_meta ≥ N_cheap ≥ N_full が維持されること |
| T5 | `pipeline_empty_result` | 境界値 | 全 stage で候補0 → 空リストが返ること |
| T6 | `pipeline_single_candidate` | 境界値 | 候補1件 → 正しく1件返ること |
| T7 | `pipeline_result_equivalence_seeded` | 正常系 | 固定 seed 条件下で改修前同等の結果が得られること（同一入力 → 同一出力） |
| T8 | `evaluate_candidate_high_semantic` | 正常系 | 高 task_embedding 類似度 → 高い適格性スコア |
| T9 | `evaluate_candidate_low_semantic` | 正常系 | 低 task_embedding 類似度 → 低い適格性スコア |
| T10 | `evaluate_candidate_high_ged` | 正常系 | 低 edit distance（類似度高）→ 高い構造スコア |
| T11 | `evaluate_candidate_low_ged` | 正常系 | 高 edit distance（類似度低）→ 低い構造スコア |
| T12 | `evaluate_candidate_zero_embedding` | 境界値 | ゼロベクトル埋め込み → S_sem=0.0 になること |
| T13 | `evaluate_candidate_max_ged` | 境界値 | GED が非常に大きい → S_struct が 0 に近づくこと |
| T14 | `get_or_load_cache_hit` | 正常系 | cache 内存在時は即時返却（既存 T1 の強化版） |
| T15 | `get_or_load_cache_miss_full` | 正常系 | cache miss → RepositoryPair から load → cache に昇格 → 次回 hit |
| T16 | `get_or_load_cache_miss_pair_missing` | 異常系 | RepositoryPair にも不在 → `CacheError::LoadFailed` |
| T17 | `pipeline_tie_break_stable` | 正常系 | tie 発生時は WorkflowGraphId の安定順序で整列 |
| T18 | `pipeline_search_trace_recorded` | 正常系 | 各 stage の中間結果が SearchTrace 互換のフィールドに記録されること |

## 計装方法・観測対象

本チケットは純粋な不変条件ベースのテストが主体であり、観測テストは限定的。

### 計装方法
- T2/T3 の lazy load パスで `println!` による cache miss → load → promote のトレース出力
- 固定シード PRNG は不使用（観測テストなし）
- 全テストは `assert_eq!` / `assert!` / `assert!(matches!())` で二値判定

### 観測対象
- パイプライン stage 通過ごとの候補数変化（デバッグ用 `println!`）
- lazy load の発生回数（デバッグ用 `println!`）

### 較正計画
本チケットでは較正不要（パラメータ α, λ, β は RFC 推奨初期値で固定）。

## Boy Scout Rule — 翻訳可能性計画

### 新規 `src/search/pipeline.rs` 作成時

- `retrieve_top_level_candidates` は動詞句として読み取れること（「最上位候補を検索する」）
- 各 stage は個別の内部関数として抽出（`stage1_semantic_retrieval`, `stage2_metadata_filter`, `stage3_cheap_ged_filter`, `stage4_full_ged_rerank`, `stage5_applicability_evaluation`）
- 関数呼び出しの並びがパイプラインの流れを散文として語ること
- 各 stage 通過後に候補数単調減少のアサーションを挿入

### `evaluate_candidate` 実装時

- 式(6)-(10)の各計算を個別の純粋関数に分割（`compute_semantic_score`, `compute_structural_score`, `compute_total_score`, `compute_workflow_applicability`, `compute_final_applicability`）
- 定数 α, λ, β は `constants.rs` の定数として定義（変更禁止の Safety Invariant または Calibration Candidate として分類）
- `ApplicabilityOutcome` 構造体は中間スコアフィールドを持つ（観測可能性の担保）

### `WorkflowCache::get_or_load` 完全実装時

- cache miss → load → promote → return の一連の流れが関数内で 3 ブロックとして明確に区切られていること
- エラーハンドリングは `CacheError` を通じて informatively に行う（`unwrap()` 不使用）

## Acceptance Criteria

- [ ] `retrieve_top_level_candidates` が `(q, cache, pair)` シグネチャで実装されている
- [ ] `evaluate_candidate` が RFC 式(6)-(10) に準拠して実装されている
- [ ] `ApplicabilityOutcome` 構造体が定義され、crate 公開 API としてエクスポートされている
- [ ] `WorkflowCache::get_or_load` が cache miss 時に RepositoryPair から正しく lazy load する
- [ ] 候補数単調減少不変条件（N_sem ≥ N_meta ≥ N_cheap ≥ N_full）が維持されている
- [ ] 全 stage の tie-break が WorkflowGraphId の安定順序で固定されている
- [ ] T1-T18 全テストが通過している
- [ ] 既存テスト（merge_and_deduplicate_candidates T1-T9、WorkflowCache T1-T17 等）に悪影響なし
- [ ] crate 公開 API として `retrieve_top_level_candidates` と `evaluate_candidate` がエクスポートされている

## Notes

- 本チケットは M-0.5-7-P（#79: WorkflowCache + RepositoryPair 型定義基盤）の完了を前提とする
- `merge_and_deduplicate_candidates` は `src/store/mod.rs` に既存。pipeline 内で semantic / structural 両系統の統合に Stage 2c として呼び出す
- `WorkflowCache::in_memory_pair()` の内部では既存の `InMemoryGraphStore` + `InMemoryMetadataStore`（M-2-1.5）を利用する
- `MockHnswIndex` の `search` は cache 内の全ベクトルとの線形探索を提供する。pipeline の semantic_topk はこれを利用する
- `DualStoreCoordinator` の `is_eligible_for_retrieval` / `filter_retrieval_eligible` は cache miss 時の Pair 側フィルタリングに利用可能
- `evaluate_candidate` は `evaluate_candidates`（M-1-1、静的閾値判定）とは別物。両方存在するが異なるユースケース

### 成果物

- 計画: context/0080-m-05-7-r-retrieve-top-level-candidates-workflowcache-repositorypair-v23-j/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0080-m-05-7-r-retrieve-top-level-candidates-workflowcache-repositorypair-v23-j/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0080-m-05-7-r-retrieve-top-level-candidates-workflowcache-repositorypair-v23-j/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0080-m-05-7-r-retrieve-top-level-candidates-workflowcache-repositorypair-v23-j/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
