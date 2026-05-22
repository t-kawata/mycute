# 実装計画: M-1-3 SearchRecursionExceeded 深さ制限ガードの強制

## 要件
- `guard_recursion_or_abort(guard, state)` インターセプタ関数の実装
- T1〜T6 ユニットテスト（全20ケース）
- OTS-1: global_allocator 計装によるアロケーションゼロ性検証
- OTS-2: スタックフレーム変位のカットオフ境界測定

## 変更ファイル一覧
| ファイル | 種別 | 内容 |
|---|---|---|
| src/types.rs:3667-3700 | 追加 | guard_recursion_or_abort 関数 |
| src/types.rs:3420-3433 | 追加 | T1〜T6 + OTS-1 + OTS-2 テスト |
| src/lib.rs:33-35 | 追加 | re-export |

## 実装手順
1. guard_recursion_or_abort を RecursionGuard impl 直後に追加
2. lib.rs の re-export に追加
3. テストモジュール末尾に全テスト追加
4. cargo test 通過確認
5. cargo clippy + cargo fmt 通過確認

## 物理的レビュー方法
- run-quality-checks.js で静的解析
- 翻訳可能性 grep（名詞始まり関数、汎用変数名）
- RFC §13.6 無矛盾チェック
