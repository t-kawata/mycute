---
ticket_id: 121
title: M1.76-KW-MTR-A: Lifecycle & Freshness Metrics Backfill
slug: m176-kw-mtr-a-lifecycle-freshness-metrics-backfill
status: draft
created_at: 2026-05-27
updated_at: 2026-05-27
---

# M1.76-KW-MTR-A: Lifecycle & Freshness Metrics Backfill

## Summary

`collect_final_metrics`（SimulationContext 版, kind_world.rs:2147）において、現在 0.0 でハードコードされている 3 指標（`mean_lifecycle_score`, `child_survival_rate`, `mean_freshness`）を実際に計算する。これにより s_growth 因子が 0.0 から脱却する。

## Background

現在の J_kw ≈ 0.002 の主要因は s_growth = 0.0 である。s_growth = (j_pop_growth + j_lifecycle + j_child_survival + j_freshness) / 4 のうち、後 3 項が 0.0 に張り付いている。しかし、これらの計算に必要なデータは SimulationContext 内に既に存在するか、シミュレーションフェーズに軽微なカウンター追加で取得可能である。

### Investigation

**mean_lifecycle_score**（現在 0.0）:
- シミュレーションは各ノードの GC 状態を `node_gc_states: HashMap<NodeId, GcEvent>` で管理している（simulation.rs:1197）。
- GcEvent は `Protected`, `Active`, `SoftDeleted`, `HardDeleteCandidate`, `Tombstoned` の 5 バリアント（event.rs:312）。
- 各バリアントにライフサイクルスコアを割り当て（例: Protected=1.0, Active=0.8, SoftDeleted=0.3, HardDeleteCandidate=0.1, Tombstoned=0.0）、全ノードの平均を取る。
- ただし `node_gc_states` は SimulationContext のフィールドではなく、シミュレーション関数のローカル変数。collect_final_metrics からアクセスするには、SimulationContext に移すか、別途集計値を保持する必要がある。

**child_survival_rate**（現在 0.0）:
- `phase1_population_growth`（simulation.rs:1510）は子ノード出生数を返す (`births: usize`)。
- 生存数は simulation 終了時に `is_adult` マップで `false` のノード数から計算できる。
- シミュレーションループの戻り値として出生数と生存数を伝播する仕組みが必要。

**mean_freshness**（現在 0.0）:
- SimulationContext は既に `tick: u64` フィールドを持つ（simulation.rs:273）。
- 各ノードの最終更新 tick を追跡する仕組みが別途必要。ノード作成時・更新時に tick を記録するカウンターを SimulationContext に追加する。

**SimulationContext の既存フィールド（simulation.rs:263-280）**:
```rust
pub memoized_graph: &'a mut MemoizedGraph,
pub trust_profiles: HashMap<NodeId, TrustProfile>,
pub village_assignments: HashMap<NodeId, VillageAssignment>,
pub positions: HashMap<NodeId, SpacePositionEmbedding>,
pub tick: u64,           // ← 既に存在。freshness 計算に使用可能
pub rng: StdRng,
pub help_sessions: Vec<HelpSession>,
pub use_gmr: bool,
```

## Scope

1. **SimulationContext へのフィールド追加**（simulation.rs）:
   - `node_gc_states: HashMap<NodeId, GcEvent>` — 各ノードの GC 状態（現在ローカル変数）
   - `node_last_update_tick: HashMap<NodeId, u64>` — 各ノードの最終更新 tick
   - `total_births: u64` — 累計出生数
   - 初期化を `SimulationContext::new` および初期化ループに追加

2. **シミュレーションフェーズでの更新**（simulation.rs）:
   - `phase1_population_growth` 内で `total_births` をインクリメント
   - GC 状態遷移後に `node_gc_states` を更新
   - ノード作成・更新時に `node_last_update_tick` を更新（`ctx.tick` を使用）

3. **kind_world.rs に関数追加**:
   - `lifecycle_score_from_gc_state(state: GcEvent) -> f64` — GcEvent をスコアにマッピング
   - `compute_mean_lifecycle_score(node_gc_states: &HashMap<NodeId, GcEvent>) -> f64`
   - `compute_child_survival_rate(total_births: u64, child_count: u64) -> f64`
   - `compute_mean_freshness(node_last_update: &HashMap<NodeId, u64>, current_tick: u64) -> f64`

4. **collect_final_metrics の更新**（kind_world.rs:2147）:
   - `mean_lifecycle_score: 0.0` → `compute_mean_lifecycle_score(&ctx.node_gc_states)`
   - `child_survival_rate: 0.0` → `compute_child_survival_rate(ctx.total_births, child_count)`
   - `mean_freshness: 0.0` → `compute_mean_freshness(&ctx.node_last_update_tick, ctx.tick)`

## Non-scope

- 他の 8 指標（benevolence, reciprocity, trust, execution, cost, capability, diffusion, reuse）は対象外
- GcEvent のバリアント追加・変更
- SimulationContext 以外のデータ構造への変更
- 6 フェーズシミュレーションループのロジック変更（カウンター追加のみ）

## Test Plan

| ID | テスト | アサーション |
|----|--------|-------------|
| A1 | lifecycle_score_from_gc_state 全網羅 | Protected=1.0, Active=0.8, SoftDeleted=0.3, HardDeleteCandidate=0.1, Tombstoned=0.0 |
| A2 | compute_mean_lifecycle_score 空 | 空マップで 0.0 |
| A3 | compute_child_survival_rate 0 出生 | total_births=0 で 0.0 |
| A4 | compute_child_survival_rate 全生存 | total_births=5, child_count=5 で 1.0 |
| A5 | compute_mean_freshness [0,1] | 全ノード同一 tick で 1.0, 更新なしで低値 |
| A6 | collect_final_metrics 3 指標非ゼロ | シミュレーション実行後に all > 0.0 |
| A7 | 既存テスト全 PASS | regression |

## 計装方法・観測対象

collect_final_metrics の出力に lifecycle/freshness 関連の 3 指標を含める。既存の KW4 TC6 出力（`--nocapture`）で s_growth の内訳として 3 指標の値が正しく表示されることを確認。シミュレーションの phase1 出生数とノード生存率の時系列変化を観測。

## Boy Scout Rule — 翻訳可能性計画

- GcEvent → スコアのマッピングは exhaustive match で記述（`_ =>` 不使用）
- `collect_final_metrics` 内の lifecycle 関連計算ブロックを名前付き関数に抽出

## Acceptance Criteria

1. mean_lifecycle_score が GcEvent 分布から正しく計算される
2. child_survival_rate が出生数と生存数から正しく計算される
3. mean_freshness がノード更新 tick から正しく計算される
4. 上記 3 指標が 0.0 以外の値を取る
5. SimulationContext に必要なフィールドが追加される
6. 既存テスト全 PASS
7. s_growth が 0.0 より大きくなる
