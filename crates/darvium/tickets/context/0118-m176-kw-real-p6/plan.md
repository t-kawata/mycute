# P6 計装インターフェース更新: 実装計画

## 要件

RFC §15.9.2 の 5 因子乗算結合モデルへの改訂（KindWorldMetricsInput +8 フィールド、compute_kind_world_objective 書き換え、KindWorldAssessment 拡張、collect_final_metrics 引数変更、observer 新メソッド、compare_j_kw_models 互換性診断、KW_ALPHA_* 削除）。

## 変更ファイル一覧

| ファイル | 種別 | 内容 |
|---------|------|------|
| src/kind_world.rs | 変更 | 構造体拡張、関数書き換え、observer 新メソッド、互換性診断、テスト更新 |
| src/constants.rs | 変更 | KW_ALPHA_* 6 定数削除 |

## 実装手順

1. KindWorldMetricsInput 拡張（8 フィールド追加 + zero() 更新）
2. KindWorldAssessment 拡張（5 因子 + 14 下位成分、flags → legacy_flags）
3. compute_kind_world_objective 書き換え（5 因子乗算結合 + 最小値ゲート）
4. constants.rs KW_ALPHA_* 6 定数削除
5. collect_final_metrics 引数型変更 + 呼び出し元更新
6. observer 新メソッド追加
7. compare_j_kw_models + JkwModelComparison 実装
8. テスト更新・追加
9. cargo build → cargo test → cargo clippy

## 計装・観測の実装計画

- src/kind_world.rs mod tests: 既存テスト更新 + 新テスト追加
- 固定シード: StdRng::seed_from_u64(12345)
- 観測テスト: n=10,000 ランダム入力で 5 因子統計出力
- 互換性診断テスト: n=10,000 で新旧 J_kw 比較

## Boy Scout 改善

- collect_final_metrics の引数変更（自然な改善）
- 特段の追加改善なし

## レビュー方法

- run-quality-checks.js + generate-report.js
- 翻訳可能性 grep（関数名・変数名・定数化）

## リスク

- collect_final_metrics 引数変更 → コンパイラが全呼び出し箇所検出
- 既存テストへの新フィールド追加漏れ → チェックリスト管理
- J_cost 定義変更（1.0 - cost_efficiency → cost_efficiency）→ RFC に従う
