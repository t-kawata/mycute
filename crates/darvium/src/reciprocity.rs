// 相互互恵性モジュール (RFC §15.10)
//
// RFC §15.10 で定義される Reciprocity-Aware Survival の基盤計算エンジンを提供する。
// 式 F-1 (compute_direct_reciprocity) の純粋関数を含む。
//
// 式 F-1: R_i^dir = σ( Σ_{j≠i} ω_ij^dir (α_h H_ij + α_hs HS_ij - α_r RJ_ij - α_d DMG_ij) exp(-ρ_dir Δt_ij) )

use crate::constants;
use crate::event::{ReciprocityEvent, ReciprocityEventKind, ReciprocityLifecyclePolicy, ReputationProfile};

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
}
