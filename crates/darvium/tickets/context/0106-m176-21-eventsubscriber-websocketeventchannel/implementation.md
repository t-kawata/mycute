# 実装サマリー: M1.76-21 外部イベント購読基盤

## 変更ファイル一覧

| ファイル | 種別 | 変更内容 |
|---------|------|---------|
| src/constants.rs | 定数追加 | MAX_SUBSCRIBERS=100, FAKE_WS_CHANNEL_BUFFER_SIZE=1024 |
| src/event_channel.rs | 新規型追加 (約600行) | SubscriptionId, SubscriberStatus, EventSubscriber, SubscriberSnapshot, SubscriberManager, FakeWebSocketEventChannel, ExternalEventClient, FakeExternalEventClient + T1-T7 全32テスト |
| src/lib.rs | re-export 更新 | 7新規型を公開APIに追加 |

## 実装した型

1. **SubscriptionId**: UUIDv4 文字列ラッパー newtype (new/Default/Display/From/Into)
2. **SubscriberStatus**: Active/Paused/Disconnected の3値 enum
3. **EventSubscriber**: subscription_id, filter, channel(Box<dyn EventChannel>), status, event_count
4. **SubscriberSnapshot**: チャネルを除いた購読者スナップショット（list() 戻り値用）
5. **SubscriberManager**: 購読登録/解除/一覧/分配 (Arc<Mutex<>> + MAX_SUBSCRIBERS 制限)
6. **FakeWebSocketEventChannel**: VecDeque ベースの EventChannel モック（capacity 制限付き）
7. **ExternalEventClient トレイト**: connect/disconnect 抽象
8. **FakeExternalEventClient**: StdRng 固定シードイベント生成（13種の DarviumEventKind）

## テスト結果

- 全1138テスト通過（既存1106 + 新規32）
- 観測テスト T7: 偽陽性率 0%、偽陰性率 0%、完全性 100% 確認済み
- clippy 新規警告なし
