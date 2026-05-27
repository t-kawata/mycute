# 変更したファイル一覧と実装内容の概要

## src/constants.rs
- `KW4_OBSERVATION_INTERVAL: u64 = 10` 追加 — mid-simulation サンプリング間隔
- `KW4_CONVERGENCE_THRESHOLD: f64 = 0.8` 追加 — s_growth × j_cov 収束閾値

## src/simulation.rs
- `run_evaluation_simulation` の戻り値を `(KindWorldMetricsInput, u64)` に変更（tick_to_convergence 付き）
- KW4_OBSERVATION_INTERVAL（10）tick ごとに mid-simulation サンプリングを追加:
  - j_pop_growth, j_lifecycle, j_child_survival, j_freshness, j_cov を計算
  - s_growth = 4 成分平均から算出
  - s_growth × j_cov > KW4_CONVERGENCE_THRESHOLD で収束判定
  - 収束した最初の tick を tick_to_convergence として記録
- 収束しなかった場合は tick_to_convergence = KW4_SIMULATION_TICKS（100）
- 既存の collect_final_metrics 呼び出しはそのまま維持

## src/kind_world.rs
- `compute_mean_lifecycle_score`, `compute_mean_freshness`, `compute_capability_coverage` を `pub(crate)` に変更（simulation.rs から呼び出し可能に）
- `compute_s_speed` 関数を追加 — tick_to_convergence から速度因子を計算
- `evaluate_single`: J_kw（品質）→ J_kw_social（品質 × 速度）に変更
- `OptimizationReport`: `best_j_kw_social`, `tick_to_convergence`, `s_speed` フィールドを追加。`best_j_kw` は互換性のため残し `#[deprecated]` 注釈
- `NelderMeadOptimizer::run`: 最終レポート生成時に tick_to_convergence, s_speed, best_j_kw_social を格納。is_kind_world 判定を J_kw_social > 0.64 に更新
- TC1e〜TC8e テスト追加（ttc 範囲、s_speed 範囲、決定論性、JSON フィールド確認、J_kw vs J_kw_social 比較）
- tc6_kw4_optimize_run 出力を拡張（CSV に J_kw_social/s_speed/ttc 列追加、JSON 出力更新）
- TC5 テストに s_speed JSON フィールド確認を追加
