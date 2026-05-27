# 変更したファイル一覧と実装内容の概要

## src/simulation.rs

### SimulationContext 構造体に 3 フィールド追加（lines ~285-289）
- `total_inheritance_fidelity: f64` — 継承 fidelity 累積和
- `inheritance_event_count: u64` — 継承イベント発生回数
- `reciprocity_pair_counts: HashMap<(NodeId, NodeId), u64>` — ペア別 HELP 提供回数
- `SimulationContext::new()` で 3 フィールドを初期化

### phase1_population_growth の fidelity 記録（lines ~1546-1566）
- `inherit_trust` 呼び出し後に parent_avg / child_avg 比から fidelity を計算し累積

### phase3_help_protocol のペアカウンティング（line 1709）
- HELP セッション作成直後に `ctx.reciprocity_pair_counts` をインクリメント

### Boy Scout: phase5_capability_diffusion のハードコード値置き換え（line 1886）
- `inherit_trust(ht, ct, 0.7)` → `inherit_trust(ht, ct, TRUST_INHERIT_DECAY)`

## src/kind_world.rs

### 3 計算関数を追加
- `compute_mean_benevolence()` — TrustProfile 3 次元平均の全ノード平均
- `compute_mean_reciprocity()` — HELP ペアバランスからの互恵性スコア
- `compute_trust_inheritance_fidelity()` — 継承 fidelity 平均

### collect_final_metrics の 3 フィールド置き換え
- `mean_benevolence_aggregate: 0.0` → `compute_mean_benevolence(&ctx.trust_profiles)`
- `mean_reciprocity_score: 0.0` → `compute_mean_reciprocity(&ctx.reciprocity_pair_counts)`
- `trust_inheritance_fidelity: 0.0` → `compute_trust_inheritance_fidelity(ctx.total_inheritance_fidelity, ctx.inheritance_event_count)`

### テスト B1-B7 追加
- B1: compute_mean_benevolence — 空マップで 0.0
- B2: compute_mean_benevolence — 全 1.0 で 1.0
- B3: compute_mean_reciprocity — 対称ペアで 1.0
- B4: compute_mean_reciprocity — 非対称ペアで 0.0
- B5: compute_trust_inheritance_fidelity — event_count=0 で 0.0
- B6: collect_final_metrics — 3 指標が妥当な値（benevolence>0, fidelity>0, reciprocity>=0）
- B7: regression — 全既存テスト PASS 確認
