# M1.76-21: 外部イベント購読基盤 — EventSubscriber + WebSocketEventChannel モック実装

## 要件
EventSubscriber + SubscriberManager + FakeWebSocketEventChannel + ExternalEventClient/FakeExternalEventClient を event_channel.rs に追加し、既存の EventChannel トレイトを活用して外部イベント購読基盤を構築する。

## RFC §12D 既存実装状態検証

### EventChannel トレイト
- connect/disconnect: 欠落（RFC async→同期版として設計変更済み）
- send: fn(DarviumEvent)→Result ✅ 同期版として一致
- subscribe: 欠落（代わりに receive/flush で設計変更）
- receive/flush: 余剰（設計変更による追加）

### WebSocketEventChannel / Subscription
- url: String ✅ 一致
- subscription: Option<Subscription> ✅ 一致
- id/kinds/channel: ✅ 全一致

## 変更ファイル一覧
| ファイル | 種別 | 内容 |
|----------|------|------|
| src/event_channel.rs | 編集 | 新規型追加 + テスト追加 |
| src/lib.rs | 編集 | 新規公開APIの re-export |
| src/constants.rs | 編集 | 購読者数上限・バッファ容量定数追加 |

## 実装手順
1. constants.rs に定数追加
2. event_channel.rs に型定義追加（SubscriptionId / SubscriberStatus / EventSubscriber / SubscriberManager / FakeWebSocketEventChannel / ExternalEventClient / FakeExternalEventClient）
3. lib.rs に re-export 追加
4. テスト追加（T1〜T7）

## レビュー方法
- cargo check + cargo clippy -- -D warnings
- cargo test -- --nocapture
- 翻訳可能性 grep
