# 変更したファイル一覧と実装内容の概要

## src/reciprocity.rs

### 追加したコンポーネント

1. **MonotonicityCondition 列挙型** (public)
   - 4 variant: DirectScoreIncrease, IndirectScoreIncrease, ReputationIncrease, BenevolenceHelperRanking
   - Serialize/Deserialize, Hash 対応

2. **MonotonicityReport 構造体** (public)
   - conditions_passed: Vec<(MonotonicityCondition, bool)>
   - failure_details: Vec<String>
   - random_sweep_violation_rates: HashMap<MonotonicityCondition, f64>

3. **MonotonicityFixedParams 構造体** (public)
   - lifecycle_score: f32 (default 0.5)
   - child_protection: f32 (default 0.5)
   - delta_t: u64 (default 100)
   - policy: ReciprocityLifecyclePolicy (default)

4. **MonotonicityTestSuite 構造体** (public)
   - direct_score_points: Vec<f32> — 5点 sweep [0.0, 0.25, 0.5, 0.75, 1.0]
   - indirect_score_points: Vec<f32> — 同上
   - reputation_points: Vec<f32> — 同上
   - benevolence_delta_points: Vec<f32> — [0.001, 0.5] を 500 分割
   - fixed_params: MonotonicityFixedParams
   - random_sweep_samples: usize (default 1000)

5. **check_monotonicity 関数** (public)
   - 条件1: direct_score → survival_probability (5点 sweep + n=1000 random)
   - 条件2: indirect_score → GC hazard (5点 sweep + n=1000 random)
   - 条件3: Reputation → GC hazard (5点 sweep + n=1000 random)
   - 条件4: benevolence → helper ranking (基本比較 + ΔB sweep [0.001, 0.5])
   - 固定シード StdRng::seed_from_u64(12345)

6. **5 つのテスト関数** (mod tests)
   - test_direct_score_survival_monotonicity
   - test_indirect_score_gc_hazard_monotonicity
   - test_reputation_gc_hazard_monotonicity
   - test_benevolence_helper_ranking_monotonicity
   - test_monotonicity_suite_full

### インポート追加
- use rand::rngs::StdRng
- use rand::{Rng, SeedableRng}

### テスト結果
- 全 1009 テスト PASS
- 全 MUST 単調性条件 PASS (違反率 0.000000)
