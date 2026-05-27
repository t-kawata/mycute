---
ticket_id: 122
title: M1.76-KW-MTR-B: Trust & Reciprocity Metrics Backfill
slug: m176-kw-mtr-b-trust-reciprocity-metrics-backfill
status: reviewed
created_at: 2026-05-27
updated_at: 2026-05-27
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0122-m176-kw-mtr-b-trust-reciprocity-metrics-backfill/plan.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0122-m176-kw-mtr-b-trust-reciprocity-metrics-backfill/observation-20260527-132953.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0122-m176-kw-mtr-b-trust-reciprocity-metrics-backfill/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0122-m176-kw-mtr-b-trust-reciprocity-metrics-backfill/review.md
---

# M1.76-KW-MTR-B: Trust & Reciprocity Metrics Backfill

## Summary

`collect_final_metrics`（SimulationContext 版、kind_world.rs:2210）において、現在 0.0 でハードコードされている 3 指標（`mean_benevolence_aggregate`, `mean_reciprocity_score`, `trust_inheritance_fidelity`）を実際に計算する。これにより s_topology 因子が現在の 0.225 から上昇する。

## Background

s_topology = (j_benevolence + j_reciprocity + j_help + j_trust + j_clustering + j_local_density) / 6 のうち、j_benevolence, j_reciprocity, j_trust の 3 指標が 0.0 のままである。しかし SimulationContext は既に `trust_profiles: HashMap<NodeId, TrustProfile>` を保持しており（simulation.rs:267）、HELP セッション履歴も `help_sessions: Vec<HelpSession>` で利用可能である（simulation.rs:277）。これらのデータから 3 指標を算出できる。

### Investigation

**mean_benevolence_aggregate**（現在 0.0, kind_world.rs:2300）:
- SimulationContext（simulation.rs:267）は `trust_profiles: HashMap<NodeId, TrustProfile>` を持つ。
- TrustProfile（types.rs:4807）は `operational`, `semantic`, `temporal` の 3 次元信頼値（各 `f64`, [0,1]範囲）を持つ。
- `add_person`（simulation.rs:357-366）で全ノードに TrustProfile がデフォルト値 (0.5, 0.5, 0.5) で初期化される。
- `run_evaluation_simulation`（simulation.rs:1414-1424）で子は (0.3, 0.3, 0.3)、成人は (0.6, 0.6, 0.6) に設定。
- benevolence のプロキシとして、各ノードの 3 次元信頼値の算術平均を全ノードで平均する。
- 関数シグネチャ: `compute_mean_benevolence(trust_profiles: &HashMap<NodeId, TrustProfile>) -> f64`

**mean_reciprocity_score**（現在 0.0, kind_world.rs:2301）:
- HELP セッション履歴（`ctx.help_sessions: Vec<HelpSession>`）から、ペア (from_workflow, to_workflow) の出現頻度をカウント。
- HelpSession（help.rs:247）の構造:
  ```rust
  pub struct HelpSession {
      pub help_id: String,
      pub from_workflow: WorkflowGraphId,  // helper
      pub to_workflow: WorkflowGraphId,    // helpee
      pub current_state: HelpState,
  }
  ```
- `phase3_help_protocol`（simulation.rs:1686）で `HelpSession::new(help_id, nid_str(helper_id), nid_str(child_id))` としてセッション作成。
- `from_workflow` / `to_workflow` は `NodeId` 型ではない（`String` の WorkflowGraphId）。そこで、新規フィールド `reciprocity_pair_counts: HashMap<(NodeId, NodeId), u64>` を SimulationContext に追加し、HELP セッション作成時にインクリメントする。
- 同一ペア (helper, child) 間の計数が対称（helper→child と child→helper が近い）か非対称かを評価する。
- 関数シグネチャ: `compute_mean_reciprocity(pair_counts: &HashMap<(NodeId, NodeId), u64>) -> f64`
- **ただし**: help_sessions の `from_workflow` / `to_workflow` は `WorkflowGraphId`（String）であり、NodeId ではない。simulation.rs の `nid_str` / `nid_from_str` で変換可能。ペアカウンティングはシミュレーションループ内（phase3 セッション作成時）で `ctx.reciprocity_pair_counts.entry((helper_id, child_id)).or_insert(0) += 1` として行う。

**trust_inheritance_fidelity**（現在 0.0, kind_world.rs:2302）:
- `phase1_population_growth`（simulation.rs:1546-1552）で信頼継承が発生：
  ```rust
  let parent_trust = ctx.trust_profiles.get(&parent_id).cloned();
  if let Some(ref pt) = parent_trust {
      if let Some(ct) = ctx.trust_profiles.get_mut(&child_id) {
          inherit_trust(pt, ct, TRUST_INHERIT_DECAY);  // TRUST_INHERIT_DECAY = 0.70
      }
  }
  ```
- `inherit_trust`（trust.rs:153-157）は以下の減衰継承を行う：
  ```rust
  child.operational = parent.operational * decay;
  child.semantic = parent.semantic * decay;
  child.temporal = parent.temporal * decay;
  ```
- 継承 fidelity の定義: 各次元について `fidelity = 1.0 - |(parent * decay) - child|` の平均。現在の実装では child = parent * decay が厳密に成立するため、各イベントの fidelity = 1.0。
- カウンター方式: `ctx.total_inheritance_fidelity` に各イベントの fidelity を加算、`ctx.inheritance_event_count` でイベント数をカウント。終了時に平均 fidelity = total / count。
- 関数シグネチャ: `compute_trust_inheritance_fidelity(total_fidelity: f64, event_count: u64) -> f64`

**collect_final_metrics の現在の該当行（kind_world.rs:2300-2302）**:
```rust
mean_benevolence_aggregate: 0.0,
mean_reciprocity_score: 0.0,
trust_inheritance_fidelity: 0.0,
```

**SimulationContext の既存フィールド（simulation.rs:263-286）**:
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
```

**主要シミュレーションパス**: `run_evaluation_simulation`（simulation.rs:1385）→ `collect_final_metrics(&ctx, config.population_size, child_count)`（simulation.rs:1522）。MTR-A と同一パス。

**RFC §15.9.3 定義 vs 実装（updated at plan time: 2026-05-27）**:

RFC §15.9.3（line 4520）は `mean_benevolence_aggregate` を未定義（15.9.3 の 11 指標中に含まれない）。これは J_kw 5 因子乗算モデル（RFC §15.9.2）の s_topology 因子の下位成分として KW-ACCEL で追加された指標であり、TrustProfile の 3 次元信頼値の平均を使用する。TrustProfile は RFC §10 の 4 軸信頼モデル（operational/semantic/temporal/human）に基づく実装であり、この平均値を benevolence 指標とすることは RFC に矛盾しない。

RFC §15.9.3 は `mean_reciprocity_score` と `trust_inheritance_fidelity` を未定義。これらも KW-ACCEL で追加された s_topology 下位成分である。reciprocity は HELP ペアバランス（RFC §6 HelpProtocol の健全性指標）、trust_inheritance_fidelity は inherit_trust（RFC §10.2）の継承精度指標として、いずれも RFC の理論体系と無矛盾。

## Scope

1. **SimulationContext へのフィールド追加**（simulation.rs）:
   - `total_inheritance_fidelity: f64` — 継承 fidelity の累積和
   - `inheritance_event_count: u64` — 継承イベント発生回数
   - `reciprocity_pair_counts: HashMap<(NodeId, NodeId), u64>` — ペア別 HELP 提供回数

2. **シミュレーションフェーズでの更新**（simulation.rs）:
   - `phase1_population_growth` 内で `inherit_trust` 呼び出し後に fidelity を記録
   - HELP セッション作成時に `reciprocity_pair_counts` を更新

3. **kind_world.rs に関数追加**:
   - `compute_mean_benevolence(trust_profiles: &HashMap<NodeId, TrustProfile>) -> f64` — 全 TrustProfile の blended trust 平均
   - `compute_mean_reciprocity(pair_counts: &HashMap<(NodeId, NodeId), u64>) -> f64` — HELP ペアバランスから reciprocity を算出
   - `compute_trust_inheritance_fidelity(total_fidelity: f64, event_count: u64) -> f64` — 継承 fidelity の平均

4. **collect_final_metrics の更新**（kind_world.rs:2147）:
   - `mean_benevolence_aggregate: 0.0` → `compute_mean_benevolence(&ctx.trust_profiles)`
   - `mean_reciprocity_score: 0.0` → `compute_mean_reciprocity(&ctx.reciprocity_pair_counts)`
   - `trust_inheritance_fidelity: 0.0` → `compute_trust_inheritance_fidelity(ctx.total_inheritance_fidelity, ctx.inheritance_event_count)`

## Non-scope

- 他の 8 指標（lifecycle, child_survival, freshness, execution, cost, capability, diffusion, reuse）は対象外
- TrustProfile のフィールド追加・変更
- ReciprocityMatrix の修正（reciprocity.rs は修正しない）
- 6 フェーズシミュレーションループのロジック変更（カウンター追加のみ）

## Test Plan

| ID | テスト | アサーション |
|----|--------|-------------|
| B1 | compute_mean_benevolence 空 | 空マップで 0.0 |
| B2 | compute_mean_benevolence 全 1.0 | 全 TrustProfile が (1.0,1.0,1.0) で 1.0 |
| B3 | compute_mean_reciprocity 対称 | 全ペアが等回数 HELP 提供で 1.0 |
| B4 | compute_mean_reciprocity 非対称 | 一方向のみの HELP で低値 |
| B5 | compute_trust_inheritance_fidelity 0 イベント | event_count=0 で 0.0 |
| B6 | collect_final_metrics 3 指標非ゼロ | シミュレーション実行後に all > 0.0 |
| B7 | 既存テスト全 PASS | regression |

## 計装方法・観測対象

collect_final_metrics の出力に trust/reciprocity 関連の 3 指標を含める。help_sessions のペアバランス分布（偏り度合い）を観測。TRUST_INHERIT_DECAY 定数の変更が繼承 fidelity に与える影響を観察。

## Boy Scout Rule — 翻訳可能性計画

- HELP セッションからの reciprocity 集計を名前付き関数に抽出
- 継承 fidelity の記録を phase1_population_growth 内で明確な名前付き変数に束縛

## Acceptance Criteria

1. mean_benevolence_aggregate が TrustProfile 分布から正しく計算される
2. mean_reciprocity_score が HELP ペアバランスから正しく計算される
3. trust_inheritance_fidelity が継承イベントから正しく計算される
4. 上記 3 指標が 0.0 以外の値を取る
5. SimulationContext に必要なフィールドが追加される
6. 既存テスト全 PASS
7. s_topology が 0.225 より大きくなる
