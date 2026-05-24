---
ticket_id: 64
title: "M1.5-R4: DarviumEvent canonical envelope + DarviumEventKind + InteractionMode 型定義"
slug: m15-r4-darviumevent-canonical-envelope-darviumeventkind-interactionmode
status: reviewed
created_at: 2026-05-24
updated_at: 2026-05-24
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0064-m15-r4-darviumevent-canonical-envelope-darviumeventkind-interactionmode/plan.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0064-m15-r4-darviumevent-canonical-envelope-darviumeventkind-interactionmode/observation-20260524-114025.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0064-m15-r4-darviumevent-canonical-envelope-darviumeventkind-interactionmode/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0064-m15-r4-darviumevent-canonical-envelope-darviumeventkind-interactionmode/review.md
---

# M1.5-R4: DarviumEvent canonical envelope + DarviumEventKind + InteractionMode 型定義

## Summary

RFC §12C で規範化された Darvium Event Architecture の基盤型をすべて定義する。
DarviumEvent（canonical envelope）、DarviumEventKind（13 subtype taxonomy）、InteractionMode（OneWay / TwoWay）に加え、補助型（EventCausality、EventMetadata、TransportMeta、EventVisibility、EventRetention、EventPrivacy、EventSource、DeliveryMode）を一括して実装する。

## Background

v2.3-g では、Darvium の全状態遷移を統一的なイベント基盤（Event Architecture）の上に記録する。その中心となるのが DarviumEvent canonical envelope であり、すべての domain event はこの envelope を通過しなければならない（MUST）。

M1.5-R1〜R3 で InteractionRecord / InteractionStatus / StoredInteraction の汎用化が完了した。本チケットは Event Architecture の根幹をなす型群を RFC §12C の定義に従って実装し、後続チケット（M1.5-R5: DarviumEventBus トレイト、M1.5-R6: VirtualClock 再定義、M1.5-R7: HumanChannel 再構成、M1.5-R8: EventChannel トレイト）の基盤を提供する。

RFC §12C では以下の型が規範として定義されているが、現時点のコード上は未実装である：
- DarviumEvent — canonical envelope（10フィールド）
- DarviumEventKind — 13 variant enum（System〜Extension）
- InteractionMode — OneWay / TwoWay
- EventCausality — 因果関係情報（6フィールド）
- EventMetadata — 経路情報・タイムスタンプ（3フィールド）
- EventSource — 発行元コンポーネント識別子（5 variant）
- TransportMeta — 外部配信制御（3フィールド）
- DeliveryMode — 配送モード（3 variant）
- EventVisibility — 購読可視性制御（3 variant）
- EventRetention — 保持ポリシー（2フィールド）
- EventPrivacy — PII・sandbox 制御（3フィールド）
- EventId — String エイリアス（UUIDv4）
- 各 subtype 固有の列挙型（SystemEvent / SearchEvent / WorkflowExecutionEvent / TrainingEvent / KnowledgeEvent / ConversationalEventEnvelope / LifecycleEvent / GcEvent / RepairEvent / ReciprocityEvent / FusionEvent / HitlEvent）

## Scope

1. `EventId` 型エイリアス（`pub type EventId = String;`）
2. `DarviumEvent` 構造体（RFC §12C.1 完全準拠、10フィールド）
3. `EventCausality` 構造体（parent_event_id / root_event_id / trace_ref / mission_id / workflow_id / run_id）
4. `EventMetadata` 構造体（clock / timestamp / source）
5. `EventSource` 列挙型（System / HumanChannel / Orchestrator / External / Test）
6. `TransportMeta` 構造体（delivery_mode / reply_to / ttl_seconds）
7. `DeliveryMode` 列挙型（AtMostOnce / AtLeastOnce / ExactlyOnce）
8. `EventVisibility` 列挙型（Public / Protected / Internal）
9. `EventRetention` 構造体（persist / ttl_days）
10. `EventPrivacy` 構造体（contains_pii / sandbox_only / pii_handling: PiiHandlingPolicy）
11. `InteractionMode` 列挙型（OneWay / TwoWay: RFC §12C.3）
12. `DarviumEventKind` 列挙型（13 variant: System / Search / WorkflowExecution / Training / Knowledge / Conversational / Lifecycle / Gc / Repair / Reciprocity / Fusion / Hitl / Extension: RFC §12C.2）
13. `SystemEvent` 列挙型（ClockAdvanced / SnapshotTaken / ReplayCompleted / StartupCompleted）— RFC 定義済み
14. `SearchEvent` 列挙型（Started / StepCompleted / Completed / Failed / Aborted）— RFC 未定義（本チケットで新規定義）
15. `WorkflowExecutionEvent` 列挙型（Started / Completed / Failed / Retried）— RFC 定義済み
16. `TrainingEvent` 列挙型（MissionGenerated / HumanReviewRequested / HumanReviewCompleted / SandboxExecutionStarted / SandboxExecutionCompleted / FeedbackIngested / PromotionCandidateCreated / PromotionApproved / PromotionRejected）— RFC 定義済み
17. `KnowledgeEvent` 列挙型（FragmentCreated / CandidateConsolidated / CanonicalPromoted / OriginTraceUpdated）— RFC 定義済み
18. `ConversationalEventEnvelope` 列挙型（UtteranceReceived / Classified / GateDecided / Consolidated / Promoted）— RFC 定義済み
19. `LifecycleEvent` 列挙型（NodeCreated / NodeActivated / NodeDeactivated / NodeArchived）— RFC 未定義（本チケットで新規定義）
20. `GcEvent` 列挙型（SoftDeleted / HardDeleteCandidate / Tombstoned）— RFC 定義済み
21. `RepairEvent` 列挙型（InconsistencyDetected / RetryAttempted / TombstoneApplied / RepairCompleted）— RFC 定義済み
22. `ReciprocityEvent` 列挙型（HelpOffered / HelpAccepted / HelpRejected / HelpExecuted / HelpSucceeded / HelpAbandoned / HarmfulMismatch / ReturnedFavor）— RFC §15.10.6 ReciprocityEventKind の variant を流用
23. `FusionEvent` 列挙型（Paired / FusionCompleted / BirthCommitInitiated / BirthCommitCompleted / FusionFailed）— RFC 未定義（本チケットで新規定義）
24. `HitlEvent` 列挙型（NotificationRequested / InteractionRequested / InteractionResolved / ChannelReconnected）— RFC 定義済み
25. 全型に `#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]` を付与

## Non-scope

- DarviumEventBus トレイトの定義（別チケット M1.5-R5）
- FakeEventBus の実装（別チケット M1.5-R5）
- VirtualClock 再定義（別チケット M1.5-R6）
- HumanChannel 再構成（別チケット M1.5-R7）
- EventChannel トレイト・外部チャネル実装（別チケット M1.5-R8）
- EventProjection フレームワーク（別チケット M1.5-R9）
- ドメイン統合 — SearchTrace 等の EventProjection 化（別チケット M1.5-R10）
- 既存の `struct ReciprocityEvent`（§15.10.6）とは別定義。本チケットで定義する `ReciprocityEvent` は DarviumEventKind の variant 用 enum であり、既存構造体との統合は M1.76 系列で行う
- `PiiHandlingPolicy` の共通型としての再利用設計（本チケットでは EventPrivacy のフィールド型として最小定義、後続チケットで共通型への昇格を検討）

## Investigation

### 物理的証拠

#### 1. 現状の型定義

`src/types.rs` には以下の関連型が既に定義されている：
- `InteractionRecord<TPayload>`（L5166）
- `InteractionStatus` 7状態列挙型（L5230）
- `HitlPayload` / `StoredInteraction`（L5183-5194）
- `HumanRequest` / `HumanOutcome` / `HumanResponse` / `HumanDecision` / `HumanReviewQueuePolicy`

DarviumEvent / DarviumEventKind / InteractionMode および関連の補助型は一切定義されていない。

#### 2. RFC §12C 定義との対応

RFC §12C.1-12C.3 に以下の完全な型定義が記載されている（`Darvium-RFC-0001-Unified-v2.3-final.md` L2314-2477）：

- **DarviumEvent**（§12C.1, L2315-2326）: event_id / kind / interaction_mode / payload / causality: EventCausality / metadata: EventMetadata / transport_meta: Option\<TransportMeta\> / visibility: EventVisibility / retention: EventRetention / privacy: EventPrivacy
- **EventCausality**（L2328-2335）: parent_event_id / root_event_id / trace_ref / mission_id / workflow_id / run_id — すべて Option
- **EventMetadata**（L2337-2341）: clock: u64 / timestamp: SystemTime / source: EventSource
- **EventSource**（L2343-2349）: System / HumanChannel / Orchestrator / External{channel_id} / Test
- **TransportMeta**（L2351-2355）: delivery_mode / reply_to / ttl_seconds
- **DeliveryMode**（L2357-2361）: AtMostOnce / AtLeastOnce / ExactlyOnce
- **EventVisibility**（L2363-2367）: Public / Protected / Internal
- **EventRetention**（L2369-2372）: persist: bool / ttl_days: Option\<u64\>
- **EventPrivacy**（L2374-2378）: contains_pii / sandbox_only / pii_handling: PiiHandlingPolicy
- **DarviumEventKind**（§12C.2, L2388-2402）: 13 variant（System〜Extension）
- **InteractionMode**（§12C.3, L2473-2477）: OneWay / TwoWay

#### 3. RFC に未定義の subtype

以下の subtype は RFC §12C.2 の enum 定義で名前のみ登場し、具体的な enum variant が定義されていない：
- SearchEvent — 最小限の variant を新規定義（Started / StepCompleted / Completed / Failed / Aborted）
- LifecycleEvent — 最小限の variant を新規定義（NodeCreated / NodeActivated / NodeDeactivated / NodeArchived）
- FusionEvent — 最小限の variant を新規定義（Paired / FusionCompleted / BirthCommitInitiated / BirthCommitCompleted / FusionFailed）
- ReciprocityEvent — RFC §15.10.6（L3691-3700）に ReciprocityEventKind（8 variant）が定義済み。これを DarviumEventKind の variant として流用

#### 4. 型配置の方針

Event Architecture の全型は `src/event.rs` に新規モジュールとして配置する（RFC §12C の実装を集約する独立モジュール）。`src/types.rs` に追加するとファイル肥大化とモジュール境界の混濁を招くため避ける。

`src/lib.rs` で `pub mod event;` として公開する。

#### 5. PiiHandlingPolicy の参照

`PiiHandlingPolicy` は RFC §16B.1 で定義され、Darvium-v2.3-final-table-and-struct-definition-spec.md（L1113）に列挙型として記載されているが、現時点の `src/types.rs` には未定義。本チケットで `EventPrivacy` が参照するため、`src/event.rs` 内に併せて定義する（後続チケットで共通定義への移行を検討）。

### 参照観察レポート

過去の観察レポートは M-2 / M-1.5 / M-1 / M-0.5 系列のものであり、Event Architecture（v2.3-g）は今回が初めての実験系列となる。直接参照すべき観測結果は存在しない。

## Test Plan

### TC-1: 全13 variant の DarviumEventKind トレイト実装確認
- コンパイル時確認として、全13 variant のインスタンスを1つずつ作成
- `format!("{:?}", ..)` がパニックしないこと、`clone()` が等価であること、`PartialEq` が成立すること、JSON シリアライズ/デシリアライズが成功することを検証
- テスト名: `test_darvium_event_kind_trait_impl`

### TC-2: DarviumEvent 全フィールド設定・アクセス
- DarviumEvent の全10フィールドを設定したインスタンスを作成し、各フィールドに `.` でアクセス可能であること、期待値と一致することを検証
- テスト名: `test_darvium_event_full_fields`

### TC-3: InteractionMode パターンマッチ網羅性
- `InteractionMode::OneWay` と `InteractionMode::TwoWay` のパターンマッチがコンパイル可能であり、両 variant を網羅していることを確認（`match` に `_` 以外で全列挙）
- テスト名: `test_interaction_mode_exhaustive_match`

### TC-4: DarviumEvent 完全 JSON ラウンドトリップ（n = 1000）
- 全13種の DarviumEventKind をランダムに選び、1000個の DarviumEvent インスタンスを生成
- `serde_json::to_string` → `serde_json::from_str` のラウンドトリップが全フィールドで一致することを確認
- PRNG 固定シード（`StdRng::seed_from_u64(12345)`）を使用
- テスト名: `test_darvium_event_json_roundtrip_n1000`

### TC-5: 補助型のシリアライズ確認
- 各補助型（EventVisibility / EventRetention / EventPrivacy / TransportMeta / EventCausality / EventMetadata / EventSource / DeliveryMode）の representative インスタンスについて JSON ラウンドトリップが成立することを確認
- テスト名: `test_auxiliary_types_serialization`

### TC-6: EventId 型エイリアスの UUIDv4 互換性
- `EventId` に uuid::Uuid の文字列表現を代入可能であること、及び文字列としての操作に問題がないことを確認
- テスト名: `test_event_id_uuid_compatibility`

### TC-7: EventSource の網羅的パターンマッチ
- `EventSource` の全5 variant を `_` 以外で網羅するパターンマッチがコンパイル可能であることを確認
- テスト名: `test_event_source_exhaustive_match`

### TC-8: 計装 — 全型のフィールド一覧出力
- DarviumEvent の全フィールド名と型、DarviumEventKind の全 variant 名と inner type を `println!` で構造化出力（JSON Lines 形式）
- RFC §12C の定義と人手照合可能な形を提供
- テスト名: `test_type_structure_instrumentation`

## 計装方法・観測対象

### 計装方法
- 全テストは Rust `#[cfg(test)]` ユニットテスト + `println!` による構造化出力
- PRNG 使用テストは `StdRng::seed_from_u64(12345)` 固定シードで完全再現性を保証
- `serde_json::to_string_pretty` で JSON表現を標準出力に書き出し、RFC § 定義と人手照合可能にする

### 観測対象
- シリアライズラウンドトリップ成功率（期待値: 100%）
- JSON 表現の構造的一貫性（必須フィールドの欠落ゼロ）
- サンプルサイズ: n = 1000（シリアライズ検証として統計的に十分）

### 較正計画
- 本チケットは純粋な型定義であり、較正すべき定数は存在しない
- 較正候補定数の導入は M1.5-R11 で行う

## Boy Scout Rule — 翻訳可能性計画

本チケットで新規作成する `src/event.rs` において、以下の翻訳可能性原則を適用する：

- **関数名は動詞句**: テスト関数は `test_<検証観点>` の命名規則に従う
- **変数名はドメイン概念**: テスト内の一時変数も `evt` ではなく `event`、`k` ではなく `kind` と記述
- **一関数一責務**: 各テスト関数は単一の検証観点のみを持つ。TC-4（ラウンドトリップ）と TC-8（計装出力）は責務が異なるため別関数とする
- **ハードコード値は名前付き定数**: `let n = 1000;` ではなく `const ROUNDTRIP_SAMPLE_SIZE: usize = 1000;` と定義
- **エラーの握りつぶし禁止**: `unwrap()` を使用せず、テスト内では `expect("意味のあるメッセージ")` で理由を明示

## Acceptance Criteria

1. 全型に `#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]` が付与され、コンパイルが通ること
2. DarviumEvent の全10フィールドにアクセス可能であること
3. DarviumEventKind の全13 variant がパターンマッチ可能であること
4. InteractionMode の2 variant（OneWay / TwoWay）がパターンマッチで網羅可能であること
5. 全補助型（EventCausality、EventMetadata、EventSource、TransportMeta、DeliveryMode、EventVisibility、EventRetention、EventPrivacy）のシリアライズ/デシリアライズが正常動作すること
6. JSON ラウンドトリップ n = 1000 で 100% 成功すること
7. RFC §12C の定義との間にフィールドの過不足がないこと（人手照合）

### 成果物

- 計画: context/0064-*/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0064-*/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0064-*/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0064-*/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成）
