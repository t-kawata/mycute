// Darvium 公開型定義
//
// 本ファイルは RFC の型定義から実装に必要な基本型を集約する。
// 詳細な実装はフェーズ/チケットごとに追加される。

// === ID 型 ===
pub type NodeId = usize;
pub type GraphVersion = u64;
pub type PairId = String;
pub type ActorId = String;

// === ワークフローグラフ関連 ===
use petgraph::graph::DiGraph;

/// ワークフローグラフのノード重み。
#[derive(Debug, Clone)]
pub struct WorkflowNode;

/// ワークフローグラフのエッジ重み。
#[derive(Debug, Clone)]
pub struct EdgeMeta;

/// ワークフローグラフ（DAG）。
pub type WorkflowGraph = DiGraph<WorkflowNode, EdgeMeta>;

/// 検索ワークフローグラフ。
pub type SearchWorkflowGraph = DiGraph<WorkflowNode, EdgeMeta>;

// === クエリ表現 列挙型 ===
/// クエリ種別 (RFC §9.5 Knowledge-Aware Extension)
#[derive(Debug, Clone, PartialEq)]
pub enum QueryType {
    Episodic,
    Canonical,
    Hybrid,
}

/// 鮮度要件 (RFC §9.5)
#[derive(Debug, Clone, PartialEq)]
pub enum FreshnessRequirement {
    Recent,
    Stable,
    Historical,
    Mixed,
}

/// 証拠厳密性 (RFC §9.5)
#[derive(Debug, Clone, PartialEq)]
pub enum EvidenceStrictness {
    Light,
    Strict,
    AuditGrade,
}

/// ドリフト感度 (RFC §9.5)
#[derive(Debug, Clone, PartialEq)]
pub enum DriftSensitivity {
    Ignore,
    PreferLatest,
    ShowHistory,
}

// === 検索関連 ===
/// クエリ表現。
///
/// Mission 入力から導出される検索クエリの完全表現。
/// RFC §9.4, §9.5 に基づく。
#[derive(Debug, Clone, PartialEq)]
pub struct QueryRepresentation {
    pub mission_text: String,
    pub task_embedding: Vec<f32>,
    pub query_design_text: String,
    pub query_design_embedding: Vec<f32>,
    pub design_template_version: String,
    pub query_type: QueryType,
    pub freshness_requirement: FreshnessRequirement,
    pub evidence_strictness: EvidenceStrictness,
    pub origin_trace_required: bool,
    pub drift_sensitivity: DriftSensitivity,
}

impl QueryRepresentation {
    /// RFC §9.5 で規定されたデフォルト値で初期化した QueryRepresentation を生成する。
    pub fn new(
        mission_text: String,
        task_embedding: Vec<f32>,
        query_design_text: String,
        query_design_embedding: Vec<f32>,
    ) -> Self {
        Self {
            mission_text,
            task_embedding,
            query_design_text,
            query_design_embedding,
            design_template_version: "v2.0-final".to_string(),
            query_type: QueryType::Hybrid,
            freshness_requirement: FreshnessRequirement::Mixed,
            evidence_strictness: EvidenceStrictness::Light,
            origin_trace_required: false,
            drift_sensitivity: DriftSensitivity::PreferLatest,
        }
    }
}

impl Default for QueryRepresentation {
    fn default() -> Self {
        Self::new(String::new(), Vec::new(), String::new(), Vec::new())
    }
}

/// 検索ポリシー (RFC §13.4)。
#[derive(Debug, Clone, PartialEq)]
pub struct RetrievalPolicy {
    pub top_k_sem: u32,
    pub top_k_struct: u32,
    pub min_trust: f32,
    pub allow_compose: bool,
    pub allow_new: bool,
}

impl Default for RetrievalPolicy {
    fn default() -> Self {
        Self {
            top_k_sem: 5,
            top_k_struct: 5,
            min_trust: 0.0,
            allow_compose: false,
            allow_new: false,
        }
    }
}

/// ランク付けされた検索候補。
#[derive(Debug, Clone, PartialEq)]
pub struct RankedCandidate {
    pub workflow_id: String,
    pub semantic_score: f64,
    pub structural_score: f64,
    pub blended_score: f64,
    pub trust_score: f64,
    pub provenance: Vec<String>,
    pub metadata: serde_json::Value,
}

/// 検索結果の候補集合 (RFC §13.4)。
#[derive(Debug, Clone, PartialEq)]
pub struct CandidateSet {
    pub candidates: Vec<RankedCandidate>,
    pub retrieval_calls_used: u32,
}

impl CandidateSet {
    /// 空の候補集合を生成する。
    pub fn empty() -> Self {
        Self {
            candidates: Vec::new(),
            retrieval_calls_used: 0,
        }
    }
}

/// GMR 検索の抽象インターフェース (RFC §13.4)。
///
/// SearchWorkflow は Stage 0–4 の GMR をこのトレイトを通じて呼び出す。
/// このトレイトは pure retrieval contract であり、最終意思決定権を持たない。
pub trait RetrievalPrimitive {
    /// クエリ表現に基づき、ワークフロー候補を検索する。
    fn search_workflows(
        &self,
        query: &QueryRepresentation,
        policy: &RetrievalPolicy,
    ) -> Result<CandidateSet, crate::error::DarviumError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::DarviumError;

    /// テスト1: RetrievalPrimitive を実装するダミー構造体がコンパイルできる。
    struct DummyRetrievalPrimitive;

    impl RetrievalPrimitive for DummyRetrievalPrimitive {
        fn search_workflows(
            &self,
            _query: &QueryRepresentation,
            _policy: &RetrievalPolicy,
        ) -> Result<CandidateSet, DarviumError> {
            Ok(CandidateSet::empty())
        }
    }

    /// テスト2: ダミー構造体の search_workflows が正しい型シグネチャで呼び出せる。
    #[test]
    fn dummy_retrieval_primitive_invocation() {
        let primitive = DummyRetrievalPrimitive;
        let query = QueryRepresentation::default();
        let policy = RetrievalPolicy::default();
        let result = primitive.search_workflows(&query, &policy);
        assert!(result.is_ok());
        let candidates = result.expect("DummyRetrievalPrimitive should always return Ok");
        assert!(candidates.candidates.is_empty());
        assert_eq!(candidates.retrieval_calls_used, 0);
    }

    /// テスト3: QueryRepresentation の全フィールドにアクセスできる。
    #[test]
    fn query_representation_field_access() {
        let query = QueryRepresentation::new(
            "mission".to_string(),
            vec![0.1, 0.2, 0.3],
            "design".to_string(),
            vec![0.4, 0.5, 0.6],
        );
        assert_eq!(query.mission_text, "mission");
        assert_eq!(query.task_embedding.len(), 3);
        assert_eq!(query.query_design_text, "design");
        assert_eq!(query.query_design_embedding.len(), 3);
        assert_eq!(query.design_template_version, "v2.0-final");
        assert_eq!(query.query_type, QueryType::Hybrid);
        assert_eq!(query.freshness_requirement, FreshnessRequirement::Mixed);
        assert_eq!(query.evidence_strictness, EvidenceStrictness::Light);
        assert!(!query.origin_trace_required);
        assert_eq!(query.drift_sensitivity, DriftSensitivity::PreferLatest);
    }

    /// テスト4: QueryRepresentation::default() が RFC §9.5 のデフォルト値を満たす。
    #[test]
    fn query_representation_default_values() {
        let query = QueryRepresentation::default();
        assert!(query.mission_text.is_empty());
        assert!(query.task_embedding.is_empty());
        assert!(query.query_design_text.is_empty());
        assert!(query.query_design_embedding.is_empty());
        assert_eq!(query.design_template_version, "v2.0-final");
        assert_eq!(query.query_type, QueryType::Hybrid);
        assert_eq!(query.freshness_requirement, FreshnessRequirement::Mixed);
        assert_eq!(query.evidence_strictness, EvidenceStrictness::Light);
        assert!(!query.origin_trace_required);
        assert_eq!(query.drift_sensitivity, DriftSensitivity::PreferLatest);
    }

    /// テスト5: RetrievalPolicy のデフォルト値が正しい。
    #[test]
    fn retrieval_policy_default_values() {
        let policy = RetrievalPolicy::default();
        assert_eq!(policy.top_k_sem, 5);
        assert_eq!(policy.top_k_struct, 5);
        assert_eq!(policy.min_trust, 0.0);
        assert!(!policy.allow_compose);
        assert!(!policy.allow_new);
    }

    /// テスト6: 全列挙型の全バリアントが網羅的マッチング可能。
    #[test]
    fn enum_variants_exhaustive_match() {
        // QueryType
        for variant in &[QueryType::Episodic, QueryType::Canonical, QueryType::Hybrid] {
            match variant {
                QueryType::Episodic => {}
                QueryType::Canonical => {}
                QueryType::Hybrid => {}
            }
        }
        // FreshnessRequirement
        for variant in &[
            FreshnessRequirement::Recent,
            FreshnessRequirement::Stable,
            FreshnessRequirement::Historical,
            FreshnessRequirement::Mixed,
        ] {
            match variant {
                FreshnessRequirement::Recent => {}
                FreshnessRequirement::Stable => {}
                FreshnessRequirement::Historical => {}
                FreshnessRequirement::Mixed => {}
            }
        }
        // EvidenceStrictness
        for variant in &[
            EvidenceStrictness::Light,
            EvidenceStrictness::Strict,
            EvidenceStrictness::AuditGrade,
        ] {
            match variant {
                EvidenceStrictness::Light => {}
                EvidenceStrictness::Strict => {}
                EvidenceStrictness::AuditGrade => {}
            }
        }
        // DriftSensitivity
        for variant in &[
            DriftSensitivity::Ignore,
            DriftSensitivity::PreferLatest,
            DriftSensitivity::ShowHistory,
        ] {
            match variant {
                DriftSensitivity::Ignore => {}
                DriftSensitivity::PreferLatest => {}
                DriftSensitivity::ShowHistory => {}
            }
        }
    }

    /// テスト7: CandidateSet が空候補・複数候補の両方で構築可能。
    #[test]
    fn candidate_set_construction() {
        // 空の候補集合
        let empty = CandidateSet::empty();
        assert!(empty.candidates.is_empty());
        assert_eq!(empty.retrieval_calls_used, 0);

        // 複数候補
        let candidates = vec![
            RankedCandidate {
                workflow_id: "wf-1".to_string(),
                semantic_score: 0.9,
                structural_score: 0.8,
                blended_score: 0.85,
                trust_score: 0.7,
                provenance: vec!["source-a".to_string()],
                metadata: serde_json::json!({}),
            },
            RankedCandidate {
                workflow_id: "wf-2".to_string(),
                semantic_score: 0.7,
                structural_score: 0.6,
                blended_score: 0.65,
                trust_score: 0.5,
                provenance: vec!["source-b".to_string()],
                metadata: serde_json::json!({"note": "fallback"}),
            },
        ];
        let set = CandidateSet {
            candidates,
            retrieval_calls_used: 1,
        };
        assert_eq!(set.candidates.len(), 2);
        assert_eq!(set.retrieval_calls_used, 1);
    }

    /// テスト8: RankedCandidate が全フィールドを指定して構築可能。
    #[test]
    fn ranked_candidate_construction() {
        let candidate = RankedCandidate {
            workflow_id: "wf-test".to_string(),
            semantic_score: 0.95,
            structural_score: 0.80,
            blended_score: 0.90,
            trust_score: 0.85,
            provenance: vec!["gemini".to_string(), "struct-v1".to_string()],
            metadata: serde_json::json!({"embedding_version": "v2"}),
        };
        assert_eq!(candidate.workflow_id, "wf-test");
        assert_eq!(candidate.semantic_score, 0.95);
        assert_eq!(candidate.structural_score, 0.80);
        assert_eq!(candidate.blended_score, 0.90);
        assert_eq!(candidate.trust_score, 0.85);
        assert_eq!(candidate.provenance.len(), 2);
        assert_eq!(
            candidate.metadata.get("embedding_version").and_then(|v| v.as_str()),
            Some("v2")
        );
    }

    /// テスト9: RetrievalPrimitive トレイトがオブジェクト安全であり、
    /// Box<dyn RetrievalPrimitive> として使用できることを確認する。
    #[test]
    fn trait_object_safety() {
        let primitive: Box<dyn RetrievalPrimitive> = Box::new(DummyRetrievalPrimitive);
        let query = QueryRepresentation::default();
        let policy = RetrievalPolicy::default();
        let result = primitive.search_workflows(&query, &policy);
        assert!(result.is_ok());
    }
}

#[derive(Debug, Clone)]
pub struct SearchBudget {
    pub max_prompt_tokens: u64,
    pub prompt_tokens_used: u64,
    pub max_depth: usize,
    pub current_depth: usize,
}

#[derive(Debug, Clone)]
pub struct RecursionGuard {
    pub max_depth: usize,
    pub current_depth: usize,
}

// === 信頼関連 ===
#[derive(Debug, Clone)]
pub struct TrustProfile;

#[derive(Debug, Clone)]
pub struct Provenance;

// === パッチ関連 ===
#[derive(Debug, Clone)]
pub struct WorkflowPatch;

// === 系列関連 ===
#[derive(Debug, Clone)]
pub struct SearchTrace;

#[derive(Debug, Clone)]
pub struct TrustAuditLog;

#[derive(Debug, Clone)]
pub struct PatchHistory;

// === Knowledge Ecosystem ===
#[derive(Debug, Clone)]
pub struct KnowledgeApplicability;

// === Training Plane ===
#[derive(Debug, Clone)]
pub struct TrainingMission;

#[derive(Debug, Clone)]
pub struct TrainingFeedback;

// === Fusion ===
#[derive(Debug, Clone)]
pub struct RepositoryPair;

#[derive(Debug, Clone)]
pub struct FusionPlan;

// === Dual-Store 補助データ型 ===

/// グラフ識別子。
/// InMemoryGraphStore 内で各ワークフローグラフを一意に識別する。
pub type GraphId = String;

/// 知識オブジェクト (Knowledge Object)。
/// LadybugDB に格納される知識の最小単位。
#[derive(Debug, Clone, PartialEq)]
pub struct KnowledgeObject {
    pub id: String,
    pub object_type: String,
    pub data: String,
}

/// リレーションレコード。
/// 知識オブジェクト間の関連を表現する。
#[derive(Debug, Clone, PartialEq)]
pub struct RelationRecord {
    pub object_id: String,
    pub related_object_id: String,
    pub relation_type: String,
}

/// 原産地トレース (Origin Trace)。
/// 知識オブジェクトの生成経路を記録する。
#[derive(Debug, Clone, PartialEq)]
pub struct OriginTrace {
    pub object_id: String,
    pub source: String,
    pub timestamp_ms: u64,
}

/// 訓練メタデータ (Training Metadata)。
#[derive(Debug, Clone, PartialEq)]
pub struct TrainingMetadata {
    pub mission_id: String,
    pub status: String,
}

/// 融合メタデータ (Fusion Metadata)。
#[derive(Debug, Clone, PartialEq)]
pub struct FusionMetadata {
    pub pair_id: String,
    pub status: String,
}
