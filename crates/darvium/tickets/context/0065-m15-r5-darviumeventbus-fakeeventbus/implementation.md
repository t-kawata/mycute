# 実装サマリ: M1.5-R5 DarviumEventBus トレイト + FakeEventBus 実装

## 変更したファイル

### src/error.rs
- DarviumError に EventBus 関連 3 variant 追加: EventBus(String), EventNotFound(String), InteractionNotFound(String)

### src/event.rs
- JsonInteractionPayload 構造体追加（InteractionPayload 実装、serde_json::Value ラッパー）
- InteractionId newtype 追加（From<String>, Into<String>, Display, Hash, Serialize, Deserialize）
- EventFilter 構造体追加（kind_filter, since_vt, until_vt + all() + matches() メソッド）
- EventSubscription トレイト追加（poll() -> Option<DarviumEvent>）
- DarviumEventBus トレイト追加（8同期メソッド: publish / open / resolve / reconnect / subscribe / replay / current_clock / quarantine_failed_events）
- FakeEventBus 構造体 + DarviumEventBus impl 追加（Vec<DarviumEvent> + Arc<Mutex<u64>> + HashMap<InteractionRecord<JsonInteractionPayload>>）
- FakeSubscription 構造体追加（EventSubscription 実装）
- 11 TC のテスト追加（read-after-write, open→resolve, subscribe, replay, clock monotonic, quarantine, trait bound, InteractionId, EventFilter, n=1000 completeness, n=64 concurrent）

### src/lib.rs
- pub use event に新規公開型 5 件追加（DarviumEventBus, EventFilter, EventSubscription, FakeEventBus, InteractionId）

## 実装状態
- 全テスト通過: 631 tests, 0 failed
- 全て同期トレイト（tokio 非依存）
- Arc<Mutex<>> でスレッドセーフ
- RFC §12C.6 VirtualClock Commit Protocol の MUST #1, #5, #6 を FakeEventBus が遵守
