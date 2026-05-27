---
ticket_id: 123
title: M1.76-KW-MTR-C: Execution & Cost Metrics Backfill
slug: m176-kw-mtr-c-execution-cost-metrics-backfill
status: draft
created_at: 2026-05-27
updated_at: 2026-05-27
---

# M1.76-KW-MTR-C: Execution & Cost Metrics Backfill

## Summary

`collect_final_metrics`（SimulationContext 版）において、現在 0.0 でハードコードされている `execution_success_rate` と、デフォルト値 0.5 の `cost_efficiency` を実際に計算する。これにより s_search 因子が現在の 0.2535 から上昇する。

## Background

s_search = (j_cost + j_execution + j_search_radius_inv + j_reasoning_steps_inv) / 4 のうち、j_execution が 0.0（s_search を約 25% 押し下げ）、j_cost（cost_efficiency）が 0.5 の仮値（実測値ではない）。シミュレーション内で発生する実行操作（HELP セッション試行・GC 実行）の成否とコストを追跡することで、両指標を実測値で置き換える。

### Investigation

**execution_success_rate**（現在 0.0）:
- シミュレーションでは、HELP セッションの `decide_help_offer` や GC 処理などの「実行」操作が発生する。
- HELP セッションの結果は `HelpState` で追跡されている：`Pending`, `Offered`, `Accepted`, `Succeeded`, `Failed`, `Cancelled`。
- 成功 = `Succeeded` 状態に到達したセッション数 / 全開始セッション数。
- 現在 `ctx.help_sessions` から直接計算可能：`help_sessions.iter().filter(|s| s.current_state == HelpState::Succeeded).count() / help_sessions.len()`。
- より完全な定義のため、GC 実行の成功率も追加する。

**cost_efficiency**（現在 0.5）:
- 「コスト」の定義：シミュレーション内で消費されたリソース（GC で削除されたノード数、失敗した HELP 試行）。
- 「出力」の定義：シミュレーションの成果（生存人口、HELP 成功数、村形成率）。
- cost_efficiency = output / (cost + output) として [0,1] に正規化。
- コストと出力のカウンターを SimulationContext に追加する。

## Scope

1. **SimulationContext へのフィールド追加**（simulation.rs）:
   - `total_gc_collections: u64` — GC で収集されたノード数
   - `total_help_attempts: u64` — HELP 試行総数
   - `total_help_successes: u64` — HELP 成功総数

2. **シミュレーションフェーズでの更新**（simulation.rs）:
   - GC 実行時に `total_gc_collections` をインクリメント
   - HELP 試行時に `total_help_attempts` をインクリメント
   - HELP 成功時に `total_help_successes` をインクリメント

3. **kind_world.rs に関数追加**:
   - `compute_execution_success_rate(help_sessions: &[HelpSession]) -> f64` — HELP セッションの成功率
   - `compute_cost_efficiency(ctx: &SimulationContext) -> f64` — コストと出力の比から算出

4. **collect_final_metrics の更新**（kind_world.rs:2147）:
   - `execution_success_rate: 0.0` → `compute_execution_success_rate(&ctx.help_sessions)`
   - `cost_efficiency: 0.5` → `compute_cost_efficiency(ctx)`

## Non-scope

- 他の 8 指標（lifecycle, child_survival, freshness, benevolence, reciprocity, trust, capability, diffusion, reuse）は対象外
- HELP セッションの状態遷移ロジックの変更
- GC 処理の変更
- コストモデルの厳密な定義（プロキシ値で十分）

## Test Plan

| ID | テスト | アサーション |
|----|--------|-------------|
| C1 | compute_execution_success_rate 空 | 空セッションで 0.0 |
| C2 | compute_execution_success_rate 全成功 | 全 Succeeded で 1.0 |
| C3 | compute_execution_success_rate 混在 | 成功率 = Succeeded / total |
| C4 | compute_cost_efficiency 0 cost | cost=0 で 1.0 |
| C5 | compute_cost_efficiency high cost | cost 大で低値 |
| C6 | collect_final_metrics 2 指標改善 | execution_success_rate > 0.0, cost_efficiency ≠ 0.5 |
| C7 | 既存テスト全 PASS | regression |

## 計装方法・観測対象

collect_final_metrics の出力に execution/cost 関連の 2 指標を含める。HELP 成功率と GC 収集数の時系列変化を観測。cost_efficiency の定義（式）を変えた場合の s_search 感度を記録。

## Boy Scout Rule — 翻訳可能性計画

- `compute_execution_success_rate` は help_sessions から直接計算するシンプルな関数として実装
- コスト効率の計算式は named constants で閾値を定義（ハードコードしない）

## Acceptance Criteria

1. execution_success_rate が HELP セッション結果から正しく計算される
2. cost_efficiency がコストと出力の比から正しく計算される（0.5 の仮値ではなくなる）
3. SimulationContext に必要なカウンターフィールドが追加される
4. 既存テスト全 PASS
5. s_search が 0.2535 より大きくなる
