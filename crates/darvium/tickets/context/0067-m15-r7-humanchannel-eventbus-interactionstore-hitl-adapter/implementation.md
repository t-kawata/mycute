# 変更したファイル一覧と実装内容の概要

## 変更ファイル

### src/human_channel.rs
- `HumanChannelConfig` 構造体を新設: `event_bus` / `interaction_store` の Option フィールド
- `EventBusHumanChannel` 構造体 + メソッドを新設:
  - `notify()` → `EventBus::publish(OneWay, HitlEvent::NotificationRequested)`
  - `communicate()` → `EventBus::open(TwoWay, HitlEvent::InteractionRequested)` + MetadataStore 保存 + mpsc pending マップ
  - `reconnect()` → `MetadataStore::reconnect_interaction()` + `EventBus::reconnect()`
  - `resolve_interaction()` → 外部解決用公開メソッド
- `FakeHumanChannel` に `eventbus_delegate` フィールド追加 + `with_config()` コンストラクタ + 委譲ロジック
- テスト 12 件追加（T1-T11 + OTS-1）
- 既存テストは全件変更なしで PASS

### src/lib.rs
- `EventBusHumanChannel`, `HumanChannelConfig` を公開エクスポートに追加

### src/store/metadata_store.rs
- `MetadataStore` トレイト: 変更なし（supertrait bounds 追加なし）
- `InMemoryMetadataStore`: `RefCell` → `Mutex` に変更（EventBusHumanChannel の Arc<dyn MetadataStore + Send + Sync> 要件のため）

### src/store/json_metadata_store.rs
- `JsonMetadataStore`: `RefCell` → `Mutex` に変更（同上）

### src/store/coordinator.rs
- `FailingMetadataStore`: `RefCell<u32>` → `Mutex<u32>` に変更（同上）

## RFC 二無矛盾チェック
- RFC §12B.3-12B.4: HumanChannel trait シグネチャ不変 (MUST NOT)。EventBusHumanChannel の adapter 変換ルールを充足。
- RFC §12C.6: VirtualClock 契約（publish/open/resolve/reconnect で +1）を遵守。
