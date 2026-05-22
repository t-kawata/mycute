# 計画: M-1.5-1 SearchState 合法状態遷移マトリクスの実装

## 要件
RFC §13.5 の 8 状態遷移規則をコード化する。
1. SearchState Enum（8 バリアント）の実装
2. is_legal_transition(from, to) -> bool 純粋関数
3. 総当たり 8×8 = 64 ペアのマトリクステスト + 観測テスト
4. pub use による公開

## 変更ファイル一覧
| ファイル | 種別 | 内容 |
|---------|------|------|
| src/types.rs | 追加 | SearchState Enum、is_legal_transition 関数、全テスト |
| src/lib.rs | 編集 | pub use types::SearchState を追加 |

## 実装手順
1. SearchState Enum 追加
2. is_legal_transition 関数追加（match パターンマッチ）
3. テスト追加（T1-T4 個別検証 + T3 総当たりマトリクス + OTS）
4. lib.rs 更新
5. cargo test 検証

## レビュー方法
- cargo build / cargo test / cargo clippy / cargo fmt
- 翻訳可能性 grep（関数名が動詞句、汎用変数名なし）
