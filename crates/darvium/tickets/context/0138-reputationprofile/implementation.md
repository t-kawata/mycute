# 変更したファイル一覧と実装内容の概要

## 変更ファイル

| ファイル | 種別 | 内容 |
|----------|------|------|
| src/spaceposition.rs | 修正 | SpacePositionEmbedding に `Serialize, Deserialize` を derive 追加（serde インポート + derive 属性） |
| src/store/graph_store.rs | 修正 | GraphStore トレイトに `store_reputation` / `load_reputation` を追加。InMemoryGraphStore に `reputations: RefCell<HashMap<...>>` フィールド + 実装を追加。 |
| src/store/coordinator.rs | 修正 | `store_memoized_graph` で reputation 保存（非 fatal）。`load_memoized_graph` で reputation 読込 + NotFound/IOエラー区別フォールバック。FailingGraphStore に新メソッドの委譲実装を追加。テスト `t66-t68` を追加。 |

## 付随修正（Boy Scout Rule: 既存コンパイルエラーの修正）

| ファイル | 内容 |
|----------|------|
| src/search_workflow.rs | `WorkflowGraph` 型が未インポートだった問題を修正（`use crate::types::WorkflowGraph` を追加） |

## 実装設計判断

1. **非 fatal エラーハンドリング**: store_reputation の失敗は store_memoized_graph の成功/失敗に影響しない。load_reputation の失敗は cold_start() にフォールバックし、I/O エラー時のみ eprintln! で警告を出力する。
2. **NotFound と I/O エラーの区別**: `DarviumError::NotFound` は無音フォールバック、その他のエラーは警告付きフォールバック。
3. **MemoizedGraph 全体への Serialize/Deserialize 不追加**: WorkflowNode/EdgeMeta に serde derive がないため、MemoizedGraph 全体ではなく reputation のみ個別に store/load する design を採用。
4. **FailingGraphStore 対応**: 新規メソッドも既存パターン（`self.maybe_fail()?; self.inner.method_name(...)`）に従う。

## テスト結果

全1336テスト PASS（0 FAIL, 62 IGNORED）
- t66_reputation_store_load_roundtrip: OK
- t67_reputation_not_found_fallback: OK
- t68_store_memoized_graph_non_fatal: OK
