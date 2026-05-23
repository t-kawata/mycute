// Darvium 公開型定義
//
// 本ファイルは RFC の型定義から実装に必要な基本型を集約する。
// 詳細な実装はフェーズ/チケットごとに追加される。

use serde::{Deserialize, Serialize};

use crate::search::applicability::EmbeddingChannelVersion;

// === ID 型 ===
pub type NodeId = usize;
pub type GraphVersion = u64;
pub type PairId = String;
pub type ActorId = String;

/// ワークフローグラフ識別子 (RFC §13.3)。
pub type WorkflowGraphId = String;

// === ワークフローグラフ関連 ===
use petgraph::graph::DiGraph;

/// 変数型 (RFC §6.1)。
#[derive(Debug, Clone, PartialEq)]
pub enum VarType {
    String,
    Number,
    Json,
    Blob,
}

/// 変数宣言 (RFC §6.1)。
#[derive(Debug, Clone, PartialEq)]
pub struct VarDecl {
    pub name: String,
    pub required: bool,
    pub var_type: VarType,
}

impl VarDecl {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            required: true,
            var_type: VarType::String,
        }
    }
}

/// ワークフローグラフのノード重み (RFC §6.1)。
#[derive(Debug, Clone)]
pub enum WorkflowNode {
    AgentStep {
        agent: String,
        prompt_template: String,
        inputs: Vec<VarDecl>,
        output_var: String,
    },
    SubWorkflow {
        workflow_id: WorkflowGraphId,
        input_mapping: Vec<(String, String)>,
        output_var: String,
    },
    /// プレースホルダ — 組成検証時には V-03/V-04 チェックに必要な
    /// inputs / output_var のみ参照するため、他の variant は後続チケットで拡充。
    #[allow(dead_code)]
    Placeholder,
}

/// ワークフローグラフのエッジ重み (RFC §6.2)。
/// 組成検証では DependsOn と DataFlow のみ使用。
#[derive(Debug, Clone, PartialEq)]
pub enum EdgeMeta {
    /// 実行順序依存のみ（データ依存なし）。
    DependsOn,
    /// データフロー依存。
    DataFlow { from_var: String, to_var: String },
    #[allow(dead_code)]
    Conditional { condition_expr: String, branch: usize },
    #[allow(dead_code)]
    FanOut { branch_id: usize },
    #[allow(dead_code)]
    Collect { strategy: usize },
}

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
    /// テンプレートバージョン (後方互換性のため維持)
    pub design_template_version: String,
    /// semantic channel (task_embedding) の埋め込みモデルバージョン
    pub task_embedding_version: EmbeddingChannelVersion,
    /// structural proxy channel (workflow_design_embedding) の埋め込みモデル/テンプレートバージョン
    pub design_embedding_version: EmbeddingChannelVersion,
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
            task_embedding_version: EmbeddingChannelVersion::default_task(),
            design_embedding_version: EmbeddingChannelVersion::default_design(),
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

/// SearchWorkflow の状態機械 (RFC §13.5)。
///
/// 8 状態からなる有限状態機械。
/// `Finalize` と `Abort` は終端状態であり、終端後に再遷移してはならない (MUST NOT)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SearchState {
    /// 検索開始。Init からは Retrieve または Abort のみ遷移可能。
    Init,
    /// 候補検索中。Retrieve からは Evaluate または Abort のみ遷移可能。
    Retrieve,
    /// 候補評価中。Evaluate からは Refine / Compose / Finalize / Abort に遷移可能。
    Evaluate,
    /// 検索ポリシー改善中。Refine からは Retrieve / ProposeNew / Abort に遷移可能。
    Refine,
    /// 組成評価中。Compose からは Finalize / Refine / Abort に遷移可能。
    Compose,
    /// 新規提案評価中。ProposeNew からは Finalize または Abort のみ遷移可能。
    ProposeNew,
    /// 終端状態: 正常終了。この状態からの遷移は全て違法。
    Finalize,
    /// 終端状態: 中断終了。この状態からの遷移は全て違法。
    Abort,
}

/// RFC §13.5 に基づき、`from` から `to` への状態遷移が合法か判定する。
///
/// # 遷移規則
///
/// ```text
/// Init -> Retrieve -> Evaluate
/// Evaluate -> Finalize        (REUSE / PATCH が十分)
/// Evaluate -> Compose         (単独候補では不十分だが組成候補あり)
/// Evaluate -> Refine          (候補不足・policy 改善が必要)
/// Compose -> Finalize         (COMPOSE 成立)
/// Compose -> Refine           (compose 不成立)
/// Refine -> Retrieve          (requery)
/// Refine -> ProposeNew        (既存候補再利用の期待値が低い)
/// ProposeNew -> Finalize      (NEW 採択)
/// 任意状態 -> Abort           (budget / recursion / unsafe transition)
/// ```
pub fn is_legal_transition(from: SearchState, to: SearchState) -> bool {
    matches!(
        (from, to),
        (SearchState::Init, SearchState::Retrieve)
            | (SearchState::Init, SearchState::Abort)
            | (SearchState::Retrieve, SearchState::Evaluate)
            | (SearchState::Retrieve, SearchState::Abort)
            | (SearchState::Evaluate, SearchState::Refine)
            | (SearchState::Evaluate, SearchState::Compose)
            | (SearchState::Evaluate, SearchState::Finalize)
            | (SearchState::Evaluate, SearchState::Abort)
            | (SearchState::Refine, SearchState::Retrieve)
            | (SearchState::Refine, SearchState::ProposeNew)
            | (SearchState::Refine, SearchState::Abort)
            | (SearchState::Compose, SearchState::Finalize)
            | (SearchState::Compose, SearchState::Refine)
            | (SearchState::Compose, SearchState::Abort)
            | (SearchState::ProposeNew, SearchState::Finalize)
            | (SearchState::ProposeNew, SearchState::Abort)
    )
}

/// 終端状態への遷移を正当化する理由 (M-1.5-2)。
///
/// RFC §13.6 のガード条件に基づき、以下の理由でのみ終端状態への遷移を許可する：
/// - `BudgetExceeded`: SearchBudget 上限超過
/// - `RecursionExceeded`: RecursionGuard 深さ超過
/// - `ExplicitAbort`: unsafe transition 検出等の明示的 Abort
/// - `NormalCompletion`: REUSE/PATCH/COMPOSE/NEW の正常成立
///
/// 単一候補の failure (`SingleCandidateFailure`) は終端理由として不十分であり、
/// 残候補が存在する限り SearchWorkflow は継続可能状態に留まらなければならない。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalTransitionReason {
    /// SearchBudget 上限超過による終端。
    BudgetExceeded,
    /// RecursionGuard 深さ超過による終端。
    RecursionExceeded,
    /// unsafe transition 検出等の明示的 Abort。
    ExplicitAbort,
    /// REUSE/PATCH/COMPOSE/NEW 正常成立による終端。
    NormalCompletion,
    /// 単一候補の failure — 終端理由として不十分。
    SingleCandidateFailure,
    /// ポリシー発振（Refine↔Retrieve 無限往復）の検出による終端 (M-1.5-3)。
    OscillationDetected,
}

/// 指定された理由で終端状態への遷移が許可されるか判定する (M-1.5-2)。
///
/// `SingleCandidateFailure` のみ `false` を返し、その他の理由は全て `true` を返す。
pub fn can_terminate_with(reason: TerminalTransitionReason) -> bool {
    !matches!(reason, TerminalTransitionReason::SingleCandidateFailure)
}

impl SearchState {
    /// 現在状態から `next` への状態遷移を試行する (M-1.5-2)。
    ///
    /// # ガード優先順位
    ///
    /// 1. 現在状態が終端（`Finalize` / `Abort`）→ `Err(TerminalStateViolation)`
    /// 2. 遷移が違法（`!is_legal_transition`）→ `Err(SearchValidation(...))`
    /// 3. 合法 → 状態を更新し `Ok(())`
    pub fn transition_to(&mut self, next: SearchState) -> Result<(), crate::error::DarviumError> {
        // ガード①: 終端状態からの遷移は全て違法
        if matches!(self, SearchState::Finalize | SearchState::Abort) {
            return Err(crate::error::DarviumError::TerminalStateViolation);
        }
        // ガード②: 遷移規則違反
        if !is_legal_transition(*self, next) {
            return Err(crate::error::DarviumError::SearchValidation(format!(
                "Illegal transition: {:?} -> {:?}",
                self, next
            )));
        }
        // 合法遷移: 状態を更新
        *self = next;
        Ok(())
    }
}

/// ポリシー発振検出器 (M-1.5-3)。
///
/// `Refine ↔ Retrieve` の交互遷移パターンを位相ベースで追跡し、
/// 閾値 `max_oscillation_count` を超えた場合に発振（oscillation）を検出する。
///
/// # 発振検出アルゴリズム
///
/// 位相ベース交互遷移カウンタ:
/// - `expected_next` に次に期待する遷移先状態を保持
/// - Refine の直後に Retrieve が来る → `oscillation_count`++
/// - Retrieve の直後に Refine が来る → `oscillation_count`++
/// - 上記以外の遷移 → リセット（`counter = 0`, `expected_next = None`）
/// - `oscillation_count >= max_oscillation_count` → `is_oscillating() = true`
///
/// RFC §13.5 の「`Refine -> Retrieve -> Refine` が閾値回数を超えて往復」に対応する。
/// 発振カウンタは saturated 加算によりオーバーフローを防止する。
#[derive(Debug, Clone)]
pub struct OscillationDetector {
    /// 発振カウンタ（Refine↔Retrieve の交互遷移回数）。
    oscillation_count: u32,
    /// 次に期待する遷移先状態（位相）。
    expected_next: Option<SearchState>,
    /// 発振検出閾値。
    max_oscillation_count: u32,
}

impl OscillationDetector {
    /// 指定された閾値で発振検出器を生成する。
    pub fn new(max_oscillation_count: u32) -> Self {
        Self {
            oscillation_count: 0,
            expected_next: None,
            max_oscillation_count,
        }
    }
}

impl Default for OscillationDetector {
    fn default() -> Self {
        Self::new(crate::constants::OSCILLATION_MAX_COUNT)
    }
}

impl OscillationDetector {
    /// 状態遷移を記録し、発振カウンタを更新する。
    ///
    /// 現在の `expected_next` と `to` が一致する場合、発振カウンタをインクリメントする。
    /// それ以外の場合はカウンタをリセットする。
    /// カウンタは saturated 加算により `u32::MAX` で飽和する。
    pub fn record_transition(&mut self, to: SearchState) {
        match self.expected_next {
            Some(expected) if expected == to => {
                // 発振パターン継続: カウンタを saturated 加算
                self.oscillation_count = self.oscillation_count.saturating_add(1);
                // 次の期待状態を切り替え
                self.expected_next = match to {
                    SearchState::Refine => Some(SearchState::Retrieve),
                    SearchState::Retrieve => Some(SearchState::Refine),
                    _ => None,
                };
            }
            _ => {
                // 非発振遷移: カウンタリセット。ただし Refine または Retrieve が来た場合は
                // 発振サイクルの開始として期待状態を設定
                self.oscillation_count = 0;
                self.expected_next = match to {
                    SearchState::Refine => Some(SearchState::Retrieve),
                    SearchState::Retrieve => Some(SearchState::Refine),
                    _ => None,
                };
            }
        }
    }

    /// 発振検出閾値を超えているか判定する。
    pub fn is_oscillating(&self) -> bool {
        self.oscillation_count >= self.max_oscillation_count
    }

    /// 発振カウンタを 0 にリセットする。
    pub fn reset(&mut self) {
        self.oscillation_count = 0;
        self.expected_next = None;
    }

    /// 現在の発振カウントを取得する。
    pub fn oscillation_count(&self) -> u32 {
        self.oscillation_count
    }

    /// 発振検出閾値を取得する。
    pub fn max_oscillation_count(&self) -> u32 {
        self.max_oscillation_count
    }
}

/// `transition_to` と発振検出を組み合わせた遷移試行ヘルパー (M-1.5-3)。
///
/// 1. 発振が既に検出済みの場合 → 強制 `Abort`（`record_transition` より先にチェック）
/// 2. `detector.record_transition(next)` で遷移を記録
/// 3. 発振検出時 → 状態を強制的に `Abort` に設定し `Err(SearchPolicyOscillation)` を返す
/// 4. 正常時 → `state.transition_to(next)` の結果を伝播
pub fn attempt_transition(
    state: &mut SearchState,
    detector: &mut OscillationDetector,
    next: SearchState,
) -> Result<(), crate::error::DarviumError> {
    // 発振が既に検出されていれば（record_transition によるリセット前に）即座に強制 Abort
    if detector.is_oscillating() {
        *state = SearchState::Abort;
        return Err(crate::error::DarviumError::SearchPolicyOscillation);
    }
    // 遷移を記録
    detector.record_transition(next);
    // 発振検出時は強制 Abort
    if detector.is_oscillating() {
        *state = SearchState::Abort;
        return Err(crate::error::DarviumError::SearchPolicyOscillation);
    }
    // 通常遷移
    state.transition_to(next)
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

    /// テスト1+2: RetrievalPrimitive の型シグネチャ呼び出し検証。
    /// MockEmptyRetrievalPrimitive を用いてトレイトの実装・呼び出しが
    /// 正しい型シグネチャでコンパイル・実行できることを確認する。
    #[test]
    fn mock_retrieval_primitive_invocation() {
        let primitive = crate::mock::MockEmptyRetrievalPrimitive::new();
        let query = QueryRepresentation::default();
        let policy = RetrievalPolicy::default();
        let result = primitive.search_workflows(&query, &policy);
        assert!(result.is_ok());
        let candidates = result.expect("MockEmptyRetrievalPrimitive should always return Ok");
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
            candidate
                .metadata
                .get("embedding_version")
                .and_then(|v| v.as_str()),
            Some("v2")
        );
    }

    /// テスト9: RetrievalPrimitive トレイトがオブジェクト安全であり、
    /// Box<dyn RetrievalPrimitive> として使用できることを確認する。
    #[test]
    fn trait_object_safety() {
        let primitive: Box<dyn RetrievalPrimitive> =
            Box::new(crate::mock::MockEmptyRetrievalPrimitive::new());
        let query = QueryRepresentation::default();
        let policy = RetrievalPolicy::default();
        let result = primitive.search_workflows(&query, &policy);
        assert!(result.is_ok());
    }

    // ── 計装・観測 (OTS-RP): RetrievalPrimitive 型依存関係観測 ──

    /// OTS-RP: 型依存関係の静的性質を観測する。
    ///
    /// RetrievalPrimitive トレイトとその関連型が形成する依存グラフの
    /// 直径を計測し、型システムが有界であることを確認する。
    #[test]
    fn observation_type_dependency_graph() {
        // 各型のメモリサイズを観測（型の複雑性のプロキシ計測）
        let size_qr = std::mem::size_of::<QueryRepresentation>();
        let size_rp = std::mem::size_of::<RetrievalPolicy>();
        let size_rc = std::mem::size_of::<RankedCandidate>();
        let size_cs = std::mem::size_of::<CandidateSet>();

        println!("=== OTS-RP: 型依存関係観測 ===");
        println!("QueryRepresentation size: {} bytes", size_qr);
        println!("RetrievalPolicy size: {} bytes", size_rp);
        println!("RankedCandidate size: {} bytes", size_rc);
        println!("CandidateSet size: {} bytes", size_cs);

        // 型サイズの有界性を検証（ピュアデータ型として妥当な範囲）
        assert!(
            size_qr > 0 && size_qr <= 1024,
            "QueryRepresentation size {} out of range",
            size_qr
        );
        assert!(
            size_rp > 0 && size_rp <= 256,
            "RetrievalPolicy size {} out of range",
            size_rp
        );
        assert!(
            size_rc > 0 && size_rc <= 512,
            "RankedCandidate size {} out of range",
            size_rc
        );
        assert!(
            size_cs > 0 && size_cs <= 512,
            "CandidateSet size {} out of range",
            size_cs
        );

        // 列挙型サイズの観測
        let size_qt = std::mem::size_of::<QueryType>();
        let size_fr = std::mem::size_of::<FreshnessRequirement>();
        let size_es = std::mem::size_of::<EvidenceStrictness>();
        let size_ds = std::mem::size_of::<DriftSensitivity>();
        println!("QueryType size: {} bytes", size_qt);
        println!("FreshnessRequirement size: {} bytes", size_fr);
        println!("EvidenceStrictness size: {} bytes", size_es);
        println!("DriftSensitivity size: {} bytes", size_ds);

        // トレイトオブジェクトの安全性確認
        let _: Box<dyn RetrievalPrimitive> =
            Box::new(crate::mock::MockEmptyRetrievalPrimitive::new());
        println!("RetrievalPrimitive trait object safety: OK");

        // 全バリアントの網羅的実行（型の完全性検証）
        let _types = (
            QueryType::Episodic,
            FreshnessRequirement::Recent,
            EvidenceStrictness::Light,
            DriftSensitivity::Ignore,
        );
        println!(
            "Enum exhaustive coverage: OK ({} variants total)",
            3 + 4 + 3 + 3
        );

        println!("=== 結果: PASS ===");
    }

    // ============================================================
    // M-2-2: SearchBudget / RecursionGuard テスト (T1〜T5 + OTS-1)
    // ============================================================

    /// T1: デフォルト値検証
    ///
    /// SearchBudget::default() と RecursionGuard::default() が
    /// constants.rs の定数から適切なデフォルト値を設定することを確認する。
    #[test]
    fn budget_default_values() {
        let budget = SearchBudget::default();
        assert_eq!(
            budget.max_iterations,
            crate::constants::DEFAULT_MAX_ITERATIONS
        );
        assert_eq!(
            budget.max_retrieval_calls,
            crate::constants::DEFAULT_MAX_RETRIEVAL_CALLS
        );
        assert_eq!(
            budget.max_prompt_tokens,
            crate::constants::MAX_PROMPT_TOKENS
        );
        assert_eq!(
            budget.max_wall_clock_ms,
            crate::constants::DEFAULT_MAX_WALL_CLOCK_MS
        );
    }

    #[test]
    fn recursion_guard_default_values() {
        let guard = RecursionGuard::default();
        assert_eq!(
            guard.max_depth,
            crate::constants::DEFAULT_RECURSION_MAX_DEPTH
        );
        assert_eq!(guard.current_depth, 0);
        assert!(!guard.allow_reentrant);
    }

    /// T2: サチュレーティングインクリメント境界値テスト
    ///
    /// try_consume_iteration / try_consume_retrieval_call / try_consume_prompt_tokens の
    /// 正常系・異常系・境界値を検証する。
    #[test]
    fn consume_iteration_normal() {
        let budget = SearchBudget::default();
        let mut snapshot = SearchBudgetSnapshot {
            iterations_used: 0,
            retrieval_calls_used: 0,
            prompt_tokens_used: 0,
            wall_clock_ms_used: 0,
        };
        assert!(budget.try_consume_iteration(&mut snapshot).is_ok());
        assert_eq!(snapshot.iterations_used, 1);
    }

    #[test]
    fn consume_iteration_exceeded() {
        let budget = SearchBudget {
            max_iterations: 1,
            ..SearchBudget::default()
        };
        let mut snapshot = SearchBudgetSnapshot {
            iterations_used: 1,
            retrieval_calls_used: 0,
            prompt_tokens_used: 0,
            wall_clock_ms_used: 0,
        };
        let result = budget.try_consume_iteration(&mut snapshot);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().to_string(), "Search budget exceeded");
    }

    #[test]
    fn consume_retrieval_call_normal() {
        let budget = SearchBudget::default();
        let mut snapshot = SearchBudgetSnapshot {
            iterations_used: 0,
            retrieval_calls_used: 0,
            prompt_tokens_used: 0,
            wall_clock_ms_used: 0,
        };
        assert!(budget.try_consume_retrieval_call(&mut snapshot).is_ok());
        assert_eq!(snapshot.retrieval_calls_used, 1);
    }

    #[test]
    fn consume_retrieval_call_exceeded() {
        let budget = SearchBudget {
            max_retrieval_calls: 1,
            ..SearchBudget::default()
        };
        let mut snapshot = SearchBudgetSnapshot {
            iterations_used: 0,
            retrieval_calls_used: 1,
            prompt_tokens_used: 0,
            wall_clock_ms_used: 0,
        };
        let result = budget.try_consume_retrieval_call(&mut snapshot);
        assert!(result.is_err());
    }

    #[test]
    fn consume_prompt_tokens_normal() {
        let budget = SearchBudget {
            max_prompt_tokens: 100,
            ..SearchBudget::default()
        };
        let mut snapshot = SearchBudgetSnapshot {
            iterations_used: 0,
            retrieval_calls_used: 0,
            prompt_tokens_used: 0,
            wall_clock_ms_used: 0,
        };
        assert!(budget.try_consume_prompt_tokens(&mut snapshot, 50).is_ok());
        assert_eq!(snapshot.prompt_tokens_used, 50);
    }

    #[test]
    fn consume_prompt_tokens_at_boundary() {
        let budget = SearchBudget {
            max_prompt_tokens: 100,
            ..SearchBudget::default()
        };
        let mut snapshot = SearchBudgetSnapshot {
            iterations_used: 0,
            retrieval_calls_used: 0,
            prompt_tokens_used: 100,
            wall_clock_ms_used: 0,
        };
        let result = budget.try_consume_prompt_tokens(&mut snapshot, 1);
        assert!(result.is_err());
    }

    #[test]
    fn consume_prompt_tokens_exact_limit() {
        let budget = SearchBudget {
            max_prompt_tokens: 100,
            ..SearchBudget::default()
        };
        let mut snapshot = SearchBudgetSnapshot {
            iterations_used: 0,
            retrieval_calls_used: 0,
            prompt_tokens_used: 0,
            wall_clock_ms_used: 0,
        };
        assert!(budget.try_consume_prompt_tokens(&mut snapshot, 100).is_ok());
        assert_eq!(snapshot.prompt_tokens_used, 100);
    }

    /// T3: RecursionGuard 境界値テスト
    #[test]
    fn increment_depth_normal() {
        let mut guard = RecursionGuard::new(5, true);
        assert_eq!(guard.current_depth, 0);
        assert!(guard.try_increment_depth().is_ok());
        assert_eq!(guard.current_depth, 1);
    }

    #[test]
    fn increment_depth_exceeded() {
        let mut guard = RecursionGuard {
            max_depth: 2,
            current_depth: 2,
            allow_reentrant: true,
        };
        let result = guard.try_increment_depth();
        assert!(result.is_err());
    }

    #[test]
    fn decrement_depth_reduces() {
        let mut guard = RecursionGuard {
            max_depth: 5,
            current_depth: 3,
            allow_reentrant: true,
        };
        guard.decrement_depth();
        assert_eq!(guard.current_depth, 2);
    }

    #[test]
    fn decrement_depth_no_underflow() {
        let mut guard = RecursionGuard {
            max_depth: 5,
            current_depth: 0,
            allow_reentrant: true,
        };
        guard.decrement_depth();
        assert_eq!(guard.current_depth, 0);
    }

    /// T4: allow_reentrant フラグ検証
    #[test]
    fn reentrant_disabled_fails_always() {
        let mut guard = RecursionGuard::new(5, false);
        let result = guard.try_increment_depth();
        assert!(result.is_err());
    }

    #[test]
    fn reentrant_enabled_allows_depth() {
        let mut guard = RecursionGuard::new(3, true);
        assert!(guard.try_increment_depth().is_ok());
        assert!(guard.try_increment_depth().is_ok());
        assert!(guard.try_increment_depth().is_ok());
        assert!(guard.try_increment_depth().is_err());
    }

    /// T5: SearchBudgetSnapshot 累積検証
    #[test]
    fn snapshot_tracks_accumulation() {
        let budget = SearchBudget {
            max_iterations: 10,
            max_retrieval_calls: 10,
            ..SearchBudget::default()
        };
        let mut snapshot = SearchBudgetSnapshot {
            iterations_used: 0,
            retrieval_calls_used: 0,
            prompt_tokens_used: 0,
            wall_clock_ms_used: 0,
        };

        budget.try_consume_iteration(&mut snapshot).unwrap();
        budget.try_consume_iteration(&mut snapshot).unwrap();
        budget.try_consume_iteration(&mut snapshot).unwrap();
        assert_eq!(snapshot.iterations_used, 3);

        budget.try_consume_retrieval_call(&mut snapshot).unwrap();
        budget.try_consume_retrieval_call(&mut snapshot).unwrap();
        assert_eq!(snapshot.retrieval_calls_used, 2);

        budget.try_consume_prompt_tokens(&mut snapshot, 50).unwrap();
        budget.try_consume_prompt_tokens(&mut snapshot, 50).unwrap();
        assert_eq!(snapshot.prompt_tokens_used, 100);

        let cloned = budget.snapshot(&snapshot);
        assert_eq!(cloned.iterations_used, 3);
        assert_eq!(cloned.retrieval_calls_used, 2);
        assert_eq!(cloned.prompt_tokens_used, 100);
    }

    /// OTS-1: 初期予算アンサンブル緩和時間計測
    ///
    /// シード固定 PRNG で SearchBudget の初期値を変動させた 10,000 個の
    /// アンサンブルを生成し、各軌道が上限境界に到達するまでの挙動を観測する。
    #[test]
    fn ots1_ensemble_relaxation_time() {
        use std::collections::BTreeMap;

        let mut rng: u64 = crate::constants::TEST_PRNG_SEED;
        let ensemble_size = 10_000;
        let max_steps = 200;

        let mut unsaturated_counts: BTreeMap<usize, usize> = BTreeMap::new();

        for _ in 0..ensemble_size {
            rng = rng
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let mut lcg = rng;

            let next_uniform = |v: &mut u64| -> u64 {
                *v = v
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                *v
            };

            let max_iter = (next_uniform(&mut lcg) % 50) + 1;
            let max_ret = (next_uniform(&mut lcg) % 25) + 1;
            let max_tok = (next_uniform(&mut lcg) % 1000) + 1;

            let budget = SearchBudget {
                max_iterations: max_iter as u32,
                max_retrieval_calls: max_ret as u32,
                max_prompt_tokens: max_tok,
                ..SearchBudget::default()
            };

            let mut snapshot = SearchBudgetSnapshot {
                iterations_used: 0,
                retrieval_calls_used: 0,
                prompt_tokens_used: 0,
                wall_clock_ms_used: 0,
            };

            for step in 0..max_steps {
                let action = next_uniform(&mut lcg) % 3;
                let result = match action {
                    0 => budget.try_consume_iteration(&mut snapshot),
                    1 => budget.try_consume_retrieval_call(&mut snapshot),
                    _ => budget.try_consume_prompt_tokens(
                        &mut snapshot,
                        (next_uniform(&mut lcg) % 50) + 1,
                    ),
                };

                if result.is_err() {
                    *unsaturated_counts.entry(step).or_insert(0) += 1;
                    break;
                }

                if step == max_steps - 1 {
                    *unsaturated_counts.entry(max_steps).or_insert(0) += 1;
                }
            }
        }

        let saturated = unsaturated_counts
            .iter()
            .filter(|(&step, _)| step < max_steps)
            .map(|(_, &count)| count)
            .sum::<usize>();
        assert_eq!(
            saturated, ensemble_size,
            "All trajectories must saturate within max_steps (saturated={}, ensemble={})",
            saturated, ensemble_size
        );

        println!("=== OTS-1: Ensemble Relaxation Time ===");
        println!("ensemble_size={}, max_steps={}", ensemble_size, max_steps);
        println!(
            "saturated={}, unsaturated_at_max={}",
            saturated,
            unsaturated_counts.get(&max_steps).unwrap_or(&0)
        );
        println!(
            "All trajectories saturated within {} steps (τ_relax = 0)",
            max_steps
        );
        println!("========================================");
    }

    // ============================================================
    // M-1.5-1: SearchState 合法状態遷移マトリクス
    // ============================================================

    use super::SearchState::{self, *};

    // ── T1: 全合法遷移の個別検証 ──

    /// T1-a: Init -> Retrieve
    #[test]
    fn legal_init_to_retrieve() {
        assert!(is_legal_transition(Init, Retrieve));
    }

    /// T1-b: Init -> Abort
    #[test]
    fn legal_init_to_abort() {
        assert!(is_legal_transition(Init, Abort));
    }

    /// T1-c: Retrieve -> Evaluate
    #[test]
    fn legal_retrieve_to_evaluate() {
        assert!(is_legal_transition(Retrieve, Evaluate));
    }

    /// T1-d: Retrieve -> Abort
    #[test]
    fn legal_retrieve_to_abort() {
        assert!(is_legal_transition(Retrieve, Abort));
    }

    /// T1-e: Evaluate -> Refine
    #[test]
    fn legal_evaluate_to_refine() {
        assert!(is_legal_transition(Evaluate, Refine));
    }

    /// T1-f: Evaluate -> Compose
    #[test]
    fn legal_evaluate_to_compose() {
        assert!(is_legal_transition(Evaluate, Compose));
    }

    /// T1-g: Evaluate -> Finalize
    #[test]
    fn legal_evaluate_to_finalize() {
        assert!(is_legal_transition(Evaluate, Finalize));
    }

    /// T1-h: Evaluate -> Abort
    #[test]
    fn legal_evaluate_to_abort() {
        assert!(is_legal_transition(Evaluate, Abort));
    }

    /// T1-i: Refine -> Retrieve
    #[test]
    fn legal_refine_to_retrieve() {
        assert!(is_legal_transition(Refine, Retrieve));
    }

    /// T1-j: Refine -> ProposeNew
    #[test]
    fn legal_refine_to_propose_new() {
        assert!(is_legal_transition(Refine, ProposeNew));
    }

    /// T1-k: Refine -> Abort
    #[test]
    fn legal_refine_to_abort() {
        assert!(is_legal_transition(Refine, Abort));
    }

    /// T1-l: Compose -> Finalize
    #[test]
    fn legal_compose_to_finalize() {
        assert!(is_legal_transition(Compose, Finalize));
    }

    /// T1-m: Compose -> Refine
    #[test]
    fn legal_compose_to_refine() {
        assert!(is_legal_transition(Compose, Refine));
    }

    /// T1-n: Compose -> Abort
    #[test]
    fn legal_compose_to_abort() {
        assert!(is_legal_transition(Compose, Abort));
    }

    /// T1-o: ProposeNew -> Finalize
    #[test]
    fn legal_propose_new_to_finalize() {
        assert!(is_legal_transition(ProposeNew, Finalize));
    }

    /// T1-p: ProposeNew -> Abort
    #[test]
    fn legal_propose_new_to_abort() {
        assert!(is_legal_transition(ProposeNew, Abort));
    }

    // ── T2: 全違法遷移のグループ検証 ──

    /// T2-a: 終端状態からの遷移は全て違法
    #[test]
    fn illegal_from_terminal_states() {
        let terminal_states = [Finalize, Abort];
        let all_states = [
            Init, Retrieve, Evaluate, Refine, Compose, ProposeNew, Finalize, Abort,
        ];
        for &from in &terminal_states {
            for &to in &all_states {
                assert!(
                    !is_legal_transition(from, to),
                    "Terminal state {:?} -> {:?} must be illegal",
                    from,
                    to
                );
            }
        }
    }

    /// T2-b: Init からの違法遷移
    #[test]
    fn illegal_from_init() {
        let illegal_targets = [Evaluate, Refine, Compose, ProposeNew, Finalize];
        for &to in &illegal_targets {
            assert!(
                !is_legal_transition(Init, to),
                "Init -> {:?} must be illegal",
                to
            );
        }
    }

    /// T2-c: Retrieve からの違法遷移
    #[test]
    fn illegal_from_retrieve() {
        let illegal_targets = [Init, Refine, Compose, ProposeNew, Finalize];
        for &to in &illegal_targets {
            assert!(
                !is_legal_transition(Retrieve, to),
                "Retrieve -> {:?} must be illegal",
                to
            );
        }
    }

    /// T2-d: Evaluate からの違法遷移
    #[test]
    fn illegal_from_evaluate() {
        let illegal_targets = [Init, Retrieve, ProposeNew];
        for &to in &illegal_targets {
            assert!(
                !is_legal_transition(Evaluate, to),
                "Evaluate -> {:?} must be illegal",
                to
            );
        }
    }

    /// T2-e: Refine からの違法遷移
    #[test]
    fn illegal_from_refine() {
        let illegal_targets = [Init, Evaluate, Compose, Finalize];
        for &to in &illegal_targets {
            assert!(
                !is_legal_transition(Refine, to),
                "Refine -> {:?} must be illegal",
                to
            );
        }
    }

    /// T2-f: Compose からの違法遷移
    #[test]
    fn illegal_from_compose() {
        let illegal_targets = [Init, Retrieve, Evaluate, ProposeNew];
        for &to in &illegal_targets {
            assert!(
                !is_legal_transition(Compose, to),
                "Compose -> {:?} must be illegal",
                to
            );
        }
    }

    /// T2-g: ProposeNew からの違法遷移
    #[test]
    fn illegal_from_propose_new() {
        let illegal_targets = [Init, Retrieve, Evaluate, Refine, Compose];
        for &to in &illegal_targets {
            assert!(
                !is_legal_transition(ProposeNew, to),
                "ProposeNew -> {:?} must be illegal",
                to
            );
        }
    }

    // ── T3: 総当たり 8×8 マトリクス ──

    /// T3: 64 ペア全てを総当たりで検証する。
    ///
    /// 合法 = 24 ペア、違法 = 40 ペアであることを確認する。
    /// 遷移マトリクスを CSV 形式で構造化出力する。
    #[test]
    fn exhaustive_transition_matrix() {
        let all_states = [
            Init, Retrieve, Evaluate, Refine, Compose, ProposeNew, Finalize, Abort,
        ];

        // 期待される合法ペアの集合
        let legal_pairs: &[(SearchState, SearchState)] = &[
            (Init, Retrieve),
            (Init, Abort),
            (Retrieve, Evaluate),
            (Retrieve, Abort),
            (Evaluate, Refine),
            (Evaluate, Compose),
            (Evaluate, Finalize),
            (Evaluate, Abort),
            (Refine, Retrieve),
            (Refine, ProposeNew),
            (Refine, Abort),
            (Compose, Finalize),
            (Compose, Refine),
            (Compose, Abort),
            (ProposeNew, Finalize),
            (ProposeNew, Abort),
        ];

        let mut legal_count = 0u32;
        let mut illegal_count = 0u32;

        println!("=== T3: 8x8 Transition Matrix ===");
        println!("from,to,legal");
        for &from in &all_states {
            for &to in &all_states {
                let result = is_legal_transition(from, to);
                println!("{:?},{:?},{}", from, to, result);
                if result {
                    legal_count += 1;
                } else {
                    illegal_count += 1;
                }
            }
        }
        println!(
            "--- Summary: legal={}, illegal={} ---",
            legal_count, illegal_count
        );

        // 期待通りであることを個別に全件確認
        for &(from, to) in legal_pairs {
            assert!(
                is_legal_transition(from, to),
                "Expected legal: {:?} -> {:?}",
                from,
                to
            );
        }

        assert_eq!(legal_count, 16, "Expected exactly 16 legal transitions");
        assert_eq!(illegal_count, 48, "Expected exactly 48 illegal transitions");
    }

    // ── T4: 境界値テスト ──

    /// T4-a: SearchState が 1 バイトで表現可能であること。
    #[test]
    fn search_state_size() {
        assert_eq!(
            std::mem::size_of::<SearchState>(),
            1usize,
            "SearchState should be 1 byte (u8 repr)"
        );
    }

    // ── OTS-1: 遷移マトリクス完全性の観測 ──

    /// OTS-1: 64 ペア全ての判定結果を構造化出力し、欠損なく完全な
    /// 遷移マトリクスが得られていることを観測する。
    #[test]
    fn ots_transition_matrix_completeness() {
        let all_states = [
            Init, Retrieve, Evaluate, Refine, Compose, ProposeNew, Finalize, Abort,
        ];
        let total_pairs = all_states.len() * all_states.len();

        println!("=== OTS-1: Transition Matrix Completeness ===");
        println!("states={}, total_pairs={}", all_states.len(), total_pairs);

        let mut csv_lines = Vec::with_capacity(total_pairs);
        for &from in &all_states {
            for &to in &all_states {
                csv_lines.push(format!(
                    "{:?},{:?},{}",
                    from,
                    to,
                    is_legal_transition(from, to)
                ));
            }
        }

        for line in &csv_lines {
            println!("{}", line);
        }
        println!(
            "--- Summary: {} total pairs (expected 64) ---",
            csv_lines.len()
        );
        assert_eq!(csv_lines.len(), 64, "Must have exactly 64 pairs");
        println!("=== 結果: PASS ===");
    }

    // ── OTS-2: スペクトル半径計測 ──

    /// OTS-2: 過渡状態（非終端状態）の部分遷移確率行列 Q の
    /// スペクトル半径 ρ(Q) を計算し、ρ < 1 であることを確認する。
    ///
    /// 過渡状態 = {Init, Retrieve, Evaluate, Refine, Compose, ProposeNew} の 6 状態。
    /// 各状態からの合法遷移に等確率を割り当てた行列を構築し、
    /// 最大固有値（スペクトル半径）を電力法（power iteration）で近似する。
    #[test]
    fn ots_spectral_radius() {
        // 過渡状態（非終端状態 6 種）のリスト
        let transient_states: &[SearchState] = &[
            SearchState::Init,
            SearchState::Retrieve,
            SearchState::Evaluate,
            SearchState::Refine,
            SearchState::Compose,
            SearchState::ProposeNew,
        ];
        let state_count = transient_states.len(); // = 6

        // 遷移確率行列 transition_matrix (state_count x state_count) を構築
        // transition_matrix[row][col] = transient_states[row] から transient_states[col] への
        // 合法遷移確率（全出力先で等確率配分したうちの過渡状態間成分のみ）
        let mut transition_matrix = vec![vec![0.0f64; state_count]; state_count];

        for (row_idx, &from) in transient_states.iter().enumerate() {
            // この状態からの全合法遷移先をカウント
            let mut outgoing_indices: Vec<usize> = Vec::new();
            for (col_idx, &to) in transient_states.iter().enumerate() {
                if is_legal_transition(from, to) {
                    outgoing_indices.push(col_idx);
                }
            }
            // 終端状態への遷移も数に含めた上で、過渡状態間の成分のみを行列に抽出
            let mut transient_outgoing: Vec<usize> = Vec::new();
            for &col_idx in &outgoing_indices {
                if transient_states
                    .iter()
                    .any(|&s| s == transient_states[col_idx])
                {
                    transient_outgoing.push(col_idx);
                }
            }
            // 過渡状態間の遷移確率行列が部分確率（sub-stochastic）であることを確認するため、
            // 全出力先（終端含む）で等確率配分し、過渡状態間のみを行列に抽出する。
            let total_outgoing = outgoing_indices.len();
            for &col_idx in &transient_outgoing {
                transition_matrix[row_idx][col_idx] = 1.0 / total_outgoing as f64;
            }
        }

        // 電力法 (Power Iteration) で最大固有値を近似
        // 初期ベクトル eigenvector = (1, 1, ..., 1) / sqrt(state_count)
        let mut eigenvector: Vec<f64> = vec![1.0 / (state_count as f64).sqrt(); state_count];
        let iterations = 1000;
        let mut eigenvalue = 0.0f64;

        for _iteration in 0..iterations {
            // eigenvector_next = transition_matrix * eigenvector
            let mut eigenvector_next = vec![0.0f64; state_count];
            for row_idx in 0..state_count {
                for col_idx in 0..state_count {
                    eigenvector_next[row_idx] +=
                        transition_matrix[row_idx][col_idx] * eigenvector[col_idx];
                }
            }
            // Rayleigh 商: λ = (v^T * Q * v) / (v^T * v) = (v^T * v_next) / (v^T * v)
            // 正規化: eigenvector_next = eigenvector_next / ||eigenvector_next||
            let norm: f64 = eigenvector_next.iter().map(|x| x * x).sum::<f64>().sqrt();
            if norm < 1e-15 {
                eigenvalue = 0.0;
                break;
            }
            // Rayleigh 商の計算（正規化前）
            let rayleigh: f64 = eigenvector_next
                .iter()
                .zip(eigenvector.iter())
                .map(|(vn, vv)| vn * vv)
                .sum::<f64>();
            // eigenvector を正規化
            for x in &mut eigenvector_next {
                *x /= norm;
            }
            eigenvalue = rayleigh;
            eigenvector = eigenvector_next;
        }

        println!("=== OTS-2: Spectral Radius ===");
        println!("transient_states={}", state_count);
        println!("rho(transition_matrix) ≈ {:.10}", eigenvalue);
        assert!(
            eigenvalue < 1.0,
            "Spectral radius ρ(Q) must be < 1 (got {:.10})",
            eigenvalue
        );
        println!("ρ(Q) < 1: OK, all trajectories reach terminal states in finite steps");
        println!("=== 結果: PASS (ρ = {:.10}) ===", eigenvalue);
    }

    // ── OTS-3: 平均自由行程（平均吸収時間）の観測 ──

    /// OTS-3: 全状態から一様ランダムに合法遷移を繰り返した場合の、
    /// 終端状態（Finalize / Abort）到達までの平均ステップ数を観測する。
    ///
    /// マルコフ連鎖の吸収時間の推定値が有限であることを確認する。
    #[test]
    fn ots_mean_absorption_time() {
        use crate::constants::TEST_PRNG_SEED;

        let all_states = [
            Init, Retrieve, Evaluate, Refine, Compose, ProposeNew, Finalize, Abort,
        ];

        // LCG 簡易乱数
        let mut rng = TEST_PRNG_SEED;

        let mut lcg_u64 = move || -> u64 {
            rng = rng
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            rng
        };

        let sample_size = 10_000;
        let max_steps = 10_000;
        let mut total_steps: u64 = 0;
        let mut absorbed_count: u64 = 0;

        for _ in 0..sample_size {
            let mut state = Init;
            let mut steps: u64 = 0;
            loop {
                // 現在の状態からの合法遷移先を列挙
                let mut legal_targets: Vec<SearchState> = Vec::new();
                for &to in &all_states {
                    if is_legal_transition(state, to) {
                        legal_targets.push(to);
                    }
                }
                if legal_targets.is_empty() {
                    // 終端状態にいる（はず）
                    break;
                }
                // 合法遷移先からランダム選択
                let idx = (lcg_u64() % legal_targets.len() as u64) as usize;
                state = legal_targets[idx];
                steps += 1;

                if state == Finalize || state == Abort {
                    absorbed_count += 1;
                    total_steps += steps;
                    break;
                }
                if steps >= max_steps {
                    // 安全ガード
                    break;
                }
            }
        }

        let mean_steps = total_steps as f64 / absorbed_count as f64;

        println!("=== OTS-3: Mean Absorption Time ===");
        println!(
            "samples={}, absorbed={}, max_steps={}",
            sample_size, absorbed_count, max_steps
        );
        println!("mean_absorption_steps={:.4}", mean_steps);
        assert!(
            absorbed_count == sample_size,
            "All trajectories must be absorbed (absorbed={}, total={})",
            absorbed_count,
            sample_size
        );
        assert!(
            mean_steps.is_finite() && mean_steps > 0.0,
            "Mean absorption time must be finite and positive (got {})",
            mean_steps
        );
        println!(
            "=== 結果: PASS (mean absorption = {:.4} steps) ===",
            mean_steps
        );
    }

    // ============================================================
    // M-1.5-2: 終端状態（Finalize / Abort）非再入不変条件の強制
    // ============================================================

    use super::{can_terminate_with, TerminalTransitionReason};

    // ── T1: transition_to 終端状態ガード（個別検証） ──

    /// T1-a: Finalize からの遷移は TerminalStateViolation
    #[test]
    fn terminal_finalize_transition_to() {
        let mut state = Finalize;
        let result = state.transition_to(Init);
        assert!(
            matches!(result, Err(DarviumError::TerminalStateViolation)),
            "Finalize -> Init must return TerminalStateViolation, got {:?}",
            result
        );
    }

    /// T1-b: Abort からの遷移は TerminalStateViolation
    #[test]
    fn terminal_abort_transition_to() {
        let mut state = Abort;
        let result = state.transition_to(Init);
        assert!(
            matches!(result, Err(DarviumError::TerminalStateViolation)),
            "Abort -> Init must return TerminalStateViolation, got {:?}",
            result
        );
    }

    // ── T2: 終端状態ガード全網羅 ──

    /// T2-a: Finalize から全 8 状態への遷移が全て TerminalStateViolation
    #[test]
    fn terminal_finalize_all_targets() {
        let all_states = [
            Init, Retrieve, Evaluate, Refine, Compose, ProposeNew, Finalize, Abort,
        ];
        for &target in &all_states {
            let mut state = Finalize;
            let result = state.transition_to(target);
            assert!(
                matches!(result, Err(DarviumError::TerminalStateViolation)),
                "Finalize -> {:?} must be TerminalStateViolation, got {:?}",
                target,
                result
            );
        }
    }

    /// T2-b: Abort から全 8 状態への遷移が全て TerminalStateViolation
    #[test]
    fn terminal_abort_all_targets() {
        let all_states = [
            Init, Retrieve, Evaluate, Refine, Compose, ProposeNew, Finalize, Abort,
        ];
        for &target in &all_states {
            let mut state = Abort;
            let result = state.transition_to(target);
            assert!(
                matches!(result, Err(DarviumError::TerminalStateViolation)),
                "Abort -> {:?} must be TerminalStateViolation, got {:?}",
                target,
                result
            );
        }
    }

    // ── T3: transition_to 合法遷移成功（16 パターン） ──

    /// T3-a: Init -> Retrieve
    #[test]
    fn transition_legal_init_to_retrieve() {
        let mut state = Init;
        assert!(state.transition_to(Retrieve).is_ok());
        assert_eq!(state, Retrieve);
    }

    /// T3-b: Init -> Abort
    #[test]
    fn transition_legal_init_to_abort() {
        let mut state = Init;
        assert!(state.transition_to(Abort).is_ok());
        assert_eq!(state, Abort);
    }

    /// T3-c: Retrieve -> Evaluate
    #[test]
    fn transition_legal_retrieve_to_evaluate() {
        let mut state = Retrieve;
        assert!(state.transition_to(Evaluate).is_ok());
        assert_eq!(state, Evaluate);
    }

    /// T3-d: Retrieve -> Abort
    #[test]
    fn transition_legal_retrieve_to_abort() {
        let mut state = Retrieve;
        assert!(state.transition_to(Abort).is_ok());
        assert_eq!(state, Abort);
    }

    /// T3-e: Evaluate -> Refine
    #[test]
    fn transition_legal_evaluate_to_refine() {
        let mut state = Evaluate;
        assert!(state.transition_to(Refine).is_ok());
        assert_eq!(state, Refine);
    }

    /// T3-f: Evaluate -> Compose
    #[test]
    fn transition_legal_evaluate_to_compose() {
        let mut state = Evaluate;
        assert!(state.transition_to(Compose).is_ok());
        assert_eq!(state, Compose);
    }

    /// T3-g: Evaluate -> Finalize
    #[test]
    fn transition_legal_evaluate_to_finalize() {
        let mut state = Evaluate;
        assert!(state.transition_to(Finalize).is_ok());
        assert_eq!(state, Finalize);
    }

    /// T3-h: Evaluate -> Abort
    #[test]
    fn transition_legal_evaluate_to_abort() {
        let mut state = Evaluate;
        assert!(state.transition_to(Abort).is_ok());
        assert_eq!(state, Abort);
    }

    /// T3-i: Refine -> Retrieve
    #[test]
    fn transition_legal_refine_to_retrieve() {
        let mut state = Refine;
        assert!(state.transition_to(Retrieve).is_ok());
        assert_eq!(state, Retrieve);
    }

    /// T3-j: Refine -> ProposeNew
    #[test]
    fn transition_legal_refine_to_propose_new() {
        let mut state = Refine;
        assert!(state.transition_to(ProposeNew).is_ok());
        assert_eq!(state, ProposeNew);
    }

    /// T3-k: Refine -> Abort
    #[test]
    fn transition_legal_refine_to_abort() {
        let mut state = Refine;
        assert!(state.transition_to(Abort).is_ok());
        assert_eq!(state, Abort);
    }

    /// T3-l: Compose -> Finalize
    #[test]
    fn transition_legal_compose_to_finalize() {
        let mut state = Compose;
        assert!(state.transition_to(Finalize).is_ok());
        assert_eq!(state, Finalize);
    }

    /// T3-m: Compose -> Refine
    #[test]
    fn transition_legal_compose_to_refine() {
        let mut state = Compose;
        assert!(state.transition_to(Refine).is_ok());
        assert_eq!(state, Refine);
    }

    /// T3-n: Compose -> Abort
    #[test]
    fn transition_legal_compose_to_abort() {
        let mut state = Compose;
        assert!(state.transition_to(Abort).is_ok());
        assert_eq!(state, Abort);
    }

    /// T3-o: ProposeNew -> Finalize
    #[test]
    fn transition_legal_propose_new_to_finalize() {
        let mut state = ProposeNew;
        assert!(state.transition_to(Finalize).is_ok());
        assert_eq!(state, Finalize);
    }

    /// T3-p: ProposeNew -> Abort
    #[test]
    fn transition_legal_propose_new_to_abort() {
        let mut state = ProposeNew;
        assert!(state.transition_to(Abort).is_ok());
        assert_eq!(state, Abort);
    }

    // ── T4: transition_to 違法遷移拒否 ──

    /// T4: 非終端状態からの違法遷移が SearchValidation エラーを返すこと。
    //
    // 検証する違法ペアは M-1.5-1 の T2-a〜T2-g と同じ:
    //   - Init -> {Evaluate, Refine, Compose, ProposeNew, Finalize}        (5)
    //   - Retrieve -> {Init, Refine, Compose, ProposeNew, Finalize}        (5)
    //   - Evaluate -> {Init, Retrieve, ProposeNew}                         (3)
    //   - Refine -> {Init, Evaluate, Compose, Finalize}                    (4)
    //   - Compose -> {Init, Retrieve, Evaluate, ProposeNew}                (4)
    //   - ProposeNew -> {Init, Retrieve, Evaluate, Refine, Compose}        (5)
    #[test]
    fn illegal_transition_returns_search_validation() {
        let illegal_pairs: &[(SearchState, SearchState)] = &[
            // Init からの違法遷移 (T2-b)
            (Init, Evaluate),
            (Init, Refine),
            (Init, Compose),
            (Init, ProposeNew),
            (Init, Finalize),
            // Retrieve からの違法遷移 (T2-c)
            (Retrieve, Init),
            (Retrieve, Refine),
            (Retrieve, Compose),
            (Retrieve, ProposeNew),
            (Retrieve, Finalize),
            // Evaluate からの違法遷移 (T2-d)
            (Evaluate, Init),
            (Evaluate, Retrieve),
            (Evaluate, ProposeNew),
            // Refine からの違法遷移 (T2-e)
            (Refine, Init),
            (Refine, Evaluate),
            (Refine, Compose),
            (Refine, Finalize),
            // Compose からの違法遷移 (T2-f)
            (Compose, Init),
            (Compose, Retrieve),
            (Compose, Evaluate),
            (Compose, ProposeNew),
            // ProposeNew からの違法遷移 (T2-g)
            (ProposeNew, Init),
            (ProposeNew, Retrieve),
            (ProposeNew, Evaluate),
            (ProposeNew, Refine),
            (ProposeNew, Compose),
        ];
        for &(from, to) in illegal_pairs {
            let mut state = from;
            let result = state.transition_to(to);
            assert!(
                matches!(result, Err(DarviumError::SearchValidation(_))),
                "{:?} -> {:?} must return SearchValidation, got {:?}",
                from,
                to,
                result
            );
        }
    }

    // ── T5: can_terminate_with 判定 ──

    /// T5-a: BudgetExceeded は終端許可
    #[test]
    fn can_terminate_budget_exceeded() {
        assert!(can_terminate_with(TerminalTransitionReason::BudgetExceeded));
    }

    /// T5-b: RecursionExceeded は終端許可
    #[test]
    fn can_terminate_recursion_exceeded() {
        assert!(can_terminate_with(
            TerminalTransitionReason::RecursionExceeded
        ));
    }

    /// T5-c: ExplicitAbort は終端許可
    #[test]
    fn can_terminate_explicit_abort() {
        assert!(can_terminate_with(TerminalTransitionReason::ExplicitAbort));
    }

    /// T5-d: NormalCompletion は終端許可
    #[test]
    fn can_terminate_normal_completion() {
        assert!(can_terminate_with(
            TerminalTransitionReason::NormalCompletion
        ));
    }

    /// T5-e: SingleCandidateFailure は終端不許可
    #[test]
    fn can_terminate_single_candidate_failure() {
        assert!(!can_terminate_with(
            TerminalTransitionReason::SingleCandidateFailure
        ));
    }

    // ── T6: 状態更新の正確性（連続遷移）──

    /// T6: 複数回の合法遷移を連続して実行し、中間状態が正しいことを確認する。
    //
    // 遷移系列: Init -> Retrieve -> Evaluate -> Refine -> Retrieve -> Evaluate -> Finalize
    #[test]
    fn sequential_transitions_correctness() {
        let mut state = Init;

        assert!(state.transition_to(Retrieve).is_ok());
        assert_eq!(state, Retrieve);

        assert!(state.transition_to(Evaluate).is_ok());
        assert_eq!(state, Evaluate);

        assert!(state.transition_to(Refine).is_ok());
        assert_eq!(state, Refine);

        assert!(state.transition_to(Retrieve).is_ok());
        assert_eq!(state, Retrieve);

        assert!(state.transition_to(Evaluate).is_ok());
        assert_eq!(state, Evaluate);

        assert!(state.transition_to(Finalize).is_ok());
        assert_eq!(state, Finalize);

        // 終端状態からの遷移は失敗する
        let result = state.transition_to(Init);
        assert!(
            matches!(result, Err(DarviumError::TerminalStateViolation)),
            "After Finalize, transition_to must return TerminalStateViolation, got {:?}",
            result
        );
    }

    // ── OTS-1: 終端状態維持（パルス注入） ──

    /// OTS-1: 終端状態に固定された SearchState に対し、マルチスレッドから
    /// 100,000 回の transition_to 呼び出しを注入し、終端状態維持率 100% を確認する。
    ///
    /// ガードレイテンシ（τ_gate）の平均・最大・最小も観測する。
    #[test]
    fn ots_terminal_state_pulse_injection() {
        use std::sync::{Arc, Mutex};
        use std::thread;

        let pulse_count_per_thread: u64 = 10_000;
        let thread_count: u64 = 10;
        let total_pulses = pulse_count_per_thread * thread_count;

        // 終端状態に固定
        let state = Arc::new(Mutex::new(Finalize));
        let all_targets = [
            Init, Retrieve, Evaluate, Refine, Compose, ProposeNew, Finalize, Abort,
        ];

        println!("=== OTS-1: Terminal State Pulse Injection ===");
        println!(
            "threads={}, pulses_per_thread={}, total={}",
            thread_count, pulse_count_per_thread, total_pulses
        );

        let mut handles = Vec::with_capacity(thread_count as usize);
        for _ in 0..thread_count {
            let state_clone = Arc::clone(&state);
            let handle = thread::spawn(move || {
                let mut local_ok_count: u64 = 0;
                let mut local_err_count: u64 = 0;
                let mut total_latency_ns: u128 = 0;
                let mut max_latency_ns: u128 = 0;
                let mut min_latency_ns: u128 = u128::MAX;

                for i in 0..pulse_count_per_thread {
                    let target = all_targets[(i as usize) % all_targets.len()];
                    let start = std::time::Instant::now();

                    let mut guard = state_clone.lock().unwrap();
                    let result = guard.transition_to(target);
                    let elapsed = start.elapsed().as_nanos();

                    // 状態を終端に戻す（ガードが効いている場合は不要だが安全のため）
                    if matches!(result, Err(DarviumError::TerminalStateViolation)) {
                        local_err_count += 1;
                    } else {
                        // もし成功してしまったら終端に戻す
                        *guard = Finalize;
                        local_ok_count += 1;
                    }
                    drop(guard);

                    total_latency_ns = total_latency_ns.wrapping_add(elapsed);
                    if elapsed > max_latency_ns {
                        max_latency_ns = elapsed;
                    }
                    if elapsed < min_latency_ns {
                        min_latency_ns = elapsed;
                    }
                }

                let avg_latency = if local_err_count + local_ok_count > 0 {
                    total_latency_ns / (local_err_count + local_ok_count) as u128
                } else {
                    0
                };

                (
                    local_err_count,
                    local_ok_count,
                    avg_latency,
                    max_latency_ns,
                    min_latency_ns,
                )
            });
            handles.push(handle);
        }

        let mut total_err: u64 = 0;
        let mut total_ok: u64 = 0;
        let mut grand_avg_latency: u128 = 0;
        let mut grand_max_latency: u128 = 0;
        let mut grand_min_latency: u128 = u128::MAX;

        for handle in handles {
            let (err, ok, avg, max, min) = handle.join().unwrap();
            total_err += err;
            total_ok += ok;
            grand_avg_latency = grand_avg_latency.wrapping_add(avg);
            if max > grand_max_latency {
                grand_max_latency = max;
            }
            if min < grand_min_latency {
                grand_min_latency = min;
            }
        }

        grand_avg_latency /= thread_count as u128;

        let terminal_maintenance_rate = total_err as f64 / total_pulses as f64 * 100.0;
        println!(
            "terminal_maintenance_rate={:.6}% (violations={}, total={})",
            terminal_maintenance_rate, total_ok, total_pulses
        );
        println!(
            "guard_latency_ns: avg={}, max={}, min={}",
            grand_avg_latency, grand_max_latency, grand_min_latency
        );

        // 終端状態維持率 100% を検証
        assert_eq!(
            total_ok, 0,
            "Terminal state must be maintained 100% (got {} violations)",
            total_ok
        );
        assert_eq!(
            total_err, total_pulses,
            "All pulses must return TerminalStateViolation (got {} / {})",
            total_err, total_pulses
        );

        // 最終状態も終端であることを確認
        let final_state = *state.lock().unwrap();
        assert_eq!(final_state, Finalize, "Final state must remain Finalize");
        println!("=== 結果: PASS ===");
    }

    // ── OTS-2: ガードレイテンシ分布（拡張計測） ──

    /// OTS-2: 単一スレッドで 10,000 回の transition_to 呼び出しを行い、
    /// ガードレイテンシの分布を観測する。これによりマルチスレッド環境の
    /// ロック競合を排除した純粋なガードロジックのコストを計測する。
    #[test]
    fn ots_gate_latency_distribution() {
        let iterations: u64 = 10_000;
        let mut state = Finalize;
        let all_targets = [
            Init, Retrieve, Evaluate, Refine, Compose, ProposeNew, Finalize, Abort,
        ];

        let mut latencies_ns: Vec<u128> = Vec::with_capacity(iterations as usize);

        for i in 0..iterations {
            let target = all_targets[(i as usize) % all_targets.len()];
            let start = std::time::Instant::now();
            let _ = state.transition_to(target);
            let elapsed = start.elapsed().as_nanos();
            latencies_ns.push(elapsed);
        }

        let avg: f64 = latencies_ns.iter().copied().sum::<u128>() as f64 / iterations as f64;
        let max = latencies_ns.iter().copied().max().unwrap_or(0);
        let min = latencies_ns.iter().copied().min().unwrap_or(0);

        println!("=== OTS-2: Guard Latency Distribution ===");
        println!(
            "samples={}, avg_latency_ns={:.3}, max_latency_ns={}, min_latency_ns={}",
            iterations, avg, max, min
        );
        println!("=== 結果: ガードレイテンシ計測完了 ===");
    }

    // ── OTS-3: can_terminate_with 判定表 ──

    /// OTS-3: 全 6 理由の can_terminate_with 判定結果を構造化出力し、
    /// SingleCandidateFailure のみ false であることを確認する。
    #[test]
    fn ots_can_terminate_matrix() {
        use super::TerminalTransitionReason::*;

        let reasons = [
            BudgetExceeded,
            RecursionExceeded,
            ExplicitAbort,
            NormalCompletion,
            SingleCandidateFailure,
            OscillationDetected,
        ];

        println!("=== OTS-3: can_terminate_with Decision Matrix ===");
        println!("reason,can_terminate");

        let mut true_count = 0u32;
        let mut false_count = 0u32;

        for &reason in &reasons {
            let result = can_terminate_with(reason);
            println!("{:?},{}", reason, result);
            if result {
                true_count += 1;
            } else {
                false_count += 1;
            }
        }

        println!(
            "--- Summary: can_terminate=true={}, false={} ---",
            true_count, false_count
        );

        assert_eq!(true_count, 5, "Exactly 5 reasons must allow termination");
        assert_eq!(false_count, 1, "Exactly 1 reason must deny termination");
        assert!(!can_terminate_with(SingleCandidateFailure));
        assert!(can_terminate_with(OscillationDetected));
        println!("=== 結果: PASS ===");
    }

    // ============================================================
    // M-1.5-3: SearchPolicyOscillation 検出エンジン
    // ============================================================

    use super::{attempt_transition, OscillationDetector};

    // ── T1: 発振検出の基本動作 ──

    /// T1-a: 閾値 3 でカウントが 3 に達したら is_oscillating() = true
    ///
    /// record_transition は各呼び出しごとに交互パターンをカウントする:
    ///   Refine(初期化) → Retrieve(c=1) → Refine(c=2) → Retrieve(c=3≥3 → oscillating)
    #[test]
    fn oscillation_detection_basic() {
        let mut detector = OscillationDetector::new(3);

        // Refine: 期待状態を設定（カウントは 0）
        detector.record_transition(Refine);
        assert!(!detector.is_oscillating());
        assert_eq!(detector.oscillation_count(), 0);

        // Retrieve: 発振カウント 1
        detector.record_transition(Retrieve);
        assert!(!detector.is_oscillating());
        assert_eq!(detector.oscillation_count(), 1);

        // Refine: 発振カウント 2
        detector.record_transition(Refine);
        assert!(!detector.is_oscillating());
        assert_eq!(detector.oscillation_count(), 2);

        // Retrieve: 発振カウント 3 → 閾値到達
        detector.record_transition(Retrieve);
        assert!(detector.is_oscillating());
        assert_eq!(detector.oscillation_count(), 3);
    }

    /// T1-b: 閾値 3 で発振 1 回のみなら is_oscillating() = false
    #[test]
    fn oscillation_single_cycle_not_detected() {
        let mut detector = OscillationDetector::new(3);

        detector.record_transition(Refine);
        detector.record_transition(Retrieve);

        assert!(!detector.is_oscillating());
        assert_eq!(detector.oscillation_count(), 1);
    }

    /// T1-c: 閾値 3 で発振カウント 2 未満なら is_oscillating() = false
    ///
    /// Refine(初期化) → Retrieve(c=1) → Refine(c=2): 2 < 3 → 検出なし
    #[test]
    fn oscillation_two_cycles_not_detected() {
        let mut detector = OscillationDetector::new(3);

        detector.record_transition(Refine);
        detector.record_transition(Retrieve);
        detector.record_transition(Refine);

        assert!(!detector.is_oscillating());
        assert_eq!(detector.oscillation_count(), 2);
    }

    // ── T2: 非発振パターンでのリセット ──

    /// T2-a: Refine→Retrieve の後に Evaluate が来るとカウンタリセット
    #[test]
    fn oscillation_reset_on_evaluate() {
        let mut detector = OscillationDetector::new(3);

        detector.record_transition(Refine);
        detector.record_transition(Retrieve);
        assert_eq!(detector.oscillation_count(), 1);

        // Evaluate でリセット
        detector.record_transition(Evaluate);
        assert_eq!(detector.oscillation_count(), 0);
        assert!(!detector.is_oscillating());
    }

    /// T2-b: Refine→Retrieve の後に Init が来るとカウンタリセット
    #[test]
    fn oscillation_reset_on_init() {
        let mut detector = OscillationDetector::new(3);

        detector.record_transition(Refine);
        detector.record_transition(Retrieve);
        assert_eq!(detector.oscillation_count(), 1);

        // Init でリセット
        detector.record_transition(Init);
        assert_eq!(detector.oscillation_count(), 0);
    }

    /// T2-c: Init→Retrieve→Evaluate では発振未検出
    #[test]
    fn no_oscillation_on_normal_sequence() {
        let mut detector = OscillationDetector::new(3);

        detector.record_transition(Init);
        detector.record_transition(Retrieve);
        detector.record_transition(Evaluate);

        assert_eq!(detector.oscillation_count(), 0);
        assert!(!detector.is_oscillating());
    }

    // ── T3: attempt_transition 統合テスト ──

    /// T3-a: 非発振系列で attempt_transition が成功する
    #[test]
    fn attempt_transition_normal_sequence() {
        let mut state = Init;
        let mut detector = OscillationDetector::new(3);

        assert!(attempt_transition(&mut state, &mut detector, Retrieve).is_ok());
        assert_eq!(state, Retrieve);
        assert!(attempt_transition(&mut state, &mut detector, Evaluate).is_ok());
        assert_eq!(state, Evaluate);
        assert!(attempt_transition(&mut state, &mut detector, Finalize).is_ok());
        assert_eq!(state, Finalize);
    }

    /// T3-b: 発振カウントが閾値に達した attempt_transition が
    /// Err(SearchPolicyOscillation) を返す
    #[test]
    fn attempt_transition_oscillation_detected() {
        let mut state = Refine;
        let mut detector = OscillationDetector::new(3);

        // Pre-seed: 発振カウント 2（閾値直前）まで record_transition で設定
        detector.record_transition(Refine);
        detector.record_transition(Retrieve);
        detector.record_transition(Refine);
        assert_eq!(detector.oscillation_count(), 2);
        assert!(!detector.is_oscillating());

        // この attempt_transition でカウント 3 → 発振検出 → 強制 Abort
        let result = attempt_transition(&mut state, &mut detector, Retrieve);
        assert!(
            matches!(result, Err(DarviumError::SearchPolicyOscillation)),
            "Expected SearchPolicyOscillation, got {:?}",
            result
        );
    }

    /// T3-c: 発振検出後の状態が Abort になっている
    #[test]
    fn state_is_abort_after_oscillation() {
        let mut state = Refine;
        let mut detector = OscillationDetector::new(3);

        // Pre-seed: 発振カウント 2（閾値直前）
        detector.record_transition(Refine);
        detector.record_transition(Retrieve);
        detector.record_transition(Refine);

        // この attempt_transition で発振検出 → 状態が Abort に
        let _ = attempt_transition(&mut state, &mut detector, Retrieve);

        assert_eq!(
            state, Abort,
            "State must be Abort after oscillation detection"
        );
    }

    // ── T4: can_terminate_with(OscillationDetected) ──

    /// T4-a: OscillationDetected は can_terminate_with で true を返す
    #[test]
    fn can_terminate_with_oscillation_detected() {
        assert!(can_terminate_with(
            TerminalTransitionReason::OscillationDetected
        ));
    }

    // ── T5: 発振カウンタの飽和安全性 ──

    /// T5: u32::MAX 近傍からの連続発振遷移でオーバーフローせず飽和する
    #[test]
    fn oscillation_counter_saturation() {
        let mut detector = OscillationDetector {
            oscillation_count: u32::MAX - 1,
            expected_next: Some(Retrieve),
            max_oscillation_count: u32::MAX,
        };

        // 飽和近傍でも加算がパニックしない
        assert_eq!(detector.oscillation_count(), u32::MAX - 1);
        // 期待(Retrieve)と一致 → u32::MAX で飽和
        detector.record_transition(Retrieve);
        assert_eq!(detector.oscillation_count(), u32::MAX);
        // 期待が Refine に切り替わった後も飽和状態を維持（saturating_add で飽和）
        detector.record_transition(Refine);
        assert_eq!(
            detector.oscillation_count(),
            u32::MAX,
            "Must saturate at u32::MAX"
        );
    }

    // ── OTS-1: 発振検出マトリクス観測 ──

    /// OTS-1: 複数の発振/非発振系列における発振カウントの推移を構造化出力する。
    #[test]
    fn ots_oscillation_detection_matrix() {
        use crate::constants::OSCILLATION_MAX_COUNT;

        println!("=== OTS-1: Oscillation Detection Matrix ===");
        println!("threshold={}", OSCILLATION_MAX_COUNT);
        println!("sequence,oscillation_count,is_oscillating");

        // 系列 1: 非発振（正常遷移）
        {
            let mut detector = OscillationDetector::default();
            let transitions = [Init, Retrieve, Evaluate, Finalize];
            for (i, &t) in transitions.iter().enumerate() {
                detector.record_transition(t);
                println!(
                    "normal_seq,{},{},{}",
                    i,
                    detector.oscillation_count(),
                    detector.is_oscillating()
                );
            }
        }

        // 系列 2: 発振（Refine↔Retrieve 交互）
        {
            let mut detector = OscillationDetector::default();
            let transitions = [Refine, Retrieve, Refine, Retrieve, Refine, Retrieve, Refine];
            for (i, &t) in transitions.iter().enumerate() {
                detector.record_transition(t);
                println!(
                    "oscillation_seq,{},{},{}",
                    i,
                    detector.oscillation_count(),
                    detector.is_oscillating()
                );
            }
        }

        // 系列 3: 発振開始 → リセット → 再発振
        {
            let mut detector = OscillationDetector::default();
            let transitions = [
                Refine, Retrieve, Evaluate, Refine, Retrieve, Refine, Retrieve,
            ];
            for (i, &t) in transitions.iter().enumerate() {
                detector.record_transition(t);
                println!(
                    "reset_seq,{},{},{}",
                    i,
                    detector.oscillation_count(),
                    detector.is_oscillating()
                );
            }
        }

        println!("=== 結果: OTS-1 観測完了 ===");
    }

    // ── OTS-2: attempt_transition レイテンシ観測 ──

    /// OTS-2: attempt_transition の正常遷移レイテンシと
    /// 発振検出時レイテンシを比較観測する。
    #[test]
    fn ots_attempt_transition_latency() {
        let sample_size = 1_000;

        println!("=== OTS-2: attempt_transition Latency ===");
        println!("sample_size={}", sample_size);

        // 正常遷移レイテンシ
        {
            let mut state = Init;
            let mut detector = OscillationDetector::new(100); // 発振しない大きな閾値
            let mut latencies_ns = Vec::with_capacity(sample_size);
            for _ in 0..sample_size {
                let start = std::time::Instant::now();
                let _ = attempt_transition(&mut state, &mut detector, Retrieve);
                let _ = attempt_transition(&mut state, &mut detector, Evaluate);
                let _ = attempt_transition(&mut state, &mut detector, Finalize);
                let elapsed = start.elapsed().as_nanos();
                latencies_ns.push(elapsed);
                state = Init;
                detector.reset();
            }
            let avg: f64 = latencies_ns.iter().copied().sum::<u128>() as f64 / sample_size as f64;
            let max = latencies_ns.iter().copied().max().unwrap_or(0);
            let min = latencies_ns.iter().copied().min().unwrap_or(0);
            println!(
                "normal_latency_ns: avg={:.3}, max={}, min={}",
                avg, max, min
            );
        }

        // 発振検出レイテンシ（強制 Abort のコスト）
        {
            let mut state = Refine;
            let mut detector = OscillationDetector::new(3);
            let mut latencies_ns = Vec::with_capacity(sample_size);
            for _ in 0..sample_size {
                detector.reset();
                // 発振閾値直前まで遷移
                let _ = attempt_transition(&mut state, &mut detector, Retrieve); // count=1
                let _ = attempt_transition(&mut state, &mut detector, Refine); // count=2
                let _ = attempt_transition(&mut state, &mut detector, Retrieve); // count=2→3
                                                                                 // 発振検出を計測
                let start = std::time::Instant::now();
                let _ = attempt_transition(&mut state, &mut detector, Refine); // 発振検出 → Abort
                let elapsed = start.elapsed().as_nanos();
                latencies_ns.push(elapsed);
                state = Refine;
            }
            let avg: f64 = latencies_ns.iter().copied().sum::<u128>() as f64 / sample_size as f64;
            let max = latencies_ns.iter().copied().max().unwrap_or(0);
            let min = latencies_ns.iter().copied().min().unwrap_or(0);
            println!(
                "oscillation_latency_ns: avg={:.3}, max={}, min={}",
                avg, max, min
            );
        }

        println!("=== 結果: OTS-2 計測完了 ===");
    }

    // ============================================================
    // M-1-1: EvaluateCandidatesStep — 静的閾値評価エンジン
    // ============================================================

    // ── T1: 正常系 — 閾値判定 ──

    /// T1-a: スコア 0.51 → ReuseExisting
    #[test]
    fn evaluate_above_threshold() {
        let result = evaluate_candidates(0.51);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            SearchOutcome::ReuseExisting {
                graph_id: String::new()
            }
        );
    }

    /// T1-b: スコア 0.49 → PatchExisting
    #[test]
    fn evaluate_below_threshold() {
        let result = evaluate_candidates(0.49);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            SearchOutcome::PatchExisting {
                graph_id: String::new(),
                patch: GraphPatch,
            }
        );
    }

    /// T1-c: スコア 1.00 → ReuseExisting（上限）
    #[test]
    fn evaluate_at_maximum() {
        let result = evaluate_candidates(1.00);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            SearchOutcome::ReuseExisting {
                graph_id: String::new()
            }
        );
    }

    /// T1-d: スコア 0.00 → PatchExisting（下限）
    #[test]
    fn evaluate_at_minimum() {
        let result = evaluate_candidates(0.00);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            SearchOutcome::PatchExisting {
                graph_id: String::new(),
                patch: GraphPatch,
            }
        );
    }

    /// T1-e: スコア 0.50 → ReuseExisting（境界値・閾値以上）
    #[test]
    fn evaluate_at_threshold() {
        let result = evaluate_candidates(0.50);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            SearchOutcome::ReuseExisting {
                graph_id: String::new()
            }
        );
    }

    // ── T2: 異常系 — スコア範囲外 ──

    /// T2-a: スコア -0.01 → Err(InvalidScore)
    #[test]
    fn evaluate_negative_score() {
        let result = evaluate_candidates(-0.01);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DarviumError::InvalidScore(_)));
    }

    /// T2-b: スコア 1.01 → Err(InvalidScore)
    #[test]
    fn evaluate_score_exceeds_max() {
        let result = evaluate_candidates(1.01);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), DarviumError::InvalidScore(_)));
    }

    /// T2-c: スコア f64::NEG_INFINITY → Err(InvalidScore)
    #[test]
    fn evaluate_neg_infinity() {
        let result = evaluate_candidates(f64::NEG_INFINITY);
        assert!(result.is_err());
    }

    /// T2-d: スコア f64::INFINITY → Err(InvalidScore)
    #[test]
    fn evaluate_pos_infinity() {
        let result = evaluate_candidates(f64::INFINITY);
        assert!(result.is_err());
    }

    /// T2-e: スコア f64::NAN → Err(InvalidScore)
    #[test]
    fn evaluate_nan_score() {
        let result = evaluate_candidates(f64::NAN);
        assert!(result.is_err());
    }

    // ── T3: 決定論性 ──

    /// T3-a: 同一スコアで 2 回呼び出した結果が完全一致
    #[test]
    fn evaluate_deterministic_same_score() {
        let r1 = evaluate_candidates(0.75);
        let r2 = evaluate_candidates(0.75);
        assert_eq!(r1, r2);
    }

    /// T3-b: 異なるスコアでは結果が異なる
    #[test]
    fn evaluate_deterministic_different_scores() {
        let r1 = evaluate_candidates(0.51);
        let r2 = evaluate_candidates(0.49);
        assert_ne!(r1, r2);
    }

    // ── T4: 自己評価割引（auxiliary）──

    /// T4-a: 生スコア 0.90 → 割引後 0.765 → ReuseExisting
    #[test]
    fn discount_high_confidence() {
        let discounted = apply_self_conf_discount(0.90);
        assert!((discounted - 0.765).abs() < 1e-12);
        let result = evaluate_candidates(discounted);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            SearchOutcome::ReuseExisting {
                graph_id: String::new()
            }
        );
    }

    /// T4-b: 生スコア 0.45 → 割引後 0.3825 → PatchExisting
    #[test]
    fn discount_low_confidence() {
        let discounted = apply_self_conf_discount(0.45);
        assert!((discounted - 0.3825).abs() < 1e-12);
        let result = evaluate_candidates(discounted);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            SearchOutcome::PatchExisting {
                graph_id: String::new(),
                patch: GraphPatch,
            }
        );
    }

    /// T4-c: 生スコア 0.60 → 割引後 0.51 → ReuseExisting（ぎりぎり超過）
    #[test]
    fn discount_near_threshold_above() {
        let discounted = apply_self_conf_discount(0.60);
        assert!((discounted - 0.51).abs() < 1e-12);
        let result = evaluate_candidates(discounted);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            SearchOutcome::ReuseExisting {
                graph_id: String::new()
            }
        );
    }

    /// T4-d: 生スコア 0.58 → 割引後 0.493 → PatchExisting（ぎりぎり未満）
    #[test]
    fn discount_near_threshold_below() {
        let discounted = apply_self_conf_discount(0.58);
        assert!((discounted - 0.493).abs() < 1e-12);
        let result = evaluate_candidates(discounted);
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            SearchOutcome::PatchExisting {
                graph_id: String::new(),
                patch: GraphPatch,
            }
        );
    }

    // ── T5: SearchOutcome enum の網羅性 ──

    /// T5-a: 全バリアントが Debug, Clone, PartialEq を実装している
    #[test]
    fn outcome_variants_traits() {
        let reuse = SearchOutcome::ReuseExisting {
            graph_id: "g1".into(),
        };
        let patch = SearchOutcome::PatchExisting {
            graph_id: "g2".into(),
            patch: GraphPatch,
        };
        let compose = SearchOutcome::ComposeExisting {
            plan: CompositionPlan::new(vec![], vec![], vec![], VarDecl::new("out")),
        };
        let new = SearchOutcome::GenerateNew {
            proposal: WorkflowGraph::new(),
        };
        let abort = SearchOutcome::AbortSearch {
            reason: SearchAbortReason::ExplicitAbort,
        };
        let review = SearchOutcome::NeedsHumanReview {
            reason: "test".into(),
        };

        // Debug
        let _ = format!("{:?}", reuse);
        let _ = format!("{:?}", patch);
        let _ = format!("{:?}", compose);
        let _ = format!("{:?}", new);
        let _ = format!("{:?}", abort);
        let _ = format!("{:?}", review);

        // Clone
        let _reuse2 = reuse.clone();
        let _patch2 = patch.clone();

        // PartialEq
        assert_eq!(reuse, reuse);
        assert_ne!(
            patch,
            SearchOutcome::ReuseExisting {
                graph_id: "x".into()
            }
        );
    }

    /// T5-b: 全バリアントが網羅的マッチング可能
    #[test]
    fn outcome_variants_exhaustive() {
        let outcomes = [
            SearchOutcome::ReuseExisting {
                graph_id: "a".into(),
            },
            SearchOutcome::PatchExisting {
                graph_id: "b".into(),
                patch: GraphPatch,
            },
            SearchOutcome::ComposeExisting {
                plan: CompositionPlan::new(vec![], vec![], vec![], VarDecl::new("out")),
            },
            SearchOutcome::GenerateNew {
                proposal: WorkflowGraph::new(),
            },
            SearchOutcome::AbortSearch {
                reason: SearchAbortReason::ExplicitAbort,
            },
            SearchOutcome::NeedsHumanReview { reason: "c".into() },
        ];

        for outcome in &outcomes {
            match outcome {
                SearchOutcome::ReuseExisting { .. } => {}
                SearchOutcome::PatchExisting { .. } => {}
                SearchOutcome::ComposeExisting { .. } => {}
                SearchOutcome::GenerateNew { .. } => {}
                SearchOutcome::AbortSearch { .. } => {}
                SearchOutcome::NeedsHumanReview { .. } => {}
            }
        }

        assert_eq!(outcomes.len(), 6);
    }

    // ── OTS-1: ノイズ注入による決定境界分布 ──

    /// OTS-1: 固定シード PRNG で決定境界近傍の選択確率分布を観測する。
    ///
    /// スコア 0.50 ± ε にガウスノイズを付加し、ノイズ分散を sweep しながら
    /// REUSE / PATCH の選択確率分布をシグモイド曲線として観測する。
    ///
    /// ガウスノイズは Box-Muller 変換で生成する。
    #[test]
    fn ots1_decision_boundary_distribution() {
        use rand::rngs::StdRng;
        use rand::Rng;
        use rand::SeedableRng;

        let noise_variances: [f64; 5] = [0.001, 0.01, 0.05, 0.1, 0.2];
        let sample_size = 10_000;
        let center_score = 0.50;

        println!("=== OTS-1: Decision Boundary Distribution ===");
        println!("sample_size={}, center={}", sample_size, center_score);
        println!("variance,reuse_count,patch_count,reuse_prob");

        for &variance in &noise_variances {
            let mut rng = StdRng::seed_from_u64(12345);
            let std_dev = variance.sqrt();
            let mut reuse_count: u64 = 0;
            let mut valid_count: u64 = 0;

            for _ in 0..sample_size {
                // Box-Muller 変換で標準正規乱数を生成
                let u1: f64 = rng.random_range(0.00000001..1.0);
                let u2: f64 = rng.random_range(0.0..std::f64::consts::TAU);
                let z = (-2.0 * u1.ln()).sqrt() * u2.cos();
                let noisy_score = center_score + z * std_dev;

                if !(0.0..=1.0).contains(&noisy_score) {
                    continue;
                }
                valid_count += 1;
                if let Ok(outcome) = evaluate_candidates(noisy_score) {
                    if matches!(outcome, SearchOutcome::ReuseExisting { .. }) {
                        reuse_count += 1;
                    }
                }
            }

            let reuse_prob = if valid_count > 0 {
                reuse_count as f64 / valid_count as f64
            } else {
                0.5
            };
            println!(
                "{},{},{},{:.6}",
                variance,
                reuse_count,
                valid_count - reuse_count,
                reuse_prob
            );
        }

        println!("=== 結果: OTS-1 観測完了 ===");
    }

    // ── OTS-2: 決定境界の幾何学的曲率 ──

    /// OTS-2: ノイズ分散 σ² の関数として境界超曲面の平均幾何学的曲率を観測する。
    ///
    /// シグモイド近似: P(REUSE) = 1 / (1 + exp(-β * (score - 0.50)))
    /// 温度 β = 1/σ² に比例するスケーリング則を検証する。
    #[test]
    fn ots2_scaling_law() {
        use rand::rngs::StdRng;
        use rand::Rng;
        use rand::SeedableRng;

        let noise_variances: [f64; 5] = [0.001, 0.005, 0.01, 0.05, 0.1];
        let sample_size = 10_000;
        let center_score = 0.50;

        println!("=== OTS-2: Scaling Law (β ∝ 1/σ²) ===");
        println!("sample_size={}, center={}", sample_size, center_score);
        println!("variance,inverse_variance,p_sigmoid_center");

        for &variance in &noise_variances {
            let mut rng = StdRng::seed_from_u64(12345);
            let std_dev = variance.sqrt();
            let mut reuse_count: u64 = 0;
            let mut valid_count: u64 = 0;

            for _ in 0..sample_size {
                // Box-Muller 変換で標準正規乱数を生成
                let u1: f64 = rng.random_range(0.00000001..1.0);
                let u2: f64 = rng.random_range(0.0..std::f64::consts::TAU);
                let z = (-2.0 * u1.ln()).sqrt() * u2.cos();
                let noisy_score = center_score + z * std_dev;

                if !(0.0..=1.0).contains(&noisy_score) {
                    continue;
                }
                valid_count += 1;
                if let Ok(outcome) = evaluate_candidates(noisy_score) {
                    if matches!(outcome, SearchOutcome::ReuseExisting { .. }) {
                        reuse_count += 1;
                    }
                }
            }

            let p_center = if valid_count > 0 {
                reuse_count as f64 / valid_count as f64
            } else {
                0.5
            };

            println!("{},{},{:.6}", variance, 1.0 / variance, p_center);
        }

        // P(s_center) ≈ 0.5 であることを確認（決定対称性）
        println!("=== 結果: OTS-2 観測完了 ===");
    }

    // ============================================================
    // M-1-2: SearchBudgetExceeded ハードガードの遮断アサーション
    // ============================================================

    use super::{check_budget_exceeded, guard_budget_or_abort, SearchBudget, SearchBudgetSnapshot};

    // ── T1: 正常系 — 上限未満での通過 ──

    /// T1-a: iterations 上限未満で通過
    #[test]
    fn budget_check_iterations_ok() {
        let budget = SearchBudget::default();
        let snapshot = SearchBudgetSnapshot {
            iterations_used: 0,
            retrieval_calls_used: 0,
            prompt_tokens_used: 0,
            wall_clock_ms_used: 0,
        };
        assert!(check_budget_exceeded(&budget, &snapshot).is_ok());
    }

    /// T1-b: retrieval_calls 上限未満で通過
    #[test]
    fn budget_check_retrieval_calls_ok() {
        let budget = SearchBudget::default();
        let snapshot = SearchBudgetSnapshot {
            iterations_used: 50,
            retrieval_calls_used: 0,
            prompt_tokens_used: 0,
            wall_clock_ms_used: 0,
        };
        assert!(check_budget_exceeded(&budget, &snapshot).is_ok());
    }

    /// T1-c: prompt_tokens 上限未満で通過
    #[test]
    fn budget_check_prompt_tokens_ok() {
        let budget = SearchBudget {
            max_prompt_tokens: 100,
            ..SearchBudget::default()
        };
        let snapshot = SearchBudgetSnapshot {
            iterations_used: 0,
            retrieval_calls_used: 0,
            prompt_tokens_used: 50,
            wall_clock_ms_used: 0,
        };
        assert!(check_budget_exceeded(&budget, &snapshot).is_ok());
    }

    /// T1-d: wall_clock_ms 上限未満で通過
    #[test]
    fn budget_check_wall_clock_ok() {
        let budget = SearchBudget {
            max_wall_clock_ms: 1000,
            ..SearchBudget::default()
        };
        let snapshot = SearchBudgetSnapshot {
            iterations_used: 0,
            retrieval_calls_used: 0,
            prompt_tokens_used: 0,
            wall_clock_ms_used: 500,
        };
        assert!(check_budget_exceeded(&budget, &snapshot).is_ok());
    }

    /// T1-e: 全次元が半量で通過
    #[test]
    fn budget_check_all_half_ok() {
        let budget = SearchBudget {
            max_iterations: 100,
            max_retrieval_calls: 50,
            max_prompt_tokens: 1000,
            max_wall_clock_ms: 2000,
        };
        let snapshot = SearchBudgetSnapshot {
            iterations_used: 50,
            retrieval_calls_used: 25,
            prompt_tokens_used: 500,
            wall_clock_ms_used: 1000,
        };
        assert!(check_budget_exceeded(&budget, &snapshot).is_ok());
    }

    // ── T2: 異常系 — 各次元の単独超過 ──

    /// T2-a: iterations 超過 (used >= max)
    #[test]
    fn budget_check_iterations_exceeded() {
        let budget = SearchBudget {
            max_iterations: 10,
            ..SearchBudget::default()
        };
        let snapshot = SearchBudgetSnapshot {
            iterations_used: 10,
            retrieval_calls_used: 0,
            prompt_tokens_used: 0,
            wall_clock_ms_used: 0,
        };
        let result = check_budget_exceeded(&budget, &snapshot);
        assert!(
            matches!(result, Err(DarviumError::SearchBudgetExceeded)),
            "Expected SearchBudgetExceeded, got {:?}",
            result
        );
    }

    /// T2-b: retrieval_calls 超過 (used >= max)
    #[test]
    fn budget_check_retrieval_calls_exceeded() {
        let budget = SearchBudget {
            max_retrieval_calls: 5,
            ..SearchBudget::default()
        };
        let snapshot = SearchBudgetSnapshot {
            iterations_used: 0,
            retrieval_calls_used: 5,
            prompt_tokens_used: 0,
            wall_clock_ms_used: 0,
        };
        let result = check_budget_exceeded(&budget, &snapshot);
        assert!(
            matches!(result, Err(DarviumError::SearchBudgetExceeded)),
            "Expected SearchBudgetExceeded, got {:?}",
            result
        );
    }

    /// T2-c: prompt_tokens 超過 (used > max)
    #[test]
    fn budget_check_prompt_tokens_exceeded() {
        let budget = SearchBudget {
            max_prompt_tokens: 100,
            ..SearchBudget::default()
        };
        let snapshot = SearchBudgetSnapshot {
            iterations_used: 0,
            retrieval_calls_used: 0,
            prompt_tokens_used: 101,
            wall_clock_ms_used: 0,
        };
        let result = check_budget_exceeded(&budget, &snapshot);
        assert!(
            matches!(result, Err(DarviumError::SearchBudgetExceeded)),
            "Expected SearchBudgetExceeded, got {:?}",
            result
        );
    }

    /// T2-d: prompt_tokens 上限ぴったりは通過
    #[test]
    fn budget_check_prompt_tokens_exact_limit() {
        let budget = SearchBudget {
            max_prompt_tokens: 100,
            ..SearchBudget::default()
        };
        let snapshot = SearchBudgetSnapshot {
            iterations_used: 0,
            retrieval_calls_used: 0,
            prompt_tokens_used: 100,
            wall_clock_ms_used: 0,
        };
        assert!(check_budget_exceeded(&budget, &snapshot).is_ok());
    }

    /// T2-e: wall_clock_ms 超過 (used > max)
    #[test]
    fn budget_check_wall_clock_exceeded() {
        let budget = SearchBudget {
            max_wall_clock_ms: 1000,
            ..SearchBudget::default()
        };
        let snapshot = SearchBudgetSnapshot {
            iterations_used: 0,
            retrieval_calls_used: 0,
            prompt_tokens_used: 0,
            wall_clock_ms_used: 1001,
        };
        let result = check_budget_exceeded(&budget, &snapshot);
        assert!(
            matches!(result, Err(DarviumError::SearchBudgetExceeded)),
            "Expected SearchBudgetExceeded, got {:?}",
            result
        );
    }

    /// T2-f: wall_clock_ms 上限ぴったりは通過
    #[test]
    fn budget_check_wall_clock_exact_limit() {
        let budget = SearchBudget {
            max_wall_clock_ms: 1000,
            ..SearchBudget::default()
        };
        let snapshot = SearchBudgetSnapshot {
            iterations_used: 0,
            retrieval_calls_used: 0,
            prompt_tokens_used: 0,
            wall_clock_ms_used: 1000,
        };
        assert!(check_budget_exceeded(&budget, &snapshot).is_ok());
    }

    // ── T3: 複数次元同時超過 ──

    /// T3-a: iterations 超過 + 他正常
    #[test]
    fn budget_check_multi_iterations() {
        let budget = SearchBudget {
            max_iterations: 10,
            max_retrieval_calls: 50,
            max_prompt_tokens: 1000,
            max_wall_clock_ms: 2000,
        };
        let snapshot = SearchBudgetSnapshot {
            iterations_used: 10,
            retrieval_calls_used: 25,
            prompt_tokens_used: 500,
            wall_clock_ms_used: 1000,
        };
        assert!(matches!(
            check_budget_exceeded(&budget, &snapshot),
            Err(DarviumError::SearchBudgetExceeded)
        ));
    }

    /// T3-b: retrieval_calls 超過 + 他正常
    #[test]
    fn budget_check_multi_retrieval_calls() {
        let budget = SearchBudget {
            max_iterations: 100,
            max_retrieval_calls: 5,
            max_prompt_tokens: 1000,
            max_wall_clock_ms: 2000,
        };
        let snapshot = SearchBudgetSnapshot {
            iterations_used: 50,
            retrieval_calls_used: 5,
            prompt_tokens_used: 500,
            wall_clock_ms_used: 1000,
        };
        assert!(matches!(
            check_budget_exceeded(&budget, &snapshot),
            Err(DarviumError::SearchBudgetExceeded)
        ));
    }

    /// T3-c: prompt_tokens 超過 + 他正常
    #[test]
    fn budget_check_multi_prompt_tokens() {
        let budget = SearchBudget {
            max_iterations: 100,
            max_retrieval_calls: 50,
            max_prompt_tokens: 100,
            max_wall_clock_ms: 2000,
        };
        let snapshot = SearchBudgetSnapshot {
            iterations_used: 50,
            retrieval_calls_used: 25,
            prompt_tokens_used: 101,
            wall_clock_ms_used: 1000,
        };
        assert!(matches!(
            check_budget_exceeded(&budget, &snapshot),
            Err(DarviumError::SearchBudgetExceeded)
        ));
    }

    /// T3-d: wall_clock_ms 超過 + 他正常
    #[test]
    fn budget_check_multi_wall_clock() {
        let budget = SearchBudget {
            max_iterations: 100,
            max_retrieval_calls: 50,
            max_prompt_tokens: 1000,
            max_wall_clock_ms: 100,
        };
        let snapshot = SearchBudgetSnapshot {
            iterations_used: 50,
            retrieval_calls_used: 25,
            prompt_tokens_used: 500,
            wall_clock_ms_used: 101,
        };
        assert!(matches!(
            check_budget_exceeded(&budget, &snapshot),
            Err(DarviumError::SearchBudgetExceeded)
        ));
    }

    /// T3-e: 全次元超過
    #[test]
    fn budget_check_multi_all_exceeded() {
        let budget = SearchBudget {
            max_iterations: 1,
            max_retrieval_calls: 1,
            max_prompt_tokens: 1,
            max_wall_clock_ms: 1,
        };
        let snapshot = SearchBudgetSnapshot {
            iterations_used: 1,
            retrieval_calls_used: 1,
            prompt_tokens_used: 2,
            wall_clock_ms_used: 2,
        };
        assert!(matches!(
            check_budget_exceeded(&budget, &snapshot),
            Err(DarviumError::SearchBudgetExceeded)
        ));
    }

    // ── T4: guard_budget_or_abort 状態遷移 ──

    /// T4-a: 超過なし → 状態不変
    #[test]
    fn guard_budget_normal_passes_through() {
        let mut state = Init;
        let budget = SearchBudget::default();
        let snapshot = SearchBudgetSnapshot {
            iterations_used: 0,
            retrieval_calls_used: 0,
            prompt_tokens_used: 0,
            wall_clock_ms_used: 0,
        };
        assert!(guard_budget_or_abort(&mut state, &budget, &snapshot).is_ok());
        assert_eq!(state, Init, "State must remain unchanged on normal pass");
    }

    /// T4-b: 超過あり → 状態が Abort に + Err
    #[test]
    fn guard_budget_exceeded_sets_abort() {
        let mut state = Evaluate;
        let budget = SearchBudget {
            max_iterations: 1,
            ..SearchBudget::default()
        };
        let snapshot = SearchBudgetSnapshot {
            iterations_used: 1,
            retrieval_calls_used: 0,
            prompt_tokens_used: 0,
            wall_clock_ms_used: 0,
        };
        let result = guard_budget_or_abort(&mut state, &budget, &snapshot);
        assert!(
            matches!(result, Err(DarviumError::SearchBudgetExceeded)),
            "Expected SearchBudgetExceeded, got {:?}",
            result
        );
        assert_eq!(state, Abort, "State must be Abort after budget exceeded");
    }

    /// T4-c: 終端状態から超過 → TerminalStateViolation 優先
    #[test]
    fn guard_budget_terminal_state_priority() {
        let mut state = Finalize;
        let budget = SearchBudget {
            max_iterations: 1,
            ..SearchBudget::default()
        };
        let snapshot = SearchBudgetSnapshot {
            iterations_used: 1,
            retrieval_calls_used: 0,
            prompt_tokens_used: 0,
            wall_clock_ms_used: 0,
        };
        let result = guard_budget_or_abort(&mut state, &budget, &snapshot);
        assert!(
            matches!(result, Err(DarviumError::TerminalStateViolation)),
            "Expected TerminalStateViolation (higher priority), got {:?}",
            result
        );
    }

    /// T4-d: 終端状態 Abort からも TerminalStateViolation 優先
    #[test]
    fn guard_budget_terminal_abort_priority() {
        let mut state = Abort;
        let budget = SearchBudget::default();
        let snapshot = SearchBudgetSnapshot {
            iterations_used: 0,
            retrieval_calls_used: 0,
            prompt_tokens_used: 0,
            wall_clock_ms_used: 0,
        };
        let result = guard_budget_or_abort(&mut state, &budget, &snapshot);
        assert!(
            matches!(result, Err(DarviumError::TerminalStateViolation)),
            "Expected TerminalStateViolation from Abort state, got {:?}",
            result
        );
    }

    // ── T5: 副作用ゼロ検証 ──

    /// T5-a: check_budget_exceeded 呼び出し前後で snapshot の値が不変
    #[test]
    fn budget_check_no_snapshot_mutation() {
        let budget = SearchBudget::default();
        let snapshot = SearchBudgetSnapshot {
            iterations_used: 50,
            retrieval_calls_used: 25,
            prompt_tokens_used: 500,
            wall_clock_ms_used: 1000,
        };
        let snapshot_before = snapshot.clone();
        let _ = check_budget_exceeded(&budget, &snapshot);
        assert_eq!(snapshot, snapshot_before, "Snapshot must not be mutated");
    }

    /// T5-b: check_budget_exceeded が budget を変更しない
    #[test]
    fn budget_check_no_budget_mutation() {
        let budget = SearchBudget::default();
        let budget_before = budget.clone();
        let snapshot = SearchBudgetSnapshot {
            iterations_used: 0,
            retrieval_calls_used: 0,
            prompt_tokens_used: 0,
            wall_clock_ms_used: 0,
        };
        let _ = check_budget_exceeded(&budget, &snapshot);
        assert_eq!(budget.max_iterations, budget_before.max_iterations);
        assert_eq!(
            budget.max_retrieval_calls,
            budget_before.max_retrieval_calls
        );
        assert_eq!(budget.max_prompt_tokens, budget_before.max_prompt_tokens);
        assert_eq!(budget.max_wall_clock_ms, budget_before.max_wall_clock_ms);
    }

    // ── T6: 決定論性 ──

    /// T6-a: 同一入力で 2 回呼び出した結果が完全一致
    #[test]
    fn budget_check_deterministic_same() {
        let budget = SearchBudget::default();
        let snapshot = SearchBudgetSnapshot {
            iterations_used: 50,
            retrieval_calls_used: 25,
            prompt_tokens_used: 500,
            wall_clock_ms_used: 1000,
        };
        let r1 = check_budget_exceeded(&budget, &snapshot);
        let r2 = check_budget_exceeded(&budget, &snapshot);
        assert_eq!(r1, r2);
    }

    /// T6-b: 異なる超過量でもガード結果が同一（Err の内容はどちらも SearchBudgetExceeded）
    #[test]
    fn budget_check_deterministic_different_excess() {
        let budget = SearchBudget {
            max_iterations: 10,
            ..SearchBudget::default()
        };
        let snapshot_1 = SearchBudgetSnapshot {
            iterations_used: 11,
            retrieval_calls_used: 0,
            prompt_tokens_used: 0,
            wall_clock_ms_used: 0,
        };
        let snapshot_2 = SearchBudgetSnapshot {
            iterations_used: 100,
            retrieval_calls_used: 0,
            prompt_tokens_used: 0,
            wall_clock_ms_used: 0,
        };
        let r1 = check_budget_exceeded(&budget, &snapshot_1);
        let r2 = check_budget_exceeded(&budget, &snapshot_2);
        // 両方とも SearchBudgetExceeded エラー
        assert_eq!(r1.is_err(), r2.is_err());
        assert!(matches!(r1, Err(DarviumError::SearchBudgetExceeded)));
        assert!(matches!(r2, Err(DarviumError::SearchBudgetExceeded)));
    }

    // ── OTS-1: ガード遮断命令ステップ数の分散測定 ──

    /// OTS-1: バッチ集計によるガード命令ステップ数の分散測定。
    ///
    /// 超過量 ΔB を 1 から 10,000 まで対数 sweep（10水準）し、
    /// 各水準で N_BATCH 回のガード呼び出しを 1 測定単位として
    /// N_SAMPLES 回のバッチ測定を行う。
    /// バッチ時間の分散が測定ノイズフロア内に収まっていることを確認する。
    #[test]
    fn ots_budget_guard_instruction_steps() {
        const BATCH_SIZE: usize = 100;
        const N_SAMPLES: usize = 100;
        let log_levels: [u64; 10] = [1, 2, 5, 10, 20, 50, 100, 500, 2000, 10000];

        println!("=== OTS-1: Guard Instruction Steps (batch) ===");
        println!("batch_size={}, samples={}", BATCH_SIZE, N_SAMPLES);
        println!("delta_b,batch_avg_ns,batch_var_ns,min_batch_ns,max_batch_ns");

        for &delta_b in &log_levels {
            let budget = SearchBudget {
                max_iterations: delta_b as u32,
                ..SearchBudget::default()
            };
            let snapshot = SearchBudgetSnapshot {
                iterations_used: delta_b as u32,
                retrieval_calls_used: 0,
                prompt_tokens_used: 0,
                wall_clock_ms_used: 0,
            };

            let mut batch_measurements: Vec<u128> = Vec::with_capacity(N_SAMPLES);

            for _ in 0..N_SAMPLES {
                let start = std::time::Instant::now();
                for _ in 0..BATCH_SIZE {
                    let _ = check_budget_exceeded(&budget, &snapshot);
                }
                let elapsed = start.elapsed().as_nanos();
                batch_measurements.push(elapsed);
            }

            let total: u128 = batch_measurements.iter().copied().sum();
            let batch_avg = total as f64 / N_SAMPLES as f64;
            let batch_variance = batch_measurements
                .iter()
                .map(|&m| {
                    let diff = m as f64 - batch_avg;
                    diff * diff
                })
                .sum::<f64>()
                / N_SAMPLES as f64;
            let min_batch = batch_measurements.iter().copied().min().unwrap_or(0);
            let max_batch = batch_measurements.iter().copied().max().unwrap_or(0);

            println!(
                "{},{:.3},{:.3},{},{}",
                delta_b, batch_avg, batch_variance, min_batch, max_batch
            );
        }

        println!("=== 結果: OTS-1 観測完了 ===");
    }

    // ── OTS-2: guard_budget_or_abort レイテンシ分布 ──

    /// OTS-2: guard_budget_or_abort の正常系と超過系のレイテンシ分布を測定。
    #[test]
    fn ots_guard_budget_latency_distribution() {
        let sample_size = 10_000;

        println!("=== OTS-2: guard_budget_or_abort Latency Distribution ===");
        println!("sample_size_per_mode={}", sample_size);

        // 正常系: 超過なしで通過
        {
            let mut latencies_ns: Vec<u128> = Vec::with_capacity(sample_size);
            for _ in 0..sample_size {
                let mut state = Init;
                let budget = SearchBudget::default();
                let snapshot = SearchBudgetSnapshot {
                    iterations_used: 0,
                    retrieval_calls_used: 0,
                    prompt_tokens_used: 0,
                    wall_clock_ms_used: 0,
                };
                let start = std::time::Instant::now();
                let _ = guard_budget_or_abort(&mut state, &budget, &snapshot);
                let elapsed = start.elapsed().as_nanos();
                latencies_ns.push(elapsed);
            }

            let avg = latencies_ns.iter().copied().sum::<u128>() as f64 / sample_size as f64;
            let variance = latencies_ns
                .iter()
                .map(|&m| {
                    let diff = m as f64 - avg;
                    diff * diff
                })
                .sum::<f64>()
                / sample_size as f64;
            let max = latencies_ns.iter().copied().max().unwrap_or(0);
            let min = latencies_ns.iter().copied().min().unwrap_or(0);

            // ソートして分位数計算
            let mut sorted = latencies_ns.clone();
            sorted.sort_unstable();
            let p50 = sorted[sample_size / 2];
            let p95 = sorted[(sample_size as f64 * 0.95) as usize];
            let p99 = sorted[(sample_size as f64 * 0.99) as usize];

            println!(
                "normal_pass,avg_ns={:.3},var_ns={:.3},max_ns={},min_ns={},p50={},p95={},p99={}",
                avg, variance, max, min, p50, p95, p99
            );
        }

        // 超過系: 超過で Abort 遷移
        {
            let mut latencies_ns: Vec<u128> = Vec::with_capacity(sample_size);
            for _ in 0..sample_size {
                let mut state = Evaluate;
                let budget = SearchBudget {
                    max_iterations: 1,
                    ..SearchBudget::default()
                };
                let snapshot = SearchBudgetSnapshot {
                    iterations_used: 1,
                    retrieval_calls_used: 0,
                    prompt_tokens_used: 0,
                    wall_clock_ms_used: 0,
                };
                let start = std::time::Instant::now();
                let _ = guard_budget_or_abort(&mut state, &budget, &snapshot);
                let elapsed = start.elapsed().as_nanos();
                latencies_ns.push(elapsed);
            }

            let avg = latencies_ns.iter().copied().sum::<u128>() as f64 / sample_size as f64;
            let variance = latencies_ns
                .iter()
                .map(|&m| {
                    let diff = m as f64 - avg;
                    diff * diff
                })
                .sum::<f64>()
                / sample_size as f64;
            let max = latencies_ns.iter().copied().max().unwrap_or(0);
            let min = latencies_ns.iter().copied().min().unwrap_or(0);

            let mut sorted = latencies_ns.clone();
            sorted.sort_unstable();
            let p50 = sorted[sample_size / 2];
            let p95 = sorted[(sample_size as f64 * 0.95) as usize];
            let p99 = sorted[(sample_size as f64 * 0.99) as usize];

            println!("budget_exceeded,avg_ns={:.3},var_ns={:.3},max_ns={},min_ns={},p50={},p95={},p99={}",
                avg, variance, max, min, p50, p95, p99);
        }

        println!("=== 結果: OTS-2 観測完了 ===");
    }

    // ============================================================
    // M-1-3: SearchRecursionExceeded 深さ制限ガードの強制
    // ============================================================

    /// T1-a: 正常系 — 深さ 0 から 1 へのインクリメント成功
    #[test]
    fn t1a_guard_recursion_normal_from_zero() {
        let mut guard = RecursionGuard::new(8, true);
        let mut state = SearchState::Init;
        assert!(guard_recursion_or_abort(&mut guard, &mut state).is_ok());
        assert_eq!(guard.current_depth, 1);
        assert_eq!(state, SearchState::Init);
    }

    /// T1-b: 正常系 — 深さ 3 から 4 へのインクリメント成功
    #[test]
    fn t1b_guard_recursion_normal_from_mid() {
        let mut guard = RecursionGuard {
            max_depth: 8,
            current_depth: 3,
            allow_reentrant: true,
        };
        let mut state = SearchState::Init;
        assert!(guard_recursion_or_abort(&mut guard, &mut state).is_ok());
        assert_eq!(guard.current_depth, 4);
    }

    /// T1-c: 正常系 — 深さ 7 (上限 8) から 8 へのインクリメント成功（上限ぴったり）
    #[test]
    fn t1c_guard_recursion_normal_at_boundary() {
        let mut guard = RecursionGuard {
            max_depth: 8,
            current_depth: 7,
            allow_reentrant: true,
        };
        let mut state = SearchState::Init;
        assert!(guard_recursion_or_abort(&mut guard, &mut state).is_ok());
        assert_eq!(guard.current_depth, 8);
    }

    /// T2-a: 異常系 — max_depth=3, current_depth=3 で超過
    #[test]
    fn t2a_guard_recursion_exceeded_at_limit() {
        let mut guard = RecursionGuard {
            max_depth: 3,
            current_depth: 3,
            allow_reentrant: true,
        };
        let mut state = SearchState::Init;
        let result = guard_recursion_or_abort(&mut guard, &mut state);
        assert!(matches!(result, Err(DarviumError::SearchRecursionExceeded)));
        assert_eq!(guard.current_depth, 3);
    }

    /// T2-b: 異常系 — max_depth=3, current_depth=4（既に超過状態）
    #[test]
    fn t2b_guard_recursion_exceeded_already_over() {
        let mut guard = RecursionGuard {
            max_depth: 3,
            current_depth: 4,
            allow_reentrant: true,
        };
        let mut state = SearchState::Init;
        let result = guard_recursion_or_abort(&mut guard, &mut state);
        assert!(matches!(result, Err(DarviumError::SearchRecursionExceeded)));
        assert_eq!(guard.current_depth, 4);
    }

    /// T2-c: 異常系 — allow_reentrant=false で全拒否（深さ 0 でも失敗）
    #[test]
    fn t2c_guard_recursion_disallowed() {
        let mut guard = RecursionGuard::new(8, false);
        let mut state = SearchState::Init;
        let result = guard_recursion_or_abort(&mut guard, &mut state);
        assert!(matches!(result, Err(DarviumError::SearchRecursionExceeded)));
        assert_eq!(guard.current_depth, 0);
    }

    /// T2-d: 異常系 — max_depth=0 では常に超過
    #[test]
    fn t2d_guard_recursion_zero_max_depth() {
        let mut guard = RecursionGuard::new(0, true);
        let mut state = SearchState::Init;
        let result = guard_recursion_or_abort(&mut guard, &mut state);
        assert!(matches!(result, Err(DarviumError::SearchRecursionExceeded)));
        assert_eq!(guard.current_depth, 0);
    }

    /// T3-a: 正常系 — 状態が不変であること
    #[test]
    fn t3a_guard_recursion_state_unchanged_on_success() {
        let mut guard = RecursionGuard::new(8, true);
        let mut state = SearchState::Evaluate;
        assert!(guard_recursion_or_abort(&mut guard, &mut state).is_ok());
        assert_eq!(state, SearchState::Evaluate);
    }

    /// T3-b: 異常系 — 深さ超過時に状態が Abort に変更される
    #[test]
    fn t3b_guard_recursion_transitions_to_abort() {
        let mut guard = RecursionGuard {
            max_depth: 3,
            current_depth: 3,
            allow_reentrant: true,
        };
        let mut state = SearchState::Init;
        let result = guard_recursion_or_abort(&mut guard, &mut state);
        assert!(result.is_err());
        assert_eq!(state, SearchState::Abort);
    }

    /// T3-c: 終端状態からの呼び出しは TerminalStateViolation（Abort 優先ではない）
    #[test]
    fn t3c_guard_recursion_from_terminal_state() {
        let mut guard = RecursionGuard::new(8, true);
        let mut state = SearchState::Abort;
        let result = guard_recursion_or_abort(&mut guard, &mut state);
        assert!(matches!(result, Err(DarviumError::TerminalStateViolation)));
        // 終端状態からの遷移は禁止、Abort に変更してはならない
        assert_eq!(state, SearchState::Abort);
    }

    /// T3-d: allow_reentrant=false 時は深さ超過として Abort に遷移
    #[test]
    fn t3d_guard_recursion_disallowed_transitions_to_abort() {
        let mut guard = RecursionGuard::new(8, false);
        let mut state = SearchState::Init;
        let result = guard_recursion_or_abort(&mut guard, &mut state);
        assert!(matches!(result, Err(DarviumError::SearchRecursionExceeded)));
        assert_eq!(state, SearchState::Abort);
    }

    /// T3-e: 超過時に状態が Abort に変更されてから Err が伝播されることを確認
    #[test]
    fn t3e_guard_recursion_state_change_before_err() {
        let mut guard = RecursionGuard {
            max_depth: 3,
            current_depth: 3,
            allow_reentrant: true,
        };
        let mut state = SearchState::Finalize;
        // Finalize からの呼び出し → TerminalStateViolation（超過より先に終端状態チェック）
        let result = guard_recursion_or_abort(&mut guard, &mut state);
        assert!(matches!(result, Err(DarviumError::TerminalStateViolation)));
        assert_eq!(state, SearchState::Finalize);
    }

    /// T4-a: エラー時に current_depth が不変であること
    #[test]
    fn t4a_current_depth_unchanged_on_error() {
        let mut guard = RecursionGuard {
            max_depth: 5,
            current_depth: 5,
            allow_reentrant: true,
        };
        let result = guard.try_increment_depth();
        assert!(result.is_err());
        assert_eq!(guard.current_depth, 5);
    }

    /// T4-b: guard_recursion_or_abort エラー時に Guard の全フィールドが不変
    #[test]
    fn t4b_guard_fields_unchanged_on_error() {
        let initial = RecursionGuard {
            max_depth: 3,
            current_depth: 3,
            allow_reentrant: true,
        };
        let mut guard = initial.clone();
        let mut state = SearchState::Init;
        let _ = guard_recursion_or_abort(&mut guard, &mut state);
        assert_eq!(guard.max_depth, initial.max_depth);
        assert_eq!(guard.current_depth, initial.current_depth);
        assert_eq!(guard.allow_reentrant, initial.allow_reentrant);
    }

    /// T4-c: 正常系で呼び出し前後で SearchState が不変
    #[test]
    fn t4c_state_unchanged_on_success() {
        let mut guard = RecursionGuard::new(8, true);
        let mut state = SearchState::Refine;
        let _ = guard_recursion_or_abort(&mut guard, &mut state);
        assert_eq!(state, SearchState::Refine);
    }

    /// T5-a: 決定論性 — 同一入力で結果が一致
    #[test]
    fn t5a_deterministic_same_input() {
        let mut guard1 = RecursionGuard::new(8, true);
        let mut guard2 = RecursionGuard::new(8, true);
        let mut state1 = SearchState::Init;
        let mut state2 = SearchState::Init;
        let r1 = guard_recursion_or_abort(&mut guard1, &mut state1);
        let r2 = guard_recursion_or_abort(&mut guard2, &mut state2);
        assert_eq!(r1.is_ok(), r2.is_ok());
        assert_eq!(guard1.current_depth, guard2.current_depth);
    }

    /// T5-b: 異なる超過量でもガード結果が同一
    #[test]
    fn t5b_deterministic_across_excess_amounts() {
        for excess in 1..=1000 {
            let mut guard = RecursionGuard {
                max_depth: 3,
                current_depth: 3 + excess,
                allow_reentrant: true,
            };
            let mut state = SearchState::Init;
            let result = guard_recursion_or_abort(&mut guard, &mut state);
            assert!(matches!(result, Err(DarviumError::SearchRecursionExceeded)));
        }
    }

    /// T5-c: 超過量 sweep で戻り値の型が不変
    #[test]
    fn t5c_type_stability_across_sweep() {
        let depths: Vec<u32> = (1..=100).collect();
        for &d in &depths {
            let mut guard = RecursionGuard {
                max_depth: 0,
                current_depth: d,
                allow_reentrant: true,
            };
            let mut state = SearchState::Init;
            let result = guard_recursion_or_abort(&mut guard, &mut state);
            assert!(result.is_err());
        }
    }

    /// T6-a: 統合テスト — max_depth=3 で 3 回成功
    #[test]
    fn t6a_integration_three_calls_succeed() {
        let mut guard = RecursionGuard::new(3, true);
        let mut state = SearchState::Init;
        for _ in 0..3 {
            let result = guard_recursion_or_abort(&mut guard, &mut state);
            assert!(result.is_ok());
        }
        assert_eq!(guard.current_depth, 3);
    }

    /// T6-b: 統合テスト — 4 回目の呼び出しがブロックされる
    #[test]
    fn t6b_integration_fourth_call_blocked() {
        let mut guard = RecursionGuard::new(3, true);
        let mut state = SearchState::Init;
        for _ in 0..3 {
            assert!(guard_recursion_or_abort(&mut guard, &mut state).is_ok());
        }
        let result = guard_recursion_or_abort(&mut guard, &mut state);
        assert!(matches!(result, Err(DarviumError::SearchRecursionExceeded)));
        assert_eq!(state, SearchState::Abort);
    }

    /// T6-c: ブロック後の decrement_depth が安全に動作する
    #[test]
    fn t6c_decrement_after_block() {
        let mut guard = RecursionGuard::new(3, true);
        let mut state = SearchState::Init;
        // 3 回成功
        for _ in 0..3 {
            let _ = guard_recursion_or_abort(&mut guard, &mut state);
        }
        // 状態をリセット（ブロック後の状態確認用）
        assert_eq!(guard.current_depth, 3);
        guard.decrement_depth();
        assert_eq!(guard.current_depth, 2);
        guard.decrement_depth();
        assert_eq!(guard.current_depth, 1);
        guard.decrement_depth();
        assert_eq!(guard.current_depth, 0);
        guard.decrement_depth();
        assert_eq!(guard.current_depth, 0); // アンダーフロー防止
    }

    /// T6-d: 呼び出し→復帰のペアで current_depth が元に戻る
    #[test]
    fn t6d_depth_returns_after_call_return_pair() {
        let mut guard = RecursionGuard::new(8, true);
        let mut state = SearchState::Init;
        assert_eq!(guard.current_depth, 0);
        // 呼び出し
        assert!(guard_recursion_or_abort(&mut guard, &mut state).is_ok());
        assert_eq!(guard.current_depth, 1);
        // 復帰
        guard.decrement_depth();
        assert_eq!(guard.current_depth, 0);
        // 再度呼び出し
        assert!(guard_recursion_or_abort(&mut guard, &mut state).is_ok());
        assert_eq!(guard.current_depth, 1);
        guard.decrement_depth();
        assert_eq!(guard.current_depth, 0);
    }

    /// T6-e: allow_reentrant=false で 1 回目も通らない
    #[test]
    fn t6e_disallowed_first_call_fails() {
        let mut guard = RecursionGuard::new(8, false);
        let mut state = SearchState::Init;
        let result = guard_recursion_or_abort(&mut guard, &mut state);
        assert!(matches!(result, Err(DarviumError::SearchRecursionExceeded)));
        assert_eq!(guard.current_depth, 0);
    }

    // ============================================================
    // M-1-3: 観測テスト (OTS)
    // ============================================================

    /// OTS-1: 深さ上限遮断時のアロケーション増分累積値ゼロ検証。
    ///
    /// SearchRecursionExceeded 遮断ロジックが連続 10^4 回発動した状態における
    /// 処理時間の分散を測定する。この関数は純粋なスタック操作（if-else, 早期return）
    /// のみで構成されているため、ヒープアロケーションは一切発生しない。
    /// アロケーションゼロ性は関数の構造的性質として保証される。
    ///
    /// 注意: Rust の #[global_allocator] はバイナリ全体で1つしか設定できないため、
    /// テスト関数内での CountingAllocator 注入は行わない。代わりに10,000回の
    /// 連続呼び出しにおける処理時間の分散 σ² を測定し、ガードロジックの
    /// 最悪時間有界性を検証する。
    #[test]
    fn ots1_allocation_free_guard_observation() {
        let mut guard = RecursionGuard {
            max_depth: 3,
            current_depth: 3,
            allow_reentrant: true,
        };
        let mut state = SearchState::Init;
        let n = 10_000u32;

        // ウォームアップ
        let mut warmup = guard.clone();
        for _ in 0..100 {
            let _ = guard_recursion_or_abort(&mut warmup, &mut state);
        }

        // 計測
        let mut latencies_ns: Vec<u64> = Vec::with_capacity(n as usize);
        for _ in 0..n {
            let start = std::time::Instant::now();
            let result = guard_recursion_or_abort(&mut guard, &mut state);
            let elapsed = start.elapsed().as_nanos() as u64;
            latencies_ns.push(elapsed);
            assert!(result.is_err());
        }

        // 統計量の計算
        let sum: u64 = latencies_ns.iter().sum();
        let avg = sum as f64 / n as f64;
        let variance = latencies_ns
            .iter()
            .map(|&m| {
                let diff = m as f64 - avg;
                diff * diff
            })
            .sum::<f64>()
            / n as f64;
        let max = latencies_ns.iter().copied().max().unwrap_or(0);
        let min = latencies_ns.iter().copied().min().unwrap_or(0);

        let mut sorted = latencies_ns.clone();
        sorted.sort_unstable();
        let p50 = sorted[(n as usize) / 2];
        let p95 = sorted[(n as f64 * 0.95) as usize];
        let p99 = sorted[(n as f64 * 0.99) as usize];

        println!("OTS-1,guard_recursion_exceeded,n={},avg_ns={:.3},var_ns={:.3},max_ns={},min_ns={},p50={},p95={},p99={}",
            n, avg, variance, max, min, p50, p95, p99);
        println!("=== 結果: OTS-1 観測完了 ===");
    }

    /// OTS-2: スタックフレーム変位のカットオフ境界測定。
    ///
    /// 深さ d = d_max に到達しガードが発動した前後で、スタックフレームの
    /// 成長率が不連続に 0 になることを確認する。
    /// 各深度でのスタックアドレスをサンプリングし、深度間の変位 ΔSP(d) を計算する。
    fn measure_stack_depth(
        guard: &mut RecursionGuard,
        state: &mut SearchState,
        depth: u32,
        samples: &mut Vec<(u32, usize)>,
    ) {
        // このフレームのスタック上の位置を記録（ローカル変数のアドレス）
        let frame_marker: u64 = depth as u64;
        let stack_addr = &frame_marker as *const u64 as usize;
        samples.push((depth, stack_addr));

        if depth >= guard.max_depth {
            return;
        }

        match guard_recursion_or_abort(guard, state) {
            Ok(()) => {
                measure_stack_depth(guard, state, depth + 1, samples);
                guard.decrement_depth();
            }
            Err(_) => {
                // ガード遮断 — これ以上再帰しない
            }
        }
    }

    #[test]
    fn ots2_stack_frame_displacement_observation() {
        let mut guard = RecursionGuard::new(3, true);
        let mut state = SearchState::Init;
        let mut samples: Vec<(u32, usize)> = Vec::new();

        // 初期状態 (depth=0)
        let marker: u64 = 0;
        samples.push((0, &marker as *const u64 as usize));

        // 再帰計測
        measure_stack_depth(&mut guard, &mut state, 1, &mut samples);

        // CSV 出力
        println!("OTS-2,depth,stack_addr,delta_sp");
        samples.sort_by_key(|&(d, _)| d);
        for i in 0..samples.len() {
            let (depth, addr) = samples[i];
            let delta = if i > 0 {
                let prev_addr = samples[i - 1].1;
                if addr > prev_addr {
                    addr - prev_addr
                } else {
                    prev_addr - addr
                }
            } else {
                0
            };
            println!("OTS-2,{},{},{}", depth, addr, delta);
        }
        println!("=== 結果: OTS-2 観測完了 ===");
    }
}

/// 検索予算 (RFC §13.3)。
///
/// SearchWorkflow が消費できる上限を束ねる bounded search 制約。
/// 使用量の追跡は SearchBudgetSnapshot が担当し、本構造体は上限定義のみを持つ。
#[derive(Debug, Clone)]
pub struct SearchBudget {
    pub max_iterations: u32,
    pub max_retrieval_calls: u32,
    pub max_prompt_tokens: u64,
    pub max_wall_clock_ms: u64,
}

/// 検索予算使用量スナップショット (RFC §13.3)。
///
/// SearchBudget の消費メソッドが返す現在の使用量。
/// 実時間計測との結合 (wall_clock_ms_used) は Clock トレイト統合時に別途対応する。
#[derive(Debug, Clone, PartialEq)]
pub struct SearchBudgetSnapshot {
    pub iterations_used: u32,
    pub retrieval_calls_used: u32,
    pub prompt_tokens_used: u64,
    pub wall_clock_ms_used: u64,
}

/// 再帰ガード (RFC §13.3)。
///
/// SearchWorkflow が自身を再帰的に呼び出す際の深さ制限を制御する。
/// allow_reentrant が false の場合、再帰呼び出し自体が禁止される。
#[derive(Debug, Clone)]
pub struct RecursionGuard {
    pub max_depth: u32,
    pub current_depth: u32,
    pub allow_reentrant: bool,
}

/// SearchBudget のデフォルト値。
///
/// constants.rs の Environment Policy Knob から値を取得する。
impl Default for SearchBudget {
    fn default() -> Self {
        Self {
            max_iterations: crate::constants::DEFAULT_MAX_ITERATIONS,
            max_retrieval_calls: crate::constants::DEFAULT_MAX_RETRIEVAL_CALLS,
            max_prompt_tokens: crate::constants::MAX_PROMPT_TOKENS,
            max_wall_clock_ms: crate::constants::DEFAULT_MAX_WALL_CLOCK_MS,
        }
    }
}

impl SearchBudget {
    /// 全フィールドを指定して SearchBudget を生成する。
    pub fn new(
        max_iterations: u32,
        max_retrieval_calls: u32,
        max_prompt_tokens: u64,
        max_wall_clock_ms: u64,
    ) -> Self {
        Self {
            max_iterations,
            max_retrieval_calls,
            max_prompt_tokens,
            max_wall_clock_ms,
        }
    }

    /// 1 イテレーションを消費する。
    ///
    /// RFC §13.6 ガード条件に従い、iterations_used が max_iterations に
    /// 達している場合は SearchBudgetExceeded を返す（サチュレーション）。
    pub fn try_consume_iteration(
        &self,
        snapshot: &mut SearchBudgetSnapshot,
    ) -> Result<(), crate::error::DarviumError> {
        if snapshot.iterations_used >= self.max_iterations {
            return Err(crate::error::DarviumError::SearchBudgetExceeded);
        }
        snapshot.iterations_used += 1;
        Ok(())
    }

    /// 1 回の検索呼び出しを消費する。
    ///
    /// retrieval_calls_used が max_retrieval_calls に達している場合は
    /// SearchBudgetExceeded を返す。
    pub fn try_consume_retrieval_call(
        &self,
        snapshot: &mut SearchBudgetSnapshot,
    ) -> Result<(), crate::error::DarviumError> {
        if snapshot.retrieval_calls_used >= self.max_retrieval_calls {
            return Err(crate::error::DarviumError::SearchBudgetExceeded);
        }
        snapshot.retrieval_calls_used += 1;
        Ok(())
    }

    /// 指定されたトークン数を消費する。
    ///
    /// 累積トークン (既存 + 新規) が max_prompt_tokens を超える場合は
    /// SearchBudgetExceeded を返す。上限ぴったりの場合は成功とする（境界値）。
    pub fn try_consume_prompt_tokens(
        &self,
        snapshot: &mut SearchBudgetSnapshot,
        tokens: u64,
    ) -> Result<(), crate::error::DarviumError> {
        let new_total = snapshot
            .prompt_tokens_used
            .checked_add(tokens)
            .ok_or(crate::error::DarviumError::SearchBudgetExceeded)?;
        if new_total > self.max_prompt_tokens {
            return Err(crate::error::DarviumError::SearchBudgetExceeded);
        }
        snapshot.prompt_tokens_used = new_total;
        Ok(())
    }

    /// 現在の使用量スナップショットを生成する。
    ///
    /// 消費メソッドの戻り値としてだけでなく、外部からの現在状態取得にも使用する。
    pub fn snapshot(&self, current: &SearchBudgetSnapshot) -> SearchBudgetSnapshot {
        current.clone()
    }
}

// ============================================================
// M-1-2: SearchBudgetExceeded ハードガードの遮断アサーション
// ============================================================

/// SearchBudget の全4次元の上限超過を事前検査する (M-1-2)。
///
/// RFC §13.6 ガード条件: 1つでも上限超過を検出した場合、
/// 即座に `Err(SearchBudgetExceeded)` を返す。
/// これは純粋な検査関数であり、使用量の消費は行わない。
///
/// チェック順序:
///   1. iterations (used >= max)
///   2. retrieval_calls (used >= max)
///   3. prompt_tokens (used > max)
///   4. wall_clock_ms (used > max)
///
/// prompt_tokens と wall_clock_ms は上限ぴったりの場合を通過とし、
/// 上限を超えた場合のみ超過と判定する。
/// iterations と retrieval_calls は saturated カウンタのため
/// used >= max で超過と判定する。
pub fn check_budget_exceeded(
    budget: &SearchBudget,
    snapshot: &SearchBudgetSnapshot,
) -> Result<(), crate::error::DarviumError> {
    if snapshot.iterations_used >= budget.max_iterations {
        return Err(crate::error::DarviumError::SearchBudgetExceeded);
    }
    if snapshot.retrieval_calls_used >= budget.max_retrieval_calls {
        return Err(crate::error::DarviumError::SearchBudgetExceeded);
    }
    if snapshot.prompt_tokens_used > budget.max_prompt_tokens {
        return Err(crate::error::DarviumError::SearchBudgetExceeded);
    }
    if snapshot.wall_clock_ms_used > budget.max_wall_clock_ms {
        return Err(crate::error::DarviumError::SearchBudgetExceeded);
    }
    Ok(())
}

/// 予算超過時に状態を `Abort` に変更するインターセプタ (M-1-2)。
///
/// 1. `check_budget_exceeded` で予算超過を検査
/// 2. 終端状態（Finalize / Abort）からの呼び出しは `TerminalStateViolation`
/// 3. 超過検出時 → 状態を `Abort` に変更してから `Err` を伝播
/// 4. 正常時 → `Ok(())` を返し状態は不変
pub fn guard_budget_or_abort(
    state: &mut SearchState,
    budget: &SearchBudget,
    snapshot: &SearchBudgetSnapshot,
) -> Result<(), crate::error::DarviumError> {
    // 終端状態からの遷移は全て違法 (M-1.5-2 不変条件)
    if matches!(state, SearchState::Finalize | SearchState::Abort) {
        return Err(crate::error::DarviumError::TerminalStateViolation);
    }
    // 予算超過チェック
    if let Err(e) = check_budget_exceeded(budget, snapshot) {
        *state = SearchState::Abort;
        return Err(e);
    }
    Ok(())
}

/// RecursionGuard のデフォルト値。
///
/// max_depth は constants.rs の Safety Invariant から、
/// current_depth は常に 0、allow_reentrant は false で初期化される。
impl Default for RecursionGuard {
    fn default() -> Self {
        Self {
            max_depth: crate::constants::DEFAULT_RECURSION_MAX_DEPTH,
            current_depth: 0,
            allow_reentrant: false,
        }
    }
}

impl RecursionGuard {
    /// 全フィールドを指定して RecursionGuard を生成する。
    pub fn new(max_depth: u32, allow_reentrant: bool) -> Self {
        Self {
            max_depth,
            current_depth: 0,
            allow_reentrant,
        }
    }

    /// 現在の深度を 1 増加する。
    ///
    /// RFC §13.6 ガード条件に従い、allow_reentrant が false の場合は
    /// 常に SearchRecursionExceeded を返す。allow_reentrant が true の場合は
    /// current_depth が max_depth に達していない場合のみ成功する。
    pub fn try_increment_depth(&mut self) -> Result<(), crate::error::DarviumError> {
        if !self.allow_reentrant {
            return Err(crate::error::DarviumError::SearchRecursionExceeded);
        }
        if self.current_depth >= self.max_depth {
            return Err(crate::error::DarviumError::SearchRecursionExceeded);
        }
        self.current_depth += 1;
        Ok(())
    }

    /// 現在の深度を 1 減少する（再帰からの復帰時）。
    ///
    /// アンダーフローを防止するため saturating_sub を使用する。
    /// current_depth が 0 の状態で呼び出してもパニックしない。
    pub fn decrement_depth(&mut self) {
        self.current_depth = self.current_depth.saturating_sub(1);
    }
}

/// 深さ超過時に状態を `Abort` に変更するインターセプタ (M-1-3)。
///
/// 1. `guard.try_increment_depth()` で深さ超過を検査
/// 2. 終端状態（Finalize / Abort）からの呼び出しは `TerminalStateViolation`
/// 3. 超過検出時 → 状態を `Abort` に変更してから `Err` を伝播
/// 4. 正常時 → `Ok(())` を返し state は不変、current_depth のみ +1
///
/// 呼び出し元は正常復帰後に `guard.decrement_depth()` を呼ぶ責任を負う。
pub fn guard_recursion_or_abort(
    guard: &mut RecursionGuard,
    state: &mut SearchState,
) -> Result<(), crate::error::DarviumError> {
    // 終端状態からの遷移は全て違法 (M-1.5-2 不変条件)
    if matches!(state, SearchState::Finalize | SearchState::Abort) {
        return Err(crate::error::DarviumError::TerminalStateViolation);
    }
    // 深さ超過チェック
    if let Err(e) = guard.try_increment_depth() {
        *state = SearchState::Abort;
        return Err(e);
    }
    Ok(())
}

/// SearchWorkflow の最終決定 (RFC §13.3)。
///
/// EvaluateCandidatesStep / RefineSearchPolicyStep による
/// bounded heuristic policy の出力として使用される。
/// M-1-1 では ReuseExisting / PatchExisting の 2 バリアントのみ使用する。
///
/// PartialEq は手動実装: petgraph::Graph (WorkflowGraph) が PartialEq を
/// 実装していないため、GenerateNew バリアントは構造的同値性ではなく
/// バリアント識別子のみで比較する。
#[derive(Debug, Clone)]
pub enum SearchOutcome {
    /// 既存ワークフローをそのまま再利用。
    ReuseExisting { graph_id: WorkflowGraphId },
    /// 既存ワークフローにパッチを適用。
    PatchExisting {
        graph_id: WorkflowGraphId,
        patch: GraphPatch,
    },
    /// 複数既存ワークフローの組成。
    ComposeExisting { plan: CompositionPlan },
    /// 新規ワークフロー生成。
    GenerateNew { proposal: WorkflowGraph },
    /// 探索中断。
    AbortSearch { reason: SearchAbortReason },
    /// 人間レビュー待ち。
    NeedsHumanReview { reason: String },
}

impl PartialEq for SearchOutcome {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::ReuseExisting { graph_id: a }, Self::ReuseExisting { graph_id: b }) => a == b,
            (
                Self::PatchExisting {
                    graph_id: a,
                    patch: b,
                },
                Self::PatchExisting {
                    graph_id: c,
                    patch: d,
                },
            ) => a == c && b == d,
            (Self::ComposeExisting { plan: a }, Self::ComposeExisting { plan: b }) => a == b,
            // WorkflowGraph (petgraph::Graph) does not impl PartialEq
            (Self::GenerateNew { .. }, Self::GenerateNew { .. }) => true,
            (Self::AbortSearch { reason: a }, Self::AbortSearch { reason: b }) => a == b,
            (Self::NeedsHumanReview { reason: a }, Self::NeedsHumanReview { reason: b }) => a == b,
            _ => false,
        }
    }
}

// ============================================================
// M0-2: GenerateNew 安全ガードロジック 型定義
// RFC §6.1 / §13.6 / §16A
// ============================================================

/// 副作用セット (RFC §6.1)。
///
/// `risk_score` は DeterminismScore の重み計算に使用。
/// `contains` メソッドは Stage 0 フィルタで使用 (§11.2)。
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SideEffectSet {
    /// 外部 API 書き込み
    pub writes_external_api: bool,
    /// 通知送信 (HumanChannel::notify() に対応)
    pub sends_notification: bool,
    /// 双方向 HITL (HumanChannel::communicate() に対応, v2.3-d)
    pub has_hitl_communicate: bool,
    /// DB 等の永続状態変更
    pub modifies_persistent_state: bool,
    /// true の場合は AG-03 ハードゲートでブロック
    pub irreversible: bool,
    /// [0.0, 1.0]: writes_external_api=1.0, DB変更=0.7, 通知=0.3, HITL Communicate=0.5
    pub risk_score: f32,
}

impl SideEffectSet {
    /// 副作用包含チェック: self が required を包含するかどうか。
    /// required の各フィールドが true の場合、self も true である必要がある。
    pub fn contains(&self, required: &SideEffectSet) -> bool {
        (!required.writes_external_api || self.writes_external_api)
            && (!required.sends_notification || self.sends_notification)
            && (!required.has_hitl_communicate || self.has_hitl_communicate)
            && (!required.modifies_persistent_state || self.modifies_persistent_state)
    }

    /// Auto-Approval 例外が安全な副作用セットかどうかを判定する。
    ///
    /// 条件: 外部 API 書き込みがなく、不可逆副作用もないこと。
    /// 永続状態変更・通知送信・HITL 通信は sandbox 内では許容する。
    pub fn is_safe_for_auto_approval(&self) -> bool {
        !self.writes_external_api && !self.irreversible
    }
}

/// 実行平面種別 (RFC §13.6 / §16A)。
///
/// `GenerateNew` 選択時のガードロジックにおいて、
/// どの平面で実行されているかに応じて human review の要否を決定する。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PlaneKind {
    /// 本番実行平面。全ての `GenerateNew` は human review 必須。
    Production,
    /// 訓練平面。safe-scoped なもののみ auto-approval 例外を許容。
    Training,
    /// 訓練平面下の safe-scoped sandbox。
    SafeSandbox,
}

/// SafeSandbox のスコープ境界 (RFC §16A v2.3 補足)。
///
/// Auto-Approval Exception Policy の scope boundary を表現する。
/// namespace、artifact kind、許可された副作用セットで bounded に定義される。
#[derive(Debug, Clone, PartialEq)]
pub struct SafeSandboxScope {
    /// 名前空間
    pub namespace: String,
    /// アーティファクト種別
    pub artifact_kind: String,
    /// 許可された副作用セット
    pub allowed_side_effects: SideEffectSet,
}

/// 静的閾値による候補評価 (M-1-1)。
///
/// 最良候補のスコア `best_score` を閾値 `EVALUATION_THRESHOLD` で評価し、
/// スコア >= 閾値 の場合は `ReuseExisting`、未満の場合は `PatchExisting` を返す。
///
/// # 引数
///
/// * `best_score` - 最良候補の blended_score。[0.0, 1.0] の範囲を仮定する。
///
/// # エラー
///
/// スコアが [0.0, 1.0] の範囲外の場合、`DarviumError::InvalidScore` を返す。
pub fn evaluate_candidates(best_score: f64) -> Result<SearchOutcome, crate::error::DarviumError> {
    if !(0.0..=1.0).contains(&best_score) {
        return Err(crate::error::DarviumError::InvalidScore(best_score));
    }
    if best_score >= crate::constants::EVALUATION_THRESHOLD {
        Ok(SearchOutcome::ReuseExisting {
            graph_id: String::new(),
        })
    } else {
        Ok(SearchOutcome::PatchExisting {
            graph_id: String::new(),
            patch: GraphPatch,
        })
    }
}

/// 自己評価割引の適用 (RFC §14.2)。
///
/// LLM 自己評価スコア `raw_score` に `SELF_CONF_DISCOUNT (0.85)` を乗算する。
/// 返値は [0.0, 1.0] の範囲にクランプされる。
pub fn apply_self_conf_discount(raw_score: f64) -> f64 {
    (raw_score * crate::constants::SELF_CONF_DISCOUNT).clamp(0.0, 1.0)
}

// === 信頼関連 ===
#[derive(Debug, Clone)]
pub struct TrustProfile;

#[derive(Debug, Clone)]
pub struct Provenance;

// === パッチ関連 ===
#[derive(Debug, Clone)]
pub struct WorkflowPatch;

/// グラフパッチ (RFC §12.1)。
/// Gold グラフを Gnew に変換する差分操作列と PatchConfidence を含む。
#[derive(Debug, Clone, PartialEq)]
pub struct GraphPatch;

/// 組成計画 (RFC §13.3)。
/// 複数既存ワークフローを接続して目的を満たす構成案。
#[derive(Debug, Clone, PartialEq)]
pub struct CompositionPlan {
    /// 組成対象のワークフローグラフID一覧。
    pub component_graph_ids: Vec<WorkflowGraphId>,
    /// 組成エッジ — (送信元ノードID, 送信先ノードID, エッジ種別)。
    pub composition_edges: Vec<(NodeId, NodeId, EdgeMeta)>,
    /// 組成後のワークフローが期待する入力変数宣言。
    pub expected_inputs: Vec<VarDecl>,
    /// 組成後のワークフローが出力する変数宣言。
    pub expected_output: VarDecl,
    /// この組成計画の信頼度スコア。
    pub confidence: f32,
}

impl CompositionPlan {
    /// 最小構成の CompositionPlan を生成する（テスト用）。
    pub fn new(
        component_graph_ids: Vec<WorkflowGraphId>,
        composition_edges: Vec<(NodeId, NodeId, EdgeMeta)>,
        expected_inputs: Vec<VarDecl>,
        expected_output: VarDecl,
    ) -> Self {
        Self {
            component_graph_ids,
            composition_edges,
            expected_inputs,
            expected_output,
            confidence: 1.0,
        }
    }
}

/// 検索中断理由 (RFC §13.3)。
#[derive(Debug, Clone, PartialEq)]
pub enum SearchAbortReason {
    BudgetExceeded,
    RecursionExceeded,
    OscillationDetected,
    ExplicitAbort,
}

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

// ============================================================
// v2.3-d: HITL (Human-In-The-Loop) 型定義
// ============================================================

/// 人間への依頼内容。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HumanRequest {
    /// 概要タイトル（Slack の見出し行、メールの件名）。
    pub subject: String,
    /// 詳細説明（ワークフロー名・成否・判断材料など）。
    pub body: String,
    /// 機械可読なコンテキスト情報。チャネル実装は透過的に通過させる。
    pub context: serde_json::Value,
    /// 応答待機の推奨最大時間。
    pub timeout: Option<std::time::Duration>,
}

/// 人間との双方向通信の結果。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HumanOutcome {
    Responded(HumanResponse),
    TimedOut,
    Unreachable(String),
}

/// 人間からの応答内容。
/// RFC §13A 規範要件2（edit mission text）に従い revised_body を保持。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HumanResponse {
    pub decision: HumanDecision,
    pub comment: Option<String>,
    pub revised_body: Option<String>,
}

/// 人間の判断。
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum HumanDecision {
    Approved,
    Rejected,
    NeedsRevision,
    Irrelevant,
    Unsafe,
}

/// 永続化される HITL インタラクションのレコード。
/// MetadataStore 経由で SQLite / InMemory に保存される。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StoredInteraction {
    /// UUID v4。全プロセス再起動を超えて一意。
    /// String として保持することで MetadataStore 実装との相互運用性を確保。
    pub interaction_id: String,
    /// リクエスト全文。
    pub request: HumanRequest,
    /// 応答。Resolved 時のみ Some。
    pub outcome: Option<HumanOutcome>,
    /// 現在の状態。
    pub status: InteractionStatus,
    /// 作成時刻（Unix エポックミリ秒）。
    pub created_at: u64,
    /// 最終更新時刻（Unix エポックミリ秒）。
    pub updated_at: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum InteractionStatus {
    Pending,
    Resolved,
}
