# Darvium RFC-0001 v2.3 更新指示書

本書は、Darvium RFC-0001 Unified Edition v2.3 を、会話由来の知識取り込み・断片蓄積・個人化成長・図書館化（canonicalization）まで含む形へ**同一バージョン v2.3 のまま増補更新**するための、専門編集者・仕様設計者向け更新指示書である。[file:1]

この更新は新しい RFC を起こさず、既存の四平面アーキテクチャ、知識プリミティブ、Training Plane、Repository Pair、Expert Namespace、dual-store consistency、trust/lifecycle/applicability の規範を維持したまま、**会話入力を起点とする知識成長経路を極めて具体的に追加規定する**ことを目的とする。[file:1]

## 1. 更新方針

### 1.1 結論

RFC-0001 は v2.3 のまま更新する。新 RFC-0002 は現時点では作成しない。[file:1]

ただし更新の性質は strictly additive でなければならず、既存の ownership boundary、source-of-truth boundary、trust rule、lifecycle rule、training invariant、knowledge applicability、promotion discipline、fusion semantics を再定義してはならない。[file:1]

### 1.2 この更新で実現すべきこと

本更新で実現すべき設計上の到達点は次の通りである。[file:1]

- 人間との日常的な会話から、Darvium が長期的価値をもつ情報を取り込みうること。
- その判定は trigger phrase ではなく、**LLM によるポリシーベース分類提案**で行われること。
- ただし LLM は「提案」を行うだけであり、永続化・promotion・production 汚染防止は deterministic gate によって統制されること。[file:1]
- 時間や日を跨いで蓄積された断片的会話が、十分な evidence / trace / coherence を満たしたとき、CandidateKnowledgeDocument を経て CanonicalDocument として図書館化されること。[file:1]
- personalization は user-specific namespace として管理され、Expert Namespace / extraction / fusion の原理と整合すること。[file:1]

### 1.3 この更新で明示的に避けるべきこと

以下は本更新で定義してはならない、または safe default を優先すべき事項である。[file:1]

- trigger phrase ベースの保存判定を正規手段として据えること。
- 会話入力から production canonical knowledge へ直接書き込む経路を作ること。
- semantic overlap を理由に knowledge object を自動破壊的マージすること。v2.0-final の既定安全則は coexistence + lineage relation である。[file:1]
- Training Plane の隔離原則、P-14 の sandbox namespace 制約、promotion discipline、dual-store consistency を弱めること。[file:1]

## 2. 背景理解

### 2.1 既存 v2.3 がすでに持っている部品

更新担当者は、今回の変更が新規アーキテクチャ導入ではなく、既存 v2.3 の部品を接続しきる作業であることを理解しなければならない。[file:1]

既存 RFC-0001 v2.3 には、少なくとも次の部品がすでに存在する。[file:1]

| 既存部品 | 既存仕様上の役割 | 今回の更新での再利用方法 |
|---|---|---|
| Knowledge Persistence Plane | LadybugDB を knowledge source of truth とし、Fragment, MemoryEvent, MemoryConcept, CanonicalDocument, SkillNode, Chunk, Entity と lineage relation を保持する。[file:1] | 会話断片、概念化、図書館化の最終的保存先として用いる。[file:1] |
| Knowledge Access Primitive Plane | `memorygetrecentevents`, `memorygetconcepts`, `memorygetconcepthistory`, `memorytraceorigin`, `memorypromotetodocument`, `kbhybridsearch` などの deterministic wrapper を提供する。[file:1] | 会話断片の読出し、履歴追跡、統合候補抽出、図書館化に用いる。[file:1] |
| Training Plane | `TrainingMission`, `TrainingRunLog`, `TrainingFeedback`, `PromotionCandidate`, `TrainingTrustProfile`, `CandidateKnowledgeDocument`, `TrainingAuditLog`, `CurriculumPolicy` を formalize している。[file:1] | 会話由来知識の sandbox 隔離、レビュー、フィードバック、promotion に用いる。[file:1] |
| Knowledge-aware evaluation | `KnowledgeEvidenceBundle`, freshness/version/drift/origin trace/evidence completeness に基づいて K および Afinal を算出する。[file:1] | 断片群が図書館化に十分かどうかの gate 設計に用いる。[file:1] |
| Repository Pair / Expert Namespace | namespace 単位での抽出・融合・再構成・closure policy を持つ。[file:1] | personalization namespace と export/import 境界の設計に用いる。[file:1] |
| Dual-store consistency | knowledge mutation path に対し shared `opid` による commit intent protocol を要求する。[file:1] | 会話由来知識の保存・promotion・repair を deterministic に維持する。[file:1] |

### 2.2 現状の欠落点

v2.3 がまだ formalize していないのは、主として以下である。[file:1]

- 日常会話をどの条件で knowledge ingestion 対象とみなすか。
- LLM による会話重要度判断と、deterministic policy gate の責務境界。
- 多日・多ターンに分散した会話断片をどの条件で一つの CandidateKnowledgeDocument に束ねるか。
- user-specific namespace を personalization 向けにどう標準化するか。
- conversational memory に特化した retention / deletion / tombstone / privacy 運用規約。[file:1]

本更新は、これらの欠落点を**規範レベルで埋める**ことを目的とする。[file:1]

## 3. 更新の基本原則

更新後の本文には、少なくとも次の規範原則を明示しなければならない。[file:1]

1. 会話理解は LLM による policy-based dynamic proposal を許容・推奨する。
2. 永続化・promotion・production exposure は deterministic gate によって拘束される。[file:1]
3. すべての conversational knowledge mutation は training-first, sandbox-first でなければならない。[file:1]
4. production canonical knowledge は promotion gate を満たすまで汚染してはならない。[file:1]
5. semantic conflict の default-safe rule は destructive merge ではなく coexistence + lineage relation である。[file:1]
6. conversational ingestion は personalization に使えても、既存の trust / lifecycle / replay / audit discipline を破壊してはならない。[file:1]

## 4. 追加すべき新規セクション

以下の新規セクションを RFC-0001 v2.3 に追記すること。既存番号との整合は編集時に再採番してよいが、内容は以下の粒度で必ず入れること。[file:1]

### 4.1 Conversational Knowledge Ingestion

この章では、会話入力を knowledge 化する最初の入口を formalize する。

#### 必須記述事項

- `ConversationalEvent` 型を定義すること。
- `ConversationalIngestionPolicy` 型を定義すること。
- `ConversationalClassificationProposal` 型を定義すること。
- LLM は trigger phrase detector ではなく、policy-conditioned classifier / assessor として動作することを規範化すること。
- その proposal は deterministic policy gate を通るまで knowledge mutation を起こしてはならないことを明記すること。[file:1]

#### 追加すべき型（規範）

```rust
struct ConversationalEvent {
    event_id: String,
    session_id: String,
    user_id: String,
    actor: ConversationActor,
    utterance: String,
    timestamp: SystemTime,
    language: String,
    context_window_id: Option<String>,
    parent_event_ids: Vec<String>,
    source_channel: ConversationChannel,
}

enum ConversationActor {
    Human,
    Darvium,
    System,
}

enum ConversationChannel {
    Chat,
    VoiceTranscript,
    ImportedLog,
    EmailBridge,
    Api,
}

struct ConversationalIngestionPolicy {
    policy_id: String,
    namespace_template: String,
    allow_auto_sandbox_ingest: bool,
    require_human_review_for_promotion: bool,
    max_candidate_span_days: u32,
    min_policy_score: f32,
    min_promotion_score: f32,
    allow_raw_transcript_persistence: bool,
    pii_handling: PiiHandlingPolicy,
    retention: RetentionPolicy,
    category_rules: Vec<ConversationCategoryRule>,
    updated_at: SystemTime,
}

struct ConversationCategoryRule {
    category: ConversationalKnowledgeCategory,
    allowed_namespace_suffix: String,
    auto_ingest_to_sandbox: bool,
    eligible_for_consolidation: bool,
    eligible_for_promotion: bool,
    require_origin_trace: bool,
    minimum_distinct_events: u32,
    minimum_distinct_days: u32,
    minimum_llm_confidence: f32,
}

enum ConversationalKnowledgeCategory {
    UserProfile,
    UserPreference,
    LongLivedProjectContext,
    StableConstraint,
    TemporaryTaskContext,
    FactualClaim,
    Reflection,
    RelationshipFact,
    Noise,
    Unsafe,
    Unknown,
}

struct RetentionPolicy {
    raw_event_ttl_days: u32,
    sandbox_candidate_ttl_days: u32,
    rejected_candidate_tombstone_hours: u32,
}

enum PiiHandlingPolicy {
    Reject,
    RedactBeforePersist,
    AllowSandboxOnly,
}

struct ConversationalClassificationProposal {
    event_id: String,
    proposed_category: ConversationalKnowledgeCategory,
    policy_score: f32,
    llm_confidence: f32,
    rationale_summary: String,
    proposed_namespace: String,
    extractive_facts: Vec<String>,
    inferred_temporality: InferredTemporality,
    inferred_scope: InferredScope,
    contains_pii: bool,
    promotion_eligibility_hint: PromotionEligibilityHint,
}

enum InferredTemporality {
    Ephemeral,
    Stable,
    Historical,
    Mixed,
}

enum InferredScope {
    Personal,
    Project,
    Global,
    Ambiguous,
}

enum PromotionEligibilityHint {
    Never,
    SandboxOnly,
    ReviewRequired,
    PotentiallyPromotable,
}
```

#### 規範文として必ず入れるべき内容

- Conversational ingestion は trigger phrase 依存であってはならない。LLM は policy text と category schema と retention / privacy rule を参照して、各 event について policy-conditioned proposal を出さなければならない。[file:1]
- `proposed_category = Noise` または `Unsafe` の event は knowledge mutation に進めてはならない。[file:1]
- `contains_pii = true` のときは `PiiHandlingPolicy` に従い、reject / redact / sandbox-only のいずれかに deterministic に分岐しなければならない。[file:1]
- `allow_auto_sandbox_ingest = true` であっても、その作用域は safe sandbox scope に限定され、production canonical knowledge への即時昇格は許可されない。[file:1]

### 4.2 LLM-driven Classification and Deterministic Gate

この章では、LLM と deterministic gate の責務分離を formalize する。

#### 必須記述事項

- LLM は提案者であり、gate keeper ではないこと。
- gate は code path として deterministic / auditable / replay-safe に定義されること。
- Training Plane への流入、CandidateKnowledgeDocument 生成、PromotionCandidate 生成、CanonicalDocument 化の各段階で、どの判定が deterministic かを明示すること。[file:1]

#### 追加すべき型（規範）

```rust
struct ConversationalGateDecision {
    event_id: String,
    action: ConversationalGateAction,
    target_namespace: Option<String>,
    normalized_facts: Vec<String>,
    reason_code: String,
    requires_human_review: bool,
    created_mission: Option<String>,
}

enum ConversationalGateAction {
    Drop,
    StoreRawEventOnly,
    StoreFragmentOnly,
    CreateTrainingMission,
    CreateTrainingMissionAndFragment,
    QueueForConsolidation,
}
```

#### 決定手順（擬似コードをそのまま入れること）

```rust
fn decide_conversational_ingest(
    event: &ConversationalEvent,
    proposal: &ConversationalClassificationProposal,
    policy: &ConversationalIngestionPolicy,
) -> ConversationalGateDecision {
    if matches!(proposal.proposed_category, ConversationalKnowledgeCategory::Noise | ConversationalKnowledgeCategory::Unsafe) {
        return drop_decision(event, "CATEGORY_REJECTED");
    }

    if proposal.contains_pii {
        match policy.pii_handling {
            PiiHandlingPolicy::Reject => return drop_decision(event, "PII_REJECTED"),
            PiiHandlingPolicy::RedactBeforePersist => {
                // normalized_facts must be redacted before any persistence
            }
            PiiHandlingPolicy::AllowSandboxOnly => {}
        }
    }

    if proposal.policy_score < policy.min_policy_score {
        return drop_decision(event, "POLICY_SCORE_TOO_LOW");
    }

    let rule = lookup_category_rule(policy, proposal.proposed_category);
    if proposal.llm_confidence < rule.minimum_llm_confidence {
        return ConversationalGateDecision {
            event_id: event.event_id.clone(),
            action: ConversationalGateAction::CreateTrainingMission,
            target_namespace: Some(proposal.proposed_namespace.clone()),
            normalized_facts: proposal.extractive_facts.clone(),
            reason_code: "LOW_CONFIDENCE_REVIEW_REQUIRED".into(),
            requires_human_review: true,
            created_mission: Some(new_training_mission_id()),
        };
    }

    if rule.auto_ingest_to_sandbox && policy.allow_auto_sandbox_ingest {
        return ConversationalGateDecision {
            event_id: event.event_id.clone(),
            action: ConversationalGateAction::CreateTrainingMissionAndFragment,
            target_namespace: Some(proposal.proposed_namespace.clone()),
            normalized_facts: proposal.extractive_facts.clone(),
            reason_code: "SANDBOX_AUTO_INGEST".into(),
            requires_human_review: false,
            created_mission: Some(new_training_mission_id()),
        };
    }

    ConversationalGateDecision {
        event_id: event.event_id.clone(),
        action: ConversationalGateAction::CreateTrainingMission,
        target_namespace: Some(proposal.proposed_namespace.clone()),
        normalized_facts: proposal.extractive_facts.clone(),
        reason_code: "REVIEW_GATED_INGEST".into(),
        requires_human_review: true,
        created_mission: Some(new_training_mission_id()),
    }
}
```

#### 編集上の重要指示

本文で「LLM に非決定論を許すのは interpretation / proposal generation までである」と言い切ること。[file:1]

同時に、「state transition, persistence, consistency, promotion, canonical exposure は deterministic gate に従う」と明示すること。[file:1]

### 4.3 Conversational TrainingMission Construction

この章では、会話イベントから生成される TrainingMission の完全仕様を与える。

#### 必須記述事項

- `MissionSource::HumanSubmitted` を conversational ingest の標準 source として使うこと。[file:1]
- mission text の書式、success criteria、sandbox policy、origin event binding を固定すること。
- training mission を生成しただけでは candidate も canonical も生えないことを明記すること。[file:1]

#### 追加すべき型（規範）

```rust
struct ConversationalMissionPayload {
    mission_id: String,
    source_event_ids: Vec<String>,
    user_id: String,
    namespace: String,
    category: ConversationalKnowledgeCategory,
    normalized_facts: Vec<String>,
    mission_text: String,
    success_criteria: Vec<String>,
    review_required: bool,
    created_at: SystemTime,
}
```

#### mission_text 生成規約

次のテンプレを規範とすること。

```text
Consolidate the provided conversational evidence into a sandbox-scoped candidate knowledge object.
Preserve origin trace.
Do not infer beyond stated evidence.
Mark unresolved ambiguity explicitly.
Target namespace: {namespace}.
Target category: {category}.
```

#### success_criteria 規約

最低限、以下を満たす success criteria を自動埋めすること。

- source_event_ids がすべて origin trace に保持されること。
- normalized_facts の各要素について evidence anchoring があること。
- ambiguity がある場合は unresolved として明示されること。
- output が sandbox namespace にのみ現れること。[file:1]

### 4.4 Fragment and Candidate Creation

この章では、会話断片を Fragment / CandidateKnowledgeDocument としてどう保存するかを規範化する。

#### 規範方針

- raw transcript 全文保存は optional であり、`allow_raw_transcript_persistence` が false の場合は normalized facts と redacted summary のみ保存すること。
- sandbox namespace 下では、会話断片は `Fragment` または `MemoryEvent` として LadybugDB に保持してよいこと。
- CandidateKnowledgeDocument は training document in sandbox namespace として保持されること。[file:1]

#### 追加すべき型（規範）

```rust
struct ConversationalFragmentMeta {
    fragment_id: String,
    source_event_ids: Vec<String>,
    user_id: String,
    namespace: String,
    category: ConversationalKnowledgeCategory,
    redacted_summary: String,
    extracted_facts: Vec<String>,
    distinct_day_count: u32,
    first_seen_at: SystemTime,
    last_seen_at: SystemTime,
}
```

#### 保存規約

- `ConversationalFragmentMeta` は LadybugDB の Fragment / MemoryEvent と結合可能でなければならない。[file:1]
- `source_event_ids` は `origintraceids` に昇格可能な stable ID として保持されなければならない。[file:1]
- CandidateKnowledgeDocument 生成時には、`knowledge_id`, `source_run_id`, `namespace`, `evidence_summary`, `origin_trace_ids`, `completeness_score`, `promotion_status`, `created_at` を既存 v1.9 定義通り埋めること。[file:1]

### 4.5 Multi-turn / Multi-day Consolidation Policy

この章は本更新の中心であり、断片的な会話が「ある程度まとまった情報になったとき図書館に入る」ための厳格な条件を規定する。

#### 新規に定義すべき型

```rust
struct ConsolidationCandidateSet {
    set_id: String,
    namespace: String,
    category: ConversationalKnowledgeCategory,
    fragment_ids: Vec<String>,
    source_event_ids: Vec<String>,
    distinct_event_count: u32,
    distinct_day_count: u32,
    semantic_coherence: f32,
    trace_completeness: f32,
    temporal_stability: f32,
    contradiction_score: f32,
    created_at: SystemTime,
}

struct ConsolidationPolicy {
    min_distinct_events: u32,
    min_distinct_days: u32,
    min_semantic_coherence: f32,
    min_trace_completeness: f32,
    min_temporal_stability: f32,
    max_contradiction_score: f32,
    require_origin_trace: bool,
    allow_auto_candidate_creation: bool,
    allow_auto_promotion: bool,
}
```

#### 規範閾値（この値を明記すること）

初期 normative default を次とすること。

- `min_distinct_events = 3`
- `min_distinct_days = 2`
- `min_semantic_coherence = 0.70`
- `min_trace_completeness = 0.80`
- `min_temporal_stability = 0.65`
- `max_contradiction_score = 0.20`
- `require_origin_trace = true`
- `allow_auto_candidate_creation = true`
- `allow_auto_promotion = false`

#### semantic_coherence の定義

semantic_coherence は、断片群が同一 long-lived fact / preference / constraint / project-context に属する度合いを 0.0〜1.0 で表した score と定義すること。実装は LLM 判定を用いてよいが、その score の採否は deterministic threshold で行うこと。[file:1]

#### contradiction_score の safe rule

contradiction_score が閾値を超えた集合は自動 canonicalization してはならない。既定動作は、

- CandidateKnowledgeDocument を separate candidate として並存させる、または
- `SUPERSEDES`/`CONSOLIDATES` 候補として human review queue に送る

のいずれかとし、destructive merge を行ってはならない。[file:1]

#### 図書館化の段階規約

以下の 4 段階を必ず図で示すこと。

1. ConversationalEvent
2. Fragment / MemoryEvent
3. CandidateKnowledgeDocument
4. CanonicalDocument

また、それぞれの間に張る lineage relation を固定すること。

- Event/Fragment -> CandidateKnowledgeDocument : `DERIVEDFROM`
- fragment bundle -> candidate document : `CONSOLIDATES`
- candidate document -> canonical document : `MATERIALIZEDAS`
- replaced canonical / preference update : `SUPERSEDES`

### 4.6 Personalization Namespace Convention

この章では、会話を通じて学んだ個人知識の namespace 規約を定める。

#### 規範命名規約

以下を推奨ではなく標準形式として明記すること。

- `user/{user_id}/profile`
- `user/{user_id}/preferences`
- `user/{user_id}/projects/{project_id}`
- `user/{user_id}/history`
- `user/{user_id}/scratch`

#### 用途規約

| Namespace | 用途 | Promotion 許可 |
|---|---|---|
| `profile` | 長期的な本人属性、恒常的自己記述 | 条件付きで可 |
| `preferences` | 安定した嗜好、好み、コミュニケーション傾向 | 条件付きで可 |
| `projects/{project_id}` | 長寿命の案件文脈、制約、方針 | 条件付きで可 |
| `history` | 過去事実の記録、履歴参照 | 通常は sandbox / review required |
| `scratch` | 一時メモ、短期作業文脈 | promotion 不可 |

この表をそのまま本文に入れること。[file:1]

#### Expert Namespace との整合

- user namespace は v2.0 の Expert Namespace として extract / fuse 可能でなければならない。[file:1]
- ただし `scratch` と tombstoned artifact は default で required dependency closure に含めてはならない。[file:1]

### 4.7 Promotion to Canonical Document

この章では、図書館入りの最終手順を formalize する。

#### 規範方針

- conversational origin の knowledge は、CandidateKnowledgeDocument を経由せずに CanonicalDocument 化してはならない。[file:1]
- `memorypromotetodocument` は唯一の mutation primitive として、promotion gate 通過後にのみ使えることを再確認すること。[file:1]
- dual-store consistency protocol はそのまま適用されること。[file:1]

#### PromotionGate 追加型

```rust
struct ConversationalPromotionGate {
    candidate_id: String,
    namespace: String,
    category: ConversationalKnowledgeCategory,
    llm_policy_score: f32,
    completeness_score: f32,
    trace_completeness: f32,
    contradiction_score: f32,
    distinct_day_count: u32,
    training_good_ratio: f32,
    sandbox_success_rate: f32,
    requires_human_review: bool,
}
```

#### 規範条件

Conversational origin knowledge が CanonicalDocument へ昇格できるのは、少なくとも以下を満たす場合のみとすること。

- `promotion_status = Approved`
- `completeness_score >= 0.80`
- `trace_completeness >= 0.80`
- `contradiction_score <= 0.20`
- `distinct_day_count >= 2`
- `training_good_ratio >= TRAININGPROMOTIONMINGOODRATIO`
- `sandbox_success_rate >= TRAININGPROMOTIONMINSUCCESSRATE`
- `requires_human_review = false` または human approval が記録済み
- `opid` を共有する dual-store commit intent が生成されること

既存 training constants は calibration candidate だが、会話由来 promotion にもそのまま適用されることを明記すること。[file:1]

### 4.8 Privacy, Retention, Tombstone, and Repair

この章では conversational memory に固有の運用規約を formalize する。

#### 必須規定

- raw conversational event は TTL に従って期限切れしうること。
- CandidateKnowledgeDocument が Rejected の場合、既存の tombstone grace を継承すること。[file:1]
- user request deletion を受けた artifact は、少なくとも namespace-local tombstone と audit log を残し、通常の retrieval path から除外されなければならない。[file:1]
- dual-store inconsistency が起きた conversational artifact は `NeedsRepair` または `Quarantined` に入り、normal REUSE/PATCH/COMPOSE path に出してはならない。[file:1]

## 5. 既存章への差分指示

新章追加だけでなく、既存章にも最低限の差分を入れること。[file:1]

### 5.1 5.5 / 5.6 の architecture overview

Knowledge Ecosystem Integration と Training Plane Integration の overview に、次の一文群を追加すること。

- Conversational ingestion is an optional policy-governed extension layered over the existing Knowledge Access Primitive Plane and Training Plane.[file:1]
- It SHALL NOT redefine ownership of canonical knowledge, WorkflowGraph, TrustProfile, Lifecycle state, SearchTrace, or training-production separation.[file:1]

### 5.2 12A Knowledge Primitive Registry

primitive set 自体は増やさなくてよいが、`memorygetrecentevents`, `memorygetconcepts`, `memorygetconcepthistory`, `memorytraceorigin`, `memorypromotetodocument` が conversational memory path の標準 primitive であることを explanatory note として追記すること。[file:1]

### 5.3 16A Training Plane

safe sandbox scope optional auto-approval の記述に、会話由来 ingestion が対象になりうることを追記すること。ただし対象は sandbox 限定であり、promotion auto-approval は禁止とすること。[file:1]

### 5.4 25 / 26 Appendix D

SQLite / LadybugDB appendix に、会話メタデータの推奨保存構造を追記すること。実装自由度は残してよいが、少なくとも次のテーブル/型を example ではなく recommended schema として掲載すること。

```rust
struct ConversationalEventLog {
    event_id: String,
    session_id: String,
    user_id: String,
    actor: String,
    timestamp: SystemTime,
    channel: String,
    redacted_text: String,
    raw_text_ref: Option<String>,
    policy_id: String,
}

struct ConversationalProposalLog {
    event_id: String,
    proposed_category: String,
    policy_score: f32,
    llm_confidence: f32,
    contains_pii: bool,
    proposed_namespace: String,
    created_at: SystemTime,
}

struct ConsolidationRunLog {
    run_id: String,
    namespace: String,
    candidate_set_id: String,
    candidate_id: Option<String>,
    semantic_coherence: f32,
    trace_completeness: f32,
    contradiction_score: f32,
    decision: String,
    created_at: SystemTime,
}
```

## 6. 本文に入れるべき規範文（そのまま使ってよい文案）

以下の趣旨を RFC 本文に規範文として入れること。[file:1]

### 6.1 trigger phrase 排除

> Conversational ingestion MUST NOT rely on trigger phrases as the primary admission mechanism. Implementations SHALL evaluate conversational events through a policy-conditioned classification proposal process in which an LLM or equivalent semantic reasoner assesses long-term reuse value, category, scope, temporality, privacy risk, and promotion eligibility under an explicit ingestion policy.[file:1]

### 6.2 LLM と gate の責務分離

> The classification proposal MAY be nondeterministic, but persistence, state transition, namespace assignment, promotion eligibility, and canonical exposure SHALL be governed by deterministic gates, auditable state transitions, and existing training-production separation invariants.[file:1]

### 6.3 sandbox-first

> Any conversationally derived knowledge mutation SHALL be sandbox-first. No conversational event, fragment, or candidate knowledge object may directly mutate production canonical knowledge without passing the existing promotion discipline, trust review, origin-trace requirements, and dual-store consistency protocol.[file:1]

### 6.4 図書館化条件

> Multi-turn or multi-day conversational fragments MAY be consolidated into a CandidateKnowledgeDocument only when the candidate set satisfies policy-declared thresholds for semantic coherence, trace completeness, temporal stability, and contradiction tolerance. Promotion to CanonicalDocument SHALL remain separately gated.[file:1]

## 7. 実装迷いを残さないための追加要求

更新担当者には、単に概念説明を書くのではなく、**実装移送可能な仕様**として以下を必須化するよう指示すること。[file:1]

- 各新型について Rust struct / enum を載せること。
- ingestion, consolidation, promotion の各判断について擬似コードを載せること。
- default threshold を数値で書くこと。
- lineages の relation 名を固定すること。
- namespace naming convention を固定すること。
- “MAY” に逃がしすぎないこと。特に trigger phrase 不採用、sandbox-first、promotion direct-write 禁止、coexistence default は MUST / MUST NOT で書くこと。[file:1]

## 8. 追加すべき図表

次の図表を本文に入れること。[file:1]

1. 四平面アーキテクチャに Conversational Ingestion Layer を重ねた論理図。[file:1]
2. `ConversationalEvent -> Fragment -> CandidateKnowledgeDocument -> CanonicalDocument` の状態遷移図。[file:1]
3. LLM proposal と deterministic gate の責務分界図。[file:1]
4. personalization namespace の一覧表。[file:1]
5. consolidation gate と promotion gate の閾値表。[file:1]
6. contradiction handling の decision table（coexistence / review / supersedes）。[file:1]

## 9. 編集判断のルール

更新担当者は、以下のルールに従って編集判断を行うこと。[file:1]

- 既存の normative text と矛盾する場合は、新章の wording を修正して整合させる。既存 invariants を弱めてはならない。[file:1]
- conversational memory は Training Plane の extension として書く。独立 plane としては書かない。[file:1]
- personalization は Expert Namespace の特殊ケースとして書く。別アーキテクチャとして書かない。[file:1]
- semantic reasoning は LLM を使ってよいが、 audit / replay / consistency の最終責任は RFC-0001 既存規範に従うと明記する。[file:1]

## 10. 更新完了条件

RFC 更新作業は、次の条件を満たしたときのみ完了とみなすこと。[file:1]

- 読者が conversational memory path を end-to-end で追えること。
- 「どのデータをどの型で持つか」「どの条件で保存するか」「どの条件で CandidateKnowledgeDocument を作るか」「どの条件で CanonicalDocument に上げるか」が、数値付きで明記されていること。
- 実装者が trigger phrase を持ち込まなくても実装できること。
- LLM proposal と deterministic gate の責務が混同されていないこと。
- dual-store consistency、training isolation、promotion discipline、coexistence default が本文中で再確認されていること。[file:1]

## 11. 最終的な推奨編集メッセージ

更新担当者への最終指示は、次の一文に要約される。[file:1]

Darvium RFC-0001 v2.3 を、日常会話から長期知識へ至る成長経路を formalize した仕様へ増補せよ。ただし、その入口判定は trigger phrase ではなく LLM による policy-conditioned proposal とし、保存・昇格・公開は既存 v2.3 invariants に拘束された deterministic gate で統制せよ。会話断片は sandbox namespace に隔離され、複数日・複数ターンにわたる evidence と origin trace が十分なときのみ CandidateKnowledgeDocument として束ねられ、さらに promotion gate を満たした場合に限って CanonicalDocument として図書館化される、という end-to-end path を数値閾値・型定義・擬似コード付きで追加規定せよ。[file:1]
