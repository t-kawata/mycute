// ランクドリフト頑健性シミュレーションテスト (RFC §12.2, §21.1 OQ-04/08)
//
// 本モジュールはガウスノイズ注入環境下における上位選択アルゴリズムの
// 頑健性を検証するテスト基盤を提供する。
// 固定シード PRNG による再現可能な実験を前提とする。

use crate::types::RankedCandidate;

/// RFC §12.2 のブレンド係数 α (セマンティックと構造の統合比率)
const BLEND_ALPHA: f64 = 0.35;

// ============================================================
// 公開関数
// ============================================================

/// スコアにガウスノイズを注入し、`[0.0, 1.0]` の範囲にクランプする。
///
/// Box-Muller 変換により標準正規分布に従う乱数を生成し、スコアに加算する。
pub fn inject_gaussian_noise(score: f64, sigma: f64, rng: &mut impl rand::Rng) -> f64 {
    use std::f64::consts::PI;
    let u1: f64 = rng.random();
    let u2: f64 = rng.random();
    let z = (-2.0 * u1.ln()).sqrt() * (2.0 * PI * u2).cos();
    let noisy = score + z * sigma;
    noisy.clamp(0.0, 1.0)
}

/// RFC §12.2 の類似度統合式により blended_score を計算する。
///
/// `blended_score = (1 - α) × semantic_score + α × structural_score` (α = 0.35)
pub fn compute_blended_score(semantic: f64, structural: f64) -> f64 {
    (1.0 - BLEND_ALPHA) * semantic + BLEND_ALPHA * structural
}

/// 候補リストから最高 `blended_score` を持つ候補を選択する。
///
/// 同値時は最初に出現した候補を優先する（安定最大値選択）。
pub fn select_top_candidate(candidates: &[RankedCandidate]) -> Option<&RankedCandidate> {
    candidates.iter().fold(None, |best, candidate| {
        match best {
            None => Some(candidate),
            Some(current) => {
                if candidate.blended_score > current.blended_score {
                    Some(candidate)
                } else {
                    Some(current) // 同値時は先頭維持
                }
            }
        }
    })
}

/// ノイズ注入下で真の最高スコア候補の順位変動をシミュレーションする。
///
/// 各イテレーションで候補のスコアにガウスノイズを注入し、ブレンドスコアを再計算。
/// 真の最高スコア候補（元の blended_score が最大の候補）の順位を追跡する。
pub fn simulate_rank_drift(
    base_candidates: &[RankedCandidate],
    noise_sigma: f64,
    iterations: usize,
    rng: &mut impl rand::Rng,
) -> RankDriftReport {
    // 真の最高候補のインデックスを特定
    let best_idx = base_candidates
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| {
            a.blended_score
                .partial_cmp(&b.blended_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(i, _)| i)
        .unwrap_or(0);

    let best_candidate = &base_candidates[best_idx];

    let mut rank_history: Vec<usize> = Vec::with_capacity(iterations);
    let mut top1_match: usize = 0;

    for _ in 0..iterations {
        let mut noisy_scores: Vec<(usize, f64)> = base_candidates
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let noisy_sem = inject_gaussian_noise(c.semantic_score, noise_sigma, rng);
                let noisy_struct = inject_gaussian_noise(c.structural_score, noise_sigma, rng);
                let blended = compute_blended_score(noisy_sem, noisy_struct);
                (i, blended)
            })
            .collect();

        noisy_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let rank = noisy_scores
            .iter()
            .position(|(i, _)| *i == best_idx)
            .unwrap_or(base_candidates.len());
        rank_history.push(rank);

        if rank == 0 {
            top1_match += 1;
        }
    }

    let top1_match_rate = top1_match as f64 / iterations as f64;

    RankDriftReport {
        true_best_id: best_candidate.workflow_id.clone(),
        iterations,
        noise_sigma,
        rank_history,
        top1_match_rate,
    }
}

/// 順位変動軌道から平均二乗変位 (MSD) 系列を計算する。
///
/// MSD(t) = (1/(N-t)) × Σ_{i=0}^{N-t-1} |x(i+t) - x(i)|²
pub fn compute_mean_squared_displacement(rank_history: &[usize]) -> Vec<f64> {
    let n = rank_history.len();
    if n < 2 {
        return Vec::new();
    }
    let max_lag = (n / 2).min(1000);
    let mut msd: Vec<f64> = Vec::with_capacity(max_lag);

    for lag in 1..=max_lag {
        let mut sum_sq: f64 = 0.0;
        let mut count: usize = 0;
        for i in 0..(n - lag) {
            let diff = rank_history[i + lag] as f64 - rank_history[i] as f64;
            sum_sq += diff * diff;
            count += 1;
        }
        msd.push(sum_sq / count as f64);
    }
    msd
}

/// MSD 系列からハースト指数 H を推定する (log-log 回帰)。
///
/// MSD(t) ∝ t^{2H} の関係を利用し、log(t) vs log(MSD(t)) の傾きから H を推定。
pub fn estimate_hurst_exponent(msd: &[f64]) -> f64 {
    let n = msd.len();
    if n < 4 {
        return 0.5;
    }
    // 初期の過渡領域を除外 (t >= 4 または n/20 の大きい方)
    let start = 4.max(n / 20);
    let mut sum_x: f64 = 0.0;
    let mut sum_y: f64 = 0.0;
    let mut sum_xy: f64 = 0.0;
    let mut sum_xx: f64 = 0.0;
    let mut count: f64 = 0.0;

    for (t, &msd_val) in msd.iter().enumerate().skip(start) {
        if msd_val <= 0.0 {
            continue;
        }
        let t_val = (t + 1) as f64;
        let log_t = t_val.ln();
        let log_msd = msd_val.ln();

        sum_x += log_t;
        sum_y += log_msd;
        sum_xy += log_t * log_msd;
        sum_xx += log_t * log_t;
        count += 1.0;
    }

    if count < 4.0 {
        return 0.5;
    }

    let denominator = count * sum_xx - sum_x * sum_x;
    if denominator.abs() < 1e-12 {
        return 0.5;
    }

    let slope = (count * sum_xy - sum_x * sum_y) / denominator;
    (slope / 2.0).clamp(0.0, 1.0)
}

// ============================================================
// データ構造
// ============================================================

/// ランクドリフトシミュレーションの結果レポート。
pub struct RankDriftReport {
    /// 真の最高スコア候補の workflow_id
    pub true_best_id: String,
    /// シミュレーションイテレーション数
    pub iterations: usize,
    /// 適用したノイズ強度
    pub noise_sigma: f64,
    /// 全イテレーションの順位履歴
    pub rank_history: Vec<usize>,
    /// トップ1一致率（真の最高候補が1位だった割合）
    pub top1_match_rate: f64,
}

// ============================================================
// テスト
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::RankedCandidate;
    use rand::rngs::StdRng;
    use rand::SeedableRng;

    /// テスト用の RankedCandidate を生成するヘルパー。
    fn make_candidate(id: &str, semantic: f64, structural: f64, blended: f64) -> RankedCandidate {
        RankedCandidate {
            workflow_id: id.to_string(),
            semantic_score: semantic,
            structural_score: structural,
            blended_score: blended,
            trust_score: 0.8,
            provenance: vec!["simulated".to_string()],
            metadata: serde_json::json!({}),
        }
    }

    // ============================================================
    // T1: ノイズ注入関数の正常動作
    // ============================================================
    #[test]
    fn noise_injection_clamps_to_range() {
        let mut rng = StdRng::seed_from_u64(crate::constants::TEST_PRNG_SEED);
        let n_trials: usize = 10_000;
        let sigma = crate::constants::NOISE_SIMULATION_SIGMA;

        for _ in 0..n_trials {
            let result = inject_gaussian_noise(0.5, sigma, &mut rng);
            assert!(
                (0.0..=1.0).contains(&result),
                "T1: 結果が [0.0, 1.0] の範囲内であること (got {})",
                result
            );
        }
    }

    // ============================================================
    // T2: 最小ノイズ時の上位選択安定性
    // ============================================================
    #[test]
    fn low_noise_top1_stability() {
        let mut rng = StdRng::seed_from_u64(crate::constants::TEST_PRNG_SEED);
        let n_iter: usize = 1_000;
        let sigma = 0.01;

        let candidates = vec![
            make_candidate("wf-best", 0.9, 0.8, 0.865),
            make_candidate("wf-mid", 0.7, 0.6, 0.635),
            make_candidate("wf-low", 0.5, 0.4, 0.435),
        ];

        let report = simulate_rank_drift(&candidates, sigma, n_iter, &mut rng);
        assert!(
            report.top1_match_rate >= 0.95,
            "T2: 最小ノイズ時 (σ=0.01) のトップ1一致率が 95% 以上であること (got {:.4})",
            report.top1_match_rate
        );
    }

    // ============================================================
    // T3: 中ノイズ時の耐障害性
    // ============================================================
    #[test]
    fn medium_noise_no_crash() {
        let mut rng = StdRng::seed_from_u64(crate::constants::TEST_PRNG_SEED);
        let n_iter: usize = 1_000;
        let sigma = 0.05;

        let candidates = vec![
            make_candidate("wf-a", 0.9, 0.8, 0.865),
            make_candidate("wf-b", 0.7, 0.6, 0.635),
            make_candidate("wf-c", 0.5, 0.4, 0.435),
        ];

        let report = simulate_rank_drift(&candidates, sigma, n_iter, &mut rng);
        assert_eq!(
            report.rank_history.len(),
            n_iter,
            "T3: 全 {} イテレーションの順位履歴が記録されていること",
            n_iter
        );
        println!("T3: σ=0.05, top1_match_rate={:.4}", report.top1_match_rate);
    }

    // ============================================================
    // T4: 高ノイズ時の耐障害性
    // ============================================================
    #[test]
    fn high_noise_no_crash() {
        let mut rng = StdRng::seed_from_u64(crate::constants::TEST_PRNG_SEED);
        let n_iter: usize = 1_000;
        let sigma = 0.20;

        let candidates = vec![
            make_candidate("wf-a", 0.9, 0.8, 0.865),
            make_candidate("wf-b", 0.7, 0.6, 0.635),
        ];

        let report = simulate_rank_drift(&candidates, sigma, n_iter, &mut rng);
        assert_eq!(
            report.rank_history.len(),
            n_iter,
            "T4: 高ノイズ下でも全イテレーションが完了すること"
        );
        println!("T4: σ=0.20, top1_match_rate={:.4}", report.top1_match_rate);
    }

    // ============================================================
    // T5: 範囲外スコアの安全ガード
    // ============================================================
    #[test]
    fn out_of_range_score_guard() {
        let mut rng = StdRng::seed_from_u64(crate::constants::TEST_PRNG_SEED);
        let sigma = 0.05;

        let clamped = inject_gaussian_noise(2.0, sigma, &mut rng);
        assert!(
            (0.0..=1.0).contains(&clamped),
            "T5: 範囲外スコアでも注入結果は [0.0, 1.0] 内であること (got {})",
            clamped
        );

        let clamped_neg = inject_gaussian_noise(-1.0, sigma, &mut rng);
        assert!(
            (0.0..=1.0).contains(&clamped_neg),
            "T5: 負スコアでも注入結果は [0.0, 1.0] 内であること (got {})",
            clamped_neg
        );
    }

    // ============================================================
    // T6: 空候補リスト
    // ============================================================
    #[test]
    fn empty_candidates() {
        let candidates: Vec<RankedCandidate> = vec![];
        let top = select_top_candidate(&candidates);
        assert!(top.is_none(), "T6: 空リストからは None が返ること");
    }

    // ============================================================
    // T7: スコア同値のタイブレーク
    // ============================================================
    #[test]
    fn tie_break_deterministic() {
        let candidates = vec![
            make_candidate("wf-a", 0.5, 0.5, 0.5),
            make_candidate("wf-b", 0.5, 0.5, 0.5),
            make_candidate("wf-c", 0.5, 0.5, 0.5),
        ];

        let r1 = select_top_candidate(&candidates);
        let r2 = select_top_candidate(&candidates);

        assert_eq!(
            r1.map(|c| c.workflow_id.as_str()),
            r2.map(|c| c.workflow_id.as_str()),
            "T7: 同値スコアでも決定論的に選択されること"
        );
        assert_eq!(
            r1.unwrap().workflow_id,
            "wf-a",
            "T7: 同値時は最初の候補が選択されること"
        );
    }

    // ============================================================
    // OTS-1: 順位変動軌道の MSD 解析
    // ============================================================
    #[test]
    fn ots1_msd_analysis() {
        let n_iter: usize = 5_000;
        let noise_levels = [0.01, 0.02, 0.05, 0.10, 0.20];

        println!("=== OTS-1: MSD Analysis (Hurst Exponent) ===");
        println!(
            "iterations={}, noise_levels=[{}]",
            n_iter,
            noise_levels
                .iter()
                .map(|s| format!("{:.2}", s))
                .collect::<Vec<_>>()
                .join(", ")
        );

        for &sigma in &noise_levels {
            let mut rng = StdRng::seed_from_u64(crate::constants::TEST_PRNG_SEED);
            let candidates = vec![
                make_candidate("wf-best", 0.95, 0.90, 0.9325),
                make_candidate("wf-mid", 0.70, 0.65, 0.6675),
                make_candidate("wf-mid2", 0.60, 0.55, 0.5675),
                make_candidate("wf-low", 0.50, 0.45, 0.4675),
                make_candidate("wf-low2", 0.40, 0.35, 0.3675),
            ];

            let report = simulate_rank_drift(&candidates, sigma, n_iter, &mut rng);
            let msd = compute_mean_squared_displacement(&report.rank_history);
            let h = estimate_hurst_exponent(&msd);
            let msd_final = msd.last().copied().unwrap_or(0.0);

            println!(
                "  sigma={:.2}: H={:.4}, msd_final={:.4}, top1_match={:.4}",
                sigma, h, msd_final, report.top1_match_rate
            );
        }

        // 低ノイズ域 (σ=0.01) の H を特別に計測
        let mut rng_low = StdRng::seed_from_u64(crate::constants::TEST_PRNG_SEED);
        let candidates_low = vec![
            make_candidate("wf-best", 0.95, 0.90, 0.9325),
            make_candidate("wf-mid", 0.70, 0.65, 0.6675),
            make_candidate("wf-mid2", 0.60, 0.55, 0.5675),
            make_candidate("wf-low", 0.50, 0.45, 0.4675),
            make_candidate("wf-low2", 0.40, 0.35, 0.3675),
        ];
        let report_low = simulate_rank_drift(&candidates_low, 0.01, n_iter, &mut rng_low);
        let msd_low = compute_mean_squared_displacement(&report_low.rank_history);
        let h_low = estimate_hurst_exponent(&msd_low);

        println!("  => σ=0.01: H={:.4}", h_low);

        if h_low > 0.7 {
            println!("  ⚠️  WARNING: H={:.4} > 0.7, 異常拡散の可能性", h_low);
        }

        assert!(
            report_low.rank_history.len() == n_iter,
            "OTS-1: 全イテレーションが完了すること"
        );
        println!("=== 結果: OTS-1 COMPLETE ===");
    }

    // ============================================================
    // OTS-2: トップ1一致率のノイズ強度依存性
    // ============================================================
    #[test]
    fn ots2_top1_match_rate_vs_noise() {
        let n_iter: usize = 1_000;
        let noise_levels = [0.01, 0.02, 0.05, 0.10, 0.20];

        println!("=== OTS-2: Top-1 Match Rate vs Noise Intensity ===");
        println!("iterations={}, candidates=5", n_iter);
        println!(
            "{:<10} {:<20} {:<20}",
            "sigma", "top1_match_rate", "top1_count"
        );

        let mut rates: Vec<f64> = Vec::new();

        for &sigma in &noise_levels {
            let mut rng = StdRng::seed_from_u64(crate::constants::TEST_PRNG_SEED);
            let candidates = vec![
                make_candidate("wf-best", 0.95, 0.90, 0.9325),
                make_candidate("wf-mid", 0.70, 0.65, 0.6675),
                make_candidate("wf-mid2", 0.60, 0.55, 0.5675),
                make_candidate("wf-low", 0.50, 0.45, 0.4675),
                make_candidate("wf-low2", 0.40, 0.35, 0.3675),
            ];

            let report = simulate_rank_drift(&candidates, sigma, n_iter, &mut rng);
            let match_count = (report.top1_match_rate * n_iter as f64).round() as usize;

            println!(
                "{:<10.2} {:<20.4} {:<20}",
                sigma, report.top1_match_rate, match_count
            );
            rates.push(report.top1_match_rate);
        }

        // 単調減少の観測（統計変動により厳密な単調性は保証されない）
        let is_monotonic = rates.windows(2).all(|w| w[0] >= w[1]);
        println!(
            "  単調減少: {}",
            if is_monotonic {
                "YES"
            } else {
                "NO (許容範囲内の統計変動)"
            }
        );

        assert!(
            rates[0] >= 0.95,
            "OTS-2: σ=0.01 の top1_match_rate が 95% 以上であること (got {:.4})",
            rates[0]
        );

        println!("=== 結果: OTS-2 COMPLETE ===");
    }

    // ============================================================
    // 補助関数の単体テスト
    // ============================================================

    #[test]
    fn blended_score_formula() {
        let sem = 0.8;
        let struct_val = 0.6;
        let expected = (1.0 - BLEND_ALPHA) * sem + BLEND_ALPHA * struct_val;
        let actual = compute_blended_score(sem, struct_val);
        assert!(
            (actual - expected).abs() < 1e-12,
            "ブレンドスコア計算が RFC §12.2 と一致すること (expected={}, got={})",
            expected,
            actual
        );
    }

    #[test]
    fn select_top_candidate_returns_highest() {
        let candidates = vec![
            make_candidate("wf-low", 0.5, 0.4, 0.435),
            make_candidate("wf-high", 0.9, 0.8, 0.865),
            make_candidate("wf-mid", 0.7, 0.6, 0.635),
        ];
        let top = select_top_candidate(&candidates);
        assert!(top.is_some());
        assert_eq!(top.unwrap().workflow_id, "wf-high");
    }

    #[test]
    fn msd_computation_simple_case() {
        let history = vec![0usize, 0, 0, 0, 0];
        let msd = compute_mean_squared_displacement(&history);
        for val in &msd {
            assert!(*val < 1e-12, "MSD が 0 であること");
        }
    }

    #[test]
    fn msd_empty_history() {
        let msd = compute_mean_squared_displacement(&[]);
        assert!(msd.is_empty(), "空履歴からは空の MSD が返ること");
    }

    #[test]
    fn hurst_estimation_short_history() {
        // 短い履歴では 0.5 を返す
        let h = estimate_hurst_exponent(&[0.0, 1.0]);
        assert!((h - 0.5).abs() < 1e-12, "短履歴では H=0.5");
    }
}
