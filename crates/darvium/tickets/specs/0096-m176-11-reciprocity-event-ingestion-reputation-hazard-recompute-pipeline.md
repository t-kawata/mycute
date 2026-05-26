---
ticket_id: 96
title: M1.76-11: ReciprocityEvent インジェスション + reputation/hazard recompute パイプライン
slug: m176-11-reciprocity-event-ingestion-reputation-hazard-recompute-pipeline
status: reviewed
created_at: 2026-05-26
updated_at: 2026-05-26
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0096-m176-11-reciprocity-event-ingestion-reputation-hazard-recompute-pipeline/observation-20260526-101011.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0096-m176-11-reciprocity-event-ingestion-reputation-hazard-recompute-pipeline/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0096-m176-11-reciprocity-event-ingestion-reputation-hazard-recompute-pipeline/review.md
---

# M1.76-11: ReciprocityEvent インジェスション + reputation/hazard recompute パイプライン

## Summary

ReciprocityEvent のイベントソーシング基盤として、EventProjection 経由で materialize された ReciprocityEvent を管理する ReciprocityEventStore を実装する。本ストアを入力として、全グラフの ReputationProfile 一括再計算（recompute_all_profiles）と全グラフの GC hazard 一括再計算（recompute_all_gc_hazards）を行うパイプラインを構築する。また、再計算結果のスナップショット比較機構（ReciprocityReplaySnapshot / compute_replay_comparison）により、再計算の決定論的再現性およびバージョン間差分を検証可能にする。

## Background

- RFC §41B.20 で定義される Reciprocity-Aware Survival の計算基盤。
- 既存の純粋関数群（F-1〜F-15）は個別の計算を提供するが、これらを直列化して全グラフの一括再計算を行うパイプラインが未実装。
- `DomainProjection::reciprocity_event()`（`src/event.rs:1473`）は既に ReciprocityEventProjection として実装済み。本チケットはその先のストアとパイプラインを担当する。

### 既存コードとの関連

- `ReciprocityEvent`（`src/event.rs:332`）: 9フィールドの基本型。`DarviumEvent` からの `TryFrom` による変換が実装済み（`src/event.rs:358`）。
- `ReciprocityEventKind`（`src/event.rs:308`）: 8 variant の種別。`DarviumEventKind::Reciprocity(kind)` として使用。
- `ReputationProfile`（`src/event.rs:402`）: 16フィールドの評判プロファイル。`cold_start()` および `recompute_reputation()` が実装済み。
- `compute_direct_reciprocity`（`src/reciprocity.rs:63`）: F-1 純粋関数（f32→f32）。
- `compute_indirect_reciprocity`（`src/reciprocity.rs:113`）: F-2 純粋関数（5入力→f32）。
- `compute_benevolence_score`（`src/reciprocity.rs:149`）: F-3 純粋関数（3入力→f32）。
- `recompute_reputation`（`src/reciprocity.rs:214`）: F-4/F-5 ReputationProfile 再計算。
- `compute_gc_hazard`（`src/reciprocity.rs:279`）: F-7 純粋関数（3入力+policy→f32）。
- `compute_child_protection`（`src/reciprocity.rs:355`）: F-10 純粋関数。パイプラインでは child_protection を外部から受け取る前提。
- `ReciprocityLifecyclePolicy`（`src/event.rs:481`）: 全 F-1〜F-15 の較正パラメータ。`Default` 実装完備。

## Scope

### 1. 定数追加（`src/constants.rs`）

| 定数名 | Default | 範囲 | 説明 |
|--------|---------|------|------|
| `RECIPROCITY_EVENT_PROJECTION_NAME` | `"reciprocity_event"` | — | ReciprocityEventProjection の登録名 |
| `RECIPROCITY_STORE_INITIAL_CAPACITY` | `64` | 16-1024 | ReciprocityEventStore の初期 HashMap 容量 |

### 2. `GraphMetrics` 構造体追加（`src/event.rs`）

Reciprocity pipeline で使用するグラフメトリクスの軽量構造体。`recompute_all_profiles` の入力として使用する。

```rust
/// グラフメトリクス — Reciprocity recompute pipeline の入力。
///
/// 各グラフの F-2（間接互恵性）および F-4（評判再計算）に必要な
/// 最小限のメトリクスを保持する。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphMetrics {
    /// Helper network 上の中心性 C_i^help ∈ [0, 1]
    pub centrality: f32,
    /// Local village 参加度 A_i^village ∈ [0, 1]
    pub village_participation: f32,
    /// Offer 受諾率 U_i^accepted ∈ [0, 1]
    pub accepted_rate: f32,
    /// 支援成功率 Q_i^success ∈ [0, 1]
    pub success_rate: f32,
    /// 負評価スコア B_i^harm ∈ [0, 1]
    pub harm_score: f32,
    /// 継承スコア I_i ∈ [0, 1]（F-4 θ_inh と乗算）
    pub inherited_score: f32,
    /// 経験値カウント experience_count(i)（F-5 κ_E と乗算）
    pub experience_count: u32,
}

impl Default for GraphMetrics {
    fn default() -> Self {
        Self {
            centrality: 0.0,
            village_participation: 0.0,
            accepted_rate: 0.0,
            success_rate: 0.0,
            harm_score: 0.0,
            inherited_score: 0.0,
            experience_count: 0,
        }
    }
}
```

### 3. `ReciprocityEventStore` 構造体（`src/reciprocity.rs`）

```rust
/// ReciprocityEvent のメモリ内ストア。
///
/// source_graph_id をキーとしてイベントを管理する。
/// EventProjection 経由で materialize されたイベントの投影結果を保持する。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReciprocityEventStore {
    /// source_graph_id → Vec<ReciprocityEvent> のマップ
    events: HashMap<WorkflowGraphId, Vec<ReciprocityEvent>>,
}
```

**メソッド:**

| メソッド | シグネチャ | 説明 |
|----------|-----------|------|
| `new` | `() -> Self` | 空のストアを作成 |
| `ingest` | `(&mut self, event: ReciprocityEvent)` | イベントを source_graph_id で索引付けして追加 |
| `get_events` | `(&self, source_graph_id: &str) -> &[ReciprocityEvent]` | 指定グラフの全イベント（空スライス可能） |
| `all_graph_ids` | `(&self) -> Vec<WorkflowGraphId>` | 全グラフ ID のリスト |
| `event_count` | `(&self, source_graph_id: &str) -> usize` | 指定グラフのイベント数 |
| `total_events` | `(&self) -> usize` | 全イベント総数 |
| `clear` | `(&mut self)` | 全イベントをクリア |

### 4. `ingest_reciprocity_event` 関数（`src/reciprocity.rs`）

```rust
/// DarviumEvent から ReciprocityEvent を materialize し、ストアに追加する。
///
/// EventBus から受け取った DarviumEventKind::Reciprocity イベントを
/// ReciprocityEvent に変換してストアに投影する。
/// 非 Reciprocity kind の場合はエラーを返す。
pub fn ingest_reciprocity_event(
    store: &mut ReciprocityEventStore,
    event: DarviumEvent,
) -> Result<(), DarviumError>
```

- 内部で `ReciprocityEvent::try_from(event)` を呼び出し、成功時に `store.ingest()` を呼ぶ。
- `DarviumError::ReciprocityError` でエラーを伝播。

### 5. `recompute_all_profiles` 関数（`src/reciprocity.rs`）

```rust
/// 全グラフの ReputationProfile を一括再計算する。
///
/// パイプライン（各グラフ ID に対して）:
/// 1. ストアから全イベントを取得
/// 2. compute_direct_reciprocity で直接互恵性スコア計算
/// 3. メトリクスから間接互恵性スコア計算（compute_indirect_reciprocity）
/// 4. recompute_reputation で評判スコア再計算
/// 5. compute_benevolence_score で慈悲スコア計算
/// 6. 上記結果を ReputationProfile に反映
pub fn recompute_all_profiles(
    store: &ReciprocityEventStore,
    metrics: &HashMap<WorkflowGraphId, GraphMetrics>,
    now: u64,
    policy: &ReciprocityLifecyclePolicy,
) -> HashMap<WorkflowGraphId, ReputationProfile>
```

**内部処理の詳細（各 graph_id に対して）:**

1. `events = store.get_events(graph_id)`
2. `direct_score = compute_direct_reciprocity(events, now, policy)`
3. メトリクスから間接互恵性スコアを計算:
   ```rust
   let m = &metrics[graph_id];
   let indirect_score = compute_indirect_reciprocity(
       m.centrality, m.village_participation,
       m.accepted_rate, m.success_rate, m.harm_score,
   );
   ```
4. `recompute_reputation(ReputationInputs { direct_score, indirect_score, experience_count: m.experience_count, inherited_score: m.inherited_score }, policy)` でベースプロファイルを取得
5. `benevolence_score = compute_benevolence_score(direct_score, indirect_score, profile.final_score)`
6. プロファイルの `benevolence_score` を上書き（F-3 は F-4 の後処理として実行）
7. プロファイルの `village_centrality` をメトリクスの `centrality` で上書き

**不変条件:**
- 空ストア → 空 HashMap（MUST）
- 同一 event stream + 同一 metrics + 同一 policy → 同一 ReputationProfile（MUST）
- 各 profile の直接スコア・間接スコア・最終スコアは [0, 1] に bounded（MUST）
- benevolence_score は [0, 1] に bounded（MUST）

### 6. `recompute_all_gc_hazards` 関数（`src/reciprocity.rs`）

```rust
/// 全グラフの GC hazard を一括再計算する。
///
/// 各 graph_id に対して:
/// 1. lifecycle_score を lifecycle_scores マップから取得
/// 2. benevolence_score を profile から取得
/// 3. child_protection_score を child_protections マップから取得（なければ 0）
/// 4. compute_gc_hazard(lifecycle, benevolence, child_protection, policy) を呼ぶ
pub fn recompute_all_gc_hazards(
    profiles: &HashMap<WorkflowGraphId, ReputationProfile>,
    lifecycle_scores: &HashMap<WorkflowGraphId, f32>,
    child_protections: &HashMap<WorkflowGraphId, f32>,
    policy: &ReciprocityLifecyclePolicy,
) -> HashMap<WorkflowGraphId, f32>
```

**不変条件:**
- 全 hazard 値は非負（MUST, softplus の性質）
- lifecycle_scores に存在し profiles に存在しないキーは hazard map に含まれない
- 空 profiles → 空 HashMap（MUST）

### 7. `ReciprocityReplaySnapshot` 構造体（`src/reciprocity.rs`）

```rust
/// 再計算結果のスナップショット。
///
/// 2 時点の再計算結果比較（compute_replay_comparison）のための
/// 比較可能な状態を保持する。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReciprocityReplaySnapshot {
    /// profiles のコピー
    pub profiles: HashMap<WorkflowGraphId, ReputationProfile>,
    /// GC hazard のコピー
    pub hazards: HashMap<WorkflowGraphId, f32>,
    /// 計算に使用したポリシーバージョン
    pub policy_version: String,
    /// 計算時の VirtualClock 値
    pub clock: u64,
}
```

### 8. `ReciprocityDiffReport` 構造体（`src/reciprocity.rs`）

```rust
/// 2 つのスナップショット間の差分レポート。
///
/// 各差分は (before, after) のタプルで記録される。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReciprocityDiffReport {
    /// 比較前のポリシーバージョン
    pub before_policy_version: String,
    /// 比較後のポリシーバージョン
    pub after_policy_version: String,
    /// 差分があったグラフ ID の一覧
    pub changed_graph_ids: Vec<WorkflowGraphId>,
    /// 各グラフの profile 差分（キーがグラフ ID、値が (before_final_score, after_final_score)）
    pub profile_diffs: HashMap<WorkflowGraphId, (f32, f32)>,
    /// 各グラフの hazard 差分（キーがグラフ ID、値が (before_hazard, after_hazard)）
    pub hazard_diffs: HashMap<WorkflowGraphId, (f32, f32)>,
    /// 新規追加されたグラフ ID
    pub added_graph_ids: Vec<WorkflowGraphId>,
    /// 削除されたグラフ ID
    pub removed_graph_ids: Vec<WorkflowGraphId>,
}
```

### 9. `compute_replay_comparison` 関数（`src/reciprocity.rs`）

```rust
/// 2 つのスナップショットを比較し、差分レポートを生成する。
///
/// # Args
/// - before: 比較元スナップショット
/// - after: 比較先スナップショット
///
/// # Returns
/// - profiles/hazards の全フィールド差分を含む ReciprocityDiffReport
///
/// # 不変条件
/// - before と after のどちらかが空でも panic しない（空の DiffReport を返す）
/// - 差分がない場合、changed_graph_ids / profile_diffs / hazard_diffs は空
pub fn compute_replay_comparison(
    before: &ReciprocityReplaySnapshot,
    after: &ReciprocityReplaySnapshot,
) -> ReciprocityDiffReport
```

### 10. テスト

以下のテストを `src/reciprocity.rs` の `mod tests` に追加する。

| ID | 種別 | 条件 | 期待結果 |
|-----|------|------|---------|
| R11-T1 | 境界値 | 空 store で recompute_all_profiles | 空 HashMap |
| R11-T2 | 決定論的リプレイ | 同一 event stream × 2 回 | 完全一致した profiles |
| R11-T3 | 差分検出 | 異なる policy_version で recompute | DiffReport に差分が正確に記録 |
| R11-T4 | イベント追加効果 | 1件 event 追加後の recompute | 追加前と異なる結果 |
| R11-T5 | policy_version 記録 | snapshot 作成後 | policy_version が snapshot に正確に保持 |
| R11-T6 | 境界値 | 空 store で recompute_all_gc_hazards | 空 HashMap |
| R11-T7 | 非負性 | ランダムパラメータ n=10⁴ | 全 hazard 非負 |
| R11-T8 | 差分なし | 同一 snapshot の比較 | DiffReport の全 diff 空 |
| R11-T9 | 観測 | 応答曲面出力 | 各パイプラインの出力を println! |

## Non-scope

- ReciprocityEventProjection の実装（`DomainProjection::reciprocity_event()` は既に M1.5-R10 で完了）
- `classify_maturity` への F-15 直接埋め込み（M1.76-11 以降の統合）
- F-14 growth increment と F-10 child_protection の実配管（M1.76-11 では child_protection を外部入力として受け取る）
- 較正ループ（J(θ) は M1.76-16 で扱う）
- 時系列イベントクリーニング（古いイベントの削除ポリシー）
- 永続ストア統合（本チケットはメモリ内操作のみ）

## Investigation

### 参照観察レポート

- `context/0095-m176-10-child-growth-increment-f-14-maturation-probability-f-15/observation-YYYYMMDD-HHmmss.md` — F-14/F-15 の観測結果をパイプラインの downstream で使用可能。

### 関連ソースコード箇所

| ファイル | 該当箇所 | 備考 |
|----------|----------|------|
| `src/event.rs:332-391` | `ReciprocityEvent` / `TryFrom<DarviumEvent>` | 変換基盤は完了済み |
| `src/event.rs:402-470` | `ReputationProfile` | 16フィールド完備 |
| `src/event.rs:481-591` | `ReciprocityLifecyclePolicy` | Default 実装完備 |
| `src/event.rs:1472-1487` | `DomainProjection::reciprocity_event()` | ReciprocityEventProjection は既存 |
| `src/reciprocity.rs:63-89` | `compute_direct_reciprocity` | F-1 |
| `src/reciprocity.rs:113-133` | `compute_indirect_reciprocity` | F-2 |
| `src/reciprocity.rs:149-161` | `compute_benevolence_score` | F-3 |
| `src/reciprocity.rs:214-246` | `recompute_reputation` | F-4/F-5 |
| `src/reciprocity.rs:279-290` | `compute_gc_hazard` | F-7 |
| `src/reciprocity.rs:355-369` | `compute_child_protection` | F-10 |

### 欠落している構成要素

1. `ReciprocityEventStore` 構造体およびメソッド — 新規作成
2. `ingest_reciprocity_event` 関数 — 新規作成
3. `recompute_all_profiles` 関数 — 新規作成
4. `recompute_all_gc_hazards` 関数 — 新規作成
5. `ReciprocityReplaySnapshot` 構造体 — 新規作成
6. `ReciprocityDiffReport` 構造体 — 新規作成
7. `compute_replay_comparison` 関数 — 新規作成
8. `GraphMetrics` 構造体 — 新規作成（`src/event.rs` に追加）
9. `RECIPROCITY_EVENT_PROJECTION_NAME` 定数 — 新規作成（`src/constants.rs` に追加）

### 実装方針

全実装は `src/reciprocity.rs` に追加する（`GraphMetrics` のみ `src/event.rs` に定義）。

`recompute_all_profiles` のデータフロー:

```
ReciprocityEventStore ──→ compute_direct_reciprocity ──→ direct_score ──┐
                                                                         ├──→ recompute_reputation → ReputationProfile
GraphMetrics ──→ compute_indirect_reciprocity ──→ indirect_score ────────┘     │
     └── experience_count, inherited_score ────────────────────────────────────┘
                                                                               ↓
                                                                     compute_benevolence_score
                                                                         │
                                                                         ↓
                                                                  ReputationProfile.benevolence_score 上書き
```

`recompute_all_gc_hazards` のデータフロー:

```
lifecycle_scores[g] ────┐
profiles[g].benevolence  ├──→ compute_gc_hazard ──→ hazard[g]
child_protections[g] ────┘
```

```rust
// recompute_all_profiles 擬似コード:
pub fn recompute_all_profiles(...) -> HashMap<WorkflowGraphId, ReputationProfile> {
    let graph_ids = store.all_graph_ids();
    let mut results = HashMap::with_capacity(graph_ids.len());
    for g_id in &graph_ids {
        let events = store.get_events(g_id);
        let direct = compute_direct_reciprocity(events, now, policy);
        let m = metrics.get(g_id).cloned().unwrap_or_default();
        let indirect = compute_indirect_reciprocity(
            m.centrality, m.village_participation,
            m.accepted_rate, m.success_rate, m.harm_score,
        );
        let mut profile = recompute_reputation(ReputationInputs {
            direct_score: direct,
            indirect_score: indirect,
            experience_count: m.experience_count,
            inherited_score: m.inherited_score,
        }, policy);
        profile.benevolence_score = compute_benevolence_score(direct, indirect, profile.final_score);
        profile.village_centrality = m.centrality;
        results.insert(g_id.clone(), profile);
    }
    results
}
```

## Test Plan

### ユニットテスト（`src/reciprocity.rs` の `mod tests` に追加）

**R11-T1: 空 store で recompute_all_profiles → 空 HashMap**

```rust
let store = ReciprocityEventStore::new();
let metrics = HashMap::new();
let profiles = recompute_all_profiles(&store, &metrics, 0, &policy);
assert!(profiles.is_empty());
```

**R11-T2: 同一 event stream × 2 で完全一致**

```rust
// 同一の DarviumEvent 系列を 2 回、別々のストアに ingest
// 同一 metrics + policy で recompute
// → 全 profile フィールドがビットレベル一致
```

**R11-T3: 異なる policy_version → DiffReport に差分**

```rust
// policy_v1 と policy_v2（lambda_gc_base のみ変更）で recompute
// → DiffReport に hazard_diffs のみ現れ profile_diffs は空（lambda は reputation に影響しない）
// → policy_version が正確に記録される
```

**R11-T4: 1件 event 追加で結果が変化**

```rust
// 1件 event を追加して recompute → 追加前と異なることを確認
// event が計算に反映されること
```

**R11-T5: policy_version が snapshot に記録**

```rust
let snapshot = ReciprocityReplaySnapshot {
    policy_version: policy.policy_version.clone(),
    ...
};
assert_eq!(snapshot.policy_version, "test-version");
```

**R11-T6: 空 store で recompute_all_gc_hazards → 空 HashMap**

```rust
let hazards = recompute_all_gc_hazards(&HashMap::new(), &HashMap::new(), &HashMap::new(), &policy);
assert!(hazards.is_empty());
```

**R11-T7: 全 hazard 非負（n=10⁴）**

```rust
// ランダムな profile / lifecycle_score / child_protection を生成
// → 全 hazard が NaN/Inf でなく非負
```

**R11-T8: 同一 snapshot → 差分なし**

```rust
let report = compute_replay_comparison(&snapshot, &snapshot);
assert!(report.changed_graph_ids.is_empty());
assert!(report.profile_diffs.is_empty());
assert!(report.hazard_diffs.is_empty());
```

**R11-T9: 観測テスト**

- recompute_all_profiles の応答: グラフ数×イベント数を sweep した計算結果
- recompute_all_gc_hazards の応答: lifecycle_score / benevolence による hazard 分布
- 固定シード PRNG（`StdRng::seed_from_u64(12345)`）を使用

### 計装方法・観測対象

#### 計装方法

- `recompute_all_profiles` の結果（グラフ数・各 profile の final_score 分布）を `println!` + `--nocapture` で標準出力
- `recompute_all_gc_hazards` の結果（hazard の分布・平均・標準偏差）を同様に出力
- `compute_replay_comparison` の差分数量（変更グラフ数・差分絶対値和）を出力

#### 観測対象

- 空ストア → 空 HashMap の保証
- 同一イベント系列の決定論的再現性
- ポリシーバージョン変更による計算結果の差異範囲
- 複数グラフ混在時のパイプライン独立性（グラフ A のイベントがグラフ B の profile に影響しない）

## Acceptance Criteria

- [ ] `GraphMetrics` 構造体が `src/event.rs` に定義されている
- [ ] `ReciprocityEventStore` 構造体が `src/reciprocity.rs` に定義され、全メソッドが実装されている
- [ ] `ingest_reciprocity_event` 関数が実装されている（DarviumEvent→ReciprocityEvent→store）
- [ ] `recompute_all_profiles` 関数が実装されている
- [ ] `recompute_all_gc_hazards` 関数が実装されている
- [ ] `ReciprocityReplaySnapshot` 構造体が定義されている
- [ ] `ReciprocityDiffReport` 構造体が定義されている
- [ ] `compute_replay_comparison` 関数が実装されている
- [ ] `RECIPROCITY_EVENT_PROJECTION_NAME` 定数が `src/constants.rs` に追加されている
- [ ] R11-T1〜R11-T8 の全不変条件テストが PASS する
- [ ] R11-T9 観測テストが応答を出力する
- [ ] `cargo test` が既存テスト含めて全件 PASS

## Notes

- plan_path: context/0096-m176-11-reciprocity-event-ingestion-reputation-hazard-recompute-pipeline/plan.md（未作成、/plan-ticket 承認後に作成）
- implementation_path: context/0096-m176-11-reciprocity-event-ingestion-reputation-hazard-recompute-pipeline/implementation.md（未作成、/start-ticket 実装完了後に作成）
- review_report_path: context/0096-m176-11-reciprocity-event-ingestion-reputation-hazard-recompute-pipeline/review.md（未作成、/review-ticket 全チェック通過後に作成）
- observation_report_path: context/0096-m176-11-reciprocity-event-ingestion-reputation-hazard-recompute-pipeline/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成）

### 注意事項

- `DomainProjection::reciprocity_event()` は既に M1.5-R10 で実装済み（`src/event.rs:1473`）。本チケットでは新しい EventProjection 実装を作成せず、既存投影の出力先として ReciprocityEventStore を構築する。
- `GraphMetrics` は `src/event.rs` に定義する（ReputationProfile と同じモジュールに配置）。
- パイプラインの child protection 項（F-10）は `child_protections` マップ経由で外部から注入する。F-10 の内部計算は本チケットでは行わない。

### 成果物

- 計画: context/0096-m176-11-reciprocity-event-ingestion-reputation-hazard-recompute-pipeline/plan.md（未作成）
- 実装サマリ: context/0096-m176-11-reciprocity-event-ingestion-reputation-hazard-recompute-pipeline/implementation.md（未作成）
- レビュー報告書: context/0096-m176-11-reciprocity-event-ingestion-reputation-hazard-recompute-pipeline/review.md（未作成）
- 観察レポート: context/0096-m176-11-reciprocity-event-ingestion-reputation-hazard-recompute-pipeline/observation-YYYYMMDD-HHmmss.md（未作成）
