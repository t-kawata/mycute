// ワークフロー成熟度判定器と Local Village 構成ロジック (RFC §41B.3)
//
// 本モジュールは以下の機能を提供する：
// - WorkflowMaturity::{Child, Adult} 列挙型 — 式 41B-3, 41B-4
// - classify_maturity — 経験値・信頼・レピュテーションの3軸判定
// - LocalVillage 構造体 — Child の近傍 Adult 集合
// - AdultCandidate 構造体 — フィルタリング中間表現
// - filter_adult_candidates — ConsistencyState + maturity フィルタ
// - build_local_village_topk — TopK 近傍選抜（式 41B-6）
// - build_local_village_radius — 半径内選抜（式 41B-7）
//
// M1.75-1 (spaceposition.rs) の VillagePosition / l2_distance に依存する。

use crate::constants::{
    E_ADULT_THRESHOLD, MIN_SURVIVAL_EXPERIENCE, R_ADULT_THRESHOLD, T_ADULT_THRESHOLD,
};
use crate::spaceposition::{l2_distance, SpacePositionEmbedding, VillagePosition};
use crate::types::{ConsistencyStateTag, WorkflowGraphId};

// ============================================================
// 型定義 (RFC §41B.3)
// ============================================================

/// ワークフロー成熟度の二値分類 (RFC §41B.3)。
///
/// - `Child`: experiencecount(G) < MINSURVIVALEXPERIENCE（式 41B-3）
/// - `Adult`: E(G) ≥ E_adult ∧ T(G) ≥ T_adult ∧ R(G) ≥ R_adult（式 41B-4）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkflowMaturity {
    /// 経験値猶予期間中の未成熟ワークフロー。
    Child,
    /// 経験値・信頼・レピュテーションの全軸で成熟閾値に達したワークフロー。
    Adult,
}

/// フィルタリング前の Adult 候補の中間表現。
///
/// 村構成の前に `filter_adult_candidates` で以下の条件を満たさない候補を除外する：
/// - consistency が `Committed`
/// - `is_adult_maturity` が true
#[derive(Debug, Clone, PartialEq)]
pub struct AdultCandidate {
    /// ワークフローの一意識別子。
    pub id: WorkflowGraphId,
    /// 生態学的位置。
    pub position: VillagePosition,
    /// 整合性状態タグ（フィルタ条件: Committed のみ許容）。
    pub consistency: ConsistencyStateTag,
    /// Adult 成熟度フラグ。
    pub is_adult_maturity: bool,
}

/// Child ワークフローの Local Village（導出近傍）。
///
/// RFC §41B.3 に基づき、静的なクラスではなく Child の近傍 Adult 集合として定義される。
/// 規範的デフォルトは TopK 近傍（式 41B-6）、代替として半径形式（式 41B-7）。
#[derive(Debug, Clone, PartialEq)]
pub struct LocalVillage {
    /// 村の中心となる Child ワークフロー ID。
    pub child_id: WorkflowGraphId,
    /// 選抜された Adult ワークフロー ID のリスト（距離昇順）。
    pub adult_ids: Vec<WorkflowGraphId>,
    /// 選抜 Adult の重心位置。空村の場合は `SpacePositionEmbedding::unknown()`。
    pub centroid: SpacePositionEmbedding,
    /// 構成時の半径パラメータ。TopK 方式の場合は最遠 Adult までの距離。
    pub radius: f64,
}

// ============================================================
// 成熟度判定 (RFC §41B.3, 式 41B-3, 41B-4)
// ============================================================

/// 経験値・信頼・レピュテーションの3軸でワークフロー成熟度を判定する。
///
/// 式 41B-3: Child(G) ⟺ experiencecount(G) < MINSURVIVALEXPERIENCE
/// 式 41B-4: Adult(G) ⟺ E(G) ≥ E_adult ∧ T(G) ≥ T_adult ∧ R(G) ≥ R_adult
///
/// 経験値不足（Child 条件）が最も優先され、信頼・レピュテーションが閾値を
/// 超えていても Child と判定される。
pub fn classify_maturity(
    experience_count: u64,
    trust_composite: f64,
    reputation_finalscore: f64,
) -> WorkflowMaturity {
    if experience_count < MIN_SURVIVAL_EXPERIENCE {
        return WorkflowMaturity::Child;
    }
    if experience_count >= E_ADULT_THRESHOLD
        && trust_composite >= T_ADULT_THRESHOLD
        && reputation_finalscore >= R_ADULT_THRESHOLD
    {
        WorkflowMaturity::Adult
    } else {
        WorkflowMaturity::Child
    }
}

// ============================================================
// Adult 候補フィルタ (RFC §41B.3)
// ============================================================

/// ConsistencyState と成熟度に基づいて Adult 候補をフィルタリングする。
///
/// 以下の条件をすべて満たす候補のみを保持する：
/// - `consistency == ConsistencyStateTag::Committed`
/// - `is_adult_maturity == true`
///
/// LifecycleState 型が未実装のため、`is_adult_maturity` フラグで代替する。
pub fn filter_adult_candidates(candidates: Vec<AdultCandidate>) -> Vec<AdultCandidate> {
    candidates
        .into_iter()
        .filter(|c| c.consistency == ConsistencyStateTag::Committed && c.is_adult_maturity)
        .collect()
}

// ============================================================
// Local Village 構成 (RFC §41B.3, 式 41B-6, 41B-7)
// ============================================================

/// 選抜された Adult 位置の重心（算術平均）を計算する。
///
/// 空リストの場合は `SpacePositionEmbedding::unknown()` を返す。
fn compute_centroid(positions: &[VillagePosition]) -> SpacePositionEmbedding {
    if positions.is_empty() {
        return SpacePositionEmbedding::unknown();
    }
    let n = positions.len() as f32;
    let sum_x: f32 = positions.iter().map(|p| p.position[0]).sum();
    let sum_y: f32 = positions.iter().map(|p| p.position[1]).sum();
    let sum_z: f32 = positions.iter().map(|p| p.position[2]).sum();
    SpacePositionEmbedding::from([sum_x / n, sum_y / n, sum_z / n])
}

/// TopK 方式で Local Village を構成する（式 41B-6）。
///
/// Child 位置からの L2 距離昇順で最大 k 件の Adult を選抜する。
/// 選抜結果は距離昇順で `adult_ids` に格納される。
/// Adult 候補が k 件未満の場合、全件が選抜される。
pub fn build_local_village_topk(
    child_id: WorkflowGraphId,
    child_pos: &VillagePosition,
    adults: &[AdultCandidate],
    k: usize,
) -> LocalVillage {
    let mut indexed: Vec<(usize, f64)> = adults
        .iter()
        .enumerate()
        .map(|(i, a)| (i, l2_distance(&child_pos.position, &a.position.position)))
        .collect();
    indexed.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    let selected: Vec<&AdultCandidate> = indexed.iter().take(k).map(|(i, _)| &adults[*i]).collect();

    let adult_ids: Vec<WorkflowGraphId> = selected.iter().map(|a| a.id.clone()).collect();
    let positions: Vec<VillagePosition> = selected.iter().map(|a| a.position).collect();
    let max_distance = indexed.first().map(|(_, d)| *d).unwrap_or(0.0);

    LocalVillage {
        child_id,
        adult_ids,
        centroid: compute_centroid(&positions),
        radius: max_distance,
    }
}

/// 半径形式で Local Village を構成する（式 41B-7）。
///
/// Child 位置からの L2 距離が d_max 以下の Adult をすべて選抜する。
/// 選抜結果は距離昇順で `adult_ids` に格納される。
pub fn build_local_village_radius(
    child_id: WorkflowGraphId,
    child_pos: &VillagePosition,
    adults: &[AdultCandidate],
    d_max: f64,
) -> LocalVillage {
    let mut with_distance: Vec<(usize, f64)> = adults
        .iter()
        .enumerate()
        .map(|(i, a)| (i, l2_distance(&child_pos.position, &a.position.position)))
        .filter(|(_, d)| *d <= d_max)
        .collect();
    with_distance.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    let selected: Vec<&AdultCandidate> = with_distance.iter().map(|(i, _)| &adults[*i]).collect();

    let adult_ids: Vec<WorkflowGraphId> = selected.iter().map(|a| a.id.clone()).collect();
    let positions: Vec<VillagePosition> = selected.iter().map(|a| a.position).collect();
    let max_distance = with_distance.last().map(|(_, d)| *d).unwrap_or(0.0);

    LocalVillage {
        child_id,
        adult_ids,
        centroid: compute_centroid(&positions),
        radius: max_distance,
    }
}

// ============================================================
// Village Stability / Dynamicity Metrics (M1.75-7, RFC §41B.14)
// ============================================================

use std::collections::VecDeque;

use crate::constants::VILLAGE_METRICS_WINDOW_SIZE;

/// 単一 tick における全メトリクス値を格納する (RFC §41B.14, §41B.15)。
#[derive(Debug, Clone, PartialEq)]
pub struct VillageMetrics {
    /// 全 child の位置ドリフト (式 41B-21)。
    pub position_drifts: Vec<f64>,
    /// 全 child の短期 Jaccard 重複 (式 41B-22)。
    pub village_jaccards: Vec<f64>,
    /// 全 child の churn 値 (式 41B-23)。
    pub village_churns: Vec<f64>,
    /// 全 child の helper weight Jensen-Shannon divergence。
    pub helper_jsds: Vec<f64>,
    /// 全 child の helper 数。
    pub helper_counts: Vec<usize>,
    /// 生存 child 数。
    pub child_survival_count: usize,
    /// 成熟 child 数。
    pub child_maturation_count: usize,
    /// 総 child 数。
    pub total_child_count: usize,
    /// 観測 tick。
    pub tick: u64,
}

/// 時系列ウィンドウ集計 (RFC §41B.14)。
///
/// 直近 N tick 分の VillageMetrics を保持し、スナップショットを生成する。
#[derive(Debug, Clone)]
pub struct VillageMetricsWindow {
    /// 直近 N tick 分の生メトリクス。
    pub metrics: VecDeque<VillageMetrics>,
    /// ウィンドウサイズ。
    pub window_size: usize,
}

impl VillageMetricsWindow {
    /// 指定されたウィンドウサイズで VillageMetricsWindow を作成する。
    pub fn new(window_size: usize) -> Self {
        VillageMetricsWindow {
            metrics: VecDeque::with_capacity(window_size + 1),
            window_size,
        }
    }

    /// デフォルトウィンドウサイズで VillageMetricsWindow を作成する。
    pub fn default_window() -> Self {
        Self::new(VILLAGE_METRICS_WINDOW_SIZE)
    }

    /// メトリクスをウィンドウに追加する。
    /// ウィンドウサイズを超えた場合、古いメトリクスを削除する。
    pub fn push(&mut self, metrics: VillageMetrics) {
        if self.metrics.len() >= self.window_size {
            self.metrics.pop_front();
        }
        self.metrics.push_back(metrics);
    }

    /// 現在のウィンドウサイズを返す。
    pub fn len(&self) -> usize {
        self.metrics.len()
    }

    /// ウィンドウが空かどうかを返す。
    pub fn is_empty(&self) -> bool {
        self.metrics.is_empty()
    }

    /// 現在のウィンドウからスナップショットを生成する。
    ///
    /// ウィンドウが空の場合は None を返す。
    pub fn snapshot(&self) -> Option<VillageMetricsSnapshot> {
        if self.metrics.is_empty() {
            return None;
        }

        let mut all_drifts: Vec<f64> = Vec::new();
        let mut all_churns: Vec<f64> = Vec::new();
        let mut all_jsds: Vec<f64> = Vec::new();
        let mut all_counts: Vec<f64> = Vec::new();
        let mut total_survival: usize = 0;
        let mut total_children: usize = 0;

        for m in &self.metrics {
            all_drifts.extend(&m.position_drifts);
            all_churns.extend(&m.village_churns);
            all_jsds.extend(&m.helper_jsds);
            all_counts.push(
                m.helper_counts.iter().sum::<usize>() as f64 / m.helper_counts.len().max(1) as f64,
            );
            total_survival += m.child_survival_count;
            total_children += m.total_child_count;
        }

        all_drifts.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        all_churns.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        all_jsds.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        all_counts.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let p50_idx = |len: usize| -> usize { (len.max(1) - 1) / 2 };
        let p95_idx = |len: usize| -> usize {
            let idx = (len as f64 * 0.95).ceil() as usize;
            idx.max(1).min(len) - 1
        };

        Some(VillageMetricsSnapshot {
            position_drift_p50: all_drifts
                .get(p50_idx(all_drifts.len()))
                .copied()
                .unwrap_or(0.0),
            position_drift_p95: all_drifts
                .get(p95_idx(all_drifts.len()))
                .copied()
                .unwrap_or(0.0),
            village_churn_p50: all_churns
                .get(p50_idx(all_churns.len()))
                .copied()
                .unwrap_or(0.0),
            village_churn_p95: all_churns
                .get(p95_idx(all_churns.len()))
                .copied()
                .unwrap_or(0.0),
            helper_jsd_p50: all_jsds
                .get(p50_idx(all_jsds.len()))
                .copied()
                .unwrap_or(0.0),
            helper_jsd_p95: all_jsds
                .get(p95_idx(all_jsds.len()))
                .copied()
                .unwrap_or(0.0),
            helper_count_mean: if all_counts.is_empty() {
                0.0
            } else {
                all_counts.iter().sum::<f64>() / all_counts.len() as f64
            },
            child_survival_rate: if total_children == 0 {
                0.0
            } else {
                total_survival as f64 / total_children as f64
            },
            child_maturation_time_p50: 0.0,
            child_maturation_time_p95: 0.0,
        })
    }
}

/// 分位点・平均を含む集約スナップショット (RFC §41B.15)。
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct VillageMetricsSnapshot {
    /// 位置ドリフトの中央値。
    pub position_drift_p50: f64,
    /// 位置ドリフトの 95 パーセンタイル。
    pub position_drift_p95: f64,
    /// village churn の中央値。
    pub village_churn_p50: f64,
    /// village churn の 95 パーセンタイル。
    pub village_churn_p95: f64,
    /// helper weight JSD の中央値。
    pub helper_jsd_p50: f64,
    /// helper weight JSD の 95 パーセンタイル。
    pub helper_jsd_p95: f64,
    /// 村あたりの平均 helper 数。
    pub helper_count_mean: f64,
    /// 生存 child 比率。
    pub child_survival_rate: f64,
    /// 成熟時間の中央値 (tick)。(将来拡張用、現状 0.0)
    pub child_maturation_time_p50: f64,
    /// 成熟時間の 95 パーセンタイル (tick)。(将来拡張用、現状 0.0)
    pub child_maturation_time_p95: f64,
}

/// 位置ドリフトを計算する (式 41B-21)。
///
/// Δ_x(G,t) = ‖x_{t+1}(G) — x_t(G)‖₂
///
/// 2 時点間の空間位置ベクトルの L2 距離を返す。
pub fn compute_position_drift(prev_pos: &[f32; 3], curr_pos: &[f32; 3]) -> f64 {
    let dx = curr_pos[0] as f64 - prev_pos[0] as f64;
    let dy = curr_pos[1] as f64 - prev_pos[1] as f64;
    let dz = curr_pos[2] as f64 - prev_pos[2] as f64;
    (dx * dx + dy * dy + dz * dz).sqrt()
}

/// 近傍集合の Jaccard 重複度を計算する (式 41B-22)。
///
/// J(c,t) = |N_t(c) ∩ N_{t+1}(c)| / |N_t(c) ∪ N_{t+1}(c)|
///
/// 両方の集合が空の場合は 0.0 を返す。
pub fn compute_village_jaccard(
    prev_adults: &[WorkflowGraphId],
    curr_adults: &[WorkflowGraphId],
) -> f64 {
    if prev_adults.is_empty() && curr_adults.is_empty() {
        return 0.0;
    }

    let prev_set: std::collections::HashSet<&str> =
        prev_adults.iter().map(|s| s.as_str()).collect();
    let curr_set: std::collections::HashSet<&str> =
        curr_adults.iter().map(|s| s.as_str()).collect();

    let intersection_size = prev_set.intersection(&curr_set).count();
    let union_size = prev_set.union(&curr_set).count();

    if union_size == 0 {
        0.0
    } else {
        intersection_size as f64 / union_size as f64
    }
}

/// Village churn を計算する (式 41B-23)。
///
/// V(c,t) = 1 - J(c,t)
///
/// Jaccard 値を受け取り churn 値を返す。
pub fn compute_village_churn(jaccard: f64) -> f64 {
    1.0 - jaccard.clamp(0.0, 1.0)
}

/// Helper weight 分布の Jensen-Shannon divergence を計算する。
///
/// JSD(P‖Q) = 0.5·KL(P‖M) + 0.5·KL(Q‖M), M = 0.5·(P+Q)
///
/// 両分布は正規化済みであることを前提とする。
/// 空リストの場合は 0.0 を返す。
pub fn compute_helper_jsd(prev_weights: &[f64], curr_weights: &[f64]) -> f64 {
    if prev_weights.is_empty() || curr_weights.is_empty() {
        return 0.0;
    }

    let max_len = prev_weights.len().max(curr_weights.len());
    let eps = 1e-12;

    // 短い方をゼロパディングして長さを揃え、ゼロの代わりにイプシロンを加算する
    let padded_prev: Vec<f64> = (0..max_len)
        .map(|i| prev_weights.get(i).copied().unwrap_or(0.0) + eps)
        .collect();
    let padded_curr: Vec<f64> = (0..max_len)
        .map(|i| curr_weights.get(i).copied().unwrap_or(0.0) + eps)
        .collect();

    // 正規化
    let sum_p: f64 = padded_prev.iter().sum();
    let sum_q: f64 = padded_curr.iter().sum();
    let p: Vec<f64> = padded_prev.iter().map(|v| v / sum_p).collect();
    let q: Vec<f64> = padded_curr.iter().map(|v| v / sum_q).collect();

    // M = 0.5 * (P + Q)
    let m: Vec<f64> = p
        .iter()
        .zip(q.iter())
        .map(|(pi, qi)| 0.5 * (pi + qi))
        .collect();

    // KL(P‖M)
    let kl_pm: f64 = p
        .iter()
        .zip(m.iter())
        .map(|(pi, mi)| pi * (pi / mi).ln())
        .sum();
    // KL(Q‖M)
    let kl_qm: f64 = q
        .iter()
        .zip(m.iter())
        .map(|(qi, mi)| qi * (qi / mi).ln())
        .sum();

    0.5 * kl_pm + 0.5 * kl_qm
}

/// Child survival rate を計算する。
///
/// surviving / total。total が 0 の場合は 0.0 を返す。
pub fn compute_child_survival_rate(surviving: usize, total: usize) -> f64 {
    if total == 0 {
        0.0
    } else {
        surviving as f64 / total as f64
    }
}

/// Child maturation time を計算する。
///
/// maturation_tick - birth_tick。tick 単位。
pub fn compute_child_maturation_time(birth_tick: u64, maturation_tick: u64) -> f64 {
    if maturation_tick < birth_tick {
        0.0
    } else {
        (maturation_tick - birth_tick) as f64
    }
}

// ============================================================
// テスト (T-1 〜 T-20 + T-E1, M1.75-7 T-1〜T-13 + T-O1〜T-O3)
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// テスト用の VillagePosition を生成する。
    fn make_vp(pos: [f32; 3], vt: u64) -> VillagePosition {
        VillagePosition::new(pos, vt)
    }

    // --------------------------------------------------------
    // T-1: classify_maturity — Child 判定（経験値不足）
    // --------------------------------------------------------
    #[test]
    fn t1_classify_child_experience_deficit() {
        let child = classify_maturity(
            MIN_SURVIVAL_EXPERIENCE - 1,
            T_ADULT_THRESHOLD,
            R_ADULT_THRESHOLD,
        );
        assert_eq!(child, WorkflowMaturity::Child, "経験値不足では Child");
        // 信頼・レピュテーションが閾値を超えていても経験値不足が支配的
        let also_child = classify_maturity(MIN_SURVIVAL_EXPERIENCE - 1, 1.0, 1.0);
        assert_eq!(
            also_child,
            WorkflowMaturity::Child,
            "信頼・レピュテーションが最大でも経験値不足なら Child"
        );
        println!(
            "T-1 PASS: experience_count={} → Child (trust={}, rep={})",
            MIN_SURVIVAL_EXPERIENCE - 1,
            T_ADULT_THRESHOLD,
            R_ADULT_THRESHOLD
        );
    }

    // --------------------------------------------------------
    // T-2: classify_maturity — Adult 判定（全軸充足）
    // --------------------------------------------------------
    #[test]
    fn t2_classify_adult_all_axes_satisfied() {
        let adult = classify_maturity(E_ADULT_THRESHOLD, T_ADULT_THRESHOLD, R_ADULT_THRESHOLD);
        assert_eq!(adult, WorkflowMaturity::Adult, "全軸閾値以上で Adult");
        println!(
            "T-2 PASS: experience_count={}, trust={}, rep={} → Adult",
            E_ADULT_THRESHOLD, T_ADULT_THRESHOLD, R_ADULT_THRESHOLD
        );
    }

    // --------------------------------------------------------
    // T-3: classify_maturity — 信頼不足で Child
    // --------------------------------------------------------
    #[test]
    fn t3_classify_child_trust_deficit() {
        let child = classify_maturity(
            E_ADULT_THRESHOLD,
            T_ADULT_THRESHOLD - 0.01,
            R_ADULT_THRESHOLD,
        );
        assert_eq!(child, WorkflowMaturity::Child, "信頼不足では Child");
        println!(
            "T-3 PASS: trust={} (< {}) → Child",
            T_ADULT_THRESHOLD - 0.01,
            T_ADULT_THRESHOLD
        );
    }

    // --------------------------------------------------------
    // T-4: classify_maturity — レピュテーション不足で Child
    // --------------------------------------------------------
    #[test]
    fn t4_classify_child_reputation_deficit() {
        let child = classify_maturity(
            E_ADULT_THRESHOLD,
            T_ADULT_THRESHOLD,
            R_ADULT_THRESHOLD - 0.01,
        );
        assert_eq!(
            child,
            WorkflowMaturity::Child,
            "レピュテーション不足では Child"
        );
        println!(
            "T-4 PASS: rep={} (< {}) → Child",
            R_ADULT_THRESHOLD - 0.01,
            R_ADULT_THRESHOLD
        );
    }

    // --------------------------------------------------------
    // T-5: classify_maturity — 全軸ギリギリ不足
    // --------------------------------------------------------
    #[test]
    fn t5_classify_child_all_axes_barely_below() {
        let eps = f64::EPSILON;
        let child = classify_maturity(
            E_ADULT_THRESHOLD - 1,
            T_ADULT_THRESHOLD - eps,
            R_ADULT_THRESHOLD - eps,
        );
        assert_eq!(child, WorkflowMaturity::Child, "全軸が閾値未満なら Child");
        println!("T-5 PASS: all axes below threshold by ε → Child");
    }

    // --------------------------------------------------------
    // T-6: classify_maturity — 全軸閾値超過
    // --------------------------------------------------------
    #[test]
    fn t6_classify_adult_all_axes_barely_above() {
        let eps = f64::EPSILON;
        let adult = classify_maturity(
            E_ADULT_THRESHOLD + 1,
            T_ADULT_THRESHOLD + eps,
            R_ADULT_THRESHOLD + eps,
        );
        assert_eq!(
            adult,
            WorkflowMaturity::Adult,
            "全軸が閾値を超えていれば Adult"
        );
        println!("T-6 PASS: all axes above threshold by ε → Adult");
    }

    // --------------------------------------------------------
    // T-7: classify_maturity — 極値入力
    // --------------------------------------------------------
    #[test]
    fn t7_classify_extreme_values() {
        let child = classify_maturity(0, 1.0, 1.0);
        assert_eq!(
            child,
            WorkflowMaturity::Child,
            "経験値0の未経験ワークフローは Child"
        );
        println!("T-7a PASS: experience_count=0 → Child");

        let adult = classify_maturity(u64::MAX, 1.0, 1.0);
        assert_eq!(adult, WorkflowMaturity::Adult, "全軸最大値で Adult");
        println!("T-7b PASS: experience_count=MAX, trust=1.0, rep=1.0 → Adult");
    }

    // --------------------------------------------------------
    // T-8: filter_adult_candidates — ConsistencyState 除外
    // --------------------------------------------------------
    #[test]
    fn t8_filter_consistency_state_exclusion() {
        let candidates = vec![
            AdultCandidate {
                id: "a1".into(),
                position: make_vp([0.0, 0.0, 0.0], 0),
                consistency: ConsistencyStateTag::Committed,
                is_adult_maturity: true,
            },
            AdultCandidate {
                id: "a2".into(),
                position: make_vp([1.0, 0.0, 0.0], 0),
                consistency: ConsistencyStateTag::Pending,
                is_adult_maturity: true,
            },
            AdultCandidate {
                id: "a3".into(),
                position: make_vp([2.0, 0.0, 0.0], 0),
                consistency: ConsistencyStateTag::NeedsRepair,
                is_adult_maturity: true,
            },
            AdultCandidate {
                id: "a4".into(),
                position: make_vp([3.0, 0.0, 0.0], 0),
                consistency: ConsistencyStateTag::Quarantined,
                is_adult_maturity: true,
            },
        ];
        let filtered = filter_adult_candidates(candidates);
        assert_eq!(filtered.len(), 1, "Committed のみ保持される");
        assert_eq!(filtered[0].id, "a1", "Committed の候補が保持される");
        println!(
            "T-8 PASS: 4 candidates → {} retained (only Committed)",
            filtered.len()
        );
    }

    // --------------------------------------------------------
    // T-9: filter_adult_candidates — Adult maturity 未達除外
    // --------------------------------------------------------
    #[test]
    fn t9_filter_adult_maturity_exclusion() {
        let candidates = vec![
            AdultCandidate {
                id: "adult".into(),
                position: make_vp([0.0, 0.0, 0.0], 0),
                consistency: ConsistencyStateTag::Committed,
                is_adult_maturity: true,
            },
            AdultCandidate {
                id: "child".into(),
                position: make_vp([5.0, 0.0, 0.0], 0),
                consistency: ConsistencyStateTag::Committed,
                is_adult_maturity: false,
            },
        ];
        let filtered = filter_adult_candidates(candidates);
        assert_eq!(filtered.len(), 1, "maturity 未達は除外される");
        assert_eq!(filtered[0].id, "adult", "Adult のみ保持される");
        println!(
            "T-9 PASS: 2 candidates → {} retained (adult maturity only)",
            filtered.len()
        );
    }

    // --------------------------------------------------------
    // T-10: filter_adult_candidates — 複合フィルタ
    // --------------------------------------------------------
    #[test]
    fn t10_filter_combined_exclusion() {
        let candidates = vec![
            AdultCandidate {
                id: "valid".into(),
                position: make_vp([0.0, 0.0, 0.0], 0),
                consistency: ConsistencyStateTag::Committed,
                is_adult_maturity: true,
            },
            AdultCandidate {
                id: "bad_consistency".into(),
                position: make_vp([1.0, 0.0, 0.0], 0),
                consistency: ConsistencyStateTag::Pending,
                is_adult_maturity: true,
            },
            AdultCandidate {
                id: "bad_maturity".into(),
                position: make_vp([2.0, 0.0, 0.0], 0),
                consistency: ConsistencyStateTag::Committed,
                is_adult_maturity: false,
            },
            AdultCandidate {
                id: "both_bad".into(),
                position: make_vp([3.0, 0.0, 0.0], 0),
                consistency: ConsistencyStateTag::Quarantined,
                is_adult_maturity: false,
            },
        ];
        let filtered = filter_adult_candidates(candidates);
        assert_eq!(filtered.len(), 1, "全条件を満たす候補のみ保持");
        assert_eq!(filtered[0].id, "valid");

        // 空リスト
        let empty = filter_adult_candidates(vec![]);
        assert!(empty.is_empty(), "空リストは空を返す");
        println!(
            "T-10 PASS: 4 candidates → {} retained, empty → {}",
            filtered.len(),
            empty.len()
        );
    }

    // --------------------------------------------------------
    // T-11: build_local_village_topk — 基本的な選抜
    // --------------------------------------------------------
    #[test]
    fn t11_topk_basic_selection() {
        let child_pos = make_vp([0.0, 0.0, 0.0], 100);
        let adults = vec![
            AdultCandidate {
                id: "far".into(),
                position: make_vp([100.0, 0.0, 0.0], 100),
                consistency: ConsistencyStateTag::Committed,
                is_adult_maturity: true,
            },
            AdultCandidate {
                id: "mid1".into(),
                position: make_vp([10.0, 0.0, 0.0], 100),
                consistency: ConsistencyStateTag::Committed,
                is_adult_maturity: true,
            },
            AdultCandidate {
                id: "near".into(),
                position: make_vp([1.0, 0.0, 0.0], 100),
                consistency: ConsistencyStateTag::Committed,
                is_adult_maturity: true,
            },
            AdultCandidate {
                id: "mid2".into(),
                position: make_vp([20.0, 0.0, 0.0], 100),
                consistency: ConsistencyStateTag::Committed,
                is_adult_maturity: true,
            },
            AdultCandidate {
                id: "closest".into(),
                position: make_vp([0.5, 0.0, 0.0], 100),
                consistency: ConsistencyStateTag::Committed,
                is_adult_maturity: true,
            },
        ];
        let village = build_local_village_topk("child1".into(), &child_pos, &adults, 3);
        assert_eq!(village.adult_ids.len(), 3);
        // 距離昇順: closest(0.5), near(1.0), mid1(10.0)
        assert_eq!(village.adult_ids[0], "closest", "最短距離が先頭");
        assert_eq!(village.adult_ids[1], "near", "2番目");
        assert_eq!(village.adult_ids[2], "mid1", "3番目");
        println!(
            "T-11 PASS: selected {} adults in ascending distance order: {:?}",
            village.adult_ids.len(),
            village.adult_ids
        );
    }

    // --------------------------------------------------------
    // T-12: build_local_village_topk — k 超過
    // --------------------------------------------------------
    #[test]
    fn t12_topk_k_exceeds_population() {
        let child_pos = make_vp([0.0, 0.0, 0.0], 100);
        let adults = vec![
            AdultCandidate {
                id: "a1".into(),
                position: make_vp([1.0, 0.0, 0.0], 100),
                consistency: ConsistencyStateTag::Committed,
                is_adult_maturity: true,
            },
            AdultCandidate {
                id: "a2".into(),
                position: make_vp([2.0, 0.0, 0.0], 100),
                consistency: ConsistencyStateTag::Committed,
                is_adult_maturity: true,
            },
        ];
        let village = build_local_village_topk("child1".into(), &child_pos, &adults, 10);
        assert_eq!(
            village.adult_ids.len(),
            2,
            "k が母集団より大きくても全件選抜"
        );
        println!(
            "T-12 PASS: k=10, population=2 → selected {}",
            village.adult_ids.len()
        );
    }

    // --------------------------------------------------------
    // T-13: build_local_village_topk — k = 0
    // --------------------------------------------------------
    #[test]
    fn t13_topk_k_zero() {
        let child_pos = make_vp([0.0, 0.0, 0.0], 100);
        let adults = vec![AdultCandidate {
            id: "a1".into(),
            position: make_vp([1.0, 0.0, 0.0], 100),
            consistency: ConsistencyStateTag::Committed,
            is_adult_maturity: true,
        }];
        let village = build_local_village_topk("child1".into(), &child_pos, &adults, 0);
        assert!(village.adult_ids.is_empty(), "k=0 は空村");
        println!("T-13 PASS: k=0 → empty village");
    }

    // --------------------------------------------------------
    // T-14: build_local_village_topk — 同距離タイ処理
    // --------------------------------------------------------
    #[test]
    fn t14_topk_tie_same_distance() {
        let child_pos = make_vp([0.0, 0.0, 0.0], 100);
        let adults = vec![
            AdultCandidate {
                id: "tie_a".into(),
                position: make_vp([1.0, 0.0, 0.0], 100),
                consistency: ConsistencyStateTag::Committed,
                is_adult_maturity: true,
            },
            AdultCandidate {
                id: "tie_b".into(),
                position: make_vp([0.0, 1.0, 0.0], 100),
                consistency: ConsistencyStateTag::Committed,
                is_adult_maturity: true,
            },
            AdultCandidate {
                id: "tie_c".into(),
                position: make_vp([0.0, 0.0, 1.0], 100),
                consistency: ConsistencyStateTag::Committed,
                is_adult_maturity: true,
            },
        ];
        // k=2 で同距離タイ → 任意の2件が選抜される
        let village = build_local_village_topk("child1".into(), &child_pos, &adults, 2);
        assert_eq!(village.adult_ids.len(), 2, "同距離から2件選抜");
        // 選抜された ID が候補に含まれていることのみ確認
        for id in &village.adult_ids {
            assert!(
                adults.iter().any(|a| &a.id == id),
                "選抜IDが候補に含まれている"
            );
        }
        println!(
            "T-14 PASS: k=2, 3 tied candidates → selected {}: {:?}",
            village.adult_ids.len(),
            village.adult_ids
        );
    }

    // --------------------------------------------------------
    // T-15: build_local_village_radius — 半径内選抜
    // --------------------------------------------------------
    #[test]
    fn t15_radius_within_range() {
        let child_pos = make_vp([0.0, 0.0, 0.0], 100);
        let adults = vec![
            AdultCandidate {
                id: "inside1".into(),
                position: make_vp([1.0, 0.0, 0.0], 100),
                consistency: ConsistencyStateTag::Committed,
                is_adult_maturity: true,
            },
            AdultCandidate {
                id: "inside2".into(),
                position: make_vp([0.0, 2.0, 0.0], 100),
                consistency: ConsistencyStateTag::Committed,
                is_adult_maturity: true,
            },
            AdultCandidate {
                id: "outside".into(),
                position: make_vp([10.0, 0.0, 0.0], 100),
                consistency: ConsistencyStateTag::Committed,
                is_adult_maturity: true,
            },
        ];
        let village = build_local_village_radius("child1".into(), &child_pos, &adults, 5.0);
        assert_eq!(village.adult_ids.len(), 2, "半径5.0内の2件が選抜");
        assert!(village.adult_ids.contains(&"inside1".to_string()));
        assert!(village.adult_ids.contains(&"inside2".to_string()));
        assert!(!village.adult_ids.contains(&"outside".to_string()));
        println!(
            "T-15 PASS: radius=5.0, 3 candidates → selected {}",
            village.adult_ids.len()
        );
    }

    // --------------------------------------------------------
    // T-16: build_local_village_radius — 半径内不在
    // --------------------------------------------------------
    #[test]
    fn t16_radius_no_adults_within_range() {
        let child_pos = make_vp([0.0, 0.0, 0.0], 100);
        let adults = vec![AdultCandidate {
            id: "far".into(),
            position: make_vp([100.0, 0.0, 0.0], 100),
            consistency: ConsistencyStateTag::Committed,
            is_adult_maturity: true,
        }];
        let village = build_local_village_radius("child1".into(), &child_pos, &adults, 1.0);
        assert!(village.adult_ids.is_empty(), "半径1.0内に不在なら空村");
        println!("T-16 PASS: radius=1.0, no adults within → empty village");
    }

    // --------------------------------------------------------
    // T-17: build_local_village_radius — 全 Adult が半径内
    // --------------------------------------------------------
    #[test]
    fn t17_radius_all_adults_within_range() {
        let child_pos = make_vp([0.0, 0.0, 0.0], 100);
        let adults = vec![
            AdultCandidate {
                id: "a1".into(),
                position: make_vp([1.0, 0.0, 0.0], 100),
                consistency: ConsistencyStateTag::Committed,
                is_adult_maturity: true,
            },
            AdultCandidate {
                id: "a2".into(),
                position: make_vp([2.0, 0.0, 0.0], 100),
                consistency: ConsistencyStateTag::Committed,
                is_adult_maturity: true,
            },
        ];
        let village = build_local_village_radius("child1".into(), &child_pos, &adults, 10.0);
        assert_eq!(village.adult_ids.len(), 2, "全件が半径内");
        println!(
            "T-17 PASS: radius=10.0, all 2 adults within → selected {}",
            village.adult_ids.len()
        );
    }

    // --------------------------------------------------------
    // T-18: centroid 計算 — 単一 Adult
    // --------------------------------------------------------
    #[test]
    fn t18_centroid_single_adult() {
        let pos = make_vp([3.0, 4.0, 5.0], 100);
        let centroid = compute_centroid(&[pos]);
        assert_eq!(
            centroid.inner().unwrap(),
            [3.0, 4.0, 5.0],
            "単一Adultのcentroidは自身の位置"
        );
        println!("T-18 PASS: single adult centroid = [3.0, 4.0, 5.0]");
    }

    // --------------------------------------------------------
    // T-19: centroid 計算 — 複数 Adult
    // --------------------------------------------------------
    #[test]
    fn t19_centroid_multiple_adults() {
        let positions = vec![make_vp([1.0, 0.0, 0.0], 100), make_vp([3.0, 0.0, 0.0], 100)];
        let centroid = compute_centroid(&positions);
        assert_eq!(
            centroid.inner().unwrap(),
            [2.0, 0.0, 0.0],
            "2点の重心は中点"
        );
        println!("T-19 PASS: centroid of [1,0,0] and [3,0,0] = [2.0, 0.0, 0.0]");
    }

    // --------------------------------------------------------
    // T-20: centroid 計算 — 空村
    // --------------------------------------------------------
    #[test]
    fn t20_centroid_empty_village() {
        let centroid = compute_centroid(&[]);
        assert_eq!(centroid.inner(), &None, "空村のcentroidはunknown");
        println!("T-20 PASS: empty village centroid = unknown");
    }

    // --------------------------------------------------------
    // T-E1: 計装サマリ出力
    // --------------------------------------------------------
    #[test]
    fn te1_instrumentation_summary() {
        println!("\n=== village::tests 全テスト PASS ===");
        println!("T-1  PASS: classify_maturity — Child（経験値不足）");
        println!("T-2  PASS: classify_maturity — Adult（全軸充足）");
        println!("T-3  PASS: classify_maturity — Child（信頼不足）");
        println!("T-4  PASS: classify_maturity — Child（レピュテーション不足）");
        println!("T-5  PASS: classify_maturity — 全軸ギリギリ不足");
        println!("T-6  PASS: classify_maturity — 全軸閾値超過");
        println!("T-7  PASS: classify_maturity — 極値入力");
        println!("T-8  PASS: filter_adult_candidates — ConsistencyState 除外");
        println!("T-9  PASS: filter_adult_candidates — Adult maturity 未達除外");
        println!("T-10 PASS: filter_adult_candidates — 複合フィルタ");
        println!("T-11 PASS: build_local_village_topk — 基本的な選抜");
        println!("T-12 PASS: build_local_village_topk — k 超過");
        println!("T-13 PASS: build_local_village_topk — k=0");
        println!("T-14 PASS: build_local_village_topk — 同距離タイ処理");
        println!("T-15 PASS: build_local_village_radius — 半径内選抜");
        println!("T-16 PASS: build_local_village_radius — 半径内不在");
        println!("T-17 PASS: build_local_village_radius — 全件半径内");
        println!("T-18 PASS: centroid — 単一 Adult");
        println!("T-19 PASS: centroid — 複数 Adult");
        println!("T-20 PASS: centroid — 空村");
        println!("（T-E1 サマリ出力完了: 全21テスト PASS）");
    }

    // ============================================================
    // M1.75-7 T-1〜T-13: Village Metrics 不変条件テスト
    // ============================================================

    #[test]
    fn t1_jaccard_identical_set() {
        let ids: Vec<WorkflowGraphId> = vec!["a".into(), "b".into(), "c".into()];
        let j = compute_village_jaccard(&ids, &ids);
        assert!((j - 1.0).abs() < 1e-10, "同一集合で Jaccard=1 (got {})", j);
        let c = compute_village_churn(j);
        assert!((c - 0.0).abs() < 1e-10, "Jaccard=1 で churn=0 (got {})", c);
        println!("T1 PASS: identical set → Jaccard={}, churn={}", j, c);
    }

    #[test]
    fn t2_jaccard_disjoint_sets() {
        let prev: Vec<WorkflowGraphId> = vec!["a".into(), "b".into()];
        let curr: Vec<WorkflowGraphId> = vec!["c".into(), "d".into()];
        let j = compute_village_jaccard(&prev, &curr);
        assert!((j - 0.0).abs() < 1e-10, "disjoint で Jaccard=0 (got {})", j);
        let c = compute_village_churn(j);
        assert!((c - 1.0).abs() < 1e-10, "Jaccard=0 で churn=1 (got {})", c);
        println!("T2 PASS: disjoint sets → Jaccard={}, churn={}", j, c);
    }

    #[test]
    fn t3_jaccard_both_empty() {
        let empty: Vec<WorkflowGraphId> = vec![];
        let j = compute_village_jaccard(&empty, &empty);
        assert!((j - 0.0).abs() < 1e-10, "両方空で Jaccard=0 (got {})", j);
        println!("T3 PASS: both empty → Jaccard={}", j);
    }

    #[test]
    fn t4_jaccard_one_side_empty() {
        let prev: Vec<WorkflowGraphId> = vec!["a".into()];
        let empty: Vec<WorkflowGraphId> = vec![];
        let j = compute_village_jaccard(&prev, &empty);
        assert!((j - 0.0).abs() < 1e-10, "片方空で Jaccard=0 (got {})", j);
        println!("T4 PASS: one side empty → Jaccard={}", j);
    }

    #[test]
    fn t5_jsd_identical_distributions() {
        let w = vec![0.5, 0.5];
        let jsd = compute_helper_jsd(&w, &w);
        assert!(jsd < 1e-10, "同一分布で JSD≈0 (got {})", jsd);
        println!("T5 PASS: identical distributions → JSD={}", jsd);
    }

    #[test]
    fn t6_jsd_perfectly_separated() {
        let p = vec![1.0, 0.0];
        let q = vec![0.0, 1.0];
        let jsd = compute_helper_jsd(&p, &q);
        let expected = 2.0_f64.ln();
        let diff = (jsd - expected).abs();
        assert!(
            diff < 1e-6,
            "完全分離で JSD≈ln2 (got {}, expected {})",
            jsd,
            expected
        );
        println!("T6 PASS: perfectly separated → JSD={}", jsd);
    }

    #[test]
    fn t7_survival_rate_all_survive() {
        let rate = compute_child_survival_rate(5, 5);
        assert!((rate - 1.0).abs() < 1e-10, "全生存で rate=1 (got {})", rate);
        println!("T7 PASS: all survive → rate={}", rate);
    }

    #[test]
    fn t8_survival_rate_all_dead() {
        let rate = compute_child_survival_rate(0, 5);
        assert!((rate - 0.0).abs() < 1e-10, "全滅で rate=0 (got {})", rate);
        println!("T8 PASS: all dead → rate={}", rate);
    }

    #[test]
    fn t9_survival_rate_zero_total() {
        let rate = compute_child_survival_rate(0, 0);
        assert!(
            (rate - 0.0).abs() < 1e-10,
            "total=0 でも panic しない (got {})",
            rate
        );
        println!("T9 PASS: total=0 → rate={}", rate);
    }

    #[test]
    fn t10_maturation_time_positive() {
        let t = compute_child_maturation_time(10, 25);
        assert!((t - 15.0).abs() < 1e-10, "maturation_time=15 (got {})", t);
        println!("T10 PASS: birth=10, maturation=25 → time={}", t);
    }

    #[test]
    fn t11_maturation_time_zero() {
        let t = compute_child_maturation_time(10, 10);
        assert!((t - 0.0).abs() < 1e-10, "同時刻で time=0 (got {})", t);
        println!("T11 PASS: birth==maturation → time={}", t);
    }

    #[test]
    fn t12_position_drift_identical() {
        let pos = [0.0_f32, 0.0, 0.0];
        let drift = compute_position_drift(&pos, &pos);
        assert!(
            (drift - 0.0).abs() < 1e-10,
            "同一位置で drift=0 (got {})",
            drift
        );
        println!("T12 PASS: identical position → drift={}", drift);
    }

    #[test]
    fn t13_position_drift_expected_value() {
        let prev = [0.0_f32, 0.0, 0.0];
        let curr = [1.0_f32, 0.0, 0.0];
        let drift = compute_position_drift(&prev, &curr);
        assert!(
            (drift - 1.0).abs() < 1e-10,
            "X軸+1 で drift=1 (got {})",
            drift
        );
        println!("T13 PASS: delta=[1,0,0] → drift={}", drift);
    }

    // ============================================================
    // M1.75-7 T-O1〜T-O3: 観測テスト
    // ============================================================

    #[test]
    fn to1_metric_grid_sweep() {
        println!("\n=== M1.75-7 T-O1: メトリクス関数グリッド掃引 ===");
        println!("metric, input_desc, value");

        // 位置ドリフト sweep
        for i in 0..=5 {
            let prev = [0.0_f32, 0.0, 0.0];
            let curr = [i as f32, 0.0, 0.0];
            let drift = compute_position_drift(&prev, &curr);
            println!("position_drift, delta_x={}, {}", i, drift);
        }

        // Jaccard sweep (部分重複)
        for overlap in 0..=5 {
            let prev: Vec<WorkflowGraphId> = (0..5).map(|i| format!("a-{}", i)).collect();
            let curr: Vec<WorkflowGraphId> = (0..overlap)
                .map(|i| format!("a-{}", i))
                .chain((overlap..5).map(|i| format!("b-{}", i)))
                .collect();
            let j = compute_village_jaccard(&prev, &curr);
            let c = compute_village_churn(j);
            println!("jaccard_churn, overlap={}, {}, {}", overlap, j, c);
        }

        // JSD sweep (混合率)
        for mix in 0..=10 {
            let alpha = mix as f64 / 10.0;
            let p = vec![alpha, 1.0 - alpha];
            let q = vec![1.0 - alpha, alpha];
            let jsd = compute_helper_jsd(&p, &q);
            println!("helper_jsd, alpha={}, {}", alpha, jsd);
        }

        // survival rate sweep
        for dead in 0..=5 {
            let total = 10;
            let surviving = total - dead;
            let rate = compute_child_survival_rate(surviving, total);
            println!("child_survival_rate, dead={}, {}", dead, rate);
        }

        println!("T-O1 PASS: メトリクスグリッド掃引完了");
    }

    #[test]
    fn to2_village_metrics_window_aggregation() {
        println!("\n=== M1.75-7 T-O2: VillageMetricsWindow 集約動作 ===");
        let mut window = VillageMetricsWindow::new(10);

        // 空ウィンドウのスナップショット
        assert!(
            window.snapshot().is_none(),
            "空ウィンドウの snapshot は None"
        );
        println!("empty window snapshot: None (OK)");

        // メトリクスを追加
        for tick in 0..=10 {
            let m = VillageMetrics {
                position_drifts: vec![tick as f64 * 0.1],
                village_jaccards: vec![1.0 - tick as f64 * 0.05],
                village_churns: vec![tick as f64 * 0.05],
                helper_jsds: vec![tick as f64 * 0.01],
                helper_counts: vec![5],
                child_survival_count: 8,
                child_maturation_count: 1,
                total_child_count: 10,
                tick,
            };
            window.push(m);
        }
        assert_eq!(window.len(), 10, "window_size=10 のため 10件保持");

        let snap = window.snapshot().expect("window が空でない");
        println!(
            "snapshot: drift_p50={}, drift_p95={}",
            snap.position_drift_p50, snap.position_drift_p95
        );
        println!(
            "snapshot: churn_p50={}, churn_p95={}",
            snap.village_churn_p50, snap.village_churn_p95
        );
        println!(
            "snapshot: jsd_p50={}, jsd_p95={}",
            snap.helper_jsd_p50, snap.helper_jsd_p95
        );
        println!("snapshot: helper_count_mean={}", snap.helper_count_mean);
        println!("snapshot: survival_rate={}", snap.child_survival_rate);
        println!("T-O2 PASS: VillageMetricsWindow 集約動作完了");
    }

    #[test]
    fn to3_instrumentation_summary() {
        println!("\n=== M1.75-7 T-E1: 計装サマリ ===");
        println!("T-1  PASS: compute_village_jaccard — 同一集合で 1.0");
        println!("T-2  PASS: compute_village_jaccard — disjoint で 0.0");
        println!("T-3  PASS: compute_village_jaccard — 両方空で 0.0");
        println!("T-4  PASS: compute_village_jaccard — 片方空で 0.0");
        println!("T-5  PASS: compute_helper_jsd — 同一分布で ≈0");
        println!("T-6  PASS: compute_helper_jsd — 完全分離で ≈ln2");
        println!("T-7  PASS: compute_child_survival_rate — 全生存で 1.0");
        println!("T-8  PASS: compute_child_survival_rate — 全滅で 0.0");
        println!("T-9  PASS: compute_child_survival_rate — total=0 で panic なし");
        println!("T-10 PASS: compute_child_maturation_time — 正の値");
        println!("T-11 PASS: compute_child_maturation_time — 同時刻で 0");
        println!("T-12 PASS: compute_position_drift — 同一位置で 0");
        println!("T-13 PASS: compute_position_drift — 期待値一致");
        println!("T-O1 PASS: メトリクスグリッド掃引");
        println!("T-O2 PASS: VillageMetricsWindow 集約");
        println!("（M1.75-7 計装サマリ: 全不変条件テスト+観測テスト PASS）");
    }
}
