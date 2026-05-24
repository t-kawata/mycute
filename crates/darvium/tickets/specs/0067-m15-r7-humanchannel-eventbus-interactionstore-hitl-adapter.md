---
ticket_id: 67
title: M1.5-R7: HumanChannel を EventBus / InteractionStore 上の HITL 特化 adapter へ再構成
slug: m15-r7-humanchannel-eventbus-interactionstore-hitl-adapter
status: reviewed
created_at: 2026-05-24
updated_at: 2026-05-24
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0067-m15-r7-humanchannel-eventbus-interactionstore-hitl-adapter/plan.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0067-m15-r7-humanchannel-eventbus-interactionstore-hitl-adapter/implementation.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0067-m15-r7-humanchannel-eventbus-interactionstore-hitl-adapter/observation-20260524-122941.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0067-m15-r7-humanchannel-eventbus-interactionstore-hitl-adapter/review.md
---

# M1.5-R7: HumanChannel を EventBus / InteractionStore 上の HITL 特化 adapter へ再構成

## Summary

`HumanChannel` トレイトの 3 メソッド（`notify` / `communicate` / `reconnect`）の内部実装を、現在の直接状態管理から `DarviumEventBus` + `MetadataStore`（InteractionStore 相当）経由の HITL 特化 adapter へ再構成する。トレイトインターフェース自体は **一切変更しない（MUST NOT）**。

## Background

v2.3-g Event Architecture の導入により、以下が利用可能になった：
- `DarviumEventBus` トレイト: OneWay publish / TwoWay open / resolve / reconnect
- `DarviumEventKind::Hitl`: HITL 特化 4 variant（NotificationRequested / InteractionRequested / InteractionResolved / ChannelReconnected）
- `MetadataStore` の汎用 Interaction API: store / load / list / resolve / abort / reconnect

現状の `HumanChannel` 実装（`FakeHumanChannel` / `StdinoutChannel`）は独自の内部状態（`HashMap` / `mpsc`）でインタラクションを管理しており、EventBus 経由の一元管理と乖離している。このチケットでは HITL の実行意味論を保ったまま、内部実装を EventBus + MetadataStore 経由に置き換え、アーキテクチャ全体の一貫性を向上させる。

## Scope

### 必須 (MUST)

1. `HumanChannel` トレイトの 3 メソッドシグネチャを一切変更しない
2. `notify` の内部実装を EventBus への `DarviumEventKind::Hitl(HitlEvent::NotificationRequested)` の OneWay publish に置き換える adapter 実装を追加
3. `communicate` の内部実装を EventBus の `open()`（TwoWay） + MetadataStore の `store_interaction()` 状態追跡に置き換える adapter 実装を追加
4. `reconnect` の内部実装を MetadataStore の `reconnect_interaction()` + EventBus の `reconnect()` に置き換える adapter 実装を追加
5. `InteractionHandle` が EventBus の `resolve()` 完了を待機する adapter 機構を追加（`mpsc` ベースの従来方式を維持しつつ、EventBus adapter モードでは EventBus の interaction 解決をポーリング）
6. `HumanChannelConfig` 構造体を新設:
   - `event_bus: Option<Arc<dyn DarviumEventBus>>`
   - `interaction_store: Option<Arc<dyn MetadataStore>>`
   - 両方とも `Option` で後方互換性を確保
7. `FakeHumanChannel` に EventBus adapter モード追加（従来モードとの切り替え可）
8. `EventBusHumanChannel` 構造体を新設 — `HumanChannel` トレイト実装として EventBus 経由動作を提供
9. 既存テストコードの全変更なし通過（後方互換性 MUST）

### 非スコープ (MUST NOT)
- `HumanChannel` トレイトシグネチャの変更
- `StdinoutChannel` の内部実装変更（これは M1.5-R8 で `EventChannel` 化される）
- `DarviumEventBus` / `MetadataStore` トレイトの変更
- `HumanOutcome` / `HumanRequest` / `HitlPayload` 等の型変更

## Investigation

### 既存コード構造

**`HumanChannel` トレイト** (`src/human_channel.rs:26`)
- `fn notify(&self, request: &HumanRequest) -> Result<(), DarviumError>`
- `fn communicate(&self, request: &HumanRequest) -> Result<InteractionHandle, DarviumError>`
- `fn reconnect(&self, interaction_id: Uuid, request: &HumanRequest) -> Result<InteractionHandle, DarviumError>`

**現状の `FakeHumanChannel`** (`src/human_channel.rs:113`)
- `sent_count（AtomicU64）` + `requests_sent（Mutex<Vec<HumanRequest>>）` + `preloaded（Mutex<VecDeque<HumanOutcome>>）` + `interactions（Mutex<HashMap<Uuid, InteractionRecord>>）`
- `notify`: カウンタ + リクエストリスト更新のみ
- `communicate`: プリロードキューから即時解決 + mpsc 送信 + 内部レコード保存
- `reconnect`: 内部レコード検索 → 解決済みならそのまま、未解決なら TimedOut

**`DarviumEventBus` トレイト** (`src/event.rs:527`)
- `fn publish(&self, event: DarviumEvent) -> Result<EventId, DarviumError>`
- `fn open(&self, event: DarviumEvent) -> Result<InteractionId, DarviumError>`
- `fn resolve(&self, interaction_id: &InteractionId, outcome: Value) -> Result<(), DarviumError>`
- `fn reconnect(&self, interaction_id: &InteractionId, new_channel: &str) -> Result<(), DarviumError>`
- `fn subscribe(&self, filter: EventFilter) -> Box<dyn EventSubscription>`
- `fn replay(&self, since_vt: u64, filter: EventFilter) -> Result<Vec<DarviumEvent>, DarviumError>`
- `fn current_clock(&self) -> u64`
- `fn quarantine_failed_events(&self, interaction_id: &InteractionId, reason: &str) -> Result<(), DarviumError>`

**`MetadataStore` トレイト** (`src/store/metadata_store.rs:20`)
- interaction 関連: `store_interaction`, `load_interaction`, `list_interactions`, `resolve_interaction`, `abort_interaction`, `reconnect_interaction`
- HITL wrapper: `store_human_interaction`, `load_human_interaction`, `list_pending_human_interactions`, `resolve_human_interaction`

**`InteractionHandle`** (`src/human_channel.rs:54`)
- `interaction_id: Uuid` + `rx: mpsc::Receiver<Result<HumanOutcome, DarviumError>>`
- `wait(None)` → 無制限ブロック, `wait(Some(dur))` → タイムアウト付き

**`DarviumEventKind::Hitl`** (`src/event.rs:326-335`)
- `NotificationRequested`, `InteractionRequested`, `InteractionResolved`, `ChannelReconnected`

### 設計判断

- **InteractionStore は MetadataStore トレイトに相当**: 仕様書の "InteractionStore" は抽象概念であり、実際の実装は `MetadataStore` トレイトの interaction API 群（`store_interaction` / `resolve_interaction` / `reconnect_interaction`）が担当する。そのため `HumanChannelConfig` のフィールド型は `Arc<dyn MetadataStore>` とする。
- **`DarviumEventKind::InteractionRequested`**: TwoWay の `communicate` は `HitlEvent::InteractionRequested` を kind とし、`InteractionMode::TwoWay` で open する。
- **`InteractionHandle` の adapter**: EventBus 経由では応答が非同期で届く。`InteractionHandle` の `rx` を EventBus の subscribe/replay 経由で解決する adapter を実装する。
- **`FakeHumanChannel` 互換性**: 既存モード（直接状態管理）をデフォルトとし、`HumanChannelConfig` で参照が設定された場合のみ EventBus adapter モードに切り替える。ref: `src/store/metadata_store.rs:353` ("暫定: 時刻のみ更新。M1.5-R7 でチャネル情報の更新を追加。")

## Test Plan

### ユニットテスト（新規: `EventBusHumanChannel`）

- **T1**: `EventBusHumanChannel` が `HumanChannel` トレイト境界を充足する
- **T2**: `notify()` 呼び出し後に EventBus が `DarviumEventKind::Hitl(HitlEvent::NotificationRequested)` のイベントを保有すること
- **T3**: `communicate()` 呼び出し後に EventBus に `HitlEvent::InteractionRequested` × `InteractionMode::TwoWay` が記録されること
- **T4**: `communicate()` 呼び出し後に MetadataStore の `list_pending_human_interactions()` で該当レコードが取得できること
- **T5**: `communicate()` → EventBus の `resolve()` 後に `InteractionHandle::wait()` が解決されること（非同期解決）
- **T6**: `reconnect()` が MetadataStore の `reconnect_interaction()` を呼び出すこと
- **T7**: `reconnect()` が EventBus の `reconnect()` を呼び出すこと
- **T8**: `notify` の HumanRequest ペイロードが EventBus のイベントペイロードとして正しく保存されること
- **T9**: `HumanChannelConfig` が `event_bus`/`interaction_store` 未設定でも動作すること（後方互換性）

### 既存テストの後方互換性検証

- **T10**: `t1_1_fake_channel_implements_trait` — 変更なしで PASS
- **T11**: `t1_2_notify_fire_and_forget` — 変更なしで PASS
- **T12**: `t2_1_basic_communicate` — 変更なしで PASS
- **T13**: `t2_2_all_outcome_variants` — 変更なしで PASS
- **T14**: 全既存テスト（T3〜T9 全 45 テスト）が変更なしで PASS

### 観測テスト

- **OTS-1**: EventBus adapter モードと従来モードの出力一致率検証（`n = 100` ランダム操作系列、固定シード `StdRng::seed_from_u64(12345)`）
  - 両モードで同一入力 → 同一出力を返すこと
  - `n = 100` の notify / communicate / reconnect 混合操作系列
  - 期待値: 一致率 100%（不変条件）
- **OTS-2**: 退行検出率 100% — 既存テストスイート全テストが adapter 変更後も同一結果を返すこと

## 計装方法・観測対象

### 計装方法
- OTS-1 専用テスト関数を `human_channel.rs` の `mod tests` に追加
- `println!` + `--nocapture` で両モードの出力を構造化形式で標準出力に書き出す
- 固定シード `StdRng::seed_from_u64(12345)` を使用

### 観測対象
- 一致率: `matching_count / total_operations`
- 不一致があった場合の詳細: 操作種別、入力、従来モード出力、adapter モード出力

### 較正計画
本チケットは純粋な再構成であり、較正すべき定数は存在しない。

## Boy Scout Rule — 翻訳可能性計画

- 新設する `EventBusHumanChannel` のメソッド名は動詞句を維持（`notify`, `communicate`, `reconnect`）
- `HumanChannelConfig` のフィールド名はドメイン概念を直接表現（`event_bus`, `interaction_store`）
- 内部の EventBus 構築ロジックを補助関数に分割し、一関数一責務を徹底
- 現状の `FakeHumanChannel` 内の `export_interactions()` が冗長なクローンを含むが、本チケットのスコープ外であるため `// TODO(M1.5-R7): ` コメントを残すに留める
- ハードコード値の定数化は既存テストリファクタリングとなるため、本チケットでは不要

## Acceptance Criteria

- [ ] `EventBusHumanChannel` が `HumanChannel` トレイトを実装し、notify/communicate/reconnect が EventBus + MetadataStore 経由で動作する
- [ ] `HumanChannelConfig` に `event_bus` / `interaction_store` の Option フィールドが追加されている
- [ ] 既存テスト（全 45 テスト + OTS-1, OTS-2）が一切の変更なく通過する
- [ ] 翻訳可能性の検証が通っている（関数名が動詞句、変数名がドメイン概念）
- [ ] OTS-1: 両モード間の出力一致率 100%（`n = 100`）
- [ ] OTS-2: 全既存テスト PASS（退行検出率 100%）

## Notes

### 参照観察レポート

- `tickets/context/0066-m15-r6-virtualclock-eventbus-commit-clock/observation-20260524-120930.md` — VirtualClock 再定義の観測結果。本チケットの EventBus 依存に影響しない。
- `tickets/context/0065-m15-r5-darviumeventbus-fakeeventbus/observation-20260524-115247.md` — DarviumEventBus + FakeEventBus 実装の観測結果。publish/open/resolve/reconnect/subscribe/replay の全操作の正常動作を確認。

### 成果物

- 計画: context/0067-m15-r7-humanchannel-eventbus-interactionstore-hitl-adapter/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0067-m15-r7-humanchannel-eventbus-interactionstore-hitl-adapter/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0067-m15-r7-humanchannel-eventbus-interactionstore-hitl-adapter/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0067-m15-r7-humanchannel-eventbus-interactionstore-hitl-adapter/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成）
