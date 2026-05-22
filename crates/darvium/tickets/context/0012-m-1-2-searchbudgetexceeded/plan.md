# 計画: M-1-2 `SearchBudgetExceeded` ハードガードの遮断アサーション

## 要件
RFC §13.6 ガード条件「SearchBudget の上限超過時は `SearchBudgetExceeded` を返し、`Abort` へ遷移すること」を実装する。ループ開始前の事前検査として全4次元をまとめてチェックするインターセプタを純粋関数として追加する。

## 変更ファイル一覧
| ファイル | 種別 | 内容 |
|---|---|---|
| `src/types.rs` | 修正 | `check_budget_exceeded` + `guard_budget_or_abort` 関数追加、テスト追加（T1-T6、OTS-1、OTS-2） |
| `src/lib.rs` | 修正 | 新規関数の re-export 追加 |

## 実装手順
1. `check_budget_exceeded(budget, snapshot)` — 4次元すべての事前チェック、純粋検査
2. `guard_budget_or_abort(state, budget, snapshot)` — 状態遷移送携版インターセプタ
3. `lib.rs` re-export 追加
4. テストケース追加（T1-T6: ユニットテスト、OTS-1/OTS-2: 観測テスト）
5. cargo test で全テスト通過確認

## 計装・観測計画
- OTS-1: check_budget_exceeded の処理時間分散測定（ΔB sweep × 10水準、各10,000回）
- OTS-2: guard_budget_or_abort レイテンシ分布（正常系/超過系 各10,000回）
- 観測出力: println! CSV形式、--nocapture 経由

## 物理的レビュー方法
1. run-quality-checks.js で cargo check/clippy/fmt/test を実行
2. 翻訳可能性 grep: 関数名が動詞句、数値リテラルが不適切にハードコードされていないことを確認

## リスク
- 低: try_consume_* との責務境界を明確化（check_budget_exceeded は消費しない）
