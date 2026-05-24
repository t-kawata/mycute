# 実装計画: M1.5-R7 HumanChannel → EventBus/InteractionStore adapter 再構成

## 要件
- HumanChannel トレイト notify/communicate/reconnect のシグネチャ不変
- 内部実装のみ DarviumEventBus + MetadataStore 経由に置き換え
- InteractionHandle は mpsc 方式を維持
- 既存 45 テストの全変更なし通過

## 変更ファイル一覧
| ファイル | 種別 | 内容 |
|----------|------|------|
| src/human_channel.rs | 編集 | EventBusHumanChannel 追加、HumanChannelConfig 追加、FakeHumanChannel adapter モード追加、新規テスト T1-T9 + OTS-1 |
| src/lib.rs | 編集 | EventBusHumanChannel, HumanChannelConfig の pub use 追加 |

## 実装設計

### EventBusHumanChannel
- event_bus: Arc<dyn DarviumEventBus>
- metadata_store: Arc<dyn MetadataStore>
- pending: Mutex<HashMap<String, mpsc::Sender<...>>>

### メソッドマッピング
- notify → publish(OneWay, HitlEvent::NotificationRequested)
- communicate → open(TwoWay, HitlEvent::InteractionRequested) + store_interaction(Pending)
- reconnect → MetadataStore::reconnect_interaction() + EventBus::reconnect()

### 解決機構
- EventBusHumanChannel::resolve_interaction() → EventBus::resolve + mpsc通知
- InteractionHandle.wait() → 従来通り mpsc::Receiver 待機

## 実装手順
1. HumanChannelConfig 追加
2. EventBusHumanChannel 構造体 + HumanChannel 実装 + resolve_interaction 公開メソッド
3. FakeHumanChannel に EventBus adapter モード用 with_config() 追加
4. lib.rs への pub use 追加
5. 新規テスト T1-T9 + OTS-1 追加
6. cargo test 全通過確認
