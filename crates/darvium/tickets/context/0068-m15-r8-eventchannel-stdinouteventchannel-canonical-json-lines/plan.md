# M1.5-R8: EventChannel トレイト + StdinoutEventChannel canonical JSON Lines プロトコル — 実装計画

## RFC 既存実装状態検証

### RFC §12D.1 EventChannel Trait
RFC は async trait (connect/disconnect/send/subscribe) を定義。本チケットは同期版 (send/receive/flush, Send + Sync, object-safe) で実装。乖離は既知の設計判断 (チケット spec 記載)。

### RFC §12D.2 StdinoutEventChannel
reader: Arc<Mutex<R>>, writer: Mutex<W>, compat: CompatMode — 完全一致。

### RFC §12D.4 Subscription
新規実装。id: String, kinds: Vec<DarviumEventKind>, channel: Option<String> — 完全一致。

## 要件

- EventChannel トレイト (send/receive/flush, Send + Sync, オブジェクト安全)
- StdinoutEventChannel (canonical JSON Lines プロトコル、7種のメッセージ)
- CompatMode (Enabled: 旧形式変換, Disabled: canonical のみ)
- StdinoutChannel 互換モードラッパー (EventChannel impl)
- WebSocketEventChannel (型定義のみ)
- Subscription 構造体
- DarviumError::EventChannel variant 追加
- 20テスト (T1-T6)

## 変更ファイル一覧

| ファイル | 種別 | 内容 |
|---|---|---|
| src/event_channel.rs | 新規 | EventChannel トレイト, 全実装, 全テスト |
| src/error.rs | 編集 +1行 | DarviumError::EventChannel variant 追加 |
| src/lib.rs | 編集 +2行 | mod event_channel + pub use |
| src/human_channel.rs | Boy Scout | write_json_line → write_legacy_json_line リネーム, EventChannel impl 追加 |

## 実装手順

1. error.rs: EventChannel variant 追加
2. event_channel.rs: 全文作成 (トレイト→CompatMode→Subscription→StdinoutEventChannel→parse/serialize 関数→テスト)
3. lib.rs: モジュール宣言 + re-export
4. human_channel.rs: Boy Scout rename + EventChannel impl
5. 検証: cargo test, cargo clippy -D warnings

## 計装・観測

- テストファイル: event_channel.rs 内の tests mod
- T2-3: n=1000 イベント一括ラウンドトリップ (println! + --nocapture)
- 較正定数: なし
