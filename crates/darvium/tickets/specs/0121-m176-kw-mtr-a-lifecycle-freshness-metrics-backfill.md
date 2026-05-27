---
ticket_id: 121
title: M1.76-KW-MTR-A: Lifecycle & Freshness Metrics Backfill
slug: m176-kw-mtr-a-lifecycle-freshness-metrics-backfill
status: reviewed
created_at: 2026-05-27
updated_at: 2026-05-27
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0121-m176-kw-mtr-a-lifecycle-freshness-metrics-backfill/plan.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0121-m176-kw-mtr-a-lifecycle-freshness-metrics-backfill/implementation.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0121-m176-kw-mtr-a-lifecycle-freshness-metrics-backfill/observation-20260527-120529.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0121-m176-kw-mtr-a-lifecycle-freshness-metrics-backfill/review.md
---

# M1.76-KW-MTR-A: Lifecycle & Freshness Metrics Backfill

## Summary

`collect_final_metrics`（SimulationContext 版, kind_world.rs:2147）において、現在 0.0 でハードコードされている 3 指標（`mean_lifecycle_score`, `child_survival_rate`, `mean_freshness`）を実際に計算する。これにより s_growth 因子が 0.0 から脱却する。

## Background

現在の J_kw ≈ 0.002 の主要因は s_growth = 0.0 である。s_growth = (j_pop_growth + j_lifecycle + j_child_survival + j_freshness) / 4 のうち、後 3 項が 0.0 に張り付いている。しかし、これらの計算に必要なデータは SimulationContext 内に既に存在するか、シミュレーションフェーズに軽微なカウンター追加で取得可能である。

### Investigation

**参照観察レポート**

- `tickets/context/0120-m176-kw4-cal-kind-world-2-24/observation-20260527-095403.md` — KW4 較正実験。s_growth=0.0 が J_kw≈0.002 の主要因と確認。collect_final_metrics の 3 lifecycle 指標が全て 0.0 であることが直接の原因。
- `tickets/context/0119-m176-kw-accel-j-kw-57/observation-20260527-091630.md` — J_kw 5因子乗算モデル導入。s_growth 因子が 0.0 のままであることが確認済み。

**updated at plan time: 2026-05-27**

**mean_lifecycle_score**（現在 0.0, `kind_world.rs:2233`）:
- シミュレーションは各ノードの GC 状態を `node_gc_states: HashMap<NodeId, GcEvent>` で管理している。
  - `run_evaluation_simulation`（`simulation.rs:1390`）でローカル変数として初期化
  - `phase4_gc_survival`（`simulation.rs:1777`）で状態遷移が行われる
- GcEvent は `Protected`, `Active`, `SoftDeleted`, `HardDeleteCandidate`, `Tombstoned` の 5 バリアント（`event.rs:284`）。
- `transition_gc_state`（`event.rs:312`）が hazard 値に基づいて状態遷移を実行。
- 各バリアントにライフサイクルスコアを割り当て（例: Protected=1.0, Active=0.8, SoftDeleted=0.3, HardDeleteCandidate=0.1, Tombstoned=0.0）、全ノードの平均を取る。
- **ただし**: `node_gc_states` は SimulationContext のフィールドではない。`collect_final_metrics`（`kind_world.rs:2147`）には ctx のみ渡されるため、SimulationContext に昇格が必要。
- `kind_world.rs` は現在 `event::GcEvent` をインポートしていない。新規関数で `use crate::event::GcEvent` が必要。

**child_survival_rate**（現在 0.0, `kind_world.rs:2234`）:
- `phase1_population_growth`（`simulation.rs:1510`）は子ノード出生数を返す (`births: usize`)。
- シミュレーションループ（`simulation.rs:1423`）で `let births = phase1_population_growth(...)` として取得されるが、**累積されていない**。
- 生存数 = シミュレーション終了時に `is_adult` マップで `false` かつ `dead` に含まれないノード数。
- `is_adult`（`simulation.rs:1387`）と `dead`（`simulation.rs:1386`）は両方ともローカル変数。
- `ctx.add_person()`（`simulation.rs:339`）で子ノードが作成されるが、出生数の累積カウンターは未実装。
- **出生数と生存数の収集方針**: `ctx.total_births` で累積出生数を追跡。子生存数は `collect_final_metrics` に渡す方法が必要。現在 `collect_final_metrics` は `(ctx, initial_population_size)` のみ受取。生存子供を計算するには `is_adult` 情報が必要。

**mean_freshness**（現在 0.0, `kind_world.rs:2235`）:
- SimulationContext は既に `tick: u64` フィールドを持つ（`simulation.rs:273`）。
- 各ノードの最終更新 tick を追跡する `node_last_update_tick: HashMap<NodeId, u64>` を SimulationContext に追加する。
- `compute_blended_freshness`（`clock.rs:155`）は既存で、per-node freshness を計算可能: `compute_blended_freshness(0, current_tick - last_update_tick, 0.0)`（human_weight=0 で仮想時刻のみ使用）。
- ノード作成時（`add_person` at `simulation.rs:339`）と GC 状態遷移時（`phase4_gc_survival` at `simulation.rs:1770`）に `ctx.node_last_update_tick.insert(id, ctx.tick)` を呼ぶ。
- `mean_freshness` = 全ノードの `compute_blended_freshness(0, ctx.tick - last_tick, 0.0)` の平均。

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

**主要シミュレーションパス**: `run_evaluation_simulation`（`simulation.rs:1372`）が `collect_final_metrics(&ctx, config.population_size)`（`simulation.rs:1506`）を呼ぶ。このパスでのみ変更が必要。旧パス `run_kw_real_simulation`（`simulation.rs:1181`）と `collect_final_metrics_from_result`（`kind_world.rs:2069`）はスコープ外。

**RFC §15.9.3 定義 vs 実装（updated at plan time: 2026-05-27）**:

RFC §15.9.3（line 4520）は `mean_lifecycle_score` を「全個人の LifecycleScore $L(G)$ の算術平均」と定義。$L(G)$ は freshness/success/trust/usage/reputation の幾何平均（line 4284, `lifecycle.rs:40`）。しかし SimulationContext は reputation（MTR-B scope）と experience（MTR-D scope）を保持しておらず、本チケット単独では RFC 完全準拠の $L(G)$ 平均を計算できない。

本チケットでは **GcEvent 状態分布** を下流指標として使用する（GcEvent は L(G) 由来の hazard 計算結果の状態であり強い相関を持つ）。GcEvent スコアリングは RFC 定義の近似値だが、実用的かつ自己完結的。真の $L(G)$ 平均への置き換えは MTR-B/MTR-D 完了後の後続チケットが対応する必要がある（現時点では未計画）。

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
