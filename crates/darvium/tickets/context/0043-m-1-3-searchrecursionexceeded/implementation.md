# 変更したファイル一覧と実装内容の概要

## src/types.rs
- `guard_recursion_or_abort()` 関数を追加（4111行目）
  - RecursionGuard の深さ超過を検査し、超過時に SearchState を Abort に遷移
  - 終端状態からの呼び出しは TerminalStateViolation として拒否
  - guard_budget_or_abort と対称的な設計
- T1〜T6 のユニットテスト 20 ケースを追加（3522〜3870行目）
  - T1: 正常系3ケース（深さ0, 3, 7からのインクリメント）
  - T2: 異常系4ケース（超過、allow_reentrant=false、max_depth=0）
  - T3: 状態遷移5ケース（正常時不変、超過時Abort、終端状態優先等）
  - T4: 副作用ゼロ3ケース（エラー時current_depth不変等）
  - T5: 決定論性3ケース（同一入力一致、超過量sweep等）
  - T6: 統合テスト5ケース（3回成功→4回目ブロック等）
- OTS-1, OTS-2 観測テスト追加
  - OTS-1: 10,000回連続ガード発動のレイテンシ分布測定
  - OTS-2: 深度別スタックフレームアドレスサンプリング

## src/lib.rs
- guard_recursion_or_abort の re-export を追加

## テスト結果
- cargo test: 263 passed, 0 failed
- cargo clippy: 警告0
- cargo fmt: 通過
