// 相互互恵性モジュール (RFC §15.10)
//
// RFC §15.10 で定義される Reciprocity-Aware Survival の基盤計算エンジンを提供する。
// 式 F-1 (compute_direct_reciprocity) の純粋関数を含む。
//
// 式 F-1: R_i^dir = σ( Σ_{j≠i} ω_ij^dir (α_h H_ij + α_hs HS_ij - α_r RJ_ij - α_d DMG_ij) exp(-ρ_dir Δt_ij) )

use crate::constants;
use crate::event::{ReciprocityEvent, ReciprocityEventKind, ReciprocityLifecyclePolicy};

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

/// ロジスティックシグモイド関数 (F-1)。
///
/// σ(x) = 1 / (1 + exp(-x))
/// 任意の実数値を (0, 1) の範囲に押し込む。
fn logistic_sigmoid(x: f32) -> f32 {
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
}
