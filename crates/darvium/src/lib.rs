// Darvium — Crystallized Ecosystems of Knowledge and Capability
//
// # Architecture Overview
//
// Darvium は4層アーキテクチャで構成される：
//
// - Layer 2: Workflow IR (WorkflowGraph) — DAG によるワークフロー表現とコンパイル
// - Layer 3a: GMR Retrieval Core — セマンティック/構造的双ルート検索
// - Layer 3b: SearchWorkflow Engine — 検索を first-class workflow として定式化
// - Layer 3c: Lifecycle / Natural Selection / GC — 資産の寿命・淘汰・評判制御
//
// 上位層として Training Plane（人間参加型トレーニング）と
// Fusion Engine（リポジトリ対結合・知識抽出）を持つ。
//
// 仕様の絶対正本:
// - Darvium-RFC-0001-Unified-v2.0-final.md: 理論・設計・数式
// - Darvium-Tickets.md: フェーズ/チケット詳細
// - Darvium-v2.0-final-table-and-struct-definition-spec.md: データ/テーブル設計

#![allow(clippy::empty_line_after_doc_comments)]

pub mod calibration;
pub mod chief_registry;
pub mod childsupport;
pub mod clock;
pub mod compiler;
pub mod composition;
pub mod constants;
pub mod error;
pub mod event;
pub mod event_channel;
pub mod gmr;
pub mod graph_query;
pub mod guard;
pub mod help;
pub mod human_channel;
pub mod search_workflow;
pub mod self_refinement;
pub mod workflow_generation;
pub mod workflow_registry;
pub mod human_review_queue;
pub mod kind_world;
pub mod lifecycle;
pub mod llm;
pub mod mock;
pub mod patch;
pub mod reciprocity;
pub mod recovery;
pub mod replay;
pub mod report;
pub mod search;
pub mod simulation;
pub mod spaceposition;
pub mod store;
pub mod trust;
pub mod types;
pub mod vector_index;
pub mod village;

#[cfg(feature = "server")]
pub mod server;

pub use types::{
    ErrorMode, OscillationDetector, PlaneKind, RecursionGuard, SafeSandboxScope, SearchBudget,
    SearchBudgetSnapshot, SearchOutcome, SearchState, SideEffectSet, StepExecutionResult,
    StepStatus, TerminalTransitionReason, TrainingMissionKind, VillageObservation,
};

pub use compiler::compile_to_steps;

pub use human_channel::{
    EventBusHumanChannel, FakeHumanChannel, HumanChannel, HumanChannelConfig, InteractionHandle,
    StdinoutChannel,
};
pub use human_review_queue::HumanReviewQueue;
pub use recovery::recover_pending_interactions;
pub use search::applicability::{
    check_ag06, check_ag07, EmbeddingChannelVersion, EmbeddingVersions,
};
pub use search::pipeline::{
    evaluate_candidate, retrieve_top_level_candidates, ApplicabilityOutcome, PipelineTrace,
};
pub use store::merge_and_deduplicate_candidates;
pub use store::{
    AnnHotIndex, CacheError, CachePolicy, DualStoreCoordinator, JsonMetadataStore, MetadataStore,
    PersistenceError, RepairScanSummary, RepositoryPair, WorkflowCache,
};

pub use guard::guard_new_proposal_or_review;

pub use lifecycle::{compute_lifecycle_score, LifecycleScore};
pub use trust::{apply_admin_fast_track, inherit_reputation, inherit_trust, MemoizedGraph};

pub use types::{
    apply_self_conf_discount, check_budget_exceeded, evaluate_candidates, guard_budget_or_abort,
    guard_recursion_or_abort, CommitPhase, ConsistencyState, RepairAction, RepairLog, TrustUpdate,
};

pub use search::mock_proposer::{decide_composition_fate, CompositionDecision, MockProposer};
pub use types::AbstractableSubgraph;
pub use types::ConfidenceVector;

pub use search_workflow::SearchWorkflow;
pub use self_refinement::{
    check_needs_abstraction, extract_abstractable_subgraphs, register_abstracted_subworkflow,
    replace_with_subworkflow, run_self_refinement_round,
};
pub use workflow_generation::{generate_new_workflow, generate_differential_mutation, mission_to_embedding};
pub use workflow_registry::WorkflowRegistry;

pub use patch::{
    apply_operation, apply_patch_atomic, compute_validator_score, validate_patch_result,
    validate_subworkflow_refs, validate_var_scope, GraphPatch, PatchConfidence, PatchError,
    PatchOperation,
};

pub use event::{
    initialize_domain_projections, transition_gc_state, ConversationalEventEnvelope, DarviumEvent,
    DarviumEventBus, DarviumEventKind, DeliveryMode, DomainProjection, EventCausality, EventFilter,
    EventId, EventMetadata, EventPrivacy, EventProjection, EventRetention, EventSource,
    EventSubscription, EventVisibility, FakeEventBus, FakeProjection, FakeProjectionCatalog,
    FusionEvent, GcEvent, HitlEvent, InteractionId, InteractionMode, KnowledgeEvent,
    LifecycleEvent, PiiHandlingPolicy, ProjectionCatalog, ProjectionEventFilter, ReciprocityEvent,
    ReciprocityEventKind, ReciprocityLifecyclePolicy, RepairEvent, ReputationProfile, SearchEvent,
    SpacePositionUpdatedPayload, SystemEvent, TrainingEvent, TransportMeta, VillageEvent,
    VirtualClock, WorkflowExecutionEvent,
};

pub use event_channel::{
    CompatMode, EventChannel, EventSubscriber, ExternalEventClient, FakeExternalEventClient,
    FakeWebSocketEventChannel, StdinoutEventChannel, SubscriberManager, SubscriberSnapshot,
    SubscriberStatus, Subscription, SubscriptionId, WebSocketEventChannel,
};

#[cfg(feature = "server")]
pub use event_channel::BroadcastEventChannel;

pub use spaceposition::{
    is_position_equal, l2_distance, publish_position_update, should_update_position,
    update_space_position, PositionUpdatePolicy, SpacePositionEmbedding, VillagePosition,
};

pub use village::{
    build_local_village_radius, build_local_village_topk, classify_maturity,
    compute_child_maturation_time, compute_child_survival_rate, compute_helper_jsd,
    compute_position_drift, compute_village_churn, compute_village_jaccard,
    filter_adult_candidates, AdultCandidate, LocalVillage, VillageMetrics, VillageMetricsSnapshot,
    VillageMetricsWindow, WorkflowMaturity,
};

pub use help::{
    child_need_score, compute_offer_score_breakdown, decide_help_offer, emit_help_event,
    is_legal_help_transition, should_offer_help, transition_to_event_kind, AdultHelpOfferPolicy,
    ChildDecision, ChildHelpAcceptancePolicy, HelpDecision, HelpExecution, HelpFailure,
    HelpFailureReason, HelpMode, HelpOffer, HelpOfferState, HelpProposal, HelpRejectionReason,
    HelpSession, HelpState, HelpSuccess, OfferDecision, OfferScoreBreakdown,
};

pub use childsupport::{
    compute_helper_weights, is_allowed_on_plane, mix_with_remote_exploration, select_helpers,
    ChildSupportMissionPayload, HelperSelectionPolicy, HelperWeight, SafetyScope,
};

pub use calibration::{
    CalibrationPhase, CalibrationRolloutReport, PhaseGate, PhaseStatus,
    ReciprocityCalibrationConfig, ReciprocityCalibrationHarness, ReciprocityCalibrationReport,
    ReciprocityCalibrationResult, ReciprocityOperationalMetrics,
};
pub use replay::{
    apply_embedding_noise, apply_helper_quarantine, apply_single_edge_patch, apply_trust_delta,
    apply_usage_increment, compare_perturbed_metrics, run_replay_scenario, FailingSeedEntry,
    ReplayTrace, StabilityRegressionSummary, SummaryMetrics, VillageReplayScenario,
};

pub use report::{
    reciprocity_report_to_markdown, write_reciprocity_json_report,
    write_reciprocity_markdown_report, BestKnownParams, ExperimentLineage, FsLineageStore,
    LineageStore, ReciprocityExperimentReport, ReportError, VillageExperimentReport,
};

/// Darvium の公開 Facade。
///
/// MYCUTE はこの構造体のコンストラクタに設定を渡してインスタンス化し、
/// メソッド呼び出しだけで最適化された動作を享受する。
/// 内部の4層・Training・Fusion は完全にカプセル化される。
pub struct Darvium {
    config: DarviumConfig,
}

impl Darvium {
    pub fn new(config: DarviumConfig) -> Self {
        Self { config }
    }

    /// 決定論的リプレイシナリオを実行し、全 tick の状態を ReplayTrace として返す。
    pub fn run_replay_scenario(&self, scenario: &VillageReplayScenario) -> ReplayTrace {
        replay::run_replay_scenario(scenario)
    }

    /// Phase 0-4 較正パイプラインを直列実行する。
    ///
    /// Phase 0: 純粋関数検証
    /// Phase 1: 決定論的リプレイ
    /// Phase 2: 摂動テスト
    /// Phase 3: 合成シミュレーション sweep
    /// Phase 4: Human-reviewed rollout
    ///
    /// # 戻り値
    /// - `Ok(CalibrationRolloutReport)`: 全 Phase 通過、ロールアウトレポート
    /// - `Err(String)`: いずれかの Phase で失敗
    pub fn run_calibration_pipeline(
        &self,
    ) -> Result<crate::calibration::CalibrationRolloutReport, String> {
        calibration::run_all_phases()
    }

    /// 全ワークフローの評判を再計算する（明示的 tick モデル）。
    ///
    /// ReciprocityEventStore に蓄積された HELP イベントをもとに、
    /// 全ワークフローの ReputationProfile を F-1〜F-5 に従って再計算する。
    /// 呼び出し元（MYCUTE）は適切な間隔（例: RECOMPUTE_REPUTATION_INTERVAL tick 毎）
    /// でこのメソッドを呼び出す責任を持つ。
    pub fn recompute_reputations(
        &self,
        store: &crate::reciprocity::ReciprocityEventStore,
        metrics: &std::collections::HashMap<crate::types::WorkflowGraphId, crate::event::GraphMetrics>,
        now: u64,
        policy: &crate::event::ReciprocityLifecyclePolicy,
    ) -> std::collections::HashMap<crate::types::WorkflowGraphId, crate::event::ReputationProfile> {
        crate::reciprocity::recompute_all_profiles(store, metrics, now, policy)
    }

    /// LifecycleScore → GC hazard → gc_state 遷移パイプラインを全グラフに対して実行する。
    ///
    /// 各グラフの LifecycleScore 5成分（freshness/success/trust/usage/reputation）を計算し、
    /// `compute_gc_hazard` → `transition_gc_state` で gc_state を進行させる。
    /// `HardDeleteCandidate` 以上の状態に達したグラフの ID を削除候補として返す。
    ///
    /// # 引数
    /// - `registry`: 全グラフを保持する WorkflowRegistry（可変参照、gc_state を更新するため）
    /// - `policy`: ReciprocityLifecyclePolicy（GC hazard パラメータ）
    /// - `now`: 現在の仮想時刻 tick
    ///
    /// # 戻り値
    /// - `Vec<WorkflowGraphId>` — 削除候補（HardDeleteCandidate 以上）
    pub fn run_lifecycle_gc(
        &self,
        registry: &mut WorkflowRegistry,
        policy: &ReciprocityLifecyclePolicy,
        now: u64,
    ) -> Vec<crate::types::WorkflowGraphId> {
        if !self.config.gc_enabled {
            return Vec::new();
        }

        let graph_ids: Vec<crate::types::WorkflowGraphId> =
            registry.all_graphs().map(|g| g.id.clone()).collect();
        // 親生存マップ: parent_id (usize) → 親が生存中か (gc_state <= Active)
        let parent_alive_map: std::collections::HashMap<usize, bool> = registry
            .all_graphs()
            .filter_map(|g| {
                // WorkflowGraphId 形式: "wf-graph-{:016x}" または "wf-{:016x}" の hex をパース
                let pid = g.id
                    .strip_prefix("wf-graph-")
                    .or_else(|| g.id.strip_prefix("wf-"))
                    .and_then(|hex| usize::from_str_radix(hex, 16).ok())?;
                Some((
                    pid,
                    matches!(g.gc_state, GcEvent::Protected | GcEvent::Active),
                ))
            })
            .collect();
        let mut deletion_candidates = Vec::new();

        for graph_id in &graph_ids {
            let graph = match registry.resolve_mut(graph_id) {
                Ok(g) => g,
                Err(_) => continue,
            };

            let elapsed = now.saturating_sub(graph.last_update_tick);
            // 親生存ガード: parent_id > 0 の場合、親の生存状態をマップから取得
            let parent_is_alive = if graph.parent_id > 0 {
                Some(parent_alive_map.get(&graph.parent_id).copied().unwrap_or(false))
            } else {
                None
            };
            crate::lifecycle::compute_and_update_gc_state(
                graph,
                elapsed,
                self.config.lifecycle_success_stub,
                0.0,
                policy,
                now,
                parent_is_alive,
            );

            if matches!(graph.gc_state, GcEvent::HardDeleteCandidate | GcEvent::Tombstoned) {
                deletion_candidates.push(graph_id.clone());
            }
        }

        deletion_candidates
    }
}

/// Darvium の公開設定。
///
/// 内部の複雑なパラメータ群はここに集約され、
/// MYCUTE 側は必要最小限のオプションのみを指定する。
#[derive(Debug, Clone)]
pub struct DarviumConfig {
    /// GC tick 間隔 (default: 1000)。
    pub gc_interval: u64,
    /// GC 有効/無効。安全のためデフォルト false (opt-in)。
    pub gc_enabled: bool,
    /// Lifecycle success 成分スタブ値 (P6 未完成のため)。
    pub lifecycle_success_stub: f64,
}

impl Default for DarviumConfig {
    fn default() -> Self {
        Self {
            gc_interval: 1000,
            gc_enabled: false,
            lifecycle_success_stub: 0.5,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{GraphMetrics, ReciprocityLifecyclePolicy, ReputationProfile};
    use crate::reciprocity::ReciprocityEventStore;
    use std::collections::HashMap;

    /// T5: 空のストア → 空の結果
    #[test]
    fn test_t5_empty_store_returns_empty_result() {
        let darvium = Darvium::new(DarviumConfig::default());
        let store = ReciprocityEventStore::new();
        let metrics = HashMap::new();
        let policy = ReciprocityLifecyclePolicy::default();

        let result = darvium.recompute_reputations(&store, &metrics, 0, &policy);
        assert!(result.is_empty(), "T5: empty store should return empty result");
        println!("T5: empty store returns {} results", result.len());
    }

    /// T5b: 同一入力 → 同一出力（決定論性）
    #[test]
    fn test_t5b_deterministic_output() {
        let darvium = Darvium::new(DarviumConfig::default());
        let policy = ReciprocityLifecyclePolicy::default();

        // HELP 成功イベントをストアに追加
        let mut store = ReciprocityEventStore::new();
        use crate::event::ReciprocityEventKind;
        store.ingest(crate::event::ReciprocityEvent {
            event_id: "evt-1".into(),
            mission_id: "mission-1".into(),
            source_graph_id: "wf-1".into(),
            target_graph_id: "wf-2".into(),
            event_kind: ReciprocityEventKind::HelpSucceeded,
            weight: 1.0,
            created_at: std::time::SystemTime::now(),
            virtual_clock: 10,
            trace_ref: None,
        });

        let mut metrics = HashMap::new();
        metrics.insert(
            "wf-1".to_string(),
            GraphMetrics {
                centrality: 0.5,
                village_participation: 0.3,
                accepted_rate: 0.8,
                success_rate: 0.7,
                harm_score: 0.1,
                inherited_score: 0.0,
                experience_count: 5,
            },
        );
        metrics.insert(
            "wf-2".to_string(),
            GraphMetrics {
                centrality: 0.3,
                village_participation: 0.2,
                accepted_rate: 0.6,
                success_rate: 0.5,
                harm_score: 0.2,
                inherited_score: 0.0,
                experience_count: 3,
            },
        );

        // 同一入力を2回実行
        let result1 = darvium.recompute_reputations(&store, &metrics, 10, &policy);
        let result2 = darvium.recompute_reputations(&store, &metrics, 10, &policy);

        assert_eq!(result1.len(), result2.len(), "T5b: both runs should return same number of profiles");
        for (id, profile1) in &result1 {
            let profile2 = result2.get(id).expect("T5b: same ID should exist in both runs");
            assert_eq!(
                profile1.final_score, profile2.final_score,
                "T5b: final_score for {} should be identical across runs",
                id
            );
        }

        println!("T5b: deterministic — {} profiles, all scores match", result1.len());
    }

    /// T5c: ReputationProfile のシリアライズ・デシリアライズ ラウンドトリップ
    #[test]
    fn test_t5c_serialization_roundtrip() {
        let profile = ReputationProfile {
            direct_score: 0.7,
            indirect_score: 0.6,
            experience_score: 0.5,
            inherited_score: 0.4,
            final_score: 0.65,
            alpha_positive: 10,
            beta_negative: 2,
            last_recomputed_at: std::time::SystemTime::now(),
            direct_help_count: 8,
            direct_success_count: 6,
            direct_reject_count: 1,
            harm_event_count: 0,
            accepted_offer_rate: 0.75,
            help_success_rate: 0.8,
            village_centrality: 0.5,
            benevolence_score: 0.72,
            chiefdom_score: 0.5,
        };

        let serialized = serde_json::to_string(&profile).expect("T5c: serialize");
        let deserialized: ReputationProfile = serde_json::from_str(&serialized).expect("T5c: deserialize");

        assert!((deserialized.final_score - profile.final_score).abs() < 1e-6, "T5c: final_score roundtrip");
        assert_eq!(deserialized.alpha_positive, profile.alpha_positive, "T5c: alpha_positive roundtrip");
        assert_eq!(deserialized.experience_score, profile.experience_score, "T5c: experience_score roundtrip");

        println!("T5c: serialization roundtrip OK — {} -> {} -> {}",
            profile.final_score, serialized.len(), deserialized.final_score);
    }

    /// T1: run_lifecycle_gc 正常系 — 複数グラフで GC 実行、削除候補が返る
    #[test]
    fn test_t1_run_lifecycle_gc_normal() {
        let config = DarviumConfig {
            gc_enabled: true,
            ..Default::default()
        };
        let darvium = Darvium::new(config);
        let policy = ReciprocityLifecyclePolicy::default();
        let mut registry = WorkflowRegistry::new();

        for i in 0..3 {
            registry.register(MemoizedGraph::new(format!("gc-test-{}", i), 0.0));
        }

        // 2 回実行: Active → SoftDeleted → HardDeleteCandidate+
        let _first = darvium.run_lifecycle_gc(&mut registry, &policy, 100);
        let candidates = darvium.run_lifecycle_gc(&mut registry, &policy, 200);

        assert!(!candidates.is_empty(), "T1: deletion candidates expected after 2 GC runs");
        for id in &candidates {
            let graph = registry.resolve(id).expect("T1: graph exists");
            println!("T1: graph {} gc_state={:?}", id, graph.gc_state);
        }
        println!("T1: {} deletion candidates after 2 GC runs", candidates.len());
    }

    /// T2: 空レジストリ → 空リスト
    #[test]
    fn test_t2_empty_registry() {
        let config = DarviumConfig {
            gc_enabled: true,
            ..Default::default()
        };
        let darvium = Darvium::new(config);
        let policy = ReciprocityLifecyclePolicy::default();
        let mut registry = WorkflowRegistry::new();

        let candidates = darvium.run_lifecycle_gc(&mut registry, &policy, 100);
        assert!(candidates.is_empty(), "T2: empty registry should return empty");
        println!("T2: empty registry returns 0 candidates");
    }

    /// T3: gc_enabled: false → 空リスト（一切処理しない）
    #[test]
    fn test_t3_gc_disabled() {
        let config = DarviumConfig {
            gc_enabled: false,
            ..Default::default()
        };
        let darvium = Darvium::new(config);
        let policy = ReciprocityLifecyclePolicy::default();
        let mut registry = WorkflowRegistry::new();
        registry.register(MemoizedGraph::new("disabled-test".to_string(), 0.0));

        let candidates = darvium.run_lifecycle_gc(&mut registry, &policy, 100);
        assert!(candidates.is_empty(), "T3: gc_enabled=false should skip all processing");
        println!("T3: gc_enabled=false — 0 candidates, pipeline skipped");
    }

    /// T4: gc_state 遷移正当性 — 全遷移ルール検証
    #[test]
    fn test_t4_gc_state_transitions() {
        use crate::event::GcEvent;

        // Protected → Active (hazard > 0)
        assert_eq!(transition_gc_state(GcEvent::Protected, 0.1), GcEvent::Active);
        // Active → SoftDeleted (hazard > 0)
        assert_eq!(transition_gc_state(GcEvent::Active, 0.1), GcEvent::SoftDeleted);
        // SoftDeleted → HardDeleteCandidate (hazard > 0.5)
        assert_eq!(transition_gc_state(GcEvent::SoftDeleted, 0.6), GcEvent::HardDeleteCandidate);
        // HardDeleteCandidate → Tombstoned (hazard > 0.8)
        assert_eq!(transition_gc_state(GcEvent::HardDeleteCandidate, 0.9), GcEvent::Tombstoned);
        // 境界値: hazard=0 で遷移しない
        assert_eq!(transition_gc_state(GcEvent::Protected, 0.0), GcEvent::Protected);
        assert_eq!(transition_gc_state(GcEvent::Active, 0.0), GcEvent::Active);
        // Protected → Active (hazard>0), 直接 Tombstoned は禁止
        assert_eq!(transition_gc_state(GcEvent::Protected, 1.0), GcEvent::Active);
        println!("T4: all 7 transition rules verified");
    }

    /// T5: delete_graph で GraphStore から削除確認
    #[test]
    fn test_t5_delete_graph_via_coordinator() {
        use crate::store::DualStoreCoordinator;
        use crate::store::InMemoryGraphStore;
        use crate::store::InMemoryMetadataStore;

        let graph_store = Box::new(InMemoryGraphStore::new());
        let meta_store = Box::new(InMemoryMetadataStore::new());
        let coordinator = DualStoreCoordinator::new(graph_store, meta_store);

        let graph = MemoizedGraph::new("delete-test".to_string(), 0.5);
        coordinator.store_memoized_graph(&graph).expect("T5: store should succeed");

        // 削除前に存在確認
        let loaded = coordinator.load_memoized_graph("delete-test")
            .expect("T5: load before delete should succeed");
        assert_eq!(loaded.id, "delete-test", "T5: loaded graph id matches");

        // 削除
        coordinator.delete_graph("delete-test").expect("T5: delete should succeed");

        // 削除後はエラー
        let result = coordinator.load_memoized_graph("delete-test");
        assert!(result.is_err(), "T5: load after delete should fail");
        println!("T5: delete_graph OK — {:?}", result.err());
    }
}
