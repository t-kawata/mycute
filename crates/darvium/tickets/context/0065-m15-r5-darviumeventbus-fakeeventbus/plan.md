# 計画: M1.5-R5 DarviumEventBus トレイト + FakeEventBus 実装

## 要件
- DarviumEventBus トレイト（8同期メソッド、Send + Sync）
- FakeEventBus メモリ内実装（Arc<Mutex<...>>）
- EventFilter 構造体 + EventSubscription トレイト + InteractionId newtype
- DarviumError に 3 variant 追加
- 11 TC のテスト実装

## 変更ファイル一覧
| ファイル | 種別 | 内容 |
|----------|------|------|
| src/event.rs | 編集（追加） | トレイト・構造体・テスト (R4 の後に追記) |
| src/error.rs | 編集（追加） | EventNotFound, InteractionNotFound, EventBus |
| src/lib.rs | 編集（追加） | pub use event に公開型追加 |

## 実装手順
1. error.rs に 3 variant 追加
2. event.rs に InteractionId, EventFilter, EventSubscription 追加
3. event.rs に DarviumEventBus トレイト追加
4. event.rs に FakeEventBus 実装追加
5. event.rs に 11 テスト追加 (R4 テスト群の後)
6. lib.rs の pub use event 更新
7. cargo test でコンパイル・テスト確認

## RFC 検証結果
DarviumEventBus トレイトは未実装（新規）。RFC §12C.5 との全差異はチケット仕様に基づく意図的変更である。

## レビュー方法
- run-quality-checks.js で静的チェック
- 翻訳可能性 grep（名詞始まり関数、1文字変数、unwrap）
- cargo test 全テスト通過確認

## リスク
- 低: 純メモリ内実装で外部依存なし
- 注意: InteractionId に Hash の derive 必須
