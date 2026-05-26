# 実装計画: M1.76-23 全ドメイン横断 Event Architecture 一貫性検証

## RFC 既存実装状態検証

### RFC §12C.1 DarviumEvent — Canonical Envelope

| フィールド | RFC の型 | 現行コードの型 | 状態 |
|---|---|---|---|
| event_id | EventId (String) | EventId (String) | ✅ 一致 |
| kind | DarviumEventKind | DarviumEventKind | ✅ 一致 |
| interaction_mode | InteractionMode | InteractionMode | ✅ 一致 |
| payload | serde_json::Value | serde_json::Value | ✅ 一致 |
| causality | EventCausality | EventCausality | ✅ 一致 |
| metadata | EventMetadata | EventMetadata | ✅ 一致 |
| transport_meta | Option\<TransportMeta> | Option\<TransportMeta> | ✅ 一致 |
| visibility | EventVisibility | EventVisibility | ✅ 一致 |
| retention | EventRetention | EventRetention | ✅ 一致 |
| privacy | EventPrivacy | EventPrivacy | ✅ 一致 |

### RFC §12C.2 DarviumEventKind — Event Taxonomy

| Variant | RFC の inner type | 現行コードの inner type | 状態 |
|---|---|---|---|
| System | SystemEvent | SystemEvent | ✅ 一致 |
| Search | SearchEvent | SearchEvent | ✅ 一致 |
| WorkflowExecution | WorkflowExecutionEvent | WorkflowExecutionEvent | ✅ 一致 |
| Training | TrainingEvent | TrainingEvent | ✅ 一致 |
| Knowledge | KnowledgeEvent | KnowledgeEvent | ✅ 一致 |
| Conversational | ConversationalEventEnvelope | ConversationalEventEnvelope | ✅ 一致 |
| Lifecycle | LifecycleEvent | LifecycleEvent | ✅ 一致 |
| Gc | GcEvent | GcEvent | ✅ 一致 |
| Repair | RepairEvent | RepairEvent | ✅ 一致 |
| Reciprocity | ReciprocityEvent | ReciprocityEventKind | ⚠️ 型名不一致（意味的に同一） |
| Fusion | FusionEvent | FusionEvent | ✅ 一致 |
| Hitl | HitlEvent | HitlEvent | ✅ 一致 |
| PresetRegistry | PresetRegistryEvent | (欠落) | ❌ 未実装（M-0.65 pending） |
| Village | (未定義) | VillageEvent | ⚠️ 余剰 variant（M1.75由来） |
| Extension | String | String | ✅ 一致 |

**評価サマリ**: 10/13 ドメイン variant が RFC 完全一致。Reciprocity の inner type 名のみ異なる（`ReciprocityEvent→ReciprocityEventKind`）が意味的に同一。PresetRegistry は M-0.65 未実装のため欠落。Village は M1.75 で追加された正当な余剰 variant。

### RFC §12C.9 不変条件（保証#11）

| 条件 | 現状 | 状態 |
|---|---|---|
| EventBus 単一性 | FakeEventBus は単一インスタンス前提 | ✅ 充足 |
| 全イベント通過 | FakeEventBus 経由の publish 必須 | ✅ 設計充足 |
| replay 分離 | replay は clock 非増加（既存テスト検証済み） | ✅ 検証済み |

## 要件の再確認

全13ドメイン（System, Search, WorkflowExecution, Training, Knowledge, Conversational, Lifecycle, Gc, Repair, Reciprocity, Fusion, Hitl, Village）の DarviumEvent が EventBus 経由で一貫して publish/replay/subscribe/projection されることを検証する。

**Non-scope 再確認**: Extension escape hatch は対象外、PresetRegistry は未実装のため対象外。

## 変更ファイル一覧

| ファイル | 種別 | 内容 |
|---|---|---|
| `src/event.rs` | modify | ① make_*_event ヘルパー13関数追加（公開API） ② DomainProjection コンストラクタ9種追加 ③ TC-1〜TC-7 テスト追加 |

変更は `src/event.rs` 1ファイルのみ。

## 実装手順

### Step 1: 全13ドメインの make_*_event 公開ヘルパー関数

各ヘルパーは `make_${domain}_event` の命名規則で `pub fn` として実装。

```rust
pub fn make_system_event(kind: SystemEvent, payload: serde_json::Value) -> DarviumEvent {
    DarviumEvent {
        event_id: uuid::Uuid::new_v4().to_string(),
        kind: DarviumEventKind::System(kind),
        interaction_mode: InteractionMode::OneWay,
        payload,
        causality: EventCausality::default(),
        metadata: EventMetadata { clock: 0, timestamp: SystemTime::UNIX_EPOCH, source: EventSource::Test },
        transport_meta: None,
        visibility: EventVisibility::Public,
        retention: EventRetention { persist: true, ttl_days: None },
        privacy: EventPrivacy::default(),
    }
}
```

13関数の内訳:
- `make_system_event(kind: SystemEvent, payload)`
- `make_search_event(kind: SearchEvent, payload)`
- `make_workflow_execution_event(kind: WorkflowExecutionEvent, payload)`
- `make_training_event(kind: TrainingEvent, payload)`
- `make_knowledge_event(kind: KnowledgeEvent, payload)`
- `make_conversational_event(kind: ConversationalEventEnvelope, payload)`
- `make_lifecycle_event(kind: LifecycleEvent, payload)`
- `make_gc_event(kind: GcEvent, payload)`
- `make_repair_event(kind: RepairEvent, payload)`
- `make_reciprocity_event(kind: ReciprocityEventKind, payload)`
- `make_fusion_event(kind: FusionEvent, payload)`
- `make_hitl_event(kind: HitlEvent, payload)`
- `make_village_event(kind: VillageEvent, payload)`

### Step 2: 不足9ドメインの DomainProjection コンストラクタ追加

既存パターン（`DomainProjection::search_trace()` 等）に従い実装:

1. `system_log()` — SystemEvent 全4種
2. `workflow_execution_log()` — WorkflowExecutionEvent 全4種
3. `knowledge_log()` — KnowledgeEvent 全4種
4. `conversational_log()` — ConversationalEventEnvelope 全5種
5. `lifecycle_log()` — LifecycleEvent 全4種
6. `gc_log()` — GcEvent 全3種
7. `repair_log()` — RepairEvent 全4種
8. `fusion_log()` — FusionEvent 全5種
9. `hitl_log()` — HitlEvent 全4種

### Step 3〜9: TC-1〜TC-7 テスト追加

spec 記載の Test Plan に従い7テスト関数を `mod tests` 内に追加。

## 計装・観測の実装計画

- テスト実装場所: `src/event.rs` の既存 `mod tests` ブロック内
- 観測出力: `println!` + `--nocapture` で JSON 構造化出力
- 固定シード: `StdRng::seed_from_u64(12345)`（TC-6 のみ）
- サンプルサイズ: TC-6 n=1300, TC-7 n=1300
- 検証コマンド: `cargo test -- --nocapture`
- 較正対象定数: なし（本チケットは検証が主目的）

## Boy Scout 改善

- 特になし。既存の `create_event_with_kind` は維持し、新規 `make_*_event` が上位互換として機能する

## 物理的レビュー方法

1. `cargo test -- --nocapture` で全テスト通過を確認
2. 翻訳可能性 grep:
   - 新規追加した関数名が動詞句（`make_` 接頭辞）であることを確認
   - 新規追加した変数名に1文字変数や汎用名がないことを確認
   - ハードコード値がないことを確認
3. `cargo clippy -- -D warnings` で clippy 警告ゼロ確認

## リスク

- **低**: 新規 DomainProjection コンストラクタ追加は既存パターンの踏襲
- **低**: make_*_event ヘルパーは純粋コンストラクタ関数、副作用なし
- **低**: テストのみの追加で既存プロダクションコードに影響なし
