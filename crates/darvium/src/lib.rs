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

pub mod clock;
pub mod composition;
pub mod constants;
pub mod error;
pub mod event;
pub mod event_channel;
pub mod guard;
pub mod human_channel;
pub mod human_review_queue;
pub mod llm;
pub mod mock;
pub mod patch;
pub mod recovery;
pub mod search;
pub mod spaceposition;
pub mod store;
pub mod village;
pub mod trust;
pub mod types;
pub mod vector_index;

pub use types::{
    OscillationDetector, PlaneKind, RecursionGuard, SafeSandboxScope, SearchBudget,
    SearchBudgetSnapshot, SearchOutcome, SearchState, SideEffectSet, TerminalTransitionReason,
    VillageObservation,
};

pub use human_channel::{
    EventBusHumanChannel, FakeHumanChannel, HumanChannel, HumanChannelConfig, InteractionHandle,
    StdinoutChannel,
};
pub use human_review_queue::HumanReviewQueue;
pub use recovery::recover_pending_interactions;
pub use search::applicability::{
    check_ag06, check_ag07, EmbeddingChannelVersion, EmbeddingVersions,
};
pub use store::merge_and_deduplicate_candidates;
pub use store::{DualStoreCoordinator, JsonMetadataStore, MetadataStore, RepairScanSummary};

pub use guard::guard_new_proposal_or_review;

pub use trust::{apply_admin_fast_track, MemoizedGraph};

pub use types::{
    apply_self_conf_discount, check_budget_exceeded, evaluate_candidates, guard_budget_or_abort,
    guard_recursion_or_abort, CommitPhase, ConsistencyState, RepairAction, RepairLog, TrustUpdate,
};

pub use search::mock_proposer::{decide_composition_fate, CompositionDecision, MockProposer};
pub use types::ConfidenceVector;

pub use patch::{
    apply_operation, apply_patch_atomic, compute_validator_score, validate_patch_result,
    validate_subworkflow_refs, validate_var_scope, GraphPatch, PatchConfidence, PatchError,
    PatchOperation,
};

pub use event::{
    ConversationalEventEnvelope, DarviumEvent, DarviumEventBus, DarviumEventKind, DeliveryMode,
    EventCausality, EventFilter, EventId, EventMetadata, EventPrivacy, EventRetention, EventSource,
    EventSubscription, EventVisibility, EventProjection, FakeEventBus, FakeProjection,
    FakeProjectionCatalog, FusionEvent, GcEvent, HitlEvent, InteractionId, InteractionMode,
    DomainProjection, initialize_domain_projections, KnowledgeEvent, LifecycleEvent,
    PiiHandlingPolicy, ProjectionCatalog, ProjectionEventFilter, ReciprocityEvent, RepairEvent,
    SearchEvent, SpacePositionUpdatedPayload, SystemEvent, TrainingEvent, TransportMeta,
    VirtualClock, WorkflowExecutionEvent,
};

pub use event_channel::{
    CompatMode, EventChannel, StdinoutEventChannel, Subscription, WebSocketEventChannel,
};

pub use spaceposition::{
    is_position_equal, l2_distance, publish_position_update, should_update_position,
    update_space_position, PositionUpdatePolicy, SpacePositionEmbedding, VillagePosition,
};

pub use village::{
    build_local_village_radius, build_local_village_topk, classify_maturity,
    filter_adult_candidates, AdultCandidate, LocalVillage, WorkflowMaturity,
};

/// Darvium の公開 Facade。
///
/// MYCUTE はこの構造体のコンストラクタに設定を渡してインスタンス化し、
/// メソッド呼び出しだけで最適化された動作を享受する。
/// 内部の4層・Training・Fusion は完全にカプセル化される。
pub struct Darvium {
    #[allow(dead_code)]
    config: DarviumConfig,
}

impl Darvium {
    pub fn new(config: DarviumConfig) -> Self {
        Self { config }
    }
}

/// Darvium の公開設定。
///
/// 内部の複雑なパラメータ群はここに集約され、
/// MYCUTE 側は必要最小限のオプションのみを指定する。
#[derive(Debug, Clone, Default)]
pub struct DarviumConfig {}
