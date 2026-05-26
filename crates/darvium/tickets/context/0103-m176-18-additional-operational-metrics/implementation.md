# 変更したファイル一覧と実装内容の概要

## 変更ファイル

### `src/constants.rs`
- `BENEVOLENT_TOP_FRACTION: f64 = 0.2` — 善良群上位 20% 閾値（Safety Invariant）
- `BENEVOLENT_BOTTOM_FRACTION: f64 = 0.2` — 非善良群下位 20% 閾値（Safety Invariant）

### `src/simulation.rs`

**新規構造体（約 line 195）:**
- `ExtendedOperationalMetrics` — RFC §41B.20.7 の 11 指標 + tick を保持する観測用構造体

**新規公開関数（Phase 6: Operational Metrics）:**
1. `compute_benevolent_survival_advantage(population: &[SimWorkflowState]) -> f64` — 上位/下位 20% 生存率差
2. `compute_harmful_gc_rate(sessions: &[SimHelpSession]) -> f64` — HarmfulMismatch / 全セッション
3. `compute_helper_accept_rate(sessions: &[SimHelpSession]) -> f64` — Accepted / (Accepted + Rejected)
4. `compute_help_abandon_rate(sessions: &[SimHelpSession]) -> f64` — Abandoned / (Succeeded + HarmfulMismatch + Abandoned)
5. `compute_child_survival_rate(population: &[SimWorkflowState]) -> f64` — 生存子 WF 数 / 子 WF 総数

**新規 observer:**
- `ReciprocityMetricsObserver` — シミュレーター TickObserver 互換の stateless observer
  - `observe()`: SimulationTickSnapshot + sessions + population → ExtendedOperationalMetrics
  - `print_csv()`: 時系列拡張 CSV 出力（14 列、tick + 13 metrics）

**新規テスト（10 件、T10〜T19）:**
- T10: benevolent_survival_advantage 全同一 → 0.0
- T11: harmful_gc_rate 零 → 0.0
- T12: helper_accept_rate 境界値（全受理/全拒否/空/混合）
- T13: help_abandon_rate 境界値（全成功/全放棄/空/混合）
- T14: child_survival_rate 境界値（成人のみ/全生存/全死亡/混合）
- T15: 空データ graceful ハンドリング（全 5 関数が 0.0 かつ有限）
- T16: ReciprocityMetricsObserver 統合（全指標有限）
- T17: 上位/下位 20% 分割（完全分離/小人口）
- T18: 拡張 CSV 出力形式（20 行、14 列）
- T19: 後方互換性（全既存テスト PASS）

## 検証結果
- `cargo check`: 成功（deprecation warnings のみ、既存）
- `cargo test`: 1080 passed, 0 failed（既存 1053 + 新規 10 + 他クレート 17）
- 観測レポート: tickets/context/0103-m176-18-additional-operational-metrics/observation-20260526-123144.md
