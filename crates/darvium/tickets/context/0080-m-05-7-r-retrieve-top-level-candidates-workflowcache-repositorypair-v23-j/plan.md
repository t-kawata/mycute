# M-0.5-7-R: retrieve_top_level_candidates の WorkflowCache + RepositoryPair 移行（v2.3-j 追従）— 実装計画

## 要件の再確認
- `retrieve_top_level_candidates(q, cache, pair)` を新規実装（5 Stage パイプライン）
- `evaluate_candidate` を RFC 式(6)-(10) に基づき実装
- `ApplicabilityOutcome` 構造体を新規定義
- `WorkflowCache::get_or_load` の cache miss → lazy load を完全実装
- T1-T18 全テスト通過

## 変更ファイル一覧
| ファイル | 種別 | 内容 |
|---------|------|------|
| src/constants.rs | 修正 | パイプライン/Applicability 定数 15 種追加 |
| src/trust.rs | 修正 | MemoizedGraph に task_embedding フィールド追加 |
| src/store/coordinator.rs | 修正 | load_memoized_graph 公開メソッド追加 |
| src/store/workflow_cache.rs | 修正 | get_or_load cache miss → lazy load |
| src/search/pipeline.rs | 新規 | 全パイプライン関数 + テスト T1-T18 |
| src/search/mod.rs | 修正 | pub mod pipeline 追加 |
| src/lib.rs | 修正 | 公開 API エクスポート |

## 実装手順
1. constants.rs: 定数追加
2. trust.rs: task_embedding 追加
3. coordinator.rs: load_memoized_graph 追加
4. workflow_cache.rs: get_or_load lazy load
5. pipeline.rs: 全パイプライン実装 + テスト
6. search/mod.rs + lib.rs: モジュール登録・公開API
7. cargo test 全通過確認

## 物理的レビュー方法
- run-quality-checks.js で変更全ファイルを対象
- 翻訳可能性 grep（名詞始まり関数・1文字変数・直接数値）
- RFC §11.3 式(6)-(10) と §12.3D 疑似コードとの一致確認
- 既存テスト回帰確認（cargo test）
