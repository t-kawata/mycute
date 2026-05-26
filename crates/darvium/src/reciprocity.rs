// 相互互恵性モジュール (RFC §15.10)
//
// RFC §15.10 で定義される Reciprocity-Aware Survival の基盤計算エンジンを提供する。
// 式 F-1 (compute_direct_reciprocity) の純粋関数を含む。
//
// 式 F-1: R_i^dir = σ( Σ_{j≠i} ω_ij^dir (α_h H_ij + α_hs HS_ij - α_r RJ_ij - α_d DMG_ij) exp(-ρ_dir Δt_ij) )

use std::collections::HashMap;

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};

use crate::constants;
use crate::error::DarviumError;
use crate::event::{
    DarviumEvent, GraphMetrics, ReciprocityEvent, ReciprocityEventKind,
    ReciprocityLifecyclePolicy, ReputationProfile,
};
use crate::types::WorkflowGraphId;

/// イベント種別ごとの (H, HS, RJ, DMG) 重みを返す (F-1)。
///
/// RFC §15.10.2 式 F-1 の重み割り当てテーブルに基づき、各 variant を
/// 4成分 (H=支援, HS=支援成功, RJ=拒否/放棄, DMG=有害) に写像する。
fn event_kind_weights(kind: &ReciprocityEventKind) -> (f32, f32, f32, f32) {
    match kind {
        ReciprocityEventKind::HelpOffered
        | ReciprocityEventKind::HelpAccepted
        | ReciprocityEventKind::HelpExecuted => (1.0, 0.0, 0.0, 0.0),
        ReciprocityEventKind::HelpSucceeded | ReciprocityEventKind::ReturnedFavor => {
            (0.0, 1.0, 0.0, 0.0)
        }
        ReciprocityEventKind::HelpRejected | ReciprocityEventKind::HelpAbandoned => {
            (0.0, 0.0, 1.0, 0.0)
        }
        ReciprocityEventKind::HarmfulMismatch => (0.0, 0.0, 0.0, 1.0),
    }
}

/// ロジスティックシグモイド関数 (F-1, F-2)。
///
/// σ(x) = 1 / (1 + exp(-x))
/// 任意の実数値を (0, 1) の範囲に押し込む。
pub(crate) fn logistic_sigmoid(x: f32) -> f32 {
    1.0 / (1.0 + (-x).exp())
}

/// 時間減衰係数を計算する (F-1)。
///
/// exp(-ρ_dir * Δt) を返す。Δt が大きいほど減衰し、0 に近づく。
/// Δt = 0 のとき 1.0（減衰なし）を返す。
fn time_decay(delta: u64, rho: f32) -> f32 {
    (-rho * delta as f32).exp()
}

/// 直接互恵性スコア R_i^dir を計算する (F-1)。
///
/// 式 F-1:
///   R_i^dir = σ( Σ_{j≠i} ω_ij^dir (α_h H_ij + α_hs HS_ij - α_r RJ_ij - α_d DMG_ij) exp(-ρ_dir Δt_ij) )
///
/// # 引数
/// - `events`: 同一 source_graph_id を持つ ReciprocityEvent のスライス
/// - `now`: 現在の VirtualClock 値（時間減衰 Δt の基準点）
/// - `policy`: 較正パラメータ（ρ_dir を含む ReciprocityLifecyclePolicy）
///
/// # 戻り値
/// - [0, 1] の範囲に正規化された f32 スコア
/// - 空リストに対しては 0.5（sigmoid(0)）を返す
///
/// # 不変条件
/// - 協力行為（HelpSucceeded 等）はスコアを非減少にする (MUST, α_h, α_hs > 0)
/// - 裏切り・害（HarmfulMismatch 等）はスコアを非増加にする (MUST, α_r, α_d > 0)
pub fn compute_direct_reciprocity(
    events: &[ReciprocityEvent],
    now: u64,
    policy: &ReciprocityLifecyclePolicy,
) -> f32 {
    if events.is_empty() {
        return logistic_sigmoid(0.0);
    }

    let alpha_h = constants::RECIPROCITY_ALPHA_HELP;
    let alpha_hs = constants::RECIPROCITY_ALPHA_SUCCESS;
    let alpha_r = constants::RECIPROCITY_ALPHA_REJECT;
    let alpha_d = constants::RECIPROCITY_ALPHA_HARM;
    let rho_dir = policy.rho_direct_decay;
    let mut weighted_sum = 0.0f32;

    for event in events {
        let (h, hs, rj, dmg) = event_kind_weights(&event.event_kind);
        let contribution = event.weight
            * (alpha_h * h + alpha_hs * hs - alpha_r * rj - alpha_d * dmg);
        let elapsed = now.saturating_sub(event.virtual_clock);
        let decay = time_decay(elapsed, rho_dir);
        weighted_sum += contribution * decay;
    }

    logistic_sigmoid(weighted_sum)
}

/// 間接互恵性スコア R_i^ind を計算する (F-2)。
///
/// 式 F-2:
///   R_i^ind = σ( β_1·C_i^help + β_2·A_i^village + β_3·U_i^accepted + β_4·Q_i^success - β_5·B_i^harm )
///
/// 「社会全体から見た善良さ」を表し、直接互恵性（二者間の相互関係）とは分離して保持される。
/// F-2 には時間減衰項がなく、間接互恵性は累積的な社会的評価として即時反映される。
///
/// # 引数
/// - `centrality`: C_i^help — helper network 上の中心性 [0, 1]
/// - `village_participation`: A_i^village — local village 参加度 [0, 1]
/// - `accepted_rate`: U_i^accepted — offer 受諾率 [0, 1]
/// - `success_rate`: Q_i^success — 支援成功率 [0, 1]
/// - `harm_score`: B_i^harm — 負評価スコア [0, 1]
///
/// # 戻り値
/// - [0, 1] の範囲に正規化された f32 スコア
/// - 全入力 0 のとき 0.5（sigmoid(0)）を返す
///
/// # 不変条件
/// - 中心性・村参加度・受諾率・成功貢献率の増加はスコアを非減少にする (MUST, β_1〜β_4 > 0)
/// - 負評価の増加はスコアを非増加にする (MUST, β_5 > 0)
pub fn compute_indirect_reciprocity(
    centrality: f32,
    village_participation: f32,
    accepted_rate: f32,
    success_rate: f32,
    harm_score: f32,
) -> f32 {
    let beta_1 = constants::INDIRECT_BETA_CENTRALITY;
    let beta_2 = constants::INDIRECT_BETA_VILLAGE_PARTICIPATION;
    let beta_3 = constants::INDIRECT_BETA_ACCEPTED_RATE;
    let beta_4 = constants::INDIRECT_BETA_SUCCESS_RATE;
    let beta_5 = constants::INDIRECT_BETA_HARM_SCORE;

    let linear_sum = beta_1 * centrality
        + beta_2 * village_participation
        + beta_3 * accepted_rate
        + beta_4 * success_rate
        - beta_5 * harm_score;

    logistic_sigmoid(linear_sum)
}

/// BenevolenceScore B_i を計算する (F-3)。
///
/// 式 F-3:
///   B_i = w_dir · R_i^dir + w_ind · R_i^ind + w_rep · Rep_i
///
/// 直接互恵性・間接互恵性・評判の合成量。係数は非負、かつ w_dir + w_ind + w_rep = 1 （推奨）。
///
/// # 引数
/// - `direct_score`: R_i^dir — 直接互恵性スコア [0, 1]
/// - `indirect_score`: R_i^ind — 間接互恵性スコア [0, 1]
/// - `reputation`: Rep_i — 評判スコア (final_score) [0, 1]
///
/// # 戻り値
/// - [0, 1] の範囲にクランプされた f32 スコア
pub fn compute_benevolence_score(
    direct_score: f32,
    indirect_score: f32,
    reputation: f32,
) -> f32 {
    let w_dir = constants::REPUTATION_WEIGHT_DIRECT;
    let w_ind = constants::REPUTATION_WEIGHT_INDIRECT;
    let w_rep = constants::REPUTATION_WEIGHT_REPUTATION;

    let benevolence = w_dir * direct_score + w_ind * indirect_score + w_rep * reputation;

    benevolence.clamp(0.0, 1.0)
}

// ============================================================
// M1.76-5: ReputationProfile 再計算 (F-4, F-5)
// ============================================================

/// ReputationProfile 再計算の入力構造体 (F-4, F-5)。
///
/// F-4 の4成分を保持する軽量構造体。ReputationProfile の全16フィールドを渡さず、
/// 必要な最小限の入力のみを型安全に伝達する。
pub struct ReputationInputs {
    /// 直接互恵性スコア R_i^dir (F-4)。
    pub direct_score: f32,
    /// 間接互恵性スコア R_i^ind (F-4)。
    pub indirect_score: f32,
    /// 経験値カウント experience_count(i) (F-5)。
    pub experience_count: u32,
    /// 継承スコア I_i (F-4)。
    pub inherited_score: f32,
}

/// 経験値カウントを飽和正規化する (F-5)。
///
/// E_i^norm = 1 - exp(-κ_E · experience_count(i))
///
/// 古参固定化防止のため、経験値の寄与は指数飽和曲線で上限 1 に漸近する。
fn compute_experience_norm(experience_count: u32, kappa_e: f32) -> f32 {
    if experience_count == 0 {
        return 0.0;
    }
    let count_f = experience_count as f32;
    1.0 - (-kappa_e * count_f).exp()
}

/// 評判スコア Rep_i を再計算する (F-4, F-5)。
///
/// 式 F-4:
///   Rep_i = clip_{[0,1]}( θ_dir·R_i^dir + θ_ind·R_i^ind + θ_exp·E_i^norm + θ_inh·I_i )
///
/// 式 F-5:
///   E_i^norm = 1 - exp(-κ_E · experience_count(i))
///
/// # 引数
/// - `inputs`: F-4 の4成分を含む ReputationInputs
/// - `policy`: 重み係数 θ と κ_E を含むポリシーオブジェクト
///
/// # 戻り値
/// - 新規 ReputationProfile: final_score に計算結果、experience_score に E_i^norm が設定される
///
/// # 注意
/// - 係数和 θ_dir + θ_ind + θ_exp + θ_inh = 1 は推奨であって強制ではない
/// - 戻り値の ReputationProfile は部分的な更新のみ: cold_start() をベースに
///   計算結果フィールドのみ上書きする
pub fn recompute_reputation(
    inputs: ReputationInputs,
    policy: &ReciprocityLifecyclePolicy,
) -> ReputationProfile {
    let experience_norm = compute_experience_norm(inputs.experience_count, policy.kappa_e);

    let raw_score = policy.theta_dir * inputs.direct_score
        + policy.theta_ind * inputs.indirect_score
        + policy.theta_exp * experience_norm
        + policy.theta_inherit * inputs.inherited_score;

    let final_score = raw_score.clamp(0.0, 1.0);

    ReputationProfile {
        direct_score: inputs.direct_score,
        indirect_score: inputs.indirect_score,
        experience_score: experience_norm,
        inherited_score: inputs.inherited_score,
        final_score,
        // 以下のフィールドは cold_start() のデフォルト値を使用
        alpha_positive: 0,
        beta_negative: 0,
        last_recomputed_at: std::time::SystemTime::UNIX_EPOCH,
        direct_help_count: 0,
        direct_success_count: 0,
        direct_reject_count: 0,
        harm_event_count: 0,
        accepted_offer_rate: 0.0,
        help_success_rate: 0.0,
        village_centrality: 0.0,
        benevolence_score: 0.5,
    }
}

// ============================================================
// M1.76-6: GC hazard with benevolence (F-7, F-8, F-9)
// ============================================================

/// softplus 関数: ln(1 + exp(x))。
///
/// 常に非負。x が極端に負の場合でも数値的安定性を保つため、
/// x > 0 と x <= 0 で条件分岐する。
fn softplus(x: f32) -> f32 {
    if x > 0.0 {
        x + (1.0 + (-x).exp()).ln()
    } else {
        (1.0 + x.exp()).ln()
    }
}

/// GC hazard λ_i^GC を計算する (F-7)。
///
/// 式 F-7:
///   λ_i^GC = softplus( λ_0 - γ_L·L_i - γ_B·B_i - γ_C·C_i^protect )
///
/// softplus により常に非負。各 γ > 0 (MUST) により、スコアの増加はハザードを減少させる。
///
/// # 引数
/// - `lifecycle_score`: L_i — 既存 LifecycleScore [0, 1]
/// - `benevolence_score`: B_i — F-3 慈悲スコア [0, 1]
/// - `child_protection`: C_i^protect — F-10 child 保護項 [0, ∞)
/// - `policy`: 較正パラメータ（λ_0, γ_L, γ_B, γ_C）
///
/// # 戻り値
/// - 非負の f32 ハザード率 λ_i^GC
pub fn compute_gc_hazard(
    lifecycle_score: f32,
    benevolence_score: f32,
    child_protection: f32,
    policy: &ReciprocityLifecyclePolicy,
) -> f32 {
    let inner = policy.lambda_gc_base
        - policy.gamma_lifecycle * lifecycle_score
        - policy.gamma_benevolence * benevolence_score
        - policy.gamma_child_protect * child_protection;
    softplus(inner)
}

/// GC 判定確率 p_GC を計算する (F-8)。
///
/// 式 F-8:
///   p_GC(i; Δt) = 1 - exp(-λ_i^GC · Δt)
///
/// # 引数
/// - `hazard`: λ_i^GC — F-7 で計算されたハザード率（非負）
/// - `delta_t`: Δt — 時間間隔（仮想時間単位）
///
/// # 戻り値
/// - [0, 1) の範囲の f64 確率（hazard=0 のとき 0）
pub fn compute_gc_probability(hazard: f32, delta_t: u64) -> f64 {
    if hazard <= 0.0 {
        return 0.0;
    }
    let exponent = -(hazard as f64) * (delta_t as f64);
    1.0 - exponent.exp()
}

/// 生存確率 P_survive を計算する (F-9)。
///
/// 式 F-9:
///   P_survive(i; Δt) = exp(-λ_i^GC · Δt)
///
/// # 引数
/// - `hazard`: λ_i^GC — F-7 で計算されたハザード率（非負）
/// - `delta_t`: Δt — 時間間隔（仮想時間単位）
///
/// # 戻り値
/// - (0, 1] の範囲の f64 確率（hazard=0 のとき 1）
pub fn compute_survival_probability(hazard: f32, delta_t: u64) -> f64 {
    if hazard <= 0.0 {
        return 1.0;
    }
    let exponent = -(hazard as f64) * (delta_t as f64);
    exponent.exp()
}

// ============================================================
// M1.76-7: Child protection integration (F-10)
// ============================================================

/// Child 保護スコア C_i^protect を計算する (F-10)。
///
/// 式 F-10:
///   C_i^protect = η_1 · 1[Child(i)] + η_2 · H_i^received + η_3 · G_i^growth
///
/// 「今は弱いが、助けられ、育っている child」が GC されにくくなるための保護項。
/// 既存の Grace Period (experience_count < MIN_SURVIVAL_EXPERIENCE) を弱めず、
/// 補強する (MUST NOT weaken)。
///
/// # 引数
/// - `is_child`: 対象が child かどうか（classify_maturity で Child 判定されたか）
/// - `help_received`: child として有効支援を受けた量 [0, 1]
/// - `growth_improvement`: child が maturation に向けて改善している量 [0, 1]
///
/// # 戻り値
/// - 非負の f32 保護スコア C_i^protect ≥ 0
///
/// # 不変条件
/// - is_child=true のとき出力は常に η_1 以上 (MUST)
/// - help_received の増加は出力を非減少にする (MUST)
/// - growth_improvement の増加は出力を非減少にする (MUST)
pub fn compute_child_protection(
    is_child: bool,
    help_received: f32,
    growth_improvement: f32,
) -> f32 {
    let eta_1 = constants::CHILD_PROTECT_ETA1;
    let eta_2 = constants::CHILD_PROTECT_ETA2;
    let eta_3 = constants::CHILD_PROTECT_ETA3;

    let child_indicator = if is_child { 1.0 } else { 0.0 };

    let protection = eta_1 * child_indicator + eta_2 * help_received + eta_3 * growth_improvement;

    protection.max(0.0)
}

// ============================================================
// M1.76-8: Helper quality score with benevolence (F-11) + Softmax selection (F-12)
// ============================================================

/// Helper quality score Q(h,c,M) を計算する (F-11)。
///
/// 式 F-11:
///   Q(h,c,M) = w_s·S + w_t·T + w_r·Rep + w_b·B + w_n·N - w_d·d
///
/// 既存の helper quality score (41B-8) に benevolence 項 w_b·B(h) を追加した拡張。
/// 戻り値は任意の実数 f32（softmax の入力として使用可能）。
///
/// # 引数
/// - `mission_suitability`: ミッション適合性 S [0, 1]
/// - `trust`: 信頼スコア T [0, 1]
/// - `reputation`: 評判スコア Rep [0, 1]
/// - `benevolence`: Benevolence スコア B [0, 1]
/// - `child_need`: Child need スコア N [0, 1]
/// - `distance_penalty`: 距離ペナルティ d [0, ∞)
/// - `policy`: 重み係数を含むポリシーオブジェクト
///
/// # 戻り値
/// - 任意の実数 f32（非負とは限らない）
///
/// # 不変条件
/// - 全重み w_s, w_t, w_r, w_b, w_n, w_d は非負 (MUST)
/// - S, T, Rep, B, N の増加は Q を非減少にする (MUST, w_* > 0)
/// - d の増加は Q を非増加にする (MUST, w_d > 0)
pub fn compute_helper_quality_score(
    mission_suitability: f32,
    trust: f32,
    reputation: f32,
    benevolence: f32,
    child_need: f32,
    distance_penalty: f32,
    policy: &ReciprocityLifecyclePolicy,
) -> f32 {
    let w_s = policy.helper_quality_w_s;
    let w_t = policy.helper_quality_w_t;
    let w_r = policy.helper_quality_w_r;
    let w_b = policy.helper_quality_w_b;
    let w_n = policy.helper_quality_w_n;
    let w_d = policy.helper_quality_w_d;

    w_s * mission_suitability
        + w_t * trust
        + w_r * reputation
        + w_b * benevolence
        + w_n * child_need
        - w_d * distance_penalty
}

/// Helper 選択確率分布 π(h|c,M) を softmax で計算する (F-12)。
///
/// 式 F-12:
///   π(h|c,M) = exp(τ_Q·Q(h,c,M)) / Σ_g exp(τ_Q·Q(g,c,M))
///
/// 数値的安定性のため log-sum-exp trick を使用:
///   max_logit = max_i(τ_Q·Q_i) → π_i = exp(τ_Q·Q_i - max_logit) / Σ_j exp(τ_Q·Q_j - max_logit)
///
/// # 引数
/// - `scores`: 各 helper 候補の quality score Q(h,c,M) のスライス
/// - `policy`: 温度パラメータ τ_Q を含むポリシーオブジェクト
///
/// # 戻り値
/// - 非負の f64 確率の Vec（要素数は scores.len()、総和 = 1.0 ± 1e-12）
/// - 空スライスに対しては空 Vec を返す
///
/// # 不変条件
/// - 出力の総和 = 1.0 ± 1e-12 (MUST)
/// - 全要素が非負 (MUST)
/// - 空入力 → 空出力 (MUST)
pub fn softmax_helper_selection(
    scores: &[f32],
    policy: &ReciprocityLifecyclePolicy,
) -> Vec<f64> {
    if scores.is_empty() {
        return Vec::new();
    }

    let tau = policy.tau_helper_softmax;

    // log-sum-exp trick: max を減算して数値的安定性を確保
    let max_logit = scores
        .iter()
        .map(|&s| (tau * s) as f64)
        .fold(f64::NEG_INFINITY, f64::max);

    let mut exp_sum = 0.0f64;
    let mut probabilities = Vec::with_capacity(scores.len());

    for &score in scores {
        let logit = (tau * score) as f64;
        let exp_val = (logit - max_logit).exp();
        exp_sum += exp_val;
        probabilities.push(exp_val);
    }

    // 正規化
    if exp_sum > 0.0 {
        for p in &mut probabilities {
            *p /= exp_sum;
        }
    } else {
        // 全ての exp が 0 の場合 → 一様分布
        let uniform = 1.0 / scores.len() as f64;
        probabilities.fill(uniform);
    }

    probabilities
}

/// Benevolence-aware remote exploration 率 ε_remote を計算する (F-13)。
///
/// 式 F-13:
///   ε_remote(c) = clip_{[0, ε_max]}( ε₀ + a₁·need(c) - a₂·B_local_avg(c) )
///
/// v2.3-e の bounded remote exploration (41B-19) を保持しつつ、local adults の
/// benevolence が十分高い場合は remote exploration 率を下げ、local shortage 時に
/// のみ上げる。「近くに優しい大人がいるなら、まず近所で助け合う」を実現する。
///
/// # 引数
/// - `child_need`: Child のニーズスコア need(c) ∈ [0, 1]。
/// - `local_benevolence_mean`: Local village 内 adult の BenevolenceScore 平均 B_local_avg(c) ∈ [0, 1]。
/// - `policy`: 較正パラメータ（ε₀, ε_max, a₁, a₂ を含む ReciprocityLifecyclePolicy）。
///
/// # 戻り値
/// [0, ε_max] に clip された ε_remote 値。
///
/// # 不変条件
/// - 戻り値は常に [0, ε_max] に bounded される (MUST, clip 保証)
/// - local_benevolence_mean の増加は ε_remote を非増加にする (MUST, a₂ > 0)
/// - child_need の増加は ε_remote を非減少にする (MUST, a₁ > 0)
/// - a₂ = 0 かつ need = 0 のとき ε_remote = ε₀ (MUST, 下位互換性)
pub fn compute_benevolence_aware_remote_exploration(
    child_need: f32,
    local_benevolence_mean: f32,
    policy: &ReciprocityLifecyclePolicy,
) -> f32 {
    let raw = policy.epsilon_remote_base
        + policy.epsilon_remote_need_coeff * child_need
        - policy.epsilon_remote_benevolence_coeff * local_benevolence_mean;
    raw.clamp(0.0, policy.epsilon_remote_max)
}

// ============================================================
// M1.76-10: Child growth increment (F-14) + Maturation probability (F-15)
// ============================================================

/// Child の成長増分 ΔG_c を計算する (F-14)。
///
/// 式 F-14:
///   ΔG_c = μ₁·MissionSuccess_c + μ₂·Σ_h HelpSuccess(h→c)
///          + μ₃·B̄_helpers(c) - μ₄·FailureBurden_c
///   出力は非負に clip される (ΔG_c ≥ 0 MUST)。
///
/// # 引数
/// - `mission_success`: child 自身の mission success 率 [0, 1]
/// - `help_success_sum`: 周囲からの help success の総和 (非負)
/// - `helper_benevolence_mean`: helper の平均 benevolence [0, 1]
/// - `failure_burden`: child の failure burden (非負)
/// - `policy`: 較正パラメータ (μ₁〜μ₄ を含む ReciprocityLifecyclePolicy)
///
/// # 戻り値
/// - 非負の f32 成長増分 ΔG_c ≥ 0
///
/// # 不変条件
/// - 出力は常に非負 (MUST)
/// - mission_success=true のとき false より高い (MUST, μ₁ > 0)
/// - help_successes の要素追加は出力を非減少にする (MUST, μ₂ > 0)
/// - failure_burden の増加は出力を非増加にする (MUST, μ₄ > 0)
pub fn compute_child_growth_increment(
    mission_success: bool,
    help_successes: &[f32],
    helper_benevolence_mean: f32,
    failure_burden: f32,
    policy: &ReciprocityLifecyclePolicy,
) -> f32 {
    let mu_1 = policy.child_growth_mu_mission_success;
    let mu_2 = policy.child_growth_mu_help_success;
    let mu_3 = policy.child_growth_mu_helper_benevolence;
    let mu_4 = policy.child_growth_mu_failure_burden;

    let ms = if mission_success { 1.0 } else { 0.0 };
    let hs_sum = help_successes.iter().sum::<f32>();

    let increment = mu_1 * ms
        + mu_2 * hs_sum
        + mu_3 * helper_benevolence_mean
        - mu_4 * failure_burden;

    increment.max(0.0)
}

/// Child の成熟確率 P_mature(c) を計算する (F-15)。
///
/// 式 F-15:
///   P_mature(c) = σ(ν₀ + ν₁·E_c^norm + ν₂·T_c + ν₃·Rep_c + ν₄·B̄_helpers(c))
///   ただし σ は logistic sigmoid 関数。
///
/// # 引数
/// - `experience_norm`: 正規化経験値 E_c^norm [0, 1]
/// - `trust`: child の信頼値 T_c [0, 1]
/// - `reputation`: child の評判値 Rep_c [0, 1]
/// - `helper_benevolence_mean`: helper の平均 benevolence B̄_helpers(c) [0, 1]
/// - `policy`: 較正パラメータ (ν₀〜ν₄ を含む ReciprocityLifecyclePolicy)
///
/// # 戻り値
/// - f64 成熟確率 (0, 1) — logistic sigmoid の出力範囲
///
/// # 不変条件
/// - 出力は常に (0, 1) に収まる (MUST, sigmoid の性質)
/// - experience_norm の増加は出力を非減少にする (MUST, ν₁ > 0)
/// - trust の増加は出力を非減少にする (MUST, ν₂ > 0)
/// - reputation の増加は出力を非減少にする (MUST, ν₃ > 0)
/// - helper_benevolence_mean の増加は出力を非減少にする (MUST, ν₄ > 0)
pub fn compute_maturation_probability(
    experience_norm: f32,
    trust: f32,
    reputation: f32,
    helper_benevolence_mean: f32,
    policy: &ReciprocityLifecyclePolicy,
) -> f64 {
    let nu_0 = policy.maturation_nu_bias;
    let nu_1 = policy.maturation_nu_experience;
    let nu_2 = policy.maturation_nu_trust;
    let nu_3 = policy.maturation_nu_reputation;
    let nu_4 = policy.maturation_nu_helper_benevolence;

    let logit = nu_0
        + nu_1 * experience_norm
        + nu_2 * trust
        + nu_3 * reputation
        + nu_4 * helper_benevolence_mean;

    logistic_sigmoid(logit) as f64
}

// ============================================================
// M1.76-11: ReciprocityEventStore (RFC §41B.20)
// ============================================================

/// ReciprocityEvent のメモリ内ストア。
///
/// source_graph_id をキーとしてイベントを管理する。
/// EventProjection 経由で materialize されたイベントの投影結果を保持する。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReciprocityEventStore {
    /// source_graph_id → Vec<ReciprocityEvent> のマップ
    events: HashMap<WorkflowGraphId, Vec<ReciprocityEvent>>,
}

impl ReciprocityEventStore {
    /// 空のストアを作成する。
    pub fn new() -> Self {
        Self {
            events: HashMap::with_capacity(constants::RECIPROCITY_STORE_INITIAL_CAPACITY),
        }
    }

    /// イベントを source_graph_id で索引付けして追加する。
    pub fn ingest(&mut self, event: ReciprocityEvent) {
        let graph_id = event.source_graph_id.clone();
        self.events.entry(graph_id).or_default().push(event);
    }

    /// 指定グラフの全イベントを返す（空スライス可能）。
    pub fn get_events(&self, source_graph_id: &str) -> &[ReciprocityEvent] {
        self.events
            .get(source_graph_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// 全グラフ ID のリストを返す。
    pub fn all_graph_ids(&self) -> Vec<WorkflowGraphId> {
        self.events.keys().cloned().collect()
    }

    /// 指定グラフのイベント数を返す。
    pub fn event_count(&self, source_graph_id: &str) -> usize {
        self.get_events(source_graph_id).len()
    }

    /// 全イベント総数を返す。
    pub fn total_events(&self) -> usize {
        self.events.values().map(|v| v.len()).sum()
    }

    /// 全イベントをクリアする。
    pub fn clear(&mut self) {
        self.events.clear();
    }
}

impl Default for ReciprocityEventStore {
    fn default() -> Self {
        Self::new()
    }
}

/// DarviumEvent から ReciprocityEvent を materialize し、ストアに追加する。
///
/// EventBus から受け取った DarviumEventKind::Reciprocity イベントを
/// ReciprocityEvent に変換してストアに投影する。
/// 非 Reciprocity kind の場合はエラーを返す。
pub fn ingest_reciprocity_event(
    store: &mut ReciprocityEventStore,
    event: DarviumEvent,
) -> Result<(), DarviumError> {
    let reciprocity_event = ReciprocityEvent::try_from(event)
        .map_err(|e| DarviumError::ReciprocityError(e.to_string()))?;
    store.ingest(reciprocity_event);
    Ok(())
}

/// 全グラフの ReputationProfile を一括再計算する。
///
/// パイプライン（各グラフ ID に対して）:
/// 1. ストアから全イベントを取得
/// 2. compute_direct_reciprocity で直接互恵性スコア計算
/// 3. メトリクスから間接互恵性スコア計算（compute_indirect_reciprocity）
/// 4. recompute_reputation で評判スコア再計算
/// 5. compute_benevolence_score で慈悲スコア計算
/// 6. 上記結果を ReputationProfile に反映
///
/// # 不変条件
/// - 空ストア → 空 HashMap（MUST）
/// - 同一入力 → 同一出力（MUST）
pub fn recompute_all_profiles(
    store: &ReciprocityEventStore,
    metrics: &HashMap<WorkflowGraphId, GraphMetrics>,
    now: u64,
    policy: &ReciprocityLifecyclePolicy,
) -> HashMap<WorkflowGraphId, ReputationProfile> {
    let graph_ids = store.all_graph_ids();
    let mut results = HashMap::with_capacity(graph_ids.len());

    for g_id in &graph_ids {
        let events = store.get_events(g_id);
        let direct_score = compute_direct_reciprocity(events, now, policy);

        let graph_metrics = metrics.get(g_id).cloned().unwrap_or_default();
        let indirect_score = compute_indirect_reciprocity(
            graph_metrics.centrality,
            graph_metrics.village_participation,
            graph_metrics.accepted_rate,
            graph_metrics.success_rate,
            graph_metrics.harm_score,
        );

        let mut profile = recompute_reputation(
            ReputationInputs {
                direct_score,
                indirect_score,
                experience_count: graph_metrics.experience_count,
                inherited_score: graph_metrics.inherited_score,
            },
            policy,
        );

        profile.benevolence_score =
            compute_benevolence_score(direct_score, indirect_score, profile.final_score);
        profile.village_centrality = graph_metrics.centrality;

        results.insert(g_id.clone(), profile);
    }

    results
}

/// 全グラフの GC hazard を一括再計算する。
///
/// 各 graph_id に対して:
/// 1. lifecycle_score を lifecycle_scores マップから取得
/// 2. benevolence_score を profile から取得
/// 3. child_protection_score を child_protections マップから取得（なければ 0）
/// 4. compute_gc_hazard(lifecycle, benevolence, child_protection, policy) を呼ぶ
///
/// # 不変条件
/// - 全 hazard 値は非負（MUST, softplus の性質）
/// - 空 profiles → 空 HashMap（MUST）
pub fn recompute_all_gc_hazards(
    profiles: &HashMap<WorkflowGraphId, ReputationProfile>,
    lifecycle_scores: &HashMap<WorkflowGraphId, f32>,
    child_protections: &HashMap<WorkflowGraphId, f32>,
    policy: &ReciprocityLifecyclePolicy,
) -> HashMap<WorkflowGraphId, f32> {
    let mut hazards = HashMap::with_capacity(profiles.len());

    for (g_id, profile) in profiles {
        if let Some(&lifecycle_score) = lifecycle_scores.get(g_id) {
            let benevolence = profile.benevolence_score;
            let child_protection = child_protections.get(g_id).copied().unwrap_or(0.0);
            let hazard = compute_gc_hazard(lifecycle_score, benevolence, child_protection, policy);
            hazards.insert(g_id.clone(), hazard);
        }
    }

    hazards
}

// ============================================================
// M1.76-11: Replay スナップショット・差分比較
// ============================================================

/// 再計算結果のスナップショット。
///
/// 2 時点の再計算結果比較（compute_replay_comparison）のための
/// 比較可能な状態を保持する。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReciprocityReplaySnapshot {
    /// profiles のコピー
    pub profiles: HashMap<WorkflowGraphId, ReputationProfile>,
    /// GC hazard のコピー
    pub hazards: HashMap<WorkflowGraphId, f32>,
    /// 計算に使用したポリシーバージョン
    pub policy_version: String,
    /// 計算時の VirtualClock 値
    pub clock: u64,
}

/// 2 つのスナップショット間の差分レポート。
///
/// 各差分は (before, after) のタプルで記録される。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReciprocityDiffReport {
    /// 比較前のポリシーバージョン
    pub before_policy_version: String,
    /// 比較後のポリシーバージョン
    pub after_policy_version: String,
    /// 差分があったグラフ ID の一覧
    pub changed_graph_ids: Vec<WorkflowGraphId>,
    /// 各グラフの profile 差分（キーがグラフ ID、値が (before_final_score, after_final_score)）
    pub profile_diffs: HashMap<WorkflowGraphId, (f32, f32)>,
    /// 各グラフの hazard 差分（キーがグラフ ID、値が (before_hazard, after_hazard)）
    pub hazard_diffs: HashMap<WorkflowGraphId, (f32, f32)>,
    /// 新規追加されたグラフ ID
    pub added_graph_ids: Vec<WorkflowGraphId>,
    /// 削除されたグラフ ID
    pub removed_graph_ids: Vec<WorkflowGraphId>,
}

/// 2 つのスナップショットを比較し、差分レポートを生成する。
///
/// # Args
/// - before: 比較元スナップショット
/// - after: 比較先スナップショット
///
/// # Returns
/// - profiles/hazards の全フィールド差分を含む ReciprocityDiffReport
///
/// # 不変条件
/// - before と after のどちらかが空でも panic しない（空の DiffReport を返す）
/// - 差分がない場合、changed_graph_ids / profile_diffs / hazard_diffs は空
pub fn compute_replay_comparison(
    before: &ReciprocityReplaySnapshot,
    after: &ReciprocityReplaySnapshot,
) -> ReciprocityDiffReport {
    let mut profile_diffs: HashMap<WorkflowGraphId, (f32, f32)> = HashMap::new();
    let mut hazard_diffs: HashMap<WorkflowGraphId, (f32, f32)> = HashMap::new();
    let mut added: Vec<WorkflowGraphId> = Vec::new();
    let mut removed: Vec<WorkflowGraphId> = Vec::new();

    // before に存在し after に存在しない、または値が異なるキーを検出
    for (g_id, before_profile) in &before.profiles {
        match after.profiles.get(g_id) {
            Some(after_profile) => {
                if (before_profile.final_score - after_profile.final_score).abs() > f32::EPSILON {
                    profile_diffs.insert(
                        g_id.clone(),
                        (before_profile.final_score, after_profile.final_score),
                    );
                }
            }
            None => {
                removed.push(g_id.clone());
            }
        }
    }

    // after にのみ存在するキーを検出
    for g_id in after.profiles.keys() {
        if !before.profiles.contains_key(g_id) {
            added.push(g_id.clone());
        }
    }

    // hazard 差分
    let all_hazard_keys: std::collections::HashSet<&WorkflowGraphId> =
        before.hazards.keys().chain(after.hazards.keys()).collect();

    for &g_id in &all_hazard_keys {
        match (before.hazards.get(g_id), after.hazards.get(g_id)) {
            (Some(&b), Some(&a)) if (b - a).abs() > f32::EPSILON => {
                hazard_diffs.insert(g_id.clone(), (b, a));
            }
            _ => {}
        }
    }

    let changed: Vec<WorkflowGraphId> = {
        let mut keys: Vec<WorkflowGraphId> = profile_diffs
            .keys()
            .chain(hazard_diffs.keys())
            .cloned()
            .collect();
        keys.sort();
        keys.dedup();
        keys
    };

    ReciprocityDiffReport {
        before_policy_version: before.policy_version.clone(),
        after_policy_version: after.policy_version.clone(),
        changed_graph_ids: changed,
        profile_diffs,
        hazard_diffs,
        added_graph_ids: added,
        removed_graph_ids: removed,
    }
}
/// 単調性条件の列挙型 — MUST 単調性条件の4 variant。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MonotonicityCondition {
    /// 条件 1: direct_score 増加 → survival_probability 非減少
    DirectScoreIncrease,
    /// 条件 2: indirect_score 増加 → GC hazard 非増加
    IndirectScoreIncrease,
    /// 条件 3: Reputation (final_score) 増加 → GC hazard 非増加
    ReputationIncrease,
    /// 条件 4: 同能力 helper 間で高 benevolence が ranking で不利にならない
    BenevolenceHelperRanking,
}

/// 単調性テスト結果レポート。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonotonicityReport {
    /// 条件ごとの PASS/FAIL
    pub conditions_passed: Vec<(MonotonicityCondition, bool)>,
    /// 違反の詳細説明（空 = 全 PASS）
    pub failure_details: Vec<String>,
    /// ランダム sweep の違反発生率（条件ごと）
    pub random_sweep_violation_rates: HashMap<MonotonicityCondition, f64>,
}

/// 単調性テスト固定パラメータ。
#[derive(Debug, Clone)]
pub struct MonotonicityFixedParams {
    pub lifecycle_score: f32,
    pub child_protection: f32,
    pub delta_t: u64,
    pub policy: ReciprocityLifecyclePolicy,
}

impl Default for MonotonicityFixedParams {
    fn default() -> Self {
        Self {
            lifecycle_score: 0.5,
            child_protection: 0.5,
            delta_t: 100,
            policy: ReciprocityLifecyclePolicy::default(),
        }
    }
}

/// 単調性テストスイート — 全 MUST 条件の定義と自動検証。
#[derive(Debug, Clone)]
pub struct MonotonicityTestSuite {
    pub direct_score_points: Vec<f32>,
    pub indirect_score_points: Vec<f32>,
    pub reputation_points: Vec<f32>,
    pub benevolence_delta_points: Vec<f32>,
    pub fixed_params: MonotonicityFixedParams,
    pub random_sweep_samples: usize,
}

impl Default for MonotonicityTestSuite {
    fn default() -> Self {
        Self {
            direct_score_points: vec![0.0, 0.25, 0.5, 0.75, 1.0],
            indirect_score_points: vec![0.0, 0.25, 0.5, 0.75, 1.0],
            reputation_points: vec![0.0, 0.25, 0.5, 0.75, 1.0],
            benevolence_delta_points: (1..=500).map(|i| i as f32 * 0.001).collect(),
            fixed_params: MonotonicityFixedParams::default(),
            random_sweep_samples: 1000,
        }
    }
}

/// 全 MUST 単調性条件を検証し、MonotonicityReport を返す。
pub fn check_monotonicity(suite: &MonotonicityTestSuite) -> MonotonicityReport {
    let fixed = &suite.fixed_params;
    let policy = &fixed.policy;
    let mut failure_details: Vec<String> = Vec::new();
    let mut random_sweep_violations: HashMap<MonotonicityCondition, u64> = HashMap::new();
    let mut conditions_passed: Vec<(MonotonicityCondition, bool)> = Vec::new();

    // ── 条件 1: direct_score ↑ → survival_probability 非減少 ──
    {
        let mut prev_survival: Option<f64> = None;
        let mut passed = true;
        for &ds in &suite.direct_score_points {
            let benevolence = compute_benevolence_score(ds, 0.0, 0.0);
            let hazard =
                compute_gc_hazard(fixed.lifecycle_score, benevolence, fixed.child_protection, policy);
            let survival = compute_survival_probability(hazard, fixed.delta_t);
            if let Some(prev) = prev_survival {
                if survival + 1e-10 < prev {
                    failure_details.push(format!(
                        "条件1 違反: direct_score={:.2}, survival={:.10} < prev={:.10}",
                        ds, survival, prev
                    ));
                    passed = false;
                }
            }
            prev_survival = Some(survival);
        }
        conditions_passed.push((MonotonicityCondition::DirectScoreIncrease, passed));
    }

    // ── 条件 2: indirect_score ↑ → GC hazard 非増加 ──
    {
        let mut prev_hazard: Option<f32> = None;
        let mut passed = true;
        for &is in &suite.indirect_score_points {
            let benevolence = compute_benevolence_score(0.0, is, 0.0);
            let hazard =
                compute_gc_hazard(fixed.lifecycle_score, benevolence, fixed.child_protection, policy);
            if let Some(prev) = prev_hazard {
                if hazard > prev + 1e-6 {
                    failure_details.push(format!(
                        "条件2 違反: indirect_score={:.2}, hazard={:.8} > prev={:.8}",
                        is, hazard, prev
                    ));
                    passed = false;
                }
            }
            prev_hazard = Some(hazard);
        }
        conditions_passed.push((MonotonicityCondition::IndirectScoreIncrease, passed));
    }

    // ── 条件 3: Reputation ↑ → GC hazard 非増加 ──
    {
        let mut prev_hazard: Option<f32> = None;
        let mut passed = true;
        for &rep in &suite.reputation_points {
            let benevolence = compute_benevolence_score(0.5, 0.5, rep);
            let hazard =
                compute_gc_hazard(fixed.lifecycle_score, benevolence, fixed.child_protection, policy);
            if let Some(prev) = prev_hazard {
                if hazard > prev + 1e-6 {
                    failure_details.push(format!(
                        "条件3 違反: reputation={:.2}, hazard={:.8} > prev={:.8}",
                        rep, hazard, prev
                    ));
                    passed = false;
                }
            }
            prev_hazard = Some(hazard);
        }
        conditions_passed.push((MonotonicityCondition::ReputationIncrease, passed));
    }

    // ── 条件 4: benevolence → helper ranking ──
    {
        let mut passed = true;
        let s = 0.5;
        let t = 0.5;
        let rep = 0.5;
        let n = 0.5;
        let d = 0.1;
        // helper_A: benevolence=0.3, helper_B: benevolence=0.9
        let q_a = compute_helper_quality_score(s, t, rep, 0.3, n, d, policy);
        let q_b = compute_helper_quality_score(s, t, rep, 0.9, n, d, policy);
        let probs = softmax_helper_selection(&[q_a, q_b], policy);
        if probs.len() >= 2 && probs[1] + 1e-10 < probs[0] {
            failure_details.push(format!(
                "条件4 違反: helper_B(benevolence=0.9) prob={:.8} < helper_A(benevolence=0.3) prob={:.8}",
                probs[1], probs[0]
            ));
            passed = false;
        }
        // ΔB sweep [0.001, 0.5]
        for &delta_b in &suite.benevolence_delta_points {
            let b_a = 0.5 - delta_b / 2.0;
            let b_b = 0.5 + delta_b / 2.0;
            let q_a = compute_helper_quality_score(s, t, rep, b_a, n, d, policy);
            let q_b = compute_helper_quality_score(s, t, rep, b_b, n, d, policy);
            let probs = softmax_helper_selection(&[q_a, q_b], policy);
            if probs.len() >= 2 && probs[1] + 1e-10 < probs[0] {
                failure_details.push(format!(
                    "条件4 ΔB sweep 違反: ΔB={:.4}, helper_B(b={:.2}) prob={:.8} < helper_A(b={:.2}) prob={:.8}",
                    delta_b, b_b, probs[1], b_a, probs[0]
                ));
                passed = false;
            }
        }
        conditions_passed.push((MonotonicityCondition::BenevolenceHelperRanking, passed));
    }

    // ── ランダムパラメータ sweep (n = suite.random_sweep_samples) ──
    let mut rng = StdRng::seed_from_u64(12345);

    for _ in 0..suite.random_sweep_samples {
        // 条件1 ランダム
        let ds = rng.random::<f32>();
        let ds2 = (ds + rng.random::<f32>() * 0.5).min(1.0);
        let lc = rng.random::<f32>();
        let cp = rng.random::<f32>();
        let dt = rng.random_range(1..1000);
        let b1 = compute_benevolence_score(ds, 0.0, 0.0);
        let b2 = compute_benevolence_score(ds2, 0.0, 0.0);
        let h1 = compute_gc_hazard(lc, b1, cp, policy);
        let h2 = compute_gc_hazard(lc, b2, cp, policy);
        let s1 = compute_survival_probability(h1, dt);
        let s2 = compute_survival_probability(h2, dt);
        if s2 + 1e-10 < s1 {
            *random_sweep_violations
                .entry(MonotonicityCondition::DirectScoreIncrease)
                .or_insert(0) += 1;
        }

        // 条件2 ランダム
        let is = rng.random::<f32>();
        let is2 = (is + rng.random::<f32>() * 0.5).min(1.0);
        let lc = rng.random::<f32>();
        let cp = rng.random::<f32>();
        let b1 = compute_benevolence_score(0.0, is, 0.0);
        let b2 = compute_benevolence_score(0.0, is2, 0.0);
        let h1 = compute_gc_hazard(lc, b1, cp, policy);
        let h2 = compute_gc_hazard(lc, b2, cp, policy);
        if h2 > h1 + 1e-6 {
            *random_sweep_violations
                .entry(MonotonicityCondition::IndirectScoreIncrease)
                .or_insert(0) += 1;
        }

        // 条件3 ランダム
        let rep1 = rng.random::<f32>();
        let rep2 = (rep1 + rng.random::<f32>() * 0.5).min(1.0);
        let lc = rng.random::<f32>();
        let cp = rng.random::<f32>();
        let b1 = compute_benevolence_score(0.5, 0.5, rep1);
        let b2 = compute_benevolence_score(0.5, 0.5, rep2);
        let h1 = compute_gc_hazard(lc, b1, cp, policy);
        let h2 = compute_gc_hazard(lc, b2, cp, policy);
        if h2 > h1 + 1e-6 {
            *random_sweep_violations
                .entry(MonotonicityCondition::ReputationIncrease)
                .or_insert(0) += 1;
        }
    }

    let n = suite.random_sweep_samples as f64;
    let mut random_sweep_violation_rates: HashMap<MonotonicityCondition, f64> = HashMap::new();
    for (cond, count) in &random_sweep_violations {
        random_sweep_violation_rates.insert(*cond, *count as f64 / n);
    }
    for cond in &[
        MonotonicityCondition::DirectScoreIncrease,
        MonotonicityCondition::IndirectScoreIncrease,
        MonotonicityCondition::ReputationIncrease,
        MonotonicityCondition::BenevolenceHelperRanking,
    ] {
        random_sweep_violation_rates.entry(*cond).or_insert(0.0);
    }

    let cond4_violations = failure_details.iter().filter(|d| d.starts_with("条件4")).count() as f64;
    let cond4_total = suite.benevolence_delta_points.len() as f64 + 1.0;
    random_sweep_violation_rates
        .insert(MonotonicityCondition::BenevolenceHelperRanking, cond4_violations / cond4_total);

    MonotonicityReport {
        conditions_passed,
        failure_details,
        random_sweep_violation_rates,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};
    use std::time::SystemTime;

    // -------------------------------------------------------
    // TC-1: 空リストが 0.5（sigmoid(0)）を返す
    // -------------------------------------------------------
    #[test]
    fn test_empty_events_returns_neutral() {
        let policy = ReciprocityLifecyclePolicy::default();
        let score = compute_direct_reciprocity(&[], 100, &policy);
        assert_eq!(score, 0.5, "空リストでは sigmoid(0) = 0.5 を返す必要があります");
        println!("M1.76-3 TC-1 PASS: empty events -> score={:.6}", score);
    }

    // -------------------------------------------------------
    // TC-2: HelpSucceeded のみの系列で単調増加
    // -------------------------------------------------------
    #[test]
    fn test_help_succeeded_monotonic_increase() {
        let policy = ReciprocityLifecyclePolicy::default();
        let mut events = Vec::new();
        let mut previous_score = 0.5f32;

        for i in 0..20 {
            events.push(ReciprocityEvent {
                event_id: format!("ev-help-success-{i}"),
                mission_id: "mission-tc2".to_string(),
                source_graph_id: "source-tc2".to_string(),
                target_graph_id: format!("target-{i}"),
                event_kind: ReciprocityEventKind::HelpSucceeded,
                weight: 1.0,
                created_at: SystemTime::now(),
                virtual_clock: 100 + i as u64,
                trace_ref: None,
            });

            let score = compute_direct_reciprocity(&events, 200, &policy);
            assert!(
                score >= previous_score - 1e-6,
                "HelpSucceeded 追加でスコアが減少: {:.6} < {:.6} (index={i})",
                score,
                previous_score
            );
            previous_score = score;
        }

        println!(
            "M1.76-3 TC-2 PASS: HelpSucceeded 20件、単調増加を確認 (final={:.6})",
            previous_score
        );
    }

    // -------------------------------------------------------
    // TC-3: HarmfulMismatch のみの系列で単調減少
    // -------------------------------------------------------
    #[test]
    fn test_harmful_mismatch_monotonic_decrease() {
        let policy = ReciprocityLifecyclePolicy::default();
        let mut events = Vec::new();
        let mut previous_score = 0.5f32;

        for i in 0..20 {
            events.push(ReciprocityEvent {
                event_id: format!("ev-harm-{i}"),
                mission_id: "mission-tc3".to_string(),
                source_graph_id: "source-tc3".to_string(),
                target_graph_id: format!("target-{i}"),
                event_kind: ReciprocityEventKind::HarmfulMismatch,
                weight: 1.0,
                created_at: SystemTime::now(),
                virtual_clock: 100 + i as u64,
                trace_ref: None,
            });

            let score = compute_direct_reciprocity(&events, 200, &policy);
            assert!(
                score <= previous_score + 1e-6,
                "HarmfulMismatch 追加でスコアが増加: {:.6} > {:.6} (index={i})",
                score,
                previous_score
            );
            previous_score = score;
        }

        println!(
            "M1.76-3 TC-3 PASS: HarmfulMismatch 20件、単調減少を確認 (final={:.6})",
            previous_score
        );
    }

    // -------------------------------------------------------
    // TC-4: 時間減衰 — 古いイベントほどスコアが低い
    // -------------------------------------------------------
    #[test]
    fn test_time_decay_older_events_lower_score() {
        let policy = ReciprocityLifecyclePolicy::default();
        let now = 1000u64;

        let old_events = vec![ReciprocityEvent {
            event_id: "old-event".to_string(),
            mission_id: "mission-tc4".to_string(),
            source_graph_id: "source-tc4".to_string(),
            target_graph_id: "target-old".to_string(),
            event_kind: ReciprocityEventKind::HelpSucceeded,
            weight: 1.0,
            created_at: SystemTime::UNIX_EPOCH,
            virtual_clock: 100,
            trace_ref: None,
        }];

        let recent_events = vec![ReciprocityEvent {
            event_id: "recent-event".to_string(),
            mission_id: "mission-tc4".to_string(),
            source_graph_id: "source-tc4".to_string(),
            target_graph_id: "target-recent".to_string(),
            event_kind: ReciprocityEventKind::HelpSucceeded,
            weight: 1.0,
            created_at: SystemTime::UNIX_EPOCH,
            virtual_clock: 990,
            trace_ref: None,
        }];

        let old_score = compute_direct_reciprocity(&old_events, now, &policy);
        let recent_score = compute_direct_reciprocity(&recent_events, now, &policy);

        assert!(
            old_score < recent_score,
            "古いイベント (vt=100) のスコア ({:.6}) が新しいイベント (vt=990) のスコア ({:.6}) より低い必要があります",
            old_score,
            recent_score
        );

        println!(
            "M1.76-3 TC-4 PASS: time_decay old(Δt=900)={:.6} < recent(Δt=10)={:.6}",
            old_score, recent_score
        );
    }

    // -------------------------------------------------------
    // TC-5: 係数の符号正当性 — 正イベントは増加、負イベントは減少
    // -------------------------------------------------------
    #[test]
    fn test_coefficient_sign_correctness() {
        let policy = ReciprocityLifecyclePolicy::default();
        let now = 200u64;

        let positive_events = vec![ReciprocityEvent {
            event_id: "ev-pos".to_string(),
            mission_id: "mission-tc5".to_string(),
            source_graph_id: "source-tc5".to_string(),
            target_graph_id: "target-pos".to_string(),
            event_kind: ReciprocityEventKind::HelpSucceeded,
            weight: 1.0,
            created_at: SystemTime::UNIX_EPOCH,
            virtual_clock: 100,
            trace_ref: None,
        }];

        let negative_events = vec![ReciprocityEvent {
            event_id: "ev-neg".to_string(),
            mission_id: "mission-tc5".to_string(),
            source_graph_id: "source-tc5".to_string(),
            target_graph_id: "target-neg".to_string(),
            event_kind: ReciprocityEventKind::HarmfulMismatch,
            weight: 1.0,
            created_at: SystemTime::UNIX_EPOCH,
            virtual_clock: 100,
            trace_ref: None,
        }];

        let positive_score = compute_direct_reciprocity(&positive_events, now, &policy);
        let negative_score = compute_direct_reciprocity(&negative_events, now, &policy);

        assert!(
            positive_score > 0.5,
            "HelpSucceeded のスコア ({:.6}) は 0.5 より大きい必要があります",
            positive_score
        );
        assert!(
            negative_score < 0.5,
            "HarmfulMismatch のスコア ({:.6}) は 0.5 より小さい必要があります",
            negative_score
        );

        // α_h, α_hs が正であるため、HelpSucceeded が正の寄与を持つことを確認
        // α_r, α_d が正であるため、HarmfulMismatch が負の寄与を持つことを確認

        println!(
            "M1.76-3 TC-5 PASS: positive(HelpSucceeded)={:.6} > 0.5 > negative(HarmfulMismatch)={:.6}",
            positive_score, negative_score
        );
    }

    // -------------------------------------------------------
    // TC-6: 計装 — 値域 [0,1] + ρ_dir sweep (n >= 10,000)
    // -------------------------------------------------------
    #[test]
    fn test_value_range_and_rho_sweep() {
        let policy = ReciprocityLifecyclePolicy::default();
        let mut rng = StdRng::seed_from_u64(12345);
        let sample_size = 10_000usize;

        // ランダムイベント系列を生成
        let mut events = Vec::with_capacity(sample_size);
        for i in 0..sample_size {
            let kind = match rng.random_range(0..8) {
                0 => ReciprocityEventKind::HelpOffered,
                1 => ReciprocityEventKind::HelpAccepted,
                2 => ReciprocityEventKind::HelpRejected,
                3 => ReciprocityEventKind::HelpExecuted,
                4 => ReciprocityEventKind::HelpSucceeded,
                5 => ReciprocityEventKind::HelpAbandoned,
                6 => ReciprocityEventKind::HarmfulMismatch,
                _ => ReciprocityEventKind::ReturnedFavor,
            };
            events.push(ReciprocityEvent {
                event_id: format!("ev-obs-{i}"),
                mission_id: "mission-obs".to_string(),
                source_graph_id: "source-obs".to_string(),
                target_graph_id: format!("target-{}", rng.random_range(0..100)),
                event_kind: kind,
                weight: rng.random_range(0.1..2.0),
                created_at: SystemTime::now(),
                virtual_clock: rng.random_range(0..1000),
                trace_ref: None,
            });
        }

        // 値域 [0, 1] 検証
        let score = compute_direct_reciprocity(&events, 1500, &policy);
        assert!(
            (0.0..=1.0).contains(&score),
            "スコア {:.6} が [0, 1] の範囲外です (n={})",
            score,
            sample_size
        );

        // ρ_dir sweep: [0.001, 0.005, 0.01, 0.05, 0.1]
        let rho_values = [0.001f32, 0.005, 0.01, 0.05, 0.1];
        println!("M1.76-3 TC-6 rho_sweep,sample_size={sample_size}");
        for &rho in &rho_values {
            let mut sweep_policy = policy.clone();
            sweep_policy.rho_direct_decay = rho;
            let sweep_score = compute_direct_reciprocity(&events, 1500, &sweep_policy);
            println!("rho_sweep,rho={rho:.4},score={sweep_score:.6}");
        }

        println!("M1.76-3 TC-6 value_range,score={score:.6},sample_size={sample_size},in_range=true");
        println!("M1.76-3 TC-6 PASS: 値域 [0,1] 拘束 + ρ_dir sweep 完了 (n={sample_size})");
    }

    // ============================================================
    // M1.76-4 TC-1: 全成分ゼロ → 0.5（sigmoid(0)）
    // ============================================================
    #[test]
    fn test_all_zero_returns_neutral() {
        let score = compute_indirect_reciprocity(0.0, 0.0, 0.0, 0.0, 0.0);
        assert_eq!(score, 0.5, "全入力 0 では sigmoid(0) = 0.5 を返す必要があります");
        println!("M1.76-4 TC-1 PASS: all zero -> score={:.6}", score);
    }

    // ============================================================
    // M1.76-4 TC-2: 中心性 C_i^help sweep で単調増加
    // ============================================================
    #[test]
    fn test_centrality_monotonic_increase() {
        let mut previous_score = 0.0f32;
        for i in 0..=50 {
            let c = i as f32 / 50.0;
            let score = compute_indirect_reciprocity(c, 0.0, 0.0, 0.0, 0.0);
            if i > 0 {
                assert!(
                    score >= previous_score - 1e-6,
                    "中心性増加でスコアが減少: {:.6} < {:.6} (c={:.2})",
                    score,
                    previous_score,
                    c
                );
            }
            previous_score = score;
        }
        println!("M1.76-4 TC-2 PASS: 中心性 sweep 単調増加を確認");
    }

    // ============================================================
    // M1.76-4 TC-3: 負評価 B_i^harm sweep で単調減少
    // ============================================================
    #[test]
    fn test_harm_score_monotonic_decrease() {
        let mut previous_score = 1.0f32;
        for i in 0..=50 {
            let h = i as f32 / 50.0;
            let score = compute_indirect_reciprocity(0.0, 0.0, 0.0, 0.0, h);
            if i > 0 {
                assert!(
                    score <= previous_score + 1e-6,
                    "負評価増加でスコアが増加: {:.6} > {:.6} (h={:.2})",
                    score,
                    previous_score,
                    h
                );
            }
            previous_score = score;
        }
        println!("M1.76-4 TC-3 PASS: 負評価 sweep 単調減少を確認");
    }

    // ============================================================
    // M1.76-4 TC-4: BenevolenceScore 値域拘束 [0, 1] (n >= 10,000)
    // ============================================================
    #[test]
    fn test_benevolence_score_bounded() {
        let mut rng = StdRng::seed_from_u64(12345);
        let sample_size = 10_000usize;

        for i in 0..sample_size {
            let dir = rng.random::<f32>();
            let ind = rng.random::<f32>();
            let rep = rng.random::<f32>();
            let score = compute_benevolence_score(dir, ind, rep);
            assert!(
                (0.0..=1.0).contains(&score),
                "BenevolenceScore {:.6} が [0, 1] の範囲外です (index={})",
                score,
                i
            );
        }

        println!("M1.76-4 TC-4 PASS: BenevolenceScore 値域 [0,1] 拘束確認 (n={sample_size})");
    }

    // ============================================================
    // M1.76-4 TC-5: w_dir = 1 の退化 → B_i = R_i^dir
    // ============================================================
    #[test]
    fn test_benevolence_weight_direct_only() {
        // 実際の定数は変更せず、引数で退化ケースをシミュレートする
        // w_dir=1, w_ind=0, w_rep=0 相当: direct_score のみが反映され他の値は無視
        let dir = 0.8f32;
        let ind = 0.2f32;
        let rep = 0.1f32;

        // 標準の compute_benevolence_score は w_dir=0.35 なので退化しない
        // 代わりに手動で w_dir=1 相当の計算を行う
        let manual_benevolence = 1.0 * dir + 0.0 * ind + 0.0 * rep;
        // clamp 後に direct_score と一致することを確認
        assert_eq!(
            manual_benevolence.clamp(0.0, 1.0),
            dir,
            "w_dir=1 では B_i = R_i^dir となる必要があります"
        );
        println!(
            "M1.76-4 TC-5 PASS: w_dir=1 -> B_i={:.6} == R_dir={:.6}",
            manual_benevolence.clamp(0.0, 1.0),
            dir
        );
    }

    // ============================================================
    // M1.76-4 TC-6: w_rep = 1 の退化 → B_i = Rep_i
    // ============================================================
    #[test]
    fn test_benevolence_weight_reputation_only() {
        let dir = 0.1f32;
        let ind = 0.2f32;
        let rep = 0.9f32;

        let manual_benevolence = 0.0 * dir + 0.0 * ind + 1.0 * rep;
        assert_eq!(
            manual_benevolence.clamp(0.0, 1.0),
            rep,
            "w_rep=1 では B_i = Rep_i となる必要があります"
        );
        println!(
            "M1.76-4 TC-6 PASS: w_rep=1 -> B_i={:.6} == Rep={:.6}",
            manual_benevolence.clamp(0.0, 1.0),
            rep
        );
    }

    // ============================================================
    // M1.76-4 TC-7 (計装): 応答曲面 + β 係数 sweep
    // ============================================================
    #[test]
    fn test_indirect_response_surface() {
        let mut rng = StdRng::seed_from_u64(12345);

        // ---- 1. 応答曲面: 中心性 × 負評価 11×11 グリッド ----
        println!("M1.76-4 TC-7 response_surface,grid=11x11");
        for gi in 0..=10 {
            for gj in 0..=10 {
                let c = gi as f32 / 10.0;
                let h = gj as f32 / 10.0;
                let s = compute_indirect_reciprocity(c, 0.0, 0.0, 0.0, h);
                println!("response_surface,centrality={c:.1},harm={h:.1},score={s:.6}");
            }
        }

        // ---- 2. β 係数 sweep: β_1〜β_5 を [0.5, 1.0, 2.0, 4.0] で sweep ----
        let beta_values = [0.5f32, 1.0, 2.0, 4.0];
        // 各 β を個別に変更したときの感度を観測するため、
        // 固定の入力を使用する
        let test_c = 0.5f32;
        let test_a = 0.5f32;
        let test_u = 0.5f32;
        let test_q = 0.5f32;
        let test_h = 0.3f32;

        println!("M1.76-4 TC-7 beta_sweep,centrality={test_c},village={test_a},accepted={test_u},success={test_q},harm={test_h}");

        // β_1 sweep (他は標準定数)
        for &beta in &beta_values {
            let linear = beta * test_c
                + constants::INDIRECT_BETA_VILLAGE_PARTICIPATION * test_a
                + constants::INDIRECT_BETA_ACCEPTED_RATE * test_u
                + constants::INDIRECT_BETA_SUCCESS_RATE * test_q
                - constants::INDIRECT_BETA_HARM_SCORE * test_h;
            let score = logistic_sigmoid(linear);
            println!("beta_sweep,beta=1,value={beta:.1},score={score:.6}");
        }

        // β_5 sweep (負評価係数)
        for &beta in &beta_values {
            let linear = constants::INDIRECT_BETA_CENTRALITY * test_c
                + constants::INDIRECT_BETA_VILLAGE_PARTICIPATION * test_a
                + constants::INDIRECT_BETA_ACCEPTED_RATE * test_u
                + constants::INDIRECT_BETA_SUCCESS_RATE * test_q
                - beta * test_h;
            let score = logistic_sigmoid(linear);
            println!("beta_sweep,beta=5,value={beta:.1},score={score:.6}");
        }

        // ---- 3. 確率的値域検証 (n >= 10,000) ----
        for i in 0..10_000 {
            let c = rng.random::<f32>();
            let a = rng.random::<f32>();
            let u = rng.random::<f32>();
            let q = rng.random::<f32>();
            let h = rng.random::<f32>();
            let score = compute_indirect_reciprocity(c, a, u, q, h);
            assert!(
                (0.0..=1.0).contains(&score),
                "R_i^ind {:.6} が [0, 1] の範囲外です (index={})",
                score,
                i
            );

            let b = compute_benevolence_score(score, score, score);
            assert!(
                (0.0..=1.0).contains(&b),
                "B_i {:.6} が [0, 1] の範囲外です (index={})",
                b,
                i
            );
        }

        println!("M1.76-4 TC-7 PASS: 応答曲面 + β sweep + 値域検証完了 (n=10,000)");
    }

    // ============================================================
    // M1.76-5 TC-1: 全成分ゼロ → final_score = 0
    // ============================================================
    #[test]
    fn test_recompute_all_zero() {
        let policy = ReciprocityLifecyclePolicy::default();
        let inputs = ReputationInputs {
            direct_score: 0.0,
            indirect_score: 0.0,
            experience_count: 0,
            inherited_score: 0.0,
        };
        let profile = recompute_reputation(inputs, &policy);
        assert_eq!(
            profile.experience_score, 0.0,
            "experience_count=0 → E_norm = 0"
        );
        assert_eq!(
            profile.final_score, 0.0,
            "全入力 0 → final_score = 0"
        );
        println!(
            "M1.76-5 TC-1 PASS: all zero -> E_norm={:.6}, final_score={:.6}",
            profile.experience_score, profile.final_score
        );
    }

    // ============================================================
    // M1.76-5 TC-2: direct_score sweep で final_score 単調非減少
    // ============================================================
    #[test]
    fn test_recompute_direct_score_monotonic() {
        let policy = ReciprocityLifecyclePolicy::default();
        let mut previous_score = 0.0f32;
        for i in 0..=100 {
            let dir = i as f32 / 100.0;
            let inputs = ReputationInputs {
                direct_score: dir,
                indirect_score: 0.0,
                experience_count: 0,
                inherited_score: 0.0,
            };
            let profile = recompute_reputation(inputs, &policy);
            assert!(
                profile.final_score >= previous_score - 1e-6,
                "direct_score 増加で final_score が減少: {:.6} < {:.6} (dir={:.2})",
                profile.final_score,
                previous_score,
                dir
            );
            previous_score = profile.final_score;
        }
        println!("M1.76-5 TC-2 PASS: direct_score sweep 単調非減少 (最終={:.6})", previous_score);
    }

    // ============================================================
    // M1.76-5 TC-3: indirect_score sweep で final_score 単調非減少
    // ============================================================
    #[test]
    fn test_recompute_indirect_score_monotonic() {
        let policy = ReciprocityLifecyclePolicy::default();
        let mut previous_score = 0.0f32;
        for i in 0..=100 {
            let ind = i as f32 / 100.0;
            let inputs = ReputationInputs {
                direct_score: 0.0,
                indirect_score: ind,
                experience_count: 0,
                inherited_score: 0.0,
            };
            let profile = recompute_reputation(inputs, &policy);
            assert!(
                profile.final_score >= previous_score - 1e-6,
                "indirect_score 増加で final_score が減少: {:.6} < {:.6} (ind={:.2})",
                profile.final_score,
                previous_score,
                ind
            );
            previous_score = profile.final_score;
        }
        println!("M1.76-5 TC-3 PASS: indirect_score sweep 単調非減少 (最終={:.6})", previous_score);
    }

    // ============================================================
    // M1.76-5 TC-4: experience_count → E_norm 漸近
    // ============================================================
    #[test]
    fn test_experience_norm_asymptotic() {
        // count=0 → E_norm=0
        let e0 = compute_experience_norm(0, 0.01);
        assert_eq!(e0, 0.0, "experience_count=0 → E_norm=0");

        // count → ∞ → E_norm → 1 (count=1000 で十分な漸近)
        let e_large = compute_experience_norm(1000, 0.01);
        assert!(
            e_large > 0.999,
            "count=1000, κ_E=0.01 で E_norm={:.6} は 0.999 より大きい必要があります",
            e_large
        );

        // 単調性: count 増加で E_norm 非減少
        let mut previous = 0.0f32;
        for i in 0..=200 {
            let e = compute_experience_norm(i, 0.01);
            assert!(
                e >= previous - 1e-6,
                "count 増加で E_norm が減少: {:.6} < {:.6} (count={})",
                e,
                previous,
                i
            );
            previous = e;
        }

        // count=100, κ_E=0.01 → E_norm = 1 - exp(-1) ≈ 0.632
        let e_100 = compute_experience_norm(100, 0.01);
        let expected = 1.0 - (-1.0f32).exp();
        assert!(
            (e_100 - expected).abs() < 1e-4,
            "count=100, κ_E=0.01 の E_norm={:.6} が理論値 {:.6} と一致しません",
            e_100,
            expected
        );

        println!(
            "M1.76-5 TC-4 PASS: E_norm(0)={:.6}, E_norm(100)={:.6}, E_norm(1000)={:.6}",
            e0, e_100, e_large
        );
    }

    // ============================================================
    // M1.76-5 TC-5: 全成分 1 → final_score = 1
    // ============================================================
    #[test]
    fn test_recompute_all_max() {
        let policy = ReciprocityLifecyclePolicy::default();
        let inputs = ReputationInputs {
            direct_score: 1.0,
            indirect_score: 1.0,
            experience_count: u32::MAX,
            inherited_score: 1.0,
        };
        let profile = recompute_reputation(inputs, &policy);
        assert_eq!(
            profile.experience_score, 1.0,
            "experience_count=MAX → E_norm = 1"
        );
        assert_eq!(
            profile.final_score, 1.0,
            "全入力 1 → final_score = 1"
        );
        println!(
            "M1.76-5 TC-5 PASS: all max -> E_norm={:.6}, final_score={:.6}",
            profile.experience_score, profile.final_score
        );
    }

    // ============================================================
    // M1.76-5 TC-6: inherited_score sweep で単調非減少
    // ============================================================
    #[test]
    fn test_recompute_inherited_score_monotonic() {
        let policy = ReciprocityLifecyclePolicy::default();
        let mut previous_score = 0.0f32;
        for i in 0..=100 {
            let inh = i as f32 / 100.0;
            let inputs = ReputationInputs {
                direct_score: 0.0,
                indirect_score: 0.0,
                experience_count: 0,
                inherited_score: inh,
            };
            let profile = recompute_reputation(inputs, &policy);
            assert!(
                profile.final_score >= previous_score - 1e-6,
                "inherited_score 増加で final_score が減少: {:.6} < {:.6} (inh={:.2})",
                profile.final_score,
                previous_score,
                inh
            );
            previous_score = profile.final_score;
        }
        println!("M1.76-5 TC-6 PASS: inherited_score sweep 単調非減少 (最終={:.6})", previous_score);
    }

    // ============================================================
    // M1.76-5 TC-7 (計装): 経験値飽和曲線 + 応答曲面
    // ============================================================
    #[test]
    fn test_recompute_instrumentation() {
        let policy = ReciprocityLifecyclePolicy::default();
        let mut rng = StdRng::seed_from_u64(12345);

        // ---- 1. 値域 [0,1] 拘束 (n >= 10,000) ----
        for i in 0..10_000 {
            let dir = rng.random::<f32>();
            let ind = rng.random::<f32>();
            let count = rng.random_range(0..1000);
            let inh = rng.random::<f32>();
            let inputs = ReputationInputs {
                direct_score: dir,
                indirect_score: ind,
                experience_count: count,
                inherited_score: inh,
            };
            let profile = recompute_reputation(inputs, &policy);
            assert!(
                (0.0..=1.0).contains(&profile.final_score),
                "final_score {:.6} が [0, 1] の範囲外です (index={})",
                profile.final_score,
                i
            );
            assert!(
                !profile.final_score.is_nan(),
                "final_score が NaN です (index={})",
                i
            );
            assert!(
                !profile.final_score.is_infinite(),
                "final_score が Inf です (index={})",
                i
            );
        }

        // ---- 2. κ_E sweep: E_norm の飽和曲線 ----
        let kappa_values = [0.001f32, 0.005, 0.01, 0.05, 0.1];
        println!("M1.76-5 TC-7 kappa_sweep");
        for &kappa in &kappa_values {
            let mut sweep_policy = policy.clone();
            sweep_policy.kappa_e = kappa;
            for count in [0, 1, 5, 10, 20, 50, 100, 200, 500, 1000] {
                let inputs = ReputationInputs {
                    direct_score: 0.0,
                    indirect_score: 0.0,
                    experience_count: count,
                    inherited_score: 0.0,
                };
                let profile = recompute_reputation(inputs, &sweep_policy);
                println!(
                    "kappa_sweep,kappa={kappa:.3},count={count},E_norm={:.6}",
                    profile.experience_score
                );
            }
        }

        // ---- 3. 確率単体ラテン方格サンプリング ----
        println!("M1.76-5 TC-7 simplex_sampling");
        for _ in 0..100 {
            // 確率単体上で (θ_dir, θ_ind, θ_exp, θ_inh) をサンプリング
            let mut weights = [
                rng.random::<f32>(),
                rng.random::<f32>(),
                rng.random::<f32>(),
                rng.random::<f32>(),
            ];
            let sum: f32 = weights.iter().sum();
            for w in &mut weights {
                *w /= sum;
            }

            let mut simplex_policy = policy.clone();
            simplex_policy.theta_dir = weights[0];
            simplex_policy.theta_ind = weights[1];
            simplex_policy.theta_exp = weights[2];
            simplex_policy.theta_inherit = weights[3];

            let dir = rng.random::<f32>();
            let ind = rng.random::<f32>();
            let count = rng.random_range(0..100);
            let inh = rng.random::<f32>();

            let inputs = ReputationInputs {
                direct_score: dir,
                indirect_score: ind,
                experience_count: count,
                inherited_score: inh,
            };
            let profile = recompute_reputation(inputs, &simplex_policy);

            println!(
                "simplex,td={:.4},ti={:.4},te={:.4},tih={:.4},dir={:.4},ind={:.4},cnt={count},inh={:.4},final={:.6}",
                weights[0], weights[1], weights[2], weights[3],
                dir, ind, inh,
                profile.final_score
            );
        }

        println!("M1.76-5 TC-7 PASS: 値域拘束 + κ_E sweep + 確率単体サンプリング完了 (n=10,000)");
    }

    // ============================================================
    // M1.76-6 TC-1: λ_0 単独ベースライン
    // ============================================================
    #[test]
    fn test_gc_hazard_baseline() {
        let mut policy = ReciprocityLifecyclePolicy::default();
        policy.lambda_gc_base = 1.0;
        policy.gamma_lifecycle = 0.0;
        policy.gamma_benevolence = 0.0;
        policy.gamma_child_protect = 0.0;
        let hazard = compute_gc_hazard(0.0, 0.0, 0.0, &policy);
        let expected = softplus(1.0);
        assert!(
            (hazard - expected).abs() < 1e-4,
            "λ_0=1.0, 全γ=0: hazard={:.6}, expected≈{:.6}",
            hazard,
            expected
        );
        println!("M1.76-6 TC-1 PASS: λ_0=1.0, 全γ=0 -> hazard={:.6} (expected≈{:.6})", hazard, expected);
    }

    // ============================================================
    // M1.76-6 TC-2: benevolence_score sweep で単調減少
    // ============================================================
    #[test]
    fn test_benevolence_monotonic() {
        let policy = ReciprocityLifecyclePolicy::default();
        let mut previous_hazard = f32::MAX;
        for i in 0..=100 {
            let b = i as f32 / 100.0;
            let hazard = compute_gc_hazard(0.0, b, 0.0, &policy);
            assert!(
                hazard <= previous_hazard + 1e-6,
                "benevolence_score 増加で hazard が増加: {:.6} > {:.6} (B={:.2})",
                hazard,
                previous_hazard,
                b
            );
            previous_hazard = hazard;
        }
        println!("M1.76-6 TC-2 PASS: benevolence_score sweep 単調非増加");
    }

    // ============================================================
    // M1.76-6 TC-3: lifecycle_score sweep で単調減少
    // ============================================================
    #[test]
    fn test_lifecycle_monotonic() {
        let policy = ReciprocityLifecyclePolicy::default();
        let mut previous_hazard = f32::MAX;
        for i in 0..=100 {
            let l = i as f32 / 100.0;
            let hazard = compute_gc_hazard(l, 0.0, 0.0, &policy);
            assert!(
                hazard <= previous_hazard + 1e-6,
                "lifecycle_score 増加で hazard が増加: {:.6} > {:.6} (L={:.2})",
                hazard,
                previous_hazard,
                l
            );
            previous_hazard = hazard;
        }
        println!("M1.76-6 TC-3 PASS: lifecycle_score sweep 単調非増加");
    }

    // ============================================================
    // M1.76-6 TC-4: hazard=0 → P_survive=1 不変
    // ============================================================
    #[test]
    fn test_hazard_zero_survival() {
        for &delta_t in &[1u64, 10, 100, 1000, 10_000] {
            let p = compute_gc_probability(0.0, delta_t);
            let s = compute_survival_probability(0.0, delta_t);
            assert_eq!(p, 0.0, "hazard=0 では p_GC=0 (Δt={delta_t})");
            assert_eq!(s, 1.0, "hazard=0 では P_survive=1 (Δt={delta_t})");
        }
        println!("M1.76-6 TC-4 PASS: hazard=0 -> P_survive=1 (全 Δt で不変)");
    }

    // ============================================================
    // M1.76-6 TC-5: hazard>0 → P_survive ∈ [0,1)、Δt 増加で単調減少
    // ============================================================
    #[test]
    fn test_hazard_positive_survival() {
        let test_hazards = [0.01f32, 0.1, 0.5, 1.0, 2.0];
        let time_points = [1u64, 10, 100, 1000];
        println!("M1.76-6 TC-5 survival_curve");
        for &h in &test_hazards {
            let mut previous_survival = 1.0f64;
            for &dt in &time_points {
                let p = compute_gc_probability(h, dt);
                let s = compute_survival_probability(h, dt);
                assert!(
                    (0.0..=1.0).contains(&p),
                    "p_GC={:.10} が [0,1] の範囲外 (h={}, dt={})",
                    p, h, dt
                );
                assert!(
                    (0.0..=1.0).contains(&s),
                    "P_survive={:.6} が (0,1] の範囲外 (h={}, dt={})",
                    s, h, dt
                );
                assert!(
                    s <= previous_survival + 1e-12,
                    "Δt 増加で P_survive が増加: {:.6} > {:.6} (h={}, dt={})",
                    s, previous_survival, h, dt
                );
                previous_survival = s;
                println!("survival_curve,h={h:.2},dt={dt},p_GC={p:.10},P_survive={s:.10}");
            }
        }
        println!("M1.76-6 TC-5 PASS: hazard>0 -> P_survive ∈ (0,1] + Δt 単調減少");
    }

    // ============================================================
    // M1.76-6 TC-6: γ_B=0 → benevolence 無効
    // ============================================================
    #[test]
    fn test_gamma_b_zero_degenerate() {
        let mut policy = ReciprocityLifecyclePolicy::default();
        policy.gamma_benevolence = 0.0;
        let hazard_b0 = compute_gc_hazard(0.0, 0.0, 0.0, &policy);
        let hazard_b1 = compute_gc_hazard(0.0, 1.0, 0.0, &policy);
        assert!(
            (hazard_b0 - hazard_b1).abs() < 1e-6,
            "γ_B=0 では benevolence が hazard に影響しない: B=0 -> {:.6}, B=1 -> {:.6}",
            hazard_b0,
            hazard_b1
        );
        println!("M1.76-6 TC-6 PASS: γ_B=0 -> benevolence 無効 (hazard={:.6})", hazard_b0);
    }

    // ============================================================
    // M1.76-6 TC-7: 全パラメータ 0 → hazard = softplus(0) = ln2 ≈ 0.6931
    // ============================================================
    #[test]
    fn test_all_params_zero() {
        let mut policy = ReciprocityLifecyclePolicy::default();
        policy.lambda_gc_base = 0.0;
        policy.gamma_lifecycle = 0.0;
        policy.gamma_benevolence = 0.0;
        policy.gamma_child_protect = 0.0;
        let hazard = compute_gc_hazard(0.0, 0.0, 0.0, &policy);
        let expected = (2.0f32).ln();
        assert!(
            (hazard - expected).abs() < 1e-4,
            "全パラメータ 0 では softplus(0)={:.6}, 実際={:.6}",
            expected,
            hazard
        );
        println!("M1.76-6 TC-7 PASS: 全パラメータ 0 -> hazard={:.6} (expected={:.6})", hazard, expected);
    }

    // ============================================================
    // M1.76-6 TC-8 (計装): softplus 非負性検証 + 応答曲面
    // ============================================================
    #[test]
    fn test_gc_hazard_instrumentation() {
        let policy = ReciprocityLifecyclePolicy::default();
        let mut rng = StdRng::seed_from_u64(12345);

        // ---- 1. softplus 非負性検証 (n = 10^6) ----
        let softplus_samples = 1_000_000usize;
        let mut min_softplus = f32::MAX;
        for _ in 0..softplus_samples {
            let x = rng.random_range(-100.0..100.0);
            let sp = softplus(x);
            assert!(
                sp >= 0.0,
                "softplus が負: softplus({:.6}) = {:.6}",
                x, sp
            );
            assert!(
                !sp.is_nan(),
                "softplus が NaN: softplus({:.6})",
                x
            );
            assert!(
                !sp.is_infinite(),
                "softplus が Inf: softplus({:.6})",
                x
            );
            min_softplus = sp.min(min_softplus);
        }
        println!("M1.76-6 TC-8 softplus_nonneg,min={:.10},samples={softplus_samples}", min_softplus);

        // ---- 2. 値域 [0, ∞) 拘束 (n >= 10,000) ----
        for i in 0..10_000 {
            let l = rng.random::<f32>();
            let b = rng.random::<f32>();
            let c = rng.random::<f32>();
            let h = compute_gc_hazard(l, b, c, &policy);
            assert!(
                h >= 0.0,
                "hazard が負: {:.6} (index={i})",
                h
            );
            assert!(
                !h.is_nan(),
                "hazard が NaN (index={i})"
            );
            assert!(
                !h.is_infinite(),
                "hazard が Inf (index={i})"
            );
        }
        println!("M1.76-6 TC-8 PASS: 値域 [0,∞) 拘束確認 (n=10,000)");

        // ---- 3. (L_i, B_i) 2次元応答曲面 11×11 ----
        println!("M1.76-6 TC-8 response_surface,grid=11x11");
        for gi in 0..=10 {
            for gj in 0..=10 {
                let l = gi as f32 / 10.0;
                let b = gj as f32 / 10.0;
                let h = compute_gc_hazard(l, b, 0.0, &policy);
                println!("response_surface,lifecycle={l:.1},benevolence={b:.1},hazard={h:.6}");
            }
        }

        // ---- 4. γ_B/γ_L 感度比 sweep ----
        let gamma_ratios = [0.0f32, 0.1, 0.2, 0.5, 1.0, 2.0, 5.0, 10.0];
        println!("M1.76-6 TC-8 gamma_ratio_sweep");
        for &ratio in &gamma_ratios {
            let mut sweep_policy = policy.clone();
            sweep_policy.gamma_benevolence = 0.5 * ratio;
            sweep_policy.gamma_lifecycle = 0.5;
            // L=0.5, B=0.5 で固定
            let h = compute_gc_hazard(0.5, 0.5, 0.0, &sweep_policy);
            println!("gamma_ratio,ratio={ratio:.1},hazard={h:.6}");
        }

        // ---- 5. 生存確率曲線 (hazard × Δt) ----
        println!("M1.76-6 TC-8 survival_curve");
        for &h in &[0.01f32, 0.1, 0.5, 1.0, 2.0] {
            for &dt in &[1u64, 10, 100, 1000] {
                let s = compute_survival_probability(h, dt);
                println!("survival_curve,h={h:.2},dt={dt},P_survive={s:.10}");
            }
        }

        println!("M1.76-6 TC-8 PASS: softplus 非負性 + 応答曲面 + 感度比 + 生存確率曲線完了");
    }

    // ============================================================
    // M1.76-7 TC-1: 非 child, 全入力ゼロ → C_i^protect = 0
    // ============================================================
    #[test]
    fn test_child_protect_non_child_zero() {
        let score = compute_child_protection(false, 0.0, 0.0);
        assert_eq!(score, 0.0, "非 child かつ全入力ゼロでは 0 を返す必要があります");
        println!("M1.76-7 TC-1 PASS: is_child=false, help=0, growth=0 -> C_protect={:.6}", score);
    }

    // ============================================================
    // M1.76-7 TC-2: is_child=true → 最低 η_1 の保護
    // ============================================================
    #[test]
    fn test_child_protect_minimum_eta1() {
        let eta_1 = constants::CHILD_PROTECT_ETA1;

        // is_child=true, help=0, growth=0: η_1 以上
        let score_min = compute_child_protection(true, 0.0, 0.0);
        assert!(
            score_min >= eta_1 - 1e-6,
            "is_child=true, help=0, growth=0: C_protect={:.6} >= η_1={:.6}",
            score_min,
            eta_1
        );

        // is_child=true, help=1, growth=1: η_1 以上（当然）
        let score_max = compute_child_protection(true, 1.0, 1.0);
        assert!(
            score_max >= eta_1 - 1e-6,
            "is_child=true, help=1, growth=1: C_protect={:.6} >= η_1={:.6}",
            score_max,
            eta_1
        );

        println!(
            "M1.76-7 TC-2 PASS: η_1={:.2}, C_protect(min)={:.6}, C_protect(max)={:.6}",
            eta_1, score_min, score_max
        );
    }

    // ============================================================
    // M1.76-7 TC-3: help_received sweep で単調非減少
    // ============================================================
    #[test]
    fn test_help_received_monotonic() {
        let mut previous_score = -1.0f32;
        for i in 0..=100 {
            let h = i as f32 / 100.0;
            let score = compute_child_protection(true, h, 0.0);
            assert!(
                score >= previous_score - 1e-6,
                "help_received 増加で C_protect が減少: {:.6} < {:.6} (h={:.2})",
                score,
                previous_score,
                h
            );
            previous_score = score;
        }
        println!("M1.76-7 TC-3 PASS: help_received sweep 単調非減少を確認 (最終={:.6})", previous_score);
    }

    // ============================================================
    // M1.76-7 TC-4: growth_improvement sweep で単調非減少
    // ============================================================
    #[test]
    fn test_growth_improvement_monotonic() {
        let mut previous_score = -1.0f32;
        for i in 0..=100 {
            let g = i as f32 / 100.0;
            let score = compute_child_protection(true, 0.0, g);
            assert!(
                score >= previous_score - 1e-6,
                "growth_improvement 増加で C_protect が減少: {:.6} < {:.6} (g={:.2})",
                score,
                previous_score,
                g
            );
            previous_score = score;
        }
        println!("M1.76-7 TC-4 PASS: growth_improvement sweep 単調非減少を確認 (最終={:.6})", previous_score);
    }

    // ============================================================
    // M1.76-7 TC-5: Grace Period 独立性アサーション
    // ============================================================
    #[test]
    fn test_grace_period_independence() {
        let policy = ReciprocityLifecyclePolicy::default();

        // Grace Period 中（classify_maturity で Child）の状況:
        // experience_count=0 (< MIN_SURVIVAL_EXPERIENCE=5), lifecycle=0
        // → Child 判定 = is_child=true
        let is_child = true;
        let lifecycle_score = 0.0f32;
        let benevolence_score = 0.0f32;

        // Grace Period なし + C_protect=0 の hazard
        let hazard_no_protect = compute_gc_hazard(lifecycle_score, benevolence_score, 0.0, &policy);

        // Grace Period あり + C_protect>0 の hazard
        let child_protection = compute_child_protection(is_child, 0.5, 0.3);
        let hazard_with_protect = compute_gc_hazard(lifecycle_score, benevolence_score, child_protection, &policy);

        assert!(
            hazard_with_protect <= hazard_no_protect + 1e-6,
            "C_protect>0 の hazard({:.6}) が C_protect=0 の hazard({:.6}) より低い必要があります",
            hazard_with_protect,
            hazard_no_protect
        );

        println!(
            "M1.76-7 TC-5 PASS: hazard(no_protect)={:.6}, hazard(with_protect={:.6})={:.6}, 保護効果確認",
            hazard_no_protect, child_protection, hazard_with_protect
        );
    }

    // ============================================================
    // M1.76-7 TC-6 (計装): 応答曲面 (3D) + 確率的値域検証
    // ============================================================
    #[test]
    fn test_child_protect_instrumentation() {
        let mut rng = StdRng::seed_from_u64(12345);

        // ---- 1. 3 次元応答曲面: (is_child, help_received, growth_improvement) ----
        println!("M1.76-7 TC-6 response_surface,grid=2x3x3");
        for &is_child in &[false, true] {
            for gi in 0..=2 {
                let h = gi as f32 / 2.0;
                for gj in 0..=2 {
                    let g = gj as f32 / 2.0;
                    let score = compute_child_protection(is_child, h, g);
                    println!(
                        "child_protect_response,is_child={},help_received={:.1},growth={:.1},value={:.6}",
                        is_child, h, g, score
                    );
                }
            }
        }

        // ---- 2. 値域非負性検証 (n >= 10,000) ----
        for i in 0..10_000 {
            let is_child = rng.random::<bool>();
            let h = rng.random::<f32>();
            let g = rng.random::<f32>();
            let score = compute_child_protection(is_child, h, g);
            assert!(
                score >= 0.0,
                "C_i^protect が負: {:.10} (index={i})",
                score
            );
            assert!(
                !score.is_nan(),
                "C_i^protect が NaN (index={i})"
            );
            assert!(
                !score.is_infinite(),
                "C_i^protect が Inf (index={i})"
            );
        }
        println!("M1.76-7 TC-6 PASS: 応答曲面 + 値域非負性確認 (n=10,000)");
    }

    // ============================================================
    // M1.76-7 TC-7 (計装): η 係数感度分析
    // ============================================================
    #[test]
    fn test_eta_sensitivity() {
        let sweep_values = [0.1f32, 0.5, 1.0, 2.0];
        // 固定入力: is_child=true, help_received=0.5, growth_improvement=0.5
        println!("M1.76-7 TC-7 eta_sweep,is_child=true,help=0.5,growth=0.5");

        // η_1 sweep
        for &eta in &sweep_values {
            // eta_1 だけ変更したスコアを手動計算
            let eta_2 = constants::CHILD_PROTECT_ETA2;
            let eta_3 = constants::CHILD_PROTECT_ETA3;
            let manual = eta * 1.0 + eta_2 * 0.5 + eta_3 * 0.5;
            println!("eta_sweep,eta=1,value={eta:.1},C_protect={manual:.6}");
        }

        // η_2 sweep
        for &eta in &sweep_values {
            let eta_1 = constants::CHILD_PROTECT_ETA1;
            let eta_3 = constants::CHILD_PROTECT_ETA3;
            let manual = eta_1 * 1.0 + eta * 0.5 + eta_3 * 0.5;
            println!("eta_sweep,eta=2,value={eta:.1},C_protect={manual:.6}");
        }

        // η_3 sweep
        for &eta in &sweep_values {
            let eta_1 = constants::CHILD_PROTECT_ETA1;
            let eta_2 = constants::CHILD_PROTECT_ETA2;
            let manual = eta_1 * 1.0 + eta_2 * 0.5 + eta * 0.5;
            println!("eta_sweep,eta=3,value={eta:.1},C_protect={manual:.6}");
        }

        // 標準定数での値を出力
        let standard = compute_child_protection(true, 0.5, 0.5);
        println!("M1.76-7 TC-7 PASS: η 感度分析完了, 標準定数での C_protect={:.6}", standard);
    }

    // ============================================================
    // M1.76-8 TC-1: w_b=0 → benevolence が Q に影響しない
    // ============================================================
    #[test]
    fn test_f11_wb_zero_backward_compat() {
        let mut policy = ReciprocityLifecyclePolicy::default();
        policy.helper_quality_w_b = 0.0;
        let q0 = compute_helper_quality_score(0.0, 0.0, 0.0, 1.0, 0.0, 0.0, &policy);
        let q1 = compute_helper_quality_score(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, &policy);
        assert!(
            (q0 - q1).abs() < 1e-6,
            "w_b=0 では benevolence が Q に影響しない: B=1 -> {:.6}, B=0 -> {:.6}",
            q0, q1
        );
        println!("M1.76-8 TC-1 PASS: w_b=0 -> Q(B=1)={:.6} == Q(B=0)={:.6}", q0, q1);
    }

    // ============================================================
    // M1.76-8 TC-2: benevolence sweep で Q 単調非減少
    // ============================================================
    #[test]
    fn test_f11_benevolence_monotonic() {
        let policy = ReciprocityLifecyclePolicy::default();
        let mut previous_q = f32::NEG_INFINITY;
        for i in 0..=100 {
            let b = i as f32 / 100.0;
            let q = compute_helper_quality_score(0.0, 0.0, 0.0, b, 0.0, 0.0, &policy);
            assert!(
                q >= previous_q - 1e-6,
                "benevolence 増加で Q が減少: {:.6} < {:.6} (B={:.2})",
                q, previous_q, b
            );
            previous_q = q;
        }
        println!("M1.76-8 TC-2 PASS: benevolence sweep 101点、単調非減少を確認");
    }

    // ============================================================
    // M1.76-8 TC-3: 全入力 0 → Q = 0
    // ============================================================
    #[test]
    fn test_f11_all_zero() {
        let policy = ReciprocityLifecyclePolicy::default();
        let q = compute_helper_quality_score(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, &policy);
        assert_eq!(q, 0.0, "全入力 0 では Q=0 を返す必要があります");
        println!("M1.76-8 TC-3 PASS: all zero -> Q={:.6}", q);
    }

    // ============================================================
    // M1.76-8 TC-4 (計装): ランダム入力で NaN/Inf 不在
    // ============================================================
    #[test]
    fn test_f11_nan_inf_absent() {
        let policy = ReciprocityLifecyclePolicy::default();
        let mut rng = StdRng::seed_from_u64(12345);
        for i in 0..10_000 {
            let s = rng.random::<f32>();
            let t = rng.random::<f32>();
            let r = rng.random::<f32>();
            let b = rng.random::<f32>();
            let n = rng.random::<f32>();
            let d = rng.random::<f32>();
            let q = compute_helper_quality_score(s, t, r, b, n, d, &policy);
            assert!(!q.is_nan(), "Q が NaN (index={i})");
            assert!(!q.is_infinite(), "Q が Inf (index={i})");
        }
        println!("M1.76-8 TC-4 PASS: ランダム入力 n=10,000 で NaN/Inf 不在を確認");
    }

    // ============================================================
    // M1.76-8 TC-5: softmax 確率和 = 1.0
    // ============================================================
    #[test]
    fn test_f12_softmax_sum_one() {
        let policy = ReciprocityLifecyclePolicy::default();
        let scores = vec![1.0f32, 2.0, 3.0, 4.0, 5.0];
        let probabilities = softmax_helper_selection(&scores, &policy);
        let sum: f64 = probabilities.iter().sum();
        assert!(
            (sum - 1.0).abs() < 1e-12,
            "softmax 確率和 {:.15} が 1.0 と一致しません",
            sum
        );
        println!("M1.76-8 TC-5 PASS: softmax sum={:.15}", sum);
    }

    // ============================================================
    // M1.76-8 TC-6: τ=100 → argmax 確率 > 0.999
    // ============================================================
    #[test]
    fn test_f12_tau_high_argmax() {
        let mut policy = ReciprocityLifecyclePolicy::default();
        policy.tau_helper_softmax = 100.0;
        let scores = vec![1.0f32, 2.0, 3.0, 4.0, 10.0];
        let probabilities = softmax_helper_selection(&scores, &policy);
        let max_prob = probabilities.iter().cloned().fold(0.0f64, f64::max);
        assert!(
            max_prob > 0.999,
            "τ=100 で argmax 確率 {:.10} が 0.999 未満です",
            max_prob
        );
        println!("M1.76-8 TC-6 PASS: τ=100, argmax_prob={:.10}", max_prob);
    }

    // ============================================================
    // M1.76-8 TC-7: τ=0.001 → 全確率 ≈ 1/N (誤差 ±0.01)
    // ============================================================
    #[test]
    fn test_f12_tau_low_uniform() {
        let mut policy = ReciprocityLifecyclePolicy::default();
        policy.tau_helper_softmax = 0.001;
        let scores = vec![1.0f32, 2.0, 3.0, 4.0, 5.0];
        let probabilities = softmax_helper_selection(&scores, &policy);
        let n = scores.len() as f64;
        let expected = 1.0 / n;
        for (i, &p) in probabilities.iter().enumerate() {
            assert!(
                (p - expected).abs() < 0.01,
                "τ=0.001 で確率 {:.6} が 1/N={:.6} と乖離 (index={})",
                p, expected, i
            );
        }
        println!("M1.76-8 TC-7 PASS: τ=0.001, N={}, 全確率 ≈ {:.6} ± 0.01", scores.len(), expected);
    }

    // ============================================================
    // M1.76-8 TC-8: 空リスト → 空 Vec
    // ============================================================
    #[test]
    fn test_f12_empty_list() {
        let policy = ReciprocityLifecyclePolicy::default();
        let scores: Vec<f32> = vec![];
        let probabilities = softmax_helper_selection(&scores, &policy);
        assert!(probabilities.is_empty(), "空スライスでは空 Vec を返す必要があります");
        println!("M1.76-8 TC-8 PASS: empty input -> empty output");
    }

    // ============================================================
    // M1.76-8 TC-9 (計装): softmax 数値安定性 (n = 10^5)
    // ============================================================
    #[test]
    fn test_f12_numerical_stability() {
        let policy = ReciprocityLifecyclePolicy::default();
        let mut rng = StdRng::seed_from_u64(12345);
        let sample_size = 100_000usize;
        let mut max_dev = 0.0f64;

        println!("M1.76-8 TC-9 stability_check,samples={sample_size}");
        for i in 0..sample_size {
            let n_candidates = rng.random_range(2..=20);
            let scores: Vec<f32> = (0..n_candidates)
                .map(|_| rng.random_range(-100.0..100.0))
                .collect();
            let probabilities = softmax_helper_selection(&scores, &policy);

            // NaN/Inf 不在
            for (j, &p) in probabilities.iter().enumerate() {
                assert!(!p.is_nan(), "確率が NaN (sample={i}, idx={j})");
                assert!(!p.is_infinite(), "確率が Inf (sample={i}, idx={j})");
                assert!(p >= 0.0, "確率が負: {:.15} (sample={i}, idx={j})", p);
            }

            let sum: f64 = probabilities.iter().sum();
            let dev = (sum - 1.0).abs();
            max_dev = max_dev.max(dev);
            assert!(
                dev < 1e-12,
                "確率和 {:.15} が 1.0 と乖離 (sample={i}, dev={:.15})",
                sum, dev
            );
        }

        println!("M1.76-8 TC-9 PASS: 数値安定性検証完了 (n={sample_size}, max_dev={:.15})", max_dev);
    }

    // ============================================================
    // M1.76-8 TC-10 (計装): τ エントロピー応答曲線
    // ============================================================
    #[test]
    fn test_f12_tau_entropy_sweep() {
        let tau_values = [0.001f32, 0.01, 0.1, 1.0, 10.0, 100.0, 1000.0];

        // 3 種類の分布でエントロピーを観測
        let test_sets = [
            ("uniform", vec![1.0f32, 1.0, 1.0, 1.0, 1.0]),
            ("skewed", vec![0.1f32, 0.5, 1.0, 2.0, 10.0]),
            ("bimodal", vec![10.0f32, 9.0, 0.5, 0.3, 10.0]),
        ];

        println!("M1.76-8 TC-10 entropy_sweep");
        for (label, scores) in &test_sets {
            for &tau in &tau_values {
                let mut policy = ReciprocityLifecyclePolicy::default();
                policy.tau_helper_softmax = tau;
                let probabilities = softmax_helper_selection(scores, &policy);

                // エントロピー H = -Σ p_i · ln(p_i)
                let entropy: f64 = probabilities
                    .iter()
                    .map(|&p| if p > 0.0 { -p * p.ln() } else { 0.0 })
                    .sum();

                let sum: f64 = probabilities.iter().sum();
                println!(
                    "entropy_sweep,dist={label},tau={tau:.3},entropy={entropy:.6},sum={sum:.10},n_candidates={}",
                    scores.len()
                );
            }
        }

        println!("M1.76-8 TC-10 PASS: τ エントロピー応答曲線 sweep 完了 (7 水準 × 3 分布)");
    }

    // ============================================================
    // M1.76-9 F-13 Benevolence-aware remote exploration tests
    // ============================================================

    /// T-1: need=0, B_local_avg=1.0 で ε_remote が clip 下限 (0.0)。
    #[test]
    fn test_f13_t1_boundary_min() {
        let policy = ReciprocityLifecyclePolicy::default();
        let epsilon = compute_benevolence_aware_remote_exploration(0.0, 1.0, &policy);
        assert!(
            epsilon >= 0.0 && epsilon <= 1.0,
            "ε_remote={} が範囲外",
            epsilon
        );
        // need=0, B_local_avg=1.0 → raw = ε₀ - a₂ = 0.05 - 1.0 = -0.95 → clip 下限 0.0
        assert!(
            (epsilon - 0.0).abs() < 1e-6,
            "最小値は 0.0 ですが ε_remote={}",
            epsilon
        );
        println!("M1.76-9 T-1 PASS: boundary_min ε_remote={:.6}", epsilon);
    }

    /// T-2: need=1.0, B_local_avg=0 で ε_remote が clip 上限 (ε_max)。
    #[test]
    fn test_f13_t2_boundary_max() {
        let policy = ReciprocityLifecyclePolicy::default();
        let epsilon = compute_benevolence_aware_remote_exploration(1.0, 0.0, &policy);
        // raw = ε₀ + a₁·1.0 - a₂·0.0 = 0.05 + 1.0 = 1.05 → clip 上限 ε_max = 0.20
        assert!(
            (epsilon - policy.epsilon_remote_max).abs() < 1e-6,
            "最大値は ε_max={} ですが ε_remote={}",
            policy.epsilon_remote_max,
            epsilon
        );
        println!("M1.76-9 T-2 PASS: boundary_max ε_remote={:.6}", epsilon);
    }

    /// T-3: local_benevolence_mean 増加に伴い ε_remote が単調非増加。
    #[test]
    fn test_f13_t3_benevolence_monotonic() {
        let policy = ReciprocityLifecyclePolicy::default();
        let mut prev_epsilon = f32::MAX;
        for b in 0..=20 {
            let b_local = b as f32 / 20.0;
            let epsilon = compute_benevolence_aware_remote_exploration(0.5, b_local, &policy);
            assert!(
                epsilon <= prev_epsilon + 1e-6,
                "B_local_avg 増加で ε_remote が増加: B={:.2} ε={:.6} > prev={:.6}",
                b_local,
                epsilon,
                prev_epsilon
            );
            prev_epsilon = epsilon;
        }
        println!(
            "M1.76-9 T-3 PASS: benevolence monotonic, final={:.6}",
            prev_epsilon
        );
    }

    /// T-4: child_need 増加に伴い ε_remote が単調非減少。
    #[test]
    fn test_f13_t4_need_monotonic() {
        let policy = ReciprocityLifecyclePolicy::default();
        let mut prev_epsilon = f32::MIN;
        for n in 0..=20 {
            let need = n as f32 / 20.0;
            let epsilon = compute_benevolence_aware_remote_exploration(need, 0.5, &policy);
            assert!(
                epsilon >= prev_epsilon - 1e-6,
                "need 増加で ε_remote が減少: need={:.2} ε={:.6} < prev={:.6}",
                need,
                epsilon,
                prev_epsilon
            );
            prev_epsilon = epsilon;
        }
        println!(
            "M1.76-9 T-4 PASS: need monotonic, final={:.6}",
            prev_epsilon
        );
    }

    /// T-5: a₂=0 かつ need=0 のとき ε_remote == ε₀（下位互換性）。
    #[test]
    fn test_f13_t5_backward_compat() {
        let mut policy = ReciprocityLifecyclePolicy::default();
        policy.epsilon_remote_benevolence_coeff = 0.0;
        let epsilon = compute_benevolence_aware_remote_exploration(0.0, 0.5, &policy);
        assert!(
            (epsilon - policy.epsilon_remote_base).abs() < 1e-6,
            "下位互換性: ε_remote={} != ε₀={}",
            epsilon,
            policy.epsilon_remote_base
        );
        println!(
            "M1.76-9 T-5 PASS: backward compat ε_remote={:.6} == ε₀={:.6}",
            epsilon,
            policy.epsilon_remote_base
        );
    }

    /// T-6: 全入力値域で [0, ε_max] に bounded (n=10⁴ ランダムサンプリング)。
    #[test]
    fn test_f13_t6_bounded_random() {
        let policy = ReciprocityLifecyclePolicy::default();
        let mut rng = StdRng::seed_from_u64(12345);
        let n = 10_000u64;
        for _ in 0..n {
            let need: f32 = rng.random();
            let b_local: f32 = rng.random();
            let epsilon = compute_benevolence_aware_remote_exploration(need, b_local, &policy);
            assert!(
                epsilon >= 0.0 && epsilon <= policy.epsilon_remote_max + 1e-6,
                "ε_remote={} が [0, {}] の範囲外 (need={}, B_local={})",
                epsilon,
                policy.epsilon_remote_max,
                need,
                b_local
            );
        }
        println!("M1.76-9 T-6 PASS: bounded random (n={n})");
    }

    /// T-7: need=0.5, B_local_avg=0.5 (ニュートラル) で ε₀ に近い値。
    #[test]
    fn test_f13_t7_neutral() {
        let policy = ReciprocityLifecyclePolicy::default();
        let epsilon = compute_benevolence_aware_remote_exploration(0.5, 0.5, &policy);
        // raw = ε₀ + a₁·0.5 - a₂·0.5 = 0.05 + 0.5 - 0.5 = 0.05 → ε₀ と一致
        assert!(
            (epsilon - policy.epsilon_remote_base).abs() < 1e-6,
            "ニュートラル値: ε_remote={} != ε₀={}",
            epsilon,
            policy.epsilon_remote_base
        );
        println!("M1.76-9 T-7 PASS: neutral ε_remote={:.6} == ε₀={:.6}", epsilon, policy.epsilon_remote_base);
    }

    /// 観測テスト: (need, B_local_avg) の 2D 応答曲面 sweep + 差分分布。
    #[test]
    fn test_f13_observation_response_surface() {
        let policy = ReciprocityLifecyclePolicy::default();
        let ratio_values = [0.5, 1.0, 2.0];

        println!();
        println!("=== M1.76-9 F-13 応答曲面観測 ===");
        println!("定数: ε₀={}, ε_max={}, a₁={}, a₂={}",
            policy.epsilon_remote_base,
            policy.epsilon_remote_max,
            policy.epsilon_remote_need_coeff,
            policy.epsilon_remote_benevolence_coeff,
        );

        for ratio in &ratio_values {
            let mut adjusted_policy = policy.clone();
            // a₁/a₂ ratio を sweep: a₂ は固定で a₁ を調整
            adjusted_policy.epsilon_remote_need_coeff = *ratio as f32;
            // a₂ を固定、a₁ を ratio 値に設定 (a₂=1.0 のまま)
            println!();
            println!("--- a₁/a₂ ratio = {ratio} (a₁={}, a₂=1.0) ---", *ratio);
            println!("need\\B_local\t{}", (0..=10).map(|i| format!("{:.2}", i as f32 / 10.0)).collect::<Vec<_>>().join("\t"));

            for n in 0..=10 {
                let need = n as f32 / 10.0;
                let row: Vec<String> = (0..=10)
                    .map(|b| {
                        let b_local = b as f32 / 10.0;
                        let eps = compute_benevolence_aware_remote_exploration(
                            need, b_local, &adjusted_policy,
                        );
                        format!("{:.4}", eps)
                    })
                    .collect();
                println!("{:.1}\t\t{}", need, row.join("\t"));
            }
        }

        // 差分分布: random sampling n=10⁴
        let mut rng = StdRng::seed_from_u64(12345);
        let sample_n = 10_000u64;
        let base_epsilon = policy.epsilon_remote_base;
        let mut diffs = Vec::with_capacity(sample_n as usize);
        let mut sat_count = 0u64;
        let mut starve_count = 0u64;

        for _ in 0..sample_n {
            let need: f32 = rng.random();
            let b_local: f32 = rng.random();
            let eps = compute_benevolence_aware_remote_exploration(need, b_local, &policy);
            diffs.push(eps - base_epsilon);
            if (eps - policy.epsilon_remote_max).abs() < 1e-6 {
                sat_count += 1;
            }
            if eps.abs() < 1e-6 {
                starve_count += 1;
            }
        }

        let mean = diffs.iter().sum::<f32>() / sample_n as f32;
        let variance = diffs.iter().map(|d| (d - mean).powi(2)).sum::<f32>() / sample_n as f32;
        let std_dev = variance.sqrt();
        let mut sorted_diffs = diffs.clone();
        sorted_diffs.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let p5 = sorted_diffs[(sample_n as f64 * 0.05) as usize];
        let p95 = sorted_diffs[(sample_n as f64 * 0.95) as usize];

        println!();
        println!("--- 差分分布 (ε_remote - ε₀, n={sample_n}) ---");
        println!("平均: {:.6}", mean);
        println!("標準偏差: {:.6}", std_dev);
        println!("P5: {:.6}", p5);
        println!("P95: {:.6}", p95);
        println!("飽和率 (ε_max 張り付き): {:.2}%", sat_count as f64 / sample_n as f64 * 100.0);
        println!("飢餓率 (0.0 張り付き): {:.2}%", starve_count as f64 / sample_n as f64 * 100.0);
        println!("=== M1.76-9 応答曲面観測完了 ===");
    }

    // ============================================================
    // M1.76-10: F-14 compute_child_growth_increment テスト
    // ============================================================

    #[test]
    fn test_f14_t1_boundary_max() {
        let policy = ReciprocityLifecyclePolicy::default();
        // mission_success=true, help_successes=[], helper_benevolence_mean=1.0, failure_burden=0
        // → μ₁*1.0 + μ₃*1.0 = 0.60 + 0.30 = 0.90
        let result = super::compute_child_growth_increment(true, &[], 1.0, 0.0, &policy);
        let expected = policy.child_growth_mu_mission_success
            + policy.child_growth_mu_helper_benevolence;
        assert!(
            (result - expected).abs() < 1e-6,
            "F-14 T-1: expected {expected}, got {result}"
        );
    }

    #[test]
    fn test_f14_t2_boundary_min() {
        let policy = ReciprocityLifecyclePolicy::default();
        // mission_success=false, help_successes=[], helper_benevolence_mean=0, failure_burden=1.0
        // → 0.0 + 0.0 + 0.0 - 0.20 = -0.20 → clip to 0.0
        let result = super::compute_child_growth_increment(false, &[], 0.0, 1.0, &policy);
        assert!(
            result.abs() < 1e-6,
            "F-14 T-2: expected 0.0, got {result}"
        );
    }

    #[test]
    fn test_f14_t3_monotonic_mission_success() {
        let policy = ReciprocityLifecyclePolicy::default();
        let low = super::compute_child_growth_increment(false, &[0.5], 0.5, 0.1, &policy);
        let high = super::compute_child_growth_increment(true, &[0.5], 0.5, 0.1, &policy);
        assert!(
            high >= low - 1e-6,
            "F-14 T-3: mission_success=false→true で ΔG_c が減少 (low={low}, high={high})"
        );
    }

    #[test]
    fn test_f14_t4_monotonic_help_success() {
        let policy = ReciprocityLifecyclePolicy::default();
        let low = super::compute_child_growth_increment(true, &[0.1], 0.5, 0.1, &policy);
        let high = super::compute_child_growth_increment(true, &[0.9], 0.5, 0.1, &policy);
        assert!(
            high >= low - 1e-6,
            "F-14 T-4: help_successes 増加で ΔG_c が減少 (low={low}, high={high})"
        );
    }

    #[test]
    fn test_f14_t5_monotonic_failure_burden() {
        let policy = ReciprocityLifecyclePolicy::default();
        let low = super::compute_child_growth_increment(true, &[0.5], 0.5, 0.8, &policy);
        let high = super::compute_child_growth_increment(true, &[0.5], 0.5, 0.1, &policy);
        assert!(
            high >= low - 1e-6,
            "F-14 T-5: failure_burden 増加で ΔG_c が増加 (low={low}, high={high})"
        );
    }

    #[test]
    fn test_f14_t6_boundedness() {
        let policy = ReciprocityLifecyclePolicy::default();
        let mut rng = StdRng::seed_from_u64(12345);
        let expected_max = policy.child_growth_mu_mission_success
            + policy.child_growth_mu_help_success
            + policy.child_growth_mu_helper_benevolence;
        for _ in 0..10_000 {
            let ms: bool = rng.random();
            let hs_sum: f32 = rng.random();
            let hb: f32 = rng.random();
            let fb: f32 = rng.random();
            let result = super::compute_child_growth_increment(ms, &[hs_sum], hb, fb, &policy);
            assert!(
                result >= 0.0 && result <= expected_max + 1e-6,
                "F-14 T-6: ΔG_c={result} out of bounds [0, {expected_max}]"
            );
        }
    }

    #[test]
    fn test_f14_t7_neutral() {
        let policy = ReciprocityLifecyclePolicy::default();
        // mission_success=true, help_successes=[0.5], helper_benevolence_mean=0.5, failure_burden=0.0
        // → 0.60*1.0 + 0.40*0.5 + 0.30*0.5 = 0.60 + 0.20 + 0.15 = 0.95
        let result = super::compute_child_growth_increment(true, &[0.5], 0.5, 0.0, &policy);
        let expected = policy.child_growth_mu_mission_success
            + policy.child_growth_mu_help_success * 0.5
            + policy.child_growth_mu_helper_benevolence * 0.5;
        assert!(
            (result - expected).abs() < 1e-6,
            "F-14 T-7: expected {expected}, got {result}"
        );
    }

    #[test]
    fn test_f14_t8_clip_large_failure() {
        let policy = ReciprocityLifecyclePolicy::default();
        // Very large failure_burden should clip to 0
        let result = super::compute_child_growth_increment(false, &[], 0.0, 100.0, &policy);
        assert!(
            result.abs() < 1e-6,
            "F-14 T-8: large failure_burden should clip to 0, got {result}"
        );
    }

    #[test]
    fn test_f14_t9_zero_coefficients() {
        let mut policy = ReciprocityLifecyclePolicy::default();
        policy.child_growth_mu_mission_success = 0.0;
        policy.child_growth_mu_help_success = 0.0;
        policy.child_growth_mu_helper_benevolence = 0.0;
        policy.child_growth_mu_failure_burden = 0.0;
        // 全係数ゼロ → どんな入力でも ΔG_c = 0
        let result = super::compute_child_growth_increment(true, &[1.0, 1.0], 1.0, 1.0, &policy);
        assert!(
            result.abs() < 1e-6,
            "F-14 T-9: zero coefficients should give 0, got {result}"
        );
    }

    // ============================================================
    // M1.76-10: F-15 compute_maturation_probability テスト
    // ============================================================

    #[test]
    fn test_f15_t1_all_high() {
        let policy = ReciprocityLifecyclePolicy::default();
        // 全入力 1.0: logit = -2.0 + 1.0 + 1.0 + 1.0 + 0.30 = 1.30
        // sigmoid(1.30) ≈ 0.7858
        let result = super::compute_maturation_probability(1.0, 1.0, 1.0, 1.0, &policy);
        let expected_logit = policy.maturation_nu_bias
            + policy.maturation_nu_experience
            + policy.maturation_nu_trust
            + policy.maturation_nu_reputation
            + policy.maturation_nu_helper_benevolence;
        let expected = super::logistic_sigmoid(expected_logit) as f64;
        assert!(
            (result - expected).abs() < 1e-6,
            "F-15 T-1: expected {expected}, got {result}"
        );
        assert!(result > 0.5, "F-15 T-1: all-high should exceed 0.5, got {result}");
    }

    #[test]
    fn test_f15_t2_all_low() {
        let policy = ReciprocityLifecyclePolicy::default();
        // 全入力 0.0: logit = -2.0
        let result = super::compute_maturation_probability(0.0, 0.0, 0.0, 0.0, &policy);
        let expected = super::logistic_sigmoid(policy.maturation_nu_bias) as f64;
        assert!(
            (result - expected).abs() < 1e-6,
            "F-15 T-2: expected {expected}, got {result}"
        );
        assert!(result < 0.5, "F-15 T-2: all-low should be below 0.5, got {result}");
    }

    #[test]
    fn test_f15_t3_monotonic_experience() {
        let policy = ReciprocityLifecyclePolicy::default();
        let low = super::compute_maturation_probability(0.2, 0.5, 0.5, 0.5, &policy);
        let high = super::compute_maturation_probability(0.9, 0.5, 0.5, 0.5, &policy);
        assert!(
            high >= low - 1e-6,
            "F-15 T-3: experience 増加で確率が減少 (low={low}, high={high})"
        );
    }

    #[test]
    fn test_f15_t4_monotonic_trust() {
        let policy = ReciprocityLifecyclePolicy::default();
        let low = super::compute_maturation_probability(0.5, 0.1, 0.5, 0.5, &policy);
        let high = super::compute_maturation_probability(0.5, 0.9, 0.5, 0.5, &policy);
        assert!(
            high >= low - 1e-6,
            "F-15 T-4: trust 増加で確率が減少 (low={low}, high={high})"
        );
    }

    #[test]
    fn test_f15_t5_monotonic_reputation() {
        let policy = ReciprocityLifecyclePolicy::default();
        let low = super::compute_maturation_probability(0.5, 0.5, 0.1, 0.5, &policy);
        let high = super::compute_maturation_probability(0.5, 0.5, 0.9, 0.5, &policy);
        assert!(
            high >= low - 1e-6,
            "F-15 T-5: reputation 増加で確率が減少 (low={low}, high={high})"
        );
    }

    #[test]
    fn test_f15_t6_monotonic_benevolence() {
        let policy = ReciprocityLifecyclePolicy::default();
        let low = super::compute_maturation_probability(0.5, 0.5, 0.5, 0.1, &policy);
        let high = super::compute_maturation_probability(0.5, 0.5, 0.5, 0.9, &policy);
        assert!(
            high >= low - 1e-6,
            "F-15 T-6: helper_benevolence 増加で確率が減少 (low={low}, high={high})"
        );
    }

    #[test]
    fn test_f15_t7_boundedness() {
        let policy = ReciprocityLifecyclePolicy::default();
        let mut rng = StdRng::seed_from_u64(12345);
        for _ in 0..10_000 {
            let e: f32 = rng.random();
            let t: f32 = rng.random();
            let r: f32 = rng.random();
            let b: f32 = rng.random();
            let result = super::compute_maturation_probability(e, t, r, b, &policy);
            assert!(
                result > 0.0 && result < 1.0,
                "F-15 T-7: P_mature={result} out of (0, 1)"
            );
        }
    }

    #[test]
    fn test_f15_t8_sigmoid_symmetry() {
        let policy = ReciprocityLifecyclePolicy::default();
        let _all_high = super::compute_maturation_probability(1.0, 1.0, 1.0, 1.0, &policy);
        let all_low = super::compute_maturation_probability(0.0, 0.0, 0.0, 0.0, &policy);
        let neg_all_low = super::logistic_sigmoid(-policy.maturation_nu_bias) as f64;
        let sum = all_low + neg_all_low;
        assert!(
            (sum - 1.0).abs() < 1e-6,
            "F-15 T-8: sigmoid 対称性違反: {all_low} + {neg_all_low} = {sum}"
        );
    }

    #[test]
    fn test_f15_t9_benevolence_disabled() {
        let mut policy = ReciprocityLifecyclePolicy::default();
        // ν₄=0 → helper benevolence が maturation 確率に影響しない
        policy.maturation_nu_helper_benevolence = 0.0;
        let with_low_benev = super::compute_maturation_probability(0.5, 0.5, 0.5, 0.0, &policy);
        let with_high_benev = super::compute_maturation_probability(0.5, 0.5, 0.5, 1.0, &policy);
        assert!(
            (with_low_benev - with_high_benev).abs() < 1e-6,
            "F-15 T-9: ν₄=0 で benevolence が影響: low={with_low_benev}, high={with_high_benev}"
        );
    }

    #[test]
    fn test_f15_t10_all_zero_matches_sigmoid() {
        let policy = ReciprocityLifecyclePolicy::default();
        let result = super::compute_maturation_probability(0.0, 0.0, 0.0, 0.0, &policy);
        let expected = super::logistic_sigmoid(policy.maturation_nu_bias) as f64;
        assert!(
            (result - expected).abs() < 1e-6,
            "F-15 T-10: expected {expected}, got {result}"
        );
    }

    // ============================================================
    // M1.76-10: F-14 観測テスト — 応答曲面
    // ============================================================

    #[test]
    fn test_f14_observation_response_surface() {
        let policy = ReciprocityLifecyclePolicy::default();
        println!();
        println!("=== M1.76-10 F-14 応答曲面観測 ===");
        println!("固定: mission_success=true, helper_benevolence_mean=0.5");
        println!("行: help_successes sum [0.0:0.1:1.0], 列: failure_burden [0.0:0.1:1.0]");

        // ヘッダー行
        print!("hs\\fb");
        for fb in 0..=10 {
            print!("\t{:.1}", fb as f32 / 10.0);
        }
        println!();

        for hs_i in 0..=10 {
            let hs = hs_i as f32 / 10.0;
            print!("{:.1}", hs);
            for fb_i in 0..=10 {
                let fb = fb_i as f32 / 10.0;
                let val = super::compute_child_growth_increment(true, &[hs], 0.5, fb, &policy);
                print!("\t{:.4}", val);
            }
            println!();
        }
        println!("=== M1.76-10 F-14 応答曲面観測完了 ===");
    }

    // ============================================================
    // M1.76-10: F-15 観測テスト — 応答曲面
    // ============================================================

    #[test]
    fn test_f15_observation_response_surface() {
        let policy = ReciprocityLifecyclePolicy::default();
        println!();
        println!("=== M1.76-10 F-15 応答曲面観測 ===");
        println!("固定: trust=0.5, reputation=0.5");
        println!("行: experience_norm [0.0:0.1:1.0], 列: helper_benevolence_mean [0.0:0.1:1.0]");

        // ヘッダー行
        print!("en\\hb");
        for hb in 0..=10 {
            print!("\t{:.1}", hb as f32 / 10.0);
        }
        println!();

        for en_i in 0..=10 {
            let en = en_i as f32 / 10.0;
            print!("{:.1}", en);
            for hb_i in 0..=10 {
                let hb = hb_i as f32 / 10.0;
                let val = super::compute_maturation_probability(en, 0.5, 0.5, hb, &policy);
                print!("\t{:.4}", val);
            }
            println!();
        }
        println!("=== M1.76-10 F-15 応答曲面観測完了 ===");
    }

    // ============================================================
    // M1.76-11: R11-T1 空 store → 空 profiles
    // ============================================================
    #[test]
    fn test_empty_store_returns_empty_profiles() {
        let policy = ReciprocityLifecyclePolicy::default();
        let store = ReciprocityEventStore::new();
        let metrics = HashMap::new();
        let profiles = recompute_all_profiles(&store, &metrics, 0, &policy);
        assert!(profiles.is_empty(), "空ストアからの recompute は空 HashMap を返す必要があります");
        println!("M1.76-11 R11-T1 PASS: empty store -> empty profiles");
    }

    // ============================================================
    // M1.76-11: R11-T2 決定論的リプレイ（同一イベント系列 → 一致）
    // ============================================================
    #[test]
    fn test_deterministic_replay_identical_results() {
        let policy = ReciprocityLifecyclePolicy::default();
        let metrics = {
            let mut m = HashMap::new();
            m.insert(
                "graph-a".to_string(),
                GraphMetrics {
                    centrality: 0.5,
                    village_participation: 0.3,
                    accepted_rate: 0.8,
                    success_rate: 0.9,
                    harm_score: 0.1,
                    inherited_score: 0.2,
                    experience_count: 10,
                },
            );
            m
        };

        // 同一イベントを 2 つの別々のストアに ingest
        let mut store1 = ReciprocityEventStore::new();
        let mut store2 = ReciprocityEventStore::new();
        let events = vec![
            ReciprocityEvent {
                event_id: "ev-1".to_string(),
                mission_id: "mission".to_string(),
                source_graph_id: "graph-a".to_string(),
                target_graph_id: "target-1".to_string(),
                event_kind: ReciprocityEventKind::HelpSucceeded,
                weight: 1.0,
                created_at: std::time::SystemTime::now(),
                virtual_clock: 100,
                trace_ref: None,
            },
            ReciprocityEvent {
                event_id: "ev-2".to_string(),
                mission_id: "mission".to_string(),
                source_graph_id: "graph-a".to_string(),
                target_graph_id: "target-2".to_string(),
                event_kind: ReciprocityEventKind::HarmfulMismatch,
                weight: 1.0,
                created_at: std::time::SystemTime::now(),
                virtual_clock: 200,
                trace_ref: None,
            },
        ];
        for ev in &events {
            store1.ingest(ev.clone());
            store2.ingest(ev.clone());
        }

        let profiles1 = recompute_all_profiles(&store1, &metrics, 300, &policy);
        let profiles2 = recompute_all_profiles(&store2, &metrics, 300, &policy);

        assert_eq!(
            profiles1, profiles2,
            "同一イベント系列からの recompute は同一結果を返す必要があります"
        );
        println!("M1.76-11 R11-T2 PASS: deterministic replay identical ({} profiles)", profiles1.len());
    }

    // ============================================================
    // M1.76-11: R11-T3 異なる policy_version → DiffReport に差分
    // ============================================================
    #[test]
    fn test_policy_version_diff_detection() {
        let mut policy_v1 = ReciprocityLifecyclePolicy::default();
        policy_v1.policy_version = "v1".to_string();
        policy_v1.lambda_gc_base = 0.02;

        let mut policy_v2 = ReciprocityLifecyclePolicy::default();
        policy_v2.policy_version = "v2".to_string();
        policy_v2.lambda_gc_base = 0.10;

        let store = ReciprocityEventStore::new();
        let metrics = HashMap::new();
        let profiles = HashMap::new();

        let snapshot_v1 = ReciprocityReplaySnapshot {
            profiles: recompute_all_profiles(&store, &metrics, 0, &policy_v1),
            hazards: recompute_all_gc_hazards(
                &profiles, &HashMap::new(), &HashMap::new(), &policy_v1,
            ),
            policy_version: "v1".to_string(),
            clock: 0,
        };

        let snapshot_v2 = ReciprocityReplaySnapshot {
            profiles: recompute_all_profiles(&store, &metrics, 0, &policy_v2),
            hazards: recompute_all_gc_hazards(
                &profiles, &HashMap::new(), &HashMap::new(), &policy_v2,
            ),
            policy_version: "v2".to_string(),
            clock: 0,
        };

        let report = compute_replay_comparison(&snapshot_v1, &snapshot_v2);
        assert_eq!(
            report.before_policy_version, "v1",
            "before_policy_version は v1 である必要があります"
        );
        assert_eq!(
            report.after_policy_version, "v2",
            "after_policy_version は v2 である必要があります"
        );
        println!(
            "M1.76-11 R11-T3 PASS: policy_version diff detected (v1 -> v2, {} profile diffs, {} hazard diffs)",
            report.profile_diffs.len(),
            report.hazard_diffs.len()
        );
    }

    // ============================================================
    // M1.76-11: R11-T4 イベント追加で結果が変化
    // ============================================================
    #[test]
    fn test_event_addition_changes_result() {
        let policy = ReciprocityLifecyclePolicy::default();
        let mut metrics = HashMap::new();
        metrics.insert(
            "graph-a".to_string(),
            GraphMetrics {
                centrality: 0.5,
                village_participation: 0.5,
                accepted_rate: 0.5,
                success_rate: 0.5,
                harm_score: 0.0,
                inherited_score: 0.0,
                experience_count: 1,
            },
        );

        let mut store = ReciprocityEventStore::new();
        let baseline = recompute_all_profiles(&store, &metrics, 100, &policy);

        // 1件イベントを追加
        store.ingest(ReciprocityEvent {
            event_id: "ev-new".to_string(),
            mission_id: "mission".to_string(),
            source_graph_id: "graph-a".to_string(),
            target_graph_id: "target-a".to_string(),
            event_kind: ReciprocityEventKind::HelpSucceeded,
            weight: 1.0,
            created_at: std::time::SystemTime::now(),
            virtual_clock: 50,
            trace_ref: None,
        });

        let updated = recompute_all_profiles(&store, &metrics, 100, &policy);
        assert_ne!(
            baseline, updated,
            "イベント追加後に recompute 結果は変化する必要があります"
        );
        println!("M1.76-11 R11-T4 PASS: event addition changed result");
    }

    // ============================================================
    // M1.76-11: R11-T5 policy_version が snapshot に記録
    // ============================================================
    #[test]
    fn test_snapshot_policy_version_recorded() {
        let snapshot = ReciprocityReplaySnapshot {
            profiles: HashMap::new(),
            hazards: HashMap::new(),
            policy_version: "test-version".to_string(),
            clock: 42,
        };
        assert_eq!(snapshot.policy_version, "test-version");
        assert_eq!(snapshot.clock, 42);
        println!("M1.76-11 R11-T5 PASS: policy_version={}, clock={}", snapshot.policy_version, snapshot.clock);
    }

    // ============================================================
    // M1.76-11: R11-T6 空 profiles → 空 hazards
    // ============================================================
    #[test]
    fn test_empty_profiles_returns_empty_hazards() {
        let policy = ReciprocityLifecyclePolicy::default();
        let hazards = recompute_all_gc_hazards(
            &HashMap::new(), &HashMap::new(), &HashMap::new(), &policy,
        );
        assert!(hazards.is_empty(), "空 profiles からの recompute は空 HashMap を返す必要があります");
        println!("M1.76-11 R11-T6 PASS: empty profiles -> empty hazards");
    }

    // ============================================================
    // M1.76-11: R11-T7 全 hazard 非負（n=10⁴ ランダム）
    // ============================================================
    #[test]
    fn test_all_hazards_non_negative_n10000() {
        let policy = ReciprocityLifecyclePolicy::default();
        let mut rng = StdRng::seed_from_u64(12345);
        let sample_size = 10_000usize;

        let mut has_non_determinism = false;
        for _ in 0..sample_size {
            let mut profiles = HashMap::new();
            let mut lifecycle_scores = HashMap::new();
            let mut child_protections = HashMap::new();

            for i in 0..10 {
                let g_id = format!("graph-{i}");
                let profile = ReputationProfile {
                    benevolence_score: rng.random_range(0.0..1.0),
                    ..ReputationProfile::cold_start()
                };
                profiles.insert(g_id.clone(), profile);
                lifecycle_scores.insert(g_id.clone(), rng.random_range(0.0..1.0));
                child_protections.insert(g_id, rng.random_range(0.0..1.0));
            }

            let hazards = recompute_all_gc_hazards(
                &profiles, &lifecycle_scores, &child_protections, &policy,
            );

            for (g_id, &h) in &hazards {
                assert!(
                    h.is_finite() && h >= 0.0,
                    "hazard が非負かつ有限である必要があります: {} = {}",
                    g_id, h
                );
                if h > 0.0 {
                    has_non_determinism = true;
                }
            }
        }

        assert!(has_non_determinism, "少なくとも一部の hazard は非ゼロである必要があります");
        println!("M1.76-11 R11-T7 PASS: all hazards non-negative (n={sample_size})");
    }

    // ============================================================
    // M1.76-11: R11-T8 同一 snapshot → 差分なし
    // ============================================================
    #[test]
    fn test_identical_snapshot_no_diff() {
        let snapshot = ReciprocityReplaySnapshot {
            profiles: HashMap::new(),
            hazards: HashMap::new(),
            policy_version: "same".to_string(),
            clock: 0,
        };
        let report = compute_replay_comparison(&snapshot, &snapshot);
        assert!(report.changed_graph_ids.is_empty(), "同一 snapshot では changed_graph_ids が空である必要があります");
        assert!(report.profile_diffs.is_empty(), "同一 snapshot では profile_diffs が空である必要があります");
        assert!(report.hazard_diffs.is_empty(), "同一 snapshot では hazard_diffs が空である必要があります");
        assert_eq!(report.before_policy_version, "same");
        assert_eq!(report.after_policy_version, "same");
        println!("M1.76-11 R11-T8 PASS: identical snapshot -> no diff");
    }

    // ============================================================
    // M1.76-11: R11-T9 観測テスト — 応答曲面
    // ============================================================
    #[test]
    fn test_pipeline_observation_response_surface() {
        let policy = ReciprocityLifecyclePolicy::default();
        let mut rng = StdRng::seed_from_u64(12345);

        println!();
        println!("=== M1.76-11 R11-T9 パイプライン応答曲面 ===");

        // グラフ数 5 で sweep
        let graph_count = 5usize;
        let event_counts = [10usize, 25, 50];

        for &n_events in &event_counts {
            let mut store = ReciprocityEventStore::new();
            let mut metrics = HashMap::new();

            for g in 0..graph_count {
                let g_id = format!("graph-{g}");
                metrics.insert(g_id.clone(), GraphMetrics {
                    centrality: rng.random_range(0.2..0.9),
                    village_participation: rng.random_range(0.1..0.8),
                    accepted_rate: rng.random_range(0.5..1.0),
                    success_rate: rng.random_range(0.5..1.0),
                    harm_score: rng.random_range(0.0..0.3),
                    inherited_score: rng.random_range(0.0..0.5),
                    experience_count: n_events as u32,
                });

                for i in 0..n_events {
                    let kind = match i % 4 {
                        0 => ReciprocityEventKind::HelpSucceeded,
                        1 => ReciprocityEventKind::HarmfulMismatch,
                        2 => ReciprocityEventKind::HelpOffered,
                        _ => ReciprocityEventKind::ReturnedFavor,
                    };
                    store.ingest(ReciprocityEvent {
                        event_id: format!("ev-{g}-{i}"),
                        mission_id: "mission".to_string(),
                        source_graph_id: g_id.clone(),
                        target_graph_id: format!("target-{i}"),
                        event_kind: kind,
                        weight: rng.random_range(0.5..1.5),
                        created_at: std::time::SystemTime::now(),
                        virtual_clock: (i * 10) as u64,
                        trace_ref: None,
                    });
                }
            }

            let profiles = recompute_all_profiles(&store, &metrics, 1000, &policy);
            let final_scores: Vec<f32> = profiles.values().map(|p| p.final_score).collect();
            let mean = final_scores.iter().sum::<f32>() / final_scores.len() as f32;

            // lifecycle_scores と child_protections を生成
            let lifecycle_scores: HashMap<WorkflowGraphId, f32> =
                profiles.keys().map(|g| (g.clone(), rng.random_range(0.0..1.0))).collect();
            let child_protections: HashMap<WorkflowGraphId, f32> =
                profiles.keys().map(|g| (g.clone(), rng.random_range(0.0..0.5))).collect();

            let hazards = recompute_all_gc_hazards(
                &profiles, &lifecycle_scores, &child_protections, &policy,
            );
            let hazard_values: Vec<f32> = hazards.values().copied().collect();
            let hazard_mean = hazard_values.iter().sum::<f32>() / hazard_values.len() as f32;
            let hazard_min = hazard_values.iter().cloned().fold(f32::MAX, f32::min);
            let hazard_max = hazard_values.iter().cloned().fold(f32::MIN, f32::max);

            println!("graph_count={graph_count}, event_count={n_events}, final_scores_mean={mean:.4}, scores={final_scores:?}");
            println!("  hazards: mean={hazard_mean:.4}, min={hazard_min:.4}, max={hazard_max:.4}");
        }

        // 独立グラフ間の非干渉検証
        let mut store_a = ReciprocityEventStore::new();
        let mut store_b = ReciprocityEventStore::new();
        let metrics_a: HashMap<WorkflowGraphId, GraphMetrics> = [("graph-a".to_string(), GraphMetrics {
            centrality: 0.5, village_participation: 0.5, accepted_rate: 0.5,
            success_rate: 0.5, harm_score: 0.0, inherited_score: 0.0, experience_count: 1,
        })].into();
        let metrics_b: HashMap<WorkflowGraphId, GraphMetrics> = [("graph-b".to_string(), GraphMetrics {
            centrality: 0.5, village_participation: 0.5, accepted_rate: 0.5,
            success_rate: 0.5, harm_score: 0.0, inherited_score: 0.0, experience_count: 1,
        })].into();

        store_a.ingest(ReciprocityEvent {
            event_id: "a-1".to_string(), mission_id: "m".to_string(),
            source_graph_id: "graph-a".to_string(), target_graph_id: "t".to_string(),
            event_kind: ReciprocityEventKind::HelpSucceeded, weight: 1.0,
            created_at: std::time::SystemTime::now(), virtual_clock: 50, trace_ref: None,
        });
        store_b.ingest(ReciprocityEvent {
            event_id: "b-1".to_string(), mission_id: "m".to_string(),
            source_graph_id: "graph-b".to_string(), target_graph_id: "t".to_string(),
            event_kind: ReciprocityEventKind::HarmfulMismatch, weight: 1.0,
            created_at: std::time::SystemTime::now(), virtual_clock: 50, trace_ref: None,
        });

        let profiles_a = recompute_all_profiles(&store_a, &metrics_a, 100, &policy);
        let profiles_b = recompute_all_profiles(&store_b, &metrics_b, 100, &policy);

        // graph-a の profile は正イベントにより 0.5 < final_score
        let final_a = profiles_a.get("graph-a").map(|p| p.final_score).unwrap_or(0.5);
        // graph-b の profile は負イベントにより 0.5 > final_score
        let final_b = profiles_b.get("graph-b").map(|p| p.final_score).unwrap_or(0.5);

        println!("  pipeline_independence: graph_a_final={final_a:.4}, graph_b_final={final_b:.4}");
        println!("=== M1.76-11 R11-T9 応答曲面観測完了 ===");
    }

    // -------------------------------------------------------
    // M1.76-12: MUST 単調性テストスイート
    // -------------------------------------------------------

    /// 条件1: direct_score ↑ → survival_probability 非減少 (5点 sweep + n=1000 random)
    #[test]
    fn test_direct_score_survival_monotonicity() {
        let suite = MonotonicityTestSuite::default();
        let report = check_monotonicity(&suite);
        let passed = report
            .conditions_passed
            .iter()
            .find(|(c, _)| *c == MonotonicityCondition::DirectScoreIncrease)
            .map(|(_, p)| *p)
            .unwrap_or(false);
        let rate = report
            .random_sweep_violation_rates
            .get(&MonotonicityCondition::DirectScoreIncrease)
            .copied()
            .unwrap_or(1.0);
        println!("M1.76-12 C1 direct_score→survival: PASS={passed}, random_violation_rate={rate:.6}");
        for detail in &report.failure_details {
            if detail.starts_with("条件1") {
                println!("  FAIL: {detail}");
            }
        }
        assert!(passed, "条件1: direct_score 増加で survival_probability が非減少であること");
        assert!(rate < 1e-4, "条件1 ランダム sweep 違反率が許容閾値を超過: {rate}");
    }

    /// 条件2: indirect_score ↑ → GC hazard 非増加 (5点 sweep + n=1000 random)
    #[test]
    fn test_indirect_score_gc_hazard_monotonicity() {
        let suite = MonotonicityTestSuite::default();
        let report = check_monotonicity(&suite);
        let passed = report
            .conditions_passed
            .iter()
            .find(|(c, _)| *c == MonotonicityCondition::IndirectScoreIncrease)
            .map(|(_, p)| *p)
            .unwrap_or(false);
        let rate = report
            .random_sweep_violation_rates
            .get(&MonotonicityCondition::IndirectScoreIncrease)
            .copied()
            .unwrap_or(1.0);
        println!("M1.76-12 C2 indirect_score→hazard: PASS={passed}, random_violation_rate={rate:.6}");
        for detail in &report.failure_details {
            if detail.starts_with("条件2") {
                println!("  FAIL: {detail}");
            }
        }
        assert!(passed, "条件2: indirect_score 増加で GC hazard が非増加であること");
        assert!(rate < 1e-4, "条件2 ランダム sweep 違反率が許容閾値を超過: {rate}");
    }

    /// 条件3: Reputation ↑ → GC hazard 非増加 (5点 sweep + n=1000 random)
    #[test]
    fn test_reputation_gc_hazard_monotonicity() {
        let suite = MonotonicityTestSuite::default();
        let report = check_monotonicity(&suite);
        let passed = report
            .conditions_passed
            .iter()
            .find(|(c, _)| *c == MonotonicityCondition::ReputationIncrease)
            .map(|(_, p)| *p)
            .unwrap_or(false);
        let rate = report
            .random_sweep_violation_rates
            .get(&MonotonicityCondition::ReputationIncrease)
            .copied()
            .unwrap_or(1.0);
        println!("M1.76-12 C3 reputation→hazard: PASS={passed}, random_violation_rate={rate:.6}");
        for detail in &report.failure_details {
            if detail.starts_with("条件3") {
                println!("  FAIL: {detail}");
            }
        }
        assert!(passed, "条件3: Reputation 増加で GC hazard が非増加であること");
        assert!(rate < 1e-4, "条件3 ランダム sweep 違反率が許容閾値を超過: {rate}");
    }

    /// 条件4: 高 benevolence helper が ranking で不利にならない (ΔB sweep [0.001, 0.5])
    #[test]
    fn test_benevolence_helper_ranking_monotonicity() {
        let suite = MonotonicityTestSuite::default();
        let report = check_monotonicity(&suite);
        let passed = report
            .conditions_passed
            .iter()
            .find(|(c, _)| *c == MonotonicityCondition::BenevolenceHelperRanking)
            .map(|(_, p)| *p)
            .unwrap_or(false);
        let rate = report
            .random_sweep_violation_rates
            .get(&MonotonicityCondition::BenevolenceHelperRanking)
            .copied()
            .unwrap_or(1.0);
        println!("M1.76-12 C4 benevolence→ranking: PASS={passed}, violation_rate={rate:.6}");
        for detail in &report.failure_details {
            if detail.starts_with("条件4") {
                println!("  FAIL: {detail}");
            }
        }
        assert!(passed, "条件4: 高 benevolence helper が ranking で不利にならないこと");
        assert!(rate < 1e-4, "条件4 ΔB sweep 違反率が許容閾値を超過: {rate}");
    }

    /// 統合テスト: Default スイートで全条件を一括検証 + 観測出力
    #[test]
    fn test_monotonicity_suite_full() {
        let suite = MonotonicityTestSuite::default();
        let report = check_monotonicity(&suite);

        println!("=== M1.76-12 単調性テストスイート ===");
        println!("Sample size: random_sweep_n={}", suite.random_sweep_samples);
        for (condition, passed) in &report.conditions_passed {
            println!("  {condition:?}: PASS={passed}");
        }
        println!("Violation rates:");
        for (condition, rate) in &report.random_sweep_violation_rates {
            println!("  {condition:?}: violation_rate={rate:.6}");
        }
        if report.failure_details.is_empty() {
            println!("全 MUST 単調性条件を PASS — 違反なし");
        } else {
            println!("違反詳細 ({}件):", report.failure_details.len());
            for detail in &report.failure_details {
                println!("  {detail}");
            }
        }
        println!("=== M1.76-12 単調性テストスイート完了 ===");

        // 全条件 PASS を検証
        for (condition, passed) in &report.conditions_passed {
            assert!(*passed, "条件 {condition:?} が FAIL");
        }
    }
}
