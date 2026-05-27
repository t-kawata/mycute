---
ticket_id: 122
title: M1.76-KW-MTR-B: Trust & Reciprocity Metrics Backfill
slug: m176-kw-mtr-b-trust-reciprocity-metrics-backfill
status: draft
created_at: 2026-05-27
updated_at: 2026-05-27
---

# M1.76-KW-MTR-B: Trust & Reciprocity Metrics Backfill

## Summary

`collect_final_metrics`（SimulationContext 版）において、現在 0.0 でハードコードされている 3 指標（`mean_benevolence_aggregate`, `mean_reciprocity_score`, `trust_inheritance_fidelity`）を実際に計算する。これにより s_topology 因子が現在の 0.225 から上昇する。

## Background

s_topology = (j_benevolence + j_reciprocity + j_help + j_trust + j_clustering + j_local_density) / 6 のうち、j_benevolence, j_reciprocity, j_trust の 3 指標が 0.0 のままである。しかし SimulationContext は既に `trust_profiles: HashMap<NodeId, TrustProfile>` を保持しており（simulation.rs:268）、HELP セッション履歴も `help_sessions: Vec<HelpSession>` で利用可能である。これらのデータから 3 指標を算出できる。

### Investigation

**mean_benevolence_aggregate**（現在 0.0）:
- SimulationContext は `trust_profiles: HashMap<NodeId, TrustProfile>` を持つ。
- TrustProfile（trust.rs）は `operational`, `semantic`, `temporal` の 3 次元信頼値を持つ。
- benevolence のプロキシとして、3 次元信頼値の平均を全ノードで平均する。
- 既存の `compute_blended_trust` 相当の集計処理で算出可能。

**mean_reciprocity_score**（現在 0.0）:
- HELP セッション履歴（`help_sessions: Vec<HelpSession>`）から、セッションごとの相互性を計算。
- 各 HELP セッションは `from_workflow`（依頼者）と `to_workflow`（提供者）を持つ。
- 同一ペア間の HELP 提供回数のバランスから reciprocity を計算。
- 直接互恵性（同一ペア間の往復）と間接互恵性（ネットワーク全体のバランス）の加重平均。

**trust_inheritance_fidelity**（現在 0.0）:
- `phase1_population_growth`（simulation.rs:1530-1535）で信頼継承が発生している：
  ```rust
  inherit_trust(pt, ct, TRUST_INHERIT_DECAY);
  ```
- `inherit_trust`（trust.rs）は減衰係数 `TRUST_INHERIT_DECAY` を適用する。
- 継承 fidelity = 継承後の子ノード信頼値 / 継承前の親ノード信頼値（平均）。
- 継承イベントの都度 fidelity を記録するカウンターを SimulationContext に追加。

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
