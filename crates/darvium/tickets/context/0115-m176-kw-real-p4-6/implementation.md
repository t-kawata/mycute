# 変更したファイル一覧と実装内容の概要

## src/simulation.rs

### 追加した関数
- `run_kw_real_simulation()` — KW-REAL シミュレーションのエントリポイント。SimulationContext を内部生成し、6 フェーズループを実行。
- `phase1_population_growth()` — Phase 1: 成人から子ノードを出生し信頼/評判/位置を継承
- `phase2_village_clustering()` — Phase 2: NodeId 順先頭 10% を anchor とした最近接村割り当て
- `phase3_help_protocol()` — Phase 3: `should_offer_help()` 呼び出しによる HELP Proposal 生成 + HelpSession 記録
- `phase4_gc_survival()` — Phase 4: `compute_lifecycle_score()` + `transition_gc_state()` による GC hazard 判定
- `phase5_capability_diffusion()` — Phase 5: 成功 HELP の信頼/評判継承（能力拡散）
- `phase6_measure_jkw()` — Phase 6: `compute_kind_world_objective()` による J_kw 測定
- `observe_kw_real_tick()` — tick 観測（SimulationTickSnapshot 生成）

### 追加したテスト（TC1-TC8）
- TC1 `tc1_kw_real_one_tick`: 1 tick 実行確認
- TC2 `tc2_kw_real_help_real_call`: mission_rate=1.0 での HelpSession 生成確認
- TC3 `tc3_kw_real_gc_interval`: gc_interval=3 周期 GC 確認
- TC4 `tc4_kw_real_village_clustering`: 村クラスタリング観測
- TC5 `tc5_kw_real_100_ticks`: 100 tick 耐久テスト
- TC6 `tc6_kw_real_deterministic`: 決定論的再現性検証
- TC7 `tc7_kw_real_child_ratio_sensitivity`: child_ratio 感度観測
- TC8 `tc8_kw_real_all_six_phases`: 全 6 フェーズ実行確認

### 依存結合先
- kind_world.rs: `compute_kind_world_objective`, `KindWorldMetricsInput`
- help.rs: `should_offer_help`, `HelpSession`
- event.rs: `transition_gc_state`
- lifecycle.rs: `compute_lifecycle_score`, `LifecycleScore`
- trust.rs: `inherit_trust`, `inherit_reputation`
- reciprocity.rs: `compute_experience_normalization`, `compute_gc_hazard`, `compute_survival_probability`
- clock/mod.rs: `compute_blended_freshness`

### 設計判断
- 死亡追跡: `HashSet<NodeId>` で管理（petgraph remove_node 回避）
- パラメータ多相: 各 phase 関数は必要な状態マップを引き数で受け取る（SimulationContext 肥大化防止）
- `println!` 出力: 観測ベース検証のための Phase マーカー（--nocapture で観測）
