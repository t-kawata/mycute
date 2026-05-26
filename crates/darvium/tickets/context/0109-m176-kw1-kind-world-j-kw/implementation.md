# 変更したファイル一覧と実装内容の概要

## src/constants.rs（定数追加、16 個）

KW1 関連定数をファイル末尾に追加：

### Safety Invariants（8 条件閾値）
- `KW_MIN_POPULATION_GROWTH_RATE: f64 = 0.01`
- `KW_MIN_CAPABILITY_COVERAGE_SHANNON: f64 = 0.5`
- `KW_MIN_REUSE_RATIO: f64 = 0.3`
- `KW_MAX_COST_EFFICIENCY_DECAY: f64 = 0.95`
- `KW_MIN_VILLAGE_FORMATION_SCORE: f64 = 0.3`
- `KW_VILLAGE_CHURN_LOWER: f64 = 0.05`
- `KW_VILLAGE_CHURN_UPPER: f64 = 0.30`
- `KW_CROSS_VILLAGE_INTERACTION_MIN: f64 = 0.1`

### Village 定数
- `VILLAGE_DISTANCE_THRESHOLD: f64 = 0.2` (Calibration Candidate)
- `VILLAGE_MIN_SIZE: usize = 3` (Safety Invariant)

### Calibration Candidates（J_kw 重み 6 個）
- `KW_ALPHA_POP: f64 = 0.25`
- `KW_ALPHA_COV: f64 = 0.20`
- `KW_ALPHA_REUSE: f64 = 0.15`
- `KW_ALPHA_COST: f64 = 0.20`
- `KW_ALPHA_VILLAGE: f64 = 0.10`
- `KW_ALPHA_PENALTY: f64 = 0.10`

## src/kind_world.rs（新規ファイル、約 640 行）

### 構造体
- `KindWorldMetricsInput` — 9 フィールドの入力構造体（population_growth_rate 〜 benevolent_vs_non_benevolent_coverage_ratio）
- `KindWorldAssessment` — 出力構造体（is_kind_world, flags: [bool; 8], j_kw）
- `MagnificentSevenParams` — 較正スイープ用 7 パラメータ（RFC §15.9.1）

### 純粋関数
- `compute_village_health_score()` — 4 指標（formation, flow_balance, cross_rate, diffusion）の等加重平均
- `compute_kind_world_objective()` — J_kw(θ) = Σαᵢ·Jᵢ、6 成分（pop, cov, reuse, cost, village, penalty）、[0,1] clamp

### テスト（11 件）
- TC-1〜TC-10: 不変条件テスト（assert!）
- kw_observational_random_stats: n=10,000 ランダム観測テスト（StdRng::seed_from_u64(12345)）

## src/lib.rs（1 行追加）

`pub mod kind_world;` を alphabetical position に追加
