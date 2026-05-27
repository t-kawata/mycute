# 実装サマリ: M1.76-KW-MTR-E: Village Churn & Benevolence Ratio Backfill

## 変更したファイル一覧

### src/simulation.rs
- SimulationContext に 2 フィールド追加: `village_assignment_total_comparisons: u64`, `village_assignment_changes: u64`
- SimulationContext::new() で両フィールドを 0 初期化
- phase2_village_clustering で子ノード割り当て時に累積カウンタを更新（以前の割り当てと比較し変化を追跡）

### src/kind_world.rs
- 新規関数: `compute_village_churn_rate(changes, comparisons) -> f64`
  - comparisons==0 で 0.0、それ以外は changes/comparisons を [0,1] に clamp
- 新規関数: `compute_benevolent_vs_non_benevolent_coverage_from_trust(trust_profiles, positions) -> f64`
  - TrustProfile 合成スコアを慈悲スコアのプロキシとして使用
  - 上位 20% / 下位 20% 分割、Shannon 多様性指数の比を計算
  - HashMap 非決定論性対策として node_ids を NodeId 順にソート
- 新規関数: `shannon_diversity_from_positions(node_ids, positions) -> f64`
  - positions から Shannon 多様性指数（正規化前 H）を計算する内部関数
- collect_final_metrics: village_churn_rate と benevolent_ratio のハードコード値を関数呼び出しに置き換え
- テスト追加: e1_mtre〜e6_mtre

### テスト結果
- 全 1272 テスト PASS（E7: 回帰なし確認）
- clippy: 警告なし
- E6 観測値: village_churn_rate=0.200000, benevolent_ratio=0.962479（両指標とも非デフォルト値）
