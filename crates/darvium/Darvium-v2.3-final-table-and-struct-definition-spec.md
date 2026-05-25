# Darvium v2.3-final テーブル定義及び構造体定義書（v2.3-h/v2.3-i 追補適用済み）

## 文書位置づけ

本書は **Darvium RFC-0001 — Unified Edition v2.3-final** の実装補助仕様であり、RFC 本文で規範化された source-of-truth 境界、状態機械、Trust / Lifecycle / Knowledge / Training / Fusion の不変条件を損なうことなく、SQLite 側テーブル定義、LadybugDB 側論理スキーマ、ならびに Rust 構造体定義を一貫した形で提示する独立文書である。[file:1]

本書の目的は「RFC を実装可能なスキーマへ落とす」ことであり、「RFC の意味論を変更する」ことではない。[file:1]
したがって、本書の各定義は RFC の additive clarification として解釈されなければならず、RFC 本文の規範に反する列、制約、状態遷移、source-of-truth 移譲を導入してはならない。[file:1]

## 設計原則

- SQLite は workflow / trust / lifecycle / lineage / audit / training / fusion metadata の正本である。[file:1]
- LadybugDB は knowledge object / relation / origin trace / evidence lineage の正本である。[file:1]
- dual-store consistency は database-native XA ではなく application-level commit intent protocol として実装する。[file:1]
- training artifact は promotion gate を通るまで production selection path を汚染してはならない。[file:1]
- fusion は non-destructive birth であり、input pair を in-place 更新してはならない。[file:1]
- v2.3-final では knowledge object の自動 semantic merge / truth arbitration は行わず、coexistence + lineage relation で扱う。[file:1]
- startup repair scan は起動時に `consistency_state != 'Committed'` の全資産を走査し、idempotent retry / NeedsRepair / Quarantined のいずれかへ明示的に遷移させる必須 recovery procedure であり、optional housekeeping ではない。[file:41A]
- Auto-Approval Exception Policy は safe sandbox scope に限定された optional policy (MAY) であり、audit log への完全記録および training / production separation の不変性を条件とする。[file:41A]

## 適用範囲

本書は以下を含む。[file:1]

- SQLite 物理テーブル定義。[file:1]
- LadybugDB 論理テーブル定義。[file:1]
- Rust 構造体・enum 定義。[file:1]
- インデックス、外部キー、整合制約、状態値の正本。[file:1]
- dual-store commit / repair / quarantine / birth commit を支える metadata 定義。[file:1]

本書は以下を含まない。[file:1]

- 分散 consensus / replication / partition handling。[file:1]
- graph embedding 専用 encoder の再導入。（v2.3-h: 構造検索が GED 系へ移行したため WorkflowDesignEmbedding / QueryDesignEmbedding は削除済み。）[file:1]
- knowledge object の semantic winner selection / dedup merge アルゴリズム。[file:1]
- RFC-0003 に委譲された search policy optimization / Pareto trust / Darwinian evolution。[file:1]

## 1. SQLite / LadybugDB の責務境界

| 領域 | 正本 | 説明 |
|---|---|---|
| WorkflowGraph blob / workflow metadata | SQLite | RepositoryPair（SQLite + LadybugDB）と GraphVersion の所有者。[file:1] |
| TrustProfile / Provenance / Metrics | SQLite | Applicability / Lifecycle / promotion 評価に関与する workflow-side state。[file:1] |
| Lifecycle / GC / Reputation / EnvironmentPolicy | SQLite | workflow asset の寿命制御。[file:1] |
| SearchTrace / SearchRunLog / TrustAuditLog / PatchHistory / LifecycleAuditLog | SQLite | 監査・再現・説明可能性のための正本ログ。[file:1] |
| TrainingMission / TrainingRunLog / TrainingFeedback / PromotionCandidate / TrainingAuditLog | SQLite | Training Plane の workflow-side formal object。[file:1] |
| ConversationalEventLog / ConversationalProposalLog / ConsolidationRunLog | SQLite | v2.3-c: conversational ingestion metadata の workflow-side 正本。[file:1] |
| FusionPlan / ExpertManifest / IdentityRemapTable / FusionAuditRecord / Pair birth state | SQLite | v2.0-final の fusion metadata 正本。[file:1] |
| HumanInteractions | SQLite | v2.3-d: HITL インタラクション永続化（HumanChannel communicate/reconnect のメタデータ）。リクエスト・応答・状態はメタデータであり LadybugDB の対象ではない (§12B.7)。[file:1] |
| Knowledge objects | LadybugDB | Fragment / MemoryEvent / MemoryConcept / CanonicalDocument / SkillNode / Chunk / Entity など。[file:1] |
| Knowledge relations | LadybugDB | `DERIVEDFROM`, `CONSOLIDATES`, `ABOUTCONCEPT`, `SUPERSEDES`, `MATERIALIZEDAS`, `COMPILEDTOSKILL` など。[file:1] |
| Origin trace / evidence lineage | LadybugDB | Knowledge Applicability と traceability の正本。[file:1] |

## 2. ID / 時刻 / 状態値の正規化規則

- すべての durable object ID は TEXT とし、UUID / ULID / namespaced stable id を許容する。[file:1]
- Unix epoch millisecond (UTC) を INTEGER で保存する。[file:1]
- bool は SQLite では INTEGER (`0` / `1`) とする。[file:1]
- enum は TEXT の canonical literal で保存する。[file:1]
- embedding ベクトルは SQLite では保持せず、ANN 用ベクトル本体は LadybugDB 側または外部ベクトルストアに置く。[file:1]
- workflow graph blob は canonical JSON または binary blob として SQLite に保持してよいが、意味論上は WorkflowGraph の正本でなければならない。[file:1]

## 3. SQLite 物理スキーマ

### 3.1 repository_pairs

```sql
CREATE TABLE repository_pairs (
    pair_id TEXT PRIMARY KEY,
    sqlite_schema_version TEXT NOT NULL,
    ladybug_schema_version TEXT NOT NULL,
    pair_kind TEXT NOT NULL,                  -- Production / Sandbox / Hybrid
    created_at_ms INTEGER NOT NULL,
    created_from_pair_id TEXT,
    birth_state TEXT NOT NULL,               -- BirthPending / BirthCommitted / BirthNeedsRepair / BirthQuarantined / BirthTombstoned
    current_op_id TEXT,
    namespace_policy TEXT,
    lineage_policy TEXT,
    training_policy TEXT,
    actor_id TEXT,
    notes TEXT
);
```

### 3.2 memoized_graphs

```sql
CREATE TABLE memoized_graphs (
    graph_id TEXT PRIMARY KEY,
    pair_id TEXT NOT NULL,
    ladybug_graph_id TEXT NOT NULL,
    graph_blob BLOB NOT NULL,
    graph_format TEXT NOT NULL,              -- json / bin
    graph_version INTEGER NOT NULL DEFAULT 0,
    workflow_kind TEXT NOT NULL DEFAULT 'Application',
    namespace TEXT,
    agentset_hash_u64 TEXT NOT NULL,
    workflow_design_text TEXT NOT NULL,
    design_template_version TEXT,
    task_embedding_ref TEXT,
    emb_task_model_version TEXT,
    trust_operational REAL NOT NULL,
    trust_semantic REAL NOT NULL,
    trust_human_score REAL NOT NULL,
    trust_human_count INTEGER NOT NULL,
    temporal_lambda_use REAL NOT NULL,
    temporal_lambda_verify REAL NOT NULL,
    temporal_alpha_blend REAL NOT NULL,
    time_decay_w_human REAL NOT NULL,
    time_decay_w_virtual REAL NOT NULL,
    time_decay_lambda_human_use REAL NOT NULL,
    time_decay_lambda_human_verify REAL NOT NULL,
    time_decay_lambda_virtual_use INTEGER NOT NULL,
    time_decay_lambda_virtual_verify INTEGER NOT NULL,
    metrics_success_rate REAL NOT NULL,
    metrics_avg_latency_ms INTEGER NOT NULL,
    metrics_token_cost_avg INTEGER NOT NULL,
    metrics_run_count INTEGER NOT NULL,
    metrics_last_run_at_ms INTEGER NOT NULL,
    prov_created_at_ms INTEGER NOT NULL,
    prov_last_used_at_ms INTEGER NOT NULL,
    prov_last_verified_at_ms INTEGER NOT NULL,
    prov_source_version TEXT NOT NULL,
    prov_environment_hash TEXT NOT NULL,
    last_virtual_seen INTEGER NOT NULL,
    experience_count INTEGER NOT NULL,
    reputation_direct_score REAL NOT NULL,
    reputation_indirect_score REAL NOT NULL,
    reputation_experience_score REAL NOT NULL,
    reputation_inherited_score REAL NOT NULL,
    reputation_final_score REAL NOT NULL,
    reputation_alpha_positive INTEGER NOT NULL,
    reputation_beta_negative INTEGER NOT NULL,
    reputation_last_recomputed_at_ms INTEGER NOT NULL,
    gc_state TEXT NOT NULL,
    gc_since_ms INTEGER,
    gc_reason TEXT,
    gc_consecutive_failures INTEGER,
    tombstone_id TEXT,
    tombstone_deleted_at_ms INTEGER,
    consistency_state TEXT NOT NULL,
    consistency_op_id TEXT,
    consistency_phase TEXT,
    consistency_reason TEXT,
    repair_epoch INTEGER NOT NULL DEFAULT 0,
    search_policy_json TEXT,
    latest_search_run_id TEXT,
    created_by_training_run_id TEXT,
    training_artifact_state TEXT,
    promotion_status TEXT,
    top_metadata_json TEXT NOT NULL DEFAULT '{}',          -- v2.3-h: TopLevelGraphMetadata JSON
    cheap_ged_signature_json TEXT NOT NULL DEFAULT '{}',   -- v2.3-h: CheapGedSignature JSON
    UNIQUE(pair_id, graph_id)
);
CREATE INDEX idx_memoized_graphs_pair ON memoized_graphs(pair_id);
CREATE INDEX idx_memoized_graphs_consistency ON memoized_graphs(consistency_state);
CREATE INDEX idx_memoized_graphs_gc_state ON memoized_graphs(gc_state);
CREATE INDEX idx_memoized_graphs_namespace ON memoized_graphs(namespace);
```

### 3.3 workflow_lineage

```sql
CREATE TABLE workflow_lineage (
    lineage_id TEXT PRIMARY KEY,
    graph_id TEXT NOT NULL,
    root_graph_id TEXT,
    parent_graph_id TEXT,
    generation INTEGER NOT NULL,
    derivation_kind TEXT NOT NULL,
    source_patch_id TEXT,
    source_training_run_id TEXT,
    source_fusion_op_id TEXT,
    created_at_ms INTEGER NOT NULL
);
CREATE INDEX idx_workflow_lineage_graph ON workflow_lineage(graph_id);
```

### 3.4 contribution_records

```sql
CREATE TABLE contribution_records (
    contribution_id TEXT PRIMARY KEY,
    graph_id TEXT NOT NULL,
    contributor_actor_id TEXT,
    contributor_public_key_ref TEXT,
    contributor_display_name_snapshot TEXT,
    identity_provider TEXT,
    contribution_kind TEXT NOT NULL,
    affected_user_count INTEGER,
    impact_score REAL,
    source_run_ids_json TEXT NOT NULL,
    source_feedback_ids_json TEXT NOT NULL,
    namespace TEXT,
    created_at_ms INTEGER NOT NULL
);
CREATE INDEX idx_contribution_records_graph ON contribution_records(graph_id);
```

### 3.5 trust_audit_log

```sql
CREATE TABLE trust_audit_log (
    audit_id INTEGER PRIMARY KEY AUTOINCREMENT,
    graph_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    actor_id TEXT NOT NULL,
    actor_public_key_ref TEXT,
    actor_display_name_snapshot TEXT,
    identity_provider TEXT,
    old_value REAL NOT NULL,
    new_value REAL NOT NULL,
    timestamp_ms INTEGER NOT NULL,
    reason TEXT
);
CREATE INDEX idx_trust_audit_graph ON trust_audit_log(graph_id);
```

### 3.6 patch_history

```sql
CREATE TABLE patch_history (
    patch_id TEXT PRIMARY KEY,
    source_graph_id TEXT NOT NULL,
    target_graph_id TEXT,
    diff_spec_hash TEXT NOT NULL,
    patch_blob BLOB NOT NULL,
    patch_format TEXT NOT NULL,
    patch_confidence REAL NOT NULL,
    llm_self_score REAL,
    validator_score REAL,
    historical_score REAL,
    success INTEGER NOT NULL,
    applied_at_ms INTEGER NOT NULL,
    applied_by_run_id TEXT,
    op_id TEXT
);
CREATE INDEX idx_patch_history_source_graph ON patch_history(source_graph_id);
```

### 3.7 determinism_observations / profiles

```sql
CREATE TABLE determinism_profiles (
    profile_id TEXT PRIMARY KEY,
    graph_id TEXT NOT NULL,
    node_id TEXT NOT NULL,
    prior_score REAL NOT NULL,
    estimated_score REAL NOT NULL,
    confidence REAL NOT NULL,
    sample_count INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL,
    UNIQUE(graph_id, node_id)
);

CREATE TABLE determinism_observations (
    observation_id TEXT PRIMARY KEY,
    graph_id TEXT NOT NULL,
    node_id TEXT NOT NULL,
    run_id TEXT NOT NULL,
    observed_score REAL NOT NULL,
    input_hash TEXT,
    output_hash TEXT,
    created_at_ms INTEGER NOT NULL
);
```

### 3.8 refinement_run_log / lifecycle_audit_log / repair_log

```sql
CREATE TABLE refinement_run_log (
    refinement_run_id TEXT PRIMARY KEY,
    graph_id TEXT NOT NULL,
    refinement_kind TEXT NOT NULL,
    input_summary TEXT,
    output_summary TEXT,
    token_cost INTEGER,
    success INTEGER NOT NULL,
    created_at_ms INTEGER NOT NULL
);

CREATE TABLE lifecycle_audit_log (
    lifecycle_audit_id TEXT PRIMARY KEY,
    graph_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    old_gc_state TEXT,
    new_gc_state TEXT,
    actor_id TEXT,
    reason TEXT,
    created_at_ms INTEGER NOT NULL
);

CREATE TABLE repair_log (
    repair_log_id TEXT PRIMARY KEY,
    op_id TEXT NOT NULL,
    graph_id TEXT,
    pair_id TEXT,
    detected_at_ms INTEGER NOT NULL,
    reason TEXT NOT NULL,
    action TEXT NOT NULL
);
```

### 3.9 search_run_log / search_trace_entries

```sql
CREATE TABLE search_run_log (
    search_run_id TEXT PRIMARY KEY,
    graph_id TEXT,
    mission_text TEXT NOT NULL,
    initial_policy_json TEXT,
    final_outcome TEXT,
    budget_token_limit INTEGER NOT NULL,
    budget_retrieval_limit INTEGER NOT NULL,
    budget_iteration_limit INTEGER NOT NULL,
    budget_wall_clock_ms INTEGER NOT NULL,
    status TEXT NOT NULL,
    started_at_ms INTEGER NOT NULL,
    finished_at_ms
);

CREATE TABLE search_trace_entries (
    trace_entry_id TEXT PRIMARY KEY,
    search_run_id TEXT NOT NULL,
    iteration INTEGER NOT NULL,
    state TEXT NOT NULL,
    query_text_hash TEXT,
    query_design_hash TEXT,
    selected_candidate_graph_id TEXT,
    selected_outcome TEXT,
    budget_snapshot_json TEXT NOT NULL,
    justification_hash TEXT,
    evidence_bundle_json TEXT,
    cheap_ged_signature_version TEXT,       -- v2.3-h: Stage 3 cheap GED signature version
    ged_cost_model_version TEXT,            -- v2.3-h: Stage 4 full GED cost model version
    created_at_ms INTEGER NOT NULL
);
CREATE INDEX idx_search_trace_run ON search_trace_entries(search_run_id, iteration);
```

### 3.10 virtual_clock_state / environment_policies

```sql
CREATE TABLE virtual_clock_state (
    singleton_key TEXT PRIMARY KEY CHECK (singleton_key = 'global'),
    current_value INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL
);

CREATE TABLE environment_policies (
    environment_name TEXT PRIMARY KEY,
    gc_theta_soft REAL NOT NULL,
    gc_theta_hard REAL NOT NULL,
    min_survival_experience INTEGER NOT NULL,
    reputation_weight REAL NOT NULL,
    inheritance_rate REAL NOT NULL,
    pressure_mode TEXT NOT NULL
);
```

### 3.11 training_missions / training_run_log / training_feedback

```sql
CREATE TABLE training_missions (
    training_mission_id TEXT PRIMARY KEY,
    pair_id TEXT NOT NULL,
    namespace TEXT,
    mission_source TEXT NOT NULL,
    review_status TEXT NOT NULL,
    mission_text TEXT NOT NULL,
    success_criteria_json TEXT NOT NULL,
    sandbox_policy_json TEXT NOT NULL,
    curriculum_policy_ref TEXT,
    duplicate_of_mission_id TEXT,
    approved_by_actor_id TEXT,
    created_at_ms INTEGER NOT NULL,
    reviewed_at_ms INTEGER
);

CREATE TABLE training_run_log (
    training_run_id TEXT PRIMARY KEY,
    training_mission_id TEXT NOT NULL,
    search_run_id TEXT,
    status TEXT NOT NULL,
    sandbox_operational_score REAL,
    sandbox_human_score REAL,
    curriculum_fit_score REAL,
    safety_score REAL,
    output_graph_id TEXT,
    output_knowledge_doc_id TEXT,
    token_cost INTEGER,
    started_at_ms INTEGER NOT NULL,
    finished_at_ms
);

CREATE TABLE training_feedback (
    feedback_id TEXT PRIMARY KEY,
    training_run_id TEXT NOT NULL,
    target_scope TEXT NOT NULL,
    target_id TEXT NOT NULL,
    rating TEXT NOT NULL,
    feedback_text TEXT,
    actor_id TEXT NOT NULL,
    actor_public_key_ref TEXT,
    actor_display_name_snapshot TEXT,
    identity_provider TEXT,
    created_at_ms INTEGER NOT NULL
);
```

### 3.12 promotion_candidates / training_audit_log / curriculum_queue

```sql
CREATE TABLE promotion_candidates (
    promotion_candidate_id TEXT PRIMARY KEY,
    training_run_id TEXT NOT NULL,
    candidate_kind TEXT NOT NULL,           -- Workflow / SubWorkflow / KnowledgeObject / QueryPattern
    candidate_id TEXT NOT NULL,
    promotion_status TEXT NOT NULL,
    evidence_summary TEXT,
    origin_trace_summary TEXT,
    human_gate_required INTEGER NOT NULL,
    approved_by_actor_id TEXT,
    created_at_ms INTEGER NOT NULL,
    decided_at_ms INTEGER
);

CREATE TABLE training_audit_log (
    training_audit_id TEXT PRIMARY KEY,
    training_mission_id TEXT,
    training_run_id TEXT,
    event_type TEXT NOT NULL,
    actor_id TEXT,
    reason TEXT,
    created_at_ms INTEGER NOT NULL
);

CREATE TABLE curriculum_queue (
    queue_id TEXT PRIMARY KEY,
    curriculum_policy_ref TEXT NOT NULL,
    training_mission_id TEXT NOT NULL,
    priority REAL NOT NULL,
    status TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL
);
```

### 3.13 fusion_plans / expert_manifests / identity_remap_entries / fusion_audit_records

```sql
CREATE TABLE fusion_plans (
    fusion_plan_id TEXT PRIMARY KEY,
    operation TEXT NOT NULL,
    output_pair_id TEXT NOT NULL,
    output_namespace_policy TEXT NOT NULL,
    lineage_policy TEXT NOT NULL,
    training_policy TEXT NOT NULL,
    id_remap_policy TEXT NOT NULL,
    human_review_required INTEGER NOT NULL,
    reason TEXT NOT NULL,
    actor_id TEXT,
    created_at_ms INTEGER NOT NULL
);

CREATE TABLE fusion_plan_inputs (
    fusion_plan_input_id TEXT PRIMARY KEY,
    fusion_plan_id TEXT NOT NULL,
    source_pair_id TEXT NOT NULL,
    sqlite_snapshot_ref TEXT NOT NULL,
    ladybug_snapshot_ref TEXT NOT NULL,
    selected_experts_json TEXT NOT NULL
);

CREATE TABLE expert_manifests (
    expert_id TEXT PRIMARY KEY,
    pair_id TEXT NOT NULL,
    namespace TEXT NOT NULL,
    kind TEXT NOT NULL,
    root_workflow_ids_json TEXT NOT NULL,
    root_knowledge_ids_json TEXT NOT NULL,
    includes_training_artifacts INTEGER NOT NULL,
    required_dependency_policy TEXT NOT NULL,
    optional_context_policy TEXT NOT NULL,
    selection_policy_json TEXT NOT NULL,
    is_provisional INTEGER NOT NULL DEFAULT 0,
    created_at_ms INTEGER NOT NULL
);
CREATE INDEX idx_expert_manifests_pair_namespace ON expert_manifests(pair_id, namespace);

CREATE TABLE identity_remap_entries (
    remap_entry_id TEXT PRIMARY KEY,
    fusion_op_id TEXT NOT NULL,
    source_pair_id TEXT NOT NULL,
    source_store TEXT NOT NULL,
    source_object_type TEXT NOT NULL,
    source_id TEXT NOT NULL,
    target_pair_id TEXT NOT NULL,
    target_id TEXT NOT NULL,
    preserved_namespace TEXT,
    remap_reason TEXT NOT NULL,
    UNIQUE(fusion_op_id, source_store, source_object_type, source_pair_id, source_id)
);

CREATE TABLE fusion_audit_records (
    fusion_op_id TEXT PRIMARY KEY,
    plan_id TEXT NOT NULL,
    operation TEXT NOT NULL,
    output_pair_id TEXT NOT NULL,
    selected_experts_json TEXT NOT NULL,
    lineage_policy TEXT NOT NULL,
    training_policy TEXT NOT NULL,
    result_state TEXT NOT NULL,
    actor_id TEXT,
    actor_public_key_ref TEXT,
    actor_display_name_snapshot TEXT,
    identity_provider TEXT,
    created_at_ms INTEGER NOT NULL,
    reason TEXT NOT NULL
);
```

### 3.14 conversational_event_log / conversational_proposal_log / consolidation_run_log (v2.3-c)

```sql
CREATE TABLE conversational_event_log (
    event_id TEXT PRIMARY KEY,
    session_id TEXT NOT NULL,
    user_id TEXT NOT NULL,
    actor TEXT NOT NULL,                    -- Human / Darvium / System
    timestamp_ms INTEGER NOT NULL,
    channel TEXT NOT NULL,
    redacted_text TEXT NOT NULL,
    raw_text_ref TEXT,
    policy_id TEXT NOT NULL
);
CREATE INDEX idx_conv_event_user_time ON conversational_event_log(user_id, timestamp_ms);

CREATE TABLE conversational_proposal_log (
    event_id TEXT PRIMARY KEY,
    proposed_category TEXT NOT NULL,
    policy_score REAL NOT NULL,
    llm_confidence REAL NOT NULL,
    contains_pii INTEGER NOT NULL DEFAULT 0,
    proposed_namespace TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL
);

CREATE TABLE consolidation_run_log (
    run_id TEXT PRIMARY KEY,
    namespace TEXT NOT NULL,
    candidate_set_id TEXT NOT NULL,
    candidate_id TEXT,
    semantic_coherence REAL NOT NULL,
    trace_completeness REAL NOT NULL,
    contradiction_score REAL NOT NULL,
    decision TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL
);
CREATE INDEX idx_consolidation_namespace ON consolidation_run_log(namespace);
```

### 3.15 human_interactions (v2.3-d)

```sql
CREATE TABLE human_interactions (
    interaction_id TEXT PRIMARY KEY NOT NULL,   -- UUID v4
    request_json   TEXT NOT NULL,                -- HumanRequest を JSON シリアライズ
    outcome_json   TEXT,                         -- HumanOutcome を JSON シリアライズ（Resolved 時のみ）
    status         TEXT NOT NULL DEFAULT 'Pending',  -- 'Pending' | 'Resolved'
    created_at_ms  INTEGER NOT NULL,             -- Unix エポックミリ秒 (UTC)
    updated_at_ms  INTEGER NOT NULL              -- 最終更新時刻 (Unix エポックミリ秒 UTC)
);

CREATE INDEX idx_human_interactions_status ON human_interactions(status);
```

## 4. LadybugDB 論理スキーマ

LadybugDB は RFC 上、knowledge object / relation の source-of-truth とされるが、v1.9 / v2.0-final は物理実装を完全固定していない。[file:1]
したがって本書では、RFC の object / relation / traceability 要件を満たす **最小十分な論理スキーマ** を canonical recommendation として定義する。[file:1]

### 4.1 knowledge_objects

```sql
CREATE TABLE knowledge_objects (
    knowledge_id TEXT PRIMARY KEY,
    pair_id TEXT NOT NULL,
    namespace TEXT,
    object_kind TEXT NOT NULL,              -- Fragment / MemoryEvent / MemoryConcept / CanonicalDocument / SkillNode / Chunk / Entity / CandidateKnowledgeDocument
    title TEXT,
    content_blob BLOB,
    content_text TEXT,
    media_type TEXT,
    status TEXT,
    valid_from_ms INTEGER,
    valid_to_ms INTEGER,
    stale INTEGER NOT NULL DEFAULT 0,
    promotion_status TEXT,
    origin_trace_root_id TEXT,
    evidence_completeness REAL,
    created_at_ms INTEGER NOT NULL,
    updated_at_ms INTEGER NOT NULL
);
CREATE INDEX idx_knowledge_objects_pair_kind ON knowledge_objects(pair_id, object_kind);
CREATE INDEX idx_knowledge_objects_namespace ON knowledge_objects(namespace);
```

### 4.2 knowledge_relations

```sql
CREATE TABLE knowledge_relations (
    relation_id TEXT PRIMARY KEY,
    pair_id TEXT NOT NULL,
    relation_type TEXT NOT NULL,            -- DERIVEDFROM / CONSOLIDATES / ABOUTCONCEPT / SUPERSEDES / MATERIALIZEDAS / COMPILEDTOSKILL
    src_knowledge_id TEXT NOT NULL,
    dst_knowledge_id TEXT NOT NULL,
    relation_weight REAL,
    evidence_summary TEXT,
    created_at_ms INTEGER NOT NULL
);
CREATE INDEX idx_knowledge_rel_src ON knowledge_relations(src_knowledge_id, relation_type);
CREATE INDEX idx_knowledge_rel_dst ON knowledge_relations(dst_knowledge_id, relation_type);
```

### 4.3 origin_traces / evidence_items

```sql
CREATE TABLE origin_traces (
    origin_trace_id TEXT PRIMARY KEY,
    pair_id TEXT NOT NULL,
    root_kind TEXT NOT NULL,
    root_ref_id TEXT NOT NULL,
    source_system TEXT,
    created_at_ms INTEGER NOT NULL
);

CREATE TABLE evidence_items (
    evidence_id TEXT PRIMARY KEY,
    origin_trace_id TEXT NOT NULL,
    knowledge_id TEXT,
    evidence_kind TEXT NOT NULL,
    version_context TEXT,
    freshness_summary TEXT,
    completeness_score REAL,
    created_at_ms INTEGER NOT NULL
);
```

### 4.4 entities / chunks / skill_nodes 専用補助表

```sql
CREATE TABLE entities (
    entity_id TEXT PRIMARY KEY,
    knowledge_id TEXT NOT NULL,
    canonical_name TEXT NOT NULL,
    entity_type TEXT,
    created_at_ms INTEGER NOT NULL
);

CREATE TABLE chunks (
    chunk_id TEXT PRIMARY KEY,
    knowledge_id TEXT NOT NULL,
    chunk_index INTEGER NOT NULL,
    text_content TEXT NOT NULL,
    token_count INTEGER,
    stale INTEGER NOT NULL DEFAULT 0,
    created_at_ms INTEGER NOT NULL,
    UNIQUE(knowledge_id, chunk_index)
);

CREATE TABLE skill_nodes (
    skill_node_id TEXT PRIMARY KEY,
    knowledge_id TEXT NOT NULL,
    parent_skill_node_id TEXT,
    skill_name TEXT NOT NULL,
    created_at_ms INTEGER NOT NULL
);
```

### 4.5 ANN / embedding registry

```sql
CREATE TABLE embedding_registry (
    embedding_ref TEXT PRIMARY KEY,
    owner_kind TEXT NOT NULL,               -- WorkflowTask / Chunk / KnowledgeObject
    owner_id TEXT NOT NULL,
    model_version TEXT NOT NULL,
    template_version TEXT,
    vector_blob BLOB,
    created_at_ms INTEGER NOT NULL
);
CREATE INDEX idx_embedding_registry_owner ON embedding_registry(owner_kind, owner_id);
```

## 5. Rust 構造体定義

以下は RFC と整合する canonical Rust shape である。[file:1]

```rust
pub type WorkflowGraphId = String;
pub type RepositoryPairId = String;
pub type KnowledgeId = String;
/// UTC ミリ秒 (UNIX epoch: 1970-01-01T00:00:00Z)
pub type TimestampMs = i64;

/// memoized_graphs テーブルの 1 行を表現する構造体。
/// RepositoryPair の SQLite 側に永続化される MemoizedGraph の正本レコード。
#[derive(Debug, Clone)]
pub struct MemoizedGraphRow {
    pub graph_id: WorkflowGraphId,
    pub pair_id: RepositoryPairId,
    pub ladybug_graph_id: String,
    pub graph_blob: Vec<u8>,
    pub graph_format: GraphFormat,
    pub graph_version: u64,
    pub workflow_kind: WorkflowKind,
    pub namespace: Option<String>,
    pub agentset_hash_u64: String,
    pub workflow_design_text: String,
    pub design_template_version: Option<String>,
    pub task_embedding_ref: Option<String>,
    pub emb_task_model_version: Option<String>,
    pub trust: TrustProfileRow,
    pub metrics: MetricsRow,
    pub provenance: ProvenanceRow,
    pub last_virtual_seen: u64,
    pub experience_count: u32,
    pub time_decay: TimeDecayProfileRow,
    pub reputation: ReputationProfileRow,
    pub gc: GcStateRow,
    pub consistency: ConsistencyStateRow,
    pub search_policy_json: Option<String>,
    pub latest_search_run_id: Option<String>,
    pub created_by_training_run_id: Option<String>,
    pub training_artifact_state: Option<TrainingArtifactState>,
    pub promotion_status: Option<PromotionStatus>,
    pub artifact_origin_kind: ArtifactOriginKind,          // v2.3-i: 出自種別
    pub preset_source_info: Option<PresetSourceInfo>,       // v2.3-i: preset source info
    pub root_policy: PresetRootPolicy,                      // v2.3-i: root 保護ポリシー
    pub capability_family: CapabilityFamily,                // v2.3-i: capability 分類
    pub registry_source: Option<RegistrySource>,            // v2.3-i: registry source
    pub top_metadata: TopLevelGraphMetadata,         // v2.3-h: 最上階 DAG メタデータ
    pub cheap_ged_signature: CheapGedSignature,      // v2.3-h: cheap GED 用 signature
}

#[derive(Debug, Clone)]
pub enum GraphFormat { Json, Bin }

#[derive(Debug, Clone)]
pub struct TrustProfileRow {
    pub operational: f32,
    pub semantic: f32,
    pub human_score: f32,
    pub human_count: u32,
    pub temporal: DualTemporalTrustRow,
}

#[derive(Debug, Clone)]
pub struct DualTemporalTrustRow {
    pub lambda_use: f32,
    pub lambda_verify: f32,
    pub alpha_blend: f32,
}

#[derive(Debug, Clone)]
pub struct MetricsRow {
    pub success_rate: f32,
    pub avg_latency_ms: u64,
    pub token_cost_avg: u32,
    pub run_count: u32,
    pub last_run_at_ms: TimestampMs,
}

#[derive(Debug, Clone)]
pub struct ProvenanceRow {
    pub created_at_ms: TimestampMs,
    pub last_used_at_ms: TimestampMs,
    pub last_verified_at_ms: TimestampMs,
    pub source_version: String,
    pub environment_hash: String,
    pub presetlineage: Option<String>,              // v2.3-i: preset lineage 追跡用
}

#[derive(Debug, Clone)]
pub struct TimeDecayProfileRow {
    pub w_human: f32,
    pub w_virtual: f32,
    pub lambda_human_use: f32,
    pub lambda_human_verify: f32,
    pub lambda_virtual_use: u64,
    pub lambda_virtual_verify: u64,
}

#[derive(Debug, Clone)]
pub struct ReputationProfileRow {
    pub direct_score: f32,
    pub indirect_score: f32,
    pub experience_score: f32,
    pub inherited_score: f32,
    pub final_score: f32,
    pub alpha_positive: u32,
    pub beta_negative: u32,
    pub last_recomputed_at_ms: TimestampMs,
}

#[derive(Debug, Clone)]
pub enum GcStateRow {
    Active,
    Protected { reason: String },                 // v2.3-i: root preset 保護（GC 対象外）
    SoftDeleted { since_ms: TimestampMs, reason: String },
    HardDeleteCandidate { since_ms: TimestampMs, consecutive_failures: u32 },
    Tombstoned { tombstone_id: String, since_ms: TimestampMs },
}

#[derive(Debug, Clone)]
pub enum ConsistencyStateRow {
    Committed,
    Pending { op_id: String, phase: CommitPhaseRow },
    NeedsRepair { op_id: String, reason: String },
    Quarantined { op_id: String, since_ms: TimestampMs },
}

#[derive(Debug, Clone)]
pub enum CommitPhaseRow {
    MetaPrepared,
    BlobPrepared,
    MetaCommitted,
    BlobCommitted,
}

#[derive(Debug, Clone)]
pub enum WorkflowKind {
    Application,
    Search,
}

#[derive(Debug, Clone)]
pub enum TrainingArtifactState {
    TrainingOnly,
    PromotionCandidate,
    Promoted,
    Rejected,
    Tombstoned,
}

#[derive(Debug, Clone)]
pub enum PromotionStatus {
    SandboxOnly,
    Candidate,
    Approved,
    Rejected,
    Promoted,
    RolledBack,
}

#[derive(Debug, Clone)]
pub struct TrainingMissionRow {
    pub training_mission_id: String,
    pub pair_id: RepositoryPairId,
    pub namespace: Option<String>,
    pub mission_source: MissionSource,
    pub review_status: MissionReviewStatus,
    pub mission_text: String,
    pub success_criteria_json: String,
    pub sandbox_policy_json: String,
    pub curriculum_policy_ref: Option<String>,
    pub duplicate_of_mission_id: Option<String>,
    pub approved_by_actor_id: Option<String>,
    pub created_at_ms: TimestampMs,
    pub reviewed_at_ms: Option<TimestampMs>,
}

#[derive(Debug, Clone)]
pub enum MissionSource {
    AiGenerated,
    HumanSubmitted,
    ReplayFromProduction,
    DerivedFromFailure,
}

#[derive(Debug, Clone)]
pub enum MissionReviewStatus {
    Pending,
    Approved,
    Rejected,
    Archived,
}

#[derive(Debug, Clone)]
pub struct TrainingFeedbackRow {
    pub feedback_id: String,
    pub training_run_id: String,
    pub target_scope: FeedbackTargetScope,
    pub target_id: String,
    pub rating: FeedbackRating,
    pub feedback_text: Option<String>,
    pub actor: Option<ActorRef>,
    pub created_at_ms: TimestampMs,
}

#[derive(Debug, Clone)]
pub enum FeedbackRating { Good, Bad, NeedsRevision, Irrelevant, Unsafe }

#[derive(Debug, Clone)]
pub enum FeedbackTargetScope { Mission, Workflow, SubWorkflow, KnowledgeObject, SearchPolicy }

#[derive(Debug, Clone)]
pub struct ActorRef {
    pub actor_id: String,
    pub public_key_ref: String,
    pub display_name_snapshot: Option<String>,
    pub identity_provider: String,
}

#[derive(Debug, Clone)]
pub struct ExpertManifestRow {
    pub expert_id: String,
    pub pair_id: RepositoryPairId,
    pub namespace: String,
    pub kind: ExpertKind,
    pub root_workflow_ids: Vec<WorkflowGraphId>,
    pub root_knowledge_ids: Vec<KnowledgeId>,
    pub includes_training_artifacts: bool,
    pub required_dependency_policy: RequiredDependencyPolicy,
    pub optional_context_policy: OptionalContextPolicy,
    pub selection_policy: ExpertSelectionPolicy,
    pub is_provisional: bool,
    pub created_at_ms: TimestampMs,
}

#[derive(Debug, Clone)]
pub enum ExpertKind { Production, Sandbox, Hybrid }

#[derive(Debug, Clone)]
pub enum RequiredDependencyPolicy { ClosureRequired, ExplicitOnly }

#[derive(Debug, Clone)]
pub enum OptionalContextPolicy { ExcludeAll, IncludeAuditAndLineage, IncludeFullReproducibilityContext }

#[derive(Debug, Clone)]
pub struct ExpertSelectionPolicy {
    pub allow_soft_deleted: bool,
    pub allow_training_only: bool,
    pub require_consistency_state: Vec<ConsistencyStateTag>,
}

#[derive(Debug, Clone)]
pub enum ConsistencyStateTag { Committed, Pending, NeedsRepair, Quarantined }

#[derive(Debug, Clone)]
pub struct FusionPlanRow {
    pub fusion_plan_id: String,
    pub operation: FusionOperation,
    pub inputs: Vec<FusionInputPair>,
    pub output: FusionOutputSpec,
    pub id_remap_policy: IdRemapPolicy,
    pub human_review_required: bool,
    pub reason: String,
    pub actor: Option<ActorRef>,
    pub created_at_ms: TimestampMs,
}

#[derive(Debug, Clone)]
pub enum FusionOperation { ExtractExpert, FuseExperts, SplitPairByExpert, RecomposePair }

#[derive(Debug, Clone)]
pub struct FusionInputPair {
    pub pair_id: RepositoryPairId,
    pub sqlite_snapshot: String,
    pub ladybug_snapshot: String,
    pub experts: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct FusionOutputSpec {
    pub target_pair_id: RepositoryPairId,
    pub output_namespace_policy: OutputNamespacePolicy,
    pub lineage_policy: LineagePolicy,
    pub training_policy: FusionTrainingPolicy,
}

#[derive(Debug, Clone)]
pub enum OutputNamespacePolicy { PreserveOriginal, RewriteTo(String), PrefixWith(String) }

#[derive(Debug, Clone)]
pub enum LineagePolicy { PreserveFull, PreserveByAncestorReference }

#[derive(Debug, Clone)]
pub enum FusionTrainingPolicy {
    ExcludeTrainingOnly,
    IncludePromotedOnly,
    IncludeCandidatesWithHumanGate,
    SandboxAllTraining,
}

#[derive(Debug, Clone)]
pub enum IdRemapPolicy { FullRegenerateWithTraceTable }

#[derive(Debug, Clone)]
pub struct IdentityRemapEntryRow {
    pub remap_entry_id: String,
    pub fusion_op_id: String,
    pub source_pair_id: RepositoryPairId,
    pub source_store: StoreKind,
    pub source_object_type: SourceObjectType,
    pub source_id: String,
    pub target_pair_id: RepositoryPairId,
    pub target_id: String,
    pub preserved_namespace: Option<String>,
    pub remap_reason: RemapReason,
}

#[derive(Debug, Clone)]
pub enum StoreKind { Sqlite, Ladybug }

#[derive(Debug, Clone)]
pub enum SourceObjectType { Workflow, Knowledge, RunLog, Audit, Relation, TrainingObject, PromotionObject }

#[derive(Debug, Clone)]
pub enum RemapReason { Extract, Fuse, Split, Recompose }

#[derive(Debug, Clone)]
pub struct KnowledgeObjectRow {
    pub knowledge_id: KnowledgeId,
    pub pair_id: RepositoryPairId,
    pub namespace: Option<String>,
    pub object_kind: KnowledgeObjectKind,
    pub title: Option<String>,
    pub content_blob: Option<Vec<u8>>,
    pub content_text: Option<String>,
    pub media_type: Option<String>,
    pub status: Option<String>,
    pub valid_from_ms: Option<TimestampMs>,
    pub valid_to_ms: Option<TimestampMs>,
    pub stale: bool,
    pub promotion_status: Option<PromotionStatus>,
    pub origin_trace_root_id: Option<String>,
    pub evidence_completeness: Option<f32>,
    pub created_at_ms: TimestampMs,
    pub updated_at_ms: TimestampMs,
}

#[derive(Debug, Clone)]
pub enum KnowledgeObjectKind {
    Fragment,
    MemoryEvent,
    MemoryConcept,
    CanonicalDocument,
    SkillNode,
    Chunk,
    Entity,
    CandidateKnowledgeDocument,
}

// ---- v2.3-c: Conversational Knowledge Path types ----

#[derive(Debug, Clone)]
pub struct ConversationalEvent {
    pub event_id: String,
    pub session_id: String,
    pub user_id: String,
    pub actor: ConversationActor,
    pub utterance: String,
    pub timestamp: TimestampMs,
    pub language: String,
    pub context_window_id: Option<String>,
    pub parent_event_ids: Vec<String>,
    pub source_channel: ConversationChannel,
}

#[derive(Debug, Clone)]
pub enum ConversationActor { Human, Darvium, System }

#[derive(Debug, Clone)]
pub enum ConversationChannel { Chat, VoiceTranscript, ImportedLog, EmailBridge, Api }

#[derive(Debug, Clone)]
pub struct ConversationalIngestionPolicy {
    pub policy_id: String,
    pub namespace_template: String,
    pub allow_auto_sandbox_ingest: bool,
    pub require_human_review_for_promotion: bool,
    pub max_candidate_span_days: u32,
    pub min_policy_score: f32,
    pub min_promotion_score: f32,
    pub allow_raw_transcript_persistence: bool,
    pub pii_handling: PiiHandlingPolicy,
    pub retention: RetentionPolicy,
    pub category_rules: Vec<ConversationCategoryRule>,
    pub updated_at: TimestampMs,
}

#[derive(Debug, Clone)]
pub struct ConversationCategoryRule {
    pub category: ConversationalKnowledgeCategory,
    pub allowed_namespace_suffix: String,
    pub auto_ingest_to_sandbox: bool,
    pub eligible_for_consolidation: bool,
    pub eligible_for_promotion: bool,
    pub require_origin_trace: bool,
    pub minimum_distinct_events: u32,
    pub minimum_distinct_days: u32,
    pub minimum_llm_confidence: f32,
}

#[derive(Debug, Clone)]
pub enum ConversationalKnowledgeCategory {
    UserProfile, UserPreference, LongLivedProjectContext, StableConstraint,
    TemporaryTaskContext, FactualClaim, Reflection, RelationshipFact,
    Noise, Unsafe, Unknown,
}

#[derive(Debug, Clone)]
pub struct RetentionPolicy {
    pub raw_event_ttl_days: u32,
    pub sandbox_candidate_ttl_days: u32,
    pub rejected_candidate_tombstone_hours: u32,
}

#[derive(Debug, Clone)]
pub enum PiiHandlingPolicy { Reject, RedactBeforePersist, AllowSandboxOnly }

#[derive(Debug, Clone)]
pub struct ConversationalClassificationProposal {
    pub event_id: String,
    pub proposed_category: ConversationalKnowledgeCategory,
    pub policy_score: f32,
    pub llm_confidence: f32,
    pub rationale_summary: String,
    pub proposed_namespace: String,
    pub extractive_facts: Vec<String>,
    pub inferred_temporality: InferredTemporality,
    pub inferred_scope: InferredScope,
    pub contains_pii: bool,
    pub promotion_eligibility_hint: PromotionEligibilityHint,
}

#[derive(Debug, Clone)]
pub enum InferredTemporality { Ephemeral, Stable, Historical, Mixed }

#[derive(Debug, Clone)]
pub enum InferredScope { Personal, Project, Global, Ambiguous }

#[derive(Debug, Clone)]
pub enum PromotionEligibilityHint { Never, SandboxOnly, ReviewRequired, PotentiallyPromotable }

#[derive(Debug, Clone)]
pub struct ConversationalGateDecision {
    pub event_id: String,
    pub action: ConversationalGateAction,
    pub target_namespace: Option<String>,
    pub normalized_facts: Vec<String>,
    pub reason_code: String,
    pub requires_human_review: bool,
    pub created_mission: Option<String>,
}

#[derive(Debug, Clone)]
pub enum ConversationalGateAction {
    Drop, StoreRawEventOnly, StoreFragmentOnly, CreateTrainingMission,
    CreateTrainingMissionAndFragment, QueueForConsolidation,
}

#[derive(Debug, Clone)]
pub struct ConversationalMissionPayload {
    pub mission_id: String,
    pub source_event_ids: Vec<String>,
    pub user_id: String,
    pub namespace: String,
    pub category: ConversationalKnowledgeCategory,
    pub normalized_facts: Vec<String>,
    pub mission_text: String,
    pub success_criteria: Vec<String>,
    pub review_required: bool,
    pub created_at: TimestampMs,
}

#[derive(Debug, Clone)]
pub struct ConversationalFragmentMeta {
    pub fragment_id: String,
    pub source_event_ids: Vec<String>,
    pub user_id: String,
    pub namespace: String,
    pub category: ConversationalKnowledgeCategory,
    pub redacted_summary: String,
    pub extracted_facts: Vec<String>,
    pub distinct_day_count: u32,
    pub first_seen_at: TimestampMs,
    pub last_seen_at: TimestampMs,
}

#[derive(Debug, Clone)]
pub struct ConsolidationCandidateSet {
    pub set_id: String,
    pub namespace: String,
    pub category: ConversationalKnowledgeCategory,
    pub fragment_ids: Vec<String>,
    pub source_event_ids: Vec<String>,
    pub distinct_event_count: u32,
    pub distinct_day_count: u32,
    pub semantic_coherence: f32,
    pub trace_completeness: f32,
    pub temporal_stability: f32,
    pub contradiction_score: f32,
    pub created_at: TimestampMs,
}

#[derive(Debug, Clone)]
pub struct ConsolidationPolicy {
    pub min_distinct_events: u32,
    pub min_distinct_days: u32,
    pub min_semantic_coherence: f32,
    pub min_trace_completeness: f32,
    pub min_temporal_stability: f32,
    pub max_contradiction_score: f32,
    pub require_origin_trace: bool,
    pub allow_auto_candidate_creation: bool,
    pub allow_auto_promotion: bool,
}

#[derive(Debug, Clone)]
pub struct ConversationalPromotionGate {
    pub candidate_id: String,
    pub namespace: String,
    pub category: ConversationalKnowledgeCategory,
    pub llm_policy_score: f32,
    pub completeness_score: f32,
    pub trace_completeness: f32,
    pub contradiction_score: f32,
    pub distinct_day_count: u32,
    pub training_good_ratio: f32,
    pub sandbox_success_rate: f32,
    pub requires_human_review: bool,
}
```

// ---- v2.3-d: HumanChannel HITL 型定義 ----

```rust
/// 人間への通知リクエスト。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HumanRequest {
    pub subject: String,
    pub body: String,
    pub context: Option<serde_json::Value>,
    pub timeout: Option<std::time::Duration>,
}

/// 人間との通信結果。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HumanOutcome {
    Responded(HumanResponse),
    TimedOut,
    Unreachable,
}

/// 人間からの応答内容。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HumanResponse {
    pub decision: HumanDecision,
    pub comment: Option<String>,
    pub revised_body: Option<String>,
}

/// 人間の判断（5値）。
/// TrainingFeedback の FeedbackRating（Good/Bad/NeedsRevision/Irrelevant/Unsafe）と
/// 1:1 対応する（Approved↔Good, Rejected↔Bad, NeedsRevision↔NeedsRevision,
/// Irrelevant↔Irrelevant, Unsafe↔Unsafe）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HumanDecision {
    Approved,
    Rejected,
    NeedsRevision,
    Irrelevant,
    Unsafe,
}

/// MetadataStore に永続化される HITL インタラクション (v2.3-g 型エイリアス)。
/// InteractionRecord<HitlPayload> のエイリアスとして再定義される。
/// 後方互換のため StoredInteraction としての公開インタフェースは保持される。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub type StoredInteraction = InteractionRecord<HitlPayload>;

/// InteractionRecord<HitlPayload> の実体となるペイロード (v2.3-g 追加)。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HitlPayload {
    pub request: HumanRequest,
}

impl InteractionPayload for HitlPayload {
    type Outcome = HumanOutcome;
}

/// インタラクションの状態。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum InteractionStatus {
    Pending,
    Resolved,
}

/// ブロッキング待機機構を提供するハンドル。
/// communicate() / reconnect() から即時返却される。
#[derive(Debug)]
pub struct InteractionHandle {
    pub interaction_id: uuid::Uuid,
    rx: std::sync::mpsc::Receiver<Result<HumanOutcome, DarviumError>>,
}

impl InteractionHandle {
    pub fn wait(self, timeout: Option<std::time::Duration>)
        -> Result<HumanOutcome, DarviumError>;
}

// DarviumError (HumanChannel 関連バリアント, v2.3-d)
#[derive(Debug, thiserror::Error)]
enum DarviumError {
    #[error("Human channel I/O error: {0}")]
    HumanChannelIo(String),
    #[error("Human channel disconnected")]
    HumanChannelClosed,
}
```

// ---- v2.3-h: 4 層検索用データ型 ----

```rust
/// 副作用セット。TopLevelGraphMetadata / TopLevelQueryMetadata で使用。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SideEffectSet {
    pub writes_external_api: bool,
    pub sends_notification: bool,
    pub has_hitl_communicate: bool,
    pub modifies_persistent_state: bool,
}

/// v2.3-h: 最上階 WorkflowGraph の軽量メタデータ（SQLite metadata filter Stage 2 入力）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopLevelGraphMetadata {
    pub top_node_count: u16,
    pub top_edge_count: u16,
    pub top_source_count: u16,
    pub top_sink_count: u16,
    pub top_longest_path_len: u16,
    pub top_max_width: u16,
    pub top_label_histogram: Vec<(String, u16)>,
    pub top_edge_type_histogram: Vec<(String, u16)>,
    pub top_determinism_summary: f32,
    pub top_sideeffect_summary: SideEffectSet,
    pub top_agentsethash: u64,
    pub top_layer_signature: Vec<u64>,
}

/// v2.3-h: cheap GED 用 replayable deterministic graph signature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheapGedSignature {
    pub topo_rank_labels: Vec<u64>,
    pub indegree_histogram: Vec<u16>,
    pub outdegree_histogram: Vec<u16>,
    pub ancestor_bitset_sketch: Vec<u64>,
    pub descendant_bitset_sketch: Vec<u64>,
    pub path_hash_multiset: Vec<(u64, u16)>,
    pub signature_version: String,
}
```

// ---- v2.3-i: Preset Registry データ型 ----

```rust
/// 出自種別。WorkflowGraph の生成経路を識別する。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ArtifactOriginKind {
    PresetSystem,
    PresetUser,
    SearchGenerated,
    TrainingDerived,
    FusionDerived,
    Conversational,
    Manual,
}

/// Registry source 識別子。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RegistrySource {
    BakedPlatform,
    MutableUser,
    MutableWorkspace,
}

/// Capability 分類。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CapabilityFamily {
    StructMem,
    Corpus2Skill,
    Search,
    Training,
    General,
}

/// Root preset 保護ポリシー。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PresetRootPolicy {
    pub immutable_root: bool,
    pub root_pinned: bool,
    pub boot_critical: bool,
    pub capability_family: CapabilityFamily,
}

/// Preset メタデータ。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PresetMetadata {
    pub workflow_id: String,
    pub kind: PresetKind,
    pub preset_source: RegistrySource,
    pub preset_scope: String,
    pub preset_trust_class: TrustClass,
    pub boot_critical: bool,
    pub immutable_root: bool,
    pub root_pinned: bool,
    pub depends_on: Vec<String>,
    pub knowledge_capability: Option<CapabilityFamily>,
    pub version: String,
}

/// Preset 種別。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PresetKind {
    PresetWorkflow,
}

/// 信頼クラス。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TrustClass {
    Trusted,
    Untrusted,
}

/// Preset 検証理由（12 variant）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PresetValidationReason {
    InvalidPresetSchema,
    DuplicateWorkflowId,
    ReservedNamespaceViolation,
    WorkflowNotFound,
    CrossRegistryDependencyViolation,
    CircularReference,
    InvalidInputMapping,
    OutputBindingMismatch,
    BootCriticalPresetMissing,
    BootCriticalPresetInvalid,
    MutableOverrideForbidden,
    PresetPolicyViolation,
}

/// Preset 検証失敗レコード。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PresetValidationFailure {
    pub workflowid: Option<String>,
    pub source: RegistrySource,
    pub source_path: Option<String>,
    pub reasons: Vec<PresetValidationReason>,
    pub detected_at: SystemTime,
}

/// Preset source info（MemoizedGraph metadata 用）。RFC §8 定義に準拠。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PresetSourceInfo {
    pub registry_source: RegistrySource,
    pub preset_metadata: PresetMetadata,
    pub loaded_at: SystemTime,
    pub validated_at: SystemTime,
}

/// Preset Registry イベント（DarviumEventKind::PresetRegistry）。RFC §12C 定義に準拠。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PresetRegistryEvent {
    StartupValidationStarted { source: RegistrySource, timestamp: SystemTime },
    StartupValidationCompleted { accepted_count: usize, quarantined_count: usize, timestamp: SystemTime },
    PresetAccepted { workflow_id: String, source: RegistrySource },
    PresetQuarantined { failure: PresetValidationFailure },
    CollisionResolved { workflow_id: String, resolution: String },
}

/// PresetWorkflow — BakedPresetRegistry / MutablePresetRegistry に格納される事前定義ワークフロー。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PresetWorkflow {
    pub metadata: PresetMetadata,
    pub workflow: PresetWorkflowGraph,
    pub root_policy: PresetRootPolicy,
}

/// PresetWorkflowGraph — PresetWorkflow 内のワークフローグラフ（stub）。
/// 実際の WorkflowGraph は M-2 以降で定義される。本マイルストーンでは ID とプレースホルダのみ保持する。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PresetWorkflowGraph {
    pub graph_id: String,
    pub description: String,
}
```

## 6. 整合制約

- `memoized_graphs.consistency_state != 'Committed'` の行は通常の REUSE / PATCH / COMPOSE / production fusion に使ってはならない。[file:1]
- `training_artifact_state = 'TrainingOnly'` は `FusionTrainingPolicy = SandboxAllTraining` 以外では output production pair に入れてはならない。[file:1]
- `gc_state = 'Tombstoned'` の asset は active object として再導入してはならず、ancestor reference のみ許される。[file:1]
- `identity_remap_entries` は materialized object ごとに完全でなければならない。[file:1]
- `knowledge_relations` による `CONSOLIDATES` / `SUPERSEDES` は lineage relation であり、v2.3-final における自動 semantic collapse を意味しない。[file:1]
- 起動時および定期 repair worker は `consistency_state != 'Committed'` の全資産を走査し、commit intent を再確認、idempotent retry / NeedsRepair / Quarantined のいずれかへ明示的に遷移させなければならない (MUST)。startup repair scan は normal selection path に戻す前の必須 recovery procedure である。[file:41A]
- Auto-Approval Exception Policy を導入する場合、少なくとも namespace / artifact kind / side-effect envelope / resource budget / external write 禁止 / production promotion 不可の条件で bounded に定義し、auto-approval の事実・適用 policy ID・理由・scope boundary・実行 trace を audit log に残さなければならない (MUST)。この policy は training / production separation を弱めてはならない (MUST NOT)。[file:41A]
- `repository_pairs.birth_state != 'BirthCommitted'` の pair を production selection path に入れてはならない。[file:1]
- Conversational origin knowledge MUST NOT bypass the four-stage pipeline (ConversationalEvent → Fragment/MemoryEvent → CandidateKnowledgeDocument → CanonicalDocument). Direct mutation of production canonical knowledge from conversational input is forbidden regardless of gate presence.[file:1]
- A conversational CandidateKnowledgeDocument whose contradiction_score exceeds the policy-declared max_contradiction_score MUST NOT be automatically canonicalized. Default safe action is coexistence + lineage relation, not destructive merge.[file:1]
- A conversational artifact in `consistency_state != 'Committed'` SHALL transition to NeedsRepair or Quarantined and MUST NOT appear in normal REUSE / PATCH / COMPOSE paths.[file:1]
- human_interactions テーブルの status は `'Pending'` または `'Resolved'` のみを許容する。これ以外の値が格納されてはならない (MUST NOT)。[file:1]
- status = `'Pending'` の human_interactions 行は、システム再起動後必ず `reconnect()` による回復を試行しなければならない (MUST)。回復不能と判断された場合は明示的に `HumanOutcome::Unreachable` に遷移させる。[file:1]
- `HumanChannel` トレイトは transport のみを抽象化する。インタラクションの永続化（store/load/list/resolve）は `MetadataStore` の責務であり、`HumanChannel` 実装内でストレージに直接書き込んではならない (MUST NOT)。[file:1]
- `HumanOutcome::Responded` に含まれる `HumanDecision` の 5 値（Approved/Rejected/NeedsRevision/Irrelevant/Unsafe）は `TrainingFeedback::FeedbackRating` の 5 値（Good/Bad/NeedsRevision/Irrelevant/Unsafe）と 1:1 対応する。両者の変換マッピングは Orchestrator 層で実装されなければならない (MUST)。`HumanChannel` 実装内でこの変換を行ってはならない (MUST NOT)。[file:1]

## 7. v2.3-h / v2.3-i 改訂追補

本定義書は v2.3-h 改訂に伴い以下の更新が加えられている。

- **memoized_graphs テーブル**: `top_metadata_json` / `cheap_ged_signature_json` カラム追加（v2.3-h: TopLevelGraphMetadata / CheapGedSignature の JSON 永続化）
- **search_trace_entries テーブル**: `cheap_ged_signature_version` / `ged_cost_model_version` カラム追加（§12.3C: cost model version の replay 用記録）
- **MemoizedGraphRow**: `top_metadata: TopLevelGraphMetadata` / `cheap_ged_signature: CheapGedSignature` フィールド追加
- **新規構造体**: `TopLevelGraphMetadata`（12 フィールド）、`CheapGedSignature`（7 フィールド）、`SideEffectSet`（v2.3-h 新規型として正規化）
- **embedding_registry**: 旧 `WorkflowDesign` / `QueryDesign` owner_kind は削除（構造検索は GED 系へ移行）

v2.3-i 改訂に伴い以下の更新が加えられている。

- **§7 header**: `v2.3-h` → `v2.3-h/v2.3-i`
- **MemoizedGraphRow**: 5 フィールド追加（`artifact_origin_kind: ArtifactOriginKind` / `preset_source_info: Option<PresetSourceInfo>` / `root_policy: PresetRootPolicy` / `capability_family: CapabilityFamily` / `registry_source: Option<RegistrySource>`）
- **ProvenanceRow**: `presetlineage: Option<String>` フィールド追加
- **GcStateRow**: `Protected { reason: String }` variant 追加
- **新規構造体/列挙型**: `ArtifactOriginKind` / `RegistrySource` / `CapabilityFamily` / `PresetRootPolicy` / `PresetMetadata` / `PresetKind` / `TrustClass` / `PresetValidationReason` / `PresetValidationFailure` / `PresetSourceInfo` / `PresetRegistryEvent` / `PresetWorkflow` / `PresetWorkflowGraph`（13 型、v2.3-i Preset Registry 基盤）
- `PresetSourceInfo` および `PresetRegistryEvent` は RFC §8 および §12C の定義に準拠。
- 本 v2.3-i 改訂による SQLite スキーマ・LadybugDB 論理スキーマへの変更はない。全 Preset Registry 型はメモリ内完結であり、永続化層には影響しない。

v2.3-h 改訂は v2.3-g（Event Architecture）と完全に直交し、v2.3-g で追加された以下のスキーマ・型定義に一切の変更を加えない:
- §12C DarviumEvent / DarviumEventKind / DarviumEventBus 関連
- §12D External Event Subscription 関連
- §12E Event Projection Framework 関連
- human_interactions テーブル（v2.3-d）

## 8. 実装上の補足

この定義書は、RFC に存在する情報だけで矛盾なく構成できる範囲を最大限 formalize したものである。[file:1]
特に LadybugDB 側は RFC 本体が概念スキーマ中心であるため、本書はその意味論を壊さない **canonical recommendation** として物理定義を提示している。[file:1]

したがって、列名やインデックスの微修正は実装都合で許容されるが、次の点は変更してはならない。[file:1]

- source-of-truth 境界
- dual-store commit / repair / quarantine の意味論
- training / production separation
- non-destructive fusion
- identity remap による full traceability
- automatic semantic merge を行わない方針
