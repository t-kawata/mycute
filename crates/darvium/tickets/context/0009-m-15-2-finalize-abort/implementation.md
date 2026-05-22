# 実装サマリー: M-1.5-2 終端状態非再入不変条件の強制

## 変更ファイル

### src/types.rs
- `TerminalTransitionReason` Enum 追加（5 バリアント: BudgetExceeded, RecursionExceeded, ExplicitAbort, NormalCompletion, SingleCandidateFailure）
- `can_terminate_with(reason) -> bool` 関数追加（SingleCandidateFailure のみ false）
- `impl SearchState` ブロック追加:
  - `transition_to(&mut self, next) -> Result<(), DarviumError>` 
  - ガード優先順位: ①終端状態→TerminalStateViolation ②違法遷移→SearchValidation ③合法→状態更新
- テスト追加（T1〜T6, OTS-1/2/3）:
  - T1: 終端ガード個別検証（2 tests）
  - T2: 終端ガード全網羅（2 tests × 8 targets each）
  - T3: 全16合法遷移の成功確認（16 tests）
  - T4: 違法遷移拒否（1 test, 26 ペア）
  - T5: can_terminate_with 全5パターン（5 tests）
  - T6: 連続遷移の正確性（1 test）
  - OTS-1: マルチスレッドパルス注入（10 threads × 10,000 pulses）
  - OTS-2: ガードレイテンシ分布（10,000 samples）
  - OTS-3: can_terminate_with 判定表

### src/lib.rs
- `TerminalTransitionReason` を `pub use` に追加

### src/error.rs
- 変更なし（TerminalStateViolation は既存）

## 検証結果
- cargo test: 179 passed, 0 failed
- cargo clippy -- -D warnings: 通過
- 既存テスト（M-1.5-1）: 退行なし
- RFC §13.5 交叉参照: 終端状態非再入不変条件は transition_to のガード①で実装済み

## Boy Scout 改善
- src/lib.rs の pub use 行を複数行スタイルに整形（可読性向上）
