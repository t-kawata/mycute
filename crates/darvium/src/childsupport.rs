// child-support training mission orchestration (RFC §41B.11)
//
// 本モジュールは以下の機能を提供する：
// - ChildSupportMissionPayload 構造体 — child-support 固有のミッション情報
// - SafetyScope 列挙型 — safe sandbox 実行範囲の指定
// - spawn_child_support_mission — child-support mission の発行
// - is_allowed_on_plane — production plane 実行可否の判定
//
// M1.75-2 (village.rs) の LocalVillage / WorkflowMaturity に依存する。
// M1.75-4 (help.rs) の AdultHelpOfferPolicy と連携して動作する。

use std::time::SystemTime;

use crate::constants::{CHILD_SUPPORT_MISSION_TIMEOUT_SECS, MAX_HELPERS_PER_MISSION};
use crate::event::{
    DarviumEvent, DarviumEventBus, DarviumEventKind, EventCausality, EventMetadata, EventPrivacy,
    EventRetention, EventSource, EventVisibility, InteractionMode, PiiHandlingPolicy, TrainingEvent,
};
use crate::types::{TrainingMissionKind, WorkflowGraphId};
use crate::village::LocalVillage;

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
        assert!(!events.is_empty(), "EventBus にイベントが publish されるべき");
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
        assert_eq!(TrainingMissionKind::Production, TrainingMissionKind::Production);
        assert_eq!(TrainingMissionKind::ChildSupport, TrainingMissionKind::ChildSupport);
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
        assert_eq!(mission_events.len(), 1, "1件の MissionGenerated が publish されるべき");
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
                let adults: Vec<String> = (0..actual_size)
                    .map(|i| format!("adult-{}", i))
                    .collect();
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
            let adults: Vec<String> = (0..num_adults)
                .map(|i| format!("adult-{}", i))
                .collect();
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
}
