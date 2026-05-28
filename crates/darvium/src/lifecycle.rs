// ライフサイクルスコアモジュール (RFC §15.3)
//
// LifecycleScore L(G) は個人の適合度を 5 成分の幾何平均として評価する。
// 自然淘汰における「適応度」を定量化し、GC ハザード計算の入力となる。
//
// RFC §4A.7 機構 39, §15.9.3 mean_lifecycle_score

use crate::clock::compute_blended_freshness;
use crate::constants::PHASE4_FRESHNESS_HUMAN_WEIGHT;
use crate::event::{transition_gc_state, ReciprocityLifecyclePolicy};
use crate::reciprocity::{compute_experience_normalization, compute_gc_hazard};
use crate::MemoizedGraph;

/// 個人のライフサイクルスコアを構成する 5 成分 (RFC §15.3)。
///
/// 幾何平均 L(G) = (freshness × success × trust × usage × reputation) ^ (1/5)
/// 全成分は [0, 1] に正規化されていることを前提とする。
#[derive(Debug, Clone, PartialEq)]
pub struct LifecycleScore {
    /// 時間鮮度 F_time — BlendedFreshness の出力
    pub freshness: f64,
    /// 実行成功率 — 成功 step / 全 step
    pub success: f64,
    /// 信頼複合値 — TrustProfile の複合値
    pub trust: f64,
    /// 使用度 — ワークフローが再利用/参照された頻度の正規化値
    pub usage: f64,
    /// 評判スコア — ReputationProfile の最終合成値
    pub reputation: f64,
}

/// 5 成分の幾何平均として LifecycleScore の複合値を計算する。
///
/// 幾何平均を用いることで、1 成分でも 0 に近づけば全体が強く減衰する。
/// 全成分が 1.0 の場合のみ 1.0 を返す。
///
/// # 引数
/// - `score`: LifecycleScore の 5 成分
///
/// # 戻り値
/// - [0, 1] 範囲の複合スコア。全ての成分が 1.0 の場合 1.0。
///
/// # 不変条件
/// - 全成分は [0, 1] 範囲内であること (MUST)
/// - 戻り値は [0, 1] 範囲内であること (MUST)
pub fn compute_lifecycle_score(score: &LifecycleScore) -> f64 {
    let product = score.freshness * score.success * score.trust * score.usage * score.reputation;
    product.powf(1.0 / 5.0)
}

/// LifecycleScore → GC hazard → gc_state 遷移を単一グラフに対して実行する。
///
/// シミュレーション (`phase4_gc_survival`) とプロダクション (`run_lifecycle_gc`) の
/// 間で重複していた GC パイプライン計算を一元化する。
///
/// # 処理
/// 1. BlendedFreshness から freshness を計算
/// 2. TrustProfile 3成分の平均値で trust を計算
/// 3. experience_count から usage を計算
/// 4. final_score から reputation を取得
/// 5. 上記5成分で LifecycleScore を計算
/// 6. compute_gc_hazard でハザード値を算出
/// 7. transition_gc_state で gc_state を遷移
/// 8. 状態変化時は graph の gc_state / last_update_tick を更新
///
/// # 戻り値
/// GC ハザード値（呼び出し元で生存確率判定等に利用可能）。
pub(crate) fn compute_and_update_gc_state(
    graph: &mut MemoizedGraph,
    elapsed: u64,
    success: f64,
    child_protection: f32,
    policy: &ReciprocityLifecyclePolicy,
    now: u64,
) -> f64 {
    let freshness = compute_blended_freshness(0, elapsed, PHASE4_FRESHNESS_HUMAN_WEIGHT);
    let trust = (graph.trust.operational + graph.trust.semantic + graph.trust.temporal) / 3.0;
    let usage = compute_experience_normalization(graph.experience_count);
    let reputation = graph.reputation.final_score as f64;

    let lifecycle_score = compute_lifecycle_score(&LifecycleScore {
        freshness,
        success,
        trust,
        usage,
        reputation,
    });

    let benevolence = graph.reputation.benevolence_score;
    let hazard = compute_gc_hazard(lifecycle_score as f32, benevolence, child_protection, policy);
    let new_gc_state = transition_gc_state(graph.gc_state, hazard as f64);
    if new_gc_state != graph.gc_state {
        graph.gc_state = new_gc_state;
        graph.last_update_tick = now;
    }

    hazard as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    // TC2: LifecycleScore 全成分 1.0 → 複合値 1.0
    #[test]
    fn test_lifecycle_score_all_max() {
        let score = LifecycleScore {
            freshness: 1.0,
            success: 1.0,
            trust: 1.0,
            usage: 1.0,
            reputation: 1.0,
        };
        let result = compute_lifecycle_score(&score);
        assert!(
            (result - 1.0).abs() < f64::EPSILON,
            "全成分 1.0 → 複合値 1.0 (got {})",
            result
        );
    }

    // TC2: LifecycleScore 全成分 0.0 → 複合値 0.0
    #[test]
    fn test_lifecycle_score_all_zero() {
        let score = LifecycleScore {
            freshness: 0.0,
            success: 0.0,
            trust: 0.0,
            usage: 0.0,
            reputation: 0.0,
        };
        let result = compute_lifecycle_score(&score);
        assert!(
            (result - 0.0).abs() < f64::EPSILON,
            "全成分 0.0 → 複合値 0.0 (got {})",
            result
        );
    }

    // TC3: 1 成分 0.0 → 全体 0.0（幾何平均の性質）
    #[test]
    fn test_lifecycle_score_one_zero() {
        let score = LifecycleScore {
            freshness: 0.0,
            success: 0.8,
            trust: 0.9,
            usage: 0.7,
            reputation: 0.85,
        };
        let result = compute_lifecycle_score(&score);
        assert!(
            (result - 0.0).abs() < f64::EPSILON,
            "1 成分 0.0 で全体 0.0 (got {})",
            result
        );
    }

    // TC2: LifecycleScore 中間値
    #[test]
    fn test_lifecycle_score_mid_values() {
        let score = LifecycleScore {
            freshness: 0.5,
            success: 0.5,
            trust: 0.5,
            usage: 0.5,
            reputation: 0.5,
        };
        let result = compute_lifecycle_score(&score);
        let expected = 0.5_f64;
        assert!(
            (result - expected).abs() < 1e-10,
            "全成分 0.5 → 複合値 0.5 (got {})",
            result
        );
    }

    // FIX-B3: usage=0.095（experience=0）、他成分 0.8 → lifecycle_score > 0.3
    #[test]
    fn test_fixb_lifecycle_score_positive() {
        let score = LifecycleScore {
            freshness: 0.8,
            success: 0.8,
            trust: 0.8,
            usage: 0.095, // compute_experience_normalization(0) with OFFSET=1.0
            reputation: 0.8,
        };
        let result = compute_lifecycle_score(&score);
        assert!(
            result > 0.3,
            "FIX-B3: usage=0.095, others=0.8 → lifecycle_score > 0.3 (got {})",
            result
        );
        assert!(
            result < 0.9,
            "FIX-B3: lifecycle_score must be < 0.9 for this input (got {})",
            result
        );
    }
}
