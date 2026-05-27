---
ticket_id: 123
title: M1.76-KW-MTR-C: Execution & Cost Metrics Backfill
slug: m176-kw-mtr-c-execution-cost-metrics-backfill
status: reviewed
created_at: 2026-05-27
updated_at: 2026-05-27
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0123-m176-kw-mtr-c-execution-cost-metrics-backfill/plan.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0123-m176-kw-mtr-c-execution-cost-metrics-backfill/observation-20260527-134254.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0123-m176-kw-mtr-c-execution-cost-metrics-backfill/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0123-m176-kw-mtr-c-execution-cost-metrics-backfill/review.md
---

# M1.76-KW-MTR-C: Execution & Cost Metrics Backfill

## Summary

`collect_final_metrics`（SimulationContext 版, kind_world.rs:2264）において、現在 0.0 でハードコードされている `execution_success_rate` と、デフォルト値 0.5 の `cost_efficiency` を実際に計算する。これにより s_search 因子が上昇する。

## Background

s_search = (j_cost + j_execution + j_search_radius_inv + j_reasoning_steps_inv) / 4 のうち、j_execution が 0.0（s_search を約 25% 押し下げ）、j_cost（cost_efficiency）が 0.5 の仮値（実測値ではない）。シミュレーション内で発生する実行操作（HELP セッション試行・GC 実行）の成否とコストを追跡することで、両指標を実測値で置き換える。

### Investigation

**updated at plan time: 2026-05-27**

**execution_success_rate**（現在 0.0, kind_world.rs:2360）:
- 現在のコード: `execution_success_rate: 0.0` — 常に 0.0 のハードコード。
- j_execution（kind_world.rs:418）: `let j_execution = metrics.execution_success_rate.clamp(0.0, 1.0);`
- RFC §15.9.2（line 4484）定義: $j_{execution} = \min(\text{execution\_success\_rate}, 1.0)$ — 「成功 step / 全 step」
- RFC §15.9.3（line 4527）定義: `execution_success_rate`: 成功実行 step / 全実行 step 数。$s_{search}$ の $j_{execution}$ 成分の入力。

計算に使用可能なデータ:
- `ctx.help_sessions: Vec<HelpSession>`（simulation.rs:277）には HelpSession が格納されている。
- HelpState は 7 バリアント（help.rs:43-58）: Proposal, Offered, Accepted, Rejected, Executing, Succeeded, Failed
- HelpSession の `current_state` フィールドで各セッションの状態が追跡可能。
- **しかし**: phase3_help_protocol（simulation.rs:1787-1794）は毎 tick の終了時に終端状態セッション（Succeeded/Failed/Rejected/Accepted 含む）を `to_remove` ベクターで削除している。
  ```rust
  // simulation.rs:1787-1794
  HelpState::Succeeded
  | HelpState::Failed
  | HelpState::Rejected
  | HelpState::Accepted => {
      to_remove.push(i);
  }
  ```
  そのため collect_final_metrics 時点の ctx.help_sessions には未終端セッション（Proposal/Offered/Executing）のみが残る。終端セッションはカウントできない。
- **解決**: シミュレーションループ内で累積カウンター `ctx.total_help_attempts` と `ctx.total_help_successes` を使用する。
  - シミュレーションループ（simulation.rs:1458-1468）:
    ```rust
    let (proposals, new_successes) = phase3_help_protocol(...);
    let successes_count = new_successes.len();
    help_successes.extend(new_successes);
    // → ここで ctx.total_help_attempts += proposals; ctx.total_help_successes += successes_count;
    ```
- 関数シグネチャ:
  ```rust
  fn compute_execution_success_rate(total_attempts: u64, total_successes: u64) -> f64;
  ```
- 計算式: `if total_attempts == 0 { 0.0 } else { total_successes as f64 / total_attempts as f64 }`

**cost_efficiency**（現在 0.5, kind_world.rs:2337）:
- 現在のコード: `cost_efficiency: 0.5` — 常に 0.5 のデフォルト仮値。
- j_cost（kind_world.rs:417）: `let j_cost = metrics.cost_efficiency.clamp(0.0, 1.0);`
- RFC §15.9.2（line 4483）定義: $j_{cost} = \min(\text{cost\_efficiency}, 1.0)$ — cost_efficiency をそのまま正の向きで使用（高いほど効率的）
- RFC §15.9.3（line 4514）定義: `cost_efficiency`: 1.0 - (失敗セッション数 + 放棄セッション数) / 全セッション数。1.0 に近いほど効率的。

計算に使用可能なデータ:
- 既存の旧パス実装（kind_world.rs:782）: `compute_cost_efficiency(sessions: &[SimHelpSession]) -> f64` は SimHelpSession（旧パス）を使用しており、SimulationContext パスでは再利用不可。
- 新パスでは累積カウンターを使用する:
  - `total_gc_collections: u64` — phase4_gc_survival の戻り値 `gc_events`（死亡ノード数）を各 tick で加算
  - `total_help_attempts: u64` — phase3 で作成された全 HELP セッション数
  - `total_help_successes: u64` — phase3 で Succeeded に到達したセッション数
- phase4_gc_survival（simulation.rs:1810-1869）は gc_events（死亡数）を `usize` で返す。現在はローカル変数としてのみ保持（simulation.rs:1471-1482）:
  ```rust
  let gc_events = if tick % config.gc_interval == 0 {
      phase4_gc_survival(...)
  } else { 0 };
  ```
  → ctx.total_gc_collections に加算が必要。
- 関数シグネチャ:
  ```rust
  fn compute_cost_efficiency(
      total_gc_collections: u64,
      total_help_attempts: u64,
      total_help_successes: u64,
  ) -> f64;
  ```
- 計算式（RFC §15.9.3 を拡張）:
  ```
  cost = total_gc_collections + (total_help_attempts - total_help_successes)
  total = total_gc_collections + total_help_attempts
  cost_efficiency = if total == 0 { 1.0 } else { 1.0 - cost as f64 / total as f64 }
  ```
  - 全 HELP 成功 + GC 死亡 0 → 1.0（最大効率）
  - 全 HELP 失敗 + 全ノード死亡 → 0.0（最小効率）
  - アクティビティなし → 1.0（コストゼロ = 最大効率）

**legacy_flag_cost との関係**（kind_world.rs:377-378）:
```rust
let legacy_flag_cost =
    metrics.cost_efficiency <= crate::constants::KW_MAX_COST_EFFICIENCY_DECAY;
```
- KW_MAX_COST_EFFICIENCY_DECAY = 0.95（constants.rs:983）
- `cost_efficiency <= 0.95` が flag 通過条件。実測値が 0.95 以下の実用的な範囲にある限り問題ない。

**SimulationContext の既存フィールド（simulation.rs:263-292）**:
```rust
pub memoized_graph: &'a mut MemoizedGraph,
pub trust_profiles: HashMap<NodeId, TrustProfile>,
pub village_assignments: HashMap<NodeId, VillageAssignment>,
pub positions: HashMap<NodeId, SpacePositionEmbedding>,
pub tick: u64,
pub rng: StdRng,
pub help_sessions: Vec<HelpSession>,
pub use_gmr: bool,
pub node_gc_states: HashMap<NodeId, GcEvent>,           // MTR-A
pub node_last_update_tick: HashMap<NodeId, u64>,          // MTR-A
pub total_births: u64,                                    // MTR-A
pub total_inheritance_fidelity: f64,                      // MTR-B
pub inheritance_event_count: u64,                         // MTR-B
pub reciprocity_pair_counts: HashMap<(NodeId, NodeId), u64>, // MTR-B
```

**主要シミュレーションパス**: `run_evaluation_simulation`（simulation.rs:1394）→ `collect_final_metrics(&ctx, config.population_size, child_count)`（simulation.rs:1531）。MTR-A/MTR-B と同一パス。

**RFC §15.9.3 定義 vs 実装（updated at plan time: 2026-05-27）**:

RFC §15.9.3 は `execution_success_rate` を「成功実行 step / 全実行 step 数」、`cost_efficiency` を「1.0 - (失敗 + 放棄) / 全セッション数」と定義する。実装では HELP セッションの Succeeded/total を execution_success_rate に、HELP 失敗 + GC 死亡をコストとする cost_efficiency を使用する。HELP セッションが唯一の「実行」操作である現状のシミュレーションにおいて、これらの定義は RFC と無矛盾。GC 死亡の追加は RFC に定義されていないが、cost_efficiency の proxy として合理的な拡張であり、単独で歪んだ評価を生まない。

## Scope

1. **SimulationContext へのフィールド追加**（simulation.rs）:
   - `total_gc_collections: u64` — GC で収集（死亡）されたノード数
   - `total_help_attempts: u64` — HELP 試行総数
   - `total_help_successes: u64` — HELP 成功総数（Succeeded 到達数）
   - 初期化を `SimulationContext::new` に追加（全 0）

2. **シミュレーションループでのカウンター更新**（simulation.rs）:
   - phase3 戻り値 `proposals` を `ctx.total_help_attempts` に加算
   - phase3 戻り値 `successes_count` を `ctx.total_help_successes` に加算
   - phase4 戻り値 `gc_events` を `ctx.total_gc_collections` に加算

3. **kind_world.rs に関数追加**:
   - `compute_execution_success_rate(total_attempts: u64, total_successes: u64) -> f64` — HELP 成功率
   - `compute_cost_efficiency(total_gc_collections: u64, total_help_attempts: u64, total_help_successes: u64) -> f64` — コスト効率

4. **collect_final_metrics の更新**（kind_world.rs:2337, 2360）:
   - `cost_efficiency: 0.5` → `compute_cost_efficiency(ctx.total_gc_collections, ctx.total_help_attempts, ctx.total_help_successes)`
   - `execution_success_rate: 0.0` → `compute_execution_success_rate(ctx.total_help_attempts, ctx.total_help_successes)`

5. **テスト追加**:
   - C1–C5: ユニットテスト（境界値 + 典型値）
   - C6: collect_final_metrics 経由での non-default 確認（観測テスト）
   - C7: 既存テスト全 PASS（回帰）

## Non-scope

- 他の 8 指標（lifecycle, child_survival, freshness, benevolence, reciprocity, trust, capability, diffusion, reuse）は対象外
- HELP セッションの状態遷移ロジック（help.rs）の変更
- GC 処理（event.rs）の変更
- 旧パス `compute_cost_efficiency`（kind_world.rs:782）の削除（将来のクリーンアップ対象）
- コストモデルの厳密な定義（HELP 失敗 + GC 死亡のプロキシで十分）
- HelpSession `current_state` の終端削除ポリシーの変更

## Test Plan

| ID | テスト | アサーション |
|----|--------|-------------|
| C1 | compute_execution_success_rate 空 | total_attempts=0 で 0.0 |
| C2 | compute_execution_success_rate 全成功 | 全成功で 1.0 |
| C3 | compute_execution_success_rate 部分成功 | 5/10 = 0.5 |
| C4 | compute_cost_efficiency 0 cost | cost=0 で 1.0 (gc=0, attempts=5, successes=5) |
| C5 | compute_cost_efficiency high cost | cost 大で低値 (gc=10, attempts=10, successes=0 → 0.0) |
| C6 | collect_final_metrics 2 指標改善 | execution_success_rate > 0.0, cost_efficiency ≠ 0.5 |
| C7 | 既存テスト全 PASS | regression |

## 計装方法・観測対象

collect_final_metrics の出力に execution/cost 関連の 2 指標（execution_success_rate, cost_efficiency）を含め、HELP 成功率と GC 収集数の時系列変化を観測。cost_efficiency の定義（GC cost の加重）を変えた場合の s_search 感度を記録。

## Boy Scout Rule — 翻訳可能性計画

- `compute_execution_success_rate` と `compute_cost_efficiency` は引数が異なる独立関数として実装（混同防止）
- GC 死亡カウントの累積コードは phase4 戻り値の加算として、HELP 累積と並べて配置

## Acceptance Criteria

1. execution_success_rate が HELP セッション結果から正しく計算される（0.0 ではなくなる）
2. cost_efficiency が HELP 失敗数 + GC 死亡数から正しく計算される（0.5 の仮値ではなくなる）
3. SimulationContext に 3 フィールド（total_gc_collections, total_help_attempts, total_help_successes）が追加され、初期化される
4. 既存テスト全 PASS
5. s_search が 0.2535 より大きくなる（j_execution が 0.0 でなくなり、j_cost が 0.5 から実測値に変わる）
