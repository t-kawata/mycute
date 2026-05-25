# M1.75-8: deterministic replay implementation summary

## 変更ファイル一覧

| ファイル | 種別 | 内容 |
|----------|------|------|
| src/constants.rs | 編集 | REPLAY_POSITION_DELTA_SIGMA 定数追加 |
| src/replay.rs | 新規 | リプレイシナリオエンジン (710行) |
| src/lib.rs | 編集 | モジュール宣言 + 公開API追加 |

## 型定義 (src/replay.rs)

- VillageReplayScenario: seed / workflows / missions / clock_schedule / policy_bundle
- WorkflowConfig: id / initial_position / initial_experience / initial_trust / initial_reputation
- MissionSpec: trigger_tick / description
- ClockSchedule: total_ticks
- PolicyBundle: offer_policy / accept_policy / selection_policy
- ReplayTrace: space_positions / villages / helper_weights / help_sessions / child_growth_events
- SummaryMetrics: churn_p50/p95 / jsd_p50/p95 / survival_rate / maturation_rate / helper_count_mean / total_sessions

## 公開関数

- run_replay_scenario(scenario) -> ReplayTrace — 全 tick 実行
- trace_eq(left, right) -> bool — f32::EPSILON 許容誤差付き完全一致比較
- trace_diff_fields(left, right) -> Vec<String> — 差分フィールド名リスト
- trace_summary_metrics(trace) -> SummaryMetrics — 要約統計量

## テスト結果

- 11 replay tests (T-1〜T-9, T-O1, T-O2) — 全 PASS
- 880 total tests (既存含む) — 全 PASS, 警告 0
- T-O1: 18 条件のメトリクスグリッド掃引
- T-O2: n=100 の決定論的再現性確認 (100%)
