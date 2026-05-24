---
ticket_id: 68
title: M1.5-R8: EventChannel トレイト + StdinoutEventChannel canonical JSON Lines プロトコル
slug: m15-r8-eventchannel-stdinouteventchannel-canonical-json-lines
status: reviewed
created_at: 2026-05-24
updated_at: 2026-05-24
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0068-m15-r8-eventchannel-stdinouteventchannel-canonical-json-lines/plan.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0068-m15-r8-eventchannel-stdinouteventchannel-canonical-json-lines/implementation.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0068-m15-r8-eventchannel-stdinouteventchannel-canonical-json-lines/observation-20260524-130605.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0068-m15-r8-eventchannel-stdinouteventchannel-canonical-json-lines/review.md
---

# M1.5-R8: EventChannel トレイト + StdinoutEventChannel canonical JSON Lines プロトコル

## Summary

Darvium Event Architecture の外部連携基盤として、`EventChannel` トレイト（DarviumEvent の外部送受信抽象）および標準入出力を介した canonical JSON Lines プロトコル実装 `StdinoutEventChannel` を追加する。既存の `StdinoutChannel`（HITL 専用旧 JSON Lines プロトコル）は互換モードラッパーとして保持する。

## Background

M1.5-R4/R5/R7 で整備された Event Architecture（DarviumEvent/DarviumEventBus/HumanChannel adapter）に対し、**外部プロセスとのイベント送受信**という最後のピースを追加する。

- 既存 `StdinoutChannel` は HITL 専用の旧 JSON Lines プロトコル（type: notify/communicate/reconnect + HumanRequest/HumanOutcome）で動作しており、Event Architecture との直接通信ができない
- 将来の外部ツール（LLM ゲートウェイ・モニタリングエージェント等）との連携には、DarviumEvent を直接送受信できる汎用チャネルが必要
- 全イベント種別（13種類）を扱う canonical JSON Lines プロトコルを新規定義し、既存の旧プロトコルは CompatMode として保持する（MUST NOT remove）

## Scope

1. **`EventChannel` トレイト定義**: `send(DarviumEvent) -> Result<()>`, `receive() -> Result<Option<DarviumEvent>>`, `flush() -> Result<()>`
2. **`StdinoutEventChannel` 実装**: 標準入出力を介した canonical JSON Lines プロトコル（RFC §12B.9a の7種のメッセージ: event.publish / interaction.open / interaction.reply / interaction.reconnect / subscribe / ack / error）
3. **`CompatMode` 実装**: 旧 `StdinoutChannel` JSON Lines 形式との互換性（notify → event.publish + NotificationRequested、communicate → interaction.open + InteractionRequested、reconnect → interaction.reconnect の変換マッピング）
4. **`StdinoutChannel` 互換モードラッパー**: 既存 `StdinoutChannel<R, W>` を `EventChannel` トレイト実装として再公開（互換モード有効時のみ旧形式を読み書き）
5. **`WebSocketEventChannel` 型定義のみ**: 構造体定義のみ（実装は将来 M1.76-21 で行う）
6. **エラー型追加**: `DarviumError::EventChannel(String)` variant を error.rs に追加
7. **`src/event_channel.rs` 新規ファイル**: EventChannel 関連の全コードを集約

## Non-scope

- 非同期（tokio/async）版の EventChannel — 本チケットは同期版のみ
- WebSocketEventChannel の実際の送受信ロジック（型定義のみ）
- EventBus 内部実装の変更（FakeEventBus は既存のまま変更しない）
- StdinoutChannel の既存 HumanChannel 実装の削除（互換性のため維持）

## Investigation

### 参照観察レポート

- tickets/context/0067-m15-r7-humanchannel-eventbus-interactionstore-hitl-adapter/observation-20260524-122941.md — HumanChannel adapter の一貫性検証結果。EventBusHumanChannel と同様の adapter パターンを StdinoutChannel の CompatMode 変換に適用可能であることが確認された

### 物理的証拠

1. **既存 `StdinoutChannel<R, W>`** (`src/human_channel.rs:367-381`):
   - 標準入出力ベースの HumanChannel 実装
   - 旧 JSON Lines プロトコル: `{"type":"notify",...}`, `{"type":"communicate",...}`, `{"type":"reconnect",...}`
   - reader: `Arc<Mutex<R>>`, writer: `Mutex<W>` の構造
   - session: `Mutex<()>` で write-read 系列を直列化

2. **`DarviumEvent` canonical envelope** (`src/event.rs:382-403`):
   - event_id, kind, interaction_mode, payload, causality, metadata, transport_meta, visibility, retention, privacy の10フィールド
   - 全フィールド `Serialize + Deserialize`

3. **`DarviumEventKind`** (`src/event.rs:345-372`):
   - 13 variant（System/Search/WorkflowExecution/Training/Knowledge/Conversational/Lifecycle/Gc/Repair/Reciprocity/Fusion/Hitl/Extension）

4. **`DarviumEventBus` トレイト** (`src/event.rs:527`):
   - publish, open, resolve, reconnect, subscribe, replay, current_clock, quarantine_failed_events の8メソッド

5. **`StdinoutResponse` パース中間型** (`src/human_channel.rs:744-748`):
   - `interaction_id: uuid::Uuid`, `outcome: Option<HumanOutcome>` のみ — 旧形式限定

6. **`write_json_line` 内部ヘルパー** (`src/human_channel.rs:730-741`):
   - type + interaction_id + request で JSON を構築 → serde_json::to_string → writeln!

7. **RFC §12B.9a canonical JSON Lines プロトコル** (Darvium-RFC-0001-Unified-v2.3-final.md:2203-2242):
   - 7 種類のメッセージ: event.publish, interaction.open, interaction.reply, interaction.reconnect, subscribe, ack, error
   - 旧→新マッピング表: notify → event.publish + HitlEvent::NotificationRequested, communicate → interaction.open + HitlEvent::InteractionRequested, reconnect → interaction.reconnect

8. **RFC §12D.1 EventChannel trait** (Darvium-RFC-0001-Unified-v2.3-final.md:2679-2693):
   - RFC では async trait（connect/disconnect/send/subscribe）だが、本チケットでは実装スコープを同期版（send/receive/flush）とする
   - receive は受信した JSON Lines を行単位でパースし `DarviumEvent` を返す。flush はバッファの出力をフラッシュする

9. **RFC §12D.2 StdinoutEventChannel** (Darvium-RFC-0001-Unified-v2.3-final.md:2701-2716):
   - `CompatMode::Enabled / Disabled` 列挙型
   - 互換モード有効時は §12B.9a の変換マッピングを適用
   - パースエラー時は `{"type":"error","code":"PARSE_ERROR","message":"..."}` を出力

10. **`DarviumError` に `EventChannel` variant 未定義** — 新規追加が必要

## Test Plan

### テストファイル: `src/event_channel.rs` 内の `#[cfg(test)] mod tests`

#### T1: EventChannel トレイト型境界テスト
- **T1-1**: `EventChannel` トレイトが `Send + Sync` を実装していることのコンパイル時確認
- **T1-2**: `Box<dyn EventChannel>` がオブジェクトとして使用可能であることの確認（トレイトの全メソッドが `&self` であること）

#### T2: StdinoutEventChannel canonical モードの send/receive ラウンドトリップ
- **T2-1**: `send()` → 内部バッファ → `receive()` で同一 `DarviumEvent` が戻ることを確認（全フィールド一致）
- **T2-2**: 13 種の `DarviumEventKind` すべてでラウンドトリップが成功すること
- **T2-3**: 1000 イベントの一括 send → 逐次 receive で全イベントが消失なく取得できること（ラウンドトリップ成功率 100%）
- **T2-4**: `flush()` の呼び出し後に出力バッファがフラッシュされること

#### T3: canonical JSON Lines プロトコルのメッセージ形式
- **T3-1**: `send(event.publish)` が `{"type":"event.publish","event_kind":"...","payload":{...}}` を出力すること
- **T3-2**: `send(interaction.open)` が `{"type":"interaction.open","interaction_id":"...","event_kind":"...","payload":{...}}` を出力すること
- **T3-3**: `send(interaction.reconnect)` が `{"type":"interaction.reconnect","interaction_id":"...","event_kind":"...","payload":{...}}` を出力すること
- **T3-4**: `send(subscribe)` が `{"type":"subscribe","event_kinds":["..."]}` を出力すること

#### T4: compat モード — 旧プロトコル変換
- **T4-1**: `CompatMode::Enabled` 時、旧 `notify` 形式の入力行が `HitlEvent::NotificationRequested` の `DarviumEvent` としてパースされること
- **T4-2**: `CompatMode::Enabled` 時、旧 `communicate` 形式の入力行が `HitlEvent::InteractionRequested + InteractionMode::TwoWay` の `DarviumEvent` としてパースされること
- **T4-3**: `CompatMode::Enabled` 時、旧 `reconnect` 形式の入力行が `interaction.reconnect` の `DarviumEvent` としてパースされること
- **T4-4**: `CompatMode::Enabled` 時の出力（send）が旧形式（type: notify/communicate/reconnect）であること
- **T4-5**: `CompatMode::Disabled` 時、旧プロトコルの入力行がパースエラーになること

#### T5: パースエラー処理
- **T5-1**: 不正 JSON の入力行に対し `receive()` がエラーを返さず `None` を返すこと（プロトコルエラーは error メッセージ出力のみ）
- **T5-2**: 不明な `type` フィールドの入力行に対して `None` が返ること
- **T5-3**: 空行・空白のみの行がスキップされること

#### T6: 互換モード往復変換の情報損失ゼロ
- **T6-1**: 旧形式（notify/communicate/reconnect）→ canonical 変換 → 旧形式 の往復変換で情報が一致すること
- **T6-2**: canonical 形式 → 旧形式変換（compat mode send）→ canonical 形式 の往復変換で情報が一致すること

## 計装方法・観測対象

### 計装方法
- テストコードは `src/event_channel.rs` 内の `#[cfg(test)] mod tests` に実装
- canonical JSON Lines 形式のラウンドトリップ検証用に `println!` + `--nocapture` で出力行を観測
- T2-3 の1000イベント一括検証では固定シード PRNG を使用せず（イベント内容はランダム性に依存しない）、確定値の列挙

### 観測対象
- **ラウンドトリップ一貫性**: T2-3 で全1000イベントの消失率 0% を `assert_eq!` で検証（不変条件）
- **互換モード変換ロス**: T6-1/T6-2 で旧形式↔canonical 間の情報損失ゼロを全フィールド一致で検証
- **パース成功率**: 1000 回の send → receive 操作で 100% の一貫性を確認（カウント一致）

### 較正計画
本チケットに較正すべき定数は存在しない。

## Boy Scout Rule — 翻訳可能性計画

1. **新規 `src/event_channel.rs` の翻訳可能性**:
   - `EventChannel` トレイトの3メソッドは `send()`, `receive()`, `flush()` と動詞句で命名済み（既存設計準拠）
   - `StdinoutEventChannel` の内部処理は責務ごとに関数抽出する: `parse_canonical_line()`, `parse_legacy_line()`, `serialize_canonical()`, `serialize_legacy()`
   - プロトコルメッセージ種別の判定は match 文で網羅的に行い、`if-else` 連鎖を避ける

2. **既存 `StdinoutChannel` の翻訳可能性改善（Boy Scout）**:
   - `src/human_channel.rs:416-459`: communicate 内の reader スレッド起動ブロックが一段落の長い関数になっている。関数抽出により「応答読み取りスレッドを起動」→「応答をパース」→「エラー処理」に分割する
   - `src/human_channel.rs:730-741`: `write_json_line` の msg_type パラメータは旧形式専用である。これを `write_legacy_json_line` に改名し、新 canonical 用の `write_canonical_json_line` と明確に分離する

3. **エラー握りつぶしの禁止**:
   - `reader` スレッド内の `tx.send(...).ok()` パターンは握りつぶしであり、エラーをログ出力するよう修正する
   - ただし本チケットのスコープ外のため、Boy Scout 範囲として touches した行のみ修正する

## Acceptance Criteria

- [ ] `EventChannel` トレイト（send/receive/flush）が定義され、`Send + Sync` + オブジェクト安全であること
- [ ] `StdinoutEventChannel<R, W>` が canonical JSON Lines プロトコルを実装していること
- [ ] `CompatMode::Enabled` で旧 JSON Lines 形式の読み書きが正しく変換されること
- [ ] `CompatMode::Disabled` で旧形式の入力がパースエラーになること
- [ ] `WebSocketEventChannel` の型定義のみが存在すること
- [ ] `DarviumError::EventChannel(String)` variant が追加されていること
- [ ] 既存の `StdinoutChannel` HumanChannel 実装が変更なくコンパイルを通ること
- [ ] 全テストが通過すること（既存テスト + 新規20テスト）
- [ ] ラウンドトリップ一貫性: 1000 イベントの send → receive で消失率 0%
- [ ] 互換モード変換: 旧形式↔canonical の往復変換で情報損失ゼロ
- [ ] 翻訳可能性: 新規コードが散文として読める関数構成であること
- [ ] `cargo clippy -- -D warnings` が通過すること

## Notes

### 成果物

- 計画: context/0068-m15-r8-eventchannel-stdinouteventchannel-canonical-json-lines/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0068-m15-r8-eventchannel-stdinouteventchannel-canonical-json-lines/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0068-m15-r8-eventchannel-stdinouteventchannel-canonical-json-lines/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0068-m15-r8-eventchannel-stdinouteventchannel-canonical-json-lines/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
