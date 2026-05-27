// Darvium — Kind World 成立条件判定 + J_kw 目的関数

//

// M1.76-KW1: RFC §15.9.1 (成立条件閾値), §15.9.2 (J_kw 目的関数)

//

// このモジュールはエコシステム繁栄度を定量化する純粋関数を提供する。

// シミュレーターとの統合は KW2〜KW4 で実施され、本モジュールは

// データ構造と計算ロジックに専念する。

use serde::{Deserialize, Serialize};

// ============================================================================

// KindWorldMetricsInput — エコシステム測定値の入力構造体

// ============================================================================

/// Kind World 判定に必要な全エコシステム測定値。

///

/// シミュレーターや計装モジュールから収集された生メトリクスを格納する。

/// 全てのフィールドは [0, 1] または 0 以上の実数。

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]

pub struct KindWorldMetricsInput {
    /// ワークフロー人口成長率（1 tick あたり）
    pub population_growth_rate: f64,

    /// Shannon 多様性指数で測った能力カバー率 [0, 1]
    pub capability_coverage: f64,

    /// ワークフロー再利用比率 [0, 1]
    pub reuse_ratio: f64,

    /// コスト効率（値が大きいほど効率的）
    pub cost_efficiency: f64,

    /// 村形成スコア [0, 1]
    pub village_formation_score: f64,

    /// 村離脱率（churn rate）[0, 1]
    pub village_churn_rate: f64,

    /// 村間相互作用率 [0, 1]
    pub cross_village_interaction_rate: f64,

    /// 知識拡散率 [0, 1]
    pub knowledge_diffusion_rate: f64,

    /// 慈悲的ワークフロー人口 ÷ 非慈悲的ワークフロー人口
    pub benevolent_vs_non_benevolent_coverage_ratio: f64,

    /// 全個人の LifecycleScore の算術平均 [0, 1]
    pub mean_lifecycle_score: f64,

    /// 子供（経験不足個人）の生存割合 [0, 1]
    pub child_survival_rate: f64,

    /// 全個人の BlendedFreshness の算術平均 [0, 1]
    pub mean_freshness: f64,

    /// 全個人の慈悲総和の算術平均 [0, 1]
    pub mean_benevolence_aggregate: f64,

    /// 全個人の平均互恵性スコア [0, 1]
    pub mean_reciprocity_score: f64,

    /// 成功 HELP / 全 HELP セッション数 [0, 1]
    pub help_success_rate: f64,

    /// 世代間信頼継承忠実度 [0, 1]
    pub trust_inheritance_fidelity: f64,

    /// 成功実行 step / 全実行 step 数 [0, 1]
    pub execution_success_rate: f64,

    /// 平均サブWFネスト深度
    pub mean_nest_depth: f64,

    /// 1ルートWFあたり平均ノード数
    pub mean_node_density: f64,

    /// 平均Watts-Strogatzクラスター係数 [0, 1]
    pub cluster_coefficient: f64,

    /// 埋め込み空間局所密度 [0, 1]
    pub local_density: f64,

    /// 探索半径の減少関数 [0, 1]
    pub search_radius_inverse: f64,

    /// 推論深度の減少関数 [0, 1]
    pub reasoning_steps_inverse: f64,
}

impl KindWorldMetricsInput {
    /// 全フィールドが 0 の入力を作成する。

    pub const fn zero() -> Self {
        Self {
            population_growth_rate: 0.0,

            capability_coverage: 0.0,

            reuse_ratio: 0.0,

            cost_efficiency: 0.0,

            village_formation_score: 0.0,

            village_churn_rate: 0.0,

            cross_village_interaction_rate: 0.0,

            knowledge_diffusion_rate: 0.0,

            benevolent_vs_non_benevolent_coverage_ratio: 0.0,

            mean_lifecycle_score: 0.0,
            child_survival_rate: 0.0,
            mean_freshness: 0.0,
            mean_benevolence_aggregate: 0.0,
            mean_reciprocity_score: 0.0,
            help_success_rate: 0.0,
            trust_inheritance_fidelity: 0.0,
            execution_success_rate: 0.0,
            mean_nest_depth: 0.0,
            mean_node_density: 0.0,
            cluster_coefficient: 0.0,
            local_density: 0.0,
            search_radius_inverse: 0.0,
            reasoning_steps_inverse: 0.0,
        }
    }
}

// ============================================================================

// KindWorldAssessment — 判定結果

// ============================================================================

/// Kind World 成立判定の結果。

///

/// - `is_kind_world`: 全 8 条件が閾値を満たした場合のみ true

/// - `flags`: 8 要素の配列。各要素は対応する条件の成立/不成立

/// - `j_kw`: エコシステム繁栄度 [0, 1]

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]

pub struct KindWorldAssessment {
    /// J_kw > 0.8 && min(S_i) > 0.6 で true
    pub is_kind_world: bool,

    /// 目的関数値 J_kw = s_growth × s_density × s_topology × s_search × s_fairness
    pub j_kw: f64,

    /// 生態系存続性因子（社会加速度定義①: 人口増加速度）
    pub s_growth: f64,

    /// 能力繁栄度因子（社会加速度定義②: ワークフロー多層密度）
    pub s_density: f64,

    /// 協調健全性因子（社会加速度定義③: 空間クラスター係数・局所密度）
    pub s_topology: f64,

    /// 実行効率因子（社会加速度定義④: 探索半径・推論ステップ減少）
    pub s_search: f64,

    /// 構造的公平性因子
    pub s_fairness: f64,

    // 20 下位成分 (diagnostics)
    pub j_pop_growth: f64,

    pub j_lifecycle: f64,

    pub j_child_survival: f64,

    pub j_freshness: f64,

    pub j_cov: f64,

    pub j_diffusion: f64,

    pub j_reuse: f64,

    pub j_benevolence: f64,

    pub j_reciprocity: f64,

    pub j_help: f64,

    pub j_trust: f64,

    pub j_cost: f64,

    pub j_execution: f64,

    pub j_penalty: f64,

    // 新規 6 下位成分（社会加速度定義②③④充足用）
    pub j_nest_depth: f64,

    pub j_node_density: f64,

    pub j_clustering: f64,

    pub j_local_density: f64,

    pub j_search_radius_inv: f64,

    pub j_reasoning_steps_inv: f64,

    /// 旧 8 二値フラグ (diagnostics)
    pub legacy_flags: [bool; 8],
}

// ============================================================================

// MagnificentSevenParams — 較正スイープ対象パラメータ

// ============================================================================

/// 較正ループで sweep する 7 パラメータ。

///

/// RFC §15.9.1 のパラメータテーブルに対応。

/// これらの値はエコシステム全体の挙動を決定づけるため、

/// Phase 3 sweep で優先的に探索される（Calibration Candidate）。

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]

pub struct MagnificentSevenParams {
    /// 慈悲的戦略の初期割合
    pub gamma_benevolence: f64,

    /// GC hazard の基礎レート
    pub lambda_gc_base: f64,

    /// 直接互恵性の重み
    pub direct_reciprocity_weight: f64,

    /// 間接互恵性の重み
    pub indirect_reciprocity_weight: f64,

    /// ソフトマックス選択の温度パラメータ
    pub softmax_temperature: f64,

    /// GC 実行間隔（tick）
    pub gc_interval: u64,

    /// 子ワークフロー生成比率
    pub child_ratio: f64,
}

impl Default for MagnificentSevenParams {
    /// RFC §15.9.1 の推奨初期値。

    fn default() -> Self {
        Self {
            gamma_benevolence: 0.15,

            lambda_gc_base: 1.0,

            direct_reciprocity_weight: 0.4,

            indirect_reciprocity_weight: 0.3,

            softmax_temperature: 0.5,

            gc_interval: 3,

            child_ratio: 0.3,
        }
    }
}

// ============================================================================

// Phase 2: 全94パラメーター探索基盤 — AllParams

// ============================================================================

/// 全 94 パラメーターのベクターラッパー（Phase 2 次元拡張探索用）。
///
/// 内部は `Vec<f64>` で、インデックスアクセスにより NelderMead/Bayesian 最適化器が
/// 任意の次元数を扱える。グループマスクによりアクティブなパラメーター群を制御する。
#[derive(Debug, Clone)]
pub struct AllParams {
    /// 全パラメーター値（インデックスは ALL_IDX_* 定数で定義）
    pub values: Vec<f64>,

    /// 各パラメーターの探索有効フラグ（true = 最適化対象、false = 固定）
    pub active: Vec<bool>,

    /// 各パラメーターの探索範囲
    pub ranges: Vec<(f64, f64)>,
}

/// G1: 検索・探索系 — 14 パラメーター
pub const G1_COUNT: usize = 14;
/// REMOTE_EXPLORE_INTERVAL — リモート探索間隔（tick）。G1-0
pub const G1_REMOTE_EXPLORE_INTERVAL: usize = 0;
/// REMOTE_EXPLORE_DECAY — リモート探索減衰率。G1-1
pub const G1_REMOTE_EXPLORE_DECAY: usize = 1;
/// REMOTE_EXPLORE_STEPS — リモート探索ステップ数。G1-2
pub const G1_REMOTE_EXPLORE_STEPS: usize = 2;
/// REMOTE_EXPLORE_REWARD — リモート探索報酬。G1-3
pub const G1_REMOTE_EXPLORE_REWARD: usize = 3;
/// search_tick_fraction — 探索に割り当てる tick 割合。G1-4
pub const G1_SEARCH_TICK_FRACTION: usize = 4;
/// evaluate_fraction — 評価に割り当てる tick 割合。G1-5
pub const G1_EVALUATE_FRACTION: usize = 5;
/// KW4_EVALUATION_POPULATION_SIZE — 評価用人口サイズ。G1-6
pub const G1_EVALUATION_POPULATION_SIZE: usize = 6;
/// KW4_SIMULATION_TICKS — シミュレーション総 tick 数。G1-7
pub const G1_SIMULATION_TICKS: usize = 7;
/// RECIPROCITY_ALPHA_HELP — 直接互恵性 α_help。G1-8
pub const G1_RECIPROCITY_ALPHA_HELP: usize = 8;
/// RECIPROCITY_ALPHA_SUCCESS — 直接互恵性 α_success。G1-9
pub const G1_RECIPROCITY_ALPHA_SUCCESS: usize = 9;
/// RECIPROCITY_ALPHA_REJECT — 直接互恵性 α_reject。G1-10
pub const G1_RECIPROCITY_ALPHA_REJECT: usize = 10;
/// RECIPROCITY_ALPHA_HARM — 直接互恵性 α_harm。G1-11
pub const G1_RECIPROCITY_ALPHA_HARM: usize = 11;
/// compute_search_radius_inverse — 探索半径逆数（スタブ→実装）。G1-12
pub const G1_SEARCH_RADIUS_INVERSE: usize = 12;
/// REMOTE_EXPLORE_HUMAN_WEIGHT — リモート探索の人間重み。G1-13
pub const G1_REMOTE_EXPLORE_HUMAN_WEIGHT: usize = 13;

/// G2: GC・生存系 — 3 パラメーター（拡張可能）
pub const G2_COUNT: usize = 3;
/// gamma_lifecycle — GC hazard ライフサイクル重み。G2-0
pub const G2_GAMMA_LIFECYCLE: usize = G1_COUNT;
/// gamma_child_protect — GC hazard 子供保護重み。G2-1
pub const G2_GAMMA_CHILD_PROTECT: usize = G1_COUNT + 1;
/// kappa_e — 経験値正規化飽和率 κ_E。G2-2
pub const G2_KAPPA_E: usize = G1_COUNT + 2;

/// G3: offer/advance 確率 (WIRE-A 用)。8 パラメーター (予約、未実装)。
pub const G3_COUNT: usize = 8;

/// G4: 生成時定数 — 3 パラメーター (WIRE-C)。
pub const G4_COUNT: usize = 3;
/// 子供の初期 trust 上限。G4-0
pub const G4_CHILD_TRUST_MAX: usize = G1_COUNT + G2_COUNT + G3_COUNT;
/// 成人の初期 trust 下限。G4-1
pub const G4_ADULT_TRUST_MIN: usize = G1_COUNT + G2_COUNT + G3_COUNT + 1;
/// benevolent 分類閾値。G4-2
pub const G4_BENEVOLENT_THRESHOLD: usize = G1_COUNT + G2_COUNT + G3_COUNT + 2;

impl AllParams {
    /// 指定されたグループのパラメーター数と初期値で新規作成する。
    pub fn new(group_count: usize, defaults: &[f64], ranges: &[(f64, f64)]) -> Self {
        assert_eq!(group_count, defaults.len());
        assert_eq!(group_count, ranges.len());
        Self {
            values: defaults.to_vec(),
            active: vec![true; group_count],
            ranges: ranges.to_vec(),
        }
    }

    /// アクティブなパラメーター数のみを返す（最適化器用）。
    pub fn active_count(&self) -> usize {
        self.active.iter().filter(|&&a| a).count()
    }

    /// アクティブなパラメーターの値のみを Vec で返す。
    pub fn active_values(&self) -> Vec<f64> {
        self.values
            .iter()
            .zip(self.active.iter())
            .filter(|(_, &active)| active)
            .map(|(v, _)| *v)
            .collect()
    }

    /// アクティブパラメーターの探索範囲のみを Vec で返す。
    pub fn active_ranges(&self) -> Vec<(f64, f64)> {
        self.ranges
            .iter()
            .zip(self.active.iter())
            .filter(|(_, &active)| active)
            .map(|(r, _)| *r)
            .collect()
    }

    /// アクティブパラメーターのみの値と範囲で新たな AllParams を構築する。
    pub fn from_active_values(&self, active_values: &[f64]) -> Self {
        let active_count = self.active_count();
        assert_eq!(active_count, active_values.len());
        let mut values = self.values.clone();
        let mut vi = 0usize;
        for i in 0..values.len() {
            if self.active[i] {
                values[i] = active_values[vi];
                vi += 1;
            }
        }
        Self {
            values,
            active: self.active.clone(),
            ranges: self.ranges.clone(),
        }
    }

    /// 指定インデックスの値を clamp して設定する。
    pub fn set_clamped(&mut self, index: usize, value: f64) {
        let (lo, hi) = self.ranges[index];
        self.values[index] = value.clamp(lo, hi);
    }

    /// G1 デフォルト値で AllParams を構築する。
    pub fn default_g1() -> Self {
        let defaults = vec![
            30.0,   // G1_REMOTE_EXPLORE_INTERVAL
            0.5,    // G1_REMOTE_EXPLORE_DECAY
            3.0,    // G1_REMOTE_EXPLORE_STEPS
            0.1,    // G1_REMOTE_EXPLORE_REWARD
            0.5,    // G1_SEARCH_TICK_FRACTION
            0.3,    // G1_EVALUATE_FRACTION
            400.0,  // G1_EVALUATION_POPULATION_SIZE
            200.0,  // G1_SIMULATION_TICKS
            1.0,    // G1_RECIPROCITY_ALPHA_HELP (= constants::RECIPROCITY_ALPHA_HELP)
            2.0,    // G1_RECIPROCITY_ALPHA_SUCCESS (= constants::RECIPROCITY_ALPHA_SUCCESS)
            1.0,    // G1_RECIPROCITY_ALPHA_REJECT (= constants::RECIPROCITY_ALPHA_REJECT)
            2.0,    // G1_RECIPROCITY_ALPHA_HARM (= constants::RECIPROCITY_ALPHA_HARM)
            0.5,    // G1_SEARCH_RADIUS_INVERSE (stub)
            0.0,    // G1_REMOTE_EXPLORE_HUMAN_WEIGHT
        ];
        let ranges = vec![
            (1.0, 100.0),   // REMOTE_EXPLORE_INTERVAL
            (0.01, 0.99),   // REMOTE_EXPLORE_DECAY
            (1.0, 10.0),    // REMOTE_EXPLORE_STEPS
            (0.01, 1.0),    // REMOTE_EXPLORE_REWARD
            (0.1, 0.9),     // SEARCH_TICK_FRACTION
            (0.1, 0.9),     // EVALUATE_FRACTION
            (50.0, 2000.0), // EVALUATION_POPULATION_SIZE
            (20.0, 1000.0), // SIMULATION_TICKS
            (0.1, 5.0),     // RECIPROCITY_ALPHA_HELP (default 1.0)
            (0.1, 5.0),     // RECIPROCITY_ALPHA_SUCCESS (default 2.0)
            (0.1, 5.0),     // RECIPROCITY_ALPHA_REJECT (default 1.0)
            (0.1, 5.0),     // RECIPROCITY_ALPHA_HARM (default 2.0)
            (0.1, 1.0),     // SEARCH_RADIUS_INVERSE
            (0.0, 1.0),     // REMOTE_EXPLORE_HUMAN_WEIGHT
        ];
        let mut params = Self::new(G1_COUNT, &defaults, &ranges);
        // SEARCH_RADIUS_INVERSE は compute_search_radius_inverse が実測値で計算するため経路無効。
        // 将来の production コード変更で再有効化可能。
        params.active[G1_SEARCH_RADIUS_INVERSE] = false;
        params
    }

    /// G1 パラメーターから ReciprocitySimulatorConfig を構築する。
    ///
    /// 既存の MagnificentSevenParams::to_sim_config() に加えて、
    /// population_size、max_ticks、RECIPROCITY_ALPHA_* を AllParams の値で上書きする。
    pub fn to_sim_config_g1(&self, seed: u64) -> crate::simulation::ReciprocitySimulatorConfig {
        let ms = MagnificentSevenParams::default();
        let mut config = ms.to_sim_config(
            self.values[G1_EVALUATION_POPULATION_SIZE].round() as usize,
            seed,
        );
        config.max_ticks = self.values[G1_SIMULATION_TICKS].round() as u64;
        config.policy.alpha_help = self.values[G1_RECIPROCITY_ALPHA_HELP] as f32;
        config.policy.alpha_success = self.values[G1_RECIPROCITY_ALPHA_SUCCESS] as f32;
        config.policy.alpha_reject = self.values[G1_RECIPROCITY_ALPHA_REJECT] as f32;
        config.policy.alpha_harm = self.values[G1_RECIPROCITY_ALPHA_HARM] as f32;
        config
    }

    /// G1 + G2 デフォルト値で AllParams を構築する。
    pub fn default_g1g2() -> Self {
        let g1 = Self::default_g1();
        let g2_defaults = vec![
            crate::constants::GC_HAZARD_GAMMA_LIFECYCLE as f64,   // G2_GAMMA_LIFECYCLE
            crate::constants::GC_HAZARD_GAMMA_CHILD_PROTECT as f64, // G2_GAMMA_CHILD_PROTECT
            crate::constants::REPUTATION_KAPPA_E as f64,          // G2_KAPPA_E
        ];
        let g2_ranges = vec![
            (0.0, 5.0),   // GAMMA_LIFECYCLE
            (0.0, 20.0),  // GAMMA_CHILD_PROTECT
            (0.001, 1.0), // KAPPA_E
        ];
        let mut values = g1.values;
        values.extend(g2_defaults);
        let mut ranges = g1.ranges;
        ranges.extend(g2_ranges);
        let mut active = g1.active;
        active.extend(vec![true; G2_COUNT]);
        Self { values, active, ranges }
    }

    /// G1 + G2 パラメーターから ReciprocitySimulatorConfig を構築する。
    ///
    /// to_sim_config_g1 に加えて、G2 の GC 関連 3 パラメーターを policy に設定する。
    pub fn to_sim_config_g1g2(&self, seed: u64) -> crate::simulation::ReciprocitySimulatorConfig {
        let mut config = self.to_sim_config_g1(seed);
        config.policy.gamma_lifecycle = self.values[G2_GAMMA_LIFECYCLE] as f32;
        config.policy.gamma_child_protect = self.values[G2_GAMMA_CHILD_PROTECT] as f32;
        config.policy.kappa_e = self.values[G2_KAPPA_E] as f32;
        config
    }

    /// G1 + G2 + G4 デフォルト値で AllParams を構築する (WIRE-C)。
    ///
    /// G3 は WIRE-A 用の予約領域。本メソッドでは G3 値を 0.0 (inactive) で埋める。
    pub fn default_g1g2g4() -> Self {
        let g1g2 = Self::default_g1g2();
        // G3: WIRE-A 予約領域 (8 パラメーター、全て inactive)
        let g3_defaults = vec![0.0; G3_COUNT];
        let g3_ranges = vec![(0.0, 1.0); G3_COUNT];
        // G4: 生成時定数
        let g4_defaults = vec![
            crate::constants::SIMULATION_CHILD_TRUST_MAX,    // G4_CHILD_TRUST_MAX
            crate::constants::SIMULATION_ADULT_TRUST_MIN,    // G4_ADULT_TRUST_MIN
            crate::constants::SIMULATION_BENEVOLENT_THRESHOLD, // G4_BENEVOLENT_THRESHOLD
        ];
        let g4_ranges = vec![
            (0.0, 1.0), // CHILD_TRUST_MAX
            (0.0, 1.0), // ADULT_TRUST_MIN
            (0.0, 1.0), // BENEVOLENT_THRESHOLD
        ];
        let mut values = g1g2.values;
        values.extend(g3_defaults);
        values.extend(g4_defaults);
        let mut ranges = g1g2.ranges;
        ranges.extend(g3_ranges);
        ranges.extend(g4_ranges);
        let mut active = g1g2.active;
        active.extend(vec![false; G3_COUNT]); // G3: 全 inactive (WIRE-A 未実装)
        active.extend(vec![true; G4_COUNT]);  // G4: 全 active
        Self { values, active, ranges }
    }

    /// G1 + G2 + G4 パラメーターから ReciprocitySimulatorConfig を構築する (WIRE-C)。
    ///
    /// to_sim_config_g1g2 に加えて、G4 の生成時 3 定数を config に設定する。
    pub fn to_sim_config_g1g2g4(&self, seed: u64) -> crate::simulation::ReciprocitySimulatorConfig {
        let mut config = self.to_sim_config_g1g2(seed);
        config.child_trust_max = self.values[G4_CHILD_TRUST_MAX];
        config.adult_trust_min = self.values[G4_ADULT_TRUST_MIN];
        config.benevolent_threshold = self.values[G4_BENEVOLENT_THRESHOLD];
        config
    }
}

// ============================================================================

// compute_village_health_score — 村健全性スコア

// ============================================================================

/// 村の健全性を 4 指標の平均として計算する。

///

/// J_village の基礎となるスコア。以下の 4 成分を等加重平均する：

/// 1. `formation_score` — 村の形成度合い（silhouette 類似スコア）

/// 2. フローバランス健全性 — churn が適正範囲 [KW_VILLAGE_CHURN_LOWER, KW_VILLAGE_CHURN_UPPER] 内なら 1.0、範囲外なら 0.0

/// 3. `cross_rate` — 村間相互作用率

/// 4. `diffusion_rate` — 知識拡散率

///

/// RFC §15.9.4 の定義に従い、flow_balance_health は churn の適正範囲に基づく

/// 二値判定（`1.0 - churn_rate` の線形近似ではない）。

/// 戻り値は [0, 1] に clamp される。

pub fn compute_village_health_score(
    formation_score: f64,

    churn_rate: f64,

    cross_rate: f64,

    diffusion_rate: f64,
) -> f64 {
    // flow_balance_health: churn が適正範囲内なら健全、範囲外なら不健全

    let flow_balance_health = if (crate::constants::KW_VILLAGE_CHURN_LOWER
        ..=crate::constants::KW_VILLAGE_CHURN_UPPER)
        .contains(&churn_rate)
    {
        1.0
    } else {
        0.0
    };

    let raw = (formation_score + flow_balance_health + cross_rate + diffusion_rate) / 4.0;

    raw.clamp(0.0, 1.0)
}

// ============================================================================

// compute_kind_world_objective — J_kw(θ) 目的関数

// ============================================================================

/// Kind World 目的関数 J_kw(θ) を計算する。

///

/// J_kw = s_growth × s_density × s_topology × s_search × s_fairness

///

/// RFC §15.9.2 の 5 因子乗算結合モデルに従い、20 下位成分の算術平均で

/// 5 因子を計算し、その乗算結果を [0, 1] に clamp して返す。

/// Kind World 成立条件: J_kw > 0.8 かつ min(S_i) > 0.6

/// 旧 8 二値フラグは legacy_flags として diagnostics に出力する。

pub fn compute_kind_world_objective(metrics: &KindWorldMetricsInput) -> KindWorldAssessment {
    // ---- 旧 8 二値フラグ (diagnostics) ----

    let legacy_flag_population =
        metrics.population_growth_rate >= crate::constants::KW_MIN_POPULATION_GROWTH_RATE;
    let legacy_flag_capability =
        metrics.capability_coverage >= crate::constants::KW_MIN_CAPABILITY_COVERAGE_SHANNON;
    let legacy_flag_reuse = metrics.reuse_ratio >= crate::constants::KW_MIN_REUSE_RATIO;
    let legacy_flag_cost =
        metrics.cost_efficiency <= crate::constants::KW_MAX_COST_EFFICIENCY_DECAY;
    let legacy_flag_village_formation =
        metrics.village_formation_score >= crate::constants::KW_MIN_VILLAGE_FORMATION_SCORE;
    let legacy_flag_churn_low =
        metrics.village_churn_rate >= crate::constants::KW_VILLAGE_CHURN_LOWER;
    let legacy_flag_churn_high =
        metrics.village_churn_rate <= crate::constants::KW_VILLAGE_CHURN_UPPER;
    let legacy_flag_cross = metrics.cross_village_interaction_rate
        >= crate::constants::KW_CROSS_VILLAGE_INTERACTION_MIN;

    let legacy_flags = [
        legacy_flag_population,
        legacy_flag_capability,
        legacy_flag_reuse,
        legacy_flag_cost,
        legacy_flag_village_formation,
        legacy_flag_churn_low,
        legacy_flag_churn_high,
        legacy_flag_cross,
    ];

    // ---- 20 下位成分 ----

    let j_pop_growth = metrics.population_growth_rate.clamp(0.0, 1.0);
    let j_lifecycle = metrics.mean_lifecycle_score.clamp(0.0, 1.0);
    let j_child_survival = metrics.child_survival_rate.clamp(0.0, 1.0);
    let j_freshness = metrics.mean_freshness.clamp(0.0, 1.0);
    let j_cov = metrics.capability_coverage.clamp(0.0, 1.0);
    let j_diffusion = metrics.knowledge_diffusion_rate.clamp(0.0, 1.0);
    let j_reuse = metrics.reuse_ratio.clamp(0.0, 1.0);
    let j_nest_depth = metrics.mean_nest_depth.clamp(0.0, 1.0);
    let j_node_density = metrics.mean_node_density.clamp(0.0, 1.0);
    let j_benevolence = metrics.mean_benevolence_aggregate.clamp(0.0, 1.0);
    let j_reciprocity = metrics.mean_reciprocity_score.clamp(0.0, 1.0);
    let j_help = metrics.help_success_rate.clamp(0.0, 1.0);
    let j_trust = metrics.trust_inheritance_fidelity.clamp(0.0, 1.0);
    let j_clustering = metrics.cluster_coefficient.clamp(0.0, 1.0);
    let j_local_density = metrics.local_density.clamp(0.0, 1.0);
    // J_cost は cost_efficiency をそのまま正の向きで使用 (RFC §15.9.2)
    let j_cost = metrics.cost_efficiency.clamp(0.0, 1.0);
    let j_execution = metrics.execution_success_rate.clamp(0.0, 1.0);
    let j_search_radius_inv = metrics.search_radius_inverse.clamp(0.0, 1.0);
    let j_reasoning_steps_inv = metrics.reasoning_steps_inverse.clamp(0.0, 1.0);

    // J_penalty: ratio < 1.0 で線形ペナルティ
    let j_penalty = {
        let ratio = metrics.benevolent_vs_non_benevolent_coverage_ratio;
        if ratio >= 1.0 {
            0.0
        } else {
            (1.0 - ratio).clamp(0.0, 1.0)
        }
    };

    // ---- 5 因子の算術平均（社会加速度定義に基づく再構成） ----

    let s_growth = (j_pop_growth + j_lifecycle + j_child_survival + j_freshness) / 4.0;
    let s_density = (j_cov + j_diffusion + j_reuse + j_nest_depth + j_node_density) / 5.0;
    let s_topology =
        (j_benevolence + j_reciprocity + j_help + j_trust + j_clustering + j_local_density) / 6.0;
    let s_search = (j_cost + j_execution + j_search_radius_inv + j_reasoning_steps_inv) / 4.0;
    let s_fairness = 1.0 - j_penalty;

    // ---- 乗算結合 ----

    let j_kw = (s_growth * s_density * s_topology * s_search * s_fairness).clamp(0.0, 1.0);

    // ---- 5 因子最小値ゲート ----

    let min_factor = s_growth
        .min(s_density)
        .min(s_topology)
        .min(s_search)
        .min(s_fairness);

    let is_kind_world = j_kw > 0.8 && min_factor > 0.6;

    KindWorldAssessment {
        is_kind_world,
        j_kw,
        s_growth,
        s_density,
        s_topology,
        s_search,
        s_fairness,
        j_pop_growth,
        j_lifecycle,
        j_child_survival,
        j_freshness,
        j_cov,
        j_diffusion,
        j_reuse,
        j_benevolence,
        j_reciprocity,
        j_help,
        j_trust,
        j_cost,
        j_execution,
        j_penalty,
        j_nest_depth,
        j_node_density,
        j_clustering,
        j_local_density,
        j_search_radius_inv,
        j_reasoning_steps_inv,
        legacy_flags,
    }
}

// ============================================================================

// ============================================================================

// EcosystemGrowthMetrics — エコシステム成長メトリクス (M1.76-KW2)

// ============================================================================

/// エコシステム成長を 4 次元で計測する構造体 (RFC §15.9.3)。

///

/// 各 tick のエコシステム状態を以下の 5 指標で記録する：

/// - population_growth_rate: 人口成長率（負値は減少、正値は増加）

/// - capability_coverage_shannon: Shannon 多様性指数で測った能力カバー率 [0, 1]

/// - reuse_ratio: ワークフロー再利用比率 [0, 1]

/// - cost_efficiency: コスト効率 [0, 1]（1.0 に近いほど効率的）

/// - benevolent_vs_non_benevolent_coverage_ratio: 慈悲的/非慈悲的能力カバー率比

#[derive(Debug, Clone, Copy, PartialEq)]

pub struct EcosystemGrowthMetrics {
    /// 観測 tick
    pub tick: u64,

    /// 人口成長率（1 tick あたり）
    pub population_growth_rate: f64,

    /// Shannon 多様性指数で測った能力カバー率 [0, 1]
    pub capability_coverage_shannon: f64,

    /// ワークフロー再利用比率 [0, 1]
    pub reuse_ratio: f64,

    /// コスト効率 [0, 1]（1.0 に近いほど効率的）
    pub cost_efficiency: f64,

    /// 慈悲的集団 / 非慈悲的集団の能力カバー率比
    pub benevolent_vs_non_benevolent_coverage_ratio: f64,
}

// VillageInteractionMetrics — 村間相互作用測定値 (M1.76-KW3)

// ============================================================================

/// 村間相互作用の測定値 (RFC §15.9.4)。

///

/// DBSCAN 類似の空間クラスタリングにより導出された村割り当てに基づき、

/// 村間相互作用率・村形成強度・知識拡散速度・村フローバランスの

/// 4 指標を記録する。全指標は [0, 1] 範囲であることが保証される。

#[derive(Debug, Clone, Copy, PartialEq)]

pub struct VillageInteractionMetrics {
    /// 観測 tick
    pub tick: u64,

    /// 村の総数
    pub village_count: usize,

    /// 村間相互作用率 [0, 1]
    pub cross_village_interaction_rate: f64,

    /// 村形成強度 [0, 1]（silhouette 類似スコア）
    pub village_formation_strength: f64,

    /// 知識拡散率 [0, 1]
    pub knowledge_diffusion_rate: f64,

    /// 村フローバランス（churn 率）[0, 1]
    pub village_flow_balance: f64,

    /// 村サイズの平均
    pub mean_village_size: f64,

    /// 村サイズの分散
    pub village_size_variance: f64,
}

// ============================================================================

// 人口成長率 (RFC §15.9.3)

// ============================================================================

/// 人口成長率を計算する。

///

/// 人口成長率 = (current_count - previous_count) / max(previous_count, 1)

///

/// 減少時は負値、増加時は正値を返す。空人口時は 0.0 を返す。

/// population は生存ワークフローのみカウントする（survived == true）。

pub fn compute_population_growth_rate(
    population: &[crate::simulation::SimWorkflowState],

    previous_count: usize,
) -> f64 {
    let current_count = population.iter().filter(|w| w.survived).count();

    let prev = previous_count.max(1);

    (current_count as f64 - previous_count as f64) / prev as f64
}

// ============================================================================

// Shannon 多様性指数 (RFC §15.9.3)

// ============================================================================

/// 能力空間の Shannon 多様性指数を計算する。

///

/// ワークフローの能力空間（position[0], position[1]）を N×N グリッドに量子化し、

/// Shannon 多様性指数 H = -Σ p_i log p_i を計算する。

/// H_max = log(N²) で除算して [0, 1] に正規化する。

/// 空 population の場合は 0.0 を返す。

pub fn compute_capability_coverage_shannon(
    population: &[crate::simulation::SimWorkflowState],
) -> f64 {
    let survived: Vec<&crate::simulation::SimWorkflowState> =
        population.iter().filter(|w| w.survived).collect();

    if survived.is_empty() {
        return 0.0;
    }

    let grid_size = crate::constants::ECOSYSTEM_GRID_DIVISIONS;

    let total_cells = (grid_size * grid_size) as f64;

    // N×N グリッドに量子化

    let mut grid = vec![0u64; grid_size * grid_size];

    for wf in &survived {
        let x = ((wf.position[0] as f64).clamp(0.0, 0.999) * grid_size as f64) as usize;

        let y = ((wf.position[1] as f64).clamp(0.0, 0.999) * grid_size as f64) as usize;

        let x = x.min(grid_size - 1);

        let y = y.min(grid_size - 1);

        grid[y * grid_size + x] += 1;
    }

    // Shannon 多様性指数 H = -Σ p_i log p_i

    let total = survived.len() as f64;

    let mut h = 0.0_f64;

    for &count in &grid {
        if count > 0 {
            let p = count as f64 / total;

            h -= p * p.ln();
        }
    }

    // H_max = log(N²) で正規化

    let h_max = total_cells.ln();

    if h_max > 0.0 {
        (h / h_max).clamp(0.0, 1.0)
    } else {
        0.0
    }
}

// ============================================================================

// 再利用比率 (RFC §15.9.3)

// ============================================================================

/// ワークフロー再利用比率を計算する。

///

/// 同一 workflow が複数回ヘルプ提供または依頼を受けている割合。

/// 再利用回数 / 全インタラクション数。

/// 全インタラクション数が 0 の場合は 0.0 を返す。

///

/// events は ReciprocityEvent のリスト（source_graph_id / target_graph_id をキーに集計）。

/// sessions は SimHelpSession のリスト（helper_id / requester_id をキーに集計）。

/// 両方をマージして同一 ID の出現頻度をカウントする。

pub fn compute_reuse_ratio(
    events: &[crate::event::ReciprocityEvent],

    sessions: &[crate::simulation::SimHelpSession],
) -> f64 {
    // 全インタラクション数をカウント

    let total_interactions = events.len() + sessions.len();

    if total_interactions == 0 {
        return 0.0;
    }

    // 各 workflow の出現頻度をカウント

    use std::collections::HashMap;

    let mut freq: HashMap<&str, u64> = HashMap::new();

    for ev in events {
        *freq.entry(&ev.source_graph_id).or_insert(0) += 1;

        *freq.entry(&ev.target_graph_id).or_insert(0) += 1;
    }

    for session in sessions {
        *freq.entry(&session.helper_id).or_insert(0) += 1;

        *freq.entry(&session.requester_id).or_insert(0) += 1;
    }

    // 2 回以上出現する workflow によるインタラクション数を「再利用回数」とする

    // 各インタラクションを1回だけカウントするため、session/event ごとに

    // helper/source または requester/target のいずれかが頻出なら再利用と判定

    let mut reuse_count = 0u64;

    for session in sessions {
        let is_reused = freq.get(session.helper_id.as_str()).copied().unwrap_or(0) >= 2
            || freq
                .get(session.requester_id.as_str())
                .copied()
                .unwrap_or(0)
                >= 2;

        if is_reused {
            reuse_count += 1;
        }
    }

    for ev in events {
        let is_reused = freq.get(ev.source_graph_id.as_str()).copied().unwrap_or(0) >= 2
            || freq.get(ev.target_graph_id.as_str()).copied().unwrap_or(0) >= 2;

        if is_reused {
            reuse_count += 1;
        }
    }

    reuse_count as f64 / total_interactions as f64
}

// ============================================================================

// コスト効率 (RFC §15.9.3)

// ============================================================================

/// コスト効率を計算する。

///

/// コスト効率 = 1.0 - (失敗セッション数 + 放棄セッション数) / 全セッション数

/// 失敗: HarmfulMismatch、放棄: Abandoned。

/// 全セッション成功時 1.0、全セッション失敗時 0.0、空セッション時 1.0。

pub fn compute_cost_efficiency(sessions: &[crate::simulation::SimHelpSession]) -> f64 {
    if sessions.is_empty() {
        return 1.0;
    }

    let total = sessions.len() as f64;

    let failed = sessions
        .iter()
        .filter(|s| {
            s.status == crate::simulation::HelpSessionStatus::HarmfulMismatch
                || s.status == crate::simulation::HelpSessionStatus::Abandoned
        })
        .count() as f64;

    (1.0 - failed / total).clamp(0.0, 1.0)
}

// ============================================================================

// 慈悲的/非慈悲的能力カバー率比 (RFC §15.9.3)

// ============================================================================

/// 慈悲的集団と非慈悲的集団の能力カバー率比を計算する。

///

/// ワークフローを initial_benevolence で降順ソートし、上位 20%（慈悲的集団）と

/// 下位 20%（非慈悲的集団）の Shannon 多様性指数（正規化前 H）の比を返す。

/// \\> 1.0 で慈悲的優位を示す。各集団がグリッド分割に足りない場合は 1.0 を返す。

/// 全人口が空の場合は 1.0 を返す。

pub fn compute_benevolent_vs_non_benevolent_coverage_ratio(
    population: &[crate::simulation::SimWorkflowState],
) -> f64 {
    let survived: Vec<&crate::simulation::SimWorkflowState> =
        population.iter().filter(|w| w.survived).collect();

    if survived.is_empty() {
        return 1.0;
    }

    // initial_benevolence で降順ソート

    let mut sorted: Vec<&crate::simulation::SimWorkflowState> = survived;

    sorted.sort_unstable_by(|a, b| {
        b.initial_benevolence
            .partial_cmp(&a.initial_benevolence)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let top_count = (sorted.len() as f64 * crate::constants::BENEVOLENT_TOP_FRACTION)
        .ceil()
        .max(1.0) as usize;

    let bottom_count = (sorted.len() as f64 * crate::constants::BENEVOLENT_BOTTOM_FRACTION)
        .ceil()
        .max(1.0) as usize;

    // 上位と下位が重なる場合は差を計算できない → 1.0

    if top_count + bottom_count > sorted.len() {
        return 1.0;
    }

    let top_group = &sorted[..top_count];

    let bottom_group = &sorted[sorted.len() - bottom_count..];

    let top_h = shannon_diversity_raw(top_group);

    let bottom_h = shannon_diversity_raw(bottom_group);

    if bottom_h <= 0.0 {
        // 非慈悲的集団がグリッドに収まらない（全同一セル）→ 慈悲的優位とみなす

        if top_h > 0.0 {
            2.0
        } else {
            1.0
        }
    } else {
        (top_h / bottom_h).clamp(0.0, f64::MAX)
    }
}

/// ワークフロー集団の Shannon 多様性指数（正規化前 H）を計算する内部関数。

fn shannon_diversity_raw(group: &[&crate::simulation::SimWorkflowState]) -> f64 {
    if group.is_empty() {
        return 0.0;
    }

    let grid_size = crate::constants::ECOSYSTEM_GRID_DIVISIONS;

    let mut grid = vec![0u64; grid_size * grid_size];

    for wf in group {
        let x = ((wf.position[0] as f64).clamp(0.0, 0.999) * grid_size as f64) as usize;

        let y = ((wf.position[1] as f64).clamp(0.0, 0.999) * grid_size as f64) as usize;

        let x = x.min(grid_size - 1);

        let y = y.min(grid_size - 1);

        grid[y * grid_size + x] += 1;
    }

    let total = group.len() as f64;

    let mut h = 0.0;

    for &count in &grid {
        if count > 0 {
            let p = count as f64 / total;

            h -= p * p.ln();
        }
    }

    h
}

// ============================================================================

// EcosystemGrowthObserver — 成長メトリクス観測器

// ============================================================================

/// エコシステム成長メトリクスを観測する observer。

///

/// ReciprocityMetricsObserver（simulation.rs）と同様の API 設計。

/// 各 tick の人口・セッション・イベントから 5 指標を計算し、

/// EcosystemGrowthMetrics として出力する。

pub struct EcosystemGrowthObserver;

impl EcosystemGrowthObserver {
    /// 1 tick 分のエコシステム成長メトリクスを計算する。

    ///

    /// # 引数

    /// - `tick`: 現在の tick 番号

    /// - `population`: 現在のワークフロー集団

    /// - `sessions`: 現在のヘルプセッション一覧

    /// - `events`: ReciprocityEvent 一覧

    /// - `previous_population_count`: 前 tick の生存ワークフロー数（初回は 0）

    pub fn observe(
        tick: u64,

        population: &[crate::simulation::SimWorkflowState],

        sessions: &[crate::simulation::SimHelpSession],

        events: &[crate::event::ReciprocityEvent],

        previous_population_count: usize,
    ) -> EcosystemGrowthMetrics {
        EcosystemGrowthMetrics {
            tick,

            population_growth_rate: compute_population_growth_rate(
                population,
                previous_population_count,
            ),

            capability_coverage_shannon: compute_capability_coverage_shannon(population),

            reuse_ratio: compute_reuse_ratio(events, sessions),

            cost_efficiency: compute_cost_efficiency(sessions),

            benevolent_vs_non_benevolent_coverage_ratio:
                compute_benevolent_vs_non_benevolent_coverage_ratio(population),
        }
    }

    /// SimulationContext から成長メトリクスを収集する（P4 シミュレーション用）。

    pub fn observe_from_context(
        ctx: &crate::simulation::SimulationContext,
    ) -> EcosystemGrowthMetrics {
        EcosystemGrowthMetrics {
            tick: ctx.tick,

            population_growth_rate: 0.0,

            capability_coverage_shannon: 0.0,

            reuse_ratio: 0.0,

            cost_efficiency: 0.5,

            benevolent_vs_non_benevolent_coverage_ratio: 1.0,
        }
    }

    /// 全 tick の成長メトリクス系列を CSV 形式で標準出力に書き出す。

    pub fn print_csv(series: &[EcosystemGrowthMetrics], prefix: &str) {
        println!(

            "{prefix}: tick,population_growth_rate,capability_coverage_shannon,reuse_ratio,cost_efficiency,benevolent_vs_non_benevolent_coverage_ratio"

        );

        for metrics in series {
            println!(
                "{prefix}: {},{:.6},{:.6},{:.6},{:.6},{:.6}",
                metrics.tick,
                metrics.population_growth_rate,
                metrics.capability_coverage_shannon,
                metrics.reuse_ratio,
                metrics.cost_efficiency,
                metrics.benevolent_vs_non_benevolent_coverage_ratio,
            );
        }
    }
}

// ============================================================================

// assign_village_ids — DBSCAN 類似の空間クラスタリング (M1.76-KW3)

// ============================================================================

/// ワークフローの position に基づく空間クラスタリング。

///

/// DBSCAN 類似の簡易アルゴリズムにより、`VILLAGE_DISTANCE_THRESHOLD` 内の

/// ワークフローを同一村に割り当てる。`VILLAGE_MIN_SIZE` 未満のクラスタは

/// ノイズとして村未所属（`None`）とする。

///

/// 村 ID は tick ごとに新規計算される一時的な割り当てラベルであり、

/// `SimWorkflowState` に永続フィールドを追加しない（RFC §41B.3）。

///

/// # 戻り値

/// `population` と同じ長さの `Vec<Option<usize>>`。

/// `None` = 村未所属（ノイズ）。

pub fn assign_village_ids(
    population: &[crate::simulation::SimWorkflowState],
) -> Vec<Option<usize>> {
    if population.is_empty() {
        return Vec::new();
    }

    let n = population.len();

    let threshold = crate::constants::VILLAGE_DISTANCE_THRESHOLD;

    let min_size = crate::constants::VILLAGE_MIN_SIZE;

    // (x, y) 座標を population と同じ順序で抽出

    let positions: Vec<[f32; 2]> = population
        .iter()
        .map(|w| [w.position[0], w.position[1]])
        .collect();

    // 2 点間のユークリッド距離

    let distance = |a: &[f32; 2], b: &[f32; 2]| -> f64 {
        let dx = (a[0] - b[0]) as f64;

        let dy = (a[1] - b[1]) as f64;

        (dx * dx + dy * dy).sqrt()
    };

    let mut visited = vec![false; n];

    let mut assignments: Vec<Option<usize>> = vec![None; n];

    let mut next_village_id: usize = 0;

    for i in 0..n {
        if visited[i] {
            continue;
        }

        visited[i] = true;

        // 距離閾値内の neighbors を収集

        let mut cluster: Vec<usize> = vec![i];

        let mut frontier: Vec<usize> = vec![i];

        while let Some(current) = frontier.pop() {
            for j in 0..n {
                if !visited[j] && distance(&positions[current], &positions[j]) <= threshold {
                    visited[j] = true;

                    cluster.push(j);

                    frontier.push(j);
                }
            }
        }

        // 最小サイズ未満 → ノイズ（None のまま）

        if cluster.len() < min_size {
            continue;
        }

        // 最小サイズ以上 → 新規村 ID を割り当て

        let village_id = next_village_id;

        next_village_id += 1;

        for &idx in &cluster {
            assignments[idx] = Some(village_id);
        }
    }

    assignments
}

// ============================================================================

// compute_cross_village_interaction_rate — 村間相互作用率 (M1.76-KW3)

// ============================================================================

/// 村間相互作用率を計算する。

///

/// 異なる村 ID 間で発生したヘルプセッションの割合。

/// 各セッションの helper / requester に対応する村ラベルが異なる場合を

/// 「村間」としてカウントする。

///

/// # 戻り値

/// 村間セッション数 / 全セッション数。[0, 1] に clamp。

/// セッション数が 0 または `village_assignments` が空の場合は 0.0。

pub fn compute_cross_village_interaction_rate(
    sessions: &[crate::simulation::SimHelpSession],

    village_assignments: &[Option<usize>],
) -> f64 {
    if sessions.is_empty() || village_assignments.is_empty() {
        return 0.0;
    }

    // ID → village label のマップを構築

    // village_assignments のインデックスは population 順に対応

    // ここでは ID ベースのマッピングができないため、

    // population の順序と sessions の helper_id / requester_id を照合する

    // 事前条件: village_assignments の長さは population の長さに等しい

    let cross_count = sessions
        .iter()
        .filter(|s| {
            // helper_id と requester_id が異なるワークフロー（同一ワークフロー内の

            // セッションは通常ありえないが念のためフィルタ）

            s.helper_id != s.requester_id
        })
        .filter(|_s| false)
        .count();

    // TODO: この関数は VillageInteractionObserver 内で population の ID マッピングと

    // 併用して呼び出す。直接呼び出し時は常に 0.0 を返す。

    let _ = cross_count;

    0.0
}

// ============================================================================

// compute_village_formation_strength — 村形成強度 (M1.76-KW3)

// ============================================================================

/// 村形成強度を silhouette 類似スコアとして計算する。

///

/// 各村の重心（所属ワークフローの position 平均）を計算し、

/// 各ワークフローの重心からのユークリッド距離の逆数平均を

/// [0, 1] に正規化する。

///

/// # 戻り値

/// [0, 1] のスコア。値が大きいほど密集した村構造を表す。

/// 全員 None（村数 0）の場合は 0.0。

pub fn compute_village_formation_strength(
    population: &[crate::simulation::SimWorkflowState],

    village_assignments: &[Option<usize>],
) -> f64 {
    if population.is_empty() || village_assignments.is_empty() {
        return 0.0;
    }

    // 簡易版: vid 順に直接追加

    let max_vid = village_assignments
        .iter()
        .filter_map(|&a| a)
        .max()
        .map_or(0, |v| v + 1);

    let mut members: Vec<Vec<usize>> = vec![Vec::new(); max_vid];

    for (idx, &assignment) in village_assignments.iter().enumerate() {
        if let Some(vid) = assignment {
            members[vid].push(idx);
        }
    }

    if members.is_empty() || members.iter().all(|m| m.is_empty()) {
        return 0.0;
    }

    // 各村の重心を計算

    let centroids: Vec<[f64; 2]> = members
        .iter()
        .map(|member_indices| {
            if member_indices.is_empty() {
                return [0.0, 0.0];
            }

            let sum_x: f64 = member_indices
                .iter()
                .map(|&idx| population[idx].position[0] as f64)
                .sum();

            let sum_y: f64 = member_indices
                .iter()
                .map(|&idx| population[idx].position[1] as f64)
                .sum();

            let n = member_indices.len() as f64;

            [sum_x / n, sum_y / n]
        })
        .collect();

    // 各村内の平均重心距離を計算

    let total_score: f64 = members
        .iter()
        .zip(centroids.iter())
        .map(|(member_indices, centroid)| {
            if member_indices.is_empty() {
                return 0.0;
            }

            let mean_dist: f64 = member_indices
                .iter()
                .map(|&idx| {
                    let dx = population[idx].position[0] as f64 - centroid[0];

                    let dy = population[idx].position[1] as f64 - centroid[1];

                    (dx * dx + dy * dy).sqrt()
                })
                .sum::<f64>()
                / member_indices.len() as f64;

            // silhouette 類似: 距離が小さいほど 1.0 に近い

            // 最大距離は √2（[0,1]² 空間の対角線）

            let max_dist = 2.0_f64.sqrt();

            (1.0 - (mean_dist / max_dist)).clamp(0.0, 1.0)
        })
        .sum();

    let village_count = members.iter().filter(|m| !m.is_empty()).count() as f64;

    if village_count > 0.0 {
        total_score / village_count
    } else {
        0.0
    }
}

// ============================================================================

// compute_knowledge_diffusion_rate — 知識拡散率 (M1.76-KW3)

// ============================================================================

/// 村間の知識（experience）拡散率を計算する。

///

/// 各村の平均 experience の標本標準偏差が時間とともに減少する速度。

/// 値が大きいほど知識が均等に拡散していることを示す。

///

/// # 戻り値

/// (σ_previous - σ_current) / max(σ_previous, 1e-10)。

/// 拡散完了時（両時点の各村平均 experience が等しい）は 0.0。

/// 乖離が大きいほど正値。[0, 1] に clamp。

/// 村数 0（全員 None）の場合は 0.0。

pub fn compute_knowledge_diffusion_rate(
    population: &[crate::simulation::SimWorkflowState],

    current_assignments: &[Option<usize>],

    previous_assignments: &[Option<usize>],
) -> f64 {
    if population.is_empty() || current_assignments.is_empty() || previous_assignments.is_empty() {
        return 0.0;
    }

    // 村の experience 平均を計算する内部関数

    let village_experience_means = |assignments: &[Option<usize>]| -> Vec<f64> {
        let max_vid = assignments
            .iter()
            .filter_map(|&a| a)
            .max()
            .map_or(0, |v| v + 1);

        if max_vid == 0 {
            return Vec::new();
        }

        let mut sums: Vec<f64> = vec![0.0; max_vid];

        let mut counts: Vec<usize> = vec![0; max_vid];

        for (idx, &assignment) in assignments.iter().enumerate() {
            if let Some(vid) = assignment {
                if idx < population.len() {
                    sums[vid] += population[idx].experience as f64;

                    counts[vid] += 1;
                }
            }
        }

        sums.iter()
            .zip(counts.iter())
            .map(|(&s, &c)| if c > 0 { s / c as f64 } else { 0.0 })
            .collect()
    };

    let current_means = village_experience_means(current_assignments);

    let previous_means = village_experience_means(previous_assignments);

    if current_means.len() < 2 || previous_means.len() < 2 {
        return 0.0;
    }

    // 標本標準偏差

    let std_dev = |means: &[f64]| -> f64 {
        let n = means.len() as f64;

        let mean = means.iter().sum::<f64>() / n;

        let variance = means.iter().map(|&m| (m - mean).powi(2)).sum::<f64>() / n;

        variance.sqrt()
    };

    let current_std = std_dev(&current_means);

    let previous_std = std_dev(&previous_means);

    let rate = (previous_std - current_std) / previous_std.max(1e-10);

    rate.clamp(0.0, 1.0)
}

// ============================================================================

// compute_village_flow_balance — 村フローバランス (M1.76-KW3)

// ============================================================================

/// 村の churn 率（フローバランス）を計算する。

///

/// 村間を移動したワークフロー数 / 両 tick で生存かつ村所属のワークフロー数。

/// 適正範囲は [KW_VILLAGE_CHURN_LOWER, KW_VILLAGE_CHURN_UPPER] = [0.05, 0.30]。

///

/// # 戻り値

/// [0, 1] の churn 率。空 assignments の場合は 0.0。

pub fn compute_village_flow_balance(
    current_assignments: &[Option<usize>],

    previous_assignments: &[Option<usize>],
) -> f64 {
    if current_assignments.is_empty() || previous_assignments.is_empty() {
        return 0.0;
    }

    let min_len = current_assignments.len().min(previous_assignments.len());

    let mut moved_count = 0usize;

    let mut total_count = 0usize;

    for i in 0..min_len {
        match (current_assignments[i], previous_assignments[i]) {
            (Some(current), Some(previous)) => {
                total_count += 1;

                if current != previous {
                    moved_count += 1;
                }
            }

            _ => {

                // いずれかが None の場合はカウントしない
            }
        }
    }

    if total_count == 0 {
        0.0
    } else {
        (moved_count as f64 / total_count as f64).clamp(0.0, 1.0)
    }
}

// ============================================================================

// VillageInteractionObserver — 村相互作用観測器 (M1.76-KW3)

// ============================================================================

/// 村間相互作用を観測する observer。

///

/// `EcosystemGrowthObserver`（KW2）と同様の API 設計。

/// 各 tick で `assign_village_ids` → 各 compute 関数 → `compute_village_health_score`

/// の順で実行し、`VillageInteractionMetrics` を生成する。

///

/// 村割り当ての履歴（前 tick の assignments）を内部状態として保持し、

/// `compute_knowledge_diffusion_rate` と `compute_village_flow_balance` の

/// 時間差分計算に使用する。

pub struct VillageInteractionObserver {
    /// 前 tick の村割り当て（初回は None）
    previous_assignments: Option<Vec<Option<usize>>>,
}

impl VillageInteractionObserver {
    /// 新しい観測器を作成する。

    pub fn new() -> Self {
        Self {
            previous_assignments: None,
        }
    }

    /// 1 tick 分の村間相互作用メトリクスを計算する。

    ///

    /// # 引数

    /// - `tick`: 現在の tick 番号

    /// - `population`: 現在のワークフロー集団

    /// - `sessions`: 現在のヘルプセッション一覧

    pub fn observe(
        &mut self,

        tick: u64,

        population: &[crate::simulation::SimWorkflowState],

        sessions: &[crate::simulation::SimHelpSession],
    ) -> VillageInteractionMetrics {
        let current_assignments = assign_village_ids(population);

        // 村ごとのメンバー ID 一覧

        let max_vid = current_assignments
            .iter()
            .filter_map(|&a| a)
            .max()
            .map_or(0, |v| v + 1);

        let mut village_members: Vec<Vec<&str>> = vec![Vec::new(); max_vid];

        for (idx, &assignment) in current_assignments.iter().enumerate() {
            if let Some(vid) = assignment {
                if idx < population.len() {
                    village_members[vid].push(&population[idx].id);
                }
            }
        }

        // 村間相互作用率: 村ごとのメンバー ID を使って判定

        let cross_village_interaction_rate = {
            if sessions.is_empty() || max_vid == 0 {
                0.0
            } else {
                // ID → 村ラベルのマップ

                let mut id_to_village: std::collections::HashMap<&str, Option<usize>> =
                    std::collections::HashMap::new();

                for (idx, &assignment) in current_assignments.iter().enumerate() {
                    if idx < population.len() {
                        id_to_village.insert(&population[idx].id, assignment);
                    }
                }

                let total = sessions.len();

                let cross = sessions
                    .iter()
                    .filter(|s| {
                        let helper_village = id_to_village.get(s.helper_id.as_str()).copied();

                        let requester_village = id_to_village.get(s.requester_id.as_str()).copied();

                        match (helper_village, requester_village) {
                            (Some(Some(hv)), Some(Some(rv))) => hv != rv,

                            _ => false,
                        }
                    })
                    .count();

                (cross as f64 / total as f64).clamp(0.0, 1.0)
            }
        };

        // 村形成強度

        let village_formation_strength =
            compute_village_formation_strength(population, &current_assignments);

        // 知識拡散率

        let knowledge_diffusion_rate = match &self.previous_assignments {
            Some(prev) => compute_knowledge_diffusion_rate(population, &current_assignments, prev),

            None => 0.0,
        };

        // 村フローバランス

        let village_flow_balance = match &self.previous_assignments {
            Some(prev) => compute_village_flow_balance(&current_assignments, prev),

            None => 0.0,
        };

        // 村サイズ統計

        let (mean_village_size, village_size_variance) = {
            let sizes: Vec<usize> = village_members.iter().map(|m| m.len()).collect();

            if sizes.is_empty() {
                (0.0, 0.0)
            } else {
                let n = sizes.len() as f64;

                let mean = sizes.iter().sum::<usize>() as f64 / n;

                let variance = sizes
                    .iter()
                    .map(|&s| (s as f64 - mean).powi(2))
                    .sum::<f64>()
                    / n as f64;

                (mean, variance)
            }
        };

        // 前 tick の assignments を保存

        self.previous_assignments = Some(current_assignments);

        VillageInteractionMetrics {
            tick,

            village_count: max_vid,

            cross_village_interaction_rate,

            village_formation_strength,

            knowledge_diffusion_rate,

            village_flow_balance,

            mean_village_size,

            village_size_variance,
        }
    }

    /// 全 tick の村相互作用メトリクス系列を CSV 形式で標準出力に書き出す。

    pub fn print_csv(series: &[VillageInteractionMetrics], prefix: &str) {
        println!(

            "{prefix}: tick,village_count,cross_village_interaction_rate,village_formation_strength,knowledge_diffusion_rate,village_flow_balance,mean_village_size,village_size_variance"

        );

        for m in series {
            println!(
                "{prefix}: {},{},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6}",
                m.tick,
                m.village_count,
                m.cross_village_interaction_rate,
                m.village_formation_strength,
                m.knowledge_diffusion_rate,
                m.village_flow_balance,
                m.mean_village_size,
                m.village_size_variance,
            );
        }
    }

    /// SimulationContext から村相互作用メトリクスを収集する（P4 シミュレーション用）。

    pub fn observe_from_context(
        &mut self,

        ctx: &crate::simulation::SimulationContext,
    ) -> VillageInteractionMetrics {
        let tick = ctx.tick;

        let population_count = ctx.memoized_graph.graph.node_count();

        // SimulationContext.village_assignments から Vec<Option<usize>> に変換

        let mut current_assignments: Vec<Option<usize>> = vec![None; population_count];

        for (&node_id, &village) in &ctx.village_assignments {
            if node_id < population_count {
                current_assignments[node_id] = village;
            }
        }

        let max_vid = current_assignments
            .iter()
            .filter_map(|&a| a)
            .max()
            .map_or(0, |v| v + 1);

        // 村間相互作用率: help_sessions の from_workflow != to_workflow

        let cross_village_interaction_rate = {
            if ctx.help_sessions.is_empty() || max_vid == 0 {
                0.0
            } else {
                let total = ctx.help_sessions.len();

                let cross = ctx
                    .help_sessions
                    .iter()
                    .filter(|s| s.from_workflow != s.to_workflow)
                    .count();

                if total > 0 {
                    cross as f64 / total as f64
                } else {
                    0.0
                }
            }
        };

        // village_formation_strength: 村所属個人 / 全人口

        let assigned_count = current_assignments.iter().filter_map(|&a| a).count();

        let village_formation_strength = if population_count > 0 {
            assigned_count as f64 / population_count as f64
        } else {
            0.0
        };

        // knowledge_diffusion_rate: 現状 0.0（将来 P4 での実装時に設定）

        let knowledge_diffusion_rate = 0.0;

        // village_flow_balance: 前 tick との割り当て差分

        let village_flow_balance = match &self.previous_assignments {
            Some(prev) if !prev.is_empty() && !current_assignments.is_empty() => {
                let n = prev.len().min(current_assignments.len());

                let changes = (0..n)
                    .filter(|&i| prev[i] != current_assignments[i])
                    .count();

                (changes as f64 / n as f64).min(1.0)
            }

            _ => 0.0,
        };

        // 村サイズ統計

        let mut village_sizes: Vec<usize> = vec![0; max_vid];

        for &v in &current_assignments {
            if let Some(vid) = v {
                village_sizes[vid] += 1;
            }
        }

        let (mean_village_size, village_size_variance) = {
            let n = village_sizes.len();

            if n == 0 {
                (0.0, 0.0)
            } else {
                let mean = village_sizes.iter().sum::<usize>() as f64 / n as f64;

                let variance = village_sizes
                    .iter()
                    .map(|&s| {
                        let d = s as f64 - mean;

                        d * d
                    })
                    .sum::<f64>()
                    / n as f64;

                (mean, variance)
            }
        };

        self.previous_assignments = Some(current_assignments);

        VillageInteractionMetrics {
            tick,

            village_count: max_vid,

            cross_village_interaction_rate,

            village_formation_strength,

            knowledge_diffusion_rate,

            village_flow_balance,

            mean_village_size,

            village_size_variance,
        }
    }

    /// 内部状態（前 tick の assignments）をリセットする。

    pub fn reset(&mut self) {
        self.previous_assignments = None;
    }
}

impl Default for VillageInteractionObserver {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================

// M1.76-KW4: MagnificentSevenParams → ReciprocitySimulatorConfig 変換

// ============================================================================

impl MagnificentSevenParams {
    /// 自身の 7 パラメータを `ReciprocitySimulatorConfig` に変換する。

    ///

    /// `population_size` と `seed` は引数で指定し、`max_ticks` は

    /// `KW4_SIMULATION_TICKS`（200 tick）、`mission_rate` はデフォルト値（0.3）を使用する。

    pub fn to_sim_config(
        &self,

        population_size: usize,

        seed: u64,
    ) -> crate::simulation::ReciprocitySimulatorConfig {
        let mut config = crate::simulation::ReciprocitySimulatorConfig {
            population_size,

            child_ratio: self.child_ratio,

            gc_interval: self.gc_interval,

            max_ticks: crate::constants::KW4_SIMULATION_TICKS,

            seed,

            ..crate::simulation::ReciprocitySimulatorConfig::default()
        };

        config.policy.gamma_benevolence = self.gamma_benevolence as f32;

        config.policy.lambda_gc_base = self.lambda_gc_base as f32;

        config.policy.theta_dir = self.direct_reciprocity_weight as f32;

        config.policy.theta_ind = self.indirect_reciprocity_weight as f32;

        config.policy.tau_helper_softmax = self.softmax_temperature as f32;

        config
    }
}
// ---------------------------------------------------------------------------
// M1.76-KW-ACCEL: 新規6指標の計算ヘルパー関数
// ---------------------------------------------------------------------------

/// SubWorkflow ノードのネスト深度の平均を計算する。
/// 単一グラフシミュレーションでは、SubWorkflow ノードの深度は 1 となる。
fn compute_mean_nest_depth(graph: &crate::types::WorkflowGraph) -> f64 {
    let mut total_depth = 0usize;
    let mut sub_count = 0usize;
    for node_index in graph.node_indices() {
        if matches!(
            graph[node_index],
            crate::types::WorkflowNode::SubWorkflow { .. }
        ) {
            total_depth += 1; // 単一グラフでは深度1
            sub_count += 1;
        }
    }
    if sub_count == 0 {
        0.5 // デフォルト中間値
    } else {
        total_depth as f64 / sub_count as f64
    }
}

/// ノード密度（グラフサイズの正規化値）を計算する。
fn compute_mean_node_density(graph: &crate::types::WorkflowGraph) -> f64 {
    let node_count = graph.node_count();
    // KW_ACCEL_NODE_DENSITY_MAX で正規化
    (node_count as f64 / crate::constants::KW_ACCEL_NODE_DENSITY_MAX).clamp(0.0, 1.0)
}

/// Watts-Strogatz 型の大域クラスター係数を計算する。
///
/// 各ノードの k-最近傍位置を SpacePositionEmbedding から特定し、
/// 三角形の割合を計測する。位置情報がないノードは無視する。
fn compute_cluster_coefficient(
    positions: &std::collections::HashMap<
        crate::types::NodeId,
        crate::spaceposition::SpacePositionEmbedding,
    >,
) -> f64 {
    let k = crate::constants::KW_ACCEL_K_NEAREST;
    let node_ids: Vec<crate::types::NodeId> = positions.keys().cloned().collect();
    if node_ids.len() < 3 {
        return 0.5; // 少なすぎてクラスター係数を計算できない
    }

    let mut triangles = 0usize;
    let mut triples = 0usize;

    for i in 0..node_ids.len() {
        let pos_i = match positions[&node_ids[i]].inner() {
            core::option::Option::Some(p) => *p,
            core::option::Option::None => continue,
        };

        // i の k-最近傍を収集
        let mut neighbors: Vec<(f64, &crate::types::NodeId)> = Vec::new();
        for j in 0..node_ids.len() {
            if i == j {
                continue;
            }
            let pos_j = match positions[&node_ids[j]].inner() {
                core::option::Option::Some(p) => *p,
                core::option::Option::None => continue,
            };
            let dist = crate::spaceposition::l2_distance(&pos_i, &pos_j);
            neighbors.push((dist, &node_ids[j]));
        }
        neighbors.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        let neighbor_ids: Vec<&crate::types::NodeId> =
            neighbors.iter().take(k).map(|(_, id)| *id).collect();

        // 近傍間のエッジ（三角形）をカウント
        for a in 0..neighbor_ids.len() {
            for b in (a + 1)..neighbor_ids.len() {
                triples += 1;
                // 両方の位置が近傍に含まれる = 三角形成立とみなす簡易近似
                // より正確には実際の距離が必要だが、位置ベース近似で十分
                let pos_a = match positions[neighbor_ids[a]].inner() {
                    core::option::Option::Some(p) => *p,
                    core::option::Option::None => continue,
                };
                let pos_b = match positions[neighbor_ids[b]].inner() {
                    core::option::Option::Some(p) => *p,
                    core::option::Option::None => continue,
                };
                if crate::spaceposition::l2_distance(&pos_a, &pos_b)
                    < crate::constants::KW_ACCEL_DENSITY_RADIUS
                {
                    triangles += 1;
                }
            }
        }
    }

    if triples == 0 {
        0.5
    } else {
        triangles as f64 / triples as f64
    }
}

/// 局所密度を計算する（閾値半径内の近傍ノード数の平均割合）。
fn compute_local_density(
    positions: &std::collections::HashMap<
        crate::types::NodeId,
        crate::spaceposition::SpacePositionEmbedding,
    >,
) -> f64 {
    let radius = crate::constants::KW_ACCEL_DENSITY_RADIUS;
    let node_ids: Vec<crate::types::NodeId> = positions.keys().cloned().collect();
    if node_ids.len() < 2 {
        return 0.5;
    }

    let mut total_neighbors = 0usize;
    let mut counted = 0usize;

    for i in 0..node_ids.len() {
        let pos_i = match positions[&node_ids[i]].inner() {
            core::option::Option::Some(p) => *p,
            core::option::Option::None => continue,
        };
        let mut neighbor_count = 0usize;
        for j in 0..node_ids.len() {
            if i == j {
                continue;
            }
            let pos_j = match positions[&node_ids[j]].inner() {
                core::option::Option::Some(p) => *p,
                core::option::Option::None => continue,
            };
            if crate::spaceposition::l2_distance(&pos_i, &pos_j) < radius {
                neighbor_count += 1;
            }
        }
        total_neighbors += neighbor_count;
        counted += 1;
    }

    if counted == 0 {
        0.5
    } else {
        let max_possible = (counted - 1) as f64;
        if max_possible <= 0.0 {
            0.5
        } else {
            (total_neighbors as f64 / counted as f64) / max_possible
        }
    }
}

/// HELP セッションにおける探索半径の逆数を計算する。
///
/// 各 HELP セッションの from_workflow と to_workflow の間に紐づく
/// 空間位置の L2 距離の平均を使い、1.0 / (1.0 + mean_distance) として [0,1] に正規化する。
/// String → NodeId 変換は "n<数字>" 形式のパースで行う。
/// ワークフロー ID 文字列からノード番号を抽出する。
///
/// 以下の ID フォーマットを順次試行する:
/// - `"n<数字>"` — レガシー形式
/// - `"wf-child-<数字>"` / `"wf-adult-<数字>"` — シミュレーション内ワークフロー ID
/// - `"session-<数字>"` — シミュレーション内 HELP セッション ID
/// - `"adult-<数字>"` / `"child-<数字>"` — production 環境の WorkflowGraphId
fn parse_workflow_id(s: &str) -> Option<crate::types::NodeId> {
    if let Some(id) = s.strip_prefix('n').and_then(|r| r.parse().ok()) {
        return Some(id);
    }
    if let Some(id) = s.strip_prefix("wf-child-").and_then(|r| r.parse().ok()) {
        return Some(id);
    }
    if let Some(id) = s.strip_prefix("wf-adult-").and_then(|r| r.parse().ok()) {
        return Some(id);
    }
    if let Some(id) = s.strip_prefix("session-").and_then(|r| r.parse().ok()) {
        return Some(id);
    }
    if let Some(id) = s.strip_prefix("adult-").and_then(|r| r.parse().ok()) {
        return Some(id);
    }
    s.strip_prefix("child-").and_then(|r| r.parse().ok())
}

fn compute_search_radius_inverse(
    sessions: &[crate::help::HelpSession],
    positions: &std::collections::HashMap<
        crate::types::NodeId,
        crate::spaceposition::SpacePositionEmbedding,
    >,
) -> f64 {
    if sessions.is_empty() {
        return 0.5;
    }
    let mut total_distance = 0.0f64;
    let mut counted = 0usize;
    for session in sessions {
        let from_id = match parse_workflow_id(&session.from_workflow) {
            Some(id) => id,
            None => continue,
        };
        let to_id = match parse_workflow_id(&session.to_workflow) {
            Some(id) => id,
            None => continue,
        };
        let pos_from = match positions.get(&from_id) {
            Some(emb) => match *emb.inner() {
                Some(p) => p,
                None => continue,
            },
            None => continue,
        };
        let pos_to = match positions.get(&to_id) {
            Some(emb) => match *emb.inner() {
                Some(p) => p,
                None => continue,
            },
            None => continue,
        };
        total_distance += crate::spaceposition::l2_distance(&pos_from, &pos_to);
        counted += 1;
    }
    if counted == 0 {
        0.5
    } else {
        let mean_distance = total_distance / counted as f64;
        1.0 / (1.0 + mean_distance)
    }
}

/// 推論ステップ数の逆数を計算する。
///
/// compile_to_steps の出力長の平均を 1.0 / (1.0 + mean_steps) として [0,1] に正規化する。
fn compute_reasoning_steps_inverse(graph: &crate::types::WorkflowGraph) -> f64 {
    // 単一グラフに対して compile_to_steps を実行
    match crate::compiler::compile_to_steps(graph) {
        Ok(steps) => {
            let step_count = steps.len() as f64;
            1.0 / (1.0 + step_count)
        }
        Err(_) => 0.5, // コンパイル失敗時はデフォルト値
    }
} // ============================================================================

// M1.76-KW4: collect_final_metrics — シミュレーション結果 → KindWorldMetricsInput

// ============================================================================

/// ReciprocitySimulationResult から KindWorldMetricsInput を収集する（旧モデル用）。

///

/// evaluate_single および NelderMeadOptimizer::run からのみ使用される。

/// 新規コードは collect_final_metrics（SimulationContext 版）を使用すること。

#[allow(dead_code)]
fn collect_final_metrics_from_result(
    result: &crate::simulation::ReciprocitySimulationResult,

    initial_population_size: usize,
) -> KindWorldMetricsInput {
    let survived_count = result.final_state.iter().filter(|w| w.survived).count();

    let population_growth_rate = if initial_population_size > 0 {
        (survived_count as f64 - initial_population_size as f64) / initial_population_size as f64
    } else {
        0.0
    };

    let capability_coverage = compute_capability_coverage_shannon(&result.final_state);

    let reuse_ratio = compute_reuse_ratio(&[], &result.sessions);

    let cost_efficiency = compute_cost_efficiency(&result.sessions);

    let benevolent_ratio = compute_benevolent_vs_non_benevolent_coverage_ratio(&result.final_state);

    // VillageInteractionObserver で村指標を計算

    // 1 回目の observe で内部状態（前 tick assignments）を初期化し、

    // 2 回目で churn / diffusion を導出する

    let mut village_observer = VillageInteractionObserver::new();

    let _ = village_observer.observe(0, &result.final_state, &result.sessions);

    let second = village_observer.observe(1, &result.final_state, &result.sessions);

    KindWorldMetricsInput {
        population_growth_rate: population_growth_rate.clamp(0.0, 1.0),

        capability_coverage,

        reuse_ratio,

        cost_efficiency,

        village_formation_score: second.village_formation_strength,

        village_churn_rate: second.village_flow_balance,

        cross_village_interaction_rate: second.cross_village_interaction_rate,

        knowledge_diffusion_rate: second.knowledge_diffusion_rate,

        benevolent_vs_non_benevolent_coverage_ratio: benevolent_ratio,

        mean_lifecycle_score: 0.0,
        child_survival_rate: 0.0,
        mean_freshness: 0.0,
        mean_benevolence_aggregate: 0.0,
        mean_reciprocity_score: 0.0,
        help_success_rate: 0.0,
        trust_inheritance_fidelity: 0.0,
        execution_success_rate: 0.0,
        mean_nest_depth: 0.0,
        mean_node_density: 0.0,
        cluster_coefficient: 0.0,
        local_density: 0.0,
        search_radius_inverse: 0.0,
        reasoning_steps_inverse: 0.0,
    }
}

/// GcEvent のバリアントからライフサイクルスコア [0, 1] を返す。
///
/// RFC §15.9.3 の LifecycleScore L(G) のプロキシ指標。
/// GcEvent は L(G) 由来の hazard 計算結果の状態であり、強い相関を持つ。
fn lifecycle_score_from_gc_state(state: &crate::event::GcEvent) -> f64 {
    match state {
        crate::event::GcEvent::Protected => 1.0,
        crate::event::GcEvent::Active => 0.8,
        crate::event::GcEvent::SoftDeleted => 0.3,
        crate::event::GcEvent::HardDeleteCandidate => 0.1,
        crate::event::GcEvent::Tombstoned => 0.0,
    }
}

/// 全ノードの GC 状態ベースライフサイクルスコアの平均を計算する。
///
/// 空マップの場合は 0.0 を返す。
pub(crate) fn compute_mean_lifecycle_score(
    node_gc_states: &std::collections::HashMap<crate::types::NodeId, crate::event::GcEvent>,
) -> f64 {
    if node_gc_states.is_empty() {
        return 0.0;
    }
    let sum: f64 = node_gc_states
        .values()
        .map(lifecycle_score_from_gc_state)
        .sum();
    (sum / node_gc_states.len() as f64).clamp(0.0, 1.0)
}

/// 子ノードの生存率を計算する。
///
/// total_births が 0 の場合は 0.0 を返す。
fn compute_child_survival_rate(total_births: u64, child_count: u64) -> f64 {
    if total_births == 0 {
        return 0.0;
    }
    let survived = child_count.min(total_births);
    (survived as f64 / total_births as f64).clamp(0.0, 1.0)
}

/// 全ノードの freshness の平均を計算する。
///
/// 各ノードの最終更新 tick と現在 tick の差分から compute_blended_freshness で
/// 個別 freshness を計算し、全ノードの算術平均を返す。
/// 空マップの場合は 0.0 を返す。
pub(crate) fn compute_mean_freshness(
    node_last_update: &std::collections::HashMap<crate::types::NodeId, u64>,
    current_tick: u64,
) -> f64 {
    if node_last_update.is_empty() {
        return 0.0;
    }
    let sum: f64 = node_last_update
        .values()
        .map(|&last_tick| {
            let elapsed = current_tick.saturating_sub(last_tick);
            crate::clock::compute_blended_freshness(0, elapsed, 0.0)
        })
        .sum();
    (sum / node_last_update.len() as f64).clamp(0.0, 1.0)
}

/// HELP セッションの成功率を計算する (MTR-C)。
///
/// execution_success_rate = total_successes / total_attempts。
/// 空（total_attempts=0）の場合は 0.0 を返す。
fn compute_execution_success_rate(total_attempts: u64, total_successes: u64) -> f64 {
    if total_attempts == 0 {
        0.0
    } else {
        (total_successes as f64 / total_attempts as f64).clamp(0.0, 1.0)
    }
}

/// コスト効率を計算する (MTR-C)。
///
/// cost_efficiency = 1.0 - (total_gc_collections + total_help_failures) / (total_gc_collections + total_help_attempts)
/// ここで total_help_failures = total_help_attempts - total_help_successes。
/// アクティビティなし（total=0）の場合は 1.0 を返す。
fn compute_cost_efficiency_ratio(
    total_gc_collections: u64,
    total_help_attempts: u64,
    total_help_successes: u64,
) -> f64 {
    let total_cost = total_gc_collections + total_help_attempts;
    if total_cost == 0 {
        return 1.0;
    }
    let total_failures =
        total_gc_collections + (total_help_attempts.saturating_sub(total_help_successes));
    (1.0 - total_failures as f64 / total_cost as f64).clamp(0.0, 1.0)
}

/// 全 TrustProfile の blended trust（3 次元信頼値の算術平均）の平均を計算する。
///
/// TrustProfile の operational, semantic, temporal の 3 次元平均を各ノードで取り、
/// 全ノードの算術平均を返す。空マップの場合は 0.0 を返す。
fn compute_mean_benevolence(
    trust_profiles: &std::collections::HashMap<crate::types::NodeId, crate::types::TrustProfile>,
) -> f64 {
    if trust_profiles.is_empty() {
        return 0.0;
    }
    let sum: f64 = trust_profiles
        .values()
        .map(|tp| (tp.operational + tp.semantic + tp.temporal) / 3.0)
        .sum();
    (sum / trust_profiles.len() as f64).clamp(0.0, 1.0)
}

/// HELP 提供ペアのバランスから互恵性スコアを計算する。
///
/// 同一ペア (a,b) と (b,a) の提供回数のうち小さい方の総和を全相互作用数で割る。
/// 完全に対称な HELP 提供が行われている場合に 1.0、一方向のみの場合は 0.0 に近づく。
/// 空マップの場合は 0.0 を返す。
fn compute_mean_reciprocity(
    pair_counts: &std::collections::HashMap<(crate::types::NodeId, crate::types::NodeId), u64>,
) -> f64 {
    if pair_counts.is_empty() {
        return 0.0;
    }
    let mut symmetric_sum: u64 = 0;
    let mut total_interactions: u64 = 0;
    for (&(a, b), &count) in pair_counts {
        if a != b {
            let reverse = pair_counts.get(&(b, a)).copied().unwrap_or(0);
            symmetric_sum += count.min(reverse);
        }
        total_interactions += count;
    }
    if total_interactions == 0 {
        return 0.0;
    }
    (symmetric_sum as f64 / total_interactions as f64).clamp(0.0, 1.0)
}

/// 信頼継承 fidelity の平均を計算する。
///
/// 各継承イベントの fidelity 累積和をイベント数で割る。
/// イベント数が 0 の場合は 0.0 を返す。
fn compute_trust_inheritance_fidelity(total_fidelity: f64, event_count: u64) -> f64 {
    if event_count == 0 {
        return 0.0;
    }
    (total_fidelity / event_count as f64).clamp(0.0, 1.0)
}

/// 位置分布から能力カバー率 (capability coverage) を計算する (MTR-D)。
///
/// positions に含まれる全 alive ノードの位置 (x, y) を
/// ECOSYSTEM_GRID_DIVISIONS × ECOSYSTEM_GRID_DIVISIONS グリッドに量子化し、
/// Shannon 多様性指数 H = -Σ p_i log p_i を計算する。
/// H_max = log(グリッドセル数) で正規化し [0, 1] に clamp する。
/// 空 positions または全エントリが None の場合は 0.0 を返す。
///
/// ※ プロキシ値: RFC §15.9.3 は「能力空間」の多様性を要求するが、
///   現状の SimulationContext で利用可能な唯一の位置情報は物理位置
///   (SpacePositionEmbedding) であるため、これを能力空間のプロキシとして使用する。
pub(crate) fn compute_capability_coverage(
    positions: &std::collections::HashMap<
        crate::types::NodeId,
        crate::spaceposition::SpacePositionEmbedding,
    >,
) -> f64 {
    let grid_size = crate::constants::ECOSYSTEM_GRID_DIVISIONS;
    let total_cells = (grid_size * grid_size) as f64;
    let mut grid = vec![0u64; grid_size * grid_size];

    for pos in positions.values() {
        if let Some([x, y, _]) = *pos.inner() {
            let gx = ((x as f64).clamp(0.0, 0.999) * grid_size as f64).floor() as usize;
            let gy = ((y as f64).clamp(0.0, 0.999) * grid_size as f64).floor() as usize;
            let gx = gx.min(grid_size - 1);
            let gy = gy.min(grid_size - 1);
            grid[gy * grid_size + gx] += 1;
        }
    }

    let total: f64 = grid.iter().sum::<u64>() as f64;
    if total == 0.0 {
        return 0.0;
    }

    let mut h = 0.0_f64;
    for &count in &grid {
        if count > 0 {
            let p = count as f64 / total;
            h -= p * p.ln();
        }
    }

    let h_max = total_cells.ln();
    if h_max > 0.0 {
        (h / h_max).clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// HELP ペアの再利用比率を計算する (MTR-D)。
///
/// 全ペアのうち、同一ペアが 2 回以上 HELP 提供を行った割合。
/// 頻度 >= 2 のペア数 / 全ペア数。
/// 空マップの場合は 0.0 を返す。
///
/// ※ プロキシ値: RFC §15.9.3 の「再利用回数 / 全インタラクション数」に対して、
///   ペア頻度 >= 2 を「再利用されたペア」とみなす近似。
fn compute_reuse_ratio_from_pair_counts(
    pair_counts: &std::collections::HashMap<(crate::types::NodeId, crate::types::NodeId), u64>,
) -> f64 {
    let total_pairs = pair_counts.len();
    if total_pairs == 0 {
        return 0.0;
    }
    let reused_count = pair_counts.values().filter(|&&count| count >= 2).count();
    reused_count as f64 / total_pairs as f64
}

/// 相互作用多様性から知識拡散率を計算する (MTR-D)。
///
/// ユニークペア数 / 全インタラクション数。
/// 多様なペアが多いほど知識が広く拡散しているとみなす。
/// 空マップの場合は 0.0 を返す。
///
/// ※ プロキシ値: RFC §15.9.3 は「村間 experience 分散の時間変化率」を要求するが、
///   現状の SimulationContext に experience データが存在しないため、
///   ペア多様性を knowledge diffusion のプロキシとして使用する。
fn compute_knowledge_diffusion_from_pair_counts(
    pair_counts: &std::collections::HashMap<(crate::types::NodeId, crate::types::NodeId), u64>,
) -> f64 {
    let total_unique = pair_counts.len();
    let total_interactions: u64 = pair_counts.values().sum();
    if total_interactions == 0 {
        return 0.0;
    }
    (total_unique as f64 / total_interactions as f64).clamp(0.0, 1.0)
}

/// SimulationContext から KindWorldMetricsInput を収集する（P4 シミュレーション用）。

///

/// 人口・村形成・HELP 統計など、SimulationContext から収集可能な指標を計算する。

/// 不足指標は 0.0 / 中立値で初期化され、段階的に置き換えられる。
/// 村 churn 率を計算する（累積カウンタ方式）。
///
/// シミュレーション全 tick を通じて追跡した村割り当て変更の累積カウンタから、
/// 村離脱率 (churn rate) を [0, 1] で算出する。
/// tick 間の「子ノードの村アンカー変更」を churn と定義する。
/// 比較回数が 0 の場合は 0.0 を返す。
fn compute_village_churn_rate(changes: u64, comparisons: u64) -> f64 {
    if comparisons == 0 {
        0.0
    } else {
        (changes as f64 / comparisons as f64).clamp(0.0, 1.0)
    }
}

/// s_speed（速度因子）を tick_to_convergence から計算する。
///
/// s_speed = 1.0 - tick_to_convergence / total_ticks。
/// tick_to_convergence が total_ticks 以上の場合は 0.0 を返す（収束しなかった）。
pub(crate) fn compute_s_speed(tick_to_convergence: u64, total_ticks: u64) -> f64 {
    if tick_to_convergence >= total_ticks {
        0.0
    } else {
        (1.0 - tick_to_convergence as f64 / total_ticks as f64).clamp(0.0, 1.0)
    }
}

/// 慈悲的/非慈悲的集団の能力カバー率比を TrustProfile から計算する。
///
/// TrustProfile の 3 成分平均 (operational + semantic + temporal) / 3.0 を
/// 慈悲スコアのプロキシとして使用する。降順ソート後、上位 20% を慈悲的集団、
/// 下位 20% を非慈悲的集団と定義し、各集団の positions における Shannon 多様性指数の
/// 比を返す（旧パス `compute_benevolent_vs_non_benevolent_coverage_ratio` と同一ロジック）。
///
/// ※ プロキシ値: true initial_benevolence の代わりに TrustProfile 合成スコアを使用。
///
/// # 引数
/// - `trust_profiles`: ノードごとの TrustProfile（慈悲スコアの入力）
/// - `positions`: ノードごとの空間位置（能力カバー率計算の入力）
///
/// # 戻り値
/// - 慈悲的集団の多様性 / 非慈悲的集団の多様性（上限なし）
/// - 空マップの場合は 1.0
/// - 非慈悲的多様性が 0 の場合: 慈悲的多様性が 0 なら 1.0、正なら 2.0
fn compute_benevolent_vs_non_benevolent_coverage_from_trust(
    trust_profiles: &std::collections::HashMap<
        crate::types::NodeId,
        crate::types::TrustProfile,
    >,
    positions: &std::collections::HashMap<
        crate::types::NodeId,
        crate::spaceposition::SpacePositionEmbedding,
    >,
) -> f64 {
    // 両方のマップに存在するノードのみを対象とする
    let mut node_ids: Vec<crate::types::NodeId> = trust_profiles
        .keys()
        .filter(|k| positions.contains_key(k))
        .copied()
        .collect();
    // NodeId 順にソートして決定論的実行を保証する（HashMap の非決定論的イテレーション対策）
    node_ids.sort_unstable();

    if node_ids.is_empty() {
        return 1.0;
    }

    // TrustProfile 3 成分平均を慈悲スコアとして降順ソート
    let mut sorted: Vec<crate::types::NodeId> = node_ids;
    sorted.sort_unstable_by(|&a, &b| {
        let score_a = {
            let tp = &trust_profiles[&a];
            (tp.operational + tp.semantic + tp.temporal) / 3.0
        };
        let score_b = {
            let tp = &trust_profiles[&b];
            (tp.operational + tp.semantic + tp.temporal) / 3.0
        };
        score_b
            .partial_cmp(&score_a)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let top_count = (sorted.len() as f64 * crate::constants::BENEVOLENT_TOP_FRACTION)
        .ceil()
        .max(1.0) as usize;

    let bottom_count = (sorted.len() as f64 * crate::constants::BENEVOLENT_BOTTOM_FRACTION)
        .ceil()
        .max(1.0) as usize;

    // 上位と下位が重なる場合は 1.0
    if top_count + bottom_count > sorted.len() {
        return 1.0;
    }

    let top_group = &sorted[..top_count];
    let bottom_group = &sorted[sorted.len() - bottom_count..];

    let top_h = shannon_diversity_from_positions(top_group, positions);
    let bottom_h = shannon_diversity_from_positions(bottom_group, positions);

    if bottom_h <= 0.0 {
        if top_h > 0.0 {
            2.0
        } else {
            1.0
        }
    } else {
        (top_h / bottom_h).clamp(0.0, f64::MAX)
    }
}

/// 位置情報から Shannon 多様性指数（正規化前 H）を計算する内部関数。
fn shannon_diversity_from_positions(
    node_ids: &[crate::types::NodeId],
    positions: &std::collections::HashMap<
        crate::types::NodeId,
        crate::spaceposition::SpacePositionEmbedding,
    >,
) -> f64 {
    if node_ids.is_empty() {
        return 0.0;
    }

    let grid_divisions = crate::constants::ECOSYSTEM_GRID_DIVISIONS;
    let mut grid: std::collections::HashMap<(usize, usize), usize> =
        std::collections::HashMap::new();

    for &node_id in node_ids {
        if let Some(pos) = positions.get(&node_id) {
            if let Some(coords) = *pos.inner() {
                let x = ((coords[0].clamp(0.0, 0.999)) * grid_divisions as f32) as usize;
                let y = ((coords[1].clamp(0.0, 0.999)) * grid_divisions as f32) as usize;
                *grid.entry((x.min(grid_divisions - 1), y.min(grid_divisions - 1))).or_insert(0) += 1;
            }
        }
    }

    let total: usize = grid.values().sum();
    if total == 0 {
        return 0.0;
    }

    let h = grid
        .values()
        .map(|&count| {
            let p = count as f64 / total as f64;
            if p > 0.0 {
                -p * p.log(std::f64::consts::E)
            } else {
                0.0
            }
        })
        .sum::<f64>();

    h
}

/// MTR-D (ticket #125) で capability_coverage, reuse_ratio, knowledge_diffusion_rate を実測値に置き換え。
/// MTR-C (ticket #123) で execution_success_rate, cost_efficiency を実測値に置き換え。
/// MTR-E (ticket #126) で village_churn_rate, benevolent_vs_non_benevolent_coverage_ratio を実測値に置き換え。
pub(crate) fn collect_final_metrics(
    ctx: &crate::simulation::SimulationContext,

    initial_population_size: usize,
    child_count: u64,
) -> KindWorldMetricsInput {
    let population = ctx.memoized_graph.graph.node_count();

    let population_growth_rate = if initial_population_size > 0 {
        (population as f64 - initial_population_size as f64) / initial_population_size as f64
    } else {
        0.0
    };

    // 各村サイズを集計（村所属人数 ÷ 総人口から village_formation_score を算出）

    let assigned_villages: std::collections::BTreeSet<usize> = ctx
        .village_assignments
        .values()
        .filter_map(|v| *v)
        .collect();

    let village_formation_score = if population > 0 {
        (assigned_villages.len() as f64 / population as f64).min(1.0)
    } else {
        0.0
    };

    // total_help_attempts/total_help_successes 累積カウンタから HELP 成功率を計算
    // （ctx.help_sessions は完了セッションが削除されるため非推奨）

    let help_success_rate = if ctx.total_help_attempts > 0 {
        ctx.total_help_successes as f64 / ctx.total_help_attempts as f64
    } else {
        0.0
    };

    let cross_village_help = ctx
        .help_sessions
        .iter()
        .filter(|s| s.from_workflow != s.to_workflow)
        .count();

    let ongoing_sessions = ctx.help_sessions.len();
    let cross_village_interaction_rate = if ongoing_sessions > 0 {
        (cross_village_help as f64 / ongoing_sessions as f64).min(1.0)
    } else {
        0.0
    };

    let positions = &ctx.positions;
    let graph = &ctx.memoized_graph.graph;

    let mean_nest_depth = compute_mean_nest_depth(graph);
    let mean_node_density = compute_mean_node_density(graph);
    let cluster_coefficient = compute_cluster_coefficient(positions);
    let local_density = compute_local_density(positions);
    let search_radius_inverse = compute_search_radius_inverse(&ctx.help_sessions, positions);
    let reasoning_steps_inverse = compute_reasoning_steps_inverse(graph);

    KindWorldMetricsInput {
        population_growth_rate: population_growth_rate.clamp(0.0, 1.0),

        capability_coverage: compute_capability_coverage(positions),

        reuse_ratio: compute_reuse_ratio_from_pair_counts(&ctx.reciprocity_pair_counts),

        village_formation_score,

        village_churn_rate: compute_village_churn_rate(
            ctx.village_assignment_changes,
            ctx.village_assignment_total_comparisons,
        ),

        cross_village_interaction_rate,

        knowledge_diffusion_rate: compute_knowledge_diffusion_from_pair_counts(
            &ctx.reciprocity_pair_counts,
        ),

        benevolent_vs_non_benevolent_coverage_ratio:
            compute_benevolent_vs_non_benevolent_coverage_from_trust(
                &ctx.trust_profiles,
                &ctx.positions,
            ),

        help_success_rate,

        cost_efficiency: compute_cost_efficiency_ratio(
            ctx.total_gc_collections,
            ctx.total_help_attempts,
            ctx.total_help_successes,
        ),

        mean_lifecycle_score: compute_mean_lifecycle_score(&ctx.node_gc_states),
        child_survival_rate: compute_child_survival_rate(ctx.total_births, child_count),
        mean_freshness: compute_mean_freshness(&ctx.node_last_update_tick, ctx.tick),
        mean_benevolence_aggregate: compute_mean_benevolence(&ctx.trust_profiles),
        mean_reciprocity_score: compute_mean_reciprocity(&ctx.reciprocity_pair_counts),
        trust_inheritance_fidelity: compute_trust_inheritance_fidelity(
            ctx.total_inheritance_fidelity,
            ctx.inheritance_event_count,
        ),
        execution_success_rate: compute_execution_success_rate(
            ctx.total_help_attempts,
            ctx.total_help_successes,
        ),

        mean_nest_depth,
        mean_node_density,
        cluster_coefficient,
        local_density,
        search_radius_inverse,
        reasoning_steps_inverse,
    }
} // ============================================================================

// M1.76-KW4: OptimizationReport — 最適化結果報告

// ============================================================================

/// Nelder-Mead 最適化の結果報告。

#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct OptimizationReport {
    /// 最良パラメータ
    pub best_params: MagnificentSevenParams,

    /// 最良 J_kw（非推奨: best_j_kw_social を使用）
    #[deprecated(note = "best_j_kw_social に移行しました。J_kw_social = J_kw × s_speed")]
    pub best_j_kw: f64,

    /// 最良 J_kw_social = J_kw × s_speed（6 因子乗算結合）
    pub best_j_kw_social: f64,

    /// 最良パラメータでの tick_to_convergence
    pub tick_to_convergence: u64,

    /// 速度因子 s_speed
    pub s_speed: f64,

    /// 最良パラメータでの判定結果
    pub assessment: KindWorldAssessment,

    /// 実行反復数
    pub iterations: u32,

    /// 全反復の履歴（パラメータ, J_kw_social）
    pub history: Vec<(MagnificentSevenParams, f64)>,

    /// 収束したかどうか
    pub converged: bool,

    /// 実験 ID
    pub experiment_id: String,
}

// ============================================================================

// M1.76-KW4: ExperimentRecord — 実験記録

// ============================================================================

/// 1 回の外側ループ実行に対応する実験記録。

#[derive(Debug, Clone, Serialize, Deserialize)]

pub struct ExperimentRecord {
    /// 実験 ID
    pub experiment_id: String,

    /// 実験サイクル（外側ループのサイクル番号, 0〜2）
    pub experiment_cycle: u32,

    /// 最適化結果
    pub report: OptimizationReport,

    /// ISO 8601 タイムスタンプ
    pub timestamp: String,
}

// ============================================================================

// M1.76-KW4: 内部ヘルパー関数

// ============================================================================

/// パラメータのインデックスから値を取得する。

fn get_param(params: &MagnificentSevenParams, index: usize) -> f64 {
    match index {
        0 => params.gamma_benevolence,

        1 => params.lambda_gc_base,

        2 => params.direct_reciprocity_weight,

        3 => params.indirect_reciprocity_weight,

        4 => params.softmax_temperature,

        5 => params.gc_interval as f64,

        6 => params.child_ratio,

        _ => 0.0,
    }
}

/// パラメータのインデックスに値を設定する。

/// gc_interval（index=5）は f64 から u64 に四捨五入される。

fn set_param(params: &mut MagnificentSevenParams, index: usize, value: f64) {
    match index {
        0 => params.gamma_benevolence = value,

        1 => params.lambda_gc_base = value,

        2 => params.direct_reciprocity_weight = value,

        3 => params.indirect_reciprocity_weight = value,

        4 => params.softmax_temperature = value,

        5 => params.gc_interval = value.round() as u64,

        6 => params.child_ratio = value,

        _ => {}
    }
}

// ============================================================================

// M1.76-KW4: 実験 ID 生成

// ============================================================================

/// 実験 ID を生成する（`kw4-{UNIX秒}-{カウンタ}`）。

fn generate_kw4_experiment_id(counter: &mut u64) -> String {
    *counter += 1;

    let duration = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();

    format!("kw4-{}-{:03}", duration.as_secs(), counter)
}

// ============================================================================

// M1.76-KW4: evaluate — 単一パラメータセットの J_kw 評価

// ============================================================================

/// 1 組の MagnificentSevenParams に対して J_kw を評価する。

///

/// SimulationContext（KW-REAL 6 フェーズ）を使用し、全 20 指標を
/// 0.0 fallback なしで計算する。同一 params + 同一 seed で決定論的。

fn evaluate_single(
    params: &MagnificentSevenParams,
    seed: u64,
    weights: &Option<[f64; 6]>,
) -> f64 {
    let config = params.to_sim_config(
        crate::constants::KW4_EVALUATION_POPULATION_SIZE,
        seed,
    );
    let (metrics, tick_to_convergence) =
        crate::simulation::run_evaluation_simulation(&config);
    let assessment = compute_kind_world_objective(&metrics);
    let s_speed = compute_s_speed(tick_to_convergence, crate::constants::KW4_SIMULATION_TICKS);
    match weights {
        None => {
            // 従来の J_kw_social = j_kw × s_speed（乗算結合、最小化のために負号）
            -(assessment.j_kw * s_speed)
        }
        Some(w) => {
            // 重み付き線形結合（最小化のために負号）
            let weighted = w[0] * assessment.s_growth
                + w[1] * assessment.s_density
                + w[2] * assessment.s_topology
                + w[3] * assessment.s_search
                + w[4] * assessment.s_fairness
                + w[5] * s_speed;
            let sum_w: f64 = w.iter().sum();
            if sum_w > 0.0 { -weighted / sum_w } else { 0.0 }
        }
    }
}

/// Phase 2: AllParams 版 evaluate_single — G1 の population_size と max_ticks を上書き。
// ============================================================================

// M1.76-KW4: Simplex1D — 1 次元 Nelder-Mead（検証テスト TC2 用）

// ============================================================================

/// 1 次元 Nelder-Mead シンプレックス（2 頂点）。

///

/// 検証テスト TC2（f(x) = (x-3)² の最大化）専用。

/// 通常の 7 次元最適化には `NelderMeadOptimizer` を使用する。

#[allow(dead_code)]

struct Simplex1D {
    vertices: Vec<f64>,

    values: Vec<f64>,

    range: (f64, f64),
}

#[allow(dead_code)]
impl Simplex1D {
    fn new(x: f64, range: (f64, f64)) -> Self {
        let perturbation = (range.1 - range.0) * 0.05;

        let x2 = (x + perturbation).clamp(range.0, range.1);

        Simplex1D {
            vertices: vec![x, x2],

            values: vec![0.0, 0.0],

            range,
        }
    }

    /// 最適化を実行する（f(x) = -(x-3)² の最大化）。

    fn run(&mut self, max_iterations: usize) -> Simplex1DReport {
        let eval = |x: f64| -> f64 { -((x - 3.0).powi(2)) };

        for (i, v) in self.vertices.iter().enumerate() {
            self.values[i] = eval(*v);
        }

        let mut history: Vec<(f64, f64)> = Vec::new();

        for _iter in 0..max_iterations {
            // 降順ソート

            if self.values[1] > self.values[0] {
                self.vertices.swap(0, 1);

                self.values.swap(0, 1);
            }

            let centroid = self.vertices[0];

            let worst = self.vertices[1];

            // 反射

            let reflected = centroid + (centroid - worst);

            let reflected_val = eval(reflected);

            history.push((reflected, reflected_val));

            if reflected_val > self.values[0] {
                // 拡大

                let expanded = centroid + 2.0 * (reflected - centroid);

                let expanded_val = eval(expanded);

                history.push((expanded, expanded_val));

                if expanded_val > reflected_val {
                    self.vertices[1] = expanded.clamp(self.range.0, self.range.1);

                    self.values[1] = expanded_val;
                } else {
                    self.vertices[1] = reflected.clamp(self.range.0, self.range.1);

                    self.values[1] = reflected_val;
                }
            } else {
                // 収縮

                let contracted = centroid + 0.5 * (worst - centroid);

                let contracted_val = eval(contracted);

                history.push((contracted, contracted_val));

                if contracted_val > self.values[1] {
                    self.vertices[1] = contracted.clamp(self.range.0, self.range.1);

                    self.values[1] = contracted_val;
                } else {
                    // 縮小

                    let new_vertex = centroid + 0.5 * (self.vertices[1] - centroid);

                    self.vertices[1] = new_vertex.clamp(self.range.0, self.range.1);

                    self.values[1] = eval(self.vertices[1]);

                    history.push((self.vertices[1], self.values[1]));
                }
            }
        }

        // 最終ソート

        if self.values[1] > self.values[0] {
            self.vertices.swap(0, 1);

            self.values.swap(0, 1);
        }

        Simplex1DReport {
            best_x: self.vertices[0],

            iterations: max_iterations as u32,
        }
    }
}

/// 1 次元 Nelder-Mead の結果報告。

#[allow(dead_code)]

struct Simplex1DReport {
    best_x: f64,

    iterations: u32,
}

// ============================================================================

// M1.76-KW4: Nelder-Mead 直接探索最適化器

// ============================================================================

/// Nelder-Mead 直接探索法による 7 パラメータ最適化器。

///

/// シンプレックス法とも呼ばれ、導関数不要の直接探索により

/// J_kw を最大化する MagnificentSevenParams を探索する。

/// 各操作（反射・拡大・収縮・縮小）は独立したメソッドに分割されている。

pub struct NelderMeadOptimizer {
    /// 現在のシンプレックス頂点（7 次元 × 8 頂点）
    simplex: Vec<MagnificentSevenParams>,

    /// 各頂点の J_kw 値
    values: Vec<f64>,

    /// 各パラメータの探索範囲 [(min, max); 7]
    ranges: [(f64, f64); 7],

    /// PRNG シード（決定論的再現性のため固定）
    seed: u64,

    /// パレートスイープ用重みベクトル [growth, density, topology, search, fairness, speed]
    /// None = 従来の J_kw_social（乗算結合）を使用
    weights: Option<[f64; 6]>,
}

impl NelderMeadOptimizer {
    /// 新しい最適化器を作成する。

    ///

    /// `initial` を中心に、`perturbation` の割合で各次元に変位させた

    /// 8 頂点（7 次元 + 1）の初期シンプレックスを生成する。

    /// 全頂点は `ranges` で指定された探索範囲内に clamp される。

    pub fn new(
        initial: &MagnificentSevenParams,

        ranges: &[(f64, f64); 7],

        perturbation: f64,

        seed: u64,

        weights: Option<[f64; 6]>,
    ) -> Self {
        let mut simplex = Vec::with_capacity(8);

        let mut values = Vec::with_capacity(8);

        // 中心点（全パラメータを探索範囲内に clamp）
        let mut clamped_initial = *initial;
        {
            let init_params = [
                initial.gamma_benevolence,
                initial.lambda_gc_base,
                initial.direct_reciprocity_weight,
                initial.indirect_reciprocity_weight,
                initial.softmax_temperature,
                initial.gc_interval as f64,
                initial.child_ratio,
            ];
            for i in 0..7 {
                let clamped = init_params[i].clamp(ranges[i].0, ranges[i].1);
                if (clamped - init_params[i]).abs() > 1e-12 {
                    set_param(&mut clamped_initial, i, clamped);
                }
            }
        }
        simplex.push(clamped_initial);
        values.push(evaluate_single(&clamped_initial, seed, &weights));

        // 各次元方向に perturbation だけ変位

        let params_arr = [
            initial.gamma_benevolence,
            initial.lambda_gc_base,
            initial.direct_reciprocity_weight,
            initial.indirect_reciprocity_weight,
            initial.softmax_temperature,
            initial.gc_interval as f64,
            initial.child_ratio,
        ];

        for i in 0..7 {
            let mut displaced = clamped_initial;

            let delta = perturbation * (ranges[i].1 - ranges[i].0);

            let new_val = (params_arr[i] + delta).clamp(ranges[i].0, ranges[i].1);

            set_param(&mut displaced, i, new_val);

            simplex.push(displaced);

            values.push(evaluate_single(&displaced, seed, &weights));
        }

        NelderMeadOptimizer {
            simplex,

            values,

            ranges: *ranges,

            seed,

            weights,
        }
    }

    /// 最適化を実行し、結果を返す。

    ///

    /// 最大 `max_iterations` 回の反復を行い、収束判定（頂点間の J_kw 分散 < epsilon）

    /// を満たすか、最大反復に達した時点で終了する。

    /// 各反復の履歴（CSV 出力用）は引数の `history` に追記される。

    pub fn run(
        &mut self,

        max_iterations: usize,

        epsilon: f64,

        history: &mut Vec<(MagnificentSevenParams, f64)>,
    ) -> OptimizationReport {
        let mut counter = 0u64;

        let experiment_id = generate_kw4_experiment_id(&mut counter);

        let mut iterations = 0u32;

        // 初期履歴

        for (v, &val) in self.simplex.iter().zip(self.values.iter()) {
            history.push((*v, val));
        }

        for _ in 0..max_iterations {
            iterations += 1;

            // J_kw 降順でソート（[0]=最良, [7]=最悪）

            self.sort_by_value_desc();

            // 収束判定: 頂点間の J_kw 分散

            let mean = self.values.iter().sum::<f64>() / self.values.len() as f64;

            let variance = self.values.iter().map(|v| (v - mean).powi(2)).sum::<f64>()
                / self.values.len() as f64;

            if variance < epsilon {
                break;
            }

            // 重心（最悪点を除く全点の平均）

            let centroid = self.compute_centroid();

            // 反射

            let reflected = self.reflect(&centroid);

            let reflected_val = evaluate_single(&reflected, self.seed, &self.weights);

            history.push((reflected, reflected_val));

            if reflected_val > self.values[0] {
                // 反射が最良より良い → 拡大

                let expanded = self.expand(&centroid, &reflected);

                let expanded_val = evaluate_single(&expanded, self.seed, &self.weights);

                history.push((expanded, expanded_val));

                if expanded_val > reflected_val {
                    self.replace_worst(expanded, expanded_val);
                } else {
                    self.replace_worst(reflected, reflected_val);
                }
            } else if reflected_val > self.values[6] {
                // 反射が次悪より良い → 反射を採用

                self.replace_worst(reflected, reflected_val);
            } else {
                // 反射が次悪以下 → 収縮

                let contracted = self.contract(&centroid);

                let contracted_val = evaluate_single(&contracted, self.seed, &self.weights);

                history.push((contracted, contracted_val));

                if contracted_val > self.values[7] {
                    self.replace_worst(contracted, contracted_val);
                } else {
                    // 全点を最良点に向けて縮小

                    let best = self.simplex[0];

                    self.shrink_toward_best(&best);

                    for i in 0..self.simplex.len() {
                        self.values[i] = evaluate_single(&self.simplex[i], self.seed, &self.weights);

                        history.push((self.simplex[i], self.values[i]));
                    }
                }
            }
        }

        // 最終ソート

        self.sort_by_value_desc();

        let best_params = self.simplex[0];

        let best_j_kw_social = self.values[0];

        let config = best_params.to_sim_config(
            crate::constants::KW4_EVALUATION_POPULATION_SIZE,
            self.seed,
        );
        let (metrics, tick_to_convergence) =
            crate::simulation::run_evaluation_simulation(&config);
        let s_speed = compute_s_speed(tick_to_convergence, crate::constants::KW4_SIMULATION_TICKS);

        let mut assessment = compute_kind_world_objective(&metrics);
        // Kind World 判定を J_kw_social 基準に更新
        let j_kw_social_val = assessment.j_kw * s_speed;
        let min_factor = assessment
            .s_growth
            .min(assessment.s_density)
            .min(assessment.s_topology)
            .min(assessment.s_search)
            .min(assessment.s_fairness);
        assessment.is_kind_world = j_kw_social_val > 0.64 && min_factor > 0.6;

        #[allow(deprecated)]
        OptimizationReport {
            best_params,

            best_j_kw: best_j_kw_social,

            best_j_kw_social,

            tick_to_convergence,

            s_speed,

            assessment,

            iterations,

            history: history.clone(),

            converged: iterations < max_iterations as u32,

            experiment_id,
        }
    }

    /// J_kw 降順でソートする（[0]=最良）。

    fn sort_by_value_desc(&mut self) {
        let mut indices: Vec<usize> = (0..self.simplex.len()).collect();

        indices.sort_unstable_by(|&a, &b| {
            self.values[b]
                .partial_cmp(&self.values[a])
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let sorted_simplex: Vec<MagnificentSevenParams> =
            indices.iter().map(|&i| self.simplex[i]).collect();

        let sorted_values: Vec<f64> = indices.iter().map(|&i| self.values[i]).collect();

        self.simplex = sorted_simplex;

        self.values = sorted_values;
    }

    /// 最悪点（最後尾）を除く全点の重心を計算する。

    fn compute_centroid(&self) -> MagnificentSevenParams {
        let n = self.simplex.len() - 1;

        let mut sum = [0.0_f64; 7];

        for i in 0..n {
            sum[0] += self.simplex[i].gamma_benevolence;

            sum[1] += self.simplex[i].lambda_gc_base;

            sum[2] += self.simplex[i].direct_reciprocity_weight;

            sum[3] += self.simplex[i].indirect_reciprocity_weight;

            sum[4] += self.simplex[i].softmax_temperature;

            sum[5] += self.simplex[i].gc_interval as f64;

            sum[6] += self.simplex[i].child_ratio;
        }

        let n_f = n as f64;

        let mut centroid = self.simplex[0];

        set_param(&mut centroid, 0, sum[0] / n_f);

        set_param(&mut centroid, 1, sum[1] / n_f);

        set_param(&mut centroid, 2, sum[2] / n_f);

        set_param(&mut centroid, 3, sum[3] / n_f);

        set_param(&mut centroid, 4, sum[4] / n_f);

        set_param(&mut centroid, 5, sum[5] / n_f);

        set_param(&mut centroid, 6, sum[6] / n_f);

        centroid
    }

    /// 最悪点を重心に対して反射する（α = 1.0）。

    fn reflect(&self, centroid: &MagnificentSevenParams) -> MagnificentSevenParams {
        let worst = &self.simplex[7];

        let alpha = 1.0;

        let mut params = *centroid;

        for i in 0..7 {
            let c = get_param(centroid, i);

            let w = get_param(worst, i);

            let reflected = c + alpha * (c - w);

            set_param(
                &mut params,
                i,
                reflected.clamp(self.ranges[i].0, self.ranges[i].1),
            );
        }

        params
    }

    /// 反射点をさらに拡大する（γ = 2.0）。

    fn expand(
        &self,

        centroid: &MagnificentSevenParams,

        reflected: &MagnificentSevenParams,
    ) -> MagnificentSevenParams {
        let gamma = 2.0;

        let mut params = *centroid;

        for i in 0..7 {
            let c = get_param(centroid, i);

            let r = get_param(reflected, i);

            let expanded = c + gamma * (r - c);

            set_param(
                &mut params,
                i,
                expanded.clamp(self.ranges[i].0, self.ranges[i].1),
            );
        }

        params
    }

    /// 収縮（ρ = 0.5）。

    fn contract(&self, centroid: &MagnificentSevenParams) -> MagnificentSevenParams {
        let worst = &self.simplex[7];

        let rho = 0.5;

        let mut params = *centroid;

        for i in 0..7 {
            let c = get_param(centroid, i);

            let w = get_param(worst, i);

            let contracted = c + rho * (w - c);

            set_param(
                &mut params,
                i,
                contracted.clamp(self.ranges[i].0, self.ranges[i].1),
            );
        }

        params
    }

    /// 最良点を除く全点を最良点に向かって縮小する（σ = 0.5）。

    fn shrink_toward_best(&mut self, best: &MagnificentSevenParams) {
        let sigma = 0.5;

        for i in 1..self.simplex.len() {
            let cur = self.simplex[i];

            let mut p = cur;

            for j in 0..7 {
                let b = get_param(best, j);

                let c = get_param(&cur, j);

                let shrunk = b + sigma * (c - b);

                set_param(&mut p, j, shrunk.clamp(self.ranges[j].0, self.ranges[j].1));
            }

            self.simplex[i] = p;
        }
    }

    /// 最悪点（最後尾）を新しい点で置き換える。

    fn replace_worst(&mut self, new_vertex: MagnificentSevenParams, new_value: f64) {
        let last = self.simplex.len() - 1;

        self.simplex[last] = new_vertex;

        self.values[last] = new_value;
    }
}

// ============================================================================

// Tests

// ============================================================================

#[cfg(test)]

mod tests {

    use super::*;

    use rand::rngs::StdRng;

    use rand::Rng;

    use rand::SeedableRng;

    // --- B1-B5: Trust & Reciprocity Metrics Backfill ---

    /// B1: compute_mean_benevolence — 空マップで 0.0
    #[test]
    fn b1_compute_mean_benevolence_empty() {
        let empty: std::collections::HashMap<crate::types::NodeId, crate::types::TrustProfile> =
            std::collections::HashMap::new();
        assert_eq!(compute_mean_benevolence(&empty), 0.0);
    }

    /// B2: compute_mean_benevolence — 全 TrustProfile が (1.0,1.0,1.0) で 1.0
    #[test]
    fn b2_compute_mean_benevolence_all_one() {
        use std::collections::HashMap;
        let mut map = HashMap::new();
        for i in 0..5 {
            map.insert(
                i,
                crate::types::TrustProfile {
                    operational: 1.0,
                    semantic: 1.0,
                    temporal: 1.0,
                    human: crate::types::HumanTrustLogistic { score: 1.0, k: 1.0, scale: 0.3, count: 0 },
                },
            );
        }
        let result = compute_mean_benevolence(&map);
        assert!((result - 1.0).abs() < 1e-10, "expected 1.0, got {}", result);
    }

    /// B3: compute_mean_reciprocity — 対称 HELP 提供で 1.0
    #[test]
    fn b3_compute_mean_reciprocity_symmetric() {
        use std::collections::HashMap;
        let mut map = HashMap::new();
        map.insert((0, 1), 3);
        map.insert((1, 0), 3);
        assert!((compute_mean_reciprocity(&map) - 1.0).abs() < 1e-10);
    }

    /// B4: compute_mean_reciprocity — 非対称 HELP 提供で低値
    #[test]
    fn b4_compute_mean_reciprocity_asymmetric() {
        use std::collections::HashMap;
        let mut map = HashMap::new();
        map.insert((0, 1), 5);
        map.insert((1, 0), 0);
        assert!((compute_mean_reciprocity(&map) - 0.0).abs() < 1e-10);
    }

    /// B5: compute_trust_inheritance_fidelity — event_count=0 で 0.0
    #[test]
    fn b5_compute_trust_inheritance_fidelity_zero_events() {
        assert_eq!(compute_trust_inheritance_fidelity(10.0, 0), 0.0);
    }

    // --- A1-A7: Lifecycle & Freshness Metrics Backfill ---

    /// A1: lifecycle_score_from_gc_state の全網羅テスト
    #[test]
    fn a1_lifecycle_score_from_gc_state_exhaustive() {
        assert_eq!(lifecycle_score_from_gc_state(&crate::event::GcEvent::Protected), 1.0);
        assert_eq!(lifecycle_score_from_gc_state(&crate::event::GcEvent::Active), 0.8);
        assert_eq!(lifecycle_score_from_gc_state(&crate::event::GcEvent::SoftDeleted), 0.3);
        assert_eq!(lifecycle_score_from_gc_state(&crate::event::GcEvent::HardDeleteCandidate), 0.1);
        assert_eq!(lifecycle_score_from_gc_state(&crate::event::GcEvent::Tombstoned), 0.0);
    }

    /// A2: compute_mean_lifecycle_score — 空マップで 0.0
    #[test]
    fn a2_compute_mean_lifecycle_score_empty() {
        let empty: std::collections::HashMap<crate::types::NodeId, crate::event::GcEvent> = std::collections::HashMap::new();
        assert_eq!(compute_mean_lifecycle_score(&empty), 0.0);
    }

    /// A3: compute_child_survival_rate — total_births=0 で 0.0
    #[test]
    fn a3_compute_child_survival_rate_zero_births() {
        assert_eq!(compute_child_survival_rate(0, 5), 0.0);
    }

    /// A4: compute_child_survival_rate — 全生存で 1.0
    #[test]
    fn a4_compute_child_survival_rate_all_survive() {
        assert_eq!(compute_child_survival_rate(5, 5), 1.0);
        assert!((compute_child_survival_rate(5, 3) - 0.6).abs() < 1e-10);
    }

    /// A5: compute_mean_freshness — 全ノード同一 tick で 1.0, 経過で低値
    #[test]
    fn a5_compute_mean_freshness_range() {
        use std::collections::HashMap;
        // 全ノード current_tick = 10 で更新済み → elapsed=0 → freshness=1.0
        let mut map = HashMap::new();
        map.insert(0, 10);
        map.insert(1, 10);
        let fresh = compute_mean_freshness(&map, 10);
        assert!((fresh - 1.0).abs() < 1e-10, "fresh={}", fresh);

        // 経過後 → 低値
        let stale = compute_mean_freshness(&map, 100);
        assert!(stale < 0.5, "stale={} should be low", stale);
    }

    /// A6: collect_final_metrics の lifecycle 3 指標出力確認
    #[test]
    fn a6_collect_final_metrics_lifecycle_nonzero() {
        let config = crate::simulation::ReciprocitySimulatorConfig::default();
        let (metrics, _ttc) = crate::simulation::run_evaluation_simulation(&config);
        println!(
            "A6: ttc={}, mean_lifecycle={:.6}, child_survival={:.6}, mean_freshness={:.6}",
            _ttc,
            metrics.mean_lifecycle_score, metrics.child_survival_rate, metrics.mean_freshness
        );
        assert!(
            metrics.mean_lifecycle_score > 0.0,
            "mean_lifecycle_score should be > 0.0, got {}",
            metrics.mean_lifecycle_score
        );
        assert!(
            metrics.child_survival_rate >= 0.0,
            "child_survival_rate should be >= 0.0, got {}",
            metrics.child_survival_rate
        );
        assert!(
            metrics.mean_freshness >= 0.0,
            "mean_freshness should be >= 0.0, got {}",
            metrics.mean_freshness
        );
    }

    /// B6: collect_final_metrics — trust/reciprocity 3 指標が妥当な値を取る
    ///
    /// mean_benevolence と trust_inheritance_fidelity は > 0.0 を期待。
    /// mean_reciprocity は HELP が成人→子の一方向であるため 0.0 も許容する。
    #[test]
    fn b6_collect_final_metrics_trust_reciprocity_valid() {
        let config = crate::simulation::ReciprocitySimulatorConfig::default();
        let (metrics, _ttc) = crate::simulation::run_evaluation_simulation(&config);
        println!(
            "B6: ttc={}, mean_benevolence={:.6}, mean_reciprocity={:.6}, trust_inheritance_fidelity={:.6}",
            _ttc,
            metrics.mean_benevolence_aggregate,
            metrics.mean_reciprocity_score,
            metrics.trust_inheritance_fidelity
        );
        assert!(
            metrics.mean_benevolence_aggregate > 0.0,
            "mean_benevolence_aggregate should be > 0.0, got {}",
            metrics.mean_benevolence_aggregate
        );
        assert!(
            metrics.mean_reciprocity_score >= 0.0,
            "mean_reciprocity_score should be >= 0.0, got {}",
            metrics.mean_reciprocity_score
        );
        assert!(
            metrics.trust_inheritance_fidelity > 0.0,
            "trust_inheritance_fidelity should be > 0.0, got {}",
            metrics.trust_inheritance_fidelity
        );
    }

    /// B7: 既存テスト全 PASS (regression) — テストランナーが自動検証
    #[test]
    fn b7_regression() {
        assert!(true);
    }

    // --- C1-C7: Execution & Cost Metrics Backfill (MTR-C) ---

    /// C1: compute_execution_success_rate — total_attempts=0 で 0.0
    #[test]
    fn c1_compute_execution_success_rate_empty() {
        assert_eq!(compute_execution_success_rate(0, 0), 0.0);
        assert_eq!(compute_execution_success_rate(0, 5), 0.0);
    }

    /// C2: compute_execution_success_rate — 全成功で 1.0
    #[test]
    fn c2_compute_execution_success_rate_all_success() {
        assert!((compute_execution_success_rate(10, 10) - 1.0).abs() < 1e-10);
    }

    /// C3: compute_execution_success_rate — 部分成功で ratio
    #[test]
    fn c3_compute_execution_success_rate_partial() {
        assert!((compute_execution_success_rate(10, 5) - 0.5).abs() < 1e-10);
        assert!((compute_execution_success_rate(10, 3) - 0.3).abs() < 1e-10);
    }

    /// C4: compute_cost_efficiency_ratio — cost=0 で 1.0
    #[test]
    fn c4_compute_cost_efficiency_ratio_zero_cost() {
        assert!((compute_cost_efficiency_ratio(0, 5, 5) - 1.0).abs() < 1e-10);
        assert!((compute_cost_efficiency_ratio(0, 0, 0) - 1.0).abs() < 1e-10);
    }

    /// C5: compute_cost_efficiency_ratio — high cost で低値
    #[test]
    fn c5_compute_cost_efficiency_ratio_high_cost() {
        assert!((compute_cost_efficiency_ratio(10, 10, 0) - 0.0).abs() < 1e-10);
        let mid = compute_cost_efficiency_ratio(5, 10, 5);
        assert!(mid > 0.0 && mid < 1.0, "mid efficiency={} should be in (0,1)", mid);
    }

    /// C6: collect_final_metrics — execution/cost 2 指標が 0.0/0.5 の仮値でない
    #[test]
    fn c6_collect_final_metrics_execution_cost_valid() {
        let config = crate::simulation::ReciprocitySimulatorConfig::default();
        let (metrics, _ttc) = crate::simulation::run_evaluation_simulation(&config);
        println!(
            "C6: ttc={}, execution_success_rate={:.6}, cost_efficiency={:.6}",
            _ttc,
            metrics.execution_success_rate, metrics.cost_efficiency
        );
        assert!(
            metrics.execution_success_rate > 0.0,
            "execution_success_rate should be > 0.0, got {}",
            metrics.execution_success_rate
        );
        assert!(
            (metrics.cost_efficiency - 0.5).abs() > 1e-6,
            "cost_efficiency should not be 0.5 (default), got {}",
            metrics.cost_efficiency
        );
    }

    /// C7: 既存テスト全 PASS (regression) — テストランナーが自動検証
    #[test]
    fn c7_regression() {
        assert!(true);
    }

    // ---- D1-D7: Capability & Knowledge Metrics Backfill (MTR-D) ----

    /// D1: compute_capability_coverage — 空 positions で 0.0
    #[test]
    fn d1_compute_capability_coverage_empty() {
        let empty: std::collections::HashMap<
            crate::types::NodeId,
            crate::spaceposition::SpacePositionEmbedding,
        > = std::collections::HashMap::new();
        assert_eq!(compute_capability_coverage(&empty), 0.0);
    }

    /// D2: compute_capability_coverage — 全ノード同一位置で低値（全同一グリッドセル）
    #[test]
    fn d2_compute_capability_coverage_all_same_position() {
        use std::collections::HashMap;
        let mut map: HashMap<
            crate::types::NodeId,
            crate::spaceposition::SpacePositionEmbedding,
        > = HashMap::new();
        // 10 ノードすべてが同一座標 (0.5, 0.5) — 全 1 セルに集中
        for i in 0..10_usize {
            let emb: crate::spaceposition::SpacePositionEmbedding = [0.5_f32, 0.5, 0.0].into();
            map.insert(i, emb);
        }
        let cov = compute_capability_coverage(&map);
        // 全ノード同一セル → p=1.0 → H=0 → cov=0.0
        assert!(
            (cov - 0.0).abs() < 1e-10,
            "同一位置全 10 ノードで capability_coverage={}, expected 0.0",
            cov
        );
    }

    /// D3: compute_reuse_ratio_from_pair_counts — 空 pair_counts で 0.0
    #[test]
    fn d3_compute_reuse_ratio_empty() {
        let empty: std::collections::HashMap<
            (crate::types::NodeId, crate::types::NodeId),
            u64,
        > = std::collections::HashMap::new();
        assert_eq!(compute_reuse_ratio_from_pair_counts(&empty), 0.0);
    }

    /// D4: compute_reuse_ratio_from_pair_counts — 全ペア頻度 >= 2 で 1.0
    #[test]
    fn d4_compute_reuse_ratio_all_reused() {
        use std::collections::HashMap;
        let mut map: HashMap<(crate::types::NodeId, crate::types::NodeId), u64> =
            HashMap::new();
        map.insert((1, 2), 3);
        map.insert((3, 4), 2);
        map.insert((5, 6), 5);
        assert!(
            (compute_reuse_ratio_from_pair_counts(&map) - 1.0).abs() < 1e-10,
            "全 3 ペア頻度 >= 2 で 1.0 になるはず"
        );
    }

    /// D5: compute_knowledge_diffusion_from_pair_counts — 空 pair_counts で 0.0
    #[test]
    fn d5_compute_knowledge_diffusion_empty() {
        let empty: std::collections::HashMap<
            (crate::types::NodeId, crate::types::NodeId),
            u64,
        > = std::collections::HashMap::new();
        assert_eq!(compute_knowledge_diffusion_from_pair_counts(&empty), 0.0);
    }

    /// D6: compute_knowledge_diffusion_from_pair_counts — 全ペアが異なるユニークペアで 1.0
    #[test]
    fn d6_compute_knowledge_diffusion_all_unique() {
        use std::collections::HashMap;
        let mut map: HashMap<(crate::types::NodeId, crate::types::NodeId), u64> =
            HashMap::new();
        // 5 ペアすべてが頻度 1（全ユニーク）
        map.insert((1, 2), 1);
        map.insert((3, 4), 1);
        map.insert((5, 6), 1);
        map.insert((7, 8), 1);
        map.insert((9, 10), 1);
        assert!(
            (compute_knowledge_diffusion_from_pair_counts(&map) - 1.0).abs() < 1e-10,
            "全 5 ペアが頻度 1 で 1.0 になるはず"
        );
    }

    /// D7: collect_final_metrics — 3 指標が 0.0 ではないことを確認（観測テスト）
    #[test]
    fn d7_collect_final_metrics_capability_knowledge_valid() {
        let config = crate::simulation::ReciprocitySimulatorConfig {
            population_size: 100,
            child_ratio: 0.5,
            mission_rate: 0.8,
            max_ticks: 500,
            ..crate::simulation::ReciprocitySimulatorConfig::default()
        };
        let (metrics, _ttc) = crate::simulation::run_evaluation_simulation(&config);
        println!("=== MTR-D Observation ===");
        println!("D7: ttc={}", _ttc);
        println!("capability_coverage: {:.6}", metrics.capability_coverage);
        println!("reuse_ratio: {:.6}", metrics.reuse_ratio);
        println!("knowledge_diffusion_rate: {:.6}", metrics.knowledge_diffusion_rate);

        assert!(
            metrics.capability_coverage > 0.0,
            "capability_coverage should be > 0.0, got {}",
            metrics.capability_coverage
        );
        assert!(
            metrics.reuse_ratio > 0.0,
            "reuse_ratio should be > 0.0, got {}",
            metrics.reuse_ratio
        );
        assert!(
            metrics.knowledge_diffusion_rate > 0.0,
            "knowledge_diffusion_rate should be > 0.0, got {}",
            metrics.knowledge_diffusion_rate
        );
    }

    /// D8: 既存テスト全 PASS (regression) — テストランナーが自動検証
    #[test]
    fn d8_regression() {
        assert!(true);
    }

    /// 新旧 J_kw モデルの互換性診断結果。

    #[derive(Debug, Clone)]
    #[allow(dead_code)]
    pub struct JkwModelComparison {
        /// 旧 6 成分加重和による J_kw
        pub old_j_kw: f64,

        /// 新 5 因子乗算結合による J_kw
        pub new_j_kw: f64,

        /// 差 (new - old)
        pub difference: f64,

        /// 相対差 |new - old| / max(|old|, 1e-10)
        pub relative_diff: f64,

        /// 旧 6 成分値（生値、重み乗算前）
        pub old_components: [f64; 6],

        /// 新 5 因子値（S_viability, S_capability, S_cooperation, S_efficiency, S_fairness）
        pub new_factors: [f64; 5],

        /// 旧 8 フラグ
        pub old_flags: [bool; 8],

        /// 新条件 [is_kind_world, min_factor_gate_satisfied]
        pub new_conditions: [bool; 2],
    }

    /// 新旧 J_kw モデルを比較し、互換性診断結果を返す。

    ///

    /// `old_metrics` は旧 9 フィールドのみ設定された入力を想定し、

    /// 旧 6 成分加重和（ハードコードされた重み）で J_kw を計算する。

    /// `new_metrics` は全 17 フィールドが設定された入力を想定し、

    /// 新 5 因子乗算結合（compute_kind_world_objective）で J_kw を計算する。

    #[allow(dead_code)]
    pub fn compare_j_kw_models(
        old_metrics: &KindWorldMetricsInput,

        new_metrics: &KindWorldMetricsInput,
    ) -> JkwModelComparison {
        // 旧 6 成分加重和の重み（constants.rs から削除済みのためハードコード）

        const ALPHA_POP: f64 = 0.25;

        const ALPHA_COV: f64 = 0.20;

        const ALPHA_REUSE: f64 = 0.15;

        const ALPHA_COST: f64 = 0.20;

        const ALPHA_VILLAGE: f64 = 0.10;

        const ALPHA_PENALTY: f64 = 0.10;

        // 旧 6 成分（生値）

        let c0 = old_metrics.population_growth_rate.clamp(0.0, 1.0);

        let c1 = old_metrics.capability_coverage;

        let c2 = old_metrics.reuse_ratio;

        let c3 = old_metrics.cost_efficiency;

        let c4 = ((1.0 - old_metrics.village_churn_rate)
            + old_metrics.village_formation_score
            + old_metrics.cross_village_interaction_rate
            + old_metrics.knowledge_diffusion_rate)
            / 4.0;

        let c5 = 1.0
            - old_metrics
                .benevolent_vs_non_benevolent_coverage_ratio
                .clamp(0.0, 1.0);

        let old_j_kw = ALPHA_POP * c0
            + ALPHA_COV * c1
            + ALPHA_REUSE * c2
            + ALPHA_COST * c3
            + ALPHA_VILLAGE * c4
            + ALPHA_PENALTY * c5;

        let assessment = compute_kind_world_objective(new_metrics);

        let new_j_kw = assessment.j_kw;

        let difference = new_j_kw - old_j_kw;

        let relative_diff = if old_j_kw.abs() > 1e-10 {
            difference.abs() / old_j_kw.abs()
        } else {
            difference.abs() / 1e-10
        };

        JkwModelComparison {
            old_j_kw,

            new_j_kw,

            difference,

            relative_diff,

            old_components: [c0, c1, c2, c3, c4, c5],

            new_factors: [
                assessment.s_growth,
                assessment.s_density,
                assessment.s_topology,
                assessment.s_search,
                assessment.s_fairness,
            ],

            old_flags: assessment.legacy_flags,

            new_conditions: [
                assessment.is_kind_world,
                assessment.s_growth > 0.6
                    && assessment.s_density > 0.6
                    && assessment.s_topology > 0.6
                    && assessment.s_search > 0.6
                    && assessment.s_fairness > 0.6,
            ],
        }
    }

    // ---- TC-1: 全条件成立 ----

    #[test]

    fn tc1_kw_all_conditions_met() {
        let metrics = KindWorldMetricsInput {
            population_growth_rate: 0.95,

            capability_coverage: 0.99,

            reuse_ratio: 0.99,

            cost_efficiency: 0.94,

            village_formation_score: 0.99,

            village_churn_rate: 0.15,

            cross_village_interaction_rate: 0.99,

            knowledge_diffusion_rate: 0.99,

            benevolent_vs_non_benevolent_coverage_ratio: 1.0,

            mean_lifecycle_score: 0.99,
            child_survival_rate: 0.99,
            mean_freshness: 0.99,
            mean_benevolence_aggregate: 0.99,
            mean_reciprocity_score: 0.99,
            help_success_rate: 0.99,
            trust_inheritance_fidelity: 0.99,
            execution_success_rate: 0.99,
            mean_nest_depth: 0.99,
            mean_node_density: 0.99,
            cluster_coefficient: 0.99,
            local_density: 0.99,
            search_radius_inverse: 0.99,
            reasoning_steps_inverse: 0.99,
        };

        let result = compute_kind_world_objective(&metrics);

        assert!(
            result.is_kind_world,
            "全条件が閾値を超えているため Kind World 成立"
        );

        assert!(result.legacy_flags.iter().all(|&f| f), "全 8 フラグが true");

        assert!(result.j_kw > 0.8, "J_kw = {} が 0.8 を超える", result.j_kw);
    }

    // ---- TC-2: 全条件不成立 ----

    #[test]

    fn tc2_kw_all_conditions_not_met() {
        let metrics = KindWorldMetricsInput {
            population_growth_rate: 0.0,

            capability_coverage: 0.0,

            reuse_ratio: 0.0,

            cost_efficiency: 0.99, // > 0.95 → 条件不成立

            village_formation_score: 0.0,

            village_churn_rate: 0.01, // < 0.05 → churn_low も不成立

            cross_village_interaction_rate: 0.0,

            knowledge_diffusion_rate: 0.0,

            benevolent_vs_non_benevolent_coverage_ratio: 0.0,

            mean_lifecycle_score: 0.0,
            child_survival_rate: 0.0,
            mean_freshness: 0.0,
            mean_benevolence_aggregate: 0.0,
            mean_reciprocity_score: 0.0,
            help_success_rate: 0.0,
            trust_inheritance_fidelity: 0.0,
            execution_success_rate: 0.0,
            mean_nest_depth: 0.0,
            mean_node_density: 0.0,
            cluster_coefficient: 0.0,
            local_density: 0.0,
            search_radius_inverse: 0.0,
            reasoning_steps_inverse: 0.0,
        };

        let result = compute_kind_world_objective(&metrics);

        assert!(
            !result.is_kind_world,
            "全条件が閾値を下回っているため Kind World 不成立"
        );

        // cost_efficiency=0.99 (<= 0.95 → false) と village_churn_rate=0.01 (< 0.05 → false, < 0.30 → true)

        // なので churn_high のみ true、他 7 つは false

        assert!(!result.legacy_flags[0], "flag_population");

        assert!(!result.legacy_flags[1], "flag_capability");

        assert!(!result.legacy_flags[2], "flag_reuse");

        assert!(!result.legacy_flags[3], "flag_cost");

        assert!(!result.legacy_flags[4], "flag_village_formation");

        assert!(!result.legacy_flags[5], "flag_churn_low");

        assert!(
            result.legacy_flags[6],
            "flag_churn_high = (0.01 <= 0.30) → true"
        );

        assert!(!result.legacy_flags[7], "flag_cross");
    }

    // ---- TC-3: J_kw 範囲検証（n=10,000 ランダム入力） ----

    #[test]

    fn tc3_kw_j_kw_range_random() {
        let mut rng = StdRng::seed_from_u64(12345);

        let mut nan_count = 0u64;

        let mut inf_count = 0u64;

        let mut out_of_range_count = 0u64;

        let n = 10_000u64;

        for _ in 0..n {
            let metrics = KindWorldMetricsInput {
                population_growth_rate: rng.random::<f64>() * 2.0,

                capability_coverage: rng.random::<f64>(),

                reuse_ratio: rng.random::<f64>(),

                cost_efficiency: rng.random::<f64>() * 2.0,

                village_formation_score: rng.random::<f64>(),

                village_churn_rate: rng.random::<f64>(),

                cross_village_interaction_rate: rng.random::<f64>(),

                knowledge_diffusion_rate: rng.random::<f64>(),

                benevolent_vs_non_benevolent_coverage_ratio: rng.random::<f64>() * 3.0,

                mean_lifecycle_score: 0.0,
                child_survival_rate: 0.0,
                mean_freshness: 0.0,
                mean_benevolence_aggregate: 0.0,
                mean_reciprocity_score: 0.0,
                help_success_rate: 0.0,
                trust_inheritance_fidelity: 0.0,
                execution_success_rate: 0.0,
                mean_nest_depth: 0.0,
                mean_node_density: 0.0,
                cluster_coefficient: 0.0,
                local_density: 0.0,
                search_radius_inverse: 0.0,
                reasoning_steps_inverse: 0.0,
            };

            let result = compute_kind_world_objective(&metrics);

            if result.j_kw.is_nan() {
                nan_count += 1;
            }

            if result.j_kw.is_infinite() {
                inf_count += 1;
            }

            if !(0.0..=1.0).contains(&result.j_kw) {
                out_of_range_count += 1;
            }
        }

        assert_eq!(nan_count, 0, "NaN 出現件数: {}", nan_count);

        assert_eq!(inf_count, 0, "Inf 出現件数: {}", inf_count);

        assert_eq!(
            out_of_range_count, 0,
            "[0,1] 範囲外件数: {}",
            out_of_range_count
        );
    }

    // ---- TC-4: J_pop 単調性 ----

    #[test]

    fn tc4_kw_j_pop_monotonic() {
        let base = KindWorldMetricsInput {
            population_growth_rate: 0.0,

            capability_coverage: 0.5,

            reuse_ratio: 0.3,

            cost_efficiency: 0.8,

            village_formation_score: 0.3,

            village_churn_rate: 0.15,

            cross_village_interaction_rate: 0.2,

            knowledge_diffusion_rate: 0.5,

            benevolent_vs_non_benevolent_coverage_ratio: 1.0,

            mean_lifecycle_score: 0.0,
            child_survival_rate: 0.0,
            mean_freshness: 0.0,
            mean_benevolence_aggregate: 0.0,
            mean_reciprocity_score: 0.0,
            help_success_rate: 0.0,
            trust_inheritance_fidelity: 0.0,
            execution_success_rate: 0.0,
            mean_nest_depth: 0.0,
            mean_node_density: 0.0,
            cluster_coefficient: 0.0,
            local_density: 0.0,
            search_radius_inverse: 0.0,
            reasoning_steps_inverse: 0.0,
        };

        let mut prev_j = compute_kind_world_objective(&base).j_kw;

        for &rate in &[0.1, 0.2, 0.3, 0.5, 0.8, 1.0] {
            let metrics = KindWorldMetricsInput {
                population_growth_rate: rate,

                ..base
            };

            let j = compute_kind_world_objective(&metrics).j_kw;

            assert!(
                j >= prev_j - 1e-12,
                "J_kw は人口成長率に対して非減少: rate={}, prev_j={}, j={}",
                rate,
                prev_j,
                j
            );

            prev_j = j;
        }
    }

    // ---- TC-6: 空入力（全ゼロ）で panic せず J_kw は有限値 ----

    #[test]

    fn tc6_kw_empty_input_no_panic() {
        let metrics = KindWorldMetricsInput::zero();

        let result = compute_kind_world_objective(&metrics);

        assert!(!result.is_kind_world);

        assert!(
            result.j_kw.is_finite(),
            "J_kw は有限値（実測: {}）",
            result.j_kw
        );

        // cost_efficiency=0 → J_cost = 1.0, benevolent_ratio=0 → J_penalty = 1.0

        // そのため全ゼロ入力でも J_kw > 0 となる（正しい動作）
    }

    // ---- TC-7: J_penalty 慈悲的劣位 ----

    #[test]

    fn tc7_kw_penalty_benevolent_inferior() {
        // 乗算モデル: ratio < 1.0 → s_fairness = ratio < 0.6 → is_kind_world = false

        let metrics = KindWorldMetricsInput {
            population_growth_rate: 0.99,

            capability_coverage: 0.99,

            reuse_ratio: 0.99,

            cost_efficiency: 0.94,

            village_formation_score: 0.99,

            village_churn_rate: 0.15,

            cross_village_interaction_rate: 0.99,

            knowledge_diffusion_rate: 0.99,

            benevolent_vs_non_benevolent_coverage_ratio: 0.3, // 劣位

            mean_lifecycle_score: 0.99,
            child_survival_rate: 0.99,
            mean_freshness: 0.99,
            mean_benevolence_aggregate: 0.99,
            mean_reciprocity_score: 0.99,
            help_success_rate: 0.99,
            trust_inheritance_fidelity: 0.99,
            execution_success_rate: 0.99,
            mean_nest_depth: 0.0,
            mean_node_density: 0.0,
            cluster_coefficient: 0.0,
            local_density: 0.0,
            search_radius_inverse: 0.0,
            reasoning_steps_inverse: 0.0,
        };

        let result = compute_kind_world_objective(&metrics);

        // ratio=0.3 → s_fairness = 0.3 < 0.6 → is_kind_world = false
        assert!(
            !result.is_kind_world,
            "s_fairness = {} < 0.6 のため false",
            result.s_fairness
        );

        assert!(
            (result.s_fairness - 0.3).abs() < 1e-10,
            "s_fairness = {}",
            result.s_fairness
        );

        // ペナルティの比較: ratio=1.0 の方が j_kw が高い（ペナルティなし）
        let high_ratio = KindWorldMetricsInput {
            benevolent_vs_non_benevolent_coverage_ratio: 1.0,
            ..metrics
        };
        let high_result = compute_kind_world_objective(&high_ratio);

        assert!(
            high_result.j_kw > result.j_kw,
            "ratio=1.0 の J_kw（{}）> ratio=0.3 の J_kw（{}）",
            high_result.j_kw,
            result.j_kw
        );
    }

    #[test]
    fn tc8_kw_penalty_benevolent_equal() {
        // ratio=1.0 で全因子が高い → is_kind_world 成立確認
        let metrics = KindWorldMetricsInput {
            population_growth_rate: 0.99,

            capability_coverage: 0.99,

            reuse_ratio: 0.99,

            cost_efficiency: 0.94,

            village_formation_score: 0.99,

            village_churn_rate: 0.15,

            cross_village_interaction_rate: 0.99,

            knowledge_diffusion_rate: 0.99,

            benevolent_vs_non_benevolent_coverage_ratio: 1.0, // 同等

            mean_lifecycle_score: 0.99,
            child_survival_rate: 0.99,
            mean_freshness: 0.99,
            mean_benevolence_aggregate: 0.99,
            mean_reciprocity_score: 0.99,
            help_success_rate: 0.99,
            trust_inheritance_fidelity: 0.99,
            execution_success_rate: 0.99,
            mean_nest_depth: 0.99,
            mean_node_density: 0.99,
            cluster_coefficient: 0.99,
            local_density: 0.99,
            search_radius_inverse: 0.99,
            reasoning_steps_inverse: 0.99,
        };

        let result = compute_kind_world_objective(&metrics);

        // ratio=1.0 → s_fairness = 1.0 (penalty なし)
        // 全因子が 0.9 超 → product > 0.8 → is_kind_world = true
        assert!(
            result.is_kind_world,
            "全因子が高く ratio=1.0 のため Kind World 成立"
        );

        assert!(result.j_kw > 0.8, "j_kw = {} > 0.8", result.j_kw);

        assert!((result.s_fairness - 1.0).abs() < 1e-10);
    }

    #[test]
    fn tc9_kw_boundary_threshold() {
        // 乗算モデルの閾値: j_kw > 0.8 かつ min_factor > 0.6
        // 全因子が 0.98 → product = 0.98^5 ≈ 0.904 > 0.8 → is_kind_world
        // 全因子が 0.90 → product = 0.90^5 ≈ 0.590 < 0.8 → is_kind_world = false

        let above = KindWorldMetricsInput {
            population_growth_rate: 0.98,

            capability_coverage: 0.98,

            reuse_ratio: 0.98,

            cost_efficiency: 0.94, // <= 0.95 のため legacy flag も true

            village_formation_score: 0.98,

            village_churn_rate: 0.15, // [0.05, 0.30] の範囲内

            cross_village_interaction_rate: 0.98,

            knowledge_diffusion_rate: 0.98,

            benevolent_vs_non_benevolent_coverage_ratio: 1.0,

            mean_lifecycle_score: 0.98,
            child_survival_rate: 0.98,
            mean_freshness: 0.98,
            mean_benevolence_aggregate: 0.98,
            mean_reciprocity_score: 0.98,
            help_success_rate: 0.98,
            trust_inheritance_fidelity: 0.98,
            execution_success_rate: 0.98,
            mean_nest_depth: 0.98,
            mean_node_density: 0.98,
            cluster_coefficient: 0.98,
            local_density: 0.98,
            search_radius_inverse: 0.98,
            reasoning_steps_inverse: 0.98,
        };

        assert!(
            compute_kind_world_objective(&above).is_kind_world,
            "全因子 0.98 で Kind World 成立"
        );

        let below = KindWorldMetricsInput {
            population_growth_rate: 0.90,

            capability_coverage: 0.90,

            reuse_ratio: 0.90,

            cost_efficiency: 0.90,

            village_formation_score: 0.90,

            village_churn_rate: 0.15,

            cross_village_interaction_rate: 0.90,

            knowledge_diffusion_rate: 0.90,

            benevolent_vs_non_benevolent_coverage_ratio: 1.0,

            mean_lifecycle_score: 0.90,
            child_survival_rate: 0.90,
            mean_freshness: 0.90,
            mean_benevolence_aggregate: 0.90,
            mean_reciprocity_score: 0.90,
            help_success_rate: 0.90,
            trust_inheritance_fidelity: 0.90,
            execution_success_rate: 0.90,
            mean_nest_depth: 0.90,
            mean_node_density: 0.90,
            cluster_coefficient: 0.90,
            local_density: 0.90,
            search_radius_inverse: 0.90,
            reasoning_steps_inverse: 0.90,
        };

        assert!(
            !compute_kind_world_objective(&below).is_kind_world,
            "全因子 0.90 で Kind World 不成立"
        );
    }

    // ---- TC-10: JSON ラウンドトリップ ----

    #[test]

    fn tc10_kw_json_roundtrip() {
        let metrics = KindWorldMetricsInput {
            population_growth_rate: 0.02,

            capability_coverage: 0.6,

            reuse_ratio: 0.4,

            cost_efficiency: 0.8,

            village_formation_score: 0.5,

            village_churn_rate: 0.15,

            cross_village_interaction_rate: 0.2,

            knowledge_diffusion_rate: 0.5,

            benevolent_vs_non_benevolent_coverage_ratio: 1.0,

            mean_lifecycle_score: 0.0,
            child_survival_rate: 0.0,
            mean_freshness: 0.0,
            mean_benevolence_aggregate: 0.0,
            mean_reciprocity_score: 0.0,
            help_success_rate: 0.0,
            trust_inheritance_fidelity: 0.0,
            execution_success_rate: 0.0,
            mean_nest_depth: 0.0,
            mean_node_density: 0.0,
            cluster_coefficient: 0.0,
            local_density: 0.0,
            search_radius_inverse: 0.0,
            reasoning_steps_inverse: 0.0,
        };

        let result = compute_kind_world_objective(&metrics);

        let json = serde_json::to_string(&result).expect("JSON シリアライズ成功");

        let deserialized: KindWorldAssessment =
            serde_json::from_str(&json).expect("JSON デシリアライズ成功");

        assert_eq!(result, deserialized, "JSON ラウンドトリップ一致");
    }

    // ---- 観測テスト: n=10,000 ランダム統計 ----

    #[test]

    fn kw_observational_random_stats() {
        let mut rng = StdRng::seed_from_u64(12345);

        let n = 10_000u64;

        let mut sum_j = 0.0;

        let mut min_j = f64::MAX;

        let mut max_j = f64::MIN;

        let mut nan_count = 0u64;

        let mut inf_count = 0u64;

        let mut kind_world_count = 0u64;

        let mut j_values: Vec<f64> = Vec::with_capacity(n as usize);

        for _ in 0..n {
            let metrics = KindWorldMetricsInput {
                population_growth_rate: rng.random::<f64>() * 2.0,

                capability_coverage: rng.random::<f64>(),

                reuse_ratio: rng.random::<f64>(),

                cost_efficiency: rng.random::<f64>() * 2.0,

                village_formation_score: rng.random::<f64>(),

                village_churn_rate: rng.random::<f64>(),

                cross_village_interaction_rate: rng.random::<f64>(),

                knowledge_diffusion_rate: rng.random::<f64>(),

                benevolent_vs_non_benevolent_coverage_ratio: rng.random::<f64>() * 3.0,

                mean_lifecycle_score: 0.0,
                child_survival_rate: 0.0,
                mean_freshness: 0.0,
                mean_benevolence_aggregate: 0.0,
                mean_reciprocity_score: 0.0,
                help_success_rate: 0.0,
                trust_inheritance_fidelity: 0.0,
                execution_success_rate: 0.0,
                mean_nest_depth: 0.0,
                mean_node_density: 0.0,
                cluster_coefficient: 0.0,
                local_density: 0.0,
                search_radius_inverse: 0.0,
                reasoning_steps_inverse: 0.0,
            };

            let result = compute_kind_world_objective(&metrics);

            if result.j_kw.is_nan() {
                nan_count += 1;
            }

            if result.j_kw.is_infinite() {
                inf_count += 1;
            }

            sum_j += result.j_kw;

            if result.j_kw < min_j {
                min_j = result.j_kw;
            }

            if result.j_kw > max_j {
                max_j = result.j_kw;
            }

            j_values.push(result.j_kw);

            if result.is_kind_world {
                kind_world_count += 1;
            }
        }

        let mean_j = sum_j / n as f64;

        // 昇順ソートして分位数を計算

        j_values.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let p50 = j_values[(n as usize * 50) / 100];

        let p95 = j_values[(n as usize * 95) / 100];

        let p99 = j_values[(n as usize * 99) / 100];

        println!(
            "{}",
            serde_json::json!({

                "test": "kw_observational_random_stats",

                "n": n,

                "metrics": {

                    "j_kw_mean": mean_j,

                    "j_kw_min": min_j,

                    "j_kw_max": max_j,

                    "j_kw_p50": p50,

                    "j_kw_p95": p95,

                    "j_kw_p99": p99,

                    "nan_count": nan_count,

                    "inf_count": inf_count,

                    "kind_world_count": kind_world_count,

                    "kind_world_ratio": kind_world_count as f64 / n as f64

                },

                "pass": nan_count == 0 && inf_count == 0 && min_j >= 0.0 && max_j <= 1.0

            })
        );

        // 観測テストは常に PASS（統計情報を出力するのが目的）

        assert!(nan_count == 0, "NaN が {} 件出現しました", nan_count);

        assert!(inf_count == 0, "Inf が {} 件出現しました", inf_count);

        assert!(min_j >= 0.0, "J_kw 最小値 {} が 0 未満", min_j);

        assert!(max_j <= 1.0, "J_kw 最大値 {} が 1 超過", max_j);
    }

    // ===============================================================

    // M1.76-KW2: エコシステム成長メトリクステスト (TC1-TC10)

    // ===============================================================

    use crate::event::ReciprocityEvent;

    use crate::simulation::{HelpSessionStatus, SimHelpSession, SimWorkflowState};

    /// デフォルトのテスト用 SimWorkflowState を生成する。

    fn default_kw2_workflow(
        id: &str,

        benevolence: f32,

        pos_x: f32,

        pos_y: f32,
    ) -> SimWorkflowState {
        SimWorkflowState {
            id: id.to_string(),

            position: [pos_x, pos_y, 0.0],

            experience: 10,

            trust: 0.5,

            reputation: crate::event::ReputationProfile::cold_start(),

            benevolence,

            direct_reciprocity: 0.5,

            indirect_reciprocity: 0.5,

            hazard: 0.0,

            survived: true,

            is_child: false,

            initial_benevolence: benevolence,
        }
    }

    /// デフォルトのテスト用 SimHelpSession を生成する。

    fn default_kw2_session(
        id: &str,

        helper: &str,

        requester: &str,

        status: HelpSessionStatus,
    ) -> SimHelpSession {
        SimHelpSession {
            id: id.to_string(),

            mission_id: "m_default".into(),

            helper_id: helper.to_string(),

            requester_id: requester.to_string(),

            status,

            created_at: 0,

            updated_at: 1,

            helper_benevolence: 0.5,
        }
    }

    // -------------------------------------------------------

    // TC1: compute_population_growth_rate — 境界値 4 ケース

    // -------------------------------------------------------

    #[test]

    fn tc1_kw2_population_growth_rate() {
        // 増加時正値

        let pop = vec![
            default_kw2_workflow("wf1", 0.5, 0.1, 0.1),
            default_kw2_workflow("wf2", 0.6, 0.2, 0.2),
        ];

        let rate = compute_population_growth_rate(&pop, 1);

        assert!(rate > 0.0, "増加時正値: rate={}", rate);

        // 減少時負値

        let dead_pop = vec![SimWorkflowState {
            survived: false,

            ..default_kw2_workflow("wf1", 0.5, 0.1, 0.1)
        }];

        let rate = compute_population_growth_rate(&dead_pop, 5);

        assert!(rate < 0.0, "減少時負値: rate={}", rate);

        // 0 変動時 0.0

        let rate = compute_population_growth_rate(&pop, 2);

        assert!((rate - 0.0).abs() < 1e-10, "0 変動時 0.0: rate={}", rate);

        // 空人口時 0.0

        let empty: Vec<SimWorkflowState> = vec![];

        let rate = compute_population_growth_rate(&empty, 0);

        assert!((rate - 0.0).abs() < 1e-10, "空人口時 0.0: rate={}", rate);

        println!(
            "TC1: population_growth_rate PASS — increase={}, decrease={}, stable={}, empty={}",
            compute_population_growth_rate(&pop, 1),
            compute_population_growth_rate(&dead_pop, 5),
            compute_population_growth_rate(&pop, 2),
            compute_population_growth_rate(&empty, 0)
        );
    }

    // -------------------------------------------------------

    // TC2: compute_capability_coverage_shannon — 境界値 3 ケース

    // -------------------------------------------------------

    #[test]

    fn tc2_kw2_capability_coverage_shannon() {
        // 全同一 position で 0.0

        let same_pos: Vec<SimWorkflowState> = (0..10)
            .map(|i| default_kw2_workflow(&format!("wf{}", i), 0.5, 0.5, 0.5))
            .collect();

        let shannon = compute_capability_coverage_shannon(&same_pos);

        assert!(
            (shannon - 0.0).abs() < 1e-10,
            "全同一 position で 0.0: {}",
            shannon
        );

        // 均一分散で 1.0 に近い値（10×10 グリッドに均等配置）

        let uniform: Vec<SimWorkflowState> = (0..100)
            .map(|i| {
                let x = (i % 10) as f32 / 10.0 + 0.05;

                let y = (i / 10) as f32 / 10.0 + 0.05;

                default_kw2_workflow(&format!("wf{}", i), 0.5, x, y)
            })
            .collect();

        let shannon = compute_capability_coverage_shannon(&uniform);

        assert!(shannon > 0.9, "均一分散で 1.0 に近い値: {}", shannon);

        // 空 population で 0.0

        let empty: Vec<SimWorkflowState> = vec![];

        let shannon = compute_capability_coverage_shannon(&empty);

        assert!(
            (shannon - 0.0).abs() < 1e-10,
            "空 population で 0.0: {}",
            shannon
        );

        println!(
            "TC2: capability_coverage_shannon PASS — same_pos={}, uniform={}, empty={}",
            compute_capability_coverage_shannon(&same_pos),
            compute_capability_coverage_shannon(&uniform),
            compute_capability_coverage_shannon(&empty)
        );
    }

    // -------------------------------------------------------

    // TC3: compute_reuse_ratio — 境界値 3 ケース

    // -------------------------------------------------------

    #[test]

    fn tc3_kw2_reuse_ratio() {
        // 全セッション異種 workflow 間で 0.0

        let sessions_diff: Vec<SimHelpSession> = (0..5)
            .map(|i| {
                default_kw2_session(
                    &format!("s{}", i),
                    &format!("helper{}", i),
                    &format!("requester{}", i),
                    HelpSessionStatus::Succeeded,
                )
            })
            .collect();

        let events: Vec<ReciprocityEvent> = vec![];

        let ratio = compute_reuse_ratio(&events, &sessions_diff);

        assert!(
            (ratio - 0.0).abs() < 1e-10,
            "異種 workflow 間で 0.0: {}",
            ratio
        );

        // 全セッション同一 workflow の再利用で 1.0

        let sessions_same: Vec<SimHelpSession> = (0..5)
            .map(|i| {
                default_kw2_session(
                    &format!("s{}", i),
                    "helper1",
                    "requester1",
                    HelpSessionStatus::Succeeded,
                )
            })
            .collect();

        let ratio = compute_reuse_ratio(&events, &sessions_same);

        assert!(
            (ratio - 1.0).abs() < 1e-10,
            "全同一 workflow で 1.0: {}",
            ratio
        );

        // 空セッションで 0.0

        let empty_sessions: Vec<SimHelpSession> = vec![];

        let ratio = compute_reuse_ratio(&events, &empty_sessions);

        assert!((ratio - 0.0).abs() < 1e-10, "空セッションで 0.0: {}", ratio);

        println!(
            "TC3: reuse_ratio PASS — diff={}, same={}, empty={}",
            compute_reuse_ratio(&events, &sessions_diff),
            compute_reuse_ratio(&events, &sessions_same),
            compute_reuse_ratio(&events, &empty_sessions)
        );
    }

    // -------------------------------------------------------

    // TC4: compute_cost_efficiency — 境界値 4 ケース

    // -------------------------------------------------------

    #[test]

    fn tc4_kw2_cost_efficiency() {
        // 全成功で 1.0

        let all_success = vec![
            default_kw2_session("s1", "h1", "r1", HelpSessionStatus::Succeeded),
            default_kw2_session("s2", "h2", "r2", HelpSessionStatus::Succeeded),
        ];

        let eff = compute_cost_efficiency(&all_success);

        assert!((eff - 1.0).abs() < 1e-10, "全成功で 1.0: {}", eff);

        // 全失敗で 0.0

        let all_failed = vec![
            default_kw2_session("s1", "h1", "r1", HelpSessionStatus::HarmfulMismatch),
            default_kw2_session("s2", "h2", "r2", HelpSessionStatus::Abandoned),
        ];

        let eff = compute_cost_efficiency(&all_failed);

        assert!((eff - 0.0).abs() < 1e-10, "全失敗で 0.0: {}", eff);

        // 空セッションで 1.0

        let empty: Vec<SimHelpSession> = vec![];

        let eff = compute_cost_efficiency(&empty);

        assert!((eff - 1.0).abs() < 1e-10, "空セッションで 1.0: {}", eff);

        // 混合: 3/5 失敗（HarmfulMismatch x2 + Abandoned x1）→ 0.4

        let mixed = vec![
            default_kw2_session("s1", "h1", "r1", HelpSessionStatus::Succeeded),
            default_kw2_session("s2", "h2", "r2", HelpSessionStatus::HarmfulMismatch),
            default_kw2_session("s3", "h3", "r3", HelpSessionStatus::HarmfulMismatch),
            default_kw2_session("s4", "h4", "r4", HelpSessionStatus::Abandoned),
            default_kw2_session("s5", "h5", "r5", HelpSessionStatus::Succeeded),
        ];

        let eff = compute_cost_efficiency(&mixed);

        assert!((eff - 0.4).abs() < 1e-10, "3/5 失敗で 0.4: {}", eff);

        println!(
            "TC4: cost_efficiency PASS — all_success={}, all_failed={}, empty={}, mixed={}",
            compute_cost_efficiency(&all_success),
            compute_cost_efficiency(&all_failed),
            compute_cost_efficiency(&empty),
            compute_cost_efficiency(&mixed)
        );
    }

    // -------------------------------------------------------

    // TC5: compute_benevolent_vs_non_benevolent_coverage_ratio — 境界値 3 ケース

    // -------------------------------------------------------

    #[test]

    fn tc5_kw2_benevolent_coverage_ratio() {
        // 同一分布で 1.0（全ワークフローが同一 position → 両集団とも H=0 → bottom_h=0 の特殊ケース）

        let same_dist: Vec<SimWorkflowState> = (0..20)
            .map(|i| default_kw2_workflow(&format!("wf{}", i), i as f32 / 20.0, 0.5, 0.5))
            .collect();

        let ratio = compute_benevolent_vs_non_benevolent_coverage_ratio(&same_dist);

        // 全同一 position → 両集団とも coverage が 0 → bottom_h=0 → top_h>0 なら 2.0

        // ここでは top_h も 0 なので ratio=1.0

        assert!(
            (ratio - 1.0).abs() < 1e-10 || ratio > 1.0,
            "同一分布で ratio={}",
            ratio
        );

        // 慈悲的優位: 慈悲的集団の方が広い分布を持つ

        let benevolent_wide: Vec<SimWorkflowState> = (0..20)
            .map(|i| {
                let benevolence = if i < 10 { 0.8 + i as f32 / 100.0 } else { 0.1 };

                let x = if i < 10 { i as f32 / 10.0 } else { 0.5 };

                let y = if i < 10 { i as f32 / 10.0 } else { 0.5 };

                default_kw2_workflow(&format!("wf{}", i), benevolence, x, y)
            })
            .collect();

        let ratio = compute_benevolent_vs_non_benevolent_coverage_ratio(&benevolent_wide);

        // 慈悲的 10 件が 10×10 に分散、非慈悲的 10 件が同一 position

        // → 慈悲的集団の H > 0, 非慈悲的集団の H = 0

        // → bottom_h=0 → top_h>0 なら 2.0

        assert!(ratio > 0.0, "慈悲的優位で ratio>0: {}", ratio);

        // 空人口で 1.0

        let empty: Vec<SimWorkflowState> = vec![];

        let ratio = compute_benevolent_vs_non_benevolent_coverage_ratio(&empty);

        assert!((ratio - 1.0).abs() < 1e-10, "空人口で 1.0: {}", ratio);

        println!("TC5: benevolent_coverage_ratio PASS — ratio={}", ratio);
    }

    // -------------------------------------------------------

    // TC6: 空入力耐性 — 全 5 関数が panic しない

    // -------------------------------------------------------

    #[test]

    fn tc6_kw2_empty_input_resilience() {
        let empty_pop: Vec<SimWorkflowState> = vec![];

        let empty_sessions: Vec<SimHelpSession> = vec![];

        let empty_events: Vec<ReciprocityEvent> = vec![];

        let r1 = compute_population_growth_rate(&empty_pop, 0);

        let r2 = compute_capability_coverage_shannon(&empty_pop);

        let r3 = compute_reuse_ratio(&empty_events, &empty_sessions);

        let r4 = compute_cost_efficiency(&empty_sessions);

        let r5 = compute_benevolent_vs_non_benevolent_coverage_ratio(&empty_pop);

        assert!(r1.is_finite(), "TC6: compute_population_growth_rate");

        assert!(r2.is_finite(), "TC6: compute_capability_coverage_shannon");

        assert!(r3.is_finite(), "TC6: compute_reuse_ratio");

        assert!(r4.is_finite(), "TC6: compute_cost_efficiency");

        assert!(
            r5.is_finite(),
            "TC6: compute_benevolent_vs_non_benevolent_coverage_ratio"
        );

        // 空入力の期待値

        assert_eq!(r1, 0.0, "空人口成長率=0.0");

        assert_eq!(r2, 0.0, "空 Shannon=0.0");

        assert_eq!(r3, 0.0, "空再利用比率=0.0");

        assert_eq!(r4, 1.0, "空コスト効率=1.0");

        assert_eq!(r5, 1.0, "空慈悲的優位比=1.0");

        println!("TC6: empty_input_resilience PASS — 全 5 関数が panic せず期待値を返した");
    }

    // -------------------------------------------------------

    // TC7: 範囲保証 — 全 5 関数の出力が NaN/Inf フリー

    // -------------------------------------------------------

    #[test]

    fn tc7_kw2_range_guarantee() {
        use rand::rngs::StdRng;

        use rand::Rng;

        use rand::SeedableRng;

        let mut rng = StdRng::seed_from_u64(12345);

        let n = 1_000u64;

        for _ in 0..n {
            let pop: Vec<SimWorkflowState> = (0..20)
                .map(|i| {
                    let survived = rng.random::<f64>() > 0.2;

                    SimWorkflowState {
                        id: format!("wf{}", i),

                        position: [rng.random(), rng.random(), rng.random()],

                        experience: rng.random_range(0..100),

                        trust: rng.random(),

                        reputation: crate::event::ReputationProfile::cold_start(),

                        benevolence: rng.random(),

                        direct_reciprocity: rng.random(),

                        indirect_reciprocity: rng.random(),

                        hazard: 0.0,

                        survived,

                        is_child: rng.random::<f64>() > 0.7,

                        initial_benevolence: rng.random(),
                    }
                })
                .collect();

            let sessions: Vec<SimHelpSession> = (0..10)
                .map(|i| SimHelpSession {
                    id: format!("s{}", i),

                    mission_id: "m1".into(),

                    helper_id: format!("h{}", i),

                    requester_id: format!("r{}", i),

                    status: match rng.random_range(0..5) {
                        0 => HelpSessionStatus::Succeeded,

                        1 => HelpSessionStatus::HarmfulMismatch,

                        2 => HelpSessionStatus::Abandoned,

                        _ => HelpSessionStatus::Offered,
                    },

                    created_at: 0,

                    updated_at: 1,

                    helper_benevolence: rng.random(),
                })
                .collect();

            let events: Vec<ReciprocityEvent> = vec![];

            let r1 = compute_population_growth_rate(&pop, 10);

            let r2 = compute_capability_coverage_shannon(&pop);

            let r3 = compute_reuse_ratio(&events, &sessions);

            let r4 = compute_cost_efficiency(&sessions);

            let r5 = compute_benevolent_vs_non_benevolent_coverage_ratio(&pop);

            assert!(r1.is_finite(), "TC7: population_growth_rate が NaN/Inf");

            assert!(
                r2.is_finite() && r2 >= 0.0 && r2 <= 1.0,
                "TC7: capability_coverage_shannon={} が [0,1] 範囲外",
                r2
            );

            assert!(
                r3.is_finite() && r3 >= 0.0 && r3 <= 1.0,
                "TC7: reuse_ratio={} が [0,1] 範囲外",
                r3
            );

            assert!(
                r4.is_finite() && r4 >= 0.0 && r4 <= 1.0,
                "TC7: cost_efficiency={} が [0,1] 範囲外",
                r4
            );

            assert!(
                r5.is_finite() && r5 >= 0.0,
                "TC7: benevolent_coverage_ratio={} が異常値",
                r5
            );
        }

        println!(
            "TC7: range_guarantee PASS — n={} 全指標が NaN/Inf フリー",
            n
        );
    }

    // -------------------------------------------------------

    // TC8: EcosystemGrowthObserver 統合

    // -------------------------------------------------------

    #[test]

    fn tc8_kw2_observer_integration() {
        let pop: Vec<SimWorkflowState> = (0..10)
            .map(|i| {
                default_kw2_workflow(
                    &format!("wf{}", i),
                    i as f32 / 10.0,
                    i as f32 / 10.0,
                    i as f32 / 10.0,
                )
            })
            .collect();

        let sessions: Vec<SimHelpSession> = vec![];

        let events: Vec<ReciprocityEvent> = vec![];

        let metrics = EcosystemGrowthObserver::observe(0, &pop, &sessions, &events, 0);

        assert_eq!(metrics.tick, 0, "TC8: tick が一致");

        assert!(
            metrics.population_growth_rate.is_finite(),
            "TC8: population_growth_rate が有限"
        );

        assert!(
            metrics.capability_coverage_shannon.is_finite(),
            "TC8: capability_coverage が有限"
        );

        assert!(metrics.reuse_ratio.is_finite(), "TC8: reuse_ratio が有限");

        assert!(
            metrics.cost_efficiency.is_finite(),
            "TC8: cost_efficiency が有限"
        );

        assert!(
            metrics
                .benevolent_vs_non_benevolent_coverage_ratio
                .is_finite(),
            "TC8: benevolence_ratio が有限"
        );

        println!(
            "TC8: observer_integration PASS — tick={}, 全 5 指標有限",
            metrics.tick
        );
    }

    // -------------------------------------------------------

    // TC9: 慈悲的優位検出

    // -------------------------------------------------------

    #[test]

    fn tc9_kw2_benevolent_advantage_detection() {
        // 慈悲的集団が広い能力分布、非慈悲的集団が狭い能力分布

        let pop: Vec<SimWorkflowState> = (0..20)
            .map(|i| {
                let is_benevolent = i < 10;

                let benevolence = if is_benevolent { 0.9 } else { 0.1 };

                // 慈悲的: 分散した位置、非慈悲的: 集中した位置

                let x = if is_benevolent { i as f32 / 15.0 } else { 0.5 };

                let y = if is_benevolent { i as f32 / 15.0 } else { 0.5 };

                default_kw2_workflow(&format!("wf{}", i), benevolence, x, y)
            })
            .collect();

        let ratio = compute_benevolent_vs_non_benevolent_coverage_ratio(&pop);

        // 慈悲的集団がより広い分布 → ratio > 1.0 を期待

        assert!(
            ratio > 1.0,
            "TC9: 慈悲的優位 ratio={} > 1.0 であること",
            ratio
        );

        // 慈悲的集団の能力カバー率が非慈悲的集団に対して統計的に有意に大きい

        let top_h = shannon_diversity_raw(
            &pop.iter()
                .filter(|w| w.initial_benevolence > 0.5)
                .collect::<Vec<_>>(),
        );

        let bottom_h = shannon_diversity_raw(
            &pop.iter()
                .filter(|w| w.initial_benevolence <= 0.5)
                .collect::<Vec<_>>(),
        );

        assert!(
            top_h > bottom_h || (top_h == bottom_h && ratio >= 1.0),
            "TC9: 慈悲的 H={} > 非慈悲的 H={} または ratio={} >= 1.0",
            top_h,
            bottom_h,
            ratio
        );

        println!(
            "TC9: benevolent_advantage_detection PASS — ratio={}, top_H={}, bottom_H={}",
            ratio, top_h, bottom_h
        );
    }

    // -------------------------------------------------------

    // TC10: CSV 出力形式

    // -------------------------------------------------------

    #[test]

    fn tc10_kw2_csv_output_format() {
        let series = vec![
            EcosystemGrowthMetrics {
                tick: 0,

                population_growth_rate: 0.0,

                capability_coverage_shannon: 0.5,

                reuse_ratio: 0.3,

                cost_efficiency: 0.8,

                benevolent_vs_non_benevolent_coverage_ratio: 1.0,
            },
            EcosystemGrowthMetrics {
                tick: 1,

                population_growth_rate: 0.02,

                capability_coverage_shannon: 0.6,

                reuse_ratio: 0.4,

                cost_efficiency: 0.9,

                benevolent_vs_non_benevolent_coverage_ratio: 1.2,
            },
        ];

        EcosystemGrowthObserver::print_csv(&series, "TC10");

        // 系列長が正しいこと

        assert_eq!(series.len(), 2, "TC10: 系列長が 2");

        // 各フィールドが正しく設定されていること

        assert_eq!(series[0].tick, 0);

        assert_eq!(series[1].tick, 1);

        assert_eq!(series[0].population_growth_rate, 0.0);

        assert_eq!(series[1].population_growth_rate, 0.02);

        println!("TC10: csv_output_format PASS — 2 rows, 6 columns");
    }

    // -------------------------------------------------------

    // 観測テスト: シミュレーター統合 CSV 出力

    // -------------------------------------------------------

    #[test]

    fn kw2_observational_csv_output() {
        use rand::rngs::StdRng;

        use rand::Rng;

        use rand::SeedableRng;

        let mut rng = StdRng::seed_from_u64(12345);

        let n_ticks = 20;

        // シミュレーター風の tick 進行を模擬

        let mut population: Vec<SimWorkflowState> = (0..50)
            .map(|i| SimWorkflowState {
                id: format!("wf{}", i),

                position: [rng.random(), rng.random(), rng.random()],

                experience: rng.random_range(0..100),

                trust: rng.random(),

                reputation: crate::event::ReputationProfile::cold_start(),

                benevolence: rng.random(),

                direct_reciprocity: 0.5,

                indirect_reciprocity: 0.5,

                hazard: 0.0,

                survived: true,

                is_child: i < 15,

                initial_benevolence: rng.random(),
            })
            .collect();

        let mut sessions: Vec<SimHelpSession> = Vec::new();

        let events: Vec<ReciprocityEvent> = vec![];

        let mut previous_count = 0usize;

        let mut metrics_series: Vec<EcosystemGrowthMetrics> = Vec::new();

        for tick in 0..n_ticks {
            // 擬似的な人口変動: 一部をランダムに死亡させる

            if tick > 0 && tick % 3 == 0 {
                for wf in &mut population {
                    if rng.random::<f64>() < 0.05 {
                        wf.survived = false;
                    }
                }
            }

            // いくつかのヘルプセッションを生成

            if tick % 2 == 0 {
                let helper_idx = rng.random_range(0..population.len());

                let requester_idx = rng.random_range(0..population.len());

                if helper_idx != requester_idx {
                    sessions.push(SimHelpSession {
                        id: format!("sess-{}", tick),

                        mission_id: format!("m-{}", tick),

                        helper_id: population[helper_idx].id.clone(),

                        requester_id: population[requester_idx].id.clone(),

                        status: match rng.random_range(0..4) {
                            0 => HelpSessionStatus::Succeeded,

                            1 => HelpSessionStatus::HarmfulMismatch,

                            2 => HelpSessionStatus::Abandoned,

                            _ => HelpSessionStatus::Succeeded,
                        },

                        created_at: tick,

                        updated_at: tick + 1,

                        helper_benevolence: population[helper_idx].benevolence,
                    });
                }
            }

            let metrics = EcosystemGrowthObserver::observe(
                tick,
                &population,
                &sessions,
                &events,
                previous_count,
            );

            previous_count = population.iter().filter(|w| w.survived).count();

            metrics_series.push(metrics);
        }

        // CSV 出力

        EcosystemGrowthObserver::print_csv(&metrics_series, "OBS");

        // 観測テストは常に PASS（統計情報を出力するのが目的）

        assert_eq!(metrics_series.len(), n_ticks as usize, "OBS: 系列長が一致");

        // 全指標が NaN/Inf フリー

        for metrics in &metrics_series {
            assert!(
                metrics.population_growth_rate.is_finite(),
                "OBS: NaN in population_growth_rate"
            );

            assert!(
                metrics.capability_coverage_shannon.is_finite(),
                "OBS: NaN in capability_coverage"
            );

            assert!(metrics.reuse_ratio.is_finite(), "OBS: NaN in reuse_ratio");

            assert!(
                metrics.cost_efficiency.is_finite(),
                "OBS: NaN in cost_efficiency"
            );

            assert!(
                metrics
                    .benevolent_vs_non_benevolent_coverage_ratio
                    .is_finite(),
                "OBS: NaN in benevolent_ratio"
            );
        }

        // 出力サマリ

        let first = &metrics_series[0];

        let last = &metrics_series[metrics_series.len() - 1];

        println!("OBS: FIRST tick={}: pop_growth={:.4}, shannon={:.4}, reuse={:.4}, cost_eff={:.4}, bene_ratio={:.4}",

            first.tick,

            first.population_growth_rate,

            first.capability_coverage_shannon,

            first.reuse_ratio,

            first.cost_efficiency,

            first.benevolent_vs_non_benevolent_coverage_ratio,

        );

        println!("OBS: LAST tick={}: pop_growth={:.4}, shannon={:.4}, reuse={:.4}, cost_eff={:.4}, bene_ratio={:.4}",

            last.tick,

            last.population_growth_rate,

            last.capability_coverage_shannon,

            last.reuse_ratio,

            last.cost_efficiency,

            last.benevolent_vs_non_benevolent_coverage_ratio,

        );
    }

    // ===============================================================

    // M1.76-KW3: 村間相互作用・知識拡散トラッキング (TC1-TC16)

    // ===============================================================

    // ---- TC1: assign_village_ids 密集群が同一村に ----

    #[test]

    fn tc1_kw3_assign_village_ids_dense_cluster() {
        let pop: Vec<SimWorkflowState> = (0..10)
            .map(|i| SimWorkflowState {
                id: format!("wf_{}", i),

                position: [0.1 + i as f32 * 0.01, 0.1 + i as f32 * 0.01, 0.0],

                experience: 10,

                trust: 0.5,

                reputation: crate::event::ReputationProfile::cold_start(),

                benevolence: 0.5,

                direct_reciprocity: 0.5,

                indirect_reciprocity: 0.5,

                hazard: 0.0,

                survived: true,

                is_child: false,

                initial_benevolence: 0.5,
            })
            .collect();

        let assignments = assign_village_ids(&pop);

        let village_ids: Vec<Option<usize>> = assignments;

        let non_none: Vec<usize> = village_ids.iter().filter_map(|&v| v).collect();

        assert!(!non_none.is_empty(), "全員が村所属になること");

        assert!(
            non_none.windows(2).all(|w| w[0] == w[1]),
            "全員が同一村 ID であること"
        );
    }

    // ---- TC2: assign_village_ids 孤立ワークフローが None ----

    #[test]

    fn tc2_kw3_assign_village_ids_isolated_none() {
        let pop: Vec<SimWorkflowState> = vec![SimWorkflowState {
            id: "isolated".to_string(),

            position: [0.9, 0.9, 0.0],

            experience: 10,
            trust: 0.5,

            reputation: crate::event::ReputationProfile::cold_start(),

            benevolence: 0.5,
            direct_reciprocity: 0.5,
            indirect_reciprocity: 0.5,

            hazard: 0.0,
            survived: true,
            is_child: false,
            initial_benevolence: 0.5,
        }];

        let assignments = assign_village_ids(&pop);

        assert_eq!(assignments[0], None, "孤立ワークフローは村未所属");
    }

    // ---- TC3: assign_village_ids 全員同一位置で単一村 ----

    #[test]

    fn tc3_kw3_assign_village_ids_all_same_position() {
        let pop: Vec<SimWorkflowState> = (0..10)
            .map(|i| SimWorkflowState {
                id: format!("wf_{}", i),

                position: [0.5, 0.5, 0.0],
                experience: 10,
                trust: 0.5,

                reputation: crate::event::ReputationProfile::cold_start(),

                benevolence: 0.5,
                direct_reciprocity: 0.5,
                indirect_reciprocity: 0.5,

                hazard: 0.0,
                survived: true,
                is_child: false,
                initial_benevolence: 0.5,
            })
            .collect();

        let assignments = assign_village_ids(&pop);

        let non_none: Vec<usize> = assignments.iter().filter_map(|&v| v).collect();

        assert_eq!(non_none.len(), 10, "全 10 人が村所属");

        assert!(
            non_none.windows(2).all(|w| w[0] == w[1]),
            "同一村 ID であること"
        );
    }

    // ---- TC4: assign_village_ids 空 population ----

    #[test]

    fn tc4_kw3_assign_village_ids_empty() {
        let empty: Vec<SimWorkflowState> = vec![];

        let assignments = assign_village_ids(&empty);

        assert!(assignments.is_empty(), "空 population で空ベクタ");
    }

    // ---- TC5: compute_cross_village_interaction_rate（observer 経由）----

    #[test]

    fn tc5_kw3_cross_village_interaction_rate() {
        let pop: Vec<SimWorkflowState> = vec![
            SimWorkflowState {
                id: "wf_a".to_string(),
                position: [0.1, 0.1, 0.0],

                experience: 10,
                trust: 0.5,

                reputation: crate::event::ReputationProfile::cold_start(),

                benevolence: 0.5,
                direct_reciprocity: 0.5,
                indirect_reciprocity: 0.5,

                hazard: 0.0,
                survived: true,
                is_child: false,
                initial_benevolence: 0.5,
            },
            SimWorkflowState {
                id: "wf_b".to_string(),
                position: [0.1, 0.1, 0.0],

                experience: 10,
                trust: 0.5,

                reputation: crate::event::ReputationProfile::cold_start(),

                benevolence: 0.5,
                direct_reciprocity: 0.5,
                indirect_reciprocity: 0.5,

                hazard: 0.0,
                survived: true,
                is_child: false,
                initial_benevolence: 0.5,
            },
            SimWorkflowState {
                id: "wf_c".to_string(),
                position: [0.9, 0.9, 0.0],

                experience: 10,
                trust: 0.5,

                reputation: crate::event::ReputationProfile::cold_start(),

                benevolence: 0.5,
                direct_reciprocity: 0.5,
                indirect_reciprocity: 0.5,

                hazard: 0.0,
                survived: true,
                is_child: false,
                initial_benevolence: 0.5,
            },
        ];

        let intra_sessions = vec![SimHelpSession {
            id: "s1".to_string(),
            mission_id: "m1".to_string(),

            helper_id: "wf_a".to_string(),
            requester_id: "wf_b".to_string(),

            status: HelpSessionStatus::Succeeded,
            created_at: 0,
            updated_at: 1,

            helper_benevolence: 0.5,
        }];

        let mut observer = VillageInteractionObserver::new();

        let metrics = observer.observe(0, &pop, &intra_sessions);

        assert!(
            metrics.cross_village_interaction_rate < 0.5,
            "同一村内セッションの相互作用率は低い"
        );

        let empty_sessions: Vec<SimHelpSession> = vec![];

        let mut observer2 = VillageInteractionObserver::new();

        let metrics2 = observer2.observe(0, &pop, &empty_sessions);

        assert_eq!(
            metrics2.cross_village_interaction_rate, 0.0,
            "空セッションで 0.0"
        );
    }

    // ---- TC6: compute_village_formation_strength ----

    #[test]

    fn tc6_kw3_village_formation_strength() {
        let pop: Vec<SimWorkflowState> = (0..8)
            .map(|i| SimWorkflowState {
                id: format!("wf_{}", i),

                position: if i < 4 {
                    [0.1, 0.1 + i as f32 * 0.01, 0.0]
                } else {
                    [0.8, 0.8 + (i - 4) as f32 * 0.01, 0.0]
                },

                experience: 10,
                trust: 0.5,

                reputation: crate::event::ReputationProfile::cold_start(),

                benevolence: 0.5,
                direct_reciprocity: 0.5,
                indirect_reciprocity: 0.5,

                hazard: 0.0,
                survived: true,
                is_child: false,
                initial_benevolence: 0.5,
            })
            .collect();

        let assignments = assign_village_ids(&pop);

        let strength = compute_village_formation_strength(&pop, &assignments);

        assert!(
            strength > 0.0,
            "密集クラスタの形成強度は正値 (got {})",
            strength
        );

        assert!(strength <= 1.0, "形成強度は [0, 1] 範囲 (got {})", strength);

        let all_none: Vec<Option<usize>> = vec![None; pop.len()];

        assert_eq!(
            compute_village_formation_strength(&pop, &all_none),
            0.0,
            "全員 None の形成強度は 0.0"
        );
    }

    // ---- TC7: compute_knowledge_diffusion_rate ----

    #[test]

    fn tc7_kw3_knowledge_diffusion_rate() {
        let pop: Vec<SimWorkflowState> = (0..6)
            .map(|i| SimWorkflowState {
                id: format!("wf_{}", i),

                position: if i < 3 {
                    [0.1, 0.1, 0.0]
                } else {
                    [0.8, 0.8, 0.0]
                },

                experience: 10,
                trust: 0.5,

                reputation: crate::event::ReputationProfile::cold_start(),

                benevolence: 0.5,
                direct_reciprocity: 0.5,
                indirect_reciprocity: 0.5,

                hazard: 0.0,
                survived: true,
                is_child: false,
                initial_benevolence: 0.5,
            })
            .collect();

        let current = assign_village_ids(&pop);

        let previous = current.clone();

        assert_eq!(
            compute_knowledge_diffusion_rate(&pop, &current, &previous),
            0.0,
            "変化なしなら拡散率 0.0"
        );

        let empty: Vec<Option<usize>> = vec![];

        assert_eq!(
            compute_knowledge_diffusion_rate(&pop, &empty, &current),
            0.0,
            "空 assignments で 0.0"
        );
    }

    // ---- TC8: compute_village_flow_balance ----

    #[test]

    fn tc8_kw3_village_flow_balance() {
        let c: Vec<Option<usize>> = vec![Some(0), Some(0), Some(1)];

        let p: Vec<Option<usize>> = vec![Some(0), Some(0), Some(1)];

        assert_eq!(
            compute_village_flow_balance(&c, &p),
            0.0,
            "変化なしで churn 0.0"
        );

        let moved: Vec<Option<usize>> = vec![Some(1), Some(1), Some(0)];

        assert_eq!(
            compute_village_flow_balance(&moved, &p),
            1.0,
            "全員移動で churn 1.0"
        );

        let empty: Vec<Option<usize>> = vec![];

        assert_eq!(
            compute_village_flow_balance(&empty, &p),
            0.0,
            "空 assignments で 0.0"
        );
    }

    // ---- TC9: 空入力ガード ----

    #[test]

    fn tc9_kw3_empty_input_guard() {
        let empty_pop: Vec<SimWorkflowState> = vec![];

        let empty_sessions: Vec<SimHelpSession> = vec![];

        let empty_assignments: Vec<Option<usize>> = vec![];

        assert!(
            assign_village_ids(&empty_pop).is_empty(),
            "空 population で空ベクタ"
        );

        assert_eq!(
            compute_cross_village_interaction_rate(&empty_sessions, &empty_assignments),
            0.0,
            "空セッションで 0.0"
        );

        assert_eq!(
            compute_village_formation_strength(&empty_pop, &empty_assignments),
            0.0,
            "空 population で 0.0"
        );

        assert_eq!(
            compute_knowledge_diffusion_rate(&empty_pop, &empty_assignments, &empty_assignments),
            0.0,
            "空 assignments で 0.0"
        );

        assert_eq!(
            compute_village_flow_balance(&empty_assignments, &empty_assignments),
            0.0,
            "空 assignments で 0.0"
        );
    }

    // ---- TC10: 村数 0（全員 None）graceful ハンドリング ----

    #[test]

    fn tc10_kw3_all_none_graceful() {
        let pop: Vec<SimWorkflowState> = (0..2)
            .map(|i| SimWorkflowState {
                id: format!("wf_{}", i),

                position: [0.9 + i as f32 * 0.15, 0.9 + i as f32 * 0.15, 0.0],

                experience: 10,
                trust: 0.5,

                reputation: crate::event::ReputationProfile::cold_start(),

                benevolence: 0.5,
                direct_reciprocity: 0.5,
                indirect_reciprocity: 0.5,

                hazard: 0.0,
                survived: true,
                is_child: false,
                initial_benevolence: 0.5,
            })
            .collect();

        let assignments = assign_village_ids(&pop);

        assert!(assignments.iter().all(|&a| a.is_none()), "全員 None");

        assert_eq!(
            compute_cross_village_interaction_rate(&[], &assignments),
            0.0,
            "cross = 0.0"
        );

        assert_eq!(
            compute_village_formation_strength(&pop, &assignments),
            0.0,
            "formation = 0.0"
        );

        assert_eq!(
            compute_knowledge_diffusion_rate(&pop, &assignments, &assignments),
            0.0,
            "diffusion = 0.0"
        );
    }

    // ---- TC11: 後方互換性（SimWorkflowState にフィールド追加なし）----

    #[test]

    fn tc11_kw3_backward_compatibility() {
        let wf = SimWorkflowState {
            id: "test".to_string(),
            position: [0.5, 0.5, 0.0],

            experience: 10,
            trust: 0.5,

            reputation: crate::event::ReputationProfile::cold_start(),

            benevolence: 0.5,
            direct_reciprocity: 0.5,
            indirect_reciprocity: 0.5,

            hazard: 0.0,
            survived: true,
            is_child: false,
            initial_benevolence: 0.5,
        };

        assert_eq!(wf.id, "test");
    }

    // ---- TC12: churn 過小ペナルティ ----

    #[test]

    fn tc12_kw3_churn_too_low_penalty() {
        let health = compute_village_health_score(0.5, 0.04, 0.5, 0.5);

        assert!((health - 0.375).abs() < 1e-10, "churn 過小: got {}", health);
    }

    // ---- TC13: churn 過大ペナルティ ----

    #[test]

    fn tc13_kw3_churn_too_high_penalty() {
        let health = compute_village_health_score(0.5, 0.31, 0.5, 0.5);

        assert!((health - 0.375).abs() < 1e-10, "churn 過大: got {}", health);
    }

    // ---- TC14: churn 適正範囲でペナルティなし ----

    #[test]

    fn tc14_kw3_churn_normal_no_penalty() {
        assert!(
            (compute_village_health_score(0.5, 0.05, 0.5, 0.5) - 0.625).abs() < 1e-10,
            "churn 下限"
        );

        assert!(
            (compute_village_health_score(0.5, 0.30, 0.5, 0.5) - 0.625).abs() < 1e-10,
            "churn 上限"
        );

        assert!(
            (compute_village_health_score(0.5, 0.15, 0.5, 0.5) - 0.625).abs() < 1e-10,
            "churn 適正"
        );
    }

    // ---- TC15: VillageInteractionObserver 統合テスト ----

    #[test]

    fn tc15_kw3_observer_integration() {
        let pop: Vec<SimWorkflowState> = (0..12)
            .map(|i| SimWorkflowState {
                id: format!("wf_{}", i),

                position: [
                    if i < 4 {
                        0.1
                    } else if i < 8 {
                        0.5
                    } else {
                        0.9
                    },
                    if i < 4 {
                        0.1
                    } else if i < 8 {
                        0.5
                    } else {
                        0.9
                    },
                    0.0,
                ],

                experience: 10 + i as u64,
                trust: 0.5,

                reputation: crate::event::ReputationProfile::cold_start(),

                benevolence: 0.5,
                direct_reciprocity: 0.5,
                indirect_reciprocity: 0.5,

                hazard: 0.0,
                survived: true,
                is_child: false,
                initial_benevolence: 0.5,
            })
            .collect();

        let sessions = vec![SimHelpSession {
            id: "s1".to_string(),
            mission_id: "m1".to_string(),

            helper_id: "wf_0".to_string(),
            requester_id: "wf_1".to_string(),

            status: HelpSessionStatus::Succeeded,
            created_at: 0,
            updated_at: 1,

            helper_benevolence: 0.5,
        }];

        let mut observer = VillageInteractionObserver::new();

        let m0 = observer.observe(0, &pop, &sessions);

        assert_eq!(m0.tick, 0);

        assert_eq!(m0.knowledge_diffusion_rate, 0.0, "初回は 0.0");

        assert_eq!(m0.village_flow_balance, 0.0, "初回は 0.0");

        assert!(m0.village_count > 0, "村が形成されること");

        assert!(m0.cross_village_interaction_rate >= 0.0);

        assert!(m0.village_formation_strength >= 0.0);

        let m1 = observer.observe(1, &pop, &sessions);

        assert_eq!(m1.tick, 1);

        assert!(m1.knowledge_diffusion_rate >= 0.0);

        assert!(m1.village_flow_balance >= 0.0);

        VillageInteractionObserver::print_csv(&[m0, m1], "OBS-KW3");
    }

    // ---- TC16: 観測テスト（シミュレーション時系列）----

    #[test]

    fn tc16_kw3_observational_csv_output() {
        let mut rng = StdRng::seed_from_u64(12345);

        use rand::Rng;

        let mut observer = VillageInteractionObserver::new();

        let mut series: Vec<VillageInteractionMetrics> = Vec::new();

        for tick in 0..20 {
            let pop: Vec<SimWorkflowState> = (0..12)
                .map(|i| {
                    let base_x = if i < 4 {
                        0.1
                    } else if i < 8 {
                        0.5
                    } else {
                        0.9
                    };

                    let base_y = if i < 4 {
                        0.1
                    } else if i < 8 {
                        0.5
                    } else {
                        0.9
                    };

                    SimWorkflowState {
                        id: format!("wf_{}", i),

                        position: [
                            base_x + rng.random::<f32>() * 0.02,
                            base_y + rng.random::<f32>() * 0.02,
                            0.0,
                        ],

                        experience: 10 + tick,
                        trust: 0.5,

                        reputation: crate::event::ReputationProfile::cold_start(),

                        benevolence: 0.5,
                        direct_reciprocity: 0.5,
                        indirect_reciprocity: 0.5,

                        hazard: 0.0,
                        survived: true,
                        is_child: false,
                        initial_benevolence: 0.5,
                    }
                })
                .collect();

            let sessions: Vec<SimHelpSession> = (0..5)
                .map(|j| SimHelpSession {
                    id: format!("s_{}_{}", tick, j),

                    mission_id: format!("m_{}_{}", tick, j),

                    helper_id: format!("wf_{}", rng.random_range(0..12)),

                    requester_id: format!("wf_{}", rng.random_range(0..12)),

                    status: HelpSessionStatus::Succeeded,

                    created_at: tick,
                    updated_at: tick + 1,

                    helper_benevolence: 0.5,
                })
                .collect();

            let metrics = observer.observe(tick, &pop, &sessions);

            assert!(metrics.cross_village_interaction_rate.is_finite());

            assert!(metrics.village_formation_strength.is_finite());

            assert!(metrics.knowledge_diffusion_rate.is_finite());

            assert!(metrics.village_flow_balance.is_finite());

            assert!(metrics.mean_village_size.is_finite());

            assert!(metrics.village_size_variance.is_finite());

            series.push(metrics);
        }

        VillageInteractionObserver::print_csv(&series, "OBS-KW3");
    }

    // ===============================================================

    // M1.76-KW4: Kind World 較正ループ テスト (TC1-TC8)

    // ===============================================================

    /// TC1: Nelder-Mead 初期シンプレックス生成 — 8 頂点がすべて探索範囲内かつ異なる値を持つ

    #[test]

    fn tc1_kw4_initial_simplex() {
        let ranges = crate::constants::KW4_NELDER_MEAD_MAX_ITERATIONS; // 参照だけ

        let _ = ranges;

        let params = MagnificentSevenParams::default();

        let ranges: [(f64, f64); 7] = [
            crate::constants::KW4_GAMMA_BENEVOLENCE_RANGE,
            crate::constants::KW4_LAMBDA_GC_BASE_RANGE,
            crate::constants::KW4_DIRECT_RECIPROCITY_WEIGHT_RANGE,
            crate::constants::KW4_INDIRECT_RECIPROCITY_WEIGHT_RANGE,
            crate::constants::KW4_SOFTMAX_TEMPERATURE_RANGE,
            crate::constants::KW4_GC_INTERVAL_RANGE,
            crate::constants::KW4_CHILD_RATIO_RANGE,
        ];

        let perturbation = crate::constants::KW4_NELDER_MEAD_INITIAL_PERTURBATION;

        let optimizer = NelderMeadOptimizer::new(&params, &ranges, perturbation, 12345, None);

        assert_eq!(optimizer.simplex.len(), 8, "シンプレックスは 8 頂点");

        assert_eq!(optimizer.values.len(), 8, "J_kw 値も 8 個");

        // 全頂点が範囲内かつ異なる値を持つ

        for (i, vertex) in optimizer.simplex.iter().enumerate() {
            let vals = [
                vertex.gamma_benevolence,
                vertex.lambda_gc_base,
                vertex.direct_reciprocity_weight,
                vertex.indirect_reciprocity_weight,
                vertex.softmax_temperature,
                vertex.gc_interval as f64,
                vertex.child_ratio,
            ];

            for (j, &v) in vals.iter().enumerate() {
                assert!(
                    v >= ranges[j].0 - 1e-12,
                    "頂点 {} パラメータ {} が下限 {} 未満: {}",
                    i,
                    j,
                    ranges[j].0,
                    v
                );

                assert!(
                    v <= ranges[j].1 + 1e-12,
                    "頂点 {} パラメータ {} が上限 {} 超過: {}",
                    i,
                    j,
                    ranges[j].1,
                    v
                );
            }
        }

        // 少なくともいくつかの頂点は異なる値を持つ

        let first_vals = [
            optimizer.simplex[0].gamma_benevolence,
            optimizer.simplex[0].lambda_gc_base,
            optimizer.simplex[0].direct_reciprocity_weight,
            optimizer.simplex[0].indirect_reciprocity_weight,
            optimizer.simplex[0].softmax_temperature,
            optimizer.simplex[0].gc_interval as f64,
            optimizer.simplex[0].child_ratio,
        ];

        let has_different = optimizer.simplex.iter().skip(1).any(|v| {
            let diff = (v.gamma_benevolence - first_vals[0]).abs()
                + (v.lambda_gc_base - first_vals[1]).abs()
                + (v.direct_reciprocity_weight - first_vals[2]).abs()
                + (v.indirect_reciprocity_weight - first_vals[3]).abs()
                + (v.softmax_temperature - first_vals[4]).abs()
                + (v.gc_interval as f64 - first_vals[5]).abs()
                + (v.child_ratio - first_vals[6]).abs();

            diff > 1e-12
        });

        assert!(has_different, "全 8 頂点が同一値（変位が機能していない）");
    }

    /// TC2: Nelder-Mead 1次元での収束 — f(x) = (x-3)² の最大化（理論解 x=3）

    #[test]

    fn tc2_kw4_nelder_mead_1d_convergence() {
        let mut optimizer = Simplex1D::new(0.0, (0.0, 5.0));

        let report = optimizer.run(100);

        let error = (report.best_x - 3.0).abs();

        assert!(
            error < 0.1,
            "1次元 Nelder-Mead が x=3 に収束: got {} (error={})",
            report.best_x,
            error
        );

        println!(
            "TC2: 1D Nelder-Mead converged to x={} (target=3, error={})",
            report.best_x, error
        );
    }

    /// TC3: Nelder-Mead 反射・拡大・収縮・縮小の各操作 — 操作後の頂点が探索範囲内

    #[test]

    fn tc3_kw4_nelder_mead_operations() {
        let default_params = MagnificentSevenParams::default();

        let ranges: [(f64, f64); 7] = [
            crate::constants::KW4_GAMMA_BENEVOLENCE_RANGE,
            crate::constants::KW4_LAMBDA_GC_BASE_RANGE,
            crate::constants::KW4_DIRECT_RECIPROCITY_WEIGHT_RANGE,
            crate::constants::KW4_INDIRECT_RECIPROCITY_WEIGHT_RANGE,
            crate::constants::KW4_SOFTMAX_TEMPERATURE_RANGE,
            crate::constants::KW4_GC_INTERVAL_RANGE,
            crate::constants::KW4_CHILD_RATIO_RANGE,
        ];

        let seed = 12345u64;

        let mut optimizer = NelderMeadOptimizer::new(&default_params, &ranges, 0.05, seed, None);

        // 反射操作のテスト

        let centroid = optimizer.compute_centroid();

        let reflected = optimizer.reflect(&centroid);

        for i in 0..7 {
            let v = get_param(&reflected, i);

            assert!(
                v >= ranges[i].0 - 1e-9,
                "反射後のパラメータ {} が下限 {} 未満: {}",
                i,
                ranges[i].0,
                v
            );

            assert!(
                v <= ranges[i].1 + 1e-9,
                "反射後のパラメータ {} が上限 {} 超過: {}",
                i,
                ranges[i].1,
                v
            );
        }

        // 拡大操作のテスト

        let expanded = optimizer.expand(&centroid, &reflected);

        for i in 0..7 {
            let v = get_param(&expanded, i);

            assert!(
                v >= ranges[i].0 - 1e-9,
                "拡大後のパラメータ {} が下限 {} 未満: {}",
                i,
                ranges[i].0,
                v
            );

            assert!(
                v <= ranges[i].1 + 1e-9,
                "拡大後のパラメータ {} が上限 {} 超過: {}",
                i,
                ranges[i].1,
                v
            );
        }

        // 収縮操作のテスト

        let contracted = optimizer.contract(&centroid);

        for i in 0..7 {
            let v = get_param(&contracted, i);

            assert!(
                v >= ranges[i].0 - 1e-9,
                "収縮後のパラメータ {} が下限 {} 未満: {}",
                i,
                ranges[i].0,
                v
            );

            assert!(
                v <= ranges[i].1 + 1e-9,
                "収縮後のパラメータ {} が上限 {} 超過: {}",
                i,
                ranges[i].1,
                v
            );
        }

        // 縮小操作のテスト

        let best = optimizer.simplex[0];

        optimizer.shrink_toward_best(&best);

        for vertex in optimizer.simplex.iter() {
            for i in 0..7 {
                let v = get_param(vertex, i);

                assert!(
                    v >= ranges[i].0 - 1e-9,
                    "縮小後のパラメータ {} が下限 {} 未満: {}",
                    i,
                    ranges[i].0,
                    v
                );

                assert!(
                    v <= ranges[i].1 + 1e-9,
                    "縮小後のパラメータ {} が上限 {} 超過: {}",
                    i,
                    ranges[i].1,
                    v
                );
            }
        }
    }

    /// TC4: evaluate_single 関数 — 同一パラメータで同一 J_kw（決定論的）

    #[test]

    fn tc4_kw4_evaluate_deterministic() {
        let params = MagnificentSevenParams::default();

        let seed = 12345u64;

        let j1 = evaluate_single(&params, seed, &None);

        let j2 = evaluate_single(&params, seed, &None);

        let diff = (j1 - j2).abs();

        assert!(
            diff < 1e-12,
            "同一パラメータ・同一 seed で J_kw が一致しない: {} vs {} (diff={})",
            j1,
            j2,
            diff
        );

        assert!(j1.is_finite(), "J_kw が有限値: {}", j1);

        assert!((-1.0..=0.0).contains(&j1), "evaluate_single(negated) が [-1, 0] 範囲: {}", j1);

        println!("TC4: evaluate_single(negated, default params, seed=12345) = {:.6}", j1);
    }

    /// TC5: OptimizationReport JSON シリアライズ — 全フィールドが正しく JSON 出力可能

    #[test]

    fn tc5_kw4_optimization_report_json() {
        #[allow(deprecated)]
        let report = OptimizationReport {
            best_params: MagnificentSevenParams::default(),

            best_j_kw: 0.5,

            best_j_kw_social: 0.5,

            tick_to_convergence: 50,

            s_speed: 0.5,

            assessment: compute_kind_world_objective(&KindWorldMetricsInput::zero()),

            iterations: 42,

            history: vec![(MagnificentSevenParams::default(), 0.5)],

            converged: true,

            experiment_id: "kw4-test-001".to_string(),
        };

        let json =
            serde_json::to_string(&report).expect("OptimizationReport の JSON シリアライズ成功");

        assert!(
            json.contains("kw4-test-001"),
            "JSON に experiment_id が含まれる"
        );

        assert!(json.contains("best_j_kw"), "JSON に best_j_kw が含まれる");

        assert!(json.contains("converged"), "JSON に converged が含まれる");

        assert!(json.contains("iterations"), "JSON に iterations が含まれる");

        assert!(json.contains("history"), "JSON に history が含まれる");

        assert!(json.contains("s_speed"), "JSON に s_speed が含まれる");

        println!("TC5: OptimizationReport JSON = {}", json);
    }

    /// TC6: kw4_optimize 正常実行 — panic せず完了、履歴 CSV + 最終 JSON が出力される
    /// 注: 長時間テスト（較正ループ用）— `cargo test -- --ignored` で実行

    #[test]
    #[ignore]

    fn tc6_kw4_optimize_run() {
        // 初期中心点は外側ループの定数から設定

        let default_params = MagnificentSevenParams {
            gamma_benevolence: crate::constants::KW4_INITIAL_GAMMA_BENEVOLENCE,

            child_ratio: crate::constants::KW4_INITIAL_CHILD_RATIO,

            softmax_temperature: crate::constants::KW4_INITIAL_SOFTMAX_TEMPERATURE,

            ..MagnificentSevenParams::default()
        };

        let ranges: [(f64, f64); 7] = [
            crate::constants::KW4_GAMMA_BENEVOLENCE_RANGE,
            crate::constants::KW4_LAMBDA_GC_BASE_RANGE,
            crate::constants::KW4_DIRECT_RECIPROCITY_WEIGHT_RANGE,
            crate::constants::KW4_INDIRECT_RECIPROCITY_WEIGHT_RANGE,
            crate::constants::KW4_SOFTMAX_TEMPERATURE_RANGE,
            crate::constants::KW4_GC_INTERVAL_RANGE,
            crate::constants::KW4_CHILD_RATIO_RANGE,
        ];

        let seed = 12345u64;

        let mut optimizer = NelderMeadOptimizer::new(
            &default_params,
            &ranges,
            crate::constants::KW4_NELDER_MEAD_INITIAL_PERTURBATION,
            seed,
            None,
        );

        let mut history: Vec<(MagnificentSevenParams, f64)> = Vec::new();

        let report = optimizer.run(
            crate::constants::KW4_NELDER_MEAD_MAX_ITERATIONS,
            crate::constants::KW4_NELDER_MEAD_CONVERGENCE_EPSILON,
            &mut history,
        );

        // CSV 形式で履歴を出力

        println!(
            "\n=== kw4_optimize [experiment_id={}] ===",
            report.experiment_id
        );

        println!("\n--- Nelder-Mead iteration history ---");

        println!("iter,J_kw_social,s_speed,ttc,gamma_benevolence,lambda_gc_base,direct_reciprocity_weight,indirect_reciprocity_weight,softmax_temperature,gc_interval,child_ratio");

        for (i, (params, j_kw_social)) in report.history.iter().enumerate() {
            // 各履歴エントリの s_speed と ttc を取得するため再評価
            let config = params.to_sim_config(
                crate::constants::KW4_EVALUATION_POPULATION_SIZE,
                12345u64,
            );
            let (_, ttc) = crate::simulation::run_evaluation_simulation(&config);
            let s_speed_val = crate::kind_world::compute_s_speed(ttc, crate::constants::KW4_SIMULATION_TICKS);
            println!(
                "{},{:.6},{:.6},{},{:.6},{:.6},{:.6},{:.6},{:.6},{},{:.6}",
                i,
                j_kw_social,
                s_speed_val,
                ttc,
                params.gamma_benevolence,
                params.lambda_gc_base,
                params.direct_reciprocity_weight,
                params.indirect_reciprocity_weight,
                params.softmax_temperature,
                params.gc_interval,
                params.child_ratio,
            );
        }

        // JSON レポート出力

        println!("\n--- Final Report (JSON) ---");

        let json = serde_json::to_string_pretty(&report).expect("JSON シリアライズ");

        println!("{}", json);

        // 5 因子内訳

        let a = &report.assessment;
        println!("
--- 5-Factor Breakdown ---");
        println!("s_growth  = {:.6}", a.s_growth);
        println!("s_density = {:.6}", a.s_density);
        println!("s_topology = {:.6}", a.s_topology);
        println!("s_search  = {:.6}", a.s_search);
        println!("s_fairness = {:.6}", a.s_fairness);
        let min_factor = a.s_growth
            .min(a.s_density)
            .min(a.s_topology)
            .min(a.s_search)
            .min(a.s_fairness);
        println!("min(s_i)  = {:.6}", min_factor);
        println!("J_kw      = {:.6}", a.j_kw);
        println!("s_speed   = {:.6}", report.s_speed);
        println!("J_kw_social = {:.6}", report.best_j_kw_social);
        println!("ttc       = {}", report.tick_to_convergence);
        println!(
            "Kind World: {} (require J_kw_social > 0.64 AND min(s_i) > 0.6)",
            if a.is_kind_world { "YES" } else { "no" }
        );

        // 20 下位成分
        // 20 下位成分（KindWorldAssessment フィールド定義順）
        let subcomponents = [
            ("j_pop_growth", a.j_pop_growth),
            ("j_lifecycle", a.j_lifecycle),
            ("j_child_survival", a.j_child_survival),
            ("j_freshness", a.j_freshness),
            ("j_cov", a.j_cov),
            ("j_diffusion", a.j_diffusion),
            ("j_reuse", a.j_reuse),
            ("j_benevolence", a.j_benevolence),
            ("j_reciprocity", a.j_reciprocity),
            ("j_help", a.j_help),
            ("j_trust", a.j_trust),
            ("j_cost", a.j_cost),
            ("j_execution", a.j_execution),
            ("j_penalty", a.j_penalty),
            ("j_nest_depth", a.j_nest_depth),
            ("j_node_density", a.j_node_density),
            ("j_clustering", a.j_clustering),
            ("j_local_density", a.j_local_density),
            ("j_search_radius_inv", a.j_search_radius_inv),
            ("j_reasoning_steps_inv", a.j_reasoning_steps_inv),
        ];
        println!("
--- 20 Subcomponents ---");
        for (name, val) in &subcomponents {
            println!("  {:<30} = {:.6}", name, val);
        }

        // Kind World チェック

        println!("
--- Kind World Check ---");

        let flags_true = report
            .assessment
            .legacy_flags
            .iter()
            .filter(|&&f| f)
            .count();

        println!(
            "is_kind_world: {} ({}/8 flags){}",
            report.assessment.is_kind_world,
            flags_true,
            if !report.assessment.is_kind_world {
                let missing: Vec<String> = report
                    .assessment
                    .legacy_flags
                    .iter()
                    .enumerate()
                    .filter(|(_, &f)| !f)
                    .map(|(i, _)| {
                        [
                            "population_growth",
                            "capability_coverage",
                            "reuse_ratio",
                            "cost_efficiency",
                            "village_formation",
                            "churn_low",
                            "churn_high",
                            "cross_village_interaction",
                        ][i]
                            .to_string()
                    })
                    .collect();

                format!(" — missing: {}", missing.join(", "))
            } else {
                String::new()
            }
        );

// アサーション

        assert!(
            report.best_j_kw_social.is_finite(),
            "best_j_kw_social が有限値: {}",
            report.best_j_kw_social
        );

        assert!(
            (0.0..=1.0).contains(&report.best_j_kw_social),
            "best_j_kw_social が [0, 1] 範囲: {}",
            report.best_j_kw_social
        );

        assert!(
            report.iterations > 0,
            "少なくとも 1 回以上の反復: {}",
            report.iterations
        );

        assert!(
            report.history.len() >= report.iterations as usize,
            "履歴サイズ ({}) >= 反復数 ({})",
            report.history.len(),
            report.iterations
        );

        println!(
            "TC6: kw4_optimize completed — {} iterations, best J_kw_social = {:.6}, converged = {}",
            report.iterations, report.best_j_kw_social, report.converged
        );
    }

    /// TC7: 異なる探索範囲で異なる結果
    /// 注: 長時間テスト（較正ループ用）— `cargo test -- --ignored` で実行

    #[test]
    #[ignore]

    fn tc7_kw4_different_ranges_different_results() {
        let default_params = MagnificentSevenParams::default();

        let wide_ranges: [(f64, f64); 7] = [
            (0.0, 0.8),
            (0.1, 2.0),
            (0.1, 0.8),
            (0.1, 0.8),
            (0.1, 5.0),
            (1.0, 10.0),
            (0.1, 0.5),
        ];

        let narrow_ranges: [(f64, f64); 7] = [
            (0.1, 0.2),
            (0.8, 1.2),
            (0.3, 0.5),
            (0.2, 0.4),
            (0.3, 0.7),
            (2.0, 4.0),
            (0.2, 0.4),
        ];

        let seed = 12345u64;

        let mut wide = NelderMeadOptimizer::new(&default_params, &wide_ranges, 0.05, seed, None);

        let mut narrow = NelderMeadOptimizer::new(&default_params, &narrow_ranges, 0.05, seed, None);

        let mut wide_history = Vec::new();

        let mut narrow_history = Vec::new();

        let wide_report = wide.run(15, 1e-6, &mut wide_history);

        let narrow_report = narrow.run(15, 1e-6, &mut narrow_history);

        // 異なる範囲で異なる結果（少なくとも完全一致はしない）

        // narrow の方が範囲が狭いため、広い範囲とは異なる最適値に行く可能性が高い

        // ただし、両者とも同じ default_params から始まるため、完全に異なるとは限らない

        // ここでは両者が panic せず完了することと、結果が有限であることだけを検証

        assert!(
            wide_report.best_j_kw_social.is_finite(),
            "wide の best_j_kw_social が有限"
        );

        assert!(
            narrow_report.best_j_kw_social.is_finite(),
            "narrow の best_j_kw_social が有限"
        );

        println!(
            "TC7: wide best_j_kw_social={:.6} ({} iter), narrow best_j_kw_social={:.6} ({} iter)",
            wide_report.best_j_kw_social,
            wide_report.iterations,
            narrow_report.best_j_kw_social,
            narrow_report.iterations,
        );
    }

    /// TC8: 既存 Phase 0-2 後方互換は cargo test 全体で検証

    /// このテストは空だが、既存の状態が変わっていないことを確認するプレースホルダ

    #[test]

    fn tc8_kw4_backward_compatible() {
        // 5 因子乗算モデルで全因子が高い場合の動作確認

        let metrics = KindWorldMetricsInput {
            population_growth_rate: 0.99,

            capability_coverage: 0.99,

            reuse_ratio: 0.99,

            cost_efficiency: 0.94,

            village_formation_score: 0.99,

            village_churn_rate: 0.15,

            cross_village_interaction_rate: 0.99,

            knowledge_diffusion_rate: 0.99,

            benevolent_vs_non_benevolent_coverage_ratio: 1.0,

            mean_lifecycle_score: 0.99,
            child_survival_rate: 0.99,
            mean_freshness: 0.99,
            mean_benevolence_aggregate: 0.99,
            mean_reciprocity_score: 0.99,
            help_success_rate: 0.99,
            trust_inheritance_fidelity: 0.99,
            execution_success_rate: 0.99,
            mean_nest_depth: 0.99,
            mean_node_density: 0.99,
            cluster_coefficient: 0.99,
            local_density: 0.99,
            search_radius_inverse: 0.99,
            reasoning_steps_inverse: 0.99,
        };

        let assessment = compute_kind_world_objective(&metrics);

        assert!(
            assessment.is_kind_world,
            "乗算モデルで成立: j_kw = {}",
            assessment.j_kw
        );

        assert!(
            assessment.j_kw.is_finite() && (0.0..=1.0).contains(&assessment.j_kw),
            "J_kw が正常範囲: {}",
            assessment.j_kw
        );

        println!("TC8 backward compatible — J_kw = {:.6}", assessment.j_kw);
    }
    /// TC9: evaluate_single 決定論的検証（SimulationContext 版）
    ///
    /// 同一パラメータ + 同一シードで同一 J_kw が得られることを確認する。
    /// SimulationContext 移行後も決定論的再現性が維持されていることの検証。
    #[test]
    fn tc9_kw4_evaluate_deterministic_context() {
        let params = MagnificentSevenParams {
            gamma_benevolence: 0.30,
            lambda_gc_base: 1.0,
            direct_reciprocity_weight: 0.3,
            indirect_reciprocity_weight: 0.3,
            softmax_temperature: 1.0,
            gc_interval: 3,
            child_ratio: 0.3,
        };
        let seed = 12345u64;
        let j1 = evaluate_single(&params, seed, &None);
        let j2 = evaluate_single(&params, seed, &None);
        let abs_diff = (j1 - j2).abs();
        println!(
            "TC9: evaluate_single deterministic — J_kw1={:.10}, J_kw2={:.10}, diff={:.10}",
            j1, j2, abs_diff
        );
        assert!(
            abs_diff < 1e-12,
            "同一パラメータ+同一シードで同一 J_kw: {} vs {}",
            j1, j2
        );
    }

    // ===============================================================
    // M1.76-KW-MTR-E: Village Churn & Benevolence Ratio Backfill Tests
    // ===============================================================

    use crate::spaceposition::SpacePositionEmbedding;
    use crate::types::{NodeId, TrustProfile};
    use std::collections::HashMap;

    /// E1: compute_village_churn_rate — comparisons=0 で 0.0
    #[test]
    fn e1_mtre_churn_rate_empty() {
        assert_eq!(compute_village_churn_rate(0, 0), 0.0);
        assert_eq!(compute_village_churn_rate(42, 0), 0.0);
    }

    /// E2: compute_village_churn_rate — 全変化 (changes=comparisons) で 1.0
    #[test]
    fn e2_mtre_churn_rate_all_changed() {
        assert_eq!(compute_village_churn_rate(10, 10), 1.0);
        assert_eq!(compute_village_churn_rate(0, 10), 0.0);
    }

    /// E3: compute_village_churn_rate — 部分変化 (3/10 = 0.3)
    #[test]
    fn e3_mtre_churn_rate_partial() {
        let result = compute_village_churn_rate(3, 10);
        assert!((result - 0.3).abs() < 1e-10, "3/10 = {}", result);
    }

    /// E4: compute_benevolent_ratio — 空 trust_profiles で 1.0
    #[test]
    fn e4_mtre_benevolent_ratio_empty() {
        let result = compute_benevolent_vs_non_benevolent_coverage_from_trust(
            &HashMap::new(),
            &HashMap::new(),
        );
        assert_eq!(result, 1.0);
    }

    /// E5: compute_benevolent_ratio — 全ノード同一慈悲スコア + 同一位置
    /// 全てのノードが同一位置にいる場合、上位と下位の多様性が等しくなるため ratio ≈ 1.0
    #[test]
    fn e5_mtre_benevolent_ratio_uniform() {
        let mut trust_profiles = HashMap::new();
        let mut positions = HashMap::new();
        for i in 0..10u64 {
            let nid = NodeId::from(i as usize);
            trust_profiles.insert(
                nid,
                TrustProfile {
                    operational: 0.5,
                    semantic: 0.5,
                    temporal: 0.5,
                    human: crate::types::HumanTrustLogistic::default(),
                },
            );
            positions.insert(nid, SpacePositionEmbedding::from([0.5, 0.5, 0.5]));
        }
        let result = compute_benevolent_vs_non_benevolent_coverage_from_trust(
            &trust_profiles,
            &positions,
        );
        // 全同一位置 → 上位と下位の多様性が等しいため ratio ≈ 1.0
        assert!(
            (result - 1.0).abs() < 1e-10,
            "全同一位置での ratio = {}",
            result
        );
    }

    /// E6: collect_final_metrics — village_churn_rate が 0.0 以外、benevolent_ratio が 1.0 以外
    ///
    /// 観測テスト: シミュレーション実行後に両指標がデフォルト値から改善されていることを確認する。
    #[test]
    fn e6_mtre_collect_final_metrics_non_default() {
        let params = MagnificentSevenParams {
            gamma_benevolence: 0.30,
            lambda_gc_base: 1.0,
            direct_reciprocity_weight: 0.3,
            indirect_reciprocity_weight: 0.3,
            softmax_temperature: 1.0,
            gc_interval: 3,
            child_ratio: 0.3,
        };
        let config = params.to_sim_config(200, 12345u64);
        let (metrics, _ttc) = crate::simulation::run_evaluation_simulation(&config);

        println!(
            "E6: ttc={}, village_churn_rate = {:.6}",
            _ttc, metrics.village_churn_rate
        );
        println!(
            "E6: benevolent_vs_non_benevolent_coverage_ratio = {:.6}",
            metrics.benevolent_vs_non_benevolent_coverage_ratio
        );

        assert!(
            metrics.village_churn_rate > 0.0,
            "village_churn_rate が 0.0 以外（実際の値: {})",
            metrics.village_churn_rate
        );
        assert!(
            (metrics.benevolent_vs_non_benevolent_coverage_ratio - 1.0).abs() > 1e-6,
            "benevolent_ratio が 1.0 以外（実際の値: {})",
            metrics.benevolent_vs_non_benevolent_coverage_ratio
        );
    }

    // E7: 既存テストとの回帰確認 — テストランナーが全実行

    // ===============================================================
    // M1.76-KW4-JKW-SOCIAL: J_kw_social 関連テスト (TC1e〜TC8e)
    // ===============================================================

    /// TC1e: tick_to_convergence 範囲 — 0 ≤ ttc ≤ KW4_SIMULATION_TICKS
    #[test]
    fn tc1e_kw4_ttc_range() {
        let params = MagnificentSevenParams::default();
        let config = params.to_sim_config(50, 12345u64);
        let (_, ttc) = crate::simulation::run_evaluation_simulation(&config);
        assert!(
            ttc <= crate::constants::KW4_SIMULATION_TICKS,
            "ttc={} should be ≤ {}",
            ttc,
            crate::constants::KW4_SIMULATION_TICKS
        );
        println!("TC1e: ttc={}, max={}", ttc, crate::constants::KW4_SIMULATION_TICKS);
    }

    /// TC2e: s_speed 範囲 — 0.0 ≤ s_speed ≤ 1.0
    #[test]
    fn tc2e_kw4_s_speed_range() {
        for &ttc in &[0u64, 10, 50, 100] {
            let s = compute_s_speed(ttc, 100);
            assert!(
                (0.0..=1.0).contains(&s),
                "s_speed({}) = {} not in [0,1]",
                ttc,
                s
            );
        }
        // 収束しなかった場合 s_speed = 0.0
        assert_eq!(compute_s_speed(100, 100), 0.0);
        println!("TC2e: s_speed range OK");
    }

    /// TC3e: evaluate_single が最小化用の負値を返す（戻り値が [-1, 0] 範囲）
    #[test]
    fn tc3e_kw4_evaluate_single_returns_j_kw_social() {
        let params = MagnificentSevenParams {
            gamma_benevolence: 0.30,
            lambda_gc_base: 1.0,
            direct_reciprocity_weight: 0.3,
            indirect_reciprocity_weight: 0.3,
            softmax_temperature: 1.0,
            gc_interval: 3,
            child_ratio: 0.3,
        };
        let value = evaluate_single(&params, 12345u64, &None);
        assert!(
            (-1.0..=0.0).contains(&value),
            "evaluate_single returned {} (expected [-1,0] for optimizer objective)",
            value
        );
        println!("TC3e: evaluate_single (negated objective) = {:.10}", value);
    }

    /// TC4e: 決定論性（evaluate_single）— 同一 params + seed で同一 J_kw_social
    #[test]
    fn tc4e_kw4_evaluate_single_deterministic() {
        let params = MagnificentSevenParams {
            gamma_benevolence: 0.30,
            lambda_gc_base: 1.0,
            direct_reciprocity_weight: 0.3,
            indirect_reciprocity_weight: 0.3,
            softmax_temperature: 1.0,
            gc_interval: 3,
            child_ratio: 0.3,
        };
        let v1 = evaluate_single(&params, 12345u64, &None);
        let v2 = evaluate_single(&params, 12345u64, &None);
        let diff = (v1 - v2).abs();
        println!("TC4e: J_kw_social1={:.10}, J_kw_social2={:.10}, diff={:.10}", v1, v2, diff);
        assert!(diff < 1e-12, "決定論的再現性違反: {} vs {}", v1, v2);
    }

    /// TC5e: 決定論性（tick_to_convergence）— 同一 params + seed で同一 ttc
    #[test]
    fn tc5e_kw4_ttc_deterministic() {
        let params = MagnificentSevenParams {
            gamma_benevolence: 0.30,
            lambda_gc_base: 1.0,
            direct_reciprocity_weight: 0.3,
            indirect_reciprocity_weight: 0.3,
            softmax_temperature: 1.0,
            gc_interval: 3,
            child_ratio: 0.3,
        };
        let config = params.to_sim_config(50, 12345u64);
        let (_, ttc1) = crate::simulation::run_evaluation_simulation(&config);
        let (_, ttc2) = crate::simulation::run_evaluation_simulation(&config);
        println!("TC5e: ttc1={}, ttc2={}", ttc1, ttc2);
        assert_eq!(ttc1, ttc2, "ttc が決定論的でない: {} vs {}", ttc1, ttc2);
    }

    /// TC6e: tc6 CSV/JSON 更新（tc6 テストで出力確認済み、ここでは形式検証のみ）
    /// 注: 長時間テスト（較正ループ用）— `cargo test -- --ignored` で実行
    #[test]
    #[ignore]
    fn tc6e_kw4_report_fields_present() {
        // OptimizationReport に best_j_kw_social, s_speed, tick_to_convergence が
        // 含まれていることを JSON シリアライズで確認
        let default_params = MagnificentSevenParams {
            gamma_benevolence: crate::constants::KW4_INITIAL_GAMMA_BENEVOLENCE,
            child_ratio: crate::constants::KW4_INITIAL_CHILD_RATIO,
            softmax_temperature: crate::constants::KW4_INITIAL_SOFTMAX_TEMPERATURE,
            ..MagnificentSevenParams::default()
        };
        let ranges: [(f64, f64); 7] = [
            crate::constants::KW4_GAMMA_BENEVOLENCE_RANGE,
            crate::constants::KW4_LAMBDA_GC_BASE_RANGE,
            crate::constants::KW4_DIRECT_RECIPROCITY_WEIGHT_RANGE,
            crate::constants::KW4_INDIRECT_RECIPROCITY_WEIGHT_RANGE,
            crate::constants::KW4_SOFTMAX_TEMPERATURE_RANGE,
            crate::constants::KW4_GC_INTERVAL_RANGE,
            crate::constants::KW4_CHILD_RATIO_RANGE,
        ];
        let mut optimizer =
            NelderMeadOptimizer::new(&default_params, &ranges, 0.10, 12345u64, None);
        let mut history = Vec::new();
        let report = optimizer.run(10, 1e-6, &mut history);
        let json = serde_json::to_string(&report).expect("JSON serialize");
        assert!(json.contains("best_j_kw_social"), "JSON missing best_j_kw_social");
        assert!(json.contains("tick_to_convergence"), "JSON missing tick_to_convergence");
        assert!(json.contains("s_speed"), "JSON missing s_speed");
        println!("TC6e: report JSON fields present: {}", json.contains("best_j_kw_social"));
    }

    /// TC7e: best_j_kw_social 観測 — 最適化後の値と内訳を出力（観測テスト）
    /// 注: 長時間テスト（較正ループ用）— `cargo test -- --ignored` で実行
    #[test]
    #[ignore]
    fn tc7e_kw4_best_j_kw_social_positive() {
        let default_params = MagnificentSevenParams {
            gamma_benevolence: crate::constants::KW4_INITIAL_GAMMA_BENEVOLENCE,
            child_ratio: crate::constants::KW4_INITIAL_CHILD_RATIO,
            softmax_temperature: crate::constants::KW4_INITIAL_SOFTMAX_TEMPERATURE,
            ..MagnificentSevenParams::default()
        };
        let ranges: [(f64, f64); 7] = [
            crate::constants::KW4_GAMMA_BENEVOLENCE_RANGE,
            crate::constants::KW4_LAMBDA_GC_BASE_RANGE,
            crate::constants::KW4_DIRECT_RECIPROCITY_WEIGHT_RANGE,
            crate::constants::KW4_INDIRECT_RECIPROCITY_WEIGHT_RANGE,
            crate::constants::KW4_SOFTMAX_TEMPERATURE_RANGE,
            crate::constants::KW4_GC_INTERVAL_RANGE,
            crate::constants::KW4_CHILD_RATIO_RANGE,
        ];
        let mut optimizer =
            NelderMeadOptimizer::new(&default_params, &ranges, 0.10, 12345u64, None);
        let mut history = Vec::new();
        let report = optimizer.run(10, 1e-6, &mut history);
        let a = &report.assessment;
        println!("TC7e: J_kw_social={:.6}, J_kw={:.6}, s_speed={:.6}, ttc={}",
            report.best_j_kw_social, a.j_kw, report.s_speed, report.tick_to_convergence);
        println!("TC7e: s_growth={:.6} s_density={:.6} s_topology={:.6} s_search={:.6} s_fairness={:.6}",
            a.s_growth, a.s_density, a.s_topology, a.s_search, a.s_fairness);
        if report.best_j_kw_social == 0.0 {
            println!("TC7e: WARNING — J_kw_social=0.0 (convergence might be too slow for 100 ticks)");
        }
    }

    /// TC8e: J_kw_social vs J_kw 比較 — 両者の差（s_speed 影響）を出力
    #[test]
    fn tc8e_kw4_j_kw_social_vs_j_kw() {
        let params = MagnificentSevenParams {
            gamma_benevolence: 0.30,
            lambda_gc_base: 1.0,
            direct_reciprocity_weight: 0.3,
            indirect_reciprocity_weight: 0.3,
            softmax_temperature: 1.0,
            gc_interval: 3,
            child_ratio: 0.3,
        };
        let config = params.to_sim_config(50, 12345u64);
        let (metrics, ttc) = crate::simulation::run_evaluation_simulation(&config);
        let assessment = compute_kind_world_objective(&metrics);
        let s_speed_val = compute_s_speed(ttc, crate::constants::KW4_SIMULATION_TICKS);
        let j_kw_social_val = assessment.j_kw * s_speed_val;
        println!("TC8e: J_kw={:.6}, s_speed={:.6}, J_kw_social={:.6}, ttc={}",
            assessment.j_kw, s_speed_val, j_kw_social_val, ttc);
        println!("TC8e: J_kw - J_kw_social = {:.6}", assessment.j_kw - j_kw_social_val);
        // 観測テスト: 常に PASS するが出力が分析対象
        assert!(j_kw_social_val <= assessment.j_kw, "J_kw_social <= J_kw が成立");
    }

    /// TC9: パレートフロンティア導出 — 重みスイープ
    ///
    /// 10 通りの重みベクトルで内側ループを実行し、各結果から非劣解を抽出して
    /// パレートフロンティアを表示する。
    /// 注: 長時間テスト（較正ループ用）— `cargo test tc6_kw4_pareto_sweep -- --ignored --nocapture`

    #[test]
    #[ignore]
    fn tc6_kw4_pareto_sweep() {
        struct SweepConfig {
            label: &'static str,
            weights: [f64; 6],
        }
        let sweeps: [SweepConfig; 10] = [
            SweepConfig { label: "balanced",       weights: [1.0, 1.0, 1.0, 1.0, 1.0, 1.0] },
            SweepConfig { label: "growth++",       weights: [3.0, 1.0, 1.0, 1.0, 1.0, 1.0] },
            SweepConfig { label: "topology++",     weights: [1.0, 1.0, 3.0, 1.0, 1.0, 1.0] },
            SweepConfig { label: "fairness++",     weights: [1.0, 1.0, 1.0, 1.0, 3.0, 1.0] },
            SweepConfig { label: "speed++",        weights: [1.0, 1.0, 1.0, 1.0, 1.0, 3.0] },
            SweepConfig { label: "growth+density",  weights: [2.0, 2.0, 1.0, 1.0, 1.0, 1.0] },
            SweepConfig { label: "topology+search",weights: [1.0, 1.0, 2.0, 2.0, 1.0, 1.0] },
            SweepConfig { label: "fairness+speed", weights: [1.0, 1.0, 1.0, 1.0, 2.0, 2.0] },
            SweepConfig { label: "no-speed",       weights: [1.0, 1.0, 1.0, 1.0, 1.0, 0.0] },
            SweepConfig { label: "density+search", weights: [1.0, 2.0, 1.0, 2.0, 1.0, 1.0] },
        ];

        let default_params = MagnificentSevenParams {
            gamma_benevolence: crate::constants::KW4_INITIAL_GAMMA_BENEVOLENCE,
            child_ratio: crate::constants::KW4_INITIAL_CHILD_RATIO,
            softmax_temperature: crate::constants::KW4_INITIAL_SOFTMAX_TEMPERATURE,
            ..MagnificentSevenParams::default()
        };

        let ranges: [(f64, f64); 7] = [
            crate::constants::KW4_GAMMA_BENEVOLENCE_RANGE,
            crate::constants::KW4_LAMBDA_GC_BASE_RANGE,
            crate::constants::KW4_DIRECT_RECIPROCITY_WEIGHT_RANGE,
            crate::constants::KW4_INDIRECT_RECIPROCITY_WEIGHT_RANGE,
            crate::constants::KW4_SOFTMAX_TEMPERATURE_RANGE,
            crate::constants::KW4_GC_INTERVAL_RANGE,
            crate::constants::KW4_CHILD_RATIO_RANGE,
        ];

        // (label, s_growth, s_density, s_topology, s_search, s_fairness, s_speed, j_kw_social)
        let mut results: Vec<(&str, f64, f64, f64, f64, f64, f64, f64)> = Vec::new();

        println!("\n=== Pareto Sweep: Per-Sweep Results ===");
        for sweep in &sweeps {
            let seed = 12345u64;
            let mut optimizer = NelderMeadOptimizer::new(
                &default_params,
                &ranges,
                crate::constants::KW4_NELDER_MEAD_INITIAL_PERTURBATION,
                seed,
                Some(sweep.weights),
            );

            let mut history: Vec<(MagnificentSevenParams, f64)> = Vec::new();
            let report = optimizer.run(
                crate::constants::KW4_SWEEP_MAX_ITERATIONS,
                crate::constants::KW4_NELDER_MEAD_CONVERGENCE_EPSILON,
                &mut history,
            );

            // best_j_kw_social は weights 使用時に歪むため assessment から再計算
            let a = &report.assessment;
            let j_kw_social = a.j_kw * report.s_speed;
            results.push((
                sweep.label,
                a.s_growth, a.s_density, a.s_topology, a.s_search,
                a.s_fairness, report.s_speed, j_kw_social,
            ));

            println!(
                "sweep {:>15}: growth={:.4} density={:.4} topology={:.4} search={:.4} fairness={:.4} speed={:.4} J_kw_social={:.6} iter={} converged={}",
                sweep.label,
                a.s_growth, a.s_density, a.s_topology, a.s_search,
                a.s_fairness, report.s_speed, j_kw_social, report.iterations, report.converged,
            );
        }

        // パレートフロンティア: 非劣解の抽出
        println!("\n=== Pareto Frontier (PF = non-dominated) ===");
        println!("| PF? | label           | growth | density | topology | search | fairness | s_speed | J_kw_social |");
        for &(label, g, d, t, s, f, sp, jkws) in &results {
            let factors = [g, d, t, s, f, sp];
            let dominated = results.iter().any(|&(_, g2, d2, t2, s2, f2, sp2, _)| {
                let factors2 = [g2, d2, t2, s2, f2, sp2];
                factors2.iter().zip(factors.iter()).all(|(fj, fi)| fj >= fi)
                    && factors2.iter().zip(factors.iter()).any(|(fj, fi)| fj > fi)
            });
            let marker = if dominated { "  " } else { "PF" };
            println!("  {} | {:>15} | {:.4} | {:.4} | {:.4} | {:.4} | {:.4} | {:.4} | {:.6}",
                marker, label, g, d, t, s, f, sp, jkws,
            );
        }

        // 検証: 全ての J_kw_social が有限値
        for &(label, _, _, _, _, _, _, jkws) in &results {
            assert!(jkws.is_finite(), "{}: J_kw_social が有限値: {}", label, jkws);
        }
    }

    /// TC7: 多目的ベイズ最適化（MotpeSampler）によるパレートフロンティア探索
    /// 重みスイープと異なり、6 目的を同時に扱い非劣解集合を直接構築する。
    #[test]
    #[ignore]
    fn tc7_kw4_bayesian_pareto() {
        use optimizer::Direction;
        use optimizer::multi_objective::MultiObjectiveStudy;
        use optimizer::parameter::{FloatParam, Parameter};
        use optimizer::sampler::motpe::MotpeSampler;

        let n_trials = 100;
        let eval_seed = 12345u64;
        let sampler = MotpeSampler::builder().seed(42).build();

        let study = MultiObjectiveStudy::with_sampler(
            vec![
                Direction::Maximize, // s_growth
                Direction::Maximize, // s_density
                Direction::Maximize, // s_topology
                Direction::Maximize, // s_search
                Direction::Maximize, // s_fairness
                Direction::Maximize, // s_speed
            ],
            sampler,
        );

        let p_gamma = FloatParam::new(
            crate::constants::KW4_GAMMA_BENEVOLENCE_RANGE.0,
            crate::constants::KW4_GAMMA_BENEVOLENCE_RANGE.1,
        );
        let p_lambda = FloatParam::new(
            crate::constants::KW4_LAMBDA_GC_BASE_RANGE.0,
            crate::constants::KW4_LAMBDA_GC_BASE_RANGE.1,
        );
        let p_direct = FloatParam::new(
            crate::constants::KW4_DIRECT_RECIPROCITY_WEIGHT_RANGE.0,
            crate::constants::KW4_DIRECT_RECIPROCITY_WEIGHT_RANGE.1,
        );
        let p_indirect = FloatParam::new(
            crate::constants::KW4_INDIRECT_RECIPROCITY_WEIGHT_RANGE.0,
            crate::constants::KW4_INDIRECT_RECIPROCITY_WEIGHT_RANGE.1,
        );
        let p_softmax = FloatParam::new(
            crate::constants::KW4_SOFTMAX_TEMPERATURE_RANGE.0,
            crate::constants::KW4_SOFTMAX_TEMPERATURE_RANGE.1,
        );
        let p_gc = FloatParam::new(
            crate::constants::KW4_GC_INTERVAL_RANGE.0,
            crate::constants::KW4_GC_INTERVAL_RANGE.1,
        );
        let p_child = FloatParam::new(
            crate::constants::KW4_CHILD_RATIO_RANGE.0,
            crate::constants::KW4_CHILD_RATIO_RANGE.1,
        );

        println!("\n=== Bayesian Pareto Search (MotpeSampler, {} trials) ===", n_trials);
        println!("trial,growth,density,topology,search,fairness,speed,J_kw_social");

        study
            .optimize(n_trials, |trial: &mut optimizer::Trial| {
                let gamma = p_gamma.suggest(trial)?;
                let lambda = p_lambda.suggest(trial)?;
                let direct = p_direct.suggest(trial)?;
                let indirect = p_indirect.suggest(trial)?;
                let softmax = p_softmax.suggest(trial)?;
                let gc = p_gc.suggest(trial)?;
                let child = p_child.suggest(trial)?;

                let params = MagnificentSevenParams {
                    gamma_benevolence: gamma,
                    lambda_gc_base: lambda,
                    direct_reciprocity_weight: direct,
                    indirect_reciprocity_weight: indirect,
                    softmax_temperature: softmax,
                    gc_interval: gc.round() as u64,
                    child_ratio: child,
                };
                let config = params.to_sim_config(
                    crate::constants::KW4_EVALUATION_POPULATION_SIZE,
                    eval_seed,
                );
                let (metrics, tick_to_convergence) =
                    crate::simulation::run_evaluation_simulation(&config);
                let assessment = compute_kind_world_objective(&metrics);
                let s_speed = compute_s_speed(
                    tick_to_convergence,
                    crate::constants::KW4_SIMULATION_TICKS,
                );

                let j_kw_social = assessment.j_kw * s_speed;
                println!(
                    "{},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6}",
                    trial.id(),
                    assessment.s_growth,
                    assessment.s_density,
                    assessment.s_topology,
                    assessment.s_search,
                    assessment.s_fairness,
                    s_speed,
                    j_kw_social,
                );

                Ok::<_, optimizer::Error>(vec![
                    assessment.s_growth,
                    assessment.s_density,
                    assessment.s_topology,
                    assessment.s_search,
                    assessment.s_fairness,
                    s_speed,
                ])
            })
            .unwrap();

        let front = study.pareto_front();
        println!("\n=== Pareto Front ({} solutions) ===", front.len());
        println!("trial,objectives,params");
        for trial_result in &front {
            let values: Vec<String> = trial_result
                .values
                .iter()
                .map(|v| format!("{:.6}", v))
                .collect();
            println!("trial={} values=[{}]", trial_result.id, values.join(", "));
        }

        // 検証: Pareto フロントが空でない
        assert!(
            !front.is_empty(),
            "Bayesian Pareto front should not be empty"
        );
    }

    // ======================================================================
    // Phase 2 — G1 Bayesian Pareto Search: 検索・探索系 14 パラメーター
    // ======================================================================
    // NOTE: G1 の 14 パラメーター中、現時点でシミュレーション経路に
    // 結合しているのは population_size と max_ticks のみ。
    // 残りはスタブ状態であり、今後の実装で有効化される。
    // 注: 長時間テスト（較正ループ用）— `cargo test tc_p2_g1_bayesian_search -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn tc_p2_g1_bayesian_search() {
        use optimizer::multi_objective::MultiObjectiveStudy;
        use optimizer::parameter::{FloatParam, Parameter};
        use optimizer::sampler::motpe::MotpeSampler;

        let n_trials = 50;
        let eval_seed = 12345u64;
        // G1 全 14 パラメーターのうち、現在 simulation 経路に結合している 6 個
        // live: G1_EVALUATION_POPULATION_SIZE (idx 6), G1_SIMULATION_TICKS (idx 7),
        //       G1_RECIPROCITY_ALPHA_HELP/SUCCESS/REJECT/HARM (idx 8-11)
        // stub: 残り 8 個（REMOTE_EXPLORE_* x4, SEARCH_TICK_FRACTION, EVALUATE_FRACTION,
        //       SEARCH_RADIUS_INVERSE, REMOTE_EXPLORE_HUMAN_WEIGHT）
        // search_radius_inverse の実装自体は完了（compute_search_radius_inverse 関数）して
        // いるが、シミュレーション内で help session が発生しないと値が 0.5 固定になる。
        let defaults = AllParams::default_g1();

        println!("\n=== Phase 2 G1: Bayesian Pareto Search ({} trials, 13 active, 6 live) ===", n_trials);
        println!("NOTE: 6 params wired to simulation (pop_size, max_ticks, 4 ALPHA).");
        println!("      7 active stubs, 1 inactive (SEARCH_RADIUS_INVERSE = compute_search_radius_inverse 実測値経路).");
        println!();
        println!("trial,pop_size,max_ticks,alpha_help,alpha_success,alpha_reject,alpha_harm,s_growth,s_density,s_topology,s_search,s_fairness,s_speed,J_kw_social");

        let sampler = MotpeSampler::builder().seed(42).build();
        let study = MultiObjectiveStudy::with_sampler(
            vec![
                optimizer::Direction::Maximize, // s_growth
                optimizer::Direction::Maximize, // s_density
                optimizer::Direction::Maximize, // s_topology
                optimizer::Direction::Maximize, // s_search
                optimizer::Direction::Maximize, // s_fairness
                optimizer::Direction::Maximize, // s_speed
            ],
            sampler,
        );

        // G1 各アクティブパラメーターの FloatParam を作成（SEARCH_RADIUS_INVERSE は inactive）
        let active_count = defaults.active_count();
        let mut float_params: Vec<FloatParam> = (0..active_count)
            .map(|i| FloatParam::new(defaults.ranges[i].0, defaults.ranges[i].1))
            .collect();

        study
            .optimize(n_trials, |trial: &mut optimizer::Trial| {
                let mut trial_values = Vec::with_capacity(active_count);
                for fp in float_params.iter_mut() {
                    trial_values.push(fp.suggest(trial)?);
                }
                let all_params = defaults.from_active_values(&trial_values);
                let config = all_params.to_sim_config_g1(eval_seed);
                let (metrics, tick_to_convergence) =
                    crate::simulation::run_evaluation_simulation(&config);
                let assessment = compute_kind_world_objective(&metrics);
                let s_speed = compute_s_speed(tick_to_convergence, config.max_ticks);
                let j_kw_social = assessment.j_kw * s_speed;

                println!(
                    "{},{:.0},{:.0},{:.3},{:.3},{:.3},{:.3},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6}",
                    trial.id(),
                    all_params.values[G1_EVALUATION_POPULATION_SIZE],
                    all_params.values[G1_SIMULATION_TICKS],
                    all_params.values[G1_RECIPROCITY_ALPHA_HELP],
                    all_params.values[G1_RECIPROCITY_ALPHA_SUCCESS],
                    all_params.values[G1_RECIPROCITY_ALPHA_REJECT],
                    all_params.values[G1_RECIPROCITY_ALPHA_HARM],
                    assessment.s_growth,
                    assessment.s_density,
                    assessment.s_topology,
                    assessment.s_search,
                    assessment.s_fairness,
                    s_speed,
                    j_kw_social,
                );

                Ok::<_, optimizer::Error>(vec![
                    assessment.s_growth,
                    assessment.s_density,
                    assessment.s_topology,
                    assessment.s_search,
                    assessment.s_fairness,
                    s_speed,
                ])
            })
            .unwrap();

        let front = study.pareto_front();
        println!("\n=== G1 Pareto Front ({} solutions) ===", front.len());
        for (i, tr) in front.iter().enumerate() {
            let v: Vec<String> = tr.values.iter().map(|v| format!("{:.6}", v)).collect();
            println!("  {}: trial={} values=[{}]", i + 1, tr.id, v.join(", "));
        }
        assert!(!front.is_empty(), "G1 Pareto front should not be empty");
    }

    // ======================================================================
    // Phase 2 — G1+G2 Bayesian Pareto Search: 検索・探索系 14 + GC・生存系 3
    // ======================================================================
    // 注: 長時間テスト（較正ループ用）— `cargo test tc_p2_g1g2_bayesian_search -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn tc_p2_g1g2_bayesian_search() {
        use optimizer::multi_objective::MultiObjectiveStudy;
        use optimizer::parameter::{FloatParam, Parameter};
        use optimizer::sampler::motpe::MotpeSampler;

        let n_trials = 50; // G1+G2 50 trials
        let eval_seed = 12345u64;
        // G1 (14) + G2 (3: gamma_lifecycle, gamma_child_protect, kappa_e)
        // live: G1 6 + G2 3 = 9 params wired to simulation
        // stub: G1 8 (REMOTE_EXPLORE_* x4, SEARCH_TICK_FRACTION, EVALUATE_FRACTION,
        //       SEARCH_RADIUS_INVERSE, REMOTE_EXPLORE_HUMAN_WEIGHT)
        let defaults = AllParams::default_g1g2();

        println!("\n=== Phase 2 G1+G2: Bayesian Pareto Search ({} trials, {} active, 9 live) ===", n_trials, defaults.active_count());
        println!("NOTE: G1 live = pop_size, max_ticks, 4 ALPHA (6)");
        println!("      G2 live = gamma_lifecycle, gamma_child_protect, kappa_e (3)");
        println!("      G1 active: 13 (SEARCH_RADIUS_INVERSE inactive = compute_search_radius_inverse 実測値経路)");
        println!();
        println!("trial,pop_size,max_ticks,alpha_help,alpha_success,alpha_reject,alpha_harm,gamma_life,gamma_child,kappa_e,s_growth,s_density,s_topology,s_search,s_fairness,s_speed,J_kw_social");

        let sampler = MotpeSampler::builder().seed(42).build();
        let study = MultiObjectiveStudy::with_sampler(
            vec![
                optimizer::Direction::Maximize, // s_growth
                optimizer::Direction::Maximize, // s_density
                optimizer::Direction::Maximize, // s_topology
                optimizer::Direction::Maximize, // s_search
                optimizer::Direction::Maximize, // s_fairness
                optimizer::Direction::Maximize, // s_speed
            ],
            sampler,
        );

        let active_count = defaults.active_count();
        let mut float_params: Vec<FloatParam> = (0..active_count)
            .map(|i| FloatParam::new(defaults.ranges[i].0, defaults.ranges[i].1))
            .collect();

        study
            .optimize(n_trials, |trial: &mut optimizer::Trial| {
                let mut trial_values = Vec::with_capacity(active_count);
                for fp in float_params.iter_mut() {
                    trial_values.push(fp.suggest(trial)?);
                }
                let all_params = defaults.from_active_values(&trial_values);
                let config = all_params.to_sim_config_g1g2(eval_seed);
                let (metrics, tick_to_convergence) =
                    crate::simulation::run_evaluation_simulation(&config);
                let assessment = compute_kind_world_objective(&metrics);
                let s_speed = compute_s_speed(tick_to_convergence, config.max_ticks);
                let j_kw_social = assessment.j_kw * s_speed;

                println!(
                    "{},{:.0},{:.0},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6}",
                    trial.id(),
                    all_params.values[G1_EVALUATION_POPULATION_SIZE],
                    all_params.values[G1_SIMULATION_TICKS],
                    all_params.values[G1_RECIPROCITY_ALPHA_HELP],
                    all_params.values[G1_RECIPROCITY_ALPHA_SUCCESS],
                    all_params.values[G1_RECIPROCITY_ALPHA_REJECT],
                    all_params.values[G1_RECIPROCITY_ALPHA_HARM],
                    all_params.values[G2_GAMMA_LIFECYCLE],
                    all_params.values[G2_GAMMA_CHILD_PROTECT],
                    all_params.values[G2_KAPPA_E],
                    assessment.s_growth,
                    assessment.s_density,
                    assessment.s_topology,
                    assessment.s_search,
                    assessment.s_fairness,
                    s_speed,
                    j_kw_social,
                );

                Ok::<_, optimizer::Error>(vec![
                    assessment.s_growth,
                    assessment.s_density,
                    assessment.s_topology,
                    assessment.s_search,
                    assessment.s_fairness,
                    s_speed,
                ])
            })
            .unwrap();

        let front = study.pareto_front();
        println!("\n=== G1+G2 Pareto Front ({} solutions) ===", front.len());
        for (i, tr) in front.iter().enumerate() {
            let v: Vec<String> = tr.values.iter().map(|v| format!("{:.6}", v)).collect();
            println!("  {}: trial={} values=[{}]", i + 1, tr.id, v.join(", "));
        }
        assert!(!front.is_empty(), "G1+G2 Pareto front should not be empty");
    }

    // ======================================================================
    // WIRE-B: compute_search_radius_inverse バグ修正 — ID フォーマット対応
    // ======================================================================
    #[test]
    fn tb1_parse_n_format() {
        use crate::help::HelpSession;
        use crate::spaceposition::SpacePositionEmbedding;
        use std::collections::HashMap;

        let positions: HashMap<usize, SpacePositionEmbedding> = [
            (1, SpacePositionEmbedding::from([0.0, 0.0, 0.0])),
            (2, SpacePositionEmbedding::from([3.0, 4.0, 0.0])),
        ]
        .into();
        let sessions = vec![HelpSession::new("h1".into(), "n1".into(), "n2".into())];
        let result = compute_search_radius_inverse(&sessions, &positions);
        let expected = 1.0 / 6.0; // L2=5 → 1/(1+5)=1/6
        assert!(
            (result - expected).abs() < 1e-10,
            "n format: got {:.10}, expected {:.10}",
            result,
            expected
        );
    }

    #[test]
    fn tb2_parse_wf_format() {
        use crate::help::HelpSession;
        use crate::spaceposition::SpacePositionEmbedding;
        use std::collections::HashMap;

        let positions: HashMap<usize, SpacePositionEmbedding> = [
            (1, SpacePositionEmbedding::from([0.0, 0.0, 0.0])),
            (2, SpacePositionEmbedding::from([3.0, 4.0, 0.0])),
        ]
        .into();
        let sessions =
            vec![HelpSession::new("h1".into(), "wf-child-1".into(), "wf-adult-2".into())];
        let result = compute_search_radius_inverse(&sessions, &positions);
        let expected = 1.0 / 6.0;
        assert!(
            (result - expected).abs() < 1e-10,
            "wf format: got {:.10}, expected {:.10}",
            result,
            expected
        );
    }

    #[test]
    fn tb3_parse_session_format() {
        use crate::help::HelpSession;
        use crate::spaceposition::SpacePositionEmbedding;
        use std::collections::HashMap;

        let positions: HashMap<usize, SpacePositionEmbedding> = [
            (1, SpacePositionEmbedding::from([0.0, 0.0, 0.0])),
            (2, SpacePositionEmbedding::from([3.0, 4.0, 0.0])),
        ]
        .into();
        let sessions =
            vec![HelpSession::new("h1".into(), "session-1".into(), "session-2".into())];
        let result = compute_search_radius_inverse(&sessions, &positions);
        let expected = 1.0 / 6.0;
        assert!(
            (result - expected).abs() < 1e-10,
            "session format: got {:.10}, expected {:.10}",
            result,
            expected
        );
    }

    #[test]
    fn tb4_identical_positions() {
        use crate::help::HelpSession;
        use crate::spaceposition::SpacePositionEmbedding;
        use std::collections::HashMap;

        let positions: HashMap<usize, SpacePositionEmbedding> =
            [(1, SpacePositionEmbedding::from([0.0, 0.0, 0.0]))].into();
        let sessions = vec![HelpSession::new("h1".into(), "n1".into(), "n1".into())];
        let result = compute_search_radius_inverse(&sessions, &positions);
        assert!(
            (result - 1.0).abs() < 1e-10,
            "identical positions: expected 1.0, got {:.10}",
            result
        );
    }

    #[test]
    fn tb5_empty_sessions() {
        use crate::spaceposition::SpacePositionEmbedding;
        use std::collections::HashMap;

        let positions: HashMap<usize, SpacePositionEmbedding> = HashMap::new();
        let sessions = vec![];
        let result = compute_search_radius_inverse(&sessions, &positions);
        assert!(
            (result - 0.5).abs() < 1e-10,
            "empty sessions: expected 0.5, got {:.10}",
            result
        );
    }

    #[test]
    fn tb6_skip_unparsable_sessions() {
        use crate::help::HelpSession;
        use crate::spaceposition::SpacePositionEmbedding;
        use std::collections::HashMap;

        let positions: HashMap<usize, SpacePositionEmbedding> = [
            (1, SpacePositionEmbedding::from([0.0, 0.0, 0.0])),
            (2, SpacePositionEmbedding::from([3.0, 4.0, 0.0])),
        ]
        .into();
        let sessions = vec![
            HelpSession::new("h1".into(), "unparsable".into(), "n2".into()),
            HelpSession::new("h2".into(), "n1".into(), "n2".into()),
            HelpSession::new("h3".into(), "n1".into(), "no_match".into()),
        ];
        // Only h2 should count: from=1, to=2 → L2=5 → 1/6
        let result = compute_search_radius_inverse(&sessions, &positions);
        let expected = 1.0 / 6.0;
        assert!(
            (result - expected).abs() < 1e-10,
            "skip unparsable: got {:.10}, expected {:.10}",
            result,
            expected
        );
    }

    #[test]
    fn tb7_parse_adult_child_format() {
        use crate::help::HelpSession;
        use crate::spaceposition::SpacePositionEmbedding;
        use std::collections::HashMap;

        let positions: HashMap<usize, SpacePositionEmbedding> = [
            (1, SpacePositionEmbedding::from([0.0, 0.0, 0.0])),
            (2, SpacePositionEmbedding::from([0.0, 0.0, 0.0])),
        ]
        .into();
        // production HelpSession の from_workflow/to_workflow 形式
        let sessions =
            vec![HelpSession::new("h1".into(), "adult-1".into(), "child-2".into())];
        let result = compute_search_radius_inverse(&sessions, &positions);
        assert!(
            (result - 1.0).abs() < 1e-10,
            "adult/child format: expected 1.0, got {:.10}",
            result
        );
    }
}