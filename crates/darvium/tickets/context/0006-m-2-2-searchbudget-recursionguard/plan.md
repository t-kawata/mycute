# 実装計画: M-2-2 SearchBudget / RecursionGuard

## 要件
- RFC §13.3 に準拠した SearchBudget / SearchBudgetSnapshot / RecursionGuard の構造体定義
- デフォルト値コンストラクタ (Default トレイト)
- サチュレーティングインクリメント演算 (try_consume_iteration, try_consume_retrieval_call, try_consume_prompt_tokens, try_increment_depth, decrement_depth)
- 境界値テスト (T1〜T5) + 観測テスト (OTS-1)

## 変更ファイル
| ファイル | 種別 | 内容 |
|---------|------|------|
| src/constants.rs | 追加 | DEFAULT_MAX_ITERATIONS, DEFAULT_MAX_RETRIEVAL_CALLS, DEFAULT_MAX_WALL_CLOCK_MS, DEFAULT_RECURSION_MAX_DEPTH |
| src/types.rs | 修正 | SearchBudget/RecursionGuard フィールド修正 + SearchBudgetSnapshot 追加 + impl ブロック + テスト追加 |
| src/lib.rs | 修正 | pub use で SearchBudget/RecursionGuard/SearchBudgetSnapshot を再エクスポート |

## レビュー方法
1. run-quality-checks.js で変更ファイルをチェック
2. cargo test で全テストが通ること
3. cargo clippy -- -D warnings 通過
4. cargo fmt --check 通過
5. RFC §13.3 との目視一致確認
