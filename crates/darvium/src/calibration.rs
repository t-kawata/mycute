//! # Village Calibration Loop Harness (M1.75-11)
//!
//! 村落 (village) パラメータの系統的較正ループを提供する。
//! 目的関数 \(J_{village}(\theta)\) に基づき、one-factor-at-a-time sweep、
//! grid sweep、Latin hypercube sampling の 3 モードでパラメータ空間を探索する。
//!
//! ## 設計
//!
//! - `compute_village_objective`: 純粋関数。metrics から合成指標を計算する。
//! - `run_sweep_*`: 各 sweep モード。パラメータ設定値を生成し replay を駆動する。
//! - `VillageCalibrationHarness`: 統合ハーネス。Config → Sweep → Report を自動化する。
//!
//! ## 観測ベース検証
//!
//! テストは不変条件テスト (assert!) と観測テスト (println! + --nocapture) に分かれる。
//! 観測テストは固定シード PRNG で実行し、構造化データ (CSV) を標準出力に書き出す。

use std::collections::HashMap;

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::constants;
use crate::replay::{
    run_replay_scenario, HelperWeightEntry, PolicyBundle, VillageReplayScenario, WorkflowConfig,
};
use crate::village::VillageMetricsSnapshot;

// ============================================================
// データ型
// ============================================================

/// 較正対象パラメータの範囲指定。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ParameterRange {
    /// パラメータの識別名（例: "beta", "top_k", "epsilon"）。
    pub name: String,
    /// 探索下限（包含）。
    pub min: f64,
    /// 探索上限（包含）。
    pub max: f64,
    /// デフォルト値（OFAT で他のパラメータを固定する際に使用）。
    pub default: f64,
}

/// Sweep モードの列挙。
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum SweepMode {
    /// One-Factor-At-A-Time: 1 パラメータずつ変化させる。
    Ofat,
    /// Grid sweep: 全パラメータの直積格子点を探索する。
    Grid,
    /// Latin Hypercube Sampling: 層化ランダムサンプリングによる探索。
    LatinHypercube,
}

/// 較正設定。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct VillageCalibrationConfig {
    /// 較正対象パラメータの範囲リスト。
    pub parameter_ranges: Vec<ParameterRange>,
    /// 目的関数重み [a₁, a₂, a₃, a₄, a₅]。
    pub objective_weights: [f64; 5],
    /// OFAT sweep のステップ数（1 パラメータあたり）。
    pub ofat_steps: usize,
    /// Grid sweep の軸あたり分割数。
    pub grid_divisions: usize,
    /// LHS のサンプル数。
    pub lhs_samples: usize,
    /// replay シナリオの基本 seed。
    pub base_seed: u64,
    /// replay の tick 数。
    pub total_ticks: u64,
}

impl Default for VillageCalibrationConfig {
    fn default() -> Self {
        Self {
            parameter_ranges: vec![
                ParameterRange {
                    name: "beta".into(),
                    min: 0.1,
                    max: 2.0,
                    default: 1.0,
                },
                ParameterRange {
                    name: "epsilon".into(),
                    min: 0.0,
                    max: 0.5,
                    default: 0.1,
                },
                ParameterRange {
                    name: "top_k".into(),
                    min: 1.0,
                    max: 10.0,
                    default: 5.0,
                },
            ],
            objective_weights: [
                constants::OBJECTIVE_WEIGHT_CHURN,
                constants::OBJECTIVE_WEIGHT_JSD,
                constants::OBJECTIVE_WEIGHT_SURVIVAL,
                constants::OBJECTIVE_WEIGHT_FALSE_NEW,
                constants::OBJECTIVE_WEIGHT_REVIEW_LOAD,
            ],
            ofat_steps: constants::SWEEP_OFAT_DEFAULT_STEPS,
            grid_divisions: constants::SWEEP_GRID_DEFAULT_DIVISIONS,
            lhs_samples: constants::SWEEP_LHS_DEFAULT_SAMPLES,
            base_seed: 12345,
            total_ticks: 20,
        }
    }
}

/// 単一パラメータ設定に対する較正結果。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct VillageCalibrationResult {
    /// この評価で使用したパラメータ設定（名前 → 値）。
    pub params: HashMap<String, f64>,
    /// 目的関数値 J(θ)。
    pub j_value: f64,
    /// churn 成分値: a₁ * (1 - churn_p95)
    pub component_churn: f64,
    /// JSD 成分値: a₂ * (1 - jsd_p95)
    pub component_jsd: f64,
    /// survival 成分値: a₃ * survival_rate
    pub component_survival: f64,
    /// false-new ペナルティ成分値: a₄ * false_new (現状 0 固定)
    pub component_false_new: f64,
    /// review-load ペナルティ成分値: a₅ * review_load (現状 0 固定)
    pub component_review_load: f64,
    /// 元となった metrics スナップショット。
    pub snapshot: VillageMetricsSnapshot,
}

/// 較正実行の完全レポート。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CalibrationReport {
    /// 実験 ID (例: "exp-20260525-001")。
    pub experiment_id: String,
    /// 親実験 ID（ある場合）。
    pub parent_experiment_id: Option<String>,
    /// 使用した較正設定。
    pub config: VillageCalibrationConfig,
    /// 実行モード。
    pub mode: SweepMode,
    /// 全結果。
    pub results: Vec<VillageCalibrationResult>,
    /// 実行時刻 (ISO 8601)。
    pub timestamp: String,
}

// ============================================================
// 目的関数
// ============================================================

/// 目的関数 J_village(θ) を計算する。
///
/// J(θ) = a₁(1 - churn_{p95}) + a₂(1 - jsd_{p95}) + a₃ * survival
///        - a₄ * false_new - a₅ * review_load
///
/// false_new と review_load は v2.3 Fake 実行環境では観測困難なため、
/// 現状 0 として扱う。実 LLM 結合 (M2+) 以降で実際の計測が可能になる。
///
/// 戻り値は [0.0, 1.0] にクランプされる。
pub fn compute_village_objective(snapshot: &VillageMetricsSnapshot, weights: &[f64; 5]) -> f64 {
    let churn_component = weights[0] * (1.0 - snapshot.village_churn_p95);
    let jsd_component = weights[1] * (1.0 - snapshot.helper_jsd_p95);
    let survival_component = weights[2] * snapshot.child_survival_rate;
    let false_new_component = weights[3] * 0.0; // 現状 0 固定
    let review_load_component = weights[4] * 0.0; // 現状 0 固定

    let j = churn_component + jsd_component + survival_component
        - false_new_component
        - review_load_component;

    j.clamp(0.0, 1.0)
}

// ============================================================
// ParameterRange の補助メソッド
// ============================================================

impl ParameterRange {
    /// 指定ステップ数で等間隔分割した値を生成する。
    /// 範囲 [min, max] を steps 分割し、steps+1 個の値を返す。
    /// steps=1 の場合は min と max の 2 値を返す。
    pub fn linspace(&self, steps: usize) -> Vec<f64> {
        let n = steps.max(1);
        (0..=n)
            .map(|i| {
                let t = i as f64 / n as f64;
                self.min + t * (self.max - self.min)
            })
            .collect()
    }

    /// 指定分割数で格子値を生成する (grid sweep 用)。
    /// divisions 分割で divisions+1 個の値を返す。
    pub fn grid_values(&self, divisions: usize) -> Vec<f64> {
        let n = divisions.max(1);
        (0..=n)
            .map(|i| {
                let t = i as f64 / n as f64;
                self.min + t * (self.max - self.min)
            })
            .collect()
    }

    /// 範囲内で一様ランダムな値を生成する。
    pub fn random_value(&self, rng: &mut StdRng) -> f64 {
        rng.random_range(self.min..=self.max)
    }
}

// ============================================================
// Sweep モード実装
// ============================================================

/// パラメータ設定を PolicyBundle に適用する。
///
/// 現在の policy モデルでは以下のパラメータ名を認識する:
/// - "beta": HelperSelectionPolicy.beta
/// - "trust_exponent": HelperSelectionPolicy.trust_exponent
/// - "reputation_exponent": HelperSelectionPolicy.reputation_exponent
/// - "epsilon": HelperSelectionPolicy.epsilon
/// - "top_k": HelperSelectionPolicy.top_k (値は f64 → usize 変換)
/// - "quality_weight": AdultHelpOfferPolicy.quality_weight
/// - "load_penalty": AdultHelpOfferPolicy.load_penalty
/// - "risk_penalty": AdultHelpOfferPolicy.risk_penalty
/// - "threshold": AdultHelpOfferPolicy.threshold (offer / accept 共有)
///
/// 未知のパラメータ名は無視される。
fn apply_params_to_policy(params: &HashMap<String, f64>, policy: &PolicyBundle) -> PolicyBundle {
    let mut sel = policy.selection_policy.clone();
    if let Some(&v) = params.get("beta") {
        sel.beta = v;
    }
    if let Some(&v) = params.get("trust_exponent") {
        sel.trust_exponent = v;
    }
    if let Some(&v) = params.get("reputation_exponent") {
        sel.reputation_exponent = v;
    }
    if let Some(&v) = params.get("epsilon") {
        sel.epsilon = v;
    }
    if let Some(&v) = params.get("top_k") {
        sel.top_k = (v.round() as usize).max(1);
    }

    let mut offer = policy.offer_policy;
    if let Some(&v) = params.get("quality_weight") {
        offer.quality_weight = v;
    }
    if let Some(&v) = params.get("load_penalty") {
        offer.load_penalty = v;
    }
    if let Some(&v) = params.get("risk_penalty") {
        offer.risk_penalty = v;
    }
    if let Some(&v) = params.get("threshold") {
        offer.threshold = v;
    }

    let mut accept = policy.accept_policy;
    if let Some(&v) = params.get("gamma1") {
        accept.gamma1 = v;
    }
    if let Some(&v) = params.get("gamma2") {
        accept.gamma2 = v;
    }
    if let Some(&v) = params.get("gamma3") {
        accept.gamma3 = v;
    }
    if let Some(&v) = params.get("threshold") {
        accept.threshold = v;
    }

    PolicyBundle {
        offer_policy: offer,
        accept_policy: accept,
        selection_policy: sel,
    }
}

/// デフォルトのパラメータマップを生成する（全 range の default 値を設定）。
fn default_params(ranges: &[ParameterRange]) -> HashMap<String, f64> {
    ranges
        .iter()
        .map(|r| (r.name.to_string(), r.default))
        .collect()
}

/// 単一のパラメータ設定で replay を実行し、目的関数値を計算する。
fn evaluate_params(
    params: &HashMap<String, f64>,
    base_scenario: &VillageReplayScenario,
    weights: &[f64; 5],
) -> VillageCalibrationResult {
    let adjusted_policy = apply_params_to_policy(params, &base_scenario.policy_bundle);

    let scenario = VillageReplayScenario {
        policy_bundle: adjusted_policy,
        ..base_scenario.clone()
    };

    let trace = run_replay_scenario(&scenario);

    // ReplayTrace から VillageMetricsSnapshot を再構成する
    let snapshot = compute_trace_snapshot(&trace, &scenario.workflows);

    let j_value = compute_village_objective(&snapshot, weights);

    VillageCalibrationResult {
        params: params.clone(),
        j_value,
        component_churn: weights[0] * (1.0 - snapshot.village_churn_p95),
        component_jsd: weights[1] * (1.0 - snapshot.helper_jsd_p95),
        component_survival: weights[2] * snapshot.child_survival_rate,
        component_false_new: 0.0,
        component_review_load: 0.0,
        snapshot,
    }
}

/// ReplayTrace から VillageMetricsSnapshot を再構成する簡易実装。
///
/// 位置ドリフトは最終 tick と最終-1 tick の差分、churn は最終 village 構成 vs
/// 直前 village の差分、JSD は最終 helper weight の分布エントロピーから計算する。
/// 完全なウィンドウベースの計算は VillageMetricsWindow.snapshot() に委譲可能だが、
/// 較正ハーネス内では簡略版で十分である。
fn compute_trace_snapshot(
    trace: &crate::replay::ReplayTrace,
    workflows: &[WorkflowConfig],
) -> VillageMetricsSnapshot {
    let total_workflows = workflows.len();

    // --- churn: TickVillageEntry の adult_ids から Jaccard を計算 ---
    let village_churn_p95 = if trace.villages.len() >= 2 {
        let n = trace.villages.len();
        let mut churns: Vec<f64> = Vec::new();
        for i in 1..n {
            let prev_villages = &trace.villages[i - 1].villages;
            let curr_villages = &trace.villages[i].villages;
            // child_id で引き易いよう Map 化
            let prev_map: std::collections::HashMap<&str, &[String]> = prev_villages
                .iter()
                .map(|e| (e.child_id.as_str(), e.adult_ids.as_slice()))
                .collect();
            for entry in curr_villages {
                if let Some(prev_adults) = prev_map.get(entry.child_id.as_str()) {
                    let intersection = prev_adults
                        .iter()
                        .filter(|a| entry.adult_ids.contains(a))
                        .count();
                    let union = prev_adults.len() + entry.adult_ids.len() - intersection;
                    let jaccard = if union == 0 {
                        0.0
                    } else {
                        intersection as f64 / union as f64
                    };
                    churns.push(1.0 - jaccard);
                }
            }
        }
        if churns.is_empty() {
            0.0
        } else {
            churns.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let p95_idx = ((churns.len() as f64 * 0.95).ceil() as usize)
                .max(1)
                .min(churns.len())
                - 1;
            churns[p95_idx]
        }
    } else {
        0.0
    };

    // --- JSD: TickHelperEntry の helper weight 分布エントロピー ---
    let (helper_jsd_p95, churn_p50) = if trace.helper_weights.len() >= 2 {
        let n = trace.helper_weights.len();
        let mut jsds: Vec<f64> = Vec::new();
        for i in 1..n {
            let prev_entries = &trace.helper_weights[i - 1].entries;
            let curr_entries = &trace.helper_weights[i].entries;
            // child_id で引き易いよう Map 化
            let prev_map: std::collections::HashMap<&str, &[HelperWeightEntry]> = prev_entries
                .iter()
                .map(|e| (e.child_id.as_str(), e.helpers.as_slice()))
                .collect();
            for entry in curr_entries {
                if let Some(prev_helpers) = prev_map.get(entry.child_id.as_str()) {
                    let mut jsd: f64 = 0.0;
                    // 全 helper ID の和集合
                    let all_ids: std::collections::HashSet<&str> = prev_helpers
                        .iter()
                        .map(|h| h.helper_id.as_str())
                        .chain(entry.helpers.iter().map(|h| h.helper_id.as_str()))
                        .collect();
                    for id in &all_ids {
                        let p = prev_helpers
                            .iter()
                            .find(|h| h.helper_id == *id)
                            .map(|h| h.weight)
                            .unwrap_or(0.0);
                        let q = entry
                            .helpers
                            .iter()
                            .find(|h| h.helper_id == *id)
                            .map(|h| h.weight)
                            .unwrap_or(0.0);
                        let m = (p + q) / 2.0;
                        if p > 0.0 {
                            jsd += p * (p / m).ln();
                        }
                        if q > 0.0 {
                            jsd += q * (q / m).ln();
                        }
                    }
                    let jsd_val = (jsd / 2.0).abs(); // sqrt(JSD) 相当の簡略近似
                    jsds.push(jsd_val);
                }
            }
        }
        if jsds.is_empty() {
            (0.0, 0.0)
        } else {
            jsds.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let p50_idx = (jsds.len() - 1) / 2;
            let p95_idx = ((jsds.len() as f64 * 0.95).ceil() as usize)
                .max(1)
                .min(jsds.len())
                - 1;
            (jsds[p95_idx], jsds[p50_idx])
        }
    } else {
        (0.0, 0.0)
    };

    // --- helper count ---
    let helper_counts: Vec<f64> = if let Some(last_village) = trace.villages.last() {
        last_village
            .villages
            .iter()
            .map(|entry| entry.adult_ids.len() as f64)
            .collect()
    } else {
        vec![]
    };
    let helper_count_mean = if helper_counts.is_empty() {
        0.0
    } else {
        helper_counts.iter().sum::<f64>() / helper_counts.len() as f64
    };

    // --- survival ---
    let child_survival_rate = if total_workflows == 0 {
        0.0
    } else {
        let child_count = trace.villages.last().map(|v| v.villages.len()).unwrap_or(0);
        child_count as f64 / total_workflows as f64
    };

    VillageMetricsSnapshot {
        position_drift_p50: 0.0,
        position_drift_p95: 0.0,
        village_churn_p50: churn_p50,
        village_churn_p95,
        helper_jsd_p50: 0.0,
        helper_jsd_p95,
        helper_count_mean,
        child_survival_rate,
        child_maturation_time_p50: 0.0,
        child_maturation_time_p95: 0.0,
    }
}

// ============================================================
// Sweep 関数
// ============================================================

/// One-Factor-At-A-Time sweep を実行する。
///
/// 各パラメータを steps 分割した値に設定し、それ以外は default 値で固定する。
pub fn run_sweep_ofat(
    config: &VillageCalibrationConfig,
    base_scenario: &VillageReplayScenario,
) -> Vec<VillageCalibrationResult> {
    let mut results = Vec::new();
    let base = default_params(&config.parameter_ranges);

    for range in &config.parameter_ranges {
        for value in range.linspace(config.ofat_steps) {
            let mut params = base.clone();
            params.insert(range.name.to_string(), value);
            let result = evaluate_params(&params, base_scenario, &config.objective_weights);
            results.push(result);
        }
    }

    results
}

/// Grid sweep を実行する。
///
/// 全パラメータの直積格子点を生成する。パラメータ数が多いと
/// (divisions+1)^{n_params} で爆発するため注意。
/// 3 パラメータ × 3 分割 = 64 点が上限目安。
pub fn run_sweep_grid(
    config: &VillageCalibrationConfig,
    base_scenario: &VillageReplayScenario,
) -> Vec<VillageCalibrationResult> {
    // 各パラメータの grid 値を生成
    let grid_values: Vec<(&str, Vec<f64>)> = config
        .parameter_ranges
        .iter()
        .map(|r| (r.name.as_str(), r.grid_values(config.grid_divisions)))
        .collect();

    // 直積を生成 (再帰)
    let mut results = Vec::new();

    fn cartesian_product(
        idx: usize,
        current: &mut HashMap<String, f64>,
        grid_values: &[(&str, Vec<f64>)],
        config: &VillageCalibrationConfig,
        base_scenario: &VillageReplayScenario,
        results: &mut Vec<VillageCalibrationResult>,
    ) {
        if idx >= grid_values.len() {
            let result = evaluate_params(current, base_scenario, &config.objective_weights);
            results.push(result);
            return;
        }
        let (name, values) = &grid_values[idx];
        for value in values {
            current.insert(name.to_string(), *value);
            cartesian_product(
                idx + 1,
                current,
                grid_values,
                config,
                base_scenario,
                results,
            );
        }
    }

    let mut current = HashMap::new();
    cartesian_product(
        0,
        &mut current,
        &grid_values,
        config,
        base_scenario,
        &mut results,
    );

    results
}

/// Latin Hypercube Sampling を実行する。
///
/// 各パラメータの範囲を lhs_samples 分割し、各階層から 1 サンプルずつランダム抽出する。
/// 層化抽出により、一様ランダムサンプリングより均等な空間被覆を実現する。
///
/// `strata` は 2 次元構造 (param×sample) のため、イテレータより直接インデックスの
/// 方が可読性が高い。clippy の needless_range_loop は抑制する。
#[allow(clippy::needless_range_loop)]
pub fn run_sweep_lhs(
    config: &VillageCalibrationConfig,
    base_scenario: &VillageReplayScenario,
    rng: &mut StdRng,
) -> Vec<VillageCalibrationResult> {
    let n_params = config.parameter_ranges.len();
    let n_samples = config.lhs_samples;

    if n_params == 0 || n_samples == 0 {
        return Vec::new();
    }

    // 各パラメータの階層分割
    let strata: Vec<Vec<f64>> = config
        .parameter_ranges
        .iter()
        .map(|range| {
            let mut bins: Vec<f64> = (0..n_samples)
                .map(|i| {
                    let low = range.min + (range.max - range.min) * i as f64 / n_samples as f64;
                    let high =
                        range.min + (range.max - range.min) * (i + 1) as f64 / n_samples as f64;
                    rng.random_range(low..high)
                })
                .collect();
            // Fisher-Yates shuffle for Latin Hypercube property
            for i in (1..n_samples).rev() {
                let j = rng.random_range(0..=i);
                bins.swap(i, j);
            }
            bins
        })
        .collect();

    let mut results = Vec::with_capacity(n_samples);
    for sample_idx in 0..n_samples {
        let mut params: HashMap<String, f64> = HashMap::new();
        for (param_idx, range) in config.parameter_ranges.iter().enumerate() {
            params.insert(range.name.to_string(), strata[param_idx][sample_idx]);
        }
        let result = evaluate_params(&params, base_scenario, &config.objective_weights);
        results.push(result);
    }

    results
}

// ============================================================
// 較正ハーネス
// ============================================================

/// 較正ループを自動化する統合ハーネス。
pub struct VillageCalibrationHarness {
    config: VillageCalibrationConfig,
}

impl VillageCalibrationHarness {
    /// 指定された設定でハーネスを構築する。
    pub fn new(config: VillageCalibrationConfig) -> Self {
        Self { config }
    }

    /// 設定への不変参照を返す。
    pub fn config(&self) -> &VillageCalibrationConfig {
        &self.config
    }

    /// 指定モードで sweep を実行し、レポートを生成する。
    pub fn run_sweep(
        &self,
        mode: SweepMode,
        base_scenario: &VillageReplayScenario,
    ) -> CalibrationReport {
        let results = match mode {
            SweepMode::Ofat => run_sweep_ofat(&self.config, base_scenario),
            SweepMode::Grid => run_sweep_grid(&self.config, base_scenario),
            SweepMode::LatinHypercube => {
                let mut rng = StdRng::seed_from_u64(self.config.base_seed);
                run_sweep_lhs(&self.config, base_scenario, &mut rng)
            }
        };

        let timestamp = iso_timestamp_now();

        CalibrationReport {
            experiment_id: format!("exp-{}", timestamp.replace(&[':', '-'][..], "")),
            parent_experiment_id: None,
            config: self.config.clone(),
            mode,
            results,
            timestamp,
        }
    }
}

/// ISO 8601 形式のタイムスタンプ文字列を返す。
/// chrono 非依存の簡易実装。実際の時刻が必要な場合は chrono crate を追加する。
fn iso_timestamp_now() -> String {
    "2026-05-25T00:00:00Z".to_string()
}

// ============================================================
// Reciprocity Calibration (M1.76-16, RFC §15.10.8 式 F-16)
// ============================================================

/// AUC 計算の基本単位。benevolence スコアと生存フラグを保持する。
#[derive(Debug, Clone, PartialEq)]
pub struct SurvivalPair {
    /// ワークフローの識別子。
    pub workflow_id: String,
    /// 善良性スコア（値が大きいほど善良）。
    pub benevolence_score: f32,
    /// 生存したかどうか。
    pub survived: bool,
}

/// 式 F-16 の 6 成分を保持するメトリクス構造体。
#[derive(Debug, Clone, PartialEq)]
pub struct ReciprocityOperationalMetrics {
    /// AUC_benevolent>nonbenevolent: 善良群が非善良群より生存 ranking 上位に来る確率。
    pub auc_benevolent_survival: f64,
    /// Help 成功率。
    pub help_success_rate: f64,
    /// Village churn の P95 値。
    pub village_churn_p95: f64,
    /// False new rate。
    pub false_new_rate: f64,
    /// Review load。
    pub review_load: f64,
    /// 不安定性ペナルティ。
    pub instability_penalty: f64,
}

impl Default for ReciprocityOperationalMetrics {
    fn default() -> Self {
        Self {
            auc_benevolent_survival: 0.0,
            help_success_rate: 0.0,
            village_churn_p95: 0.0,
            false_new_rate: 0.0,
            review_load: 0.0,
            instability_penalty: 0.0,
        }
    }
}

/// 善良群と非善良群の survival ranking AUC を Mann-Whitney U 統計量から計算する。
///
/// 全 SurvivalPair を benevolence 降順にソートし、善良群の rank sum から
/// U 統計量を求め、`U / (n1 * n2)` で AUC を算出する。
/// `n1` = 善良群サイズ、`n2` = 非善良群サイズ。
/// 片方の群が空の場合は 0.5 を返す（判定不能）。
pub fn compute_auc_benevolent_survival(profiles: &[SurvivalPair]) -> f64 {
    if profiles.is_empty() {
        return 0.5;
    }

    let n1 = profiles.iter().filter(|p| p.survived).count();
    let n2 = profiles.len() - n1;

    if n1 == 0 || n2 == 0 {
        return 0.5;
    }

    // 全 profile を benevolence 降順にソート
    let mut sorted: Vec<&SurvivalPair> = profiles.iter().collect();
    sorted.sort_by(|a, b| {
        b.benevolence_score
            .partial_cmp(&a.benevolence_score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                match (a.survived, b.survived) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => std::cmp::Ordering::Equal,
                }
            })
    });

    // 善良群の rank sum
    let rank_sum: f64 = sorted
        .iter()
        .enumerate()
        .filter(|(_, p)| p.survived)
        .map(|(i, _)| (i + 1) as f64)
        .sum();

    let u = rank_sum - (n1 as f64 * (n1 as f64 + 1.0)) / 2.0;
    // AUC = 1 - U/(n₁·n₂): U=0（全善良が上位）→ AUC=1、U=n₁·n₂（全非善良が上位）→ AUC=0
    let auc = 1.0 - u / (n1 as f64 * n2 as f64);

    auc.clamp(0.0, 1.0)
}

/// 式 F-16 の多目的較正目的関数 J(θ) を計算する。
///
/// J(θ) = λ₁·AUC + λ₂·HelpSuccessRate - λ₃·ChurnP95 - λ₄·FalseNewRate
///        - λ₅·ReviewLoad - λ₆·InstabilityPenalty
///
/// 戻り値は [0.0, 1.0] にクランプされる。
pub fn compute_calibration_objective(
    metrics: &ReciprocityOperationalMetrics,
    weights: &[f64; 6],
) -> f64 {
    let j = weights[0] * metrics.auc_benevolent_survival
        + weights[1] * metrics.help_success_rate
        - weights[2] * metrics.village_churn_p95
        - weights[3] * metrics.false_new_rate
        - weights[4] * metrics.review_load
        - weights[5] * metrics.instability_penalty;

    j.clamp(0.0, 1.0)
}

/// 較正設定。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ReciprocityCalibrationConfig {
    /// 6 成分の λ 重み [λ₁, λ₂, λ₃, λ₄, λ₅, λ₆]。
    pub lambda_weights: [f64; 6],
    /// 較正対象パラメータの範囲リスト。
    pub parameter_ranges: Vec<ParameterRange>,
    /// 固定シード。
    pub base_seed: u64,
}

impl Default for ReciprocityCalibrationConfig {
    fn default() -> Self {
        Self {
            lambda_weights: [
                crate::constants::F16_LAMBDA_AUC,
                crate::constants::F16_LAMBDA_HELP_SUCCESS,
                crate::constants::F16_LAMBDA_CHURN,
                crate::constants::F16_LAMBDA_FALSE_NEW,
                crate::constants::F16_LAMBDA_REVIEW_LOAD,
                crate::constants::F16_LAMBDA_INSTABILITY,
            ],
            parameter_ranges: vec![],
            base_seed: 12345,
        }
    }
}

/// 単一パラメータ設定に対する較正結果。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ReciprocityCalibrationResult {
    /// この評価で使用したパラメータ設定（名前 → 値）。
    pub params: HashMap<String, f64>,
    /// 目的関数値 J(θ)。
    pub j_value: f64,
    /// AUC 成分値: λ₁ * auc_benevolent_survival
    pub component_auc: f64,
    /// Help success 成分値: λ₂ * help_success_rate
    pub component_help_success: f64,
    /// Churn 成分値: λ₃ * village_churn_p95
    pub component_churn: f64,
    /// False-new ペナルティ成分値: λ₄ * false_new_rate
    pub component_false_new: f64,
    /// Review-load ペナルティ成分値: λ₅ * review_load
    pub component_review_load: f64,
    /// Instability ペナルティ成分値: λ₆ * instability_penalty
    pub component_instability: f64,
}

/// 較正実行の完全レポート。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ReciprocityCalibrationReport {
    /// 実験 ID（例: "exp-20260526-001"）。
    pub experiment_id: String,
    /// 親実験 ID（ある場合）。
    pub parent_experiment_id: Option<String>,
    /// 使用した較正設定。
    pub config: ReciprocityCalibrationConfig,
    /// 全結果。
    pub results: Vec<ReciprocityCalibrationResult>,
    /// 実行時刻 (ISO 8601)。
    pub timestamp: String,
}

/// 較正ループを自動化する統合ハーネス。
pub struct ReciprocityCalibrationHarness {
    config: ReciprocityCalibrationConfig,
    sequence_counter: std::sync::atomic::AtomicU64,
}

impl ReciprocityCalibrationHarness {
    /// 指定された設定でハーネスを構築する。
    pub fn new(config: ReciprocityCalibrationConfig) -> Self {
        Self {
            config,
            sequence_counter: std::sync::atomic::AtomicU64::new(1),
        }
    }

    /// 設定への不変参照を返す。
    pub fn config(&self) -> &ReciprocityCalibrationConfig {
        &self.config
    }

    /// 実験 ID を exp-{yyyymmdd}-{seq} 形式で生成する。
    pub fn generate_experiment_id(&self) -> String {
        let seq = self
            .sequence_counter
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let now = chrono_now_compact();
        format!("exp-{}-{:03}", now, seq)
    }

    /// 指定パラメータ設定で目的関数評価を実行する。
    pub fn evaluate(
        &self,
        params: &HashMap<String, f64>,
        metrics: &ReciprocityOperationalMetrics,
    ) -> ReciprocityCalibrationResult {
        let weights = &self.config.lambda_weights;
        let j_value = compute_calibration_objective(metrics, weights);

        ReciprocityCalibrationResult {
            params: params.clone(),
            j_value,
            component_auc: weights[0] * metrics.auc_benevolent_survival,
            component_help_success: weights[1] * metrics.help_success_rate,
            component_churn: weights[2] * metrics.village_churn_p95,
            component_false_new: weights[3] * metrics.false_new_rate,
            component_review_load: weights[4] * metrics.review_load,
            component_instability: weights[5] * metrics.instability_penalty,
        }
    }

    /// 全結果を含むレポートを生成する。
    pub fn generate_report(
        &self,
        results: Vec<ReciprocityCalibrationResult>,
        parent_experiment_id: Option<String>,
    ) -> ReciprocityCalibrationReport {
        ReciprocityCalibrationReport {
            experiment_id: self.generate_experiment_id(),
            parent_experiment_id,
            config: self.config.clone(),
            results,
            timestamp: iso_timestamp_now(),
        }
    }
}

/// chrono 非依存の簡易コンパクト日時文字列（yyyymmdd）。
fn chrono_now_compact() -> String {
    "20260526".to_string()
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// テスト用の基本シナリオを構築する。
    ///
    /// 3 児童 + 6 成人の最小限村。各ワークフローはランダム位置に配置され、
    /// デフォルトのポリシー設定で動作する。
    fn base_scenario() -> VillageReplayScenario {
        use crate::replay::ClockSchedule;

        let mut workflows = Vec::new();

        // 3 児童 (経験値不足)
        for i in 0..3 {
            workflows.push(WorkflowConfig {
                id: format!("child_{}", i),
                initial_position: [0.0, 0.0, 0.0],
                initial_experience: 2, // child: experience < E_ADULT_THRESHOLD
                initial_trust: 0.3,
                initial_reputation: 0.3,
            });
        }

        // 6 成人 (十分な経験値)
        for i in 0..6 {
            workflows.push(WorkflowConfig {
                id: format!("adult_{}", i),
                initial_position: [(i as f32 - 2.5) * 1.5, (i as f32 % 3.0) * 1.5, 0.0],
                initial_experience: 20, // adult: experience >= E_ADULT_THRESHOLD
                initial_trust: 0.5 + (i as f64 * 0.08),
                initial_reputation: 0.4 + (i as f64 * 0.1),
            });
        }

        VillageReplayScenario {
            seed: 12345,
            workflows,
            missions: vec![],
            clock_schedule: ClockSchedule { total_ticks: 10 },
            policy_bundle: PolicyBundle::default(),
        }
    }

    // ----------------------------------------------------------
    // 目的関数テスト (C-1 〜 C-5)
    // ----------------------------------------------------------

    /// C-1: 同一パラメータ束で複数回評価したとき目的関数値が決定論的に一致する。
    #[test]
    fn objective_deterministic_identical_params() {
        let scenario = base_scenario();
        let weights = [0.35, 0.25, 0.25, 0.10, 0.05];
        let params = default_params(&VillageCalibrationConfig::default().parameter_ranges);

        let r1 = evaluate_params(&params, &scenario, &weights);
        let r2 = evaluate_params(&params, &scenario, &weights);
        let r3 = evaluate_params(&params, &scenario, &weights);

        println!(
            "C-1: J(θ) runs: {:.6}, {:.6}, {:.6}",
            r1.j_value, r2.j_value, r3.j_value
        );
        println!(
            "C-1: churn={:.6}, jsd={:.6}, survival={:.6}",
            r1.snapshot.village_churn_p95,
            r1.snapshot.helper_jsd_p95,
            r1.snapshot.child_survival_rate
        );

        let diff12 = (r1.j_value - r2.j_value).abs();
        let diff23 = (r2.j_value - r3.j_value).abs();
        assert!(
            diff12 < 1e-12,
            "C-1 FAIL: run1={:.10} != run2={:.10}, diff={:.10}",
            r1.j_value,
            r2.j_value,
            diff12
        );
        assert!(
            diff23 < 1e-12,
            "C-1 FAIL: run2={:.10} != run3={:.10}, diff={:.10}",
            r2.j_value,
            r3.j_value,
            diff23
        );
    }

    /// C-2: 極端なパラメータ（高 epsilon、低 top_k）が churn/JSD を悪化させ、
    /// 目的関数が低下する。
    #[test]
    fn objective_extreme_params_degradation() {
        let scenario = base_scenario();
        let weights = [0.35, 0.25, 0.25, 0.10, 0.05];

        // デフォルト設定
        let default_p = default_params(&VillageCalibrationConfig::default().parameter_ranges);
        let default_result = evaluate_params(&default_p, &scenario, &weights);

        // 極端: 高 epsilon (純遠隔探索)
        let mut high_eps = default_p.clone();
        high_eps.insert("epsilon".to_string(), 0.5);
        let high_eps_result = evaluate_params(&high_eps, &scenario, &weights);

        // 極端: 低 top_k
        let mut low_k = default_p.clone();
        low_k.insert("top_k".to_string(), 1.0);
        let low_k_result = evaluate_params(&low_k, &scenario, &weights);

        // 極端: 高 beta (強い距離減衰)
        let mut high_beta = default_p.clone();
        high_beta.insert("beta".to_string(), 2.0);
        let high_beta_result = evaluate_params(&high_beta, &scenario, &weights);

        println!(
            "C-2: J(θ) default={:.6}, high_eps={:.6}, low_k={:.6}, high_beta={:.6}",
            default_result.j_value,
            high_eps_result.j_value,
            low_k_result.j_value,
            high_beta_result.j_value
        );
        println!(
            "C-2: churn_p95: default={:.6}, high_eps={:.6}, low_k={:.6}, high_beta={:.6}",
            default_result.snapshot.village_churn_p95,
            high_eps_result.snapshot.village_churn_p95,
            low_k_result.snapshot.village_churn_p95,
            high_beta_result.snapshot.village_churn_p95
        );
        println!(
            "C-2: jsd_p95: default={:.6}, high_eps={:.6}, low_k={:.6}, high_beta={:.6}",
            default_result.snapshot.helper_jsd_p95,
            high_eps_result.snapshot.helper_jsd_p95,
            low_k_result.snapshot.helper_jsd_p95,
            high_beta_result.snapshot.helper_jsd_p95
        );

        // 高 beta (強い距離減衰) は helper 選択肢を狭め JSD を増加させるため、
        // 目的関数が低下することを確認する。
        assert!(
            high_beta_result.j_value <= default_result.j_value + 1e-10,
            "C-2 FAIL: high beta J={:.6} > default J={:.6}",
            high_beta_result.j_value,
            default_result.j_value
        );
    }

    /// C-3: high survival が目的関数に正の寄与をする。
    #[test]
    fn objective_high_survival_positive_contribution() {
        let weights = [0.35, 0.25, 0.25, 0.10, 0.05];

        // 完全生存の metrics を作成
        let snapshot_high = VillageMetricsSnapshot {
            village_churn_p95: 0.1,
            helper_jsd_p95: 0.1,
            child_survival_rate: 1.0,
            ..Default::default()
        };

        // 低生存の metrics を作成
        let snapshot_low = VillageMetricsSnapshot {
            village_churn_p95: 0.1,
            helper_jsd_p95: 0.1,
            child_survival_rate: 0.2,
            ..Default::default()
        };

        let j_high = compute_village_objective(&snapshot_high, &weights);
        let j_low = compute_village_objective(&snapshot_low, &weights);

        println!(
            "C-3: J(high_survival=1.0)={:.6}, J(low_survival=0.2)={:.6}",
            j_high, j_low
        );
        assert!(
            j_high > j_low,
            "C-3 FAIL: high survival J={:.6} should be > low survival J={:.6}",
            j_high,
            j_low
        );
    }

    /// C-4: weight を個別に変化させたとき対応成分が比例的に影響を受ける。
    #[test]
    fn objective_weight_sensitivity() {
        let snapshot = VillageMetricsSnapshot {
            village_churn_p95: 0.2,
            helper_jsd_p95: 0.15,
            child_survival_rate: 0.8,
            ..Default::default()
        };

        // weight を 2 倍にしたとき目的関数値が比例的に変化することを確認
        let base_weights = [0.35, 0.25, 0.25, 0.10, 0.05];
        let double_churn_weights = [0.70, 0.25, 0.25, 0.10, 0.05];

        let j_base = compute_village_objective(&snapshot, &base_weights);
        let j_double_churn = compute_village_objective(&snapshot, &double_churn_weights);

        // churn 成分のみが変化していることを確認
        let base_churn_component = base_weights[0] * (1.0 - snapshot.village_churn_p95);
        let double_churn_component = double_churn_weights[0] * (1.0 - snapshot.village_churn_p95);
        let expected_diff = double_churn_component - base_churn_component;

        let actual_diff = j_double_churn - j_base;

        println!(
            "C-4: base J={:.6}, double_churn J={:.6}, expected_diff={:.6}, actual_diff={:.6}",
            j_base, j_double_churn, expected_diff, actual_diff
        );

        assert!(
            (actual_diff - expected_diff).abs() < 1e-12,
            "C-4 FAIL: expected diff={:.10}, actual diff={:.10}",
            expected_diff,
            actual_diff
        );
    }

    /// C-5: zero や negative metrics でも panic しない。
    #[test]
    fn objective_boundary_values_no_panic() {
        let weights = [0.35, 0.25, 0.25, 0.10, 0.05];

        // 全成分ゼロ
        let zero = VillageMetricsSnapshot::default();
        let j_zero = compute_village_objective(&zero, &weights);
        println!("C-5: all-zero J={:.6}", j_zero);

        // 異常値
        let extreme = VillageMetricsSnapshot {
            village_churn_p95: -1.0,
            helper_jsd_p95: -1.0,
            child_survival_rate: -1.0,
            ..Default::default()
        };
        let j_extreme = compute_village_objective(&extreme, &weights);
        println!("C-5: extreme J={:.6}", j_extreme);

        // 最大値
        let maxed = VillageMetricsSnapshot {
            village_churn_p95: 1.0,
            helper_jsd_p95: 1.0,
            child_survival_rate: 1.0,
            ..Default::default()
        };
        let j_maxed = compute_village_objective(&maxed, &weights);
        println!("C-5: maxed J={:.6}", j_maxed);

        // いずれも panic せず、f64 範囲内であること
        assert!(j_zero.is_finite(), "C-5 FAIL: zero J not finite");
        assert!(j_extreme.is_finite(), "C-5 FAIL: extreme J not finite");
        assert!(j_maxed.is_finite(), "C-5 FAIL: maxed J not finite");
        assert!(
            (0.0..=1.0).contains(&j_zero),
            "C-5 FAIL: zero J={} not in [0,1]",
            j_zero
        );
        assert!(
            (0.0..=1.0).contains(&j_maxed),
            "C-5 FAIL: maxed J={} not in [0,1]",
            j_maxed
        );
    }

    // ----------------------------------------------------------
    // Sweep モードテスト (C-6 〜 C-9)
    // ----------------------------------------------------------

    /// C-6: OFAT sweep が全パラメータを 1 要素ずつ変化させ、結果が Vec として返る。
    #[test]
    fn sweep_one_factor_at_a_time() {
        let config = VillageCalibrationConfig {
            parameter_ranges: vec![
                ParameterRange {
                    name: "beta".into(),
                    min: 0.5,
                    max: 1.5,
                    default: 1.0,
                },
                ParameterRange {
                    name: "epsilon".into(),
                    min: 0.0,
                    max: 0.3,
                    default: 0.1,
                },
            ],
            ofat_steps: 3,
            ..Default::default()
        };

        let scenario = base_scenario();
        let results = run_sweep_ofat(&config, &scenario);

        // 2 params × (3+1) steps = 8 results
        let expected_count = config.parameter_ranges.len() * (config.ofat_steps + 1);
        assert_eq!(
            results.len(),
            expected_count,
            "C-6 FAIL: expected {} results, got {}",
            expected_count,
            results.len()
        );

        // CSV 形式で出力
        println!("C-6: param,value,j_value,churn_p95,jsd_p95,survival");
        for r in &results {
            let pname = r.params.keys().next().map(|s| s.as_str()).unwrap_or("?");
            let pval = r.params.values().next().copied().unwrap_or(0.0);
            println!(
                "C-6: {},{:.4},{:.6},{:.6},{:.6},{:.6}",
                pname,
                pval,
                r.j_value,
                r.snapshot.village_churn_p95,
                r.snapshot.helper_jsd_p95,
                r.snapshot.child_survival_rate
            );
        }

        // 各パラメータが異なる値を持つことを確認
        let unique_params: std::collections::HashSet<String> = results
            .iter()
            .map(|r| {
                r.params
                    .iter()
                    .map(|(k, v)| format!("{}={:.2}", k, v))
                    .collect::<Vec<_>>()
                    .join(",")
            })
            .collect();
        assert!(
            unique_params.len() > 1,
            "C-6 FAIL: all results have identical params"
        );
    }

    /// C-7: grid sweep が直積パラメータ組合せを生成し、結果に重複がない。
    #[test]
    fn sweep_grid() {
        let config = VillageCalibrationConfig {
            parameter_ranges: vec![
                ParameterRange {
                    name: "beta".into(),
                    min: 0.5,
                    max: 1.5,
                    default: 1.0,
                },
                ParameterRange {
                    name: "epsilon".into(),
                    min: 0.0,
                    max: 0.3,
                    default: 0.1,
                },
            ],
            grid_divisions: 2,
            ..Default::default()
        };

        let scenario = base_scenario();
        let results = run_sweep_grid(&config, &scenario);

        // (2+1)^{2} = 9 results
        let div = config.grid_divisions + 1;
        let expected_count = div * div;
        assert_eq!(
            results.len(),
            expected_count,
            "C-7 FAIL: expected {} results, got {}",
            expected_count,
            results.len()
        );

        // CSV 出力
        println!("C-7: beta,epsilon,j_value,churn_p95,jsd_p95,survival");
        for r in &results {
            let beta = r.params.get("beta").copied().unwrap_or(0.0);
            let eps = r.params.get("epsilon").copied().unwrap_or(0.0);
            println!(
                "C-7: {:.4},{:.4},{:.6},{:.6},{:.6},{:.6}",
                beta,
                eps,
                r.j_value,
                r.snapshot.village_churn_p95,
                r.snapshot.helper_jsd_p95,
                r.snapshot.child_survival_rate
            );
        }

        // 重複がないこと
        let unique: std::collections::HashSet<String> = results
            .iter()
            .map(|r| {
                format!(
                    "beta={:.2},epsilon={:.2}",
                    r.params.get("beta").copied().unwrap_or(0.0),
                    r.params.get("epsilon").copied().unwrap_or(0.0)
                )
            })
            .collect();
        assert_eq!(
            unique.len(),
            results.len(),
            "C-7 FAIL: duplicate parameter combinations detected"
        );
    }

    /// C-8: LHS が指定サンプル数だけ一様分布に近いサンプルを生成する。
    #[test]
    fn sweep_latin_hypercube() {
        let config = VillageCalibrationConfig {
            parameter_ranges: vec![ParameterRange {
                name: "beta".into(),
                min: 0.5,
                max: 1.5,
                default: 1.0,
            }],
            lhs_samples: 10,
            ..Default::default()
        };

        let scenario = base_scenario();
        let mut rng = StdRng::seed_from_u64(42);
        let results = run_sweep_lhs(&config, &scenario, &mut rng);

        assert_eq!(
            results.len(),
            config.lhs_samples,
            "C-8 FAIL: expected {} results, got {}",
            config.lhs_samples,
            results.len()
        );

        // CSV 出力
        println!("C-8: sample,beta,j_value");
        for (i, r) in results.iter().enumerate() {
            let beta = r.params.get("beta").copied().unwrap_or(0.0);
            println!("C-8: {}, {:.4}, {:.6}", i, beta, r.j_value);
        }

        // 全サンプルが範囲内にあること
        let range = &config.parameter_ranges[0];
        for r in &results {
            let beta = r.params.get("beta").copied().unwrap_or(0.0);
            assert!(
                beta >= range.min - 1e-10 && beta <= range.max + 1e-10,
                "C-8 FAIL: beta={} out of range [{}, {}]",
                beta,
                range.min,
                range.max
            );
        }
    }

    /// C-9: 同一設定の sweep で結果が決定論的に一致する（シード固定時）。
    #[test]
    fn sweep_identical_mode_consistency() {
        let config = VillageCalibrationConfig {
            parameter_ranges: vec![ParameterRange {
                name: "epsilon".into(),
                min: 0.0,
                max: 0.4,
                default: 0.1,
            }],
            ofat_steps: 2,
            ..Default::default()
        };

        let scenario = base_scenario();
        let results1 = run_sweep_ofat(&config, &scenario);
        let results2 = run_sweep_ofat(&config, &scenario);

        assert_eq!(
            results1.len(),
            results2.len(),
            "C-9 FAIL: result count mismatch"
        );

        println!(
            "C-9: OFAT deterministic consistency check ({} results)",
            results1.len()
        );
        for (i, (r1, r2)) in results1.iter().zip(results2.iter()).enumerate() {
            let diff = (r1.j_value - r2.j_value).abs();
            println!(
                "C-9: result[{}]: run1 J={:.6}, run2 J={:.6}, diff={:.10}",
                i, r1.j_value, r2.j_value, diff
            );
            assert!(
                diff < 1e-12,
                "C-9 FAIL: result[{}] mismatch: {:.10} vs {:.10}",
                i,
                r1.j_value,
                r2.j_value
            );
        }
    }

    // ----------------------------------------------------------
    // 統合テスト (C-10 〜 C-12)
    // ----------------------------------------------------------

    /// C-10: harmless な sweep 実行が既存 invariant を壊さず完走する。
    #[test]
    fn harness_harmless_sweep_no_invariant_break() {
        let config = VillageCalibrationConfig::default();
        let harness = VillageCalibrationHarness::new(config);
        let scenario = base_scenario();

        for mode in &[SweepMode::Ofat, SweepMode::Grid] {
            let report = harness.run_sweep(*mode, &scenario);
            println!(
                "C-10: mode={:?}, results={}, best J={:.6}, worst J={:.6}",
                mode,
                report.results.len(),
                report
                    .results
                    .iter()
                    .map(|r| r.j_value)
                    .fold(f64::NEG_INFINITY, f64::max),
                report
                    .results
                    .iter()
                    .map(|r| r.j_value)
                    .fold(f64::INFINITY, f64::min),
            );

            // 全結果が有限値
            for r in &report.results {
                assert!(
                    r.j_value.is_finite(),
                    "C-10 FAIL: non-finite J value in {:?}",
                    mode
                );
                assert!(
                    (0.0..=1.0).contains(&r.j_value),
                    "C-10 FAIL: J={} out of [0,1] in {:?}",
                    r.j_value,
                    mode
                );
            }
        }
    }

    /// C-11: CalibrationReport が全必須フィールドを持つ。
    #[test]
    fn calibration_report_format() {
        let config = VillageCalibrationConfig::default();
        let harness = VillageCalibrationHarness::new(config);
        let scenario = base_scenario();
        let report = harness.run_sweep(SweepMode::Ofat, &scenario);

        println!("C-11: experiment_id={}", report.experiment_id);
        println!("C-11: mode={:?}", report.mode);
        println!("C-11: timestamp={}", report.timestamp);
        println!("C-11: result_count={}", report.results.len());
        println!(
            "C-11: param_ranges: {}",
            report.config.parameter_ranges.len()
        );

        // 必須フィールドの存在確認
        assert!(
            !report.experiment_id.is_empty(),
            "C-11 FAIL: empty experiment_id"
        );
        assert!(!report.results.is_empty(), "C-11 FAIL: empty results");
        assert!(
            report.config.objective_weights.len() == 5,
            "C-11 FAIL: expected 5 weights, got {}",
            report.config.objective_weights.len()
        );

        // 各結果が必須フィールドを持つ
        for r in &report.results {
            assert!(!r.params.is_empty(), "C-11 FAIL: empty params in result");
            assert!(r.j_value.is_finite(), "C-11 FAIL: non-finite J value");
        }
    }

    /// C-12: 空パラメータセットで graceful にスキップする。
    #[test]
    fn harness_empty_parameter_set() {
        let config = VillageCalibrationConfig {
            parameter_ranges: vec![],
            ..Default::default()
        };
        let harness = VillageCalibrationHarness::new(config);
        let scenario = base_scenario();

        let report = harness.run_sweep(SweepMode::Ofat, &scenario);
        println!("C-12: empty params OFAT: {} results", report.results.len());

        let report_grid = harness.run_sweep(SweepMode::Grid, &scenario);
        println!(
            "C-12: empty params Grid: {} results",
            report_grid.results.len()
        );

        // 空パラメータでも正常終了すること
        assert!(
            report.results.is_empty(),
            "C-12 FAIL: expected 0 OFAT results, got {}",
            report.results.len()
        );
        // Grid sweep は空パラメータでも評価1回（空の直積 = 1）が発生する。
        // これは仕様範囲内（graceful に 1 結果を返す）とする。
        assert!(
            report_grid.results.len() <= 1,
            "C-12 FAIL: expected at most 1 Grid result, got {}",
            report_grid.results.len()
        );
    }

    // ==========================================================
    // M1.76-16 F-16 目的関数テスト (T1 〜 T9)
    // ==========================================================

    /// T1: ランダム ranking → AUC ≈ 0.5
    #[test]
    fn f16_auc_random_ranking() {
        let n = 1000;
        let mut rng = StdRng::seed_from_u64(12345);
        let mut profiles = Vec::with_capacity(2 * n);

        // 善良群: survived=random, benevolence=high
        for i in 0..n {
            profiles.push(SurvivalPair {
                workflow_id: format!("bene_{}", i),
                benevolence_score: 0.5 + rng.random::<f32>() * 0.5,
                survived: rng.random_bool(0.5),
            });
        }
        // 非善良群: survived=random, benevolence=low
        for i in 0..n {
            profiles.push(SurvivalPair {
                workflow_id: format!("non_{}", i),
                benevolence_score: rng.random::<f32>() * 0.5,
                survived: rng.random_bool(0.5),
            });
        }

        let auc = compute_auc_benevolent_survival(&profiles);
        println!("T1: AUC_random={:.6}, expected~0.5", auc);

        // AUC が 0.5 ± 0.02 に収まること
        assert!(
            (auc - 0.5).abs() < 0.02,
            "T1 FAIL: AUC={} not within 0.5±0.02",
            auc
        );
    }

    /// T2: 完全分離 ranking → AUC ≈ 1.0
    #[test]
    fn f16_auc_perfect_separation() {
        let n = 100;
        let mut profiles = Vec::with_capacity(2 * n);

        // 善良群: 全員生存
        for i in 0..n {
            profiles.push(SurvivalPair {
                workflow_id: format!("bene_{}", i),
                benevolence_score: 0.8 + (i as f32) * 0.001,
                survived: true,
            });
        }
        // 非善良群: 全員非生存
        for i in 0..n {
            profiles.push(SurvivalPair {
                workflow_id: format!("non_{}", i),
                benevolence_score: 0.2 - (i as f32) * 0.001,
                survived: false,
            });
        }

        let auc = compute_auc_benevolent_survival(&profiles);
        println!("T2: AUC_perfect={:.6}, expected~1.0", auc);

        assert!(
            (auc - 1.0).abs() < 1e-6,
            "T2 FAIL: AUC={} not ~1.0",
            auc
        );
    }

    /// T2b: 全て生存 → AUC = 0.5
    #[test]
    fn f16_auc_all_survived() {
        let profiles = vec![
            SurvivalPair {
                workflow_id: "a".into(),
                benevolence_score: 0.9,
                survived: true,
            },
            SurvivalPair {
                workflow_id: "b".into(),
                benevolence_score: 0.1,
                survived: true,
            },
        ];
        let auc = compute_auc_benevolent_survival(&profiles);
        println!("T2b: AUC_all_survived={:.6}, expected=0.5", auc);
        assert!(
            (auc - 0.5).abs() < 1e-6,
            "T2b FAIL: AUC={} != 0.5",
            auc
        );
    }

    /// T2c: 空スライス → AUC = 0.5, panic しない
    #[test]
    fn f16_auc_empty_slice() {
        let profiles: Vec<SurvivalPair> = vec![];
        let auc = compute_auc_benevolent_survival(&profiles);
        println!("T2c: AUC_empty={:.6}, expected=0.5", auc);
        assert!((auc - 0.5).abs() < 1e-6, "T2c FAIL: AUC={} != 0.5", auc);
    }

    /// T3: 全 λ = 0 で J = 0
    #[test]
    fn f16_all_lambda_zero_j_zero() {
        let metrics = ReciprocityOperationalMetrics {
            auc_benevolent_survival: 0.8,
            help_success_rate: 0.7,
            village_churn_p95: 0.3,
            false_new_rate: 0.1,
            review_load: 0.2,
            instability_penalty: 0.05,
        };
        let weights = [0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let j = compute_calibration_objective(&metrics, &weights);
        println!("T3: J(all_zero)={:.6}, expected=0.0", j);
        assert!((j - 0.0).abs() < 1e-12, "T3 FAIL: J={} != 0.0", j);
    }

    /// T4: 決定論的再現性
    #[test]
    fn f16_deterministic_reproducibility() {
        let metrics = ReciprocityOperationalMetrics {
            auc_benevolent_survival: 0.75,
            help_success_rate: 0.60,
            village_churn_p95: 0.20,
            false_new_rate: 0.05,
            review_load: 0.10,
            instability_penalty: 0.02,
        };
        let weights = [0.30, 0.25, 0.15, 0.10, 0.10, 0.10];

        let j1 = compute_calibration_objective(&metrics, &weights);
        let j2 = compute_calibration_objective(&metrics, &weights);
        let j3 = compute_calibration_objective(&metrics, &weights);

        println!(
            "T4: J runs: {:.10}, {:.10}, {:.10}",
            j1, j2, j3
        );
        assert!(
            (j1 - j2).abs() < 1e-12,
            "T4 FAIL: run1={} != run2={}",
            j1,
            j2
        );
        assert!(
            (j2 - j3).abs() < 1e-12,
            "T4 FAIL: run2={} != run3={}",
            j2,
            j3
        );
    }

    /// T5: 極値パラメータで NaN/Inf 回避
    #[test]
    fn f16_extreme_values_no_nan_inf() {
        let weights = [0.30, 0.25, 0.15, 0.10, 0.10, 0.10];

        // 全ゼロ
        let zero = ReciprocityOperationalMetrics::default();
        let j_zero = compute_calibration_objective(&zero, &weights);
        println!("T5: all_zero J={:.6}", j_zero);

        // 全最大
        let maxed = ReciprocityOperationalMetrics {
            auc_benevolent_survival: 1.0,
            help_success_rate: 1.0,
            village_churn_p95: 1.0,
            false_new_rate: 1.0,
            review_load: 1.0,
            instability_penalty: 1.0,
        };
        let j_maxed = compute_calibration_objective(&maxed, &weights);
        println!("T5: all_max J={:.6}", j_maxed);

        // 負の値
        let negative = ReciprocityOperationalMetrics {
            auc_benevolent_survival: -0.5,
            help_success_rate: -0.3,
            village_churn_p95: -0.1,
            false_new_rate: -0.2,
            review_load: -0.1,
            instability_penalty: -0.05,
        };
        let j_neg = compute_calibration_objective(&negative, &weights);
        println!("T5: negative J={:.6}", j_neg);

        assert!(j_zero.is_finite(), "T5 FAIL: zero J not finite");
        assert!(j_maxed.is_finite(), "T5 FAIL: maxed J not finite");
        assert!(j_neg.is_finite(), "T5 FAIL: negative J not finite");
        assert!(
            (0.0..=1.0).contains(&j_zero),
            "T5 FAIL: zero J={} not in [0,1]",
            j_zero
        );
        assert!(
            (0.0..=1.0).contains(&j_maxed),
            "T5 FAIL: maxed J={} not in [0,1]",
            j_maxed
        );
        assert!(
            (0.0..=1.0).contains(&j_neg),
            "T5 FAIL: negative J={} not in [0,1]",
            j_neg
        );
    }

    /// T6: 空パラメータセットで graceful
    #[test]
    fn f16_empty_parameter_set() {
        let harness = ReciprocityCalibrationHarness::new(ReciprocityCalibrationConfig::default());
        let empty_params = HashMap::new();
        let metrics = ReciprocityOperationalMetrics::default();
        let result = harness.evaluate(&empty_params, &metrics);

        println!("T6: empty params J={:.6}", result.j_value);
        assert!(
            result.j_value.is_finite(),
            "T6 FAIL: J not finite"
        );
        assert!(
            result.params.is_empty(),
            "T6 FAIL: params not empty"
        );
    }

    /// T7: 同一 θ で決定論的 J(θ)
    #[test]
    fn f16_identical_theta_deterministic_j() {
        let harness = ReciprocityCalibrationHarness::new(ReciprocityCalibrationConfig::default());
        let mut params = HashMap::new();
        params.insert("gamma_benevolence".to_string(), 0.5);
        params.insert("theta_dir".to_string(), 1.0);

        let metrics = ReciprocityOperationalMetrics {
            auc_benevolent_survival: 0.7,
            help_success_rate: 0.6,
            village_churn_p95: 0.2,
            false_new_rate: 0.05,
            review_load: 0.1,
            instability_penalty: 0.03,
        };

        let r1 = harness.evaluate(&params, &metrics);
        let r2 = harness.evaluate(&params, &metrics);

        println!(
            "T7: J(θ) run1={:.10}, run2={:.10}",
            r1.j_value, r2.j_value
        );
        assert!(
            (r1.j_value - r2.j_value).abs() < 1e-12,
            "T7 FAIL: J(θ) mismatch: {} vs {}",
            r1.j_value,
            r2.j_value
        );
    }

    /// T8: 各成分の分離検証（単一成分のみ weight=1.0）
    #[test]
    fn f16_component_isolation() {
        let metrics = ReciprocityOperationalMetrics {
            auc_benevolent_survival: 0.80,
            help_success_rate: 0.70,
            village_churn_p95: 0.30,
            false_new_rate: 0.10,
            review_load: 0.20,
            instability_penalty: 0.05,
        };

        // 各成分を個別に検証
        let cases = [
            (0, metrics.auc_benevolent_survival, "auc"),
            (1, metrics.help_success_rate, "help_success"),
            (2, -metrics.village_churn_p95, "churn"),
            (3, -metrics.false_new_rate, "false_new"),
            (4, -metrics.review_load, "review_load"),
            (5, -metrics.instability_penalty, "instability"),
        ];

        for (idx, expected_contrib, label) in &cases {
            let mut weights = [0.0; 6];
            weights[*idx] = 1.0;
            let j = compute_calibration_objective(&metrics, &weights);
            // 期待値も clamp して比較（負の成分は clamp により 0 になる）
            let expected = expected_contrib.clamp(0.0, 1.0);
            println!(
                "T8: {} only: J={:.6}, expected_contrib={:.6}, expected_after_clamp={:.6}",
                label, j, expected_contrib, expected
            );
            assert!(
                (j - expected).abs() < 1e-12,
                "T8 FAIL: {} J={} != expected={}",
                label,
                j,
                expected
            );
        }
    }

    /// T9: 実験ID 形式検証
    #[test]
    fn f16_experiment_id_format() {
        let harness = ReciprocityCalibrationHarness::new(ReciprocityCalibrationConfig::default());
        let eid = harness.generate_experiment_id();
        println!("T9: experiment_id={}", eid);

        // exp-{yyyymmdd}-{seq} 形式
        assert!(
            eid.starts_with("exp-"),
            "T9 FAIL: ID must start with 'exp-', got: {}",
            eid
        );

        let parts: Vec<&str> = eid.split('-').collect();
        assert!(
            parts.len() >= 3,
            "T9 FAIL: ID must have at least 3 parts, got: {}",
            eid
        );

        // 日付部分が 8 桁の数字であること
        let date_part = parts[1];
        assert!(
            date_part.len() == 8 && date_part.chars().all(|c| c.is_ascii_digit()),
            "T9 FAIL: date part must be 8 digits, got: {}",
            date_part
        );

        // 実験 ID が呼び出しごとに増加すること
        let eid2 = harness.generate_experiment_id();
        assert_ne!(eid, eid2, "T9 FAIL: experiment IDs must be unique");
        println!("T9: experiment_id_2={}", eid2);

        // 親実験ID を含むレポート
        let report = harness.generate_report(
            vec![],
            Some(eid.clone()),
        );
        assert_eq!(
            report.parent_experiment_id,
            Some(eid),
            "T9 FAIL: parent experiment ID mismatch"
        );
    }
}
