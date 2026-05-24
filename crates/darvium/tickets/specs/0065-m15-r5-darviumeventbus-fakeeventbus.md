---
ticket_id: 65
title: "M1.5-R5: DarviumEventBus トレイト + FakeEventBus 実装"
slug: m15-r5-darviumeventbus-fakeeventbus
status: reviewed
created_at: 2026-05-24
updated_at: 2026-05-24
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0065-m15-r5-darviumeventbus-fakeeventbus/plan.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0065-m15-r5-darviumeventbus-fakeeventbus/observation-20260524-115247.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0065-m15-r5-darviumeventbus-fakeeventbus/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0065-m15-r5-darviumeventbus-fakeeventbus/review.md
---

# M1.5-R5: DarviumEventBus トレイト + FakeEventBus 実装

## Summary

RFC §12C.5 で規範化された `DarviumEventBus` トレイト（8メソッド）を定義し、そのメモリ内テスト用実装 `FakeEventBus` を実装する。また、`EventFilter` 構造体、`EventSubscription` トレイト、`InteractionId` newtype を併せて定義する。

## Background

v2.3-g Event Architecture において、DarviumEventBus は全状態遷移の canonical 経路である。すべてのドメインイベントは EventBus を通過しなければならず (MUST)、VirtualClock の唯一の authority として機能する。

M1.5-R4 で `DarviumEvent` canonical envelope を始めとする全基盤型の定義が完了した。本チケットではこれらの型を利用して、イベントの publish / subscribe / replay / 管理を行う DarviumEventBus トレイトと、テスト用の FakeEventBus を実装する。

**RFC との関係**: RFC §12C.5 に DarviumEventBus トレイトと InteractionHandle の定義、§12C.6 に VirtualClock Commit Protocol の全不変条件、§12C.10 に FakeEventBus の具象構造が記載されている。本実装はこれらと整合するが、以下の点でチケット仕様（Darvium-Tickets-v2.3.md M1.5-R5）を優先する：
- トレイトメソッドは `DarviumEvent`（完全な canonical envelope）を入力として受け取る（RFC は kind + payload の分割引数だが、チケット仕様は envelope 直接渡し）
- `subscribe()` および `replay()` は `EventFilter` 構造体でフィルタ条件を表現する（RFC は kind スライス + Range<u64> だが、チケット仕様は EventFilter で統一）
- 同期トレイト（async 不使用）— Darvium に tokio 依存がないため。FakeEventBus は完全にメモリ内で動作するため同期で実装可能

## Scope

1. `InteractionId` newtype（`pub struct InteractionId(pub String);`）— Into&lt;String&gt;, From&lt;String&gt;, Display, Debug, Clone, PartialEq, Hash, Serialize, Deserialize
2. `EventFilter` 構造体 — kind_filter: Option&lt;Vec&lt;DarviumEventKind&gt;&gt;, since_vt: Option&lt;u64&gt;, until_vt: Option&lt;u64&gt;
3. `EventSubscription` トレイト（Send + Sync）— `fn poll(&self) -> Option<DarviumEvent>`
4. `DarviumEventBus` トレイト（Send + Sync）— 8メソッド:
   - `publish(&self, event: DarviumEvent) -> Result<EventId, DarviumError>` — OneWay publish、VirtualClock を 1 以上進める (MUST)
   - `open(&self, event: DarviumEvent) -> Result<InteractionId, DarviumError>` — TwoWay open
   - `resolve(&self, interaction_id: &InteractionId, outcome: serde_json::Value) -> Result<(), DarviumError>` — TwoWay resolve
   - `reconnect(&self, interaction_id: &InteractionId, new_channel: &str) -> Result<(), DarviumError>` — チャネル再接続
   - `subscribe(&self, filter: EventFilter) -> Box<dyn EventSubscription>` — フィルタ購読
   - `replay(&self, since_vt: u64, filter: EventFilter) -> Result<Vec<DarviumEvent>, DarviumError>` — 時系列リプレイ（VirtualClock を進めてはならない MUST NOT）
   - `current_clock(&self) -> u64` — 現在の VirtualClock 値
   - `quarantine_failed_events(&self, interaction_id: &InteractionId, reason: &str) -> Result<(), DarviumError>` — 失敗イベント隔離
5. `FakeEventBus` 構造体:
   - `events: Arc<Mutex<Vec<DarviumEvent>>>` — 全イベントの追記専用ストア
   - `clock: Arc<Mutex<u64>>` — 内部イベントカウンタ（=VirtualClock）
   - `interactions: Arc<Mutex<HashMap<String, InteractionRecord<serde_json::Value>>>>` — TwoWay インタラクションストア
   - 公開メソッド: `new()`, `published_events()`, `current_clock()`, `reset()`
   - `DarviumEventBus` トレイト実装:
     - `publish`: イベントに clock 値を割り当て（metadata.clock を現在値に設定）、events に追記、clock を +1 進め、event_id を返す
     - `open`: events に追記 + interactions に `InteractionRecord`（status: Pending）を作成、InteractionId を返す
     - `resolve`: interaction_id に対応するレコードの status を Resolved、outcome を設定
     - `reconnect`: interaction_id に対応するレコードの channel_id 相当フィールドを更新（Fake では metadata 更新で代用）
     - `subscribe`: EventFilter に合致するイベントのみを返す購読ストリームを生成（簡易実装: 現在の events ベクタからフィルタ条件で抽出）
     - `replay`: since_vt 以降で EventFilter に合致するイベントを時系列順で返す
     - `current_clock`: 現在の clock 値を返す
     - `quarantine_failed_events`: 該当 interaction を events から remove（Fake: 実装は削除で代用、本番では隔離テーブルへの移動）
6. `DarviumError` への EventBus 関連 variant 追加:
   - `EventNotFound(String)` — イベント未検出
   - `InteractionNotFound(String)` — インタラクション未検出
   - `EventBus(String)` — その他の EventBus エラー
7. 全型に `#[derive(Debug, Clone, PartialEq)]` + シリアライズに必要なものに `Serialize, Deserialize` を付与
8. `src/lib.rs` の `pub use event::{...}` に新しい公開型（DarviumEventBus, FakeEventBus, EventFilter, EventSubscription, InteractionId）を追加

## Non-scope

- VirtualClock 再定義（別チケット M1.5-R6）
- HumanChannel 再構成（別チケット M1.5-R7）
- EventChannel トレイト・外部チャネル実装（別チケット M1.5-R8）
- EventProjection フレームワーク（別チケット M1.5-R9）
- ドメイン統合 — SearchTrace 等の EventProjection 化（別チケット M1.5-R10）
- 較正候補定数（別チケット M1.5-R11）
- ConcreteEventBus（MetadataStore 連携）の実装 — 本チケットでは FakeEventBus のみ
- InteractionStore トレイトの定義（RFC §12C.7）— 後続チケット

## Investigation

### 物理的証拠

#### 1. 現状の Event Architecture 実装状況

`src/event.rs` (M1.5-R4 完了) には以下の全基盤型が実装済み:
- `EventId` (String 型エイリアス, L14)
- 補助型: `DeliveryMode`, `TransportMeta`, `EventVisibility`, `EventRetention`, `PiiHandlingPolicy`, `EventPrivacy`, `EventSource`, `EventMetadata`, `EventCausality` (L16-124)
- `InteractionMode` (L130-140)
- 13 の subtype enum: `SystemEvent`〜`HitlEvent` (L147-330)
- `DarviumEventKind` 13 variant enum (L339-367)
- `DarviumEvent` canonical envelope 10フィールド構造体 (L376-398)

DarviumEventBus トレイト、FakeEventBus、EventFilter、EventSubscription、InteractionId はすべて未実装。

#### 2. 関連型の既存定義

`src/types.rs`:
- `InteractionRecord<TPayload: InteractionPayload>` (L5166) — interaction_id: String, payload, outcome, status, created_at, updated_at
- `InteractionStatus` 7状態列挙型 (L5230-5244) — Pending / AwaitingExternal / Resolved / TimedOut / Unreachable / ChannelClosed / Aborted
- `InteractionPayload` トレイト (L5156-5159) — Outcome 型の関連付け

`src/error.rs`:
- `DarviumError` 列挙型 — EventBus 関連の variant は未定義。以下の variant を追加する必要あり:
  - `EventNotFound(String)`
  - `InteractionNotFound(String)`
  - `EventBus(String)`

#### 3. RFC §12C.5 定義との対応

RFC §12C.5 (L2523-2568) に DarviumEventBus トレイトおよび InteractionHandle の完全な定義が記載されている。

チケット仕様（Darvium-Tickets-v2.3.md M1.5-R5, L730-755）との差異:
| 項目 | RFC §12C.5 | チケット仕様（優先） |
|------|-----------|-------------------|
| publish 引数 | `(kind: DarviumEventKind, payload: Value)` | `(event: DarviumEvent)` |
| open 引数 | `(kind, payload, timeout)` | `(event: DarviumEvent)` |
| open 戻り値 | `InteractionHandle`（oneshot rx） | `InteractionId` |
| subscribe | `(kinds: &[DarviumEventKind]) -> Subscription` | `(filter: EventFilter) -> Box<dyn EventSubscription>` |
| replay | `(clock_range: Range<u64>)` | `(since_vt: u64, filter: EventFilter)` |
| quarantine | `() -> Vec<DarviumEvent>` | `(interaction_id, reason)` |
| async | `#[async_trait]` + async fn | sync fn（tokio 非依存） |

チケット仕様を優先する。これは Darvium-Tickets-v2.3.md が絶対正本文書であり、かつ同期トレイトの方が現状の依存関係（tokio 未導入）に適合するため。

#### 4. 既存の依存関係とロックパターン

`Cargo.toml` に tokio / async-trait は存在しない。そのため DarviumEventBus トレイトは同期メソッドで定義する。

ロックパターンは `std::sync::{Arc, Mutex}` が既存コード（human_channel.rs, human_review_queue.rs, vector_index.rs）で使用されている。FakeEventBus の内部状態保護もこれに倣う。

#### 5. 型配置の方針

DarviumEventBus / FakeEventBus / EventFilter / EventSubscription / InteractionId は全型が Event Architecture の中核コンポーネントであるため、`src/event.rs` に追加する（R4 で作成したモジュールに追記）。DarviumEventBus の実装はファイル後半、既存テストの前（L400 付近）に挿入する。

#### 6. VirtualClock Commit Protocol 不変条件

RFC §12C.6 (L2570-2586) に以下の MUST/MUST NOT が定義されている。FakeEventBus 実装はこれらを遵守しなければならない：
- MUST #1: EventBus は commit ごとに VirtualClock を 1 以上単調増加
- MUST #2: 同一 event に対して重複 commit 禁止
- MUST #3: replay は VirtualClock を再増加させてはならない
- MUST #4: advance_virtual_clock は EventBus 内部実装のみが呼び出せる
- MUST #5: VirtualClock の初期値は 0
- MUST #6: clock 値は commit の全順序を表現する（部分順序は認めない）

### 参照観察レポート

M1.5-R4（0064）の完了により Event Architecture 基盤型は全て揃った。本チケットはその上に構築する初めての「振る舞い」を持つコンポーネントとなる。直接参照すべき過去の観測結果は存在しない。

## Test Plan

### TC-1: FakeEventBus publish → replay の read-after-write 一貫性
- `publish()` で1件の DarviumEvent を発行後、`replay(since_vt=0, EventFilter::all())` で同一イベントが取得できること
- 発行した event_id が replay 結果に含まれることを確認
- テスト名: `test_fake_eventbus_publish_replay_read_after_write`

### TC-2: FakeEventBus open → resolve で TwoWay インタラクション完了
- `open()` で TwoWay インタラクションを開始後、`resolve()` で outcome を設定し完了すること
- resolve 後に `replay()` でイベントが確認できること
- テスト名: `test_fake_eventbus_open_resolve_two_way`

### TC-3: FakeEventBus subscribe フィルタリング
- `subscribe(EventFilter { kind_filter: Some(vec![DarviumEventKind::System(SystemEvent::ClockAdvanced)]), .. })` で該当 kind のみが購読できること
- 異種 kind を publish してもフィルタに合致しないイベントが購読結果に含まれないこと
- テスト名: `test_fake_eventbus_subscribe_filter`

### TC-4: FakeEventBus replay(since_vt=0) 全件時系列順取得
- n = 50 件のイベントを連続 publish 後、`replay(since_vt=0, EventFilter::all())` で全件が clock 値の昇順で取得できること
- 件数が 50 であること、各イベントの metadata.clock が 0..50 の範囲で単調増加していること
- テスト名: `test_fake_eventbus_replay_all_events_chronological`

### TC-5: FakeEventBus current_clock 単調増加
- publish/open を10回繰り返すごとに `current_clock()` が単調増加していること
- clock 値が決して減少しないこと
- テスト名: `test_fake_eventbus_clock_monotonic`

### TC-6: FakeEventBus quarantine 後除外
- `open()` でインタラクション作成後、`quarantine_failed_events()` を実行し、該当インタラクションが `replay()` 結果から除外されること
- テスト名: `test_fake_eventbus_quarantine_excludes_interaction`

### TC-7: FakeEventBus が DarviumEventBus トレイト境界を充足することのコンパイル時検証
- `fn assert_darvium_event_bus<T: DarviumEventBus>(_t: &T) {}` に `&FakeEventBus` を渡せること
- テスト名: `test_fake_eventbus_trait_bound`

### TC-8: InteractionId newtype の変換
- `InteractionId::from(String)` で生成し、`Into<String>` で元の文字列が復元できること
- `HashMap<InteractionId, ...>` のキーとして使用可能であること（Hash 実装確認）
- JSON シリアライズ/デシリアライズが成功すること
- テスト名: `test_interaction_id_newtype_conversion`

### TC-9: EventFilter の複合条件フィルタリング精度
- kind_filter と since_vt の複合条件が正しく動作すること
- 条件に合致するイベントのみが `replay()` 結果に含まれること
- テスト名: `test_event_filter_combined_conditions`

### TC-10: 計装 — n = 1000 イベント一括発行 + replay 完全性
- 1000件の DarviumEvent を一括 publish 後、`replay(since_vt=0, EventFilter::all())` で全件取得できることを検証
- イベント消失率 0% を確認
- PRNG 固定シード（`StdRng::seed_from_u64(12345)`）を使用
- n = 1000, 各イベントの clock 値の分布を計装出力
- テスト名: `test_eventbus_publish_replay_completeness_n1000`

### TC-11: 計装 — 並行アクセス下での clock 単調増加性
- n = 64 スレッドから concurrent に publish を呼び出し、最終的な clock 値が 64 以上であること、および全イベントの clock 値に重複がないことを確認
- `std::thread::scope` を使用（スコープドスレッド）
- テスト名: `test_eventbus_concurrent_clock_monotonic_n64`

## 計装方法・観測対象

### 計装方法
- 全テストは Rust `#[cfg(test)]` ユニットテスト + `println!` による構造化出力
- PRNG 使用テスト（TC-10）は `StdRng::seed_from_u64(12345)` 固定シードで完全再現性を保証
- 並行アクセステスト（TC-11）は `std::thread::scope` を使用、データ競合が無いことを担保

### 観測対象
- publish → replay のイベント完全性: n = 1000 で消失率 0%（期待値）
- `current_clock()` の単調増加性: シーケンシャルアクセス下で巻き戻りゼロ
- `current_clock()` の単調増加性: 並行 n = 64 スレッドアクセス下でもクロック値が一意（重複ゼロ）
- EventFilter によるフィルタリング精度: 適合率 100%、再現率 100%

### 較正計画
- 本チケットは純粋なトレイト定義 + Fake 実装であり、較正すべき定数は存在しない
- 較正候補定数の導入は M1.5-R11 で行う

## Boy Scout Rule — 翻訳可能性計画

本チケットで編集する `src/event.rs` および `src/error.rs` において、以下の翻訳可能性原則を適用する：

- **関数名は動詞句**: テスト関数は `test_<検証観点>` の命名規則に従う。トレイトメソッドは `publish`, `open`, `resolve`, `reconnect`, `subscribe`, `replay`, `current_clock`, `quarantine_failed_events` とし、「何をするか」が関数名から自明であることを保証する
- **変数名はドメイン概念**: テスト内の一時変数も `bus`（EventBus）、`evt` ではなく `event`、`id` ではなく `interaction_id` と記述
- **一関数一責務**: 各テスト関数は単一の検証観点のみを持つ。TC-10（完全性）と TC-11（並行アクセス）は責務が異なるため別関数とする
- **ハードコード値は名前付き定数**: `let n = 1000;` ではなく `const BULK_PUBLISH_COUNT: usize = 1000;`、`const CONCURRENT_THREADS: usize = 64;` と定義
- **エラーの握りつぶし禁止**: `unwrap()` を使用せず、テスト内では `expect("意味のあるメッセージ")` で理由を明示
- **既存コードの改善**: `src/error.rs` に EventBus 関連の variant を追加する際、既存 error enum のパターン（`#[error("...")]` + フィールド）に統一し、一貫性を高める

## Acceptance Criteria

1. DarviumEventBus トレイトが 8 メソッドを備え、Send + Sync を実装していること
2. FakeEventBus が DarviumEventBus トレイトを実装し、全メソッドが正常動作すること
3. publish 後の replay で read-after-write 一貫性が成立すること
4. open → resolve で TwoWay インタラクションが完了すること
5. subscribe の EventFilter によるフィルタリングが正しく機能すること
6. current_clock が単調増加すること（シーケンシャル・並行アクセス双方）
7. quarantine で該当インタラクションが replay 対象から除外されること
8. InteractionId newtype が String との間で正しく変換可能で、HashMap キーとして使えること
9. n = 1000 イベント一括発行で消失率 0% であること
10. n = 64 スレッド並行アクセス下でクロック値の一意性が保たれること
11. 全テストがコンパイル・通過すること

## Notes

- plan_path: /plan-ticket が plan.md を作成後に frontmatter に更新する
- implementation_path: /start-ticket が implementation.md を作成後に frontmatter に更新する
- review_report_path: /review-ticket が review.md を作成後に frontmatter に更新する
- observation_report_path: /start-ticket が observation-YYYYMMDD-HHmmss.md を作成後に frontmatter に最新パスを更新する

### 成果物

- 計画: context/0065-*/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0065-*/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0065-*/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0065-*/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成）
