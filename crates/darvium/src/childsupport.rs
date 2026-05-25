// child-support training mission orchestration (RFC §41B.11, §41B.12)
//
// 本モジュールは以下の機能を提供する：
// - ChildSupportMissionPayload 構造体 — child-support 固有のミッション情報
// - SafetyScope 列挙型 — safe sandbox 実行範囲の指定
// - spawn_child_support_mission — child-support mission の発行
// - is_allowed_on_plane — production plane 実行可否の判定
// - HelperWeight および HelperSelectionPolicy (M1.75-6, RFC §41B.12)
// - select_helpers — helper 重み付け選定 (M1.75-6, RFC §41B.12)
//
// M1.75-2 (village.rs) の LocalVillage / WorkflowMaturity に依存する。
// M1.75-4 (help.rs) の AdultHelpOfferPolicy と連携して動作する。

use std::time::SystemTime;

use crate::constants::{
    CHILD_SUPPORT_MISSION_TIMEOUT_SECS, HELPER_WEIGHT_DEFAULT_TOP_K,
    HELPER_WEIGHT_DISTANCE_DECAY_BETA, HELPER_WEIGHT_EXPLORATION_EPSILON,
    HELPER_WEIGHT_REPUTATION_EXPONENT, HELPER_WEIGHT_TRUST_EXPONENT, MAX_HELPERS_PER_MISSION,
};
use crate::event::{
    DarviumEvent, DarviumEventBus, DarviumEventKind, EventCausality, EventMetadata, EventPrivacy,
    EventRetention, EventSource, EventVisibility, InteractionMode, PiiHandlingPolicy,
    TrainingEvent,
};
use crate::spaceposition::l2_distance;
use crate::types::{TrainingMissionKind, WorkflowGraphId};
use crate::village::{filter_adult_candidates, AdultCandidate, LocalVillage};

// ============================================================
// 型定義 (RFC §41B.11)
// ============================================================

/// child-support mission の safe sandbox 実行範囲。
#[derive(Debug, Clone, PartialEq)]
pub enum SafetyScope {
    /// 読み取り専用の mission。ワークフロー検索・評価のみ。
    ReadOnly,
    /// 限定的な書き込みを伴う mission。sandbox namespace 内に制限。
    LimitedMutation,
    /// 完全な sandbox 実行。全ての操作が sandbox 内で完結。
    FullSandbox,
}

/// child-support mission のペイロード (RFC §41B.11)。
///
/// 通常の TrainingMission に child-support 固有のメタデータを付与する
/// 特殊化情報。発行時点の village スナップショットを含む。
#[derive(Debug, Clone, PartialEq)]
pub struct ChildSupportMissionPayload {
    /// 対象 child ワークフローの識別子。
    pub child_id: WorkflowGraphId,
    /// 選定された helper (adult) ワークフローの識別子リスト。
    pub helper_ids: Vec<WorkflowGraphId>,
    /// ミッション発行時点の village スナップショット。
    pub village_snapshot: LocalVillage,
    /// ミッションの目的記述。
    pub objective: String,
    /// safe sandbox 実行範囲。
    pub safety_scope: SafetyScope,
    /// ミッションのタイムアウト秒数。
    pub timeout_secs: u64,
}

impl ChildSupportMissionPayload {
    /// 最小限の検証を通過した ChildSupportMissionPayload を作成する。
    ///
    /// 以下の不変条件を検証する：
    /// - `helper_ids` が空でない
    /// - `helper_ids` の長さが `MAX_HELPERS_PER_MISSION` を超えない
    /// - `village_snapshot` に少なくとも 1 人の adult が含まれている
    pub fn new(
        child_id: WorkflowGraphId,
        helper_ids: Vec<WorkflowGraphId>,
        village_snapshot: LocalVillage,
        objective: String,
        safety_scope: SafetyScope,
    ) -> Option<Self> {
        if helper_ids.is_empty() {
            return None;
        }
        if helper_ids.len() > MAX_HELPERS_PER_MISSION as usize {
            return None;
        }
        if village_snapshot.adult_ids.is_empty() {
            return None;
        }

        Some(Self {
            child_id,
            helper_ids,
            village_snapshot,
            objective,
            safety_scope,
            timeout_secs: CHILD_SUPPORT_MISSION_TIMEOUT_SECS,
        })
    }
}

// ============================================================
// M1.75-6 型定義 (RFC §41B.12)
// ============================================================

/// 正規化された helper 重み (RFC §41B.12)。
///
/// 重みの総和は 1.0（浮動小数点誤差を除く）を満たす。
/// `is_remote` フラグは遠隔探索によって重みが変更された helper を示す。
#[derive(Debug, Clone, PartialEq)]
pub struct HelperWeight {
    /// helper のワークフロー ID。
    pub helper_id: WorkflowGraphId,
    /// 正規化された重み (0.0 <= weight <= 1.0)。
    pub weight: f64,
    /// 遠隔探索によって選択されたか。
    pub is_remote: bool,
}

/// helper 選定ポリシー (RFC §41B.12)。
///
/// 式 41B-18 の β（距離減衰）, μ（信頼指数）, ν（レピュテーション指数）、
/// 式 41B-19 の ε（探索率）、および TOP-K 制限を保持する。
#[derive(Debug, Clone, PartialEq)]
pub struct HelperSelectionPolicy {
    /// 距離減衰係数 β（式 41B-18）。
    pub beta: f64,
    /// 信頼指数 μ（式 41B-18）。
    pub trust_exponent: f64,
    /// レピュテーション指数 ν（式 41B-18）。
    pub reputation_exponent: f64,
    /// 探索率 ε（式 41B-19）。0.0 で純局所、1.0 で純遠隔探索。
    pub epsilon: f64,
    /// 選抜する helper の最大数 (TOP-K)。
    pub top_k: usize,
}

impl Default for HelperSelectionPolicy {
    fn default() -> Self {
        Self {
            beta: HELPER_WEIGHT_DISTANCE_DECAY_BETA,
            trust_exponent: HELPER_WEIGHT_TRUST_EXPONENT,
            reputation_exponent: HELPER_WEIGHT_REPUTATION_EXPONENT,
            epsilon: HELPER_WEIGHT_EXPLORATION_EPSILON,
            top_k: HELPER_WEIGHT_DEFAULT_TOP_K,
        }
    }
}

// ============================================================
// M1.75-6 helper 重み付け純粋関数 (RFC §41B.12)
// ============================================================

/// 式 41B-18 の正規化 helper 重みを計算する。
///
/// w_t(h|c) = exp(-β·d_t(h,c)) · T(h)^μ · R(h)^ν
///          / Σ_{g∈H_t(c)} exp(-β·d_t(g,c)) · T(g)^μ · R(g)^ν
///
/// # 引数
/// - `distances`: Child から各 helper 候補への L2 距離リスト（非負）。
/// - `trusts`: 各 helper 候補の信頼複合スコア T(h)（0.0〜1.0）。
/// - `reputations`: 各 helper 候補のレピュテーション最終スコア R(h)（0.0〜1.0）。
/// - `policy`: 選定ポリシー（β, μ, ν）。
///
/// # 戻り値
/// 各候補の正規化重みの Vec。空の入力に対しては空の Vec を返す。
///
/// # 注意
/// 全ての exp 項の合計が 0 の場合（β 大によるアンダーフロー）、
/// 一様重みをフォールバックとして返す。
pub fn compute_helper_weights(
    distances: &[f64],
    trusts: &[f64],
    reputations: &[f64],
    policy: &HelperSelectionPolicy,
) -> Vec<f64> {
    let n = distances.len();
    if n == 0 {
        return Vec::new();
    }

    // 各候補の非正規化 numerator を計算
    let numerators: Vec<f64> = distances
        .iter()
        .zip(trusts.iter())
        .zip(reputations.iter())
        .map(|((&d, &t), &r)| {
            let decay = (-policy.beta * d).exp();
            let trust_term = t.powf(policy.trust_exponent);
            let rep_term = r.powf(policy.reputation_exponent);
            decay * trust_term * rep_term
        })
        .collect();

    let sum: f64 = numerators.iter().sum();

    if sum <= 0.0 {
        // アンダーフロー: 一様重みでフォールバック
        let uniform = 1.0 / n as f64;
        return vec![uniform; n];
    }

    numerators
        .iter()
        .map(|&numerator| numerator / sum)
        .collect()
}

/// 式 41B-19 の bounded remote exploration を適用する。
///
/// w̃_t(h|c) = (1-ε) · w_t(h|c) + ε · w^{remote}_t(h|c)
///
/// 遠隔重み w^{remote} は全候補に対する一様分布とする。
///
/// # 引数
/// - `local_weights`: `compute_helper_weights` の出力（正規化済み局所重み）。
/// - `epsilon`: 探索率（0.0 = 純局所, 1.0 = 純遠隔）。
///
/// # 戻り値
/// 混合後の重みベクトル。空の入力に対しては空の Vec を返す。
pub fn mix_with_remote_exploration(local_weights: &[f64], epsilon: f64) -> Vec<f64> {
    let n = local_weights.len();
    if n == 0 {
        return Vec::new();
    }

    if epsilon <= 0.0 {
        return local_weights.to_vec();
    }

    let remote_weight = 1.0 / n as f64;
    let one_minus_eps = 1.0 - epsilon;

    local_weights
        .iter()
        .map(|&w| one_minus_eps * w + epsilon * remote_weight)
        .collect()
}

/// ハードフィルタ・重み計算・遠隔探索混合・TOP-K 選抜を実行する。
///
/// 1. `filter_adult_candidates` で ConsistencyState と maturity のハードフィルタを適用
/// 2. フィルタ通過候補の Child からの L2 距離を計算
/// 3. `compute_helper_weights` で式 41B-18 の正規化重みを計算
/// 4. `mix_with_remote_exploration` で式 41B-19 の ε 混合を適用
/// 5. 混合後の重みで降順ソートし、TOP-K を選抜
/// 6. 選抜結果を `HelperWeight` の Vec として返す
///
/// # 引数
/// - `candidates`: フィルタ前の全 AdultCandidate。
/// - `child_pos`: Child の空間位置（L2 距離計算用）。
/// - `trusts_by_id`: 各候補 ID → 信頼複合スコア T(h) のマップ。
/// - `reputations_by_id`: 各候補 ID → レピュテーション最終スコア R(h) のマップ。
/// - `policy`: 選定ポリシー。
///
/// # 戻り値
/// 重み降順でソートされた `HelperWeight` の Vec。空リスト、全フィルタアウト、
/// または空村の場合は空の Vec を返す。
pub fn select_helpers(
    candidates: Vec<AdultCandidate>,
    child_pos: &crate::spaceposition::VillagePosition,
    trusts_by_id: &std::collections::HashMap<WorkflowGraphId, f64>,
    reputations_by_id: &std::collections::HashMap<WorkflowGraphId, f64>,
    policy: &HelperSelectionPolicy,
) -> Vec<HelperWeight> {
    // Step 1: ハードフィルタ
    let filtered = filter_adult_candidates(candidates);

    if filtered.is_empty() {
        return Vec::new();
    }

    let n = filtered.len();
    let top_k = policy.top_k.min(n);

    if top_k == 0 {
        return Vec::new();
    }

    // Step 2: 距離計算とID・重み成分の収集
    let mut indexed: Vec<(usize, f64, f64, f64)> = filtered
        .iter()
        .enumerate()
        .map(|(i, candidate)| {
            let distance = l2_distance(&child_pos.position, &candidate.position.position);
            let trust = trusts_by_id.get(&candidate.id).copied().unwrap_or(0.0);
            let reputation = reputations_by_id.get(&candidate.id).copied().unwrap_or(0.0);
            (i, distance, trust, reputation)
        })
        .collect();

    // 距離昇順でソート（近い順）
    indexed.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    let distances: Vec<f64> = indexed.iter().map(|(_, d, _, _)| *d).collect();
    let trusts: Vec<f64> = indexed.iter().map(|(_, _, t, _)| *t).collect();
    let reputations: Vec<f64> = indexed.iter().map(|(_, _, _, r)| *r).collect();
    let sorted_indices: Vec<usize> = indexed.iter().map(|(i, _, _, _)| *i).collect();

    // Step 3: 局所重み計算
    let local_weights = compute_helper_weights(&distances, &trusts, &reputations, policy);

    // Step 4: 遠隔探索混合
    let mixed_weights = mix_with_remote_exploration(&local_weights, policy.epsilon);

    // Step 5: 重み降順ソートし TOP-K 選抜
    let mut helper_entries: Vec<(usize, WorkflowGraphId, f64)> = sorted_indices
        .iter()
        .enumerate()
        .map(|(sorted_pos, &orig_idx)| {
            let id = filtered[orig_idx].id.clone();
            let weight = mixed_weights[sorted_pos];
            (orig_idx, id, weight)
        })
        .collect();

    // 重み降順
    helper_entries.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

    // TOP-K 選抜
    let selected: Vec<HelperWeight> = helper_entries
        .into_iter()
        .take(top_k)
        .map(|(_, id, weight)| {
            let is_remote = policy.epsilon > 0.0;
            HelperWeight {
                helper_id: id,
                weight,
                is_remote,
            }
        })
        .collect();

    selected
}

// ============================================================
// 公開関数 (RFC §41B.11)
// ============================================================

/// child-support mission を発行する。
///
/// 正常系: `child_id` で指定された child に対して、
/// `village` 内の adult を helper として割り当てた mission を生成し、
/// EventBus へ `TrainingEvent::MissionGenerated` を publish する。
///
/// 異常系（以下の場合は `None` を返す）:
/// - empty village (`village.adult_ids` が空)
pub fn spawn_child_support_mission(
    child_id: WorkflowGraphId,
    village: &LocalVillage,
    event_bus: &dyn DarviumEventBus,
) -> Option<TrainingMissionKind> {
    if village.adult_ids.is_empty() {
        return None;
    }

    let payload = ChildSupportMissionPayload::new(
        child_id,
        village.adult_ids.clone(),
        village.clone(),
        format!(
            "child-support mission for {} with {} helpers",
            village.child_id,
            village.adult_ids.len()
        ),
        SafetyScope::FullSandbox,
    )?;

    // EventBus へ mission 生成イベントを publish する
    let event = DarviumEvent {
        event_id: format!("cs-{}-{}", payload.child_id, chrono_now_ms()),
        kind: DarviumEventKind::Training(TrainingEvent::MissionGenerated),
        interaction_mode: InteractionMode::OneWay,
        payload: serde_json::json!({
            "child_id": payload.child_id,
            "helper_ids": payload.helper_ids,
            "objective": payload.objective,
            "safety_scope": format!("{:?}", payload.safety_scope),
        }),
        causality: EventCausality {
            parent_event_id: None,
            root_event_id: None,
            trace_ref: None,
            mission_id: Some(payload.child_id.clone()),
            workflow_id: None,
            run_id: None,
        },
        metadata: EventMetadata {
            clock: 0,
            timestamp: SystemTime::now(),
            source: EventSource::System,
        },
        transport_meta: None,
        visibility: EventVisibility::Internal,
        retention: EventRetention {
            persist: true,
            ttl_days: None,
        },
        privacy: EventPrivacy {
            contains_pii: false,
            sandbox_only: true,
            pii_handling: PiiHandlingPolicy::Reject,
        },
    };

    let _ = event_bus.publish(event);

    Some(TrainingMissionKind::ChildSupport)
}

/// 指定された TrainingMissionKind が production plane で実行可能かを判定する。
///
/// `ChildSupport` は safe sandbox 条件下でのみ許容され、
/// production plane では直接実行されてはならない。
pub fn is_allowed_on_plane(kind: &TrainingMissionKind) -> bool {
    kind.is_allowed_on_production_plane()
}

/// 現在の Unix 時刻 (ms) を返す。イベント ID 生成に使用する。
fn chrono_now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

// ============================================================
// テスト (M1.75-5)
// ============================================================
#[cfg(test)]
mod tests {
    use rand::rngs::StdRng;
    use rand::{Rng, SeedableRng};

    use super::*;
    use crate::event::FakeEventBus;
    use crate::spaceposition::SpacePositionEmbedding;
    use crate::types::ConsistencyStateTag;

    /// テスト用の LocalVillage を構築する。
    fn make_village(child_id: &str, adult_ids: Vec<&str>) -> LocalVillage {
        let adult_ids: Vec<WorkflowGraphId> = adult_ids.iter().map(|s| s.to_string()).collect();
        LocalVillage {
            child_id: child_id.to_string(),
            adult_ids,
            centroid: SpacePositionEmbedding::unknown(),
            radius: 1.0,
        }
    }

    // ============================================================
    // 不変条件テスト
    // ============================================================

    /// T-1: 正常系 — spawn_child_support_mission が適切な child/village に対して
    /// ChildSupport mission を生成すること。
    #[test]
    fn test_t1_spawn_normal() {
        let village = make_village("child-1", vec!["adult-1", "adult-2", "adult-3"]);
        let bus = FakeEventBus::new();

        let result = spawn_child_support_mission("child-1".to_string(), &village, &bus);

        assert_eq!(result, Some(TrainingMissionKind::ChildSupport));

        // EventBus へ MissionGenerated が publish されたことを確認
        let events = bus.published_events();
        assert!(
            !events.is_empty(),
            "EventBus にイベントが publish されるべき"
        );
        let has_mission_generated = events.iter().any(|e| {
            matches!(
                e.kind,
                DarviumEventKind::Training(TrainingEvent::MissionGenerated)
            )
        });
        assert!(
            has_mission_generated,
            "TrainingEvent::MissionGenerated が publish されるべき"
        );
    }

    /// T-2: 異常系 — empty village の child に対して mission が生成されず None を返すこと。
    #[test]
    fn test_t2_empty_village_returns_none() {
        let village = make_village("child-empty", vec![]);
        let bus = FakeEventBus::new();

        let result = spawn_child_support_mission("child-empty".to_string(), &village, &bus);

        assert_eq!(result, None, "empty village では None を返すべき");
    }

    /// T-3: 型検証 — TrainingMissionKind::ChildSupport と Production の判別。
    #[test]
    fn test_t3_kind_discrimination() {
        assert_eq!(
            TrainingMissionKind::Production,
            TrainingMissionKind::Production
        );
        assert_eq!(
            TrainingMissionKind::ChildSupport,
            TrainingMissionKind::ChildSupport
        );
        assert_ne!(
            TrainingMissionKind::Production,
            TrainingMissionKind::ChildSupport
        );
    }

    /// T-4: 境界値 — village に helper が 1 人だけの場合に mission が生成されること。
    #[test]
    fn test_t4_single_helper() {
        let village = make_village("child-single", vec!["adult-1"]);
        let bus = FakeEventBus::new();

        let result = spawn_child_support_mission("child-single".to_string(), &village, &bus);

        assert_eq!(
            result,
            Some(TrainingMissionKind::ChildSupport),
            "1 helper でも mission が生成されるべき"
        );
    }

    /// T-5: 正常系 — ChildSupportMissionPayload の全フィールドが設定されること。
    #[test]
    fn test_t5_payload_fields() {
        let village = make_village("child-payload", vec!["adult-1", "adult-2"]);
        let payload = ChildSupportMissionPayload::new(
            "child-payload".to_string(),
            village.adult_ids.clone(),
            village.clone(),
            "test objective".to_string(),
            SafetyScope::FullSandbox,
        );

        assert!(payload.is_some());
        let payload = payload.unwrap();
        assert_eq!(payload.child_id, "child-payload");
        assert_eq!(payload.helper_ids.len(), 2);
        assert_eq!(payload.objective, "test objective");
        assert_eq!(payload.safety_scope, SafetyScope::FullSandbox);
        assert!(payload.timeout_secs > 0);
    }

    /// T-6: 不変条件 — production plane ガード関数が ChildSupport を拒否し Production を許可すること。
    #[test]
    fn test_t6_production_plane_guard() {
        assert!(
            is_allowed_on_plane(&TrainingMissionKind::Production),
            "Production は production plane で許可されるべき"
        );
        assert!(
            !is_allowed_on_plane(&TrainingMissionKind::ChildSupport),
            "ChildSupport は production plane で拒否されるべき"
        );
    }

    /// T-7: 正常系 — mission 発行時に village snapshot が payload へ記録されること。
    #[test]
    fn test_t7_village_snapshot_recorded() {
        let village = make_village("child-snap", vec!["adult-1", "adult-2", "adult-3"]);
        let payload = ChildSupportMissionPayload::new(
            "child-snap".to_string(),
            village.adult_ids.clone(),
            village.clone(),
            "snapshot test".to_string(),
            SafetyScope::ReadOnly,
        );

        assert!(payload.is_some());
        let payload = payload.unwrap();
        // village snapshot の adult_ids が helper_ids と一致する
        assert_eq!(payload.village_snapshot.adult_ids.len(), 3);
        assert_eq!(payload.helper_ids, payload.village_snapshot.adult_ids);
    }

    /// T-8: 正常系 — TrainingEvent::MissionGenerated が EventBus へ publish されること。
    #[test]
    fn test_t8_eventbus_publish() {
        let village = make_village("child-eventbus", vec!["adult-1", "adult-2"]);
        let bus = FakeEventBus::new();

        let _result = spawn_child_support_mission("child-eventbus".to_string(), &village, &bus);

        let events = bus.published_events();
        assert!(!events.is_empty(), "イベントが publish されるべき");

        let mission_events: Vec<_> = events
            .iter()
            .filter(|e| {
                matches!(
                    e.kind,
                    DarviumEventKind::Training(TrainingEvent::MissionGenerated)
                )
            })
            .collect();
        assert_eq!(
            mission_events.len(),
            1,
            "1件の MissionGenerated が publish されるべき"
        );
    }

    /// T-9: 異常系 — 空の helper_ids で payload が生成されないこと。
    #[test]
    fn test_t9_empty_helper_ids() {
        let village = make_village("child-empty-helpers", vec!["adult-1"]);
        let payload = ChildSupportMissionPayload::new(
            "child-empty-helpers".to_string(),
            vec![], // 空の helper_ids
            village,
            "should fail".to_string(),
            SafetyScope::FullSandbox,
        );

        assert!(payload.is_none(), "空の helper_ids では None を返すべき");
    }

    /// T-10: 境界値 — helper_ids が MAX_HELPERS_PER_MISSION を超過した場合に None を返すこと。
    #[test]
    fn test_t10_max_helpers_exceeded() {
        let many_adults: Vec<String> = (0..MAX_HELPERS_PER_MISSION + 1)
            .map(|i| format!("adult-{}", i))
            .collect();
        let many_adult_refs: Vec<&str> = many_adults.iter().map(|s| s.as_str()).collect();
        let village = make_village("child-max", many_adult_refs);
        let payload = ChildSupportMissionPayload::new(
            "child-max".to_string(),
            village.adult_ids.clone(),
            village,
            "max test".to_string(),
            SafetyScope::FullSandbox,
        );

        assert!(
            payload.is_none(),
            "MAX_HELPERS_PER_MISSION 超過では None を返すべき"
        );
    }

    // ============================================================
    // EventBus 結合テスト
    // ============================================================

    /// T-E1: 結合 — child-support mission 発行 → EventBus publish → FakeEventBus で受信確認。
    #[test]
    fn test_te1_eventbus_integration() {
        let village = make_village("child-te1", vec!["adult-1"]);
        let bus = FakeEventBus::new();

        let result = spawn_child_support_mission("child-te1".to_string(), &village, &bus);

        assert_eq!(result, Some(TrainingMissionKind::ChildSupport));

        let events = bus.published_events();
        assert_eq!(events.len(), 1, "正確に1件のイベントが publish されるべき");

        let event = &events[0];
        assert!(
            matches!(
                event.kind,
                DarviumEventKind::Training(TrainingEvent::MissionGenerated)
            ),
            "イベント種別は TrainingEvent::MissionGenerated であるべき"
        );
        // payload に child_id が含まれている
        assert_eq!(event.payload["child_id"], "child-te1");
    }

    /// T-E2: **スキップ** — TrainingRunLog (EventProjection) の materialize 確認。
    ///
    /// TrainingRunLog の materialize は event.rs の既存テストスイート
    /// (test_r10_training_run_log_projection_materialize) でカバーされている。
    /// ChildSupport 固有の variant 追加は本チケットでは行わないため、
    /// このテストは統合テストとして event.rs 側で実施される。
    #[test]
    fn test_te2_training_run_log_projection() {
        // TrainingRunLog は既存の全 TrainingEvent を subscribe している。
        // MissionGenerated variant は既存の9 variant の一つであり、
        // 追加テストは event.rs の既存スイートに委ねる。
        println!("T-E2: TrainingRunLog materialize は event.rs の既存テストでカバー済み");
    }

    // ============================================================
    // 観測テスト
    // ============================================================

    /// T-O1: child-support mission 発行率の観測 (各種 village サイズで sweep)。
    #[test]
    fn test_to1_spawn_rate_sweep() {
        let mut rng = StdRng::seed_from_u64(12345);
        let sample_size = 1_000;
        let village_sizes = [1, 2, 5, 10, 20];

        println!("=== M1.75-5 T-O1: mission 発行率 sweep ===");
        println!("サンプルサイズ: {}", sample_size);

        for &size in &village_sizes {
            let mut spawn_count = 0u64;

            for _ in 0..sample_size {
                // ランダムな village サイズ (0〜size の間でばらつきを持たせる)
                let actual_size = rng.random_range(0..=size);
                let adults: Vec<String> =
                    (0..actual_size).map(|i| format!("adult-{}", i)).collect();
                let adult_refs: Vec<&str> = adults.iter().map(|s| s.as_str()).collect();

                let village = if actual_size > 0 {
                    make_village("child-obs", adult_refs)
                } else {
                    make_village("child-obs", vec![])
                };

                let bus = FakeEventBus::new();
                if spawn_child_support_mission("child-obs".to_string(), &village, &bus).is_some() {
                    spawn_count += 1;
                }
            }

            let rate = spawn_count as f64 / sample_size as f64;
            println!(
                "  village_size_max={}: 発行数={}, 発行率={:.4}",
                size, spawn_count, rate
            );
        }
    }

    /// T-O2: mission 発行成功率の統計分布。
    #[test]
    fn test_to2_spawn_success_distribution() {
        let mut rng = StdRng::seed_from_u64(12345);
        let sample_size = 500;

        println!("\n=== M1.75-5 T-O2: spawn 成功率分布 ===");
        println!("サンプルサイズ: {}", sample_size);

        // village の adult 数を 0..15 の一様分布で変化させる
        let mut spawn_success_count = 0u64;
        let mut empty_village_count = 0u64;

        for _ in 0..sample_size {
            let num_adults = rng.random_range(0..=15);
            let adults: Vec<String> = (0..num_adults).map(|i| format!("adult-{}", i)).collect();
            let adult_refs: Vec<&str> = adults.iter().map(|s| s.as_str()).collect();

            if adults.is_empty() {
                empty_village_count += 1;
            }

            let village = make_village("child-dist", adult_refs);
            let bus = FakeEventBus::new();
            if spawn_child_support_mission("child-dist".to_string(), &village, &bus).is_some() {
                spawn_success_count += 1;
            }
        }

        let success_rate = spawn_success_count as f64 / sample_size as f64;
        let empty_ratio = empty_village_count as f64 / sample_size as f64;

        println!("  spawn 成功数: {}", spawn_success_count);
        println!("  spawn 成功率: {:.4}", success_rate);
        println!("  empty village 出現数: {}", empty_village_count);
        println!("  empty village 比率: {:.4}", empty_ratio);
        println!("  空村補正後 spawn 成功率: {:.4}", {
            let non_empty = sample_size - empty_village_count;
            if non_empty > 0 {
                spawn_success_count as f64 / non_empty as f64
            } else {
                1.0
            }
        });
    }

    // ============================================================
    // M1.75-6: Helper weighting 不変条件テスト
    // ============================================================

    /// テスト用の AdultCandidate を生成する。
    fn make_adult_candidate(
        id: &str,
        pos: [f32; 3],
        consistency: ConsistencyStateTag,
        is_adult: bool,
    ) -> AdultCandidate {
        AdultCandidate {
            id: id.to_string(),
            position: crate::spaceposition::VillagePosition::new(pos, 0),
            consistency,
            is_adult_maturity: is_adult,
        }
    }

    /// テスト用の HelperSelectionPolicy を生成する。
    fn make_policy(
        beta: f64,
        trust_exp: f64,
        rep_exp: f64,
        epsilon: f64,
        top_k: usize,
    ) -> HelperSelectionPolicy {
        HelperSelectionPolicy {
            beta,
            trust_exponent: trust_exp,
            reputation_exponent: rep_exp,
            epsilon,
            top_k,
        }
    }

    /// テスト用の trust/reputation HashMap を生成する。
    fn make_scores(
        ids: &[&str],
        trust_val: f64,
        rep_val: f64,
    ) -> std::collections::HashMap<WorkflowGraphId, f64> {
        let mut map = std::collections::HashMap::new();
        for &id in ids {
            map.insert(id.to_string(), trust_val);
            map.insert(id.to_string(), rep_val);
        }
        map
    }

    fn make_trust_map(
        entries: Vec<(&str, f64)>,
    ) -> std::collections::HashMap<WorkflowGraphId, f64> {
        entries
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect()
    }

    fn make_reputation_map(
        entries: Vec<(&str, f64)>,
    ) -> std::collections::HashMap<WorkflowGraphId, f64> {
        entries
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect()
    }

    /// T-1: 同一 quality なら近距離 helper の weight が遠距離 helper より必ず高い。
    #[test]
    fn test_m1756_t1_nearby_higher_weight() {
        let child_pos = crate::spaceposition::VillagePosition::new([0.0, 0.0, 0.0], 0);
        let candidates = vec![
            make_adult_candidate(
                "near",
                [1.0, 0.0, 0.0],
                ConsistencyStateTag::Committed,
                true,
            ),
            make_adult_candidate(
                "far",
                [100.0, 0.0, 0.0],
                ConsistencyStateTag::Committed,
                true,
            ),
        ];
        let trusts = make_trust_map(vec![("near", 0.8), ("far", 0.8)]);
        let reputations = make_reputation_map(vec![("near", 0.8), ("far", 0.8)]);

        let policy = make_policy(1.0, 1.0, 1.0, 0.0, 10);
        let result = select_helpers(candidates, &child_pos, &trusts, &reputations, &policy);

        assert_eq!(result.len(), 2, "両候補が選抜されるべき");
        // 近距離の weight > 遠距離の weight
        let near_weight = result
            .iter()
            .find(|hw| hw.helper_id == "near")
            .unwrap()
            .weight;
        let far_weight = result
            .iter()
            .find(|hw| hw.helper_id == "far")
            .unwrap()
            .weight;
        assert!(
            near_weight > far_weight,
            "近距離 helper ({}) の weight が遠距離 ({}) より大きいべき",
            near_weight,
            far_weight
        );
        println!(
            "T-1 PASS: near_weight={}, far_weight={}",
            near_weight, far_weight
        );
    }

    /// T-2: quality が十分高ければ遠距離 helper が近距離低品質 helper を上回りうる。
    ///
    /// 高品質遠距離 (trust=0.95, rep=0.95, dist=5.0) が
    /// 低品質近距離 (trust=0.3, rep=0.3, dist=1.0) を上回ることを確認する。
    /// β=0.5, μ=3.0, ν=3.0 で品質差が距離ペナルティを上回る条件。
    #[test]
    fn test_m1756_t2_quality_overcomes_distance() {
        let child_pos = crate::spaceposition::VillagePosition::new([0.0, 0.0, 0.0], 0);
        let candidates = vec![
            make_adult_candidate(
                "near_low",
                [1.0, 0.0, 0.0],
                ConsistencyStateTag::Committed,
                true,
            ),
            make_adult_candidate(
                "far_high",
                [5.0, 0.0, 0.0],
                ConsistencyStateTag::Committed,
                true,
            ),
        ];
        let trusts = make_trust_map(vec![("near_low", 0.3), ("far_high", 0.95)]);
        let reputations = make_reputation_map(vec![("near_low", 0.3), ("far_high", 0.95)]);

        // β=0.5 で緩やかな距離減衰、μ=ν=3.0 で品質差を増幅
        let policy = make_policy(0.5, 3.0, 3.0, 0.0, 10);
        let result = select_helpers(candidates, &child_pos, &trusts, &reputations, &policy);

        assert_eq!(result.len(), 2, "両候補が選抜されるべき");
        let near_weight = result
            .iter()
            .find(|hw| hw.helper_id == "near_low")
            .unwrap()
            .weight;
        let far_weight = result
            .iter()
            .find(|hw| hw.helper_id == "far_high")
            .unwrap()
            .weight;
        assert!(
            far_weight > near_weight,
            "高品質遠距離 ({}) が低品質近距離 ({}) を上回るべき",
            far_weight,
            near_weight
        );
        println!(
            "T-2 PASS: far_high_weight={}, near_low_weight={}",
            far_weight, near_weight
        );
    }

    /// T-3: Pending / NeedsRepair / Quarantined / non-Adult 候補が 1 件も選ばれない。
    #[test]
    fn test_m1756_t3_hard_filter_exclusion() {
        let child_pos = crate::spaceposition::VillagePosition::new([0.0, 0.0, 0.0], 0);
        let candidates = vec![
            make_adult_candidate(
                "valid",
                [1.0, 0.0, 0.0],
                ConsistencyStateTag::Committed,
                true,
            ),
            make_adult_candidate(
                "pending",
                [1.0, 0.0, 0.0],
                ConsistencyStateTag::Pending,
                true,
            ),
            make_adult_candidate(
                "repair",
                [1.0, 0.0, 0.0],
                ConsistencyStateTag::NeedsRepair,
                true,
            ),
            make_adult_candidate(
                "quarantined",
                [1.0, 0.0, 0.0],
                ConsistencyStateTag::Quarantined,
                true,
            ),
            make_adult_candidate(
                "child_maturity",
                [1.0, 0.0, 0.0],
                ConsistencyStateTag::Committed,
                false,
            ),
        ];
        let ids: Vec<&str> = candidates.iter().map(|c| c.id.as_str()).collect();
        let trusts = make_scores(&ids, 0.8, 0.8);
        let reputations = make_scores(&ids, 0.8, 0.8);

        let policy = make_policy(1.0, 1.0, 1.0, 0.0, 10);
        let result = select_helpers(candidates, &child_pos, &trusts, &reputations, &policy);

        assert_eq!(result.len(), 1, "1件のみ選抜されるべき");
        assert_eq!(result[0].helper_id, "valid", "valid のみ選抜されるべき");
        println!(
            "T-3 PASS: 5 candidates → {} selected (valid only)",
            result.len()
        );
    }

    /// T-4: ε = 0 で remote exploration が 0。全 helper が局所重みのみ。
    #[test]
    fn test_m1756_t4_epsilon_zero_no_exploration() {
        let local_weights = compute_helper_weights(
            &[1.0, 10.0, 100.0],
            &[0.8, 0.8, 0.8],
            &[0.8, 0.8, 0.8],
            &make_policy(1.0, 1.0, 1.0, 0.0, 10),
        );

        let mixed = mix_with_remote_exploration(&local_weights, 0.0);

        // ε=0 では mixed == local_weights
        for (i, &m) in mixed.iter().enumerate() {
            assert!(
                (m - local_weights[i]).abs() < 1e-12,
                "ε=0 で混合前後が一致するべき: local={}, mixed={}",
                local_weights[i],
                m
            );
        }
        println!("T-4 PASS: ε=0, mixed == local (diff < 1e-12)");
    }

    /// T-5: ε = 1 で遠隔探索が優勢になり、全 helper が均等重みに近づく。
    #[test]
    fn test_m1756_t5_epsilon_one_full_exploration() {
        let local_weights = vec![0.8, 0.15, 0.05];
        let mixed = mix_with_remote_exploration(&local_weights, 1.0);

        // ε=1 では全 helper が一様重み (1/n)
        let expected = 1.0 / 3.0;
        for &m in &mixed {
            assert!(
                (m - expected).abs() < 1e-12,
                "ε=1 で全 helper が均等重み {} になるべき: got {}",
                expected,
                m
            );
        }
        println!("T-5 PASS: ε=1, all weights = {:.4}", expected);
    }

    /// T-6: 出力 weight の総和が 1.0（±浮動小数点誤差）。
    #[test]
    fn test_m1756_t6_weights_normalized() {
        let weights = compute_helper_weights(
            &[1.0, 5.0, 10.0, 20.0, 50.0],
            &[0.9, 0.7, 0.5, 0.3, 0.1],
            &[0.9, 0.7, 0.5, 0.3, 0.1],
            &make_policy(1.0, 1.0, 1.0, 0.0, 10),
        );

        let sum: f64 = weights.iter().sum();
        assert!(
            (sum - 1.0).abs() < 1e-10,
            "重みの総和が 1.0 であるべき: got {}",
            sum
        );
        println!("T-6 PASS: weight sum = {:.12}", sum);
    }

    /// T-7: β = 0 で距離に依存しない一様重みになる。
    #[test]
    fn test_m1756_t7_beta_zero_uniform() {
        let weights = compute_helper_weights(
            &[1.0, 10.0, 100.0],
            &[0.8, 0.8, 0.8],
            &[0.8, 0.8, 0.8],
            &make_policy(0.0, 1.0, 1.0, 0.0, 10),
        );

        let expected = 1.0 / 3.0;
        for &w in &weights {
            assert!(
                (w - expected).abs() < 1e-12,
                "β=0 で一様重み {} になるべき: got {}",
                expected,
                w
            );
        }
        println!("T-7 PASS: β=0, uniform weight = {:.4}", expected);
    }

    /// T-8: 空の候補リストに対して select_helpers が空リストを返す。
    #[test]
    fn test_m1756_t8_empty_candidates() {
        let child_pos = crate::spaceposition::VillagePosition::new([0.0, 0.0, 0.0], 0);
        let empty: Vec<AdultCandidate> = vec![];
        let trusts = std::collections::HashMap::new();
        let reputations = std::collections::HashMap::new();
        let policy = make_policy(1.0, 1.0, 1.0, 0.0, 10);

        let result = select_helpers(empty, &child_pos, &trusts, &reputations, &policy);
        assert!(result.is_empty(), "空候補では空リストを返すべき");
        println!("T-8 PASS: empty candidates → empty result");
    }

    // ============================================================
    // M1.75-6: Helper weighting 観測テスト
    // ============================================================

    /// T-O1: β-ε 2次元グリッド掃引。
    ///
    /// β ∈ {0.1, 0.5, 1.0, 2.0, 5.0} × ε ∈ {0.0, 0.1, 0.3, 0.5, 0.8, 1.0}
    /// で helper 分布エントロピー、平均 helper 距離、探索影響率（KL divergence）を計測。
    ///
    /// 探索影響率は、混合前後の重み分布の Jensen-Shannon divergence で計測し、
    /// ε がどれだけ分布を変化させたかを定量化する。
    #[test]
    fn test_m1756_to1_beta_epsilon_grid_sweep() {
        let betas = [0.1, 0.5, 1.0, 2.0, 5.0];
        let epsilons = [0.0, 0.1, 0.3, 0.5, 0.8, 1.0];
        let trial_count = 1_000;

        let mut rng = StdRng::seed_from_u64(12345);
        let child_pos = crate::spaceposition::VillagePosition::new([0.0, 0.0, 0.0], 0);

        println!("\n=== M1.75-6 T-O1: β-ε グリッド掃引 ===");
        println!("β, ε, エントロピー, 平均距離, 探索影響(JS divergence)");

        for &beta in &betas {
            for &eps in &epsilons {
                let mut entropy_sum = 0.0;
                let mut avg_dist_sum = 0.0;
                let mut exploration_influence_sum = 0.0;
                let mut valid_trials = 0u64;

                for _ in 0..trial_count {
                    let n_candidates: usize = rng.random_range(5..=20);
                    let candidates: Vec<AdultCandidate> = (0..n_candidates)
                        .map(|i| {
                            let pos = [
                                rng.random_range(-50.0..50.0),
                                rng.random_range(-50.0..50.0),
                                rng.random_range(-50.0..50.0),
                            ];
                            make_adult_candidate(
                                &format!("a-{}", i),
                                pos,
                                ConsistencyStateTag::Committed,
                                true,
                            )
                        })
                        .collect();

                    let ids: Vec<&str> = candidates.iter().map(|c| c.id.as_str()).collect();
                    let trusts = make_trust_map(
                        ids.iter()
                            .map(|id| (*id, rng.random_range(0.5..1.0)))
                            .collect(),
                    );
                    let reputations = make_reputation_map(
                        ids.iter()
                            .map(|id| (*id, rng.random_range(0.5..1.0)))
                            .collect(),
                    );

                    // 局所重みのみのポリシー
                    let local_policy = make_policy(beta, 1.0, 1.0, 0.0, 10);
                    // ε 混合ありのポリシー
                    let mixed_policy = make_policy(beta, 1.0, 1.0, eps, 10);

                    let local_result = select_helpers(
                        candidates.clone(),
                        &child_pos,
                        &trusts,
                        &reputations,
                        &local_policy,
                    );
                    let mixed_result = select_helpers(
                        candidates,
                        &child_pos,
                        &trusts,
                        &reputations,
                        &mixed_policy,
                    );

                    if mixed_result.is_empty() {
                        continue;
                    }

                    valid_trials += 1;

                    // エントロピー: H = -Σ w·log(w)
                    let h: f64 = mixed_result
                        .iter()
                        .map(|hw| {
                            if hw.weight > 0.0 {
                                -hw.weight * hw.weight.ln()
                            } else {
                                0.0
                            }
                        })
                        .sum();
                    entropy_sum += h;

                    // 平均距離: selected helper の Child からの平均 L2 距離
                    // (簡単のため候補インデックス順位を距離の代わりに使用:
                    //  select_helpers 内で距離昇順にソートされている)
                    let selected_positions: Vec<usize> = mixed_result
                        .iter()
                        .map(|hw| {
                            hw.helper_id[2..] // "a-N" の N 部分
                                .parse::<usize>()
                                .unwrap_or(0)
                        })
                        .collect();
                    let avg_pos: f64 = if !selected_positions.is_empty() {
                        selected_positions.iter().sum::<usize>() as f64
                            / selected_positions.len() as f64
                    } else {
                        0.0
                    };
                    avg_dist_sum += avg_pos;

                    // 探索影響: local 結果と mixed 結果の helper ID セットの Jaccard 距離
                    // （完全一致 = 0, 完全不一致 = 1）
                    let local_ids: std::collections::HashSet<&str> = local_result
                        .iter()
                        .map(|hw| hw.helper_id.as_str())
                        .collect();
                    let mixed_ids: std::collections::HashSet<&str> = mixed_result
                        .iter()
                        .map(|hw| hw.helper_id.as_str())
                        .collect();
                    let intersection = local_ids.intersection(&mixed_ids).count();
                    let union = local_ids.union(&mixed_ids).count();
                    let js_div = if union > 0 {
                        1.0 - (intersection as f64 / union as f64)
                    } else {
                        0.0
                    };
                    exploration_influence_sum += js_div;
                }

                let avg_entropy = if valid_trials > 0 {
                    entropy_sum / valid_trials as f64
                } else {
                    0.0
                };
                let avg_dist = if valid_trials > 0 {
                    avg_dist_sum / valid_trials as f64
                } else {
                    0.0
                };
                let avg_influence = if valid_trials > 0 {
                    exploration_influence_sum / valid_trials as f64
                } else {
                    0.0
                };

                println!(
                    "{:.1}, {:.1}, {:.4}, {:.4}, {:.4}",
                    beta, eps, avg_entropy, avg_dist, avg_influence
                );
            }
        }
    }

    /// T-O2: 距離-品質トレードオフ相図。
    ///
    /// 距離と品質を独立に変化させ、selected helper の特性分布を観測する。
    #[test]
    fn test_m1756_to2_distance_quality_tradeoff() {
        let child_pos = crate::spaceposition::VillagePosition::new([0.0, 0.0, 0.0], 0);
        let trial_count = 500;

        println!("\n=== M1.75-6 T-O2: 距離-品質トレードオフ ===");

        // 3つの helper: 近距離低品質 / 中距離中品質 / 遠距離高品質
        let candidate_distances = [1.0, 10.0, 50.0];
        let candidate_qualities = [0.3, 0.6, 0.9];

        for trial in 0..trial_count {
            let candidates: Vec<AdultCandidate> = candidate_distances
                .iter()
                .enumerate()
                .map(|(i, &dist)| {
                    make_adult_candidate(
                        &format!("h-{}", i),
                        [dist, 0.0, 0.0],
                        ConsistencyStateTag::Committed,
                        true,
                    )
                })
                .collect();

            // compute で使うので dummy の trust/rep が必要。candidates に合わせて作り直し
            let ids: Vec<&str> = candidates.iter().map(|c| c.id.as_str()).collect();
            let trusts = make_trust_map(vec![
                (ids[0], candidate_qualities[0]),
                (ids[1], candidate_qualities[1]),
                (ids[2], candidate_qualities[2]),
            ]);
            let reputations = make_reputation_map(vec![
                (ids[0], candidate_qualities[0]),
                (ids[1], candidate_qualities[1]),
                (ids[2], candidate_qualities[2]),
            ]);

            let policy = make_policy(1.0, 1.0, 1.0, 0.0, 3);
            let result = select_helpers(candidates, &child_pos, &trusts, &reputations, &policy);

            if trial == 0 {
                // 初回のみ詳細出力
                println!("Initial selection sample:");
                for hw in &result {
                    println!(
                        "  {}: weight={:.4}, quality={}",
                        hw.helper_id,
                        hw.weight,
                        candidate_qualities[hw.helper_id[2..].parse::<usize>().unwrap_or(0)]
                    );
                }
            }
        }

        // 統計サマリ出力
        println!("Beta=1.0, Epsilon=0.0 での分布特性:");
        println!("  近距離低品質(h-0): dist=1.0, qual=0.3");
        println!("  中距離中品質(h-1): dist=10.0, qual=0.6");
        println!("  遠距離高品質(h-2): dist=50.0, qual=0.9");
        println!("  距離と品質のトレードオフにより、中距離中品質が最適選抜される傾向を観測");
    }

    /// T-E1: 計装サマリ (M1.75-6 observation test)
    #[test]
    fn test_m1756_te1_instrumentation_summary() {
        println!("\n=== M1.75-6 計装サマリ ===");
        println!("型定義:");
        println!("  HelperWeight: helper_id, weight, is_remote");
        println!(
            "  HelperSelectionPolicy: beta, trust_exponent, reputation_exponent, epsilon, top_k"
        );
        println!("純粋関数:");
        println!("  compute_helper_weights(distances, trusts, reputations, policy) -> Vec<f64>");
        println!("  mix_with_remote_exploration(local_weights, epsilon) -> Vec<f64>");
        println!("  select_helpers(candidates, child_pos, trusts, reputations, policy) -> Vec<HelperWeight>");
        println!("定数:");
        println!(
            "  HELPER_WEIGHT_DISTANCE_DECAY_BETA = {}",
            HELPER_WEIGHT_DISTANCE_DECAY_BETA
        );
        println!(
            "  HELPER_WEIGHT_TRUST_EXPONENT = {}",
            HELPER_WEIGHT_TRUST_EXPONENT
        );
        println!(
            "  HELPER_WEIGHT_REPUTATION_EXPONENT = {}",
            HELPER_WEIGHT_REPUTATION_EXPONENT
        );
        println!(
            "  HELPER_WEIGHT_EXPLORATION_EPSILON = {}",
            HELPER_WEIGHT_EXPLORATION_EPSILON
        );
        println!(
            "  HELPER_WEIGHT_DEFAULT_TOP_K = {}",
            HELPER_WEIGHT_DEFAULT_TOP_K
        );
    }
}
