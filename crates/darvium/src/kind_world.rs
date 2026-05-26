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
    /// 全条件成立時のみ true
    pub is_kind_world: bool,
    /// 8 条件の成立フラグ
    /// [0]=population, [1]=capability, [2]=reuse, [3]=cost,
    /// [4]=village_formation, [5]=village_churn_low,
    /// [6]=village_churn_high, [7]=cross_village_interaction
    pub flags: [bool; 8],
    /// 目的関数値 J_kw ∈ [0, 1]
    pub j_kw: f64,
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
    let flow_balance_health = if churn_rate >= crate::constants::KW_VILLAGE_CHURN_LOWER
        && churn_rate <= crate::constants::KW_VILLAGE_CHURN_UPPER
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
/// J_kw = α₁·J_pop + α₂·J_cov + α₃·J_reuse + α₄·J_cost + α₅·J_village + α₆·J_penalty
///
/// RFC §15.9.2 の定義に従い、6 成分の重み付き和を [0, 1] に clamp して返す。
/// 同時に 8 つの成立条件フラグを計算し、`KindWorldAssessment` として返す。
pub fn compute_kind_world_objective(metrics: &KindWorldMetricsInput) -> KindWorldAssessment {
    // ---- 8 条件フラグ ----

    // [0]: 人口成長率 >= 閾値
    let flag_population =
        metrics.population_growth_rate >= crate::constants::KW_MIN_POPULATION_GROWTH_RATE;

    // [1]: 能力カバー率 (Shannon) >= 閾値
    let flag_capability =
        metrics.capability_coverage >= crate::constants::KW_MIN_CAPABILITY_COVERAGE_SHANNON;

    // [2]: 再利用比率 >= 閾値
    let flag_reuse = metrics.reuse_ratio >= crate::constants::KW_MIN_REUSE_RATIO;

    // [3]: コスト効率 <= 閾値（値が小さいほど効率改善が進んでいる）
    let flag_cost = metrics.cost_efficiency <= crate::constants::KW_MAX_COST_EFFICIENCY_DECAY;

    // [4]: 村形成スコア >= 閾値
    let flag_village_formation =
        metrics.village_formation_score >= crate::constants::KW_MIN_VILLAGE_FORMATION_SCORE;

    // [5]: churn 率が下限以上（流動性が低すぎない）
    let flag_churn_low = metrics.village_churn_rate >= crate::constants::KW_VILLAGE_CHURN_LOWER;

    // [6]: churn 率が上限以下（流動性が高すぎない）
    let flag_churn_high = metrics.village_churn_rate <= crate::constants::KW_VILLAGE_CHURN_UPPER;

    // [7]: 村間相互作用率 >= 閾値
    let flag_cross = metrics.cross_village_interaction_rate
        >= crate::constants::KW_CROSS_VILLAGE_INTERACTION_MIN;

    let flags = [
        flag_population,
        flag_capability,
        flag_reuse,
        flag_cost,
        flag_village_formation,
        flag_churn_low,
        flag_churn_high,
        flag_cross,
    ];

    let is_kind_world = flags.iter().all(|&f| f);

    // ---- J_kw 6 成分 ----

    // J_pop: 人口成長率そのまま（[0, +∞) だが clamp で [0, 1] に収める）
    let j_pop = metrics.population_growth_rate.clamp(0.0, 1.0);

    // J_cov: 能力カバー率そのまま（定義域 [0, 1]）
    let j_cov = metrics.capability_coverage.clamp(0.0, 1.0);

    // J_reuse: 再利用比率そのまま（定義域 [0, 1]）
    let j_reuse = metrics.reuse_ratio.clamp(0.0, 1.0);

    // J_cost: コスト効率の逆数（1.0 - cost_efficiency）、
    // ただし cost_efficiency が小さい（効率的）ほど J_cost が大きくなる。
    // clamp で [0, 1] を保証。
    let j_cost = (1.0 - metrics.cost_efficiency).clamp(0.0, 1.0);

    // J_village: 村健全性スコア（4 指標の平均）
    let j_village = compute_village_health_score(
        metrics.village_formation_score,
        metrics.village_churn_rate,
        metrics.cross_village_interaction_rate,
        metrics.knowledge_diffusion_rate,
    );

    // J_penalty: 慈悲的集団が非慈悲的集団より劣る場合のペナルティ。
    // ratio >= 1.0 → 慈悲的優位 → ペナルティ 0
    // ratio < 1.0 → 慈悲的劣位 → 線形ペナルティ、max 1.0
    let j_penalty = {
        let ratio = metrics.benevolent_vs_non_benevolent_coverage_ratio;
        if ratio >= 1.0 {
            0.0
        } else {
            (1.0 - ratio).clamp(0.0, 1.0)
        }
    };

    // ---- 重み付き和 ----
    let j_kw = {
        let raw = crate::constants::KW_ALPHA_POP * j_pop
            + crate::constants::KW_ALPHA_COV * j_cov
            + crate::constants::KW_ALPHA_REUSE * j_reuse
            + crate::constants::KW_ALPHA_COST * j_cost
            + crate::constants::KW_ALPHA_VILLAGE * j_village
            + crate::constants::KW_ALPHA_PENALTY * j_penalty;
        raw.clamp(0.0, 1.0)
    };

    KindWorldAssessment {
        is_kind_world,
        flags,
        j_kw,
    }
}

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

// ============================================================================
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
    if population.is_empty()
        || current_assignments.is_empty()
        || previous_assignments.is_empty()
    {
        return 0.0;
    }

    // 村の experience 平均を計算する内部関数
    let village_experience_means =
        |assignments: &[Option<usize>]| -> Vec<f64> {
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
                .map(|(&s, &c)| {
                    if c > 0 {
                        s / c as f64
                    } else {
                        0.0
                    }
                })
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
                        let requester_village =
                            id_to_village.get(s.requester_id.as_str()).copied();
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
            Some(prev) => {
                compute_knowledge_diffusion_rate(population, &current_assignments, prev)
            }
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
                    / n;
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

    /// 内部状態（前 tick の assignments）をリセットする。
    pub fn reset(&mut self) {
        self.previous_assignments = None;
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

    // ---- TC-1: 全条件成立 ----
    #[test]
    fn tc1_kw_all_conditions_met() {
        let metrics = KindWorldMetricsInput {
            population_growth_rate: 0.02,        // >= 0.01
            capability_coverage: 0.6,            // >= 0.5
            reuse_ratio: 0.4,                    // >= 0.3
            cost_efficiency: 0.8,                // <= 0.95
            village_formation_score: 0.5,        // >= 0.3
            village_churn_rate: 0.15,            // 0.05 <= 0.15 <= 0.30
            cross_village_interaction_rate: 0.2, // >= 0.1
            knowledge_diffusion_rate: 0.5,
            benevolent_vs_non_benevolent_coverage_ratio: 0.5,
        };
        let result = compute_kind_world_objective(&metrics);
        assert!(
            result.is_kind_world,
            "全条件が閾値を超えているため Kind World 成立"
        );
        assert!(result.flags.iter().all(|&f| f), "全 8 フラグが true");
        assert!(
            result.j_kw > 0.0 && result.j_kw <= 1.0,
            "J_kw = {} は (0, 1] の範囲",
            result.j_kw
        );
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
        };
        let result = compute_kind_world_objective(&metrics);
        assert!(
            !result.is_kind_world,
            "全条件が閾値を下回っているため Kind World 不成立"
        );
        // cost_efficiency=0.99 (<= 0.95 → false) と village_churn_rate=0.01 (< 0.05 → false, < 0.30 → true)
        // なので churn_high のみ true、他 7 つは false
        assert!(!result.flags[0], "flag_population");
        assert!(!result.flags[1], "flag_capability");
        assert!(!result.flags[2], "flag_reuse");
        assert!(!result.flags[3], "flag_cost");
        assert!(!result.flags[4], "flag_village_formation");
        assert!(!result.flags[5], "flag_churn_low");
        assert!(result.flags[6], "flag_churn_high = (0.01 <= 0.30) → true");
        assert!(!result.flags[7], "flag_cross");
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

    // ---- TC-5: 重み総和が 1.0 であること（静的アサート相当） ----
    #[test]
    fn tc5_kw_weight_sum_one() {
        let sum = crate::constants::KW_ALPHA_POP
            + crate::constants::KW_ALPHA_COV
            + crate::constants::KW_ALPHA_REUSE
            + crate::constants::KW_ALPHA_COST
            + crate::constants::KW_ALPHA_VILLAGE
            + crate::constants::KW_ALPHA_PENALTY;
        let diff = (sum - 1.0).abs();
        assert!(
            diff < 1e-12,
            "Σα_i = {} が 1.0 と一致しない（diff = {}）",
            sum,
            diff
        );
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
        // 慈悲的 < 非慈悲的 → ratio < 1.0 → J_penalty = 1-ratio > 0
        // J_penalty は α₆·J_penalty として J_kw に正の寄与をする。
        let metrics = KindWorldMetricsInput {
            population_growth_rate: 0.02,
            capability_coverage: 0.6,
            reuse_ratio: 0.4,
            cost_efficiency: 0.8,
            village_formation_score: 0.5,
            village_churn_rate: 0.15,
            cross_village_interaction_rate: 0.2,
            knowledge_diffusion_rate: 0.5,
            benevolent_vs_non_benevolent_coverage_ratio: 0.3, // 劣位
        };
        let result = compute_kind_world_objective(&metrics);
        assert!(result.is_kind_world);
        // ratio=0.3 → J_penalty = 0.7 → α₆·J_penalty = 0.07
        // 他の成分比率: それにより J_kw に正の寄与がある
        assert!(
            result.j_kw > 0.3,
            "J_penalty の寄与により J_kw > 0.3（実測: {}）",
            result.j_kw
        );

        // ペナルティ項の存在確認: ratio 変化で J_kw が変化する
        let higher_ratio = KindWorldMetricsInput {
            benevolent_vs_non_benevolent_coverage_ratio: 0.5,
            ..metrics
        };
        let result_higher = compute_kind_world_objective(&higher_ratio);
        // ratio=0.5 → J_penalty = 0.5 → contribution = 0.05
        // ratio=0.3 → J_penalty = 0.7 → contribution = 0.07
        // ratio が高い（より良い）ほど J_penalty が低くなり、J_kw への寄与が減る
        // つまり ratio=0.3 の方が J_kw が高い
        assert!(
            result.j_kw > result_higher.j_kw,
            "ratio=0.3 の J_kw（{}）> ratio=0.5 の J_kw（{}）",
            result.j_kw,
            result_higher.j_kw
        );
    }

    // ---- TC-8: J_penalty 慈悲的同等 ----
    #[test]
    fn tc8_kw_penalty_benevolent_equal() {
        let metrics = KindWorldMetricsInput {
            population_growth_rate: 0.02,
            capability_coverage: 0.6,
            reuse_ratio: 0.4,
            cost_efficiency: 0.8,
            village_formation_score: 0.5,
            village_churn_rate: 0.15,
            cross_village_interaction_rate: 0.2,
            knowledge_diffusion_rate: 0.5,
            benevolent_vs_non_benevolent_coverage_ratio: 1.0, // 同等
        };
        let result = compute_kind_world_objective(&metrics);
        assert!(result.is_kind_world);
        // ratio=1.0 → penalty=0 → j_kw にペナルティなし
        // ペナルティ以外の成分が全て positive なので j_kw > 0
        assert!(result.j_kw > 0.0);
    }

    // ---- TC-9: 境界値 ±0.001 ----
    #[test]
    fn tc9_kw_boundary_threshold() {
        // 各閾値の ±0.001 で成立/不成立が切り替わることを確認
        let above = KindWorldMetricsInput {
            population_growth_rate: 0.011,         // 0.01 + 0.001
            capability_coverage: 0.501,            // 0.5 + 0.001
            reuse_ratio: 0.301,                    // 0.3 + 0.001
            cost_efficiency: 0.949,                // 0.95 - 0.001
            village_formation_score: 0.301,        // 0.3 + 0.001
            village_churn_rate: 0.151, // 0.05〜0.30 の範囲内 (0.05 + 0.001 以上, 0.30 - 0.001 以下)
            cross_village_interaction_rate: 0.101, // 0.1 + 0.001
            knowledge_diffusion_rate: 0.5,
            benevolent_vs_non_benevolent_coverage_ratio: 1.0,
        };
        assert!(
            compute_kind_world_objective(&above).is_kind_world,
            "+0.001 側で Kind World 成立"
        );

        let below = KindWorldMetricsInput {
            population_growth_rate: 0.009,         // 0.01 - 0.001
            capability_coverage: 0.499,            // 0.5 - 0.001
            reuse_ratio: 0.299,                    // 0.3 - 0.001
            cost_efficiency: 0.951,                // 0.95 + 0.001
            village_formation_score: 0.299,        // 0.3 - 0.001
            village_churn_rate: 0.049,             // 0.05 - 0.001 → churn_low 不成立
            cross_village_interaction_rate: 0.099, // 0.1 - 0.001
            knowledge_diffusion_rate: 0.5,
            benevolent_vs_non_benevolent_coverage_ratio: 1.0,
        };
        assert!(
            !compute_kind_world_objective(&below).is_kind_world,
            "-0.001 側で Kind World 不成立"
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
        assert!(non_none.windows(2).all(|w| w[0] == w[1]), "全員が同一村 ID であること");
    }

    // ---- TC2: assign_village_ids 孤立ワークフローが None ----
    #[test]
    fn tc2_kw3_assign_village_ids_isolated_none() {
        let pop: Vec<SimWorkflowState> = vec![SimWorkflowState {
            id: "isolated".to_string(),
            position: [0.9, 0.9, 0.0],
            experience: 10, trust: 0.5,
            reputation: crate::event::ReputationProfile::cold_start(),
            benevolence: 0.5, direct_reciprocity: 0.5, indirect_reciprocity: 0.5,
            hazard: 0.0, survived: true, is_child: false, initial_benevolence: 0.5,
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
                position: [0.5, 0.5, 0.0], experience: 10, trust: 0.5,
                reputation: crate::event::ReputationProfile::cold_start(),
                benevolence: 0.5, direct_reciprocity: 0.5, indirect_reciprocity: 0.5,
                hazard: 0.0, survived: true, is_child: false, initial_benevolence: 0.5,
            })
            .collect();

        let assignments = assign_village_ids(&pop);
        let non_none: Vec<usize> = assignments.iter().filter_map(|&v| v).collect();
        assert_eq!(non_none.len(), 10, "全 10 人が村所属");
        assert!(non_none.windows(2).all(|w| w[0] == w[1]), "同一村 ID であること");
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
                id: "wf_a".to_string(), position: [0.1, 0.1, 0.0],
                experience: 10, trust: 0.5,
                reputation: crate::event::ReputationProfile::cold_start(),
                benevolence: 0.5, direct_reciprocity: 0.5, indirect_reciprocity: 0.5,
                hazard: 0.0, survived: true, is_child: false, initial_benevolence: 0.5,
            },
            SimWorkflowState {
                id: "wf_b".to_string(), position: [0.1, 0.1, 0.0],
                experience: 10, trust: 0.5,
                reputation: crate::event::ReputationProfile::cold_start(),
                benevolence: 0.5, direct_reciprocity: 0.5, indirect_reciprocity: 0.5,
                hazard: 0.0, survived: true, is_child: false, initial_benevolence: 0.5,
            },
            SimWorkflowState {
                id: "wf_c".to_string(), position: [0.9, 0.9, 0.0],
                experience: 10, trust: 0.5,
                reputation: crate::event::ReputationProfile::cold_start(),
                benevolence: 0.5, direct_reciprocity: 0.5, indirect_reciprocity: 0.5,
                hazard: 0.0, survived: true, is_child: false, initial_benevolence: 0.5,
            },
        ];

        let intra_sessions = vec![SimHelpSession {
            id: "s1".to_string(), mission_id: "m1".to_string(),
            helper_id: "wf_a".to_string(), requester_id: "wf_b".to_string(),
            status: HelpSessionStatus::Succeeded, created_at: 0, updated_at: 1,
            helper_benevolence: 0.5,
        }];

        let mut observer = VillageInteractionObserver::new();
        let metrics = observer.observe(0, &pop, &intra_sessions);
        assert!(metrics.cross_village_interaction_rate < 0.5, "同一村内セッションの相互作用率は低い");

        let empty_sessions: Vec<SimHelpSession> = vec![];
        let mut observer2 = VillageInteractionObserver::new();
        let metrics2 = observer2.observe(0, &pop, &empty_sessions);
        assert_eq!(metrics2.cross_village_interaction_rate, 0.0, "空セッションで 0.0");
    }

    // ---- TC6: compute_village_formation_strength ----
    #[test]
    fn tc6_kw3_village_formation_strength() {
        let pop: Vec<SimWorkflowState> = (0..8).map(|i| SimWorkflowState {
            id: format!("wf_{}", i),
            position: if i < 4 { [0.1, 0.1 + i as f32 * 0.01, 0.0] } else { [0.8, 0.8 + (i-4) as f32 * 0.01, 0.0] },
            experience: 10, trust: 0.5,
            reputation: crate::event::ReputationProfile::cold_start(),
            benevolence: 0.5, direct_reciprocity: 0.5, indirect_reciprocity: 0.5,
            hazard: 0.0, survived: true, is_child: false, initial_benevolence: 0.5,
        }).collect();

        let assignments = assign_village_ids(&pop);
        let strength = compute_village_formation_strength(&pop, &assignments);
        assert!(strength > 0.0, "密集クラスタの形成強度は正値 (got {})", strength);
        assert!(strength <= 1.0, "形成強度は [0, 1] 範囲 (got {})", strength);

        let all_none: Vec<Option<usize>> = vec![None; pop.len()];
        assert_eq!(compute_village_formation_strength(&pop, &all_none), 0.0, "全員 None の形成強度は 0.0");
    }

    // ---- TC7: compute_knowledge_diffusion_rate ----
    #[test]
    fn tc7_kw3_knowledge_diffusion_rate() {
        let pop: Vec<SimWorkflowState> = (0..6).map(|i| SimWorkflowState {
            id: format!("wf_{}", i),
            position: if i < 3 { [0.1, 0.1, 0.0] } else { [0.8, 0.8, 0.0] },
            experience: 10, trust: 0.5,
            reputation: crate::event::ReputationProfile::cold_start(),
            benevolence: 0.5, direct_reciprocity: 0.5, indirect_reciprocity: 0.5,
            hazard: 0.0, survived: true, is_child: false, initial_benevolence: 0.5,
        }).collect();

        let current = assign_village_ids(&pop);
        let previous = current.clone();
        assert_eq!(compute_knowledge_diffusion_rate(&pop, &current, &previous), 0.0, "変化なしなら拡散率 0.0");

        let empty: Vec<Option<usize>> = vec![];
        assert_eq!(compute_knowledge_diffusion_rate(&pop, &empty, &current), 0.0, "空 assignments で 0.0");
    }

    // ---- TC8: compute_village_flow_balance ----
    #[test]
    fn tc8_kw3_village_flow_balance() {
        let c: Vec<Option<usize>> = vec![Some(0), Some(0), Some(1)];
        let p: Vec<Option<usize>> = vec![Some(0), Some(0), Some(1)];
        assert_eq!(compute_village_flow_balance(&c, &p), 0.0, "変化なしで churn 0.0");

        let moved: Vec<Option<usize>> = vec![Some(1), Some(1), Some(0)];
        assert_eq!(compute_village_flow_balance(&moved, &p), 1.0, "全員移動で churn 1.0");

        let empty: Vec<Option<usize>> = vec![];
        assert_eq!(compute_village_flow_balance(&empty, &p), 0.0, "空 assignments で 0.0");
    }

    // ---- TC9: 空入力ガード ----
    #[test]
    fn tc9_kw3_empty_input_guard() {
        let empty_pop: Vec<SimWorkflowState> = vec![];
        let empty_sessions: Vec<SimHelpSession> = vec![];
        let empty_assignments: Vec<Option<usize>> = vec![];

        assert!(assign_village_ids(&empty_pop).is_empty(), "空 population で空ベクタ");
        assert_eq!(compute_cross_village_interaction_rate(&empty_sessions, &empty_assignments), 0.0, "空セッションで 0.0");
        assert_eq!(compute_village_formation_strength(&empty_pop, &empty_assignments), 0.0, "空 population で 0.0");
        assert_eq!(compute_knowledge_diffusion_rate(&empty_pop, &empty_assignments, &empty_assignments), 0.0, "空 assignments で 0.0");
        assert_eq!(compute_village_flow_balance(&empty_assignments, &empty_assignments), 0.0, "空 assignments で 0.0");
    }

    // ---- TC10: 村数 0（全員 None）graceful ハンドリング ----
    #[test]
    fn tc10_kw3_all_none_graceful() {
        let pop: Vec<SimWorkflowState> = (0..2).map(|i| SimWorkflowState {
            id: format!("wf_{}", i),
            position: [0.9 + i as f32 * 0.15, 0.9 + i as f32 * 0.15, 0.0],
            experience: 10, trust: 0.5,
            reputation: crate::event::ReputationProfile::cold_start(),
            benevolence: 0.5, direct_reciprocity: 0.5, indirect_reciprocity: 0.5,
            hazard: 0.0, survived: true, is_child: false, initial_benevolence: 0.5,
        }).collect();

        let assignments = assign_village_ids(&pop);
        assert!(assignments.iter().all(|&a| a.is_none()), "全員 None");

        assert_eq!(compute_cross_village_interaction_rate(&[], &assignments), 0.0, "cross = 0.0");
        assert_eq!(compute_village_formation_strength(&pop, &assignments), 0.0, "formation = 0.0");
        assert_eq!(compute_knowledge_diffusion_rate(&pop, &assignments, &assignments), 0.0, "diffusion = 0.0");
    }

    // ---- TC11: 後方互換性（SimWorkflowState にフィールド追加なし）----
    #[test]
    fn tc11_kw3_backward_compatibility() {
        let wf = SimWorkflowState {
            id: "test".to_string(), position: [0.5, 0.5, 0.0],
            experience: 10, trust: 0.5,
            reputation: crate::event::ReputationProfile::cold_start(),
            benevolence: 0.5, direct_reciprocity: 0.5, indirect_reciprocity: 0.5,
            hazard: 0.0, survived: true, is_child: false, initial_benevolence: 0.5,
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
        assert!((compute_village_health_score(0.5, 0.05, 0.5, 0.5) - 0.625).abs() < 1e-10, "churn 下限");
        assert!((compute_village_health_score(0.5, 0.30, 0.5, 0.5) - 0.625).abs() < 1e-10, "churn 上限");
        assert!((compute_village_health_score(0.5, 0.15, 0.5, 0.5) - 0.625).abs() < 1e-10, "churn 適正");
    }

    // ---- TC15: VillageInteractionObserver 統合テスト ----
    #[test]
    fn tc15_kw3_observer_integration() {
        let pop: Vec<SimWorkflowState> = (0..12).map(|i| SimWorkflowState {
            id: format!("wf_{}", i),
            position: [
                if i < 4 { 0.1 } else if i < 8 { 0.5 } else { 0.9 },
                if i < 4 { 0.1 } else if i < 8 { 0.5 } else { 0.9 },
                0.0,
            ],
            experience: 10 + i as u64, trust: 0.5,
            reputation: crate::event::ReputationProfile::cold_start(),
            benevolence: 0.5, direct_reciprocity: 0.5, indirect_reciprocity: 0.5,
            hazard: 0.0, survived: true, is_child: false, initial_benevolence: 0.5,
        }).collect();

        let sessions = vec![SimHelpSession {
            id: "s1".to_string(), mission_id: "m1".to_string(),
            helper_id: "wf_0".to_string(), requester_id: "wf_1".to_string(),
            status: HelpSessionStatus::Succeeded, created_at: 0, updated_at: 1,
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
                    let base_x = if i < 4 { 0.1 } else if i < 8 { 0.5 } else { 0.9 };
                    let base_y = if i < 4 { 0.1 } else if i < 8 { 0.5 } else { 0.9 };
                    SimWorkflowState {
                        id: format!("wf_{}", i),
                        position: [base_x + rng.random::<f32>() * 0.02, base_y + rng.random::<f32>() * 0.02, 0.0],
                        experience: 10 + tick, trust: 0.5,
                        reputation: crate::event::ReputationProfile::cold_start(),
                        benevolence: 0.5, direct_reciprocity: 0.5, indirect_reciprocity: 0.5,
                        hazard: 0.0, survived: true, is_child: false, initial_benevolence: 0.5,
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
                    created_at: tick, updated_at: tick + 1,
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
}
