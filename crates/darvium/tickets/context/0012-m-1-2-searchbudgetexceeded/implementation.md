# 変更したファイル一覧と実装内容の概要

## 変更ファイル

### `src/types.rs`
- `check_budget_exceeded(budget, snapshot) -> Result<(), DarviumError>` 関数を追加
  - 純粋検査関数。SearchBudget の 4 次元 (iterations, retrieval_calls, prompt_tokens, wall_clock_ms) すべてを事前チェック
  - iterations/retrieval_calls は `used >= max` (飽和セマンティクス)、prompt_tokens/wall_clock_ms は `used > max` (超過セマンティクス)
  - 副作用ゼロ (使用量の消費は行わない)
  - 早期 return 最適化: iterations → retrieval_calls → prompt_tokens → wall_clock_ms の順でチェック
- `guard_budget_or_abort(state, budget, snapshot) -> Result<(), DarviumError>` 関数を追加
  - 高階インターセプタ。`check_budget_exceeded` の結果が Err の場合、状態を `Abort` に変更
  - TerminalStateViolation 優先: 状態がすでに Finalize/Abort の場合は追加チェックを行わず Err を返す
- テスト追加: T1〜T6 (計 24 テスト関数)、OTS-1/OTS-2 (計 4 テスト関数)
  - T1: 正常系 5 テスト (各次元の上限未満 + 全次元半量)
  - T2: 異常系 6 テスト (各次元超過 + 境界値 exact limit)
  - T3: 複数次元超過 5 テスト
  - T4: guard_budget_or_abort 状態遷移 4 テスト
  - T5: 副作用ゼロ検証 2 テスト
  - T6: 決定論性 2 テスト
  - OTS-1: バッチ測定によるガード命令ステップ数分散 (11 ΔB 水準 × 100 サンプル)
  - OTS-2: guard_budget_or_abort レイテンシ分布 (10,000 サンプル × 2 系統)

### `src/lib.rs`
- re-export に `check_budget_exceeded`, `guard_budget_or_abort` を追加
