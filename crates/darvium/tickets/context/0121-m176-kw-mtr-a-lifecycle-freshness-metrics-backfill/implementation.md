# 変更したファイル一覧と実装内容の概要

## src/simulation.rs
- **SimulationContext に 3 フィールド追加**: `node_gc_states`, `node_last_update_tick`, `total_births`
- **SimulationContext::new()**: 新フィールドの初期化（空 HashMap, 0）
- **add_person()**: 新規ノード作成時に `node_gc_states` と `node_last_update_tick` を初期化
- **remove_node()**: ノード削除時に新フィールドもクリーンアップ
- **run_evaluation_simulation()**: ローカル変数 `node_gc_states` を削除し `ctx.node_gc_states` で一元管理。child_count を計算して `collect_final_metrics` に渡す
- **phase1_population_growth()**: 出生時に `ctx.total_births` をインクリメント
- **phase4_gc_survival()**: GC 状態遷移パラメータを削除（ctx 内の `node_gc_states` を直接参照）。状態変更時に `ctx.node_last_update_tick` を更新
- **run_kw_real_simulation()**: 旧パスも `ctx.node_gc_states` 使用に統一

## src/kind_world.rs
- **lifecycle_score_from_gc_state()**: GcEvent→スコア マッピング（exhaustive match）
- **compute_mean_lifecycle_score()**: 全ノード GC 状態スコア平均
- **compute_child_survival_rate()**: 出生数/生存数から生存率
- **compute_mean_freshness()**: 最終更新 tick から freshness 平均
- **collect_final_metrics()**: `child_count` パラメータ追加、3 指標を実測値で計算
- **テスト A1-A6**: 6 つのテストケース追加
