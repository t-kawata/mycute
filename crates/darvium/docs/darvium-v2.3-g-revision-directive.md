# Darvium RFC-0001 Unified Edition v2.3-g 改訂指示書

## 文書位置づけ

本書は、Darvium RFC-0001 Unified Edition v2.3-f を v2.3-g へ改訂するための**規範的改訂指示書**である。[file:1] 目的は、v2.3-f までに定義された `VirtualClock`、`HumanChannel`、`MetadataStore`、`SearchTrace`、`TrainingRunLog`、`ConversationalEvent`、`ReciprocityEvent` などの既存要素を毀損せずに保持しつつ、それらを上位の統一的な **Darvium Event Architecture** のもとに再編成し、Darvium を「出来事の連続によって時間が進行する仮想社会」として明文化することである。[file:1]

本改訂は strictly additive を基本方針とするが、`HumanChannel` まわりの責務は一般化・再命名・抽象化の対象となる。[file:1] ただし、既存の HITL 実行意味論、`notify / communicate / reconnect`、`InteractionHandle.wait()`、`StoredInteraction` の永続化、`MetadataStore` による再接続、`StdinoutChannel` の JSON Lines プロトコル、`Training Orchestrator` との統合、`HumanDecision` / `HumanOutcome` の意味論は後方互換で保持しなければならない（MUST）。[file:1]

## 改訂背景

v2.3-f では `VirtualClock` は Darvium 内部イベントにより単調増加する内部時間軸として定義され、`VirtualClockState { current: u64, updated_at: SystemTime }`、`advance_virtual_clock`、`mark_virtual_seen`、`last_virtual_seen`、`compute_virtual_freshness`、`compute_temporal_freshness` によって Lifecycle / Trust / GC に接続されている。[file:1] しかし v2.3-f は、「Darvium 内で発生するあらゆる出来事を網羅的に捕捉し、どのような経路で VirtualClock を進めるか」という中心的イベント基盤を明示していない。[file:1]

同時に v2.3-f は、HITL のためにかなり完成度の高い通信・永続化・再接続基盤をすでに持っている。[file:1] `HumanChannel` は `notify`、`communicate`、`reconnect` を備え、`InteractionHandle` は blocking wait と timeout を提供し、`StoredInteraction` は `interaction_id`、`request`、`outcome`、`status`、`created_at`、`updated_at` を保持し、`MetadataStore` は `store_humaninteraction`、`load_humaninteraction`、`list_pending_humaninteractions`、`resolve_humaninteraction` を定義し、`StdinoutChannel` は JSON Lines による transport と crash recovery を実装している。[file:1]

この非対称性が問題である。すなわち、Darvium の「時間」は全体的には定義されているが、その時間を進める出来事の正本がない一方で、HITL だけが interaction-oriented event system を先行して持っている。[file:1] v2.3-g はこの歪みを解消し、HITL を特別扱いされた例外機構ではなく、**Darvium 全体の event / interaction architecture の先行実装**として位置づけ直さなければならない。[file:1]

## 改訂目的

v2.3-g の目的は次の 8 点である。

1. Darvium の時間を「出来事の記憶列」に基づく仮想時間として明文化すること。[file:1]
2. Darvium 内で発生する**全ての出来事**が中央の Event Bus を通ることを規範化すること。[file:1]
3. `VirtualClock` を Event Bus commit によってのみ進む共通時間基盤へ昇格させること。[file:1]
4. 既存の `HumanChannel` を一般化し、双方型イベントの基盤として共通化すること。[file:1]
5. DarviumEvent を extensible な共通 envelope とし、将来の event subtype 追加を可能にすること。[file:1]
6. 一方向型イベントと双方型イベントを明確に分離し、HITL を双方型イベントの代表的サブタイプとして位置づけること。[file:1]
7. 外部世界が Darvium の全イベントを subscribe できる標準経路を、主として標準入出力および WebSocket 上に規範化すること。[file:1]
8. 実装者が v2.3-f のコードベースから v2.3-g に移行できるよう、具体的作業手順とリファクタリング順序を示すこと。[file:1]

## 設計思想

### Darvium 時間の再定義

v2.3-g では、Darvium の `VirtualClock` は単なる内部カウンタではなく、「Darvium が記憶した出来事列の順序番号」であると解釈しなければならない。[file:1] これは v2.3-f の `VirtualClock`、`last_virtual_seen`、`ReciprocityEvent.virtual_clock`、`compute_virtual_freshness` を否定するものではなく、それらに対して一貫した存在論を与えるものである。[file:1]

したがって、VirtualClock は「重要イベントのみ」ではなく、Darvium 内で発生し、システムが観測し、記録しうる**全ての出来事**に応じて進まなければならない（MUST）。[file:1] SearchWorkflow の状態遷移、Workflow 実行開始・終了、Trust 更新、Applicability 判定、Knowledge primitive invocation、TrainingMission 生成、Human interaction request/response、Conversational ingestion、GC 状態遷移、Repair 実行、Fusion birth commit、HELP proposal/offer/decision/execution/success、ReciprocityEvent 記録などはすべて DarviumEvent として記録されなければならない。[file:1]

### Event First 原則

Darvium 内の規範的 state transition は、直接ログテーブルや個別メタデータストアを書き換える前に、まず `DarviumEventBus` に publish されなければならない（MUST）。[file:1] Event Bus は publish / commit を通じて event に `event_id` と `virtual_clock` を付与し、その後に永続化・投影・配信・副作用を駆動する唯一の正本経路でなければならない（MUST）。[file:1]

この原則により、既存の `SearchTrace`、`SearchRunLog`、`TrainingRunLog`、`TrustAuditLog`、`RepairLog`、`ReciprocityEvent`、`StoredInteraction`、`ConversationalEvent` は廃止されるのではなく、DarviumEvent から materialize される domain-specific projection として再解釈される。[file:1]

### Interaction は event の一類型である

一般に event は fire-and-forget の一方向 publish と結び付けて理解されがちであるが、v2.3-g ではその理解を採用しない。[file:1] DarviumEvent は「一方向型イベント」と「双方型イベント」の 2 種の interaction semantics を持ち、双方型イベントは request / response / reconnect / timeout / unresolved session recovery を備える会話的イベントであると定義する。[file:1]

この枠組みのもとで、HITL は DarviumEvent の一つの domain subtype であり、同時に双方型イベントの代表例となる。[file:1] ただし HITL 内にも `notify` のような fire-and-forget 通知があるため、「HITL = 常に双方型」ではなく、「HITL は event domain、OneWay / TwoWay は interaction mode」という直交関係で整理しなければならない（MUST）。[file:1]

## v2.3-g で新設すべき中心概念

### 1. DarviumEvent

`DarviumEvent` は Darvium 世界内で観測・記録される全出来事の canonical envelope である。[file:1] v2.3-g は少なくとも以下のフィールド群を規範化しなければならない。

- `event_id: String` — event の安定識別子、UUID v4 以上を推奨。[file:1]
- `virtual_clock: u64` — Event Bus commit 順序として付与される仮想時間。[file:1]
- `created_at: SystemTime` — Human Time 軸の時刻、UTC MUST の既存規範に従う。[file:1]
- `causality` — 親 event、root event、trace ref、mission / workflow / run への接続情報。[file:1]
- `interaction_mode` — OneWay または TwoWay。[file:1]
- `kind` — Search / Training / Knowledge / Lifecycle / Reciprocity / Conversation / HITL / System / Fusion / Extension 等の subtype。[file:1]
- `payload` — subtype ごとの構造化データ。[file:1]
- `transport_meta` — 外部配信や reply routing に必要な付随情報。[file:1]
- `visibility` / `retention` / `privacy` — 外部 subscribe・監査・PII・sandbox 制御のためのメタデータ。[file:1]

### 2. DarviumEventKind

v2.3-g は event subtype を extensible に設計しなければならない。[file:1] 少なくとも以下を標準種別として規範化すること。

- `SearchEvent`
- `WorkflowExecutionEvent`
- `TrainingEvent`
- `KnowledgeEvent`
- `ConversationalEventEnvelope`
- `LifecycleEvent`
- `GcEvent`
- `RepairEvent`
- `ReciprocityEventEnvelope`
- `FusionEvent`
- `HitlEvent`
- `SystemEvent`
- `ExtensionEvent`

`ExtensionEvent` は、将来の RFC 改訂や実験拡張が RFC 本体の enum を破壊せずに追加できる escape hatch として必須である。[file:1]

### 3. InteractionMode

`InteractionMode` は event の interaction semantics を表し、`OneWay` と `TwoWay` を持つ。[file:1] `kind` と `interaction_mode` は直交であり、任意の domain subtype が将来 OneWay または TwoWay を選択できる設計でなければならない（MUST）。[file:1]

- `OneWay` は publish-only であり、delivery semantics のみを持つ。[file:1]
- `TwoWay` は `interaction_id`、状態遷移、timeout、reconnect、pending session recovery、reply correlation を持つ。[file:1]

### 4. DarviumEventBus

`DarviumEventBus` は VirtualClock の唯一の authority であり、全 DarviumEvent の commit, persistence, fan-out, replay entrypoint を担う中心コンポーネントである。[file:1] いかなる domain も VirtualClock を直接進めてはならず、Event Bus を経由しなければならない（MUST NOT）。[file:1]

Event Bus は少なくとも以下を提供しなければならない。

- one-way event の publish
- two-way interaction の open
- interaction response の resolve / reply
- pending interaction の reconnect
- subscribe / fan-out
- current virtual clock の参照
- replay / scan
- persistence failure 時の quarantine / repair

### 5. InteractionStore

現行 `MetadataStore` の HITL interaction 部分は一般化され、`InteractionStore` もしくは同等の汎用名へ昇格されなければならない。[file:1] 既存 `store_humaninteraction` 等は compatibility layer として残してよいが、本体の規範は `store_interaction`、`load_interaction`、`list_pending_interactions`、`resolve_interaction`、`abort_interaction`、`reconnect_interaction` のような汎用 API に移行するべきである。[file:1]

## HumanChannel の改修方針

### 現状評価

現行の `HumanChannel` はすでに高品質な interaction 抽象である。[file:1] `notify` は一方向 fire-and-forget、`communicate` は `InteractionHandle` を返す双方型 interaction、`reconnect` は pending interaction の再接続、`StdinoutChannel` は JSON Lines transport、`MetadataStore` は永続化と復旧を担う。[file:1]

### v2.3-g での再定義

v2.3-g では `HumanChannel` を廃止する必要はないが、その責務を下位の domain-specific adapter へ縮退させる必要がある。[file:1] すなわち、`HumanChannel` は DarviumEventBus / InteractionStore の上に構築された **HITL-specific transport adapter** として再定義されなければならない。[file:1]

変更方針は次の通り。

1. `HumanRequest` / `HumanOutcome` / `HumanResponse` / `StoredInteraction` は保持する。[file:1]
2. ただし `StoredInteraction` は一般化された `InteractionRecord<HitlPayload>` へマッピングされる。[file:1]
3. `communicate()` は `DarviumEventKind::Hitl(HitlEvent::InteractionRequested)` を `InteractionMode::TwoWay` で Event Bus に送る façade となる。[file:1]
4. `notify()` は `DarviumEventKind::Hitl(HitlEvent::NotificationRequested)` を `InteractionMode::OneWay` で Event Bus に publish する façade となる。[file:1]
5. `reconnect()` は `InteractionStore` の pending interaction から `interaction_id` を復旧し、Event Bus または transport adapter に対して再接続を行う façade となる。[file:1]
6. `InteractionHandle.wait()` は Event Bus / InteractionStore により管理される汎用 interaction state machine の view として実装されるべきである。[file:1]

### 強制すべき互換要件

- 既存の `StdinoutChannel` JSON Lines プロトコルは互換モードで維持しなければならない（MUST）。[file:1]
- `FakeHumanChannel` は `FakeInteractionTransport` の HITL 専用 alias もしくは wrapper として再実装しなければならない（MUST）。[file:1]
- `Training Orchestrator` が `HumanChannel` に依存している部分は、そのままコンパイル可能な adapter 層で保持しなければならない（MUST）。[file:1]

## 双方型イベントの規範要件

双方型イベントは次の要件を必ず満たさなければならない。[file:1]

1. `interaction_id` を持つこと。[file:1]
2. Request event と Response event がそれぞれ独立した DarviumEvent として記録されること。[file:1]
3. Request publish 時と Response resolve 時の双方で VirtualClock が進むこと。[file:1]
4. request payload と state が永続化されること。[file:1]
5. システム停止・再起動後に pending interaction を再取得できること。[file:1]
6. `reconnect` に相当する再送 / 再接続プロトコルを持つこと。[file:1]
7. timeout / unreachable / channel closed / aborted を明示的状態として持つこと。[file:1]
8. domain-specific ad-hoc 実装を禁止し、共通 `InteractionStore` と共通 state machine を用いること。[file:1]

### 状態機械

双方型イベントの canonical state machine は最低限次を含まなければならない。

- `Pending`
- `AwaitingExternal`
- `Resolved`
- `TimedOut`
- `Unreachable`
- `ChannelClosed`
- `Aborted`

HITL について規定済みの `Pending / Resolved / TimedOut / Unreachable / ChannelClosed` の復旧思想は、そのまま全 TwoWay interaction に一般化されなければならない。[file:1]

## 外部 subscribe の規範

v2.3-g は、Darvium の外部が全イベントを subscribe できることを明文化しなければならない。[file:1] 標準 transport は少なくとも次の 2 系統を含むべきである。

- `StdinoutEventChannel` — JSON Lines による標準入出力 transport。[file:1]
- `WebSocketEventChannel` — 双方向 subscribe / publish / reply を扱うリアルタイム transport。[file:1]

### 標準入出力

現行 `StdinoutChannel` は HITL 専用 JSON Lines を定義しているが、v2.3-g ではこれを一般 event stream に拡張する。[file:1] 互換性のため旧 `type=notify|communicate|reconnect` は受理してよいが、canonical envelope としては次を定義すること。

- `type = event.publish`
- `type = interaction.open`
- `type = interaction.reply`
- `type = interaction.reconnect`
- `type = subscribe`
- `type = ack`
- `type = error`

### WebSocket

`WebSocketEventChannel` は少なくとも次を提供すること。

- filter 付き subscribe
- backpressure / sequence ack
- reconnect with last seen virtual clock
- replay from virtual clock N
- interaction reply correlation

## VirtualClock 規範の改訂

v2.3-g では `advance_virtual_clock(clock, delta)` を残してもよいが、それは `DarviumEventBus` の内部実装詳細としてのみ使用されるべきである。[file:1] application code が任意の箇所で `advance_virtual_clock` を直接呼ぶことは禁止されなければならない（MUST NOT）。[file:1]

新しい規範は以下とする。

- Event Bus は commit ごとに VirtualClock を 1 以上単調増加させなければならない（MUST）。[file:1]
- 同一 event に対して重複 commit を行ってはならない（MUST NOT）。[file:1]
- replay は既存 event を再利用し、VirtualClock を再増加させてはならない（MUST NOT）。[file:1]
- domain projection は `event.virtual_clock` を source of truth としなければならない（MUST）。[file:1]
- `last_virtual_seen`、`ReciprocityEvent.virtual_clock`、その他 virtual freshness 依存ロジックは Event Bus 由来の値を使用しなければならない（MUST）。[file:1]

## Search / Training / Knowledge / Lifecycle への接続

### Search

`SearchTrace` と `SearchRunLog` は projection として維持しつつ、その生成は Search engine 直接書き込みではなく Event Bus 由来に切り替えるべきである。[file:1] `SearchState` 遷移ごと、candidate evaluation ごと、final outcome ごとに event を出すことが望ましい（SHOULD）。[file:1]

### Training

`TrainingMission` intake、human review、sandbox execution、feedback ingestion、promotion review、promotion commit はすべて DarviumEvent 化されるべきである。[file:1] `TrainingRunLog` は event projection とし、HITL review request / response も同一タイムライン上の event として並べられなければならない。[file:1]

### Knowledge / Conversation

`ConversationalEvent`、`ConversationalClassificationProposal`、`ConversationalGateDecision`、fragment creation、candidate creation、promotion gate は event envelope の domain payload として扱うべきである。[file:1] v2.3-c で規定された policy-governed deterministic gate は保持しつつ、Event Bus がその観測タイムラインを一元化する。[file:1]

### Lifecycle / Reciprocity

`GcState` 遷移、`RepairLog`、`ReciprocityEvent`、HELP proposal / offer / decision / execution / success はすべて event 化されなければならない。[file:1] とくに `ReciprocityEvent` はすでに `eventid` と `virtualclock` を持つため、DarviumEvent への統合に最も近い既存型であり、v2.3-g では envelope との二重記録を避けて整流化する必要がある。[file:1]

## 実装作業手順

### Phase 0: 設計追補の挿入

1. RFC 目次へ `Darvium Event Architecture` 章を追加する。[file:1]
2. 用語集へ `DarviumEvent`、`InteractionMode`、`DarviumEventBus`、`InteractionStore`、`EventProjection`、`EventChannel`、`TwoWayInteraction` を追加する。[file:1]
3. `VirtualClock` の定義を「内部イベントで進むカウンタ」から「commit 済み出来事列の順序番号」へ説明補強する。[file:1]

### Phase 1: 型の一般化

1. `StoredInteraction` をベースに `InteractionRecord<TPayload>` を新設する。
2. `InteractionStatus` を汎用状態機械へ拡張する。
3. `MetadataStore` に汎用 interaction API を追加する。
4. 既存 `store_humaninteraction` 等は shim として残す。
5. `HumanRequest` / `HumanOutcome` を `HitlPayload` として包めるようにする。

### Phase 2: Event Envelope 導入

1. `DarviumEvent` と `DarviumEventKind` を新設する。
2. `InteractionMode::{OneWay, TwoWay}` を新設する。
3. `DarviumEventBus` trait を追加する。
4. in-memory fake implementation と SQLite-backed implementation を用意する。
5. Event commit 時にのみ VirtualClock が進む構造へ書き換える。

### Phase 3: HumanChannel のリフト

1. `HumanChannel` 実装を Event Bus / InteractionStore 上の adapter へ変更する。
2. `notify()` は one-way publish に変換する。
3. `communicate()` は two-way interaction open に変換する。
4. `reconnect()` は generic interaction reconnect を呼ぶ façade に変換する。
5. `InteractionHandle.wait()` は generic interaction resolver の view に差し替える。

### Phase 4: Transport 共通化

1. `StdinoutChannel` を `StdinoutEventChannel` として一般化する。
2. 旧 HITL JSON Lines を compatibility mode として保持する。
3. canonical event envelope の JSON schema を追加する。
4. `WebSocketEventChannel` を追加する。
5. subscribe / replay / reconnect / ack の仕様を定義する。

### Phase 5: Projection 化

1. `SearchTrace` を EventProjection に置き換える。
2. `SearchRunLog` を EventProjection に置き換える。
3. `TrainingRunLog` を EventProjection に置き換える。
4. `TrustAuditLog`、`RepairLog`、`ReciprocityEvent` も event projection 化する。
5. projection failure と source event failure の責務境界を分離する。

### Phase 6: Domain 統合

1. SearchWorkflow state transitions を全面 event 化する。
2. Workflow execution start / finish / error / retry を event 化する。
3. Training plane review / feedback / promotion を event 化する。
4. Conversational ingestion pipeline を event 化する。
5. GC / repair / reciprocity / fusion / help を event 化する。

### Phase 7: 移行と較正

1. replay test: 既存 v2.3-f run と v2.3-g event projection の整合を比較する。[file:1]
2. property-based test: two-way interaction の crash / reconnect / timeout 不変条件を検証する。[file:1]
3. perturbation test: duplicate delivery, delayed reply, out-of-order transport を注入しても Event Bus source-of-truth が壊れないことを確認する。[file:1]
4. observation metrics: pending interaction age, reconnect success rate, projection lag, event publish latency, replay drift を追加する。[file:1]

## RFC 本文へ追加すべき規範文の例

以下の要旨を v2.3-g 本文へ明記すること。

- Darvium における VirtualClock は commit 済み DarviumEvent 列の順序番号である。[file:1]
- Darvium 内で観測・記録される全出来事は DarviumEventBus を通過しなければならない（MUST）。[file:1]
- いかなる domain subsystem も VirtualClock を直接更新してはならない（MUST NOT）。[file:1]
- DarviumEvent は OneWay / TwoWay の interaction semantics を持つ。[file:1]
- HITL は DarviumEvent の domain subtype であり、OneWay と TwoWay の双方を取りうる。[file:1]
- 全ての TwoWay interaction は crash-safe persistent session recovery を実装しなければならない（MUST）。[file:1]
- HITL のために定義された reconnect / pending listing / resolve protocol は、汎用 TwoWay interaction 基盤として共通化されなければならない（MUST）。[file:1]
- 外部 observer は stdin/stdout または WebSocket により DarviumEvent を subscribe できなければならない（MUST）。[file:1]

## Open Questions として残すべき事項

v2.3-g で規範化しつつも、次の点は Open Question として併記するのが妥当である。

- Event payload を fully typed enum にするか、registry + `serde_json::Value` 拡張をどこまで許すか。[file:1]
- replay channel と live subscription channel を統合するか分離するか。[file:1]
- WebSocket subscribe における backpressure / windowing / auth の詳細。[file:1]
- Event projection を SQLite に集約するか domain 別ストアを併用するか。[file:1]
- cross-process / distributed node 化を将来 Annex へ送る場合の event ordering guarantees。[file:1]

## 結論

v2.3-g の本質は、Darvium を「workflow system」に留めず、「出来事の連続によって時間が進行する仮想社会」として制度化することにある。[file:1] そのためには、既存の `VirtualClock` を Event Bus の commit clock として再定義し、HITL に先行実装されていた `communicate / reconnect / persisted interaction` を一般化して、全 TwoWay interaction の共通基盤に昇格させなければならない。[file:1]

この改訂は既存 v2.3-f の Search / Training / Lifecycle / Conversational / Reciprocity / HELP / Fusion の各系統を破壊するものではなく、それらを同一の時間・同一の監査列・同一の外部観測面に束ねるものである。[file:1] 実装者は、まず HumanChannel を一般化し、次に DarviumEvent envelope と Event Bus を導入し、最後に各 domain を projection 化していく順で移行を進めるべきである。[file:1]
