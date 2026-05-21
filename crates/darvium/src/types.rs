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

// === 検索関連 ===
#[derive(Debug, Clone)]
pub struct QueryRepresentation;

#[derive(Debug, Clone)]
pub struct RankedCandidate;

#[derive(Debug, Clone)]
pub struct CandidateSet;

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
