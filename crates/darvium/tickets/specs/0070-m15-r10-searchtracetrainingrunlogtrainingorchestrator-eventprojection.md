---
ticket_id: 70
title: M1.5-R10: ドメイン統合 — SearchTrace・TrainingRunLog・TrainingOrchestrator の EventProjection 化
slug: m15-r10-searchtracetrainingrunlogtrainingorchestrator-eventprojection
status: reviewed
created_at: 2026-05-24
updated_at: 2026-05-24
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0070-m15-r10-searchtracetrainingrunlogtrainingorchestrator-eventprojection/plan.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0070-m15-r10-searchtracetrainingrunlogtrainingorchestrator-eventprojection/observation-20260524-133216.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0070-m15-r10-searchtracetrainingrunlogtrainingorchestrator-eventprojection/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0070-m15-r10-searchtracetrainingrunlogtrainingorchestrator-eventprojection/review.md
---

# M1.5-R10: ドメイン統合 — SearchTrace・TrainingRunLog・TrainingOrchestrator の EventProjection 化

## Summary

R9 で整備された `EventProjection` フレームワーク上に、検索・訓練・相互互恵性の各ドメイン特化 Projection を実装する。具体的には `SearchTraceProjection`、`TrainingRunLogProjection`、`ReciprocityEventProjection`、`SearchRunLogProjection` の4つの具象 Projection を `EventProjection` トレイトの実装として提供し、これらを `ProjectionCatalog` に一括登録する `initialize_domain_projections()` 関数を追加する。さらに既存の `SearchTrace` 保存コードに EventBus publish 経路を追加する双方向対応（dual-path）を行う。

## Background

v2.3-g Event Architecture の最終目標は「全ドメイン状態が DarviumEvent ストリームから materialize 可能であること」である。R9 で基盤（`EventProjection` トレイト・`ProjectionCatalog`）が完成したため、現時点では unit struct に留まっている以下のドメイン状態を EventProjection として具体化する：

- **SearchTrace** (`types.rs:5023`): unit struct (`pub struct SearchTrace;`)。`MetadataStore` 経由で store/load は実装済みだが、内容は空。EventProjection として検索イベント系列を materialize する経路を追加する。
- **TrainingRunLog**: コードベースに存在しない。`TrainingEvent` 系列から訓練実行ログを materialize する。
- **ReciprocityEvent**: `event.rs:288` で8 variant の enum として定義済み。`ReciprocityEventProjection` により互恵性イベント系列を materialize する。
- **SearchRunLog**: 検索実行ログの Projection（`DarviumEventKind::Search` の subset、主に StepCompleted/Completed/Failed/Aborted を対象）。

これらの Projection が確立されることで、M1.76-21 の外部イベント購読基盤（`EventSubscriber` + `WebSocketEventChannel`）におけるドメイン別イベント再配送の基盤が整う。

## Scope

- `SearchTraceProjection`: `DarviumEventKind::Search` イベントから SearchTrace を materialize する `EventProjection` 実装
- `TrainingRunLogProjection`: `DarviumEventKind::Training` イベントから TrainingRunLog を materialize する `EventProjection` 実装
- `ReciprocityEventProjection`: `DarviumEventKind::Reciprocity` イベントから ReciprocityEvent 系列を materialize する `EventProjection` 実装
- `SearchRunLogProjection`: 検索実行ログの `EventProjection` 実装（Search の subset: StepCompleted, Completed, Failed, Aborted）
- `initialize_domain_projections()`: 上記4 Projection を `ProjectionCatalog` に登録する初期化関数
- Dual-path 対応: 既存の `SearchTrace` 保存箇所（`DualStoreCoordinator`）に EventBus publish 経路を追加（既存の MetadataStore 経路は互換性のため存続）
- エラー型: 既存の `DarviumError::Projection` variant で対応可能なため追加不要

## Non-scope

- `EventProjection` トレイト自体の変更（R9 で確定済み）
- `ProjectionCatalog` トレイト自体の変更（同上）
- 非同期版 `EventProjection`（M1.76-21 と併せて検討）
- `TrainingOrchestrator` 本体の実装（別チケット。本チケットでは `TrainingRunLogProjection` による訓練イベントの projection のみ）
- `SearchTrace` 構造体のフィールド拡張（本チケットでは projection の materialize 検証に必要な最小限の拡張に留める）
- Snapshot の永続化戦略（OQ-15）

## Investigation

### 現状確認 (2026-05-24)

**EventProjection フレームワーク（R9 完了）:**

- `src/event.rs` に以下が実装済み（全39 tests PASS）:
  - `EventProjection` トレイト: `name()`, `interested_kinds()`, `project()`, `snapshot()`, `clear()`
  - `ProjectionEventFilter`: `all()`, `from_kinds()`, `matches()`
  - `ProjectionCatalog` トレイト: `register()`, `get()`, `project_all()`
  - `FakeProjection`: テスト用メモリ内実装
  - `FakeProjectionCatalog`: テスト用メモリ内実装

**DarviumEventKind variant 一覧（全13 variant, event.rs:345-372）:**

| Kind | Subtype | 用途 |
|------|---------|------|
| `System` | `SystemEvent` (4種) | システム内部 |
| `Search` | `SearchEvent` (5種: Started, StepCompleted, Completed, Failed, Aborted) | 検索ライフサイクル |
| `WorkflowExecution` | `WorkflowExecutionEvent` (4種) | ワークフロー実行 |
| `Training` | `TrainingEvent` (9種: MissionGenerated, HumanReviewRequested, HumanReviewCompleted, SandboxExecutionStarted, SandboxExecutionCompleted, FeedbackIngested, PromotionCandidateCreated, PromotionApproved, PromotionRejected) | 訓練 |
| `Knowledge` | `KnowledgeEvent` (4種) | 知識 |
| `Conversational` | `ConversationalEventEnvelope` (5種) | 会話 |
| `Lifecycle` | `LifecycleEvent` (4種) | ライフサイクル |
| `Gc` | `GcEvent` (3種) | GC |
| `Repair` | `RepairEvent` (4種) | 修復 |
| `Reciprocity` | `ReciprocityEvent` (8種: HelpOffered, HelpAccepted, HelpRejected, HelpExecuted, HelpSucceeded, HelpAbandoned, HarmfulMismatch, ReturnedFavor) | 互恵性 |
| `Fusion` | `FusionEvent` (5種) | 融合 |
| `Hitl` | `HitlEvent` (4種) | HITL |
| `Extension` | `String` | 拡張用 escape hatch |

**SearchTrace 現状:**

- `types.rs:5023`: `pub struct SearchTrace;` — unit struct（空）
- `MetadataStore` トレイト (`src/store/metadata_store.rs:22-25`): `store_search_trace()`, `load_search_traces()` 定義済み
- `InMemoryMetadataStore` (`src/store/metadata_store.rs:141`): 内部で `HashMap<String, Vec<SearchTrace>>` として管理
- `JsonMetadataStore` (`src/store/json_metadata_store.rs:46`): 同様に `Mutex<HashMap<String, Vec<SearchTrace>>>`
- `DualStoreCoordinator` (`src/store/coordinator.rs:118,234`): `store_search_trace(&SearchTrace)` を呼び出し中

**未実装の型:**

- `TrainingRunLog`: コードベースに存在しない。新規定義が必要。
- `TrainingOrchestrator`: 本体は未実装（別チケット）。本チケットでは `TrainingRunLogProjection` による訓練イベントの観測のみ。

### 参照観察レポート

- `tickets/context/0069-m15-r9-eventprojection-projectioncatalog/observation-20260524-131633.md`
  — R9 完了。EventProjection フレームワーク全 39 tests PASS。
  フィルタリング精度 100%、クロスプロジェクション汚染 0 を確認。
  次のステップとしてドメイン特化 Projection の実装を記録。

### 設計判断

1. **SearchTraceProjection**: Search イベントを時系列で蓄積する単純な `Vec<DarviumEvent>` を内部状態とする。`snapshot()` は蓄積された Search イベントの JSON 配列を返す。`interested_kinds()` は `DarviumEventKind::Search` のみ。
2. **TrainingRunLogProjection**: 同上、Training イベントのみを蓄積。`TrainingRunLog` 型は Vec の newtype または型エイリアスとして定義。
3. **ReciprocityEventProjection**: 同上、Reciprocity イベントのみを蓄積。
4. **SearchRunLogProjection**: SearchEvent の subset（StepCompleted, Completed, Failed, Aborted）のみを蓄積。`ProjectionEventFilter::from_kinds()` でフィルタリング。
5. **initialize_domain_projections()**: `FakeProjectionCatalog` または任意の `ProjectionCatalog` 実装に対して4 Projection を登録する関数。トレイト境界は `ProjectionCatalog` のみ。
6. **Dual-path**: `DualStoreCoordinator` の SearchTrace 保存箇所に `EventBus::publish()` 呼び出しを追加（Optional、設定で有効化）。既存の MetadataStore 経路は変更しない。
7. **配置**: 全ドメイン Projection は `src/event.rs` に追加（additive）。FakeProjection のセクション直後に配置。

## Test Plan

### TC-1: SearchTraceProjection の materialize 完全性
- DarviumEventKind::Search(SearchEvent::Started) を publish
- SearchTraceProjection の snapshot() が当該イベントを含むことを確認
- 5種の SearchEvent を順次 publish → 全5件が materialize されること

### TC-2: TrainingRunLogProjection の materialize 完全性
- DarviumEventKind::Training(TrainingEvent::MissionGenerated) を publish
- TrainingRunLogProjection の snapshot() が当該イベントを含むことを確認
- 9種の TrainingEvent を順次 publish → 全9件が materialize されること

### TC-3: ReciprocityEventProjection の materialize 完全性
- DarviumEventKind::Reciprocity(ReciprocityEvent::HelpOffered) を publish
- ReciprocityEventProjection の snapshot() が当該イベントを含むことを確認
- 8種の ReciprocityEvent を順次 publish → 全8件が materialize されること

### TC-4: SearchRunLogProjection の subset フィルタリング
- 5種の SearchEvent を publish（Started x1, StepCompleted x2, Completed x1, Failed x1, Aborted x1）
- SearchRunLogProjection の snapshot() に Started が含まれず、StepCompleted/Completed/Failed/Aborted のみ含まれること
- subset フィルタの正確性を確認

### TC-5: initialize_domain_projections() による全 Projection の一括登録
- 空の ProjectionCatalog に対して initialize_domain_projections() を実行
- 4つの Projection が全て登録されていること（registered_names() で確認）
- 各 name が一意であること

### TC-6: ドメイン混在 publish 時の分離完全性（cross-domain contamination 0）
- Search / Training / Reciprocity / WorkflowExecution イベントを混在 publish
- SearchTraceProjection には Search イベントのみ含まれる
- TrainingRunLogProjection には Training イベントのみ含まれる
- ReciprocityEventProjection には Reciprocity イベントのみ含まれる
- SearchRunLogProjection には Search subset のみ含まれる
- 全 Projection 間で contamination 0 であること

### TC-7: clear() 後の state リセット
- 各 Projection に複数イベントを project した後、clear() を呼び出し
- 全 Projection の snapshot() が空を返すこと

### TC-8: 計装 — n = 1000 一括配送後、各 Projection の独立完全性
- 4つの Projection を catalog に登録
- 1000 ランダムイベントを生成し project_all() で一括配送
- 各 Projection の snapshot に正しい kind のイベントのみ含まれること
- フィルタリング精度（配送イベントの kind 一致率 100%）を検証
- SearchRunLog の subset フィルタが正しく機能すること

### TC-9: Dual-path — EventBus publish + Projection materialize の一貫性
- FakeEventBus に Search イベントを publish
- EventBus の replay() で取得したイベントと Projection の snapshot() 内容が一致すること
- publish 前の EventBus 状態が Projection に影響を与えないこと

## 計装方法・観測対象

### 計装方法
- 全テストは `src/event.rs` の `mod tests` に追加（additive）
- 固定シード PRNG (`StdRng::seed_from_u64(12345)`) を使用
- `println!` + `--nocapture` で観測データを標準出力に書き出す
- n = 1000 の計装テスト（TC-8）で配送完全性・フィルタリング精度を観測

### 観測対象

| 観測量 | サンプルサイズ | 検証方法 |
|--------|---------------|----------|
| 各 Projection の materialize 完全性 | 全 variant 網羅 | assert_eq! |
| フィルタリング精度（kind 一致率） | n = 1000 | 統計的観測（100% 期待） |
| クロスプロジェクション汚染数 | n = 1000 | 0 であることを assert |
| SearchRunLog subset フィルタ精度 | 全 Search variant | assert (Started 除外確認) |
| EventBus ↔ Projection 一貫性 | TC-9 | assert_eq! |
| clear 後 state リセット | 全 Projection | assert (空確認) |

### 較正計画
本チケットに較正すべき定数は存在しない。純粋な型定義とトレイト実装。

## Boy Scout Rule — 翻訳可能性計画

### 対象ファイル
- `src/event.rs`: 本チケットの実装を `FakeProjectionCatalog` 実装の直後に additive 追加。既存コードは編集しない。
- `src/types.rs`: 必要に応じて `TrainingRunLog` 型エイリアスを追加（最小限）。

### 改善項目
- 関数名は動詞句 (`project`, `snapshot`, `clear`, `initialize_domain_projections`) で統一
- 変数名はドメイン概念 (`search_trace_projection`, `training_run_log_projection`, `reciprocity_projection`) で統一
- ハードコードせず、テストサンプルサイズは `const BULK_EVENT_COUNT: usize = 1000` として定数化
- SearchRunLog の subset 定義は名前付き定数 `SEARCH_RUN_LOG_KINDS` として宣言

## Acceptance Criteria

- [ ] SearchTraceProjection が EventProjection を実装し、Search イベントを materialize できる
- [ ] TrainingRunLogProjection が EventProjection を実装し、Training イベントを materialize できる
- [ ] ReciprocityEventProjection が EventProjection を実装し、Reciprocity イベントを materialize できる
- [ ] SearchRunLogProjection が EventProjection を実装し、Search subset のみを materialize できる
- [ ] initialize_domain_projections() が4 Projection を catalog に一括登録できる
- [ ] TC-1〜TC-9 が全て PASS すること
- [ ] 既存テスト（event モジュール39 tests + 全 crate tests）に影響を与えないこと
- [ ] フィルタリング精度 100%、クロスプロジェクション汚染 0 を n = 1000 で確認

## Notes

- plan_path: context/0070-m15-r10-searchtracetrainingrunlogtrainingorchestrator-eventprojection/plan.md（未作成、/plan-ticket 承認後に作成）
- implementation_path: ../context/0070-m15-r10-searchtracetrainingrunlogtrainingorchestrator-eventprojection/implementation.md（未作成、/start-ticket 実装完了後に作成）
- review_report_path: ../context/0070-m15-r10-searchtracetrainingrunlogtrainingorchestrator-eventprojection/review.md（未作成、/review-ticket 全チェック通過後に作成）
- observation_report_path: ../context/0070-m15-r10-searchtracetrainingrunlogtrainingorchestrator-eventprojection/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）

### 成果物

- 計画: context/0070-m15-r10-searchtracetrainingrunlogtrainingorchestrator-eventprojection/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0070-m15-r10-searchtracetrainingrunlogtrainingorchestrator-eventprojection/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0070-m15-r10-searchtracetrainingrunlogtrainingorchestrator-eventprojection/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0070-m15-r10-searchtracetrainingrunlogtrainingorchestrator-eventprojection/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
