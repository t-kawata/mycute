# 実装サマリ: M-2-2 SearchBudget / RecursionGuard

## 変更内容

### src/constants.rs
- DEFAULT_MAX_ITERATIONS: u32 = 100 (Environment Policy Knob)
- DEFAULT_MAX_RETRIEVAL_CALLS: u32 = 50 (Environment Policy Knob)
- DEFAULT_MAX_WALL_CLOCK_MS: u64 = 30_000 (Environment Policy Knob)
- DEFAULT_RECURSION_MAX_DEPTH: u32 = 8 (Safety Invariant)

### src/types.rs
- SearchBudget を RFC §13.3 準拠に修正 (max_iterations, max_retrieval_calls, max_prompt_tokens, max_wall_clock_ms)
- SearchBudgetSnapshot を新規定義 (iterations_used, retrieval_calls_used, prompt_tokens_used, wall_clock_ms_used)
- RecursionGuard を RFC §13.3 準拠に修正 (max_depth: u32, current_depth: u32, allow_reentrant: bool)
- impl Default for SearchBudget — 全定数からデフォルト値設定
- impl SearchBudget: new(), try_consume_iteration(), try_consume_retrieval_call(), try_consume_prompt_tokens(), snapshot()
- impl Default for RecursionGuard — max_depth=8, current_depth=0, allow_reentrant=false
- impl RecursionGuard: new(), try_increment_depth(), decrement_depth()
- テスト16個追加 (T1-T5 + OTS-1)

### src/lib.rs
- pub use types::{SearchBudget, RecursionGuard, SearchBudgetSnapshot} を追加

## 検証結果
- cargo test: 100 passed, 0 failed
- cargo clippy -- -D warnings: 通過
- cargo fmt --check: 通過
- OTS-1: 10,000 アンサンブル全軌道が有限ステップ内で飽⼀することを確認 (τ_relax = 0)
- RFC §13.3 とのフィールド名・型一致: 確認済み
