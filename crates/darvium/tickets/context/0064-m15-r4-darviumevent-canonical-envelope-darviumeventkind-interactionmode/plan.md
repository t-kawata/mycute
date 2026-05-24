# 実装計画: M1.5-R4 DarviumEvent canonical envelope + DarviumEventKind + InteractionMode 型定義

## 要件
RFC §12C に定義された DarviumEvent Architecture の全基盤型（25種類）を src/event.rs に新規定義する。純粋な型定義のみ。

## RFC 既存実装状態検証
- DarviumEvent 関連型: src/ 配下に一切の実装なし（ゼロからの新規実装）
- PiiHandlingPolicy: src/ 配下に未定義（Darvium-v2.3-final-table L1113 に定義あり）
- ReciprocityEvent struct: src/ 配下に未定義（命名衝突リスクなし）
- 依存関係: serde/serde_json/uuid/rand 全て Cargo.toml に存在

## 変更ファイル一覧
| ファイル | 種別 | 内容 |
| src/event.rs | 新規作成 | 全25種の型定義 + テストコード |
| src/lib.rs | 修正 | pub mod event; + 主要型の pub use エクスポート |

## 実装手順
1. src/event.rs に全補助型を定義（DeliveryMode → TransportMeta → EventVisibility → EventRetention → PiiHandlingPolicy → EventPrivacy → EventSource → EventMetadata → EventCausality）
2. InteractionMode を定義
3. 全11個の subtype enum を定義（SystemEvent 〜 HitlEvent）
4. DarviumEventKind を定義（13 variant）
5. DarviumEvent 構造体を定義（10フィールド）
6. 全型に #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)] を付与
7. src/lib.rs に pub mod event; + 主要型の pub use を追加
8. cargo check でコンパイル確認
9. テストコード（8 TC）を #[cfg(test)] mod tests に記述
10. cargo test で全テスト通過確認

## レビュー方法
1. run-quality-checks.js で品質チェック
2. 翻訳可能性 grep（名詞始まり関数、1文字変数、ハードコード数値）
3. RFC §12C.1-12C.3 の定義と実装を1フィールド単位で人手照合

## リスク
- ReciprocityEvent の将来の命名衝突: spec に明記済み
- SystemTime の serde: 標準サポートあり
