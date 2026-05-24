# 変更したファイル一覧と実装内容の概要

## 変更ファイル

| ファイル | 種別 | 内容 |
|----------|------|------|
| `src/event_channel.rs` | 新規 | EventChannel トレイト + StdinoutEventChannel + WebSocketEventChannel 型定義 + 全テスト (21 tests) |
| `src/error.rs` | 編集 | DarviumError::EventChannel(String) variant 追加 |
| `src/lib.rs` | 編集 | event_channel モジュール追加 + 公開 API エクスポート |
| `src/human_channel.rs` | Boy Scout | write_json_line → write_legacy_json_line 改名 |

## EventChannel トレイト

- `send(DarviumEvent) -> Result<()>`: イベントを JSON Lines としてシリアライズして出力
- `receive() -> Result<Option<DarviumEvent>>`: 入力行をパースして DarviumEvent を返す
- `flush() -> Result<()>`: 出力バッファをフラッシュ
- Send + Sync + オブジェクト安全 (Box<dyn EventChannel>)

## StdinoutEventChannel<R, W>

- canonical JSON Lines プロトコル (7 メッセージ種別) を実装
- CompatMode::Enabled で旧 format ↔ canonical 変換
- 全 13 DarviumEventKind 対応

## テスト結果

- 全 21 新規テスト PASS (T1-T6)
- 既存 670 テスト PASS (回帰なし)
- cargo clippy -- -D warnings PASS
- cargo fmt PASS
