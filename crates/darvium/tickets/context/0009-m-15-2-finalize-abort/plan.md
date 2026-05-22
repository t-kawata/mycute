# 計画: M-1.5-2 終端状態非再入不変条件の強制

## 変更ファイル一覧

| ファイル | 種別 | 内容 |
|---------|------|------|
| src/types.rs | 実装 | impl SearchState { transition_to(...) } ブロック追加 |
| src/types.rs | 実装 | TerminalTransitionReason Enum 追加 |
| src/types.rs | 実装 | can_terminate_with() 関数追加 |
| src/types.rs | テスト | T1〜T6、OTS-1/2/3 の全テスト追加 |
| src/lib.rs | 更新 | TerminalTransitionReason の pub use 行追加 |

## 実装手順

1. TerminalTransitionReason Enum + can_terminate_with 追加
2. impl SearchState と transition_to メソッド追加
3. src/lib.rs に TerminalTransitionReason の公開追加
4. テスト追加（T1〜T6、OTS-1〜OTS-3）
5. cargo test 検証

## 物理的レビュー方法

- cargo test 全テスト PASS
- cargo clippy -- -D warnings
- 翻訳可能性 grep（関数名が動詞句、1文字変数なし）
- RFC §13.5 交叉参照

## リスク

- TerminalStateViolation の網羅漏れ（低）
- 既存テスト退行（低、対策:cargo test 前後実行）
- マルチスレッドパルステスト flaky（低、対策:Arc<Mutex>）
