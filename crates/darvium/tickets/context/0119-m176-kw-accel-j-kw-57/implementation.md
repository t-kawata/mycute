# 変更したファイル一覧と実装内容の概要

## src/kind_world.rs
- KindWorldMetricsInput に 6 フィールド追加: mean_nest_depth, mean_node_density, cluster_coefficient, local_density, search_radius_inverse, reasoning_steps_inverse
- KindWorldAssessment 構造体フィールド: s_viability→s_growth, s_capability→s_density, s_cooperation→s_topology, s_efficiency→s_search, j_pop→j_pop_growth
- KindWorldAssessment に 6 下位成分追加: j_nest_depth, j_node_density, j_clustering, j_local_density, j_search_radius_inv, j_reasoning_steps_inv
- zero() コンストラクタに 14 フィールド追加
- compute_kind_world_objective: 5因子乗算結合に再構成（20下位成分→5因子算術平均→乗算→J_kw）
- 6 ヘルパー関数追加: compute_mean_nest_depth, compute_mean_node_density, compute_cluster_coefficient, compute_local_density, compute_search_radius_inverse, compute_reasoning_steps_inverse
- collect_final_metrics: ヘルパー関数呼び出しを統合
- zero() コーディング修正（Boy Scout）
- 全テスト（TC1-TC12）更新: 新規6フィールド対応

## src/constants.rs
- KW_ACCEL_K_NEAREST: usize = 5（追加）
- KW_ACCEL_DENSITY_RADIUS: f64 = 0.3（追加）

## src/simulation.rs
- collect_final_metrics コンストラクタに 6 フィールド追加（0.5初期値）
