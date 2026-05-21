// Darvium エラー型
//
// 本ファイルは RFC Annex B のエラー型全体を網羅する。
// 全てのエラー型は thiserror で定義する。

use thiserror::Error;

/// Darvium 全体のエラー型。
#[derive(Error, Debug, Clone, PartialEq)]
pub enum DarviumError {
    // === Layer 2: Workflow IR ===
    #[error("Graph validation error: {0}")]
    GraphValidation(String),

    #[error("Cycle detected: {0}")]
    CycleDetected(String),

    #[error("Variable scope violation: {0}")]
    VariableScopeViolation(String),

    // === Layer 2 → 1 コンパイル ===
    #[error("Compilation error: {0}")]
    Compilation(String),

    // === Layer 3a: GMR Retrieval ===
    #[error("Retrieval error: {0}")]
    Retrieval(String),

    #[error("Retrieval timeout")]
    RetrievalTimeout,

    #[error("Embedding version mismatch: {0}")]
    EmbeddingVersionMismatch(String),

    // === Layer 3b: SearchWorkflow ===
    #[error("Search validation error: {0}")]
    SearchValidation(String),

    #[error("Terminal state violation")]
    TerminalStateViolation,

    #[error("Search budget exceeded")]
    SearchBudgetExceeded,

    #[error("Search recursion exceeded")]
    SearchRecursionExceeded,

    #[error("Search policy oscillation detected")]
    SearchPolicyOscillation,

    // === Layer 2.5: Patch ===
    #[error("Patch conflict: {0}")]
    PatchConflict(String),

    #[error("Patch cycle created")]
    PatchCycleCreated,

    #[error("Graph version conflict: expected {expected}, actual {actual}")]
    GraphVersionConflict { expected: u64, actual: u64 },

    // === Applicability ===
    #[error("Applicability gate {gate} rejected: {reason}")]
    ApplicabilityRejected { gate: String, reason: String },

    // === Dual-store ===
    #[error("Dual-store commit failed: {0}")]
    DualStoreCommit(String),

    #[error("Dual-store inconsistency: {0}")]
    DualStoreInconsistency(String),

    // === Training Plane ===
    #[error("Training error: {0}")]
    Training(String),

    #[error("Promotion gate rejected: {0}")]
    PromotionRejected(String),

    // === Fusion ===
    #[error("Fusion error: {0}")]
    Fusion(String),

    #[error("Fusion admissibility rejected: {0}")]
    FusionAdmissibilityRejected(String),

    #[error("Fusion identity remap conflict: {0}")]
    FusionIdentityConflict(String),

    // === Internal / Unexpected ===
    #[error("Internal error: {0}")]
    Internal(String),
}
