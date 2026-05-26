# 変更したファイル一覧と実装内容の概要

## src/compiler.rs (新規作成)
- `compile_to_steps(graph: &WorkflowGraph) -> Result<Vec<NodeId>, DarviumError>` 関数
  - petgraph の toposort を使用した DAG のトポロジカルソート
  - 循環依存検出時は DarviumError::CycleDetected を返す
  - 空グラフは空の Vec を返す
  - SubWorkflow ノードはそのまま NodeId として含める
  - エラーハンドリングは ? 演算子で伝播、unwrap 不使用
- mod tests: TC1-TC10 (11 tests) を含む
  - TC1: 単純 DAG ソート
  - TC2: 空グラフ
  - TC3: 単一ノード
  - TC4: 循環依存検出
  - TC4b: 自己ループ検出
  - TC5: 分岐・合流 DAG
  - TC6: SubWorkflow ノード
  - TC7: ErrorMode 全バリアント構築
  - TC8: StepExecutionResult 全フィールド構築・アクセス
  - TC9: StepStatus 全バリアント構築
  - TC10: 100-node DAG パフォーマンス観測 (16.5μs)

## src/types.rs (修正)
- ErrorMode 列挙型: FailOnAny, SkipOnError, Degrade, RetryOnError(u32)
- StepStatus 列挙型: Success, Failure, PartialSuccess, Skipped
- StepExecutionResult 構造体: node_id, status, output, error, duration_ticks
- 全て Debug/Clone/PartialEq を derive (SideEffectSet と同一スタイル)

## src/lib.rs (修正)
- pub mod compiler; の追加
- pub use compiler::compile_to_steps; の追加
- pub use types::{ErrorMode, StepStatus, StepExecutionResult}; の追加
