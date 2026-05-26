---
ticket_id: 106
title: M1.76-21: 外部イベント購読基盤 — EventSubscriber + WebSocketEventChannel モック実装
slug: m176-21-eventsubscriber-websocketeventchannel
status: reviewed
created_at: 2026-05-26
updated_at: 2026-05-26
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0106-m176-21-eventsubscriber-websocketeventchannel/plan.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0106-m176-21-eventsubscriber-websocketeventchannel/implementation.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0106-m176-21-eventsubscriber-websocketeventchannel/observation-20260526-143420.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0106-m176-21-eventsubscriber-websocketeventchannel/review.md
---
# M1.76-21: 外部イベント購読基盤 — EventSubscriber + WebSocketEventChannel モック実装

## Summary

外部システムからの DarviumEvent 購読・受信を可能にする購読管理基盤（`EventSubscriber` / `SubscriberManager`）と、WebSocket 経由の購読を模倣するモック実装（`FakeWebSocketEventChannel` / `FakeExternalEventClient`）を追加する。

購読したイベントは EventBus の `subscribe()` 経由で内部分配される (MUST)。

## Background

v2.3-g Event Architecture（RFC §12D External Event Subscription）において、EventChannel を介した外部システムからのイベント購読機能が定義されている。既存実装では `EventChannel` トレイトと `StdinoutEventChannel`（標準入出力）のみが実装されており、以下が未実装である：

- 購読の登録・解除・一覧を管理する `SubscriberManager`
- 購読状態とフィルタを保持する `EventSubscriber` 
- WebSocket 相当の双方向通信を模倣する `FakeWebSocketEventChannel`
- 外部システムからのイベント購読を抽象化する `ExternalEventClient` トレイト
- 固定シードで購読イベント系列を生成する `FakeExternalEventClient`

## Scope

### 実装対象

1. **`SubscriberStatus` 列挙型**: `Active`, `Paused`, `Disconnected` の3状態
2. **`EventSubscriber` 構造体**: `subscription_id`, `filter: EventFilter`, `channel: Box<dyn EventChannel>`, `status: SubscriberStatus`, `event_count: u64`
3. **`SubscriberManager`**: 購読の登録・解除・一覧・分配を管理
   - `register(filter, channel) -> SubscriptionId`
   - `unregister(id)` 
   - `list() -> Vec<EventSubscriber>`
   - `distribute(event: &DarviumEvent) -> Result<()>`
4. **`FakeWebSocketEventChannel`**: メモリ内バッファで WebSocket 相当の双方向通信を模倣する `EventChannel` 実装
5. **`ExternalEventClient` トレイト**: 外部システムからのイベント購読・受信を抽象化
   - `fn connect(&self, url) -> Result<Box<dyn EventChannel>>`
   - `fn disconnect(&self, id)`
6. **`FakeExternalEventClient`**: 固定シード PRNG で購読イベント系列を生成するメモリ内モック実装
7. **`SubscriptionId` newtype**: 購読識別子（UUIDv4 文字列ラッパー）
8. **イベントチャネルモジュールの再編成**: `event_channel.rs` に上記全構造体を追加

### 非対象

- 実際の WebSocket 接続（TCP/TLS）の実装
- 認証・認可機構
- イベント永続化・リプレイとの統合（別チケット M1.76-22）
- パフォーマンス最適化（NIO、スレッドプール等）
- 障害回復（再接続ロジックの完全実装）

## Investigation

### 既存コード構造

#### `src/event_channel.rs`（event_channel.rs の全体構造）

- **`EventChannel` トレイト** (L56-68): `send`, `receive`, `flush` の3メソッド。`Send + Sync` 要求。
- **`CompatMode` 列挙型** (L22-27): `Enabled` / `Disabled`。
- **`Subscription` 構造体** (L35-42): `id: String`, `kinds: Vec<DarviumEventKind>`, `channel: Option<String>` — 既存だが購読管理とは別用途（プロトコルレベルの購読要求表現として使用）。
- **`StdinoutEventChannel<R, W>`** (L78-172): 標準入出力経由の EventChannel 実装。完全実装済み。
- **`WebSocketEventChannel`** (L182-187): **スタブのみ。** `url: String` と `subscription: Option<Subscription>` の2フィールドを持つ構造体として定義されているが、`EventChannel` トレイトは実装していない。
- **シリアライズ/デシリアライズ関数**: canonical JSON Lines ↔ DarviumEvent 変換の全実装が完了。テストも完備（T2〜T6、48テスト）。

#### `src/event.rs` の関連既存型

- **`DarviumEventBus` トレイト** (L901-939): `subscribe(EventFilter) -> Box<dyn EventSubscription>` で購読機能を提供。
- **`FakeEventBus`** (L949-1199): subscribe 実装済み（`FakeSubscription` を返す）。
- **`EventSubscription` トレイト** (L872-876): `poll() -> Option<DarviumEvent>`。
- **`EventFilter`** (L826-865): `kind_filter`, `since_vt`, `until_vt` と `matches()` メソッド。
- **`InteractionId`** (L800-819): UUIDv4 文字列ラッパーの newtype。

#### `src/lib.rs` の公開API

- `event_channel` モジュールからは `CompatMode`, `EventChannel`, `StdinoutEventChannel`, `Subscription`, `WebSocketEventChannel` が re-export されている（L102-104）。

#### 未発見の型

- `EventSubscriber`（未実装）
- `SubscriberManager`（未実装）
- `SubscriberStatus`（未実装）
- `ExternalEventClient` トレイト（未実装）
- `FakeExternalEventClient`（未実装）
- `FakeWebSocketEventChannel`（未実装）

#### `src/constants.rs` の関連定数

- `EVENTBUS_SUBSCRIPTION_MAX_KINDS: u32 = 32` — 購読対象種別の最大数。本チケットでは購読者数上限の定数も追加。

#### `src/error.rs`

- `DarviumError::EventChannel(String)` — 既存。イベントチャネル関連エラーとして使用可能。

### WebSocketEventChannel の現状

現在の `WebSocketEventChannel` 定義（event_channel.rs:182-187）:
```rust
pub struct WebSocketEventChannel {
    pub url: String,
    pub subscription: Option<Subscription>,
}
```

`EventChannel` トレイトを実装しておらず、send/receive/flush が呼び出せない状態。

## Test Plan

### T1: EventSubscriber 基本操作（正常系）

| ID | テスト内容 | 検証項目 |
|----|-----------|---------|
| T1-1 | `EventSubscriber` 構造体の全フィールド設定・取得 | id/filter/channel/status/event_count が設定通り |
| T1-2 | `SubscriberStatus` 全 variant のパターンマッチ網羅 | 全 variant が match 可能 |
| T1-3 | `SubscriptionId` newtype の UUIDv4 互換性 | 文字列変換、Clone、Debug、PartialEq |
| T1-4 | `SubscriptionId` from String / Display | ラウンドトリップ整合性 |

### T2: SubscriberManager 購読管理

| ID | テスト内容 | 検証項目 |
|----|-----------|---------|
| T2-1 | 購読登録 → 一覧に含まれる | `register()` 後に `list()` が当該 subscriber を含む |
| T2-2 | 購読解除 → 一覧から削除される | `unregister()` 後に `list()` が当該 subscriber を含まない |
| T2-3 | 複数購読登録・一覧サイズ確認 | n 件登録後、list 長が n |
| T2-4 | 存在しない ID の unregister が Err を返す | 未登録 ID でエラー |
| T2-5 | 重複登録の動作確認 | 同じチャネルの重複登録を許容するか |
| T2-6 | 空の manager で distribute が正常終了 | 購読者0件でエラーを返さない |
| T2-7 | 空の manager で list が空 Vec を返す | list().is_empty() |

### T3: SubscriberManager distribute — フィルタリング

| ID | テスト内容 | 検証項目 |
|----|-----------|---------|
| T3-1 | フィルタに合致するイベント → subscriber の event_count 増加 | count が 1 増える |
| T3-2 | フィルタに合致しないイベント → event_count 不変 | count が変化しない |
| T3-3 | 複数購読者全員に合致 → 全員の count 増加 | 全 subscriber の count が増加 |
| T3-4 | 購読解除後に distribute → count 不変 | unregister 後は配送されない |
| T3-5 | 複数フィルタ種別（kind_filter / since_vt / until_vt）の組み合わせ | EventFilter::matches の既存ロジックを活用 |
| T3-6 | EventFilter::all() で購読 → 全イベント受信 | フィルタ制限なしの動作確認 |

### T4: FakeWebSocketEventChannel ラウンドトリップ

| ID | テスト内容 | 検証項目 |
|----|-----------|---------|
| T4-1 | `send` → `receive` ラウンドトリップ | 送出した DarviumEvent が同一内容で受信可能 |
| T4-2 | 送信順序の保存 | FIFO 順序が保たれること |
| T4-3 | 空チャネルで receive → None | 未送信状態での受信 |
| T4-4 | flush 後の状態確認 | flush がエラーを返さない |
| T4-5 | Send + Sync のコンパイル時確認 | `assert_send`, `assert_sync` |
| T4-6 | Box\<dyn EventChannel\> としての利用確認 | トレイトオブジェクト化 |

### T5: ExternalEventClient トレイト

| ID | テスト内容 | 検証項目 |
|----|-----------|---------|
| T5-1 | connect → Box<dyn EventChannel> 取得 | 接続成功時にチャネルが返る |
| T5-2 | disconnect 後はチャネルが利用不可 | 切断後に send/receive がエラー |
| T5-3 | 重複 disconnect の安全確認 | 2回目の切断がエラーを返すか無視 |
| T5-4 | 不正 URL → Err | 接続不可能な URL でエラー |

### T6: FakeExternalEventClient — 固定シードイベント生成

| ID | テスト内容 | 検証項目 |
|----|-----------|---------|
| T6-1 | connect 後にイベントの受信が可能 | receive で Some が返る |
| T6-2 | 同一シードで同一系列のイベントが生成される | 決定論的再現性 |
| T6-3 | 異なるシードで異なる系列 | PRNG シード分離 |
| T6-4 | disconnect 後に receive → None | 切断後の動作 |
| T6-5 | 生成される DarviumEventKind が多様である | 複数種別が含まれる |

### T7: 統合テスト — ExternalEventClient → EventBus

| ID | テスト内容 | 検証項目 |
|----|-----------|---------|
| T7-1 | FakeExternalEventClient のイベントを EventBus publish → SubscriberManager distribute 経由で購読者に届く | End-to-End 配送 |
| T7-2 | 購読フィルタで特定の event_kind のみ受信 | フィルタ精度（偽陽性0%、偽陰性0%） |
| T7-3 | n_sub 購読者、n_event イベントでの完全性検証 | 各購読者の受信完全性（100%） |

### 計装方法

- 全テストで `StdRng::seed_from_u64(12345)` を使用し完全再現性を保証
- T7 統合テストで `println!` + `--nocapture` により以下の観測データを構造化テキストで出力：
  - 購読者数 n_sub、イベント発行数 n_event
  - 各購読者の受信数、フィルタ合致数、ミス率
  - 偽陽性率（0% を確認）・偽陰性率（0% を確認）

### 観測対象

- 購読者数 n_sub ∈ [1, 100]、イベント発行数 n_event = 1000 の条件で、各購読者の受信完全性（フィルタ条件に合致する全イベントの 100% 受信）を検証する
- 購読フィルタの精度: 偽陽性率 0%、偽陰性率 0%

## Boy Scout Rule — 翻訳可能性計画

### WebSocketEventChannel の完全実装（スタブからの昇格）

現状 `WebSocketEventChannel` は `EventChannel` トレイトを実装しておらず、send/receive/flush が使えない不完全な状態である。本チケットで `FakeWebSocketEventChannel`（メモリ内モック）を実装する際に、`WebSocketEventChannel` 自体のスタブは将来の本実装に備えて残すが、少なくともトレイト実装の型として完成させるのではなく、モック版を完全なトレイト実装として提供する。

### 関数名の翻訳可能性

新規追加する関数は全て動詞句とする:
- `SubscriberManager::register` / `unregister` / `list` / `distribute`

### 責務分割

- `SubscriberManager` は購読管理とイベント分配のみに責務を限定
- `FakeWebSocketEventChannel` は `EventChannel` トレイト実装のみ
- `ExternalEventClient` トレイトは接続管理のみ

### 定数化

- デフォルトのバッファ容量（FakeWebSocketEventChannel の内部キューサイズ）は `constants.rs` に定数として定義する
- 購読者最大数のデフォルトも定数化する

## Acceptance Criteria

- [ ] EventSubscriber 基本構造体の全フィールド設定・取得が可能
- [ ] SubscriberManager で購読登録・解除・一覧・分配の全操作が一貫している
- [ ] 複数の購読者が異なる EventFilter で特定の event_kind のみを受信できる
- [ ] FakeWebSocketEventChannel の send → receive ラウンドトリップが成立する
- [ ] 購読解除後のイベント配送が行われない
- [ ] FakeExternalEventClient から受信したイベントが EventBus の publish() を経由して全購読者へ分配される
- [ ] 既存テストが全て通過している（regression ゼロ）
- [ ] 翻訳可能性の検証が通っている

## Notes

<!--
注: このコメントは人間向けの説明である。AI は以下の手順に従うこと。

- plan_path: /plan-ticket が plan.md を作成後に frontmatter に更新する
- implementation_path: /start-ticket が implementation.md を作成後に frontmatter に更新する
- review_report_path: /review-ticket が review.md を作成後に frontmatter に更新する
- observation_report_path: /start-ticket が observation-YYYYMMDD-HHmmss.md を作成後に frontmatter に最新パスを更新する

各コマンドのワークフロー手順が frontmatter 更新の正しい手順である。
-->

### 成果物

- 計画: context/0106-m176-21-eventsubscriber-websocketeventchannel/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0106-m176-21-eventsubscriber-websocketeventchannel/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0106-m176-21-eventsubscriber-websocketeventchannel/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0106-m176-21-eventsubscriber-websocketeventchannel/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
