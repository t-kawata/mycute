# 変更したファイル一覧と実装内容の概要

## src/kind_world.rs

### 追加した構造体
- `EcosystemGrowthMetrics` — 6フィールド（tick, population_growth_rate, capability_coverage_shannon, reuse_ratio, cost_efficiency, benevolent_vs_non_benevolent_coverage_ratio）

### 追加した5つの純粋関数
1. `compute_population_growth_rate(population, previous_count) -> f64` — 人口成長率 (RFC §15.9.3)
2. `compute_capability_coverage_shannon(population) -> f64` — 能力カバー率（Shannon多様性指数、10x10グリッド量子化、H_max=log(100)で正規化）
3. `compute_reuse_ratio(events, sessions) -> f64` — ワークフロー再利用比率（同一workflowが2回以上出現するインタラクションの割合）
4. `compute_cost_efficiency(sessions) -> f64` — コスト効率（1.0 - (HarmfulMismatch + Abandoned) / total）
5. `compute_benevolent_vs_non_benevolent_coverage_ratio(population) -> f64` — 慈悲的/非慈悲的能力カバー率比（上位20%/下位20%のShannon H比）

### 内部ヘルパー関数
- `shannon_diversity_raw(group) -> f64` — 正規化前のShannon多様性指数

### 追加したObserver
- `EcosystemGrowthObserver` — `observe()` で5指標を一括計算、`print_csv()` でCSV出力

### 追加したテスト（10ユニットテスト + 1観測テスト）
- TC1-TC10: 境界値テスト、空入力耐性テスト、範囲保証テスト、Observer統合テスト、CSV出力テスト
- kw2_observational_csv_output: 20 tick のシミュレーションデータで観測テスト

## src/constants.rs

### 追加した定数
- `ECOSYSTEM_GRID_DIVISIONS: usize = 10` — Shannon多様性指数計算のグリッド分割数 (Calibration Candidate)
