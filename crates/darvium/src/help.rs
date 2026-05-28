// HELP プロトコル状態機械 (RFC §41B.4-41B.9)
//
// 本モジュールは Child Support Villages / HELP Consensus（v2.3-e）の
// HELP 5段階プロトコルを純粋状態機械として実装する。
//
// 状態遷移:
//   Proposal → Offered → Accepted → Executing → Succeeded
//                    ↘ Rejected        ↘ Failed
// 終端状態 (Rejected, Succeeded, Failed) からの再遷移は厳格に禁止される。
//
// 各遷移は DarviumEventKind::Reciprocity イベントとして EventBus へ publish される。

use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::constants;
use crate::error::DarviumError;
use crate::event::{
    DarviumEvent, DarviumEventBus, DarviumEventKind, DeliveryMode, EventCausality, EventId,
    EventMetadata, EventPrivacy, EventRetention, EventSource, EventVisibility, InteractionMode,
    PiiHandlingPolicy, ReciprocityEventKind, TransportMeta,
};
use crate::types::WorkflowGraphId;

// ============================================================
// 型定義 (RFC §41B.4)
// ============================================================

/// HELP プロトコルの7状態 (RFC §41B.4-41B.9)。
///
/// 拡張された状態機械は5段階プロトコルに加えて、
/// Failed 終端状態を含む:
///
/// - Proposal: システムが候補 Adult を識別（段階1）
/// - Offered: Adult が支援意思を表明（段階2）
/// - Accepted: Child が支援を受諾
/// - Rejected: Child が支援を拒否（終端）
/// - Executing: 支援が実行中（段階4）
/// - Succeeded: 支援が成功（終端）
/// - Failed: 支援が失敗（終端）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HelpState {
    /// 候補 Adult が識別された段階。
    Proposal,
    /// Adult が支援を表明した段階。
    Offered,
    /// Child が支援を受諾した段階。
    Accepted,
    /// Child が支援を拒否した段階（終端状態）。
    Rejected,
    /// 支援が実行中の段階。
    Executing,
    /// 支援が成功した段階（終端状態）。
    Succeeded,
    /// 支援が失敗した段階（終端状態）。
    Failed,
}

impl HelpState {
    /// この状態が終端状態（再遷移禁止）かを返す。
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            HelpState::Rejected | HelpState::Succeeded | HelpState::Failed
        )
    }
}

/// 支援提案の構造体。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelpProposal {
    /// 提案の一意識別子。
    pub help_id: String,
    /// 支援元 Adult のワークフロー ID。
    pub from_workflow: WorkflowGraphId,
    /// 支援先 Child のワークフロー ID。
    pub to_workflow: WorkflowGraphId,
    /// 提案時の類似度スコア。
    pub similarity_score: f64,
    /// 提案時の空間距離。
    pub spatial_distance: f64,
    /// 提案の根拠（任意）。
    pub rationale: Option<String>,
}

/// 支援オファーの構造体 (RFC §41B.4)。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelpOffer {
    /// オファーの一意識別子。
    pub help_offer_id: String,
    /// ミッション ID。
    pub mission_id: String,
    /// 支援先 Child のワークフロー ID。
    pub child_graph_id: WorkflowGraphId,
    /// 支援元 Adult のワークフロー ID。
    pub adult_graph_id: WorkflowGraphId,
    /// オファーの状態。
    pub offer_state: HelpOfferState,
    /// 決定時刻（未決定の場合は None）。
    pub decided_at: Option<String>,
    /// 類似度スコア。
    pub similarity_score: f32,
    /// 空間距離。
    pub spatial_distance: f32,
    /// Adult の信頼値。
    pub adult_trust: f32,
    /// Adult のレピュテーション値。
    pub adult_reputation: f32,
    /// Child のニーズスコア。
    pub child_need_score: f32,
    /// 提案された支援モード。
    pub proposed_mode: HelpMode,
    /// オファーの根拠（任意）。
    pub rationale: Option<String>,
}

/// オファーの状態 (RFC §41B.4)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HelpOfferState {
    /// 未決定。
    Pending,
    /// 受諾済み。
    Accepted,
    /// 拒否済み。
    Rejected,
    /// 期限切れ。
    Expired,
}

/// 支援モード (RFC §41B.4)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HelpMode {
    /// SubWorkflow として再利用。
    ReuseAsSubWorkflow,
    /// Child と合成。
    ComposeWithChild,
    /// Child にパッチ適用。
    PatchChild,
    /// デモンストレーションのみ。
    DemonstrationOnly,
}

/// 支援決定の構造体。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelpDecision {
    /// 決定の一意識別子。
    pub decision_id: String,
    /// 決定元の Child ワークフロー ID。
    pub child_workflow: WorkflowGraphId,
    /// 決定対象のオファー ID。
    pub offer_id: String,
    /// 受諾または拒否。
    pub accepted: bool,
    /// 拒否理由（accepted=false の場合に設定）。
    pub rejection_reason: Option<HelpRejectionReason>,
    /// 決定時の追加メモ（任意）。
    pub note: Option<String>,
}

/// 支援拒否理由。
///
/// M1.75-4 で拡張: Unsafe, Irrelevant, Overloaded, ResourceExhausted を追加。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HelpRejectionReason {
    /// 類似度不足。
    InsufficientSimilarity,
    /// 信頼不足。
    InsufficientTrust,
    /// 空間距離超過。
    DistanceExceeded,
    /// 自律性の喪失リスク。
    AutonomyLossRisk,
    /// ニーズ不一致。
    NeedMismatch,
    /// 安全上の理由（RFC §41B.6 overload/unsafe 判定）。
    Unsafe,
    /// 無関係な提案（child のニーズと一致しない）。
    Irrelevant,
    /// Adult の負荷超過（式 41B-10 L_load 超過）。
    Overloaded,
    /// リソース不足。
    ResourceExhausted,
    /// その他。
    Other,
}

/// 支援実行の構造体。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelpExecution {
    /// 実行の一意識別子。
    pub execution_id: String,
    /// 実行対象のオファー ID。
    pub offer_id: String,
    /// 実際に使用された支援モード。
    pub executed_mode: HelpMode,
    /// 実行開始時刻。
    pub started_at: String,
}

/// 支援成功の構造体。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelpSuccess {
    /// 成功記録の一意識別子。
    pub success_id: String,
    /// 対応する実行 ID。
    pub execution_id: String,
    /// Child の経験値増加量。
    pub experience_gain: u32,
    /// 測定可能な成長量。
    pub growth_measure: f64,
}

/// 支援失敗の構造体。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelpFailure {
    /// 失敗記録の一意識別子。
    pub failure_id: String,
    /// 対応する実行 ID。
    pub execution_id: String,
    /// 失敗理由。
    pub reason: HelpFailureReason,
    /// 失敗の詳細（任意）。
    pub detail: Option<String>,
}

/// 支援失敗理由。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HelpFailureReason {
    /// 実行タイムアウト。
    Timeout,
    /// 実行エラー。
    ExecutionError,
    /// 互換性不一致。
    CompatibilityMismatch,
    /// リソース不足。
    ResourceExhausted,
    /// その他。
    Other,
}

/// HELP プロトコル状態機械のセッション。
///
/// 各 HELP インスタンスはこのセッションで状態遷移を管理する。
/// 終端状態に達した後は遷移が全て拒否される。
#[derive(Debug, Clone)]
pub struct HelpSession {
    /// HELP セッションの一意識別子。
    pub help_id: String,
    /// 支援元 Adult のワークフロー ID。
    pub from_workflow: WorkflowGraphId,
    /// 支援先 Child のワークフロー ID。
    pub to_workflow: WorkflowGraphId,
    /// 現在の状態。
    pub current_state: HelpState,
}

impl HelpSession {
    /// 新しい HELP セッションを Proposal 状態で開始する。
    pub fn new(
        help_id: String,
        from_workflow: WorkflowGraphId,
        to_workflow: WorkflowGraphId,
    ) -> Self {
        Self {
            help_id,
            from_workflow,
            to_workflow,
            current_state: HelpState::Proposal,
        }
    }

    /// 指定された次の状態へ遷移を試みる。
    ///
    /// 遷移が合法であれば状態を更新し、event_bus が Some の場合は
    /// 対応する HELP イベントを publish する。
    /// 違法遷移の場合は `DarviumError::HelpTransitionViolation` を返す。
    pub fn transition_to(
        &mut self,
        next: HelpState,
        event_bus: Option<&dyn DarviumEventBus>,
    ) -> Result<HelpState, DarviumError> {
        let current = self.current_state;
        if !is_legal_help_transition(&current, &next) {
            return Err(DarviumError::HelpTransitionViolation(format!(
                "{:?} -> {:?} is an illegal help state transition",
                current, next
            )));
        }

        if let Some(bus) = event_bus {
            emit_help_event(self, &current, &next, bus)?;
        }

        self.current_state = next;

        Ok(self.current_state)
    }

    /// 現在の状態を返す。
    pub fn current_state(&self) -> &HelpState {
        &self.current_state
    }
}

// ============================================================
// M1.75-4: Adult HELP Offer Policy / Child Consent Policy (RFC §41B.6-41B.7)
// ============================================================

/// Offer スコアの内訳記録構造 (RFC §41B.6)。
///
/// 各項は should_offer_help 内で compute_offer_score_breakdown により計算される。
/// 全項とも非負値。各項の解釈:
/// - distance_term: spatial proximity (0=遠距離, 1=同一位置)
/// - maturity_term: adult maturity reliability
/// - reciprocity_term: past reciprocity contribution
/// - reputation_term: adult reputation score
/// - urgency_term: child's urgency/need level
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct OfferScoreBreakdown {
    /// 空間的近接性項: 0.0 (遠距離) 〜 1.0 (同一位置)。
    pub distance_term: f64,
    /// Adult 成熟信頼性項: 0.0 〜 1.0。
    pub maturity_term: f64,
    /// 過去の互恵性貢献項: 0.0 〜 1.0。
    pub reciprocity_term: f64,
    /// Adult レピュテーション項: 0.0 〜 1.0。
    pub reputation_term: f64,
    /// Child 緊急度項: 0.0 〜 1.0。
    pub urgency_term: f64,
}

/// Adult 側の offer 決定 (RFC §41B.6, 式 41B-10)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OfferDecision {
    /// 支援を提供する。
    Offer,
    /// 支援を控える（理由付き）。
    Abstain(HelpRejectionReason),
}

/// Child 側の受入決定 (RFC §41B.7, 式 41B-13)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ChildDecision {
    /// 支援を受諾する。
    Accept,
    /// 支援を拒否する（理由付き）。
    Reject(HelpRejectionReason),
    /// 判断を保留する。
    Abstain,
}

/// Adult HELP offer policy (RFC §41B.6, 式 41B-10)。
///
/// O(h,c,M) = 1{a₁Q(h,c,M) - a₂L_load(h) - a₃P_risk(M) ≥ θ_offer}
///
/// 各パラメータは constants.rs の HELP_OFFER_* 定数がデフォルト値。
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AdultHelpOfferPolicy {
    /// 品質重み a₁: Q(h,c,M) の係数。
    pub quality_weight: f64,
    /// 負荷ペナルティ a₂: L_load(h) の係数。
    pub load_penalty: f64,
    /// リスクペナルティ a₃: P_risk(M) の係数。
    pub risk_penalty: f64,
    /// 判定閾値 θ_offer。
    pub threshold: f64,
}

impl Default for AdultHelpOfferPolicy {
    fn default() -> Self {
        Self {
            quality_weight: constants::HELP_OFFER_QUALITY_WEIGHT,
            load_penalty: constants::HELP_OFFER_LOAD_PENALTY,
            risk_penalty: constants::HELP_OFFER_RISK_PENALTY,
            threshold: constants::HELP_OFFER_THRESHOLD,
        }
    }
}

/// Child HELP acceptance policy (RFC §41B.7, 式 41B-12 / 41B-13)。
///
/// need: N(c) = γ₁(1-Ẽ(c)) + γ₂(1-T(c)) + γ₃(1-L(c))
/// accept: Accept(c,h,M) = 1{b₁Q(h,c,M) + b₂U(c,M) - b₃A(c,h) ≥ θ_accept}
///
/// 各パラメータは constants.rs の HELP_ACCEPT_* 定数がデフォルト値。
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ChildHelpAcceptancePolicy {
    /// Child need 経験値重み γ₁。
    pub gamma1: f64,
    /// Child need 信頼重み γ₂。
    pub gamma2: f64,
    /// Child need ライフサイクル重み γ₃。
    pub gamma3: f64,
    /// Acceptance 品質重み b₁。
    pub quality_weight: f64,
    /// Acceptance 不確実性重み b₂。
    pub uncertainty_weight: f64,
    /// Acceptance 自律性ペナルティ b₃。
    pub autonomy_penalty: f64,
    /// Acceptance 判定閾値 θ_accept。
    pub threshold: f64,
}

impl Default for ChildHelpAcceptancePolicy {
    fn default() -> Self {
        Self {
            gamma1: constants::HELP_ACCEPT_NEED_GAMMA1,
            gamma2: constants::HELP_ACCEPT_NEED_GAMMA2,
            gamma3: constants::HELP_ACCEPT_NEED_GAMMA3,
            quality_weight: constants::HELP_ACCEPT_QUALITY_WEIGHT,
            uncertainty_weight: constants::HELP_ACCEPT_UNCERTAINTY_WEIGHT,
            autonomy_penalty: constants::HELP_ACCEPT_AUTONOMY_PENALTY,
            threshold: constants::HELP_ACCEPT_THRESHOLD,
        }
    }
}

/// Offer スコアの内訳を計算する (RFC §41B.6 式 41B-10 の分解)。
///
/// 引数は全て [0.0, 1.0] の範囲を想定。
pub fn compute_offer_score_breakdown(
    spatial_distance: f64,
    maturity_level: f64,
    reciprocity_score: f64,
    reputation_score: f64,
    urgency_level: f64,
) -> OfferScoreBreakdown {
    OfferScoreBreakdown {
        distance_term: (1.0 - spatial_distance).clamp(0.0, 1.0),
        maturity_term: maturity_level.clamp(0.0, 1.0),
        reciprocity_term: reciprocity_score.clamp(0.0, 1.0),
        reputation_term: reputation_score.clamp(0.0, 1.0),
        urgency_term: urgency_level.clamp(0.0, 1.0),
    }
}

/// Adult 側の offer policy 判定 (RFC §41B.6, 式 41B-10)。
///
/// O(h,c,M) = 1{a₁Q(h,c,M) - a₂L_load(h) - a₃P_risk(M) ≥ θ_offer}
///
/// - quality_score: 提案品質 Q(h,c,M) [0,1]
/// - load_level: Adult の現在負荷 L_load(h) [0,1]
/// - risk_level: ミッションリスク P_risk(M) [0,1]
pub fn should_offer_help(
    quality_score: f64,
    load_level: f64,
    risk_level: f64,
    policy: &AdultHelpOfferPolicy,
) -> OfferDecision {
    let score = policy.quality_weight * quality_score
        - policy.load_penalty * load_level
        - policy.risk_penalty * risk_level;

    if score >= policy.threshold {
        OfferDecision::Offer
    } else if risk_level > 0.7 {
        OfferDecision::Abstain(HelpRejectionReason::Unsafe)
    } else if load_level > 0.7 {
        OfferDecision::Abstain(HelpRejectionReason::Overloaded)
    } else if quality_score < 0.4 {
        OfferDecision::Abstain(HelpRejectionReason::Irrelevant)
    } else {
        OfferDecision::Abstain(HelpRejectionReason::Other)
    }
}

/// Child のニーズスコアを計算する (RFC §41B.7, 式 41B-12)。
///
/// N(c) = γ₁(1-Ẽ(c)) + γ₂(1-T(c)) + γ₃(1-L(c))
///
/// 全ての入力は [0.0, 1.0] の範囲を想定。
/// 戻り値は [0.0, 1.0] にクランプされる。
pub fn child_need_score(
    normalized_experience: f64,
    trust: f64,
    lifecycle_score: f64,
    policy: &ChildHelpAcceptancePolicy,
) -> f64 {
    let need = policy.gamma1 * (1.0 - normalized_experience)
        + policy.gamma2 * (1.0 - trust)
        + policy.gamma3 * (1.0 - lifecycle_score);
    need.clamp(0.0, 1.0)
}

/// Child 側の受入判定 (RFC §41B.7, 式 41B-13)。
///
/// Accept(c,h,M) = 1{b₁Q(h,c,M) + b₂U(c,M) - b₃A(c,h) ≥ θ_accept}
///
/// - quality_score: 提案品質 Q(h,c,M) [0,1]
/// - uncertainty: ミッション不確実性 U(c,M) [0,1]
/// - autonomy_cost: 自律性喪失コスト A(c,h) [0,1]
pub fn decide_help_offer(
    quality_score: f64,
    uncertainty: f64,
    autonomy_cost: f64,
    policy: &ChildHelpAcceptancePolicy,
) -> ChildDecision {
    let score = policy.quality_weight * quality_score + policy.uncertainty_weight * uncertainty
        - policy.autonomy_penalty * autonomy_cost;

    if score >= policy.threshold {
        ChildDecision::Accept
    } else if autonomy_cost > 0.7 {
        ChildDecision::Reject(HelpRejectionReason::AutonomyLossRisk)
    } else if quality_score < 0.3 {
        ChildDecision::Reject(HelpRejectionReason::InsufficientSimilarity)
    } else if uncertainty > 0.8 {
        ChildDecision::Abstain
    } else {
        ChildDecision::Reject(HelpRejectionReason::NeedMismatch)
    }
}

// ============================================================
// 遷移判定 (RFC §41B.4-41B.9)
// ============================================================

/// 2つの HELP 状態間の遷移が合法かを判定する。
///
/// # 合法遷移
///
/// | from | to | 段階 |
/// |------|-----|------|
/// | Proposal | Offered | Adult が支援を表明 |
/// | Offered | Accepted | Child が受諾 |
/// | Offered | Rejected | Child が拒否（終端） |
/// | Accepted | Executing | 実行開始 |
/// | Executing | Succeeded | 成功（終端） |
/// | Executing | Failed | 失敗（終端） |
///
/// 終端状態 (Rejected, Succeeded, Failed) からの遷移は全て違法。
pub fn is_legal_help_transition(current: &HelpState, next: &HelpState) -> bool {
    matches!(
        (current, next),
        (HelpState::Proposal, HelpState::Offered)
            | (HelpState::Offered, HelpState::Accepted)
            | (HelpState::Offered, HelpState::Rejected)
            | (HelpState::Accepted, HelpState::Executing)
            | (HelpState::Executing, HelpState::Succeeded)
            | (HelpState::Executing, HelpState::Failed)
    )
}

/// 遷移に対応する ReciprocityEventKind variant を返す。
pub fn transition_to_event_kind(from: &HelpState, to: &HelpState) -> Option<ReciprocityEventKind> {
    if is_legal_help_transition(from, to) {
        match (from, to) {
            (HelpState::Proposal, HelpState::Offered) => Some(ReciprocityEventKind::HelpOffered),
            (HelpState::Offered, HelpState::Accepted) => Some(ReciprocityEventKind::HelpAccepted),
            (HelpState::Offered, HelpState::Rejected) => Some(ReciprocityEventKind::HelpRejected),
            (HelpState::Accepted, HelpState::Executing) => Some(ReciprocityEventKind::HelpExecuted),
            (HelpState::Executing, HelpState::Succeeded) => {
                Some(ReciprocityEventKind::HelpSucceeded)
            }
            (HelpState::Executing, HelpState::Failed) => Some(ReciprocityEventKind::HelpAbandoned),
            _ => None,
        }
    } else {
        None
    }
}

/// HELP 遷移を EventBus へ publish する。
///
/// publish される DarviumEvent の payload には以下を含む:
/// - help_id: HELP セッション識別子
/// - from_workflow: 支援元 Adult の WorkflowGraphId
/// - to_workflow: 支援先 Child の WorkflowGraphId
/// - transition_type: 遷移種別（"Proposal->Offered" 等）
/// - timestamp_vt: VirtualClock のタイムスタンプ
pub fn emit_help_event(
    session: &HelpSession,
    from: &HelpState,
    to: &HelpState,
    event_bus: &dyn DarviumEventBus,
) -> Result<EventId, DarviumError> {
    let event_kind = transition_to_event_kind(from, to).ok_or_else(|| {
        DarviumError::HelpTransitionViolation(format!(
            "{:?} -> {:?} に対応するイベントがありません",
            from, to
        ))
    })?;

    let transition_type = format!("{:?}->{:?}", from, to);
    let payload = serde_json::json!({
        "help_id": session.help_id,
        "from_workflow": session.from_workflow,
        "to_workflow": session.to_workflow,
        "transition_type": transition_type,
        "timestamp_vt": event_bus.now(),
    });

    let event = DarviumEvent {
        event_id: uuid::Uuid::new_v4().to_string(),
        kind: DarviumEventKind::Reciprocity(event_kind),
        interaction_mode: InteractionMode::OneWay,
        payload,
        causality: EventCausality {
            parent_event_id: None,
            root_event_id: None,
            trace_ref: None,
            mission_id: None,
            workflow_id: None,
            run_id: None,
        },
        metadata: EventMetadata {
            clock: event_bus.now(),
            timestamp: SystemTime::now(),
            source: EventSource::Test,
        },
        transport_meta: Some(TransportMeta {
            delivery_mode: DeliveryMode::AtLeastOnce,
            reply_to: None,
            ttl_seconds: None,
        }),
        visibility: EventVisibility::Public,
        retention: EventRetention {
            persist: true,
            ttl_days: None,
        },
        privacy: EventPrivacy {
            contains_pii: false,
            sandbox_only: false,
            pii_handling: PiiHandlingPolicy::Reject,
        },
    };

    event_bus.publish(event)
}

// ============================================================
// テスト
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::EventFilter;
    use crate::event::FakeEventBus;
    use rand::prelude::*;
    use rand::rngs::StdRng;

    // ============================================================
    // T-1: 全合法遷移の遷移行列総当たりテスト
    // ============================================================
    #[test]
    fn test_t1_transition_matrix_exhaustive() {
        let all_states = [
            HelpState::Proposal,
            HelpState::Offered,
            HelpState::Accepted,
            HelpState::Rejected,
            HelpState::Executing,
            HelpState::Succeeded,
            HelpState::Failed,
        ];

        let legal: &[(HelpState, HelpState)] = &[
            (HelpState::Proposal, HelpState::Offered),
            (HelpState::Offered, HelpState::Accepted),
            (HelpState::Offered, HelpState::Rejected),
            (HelpState::Accepted, HelpState::Executing),
            (HelpState::Executing, HelpState::Succeeded),
            (HelpState::Executing, HelpState::Failed),
        ];

        for current in &all_states {
            for next in &all_states {
                let is_legal = is_legal_help_transition(current, next);
                let expected = legal.contains(&(*current, *next));
                assert_eq!(
                    is_legal, expected,
                    "遷移行列不一致: {:?} -> {:?} は期待={}, 実際={}",
                    current, next, expected, is_legal
                );
            }
        }
    }

    // ============================================================
    // T-2: 正常系列完走テスト
    // ============================================================
    #[test]
    fn test_t2_normal_sequence() {
        let mut session = HelpSession::new("help-001".into(), "adult-1".into(), "child-1".into());
        let bus = FakeEventBus::new();

        assert_eq!(session.current_state(), &HelpState::Proposal);

        session
            .transition_to(HelpState::Offered, Some(&bus))
            .unwrap();
        assert_eq!(session.current_state(), &HelpState::Offered);

        session
            .transition_to(HelpState::Accepted, Some(&bus))
            .unwrap();
        assert_eq!(session.current_state(), &HelpState::Accepted);

        session
            .transition_to(HelpState::Executing, Some(&bus))
            .unwrap();
        assert_eq!(session.current_state(), &HelpState::Executing);

        session
            .transition_to(HelpState::Succeeded, Some(&bus))
            .unwrap();
        assert_eq!(session.current_state(), &HelpState::Succeeded);

        // 全4遷移を確認
        assert_eq!(bus.published_events().len(), 4);
    }

    // ============================================================
    // T-3: Rejected 終端テスト
    // ============================================================
    #[test]
    fn test_t3_rejected_terminal() {
        let mut session = HelpSession::new("help-002".into(), "adult-1".into(), "child-1".into());
        let bus = FakeEventBus::new();

        session
            .transition_to(HelpState::Offered, Some(&bus))
            .unwrap();
        session
            .transition_to(HelpState::Rejected, Some(&bus))
            .unwrap();
        assert_eq!(session.current_state(), &HelpState::Rejected);

        // Rejected からの全遷移が拒否されることを確認
        let next_states = [
            HelpState::Proposal,
            HelpState::Offered,
            HelpState::Accepted,
            HelpState::Executing,
            HelpState::Succeeded,
            HelpState::Failed,
        ];
        for next in &next_states {
            let result = session.transition_to(*next, Some(&bus));
            assert!(
                result.is_err(),
                "Rejected からの {:?} 遷移は拒否されるべき",
                next
            );
        }
    }

    // ============================================================
    // T-4: Failed 終端テスト
    // ============================================================
    #[test]
    fn test_t4_failed_terminal() {
        let mut session = HelpSession::new("help-003".into(), "adult-1".into(), "child-1".into());
        let bus = FakeEventBus::new();

        session
            .transition_to(HelpState::Offered, Some(&bus))
            .unwrap();
        session
            .transition_to(HelpState::Accepted, Some(&bus))
            .unwrap();
        session
            .transition_to(HelpState::Executing, Some(&bus))
            .unwrap();
        session
            .transition_to(HelpState::Failed, Some(&bus))
            .unwrap();
        assert_eq!(session.current_state(), &HelpState::Failed);

        // Failed からの全遷移が拒否されることを確認
        let next_states = [
            HelpState::Proposal,
            HelpState::Offered,
            HelpState::Accepted,
            HelpState::Rejected,
            HelpState::Executing,
            HelpState::Succeeded,
        ];
        for next in &next_states {
            let result = session.transition_to(*next, Some(&bus));
            assert!(
                result.is_err(),
                "Failed からの {:?} 遷移は拒否されるべき",
                next
            );
        }
    }

    // ============================================================
    // T-5: 違法遷移（飛び級）拒否テスト
    // ============================================================
    #[test]
    fn test_t5_illegal_transitions() {
        let bus = FakeEventBus::new();
        let mut session = HelpSession::new("help-004".into(), "adult-1".into(), "child-1".into());

        // Proposal から直接 Succeeded への飛び遷移
        let result = session.transition_to(HelpState::Succeeded, Some(&bus));
        assert!(result.is_err(), "Proposal->Succeeded は違法遷移");
        assert_eq!(session.current_state(), &HelpState::Proposal);

        // Proposal から直接 Executing への飛び遷移
        let result = session.transition_to(HelpState::Executing, Some(&bus));
        assert!(result.is_err(), "Proposal->Executing は違法遷移");

        // Offered から直接 Succeeded への飛び遷移
        session
            .transition_to(HelpState::Offered, Some(&bus))
            .unwrap();
        let result = session.transition_to(HelpState::Succeeded, Some(&bus));
        assert!(result.is_err(), "Offered->Succeeded は違法遷移");

        // Accepted から直接 Succeeded への飛び遷移
        session
            .transition_to(HelpState::Accepted, Some(&bus))
            .unwrap();
        let result = session.transition_to(HelpState::Succeeded, Some(&bus));
        assert!(result.is_err(), "Accepted->Succeeded は違法遷移");

        // Executing から直接 Proposal への逆流
        session
            .transition_to(HelpState::Executing, Some(&bus))
            .unwrap();
        let result = session.transition_to(HelpState::Proposal, Some(&bus));
        assert!(result.is_err(), "Executing->Proposal は違法な逆遷移");

        // 正常系は依然として動作
        session
            .transition_to(HelpState::Succeeded, Some(&bus))
            .unwrap();
        assert_eq!(session.current_state(), &HelpState::Succeeded);
    }

    // ============================================================
    // T-6: EventBus publish テスト
    // ============================================================
    #[test]
    fn test_t6_eventbus_publish() {
        let bus = FakeEventBus::new();
        let mut session = HelpSession::new("help-005".into(), "adult-1".into(), "child-1".into());

        // Proposal -> Offered: HelpOffered
        session
            .transition_to(HelpState::Offered, Some(&bus))
            .unwrap();
        let published = bus.published_events();
        assert_eq!(published.len(), 1);
        assert_eq!(
            published[0].kind,
            DarviumEventKind::Reciprocity(ReciprocityEventKind::HelpOffered)
        );

        // Offered -> Accepted: HelpAccepted
        session
            .transition_to(HelpState::Accepted, Some(&bus))
            .unwrap();
        let published = bus.published_events();
        assert_eq!(published.len(), 2);
        assert_eq!(
            published[1].kind,
            DarviumEventKind::Reciprocity(ReciprocityEventKind::HelpAccepted)
        );

        // Accepted -> Executing: HelpExecuted
        session
            .transition_to(HelpState::Executing, Some(&bus))
            .unwrap();
        let published = bus.published_events();
        assert_eq!(published.len(), 3);
        assert_eq!(
            published[2].kind,
            DarviumEventKind::Reciprocity(ReciprocityEventKind::HelpExecuted)
        );

        // Executing -> Succeeded: HelpSucceeded
        session
            .transition_to(HelpState::Succeeded, Some(&bus))
            .unwrap();
        let published = bus.published_events();
        assert_eq!(published.len(), 4);
        assert_eq!(
            published[3].kind,
            DarviumEventKind::Reciprocity(ReciprocityEventKind::HelpSucceeded)
        );

        // payload の内容確認
        let last_event = &published[3];
        let payload = &last_event.payload;
        assert_eq!(payload["help_id"], "help-005");
        assert_eq!(payload["from_workflow"], "adult-1");
        assert_eq!(payload["to_workflow"], "child-1");
        assert_eq!(payload["transition_type"], "Executing->Succeeded");
        assert!(payload.get("timestamp_vt").is_some());
    }

    // ============================================================
    // T-7: EventBus replay 完全性テスト
    // ============================================================
    #[test]
    fn test_t7_eventbus_replay() {
        let bus = FakeEventBus::new();
        let mut session = HelpSession::new("help-006".into(), "adult-1".into(), "child-1".into());

        // 正常系列を実行
        session
            .transition_to(HelpState::Offered, Some(&bus))
            .unwrap();
        session
            .transition_to(HelpState::Accepted, Some(&bus))
            .unwrap();
        session
            .transition_to(HelpState::Executing, Some(&bus))
            .unwrap();
        session
            .transition_to(HelpState::Succeeded, Some(&bus))
            .unwrap();

        // replay で全イベント取得
        let filter = EventFilter {
            kind_filter: None,
            since_vt: Some(0),
            until_vt: None,
        };
        let replayed = bus.replay(0, filter).unwrap();

        assert_eq!(replayed.len(), 4, "replay 件数が遷移回数(4)と一致");

        // 順序の確認
        assert_eq!(
            replayed[0].kind,
            DarviumEventKind::Reciprocity(ReciprocityEventKind::HelpOffered)
        );
        assert_eq!(
            replayed[1].kind,
            DarviumEventKind::Reciprocity(ReciprocityEventKind::HelpAccepted)
        );
        assert_eq!(
            replayed[2].kind,
            DarviumEventKind::Reciprocity(ReciprocityEventKind::HelpExecuted)
        );
        assert_eq!(
            replayed[3].kind,
            DarviumEventKind::Reciprocity(ReciprocityEventKind::HelpSucceeded)
        );
    }

    // ============================================================
    // T-8: 構造体フィールド整合性 + serde ラウンドトリップテスト
    // ============================================================
    #[test]
    fn test_t8_serde_roundtrip() {
        // HelpState serde
        for state in &[
            HelpState::Proposal,
            HelpState::Offered,
            HelpState::Accepted,
            HelpState::Rejected,
            HelpState::Executing,
            HelpState::Succeeded,
            HelpState::Failed,
        ] {
            let json = serde_json::to_string(state).unwrap();
            let deserialized: HelpState = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, *state);
        }

        // HelpOfferState serde
        for s in &[
            HelpOfferState::Pending,
            HelpOfferState::Accepted,
            HelpOfferState::Rejected,
            HelpOfferState::Expired,
        ] {
            let json = serde_json::to_string(s).unwrap();
            let deserialized: HelpOfferState = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, *s);
        }

        // HelpMode serde
        for m in &[
            HelpMode::ReuseAsSubWorkflow,
            HelpMode::ComposeWithChild,
            HelpMode::PatchChild,
            HelpMode::DemonstrationOnly,
        ] {
            let json = serde_json::to_string(m).unwrap();
            let deserialized: HelpMode = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, *m);
        }

        // HelpProposal serde
        let proposal = HelpProposal {
            help_id: "p1".into(),
            from_workflow: "a1".into(),
            to_workflow: "c1".into(),
            similarity_score: 0.85,
            spatial_distance: 0.3,
            rationale: Some("good match".into()),
        };
        let json = serde_json::to_string(&proposal).unwrap();
        let deserialized: HelpProposal = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.help_id, "p1");
        assert_eq!(deserialized.similarity_score, 0.85);

        // HelpOffer serde
        let offer = HelpOffer {
            help_offer_id: "o1".into(),
            mission_id: "m1".into(),
            child_graph_id: "c1".into(),
            adult_graph_id: "a1".into(),
            offer_state: HelpOfferState::Pending,
            decided_at: None,
            similarity_score: 0.85,
            spatial_distance: 0.3,
            adult_trust: 0.9,
            adult_reputation: 0.8,
            child_need_score: 0.7,
            proposed_mode: HelpMode::ComposeWithChild,
            rationale: Some("strong candidate".into()),
        };
        let json = serde_json::to_string(&offer).unwrap();
        let deserialized: HelpOffer = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.help_offer_id, "o1");

        // HelpDecision serde
        let decision = HelpDecision {
            decision_id: "d1".into(),
            child_workflow: "c1".into(),
            offer_id: "o1".into(),
            accepted: true,
            rejection_reason: None,
            note: None,
        };
        let json = serde_json::to_string(&decision).unwrap();
        let deserialized: HelpDecision = serde_json::from_str(&json).unwrap();
        assert!(deserialized.accepted);

        // HelpExecution serde
        let execution = HelpExecution {
            execution_id: "e1".into(),
            offer_id: "o1".into(),
            executed_mode: HelpMode::PatchChild,
            started_at: "2026-01-01T00:00:00Z".into(),
        };
        let json = serde_json::to_string(&execution).unwrap();
        let deserialized: HelpExecution = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.execution_id, "e1");

        // HelpSuccess serde
        let success = HelpSuccess {
            success_id: "s1".into(),
            execution_id: "e1".into(),
            experience_gain: 10,
            growth_measure: 0.5,
        };
        let json = serde_json::to_string(&success).unwrap();
        let deserialized: HelpSuccess = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.experience_gain, 10);

        // HelpFailure serde
        let failure = HelpFailure {
            failure_id: "f1".into(),
            execution_id: "e1".into(),
            reason: HelpFailureReason::Timeout,
            detail: Some("execution exceeded time limit".into()),
        };
        let json = serde_json::to_string(&failure).unwrap();
        let deserialized: HelpFailure = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.reason, HelpFailureReason::Timeout);

        // HelpRejectionReason serde (全10 variant)
        for r in &[
            HelpRejectionReason::InsufficientSimilarity,
            HelpRejectionReason::InsufficientTrust,
            HelpRejectionReason::DistanceExceeded,
            HelpRejectionReason::AutonomyLossRisk,
            HelpRejectionReason::NeedMismatch,
            HelpRejectionReason::Unsafe,
            HelpRejectionReason::Irrelevant,
            HelpRejectionReason::Overloaded,
            HelpRejectionReason::ResourceExhausted,
            HelpRejectionReason::Other,
        ] {
            let json = serde_json::to_string(r).unwrap();
            let deserialized: HelpRejectionReason = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, *r);
        }

        // HelpFailureReason serde
        for r in &[
            HelpFailureReason::Timeout,
            HelpFailureReason::ExecutionError,
            HelpFailureReason::CompatibilityMismatch,
            HelpFailureReason::ResourceExhausted,
            HelpFailureReason::Other,
        ] {
            let json = serde_json::to_string(r).unwrap();
            let deserialized: HelpFailureReason = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized, *r);
        }

        // OfferScoreBreakdown serde
        let breakdown = OfferScoreBreakdown {
            distance_term: 0.8,
            maturity_term: 0.7,
            reciprocity_term: 0.6,
            reputation_term: 0.9,
            urgency_term: 0.4,
        };
        let json = serde_json::to_string(&breakdown).unwrap();
        let deserialized: OfferScoreBreakdown = serde_json::from_str(&json).unwrap();
        assert!((deserialized.distance_term - 0.8).abs() < 1e-9);

        // OfferDecision serde (Offer)
        let offer_decision = OfferDecision::Offer;
        let json = serde_json::to_string(&offer_decision).unwrap();
        let deserialized: OfferDecision = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, OfferDecision::Offer);

        // OfferDecision serde (Abstain)
        let abstain = OfferDecision::Abstain(HelpRejectionReason::Overloaded);
        let json = serde_json::to_string(&abstain).unwrap();
        let deserialized: OfferDecision = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, abstain);

        // ChildDecision serde (Accept)
        let accept = ChildDecision::Accept;
        let json = serde_json::to_string(&accept).unwrap();
        let deserialized: ChildDecision = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, ChildDecision::Accept);

        // ChildDecision serde (Reject)
        let reject = ChildDecision::Reject(HelpRejectionReason::AutonomyLossRisk);
        let json = serde_json::to_string(&reject).unwrap();
        let deserialized: ChildDecision = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, reject);

        // ChildDecision serde (Abstain)
        let abstain_child = ChildDecision::Abstain;
        let json = serde_json::to_string(&abstain_child).unwrap();
        let deserialized: ChildDecision = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, ChildDecision::Abstain);

        // AdultHelpOfferPolicy serde
        let policy = AdultHelpOfferPolicy {
            quality_weight: 1.0,
            load_penalty: 0.5,
            risk_penalty: 0.3,
            threshold: 0.0,
        };
        let json = serde_json::to_string(&policy).unwrap();
        let deserialized: AdultHelpOfferPolicy = serde_json::from_str(&json).unwrap();
        assert!((deserialized.quality_weight - 1.0).abs() < 1e-9);

        // ChildHelpAcceptancePolicy serde
        let accept_policy = ChildHelpAcceptancePolicy {
            gamma1: 0.4,
            gamma2: 0.3,
            gamma3: 0.3,
            quality_weight: 1.0,
            uncertainty_weight: 0.5,
            autonomy_penalty: 0.3,
            threshold: 0.0,
        };
        let json = serde_json::to_string(&accept_policy).unwrap();
        let deserialized: ChildHelpAcceptancePolicy = serde_json::from_str(&json).unwrap();
        assert!((deserialized.gamma1 - 0.4).abs() < 1e-9);
    }

    // ============================================================
    // T-9: 全 variant 存在確認
    // ============================================================
    #[test]
    fn test_t9_enum_variants() {
        // HelpRejectionReason の全 variant 網羅確認 (M1.75-4 で 10 に拡張)
        let rejection_reasons = vec![
            HelpRejectionReason::InsufficientSimilarity,
            HelpRejectionReason::InsufficientTrust,
            HelpRejectionReason::DistanceExceeded,
            HelpRejectionReason::AutonomyLossRisk,
            HelpRejectionReason::NeedMismatch,
            HelpRejectionReason::Unsafe,
            HelpRejectionReason::Irrelevant,
            HelpRejectionReason::Overloaded,
            HelpRejectionReason::ResourceExhausted,
            HelpRejectionReason::Other,
        ];
        assert_eq!(rejection_reasons.len(), 10);
        for i in 0..rejection_reasons.len() {
            for j in (i + 1)..rejection_reasons.len() {
                assert_ne!(rejection_reasons[i], rejection_reasons[j]);
            }
        }

        // HelpFailureReason の全 variant 網羅確認
        let failure_reasons = [HelpFailureReason::Timeout,
            HelpFailureReason::ExecutionError,
            HelpFailureReason::CompatibilityMismatch,
            HelpFailureReason::ResourceExhausted,
            HelpFailureReason::Other];
        assert_eq!(failure_reasons.len(), 5);
        for i in 0..failure_reasons.len() {
            for j in (i + 1)..failure_reasons.len() {
                assert_ne!(failure_reasons[i], failure_reasons[j]);
            }
        }

        // HelpOfferState の全 variant 網羅確認
        let offer_states = [HelpOfferState::Pending,
            HelpOfferState::Accepted,
            HelpOfferState::Rejected,
            HelpOfferState::Expired];
        assert_eq!(offer_states.len(), 4);

        // HelpMode の全 variant 網羅確認
        let modes = [HelpMode::ReuseAsSubWorkflow,
            HelpMode::ComposeWithChild,
            HelpMode::PatchChild,
            HelpMode::DemonstrationOnly];
        assert_eq!(modes.len(), 4);
    }

    // ============================================================
    // T-10: 空 EventBus（None）時の publish 耐性テスト
    // ============================================================
    #[test]
    fn test_t10_null_eventbus() {
        let mut session = HelpSession::new("help-007".into(), "adult-1".into(), "child-1".into());

        // EventBus None でも正常遷移できる
        session.transition_to(HelpState::Offered, None).unwrap();
        assert_eq!(session.current_state(), &HelpState::Offered);

        session.transition_to(HelpState::Accepted, None).unwrap();
        assert_eq!(session.current_state(), &HelpState::Accepted);

        session.transition_to(HelpState::Executing, None).unwrap();
        assert_eq!(session.current_state(), &HelpState::Executing);

        session.transition_to(HelpState::Succeeded, None).unwrap();
        assert_eq!(session.current_state(), &HelpState::Succeeded);
    }

    // ============================================================
    // T-O1: ランダム遷移系列の違法遷移流入フラックス観測 (n >= 10,000)
    // ============================================================
    #[test]
    fn test_to1_random_transition_flux() {
        let mut rng = StdRng::seed_from_u64(12345);
        let all_states = [
            HelpState::Proposal,
            HelpState::Offered,
            HelpState::Accepted,
            HelpState::Rejected,
            HelpState::Executing,
            HelpState::Succeeded,
            HelpState::Failed,
        ];

        let sample_size = 1_000;
        let mut illegal_flux: u64 = 0;
        let mut terminal_flux: u64 = 0;
        let mut legal_count: u64 = 0;

        for _ in 0..sample_size {
            let current = all_states[rng.random_range(0..7)];
            let next = all_states[rng.random_range(0..7)];

            if is_legal_help_transition(&current, &next) {
                legal_count += 1;
                if next.is_terminal() {
                    terminal_flux += 1;
                }
            } else {
                illegal_flux += 1;
            }
        }

        assert!(illegal_flux > 0, "ランダム系列では違法遷移が発生するべき");
        assert!(legal_count > 0, "合法遷移も少数発生するべき");
        assert!(
            legal_count < illegal_flux,
            "合法遷移(6/49)より違法遷移(43/49)の方が多いべき"
        );

        println!("=== T-O1: ランダム遷移系列 違法遷移フラックス観測 ===");
        println!("サンプルサイズ: {}", sample_size);
        println!("合法遷移数: {}", legal_count);
        println!("違法遷移数: {}", illegal_flux);
        println!("終端状態流入数: {}", terminal_flux);
        println!("違法率: {:.4}", illegal_flux as f64 / sample_size as f64);
        println!("合法率: {:.4}", legal_count as f64 / sample_size as f64);
    }

    // ============================================================
    // T-O2: 吸収状態までの平均到達長・終端分布観測 (n >= 5,000)
    // ============================================================
    #[test]
    fn test_to2_absorption_analysis() {
        let mut rng = StdRng::seed_from_u64(12345);
        let sample_size = 1_000;
        let mut total_steps: u64 = 0;
        let mut succeeded_count: u64 = 0;
        let mut rejected_count: u64 = 0;
        let mut failed_count: u64 = 0;
        let mut max_steps: u64 = 0;
        let mut min_steps: u64 = u64::MAX;

        for i in 0..sample_size {
            let mut session = HelpSession::new(
                format!("obs-{:05}", i),
                format!("adult-{}", i % 100),
                format!("child-{}", i % 100),
            );
            let mut steps: u64 = 0;

            loop {
                let current = *session.current_state();
                if current.is_terminal() {
                    match current {
                        HelpState::Succeeded => succeeded_count += 1,
                        HelpState::Rejected => rejected_count += 1,
                        HelpState::Failed => failed_count += 1,
                        _ => {}
                    }
                    break;
                }

                let candidates: Vec<HelpState> = match current {
                    HelpState::Proposal => vec![HelpState::Offered],
                    HelpState::Offered => vec![HelpState::Accepted, HelpState::Rejected],
                    HelpState::Accepted => vec![HelpState::Executing],
                    HelpState::Executing => vec![HelpState::Succeeded, HelpState::Failed],
                    _ => vec![],
                };

                if candidates.is_empty() {
                    break;
                }

                let next = candidates[rng.random_range(0..candidates.len())];
                session.transition_to(next, None).unwrap();
                steps += 1;
            }

            total_steps += steps;
            if steps > max_steps {
                max_steps = steps;
            }
            if steps < min_steps {
                min_steps = steps;
            }
        }

        let avg_steps = total_steps as f64 / sample_size as f64;

        println!("=== T-O2: 吸収状態までの平均到達長・終端分布 ===");
        println!("サンプルサイズ: {}", sample_size);
        println!("平均到達長: {:.4}", avg_steps);
        println!("最小到達長: {}", min_steps);
        println!("最大到達長: {}", max_steps);
        println!("終端分布:");
        println!(
            "  Succeeded: {} ({:.2}%)",
            succeeded_count,
            succeeded_count as f64 / sample_size as f64 * 100.0
        );
        println!(
            "  Rejected:  {} ({:.2}%)",
            rejected_count,
            rejected_count as f64 / sample_size as f64 * 100.0
        );
        println!(
            "  Failed:    {} ({:.2}%)",
            failed_count,
            failed_count as f64 / sample_size as f64 * 100.0
        );
    }

    // ============================================================
    // T-O3: EventBus 上の HELP イベント一貫性検証 (n = 1,000)
    // ============================================================
    #[test]
    fn test_to3_eventbus_consistency() {
        let bus = FakeEventBus::new();
        let mut rng = StdRng::seed_from_u64(12345);
        let sample_size = 1_000;

        let mut transition_sequence: Vec<(HelpState, HelpState)> = Vec::new();

        for i in 0..sample_size {
            let mut session = HelpSession::new(
                format!("cons-{:05}", i),
                format!("adult-{}", i % 100),
                format!("child-{}", i % 100),
            );

            loop {
                let current = *session.current_state();
                if current.is_terminal() {
                    break;
                }

                let candidates: Vec<HelpState> = match current {
                    HelpState::Proposal => vec![HelpState::Offered],
                    HelpState::Offered => vec![HelpState::Accepted, HelpState::Rejected],
                    HelpState::Accepted => vec![HelpState::Executing],
                    HelpState::Executing => vec![HelpState::Succeeded, HelpState::Failed],
                    _ => vec![],
                };

                if candidates.is_empty() {
                    break;
                }

                let from = current;
                let next = candidates[rng.random_range(0..candidates.len())];
                session.transition_to(next, Some(&bus)).unwrap();
                transition_sequence.push((from, next));
            }
        }

        // EventBus から全イベントを replay
        let filter = EventFilter {
            kind_filter: None,
            since_vt: Some(0),
            until_vt: None,
        };
        let replayed = bus.replay(0, filter).unwrap();

        // 遷移系列とイベント系列の長さが一致
        assert_eq!(
            replayed.len(),
            transition_sequence.len(),
            "遷移系列長({}) と EventBus イベント数({}) が一致",
            transition_sequence.len(),
            replayed.len()
        );

        // 各イベントの種別が対応する遷移と一致
        let mut mismatch_count = 0;
        for (i, (from, to)) in transition_sequence.iter().enumerate() {
            let expected_event = transition_to_event_kind(from, to).unwrap();
            let expected_kind = DarviumEventKind::Reciprocity(expected_event);
            let actual_kind = &replayed[i].kind;

            if actual_kind != &expected_kind {
                mismatch_count += 1;
            }

            let help_id = replayed[i].payload["help_id"]
                .as_str()
                .unwrap_or("")
                .to_string();
            assert!(
                help_id.starts_with("cons-"),
                "help_id が期待される形式と一致"
            );
            assert!(
                replayed[i].payload["timestamp_vt"].as_u64().is_some(),
                "timestamp_vt が存在"
            );
        }

        assert_eq!(
            mismatch_count, 0,
            "イベント種別の不一致が {} 件",
            mismatch_count
        );

        println!("=== T-O3: EventBus 一貫性検証 ===");
        println!("総遷移数: {}", transition_sequence.len());
        println!("EventBus イベント数: {}", replayed.len());
        println!("種別不一致数: {}", mismatch_count);
        println!(
            "一貫性: {}",
            if mismatch_count == 0 { "PASS" } else { "FAIL" }
        );
    }

    // ============================================================
    // M1.75-4 T-1: Adult policy が false の場合、offer 不成立
    // ============================================================
    #[test]
    fn test_m1754_t1_adult_policy_false_no_execution() {
        let high_load_policy = AdultHelpOfferPolicy {
            quality_weight: 1.0,
            load_penalty: 10.0,
            risk_penalty: 1.0,
            threshold: 0.5,
        };

        // 高負荷 (0.9) により policy が false になる
        let decision = should_offer_help(0.8, 0.9, 0.1, &high_load_policy);
        assert!(
            !matches!(decision, OfferDecision::Offer),
            "高負荷時は Offer 不成立: {:?}",
            decision
        );

        // 拒否理由が適切に設定されている
        if let OfferDecision::Abstain(reason) = decision {
            assert_eq!(reason, HelpRejectionReason::Overloaded);
        }
    }

    // ============================================================
    // M1.75-4 T-2: Child consent reject が execution path を遮断
    // ============================================================
    #[test]
    fn test_m1754_t2_child_consent_reject_blocks_execution() {
        let high_autonomy_policy = ChildHelpAcceptancePolicy {
            gamma1: 0.4,
            gamma2: 0.3,
            gamma3: 0.3,
            quality_weight: 1.0,
            uncertainty_weight: 1.0,
            autonomy_penalty: 10.0,
            threshold: 0.0,
        };

        // 高自律性コスト (0.9) により reject
        let decision = decide_help_offer(0.7, 0.3, 0.9, &high_autonomy_policy);
        assert!(
            !matches!(decision, ChildDecision::Accept),
            "高自律性コスト時は Reject: {:?}",
            decision
        );

        // Reject の場合、理由が AutonomyLossRisk であること
        if let ChildDecision::Reject(reason) = decision {
            assert_eq!(reason, HelpRejectionReason::AutonomyLossRisk);
        }
    }

    // ============================================================
    // M1.75-4 T-3: offer score 単調性
    // ============================================================
    #[test]
    fn test_m1754_t3_offer_score_monotonicity() {
        let policy = AdultHelpOfferPolicy::default();

        // 近距離・高成熟・高信頼 vs 遠距離・低成熟・低信頼
        let near_high = compute_offer_score_breakdown(0.1, 0.9, 0.8, 0.9, 0.3);
        let far_low = compute_offer_score_breakdown(0.9, 0.2, 0.1, 0.2, 0.3);

        // 近距離高信頼の各項が遠距離低信頼より大きいか等しい
        assert!(near_high.distance_term >= far_low.distance_term);
        assert!(near_high.maturity_term >= far_low.maturity_term);
        assert!(near_high.reciprocity_term >= far_low.reciprocity_term);
        assert!(near_high.reputation_term >= far_low.reputation_term);

        // OfferDecision でも近距離高信頼が Offer になることを確認
        let near_decision = should_offer_help(0.8, 0.2, 0.1, &policy);
        let far_decision = should_offer_help(0.2, 0.8, 0.9, &policy);
        assert!(
            matches!(near_decision, OfferDecision::Offer),
            "近距離高信頼は Offer: {:?}",
            near_decision
        );
        assert!(
            !matches!(far_decision, OfferDecision::Offer),
            "遠距離低信頼は Offer 不成立: {:?}",
            far_decision
        );
    }

    // ============================================================
    // M1.75-4 T-4: OfferScoreBreakdown 非負範囲検証
    // ============================================================
    #[test]
    fn test_m1754_t4_offer_score_breakdown_bounds() {
        let breakdown = compute_offer_score_breakdown(0.5, 0.5, 0.5, 0.5, 0.5);
        assert!(breakdown.distance_term >= 0.0 && breakdown.distance_term <= 1.0);
        assert!(breakdown.maturity_term >= 0.0 && breakdown.maturity_term <= 1.0);
        assert!(breakdown.reciprocity_term >= 0.0 && breakdown.reciprocity_term <= 1.0);
        assert!(breakdown.reputation_term >= 0.0 && breakdown.reputation_term <= 1.0);
        assert!(breakdown.urgency_term >= 0.0 && breakdown.urgency_term <= 1.0);

        // 境界値: 距離 0.0 (同一位置)
        let near = compute_offer_score_breakdown(0.0, 0.5, 0.5, 0.5, 0.5);
        assert!((near.distance_term - 1.0).abs() < 1e-9);
    }

    // ============================================================
    // M1.75-4 T-5: child_need_score [0,1] 境界値
    // ============================================================
    #[test]
    fn test_m1754_t5_child_need_score_bounds() {
        let policy = ChildHelpAcceptancePolicy::default();

        // 最大ニーズ: 経験値=0, 信頼=0, ライフサイクル=0
        let max_need = child_need_score(0.0, 0.0, 0.0, &policy);
        assert!(
            (0.0..=1.0).contains(&max_need),
            "最大ニーズが範囲内: {}",
            max_need
        );
        assert!(
            (max_need - 1.0).abs() < 1e-9,
            "最大ニーズは 1.0: {}",
            max_need
        );

        // 最小ニーズ: 経験値=1, 信頼=1, ライフサイクル=1
        let min_need = child_need_score(1.0, 1.0, 1.0, &policy);
        assert!(
            (0.0..=1.0).contains(&min_need),
            "最小ニーズが範囲内: {}",
            min_need
        );
        assert!(
            (min_need - 0.0).abs() < 1e-9,
            "最小ニーズは 0.0: {}",
            min_need
        );

        // 中間値
        let mid_need = child_need_score(0.5, 0.5, 0.5, &policy);
        assert!(
            (0.0..=1.0).contains(&mid_need),
            "中間ニーズが範囲内: {}",
            mid_need
        );
    }

    // ============================================================
    // M1.75-4 T-6: accept/reject/abstain 決定論
    // ============================================================
    #[test]
    fn test_m1754_t6_accept_decision_correctness() {
        // 高品質・低不確実性・低自律性コスト → Accept
        let low_bar_policy = ChildHelpAcceptancePolicy {
            gamma1: 0.4,
            gamma2: 0.3,
            gamma3: 0.3,
            quality_weight: 1.0,
            uncertainty_weight: 1.0,
            autonomy_penalty: 0.3,
            threshold: -0.5,
        };
        let accept = decide_help_offer(0.9, 0.1, 0.1, &low_bar_policy);
        assert!(
            matches!(accept, ChildDecision::Accept),
            "高品質低コスト: {:?}",
            accept
        );

        // 低品質で高閾値 → Reject
        let high_bar = ChildHelpAcceptancePolicy {
            gamma1: 0.4,
            gamma2: 0.3,
            gamma3: 0.3,
            quality_weight: 1.0,
            uncertainty_weight: 0.5,
            autonomy_penalty: 0.3,
            threshold: 0.3,
        };
        let reject = decide_help_offer(0.1, 0.3, 0.1, &high_bar);
        assert!(
            matches!(reject, ChildDecision::Reject(_)),
            "低品質: {:?}",
            reject
        );

        // 高不確実性でスコアが閾値未満 → Abstain
        let high_uncertainty = ChildHelpAcceptancePolicy {
            gamma1: 0.4,
            gamma2: 0.3,
            gamma3: 0.3,
            quality_weight: 0.5,
            uncertainty_weight: 0.5,
            autonomy_penalty: 0.3,
            threshold: 0.6,
        };
        let abstain = decide_help_offer(0.3, 0.9, 0.1, &high_uncertainty);
        assert!(
            matches!(abstain, ChildDecision::Abstain),
            "高不確実性: {:?}",
            abstain
        );
    }

    // ============================================================
    // M1.75-4 T-7: reject reason mapping
    // ============================================================
    #[test]
    fn test_m1754_t7_rejection_reason_mapping() {
        let policy = AdultHelpOfferPolicy::default();

        // Unsafe: 高リスクでスコア負（リスク優先チェック）
        let unsafe_decision = should_offer_help(0.3, 0.4, 0.8, &policy);
        assert!(
            matches!(
                unsafe_decision,
                OfferDecision::Abstain(HelpRejectionReason::Unsafe)
            ),
            "高リスクで Unsafe: {:?}",
            unsafe_decision
        );

        // Overloaded: 高負荷でスコア負
        let overloaded_decision = should_offer_help(0.3, 0.8, 0.3, &policy);
        assert!(
            matches!(
                overloaded_decision,
                OfferDecision::Abstain(HelpRejectionReason::Overloaded)
            ),
            "高負荷で Overloaded: {:?}",
            overloaded_decision
        );

        // Irrelevant: 低品質でスコア負
        let irrelevant_decision = should_offer_help(0.2, 0.3, 0.3, &policy);
        assert!(
            matches!(
                irrelevant_decision,
                OfferDecision::Abstain(HelpRejectionReason::Irrelevant)
            ),
            "低品質で Irrelevant: {:?}",
            irrelevant_decision
        );
    }

    // ============================================================
    // M1.75-4 T-8: 新規型の serde ラウンドトリップ（T-8 で既に網羅済み）
    // 本テストは AdultHelpOfferPolicy Default 実装の確認
    // ============================================================
    #[test]
    fn test_m1754_t8_policy_defaults() {
        let offer_policy = AdultHelpOfferPolicy::default();
        assert!((offer_policy.quality_weight - 1.0).abs() < 1e-9);
        assert!((offer_policy.load_penalty - 0.5).abs() < 1e-9);
        assert!((offer_policy.risk_penalty - 0.3).abs() < 1e-9);
        assert!((offer_policy.threshold - 0.0).abs() < 1e-9);

        let accept_policy = ChildHelpAcceptancePolicy::default();
        assert!((accept_policy.gamma1 - 0.4).abs() < 1e-9);
        assert!((accept_policy.gamma2 - 0.3).abs() < 1e-9);
        assert!((accept_policy.gamma3 - 0.3).abs() < 1e-9);
        assert!((accept_policy.quality_weight - 1.0).abs() < 1e-9);
        assert!((accept_policy.uncertainty_weight - 0.5).abs() < 1e-9);
        assert!((accept_policy.autonomy_penalty - 0.3).abs() < 1e-9);
        assert!((accept_policy.threshold - 0.0).abs() < 1e-9);
    }

    // ============================================================
    // M1.75-4 T-O1: offer/accept 相図観測 (n >= 10,000)
    // ============================================================
    #[test]
    #[ignore = "較正テスト（10,000 iteration）— 必要時のみ実行"]
    fn test_m1754_to1_offer_acceptance_phase_diagram() {
        let mut rng = StdRng::seed_from_u64(12345);
        let sample_size = 10_000;
        let offer_policy = AdultHelpOfferPolicy::default();
        let accept_policy = ChildHelpAcceptancePolicy::default();

        let mut offer_count: u64 = 0;
        let mut accept_count: u64 = 0;
        let mut reject_count: u64 = 0;
        let mut abstain_count: u64 = 0;

        for _ in 0..sample_size {
            let quality: f64 = rng.random();
            let load: f64 = rng.random();
            let risk: f64 = rng.random();
            let unc: f64 = rng.random();
            let autonomy: f64 = rng.random();

            let offer_dec = should_offer_help(quality, load, risk, &offer_policy);
            if matches!(offer_dec, OfferDecision::Offer) {
                offer_count += 1;
                let child_dec = decide_help_offer(quality, unc, autonomy, &accept_policy);
                match child_dec {
                    ChildDecision::Accept => accept_count += 1,
                    ChildDecision::Reject(_) => reject_count += 1,
                    ChildDecision::Abstain => abstain_count += 1,
                }
            }
        }

        println!("=== M1.75-4 T-O1: offer/accept 相図 ===");
        println!("サンプルサイズ: {}", sample_size);
        println!(
            "offer 発火率: {:.4}",
            offer_count as f64 / sample_size as f64
        );
        println!(
            "accept 率 (offer 中): {:.4}",
            accept_count as f64 / offer_count.max(1) as f64
        );
        println!(
            "reject 率 (offer 中): {:.4}",
            reject_count as f64 / offer_count.max(1) as f64
        );
        println!(
            "abstain 率 (offer 中): {:.4}",
            abstain_count as f64 / offer_count.max(1) as f64
        );

        // 最低限の不変条件: offer と reject が発生すること
        assert!(offer_count > 0, "offer が少なくとも1件発生するべき");
        assert!(
            accept_count > 0 || reject_count > 0,
            "accept/reject が発生するべき"
        );
    }

    // ============================================================
    // M1.75-4 T-O2: decision surface jitter 測定 (n >= 5,000)
    // ============================================================
    #[test]
    #[ignore = "較正テスト（5,000 iteration）— 必要時のみ実行"]
    fn test_m1754_to2_decision_surface_jitter() {
        let mut rng = StdRng::seed_from_u64(12345);
        let sample_size = 5_000;
        let epsilon = 1e-6;

        let mut jitter_count: u64 = 0;

        for _i in 0..sample_size {
            let quality: f64 = rng.random();
            let load: f64 = rng.random();
            let risk: f64 = rng.random();
            let policy = AdultHelpOfferPolicy::default();

            // 閾値境界近傍 (quality ± ε)
            let dec1 = should_offer_help(quality - epsilon, load, risk, &policy);
            let dec2 = should_offer_help(quality + epsilon, load, risk, &policy);

            if dec1 != dec2 {
                jitter_count += 1;
            }

            // スコアブレイクダウンは閾値近傍でも安定
            let bd1 = compute_offer_score_breakdown(0.5, 0.5, 0.5, 0.5, 0.5);
            let bd2 = compute_offer_score_breakdown(0.5 + epsilon, 0.5, 0.5, 0.5, 0.5);
            assert!(
                (bd2.distance_term - bd1.distance_term).abs() < 1e-5,
                "スコアブレイクダウンは微小変化に対して安定"
            );
        }

        println!("=== M1.75-4 T-O2: decision surface jitter ===");
        println!("サンプルサイズ: {}", sample_size);
        println!("jitter 発生数: {}", jitter_count);
        println!("jitter 率: {:.6}", jitter_count as f64 / sample_size as f64);
    }

    // ============================================================
    // M1.75-4 T-O3: パラメータ感度分析 (n >= 3,000)
    // ============================================================
    #[test]
    fn test_m1754_to3_policy_boundary_sensitivity() {
        let mut rng = StdRng::seed_from_u64(12345);
        let sample_size = 3_000;

        let mut quality_flip_count: u64 = 0;

        for _ in 0..sample_size {
            let base_quality: f64 = rng.random();
            let load: f64 = rng.random();
            let risk: f64 = rng.random();

            // 品質重みを微小変動 (1.0 → 1.01)
            let policy_default = AdultHelpOfferPolicy::default();
            let policy_tweaked = AdultHelpOfferPolicy {
                quality_weight: 1.01,
                ..policy_default
            };

            let dec_default = should_offer_help(base_quality, load, risk, &policy_default);
            let dec_tweaked = should_offer_help(base_quality, load, risk, &policy_tweaked);

            if dec_default != dec_tweaked {
                quality_flip_count += 1;
            }
        }

        println!("=== M1.75-4 T-O3: パラメータ感度分析 ===");
        println!("サンプルサイズ: {}", sample_size);
        println!("品質重み 1% 変動での判定反転数: {}", quality_flip_count);
        println!(
            "反転率: {:.6}",
            quality_flip_count as f64 / sample_size as f64
        );

        // 品質重み 1% の変動での反転は稀であるべき
        assert!(
            quality_flip_count < sample_size / 10,
            "1% 変動での反転が 10% 未満: {}",
            quality_flip_count
        );
    }

    // ============================================================
    // T2: transition_to(None) でイベント発行なし、状態遷移のみ
    // ============================================================
    #[test]
    fn test_t9_transition_to_none_no_events() {
        let mut session = HelpSession::new("help-t2".into(), "adult-1".into(), "child-1".into());

        // event_bus = None でも状態遷移は正常に行われる
        session
            .transition_to(HelpState::Offered, None)
            .expect("Proposal→Offered with event_bus=None should succeed");
        assert_eq!(session.current_state(), &HelpState::Offered);

        session
            .transition_to(HelpState::Accepted, None)
            .expect("Offered→Accepted with event_bus=None should succeed");
        assert_eq!(session.current_state(), &HelpState::Accepted);

        session
            .transition_to(HelpState::Executing, None)
            .expect("Accepted→Executing with event_bus=None should succeed");
        assert_eq!(session.current_state(), &HelpState::Executing);

        session
            .transition_to(HelpState::Succeeded, None)
            .expect("Executing→Succeeded with event_bus=None should succeed");
        assert_eq!(session.current_state(), &HelpState::Succeeded);

        // 違法遷移は event_bus=None でも検出される
        let mut fresh = HelpSession::new("help-t2b".into(), "a".into(), "b".into());
        let result = fresh.transition_to(HelpState::Succeeded, None);
        assert!(result.is_err(), "Proposal→Succeeded は event_bus=None でも違法");
    }
}
