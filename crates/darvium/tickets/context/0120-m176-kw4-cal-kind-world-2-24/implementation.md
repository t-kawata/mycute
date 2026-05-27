# 変更したファイル一覧と実装内容の概要

## kind_world.rs (src/kind_world.rs)

### evaluate_single の SimulationContext 移行
- `evaluate_single` 関数を ReciprocitySimulator（`run_simulation`）→ SimulationContext（`run_evaluation_simulation`）に書き換え
- 引数は同一（`params: &MagnificentSevenParams, seed: u64 → f64`）
- 戻り値も同一（J_kw: f64）
- 決定論的再現性を維持（同一パラメータ + 同一シードで同一 J_kw）

### collect_final_metrics の公開
- `#[allow(dead_code)]` 属性を除去
- `pub(crate)` に変更（simulation.rs から呼び出せるよう）

### collect_final_metrics_from_result の deprecated 化
- `#[allow(dead_code)]` を追加（現在未使用）
- 古い ReciprocitySimulator 版として維持

### tc6_kw4_optimize_run の出力拡張
- 5-Factor Breakdown セクションを追加（s_growth, s_density, s_topology, s_search, s_fairness の内訳）
- 20 Subcomponents セクションを追加（全 20 下位成分の一覧）
- KindWorldAssessment による詳細評価出力

### テスト追加
- **TC9** (`tc9_kw4_evaluate_single_deterministic`): 同一パラメータ + 同一シードで J_kw が完全一致することを検証
- **TC10** (`tc10_kw4_simulation_context_metrics_nonzero`): 6 指標（j_nest_depth, j_node_density, j_clustering, j_local_density, j_search_radius_inv, j_reasoning_steps_inv）が 0.0 ではないことを検証

## simulation.rs (src/simulation.rs)

### run_evaluation_simulation 関数の追加
- ReciprocitySimulatorConfig を受け取り、SimulationContext を構築
- KW-REAL 6 フェーズ（init → growth → social → search → evaluation → cleanup）を実行
- `collect_final_metrics(&ctx, config.population_size)` を呼び出し、全 20 指標を計算
- `run_kw_real_simulation` と同様のシミュレーションループを持つ
- SimulationContext<'a> のライフタイム制約により共通化できないため独立した関数として実装

## constants.rs (src/constants.rs)

- KW4_NELDER_MEAD_CONVERGENCE_EPSILON: 1e-6 → 1e-10（外側サイクル 2）→ 1e-6（リセット）
- KW4_NELDER_MEAD_INITIAL_PERTURBATION: 0.05 → 0.10（外側サイクル 2）

## 実験記録

- `tickets/context/0120-m176-kw4-cal-kind-world-2-24/experiments.md` を作成
- 外側サイクル 1/24: evaluate_single 移行、J_kw=0.001522, 1 iter
- 外側サイクル 2/24: epsilon/perturbation 調整、J_kw=0.002095, 30 iter

## 重要な発見

SimulationContext 移行後も 12 の下位成分が `collect_final_metrics` 内でハードコード 0.0 であり、5 因子乗算結合モデルでは J_kw が 0.0 に張り付く。この問題はパラメータ調整では解決できず、下位成分の実装が完了して初めて意味のある較正が可能になる。
