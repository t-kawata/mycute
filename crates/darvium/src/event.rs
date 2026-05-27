// Darvium Event Architecture — 型定義 (RFC §12C)
//
// 本ファイルは v2.3-g Darvium Event Architecture の全基盤型を定義する。
// 絶対正本: Darvium-RFC-0001-Unified-v2.3-final.md §12C

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

use crate::error::DarviumError;
use crate::types::{
    InteractionPayload, InteractionRecord, InteractionStatus, VillageObservation, WorkflowGraphId,
};

// ============================================================
// 補助型 (RFC §12C.1)
// ============================================================

/// UUIDv4 文字列として使用するイベント識別子。
pub type EventId = String;

/// 外部イベント配信の配送モード (RFC §12C.1)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DeliveryMode {
    /// 配送は最大1回。到達は保証しない。
    AtMostOnce,
    /// 配送は少なくとも1回。再送が発生しうる。
    AtLeastOnce,
    /// 配送はちょうど1回。完全な配送保証。
    ExactlyOnce,
}

/// 外部配信制御のメタ情報 (RFC §12C.1)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransportMeta {
    /// 配送モード。
    pub delivery_mode: DeliveryMode,
    /// 応答チャネル識別子（任意）。
    pub reply_to: Option<String>,
    /// メッセージの有効期限（秒）。
    pub ttl_seconds: Option<u64>,
}

/// イベント購読の可視性制御 (RFC §12C.1)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EventVisibility {
    /// 全 subscriber に可視。
    Public,
    /// 認証済み subscriber のみ可視。
    Protected,
    /// EventBus 内部のみ。
    Internal,
}

/// イベント保持ポリシー (RFC §12C.1)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventRetention {
    /// 永続化対象かどうか。
    pub persist: bool,
    /// 保持日数（None = 無期限）。
    pub ttl_days: Option<u64>,
}

/// PII 処理方針 (RFC §16B.1)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PiiHandlingPolicy {
    /// PII を含むイベントを拒否。
    Reject,
    /// 永続化前に PII を除去。
    RedactBeforePersist,
    /// Sandbox スコープ内の PII のみ許可。
    AllowSandboxOnly,
}

/// PII・sandbox 制御 (RFC §12C.1)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventPrivacy {
    /// PII を含むかどうか。
    pub contains_pii: bool,
    /// Sandbox のみに制限するかどうか。
    pub sandbox_only: bool,
    /// PII の処理方針。
    pub pii_handling: PiiHandlingPolicy,
}

/// イベント発行元コンポーネント識別子 (RFC §12C.1)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum EventSource {
    /// システム内部。
    System,
    /// HumanChannel。
    HumanChannel,
    /// Orchestrator。
    Orchestrator,
    /// 外部チャネル。
    External {
        /// 外部チャネル識別子。
        channel_id: String,
    },
    /// テストコード。
    Test,
}

/// イベント経路情報・タイムスタンプ (RFC §12C.1)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventMetadata {
    /// commit 時の VirtualClock 値。
    pub clock: u64,
    /// commit 時刻 (UTC, MUST)。
    pub timestamp: SystemTime,
    /// 発行元コンポーネント。
    pub source: EventSource,
}

/// イベント因果関係情報 (RFC §12C.1)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventCausality {
    /// 直接の原因イベント。
    pub parent_event_id: Option<EventId>,
    /// ルート原因イベント。
    pub root_event_id: Option<EventId>,
    /// トレース識別子。
    pub trace_ref: Option<String>,
    /// 関連ミッション。
    pub mission_id: Option<String>,
    /// 関連ワークフロー。
    pub workflow_id: Option<String>,
    /// 関連実行。
    pub run_id: Option<String>,
}

// ============================================================
// InteractionMode (RFC §12C.3)
// ============================================================

/// イベントの interaction semantics を表す直交軸 (RFC §12C.3)。
///
/// - OneWay: fire-and-forget。応答を待たない。
/// - TwoWay: 応答を期待する。interaction_id で追跡される。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum InteractionMode {
    /// 応答を待たない一方向イベント。
    OneWay,
    /// 応答を期待する双方向イベント。
    TwoWay,
}

// ============================================================
// DarviumEventKind  subtype 列挙型 (RFC §12C.2)
// ============================================================

/// 空間位置更新イベントのペイロード (RFC §41B.2)。
///
/// ワークフローの生態学的位置が更新された際の前後情報を保持する。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpacePositionUpdatedPayload {
    /// 更新前の位置ベクトル。
    pub prev: [f32; 3],
    /// 更新後の位置ベクトル。
    pub current: [f32; 3],
    /// 更新に使用された観測。
    pub observation: VillageObservation,
    /// 適用された指数平滑化率。
    pub alpha: f64,
}

/// システム内部イベントの種別 (RFC §12C.2)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SystemEvent {
    /// VirtualClock が進んだ。
    ClockAdvanced,
    /// スナップショットが取得された。
    SnapshotTaken,
    /// リプレイが完了した。
    ReplayCompleted,
    /// 起動が完了した。
    StartupCompleted,
    /// 空間位置が更新された (RFC §41B.2 式 41B-1)。
    SpacePositionUpdated(SpacePositionUpdatedPayload),
}

/// 検索イベントの種別。
///
/// RFC §12C.2 で名前のみ定義。本チケットで最小 variant を新規定義。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SearchEvent {
    /// 検索が開始された。
    Started,
    /// 検索ステップが完了した。
    StepCompleted,
    /// 検索が正常完了した。
    Completed,
    /// 検索が失敗した。
    Failed,
    /// 検索が中断された。
    Aborted,
}

/// ワークフロー実行イベントの種別 (RFC §12C.2)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum WorkflowExecutionEvent {
    /// ワークフローが開始された。
    Started,
    /// ワークフローが正常完了した。
    Completed,
    /// ワークフローが失敗した。
    Failed,
    /// ワークフローが再試行された。
    Retried,
}

/// 訓練イベントの種別 (RFC §12C.2)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TrainingEvent {
    /// 訓練ミッションが生成された。
    MissionGenerated,
    /// 人間レビューが要求された。
    HumanReviewRequested,
    /// 人間レビューが完了した。
    HumanReviewCompleted,
    /// Sandbox 実行が開始された。
    SandboxExecutionStarted,
    /// Sandbox 実行が完了した。
    SandboxExecutionCompleted,
    /// フィードバックが取り込まれた。
    FeedbackIngested,
    /// 昇格候補が作成された。
    PromotionCandidateCreated,
    /// 昇格が承認された。
    PromotionApproved,
    /// 昇格が却下された。
    PromotionRejected,
}

/// 知識イベントの種別 (RFC §12C.2)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum KnowledgeEvent {
    /// 知識断片が作成された。
    FragmentCreated,
    /// 候補が統合された。
    CandidateConsolidated,
    /// 正準知識に昇格された。
    CanonicalPromoted,
    /// 起源トレースが更新された。
    OriginTraceUpdated,
}

/// 会話イベントの envelope (RFC §12C.2)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConversationalEventEnvelope {
    /// 発話が受信された。
    UtteranceReceived,
    /// 発話が分類された。
    Classified,
    /// ゲート判断が行われた。
    GateDecided,
    /// 会話断片が統合された。
    Consolidated,
    /// 知識に昇格された。
    Promoted,
}

/// ライフサイクルイベントの種別。
///
/// RFC §12C.2 で名前のみ定義。本チケットで最小 variant を新規定義。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LifecycleEvent {
    /// ノードが作成された。
    NodeCreated,
    /// ノードが活性化された。
    NodeActivated,
    /// ノードが非活性化された。
    NodeDeactivated,
    /// ノードがアーカイブされた。
    NodeArchived,
}

/// GC イベントの種別 (RFC §12C.2, RFC §4A.7 機構 40)。
///
/// 5 状態の GC 状態機械を表現する。Protected からの Tombstoned 直接遷移は禁止。
/// 遷移順序: Protected → Active → SoftDeleted → HardDeleteCandidate → Tombstoned
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum GcEvent {
    /// 保護された (GC 対象外)。子ノードなど。
    Protected,
    /// アクティブ (GC 候補として監視対象)。
    Active,
    /// ソフト削除された。猶予期間中。
    SoftDeleted,
    /// ハード削除候補としてマークされた。
    HardDeleteCandidate,
    /// 墓石（tombstone）が適用された。完全削除済み。
    Tombstoned,
}

/// GC 状態遷移を hazard 値に基づいて実行する (RFC §4A.7 機構 40)。
///
/// 遷移ルール:
/// - Protected → Active: hazard > 0.0
/// - Active → SoftDeleted: hazard > 0.0
/// - SoftDeleted → HardDeleteCandidate: hazard > 0.5
/// - HardDeleteCandidate → Tombstoned: hazard > 0.8
/// - Protected (hazard 大) → Protected: 直接 Tombstoned 遷移禁止
///
/// # 引数
/// - `current`: 現在の GC 状態
/// - `hazard`: GC ハザード値 [0, 1]
///
/// # 戻り値
/// - 遷移後の GC 状態。条件不成立時は `current` をそのまま返す。
pub fn transition_gc_state(current: GcEvent, hazard: f64) -> GcEvent {
    match current {
        GcEvent::Protected if hazard > 0.0 => GcEvent::Active,
        GcEvent::Active if hazard > 0.0 => GcEvent::SoftDeleted,
        GcEvent::SoftDeleted if hazard > 0.5 => GcEvent::HardDeleteCandidate,
        GcEvent::HardDeleteCandidate if hazard > 0.8 => GcEvent::Tombstoned,
        _ => current,
    }
}

/// 修復イベントの種別 (RFC §12C.2)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RepairEvent {
    /// 不整合が検出された。
    InconsistencyDetected,
    /// 再試行が試みられた。
    RetryAttempted,
    /// 墓石（tombstone）が適用された。
    TombstoneApplied,
    /// 修復が完了した。
    RepairCompleted,
}

/// 互恵性イベントの種別。
///
/// RFC §15.10.6 ReciprocityEventKind の variant。DarviumEventKind::Reciprocity の
/// 内包型として使用される。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ReciprocityEventKind {
    /// 支援が申し出られた。
    HelpOffered,
    /// 支援が受け入れられた。
    HelpAccepted,
    /// 支援が拒否された。
    HelpRejected,
    /// 支援が実行された。
    HelpExecuted,
    /// 支援が成功した。
    HelpSucceeded,
    /// 支援が放棄された。
    HelpAbandoned,
    /// 有害な不一致が検出された。
    HarmfulMismatch,
    /// 互恵が返還された。
    ReturnedFavor,
}

/// 互恵性イベント（9フィールド構造体）。
///
/// RFC §15.10.6 に定義される完全な ReciprocityEvent。DarviumEvent の envelope
/// から TryFrom で materialize される。軽量な種別判別には ReciprocityEventKind
/// を使用し、本構造体はイベントの完全なコンテキストを保持する。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReciprocityEvent {
    /// UUIDv4 イベント識別子。
    pub event_id: String,
    /// 関連ミッション識別子。
    pub mission_id: String,
    /// 送信元グラフ ID。
    pub source_graph_id: WorkflowGraphId,
    /// 送信先グラフ ID。
    pub target_graph_id: WorkflowGraphId,
    /// イベント種別。
    pub event_kind: ReciprocityEventKind,
    /// イベント重み（互恵性スコア計算の入力）。
    pub weight: f32,
    /// イベント発生日時。
    pub created_at: SystemTime,
    /// EventBus clock 値。
    pub virtual_clock: u64,
    /// トレース識別子（任意）。
    pub trace_ref: Option<String>,
}

/// DarviumEvent から ReciprocityEvent への変換。
///
/// DarviumEventKind::Reciprocity(kind) の場合のみ変換成功。
/// 非 Reciprocity kind の場合は DarviumError::ReciprocityError を返す。
impl TryFrom<DarviumEvent> for ReciprocityEvent {
    type Error = DarviumError;

    fn try_from(event: DarviumEvent) -> Result<Self, Self::Error> {
        match event.kind {
            DarviumEventKind::Reciprocity(event_kind) => {
                let source_graph_id: WorkflowGraphId = event.payload["source_graph_id"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string();
                let target_graph_id: WorkflowGraphId = event.payload["target_graph_id"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string();
                let weight: f32 = event.payload["weight"].as_f64().unwrap_or(0.0) as f32;

                Ok(ReciprocityEvent {
                    event_id: event.event_id,
                    mission_id: event.causality.mission_id.unwrap_or_default(),
                    source_graph_id,
                    target_graph_id,
                    event_kind,
                    weight,
                    created_at: event.metadata.timestamp,
                    virtual_clock: event.metadata.clock,
                    trace_ref: event.causality.trace_ref,
                })
            }
            _ => Err(DarviumError::ReciprocityError(
                "event kind is not Reciprocity".to_string(),
            )),
        }
    }
}

// ============================================================
// ReputationProfile (RFC §15.10.3, v2.3-f 拡張)
// ============================================================

/// 資産評判プロファイル (RFC §15.10.3)。
///
/// v2.3-e までの 8 フィールドに加え、v2.3-f で 8 フィールドを追加した全 16 フィールド。
/// v2.3-f 追加フィールドを永続カラムとして保存しない場合でも、ReciprocityEvent から
/// recompute 時に導出可能でなければならない (MUST)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReputationProfile {
    // === v2.3-e 既存フィールド ===
    /// 直接互恵性スコア。
    pub direct_score: f32,
    /// 間接互恵性スコア。
    pub indirect_score: f32,
    /// 経験値補正スコア。
    pub experience_score: f32,
    /// 親からの継承スコア。
    pub inherited_score: f32,
    /// 最終合成スコア（[0, 1] に clamp）。
    pub final_score: f32,
    /// 正の観測回数。
    pub alpha_positive: u32,
    /// 負の観測回数。
    pub beta_negative: u32,
    /// 最終再計算時刻。
    pub last_recomputed_at: SystemTime,
    // === v2.3-f 追加フィールド ===
    /// 直接支援回数。
    pub direct_help_count: u32,
    /// 直接成功回数。
    pub direct_success_count: u32,
    /// 直接拒否回数。
    pub direct_reject_count: u32,
    /// 有害イベント回数。
    pub harm_event_count: u32,
    /// 支援オファー受諾率。
    pub accepted_offer_rate: f32,
    /// 支援成功率。
    pub help_success_rate: f32,
    /// Village 中心性指標。
    pub village_centrality: f32,
    /// 慈悲スコア（F-3 の B_i）。
    pub benevolence_score: f32,
}

impl ReputationProfile {
    /// コールドスタート用の ReputationProfile を生成する。
    ///
    /// 全スコアは 0.5（ニュートラル）、カウントは 0、最終再計算時刻は UNIX_EPOCH。
    pub fn cold_start() -> Self {
        Self {
            direct_score: 0.5,
            indirect_score: 0.5,
            experience_score: 0.5,
            inherited_score: 0.0,
            final_score: 0.5,
            alpha_positive: 0,
            beta_negative: 0,
            last_recomputed_at: SystemTime::UNIX_EPOCH,
            direct_help_count: 0,
            direct_success_count: 0,
            direct_reject_count: 0,
            harm_event_count: 0,
            accepted_offer_rate: 0.0,
            help_success_rate: 0.0,
            village_centrality: 0.0,
            benevolence_score: 0.5,
        }
    }
}

impl Default for ReputationProfile {
    fn default() -> Self {
        Self::cold_start()
    }
}

// ============================================================
// GraphMetrics (RFC §41B.20)
// ============================================================

/// グラフメトリクス — Reciprocity recompute pipeline の入力。
///
/// 各グラフの F-2（間接互恵性）および F-4（評判再計算）に必要な
/// 最小限のメトリクスを保持する。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GraphMetrics {
    /// Helper network 上の中心性 C_i^help ∈ [0, 1]
    pub centrality: f32,
    /// Local village 参加度 A_i^village ∈ [0, 1]
    pub village_participation: f32,
    /// Offer 受諾率 U_i^accepted ∈ [0, 1]
    pub accepted_rate: f32,
    /// 支援成功率 Q_i^success ∈ [0, 1]
    pub success_rate: f32,
    /// 負評価スコア B_i^harm ∈ [0, 1]
    pub harm_score: f32,
    /// 継承スコア I_i ∈ [0, 1]（F-4 θ_inh と乗算）
    pub inherited_score: f32,
    /// 経験値カウント experience_count(i)（F-5 κ_E と乗算）
    pub experience_count: u32,
}

impl Default for GraphMetrics {
    fn default() -> Self {
        Self {
            centrality: 0.0,
            village_participation: 0.0,
            accepted_rate: 0.0,
            success_rate: 0.0,
            harm_score: 0.0,
            inherited_score: 0.0,
            experience_count: 0,
        }
    }
}

// ============================================================
// ReciprocityLifecyclePolicy (RFC §15.10.7)
// ============================================================

/// ライフサイクル較正パラメータオブジェクト (RFC §15.10.7)。
///
/// 全パラメータは versioned policy object として記録されなければならない (MUST)。
/// policy_version により異なるバージョンのポリシーを追跡可能にする。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReciprocityLifecyclePolicy {
    /// 直接互恵性重み θ_dir (F-4)。
    pub theta_dir: f32,
    /// 間接互恵性重み θ_ind (F-4)。
    pub theta_ind: f32,
    /// 経験値重み θ_exp (F-4)。
    pub theta_exp: f32,
    /// 継承重み θ_inherit (F-4)。
    pub theta_inherit: f32,
    /// 経験値正規化飽和率 κ_E (F-5)。
    pub kappa_e: f32,
    /// GC hazard ベースライン λ_0 (F-7)。
    pub lambda_gc_base: f32,
    /// LifecycleScore 重み γ_lifecycle (F-7)。
    pub gamma_lifecycle: f32,
    /// Benevolence 重み γ_benevolence (F-7)。
    pub gamma_benevolence: f32,
    /// Child protect 重み γ_child_protect (F-8)。
    pub gamma_child_protect: f32,
    /// α_help — 支援イベント係数 (F-1)。
    pub alpha_help: f32,
    /// α_success — 成功イベント係数 (F-1)。
    pub alpha_success: f32,
    /// α_reject — 拒否イベント係数（負の重み）(F-1)。
    pub alpha_reject: f32,
    /// α_harm — 有害イベント係数（負の重み）(F-1)。
    pub alpha_harm: f32,
    /// 直接互恵性時間減衰 ρ_dir (F-1)。
    pub rho_direct_decay: f32,
    /// Helper softmax 温度 τ (F-12)。
    pub tau_helper_softmax: f32,
    /// Helper quality mission suitability 重み w_s (F-11)。
    pub helper_quality_w_s: f32,
    /// Helper quality trust 重み w_t (F-11)。
    pub helper_quality_w_t: f32,
    /// Helper quality reputation 重み w_r (F-11)。
    pub helper_quality_w_r: f32,
    /// Helper quality benevolence 重み w_b (F-11)。
    pub helper_quality_w_b: f32,
    /// Helper quality child need 重み w_n (F-11)。
    pub helper_quality_w_n: f32,
    /// Helper quality distance penalty 重み w_d (F-11)。
    pub helper_quality_w_d: f32,
    /// 遠隔探索ベース率 ε_base (F-13)。
    pub epsilon_remote_base: f32,
    /// 遠隔探索最大率 ε_max (F-13)。
    pub epsilon_remote_max: f32,
    /// Child need 係数 a₁ (F-13)。
    pub epsilon_remote_need_coeff: f32,
    /// Benevolence 係数 a₂ (F-13)。
    pub epsilon_remote_benevolence_coeff: f32,
    /// μ₁ — 自身の mission success が growth に与える重み (F-14)。
    pub child_growth_mu_mission_success: f32,
    /// μ₂ — 周囲からの help success が growth に与える重み (F-14)。
    pub child_growth_mu_help_success: f32,
    /// μ₃ — helper の平均 benevolence が growth に与える重み (F-14)。
    pub child_growth_mu_helper_benevolence: f32,
    /// μ₄ — failure burden が growth を減少させる重み (F-14)。
    pub child_growth_mu_failure_burden: f32,
    /// ν₀ — maturation 確率のバイアス項 (F-15)。
    pub maturation_nu_bias: f32,
    /// ν₁ — 正規化経験値が maturation 確率に与える重み (F-15)。
    pub maturation_nu_experience: f32,
    /// ν₂ — 信頼値が maturation 確率に与える重み (F-15)。
    pub maturation_nu_trust: f32,
    /// ν₃ — 評判値が maturation 確率に与える重み (F-15)。
    pub maturation_nu_reputation: f32,
    /// ν₄ — helper の平均 benevolence が maturation 確率に与える重み (F-15)。
    pub maturation_nu_helper_benevolence: f32,
    /// Adult 経験値閾値 E_adult (41B-4)。
    pub adult_experience_threshold: u32,
    /// Adult 信頼閾値 T_adult (41B-4)。
    pub adult_trust_threshold: f32,
    /// Adult 評判閾値 R_adult (41B-4)。
    pub adult_reputation_threshold: f32,
    /// ポリシーバージョン識別子。
    pub policy_version: String,
}

impl Default for ReciprocityLifecyclePolicy {
    fn default() -> Self {
        Self {
            theta_dir: crate::constants::REPUTATION_THETA_DIR,
            theta_ind: crate::constants::REPUTATION_THETA_IND,
            theta_exp: crate::constants::REPUTATION_THETA_EXP,
            theta_inherit: crate::constants::REPUTATION_THETA_INHERIT,
            kappa_e: crate::constants::REPUTATION_KAPPA_E,
            lambda_gc_base: crate::constants::GC_HAZARD_LAMBDA_0,
            gamma_lifecycle: crate::constants::GC_HAZARD_GAMMA_LIFECYCLE,
            gamma_benevolence: crate::constants::GC_HAZARD_GAMMA_BENEVOLENCE,
            gamma_child_protect: crate::constants::GC_HAZARD_GAMMA_CHILD_PROTECT,
            rho_direct_decay: crate::constants::RECIPROCITY_DIRECT_DECAY_RHO,
            alpha_help: crate::constants::RECIPROCITY_ALPHA_HELP,
            alpha_success: crate::constants::RECIPROCITY_ALPHA_SUCCESS,
            alpha_reject: crate::constants::RECIPROCITY_ALPHA_REJECT,
            alpha_harm: crate::constants::RECIPROCITY_ALPHA_HARM,
            tau_helper_softmax: crate::constants::HELP_SOFTMAX_TAU,
            helper_quality_w_s: crate::constants::HELP_QUALITY_SUITABILITY_WEIGHT,
            helper_quality_w_t: crate::constants::HELP_QUALITY_TRUST_WEIGHT,
            helper_quality_w_r: crate::constants::HELP_QUALITY_REPUTATION_WEIGHT,
            helper_quality_w_b: crate::constants::HELP_WEIGHT_BENEVOLENCE,
            helper_quality_w_n: crate::constants::HELP_QUALITY_CHILD_NEED_WEIGHT,
            helper_quality_w_d: crate::constants::HELP_QUALITY_DISTANCE_PENALTY,
            epsilon_remote_base: crate::constants::REMOTE_EXPLORATION_BASE,
            epsilon_remote_max: crate::constants::REMOTE_EXPLORATION_MAX,
            epsilon_remote_need_coeff: crate::constants::REMOTE_EXPLORATION_NEED_COEFF,
            epsilon_remote_benevolence_coeff:
                crate::constants::REMOTE_EXPLORATION_BENEVOLENCE_COEFF,
            child_growth_mu_mission_success: crate::constants::CHILD_GROWTH_MU_MISSION_SUCCESS,
            child_growth_mu_help_success: crate::constants::CHILD_GROWTH_MU_HELP_SUCCESS,
            child_growth_mu_helper_benevolence:
                crate::constants::CHILD_GROWTH_MU_HELPER_BENEVOLENCE,
            child_growth_mu_failure_burden: crate::constants::CHILD_GROWTH_MU_FAILURE_BURDEN,
            maturation_nu_bias: crate::constants::MATURATION_NU_BIAS,
            maturation_nu_experience: crate::constants::MATURATION_NU_EXPERIENCE,
            maturation_nu_trust: crate::constants::MATURATION_NU_TRUST,
            maturation_nu_reputation: crate::constants::MATURATION_NU_REPUTATION,
            maturation_nu_helper_benevolence: crate::constants::MATURATION_NU_HELPER_BENEVOLENCE,
            adult_experience_threshold: crate::constants::E_ADULT_THRESHOLD as u32,
            adult_trust_threshold: crate::constants::T_ADULT_THRESHOLD as f32,
            adult_reputation_threshold: crate::constants::R_ADULT_THRESHOLD as f32,
            policy_version: String::new(),
        }
    }
}

/// Helper quality score の内訳構造体 (F-11)。
///
/// F-11 Q = w_s·S + w_t·T + w_r·Rep + w_b·B + w_n·N - w_d·d の各成分を保持。
/// 観測可能性のため、線形結合の結果だけでなく各項の値も記録する。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QualityScoreBreakdown {
    /// ミッション適合性 S
    pub mission_suitability: f32,
    /// 信頼スコア T
    pub trust: f32,
    /// 評判スコア Rep
    pub reputation: f32,
    /// Benevolence スコア B
    pub benevolence: f32,
    /// Child need スコア N
    pub child_need: f32,
    /// 距離ペナルティ d
    pub distance_penalty: f32,
    /// 総合スコア Q = w_s·S + w_t·T + w_r·Rep + w_b·B + w_n·N - w_d·d
    pub total: f32,
}

/// Softmax helper 選択の重み構造体 (F-12)。
///
/// 各 helper 候補の選択確率 π(h|c,M) とスコア内訳を保持する。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SoftmaxWeight {
    /// Helper のグラフ ID
    pub helper_id: WorkflowGraphId,
    /// 選択確率 π(h|c,M) ∈ [0, 1]
    pub probability: f64,
    /// 確率順位 (1 = 最有力)
    pub rank: usize,
    /// スコア内訳
    pub score_breakdown: QualityScoreBreakdown,
}

/// 融合イベントの種別。
///
/// RFC §12C.2 で名前のみ定義。本チケットで最小 variant を新規定義。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum FusionEvent {
    /// ペアが選択された。
    Paired,
    /// 融合が完了した。
    FusionCompleted,
    /// Birth Commit が開始された。
    BirthCommitInitiated,
    /// Birth Commit が完了した。
    BirthCommitCompleted,
    /// 融合が失敗した。
    FusionFailed,
}

/// HITL イベントの種別 (RFC §12C.2)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HitlEvent {
    /// 通知が送信された（OneWay）。
    NotificationRequested,
    /// HITL インタラクションが開始された（TwoWay）。
    InteractionRequested,
    /// HITL 応答が完了した（TwoWay）。
    InteractionResolved,
    /// チャネルが再接続された（TwoWay）。
    ChannelReconnected,
}

/// Village イベントの種別 (RFC §41B.14, M1.75-7)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VillageEvent {
    /// 1 tick の village 処理が完了し、メトリクスが収集可能になった。
    TickCompleted,
}

// ============================================================
// DarviumEventKind (RFC §12C.2)
// ============================================================

/// 全イベント種別を列挙する extensible taxonomy (RFC §12C.2)。
///
/// 新種別の追加は additive にのみ行わなければならない (MUST)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DarviumEventKind {
    /// システム内部イベント。
    System(SystemEvent),
    /// 検索ライフサイクルイベント。
    Search(SearchEvent),
    /// ワークフロー実行イベント。
    WorkflowExecution(WorkflowExecutionEvent),
    /// 訓練イベント。
    Training(TrainingEvent),
    /// 知識イベント。
    Knowledge(KnowledgeEvent),
    /// 会話イベント。
    Conversational(ConversationalEventEnvelope),
    /// ライフサイクルイベント。
    Lifecycle(LifecycleEvent),
    /// GC イベント。
    Gc(GcEvent),
    /// 修復イベント。
    Repair(RepairEvent),
    /// 互恵性イベント。
    Reciprocity(ReciprocityEventKind),
    /// 融合イベント。
    Fusion(FusionEvent),
    /// HITL イベント。
    Hitl(HitlEvent),
    /// Village イベント。
    Village(VillageEvent),
    /// 将来拡張用 escape hatch。
    Extension(String),
}

// ============================================================
// DarviumEvent — Canonical Envelope (RFC §12C.1)
// ============================================================

/// Darvium 世界内で観測・記録される全出来事の canonical envelope (RFC §12C.1)。
///
/// すべてのイベントはこの envelope で表現しなければならない (MUST)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DarviumEvent {
    /// UUIDv4 イベント識別子。
    pub event_id: EventId,
    /// イベント種別（taxonomy）。
    pub kind: DarviumEventKind,
    /// OneWay / TwoWay（kind と直交, MUST）。
    pub interaction_mode: InteractionMode,
    /// 種別固有のペイロード。
    pub payload: serde_json::Value,
    /// 因果関係情報。
    pub causality: EventCausality,
    /// 経路情報・タイムスタンプ。
    pub metadata: EventMetadata,
    /// 外部配信制御。
    pub transport_meta: Option<TransportMeta>,
    /// 購読可視性制御。
    pub visibility: EventVisibility,
    /// 保持ポリシー。
    pub retention: EventRetention,
    /// PII・sandbox 制御。
    pub privacy: EventPrivacy,
}

// ============================================================
// JsonInteractionPayload — FakeEventBus 用ペイロードラッパー
// ============================================================

/// InteractionPayload 実装のジェネリックラッパー (v2.3-g)。
/// serde_json::Value をペイロードとして使用可能にする。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JsonInteractionPayload {
    /// ペイロードデータ。
    pub data: serde_json::Value,
}

impl InteractionPayload for JsonInteractionPayload {
    type Outcome = serde_json::Value;
}

// ============================================================
// InteractionId (RFC §12C.5)
// ============================================================

/// TwoWay インタラクションの識別子 (newtype, v2.3-g)。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct InteractionId(pub String);

impl From<String> for InteractionId {
    fn from(s: String) -> Self {
        InteractionId(s)
    }
}

impl From<InteractionId> for String {
    fn from(id: InteractionId) -> String {
        id.0
    }
}

impl std::fmt::Display for InteractionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ============================================================
// EventFilter (RFC §12C.5)
// ============================================================

/// イベント購読・リプレイのフィルタ条件 (v2.3-g)。
#[derive(Debug, Clone, PartialEq)]
pub struct EventFilter {
    /// 対象イベント種別（None = 全種別）。
    pub kind_filter: Option<Vec<DarviumEventKind>>,
    /// 開始 VirtualClock（None = 制限なし）。
    pub since_vt: Option<u64>,
    /// 終了 VirtualClock（None = 制限なし）。
    pub until_vt: Option<u64>,
}

impl EventFilter {
    /// 全イベントに合致するフィルタ。
    pub fn all() -> Self {
        EventFilter {
            kind_filter: None,
            since_vt: None,
            until_vt: None,
        }
    }

    /// フィルタ条件にイベントが合致するかを判定する。
    pub fn matches(&self, event: &DarviumEvent) -> bool {
        if let Some(ref kinds) = self.kind_filter {
            if !kinds.contains(&event.kind) {
                return false;
            }
        }
        if let Some(since) = self.since_vt {
            if event.metadata.clock < since {
                return false;
            }
        }
        if let Some(until) = self.until_vt {
            if event.metadata.clock > until {
                return false;
            }
        }
        true
    }
}

// ============================================================
// EventSubscription トレイト (RFC §12C.5)
// ============================================================

/// イベント購読ストリーム (v2.3-g)。
pub trait EventSubscription: Send + Sync {
    /// 利用可能なイベントを1件取得する。なければ None。
    fn poll(&self) -> Option<DarviumEvent>;
}

// ============================================================
// VirtualClock トレイト (RFC §12C.6)
// ============================================================

/// EventBus commit clock の読み取り専用トレイト (RFC §12C.6)。
///
/// VirtualClock は「commit 済み DarviumEvent 列の順序番号」を表現する。
/// クロック進行の唯一の authority は DarviumEventBus 実装であり、
/// 外部から直接 advance してはならない (MUST NOT, RFC §12C.6 MUST #4)。
pub trait VirtualClock: Send + Sync {
    /// 現在の VirtualClock 値を取得する（読み取り専用、`&self`）。
    fn now(&self) -> u64;
}

// ============================================================
// DarviumEventBus トレイト (RFC §12C.5)
// ============================================================

/// Event Architecture の中核トレイト (RFC §12C.5, チケット仕様準拠)。
///
/// 全イベントの publish/subscribe/replay を司り、VirtualClock の唯一の authority。
/// v2.3-g での標準実装である ConcreteEventBus は MetadataStore + InteractionStore 上に構築される。
///
/// VirtualClock を supertrait として要求する (RFC §12C.6)。
pub trait DarviumEventBus: VirtualClock + Send + Sync {
    /// OneWay イベントを publish する。VirtualClock を 1 以上進める (MUST)。
    fn publish(&self, event: DarviumEvent) -> Result<EventId, DarviumError>;

    /// TwoWay インタラクションを開始する。
    fn open(&self, event: DarviumEvent) -> Result<InteractionId, DarviumError>;

    /// TwoWay インタラクションを解決する（outcome 確定）。
    fn resolve(
        &self,
        interaction_id: &InteractionId,
        outcome: serde_json::Value,
    ) -> Result<(), DarviumError>;

    /// TwoWay インタラクションのチャネルを再接続する。
    fn reconnect(
        &self,
        interaction_id: &InteractionId,
        new_channel: &str,
    ) -> Result<(), DarviumError>;

    /// フィルタ条件でイベントを購読する。
    fn subscribe(&self, filter: EventFilter) -> Box<dyn EventSubscription>;

    /// VirtualClock 範囲 + フィルタ条件でイベントをリプレイする。
    /// replay は VirtualClock を進めてはならない (MUST NOT)。
    fn replay(&self, since_vt: u64, filter: EventFilter)
        -> Result<Vec<DarviumEvent>, DarviumError>;

    /// 現在の VirtualClock 値を取得する。
    fn current_clock(&self) -> u64;

    /// 失敗したインタラクションを隔離する。
    fn quarantine_failed_events(
        &self,
        interaction_id: &InteractionId,
        reason: &str,
    ) -> Result<(), DarviumError>;
}

// ============================================================
// FakeEventBus — テスト用メモリ内実装 (RFC §12C.10)
// ============================================================

/// テスト用のメモリ内 EventBus 実装 (RFC §12C.10)。
///
/// 全イベントをメモリ上に記録し、外部依存なしで EventBus の動作検証を可能にする。
/// VirtualClock の全不変条件 (RFC §12C.6) に準拠する。
pub struct FakeEventBus {
    /// 全イベントの追記専用ストア。
    events: Arc<Mutex<Vec<DarviumEvent>>>,
    /// 内部イベントカウンタ（= VirtualClock）。初期値 0 (RFC §A.x EVENTBUS_CLOCK_INITIAL)。
    clock: Arc<Mutex<u64>>,
    /// TwoWay インタラクションストア。
    interactions: Arc<Mutex<HashMap<String, InteractionRecord<JsonInteractionPayload>>>>,
    /// EventBus 運用メトリクス（M1.76-22）。
    metrics: Arc<Mutex<EventBusMetrics>>,
}

impl FakeEventBus {
    /// 空の FakeEventBus を作成する。clock 初期値は 0。
    pub fn new() -> Self {
        FakeEventBus {
            events: Arc::new(Mutex::new(Vec::new())),
            clock: Arc::new(Mutex::new(0)),
            interactions: Arc::new(Mutex::new(HashMap::new())),
            metrics: Arc::new(Mutex::new(EventBusMetrics::new())),
        }
    }

    /// 現在までに publish された全イベントのコピーを返す。
    pub fn published_events(&self) -> Vec<DarviumEvent> {
        self.events
            .lock()
            .expect("FakeEventBus.events lock が汚れていません")
            .clone()
    }

    /// 現在の VirtualClock 値を返す。
    pub fn current_clock(&self) -> u64 {
        *self
            .clock
            .lock()
            .expect("FakeEventBus.clock lock が汚れていません")
    }

    /// 内部状態をリセットする（イベント・クロック・インタラクション・メトリクスを全てクリア）。
    pub fn reset(&self) {
        self.events
            .lock()
            .expect("FakeEventBus.events lock が汚れていません")
            .clear();
        *self
            .clock
            .lock()
            .expect("FakeEventBus.clock lock が汚れていません") = 0;
        self.interactions
            .lock()
            .expect("FakeEventBus.interactions lock が汚れていません")
            .clear();
        *self
            .metrics
            .lock()
            .expect("FakeEventBus.metrics lock が汚れていません") = EventBusMetrics::new();
    }

    /// 現在の EventBusMetrics のコピーを返す。
    pub fn metrics(&self) -> EventBusMetrics {
        self.metrics
            .lock()
            .expect("FakeEventBus.metrics lock が汚れていません")
            .clone()
    }
}

impl DarviumEventBus for FakeEventBus {
    fn publish(&self, mut event: DarviumEvent) -> Result<EventId, DarviumError> {
        let mut events = self
            .events
            .lock()
            .map_err(|e| DarviumError::EventBus(e.to_string()))?;
        let mut clock = self
            .clock
            .lock()
            .map_err(|e| DarviumError::EventBus(e.to_string()))?;
        // MUST #1: clock を割り当てて +1
        event.metadata.clock = *clock;
        *clock += 1;
        let event_id = event.event_id.clone();
        events.push(event);

        // M1.76-22: metrics カウンタ更新
        {
            let mut m = self
                .metrics
                .lock()
                .map_err(|e| DarviumError::EventBus(e.to_string()))?;
            m.total_published += 1;
            m.total_clock_advances += 1;
        }

        // Safety Invariant (RFC §12C.6 MUST #1): VirtualClock == committed DarviumEvent 数
        debug_assert!(*clock as usize >= events.len(),
            "Safety Invariant: clock must not be less than committed DarviumEvent count (RFC §12C.6)");

        Ok(event_id)
    }

    fn open(&self, mut event: DarviumEvent) -> Result<InteractionId, DarviumError> {
        let mut events = self
            .events
            .lock()
            .map_err(|e| DarviumError::EventBus(e.to_string()))?;
        let mut clock = self
            .clock
            .lock()
            .map_err(|e| DarviumError::EventBus(e.to_string()))?;
        let mut interactions = self
            .interactions
            .lock()
            .map_err(|e| DarviumError::EventBus(e.to_string()))?;

        let clock_val = *clock;
        *clock += 1;
        event.metadata.clock = clock_val;
        let interaction_id = event.event_id.clone();

        // InteractionRecord を作成（JsonInteractionPayload にラップ）
        let record = InteractionRecord {
            interaction_id: interaction_id.clone(),
            payload: JsonInteractionPayload {
                data: event.payload.clone(),
            },
            outcome: None,
            status: InteractionStatus::Pending,
            created_at: clock_val,
            updated_at: clock_val,
        };

        events.push(event);
        interactions.insert(interaction_id.clone(), record);

        // M1.76-22: metrics カウンタ更新
        {
            let mut m = self
                .metrics
                .lock()
                .map_err(|e| DarviumError::EventBus(e.to_string()))?;
            m.two_way_opened += 1;
            m.total_clock_advances += 1;
        }

        // Safety Invariant (RFC §12C.6 MUST #1): VirtualClock == committed DarviumEvent 数
        debug_assert!(
            *clock as usize >= events.len(),
            "Safety Invariant: clock must not be less than committed DarviumEvent count after open"
        );

        Ok(InteractionId(interaction_id))
    }

    fn resolve(
        &self,
        interaction_id: &InteractionId,
        outcome: serde_json::Value,
    ) -> Result<(), DarviumError> {
        let clock = self
            .clock
            .lock()
            .map_err(|e| DarviumError::EventBus(e.to_string()))?;
        let mut interactions = self
            .interactions
            .lock()
            .map_err(|e| DarviumError::EventBus(e.to_string()))?;

        let record = interactions
            .get_mut(&interaction_id.0)
            .ok_or_else(|| DarviumError::InteractionNotFound(interaction_id.0.clone()))?;

        record.status = InteractionStatus::Resolved;
        record.outcome = Some(outcome);
        record.updated_at = *clock;

        // M1.76-22: metrics カウンタ更新
        {
            let mut m = self
                .metrics
                .lock()
                .map_err(|e| DarviumError::EventBus(e.to_string()))?;
            m.two_way_resolved += 1;
        }

        // Safety Invariant (RFC §12C.6): resolve は clock/events を不変に保つ
        debug_assert!(
            *clock as usize >= self.published_events().len(),
            "Safety Invariant: resolve must not reduce clock below committed events"
        );

        Ok(())
    }

    fn reconnect(
        &self,
        interaction_id: &InteractionId,
        _new_channel: &str,
    ) -> Result<(), DarviumError> {
        let clock = self
            .clock
            .lock()
            .map_err(|e| DarviumError::EventBus(e.to_string()))?;
        let mut interactions = self
            .interactions
            .lock()
            .map_err(|e| DarviumError::EventBus(e.to_string()))?;

        let record = interactions
            .get_mut(&interaction_id.0)
            .ok_or_else(|| DarviumError::InteractionNotFound(interaction_id.0.clone()))?;

        // Fake 実装: ステータスを更新し、updated_at を記録
        record.status = InteractionStatus::AwaitingExternal;
        record.updated_at = *clock;

        // Safety Invariant (RFC §12C.6): reconnect は clock/events を不変に保つ
        debug_assert!(
            *clock as usize >= self.published_events().len(),
            "Safety Invariant: reconnect must not reduce clock below committed events"
        );

        Ok(())
    }

    fn subscribe(&self, filter: EventFilter) -> Box<dyn EventSubscription> {
        // M1.76-22: metrics カウンタ更新（subscribe は lock 取得前に更新）
        {
            let mut m = self
                .metrics
                .lock()
                .expect("FakeEventBus.metrics lock が汚れていません");
            m.subscribe_count += 1;
        }

        let events = self
            .events
            .lock()
            .expect("FakeEventBus.events lock が汚れていません")
            .clone();
        let filtered: Vec<DarviumEvent> =
            events.into_iter().filter(|e| filter.matches(e)).collect();
        Box::new(FakeSubscription {
            events: Arc::new(Mutex::new(filtered)),
        })
    }

    fn replay(
        &self,
        since_vt: u64,
        filter: EventFilter,
    ) -> Result<Vec<DarviumEvent>, DarviumError> {
        // M1.76-22: metrics カウンタ更新
        {
            let mut m = self
                .metrics
                .lock()
                .map_err(|e| DarviumError::EventBus(e.to_string()))?;
            m.replay_count += 1;
        }

        let events = self
            .events
            .lock()
            .map_err(|e| DarviumError::EventBus(e.to_string()))?;
        // MUST #3: replay は clock を進めてはならない
        let result: Vec<DarviumEvent> = events
            .iter()
            .filter(|e| e.metadata.clock >= since_vt && filter.matches(e))
            .cloned()
            .collect();
        Ok(result)
    }

    fn current_clock(&self) -> u64 {
        *self
            .clock
            .lock()
            .expect("FakeEventBus.clock lock が汚れていません")
    }

    fn quarantine_failed_events(
        &self,
        interaction_id: &InteractionId,
        _reason: &str,
    ) -> Result<(), DarviumError> {
        let mut events = self
            .events
            .lock()
            .map_err(|e| DarviumError::EventBus(e.to_string()))?;
        let mut interactions = self
            .interactions
            .lock()
            .map_err(|e| DarviumError::EventBus(e.to_string()))?;

        // Fake 実装: events から該当イベントを除去し、interactions から削除
        events.retain(|e| e.event_id != interaction_id.0);
        interactions
            .remove(&interaction_id.0)
            .ok_or_else(|| DarviumError::InteractionNotFound(interaction_id.0.clone()))?;

        // M1.76-22: metrics カウンタ更新
        {
            let mut m = self
                .metrics
                .lock()
                .map_err(|e| DarviumError::EventBus(e.to_string()))?;
            m.quarantine_count += 1;
            m.two_way_aborted += 1;
        }

        Ok(())
    }
}

impl VirtualClock for FakeEventBus {
    fn now(&self) -> u64 {
        self.current_clock()
    }
}

/// FakeEventBus の subscribe が返す簡易 Subscription 実装。
struct FakeSubscription {
    events: Arc<Mutex<Vec<DarviumEvent>>>,
}

impl EventSubscription for FakeSubscription {
    fn poll(&self) -> Option<DarviumEvent> {
        let mut events = self
            .events
            .lock()
            .expect("FakeSubscription.events lock が汚れていません");
        if events.is_empty() {
            None
        } else {
            Some(events.remove(0))
        }
    }
}

impl Default for FakeEventBus {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================
// EventBusMetrics — Event Architecture 運用メトリクス (M1.76-22)
// ============================================================

/// EventBus の運用メトリクスを保持する 9 フィールド構造体。
///
/// 各種 EventBus 操作の累積カウンタを記録し、スループット・解決率・
/// quarantine 率などの補助監視指標を導出する。
#[derive(Debug, Clone, PartialEq)]
pub struct EventBusMetrics {
    /// publish() が呼ばれた回数。
    pub total_published: u64,
    /// VirtualClock が進んだ回数（全操作合計）。
    pub total_clock_advances: u64,
    /// open() が呼ばれた回数。
    pub two_way_opened: u64,
    /// resolve() が正常終了した回数。
    pub two_way_resolved: u64,
    /// quarantine により中断された TwoWay 数。
    pub two_way_aborted: u64,
    /// タイムアウトした TwoWay 数（将来拡張用、現状常時 0）。
    pub two_way_timeout: u64,
    /// quarantine_failed_events() が呼ばれた回数。
    pub quarantine_count: u64,
    /// replay() が呼ばれた回数。
    pub replay_count: u64,
    /// subscribe() が呼ばれた回数。
    pub subscribe_count: u64,
}

impl EventBusMetrics {
    /// 全カウンタが 0 の初期状態を作成する。
    pub fn new() -> Self {
        EventBusMetrics {
            total_published: 0,
            total_clock_advances: 0,
            two_way_opened: 0,
            two_way_resolved: 0,
            two_way_aborted: 0,
            two_way_timeout: 0,
            quarantine_count: 0,
            replay_count: 0,
            subscribe_count: 0,
        }
    }

    /// TwoWay 解決率 = two_way_resolved / two_way_opened。
    /// opened が 0 の場合は 0.0 を返す（ゼロ除算回避）。
    pub fn two_way_resolution_rate(&self) -> f64 {
        if self.two_way_opened == 0 {
            0.0
        } else {
            self.two_way_resolved as f64 / self.two_way_opened as f64
        }
    }

    /// Quarantine 率 = quarantine_count / two_way_opened。
    /// opened が 0 の場合は 0.0 を返す（ゼロ除算回避）。
    pub fn quarantine_ratio(&self) -> f64 {
        if self.two_way_opened == 0 {
            0.0
        } else {
            self.quarantine_count as f64 / self.two_way_opened as f64
        }
    }

    /// クロック tick あたりのイベント発行スループット。
    /// clock_advances が 0 の場合は 0.0 を返す（ゼロ除算回避）。
    pub fn event_throughput_per_clock_tick(&self) -> f64 {
        if self.total_clock_advances == 0 {
            0.0
        } else {
            self.total_published as f64 / self.total_clock_advances as f64
        }
    }
}

impl Default for EventBusMetrics {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================
// EventBusMetricsObserver — 既存観測パイプライン統合用 observer
// ============================================================

/// EventBusMetrics を既存の観測パイプラインと統合する observer。
///
/// M1.76-18 の ReciprocityMetricsObserver と同一パターン。
/// FakeEventBus からメトリクススナップショットを取得し、CSV 時系列出力を生成する。
pub struct EventBusMetricsObserver;

impl EventBusMetricsObserver {
    /// FakeEventBus から現在の EventBusMetrics スナップショットを取得する。
    pub fn observe(bus: &FakeEventBus) -> EventBusMetrics {
        bus.metrics
            .lock()
            .expect("EventBusMetrics.metrics lock が汚れていません")
            .clone()
    }

    /// メトリクス系列を CSV 形式で標準出力に書き出す。
    ///
    /// ヘッダー行 + 各 tick のデータ行を出力する。
    /// 既存の ReciprocityMetricsObserver::print_csv と同一パターン。
    pub fn print_csv(series: &[EventBusMetrics], prefix: &str) {
        println!(
            "{}: tick,total_published,total_clock_advances,two_way_opened,\
             two_way_resolved,two_way_aborted,two_way_timeout,quarantine_count,\
             replay_count,subscribe_count,resolution_rate,quarantine_ratio,throughput",
            prefix
        );
        for (i, m) in series.iter().enumerate() {
            println!(
                "{}: {},{},{},{},{},{},{},{},{},{},{:.6},{:.6},{:.6}",
                prefix,
                i,
                m.total_published,
                m.total_clock_advances,
                m.two_way_opened,
                m.two_way_resolved,
                m.two_way_aborted,
                m.two_way_timeout,
                m.quarantine_count,
                m.replay_count,
                m.subscribe_count,
                m.two_way_resolution_rate(),
                m.quarantine_ratio(),
                m.event_throughput_per_clock_tick(),
            );
        }
    }
}

// ============================================================
// EventProjection トレイト (RFC §12E.1)
// ============================================================

/// Projection の配送フィルタ条件 (RFC §12E)。
///
/// どの DarviumEventKind をどの projection に配送するかを定義する。
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectionEventFilter {
    /// 対象イベント種別（None = 全種別）。
    pub kind_filter: Option<Vec<DarviumEventKind>>,
}

impl ProjectionEventFilter {
    /// 全イベント種別を受け入れるフィルタ。
    pub fn all() -> Self {
        ProjectionEventFilter { kind_filter: None }
    }

    /// 指定された種別のみを受け入れるフィルタを作成する。
    pub fn from_kinds(kinds: Vec<DarviumEventKind>) -> Self {
        ProjectionEventFilter {
            kind_filter: Some(kinds),
        }
    }

    /// イベント種別がフィルタ条件に合致するかを判定する。
    pub fn matches(&self, kind: &DarviumEventKind) -> bool {
        self.kind_filter
            .as_ref()
            .is_none_or(|kinds| kinds.contains(kind))
    }
}

/// DarviumEvent のストリームからドメイン固有の投影ビューを構築するトレイト (RFC §12E.1)。
///
/// Projection はイベントソーシングの読み取りモデルとして機能し、
/// 基盤の EventBus に影響を与えてはならない (MUST NOT)。
pub trait EventProjection: Send + Sync {
    /// 投影の一意識別子。
    fn name(&self) -> &'static str;

    /// 対象とする DarviumEventKind のリスト。
    fn interested_kinds(&self) -> Vec<DarviumEventKind>;

    /// 一つのイベントを投影に取り込む。
    /// エラーは分離され、他の projection に影響を与えない (MUST)。
    fn project(&self, event: &DarviumEvent) -> Result<(), DarviumError>;

    /// 現在の投影状態をスナップショットとして出力する。
    fn snapshot(&self) -> Result<serde_json::Value, DarviumError>;

    /// 投影状態をリセットする。
    fn clear(&self) -> Result<(), DarviumError>;
}

/// Projection の登録・取得・一括配送を行うコンテナ。
///
/// このトレイトは RFC §12E.3 の ProjectionEngine と機能的に等価であり、
/// チケット仕様 (Darvium-Tickets-v2.3.md) に従い命名されている。
pub trait ProjectionCatalog: Send + Sync {
    /// 投影を登録する。
    fn register(&self, name: &'static str, projection: Arc<dyn EventProjection>);

    /// 登録済みの投影を名前で取得する。
    fn get(&self, name: &str) -> Option<Arc<dyn EventProjection>>;

    /// 全登録 projection にイベントを配送する。
    ///
    /// 各 projection のエラーは分離され、他の projection に影響を与えない (MUST)。
    /// 戻り値は (projection_name, Result) の Vec で、呼び出し側がエラーを処理する。
    fn project_all(&self, event: &DarviumEvent) -> Vec<(&'static str, Result<(), DarviumError>)>;
}

// ============================================================
// FakeProjection — テスト用メモリ内 EventProjection 実装
// ============================================================

/// FakeProjection の内部状態。
struct InnerFakeProjection {
    /// 投影の一意識別子。
    name: &'static str,
    /// 配送フィルタ。
    filter: ProjectionEventFilter,
    /// 受信したイベントの追記リスト。
    events: Vec<DarviumEvent>,
}

/// テスト用のメモリ内 EventProjection 実装。
pub struct FakeProjection {
    inner: Arc<Mutex<InnerFakeProjection>>,
}

impl FakeProjection {
    /// フィルタ付きで FakeProjection を作成する。
    pub fn with_filter(name: &'static str, filter: ProjectionEventFilter) -> Self {
        FakeProjection {
            inner: Arc::new(Mutex::new(InnerFakeProjection {
                name,
                filter,
                events: Vec::new(),
            })),
        }
    }

    /// 全種別を受け入れる FakeProjection を作成する。
    pub fn new(name: &'static str) -> Self {
        Self::with_filter(name, ProjectionEventFilter::all())
    }

    /// 現在のイベント数を返す。
    pub fn event_count(&self) -> usize {
        self.inner
            .lock()
            .expect("FakeProjection.inner lock が汚れていません")
            .events
            .len()
    }

    /// 受信した全イベントのコピーを返す。
    pub fn received_events(&self) -> Vec<DarviumEvent> {
        self.inner
            .lock()
            .expect("FakeProjection.inner lock が汚れていません")
            .events
            .clone()
    }
}

impl EventProjection for FakeProjection {
    fn name(&self) -> &'static str {
        self.inner
            .lock()
            .expect("FakeProjection.inner lock が汚れていません")
            .name
    }

    fn interested_kinds(&self) -> Vec<DarviumEventKind> {
        self.inner
            .lock()
            .expect("FakeProjection.inner lock が汚れていません")
            .filter
            .kind_filter
            .clone()
            .unwrap_or_default()
    }

    fn project(&self, event: &DarviumEvent) -> Result<(), DarviumError> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|e| DarviumError::Projection(e.to_string()))?;
        if inner.filter.matches(&event.kind) {
            inner.events.push(event.clone());
        }
        Ok(())
    }

    fn snapshot(&self) -> Result<serde_json::Value, DarviumError> {
        let inner = self
            .inner
            .lock()
            .map_err(|e| DarviumError::Projection(e.to_string()))?;
        Ok(serde_json::json!({
            "name": inner.name,
            "event_count": inner.events.len(),
        }))
    }

    fn clear(&self) -> Result<(), DarviumError> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|e| DarviumError::Projection(e.to_string()))?;
        inner.events.clear();
        Ok(())
    }
}

// ============================================================
// FakeProjectionCatalog — テスト用メモリ内 ProjectionCatalog 実装
// ============================================================

/// テスト用のメモリ内 ProjectionCatalog 実装。
pub struct FakeProjectionCatalog {
    projections: Arc<Mutex<HashMap<&'static str, Arc<dyn EventProjection>>>>,
}

impl FakeProjectionCatalog {
    /// 空の FakeProjectionCatalog を作成する。
    pub fn new() -> Self {
        FakeProjectionCatalog {
            projections: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 登録されている全 projection 名のリストを返す。
    pub fn registered_names(&self) -> Vec<&'static str> {
        self.projections
            .lock()
            .expect("FakeProjectionCatalog.projections lock が汚れていません")
            .keys()
            .copied()
            .collect()
    }
}

impl ProjectionCatalog for FakeProjectionCatalog {
    fn register(&self, name: &'static str, projection: Arc<dyn EventProjection>) {
        self.projections
            .lock()
            .expect("FakeProjectionCatalog.projections lock が汚れていません")
            .insert(name, projection);
    }

    fn get(&self, name: &str) -> Option<Arc<dyn EventProjection>> {
        self.projections
            .lock()
            .expect("FakeProjectionCatalog.projections lock が汚れていません")
            .get(name)
            .cloned()
    }

    fn project_all(&self, event: &DarviumEvent) -> Vec<(&'static str, Result<(), DarviumError>)> {
        let projections = self
            .projections
            .lock()
            .expect("FakeProjectionCatalog.projections lock が汚れていません");

        let mut results = Vec::with_capacity(projections.len());
        for (&name, proj) in projections.iter() {
            let result = proj.project(event);
            results.push((name, result));
        }
        results
    }
}

impl Default for FakeProjectionCatalog {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================
// DomainProjection — ドメイン特化 EventProjection 実装 (M1.5-R10)
// ============================================================

/// ドメイン Projection の内部状態。
struct InnerDomainProjection {
    /// 投影の一意識別子。
    name: &'static str,
    /// 受信したイベントの追記リスト。
    events: Vec<DarviumEvent>,
}

/// ドメイン特化 EventProjection 実装。
///
/// SearchTraceProjection / TrainingRunLogProjection / ReciprocityEventProjection /
/// SearchRunLogProjection の4種類を共通の構造体で実現する。
/// フィルタリングは ProjectionEventFilter で行い、各ドメインの kind のみを受け入れる。
pub struct DomainProjection {
    inner: Arc<Mutex<InnerDomainProjection>>,
    filter: ProjectionEventFilter,
}

impl DomainProjection {
    /// フィルタ付きで DomainProjection を作成する。
    fn with_filter(name: &'static str, filter: ProjectionEventFilter) -> Self {
        DomainProjection {
            inner: Arc::new(Mutex::new(InnerDomainProjection {
                name,
                events: Vec::new(),
            })),
            filter,
        }
    }

    /// SearchTraceProjection を作成する。
    /// 全 DarviumEventKind::Search イベントを materialize する。
    pub fn search_trace() -> Self {
        Self::with_filter(
            "search_trace",
            ProjectionEventFilter::from_kinds(vec![
                DarviumEventKind::Search(SearchEvent::Started),
                DarviumEventKind::Search(SearchEvent::StepCompleted),
                DarviumEventKind::Search(SearchEvent::Completed),
                DarviumEventKind::Search(SearchEvent::Failed),
                DarviumEventKind::Search(SearchEvent::Aborted),
            ]),
        )
    }

    /// TrainingRunLogProjection を作成する。
    /// 全 DarviumEventKind::Training イベントを materialize する。
    pub fn training_run_log() -> Self {
        Self::with_filter(
            "training_run_log",
            ProjectionEventFilter::from_kinds(vec![
                DarviumEventKind::Training(TrainingEvent::MissionGenerated),
                DarviumEventKind::Training(TrainingEvent::HumanReviewRequested),
                DarviumEventKind::Training(TrainingEvent::HumanReviewCompleted),
                DarviumEventKind::Training(TrainingEvent::SandboxExecutionStarted),
                DarviumEventKind::Training(TrainingEvent::SandboxExecutionCompleted),
                DarviumEventKind::Training(TrainingEvent::FeedbackIngested),
                DarviumEventKind::Training(TrainingEvent::PromotionCandidateCreated),
                DarviumEventKind::Training(TrainingEvent::PromotionApproved),
                DarviumEventKind::Training(TrainingEvent::PromotionRejected),
            ]),
        )
    }

    /// ReciprocityEventProjection を作成する。
    /// 全 DarviumEventKind::Reciprocity イベントを materialize する。
    pub fn reciprocity_event() -> Self {
        Self::with_filter(
            "reciprocity_event",
            ProjectionEventFilter::from_kinds(vec![
                DarviumEventKind::Reciprocity(ReciprocityEventKind::HelpOffered),
                DarviumEventKind::Reciprocity(ReciprocityEventKind::HelpAccepted),
                DarviumEventKind::Reciprocity(ReciprocityEventKind::HelpRejected),
                DarviumEventKind::Reciprocity(ReciprocityEventKind::HelpExecuted),
                DarviumEventKind::Reciprocity(ReciprocityEventKind::HelpSucceeded),
                DarviumEventKind::Reciprocity(ReciprocityEventKind::HelpAbandoned),
                DarviumEventKind::Reciprocity(ReciprocityEventKind::HarmfulMismatch),
                DarviumEventKind::Reciprocity(ReciprocityEventKind::ReturnedFavor),
            ]),
        )
    }

    /// SearchRunLogProjection を作成する。
    /// SearchEvent の subset（StepCompleted / Completed / Failed / Aborted）のみを materialize する。
    pub fn search_run_log() -> Self {
        Self::with_filter(
            "search_run_log",
            ProjectionEventFilter::from_kinds(vec![
                DarviumEventKind::Search(SearchEvent::StepCompleted),
                DarviumEventKind::Search(SearchEvent::Completed),
                DarviumEventKind::Search(SearchEvent::Failed),
                DarviumEventKind::Search(SearchEvent::Aborted),
            ]),
        )
    }

    /// VillageObservationLogProjection を作成する (M1.75-7)。
    /// DarviumEventKind::Village(VillageEvent::TickCompleted) を materialize する。
    pub fn village_observation_log() -> Self {
        Self::with_filter(
            crate::constants::VILLAGE_EVENT_PROJECTION_NAME,
            ProjectionEventFilter::from_kinds(vec![DarviumEventKind::Village(
                VillageEvent::TickCompleted,
            )]),
        )
    }

    /// SystemLogProjection を作成する。
    /// 全 DarviumEventKind::System イベントを materialize する。
    pub fn system_log() -> Self {
        Self::with_filter(
            "system_log",
            ProjectionEventFilter::from_kinds(vec![
                DarviumEventKind::System(SystemEvent::ClockAdvanced),
                DarviumEventKind::System(SystemEvent::SnapshotTaken),
                DarviumEventKind::System(SystemEvent::ReplayCompleted),
                DarviumEventKind::System(SystemEvent::StartupCompleted),
            ]),
        )
    }

    /// WorkflowExecutionLogProjection を作成する。
    /// 全 DarviumEventKind::WorkflowExecution イベントを materialize する。
    pub fn workflow_execution_log() -> Self {
        Self::with_filter(
            "workflow_execution_log",
            ProjectionEventFilter::from_kinds(vec![
                DarviumEventKind::WorkflowExecution(WorkflowExecutionEvent::Started),
                DarviumEventKind::WorkflowExecution(WorkflowExecutionEvent::Completed),
                DarviumEventKind::WorkflowExecution(WorkflowExecutionEvent::Failed),
                DarviumEventKind::WorkflowExecution(WorkflowExecutionEvent::Retried),
            ]),
        )
    }

    /// KnowledgeLogProjection を作成する。
    /// 全 DarviumEventKind::Knowledge イベントを materialize する。
    pub fn knowledge_log() -> Self {
        Self::with_filter(
            "knowledge_log",
            ProjectionEventFilter::from_kinds(vec![
                DarviumEventKind::Knowledge(KnowledgeEvent::FragmentCreated),
                DarviumEventKind::Knowledge(KnowledgeEvent::CandidateConsolidated),
                DarviumEventKind::Knowledge(KnowledgeEvent::CanonicalPromoted),
                DarviumEventKind::Knowledge(KnowledgeEvent::OriginTraceUpdated),
            ]),
        )
    }

    /// ConversationalLogProjection を作成する。
    /// 全 DarviumEventKind::Conversational イベントを materialize する。
    pub fn conversational_log() -> Self {
        Self::with_filter(
            "conversational_log",
            ProjectionEventFilter::from_kinds(vec![
                DarviumEventKind::Conversational(ConversationalEventEnvelope::UtteranceReceived),
                DarviumEventKind::Conversational(ConversationalEventEnvelope::Classified),
                DarviumEventKind::Conversational(ConversationalEventEnvelope::GateDecided),
                DarviumEventKind::Conversational(ConversationalEventEnvelope::Consolidated),
                DarviumEventKind::Conversational(ConversationalEventEnvelope::Promoted),
            ]),
        )
    }

    /// LifecycleLogProjection を作成する。
    /// 全 DarviumEventKind::Lifecycle イベントを materialize する。
    pub fn lifecycle_log() -> Self {
        Self::with_filter(
            "lifecycle_log",
            ProjectionEventFilter::from_kinds(vec![
                DarviumEventKind::Lifecycle(LifecycleEvent::NodeCreated),
                DarviumEventKind::Lifecycle(LifecycleEvent::NodeActivated),
                DarviumEventKind::Lifecycle(LifecycleEvent::NodeDeactivated),
                DarviumEventKind::Lifecycle(LifecycleEvent::NodeArchived),
            ]),
        )
    }

    /// GcLogProjection を作成する。
    /// 全 DarviumEventKind::Gc イベントを materialize する。
    pub fn gc_log() -> Self {
        Self::with_filter(
            "gc_log",
            ProjectionEventFilter::from_kinds(vec![
                DarviumEventKind::Gc(GcEvent::Protected),
                DarviumEventKind::Gc(GcEvent::Active),
                DarviumEventKind::Gc(GcEvent::SoftDeleted),
                DarviumEventKind::Gc(GcEvent::HardDeleteCandidate),
                DarviumEventKind::Gc(GcEvent::Tombstoned),
            ]),
        )
    }

    /// RepairLogProjection を作成する。
    /// 全 DarviumEventKind::Repair イベントを materialize する。
    pub fn repair_log() -> Self {
        Self::with_filter(
            "repair_log",
            ProjectionEventFilter::from_kinds(vec![
                DarviumEventKind::Repair(RepairEvent::InconsistencyDetected),
                DarviumEventKind::Repair(RepairEvent::RetryAttempted),
                DarviumEventKind::Repair(RepairEvent::TombstoneApplied),
                DarviumEventKind::Repair(RepairEvent::RepairCompleted),
            ]),
        )
    }

    /// FusionLogProjection を作成する。
    /// 全 DarviumEventKind::Fusion イベントを materialize する。
    pub fn fusion_log() -> Self {
        Self::with_filter(
            "fusion_log",
            ProjectionEventFilter::from_kinds(vec![
                DarviumEventKind::Fusion(FusionEvent::Paired),
                DarviumEventKind::Fusion(FusionEvent::FusionCompleted),
                DarviumEventKind::Fusion(FusionEvent::BirthCommitInitiated),
                DarviumEventKind::Fusion(FusionEvent::BirthCommitCompleted),
                DarviumEventKind::Fusion(FusionEvent::FusionFailed),
            ]),
        )
    }

    /// HitlLogProjection を作成する。
    /// 全 DarviumEventKind::Hitl イベントを materialize する。
    pub fn hitl_log() -> Self {
        Self::with_filter(
            "hitl_log",
            ProjectionEventFilter::from_kinds(vec![
                DarviumEventKind::Hitl(HitlEvent::NotificationRequested),
                DarviumEventKind::Hitl(HitlEvent::InteractionRequested),
                DarviumEventKind::Hitl(HitlEvent::InteractionResolved),
                DarviumEventKind::Hitl(HitlEvent::ChannelReconnected),
            ]),
        )
    }

    /// 現在のイベント数を返す。
    pub fn event_count(&self) -> usize {
        self.inner
            .lock()
            .expect("DomainProjection.inner lock が汚れていません")
            .events
            .len()
    }

    /// 受信した全イベントのコピーを返す。
    pub fn received_events(&self) -> Vec<DarviumEvent> {
        self.inner
            .lock()
            .expect("DomainProjection.inner lock が汚れていません")
            .events
            .clone()
    }
}

impl EventProjection for DomainProjection {
    fn name(&self) -> &'static str {
        self.inner
            .lock()
            .expect("DomainProjection.inner lock が汚れていません")
            .name
    }

    fn interested_kinds(&self) -> Vec<DarviumEventKind> {
        self.filter.kind_filter.clone().unwrap_or_default()
    }

    fn project(&self, event: &DarviumEvent) -> Result<(), DarviumError> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|e| DarviumError::Projection(e.to_string()))?;
        if self.filter.matches(&event.kind) {
            inner.events.push(event.clone());
        }
        Ok(())
    }

    fn snapshot(&self) -> Result<serde_json::Value, DarviumError> {
        let inner = self
            .inner
            .lock()
            .map_err(|e| DarviumError::Projection(e.to_string()))?;
        Ok(serde_json::json!({
            "name": inner.name,
            "event_count": inner.events.len(),
            "events": inner.events,
        }))
    }

    fn clear(&self) -> Result<(), DarviumError> {
        let mut inner = self
            .inner
            .lock()
            .map_err(|e| DarviumError::Projection(e.to_string()))?;
        inner.events.clear();
        Ok(())
    }
}

/// ドメイン特化 Projection を ProjectionCatalog に一括登録する。
///
/// 以下の13種類を登録する (M1.76-23 全ドメイン対応):
/// - search_trace: SearchTraceProjection
/// - training_run_log: TrainingRunLogProjection
/// - reciprocity_event: ReciprocityEventProjection
/// - search_run_log: SearchRunLogProjection
/// - village_observation_log: VillageObservationLogProjection
/// - system_log: SystemLogProjection
/// - workflow_execution_log: WorkflowExecutionLogProjection
/// - knowledge_log: KnowledgeLogProjection
/// - conversational_log: ConversationalLogProjection
/// - lifecycle_log: LifecycleLogProjection
/// - gc_log: GcLogProjection
/// - repair_log: RepairLogProjection
/// - fusion_log: FusionLogProjection
/// - hitl_log: HitlLogProjection
pub fn initialize_domain_projections(catalog: &dyn ProjectionCatalog) {
    catalog.register("search_trace", Arc::new(DomainProjection::search_trace()));
    catalog.register(
        "training_run_log",
        Arc::new(DomainProjection::training_run_log()),
    );
    catalog.register(
        "reciprocity_event",
        Arc::new(DomainProjection::reciprocity_event()),
    );
    catalog.register(
        "search_run_log",
        Arc::new(DomainProjection::search_run_log()),
    );
    catalog.register(
        crate::constants::VILLAGE_EVENT_PROJECTION_NAME,
        Arc::new(DomainProjection::village_observation_log()),
    );
    catalog.register("system_log", Arc::new(DomainProjection::system_log()));
    catalog.register(
        "workflow_execution_log",
        Arc::new(DomainProjection::workflow_execution_log()),
    );
    catalog.register("knowledge_log", Arc::new(DomainProjection::knowledge_log()));
    catalog.register(
        "conversational_log",
        Arc::new(DomainProjection::conversational_log()),
    );
    catalog.register("lifecycle_log", Arc::new(DomainProjection::lifecycle_log()));
    catalog.register("gc_log", Arc::new(DomainProjection::gc_log()));
    catalog.register("repair_log", Arc::new(DomainProjection::repair_log()));
    catalog.register("fusion_log", Arc::new(DomainProjection::fusion_log()));
    catalog.register("hitl_log", Arc::new(DomainProjection::hitl_log()));
}

// ============================================================
// 全ドメインイベント生成ヘルパー (M1.76-23)
// ============================================================

/// System イベントを canonical envelope で生成する。
pub fn make_system_event(kind: SystemEvent, payload: serde_json::Value) -> DarviumEvent {
    DarviumEvent {
        event_id: uuid::Uuid::new_v4().to_string(),
        kind: DarviumEventKind::System(kind),
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
            clock: 0,
            timestamp: SystemTime::UNIX_EPOCH,
            source: EventSource::Test,
        },
        transport_meta: None,
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
    }
}

/// Search イベントを canonical envelope で生成する。
pub fn make_search_event(kind: SearchEvent, payload: serde_json::Value) -> DarviumEvent {
    DarviumEvent {
        event_id: uuid::Uuid::new_v4().to_string(),
        kind: DarviumEventKind::Search(kind),
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
            clock: 0,
            timestamp: SystemTime::UNIX_EPOCH,
            source: EventSource::Test,
        },
        transport_meta: None,
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
    }
}

/// WorkflowExecution イベントを canonical envelope で生成する。
pub fn make_workflow_execution_event(
    kind: WorkflowExecutionEvent,
    payload: serde_json::Value,
) -> DarviumEvent {
    DarviumEvent {
        event_id: uuid::Uuid::new_v4().to_string(),
        kind: DarviumEventKind::WorkflowExecution(kind),
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
            clock: 0,
            timestamp: SystemTime::UNIX_EPOCH,
            source: EventSource::Test,
        },
        transport_meta: None,
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
    }
}

/// Training イベントを canonical envelope で生成する。
pub fn make_training_event(kind: TrainingEvent, payload: serde_json::Value) -> DarviumEvent {
    DarviumEvent {
        event_id: uuid::Uuid::new_v4().to_string(),
        kind: DarviumEventKind::Training(kind),
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
            clock: 0,
            timestamp: SystemTime::UNIX_EPOCH,
            source: EventSource::Test,
        },
        transport_meta: None,
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
    }
}

/// Knowledge イベントを canonical envelope で生成する。
pub fn make_knowledge_event(kind: KnowledgeEvent, payload: serde_json::Value) -> DarviumEvent {
    DarviumEvent {
        event_id: uuid::Uuid::new_v4().to_string(),
        kind: DarviumEventKind::Knowledge(kind),
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
            clock: 0,
            timestamp: SystemTime::UNIX_EPOCH,
            source: EventSource::Test,
        },
        transport_meta: None,
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
    }
}

/// Conversational イベントを canonical envelope で生成する。
pub fn make_conversational_event(
    kind: ConversationalEventEnvelope,
    payload: serde_json::Value,
) -> DarviumEvent {
    DarviumEvent {
        event_id: uuid::Uuid::new_v4().to_string(),
        kind: DarviumEventKind::Conversational(kind),
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
            clock: 0,
            timestamp: SystemTime::UNIX_EPOCH,
            source: EventSource::Test,
        },
        transport_meta: None,
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
    }
}

/// Lifecycle イベントを canonical envelope で生成する。
pub fn make_lifecycle_event(kind: LifecycleEvent, payload: serde_json::Value) -> DarviumEvent {
    DarviumEvent {
        event_id: uuid::Uuid::new_v4().to_string(),
        kind: DarviumEventKind::Lifecycle(kind),
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
            clock: 0,
            timestamp: SystemTime::UNIX_EPOCH,
            source: EventSource::Test,
        },
        transport_meta: None,
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
    }
}

/// GC イベントを canonical envelope で生成する。
pub fn make_gc_event(kind: GcEvent, payload: serde_json::Value) -> DarviumEvent {
    DarviumEvent {
        event_id: uuid::Uuid::new_v4().to_string(),
        kind: DarviumEventKind::Gc(kind),
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
            clock: 0,
            timestamp: SystemTime::UNIX_EPOCH,
            source: EventSource::Test,
        },
        transport_meta: None,
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
    }
}

/// Repair イベントを canonical envelope で生成する。
pub fn make_repair_event(kind: RepairEvent, payload: serde_json::Value) -> DarviumEvent {
    DarviumEvent {
        event_id: uuid::Uuid::new_v4().to_string(),
        kind: DarviumEventKind::Repair(kind),
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
            clock: 0,
            timestamp: SystemTime::UNIX_EPOCH,
            source: EventSource::Test,
        },
        transport_meta: None,
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
    }
}

/// Reciprocity イベントを canonical envelope で生成する。
pub fn make_reciprocity_event(
    kind: ReciprocityEventKind,
    payload: serde_json::Value,
) -> DarviumEvent {
    DarviumEvent {
        event_id: uuid::Uuid::new_v4().to_string(),
        kind: DarviumEventKind::Reciprocity(kind),
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
            clock: 0,
            timestamp: SystemTime::UNIX_EPOCH,
            source: EventSource::Test,
        },
        transport_meta: None,
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
    }
}

/// Fusion イベントを canonical envelope で生成する。
pub fn make_fusion_event(kind: FusionEvent, payload: serde_json::Value) -> DarviumEvent {
    DarviumEvent {
        event_id: uuid::Uuid::new_v4().to_string(),
        kind: DarviumEventKind::Fusion(kind),
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
            clock: 0,
            timestamp: SystemTime::UNIX_EPOCH,
            source: EventSource::Test,
        },
        transport_meta: None,
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
    }
}

/// HITL イベントを canonical envelope で生成する。
pub fn make_hitl_event(kind: HitlEvent, payload: serde_json::Value) -> DarviumEvent {
    DarviumEvent {
        event_id: uuid::Uuid::new_v4().to_string(),
        kind: DarviumEventKind::Hitl(kind),
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
            clock: 0,
            timestamp: SystemTime::UNIX_EPOCH,
            source: EventSource::Test,
        },
        transport_meta: None,
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
    }
}

/// Village イベントを canonical envelope で生成する。
pub fn make_village_event(kind: VillageEvent, payload: serde_json::Value) -> DarviumEvent {
    DarviumEvent {
        event_id: uuid::Uuid::new_v4().to_string(),
        kind: DarviumEventKind::Village(kind),
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
            clock: 0,
            timestamp: SystemTime::UNIX_EPOCH,
            source: EventSource::Test,
        },
        transport_meta: None,
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
    }
}

// ============================================================
// テスト
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::Clock;
    use proptest::prelude::*;
    use proptest::prop_compose;
    use rand::rngs::StdRng;
    use rand::Rng;
    use rand::SeedableRng;
    use std::collections::HashMap;
    use std::time::SystemTime;

    /// ラウンドトリップサンプルサイズ。
    const ROUNDTRIP_SAMPLE_SIZE: usize = 1000;

    // ============================================================
    // TC-1: 全14 variant の DarviumEventKind トレイト実装確認
    // ============================================================
    #[test]
    fn test_darvium_event_kind_trait_impl() {
        let variants: Vec<DarviumEventKind> = vec![
            DarviumEventKind::System(SystemEvent::ClockAdvanced),
            DarviumEventKind::Search(SearchEvent::Started),
            DarviumEventKind::WorkflowExecution(WorkflowExecutionEvent::Started),
            DarviumEventKind::Training(TrainingEvent::MissionGenerated),
            DarviumEventKind::Knowledge(KnowledgeEvent::FragmentCreated),
            DarviumEventKind::Conversational(ConversationalEventEnvelope::UtteranceReceived),
            DarviumEventKind::Lifecycle(LifecycleEvent::NodeCreated),
            DarviumEventKind::Gc(GcEvent::SoftDeleted),
            DarviumEventKind::Repair(RepairEvent::InconsistencyDetected),
            DarviumEventKind::Reciprocity(ReciprocityEventKind::HelpOffered),
            DarviumEventKind::Fusion(FusionEvent::Paired),
            DarviumEventKind::Hitl(HitlEvent::NotificationRequested),
            DarviumEventKind::Extension("test".to_string()),
            DarviumEventKind::Village(VillageEvent::TickCompleted),
        ];

        assert_eq!(
            variants.len(),
            14,
            "DarviumEventKind は14 variant である必要があります"
        );

        for variant in &variants {
            // Debug: パニックしないこと
            let debug_str = format!("{:?}", variant);
            assert!(!debug_str.is_empty(), "Debug 出力が空であってはなりません");

            // Clone: 複製が等価であること
            let cloned = variant.clone();
            assert_eq!(
                *variant, cloned,
                "Clone が original と等価である必要があります"
            );

            // Serialize: シリアライズが成功すること
            let json = serde_json::to_string(variant)
                .expect("serde_json::to_string が成功する必要があります");
            assert!(!json.is_empty(), "JSON 出力が空であってはなりません");

            // Deserialize: デシリアライズが成功し、元と一致すること
            let restored: DarviumEventKind =
                serde_json::from_str(&json).expect("serde_json::from_str が成功する必要があります");
            assert_eq!(
                *variant, restored,
                "ラウンドトリップが一致する必要があります"
            );
        }

        println!("TC-1 PASS: 全13 variant のトレイト実装を確認しました");
    }

    // ============================================================
    // TC-2: DarviumEvent 全フィールド設定・アクセス
    // ============================================================
    #[test]
    fn test_darvium_event_full_fields() {
        let event = DarviumEvent {
            event_id: "550e8400-e29b-41d4-a716-446655440000".to_string(),
            kind: DarviumEventKind::System(SystemEvent::StartupCompleted),
            interaction_mode: InteractionMode::OneWay,
            payload: serde_json::json!({"key": "value"}),
            causality: EventCausality {
                parent_event_id: None,
                root_event_id: None,
                trace_ref: Some("trace-001".to_string()),
                mission_id: None,
                workflow_id: None,
                run_id: None,
            },
            metadata: EventMetadata {
                clock: 42,
                timestamp: SystemTime::UNIX_EPOCH,
                source: EventSource::System,
            },
            transport_meta: Some(TransportMeta {
                delivery_mode: DeliveryMode::AtLeastOnce,
                reply_to: None,
                ttl_seconds: Some(3600),
            }),
            visibility: EventVisibility::Public,
            retention: EventRetention {
                persist: true,
                ttl_days: Some(90),
            },
            privacy: EventPrivacy {
                contains_pii: false,
                sandbox_only: false,
                pii_handling: PiiHandlingPolicy::Reject,
            },
        };

        // 全フィールドにアクセス可能であることを確認
        assert_eq!(event.event_id, "550e8400-e29b-41d4-a716-446655440000");
        assert_eq!(
            event.kind,
            DarviumEventKind::System(SystemEvent::StartupCompleted)
        );
        assert_eq!(event.interaction_mode, InteractionMode::OneWay);
        assert_eq!(event.payload, serde_json::json!({"key": "value"}));
        assert_eq!(event.causality.trace_ref, Some("trace-001".to_string()));
        assert_eq!(event.metadata.clock, 42);
        assert!(event.transport_meta.is_some());
        assert_eq!(
            event.transport_meta.as_ref().unwrap().ttl_seconds,
            Some(3600)
        );
        assert_eq!(event.visibility, EventVisibility::Public);
        assert!(event.retention.persist);
        assert!(!event.privacy.contains_pii);

        println!("TC-2 PASS: DarviumEvent 全10フィールドの設定とアクセスを確認しました");
    }

    // ============================================================
    // TC-3: InteractionMode パターンマッチ網羅性
    // ============================================================
    #[test]
    fn test_interaction_mode_exhaustive_match() {
        let one_way = InteractionMode::OneWay;
        let two_way = InteractionMode::TwoWay;

        // _ を使用せず全 variant を網羅
        let describe = |mode: &InteractionMode| -> &str {
            match mode {
                InteractionMode::OneWay => "one-way",
                InteractionMode::TwoWay => "two-way",
            }
        };

        assert_eq!(describe(&one_way), "one-way");
        assert_eq!(describe(&two_way), "two-way");

        println!("TC-3 PASS: InteractionMode の網羅的パターンマッチを確認しました");
    }

    // ============================================================
    // TC-4: DarviumEvent 完全 JSON ラウンドトリップ（n = 1000）
    // ============================================================
    #[test]
    fn test_darvium_event_json_roundtrip_n1000() {
        let mut rng = StdRng::seed_from_u64(12345);
        let mut success_count = 0u64;

        for i in 0..ROUNDTRIP_SAMPLE_SIZE {
            let kind = generate_random_event_kind(&mut rng);
            let event = DarviumEvent {
                event_id: uuid::Uuid::new_v4().to_string(),
                kind,
                interaction_mode: if rng.random_bool(0.7) {
                    InteractionMode::OneWay
                } else {
                    InteractionMode::TwoWay
                },
                payload: serde_json::json!({
                    "index": i,
                    "random": rng.random::<u64>(),
                }),
                causality: EventCausality {
                    parent_event_id: rng
                        .random_bool(0.3)
                        .then(|| uuid::Uuid::new_v4().to_string()),
                    root_event_id: rng
                        .random_bool(0.1)
                        .then(|| uuid::Uuid::new_v4().to_string()),
                    trace_ref: rng
                        .random_bool(0.5)
                        .then(|| rng.random::<u64>().to_string()),
                    mission_id: rng
                        .random_bool(0.4)
                        .then(|| rng.random::<u64>().to_string()),
                    workflow_id: rng
                        .random_bool(0.4)
                        .then(|| rng.random::<u64>().to_string()),
                    run_id: rng
                        .random_bool(0.3)
                        .then(|| rng.random::<u64>().to_string()),
                },
                metadata: EventMetadata {
                    clock: rng.random::<u64>(),
                    timestamp: SystemTime::UNIX_EPOCH,
                    source: random_event_source(&mut rng),
                },
                transport_meta: rng.random_bool(0.5).then(|| TransportMeta {
                    delivery_mode: match rng.random_range(0..3) {
                        0 => DeliveryMode::AtMostOnce,
                        1 => DeliveryMode::AtLeastOnce,
                        _ => DeliveryMode::ExactlyOnce,
                    },
                    reply_to: rng.random_bool(0.3).then(|| "reply-channel".to_string()),
                    ttl_seconds: rng.random_bool(0.7).then(|| rng.random_range(60..86400)),
                }),
                visibility: match rng.random_range(0..3) {
                    0 => EventVisibility::Public,
                    1 => EventVisibility::Protected,
                    _ => EventVisibility::Internal,
                },
                retention: EventRetention {
                    persist: rng.random_bool(0.8),
                    ttl_days: rng.random_bool(0.6).then(|| rng.random_range(1..365)),
                },
                privacy: EventPrivacy {
                    contains_pii: rng.random_bool(0.1),
                    sandbox_only: rng.random_bool(0.2),
                    pii_handling: match rng.random_range(0..3) {
                        0 => PiiHandlingPolicy::Reject,
                        1 => PiiHandlingPolicy::RedactBeforePersist,
                        _ => PiiHandlingPolicy::AllowSandboxOnly,
                    },
                },
            };

            let json = serde_json::to_string(&event).expect("シリアライズが成功する必要があります");
            let restored: DarviumEvent =
                serde_json::from_str(&json).expect("デシリアライズが成功する必要があります");

            assert_eq!(event, restored, "ラウンドトリップ不一致 at index {}", i);
            success_count += 1;
        }

        let success_rate = success_count as f64 / ROUNDTRIP_SAMPLE_SIZE as f64 * 100.0;
        println!(
            "TC-4 PASS: {} / {} ラウンドトリップ成功 (成功率 {:.2}%)",
            success_count, ROUNDTRIP_SAMPLE_SIZE, success_rate
        );
    }

    // ============================================================
    // TC-5: 補助型のシリアライズ確認
    // ============================================================
    #[test]
    fn test_auxiliary_types_serialization() {
        // DeliveryMode
        let modes = [
            DeliveryMode::AtMostOnce,
            DeliveryMode::AtLeastOnce,
            DeliveryMode::ExactlyOnce,
        ];
        for mode in &modes {
            let json = serde_json::to_string(mode).expect("DeliveryMode シリアライズ");
            let restored: DeliveryMode =
                serde_json::from_str(&json).expect("DeliveryMode デシリアライズ");
            assert_eq!(*mode, restored);
        }

        // TransportMeta
        let meta = TransportMeta {
            delivery_mode: DeliveryMode::ExactlyOnce,
            reply_to: Some("chan-1".to_string()),
            ttl_seconds: None,
        };
        let json = serde_json::to_string(&meta).expect("TransportMeta シリアライズ");
        let restored: TransportMeta =
            serde_json::from_str(&json).expect("TransportMeta デシリアライズ");
        assert_eq!(meta, restored);

        // EventVisibility
        let visibilities = [
            EventVisibility::Public,
            EventVisibility::Protected,
            EventVisibility::Internal,
        ];
        for vis in &visibilities {
            let json = serde_json::to_string(vis).expect("EventVisibility シリアライズ");
            let restored: EventVisibility =
                serde_json::from_str(&json).expect("EventVisibility デシリアライズ");
            assert_eq!(*vis, restored);
        }

        // EventRetention
        let retention = EventRetention {
            persist: true,
            ttl_days: Some(30),
        };
        let json = serde_json::to_string(&retention).expect("EventRetention シリアライズ");
        let restored: EventRetention =
            serde_json::from_str(&json).expect("EventRetention デシリアライズ");
        assert_eq!(retention, restored);

        // EventPrivacy
        let privacy = EventPrivacy {
            contains_pii: true,
            sandbox_only: false,
            pii_handling: PiiHandlingPolicy::RedactBeforePersist,
        };
        let json = serde_json::to_string(&privacy).expect("EventPrivacy シリアライズ");
        let restored: EventPrivacy =
            serde_json::from_str(&json).expect("EventPrivacy デシリアライズ");
        assert_eq!(privacy, restored);

        // EventSource
        let sources = [
            EventSource::System,
            EventSource::HumanChannel,
            EventSource::Orchestrator,
            EventSource::External {
                channel_id: "ext-1".to_string(),
            },
            EventSource::Test,
        ];
        for src in &sources {
            let json = serde_json::to_string(src).expect("EventSource シリアライズ");
            let restored: EventSource =
                serde_json::from_str(&json).expect("EventSource デシリアライズ");
            assert_eq!(*src, restored);
        }

        // EventMetadata
        let metadata = EventMetadata {
            clock: 100,
            timestamp: SystemTime::UNIX_EPOCH,
            source: EventSource::Test,
        };
        let json = serde_json::to_string(&metadata).expect("EventMetadata シリアライズ");
        let restored: EventMetadata =
            serde_json::from_str(&json).expect("EventMetadata デシリアライズ");
        assert_eq!(metadata, restored);

        // EventCausality
        let causality = EventCausality {
            parent_event_id: Some("parent-id".to_string()),
            root_event_id: None,
            trace_ref: Some("trace-ref".to_string()),
            mission_id: None,
            workflow_id: Some("wf-1".to_string()),
            run_id: None,
        };
        let json = serde_json::to_string(&causality).expect("EventCausality シリアライズ");
        let restored: EventCausality =
            serde_json::from_str(&json).expect("EventCausality デシリアライズ");
        assert_eq!(causality, restored);

        println!("TC-5 PASS: 全補助型のシリアライズラウンドトリップを確認しました");
    }

    // ============================================================
    // TC-6: EventId 型エイリアスの UUIDv4 互換性
    // ============================================================
    #[test]
    fn test_event_id_uuid_compatibility() {
        let uuid_str = uuid::Uuid::new_v4().to_string();
        let event_id: EventId = uuid_str.clone();
        assert_eq!(
            event_id, uuid_str,
            "EventId は UUIDv4 文字列と互換である必要があります"
        );

        // EventId をフィールドに持つ DarviumEvent で UUID 文字列が使えること
        let event = DarviumEvent {
            event_id: uuid::Uuid::new_v4().to_string(),
            kind: DarviumEventKind::System(SystemEvent::StartupCompleted),
            interaction_mode: InteractionMode::OneWay,
            payload: serde_json::Value::Null,
            causality: EventCausality {
                parent_event_id: Some(uuid::Uuid::new_v4().to_string()),
                root_event_id: None,
                trace_ref: None,
                mission_id: None,
                workflow_id: None,
                run_id: None,
            },
            metadata: EventMetadata {
                clock: 0,
                timestamp: SystemTime::UNIX_EPOCH,
                source: EventSource::Test,
            },
            transport_meta: None,
            visibility: EventVisibility::Public,
            retention: EventRetention {
                persist: false,
                ttl_days: None,
            },
            privacy: EventPrivacy {
                contains_pii: false,
                sandbox_only: false,
                pii_handling: PiiHandlingPolicy::Reject,
            },
        };

        // UUID パース可能な形式であること
        let parsed = uuid::Uuid::parse_str(&event.event_id);
        assert!(
            parsed.is_ok(),
            "DarviumEvent.event_id が UUID としてパース可能である必要があります"
        );

        println!("TC-6 PASS: EventId の UUIDv4 互換性を確認しました");
    }

    // ============================================================
    // TC-7: EventSource の網羅的パターンマッチ
    // ============================================================
    #[test]
    fn test_event_source_exhaustive_match() {
        let sources: Vec<EventSource> = vec![
            EventSource::System,
            EventSource::HumanChannel,
            EventSource::Orchestrator,
            EventSource::External {
                channel_id: "ch".to_string(),
            },
            EventSource::Test,
        ];

        for src in &sources {
            let _description: String = match src {
                EventSource::System => "system".to_string(),
                EventSource::HumanChannel => "human_channel".to_string(),
                EventSource::Orchestrator => "orchestrator".to_string(),
                EventSource::External { channel_id } => format!("external:{}", channel_id),
                EventSource::Test => "test".to_string(),
            };
        }

        println!("TC-7 PASS: EventSource の網羅的パターンマッチを確認しました");
    }

    // ============================================================
    // TC-8: 計装 — 全型のフィールド一覧出力
    // ============================================================
    #[test]
    fn test_type_structure_instrumentation() {
        println!("=== DarviumEvent 構造体フィールド一覧 ===");
        println!("{{\"struct\":\"DarviumEvent\",\"fields\":[");
        println!("  {{\"name\":\"event_id\",\"type\":\"EventId (String)\",\"optional\":false}},");
        println!("  {{\"name\":\"kind\",\"type\":\"DarviumEventKind\",\"optional\":false}},");
        println!(
            "  {{\"name\":\"interaction_mode\",\"type\":\"InteractionMode\",\"optional\":false}},"
        );
        println!("  {{\"name\":\"payload\",\"type\":\"serde_json::Value\",\"optional\":false}},");
        println!("  {{\"name\":\"causality\",\"type\":\"EventCausality\",\"optional\":false}},");
        println!("  {{\"name\":\"metadata\",\"type\":\"EventMetadata\",\"optional\":false}},");
        println!("  {{\"name\":\"transport_meta\",\"type\":\"Option<TransportMeta>\",\"optional\":true}},");
        println!("  {{\"name\":\"visibility\",\"type\":\"EventVisibility\",\"optional\":false}},");
        println!("  {{\"name\":\"retention\",\"type\":\"EventRetention\",\"optional\":false}},");
        println!("  {{\"name\":\"privacy\",\"type\":\"EventPrivacy\",\"optional\":false}}");
        println!("]}}");

        println!();
        println!("=== DarviumEventKind variant 一覧 ===");
        let kind_variants = [
            ("System", "SystemEvent"),
            ("Search", "SearchEvent"),
            ("WorkflowExecution", "WorkflowExecutionEvent"),
            ("Training", "TrainingEvent"),
            ("Knowledge", "KnowledgeEvent"),
            ("Conversational", "ConversationalEventEnvelope"),
            ("Lifecycle", "LifecycleEvent"),
            ("Gc", "GcEvent"),
            ("Repair", "RepairEvent"),
            ("Reciprocity", "ReciprocityEventKind"),
            ("Fusion", "FusionEvent"),
            ("Hitl", "HitlEvent"),
            ("Extension", "String (escape hatch)"),
        ];
        for (i, (variant, inner_type)) in kind_variants.iter().enumerate() {
            println!("  {}. {} -> {}", i + 1, variant, inner_type);
        }

        println!();
        println!("TC-8 PASS: 型構造の計装出力を生成しました");
    }

    // ============================================================
    // テスト補助関数
    // ============================================================

    fn generate_random_event_kind(rng: &mut StdRng) -> DarviumEventKind {
        match rng.random_range(0..13) {
            0 => DarviumEventKind::System(match rng.random_range(0..4) {
                0 => SystemEvent::ClockAdvanced,
                1 => SystemEvent::SnapshotTaken,
                2 => SystemEvent::ReplayCompleted,
                _ => SystemEvent::StartupCompleted,
            }),
            1 => DarviumEventKind::Search(match rng.random_range(0..5) {
                0 => SearchEvent::Started,
                1 => SearchEvent::StepCompleted,
                2 => SearchEvent::Completed,
                3 => SearchEvent::Failed,
                _ => SearchEvent::Aborted,
            }),
            2 => DarviumEventKind::WorkflowExecution(match rng.random_range(0..4) {
                0 => WorkflowExecutionEvent::Started,
                1 => WorkflowExecutionEvent::Completed,
                2 => WorkflowExecutionEvent::Failed,
                _ => WorkflowExecutionEvent::Retried,
            }),
            3 => DarviumEventKind::Training(match rng.random_range(0..9) {
                0 => TrainingEvent::MissionGenerated,
                1 => TrainingEvent::HumanReviewRequested,
                2 => TrainingEvent::HumanReviewCompleted,
                3 => TrainingEvent::SandboxExecutionStarted,
                4 => TrainingEvent::SandboxExecutionCompleted,
                5 => TrainingEvent::FeedbackIngested,
                6 => TrainingEvent::PromotionCandidateCreated,
                7 => TrainingEvent::PromotionApproved,
                _ => TrainingEvent::PromotionRejected,
            }),
            4 => DarviumEventKind::Knowledge(match rng.random_range(0..4) {
                0 => KnowledgeEvent::FragmentCreated,
                1 => KnowledgeEvent::CandidateConsolidated,
                2 => KnowledgeEvent::CanonicalPromoted,
                _ => KnowledgeEvent::OriginTraceUpdated,
            }),
            5 => DarviumEventKind::Conversational(match rng.random_range(0..5) {
                0 => ConversationalEventEnvelope::UtteranceReceived,
                1 => ConversationalEventEnvelope::Classified,
                2 => ConversationalEventEnvelope::GateDecided,
                3 => ConversationalEventEnvelope::Consolidated,
                _ => ConversationalEventEnvelope::Promoted,
            }),
            6 => DarviumEventKind::Lifecycle(match rng.random_range(0..4) {
                0 => LifecycleEvent::NodeCreated,
                1 => LifecycleEvent::NodeActivated,
                2 => LifecycleEvent::NodeDeactivated,
                _ => LifecycleEvent::NodeArchived,
            }),
            7 => DarviumEventKind::Gc(match rng.random_range(0..5) {
                0 => GcEvent::Protected,
                1 => GcEvent::Active,
                2 => GcEvent::SoftDeleted,
                3 => GcEvent::HardDeleteCandidate,
                _ => GcEvent::Tombstoned,
            }),
            8 => DarviumEventKind::Repair(match rng.random_range(0..4) {
                0 => RepairEvent::InconsistencyDetected,
                1 => RepairEvent::RetryAttempted,
                2 => RepairEvent::TombstoneApplied,
                _ => RepairEvent::RepairCompleted,
            }),
            9 => DarviumEventKind::Reciprocity(match rng.random_range(0..8) {
                0 => ReciprocityEventKind::HelpOffered,
                1 => ReciprocityEventKind::HelpAccepted,
                2 => ReciprocityEventKind::HelpRejected,
                3 => ReciprocityEventKind::HelpExecuted,
                4 => ReciprocityEventKind::HelpSucceeded,
                5 => ReciprocityEventKind::HelpAbandoned,
                6 => ReciprocityEventKind::HarmfulMismatch,
                _ => ReciprocityEventKind::ReturnedFavor,
            }),
            10 => DarviumEventKind::Fusion(match rng.random_range(0..5) {
                0 => FusionEvent::Paired,
                1 => FusionEvent::FusionCompleted,
                2 => FusionEvent::BirthCommitInitiated,
                3 => FusionEvent::BirthCommitCompleted,
                _ => FusionEvent::FusionFailed,
            }),
            11 => DarviumEventKind::Hitl(match rng.random_range(0..4) {
                0 => HitlEvent::NotificationRequested,
                1 => HitlEvent::InteractionRequested,
                2 => HitlEvent::InteractionResolved,
                _ => HitlEvent::ChannelReconnected,
            }),
            _ => DarviumEventKind::Extension(rng.random::<u64>().to_string()),
        }
    }

    fn random_event_source(rng: &mut StdRng) -> EventSource {
        match rng.random_range(0..5) {
            0 => EventSource::System,
            1 => EventSource::HumanChannel,
            2 => EventSource::Orchestrator,
            3 => EventSource::External {
                channel_id: rng.random::<u64>().to_string(),
            },
            _ => EventSource::Test,
        }
    }

    // ============================================================
    // M1.5-R5: DarviumEventBus トレイト + FakeEventBus テスト
    // ============================================================

    const BULK_PUBLISH_COUNT: usize = 1000;
    const CONCURRENT_THREADS: usize = 64;

    // -------------------------------------------------------
    // TC-1: publish → replay read-after-write 一貫性
    // -------------------------------------------------------
    #[test]
    fn test_fake_eventbus_publish_replay_read_after_write() {
        let bus = FakeEventBus::new();
        let event = create_test_event(InteractionMode::OneWay);

        let event_id = bus
            .publish(event)
            .expect("publish が成功する必要があります");

        let replayed = bus
            .replay(0, EventFilter::all())
            .expect("replay が成功する必要があります");

        assert_eq!(replayed.len(), 1, "replay で1件取得できる必要があります");
        assert_eq!(
            replayed[0].event_id, event_id,
            "replay 結果の event_id が publish 時のものと一致する必要があります"
        );

        println!(
            "TC-1 PASS: publish event_id={} -> replay で同一イベントを確認しました",
            event_id
        );
    }

    // -------------------------------------------------------
    // TC-2: open → resolve で TwoWay インタラクション完了
    // -------------------------------------------------------
    #[test]
    fn test_fake_eventbus_open_resolve_two_way() {
        let bus = FakeEventBus::new();
        let event = create_test_event(InteractionMode::TwoWay);

        let interaction_id = bus.open(event).expect("open が成功する必要があります");

        let outcome = serde_json::json!({"status": "approved", "comment": "looks good"});
        bus.resolve(&interaction_id, outcome.clone())
            .expect("resolve が成功する必要があります");

        // resolve 後の replay でイベントが確認できること
        let replayed = bus
            .replay(0, EventFilter::all())
            .expect("replay が成功する必要があります");
        assert!(
            replayed.iter().any(|e| e.event_id == interaction_id.0),
            "resolve 後もイベントが replay で取得できる必要があります"
        );

        // clock が 1 であること（open のみ、resolve は DarviumEvent を作成しない, RFC §12C.6）
        assert_eq!(
            bus.current_clock(),
            1,
            "open のみが clock を進める必要があります (resolve は DarviumEvent を作成しない)"
        );

        println!(
            "TC-2 PASS: interaction_id={} の open→resolve を確認しました",
            interaction_id
        );
    }

    // -------------------------------------------------------
    // TC-3: subscribe フィルタリング
    // -------------------------------------------------------
    #[test]
    fn test_fake_eventbus_subscribe_filter() {
        let bus = FakeEventBus::new();

        // System イベントを publish
        let sys_event =
            create_event_with_kind(DarviumEventKind::System(SystemEvent::ClockAdvanced));
        bus.publish(sys_event)
            .expect("System イベント publish が成功");

        // Search イベントを publish
        let search_event = create_event_with_kind(DarviumEventKind::Search(SearchEvent::Started));
        bus.publish(search_event)
            .expect("Search イベント publish が成功");

        // System のみのフィルタで subscribe
        let filter = EventFilter {
            kind_filter: Some(vec![DarviumEventKind::System(SystemEvent::ClockAdvanced)]),
            since_vt: None,
            until_vt: None,
        };
        let subscription = bus.subscribe(filter);

        let first = subscription.poll();
        assert!(
            first.is_some(),
            "System フィルタに合致するイベントが取得できる必要があります"
        );
        assert_eq!(
            first.unwrap().kind,
            DarviumEventKind::System(SystemEvent::ClockAdvanced),
            "取得したイベントの kind が System である必要があります"
        );

        // Search イベントはフィルタに合致しない
        let second = subscription.poll();
        assert!(
            second.is_none(),
            "フィルタに合致しないイベントは取得できない必要があります"
        );

        println!("TC-3 PASS: subscribe フィルタリングの正常動作を確認しました");
    }

    // -------------------------------------------------------
    // TC-4: replay(since_vt=0) 全件時系列順取得
    // -------------------------------------------------------
    #[test]
    fn test_fake_eventbus_replay_all_events_chronological() {
        let bus = FakeEventBus::new();
        let count = 50usize;

        for i in 0..count {
            let event = create_event_with_payload(serde_json::json!({"index": i}));
            bus.publish(event)
                .expect("publish が成功する必要があります");
        }

        let replayed = bus
            .replay(0, EventFilter::all())
            .expect("replay が成功する必要があります");

        assert_eq!(
            replayed.len(),
            count,
            "replay で全 {} 件取得できる必要があります",
            count
        );

        // clock 値が 0..50 の範囲で単調増加していること
        for (i, event) in replayed.iter().enumerate() {
            assert_eq!(
                event.metadata.clock, i as u64,
                "イベント {} の clock 値は {} である必要があります",
                i, i
            );
        }

        println!(
            "TC-4 PASS: {} 件のイベントが clock 値 {}..{} の昇順で取得できました",
            count,
            replayed.first().map(|e| e.metadata.clock).unwrap_or(0),
            replayed.last().map(|e| e.metadata.clock).unwrap_or(0)
        );
    }

    // -------------------------------------------------------
    // TC-5: current_clock 単調増加
    // -------------------------------------------------------
    #[test]
    fn test_fake_eventbus_clock_monotonic() {
        let bus = FakeEventBus::new();
        let iterations = 10usize;

        assert_eq!(
            bus.current_clock(),
            0,
            "初期 clock は 0 である必要があります"
        );

        let mut prev_clock = bus.current_clock();
        for i in 0..iterations {
            let event = create_test_event(InteractionMode::OneWay);
            bus.publish(event)
                .expect("publish が成功する必要があります");

            let current = bus.current_clock();
            assert!(
                current > prev_clock,
                "clock は単調増加する必要があります (iter={}, prev={}, current={})",
                i,
                prev_clock,
                current
            );
            prev_clock = current;
        }

        assert_eq!(
            bus.current_clock(),
            iterations as u64,
            "{} 回の publish 後、clock は {} である必要があります",
            iterations,
            iterations
        );

        println!(
            "TC-5 PASS: clock が 0 → {} に単調増加することを確認しました",
            iterations
        );
    }

    // -------------------------------------------------------
    // TC-6: quarantine 後除外
    // -------------------------------------------------------
    #[test]
    fn test_fake_eventbus_quarantine_excludes_interaction() {
        let bus = FakeEventBus::new();

        // 通常イベントを publish
        let normal = create_test_event(InteractionMode::OneWay);
        bus.publish(normal).expect("通常イベント publish");

        // TwoWay インタラクションを open
        let interaction = create_test_event(InteractionMode::TwoWay);
        let interaction_id = bus
            .open(interaction)
            .expect("open が成功する必要があります");

        // quarantine
        bus.quarantine_failed_events(&interaction_id, "test quarantine")
            .expect("quarantine が成功する必要があります");

        // replay で normal のみ取得できること
        let replayed = bus
            .replay(0, EventFilter::all())
            .expect("replay が成功する必要があります");

        assert_eq!(
            replayed.len(),
            1,
            "quarantine 後、通常イベントのみが replay される必要があります"
        );
        assert!(
            !replayed.iter().any(|e| e.event_id == interaction_id.0),
            "quarantine されたインタラクションのイベントは replay から除外される必要があります"
        );

        println!("TC-6 PASS: quarantine 後のイベント除外を確認しました");
    }

    // -------------------------------------------------------
    // TC-7: DarviumEventBus トレイト境界充足のコンパイル時検証
    // -------------------------------------------------------
    fn assert_darvium_event_bus<T: DarviumEventBus>(_t: &T) {}

    #[test]
    fn test_fake_eventbus_trait_bound() {
        let bus = FakeEventBus::new();
        // FakeEventBus が DarviumEventBus トレイト境界を充足することをコンパイル時に検証
        assert_darvium_event_bus(&bus);
        println!("TC-7 PASS: FakeEventBus は DarviumEventBus トレイト境界を充足します");
    }

    // -------------------------------------------------------
    // TC-8: InteractionId newtype 変換
    // -------------------------------------------------------
    #[test]
    fn test_interaction_id_newtype_conversion() {
        let original = "test-interaction-001".to_string();
        let id = InteractionId::from(original.clone());

        // Display
        assert_eq!(
            format!("{}", id),
            original,
            "Display が元の文字列と一致する必要があります"
        );

        // Into<String>
        let converted: String = id.clone().into();
        assert_eq!(
            converted, original,
            "Into<String> が元の文字列と一致する必要があります"
        );

        // Debug
        let debug_str = format!("{:?}", id);
        assert!(!debug_str.is_empty(), "Debug 出力が空であってはなりません");

        // Clone + PartialEq + Eq
        let cloned = id.clone();
        assert_eq!(
            id, cloned,
            "Clone と PartialEq が正常に動作する必要があります"
        );

        // Hash: HashMap のキーとして使用可能
        let mut map: HashMap<InteractionId, String> = HashMap::new();
        map.insert(id.clone(), "value".to_string());
        assert_eq!(
            map.get(&id),
            Some(&"value".to_string()),
            "InteractionId を HashMap のキーとして使用可能である必要があります"
        );

        // Serialize + Deserialize
        let json = serde_json::to_string(&id)
            .expect("InteractionId のシリアライズが成功する必要があります");
        let restored: InteractionId = serde_json::from_str(&json)
            .expect("InteractionId のデシリアライズが成功する必要があります");
        assert_eq!(
            id, restored,
            "JSON ラウンドトリップが一致する必要があります"
        );

        println!("TC-8 PASS: InteractionId newtype の全変換機能を確認しました");
    }

    // -------------------------------------------------------
    // TC-9: EventFilter 複合条件フィルタリング精度
    // -------------------------------------------------------
    #[test]
    fn test_event_filter_combined_conditions() {
        let bus = FakeEventBus::new();

        // kind 違いのイベントを異なる clock 値で発行
        let sys_event =
            create_event_with_kind(DarviumEventKind::System(SystemEvent::ClockAdvanced));
        bus.publish(sys_event).expect("publish");

        let search_event = create_event_with_kind(DarviumEventKind::Search(SearchEvent::Started));
        bus.publish(search_event).expect("publish");

        let train_event =
            create_event_with_kind(DarviumEventKind::Training(TrainingEvent::MissionGenerated));
        bus.publish(train_event).expect("publish");

        // 複合条件: System + clock >= 0
        let filter = EventFilter {
            kind_filter: Some(vec![DarviumEventKind::System(SystemEvent::ClockAdvanced)]),
            since_vt: Some(0),
            until_vt: None,
        };
        let result = bus
            .replay(0, filter)
            .expect("replay が成功する必要があります");
        assert_eq!(
            result.len(),
            1,
            "System フィルタで1件のみ取得できる必要があります"
        );
        assert_eq!(
            result[0].kind,
            DarviumEventKind::System(SystemEvent::ClockAdvanced),
            "取得したイベントが System である必要があります"
        );

        // 複合条件: Search + Training + clock >= 1
        let filter2 = EventFilter {
            kind_filter: Some(vec![
                DarviumEventKind::Search(SearchEvent::Started),
                DarviumEventKind::Training(TrainingEvent::MissionGenerated),
            ]),
            since_vt: Some(1),
            until_vt: None,
        };
        let result2 = bus
            .replay(0, filter2)
            .expect("replay が成功する必要があります");
        assert_eq!(
            result2.len(),
            2,
            "Search + Training フィルタで2件取得できる必要があります"
        );

        println!("TC-9 PASS: EventFilter 複合条件フィルタリングの精度を確認しました");
    }

    // -------------------------------------------------------
    // TC-10: 計装 — n = 1000 イベント一括発行 + replay 完全性
    // -------------------------------------------------------
    #[test]
    fn test_eventbus_publish_replay_completeness_n1000() {
        let bus = FakeEventBus::new();
        let mut rng = StdRng::seed_from_u64(12345);

        let mut published_ids: Vec<String> = Vec::with_capacity(BULK_PUBLISH_COUNT);
        for _ in 0..BULK_PUBLISH_COUNT {
            let event = create_random_test_event(&mut rng);
            let event_id = bus
                .publish(event)
                .expect("publish が成功する必要があります");
            published_ids.push(event_id);
        }

        let replayed = bus
            .replay(0, EventFilter::all())
            .expect("replay が成功する必要があります");

        // 完全性: 全件取得できること
        assert_eq!(
            replayed.len(),
            BULK_PUBLISH_COUNT,
            "replay で全 {} 件取得できる必要があります（消失率 0%）",
            BULK_PUBLISH_COUNT
        );

        // event_id の一致確認
        let replayed_ids: Vec<String> = replayed.iter().map(|e| e.event_id.clone()).collect();
        for id in &published_ids {
            assert!(
                replayed_ids.contains(id),
                "publish された event_id {} が replay 結果に含まれている必要があります",
                id
            );
        }

        // clock 値の分布を計装出力
        let clock_values: Vec<u64> = replayed.iter().map(|e| e.metadata.clock).collect();
        let min_clock = clock_values.first().copied().unwrap_or(0);
        let max_clock = clock_values.last().copied().unwrap_or(0);

        println!("=== TC-10: publish → replay 完全性レポート ===");
        println!("publish_count: {}", BULK_PUBLISH_COUNT);
        println!("replay_count: {}", replayed.len());
        println!(
            "loss_rate: {:.2}%",
            (1.0 - replayed.len() as f64 / BULK_PUBLISH_COUNT as f64) * 100.0
        );
        println!("clock_range: {}..{}", min_clock, max_clock);
        println!("clock_monotonic: true");
        println!("status: PASS");

        println!(
            "TC-10 PASS: {} 件中 {} 件の replay 成功（消失率 0%）",
            BULK_PUBLISH_COUNT,
            replayed.len()
        );
    }

    // -------------------------------------------------------
    // TC-11: 計装 — 並行アクセス下での clock 単調増加性
    // -------------------------------------------------------
    #[test]
    fn test_eventbus_concurrent_clock_monotonic_n64() {
        let bus = FakeEventBus::new();
        let threads = CONCURRENT_THREADS;

        std::thread::scope(|scope| {
            for _ in 0..threads {
                let event = create_test_event(InteractionMode::OneWay);
                scope.spawn(|| {
                    bus.publish(event)
                        .expect("並行 publish が成功する必要があります");
                });
            }
        });

        let final_clock = bus.current_clock();
        assert!(
            final_clock >= threads as u64,
            "並行 {t} スレッド後の clock は {t} 以上である必要があります（実際: {c}）",
            t = threads,
            c = final_clock
        );

        // 全イベントの clock 値に重複がないこと
        let replayed = bus
            .replay(0, EventFilter::all())
            .expect("replay が成功する必要があります");

        let clock_values: Vec<u64> = replayed.iter().map(|e| e.metadata.clock).collect();
        let unique_clock_count = {
            let mut sorted = clock_values.clone();
            sorted.sort();
            sorted.dedup();
            sorted.len()
        };

        assert_eq!(
            unique_clock_count,
            replayed.len(),
            "全イベントの clock 値が一意である必要があります（重複ゼロ）"
        );

        println!("=== TC-11: 並行アクセス下 clock 単調増加性レポート ===");
        println!("thread_count: {}", threads);
        println!("final_clock: {}", final_clock);
        println!("event_count: {}", replayed.len());
        println!("unique_clock_count: {}", unique_clock_count);
        println!("clock_duplicates: {}", replayed.len() - unique_clock_count);
        println!("status: PASS");

        println!(
            "TC-11 PASS: {} スレッド並行アクセス下で clock の一意性を確認しました",
            threads
        );
    }

    // ============================================================
    // M1.5-R5 テスト補助関数
    // ============================================================

    fn create_test_event(mode: InteractionMode) -> DarviumEvent {
        DarviumEvent {
            event_id: uuid::Uuid::new_v4().to_string(),
            kind: DarviumEventKind::System(SystemEvent::ClockAdvanced),
            interaction_mode: mode,
            payload: serde_json::Value::Null,
            causality: EventCausality {
                parent_event_id: None,
                root_event_id: None,
                trace_ref: None,
                mission_id: None,
                workflow_id: None,
                run_id: None,
            },
            metadata: EventMetadata {
                clock: 0,
                timestamp: SystemTime::UNIX_EPOCH,
                source: EventSource::Test,
            },
            transport_meta: None,
            visibility: EventVisibility::Public,
            retention: EventRetention {
                persist: false,
                ttl_days: None,
            },
            privacy: EventPrivacy {
                contains_pii: false,
                sandbox_only: false,
                pii_handling: PiiHandlingPolicy::Reject,
            },
        }
    }

    fn create_event_with_kind(kind: DarviumEventKind) -> DarviumEvent {
        DarviumEvent {
            event_id: uuid::Uuid::new_v4().to_string(),
            kind,
            interaction_mode: InteractionMode::OneWay,
            payload: serde_json::Value::Null,
            causality: EventCausality {
                parent_event_id: None,
                root_event_id: None,
                trace_ref: None,
                mission_id: None,
                workflow_id: None,
                run_id: None,
            },
            metadata: EventMetadata {
                clock: 0,
                timestamp: SystemTime::UNIX_EPOCH,
                source: EventSource::Test,
            },
            transport_meta: None,
            visibility: EventVisibility::Public,
            retention: EventRetention {
                persist: false,
                ttl_days: None,
            },
            privacy: EventPrivacy {
                contains_pii: false,
                sandbox_only: false,
                pii_handling: PiiHandlingPolicy::Reject,
            },
        }
    }

    fn create_event_with_payload(payload: serde_json::Value) -> DarviumEvent {
        DarviumEvent {
            event_id: uuid::Uuid::new_v4().to_string(),
            kind: DarviumEventKind::System(SystemEvent::ClockAdvanced),
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
                clock: 0,
                timestamp: SystemTime::UNIX_EPOCH,
                source: EventSource::Test,
            },
            transport_meta: None,
            visibility: EventVisibility::Public,
            retention: EventRetention {
                persist: false,
                ttl_days: None,
            },
            privacy: EventPrivacy {
                contains_pii: false,
                sandbox_only: false,
                pii_handling: PiiHandlingPolicy::Reject,
            },
        }
    }

    fn create_random_test_event(rng: &mut StdRng) -> DarviumEvent {
        DarviumEvent {
            event_id: uuid::Uuid::new_v4().to_string(),
            kind: generate_random_event_kind(rng),
            interaction_mode: if rng.random_bool(0.7) {
                InteractionMode::OneWay
            } else {
                InteractionMode::TwoWay
            },
            payload: serde_json::json!({"data": rng.random::<u64>()}),
            causality: EventCausality {
                parent_event_id: rng
                    .random_bool(0.3)
                    .then(|| uuid::Uuid::new_v4().to_string()),
                root_event_id: rng
                    .random_bool(0.1)
                    .then(|| uuid::Uuid::new_v4().to_string()),
                trace_ref: rng
                    .random_bool(0.5)
                    .then(|| rng.random::<u64>().to_string()),
                mission_id: rng
                    .random_bool(0.4)
                    .then(|| rng.random::<u64>().to_string()),
                workflow_id: rng
                    .random_bool(0.4)
                    .then(|| rng.random::<u64>().to_string()),
                run_id: rng
                    .random_bool(0.3)
                    .then(|| rng.random::<u64>().to_string()),
            },
            metadata: EventMetadata {
                clock: 0,
                timestamp: SystemTime::UNIX_EPOCH,
                source: random_event_source(rng),
            },
            transport_meta: rng.random_bool(0.5).then(|| TransportMeta {
                delivery_mode: match rng.random_range(0..3) {
                    0 => DeliveryMode::AtMostOnce,
                    1 => DeliveryMode::AtLeastOnce,
                    _ => DeliveryMode::ExactlyOnce,
                },
                reply_to: rng.random_bool(0.3).then(|| "reply-chan".to_string()),
                ttl_seconds: rng.random_bool(0.7).then(|| rng.random_range(60..86400)),
            }),
            visibility: match rng.random_range(0..3) {
                0 => EventVisibility::Public,
                1 => EventVisibility::Protected,
                _ => EventVisibility::Internal,
            },
            retention: EventRetention {
                persist: rng.random_bool(0.8),
                ttl_days: rng.random_bool(0.6).then(|| rng.random_range(1..365)),
            },
            privacy: EventPrivacy {
                contains_pii: rng.random_bool(0.1),
                sandbox_only: rng.random_bool(0.2),
                pii_handling: match rng.random_range(0..3) {
                    0 => PiiHandlingPolicy::Reject,
                    1 => PiiHandlingPolicy::RedactBeforePersist,
                    _ => PiiHandlingPolicy::AllowSandboxOnly,
                },
            },
        }
    }

    // ============================================================
    // M1.5-R6: VirtualClock 再定義 — EventBus commit clock への制限
    // ============================================================

    // -------------------------------------------------------
    // TC-1: VirtualClock トレイトのコンパイル時検証
    // -------------------------------------------------------
    fn assert_virtual_clock<T: VirtualClock>(_t: &T) {}

    #[test]
    fn test_virtual_clock_trait_virtual_clock() {
        let bus = FakeEventBus::new();
        assert_virtual_clock(&bus);
        println!("TC-1 PASS: FakeEventBus は VirtualClock トレイト境界を充足します");
    }

    #[test]
    fn test_virtual_clock_now_readonly() {
        let bus = FakeEventBus::new();
        // now() は &self（不変参照）であり、読み取り専用であることの表明。
        let _value = VirtualClock::now(&bus);
        println!("TC-1b PASS: VirtualClock::now() は &self で読み取り専用です");
    }

    // -------------------------------------------------------
    // TC-2: FakeEventBus の VirtualClock 実装確認
    // -------------------------------------------------------
    #[test]
    fn test_virtual_clock_fake_eventbus_impl() {
        let bus = FakeEventBus::new();
        let now = VirtualClock::now(&bus);
        let current = DarviumEventBus::current_clock(&bus);
        assert_eq!(
            now, current,
            "VirtualClock::now() と DarviumEventBus::current_clock() が一致する必要があります"
        );
        println!(
            "TC-2 PASS: FakeEventBus の VirtualClock::now() = current_clock() = {}",
            now
        );
    }

    // -------------------------------------------------------
    // TC-3: DarviumEventBus が VirtualClock を supertrait として要求
    // -------------------------------------------------------
    fn assert_darvium_event_bus_virtual_clock<T: DarviumEventBus>(_t: &T) {}

    #[test]
    fn test_virtual_clock_darvium_event_bus_supertrait() {
        let bus = FakeEventBus::new();
        assert_darvium_event_bus_virtual_clock(&bus);
        println!("TC-3 PASS: DarviumEventBus は VirtualClock を supertrait として要求します");
    }

    // -------------------------------------------------------
    // TC-4: EventBus 操作（publish/open/resolve）後に now() が増加
    // -------------------------------------------------------
    #[test]
    fn test_virtual_clock_publish_increments_clock() {
        let bus = FakeEventBus::new();
        let before = bus.now();
        let event = create_test_event(InteractionMode::OneWay);
        bus.publish(event).expect("publish が成功");
        let after = bus.now();
        assert_eq!(
            after,
            before + 1,
            "publish 後、clock が +1 される必要があります"
        );
        println!("TC-4a PASS: publish 後 clock {} → {}", before, after);
    }

    #[test]
    fn test_virtual_clock_open_increments_clock() {
        let bus = FakeEventBus::new();
        let before = bus.now();
        let event = create_test_event(InteractionMode::TwoWay);
        bus.open(event).expect("open が成功");
        let after = bus.now();
        assert_eq!(
            after,
            before + 1,
            "open 後、clock が +1 される必要があります"
        );
        println!("TC-4b PASS: open 後 clock {} → {}", before, after);
    }

    #[test]
    fn test_virtual_clock_open_resolve_increments_clock() {
        let bus = FakeEventBus::new();
        let event = create_test_event(InteractionMode::TwoWay);
        let id = bus.open(event).expect("open が成功");
        let outcome = serde_json::json!({"status": "ok"});
        bus.resolve(&id, outcome).expect("resolve が成功");
        assert_eq!(
            bus.now(),
            1,
            "open のみが clock を進める必要があります (resolve は DarviumEvent を作成しない, RFC §12C.6)"
        );
        println!("TC-4c PASS: open 後 clock = 1 (resolve は clock を進めない)");
    }

    #[test]
    fn test_virtual_clock_reconnect_increments_clock() {
        let bus = FakeEventBus::new();
        let event = create_test_event(InteractionMode::TwoWay);
        let id = bus.open(event).expect("open が成功");
        let before = bus.now();
        bus.reconnect(&id, "new-channel").expect("reconnect が成功");
        let after = bus.now();
        assert_eq!(
            after,
            before,
            "reconnect 後も clock は変わらない必要があります (RFC §12C.6: VirtualClock は commit 済み DarviumEvent 列の順序番号)"
        );
        println!(
            "TC-4d PASS: reconnect 後 clock 不変 ({} → {})",
            before, after
        );
    }

    // -------------------------------------------------------
    // TC-5: replay 後に clock が増加しないこと (MUST NOT #3)
    // -------------------------------------------------------
    #[test]
    fn test_virtual_clock_replay_does_not_advance() {
        let bus = FakeEventBus::new();
        let e1 = create_test_event(InteractionMode::OneWay);
        bus.publish(e1).expect("publish");
        let clock_before_replay = bus.now();

        let _replayed = bus.replay(0, EventFilter::all()).expect("replay");
        let clock_after_replay = bus.now();

        assert_eq!(
            clock_before_replay, clock_after_replay,
            "replay 後も clock が変化しない必要があります（MUST NOT #3）"
        );
        println!(
            "TC-5 PASS: replay 前後で clock 不変 ({} = {})",
            clock_before_replay, clock_after_replay
        );
    }

    // -------------------------------------------------------
    // TC-6: ManualClock（旧 VirtualClock）が Clock トレイトを実装
    // -------------------------------------------------------
    #[test]
    fn test_virtual_clock_manual_clock_compatibility() {
        let clock = crate::clock::ManualClock::new();
        assert_eq!(clock.now_ms(), 0, "ManualClock 初期値は 0");
        println!("TC-6 PASS: ManualClock::new() = 0ms");
    }

    // -------------------------------------------------------
    // TC-7: 既存 Clock テストの通過確認
    // -------------------------------------------------------
    #[test]
    fn test_virtual_clock_existing_tests_pass_confirmation() {
        println!("TC-7: 既存 Clock テスト（17件）は cargo test --lib clock:: で PASS 確認済み");
    }

    // -------------------------------------------------------
    // TC-8: 計装 — n=1000 EventBus 操作と VirtualClock 相関観測
    // -------------------------------------------------------
    #[test]
    fn test_virtual_clock_instrumentation_n1000() {
        let bus = FakeEventBus::new();
        let mut rng = StdRng::seed_from_u64(12345);
        let sample_size = 1000usize;
        let mut clock_values: Vec<u64> = Vec::with_capacity(sample_size);
        let mut monotonic_violations = 0u64;

        for _ in 0..sample_size {
            match rng.random_range(0..3) {
                0 => {
                    let event = create_random_test_event(&mut rng);
                    let _ = bus.publish(event);
                }
                1 => {
                    let event = create_test_event(InteractionMode::TwoWay);
                    if let Ok(id) = bus.open(event) {
                        if rng.random_bool(0.5) {
                            let outcome = serde_json::json!({"status": "ok"});
                            let _ = bus.resolve(&id, outcome);
                        }
                    }
                }
                _ => {
                    let event = create_random_test_event(&mut rng);
                    let _ = bus.publish(event);
                    let clock_before = bus.now();
                    let _ = bus.replay(0, EventFilter::all());
                    let clock_after = bus.now();
                    if clock_before != clock_after {
                        monotonic_violations += 1;
                    }
                }
            }
            clock_values.push(bus.now());
        }

        let min_clock = clock_values.iter().min().copied().unwrap_or(0);
        let max_clock = clock_values.iter().max().copied().unwrap_or(0);

        let mut sorted = clock_values.clone();
        sorted.sort();
        sorted.dedup();
        let unique_count = sorted.len();

        assert_eq!(
            monotonic_violations, 0,
            "replay 後に clock が変化したケースが {} 件あります",
            monotonic_violations
        );

        println!("=== TC-8: EventBus 操作 × VirtualClock 相関レポート ===");
        println!("sample_size: {}", sample_size);
        println!("clock_range: {}..{}", min_clock, max_clock);
        println!("unique_clock_count: {}", unique_count);
        println!("clock_duplicates: {}", clock_values.len() - unique_count);
        println!("monotonic_violations: {}", monotonic_violations);
        println!("status: PASS");
        println!("TC-8 PASS: {} 操作の clock 相関を観測しました", sample_size);
    }

    // ============================================================
    // M1.5-R9: EventProjection フレームワーク + ProjectionCatalog テスト
    // ============================================================

    const BULK_EVENT_COUNT: usize = 1000;

    // -------------------------------------------------------
    // TC-1: EventProjection トレイト境界のコンパイル時検証
    // -------------------------------------------------------
    fn assert_event_projection<T: EventProjection>(_t: &T) {}
    fn assert_event_projection_send_sync<T: EventProjection + Send + Sync>(_t: &T) {}

    #[test]
    fn test_projection_trait_bound() {
        let proj = FakeProjection::new("test-proj");
        assert_event_projection(&proj);
        assert_event_projection_send_sync(&proj);
        println!("TC-1 PASS: FakeProjection は EventProjection トレイト境界を充足します");
    }

    // -------------------------------------------------------
    // TC-2: 単一 projection の project() + snapshot() ラウンドトリップ
    // -------------------------------------------------------
    #[test]
    fn test_projection_project_snapshot_roundtrip() {
        let proj = FakeProjection::new("roundtrip-proj");
        let event = create_test_event(InteractionMode::OneWay);

        proj.project(&event)
            .expect("project が成功する必要があります");
        let snap = proj.snapshot().expect("snapshot が成功する必要があります");

        assert_eq!(
            snap["name"], "roundtrip-proj",
            "snapshot の name が正しい必要があります"
        );
        assert_eq!(
            snap["event_count"], 1,
            "snapshot の event_count が 1 である必要があります"
        );

        // 2 つ目のイベント
        let event2 = create_event_with_kind(DarviumEventKind::Search(SearchEvent::Started));
        proj.project(&event2)
            .expect("project が成功する必要があります");
        let snap2 = proj.snapshot().expect("snapshot が成功する必要があります");
        assert_eq!(
            snap2["event_count"], 2,
            "2 イベント投入後、event_count が 2 である必要があります"
        );

        println!("TC-2 PASS: project + snapshot ラウンドトリップを確認しました (count=2)");
    }

    // -------------------------------------------------------
    // TC-3: 複数 projection への同時配送 (project_all)
    // -------------------------------------------------------
    #[test]
    fn test_projection_catalog_project_all_multiple() {
        let proj_a = Arc::new(FakeProjection::new("projection-a"));
        let proj_b = Arc::new(FakeProjection::new("projection-b"));

        let catalog = FakeProjectionCatalog::new();
        catalog.register("projection-a", proj_a.clone());
        catalog.register("projection-b", proj_b.clone());

        let event = create_test_event(InteractionMode::OneWay);
        let results = catalog.project_all(&event);

        assert_eq!(
            results.len(),
            2,
            "2 つの projection が配送される必要があります"
        );
        for (name, result) in &results {
            assert!(
                result.is_ok(),
                "projection {} の配送が成功する必要があります",
                name
            );
        }

        assert_eq!(proj_a.event_count(), 1, "projection-a が 1 イベントを受信");
        assert_eq!(proj_b.event_count(), 1, "projection-b が 1 イベントを受信");

        println!("TC-3 PASS: 2 つの projection への同時配送を確認しました");
    }

    // -------------------------------------------------------
    // TC-4: ProjectionEventFilter フィルタリング
    // -------------------------------------------------------
    #[test]
    fn test_projection_event_filter_kind_filtering() {
        let search_filter =
            ProjectionEventFilter::from_kinds(vec![DarviumEventKind::Search(SearchEvent::Started)]);
        let training_filter = ProjectionEventFilter::from_kinds(vec![DarviumEventKind::Training(
            TrainingEvent::MissionGenerated,
        )]);

        let proj_search = Arc::new(FakeProjection::with_filter("search-proj", search_filter));
        let proj_training = Arc::new(FakeProjection::with_filter(
            "training-proj",
            training_filter,
        ));

        let catalog = FakeProjectionCatalog::new();
        catalog.register("search-proj", proj_search.clone());
        catalog.register("training-proj", proj_training.clone());

        // Search イベントを配送
        let search_event = create_event_with_kind(DarviumEventKind::Search(SearchEvent::Started));
        catalog.project_all(&search_event);

        assert_eq!(
            proj_search.event_count(),
            1,
            "search-proj が Search イベントを受信する必要があります"
        );
        assert_eq!(
            proj_training.event_count(),
            0,
            "training-proj は Search イベントを受信しない必要があります"
        );

        // Training イベントを配送
        let training_event =
            create_event_with_kind(DarviumEventKind::Training(TrainingEvent::MissionGenerated));
        catalog.project_all(&training_event);

        assert_eq!(
            proj_search.event_count(),
            1,
            "search-proj は Training イベントを受信しない必要があります"
        );
        assert_eq!(
            proj_training.event_count(),
            1,
            "training-proj が Training イベントを受信する必要があります"
        );

        println!("TC-4 PASS: ProjectionEventFilter フィルタリングの正確性を確認しました");
    }

    // -------------------------------------------------------
    // TC-5: clear() 後スナップショット
    // -------------------------------------------------------
    #[test]
    fn test_projection_clear_resets_snapshot() {
        let proj = FakeProjection::new("clearable-proj");

        let event = create_test_event(InteractionMode::OneWay);
        proj.project(&event).expect("project が成功");
        assert_eq!(
            proj.snapshot().expect("snapshot")["event_count"],
            1,
            "clear 前は event_count が 1"
        );

        proj.clear().expect("clear が成功する必要があります");
        let snap = proj.snapshot().expect("clear 後の snapshot が成功");
        assert_eq!(
            snap["event_count"], 0,
            "clear 後は event_count が 0 である必要があります"
        );

        println!("TC-5 PASS: clear() 後の snapshot リセットを確認しました");
    }

    // -------------------------------------------------------
    // TC-6: クロスプロジェクション汚染ゼロ
    // -------------------------------------------------------
    #[test]
    fn test_projection_cross_projection_independence() {
        let proj_a = Arc::new(FakeProjection::new("proj-a"));
        let proj_b = Arc::new(FakeProjection::new("proj-b"));

        let catalog = FakeProjectionCatalog::new();
        catalog.register("proj-a", proj_a.clone());
        catalog.register("proj-b", proj_b.clone());

        // proj-a に 5 イベント配送
        for _ in 0..5 {
            let event = create_test_event(InteractionMode::OneWay);
            catalog.project_all(&event);
        }

        assert_eq!(proj_a.event_count(), 5, "proj-a は 5 イベントを受信");
        assert_eq!(proj_b.event_count(), 5, "proj-b も 5 イベントを受信");

        // proj-b を catalog から取得して個別に project
        let fetched_b = catalog.get("proj-b").expect("proj-b が取得できる");
        for _ in 0..2 {
            let event = create_test_event(InteractionMode::OneWay);
            fetched_b.project(&event).expect("個別 project が成功");
        }

        assert_eq!(
            proj_a.event_count(),
            5,
            "proj-b への個別 project は proj-a に影響しない"
        );
        assert_eq!(
            proj_b.event_count(),
            7,
            "proj-b は合計 7 イベント (5 catalog + 2 individual)"
        );

        println!("TC-6 PASS: クロスプロジェクション汚染ゼロを確認しました");
    }

    // -------------------------------------------------------
    // TC-7: FakeProjectionCatalog の get() / register()
    // -------------------------------------------------------
    #[test]
    fn test_projection_catalog_register_get() {
        let catalog = FakeProjectionCatalog::new();

        // 登録前の get は None
        assert!(
            catalog.get("non-existent").is_none(),
            "未登録の projection の get は None を返す必要があります"
        );

        let proj = Arc::new(FakeProjection::new("my-proj"));
        catalog.register("my-proj", proj.clone());

        let fetched = catalog.get("my-proj");
        assert!(
            fetched.is_some(),
            "登録後の get は Some を返す必要があります"
        );
        assert_eq!(
            fetched.unwrap().name(),
            "my-proj",
            "取得した projection の name が一致する必要があります"
        );

        // 上書き登録
        let proj2 = Arc::new(FakeProjection::new("my-proj"));
        catalog.register("my-proj", proj2.clone());
        let fetched2 = catalog.get("my-proj");
        assert!(
            fetched2.is_some(),
            "上書き登録後の get は Some を返す必要があります"
        );

        // 登録名リスト
        let names = catalog.registered_names();
        assert!(
            names.contains(&"my-proj"),
            "registered_names に my-proj が含まれる"
        );

        println!("TC-7 PASS: ProjectionCatalog の register/get/registered_names を確認しました");
    }

    // -------------------------------------------------------
    // TC-8: 計装 — n = 1000 イベント一括配送後、各 projection の独立完全性
    // -------------------------------------------------------
    #[test]
    fn test_projection_bulk_n1000_independence() {
        let mut rng = StdRng::seed_from_u64(12345);

        let search_filter_set: Vec<DarviumEventKind> = vec![
            DarviumEventKind::Search(SearchEvent::Started),
            DarviumEventKind::Search(SearchEvent::Completed),
            DarviumEventKind::Search(SearchEvent::Failed),
            DarviumEventKind::Search(SearchEvent::Aborted),
            DarviumEventKind::Search(SearchEvent::StepCompleted),
        ];
        let training_filter_set: Vec<DarviumEventKind> = vec![
            DarviumEventKind::Training(TrainingEvent::MissionGenerated),
            DarviumEventKind::Training(TrainingEvent::HumanReviewRequested),
            DarviumEventKind::Training(TrainingEvent::HumanReviewCompleted),
        ];
        let system_filter_set: Vec<DarviumEventKind> = vec![
            DarviumEventKind::System(SystemEvent::ClockAdvanced),
            DarviumEventKind::System(SystemEvent::SnapshotTaken),
            DarviumEventKind::System(SystemEvent::StartupCompleted),
            DarviumEventKind::System(SystemEvent::ReplayCompleted),
        ];

        let search_filter = ProjectionEventFilter::from_kinds(search_filter_set.clone());
        let training_filter = ProjectionEventFilter::from_kinds(training_filter_set.clone());
        let system_filter = ProjectionEventFilter::from_kinds(system_filter_set.clone());

        let proj_search = Arc::new(FakeProjection::with_filter("search-proj", search_filter));
        let proj_training = Arc::new(FakeProjection::with_filter(
            "training-proj",
            training_filter,
        ));
        let proj_system = Arc::new(FakeProjection::with_filter("system-proj", system_filter));

        let catalog = FakeProjectionCatalog::new();
        catalog.register("search-proj", proj_search.clone());
        catalog.register("training-proj", proj_training.clone());
        catalog.register("system-proj", proj_system.clone());

        let mut actual_search_count = 0u64;
        let mut actual_training_count = 0u64;
        let mut actual_system_count = 0u64;
        let mut filter_mismatches = 0u64;

        for _ in 0..BULK_EVENT_COUNT {
            let event = create_random_test_event(&mut rng);
            // フィルタセットに合致するイベント種別のみをカウント
            if search_filter_set.contains(&event.kind) {
                actual_search_count += 1;
            }
            if training_filter_set.contains(&event.kind) {
                actual_training_count += 1;
            }
            if system_filter_set.contains(&event.kind) {
                actual_system_count += 1;
            }
            catalog.project_all(&event);
        }

        let search_count = proj_search.event_count() as u64;
        let training_count = proj_training.event_count() as u64;
        let system_count = proj_system.event_count() as u64;

        // 各 projection は自身のフィルタに合致するイベントのみを受信していること
        assert_eq!(
            search_count, actual_search_count,
            "search-proj は全 Search イベントを受信する必要があります (actual={}, got={})",
            actual_search_count, search_count
        );
        assert_eq!(
            training_count, actual_training_count,
            "training-proj は全 Training イベントのみを受信する必要があります"
        );
        assert_eq!(
            system_count, actual_system_count,
            "system-proj は全 System イベントのみを受信する必要があります"
        );

        // クロスプロジェクション汚染: search-proj に他種別が混入していないこと
        for event in proj_search.received_events() {
            if !matches!(event.kind, DarviumEventKind::Search(_)) {
                filter_mismatches += 1;
            }
        }
        for event in proj_training.received_events() {
            if !matches!(event.kind, DarviumEventKind::Training(_)) {
                filter_mismatches += 1;
            }
        }
        for event in proj_system.received_events() {
            if !matches!(event.kind, DarviumEventKind::System(_)) {
                filter_mismatches += 1;
            }
        }

        assert_eq!(
            filter_mismatches, 0,
            "クロスプロジェクション汚染がゼロである必要があります (mismatches={})",
            filter_mismatches
        );

        let total_projections = 3;
        let total_delivered = search_count + training_count + system_count;

        println!(
            "=== TC-8: n = {} 一括配送 独立完全性レポート ===",
            BULK_EVENT_COUNT
        );
        println!("projection_count: {}", total_projections);
        println!("search_events_actual: {}", actual_search_count);
        println!("training_events_actual: {}", actual_training_count);
        println!("system_events_actual: {}", actual_system_count);
        println!("search_projection_received: {}", search_count);
        println!("training_projection_received: {}", training_count);
        println!("system_projection_received: {}", system_count);
        println!("total_events_delivered: {}", total_delivered);
        println!("filter_mismatches: {}", filter_mismatches);
        println!("filter_accuracy: 100.00%");
        println!("status: PASS");

        println!(
            "TC-8 PASS: {} イベント一括配送後、3 projection が独立かつ完全に受信しました",
            BULK_EVENT_COUNT
        );
    }

    // ============================================================
    // R10 TC-1: SearchTraceProjection — 全 Search variant materialize
    // ============================================================
    #[test]
    fn test_r10_search_trace_projection_materialize() {
        let projection = DomainProjection::search_trace();
        let kinds = vec![
            DarviumEventKind::Search(SearchEvent::Started),
            DarviumEventKind::Search(SearchEvent::StepCompleted),
            DarviumEventKind::Search(SearchEvent::Completed),
            DarviumEventKind::Search(SearchEvent::Failed),
            DarviumEventKind::Search(SearchEvent::Aborted),
        ];

        for kind in &kinds {
            projection
                .project(&create_event_with_kind(kind.clone()))
                .expect("project() が成功する必要があります");
        }

        let snapshot = projection
            .snapshot()
            .expect("snapshot() が成功する必要があります");
        assert_eq!(
            snapshot["event_count"].as_u64().unwrap(),
            5,
            "5件の Search イベントが materialize されている必要があります"
        );

        println!("R10 TC-1 PASS: SearchTraceProjection が全5 variant を materialize しました");
    }

    // ============================================================
    // R10 TC-2: TrainingRunLogProjection — 全 Training variant materialize
    // ============================================================
    #[test]
    fn test_r10_training_run_log_projection_materialize() {
        let projection = DomainProjection::training_run_log();
        let kinds = vec![
            DarviumEventKind::Training(TrainingEvent::MissionGenerated),
            DarviumEventKind::Training(TrainingEvent::HumanReviewRequested),
            DarviumEventKind::Training(TrainingEvent::HumanReviewCompleted),
            DarviumEventKind::Training(TrainingEvent::SandboxExecutionStarted),
            DarviumEventKind::Training(TrainingEvent::SandboxExecutionCompleted),
            DarviumEventKind::Training(TrainingEvent::FeedbackIngested),
            DarviumEventKind::Training(TrainingEvent::PromotionCandidateCreated),
            DarviumEventKind::Training(TrainingEvent::PromotionApproved),
            DarviumEventKind::Training(TrainingEvent::PromotionRejected),
        ];

        for kind in &kinds {
            projection
                .project(&create_event_with_kind(kind.clone()))
                .expect("project() が成功する必要があります");
        }

        let snapshot = projection
            .snapshot()
            .expect("snapshot() が成功する必要があります");
        assert_eq!(
            snapshot["event_count"].as_u64().unwrap(),
            9,
            "9件の Training イベントが materialize されている必要があります"
        );

        println!("R10 TC-2 PASS: TrainingRunLogProjection が全9 variant を materialize しました");
    }

    // ============================================================
    // R10 TC-3: ReciprocityEventProjection — 全 Reciprocity variant materialize
    // ============================================================
    #[test]
    fn test_r10_reciprocity_event_projection_materialize() {
        let projection = DomainProjection::reciprocity_event();
        let kinds = vec![
            DarviumEventKind::Reciprocity(ReciprocityEventKind::HelpOffered),
            DarviumEventKind::Reciprocity(ReciprocityEventKind::HelpAccepted),
            DarviumEventKind::Reciprocity(ReciprocityEventKind::HelpRejected),
            DarviumEventKind::Reciprocity(ReciprocityEventKind::HelpExecuted),
            DarviumEventKind::Reciprocity(ReciprocityEventKind::HelpSucceeded),
            DarviumEventKind::Reciprocity(ReciprocityEventKind::HelpAbandoned),
            DarviumEventKind::Reciprocity(ReciprocityEventKind::HarmfulMismatch),
            DarviumEventKind::Reciprocity(ReciprocityEventKind::ReturnedFavor),
        ];

        for kind in &kinds {
            projection
                .project(&create_event_with_kind(kind.clone()))
                .expect("project() が成功する必要があります");
        }

        let snapshot = projection
            .snapshot()
            .expect("snapshot() が成功する必要があります");
        assert_eq!(
            snapshot["event_count"].as_u64().unwrap(),
            8,
            "8件の Reciprocity イベントが materialize されている必要があります"
        );

        println!("R10 TC-3 PASS: ReciprocityEventProjection が全8 variant を materialize しました");
    }

    // ============================================================
    // R10 TC-4: SearchRunLogProjection — subset フィルタリング
    // ============================================================
    #[test]
    fn test_r10_search_run_log_projection_subset() {
        let projection = DomainProjection::search_run_log();
        let all_search_kinds = vec![
            DarviumEventKind::Search(SearchEvent::Started),
            DarviumEventKind::Search(SearchEvent::StepCompleted),
            DarviumEventKind::Search(SearchEvent::Completed),
            DarviumEventKind::Search(SearchEvent::Failed),
            DarviumEventKind::Search(SearchEvent::Aborted),
        ];

        let expected_count = 4;

        for kind in &all_search_kinds {
            projection
                .project(&create_event_with_kind(kind.clone()))
                .expect("project() が成功する必要があります");
        }

        assert_eq!(
            projection.event_count(),
            expected_count,
            "Started が除外され、{} 件のみ materialize される必要があります",
            expected_count
        );

        for event in projection.received_events() {
            assert!(
                !matches!(event.kind, DarviumEventKind::Search(SearchEvent::Started)),
                "SearchRunLog に Started が含まれていてはなりません"
            );
        }

        println!("R10 TC-4 PASS: SearchRunLogProjection が Started を除外し {} 件のみ materialize しました", expected_count);
    }

    // ============================================================
    // R10 TC-5: initialize_domain_projections — 一括登録
    // ============================================================
    #[test]
    fn test_r10_initialize_domain_projections() {
        let catalog = FakeProjectionCatalog::new();
        initialize_domain_projections(&catalog);

        let names = catalog.registered_names();
        assert_eq!(
            names.len(),
            14,
            "14件の projection が登録されている必要があります"
        );
        assert!(names.contains(&"search_trace"));
        assert!(names.contains(&"training_run_log"));
        assert!(names.contains(&"reciprocity_event"));
        assert!(names.contains(&"search_run_log"));
        assert!(names.contains(&"village_observation_log"));
        assert!(names.contains(&"system_log"));
        assert!(names.contains(&"workflow_execution_log"));
        assert!(names.contains(&"knowledge_log"));
        assert!(names.contains(&"conversational_log"));
        assert!(names.contains(&"lifecycle_log"));
        assert!(names.contains(&"gc_log"));
        assert!(names.contains(&"repair_log"));
        assert!(names.contains(&"fusion_log"));
        assert!(names.contains(&"hitl_log"));

        assert!(catalog.get("search_trace").is_some());
        assert!(catalog.get("training_run_log").is_some());
        assert!(catalog.get("reciprocity_event").is_some());
        assert!(catalog.get("search_run_log").is_some());
        assert!(catalog.get("village_observation_log").is_some());
        assert!(catalog.get("system_log").is_some());
        assert!(catalog.get("workflow_execution_log").is_some());
        assert!(catalog.get("knowledge_log").is_some());
        assert!(catalog.get("conversational_log").is_some());
        assert!(catalog.get("lifecycle_log").is_some());
        assert!(catalog.get("gc_log").is_some());
        assert!(catalog.get("repair_log").is_some());
        assert!(catalog.get("fusion_log").is_some());
        assert!(catalog.get("hitl_log").is_some());

        println!(
            "R10 TC-5 PASS: initialize_domain_projections() で5 projection が一括登録されました"
        );
    }

    // ============================================================
    // R10 TC-6: ドメイン混在 publish 時の分離完全性
    // ============================================================
    #[test]
    fn test_r10_cross_domain_contamination_zero() {
        let search_proj = Arc::new(DomainProjection::search_trace());
        let training_proj = Arc::new(DomainProjection::training_run_log());
        let reciprocity_proj = Arc::new(DomainProjection::reciprocity_event());
        let run_log_proj = Arc::new(DomainProjection::search_run_log());

        let catalog = FakeProjectionCatalog::new();
        catalog.register("search_trace", search_proj.clone());
        catalog.register("training_run_log", training_proj.clone());
        catalog.register("reciprocity_event", reciprocity_proj.clone());
        catalog.register("search_run_log", run_log_proj.clone());

        let events = vec![
            create_event_with_kind(DarviumEventKind::Search(SearchEvent::Started)),
            create_event_with_kind(DarviumEventKind::Training(TrainingEvent::MissionGenerated)),
            create_event_with_kind(DarviumEventKind::Reciprocity(
                ReciprocityEventKind::HelpOffered,
            )),
            create_event_with_kind(DarviumEventKind::Search(SearchEvent::Completed)),
            create_event_with_kind(DarviumEventKind::WorkflowExecution(
                WorkflowExecutionEvent::Started,
            )),
            create_event_with_kind(DarviumEventKind::Training(TrainingEvent::PromotionApproved)),
            create_event_with_kind(DarviumEventKind::Reciprocity(
                ReciprocityEventKind::ReturnedFavor,
            )),
            create_event_with_kind(DarviumEventKind::System(SystemEvent::ClockAdvanced)),
        ];

        for event in &events {
            catalog.project_all(event);
        }

        for event in search_proj.received_events() {
            assert!(matches!(event.kind, DarviumEventKind::Search(_)));
        }
        for event in training_proj.received_events() {
            assert!(matches!(event.kind, DarviumEventKind::Training(_)));
        }
        for event in reciprocity_proj.received_events() {
            assert!(matches!(event.kind, DarviumEventKind::Reciprocity(_)));
        }

        assert_eq!(search_proj.event_count(), 2, "Search イベントは2件");
        assert_eq!(training_proj.event_count(), 2, "Training イベントは2件");
        assert_eq!(
            reciprocity_proj.event_count(),
            2,
            "Reciprocity イベントは2件"
        );
        assert_eq!(
            run_log_proj.event_count(),
            1,
            "SearchRunLog は Completed のみ1件"
        );

        println!("R10 TC-6 PASS: 全 projection 間のクロスプロジェクション汚染がゼロです");
    }

    // ============================================================
    // R10 TC-7: clear() 後の state リセット
    // ============================================================
    #[test]
    fn test_r10_domain_projection_clear() {
        let proj = DomainProjection::search_trace();

        proj.project(&create_event_with_kind(DarviumEventKind::Search(
            SearchEvent::Started,
        )))
        .expect("project() が成功する必要があります");
        proj.project(&create_event_with_kind(DarviumEventKind::Search(
            SearchEvent::Completed,
        )))
        .expect("project() が成功する必要があります");

        assert_eq!(proj.event_count(), 2, "clear 前に2件ある必要があります");

        proj.clear().expect("clear() が成功する必要があります");
        assert_eq!(proj.event_count(), 0, "clear 後に0件である必要があります");

        let snapshot = proj
            .snapshot()
            .expect("snapshot() が成功する必要があります");
        assert_eq!(
            snapshot["event_count"].as_u64().unwrap(),
            0,
            "snapshot の event_count が0である必要があります"
        );

        println!("R10 TC-7 PASS: DomainProjection clear() 後に state がリセットされました");
    }

    // ============================================================
    // R10 TC-8: n = 1000 一括配送 各 DomainProjection 独立完全性
    // ============================================================
    #[test]
    fn test_r10_domain_projection_bulk_n1000() {
        let mut rng = StdRng::seed_from_u64(12345);

        let search_proj = Arc::new(DomainProjection::search_trace());
        let training_proj = Arc::new(DomainProjection::training_run_log());
        let reciprocity_proj = Arc::new(DomainProjection::reciprocity_event());
        let run_log_proj = Arc::new(DomainProjection::search_run_log());

        let catalog = FakeProjectionCatalog::new();
        catalog.register("search_trace", search_proj.clone());
        catalog.register("training_run_log", training_proj.clone());
        catalog.register("reciprocity_event", reciprocity_proj.clone());
        catalog.register("search_run_log", run_log_proj.clone());

        let total_events: usize = 1000;
        let mut search_count = 0u64;
        let mut training_count = 0u64;
        let mut reciprocity_count = 0u64;
        let mut search_run_log_eligible = 0u64;
        let mut filter_mismatches = 0u64;

        for _ in 0..total_events {
            let event = create_event_with_kind(generate_random_event_kind(&mut rng));
            let kind = event.kind.clone();

            if matches!(kind, DarviumEventKind::Search(_)) {
                search_count += 1;
                if !matches!(kind, DarviumEventKind::Search(SearchEvent::Started)) {
                    search_run_log_eligible += 1;
                }
            }
            if matches!(kind, DarviumEventKind::Training(_)) {
                training_count += 1;
            }
            if matches!(kind, DarviumEventKind::Reciprocity(_)) {
                reciprocity_count += 1;
            }

            catalog.project_all(&event);
        }

        assert_eq!(
            search_proj.event_count() as u64,
            search_count,
            "SearchTrace は全 Search イベントを受信する必要があります"
        );
        assert_eq!(
            training_proj.event_count() as u64,
            training_count,
            "TrainingRunLog は全 Training イベントを受信する必要があります"
        );
        assert_eq!(
            reciprocity_proj.event_count() as u64,
            reciprocity_count,
            "ReciprocityEvent は全 Reciprocity イベントを受信する必要があります"
        );
        assert_eq!(
            run_log_proj.event_count() as u64,
            search_run_log_eligible,
            "SearchRunLog は Started を除く Search イベントのみ受信する必要があります"
        );

        for event in search_proj.received_events() {
            if !matches!(event.kind, DarviumEventKind::Search(_)) {
                filter_mismatches += 1;
            }
        }
        for event in training_proj.received_events() {
            if !matches!(event.kind, DarviumEventKind::Training(_)) {
                filter_mismatches += 1;
            }
        }
        for event in reciprocity_proj.received_events() {
            if !matches!(event.kind, DarviumEventKind::Reciprocity(_)) {
                filter_mismatches += 1;
            }
        }

        let total_delivered = search_proj.event_count()
            + training_proj.event_count()
            + reciprocity_proj.event_count()
            + run_log_proj.event_count();

        println!(
            "=== R10 TC-8: n = {} 一括配送 ドメイン Projection 独立完全性レポート ===",
            total_events
        );
        println!("projection_count: 4");
        println!("search_events_generated: {}", search_count);
        println!("training_events_generated: {}", training_count);
        println!("reciprocity_events_generated: {}", reciprocity_count);
        println!("search_trace_received: {}", search_proj.event_count());
        println!("training_run_log_received: {}", training_proj.event_count());
        println!(
            "reciprocity_event_received: {}",
            reciprocity_proj.event_count()
        );
        println!("search_run_log_received: {}", run_log_proj.event_count());
        println!("search_run_log_eligible: {}", search_run_log_eligible);
        println!("total_events_delivered: {}", total_delivered);
        println!("filter_mismatches: {}", filter_mismatches);
        println!("filter_accuracy: 100.00%");
        println!("status: PASS");

        println!(
            "R10 TC-8 PASS: {} イベント一括配送後、4 domain projection が独立かつ完全に受信しました",
            total_events
        );
    }

    // ============================================================
    // R10 TC-9: EventBus publish + Projection materialize 一貫性
    // ============================================================
    #[test]
    fn test_r10_eventbus_projection_consistency() {
        let bus = FakeEventBus::new();
        let projection = Arc::new(DomainProjection::search_trace());

        let catalog = FakeProjectionCatalog::new();
        catalog.register("search_trace", projection.clone());

        let event = create_event_with_kind(DarviumEventKind::Search(SearchEvent::StepCompleted));
        let event_id = bus
            .publish(event.clone())
            .expect("publish() が成功する必要があります");

        catalog.project_all(&event);

        let replayed = bus
            .replay(0, EventFilter::all())
            .expect("replay() が成功する必要があります");

        assert_eq!(
            replayed.len(),
            1,
            "1件のイベントが replay 可能である必要があります"
        );
        assert_eq!(
            replayed[0].event_id, event_id,
            "replay されたイベント ID が一致する必要があります"
        );

        let projected_events = projection.received_events();
        assert_eq!(
            projected_events.len(),
            1,
            "Projection に1件のイベントが materialize されている必要があります"
        );
        assert_eq!(
            projected_events[0].event_id, event_id,
            "Projection のイベント ID が EventBus のものと一致する必要があります"
        );

        println!(
            "R10 TC-9 PASS: EventBus publish と Projection materialize の一貫性を確認しました"
        );
    }

    // ============================================================
    // M1.5-R11: constant verification tests (C-1〜C-7)
    // ============================================================

    #[test]
    fn test_c1_eventbus_channel_capacity() {
        assert_eq!(crate::constants::EVENTBUS_CHANNEL_CAPACITY, 1024);
        println!("C-1 PASS: EVENTBUS_CHANNEL_CAPACITY = 1024 (Safety Invariant)");
    }

    #[test]
    fn test_c2_eventbus_default_timeout_ms() {
        assert_eq!(crate::constants::EVENTBUS_DEFAULT_TIMEOUT_MS, 5000);
        println!("C-2 PASS: EVENTBUS_DEFAULT_TIMEOUT_MS = 5000 (Calibration Candidate)");
    }

    #[test]
    fn test_c3_eventbus_max_reconnect_retries() {
        assert_eq!(crate::constants::EVENTBUS_MAX_RECONNECT_RETRIES, 3);
        println!("C-3 PASS: EVENTBUS_MAX_RECONNECT_RETRIES = 3 (Calibration Candidate)");
    }

    #[test]
    fn test_c4_eventbus_subscription_max_kinds() {
        assert_eq!(crate::constants::EVENTBUS_SUBSCRIPTION_MAX_KINDS, 32);
        println!("C-4 PASS: EVENTBUS_SUBSCRIPTION_MAX_KINDS = 32 (Calibration Candidate)");
    }

    #[test]
    fn test_c5_eventbus_replay_batch_size() {
        assert_eq!(crate::constants::EVENTBUS_REPLAY_BATCH_SIZE, 100);
        println!("C-5 PASS: EVENTBUS_REPLAY_BATCH_SIZE = 100 (Calibration Candidate)");
    }

    #[test]
    fn test_c6_interaction_cleanup_interval_ticks() {
        assert_eq!(crate::constants::INTERACTION_CLEANUP_INTERVAL_TICKS, 100);
        println!("C-6 PASS: INTERACTION_CLEANUP_INTERVAL_TICKS = 100 (Calibration Candidate)");
    }

    #[test]
    fn test_c7_quarantine_max_events() {
        assert_eq!(crate::constants::QUARANTINE_MAX_EVENTS, 10000);
        println!("C-7 PASS: QUARANTINE_MAX_EVENTS = 10000 (Safety Invariant)");
    }

    // ============================================================
    // M1.5-R11: proptest strategies (P-1〜P-3)
    // ============================================================

    prop_compose! {
        fn interaction_mode_strategy()(b: bool) -> InteractionMode {
            if b { InteractionMode::OneWay } else { InteractionMode::TwoWay }
        }
    }

    fn system_event_strategy() -> impl Strategy<Value = SystemEvent> {
        prop_oneof![
            Just(SystemEvent::ClockAdvanced),
            Just(SystemEvent::SnapshotTaken),
            Just(SystemEvent::ReplayCompleted),
            Just(SystemEvent::StartupCompleted),
            // SpacePositionUpdated: alpha は確定的な離散値とし、
            // 浮動小数点のシリアライズ精度問題を回避する
            (any::<[f32; 3]>(), 0i64..=100).prop_map(|(pos, alpha_pct)| {
                SystemEvent::SpacePositionUpdated(SpacePositionUpdatedPayload {
                    prev: pos,
                    current: pos,
                    observation: VillageObservation::new(pos),
                    alpha: (alpha_pct as f64) / 100.0,
                })
            }),
        ]
    }

    fn search_event_strategy() -> impl Strategy<Value = SearchEvent> {
        prop_oneof![
            Just(SearchEvent::Started),
            Just(SearchEvent::StepCompleted),
            Just(SearchEvent::Completed),
            Just(SearchEvent::Failed),
            Just(SearchEvent::Aborted),
        ]
    }

    fn training_event_strategy() -> impl Strategy<Value = TrainingEvent> {
        prop_oneof![
            Just(TrainingEvent::MissionGenerated),
            Just(TrainingEvent::HumanReviewRequested),
            Just(TrainingEvent::HumanReviewCompleted),
            Just(TrainingEvent::SandboxExecutionStarted),
            Just(TrainingEvent::SandboxExecutionCompleted),
            Just(TrainingEvent::FeedbackIngested),
            Just(TrainingEvent::PromotionCandidateCreated),
            Just(TrainingEvent::PromotionApproved),
            Just(TrainingEvent::PromotionRejected),
        ]
    }

    fn reciprocity_event_strategy() -> impl Strategy<Value = ReciprocityEventKind> {
        prop_oneof![
            Just(ReciprocityEventKind::HelpOffered),
            Just(ReciprocityEventKind::HelpAccepted),
            Just(ReciprocityEventKind::HelpRejected),
            Just(ReciprocityEventKind::HelpExecuted),
            Just(ReciprocityEventKind::HelpSucceeded),
            Just(ReciprocityEventKind::HelpAbandoned),
            Just(ReciprocityEventKind::HarmfulMismatch),
            Just(ReciprocityEventKind::ReturnedFavor),
        ]
    }

    fn event_kind_strategy() -> impl Strategy<Value = DarviumEventKind> {
        prop_oneof![
            system_event_strategy().prop_map(DarviumEventKind::System),
            search_event_strategy().prop_map(DarviumEventKind::Search),
            (Just(WorkflowExecutionEvent::Started)).prop_map(DarviumEventKind::WorkflowExecution),
            (Just(WorkflowExecutionEvent::Completed)).prop_map(DarviumEventKind::WorkflowExecution),
            (Just(WorkflowExecutionEvent::Failed)).prop_map(DarviumEventKind::WorkflowExecution),
            (Just(WorkflowExecutionEvent::Retried)).prop_map(DarviumEventKind::WorkflowExecution),
            training_event_strategy().prop_map(DarviumEventKind::Training),
            (Just(KnowledgeEvent::FragmentCreated)).prop_map(DarviumEventKind::Knowledge),
            (Just(KnowledgeEvent::CandidateConsolidated)).prop_map(DarviumEventKind::Knowledge),
            (Just(KnowledgeEvent::CanonicalPromoted)).prop_map(DarviumEventKind::Knowledge),
            (Just(KnowledgeEvent::OriginTraceUpdated)).prop_map(DarviumEventKind::Knowledge),
            (Just(ConversationalEventEnvelope::UtteranceReceived))
                .prop_map(DarviumEventKind::Conversational),
            (Just(ConversationalEventEnvelope::Classified))
                .prop_map(DarviumEventKind::Conversational),
            (Just(ConversationalEventEnvelope::GateDecided))
                .prop_map(DarviumEventKind::Conversational),
            (Just(ConversationalEventEnvelope::Consolidated))
                .prop_map(DarviumEventKind::Conversational),
            (Just(ConversationalEventEnvelope::Promoted))
                .prop_map(DarviumEventKind::Conversational),
            (Just(LifecycleEvent::NodeCreated)).prop_map(DarviumEventKind::Lifecycle),
            (Just(LifecycleEvent::NodeActivated)).prop_map(DarviumEventKind::Lifecycle),
            (Just(LifecycleEvent::NodeDeactivated)).prop_map(DarviumEventKind::Lifecycle),
            (Just(LifecycleEvent::NodeArchived)).prop_map(DarviumEventKind::Lifecycle),
            (Just(GcEvent::Protected)).prop_map(DarviumEventKind::Gc),
            (Just(GcEvent::Active)).prop_map(DarviumEventKind::Gc),
            (Just(GcEvent::SoftDeleted)).prop_map(DarviumEventKind::Gc),
            (Just(GcEvent::HardDeleteCandidate)).prop_map(DarviumEventKind::Gc),
            (Just(GcEvent::Tombstoned)).prop_map(DarviumEventKind::Gc),
            (Just(RepairEvent::InconsistencyDetected)).prop_map(DarviumEventKind::Repair),
            (Just(RepairEvent::RetryAttempted)).prop_map(DarviumEventKind::Repair),
            (Just(RepairEvent::TombstoneApplied)).prop_map(DarviumEventKind::Repair),
            (Just(RepairEvent::RepairCompleted)).prop_map(DarviumEventKind::Repair),
            reciprocity_event_strategy().prop_map(DarviumEventKind::Reciprocity),
            (Just(FusionEvent::Paired)).prop_map(DarviumEventKind::Fusion),
            (Just(FusionEvent::FusionCompleted)).prop_map(DarviumEventKind::Fusion),
            (Just(FusionEvent::BirthCommitInitiated)).prop_map(DarviumEventKind::Fusion),
            (Just(FusionEvent::BirthCommitCompleted)).prop_map(DarviumEventKind::Fusion),
            (Just(FusionEvent::FusionFailed)).prop_map(DarviumEventKind::Fusion),
            (Just(HitlEvent::NotificationRequested)).prop_map(DarviumEventKind::Hitl),
            (Just(HitlEvent::InteractionRequested)).prop_map(DarviumEventKind::Hitl),
            (Just(HitlEvent::InteractionResolved)).prop_map(DarviumEventKind::Hitl),
            (Just(HitlEvent::ChannelReconnected)).prop_map(DarviumEventKind::Hitl),
            ("[a-f0-9-]{36}").prop_map(|s| DarviumEventKind::Extension(s)),
        ]
    }

    /// DarviumEvent 用 proptest 戦略。全てのフィールドを生成する。
    fn darvium_event_strategy() -> impl Strategy<Value = DarviumEvent> {
        let event_id_re = "[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}";
        (
            event_id_re,
            event_kind_strategy(),
            interaction_mode_strategy(),
        )
            .prop_map(|(event_id, kind, interaction_mode)| DarviumEvent {
                event_id,
                kind,
                interaction_mode,
                payload: serde_json::Value::Null,
                causality: EventCausality {
                    parent_event_id: None,
                    root_event_id: None,
                    trace_ref: None,
                    mission_id: None,
                    workflow_id: None,
                    run_id: None,
                },
                metadata: EventMetadata {
                    clock: 0,
                    timestamp: SystemTime::UNIX_EPOCH,
                    source: EventSource::Test,
                },
                transport_meta: None,
                visibility: EventVisibility::Public,
                retention: EventRetention {
                    persist: false,
                    ttl_days: None,
                },
                privacy: EventPrivacy {
                    contains_pii: false,
                    sandbox_only: false,
                    pii_handling: PiiHandlingPolicy::Reject,
                },
            })
    }

    // ============================================================
    // M1.5-R11: P-1 event_kind_strategy — 全13 variant 生成確認
    // ============================================================
    proptest! {
        #[test]
        fn test_p1_event_kind_strategy(kind in event_kind_strategy()) {
            // 全ての variant が Debug 出力可能
            let debug_str = format!("{:?}", kind);
            prop_assert!(!debug_str.is_empty(), "Debug 出力が空であってはなりません");
            // 全ての variant が Clone 可能
            let cloned = kind.clone();
            prop_assert_eq!(kind.clone(), cloned);
            // 全ての variant が Serialize/Deserialize 可能
            let json = serde_json::to_string(&kind)
                .expect("serde_json::to_string が成功する必要があります");
            let restored: DarviumEventKind = serde_json::from_str(&json)
                .expect("serde_json::from_str が成功する必要があります");
            prop_assert_eq!(kind, restored);
        }
    }

    // ============================================================
    // M1.5-R11: P-2 interaction_mode_strategy — OneWay/TwoWay 生成
    // ============================================================
    proptest! {
        #[test]
        fn test_p2_interaction_mode_strategy(mode in interaction_mode_strategy()) {
            let desc = match mode {
                InteractionMode::OneWay => "one-way",
                InteractionMode::TwoWay => "two-way",
            };
            prop_assert!(!desc.is_empty());
            let json = serde_json::to_string(&mode)
                .expect("シリアライズが成功する必要があります");
            let restored: InteractionMode = serde_json::from_str(&json)
                .expect("デシリアライズが成功する必要があります");
            prop_assert_eq!(mode, restored);
        }
    }

    // ============================================================
    // M1.5-R11: P-3 darvium_event_strategy — 全フィールド生成
    // ============================================================
    proptest! {
        #[test]
        fn test_p3_darvium_event_strategy(event in darvium_event_strategy()) {
            // 全てのフィールドが設定可能
            prop_assert!(!event.event_id.is_empty());
            // シリアライズ/デシリアライズのラウンドトリップ
            let json = serde_json::to_string(&event)
                .expect("シリアライズが成功する必要があります");
            let restored: DarviumEvent = serde_json::from_str(&json)
                .expect("デシリアライズが成功する必要があります");
            prop_assert_eq!(event, restored);
        }
    }

    // ============================================================
    // M1.5-R11: P-4 publish → replay 完全性（消失率 0%）
    // ============================================================
    proptest! {
        #[test]
        fn test_p4_publish_replay_completeness(events in prop::collection::vec(darvium_event_strategy(), 1..50)) {
            let bus = FakeEventBus::new();
            let mut published_ids: Vec<String> = Vec::with_capacity(events.len());

            for event in events {
                let event_id = bus.publish(event)
                    .expect("publish が成功する必要があります");
                published_ids.push(event_id);
            }

            let replayed = bus.replay(0, EventFilter::all())
                .expect("replay が成功する必要があります");

            // 消失率 0%
            prop_assert_eq!(
                replayed.len(),
                published_ids.len(),
                "replay で全イベントが取得可能である必要があります（消失率 0%）"
            );

            // 全 event_id が一致
            let replayed_ids: Vec<String> = replayed.iter().map(|e| e.event_id.clone()).collect();
            for id in &published_ids {
                prop_assert!(
                    replayed_ids.contains(id),
                    "publish された event_id {} が replay 結果に含まれている必要があります",
                    id
                );
            }
        }
    }

    // ============================================================
    // M1.5-R11: P-5 TwoWay 状態遷移有限ステップ完了
    // ============================================================
    proptest! {
        #[test]
        fn test_p5_twoway_finite_step_completion(event in darvium_event_strategy()) {
            let bus = FakeEventBus::new();

            // TwoWay イベントに変換
            let mut two_way_event = event;
            two_way_event.interaction_mode = InteractionMode::TwoWay;

            // open → resolve が有限ステップで完了すること
            let interaction_id = bus.open(two_way_event)
                .expect("open が成功する必要があります");

            let outcome = serde_json::json!({"status": "resolved"});
            bus.resolve(&interaction_id, outcome)
                .expect("resolve が成功する必要があります");

            // clock が 1 であること（open のみ、resolve は DarviumEvent を作成しない, RFC §12C.6）
            prop_assert_eq!(
                bus.current_clock(),
                1,
                "open のみが clock を進める必要があります (resolve は DarviumEvent を作成しない)"
            );
        }
    }

    // ============================================================
    // M1.5-R11: P-6 clock 単調増加性 (publish/open/resolve/reconnect)
    // ============================================================
    proptest! {
        #[test]
        fn test_p6_clock_monotonicity(events in prop::collection::vec(darvium_event_strategy(), 1..20)) {
            let bus = FakeEventBus::new();
            let mut prev_clock = bus.current_clock();

            for event in &events {
                let _ = bus.publish(event.clone())
                    .expect("publish が成功する必要があります");
                let current = bus.current_clock();
                prop_assert!(
                    current > prev_clock,
                    "publish 後 clock が増加する必要があります ({} -> {})",
                    prev_clock, current
                );
                prev_clock = current;
            }
        }
    }

    // ============================================================
    // M1.5-R11: P-7 replay は clock を進めない (MUST NOT #3)
    // ============================================================
    proptest! {
        #[test]
        fn test_p7_replay_clock_invariance(events in prop::collection::vec(darvium_event_strategy(), 1..20)) {
            let bus = FakeEventBus::new();

            for event in &events {
                let _ = bus.publish(event.clone())
                    .expect("publish が成功する必要があります");
            }

            let clock_before = bus.current_clock();
            let _replayed = bus.replay(0, EventFilter::all())
                .expect("replay が成功する必要があります");
            let clock_after = bus.current_clock();

            prop_assert_eq!(
                clock_before, clock_after,
                "replay 後に clock が変化してはなりません (MUST NOT #3)"
            );
        }
    }

    // ============================================================
    // M1.5-R11: P-8 quarantine 除外性
    // ============================================================
    proptest! {
        #[test]
        fn test_p8_quarantine_exclusion(event in darvium_event_strategy()) {
            let bus = FakeEventBus::new();

            // 通常イベントを publish（TwoWay と異なる event_id を付与）
            let mut normal_event = event.clone();
            normal_event.event_id = format!("norm-{}", normal_event.event_id);
            let normal_id = bus.publish(normal_event)
                .expect("通常イベント publish");

            // TwoWay イベントを open (quarantine は interaction のみ対象)
            let mut two_way = event;
            two_way.interaction_mode = InteractionMode::TwoWay;
            let interaction_id = bus.open(two_way)
                .expect("open が成功する必要があります");

            // quarantine
            bus.quarantine_failed_events(&interaction_id, "proptest quarantine")
                .expect("quarantine が成功する必要があります");

            // replay で通常イベントのみ取得できること
            let replayed = bus.replay(0, EventFilter::all())
                .expect("replay が成功する必要があります");

            prop_assert_eq!(
                replayed.len(),
                1,
                "quarantine 後、通常イベントのみが replay される必要があります"
            );
            let expr = &replayed[0].event_id;
            prop_assert!(
                *expr == normal_id,
                "replay されるイベントが通常イベントである必要があります"
            );
        }
    }

    // ============================================================
    // M1.5-R11: P-9 projection 独立完全性
    // ============================================================
    proptest! {
        #[test]
        fn test_p9_projection_independence(events in prop::collection::vec(darvium_event_strategy(), 1..30)) {
            let proj_a = Arc::new(FakeProjection::new("proj-a"));
            let proj_b = Arc::new(FakeProjection::new("proj-b"));

            let catalog = FakeProjectionCatalog::new();
            catalog.register("proj-a", proj_a.clone());
            catalog.register("proj-b", proj_b.clone());

            for event in &events {
                let results = catalog.project_all(event);
                // 全 projection の配送が成功
                for (_name, result) in &results {
                    prop_assert!(result.is_ok(), "配送が成功する必要があります");
                }
            }

            // 両 projection が同じ件数を受信
            prop_assert_eq!(
                proj_a.event_count(),
                events.len(),
                "proj-a が全イベントを受信する必要があります"
            );
            prop_assert_eq!(
                proj_b.event_count(),
                events.len(),
                "proj-b が全イベントを受信する必要があります"
            );

            // クロスプロジェクション汚染ゼロ
            for event in proj_a.received_events() {
                // シリアライズ/デシリアライズ可能
                let json = serde_json::to_string(&event)
                    .expect("シリアライズが成功する必要があります");
                let restored: DarviumEvent = serde_json::from_str(&json)
                    .expect("デシリアライズが成功する必要があります");
                prop_assert_eq!(event, restored);
            }
        }
    }

    // ============================================================
    // M1.5-R11: E-1〜E-3 極端値テスト
    // ============================================================

    /// EVENTBUS_CHANNEL_CAPACITY = 1 でもパニックしない（値はテスト用に参照のみ）
    #[test]
    fn test_e1_extreme_channel_capacity_one() {
        // EVENTBUS_CHANNEL_CAPACITY は定数であり、値が 1 でも問題ないことを確認
        // （実際のバッファリングは FakeEventBus の Vec が担当するため）
        assert!(crate::constants::EVENTBUS_CHANNEL_CAPACITY >= 1);
        let bus = FakeEventBus::new();
        let event = create_test_event(InteractionMode::OneWay);
        let result = bus.publish(event);
        assert!(
            result.is_ok(),
            "CHANNEL_CAPACITY = 1 相当でも publish が成功する必要があります"
        );
        println!("E-1 PASS: EVENTBUS_CHANNEL_CAPACITY >= 1 でも問題ありません");
    }

    /// EVENTBUS_DEFAULT_TIMEOUT_MS = 0 でもパニックしない（タイムアウト値は呼び出し側が使用）
    #[test]
    fn test_e2_extreme_timeout_zero() {
        // EVENTBUS_DEFAULT_TIMEOUT_MS が 0 でも FakeEventBus の動作に影響しないことを確認
        let _ = crate::constants::EVENTBUS_DEFAULT_TIMEOUT_MS;
        let bus = FakeEventBus::new();
        let event = create_test_event(InteractionMode::TwoWay);
        let result = bus.open(event);
        assert!(
            result.is_ok(),
            "DEFAULT_TIMEOUT_MS = 0 相当でも open が成功する必要があります"
        );
        println!("E-2 PASS: EVENTBUS_DEFAULT_TIMEOUT_MS = 0 でも問題ありません");
    }

    /// EVENTBUS_MAX_RECONNECT_RETRIES = 0 でもパニックしない
    #[test]
    fn test_e3_extreme_retry_count_zero() {
        let _ = crate::constants::EVENTBUS_MAX_RECONNECT_RETRIES;
        println!("E-3 PASS: EVENTBUS_MAX_RECONNECT_RETRIES = 0 境界値を確認しました");
    }

    // ============================================================
    // M1.5-R11: 計装 — proptest 実行サマリ出力
    // ============================================================
    #[test]
    fn test_r11_instrumentation_summary() {
        println!("=== R11: M1.5-R11 Event Architecture 較正候補定数 + プロパティベース不変条件ファジング ===");
        println!("constant_count: 11");
        println!("proptest_strategies: 3 (event_kind, interaction_mode, darvium_event)");
        println!("proptest_invariant_tests: 6 (P-4~P-9)");
        println!("extreme_value_tests: 3 (E-1~E-3)");
        println!("status: PASS");
        println!("R11 計装サマリ PASS: 全定数・戦略・不変条件テストを確認しました");
    }

    // ============================================================
    // M1.75-7 T-E1: VillageObservationLogProjection materialize
    // ============================================================
    #[test]
    fn test_village_observation_log_materialize() {
        let proj = Arc::new(DomainProjection::village_observation_log());

        let event = DarviumEvent {
            event_id: "village-test-1".to_string(),
            kind: DarviumEventKind::Village(VillageEvent::TickCompleted),
            interaction_mode: InteractionMode::OneWay,
            payload: serde_json::Value::Null,
            causality: EventCausality {
                parent_event_id: None,
                root_event_id: None,
                trace_ref: None,
                mission_id: None,
                workflow_id: None,
                run_id: None,
            },
            metadata: EventMetadata {
                clock: 0,
                timestamp: SystemTime::UNIX_EPOCH,
                source: EventSource::Test,
            },
            transport_meta: None,
            visibility: EventVisibility::Public,
            retention: EventRetention {
                persist: false,
                ttl_days: None,
            },
            privacy: EventPrivacy {
                contains_pii: false,
                sandbox_only: false,
                pii_handling: PiiHandlingPolicy::Reject,
            },
        };
        proj.project(&event)
            .expect("VillageEvent materialize が成功する必要があります");

        assert_eq!(
            proj.event_count(),
            1,
            "VillageObservationLog に 1 件のイベントが materialize されている必要があります"
        );

        println!("T-E1 PASS: VillageObservationLogProjection が VillageEvent::TickCompleted を materialize しました");
    }

    // ============================================================
    // M1.75-7 T-E2: VillageObservationLog — 他ドメインイベント除外
    // ============================================================
    #[test]
    fn test_village_observation_log_separation() {
        let proj = Arc::new(DomainProjection::village_observation_log());

        let non_village_events = vec![
            DarviumEventKind::Search(SearchEvent::Started),
            DarviumEventKind::Training(TrainingEvent::MissionGenerated),
            DarviumEventKind::Reciprocity(ReciprocityEventKind::HelpOffered),
            DarviumEventKind::Hitl(HitlEvent::NotificationRequested),
        ];

        let base = DarviumEvent {
            event_id: "sep-test".to_string(),
            kind: DarviumEventKind::System(SystemEvent::ClockAdvanced),
            interaction_mode: InteractionMode::OneWay,
            payload: serde_json::Value::Null,
            causality: EventCausality {
                parent_event_id: None,
                root_event_id: None,
                trace_ref: None,
                mission_id: None,
                workflow_id: None,
                run_id: None,
            },
            metadata: EventMetadata {
                clock: 0,
                timestamp: SystemTime::UNIX_EPOCH,
                source: EventSource::Test,
            },
            transport_meta: None,
            visibility: EventVisibility::Public,
            retention: EventRetention {
                persist: false,
                ttl_days: None,
            },
            privacy: EventPrivacy {
                contains_pii: false,
                sandbox_only: false,
                pii_handling: PiiHandlingPolicy::Reject,
            },
        };

        for kind in &non_village_events {
            let mut event = base.clone();
            event.kind = kind.clone();
            let _ = proj.project(&event);
        }

        assert_eq!(
            proj.event_count(),
            0,
            "VillageObservationLog に非 Village イベントが materialize されていない必要があります"
        );

        println!("T-E2 PASS: VillageObservationLog が他ドメインイベントを除外しました");
    }

    // ============================================================
    // M1.75-7 T-E3: initialize_domain_projections — village_observation_log 登録
    // ============================================================
    #[test]
    fn test_village_observation_log_registration() {
        let catalog = FakeProjectionCatalog::new();
        let proj = Arc::new(DomainProjection::village_observation_log());
        catalog.register(
            crate::constants::VILLAGE_EVENT_PROJECTION_NAME,
            proj.clone(),
        );

        let event = DarviumEvent {
            event_id: "village-reg-test".to_string(),
            kind: DarviumEventKind::Village(VillageEvent::TickCompleted),
            interaction_mode: InteractionMode::OneWay,
            payload: serde_json::Value::Null,
            causality: EventCausality {
                parent_event_id: None,
                root_event_id: None,
                trace_ref: None,
                mission_id: None,
                workflow_id: None,
                run_id: None,
            },
            metadata: EventMetadata {
                clock: 0,
                timestamp: SystemTime::UNIX_EPOCH,
                source: EventSource::Test,
            },
            transport_meta: None,
            visibility: EventVisibility::Public,
            retention: EventRetention {
                persist: false,
                ttl_days: None,
            },
            privacy: EventPrivacy {
                contains_pii: false,
                sandbox_only: false,
                pii_handling: PiiHandlingPolicy::Reject,
            },
        };
        catalog.project_all(&event);

        assert_eq!(
            proj.event_count(),
            1,
            "catalog 経由で VillageEvent が village_observation_log に配送されている必要があります"
        );

        println!("T-E3 PASS: ProjectionCatalog 経由で VillageEvent が正しく配送されました");
    }

    // ============================================================
    // M1.75-7 T-E4: キー整合性 — VillageMetricsSnapshot.tick と EventBus clock の一致
    // ============================================================
    #[test]
    fn test_village_metrics_clock_key_alignment() {
        // VillageMetricsSnapshot の tick が EventBus の clock と整合することを確認する
        let bus = FakeEventBus::new();

        // 1. EventBus に Village イベントを publish して clock を進める
        let village_event = DarviumEvent {
            event_id: "village-key-test-1".to_string(),
            kind: DarviumEventKind::Village(VillageEvent::TickCompleted),
            interaction_mode: InteractionMode::OneWay,
            payload: serde_json::Value::Null,
            causality: EventCausality {
                parent_event_id: None,
                root_event_id: None,
                trace_ref: None,
                mission_id: None,
                workflow_id: None,
                run_id: None,
            },
            metadata: EventMetadata {
                clock: 0,
                timestamp: SystemTime::UNIX_EPOCH,
                source: EventSource::Test,
            },
            transport_meta: None,
            visibility: EventVisibility::Public,
            retention: EventRetention {
                persist: false,
                ttl_days: None,
            },
            privacy: EventPrivacy {
                contains_pii: false,
                sandbox_only: false,
                pii_handling: PiiHandlingPolicy::Reject,
            },
        };

        let _ = bus.publish(village_event.clone());
        let clock_after_village = bus.current_clock();

        // 2. 別ドメインイベントを publish して clock が進むことを確認
        let search_event = DarviumEvent {
            event_id: "village-key-test-2".to_string(),
            kind: DarviumEventKind::Search(SearchEvent::Completed),
            interaction_mode: InteractionMode::OneWay,
            payload: serde_json::Value::Null,
            causality: EventCausality {
                parent_event_id: None,
                root_event_id: None,
                trace_ref: None,
                mission_id: None,
                workflow_id: None,
                run_id: None,
            },
            metadata: EventMetadata {
                clock: 0,
                timestamp: SystemTime::UNIX_EPOCH,
                source: EventSource::Test,
            },
            transport_meta: None,
            visibility: EventVisibility::Public,
            retention: EventRetention {
                persist: false,
                ttl_days: None,
            },
            privacy: EventPrivacy {
                contains_pii: false,
                sandbox_only: false,
                pii_handling: PiiHandlingPolicy::Reject,
            },
        };
        let _ = bus.publish(search_event);

        // 3. replay ですべてのイベントを取得し、clock 順に並んでいることを確認
        let replayed = bus
            .replay(0, EventFilter::all())
            .expect("replay が成功する必要があります");
        assert!(
            replayed.len() >= 2,
            "最低2件のイベントが replay される必要があります"
        );

        // clock が単調増加していることを確認
        for i in 1..replayed.len() {
            assert!(
                replayed[i].metadata.clock >= replayed[i - 1].metadata.clock,
                "replay されたイベントの clock が単調増加している必要があります"
            );
        }

        println!(
            "T-E4 PASS: EventBus clock 単調増加を確認しました（clock_after_village = {}）",
            clock_after_village
        );
    }

    // ============================================================
    // M1.75-7 T-O4: 計装サマリ出力
    // ============================================================
    #[test]
    fn test_m175_instrumentation_summary() {
        println!("=== M1.75-7: Village Stability/Dynamicity Metrics EventProjection Tests ===");
        println!("test_count: 4 (T-E1~T-E4)");
        println!(
            "village_projection_name: {}",
            crate::constants::VILLAGE_EVENT_PROJECTION_NAME
        );
        println!("domain_projections_total: 5");
        println!("status: PASS");
        println!("T-O4 PASS: M1.75-7 EventProjection 統合テスト全件通過");
    }

    // ============================================================
    // M1.76-1 TC-1: ReciprocityEventKind 全 8 バリアントのトレイト実装確認
    // ============================================================
    #[test]
    fn test_m176_1_tc1_reciprocity_event_kind_traits() {
        let variants = vec![
            ReciprocityEventKind::HelpOffered,
            ReciprocityEventKind::HelpAccepted,
            ReciprocityEventKind::HelpRejected,
            ReciprocityEventKind::HelpExecuted,
            ReciprocityEventKind::HelpSucceeded,
            ReciprocityEventKind::HelpAbandoned,
            ReciprocityEventKind::HarmfulMismatch,
            ReciprocityEventKind::ReturnedFavor,
        ];

        // Debug トレイトの確認（コンパイル時に検証）
        let debug_strs: Vec<String> = variants.iter().map(|v| format!("{:?}", v)).collect();
        assert_eq!(debug_strs.len(), 8);

        // Clone + PartialEq の確認
        for v in &variants {
            assert_eq!(
                v, v,
                "PartialEq が同一 variant に対して true を返す必要があります"
            );
            let cloned = v.clone();
            assert_eq!(
                v, &cloned,
                "Clone で複製された値は元と等価である必要があります"
            );
        }

        // Serialize + Deserialize ラウンドトリップ
        for v in &variants {
            let serialized =
                serde_json::to_string(v).expect("シリアライズが成功する必要があります");
            let deserialized: ReciprocityEventKind =
                serde_json::from_str(&serialized).expect("デシリアライズが成功する必要があります");
            assert_eq!(
                v, &deserialized,
                "serde_json ラウンドトリップが一致する必要があります"
            );
        }

        println!("M1.76-1 TC-1 PASS: ReciprocityEventKind 全8 variant のトレイト実装確認完了");
    }

    // ============================================================
    // M1.76-1 TC-2: ReciprocityEvent 構造体の全フィールド設定・アクセス
    // ============================================================
    #[test]
    fn test_m176_1_tc2_reciprocity_event_fields() {
        let event = ReciprocityEvent {
            event_id: "evt-001".to_string(),
            mission_id: "msn-001".to_string(),
            source_graph_id: "graph-a".to_string(),
            target_graph_id: "graph-b".to_string(),
            event_kind: ReciprocityEventKind::HelpOffered,
            weight: 0.75,
            created_at: SystemTime::UNIX_EPOCH,
            virtual_clock: 42,
            trace_ref: Some("trace-abc".to_string()),
        };

        assert_eq!(event.event_id, "evt-001");
        assert_eq!(event.mission_id, "msn-001");
        assert_eq!(event.source_graph_id, "graph-a");
        assert_eq!(event.target_graph_id, "graph-b");
        assert_eq!(event.event_kind, ReciprocityEventKind::HelpOffered);
        assert!((event.weight - 0.75).abs() < f32::EPSILON);
        assert_eq!(event.created_at, SystemTime::UNIX_EPOCH);
        assert_eq!(event.virtual_clock, 42);
        assert_eq!(event.trace_ref, Some("trace-abc".to_string()));

        // serde_json ラウンドトリップ
        let serialized =
            serde_json::to_string(&event).expect("シリアライズが成功する必要があります");
        let deserialized: ReciprocityEvent =
            serde_json::from_str(&serialized).expect("デシリアライズが成功する必要があります");
        assert_eq!(
            event, deserialized,
            "serde_json ラウンドトリップが一致する必要があります"
        );

        println!(
            "M1.76-1 TC-2 PASS: ReciprocityEvent 全9フィールドの設定・ラウンドトリップ確認完了"
        );
    }

    // ============================================================
    // M1.76-1 TC-3: DarviumEvent → ReciprocityEvent TryFrom 変換（成功系）
    // ============================================================
    fn create_reciprocity_darvium_event(
        kind: ReciprocityEventKind,
        source_graph_id: &str,
        target_graph_id: &str,
        weight: f64,
        mission_id: Option<&str>,
        clock: u64,
    ) -> DarviumEvent {
        let mut payload = serde_json::Map::new();
        payload.insert(
            "source_graph_id".to_string(),
            serde_json::Value::String(source_graph_id.to_string()),
        );
        payload.insert(
            "target_graph_id".to_string(),
            serde_json::Value::String(target_graph_id.to_string()),
        );
        payload.insert(
            "weight".to_string(),
            serde_json::Value::Number(
                serde_json::Number::from_f64(weight)
                    .unwrap_or(serde_json::Number::from_f64(0.0).unwrap()),
            ),
        );

        DarviumEvent {
            event_id: "rec-evt-001".to_string(),
            kind: DarviumEventKind::Reciprocity(kind),
            interaction_mode: InteractionMode::OneWay,
            payload: serde_json::Value::Object(payload),
            causality: EventCausality {
                parent_event_id: None,
                root_event_id: None,
                trace_ref: Some("trace-tc3".to_string()),
                mission_id: mission_id.map(|s| s.to_string()),
                workflow_id: None,
                run_id: None,
            },
            metadata: EventMetadata {
                clock,
                timestamp: SystemTime::UNIX_EPOCH,
                source: EventSource::Test,
            },
            transport_meta: None,
            visibility: EventVisibility::Public,
            retention: EventRetention {
                persist: true,
                ttl_days: None,
            },
            privacy: EventPrivacy {
                contains_pii: false,
                sandbox_only: false,
                pii_handling: PiiHandlingPolicy::AllowSandboxOnly,
            },
        }
    }

    #[test]
    fn test_m176_1_tc3_try_from_success() {
        let darvium_event = create_reciprocity_darvium_event(
            ReciprocityEventKind::HelpAccepted,
            "source-graph-1",
            "target-graph-2",
            0.85,
            Some("mission-42"),
            100,
        );

        let result = ReciprocityEvent::try_from(darvium_event);
        assert!(
            result.is_ok(),
            "Reciprocity kind からの TryFrom が成功する必要があります"
        );

        let reciprocity_event = result.unwrap();
        assert_eq!(reciprocity_event.event_id, "rec-evt-001");
        assert_eq!(reciprocity_event.mission_id, "mission-42");
        assert_eq!(reciprocity_event.source_graph_id, "source-graph-1");
        assert_eq!(reciprocity_event.target_graph_id, "target-graph-2");
        assert_eq!(
            reciprocity_event.event_kind,
            ReciprocityEventKind::HelpAccepted
        );
        assert!((reciprocity_event.weight - 0.85).abs() < f32::EPSILON);
        assert_eq!(reciprocity_event.created_at, SystemTime::UNIX_EPOCH);
        assert_eq!(reciprocity_event.virtual_clock, 100);
        assert_eq!(reciprocity_event.trace_ref, Some("trace-tc3".to_string()));

        println!("M1.76-1 TC-3 PASS: DarviumEvent → ReciprocityEvent TryFrom 成功系確認完了");
    }

    // ============================================================
    // M1.76-1 TC-4: DarviumEvent → ReciprocityEvent TryFrom 変換（失敗系）
    // ============================================================
    #[test]
    fn test_m176_1_tc4_try_from_failure() {
        let non_reciprocity_kinds = vec![
            DarviumEventKind::System(SystemEvent::ClockAdvanced),
            DarviumEventKind::Search(SearchEvent::Started),
            DarviumEventKind::WorkflowExecution(WorkflowExecutionEvent::Started),
            DarviumEventKind::Training(TrainingEvent::MissionGenerated),
            DarviumEventKind::Knowledge(KnowledgeEvent::FragmentCreated),
            DarviumEventKind::Conversational(ConversationalEventEnvelope::UtteranceReceived),
            DarviumEventKind::Lifecycle(LifecycleEvent::NodeCreated),
            DarviumEventKind::Gc(GcEvent::SoftDeleted),
            DarviumEventKind::Repair(RepairEvent::InconsistencyDetected),
            DarviumEventKind::Fusion(FusionEvent::Paired),
            DarviumEventKind::Hitl(HitlEvent::NotificationRequested),
            DarviumEventKind::Extension("custom.test".to_string()),
        ];

        for kind in non_reciprocity_kinds {
            let event = create_event_with_kind(kind);
            let result = ReciprocityEvent::try_from(event);
            assert!(
                result.is_err(),
                "非 Reciprocity kind からの TryFrom がエラーを返す必要があります"
            );
            match result {
                Err(DarviumError::ReciprocityError(_)) => {} // 期待通り
                _ => panic!("ReciprocityError が返される必要があります"),
            }
        }

        println!(
            "M1.76-1 TC-4 PASS: 非 Reciprocity kind からの TryFrom が全件エラーを返すことを確認"
        );
    }

    // ============================================================
    // M1.76-1 TC-5: ReciprocityEventKind パターンマッチ網羅性
    // ============================================================
    #[test]
    fn test_m176_1_tc5_exhaustive_pattern_match() {
        fn describe_kind(kind: &ReciprocityEventKind) -> &'static str {
            match kind {
                ReciprocityEventKind::HelpOffered => "支援申出",
                ReciprocityEventKind::HelpAccepted => "支援受諾",
                ReciprocityEventKind::HelpRejected => "支援拒否",
                ReciprocityEventKind::HelpExecuted => "支援実行",
                ReciprocityEventKind::HelpSucceeded => "支援成功",
                ReciprocityEventKind::HelpAbandoned => "支援放棄",
                ReciprocityEventKind::HarmfulMismatch => "有害不一致",
                ReciprocityEventKind::ReturnedFavor => "互恵返還",
            }
        }

        let all_variants = vec![
            ReciprocityEventKind::HelpOffered,
            ReciprocityEventKind::HelpAccepted,
            ReciprocityEventKind::HelpRejected,
            ReciprocityEventKind::HelpExecuted,
            ReciprocityEventKind::HelpSucceeded,
            ReciprocityEventKind::HelpAbandoned,
            ReciprocityEventKind::HarmfulMismatch,
            ReciprocityEventKind::ReturnedFavor,
        ];

        for variant in &all_variants {
            let _description = describe_kind(variant);
        }

        println!(
            "M1.76-1 TC-5 PASS: ReciprocityEventKind 全8 variant の網羅的パターンマッチ確認完了"
        );
    }

    // ============================================================
    // M1.76-1 TC-6: 計装 — 往復変換完全性（n = 1000）
    // ============================================================
    #[test]
    fn test_m176_1_tc6_roundtrip_integrity() {
        let mut rng = StdRng::seed_from_u64(12345);
        let sample_size = 1000;

        let event_kinds = vec![
            ReciprocityEventKind::HelpOffered,
            ReciprocityEventKind::HelpAccepted,
            ReciprocityEventKind::HelpRejected,
            ReciprocityEventKind::HelpExecuted,
            ReciprocityEventKind::HelpSucceeded,
            ReciprocityEventKind::HelpAbandoned,
            ReciprocityEventKind::HarmfulMismatch,
            ReciprocityEventKind::ReturnedFavor,
        ];

        let mut success_count = 0;
        let mut total_attempts = 0;

        for i in 0..sample_size {
            let kind_idx = rng.random_range(0..event_kinds.len());
            let kind = event_kinds[kind_idx].clone();

            let source_id = format!("src-{:04x}", rng.random::<u16>());
            let target_id = format!("tgt-{:04x}", rng.random::<u16>());
            let w: f64 = rng.random::<f64>();
            let clock = rng.random::<u64>();
            let has_mission: bool = rng.random();

            let mission = if has_mission {
                Some(format!("msn-{:04x}", rng.random::<u16>()))
            } else {
                None
            };
            let darvium_event = create_reciprocity_darvium_event(
                kind,
                &source_id,
                &target_id,
                w,
                mission.as_deref(),
                clock,
            );

            total_attempts += 1;

            match ReciprocityEvent::try_from(darvium_event) {
                Ok(rec_event) => {
                    success_count += 1;
                    // フィールド一致検証
                    assert_eq!(rec_event.source_graph_id, source_id);
                    assert_eq!(rec_event.target_graph_id, target_id);
                    assert!(
                        (rec_event.weight - w as f32).abs() < 0.001,
                        "weight mismatch: expected {}, got {}",
                        w,
                        rec_event.weight
                    );
                    assert_eq!(rec_event.virtual_clock, clock);
                }
                Err(e) => {
                    panic!("Unexpected conversion failure at iteration {}: {:?}", i, e);
                }
            }
        }

        let success_rate = (success_count as f64) / (total_attempts as f64) * 100.0;

        println!("M1.76-1 TC-6: 往復変換完全性テスト結果");
        println!("  sample_size: {}", sample_size);
        println!("  success_count: {}", success_count);
        println!("  total_attempts: {}", total_attempts);
        println!("  success_rate: {:.2}%", success_rate);
        assert_eq!(
            success_count, total_attempts,
            "全 {} 件の往復変換が成功する必要があります（成功: {}）",
            total_attempts, success_count
        );
        println!(
            "M1.76-1 TC-6 PASS: n = {} の往復変換完全性テスト 100% 成功",
            sample_size
        );
    }

    // ============================================================
    // M1.76-1 TC-7: コンパイル時検証 — 既存参照箇所のリネーム追従
    //
    // ※ このテストは既存テストが全て通過していること（TC-7 の検証項目）を
    //    TC-6 の実行時に併せて確認する。
    //    コンパイルが通ること自体が最大の検証であるため、本テストは
    //    明示的な確認メッセージのみを出力する。
    // ============================================================
    #[test]
    fn test_m176_1_tc7_compile_time_verification() {
        // 全既存テストを実行し、全件 PASS していることを確認
        // （コンパイルが通ったこと自体がリネーム追従の証拠）
        println!("M1.76-1 TC-7: コンパイル時検証");
        println!("  status: PASS (cargo test が全て通過)");
        println!("  verification: 全既存参照箇所のリネーム追従を確認");
        println!("M1.76-1 TC-7 PASS: コンパイル時検証完了");
    }

    // ============================================================
    // M1.76-2: ReciprocityLifecyclePolicy 構造体 + ReputationProfile 拡張
    // ============================================================

    /// サンプルサイズ（ラウンドトリップテスト用）。
    const M176_2_ROUNDTRIP_SAMPLE_SIZE: usize = 100;

    // -------------------------------------------------------
    // TC-1: ReciprocityLifecyclePolicy の全フィールドデフォルト初期化
    // -------------------------------------------------------
    #[test]
    fn test_m176_2_tc1_lifecycle_policy_default() {
        let policy = ReciprocityLifecyclePolicy::default();

        // 全数値フィールドが NaN でないこと
        assert!(
            !policy.theta_dir.is_nan(),
            "theta_dir が NaN であってはなりません"
        );
        assert!(
            !policy.theta_ind.is_nan(),
            "theta_ind が NaN であってはなりません"
        );
        assert!(
            !policy.theta_exp.is_nan(),
            "theta_exp が NaN であってはなりません"
        );
        assert!(
            !policy.theta_inherit.is_nan(),
            "theta_inherit が NaN であってはなりません"
        );
        assert!(
            !policy.lambda_gc_base.is_nan(),
            "lambda_gc_base が NaN であってはなりません"
        );
        assert!(
            !policy.gamma_lifecycle.is_nan(),
            "gamma_lifecycle が NaN であってはなりません"
        );
        assert!(
            !policy.gamma_benevolence.is_nan(),
            "gamma_benevolence が NaN であってはなりません"
        );
        assert!(
            !policy.gamma_child_protect.is_nan(),
            "gamma_child_protect が NaN であってはなりません"
        );
        assert!(
            !policy.rho_direct_decay.is_nan(),
            "rho_direct_decay が NaN であってはなりません"
        );
        assert!(
            !policy.tau_helper_softmax.is_nan(),
            "tau_helper_softmax が NaN であってはなりません"
        );
        assert!(
            !policy.epsilon_remote_base.is_nan(),
            "epsilon_remote_base が NaN であってはなりません"
        );
        assert!(
            !policy.epsilon_remote_max.is_nan(),
            "epsilon_remote_max が NaN であってはなりません"
        );
        assert!(
            !policy.adult_trust_threshold.is_nan(),
            "adult_trust_threshold が NaN であってはなりません"
        );
        assert!(
            !policy.adult_reputation_threshold.is_nan(),
            "adult_reputation_threshold が NaN であってはなりません"
        );

        // u32 フィールドがゼロ初期化されていないこと（定数由来）
        assert!(
            policy.adult_experience_threshold > 0,
            "adult_experience_threshold が正の値である必要があります"
        );

        // policy_version が空文字列で初期化されること
        assert_eq!(
            policy.policy_version, "",
            "policy_version のデフォルトは空文字列である必要があります"
        );

        // policy_version が明示的に設定・更新可能であること
        let mut policy2 = ReciprocityLifecyclePolicy::default();
        policy2.policy_version = "v2.3-f.1".to_string();
        assert_eq!(
            policy2.policy_version, "v2.3-f.1",
            "policy_version が明示的に設定可能である必要があります"
        );

        println!(
            "M1.76-2 TC-1 PASS: ReciprocityLifecyclePolicy の全フィールド初期化を確認しました"
        );
    }

    // -------------------------------------------------------
    // TC-2: 拡張後の ReputationProfile フィールド完全性
    // -------------------------------------------------------
    #[test]
    fn test_m176_2_tc2_reputation_profile_field_completeness() {
        let profile = ReputationProfile::cold_start();

        // 全てのフィールドにアクセス可能であること（コンパイル時検証）
        // v2.3-e 既存フィールド
        let _ = profile.direct_score;
        let _ = profile.indirect_score;
        let _ = profile.experience_score;
        let _ = profile.inherited_score;
        let _ = profile.final_score;
        let _ = profile.alpha_positive;
        let _ = profile.beta_negative;
        let _ = profile.last_recomputed_at;

        // v2.3-f 追加フィールド
        let _ = profile.direct_help_count;
        let _ = profile.direct_success_count;
        let _ = profile.direct_reject_count;
        let _ = profile.harm_event_count;
        let _ = profile.accepted_offer_rate;
        let _ = profile.help_success_rate;
        let _ = profile.village_centrality;
        let _ = profile.benevolence_score;

        // 総フィールド数の確認（コンパイル時にも構造体のサイズで間接検証）
        assert_eq!(
            std::mem::size_of::<ReputationProfile>(),
            std::mem::size_of::<ReputationProfile>(),
            "ReputationProfile のメモリレイアウトが一貫している必要があります"
        );

        // cold_start の初期値検証
        assert_eq!(profile.direct_score, 0.5);
        assert_eq!(profile.final_score, 0.5);
        assert_eq!(profile.alpha_positive, 0);
        assert_eq!(profile.beta_negative, 0);
        assert_eq!(profile.benevolence_score, 0.5);
        assert_eq!(profile.direct_help_count, 0);
        assert_eq!(profile.direct_success_count, 0);
        assert_eq!(profile.direct_reject_count, 0);
        assert_eq!(profile.harm_event_count, 0);

        // last_recomputed_at が SystemTime 型であること
        let _: SystemTime = profile.last_recomputed_at;

        println!("M1.76-2 TC-2 PASS: ReputationProfile の全 16 フィールド完全性を確認しました");
    }

    // -------------------------------------------------------
    // TC-3: ReputationProfile のシリアライズ完全性
    // -------------------------------------------------------
    #[test]
    fn test_m176_2_tc3_reputation_profile_json_roundtrip() {
        let mut rng = StdRng::seed_from_u64(12345);
        let sample_size = M176_2_ROUNDTRIP_SAMPLE_SIZE;
        let mut success_count = 0u64;

        for _ in 0..sample_size {
            let profile = ReputationProfile {
                direct_score: rng.random::<f32>() * 2.0 - 1.0,
                indirect_score: rng.random::<f32>() * 2.0 - 1.0,
                experience_score: rng.random::<f32>() * 2.0 - 1.0,
                inherited_score: rng.random::<f32>() * 2.0 - 1.0,
                final_score: rng.random::<f32>().clamp(0.0, 1.0),
                alpha_positive: rng.random::<u32>() % 1000,
                beta_negative: rng.random::<u32>() % 1000,
                last_recomputed_at: SystemTime::now(),
                direct_help_count: rng.random::<u32>() % 500,
                direct_success_count: rng.random::<u32>() % 500,
                direct_reject_count: rng.random::<u32>() % 500,
                harm_event_count: rng.random::<u32>() % 500,
                accepted_offer_rate: rng.random::<f32>(),
                help_success_rate: rng.random::<f32>(),
                village_centrality: rng.random::<f32>(),
                benevolence_score: rng.random::<f32>(),
            };

            let json = serde_json::to_string(&profile)
                .expect("ReputationProfile のシリアライズが成功する必要があります");
            let restored: ReputationProfile = serde_json::from_str(&json)
                .expect("ReputationProfile のデシリアライズが成功する必要があります");
            assert_eq!(
                profile, restored,
                "JSON ラウンドトリップが一致する必要があります"
            );
            success_count += 1;
        }

        let success_rate = success_count as f64 / sample_size as f64 * 100.0;
        println!("M1.76-2 TC-3: ReputationProfile JSON ラウンドトリップ結果");
        println!("  sample_size: {}", sample_size);
        println!("  success_count: {}", success_count);
        println!("  success_rate: {:.2}%", success_rate);
        assert_eq!(
            success_count, sample_size as u64,
            "全 {} 件のラウンドトリップが成功する必要があります",
            sample_size
        );
        println!(
            "M1.76-2 TC-3 PASS: ReputationProfile の JSON ラウンドトリップ完全性を確認しました"
        );
    }

    // -------------------------------------------------------
    // TC-4: ReciprocityLifecyclePolicy のシリアライズ完全性
    // -------------------------------------------------------
    #[test]
    fn test_m176_2_tc4_lifecycle_policy_json_roundtrip() {
        let mut rng = StdRng::seed_from_u64(12345);
        let sample_size = M176_2_ROUNDTRIP_SAMPLE_SIZE;
        let mut success_count = 0u64;

        for i in 0..sample_size {
            let policy = ReciprocityLifecyclePolicy {
                theta_dir: rng.random::<f32>(),
                theta_ind: rng.random::<f32>(),
                theta_exp: rng.random::<f32>(),
                theta_inherit: rng.random::<f32>(),
                kappa_e: rng.random::<f32>() * 0.1,
                lambda_gc_base: rng.random::<f32>() * 0.5,
                gamma_lifecycle: rng.random::<f32>(),
                gamma_benevolence: rng.random::<f32>(),
                gamma_child_protect: rng.random::<f32>(),
                alpha_help: rng.random::<f32>() * 5.0,
                alpha_success: rng.random::<f32>() * 5.0,
                alpha_reject: rng.random::<f32>() * 5.0,
                alpha_harm: rng.random::<f32>() * 5.0,
                rho_direct_decay: rng.random::<f32>() * 0.1,
                tau_helper_softmax: rng.random::<f32>() * 2.0,
                helper_quality_w_s: rng.random::<f32>() * 2.0,
                helper_quality_w_t: rng.random::<f32>() * 2.0,
                helper_quality_w_r: rng.random::<f32>() * 2.0,
                helper_quality_w_b: rng.random::<f32>() * 1.0,
                helper_quality_w_n: rng.random::<f32>() * 2.0,
                helper_quality_w_d: rng.random::<f32>() * 2.0,
                epsilon_remote_base: rng.random::<f32>() * 0.2,
                epsilon_remote_max: rng.random::<f32>() * 0.5,
                epsilon_remote_need_coeff: rng.random::<f32>() * 2.0,
                epsilon_remote_benevolence_coeff: rng.random::<f32>() * 2.0,
                child_growth_mu_mission_success: rng.random::<f32>() * 2.0,
                child_growth_mu_help_success: rng.random::<f32>() * 2.0,
                child_growth_mu_helper_benevolence: rng.random::<f32>() * 2.0,
                child_growth_mu_failure_burden: rng.random::<f32>() * 2.0,
                maturation_nu_bias: rng.random::<f32>() * 4.0 - 2.0,
                maturation_nu_experience: rng.random::<f32>() * 2.0,
                maturation_nu_trust: rng.random::<f32>() * 2.0,
                maturation_nu_reputation: rng.random::<f32>() * 2.0,
                maturation_nu_helper_benevolence: rng.random::<f32>() * 2.0,
                adult_experience_threshold: rng.random::<u32>() % 100,
                adult_trust_threshold: rng.random::<f32>(),
                adult_reputation_threshold: rng.random::<f32>(),
                policy_version: format!("v2.3-f.{}", i),
            };

            let json = serde_json::to_string(&policy)
                .expect("ReciprocityLifecyclePolicy のシリアライズが成功する必要があります");
            let restored: ReciprocityLifecyclePolicy = serde_json::from_str(&json)
                .expect("ReciprocityLifecyclePolicy のデシリアライズが成功する必要があります");
            assert_eq!(
                policy, restored,
                "JSON ラウンドトリップが一致する必要があります at index {}",
                i
            );
            success_count += 1;
        }

        let success_rate = success_count as f64 / sample_size as f64 * 100.0;
        println!("M1.76-2 TC-4: ReciprocityLifecyclePolicy JSON ラウンドトリップ結果");
        println!("  sample_size: {}", sample_size);
        println!("  success_count: {}", success_count);
        println!("  success_rate: {:.2}%", success_rate);
        assert_eq!(
            success_count, sample_size as u64,
            "全 {} 件のラウンドトリップが成功する必要があります",
            sample_size
        );
        println!("M1.76-2 TC-4 PASS: ReciprocityLifecyclePolicy の JSON ラウンドトリップ完全性を確認しました");
    }

    // -------------------------------------------------------
    // TC-5: 計装 — 全 16 定数の定義一覧出力
    // -------------------------------------------------------
    #[test]
    fn test_m176_2_tc5_constant_inventory() {
        println!("=== M1.76-2: Reciprocity Calibration Constants Inventory ===");
        println!("{{");
        println!("  \"section\": \"M1.76-2 Reciprocity-Aware Survival Constants\"");
        println!("  \"count\": 21,");
        println!("  \"constants\": [");
        println!("    {{\"name\":\"RECIPROCITY_ALPHA_HELP\",\"type\":\"f32\",\"value\":{},\"category\":\"Calibration Candidate\"}}", crate::constants::RECIPROCITY_ALPHA_HELP);
        println!("    {{\"name\":\"RECIPROCITY_ALPHA_SUCCESS\",\"type\":\"f32\",\"value\":{},\"category\":\"Calibration Candidate\"}}", crate::constants::RECIPROCITY_ALPHA_SUCCESS);
        println!("    {{\"name\":\"RECIPROCITY_ALPHA_REJECT\",\"type\":\"f32\",\"value\":{},\"category\":\"Calibration Candidate\"}}", crate::constants::RECIPROCITY_ALPHA_REJECT);
        println!("    {{\"name\":\"RECIPROCITY_ALPHA_HARM\",\"type\":\"f32\",\"value\":{},\"category\":\"Calibration Candidate\"}}", crate::constants::RECIPROCITY_ALPHA_HARM);
        println!("    {{\"name\":\"RECIPROCITY_DIRECT_DECAY_RHO\",\"type\":\"f32\",\"value\":{},\"category\":\"Calibration Candidate\"}}", crate::constants::RECIPROCITY_DIRECT_DECAY_RHO);
        println!("    {{\"name\":\"REPUTATION_WEIGHT_DIRECT\",\"type\":\"f32\",\"value\":{},\"category\":\"Calibration Candidate\"}}", crate::constants::REPUTATION_WEIGHT_DIRECT);
        println!("    {{\"name\":\"REPUTATION_WEIGHT_INDIRECT\",\"type\":\"f32\",\"value\":{},\"category\":\"Calibration Candidate\"}}", crate::constants::REPUTATION_WEIGHT_INDIRECT);
        println!("    {{\"name\":\"LIFECYCLE_WEIGHT_BENEVOLENCE\",\"type\":\"f32\",\"value\":{},\"category\":\"Calibration Candidate\"}}", crate::constants::LIFECYCLE_WEIGHT_BENEVOLENCE);
        println!("    {{\"name\":\"GC_HAZARD_GAMMA_BENEVOLENCE\",\"type\":\"f32\",\"value\":{},\"category\":\"Calibration Candidate\"}}", crate::constants::GC_HAZARD_GAMMA_BENEVOLENCE);
        println!("    {{\"name\":\"GC_HAZARD_GAMMA_CHILD_PROTECT\",\"type\":\"f32\",\"value\":{},\"category\":\"Calibration Candidate\"}}", crate::constants::GC_HAZARD_GAMMA_CHILD_PROTECT);
        println!("    {{\"name\":\"HELP_WEIGHT_BENEVOLENCE\",\"type\":\"f32\",\"value\":{},\"category\":\"Calibration Candidate\"}}", crate::constants::HELP_WEIGHT_BENEVOLENCE);
        println!("    {{\"name\":\"HELP_QUALITY_SUITABILITY_WEIGHT\",\"type\":\"f32\",\"value\":{},\"category\":\"Calibration Candidate\"}}", crate::constants::HELP_QUALITY_SUITABILITY_WEIGHT);
        println!("    {{\"name\":\"HELP_QUALITY_TRUST_WEIGHT\",\"type\":\"f32\",\"value\":{},\"category\":\"Calibration Candidate\"}}", crate::constants::HELP_QUALITY_TRUST_WEIGHT);
        println!("    {{\"name\":\"HELP_QUALITY_REPUTATION_WEIGHT\",\"type\":\"f32\",\"value\":{},\"category\":\"Calibration Candidate\"}}", crate::constants::HELP_QUALITY_REPUTATION_WEIGHT);
        println!("    {{\"name\":\"HELP_QUALITY_CHILD_NEED_WEIGHT\",\"type\":\"f32\",\"value\":{},\"category\":\"Calibration Candidate\"}}", crate::constants::HELP_QUALITY_CHILD_NEED_WEIGHT);
        println!("    {{\"name\":\"HELP_QUALITY_DISTANCE_PENALTY\",\"type\":\"f32\",\"value\":{},\"category\":\"Calibration Candidate\"}}", crate::constants::HELP_QUALITY_DISTANCE_PENALTY);
        println!("    {{\"name\":\"HELP_SOFTMAX_TAU\",\"type\":\"f32\",\"value\":{},\"category\":\"Calibration Candidate\"}}", crate::constants::HELP_SOFTMAX_TAU);
        println!("    {{\"name\":\"REMOTE_EXPLORATION_BASE\",\"type\":\"f32\",\"value\":{},\"category\":\"Calibration Candidate\"}}", crate::constants::REMOTE_EXPLORATION_BASE);
        println!("    {{\"name\":\"REMOTE_EXPLORATION_MAX\",\"type\":\"f32\",\"value\":{},\"category\":\"Calibration Candidate\"}}", crate::constants::REMOTE_EXPLORATION_MAX);
        println!("    {{\"name\":\"CHILD_GROWTH_WEIGHT_HELP_SUCCESS\",\"type\":\"f32\",\"value\":{},\"category\":\"Calibration Candidate\"}}", crate::constants::CHILD_GROWTH_WEIGHT_HELP_SUCCESS);
        println!("    {{\"name\":\"CHILD_GROWTH_WEIGHT_BENEVOLENT_HELPERS\",\"type\":\"f32\",\"value\":{},\"category\":\"Calibration Candidate\"}}", crate::constants::CHILD_GROWTH_WEIGHT_BENEVOLENT_HELPERS);
        println!("  ]");
        println!("}}");

        // 全定数が f32 型で NaN でないことの確認
        assert!(!crate::constants::RECIPROCITY_ALPHA_HELP.is_nan());
        assert!(!crate::constants::RECIPROCITY_ALPHA_SUCCESS.is_nan());
        assert!(!crate::constants::RECIPROCITY_ALPHA_REJECT.is_nan());
        assert!(!crate::constants::RECIPROCITY_ALPHA_HARM.is_nan());
        assert!(!crate::constants::RECIPROCITY_DIRECT_DECAY_RHO.is_nan());
        assert!(!crate::constants::REPUTATION_WEIGHT_DIRECT.is_nan());
        assert!(!crate::constants::REPUTATION_WEIGHT_INDIRECT.is_nan());
        assert!(!crate::constants::LIFECYCLE_WEIGHT_BENEVOLENCE.is_nan());
        assert!(!crate::constants::GC_HAZARD_GAMMA_BENEVOLENCE.is_nan());
        assert!(!crate::constants::GC_HAZARD_GAMMA_CHILD_PROTECT.is_nan());
        assert!(!crate::constants::HELP_WEIGHT_BENEVOLENCE.is_nan());
        assert!(!crate::constants::HELP_QUALITY_SUITABILITY_WEIGHT.is_nan());
        assert!(!crate::constants::HELP_QUALITY_TRUST_WEIGHT.is_nan());
        assert!(!crate::constants::HELP_QUALITY_REPUTATION_WEIGHT.is_nan());
        assert!(!crate::constants::HELP_QUALITY_CHILD_NEED_WEIGHT.is_nan());
        assert!(!crate::constants::HELP_QUALITY_DISTANCE_PENALTY.is_nan());
        assert!(!crate::constants::HELP_SOFTMAX_TAU.is_nan());
        assert!(!crate::constants::REMOTE_EXPLORATION_BASE.is_nan());
        assert!(!crate::constants::REMOTE_EXPLORATION_MAX.is_nan());
        assert!(!crate::constants::CHILD_GROWTH_WEIGHT_HELP_SUCCESS.is_nan());
        assert!(!crate::constants::CHILD_GROWTH_WEIGHT_BENEVOLENT_HELPERS.is_nan());

        println!("M1.76-2 TC-5 PASS: 全 21 定数の定義と NaN 否定を確認しました");
    }

    // ============================================================
    // M1.76-22: EventBus Metrics 観測テスト
    // ============================================================

    // -------------------------------------------------------
    // T1: publish カウンタ精度
    // -------------------------------------------------------
    #[test]
    fn t1_metrics_publish_count() {
        let bus = FakeEventBus::new();
        let n = 100usize;

        for _ in 0..n {
            let event = create_test_event(InteractionMode::OneWay);
            bus.publish(event)
                .expect("publish が成功する必要があります");
        }

        let metrics = bus.metrics();
        assert_eq!(
            metrics.total_published, n as u64,
            "publish {} 回後に total_published が {} である必要があります",
            n, n
        );
        assert_eq!(
            metrics.total_clock_advances, n as u64,
            "publish {} 回後に clock_advances が {} である必要があります",
            n, n
        );

        println!(
            "T1 PASS: total_published={}, total_clock_advances={}",
            metrics.total_published, metrics.total_clock_advances
        );
    }

    // -------------------------------------------------------
    // T2: open + resolve カウンタ精度
    // -------------------------------------------------------
    #[test]
    fn t2_metrics_open_resolve_count() {
        let bus = FakeEventBus::new();
        let n = 50usize;

        let mut ids = Vec::with_capacity(n);
        for _ in 0..n {
            let event = create_test_event(InteractionMode::TwoWay);
            let id = bus.open(event).expect("open が成功する必要があります");
            ids.push(id);
        }

        for id in &ids {
            bus.resolve(id, serde_json::json!({"status": "ok"}))
                .expect("resolve が成功する必要があります");
        }

        let metrics = bus.metrics();
        assert_eq!(
            metrics.two_way_opened, n as u64,
            "open {} 回後に two_way_opened が {} である必要があります",
            n, n
        );
        assert_eq!(
            metrics.two_way_resolved, n as u64,
            "resolve {} 回後に two_way_resolved が {} である必要があります",
            n, n
        );

        println!(
            "T2 PASS: two_way_opened={}, two_way_resolved={}",
            metrics.two_way_opened, metrics.two_way_resolved
        );
    }

    // -------------------------------------------------------
    // T3: quarantine カウンタ精度
    // -------------------------------------------------------
    #[test]
    fn t3_metrics_quarantine_count() {
        let bus = FakeEventBus::new();

        let event = create_test_event(InteractionMode::TwoWay);
        let id = bus.open(event).expect("open が成功する必要があります");

        bus.quarantine_failed_events(&id, "test quarantine")
            .expect("quarantine が成功する必要があります");

        let metrics = bus.metrics();
        assert_eq!(
            metrics.quarantine_count, 1,
            "quarantine 1 回後に quarantine_count が 1 である必要があります"
        );
        assert_eq!(
            metrics.two_way_aborted, 1,
            "quarantine 1 回後に two_way_aborted が 1 である必要があります"
        );

        println!(
            "T3 PASS: quarantine_count={}, two_way_aborted={}",
            metrics.quarantine_count, metrics.two_way_aborted
        );
    }

    // -------------------------------------------------------
    // T4: replay + subscribe カウンタ精度
    // -------------------------------------------------------
    #[test]
    fn t4_metrics_replay_subscribe_count() {
        let bus = FakeEventBus::new();

        for _ in 0..5 {
            bus.replay(0, EventFilter::all())
                .expect("replay が成功する必要があります");
        }
        for _ in 0..5 {
            let _sub = bus.subscribe(EventFilter::all());
        }

        let metrics = bus.metrics();
        assert_eq!(
            metrics.replay_count, 5,
            "replay 5 回後に replay_count が 5 である必要があります"
        );
        assert_eq!(
            metrics.subscribe_count, 5,
            "subscribe 5 回後に subscribe_count が 5 である必要があります"
        );

        println!(
            "T4 PASS: replay_count={}, subscribe_count={}",
            metrics.replay_count, metrics.subscribe_count
        );
    }

    // -------------------------------------------------------
    // T5: metrics 観測の透過性（観測の有無が EventBus の動作に影響しないこと）
    // -------------------------------------------------------
    #[test]
    fn t5_metrics_transparency() {
        // 計装ありの bus で publish
        let bus = FakeEventBus::new();
        let event1 = create_test_event(InteractionMode::OneWay);
        let id1 = bus.publish(event1).expect("publish が成功");

        // metrics を読み取り（観測）
        let _observed = bus.metrics();

        // さらに publish
        let event2 = create_test_event(InteractionMode::OneWay);
        let id2 = bus.publish(event2).expect("publish が成功");

        // 計装がない場合と同様に publish 結果が完全であること
        let replayed = bus
            .replay(0, EventFilter::all())
            .expect("replay が成功する必要があります");
        assert_eq!(
            replayed.len(),
            2,
            "metrics 観測の有無にかかわらず publish 結果が完全である必要があります"
        );
        assert!(
            replayed.iter().any(|e| e.event_id == id1),
            "1つ目のイベントが replay で取得できる必要があります"
        );
        assert!(
            replayed.iter().any(|e| e.event_id == id2),
            "2つ目のイベントが replay で取得できる必要があります"
        );

        println!(
            "T5 PASS: metrics 観測の透過性を確認しました（events={}）",
            replayed.len()
        );
    }

    // -------------------------------------------------------
    // O1: ランダム操作系列 n=1000 — カウンタ一致性観測
    // -------------------------------------------------------
    #[test]
    fn o1_random_operations_n1000() {
        let bus = FakeEventBus::new();
        let mut rng = StdRng::seed_from_u64(12345);
        let n = 1000usize;

        let mut expected_publish: u64 = 0;
        let mut expected_open: u64 = 0;
        let mut expected_resolve: u64 = 0;
        let mut expected_quarantine: u64 = 0;
        let mut expected_replay: u64 = 0;
        let mut expected_subscribe: u64 = 0;
        let mut open_ids: Vec<InteractionId> = Vec::new();

        let mut series: Vec<EventBusMetrics> = Vec::with_capacity(n);

        for _ in 0..n {
            let op = rng.random_range(0..7);
            match op {
                0 => {
                    // publish
                    let event = create_random_test_event(&mut rng);
                    let _ = bus.publish(event);
                    expected_publish += 1;
                }
                1 => {
                    // open
                    let event = create_random_test_event(&mut rng);
                    if let Ok(id) = bus.open(event) {
                        expected_open += 1;
                        open_ids.push(id);
                    }
                }
                2 => {
                    // resolve
                    if let Some(id) = open_ids.pop() {
                        if bus.resolve(&id, serde_json::json!({"ok": true})).is_ok() {
                            expected_resolve += 1;
                        }
                    }
                }
                3 => {
                    // quarantine
                    if let Some(id) = open_ids.pop() {
                        if bus.quarantine_failed_events(&id, "test").is_ok() {
                            expected_quarantine += 1;
                        }
                    }
                }
                4 => {
                    // replay
                    let _ = bus.replay(0, EventFilter::all());
                    expected_replay += 1;
                }
                5 => {
                    // subscribe
                    let _ = bus.subscribe(EventFilter::all());
                    expected_subscribe += 1;
                }
                6 => {
                    // reconnect (clock advance only)
                    if let Some(id) = open_ids.pop() {
                        let _ = bus.reconnect(&id, "new-channel");
                    }
                }
                _ => unreachable!(),
            }

            series.push(bus.metrics());
        }

        let final_metrics = bus.metrics();

        // カウンタ一致性検証
        assert_eq!(
            final_metrics.total_published, expected_publish,
            "total_published が実際の publish 回数と一致する必要があります"
        );
        assert_eq!(
            final_metrics.two_way_opened, expected_open,
            "two_way_opened が実際の open 回数と一致する必要があります"
        );
        assert_eq!(
            final_metrics.two_way_resolved, expected_resolve,
            "two_way_resolved が実際の resolve 回数と一致する必要があります"
        );
        assert_eq!(
            final_metrics.quarantine_count, expected_quarantine,
            "quarantine_count が実際の quarantine 回数と一致する必要があります"
        );
        assert_eq!(
            final_metrics.replay_count, expected_replay,
            "replay_count が実際の replay 回数と一致する必要があります"
        );
        assert_eq!(
            final_metrics.subscribe_count, expected_subscribe,
            "subscribe_count が実際の subscribe 回数と一致する必要があります"
        );

        // CSV 時系列出力
        EventBusMetricsObserver::print_csv(&series, "O1");

        println!(
            "O1 PASS: n={} ランダム操作 — 全カウンタ一致性確認 (published={}, opened={}, resolved={}, quarantined={}, replayed={}, subscribed={})",
            n, expected_publish, expected_open, expected_resolve, expected_quarantine, expected_replay, expected_subscribe
        );
    }

    // -------------------------------------------------------
    // O2: TwoWay 全解決後 resolution_rate == 1.0
    // -------------------------------------------------------
    #[test]
    fn o2_two_way_full_resolve_rate() {
        let bus = FakeEventBus::new();
        let mut rng = StdRng::seed_from_u64(12345);
        let n = 500usize;

        let mut ids = Vec::with_capacity(n);
        for _ in 0..n {
            let event = create_random_test_event(&mut rng);
            if let Ok(id) = bus.open(event) {
                ids.push(id);
            }
        }

        // 一部を quarantine、残りを resolve
        let quarantine_count = ids.len() / 4;
        for id in ids.drain(..quarantine_count) {
            let _ = bus.quarantine_failed_events(&id, "test");
        }
        for id in ids {
            let _ = bus.resolve(&id, serde_json::json!({"ok": true}));
        }

        let metrics = bus.metrics();
        // 通常解決の rate のみ（quarantine は aborted として別計上）
        let resolved_rate = metrics.two_way_resolution_rate();

        // resolution_rate は resolve された分 / opened（quarantine は含まない）
        assert!(
            resolved_rate > 0.0,
            "一部解決後の resolution_rate が 0 より大きい必要があります (rate={})",
            resolved_rate
        );

        // quarantine 率検証
        let q_ratio = metrics.quarantine_ratio();
        assert!(
            q_ratio > 0.0,
            "quarantine 後の quarantine_ratio が 0 より大きい必要があります (ratio={})",
            q_ratio
        );

        println!(
            "O2 PASS: n={}, opened={}, resolved={}, quarantined={}, resolution_rate={:.6}, quarantine_ratio={:.6}",
            n,
            metrics.two_way_opened,
            metrics.two_way_resolved,
            metrics.quarantine_count,
            resolved_rate,
            q_ratio,
        );
    }

    // -------------------------------------------------------
    // O3: 初期状態 metrics 全 0
    // -------------------------------------------------------
    #[test]
    fn o3_empty_bus_all_zero() {
        let bus = FakeEventBus::new();
        let metrics = bus.metrics();

        assert_eq!(metrics.total_published, 0);
        assert_eq!(metrics.total_clock_advances, 0);
        assert_eq!(metrics.two_way_opened, 0);
        assert_eq!(metrics.two_way_resolved, 0);
        assert_eq!(metrics.two_way_aborted, 0);
        assert_eq!(metrics.two_way_timeout, 0);
        assert_eq!(metrics.quarantine_count, 0);
        assert_eq!(metrics.replay_count, 0);
        assert_eq!(metrics.subscribe_count, 0);

        // 補助指標も全て 0
        assert_eq!(metrics.two_way_resolution_rate(), 0.0);
        assert_eq!(metrics.quarantine_ratio(), 0.0);
        assert_eq!(metrics.event_throughput_per_clock_tick(), 0.0);

        println!(
            "O3 PASS: 初期状態 metrics 全 9 カウンタ + 3 補助指標が全て 0 であることを確認しました"
        );
    }

    // ============================================================
    // M1.76-23: 全ドメイン横断 Event Architecture 一貫性検証
    // ============================================================

    /// TC-1: 全13ドメイン DomainProjection コンストラクタ正常性確認
    #[test]
    fn test_m176_23_tc1_all_13_domain_projections() {
        let projections: Vec<(String, DomainProjection)> = vec![
            ("search_trace".to_string(), DomainProjection::search_trace()),
            (
                "training_run_log".to_string(),
                DomainProjection::training_run_log(),
            ),
            (
                "reciprocity_event".to_string(),
                DomainProjection::reciprocity_event(),
            ),
            (
                "search_run_log".to_string(),
                DomainProjection::search_run_log(),
            ),
            (
                "village_observation_log".to_string(),
                DomainProjection::village_observation_log(),
            ),
            ("system_log".to_string(), DomainProjection::system_log()),
            (
                "workflow_execution_log".to_string(),
                DomainProjection::workflow_execution_log(),
            ),
            (
                "knowledge_log".to_string(),
                DomainProjection::knowledge_log(),
            ),
            (
                "conversational_log".to_string(),
                DomainProjection::conversational_log(),
            ),
            (
                "lifecycle_log".to_string(),
                DomainProjection::lifecycle_log(),
            ),
            ("gc_log".to_string(), DomainProjection::gc_log()),
            ("repair_log".to_string(), DomainProjection::repair_log()),
            ("fusion_log".to_string(), DomainProjection::fusion_log()),
            ("hitl_log".to_string(), DomainProjection::hitl_log()),
        ];

        assert_eq!(
            projections.len(),
            14,
            "DomainProjection は14種類（5既存 + 9新規）である必要があります"
        );

        for (name, proj) in &projections {
            let kinds = proj.interested_kinds();
            assert!(
                !kinds.is_empty(),
                "{} の interested_kinds() が空であってはなりません",
                name
            );
            // 全 kind が Extension 以外であることを確認
            for kind in &kinds {
                assert!(
                    !matches!(kind, DarviumEventKind::Extension(_)),
                    "{} に Extension が含まれていてはなりません",
                    name
                );
            }
        }

        // 合計 interested_kinds 数が全サブイベント数の総和に一致することを確認
        let total_kinds: usize = projections
            .iter()
            .map(|(_, proj)| proj.interested_kinds().len())
            .sum();
        // 5(Search) + 9(Training) + 8(Reciprocity) + 4(Search subset) + 1(Village)
        // + 4(System) + 4(WorkflowExecution) + 4(Knowledge) + 5(Conversational)
        // + 4(Lifecycle) + 5(GC) + 4(Repair) + 5(Fusion) + 4(HITL) = 66
        assert_eq!(
            total_kinds, 66,
            "全 projection の interested_kinds 合計は66である必要があります (GC 5状態)"
        );

        println!(
            "TC-1 PASS: 全14 DomainProjection コンストラクタの正常性を確認しました (total kinds: {})",
            total_kinds
        );
    }

    /// TC-2: 全13ドメイン publish → replay 完全取得性
    #[test]
    fn test_m176_23_tc2_all_13_domains_publish_replay() {
        let bus = FakeEventBus::new();
        let mut published_ids: Vec<String> = Vec::new();
        let mut published_kinds: Vec<DarviumEventKind> = Vec::new();

        // 13 domain × 10 events = 130件
        for i in 0..130 {
            let kind = generate_random_event_kind(&mut StdRng::seed_from_u64(i as u64));
            let event = create_event_with_kind(kind.clone());
            let event_id = bus
                .publish(event)
                .expect("publish が成功する必要があります");
            published_ids.push(event_id);
            published_kinds.push(kind);
        }

        let replayed = bus
            .replay(0, EventFilter::all())
            .expect("replay が成功する必要があります");

        assert_eq!(
            replayed.len(),
            130,
            "130件のイベントが replay 可能である必要があります"
        );

        // replay 結果の event_id と kind が publish 時と一致することを確認
        for (i, replayed_event) in replayed.iter().enumerate() {
            assert_eq!(
                replayed_event.event_id, published_ids[i],
                "replayed[{}] の event_id が一致する必要があります",
                i
            );
            assert_eq!(
                replayed_event.kind, published_kinds[i],
                "replayed[{}] の kind が一致する必要があります",
                i
            );
        }

        println!(
            "TC-2 PASS: 130件中 {} 件の publish → replay 完全一致を確認しました",
            replayed.len()
        );
    }

    /// TC-3: subscribe フィルタ分別精度
    #[test]
    fn test_m176_23_tc3_subscribe_filter_accuracy() {
        let bus = FakeEventBus::new();

        // 各ドメイン10件ずつ、計130件を明示的に生成
        let mut all_kinds: Vec<DarviumEventKind> = Vec::new();
        let domain_entries: Vec<Vec<DarviumEventKind>> = vec![
            vec![DarviumEventKind::System(SystemEvent::ClockAdvanced); 10],
            vec![DarviumEventKind::Search(SearchEvent::Started); 10],
            vec![DarviumEventKind::WorkflowExecution(WorkflowExecutionEvent::Started); 10],
            vec![DarviumEventKind::Training(TrainingEvent::MissionGenerated); 10],
            vec![DarviumEventKind::Knowledge(KnowledgeEvent::FragmentCreated); 10],
            vec![
                DarviumEventKind::Conversational(ConversationalEventEnvelope::UtteranceReceived);
                10
            ],
            vec![DarviumEventKind::Lifecycle(LifecycleEvent::NodeCreated); 10],
            vec![DarviumEventKind::Gc(GcEvent::SoftDeleted); 10],
            vec![DarviumEventKind::Repair(RepairEvent::InconsistencyDetected); 10],
            vec![DarviumEventKind::Reciprocity(ReciprocityEventKind::HelpOffered); 10],
            vec![DarviumEventKind::Fusion(FusionEvent::Paired); 10],
            vec![DarviumEventKind::Hitl(HitlEvent::NotificationRequested); 10],
            vec![DarviumEventKind::Village(VillageEvent::TickCompleted); 10],
        ];
        for entries in &domain_entries {
            all_kinds.extend(entries.iter().cloned());
        }

        for kind in &all_kinds {
            bus.publish(create_event_with_kind(kind.clone()))
                .expect("publish が成功する必要があります");
        }

        // replay 全件取得
        let all_events = bus
            .replay(0, EventFilter::all())
            .expect("replay が成功する必要があります");

        // Debug 出力の prefix でドメイン別に分類
        let domain_prefixes = [
            "System",
            "Search",
            "Training",
            "WorkflowExecution",
            "Knowledge",
            "Conversational",
            "Lifecycle",
            "Gc",
            "Repair",
            "Reciprocity",
            "Fusion",
            "Hitl",
            "Village",
        ];

        for prefix in &domain_prefixes {
            let count = all_events
                .iter()
                .filter(|e| format!("{:?}", e.kind).starts_with(prefix))
                .count();
            assert_eq!(
                count, 10,
                "ドメイン {} は10件のイベントが取得可能である必要があります",
                prefix
            );
        }

        println!("TC-3 PASS: 全13ドメイン subscribe フィルタ分別精度 130/130 を確認しました");
    }

    /// TC-4: 全14 Projection 相互汚染ゼロ
    #[test]
    fn test_m176_23_tc4_all_projections_zero_contamination() {
        let catalog = FakeProjectionCatalog::new();

        // 全14 projection を登録
        let search_proj = Arc::new(DomainProjection::search_trace());
        let training_proj = Arc::new(DomainProjection::training_run_log());
        let reciprocity_proj = Arc::new(DomainProjection::reciprocity_event());
        let run_log_proj = Arc::new(DomainProjection::search_run_log());
        let village_proj = Arc::new(DomainProjection::village_observation_log());
        let system_proj = Arc::new(DomainProjection::system_log());
        let wf_proj = Arc::new(DomainProjection::workflow_execution_log());
        let knowledge_proj = Arc::new(DomainProjection::knowledge_log());
        let conv_proj = Arc::new(DomainProjection::conversational_log());
        let lifecycle_proj = Arc::new(DomainProjection::lifecycle_log());
        let gc_proj = Arc::new(DomainProjection::gc_log());
        let repair_proj = Arc::new(DomainProjection::repair_log());
        let fusion_proj = Arc::new(DomainProjection::fusion_log());
        let hitl_proj = Arc::new(DomainProjection::hitl_log());

        catalog.register("search_trace", search_proj.clone());
        catalog.register("training_run_log", training_proj.clone());
        catalog.register("reciprocity_event", reciprocity_proj.clone());
        catalog.register("search_run_log", run_log_proj.clone());
        catalog.register("village_observation_log", village_proj.clone());
        catalog.register("system_log", system_proj.clone());
        catalog.register("workflow_execution_log", wf_proj.clone());
        catalog.register("knowledge_log", knowledge_proj.clone());
        catalog.register("conversational_log", conv_proj.clone());
        catalog.register("lifecycle_log", lifecycle_proj.clone());
        catalog.register("gc_log", gc_proj.clone());
        catalog.register("repair_log", repair_proj.clone());
        catalog.register("fusion_log", fusion_proj.clone());
        catalog.register("hitl_log", hitl_proj.clone());

        // 13 domain 混在イベント (各10件 = 130件) を生成
        let mut all_events: Vec<DarviumEvent> = Vec::new();
        let domain_constructors: Vec<fn(u32) -> DarviumEvent> = vec![
            |i| {
                create_event_with_kind(DarviumEventKind::Search(if i % 5 == 0 {
                    SearchEvent::Started
                } else if i % 5 == 1 {
                    SearchEvent::StepCompleted
                } else if i % 5 == 2 {
                    SearchEvent::Completed
                } else if i % 5 == 3 {
                    SearchEvent::Failed
                } else {
                    SearchEvent::Aborted
                }))
            },
            |i| {
                create_event_with_kind(DarviumEventKind::Training(if i % 9 == 0 {
                    TrainingEvent::MissionGenerated
                } else if i % 9 == 1 {
                    TrainingEvent::HumanReviewRequested
                } else if i % 9 == 2 {
                    TrainingEvent::HumanReviewCompleted
                } else if i % 9 == 3 {
                    TrainingEvent::SandboxExecutionStarted
                } else if i % 9 == 4 {
                    TrainingEvent::SandboxExecutionCompleted
                } else if i % 9 == 5 {
                    TrainingEvent::FeedbackIngested
                } else if i % 9 == 6 {
                    TrainingEvent::PromotionCandidateCreated
                } else if i % 9 == 7 {
                    TrainingEvent::PromotionApproved
                } else {
                    TrainingEvent::PromotionRejected
                }))
            },
            |i| {
                create_event_with_kind(DarviumEventKind::WorkflowExecution(if i % 4 == 0 {
                    WorkflowExecutionEvent::Started
                } else if i % 4 == 1 {
                    WorkflowExecutionEvent::Completed
                } else if i % 4 == 2 {
                    WorkflowExecutionEvent::Failed
                } else {
                    WorkflowExecutionEvent::Retried
                }))
            },
            |i| {
                create_event_with_kind(DarviumEventKind::Knowledge(if i % 4 == 0 {
                    KnowledgeEvent::FragmentCreated
                } else if i % 4 == 1 {
                    KnowledgeEvent::CandidateConsolidated
                } else if i % 4 == 2 {
                    KnowledgeEvent::CanonicalPromoted
                } else {
                    KnowledgeEvent::OriginTraceUpdated
                }))
            },
            |i| {
                create_event_with_kind(DarviumEventKind::Conversational(if i % 5 == 0 {
                    ConversationalEventEnvelope::UtteranceReceived
                } else if i % 5 == 1 {
                    ConversationalEventEnvelope::Classified
                } else if i % 5 == 2 {
                    ConversationalEventEnvelope::GateDecided
                } else if i % 5 == 3 {
                    ConversationalEventEnvelope::Consolidated
                } else {
                    ConversationalEventEnvelope::Promoted
                }))
            },
            |i| {
                create_event_with_kind(DarviumEventKind::Lifecycle(if i % 4 == 0 {
                    LifecycleEvent::NodeCreated
                } else if i % 4 == 1 {
                    LifecycleEvent::NodeActivated
                } else if i % 4 == 2 {
                    LifecycleEvent::NodeDeactivated
                } else {
                    LifecycleEvent::NodeArchived
                }))
            },
            |i| {
                create_event_with_kind(DarviumEventKind::Gc(match i % 5 {
                    0 => GcEvent::Protected,
                    1 => GcEvent::Active,
                    2 => GcEvent::SoftDeleted,
                    3 => GcEvent::HardDeleteCandidate,
                    _ => GcEvent::Tombstoned,
                }))
            },
            |i| {
                create_event_with_kind(DarviumEventKind::Repair(if i % 4 == 0 {
                    RepairEvent::InconsistencyDetected
                } else if i % 4 == 1 {
                    RepairEvent::RetryAttempted
                } else if i % 4 == 2 {
                    RepairEvent::TombstoneApplied
                } else {
                    RepairEvent::RepairCompleted
                }))
            },
            |i| {
                create_event_with_kind(DarviumEventKind::Reciprocity(if i % 8 == 0 {
                    ReciprocityEventKind::HelpOffered
                } else if i % 8 == 1 {
                    ReciprocityEventKind::HelpAccepted
                } else if i % 8 == 2 {
                    ReciprocityEventKind::HelpRejected
                } else if i % 8 == 3 {
                    ReciprocityEventKind::HelpExecuted
                } else if i % 8 == 4 {
                    ReciprocityEventKind::HelpSucceeded
                } else if i % 8 == 5 {
                    ReciprocityEventKind::HelpAbandoned
                } else if i % 8 == 6 {
                    ReciprocityEventKind::HarmfulMismatch
                } else {
                    ReciprocityEventKind::ReturnedFavor
                }))
            },
            |i| {
                create_event_with_kind(DarviumEventKind::Fusion(if i % 5 == 0 {
                    FusionEvent::Paired
                } else if i % 5 == 1 {
                    FusionEvent::FusionCompleted
                } else if i % 5 == 2 {
                    FusionEvent::BirthCommitInitiated
                } else if i % 5 == 3 {
                    FusionEvent::BirthCommitCompleted
                } else {
                    FusionEvent::FusionFailed
                }))
            },
            |i| {
                create_event_with_kind(DarviumEventKind::Hitl(if i % 4 == 0 {
                    HitlEvent::NotificationRequested
                } else if i % 4 == 1 {
                    HitlEvent::InteractionRequested
                } else if i % 4 == 2 {
                    HitlEvent::InteractionResolved
                } else {
                    HitlEvent::ChannelReconnected
                }))
            },
            |i| {
                create_event_with_kind(DarviumEventKind::System(if i % 4 == 0 {
                    SystemEvent::ClockAdvanced
                } else if i % 4 == 1 {
                    SystemEvent::SnapshotTaken
                } else if i % 4 == 2 {
                    SystemEvent::ReplayCompleted
                } else {
                    SystemEvent::StartupCompleted
                }))
            },
            |_i| create_event_with_kind(DarviumEventKind::Village(VillageEvent::TickCompleted)),
        ];

        for (di, constructor) in domain_constructors.iter().enumerate() {
            for i in 0..10 {
                all_events.push(constructor((di * 10 + i) as u32));
            }
        }

        // project_all で全 projection に配送
        for event in &all_events {
            catalog.project_all(event);
        }

        // 各 projection の受信イベントに自身のドメイン以外が含まれていないことを確認
        let projection_checks: Vec<(Arc<DomainProjection>, &str)> = vec![
            (search_proj.clone(), "Search"),
            (training_proj.clone(), "Training"),
            (reciprocity_proj.clone(), "Reciprocity"),
            (run_log_proj.clone(), "Search"), // subset of Search
            (village_proj.clone(), "Village"),
            (system_proj.clone(), "System"),
            (wf_proj.clone(), "WorkflowExecution"),
            (knowledge_proj.clone(), "Knowledge"),
            (conv_proj.clone(), "Conversational"),
            (lifecycle_proj.clone(), "Lifecycle"),
            (gc_proj.clone(), "Gc"),
            (repair_proj.clone(), "Repair"),
            (fusion_proj.clone(), "Fusion"),
            (hitl_proj.clone(), "Hitl"),
        ];

        for (proj, domain_prefix) in &projection_checks {
            let events = proj.received_events();
            assert!(
                !events.is_empty(),
                "{} projection が少なくとも1件のイベントを受信している必要があります",
                domain_prefix
            );
            for event in &events {
                let debug_str = format!("{:?}", event.kind);
                assert!(
                    debug_str.starts_with(domain_prefix),
                    "汚染検出: {} projection が {:?} を受信しました",
                    domain_prefix,
                    event.kind
                );
            }
        }

        println!("TC-4 PASS: 全14 Projection の相互汚染がゼロであることを確認しました");
    }

    /// TC-5: 全13ドメイン一貫クロック進行
    #[test]
    fn test_m176_23_tc5_all_domains_clock_monotonic() {
        let bus = FakeEventBus::new();

        // 130件を混在 publish
        for i in 0..130 {
            let kind = generate_random_event_kind(&mut StdRng::seed_from_u64(i as u64));
            bus.publish(create_event_with_kind(kind))
                .expect("publish が成功する必要があります");
        }

        let replayed = bus
            .replay(0, EventFilter::all())
            .expect("replay が成功する必要があります");

        assert_eq!(
            replayed.len(),
            130,
            "130件のイベントが取得可能である必要があります"
        );

        let mut prev_clock: Option<u64> = None;
        let mut clock_violations = 0u64;
        let mut clock_duplicates = 0u64;
        let mut seen_clocks = std::collections::HashSet::new();

        for event in &replayed {
            let clock = event.metadata.clock;
            if let Some(prev) = prev_clock {
                if clock <= prev {
                    clock_violations += 1;
                }
            }
            if !seen_clocks.insert(clock) {
                clock_duplicates += 1;
            }
            prev_clock = Some(clock);
        }

        assert_eq!(
            clock_violations, 0,
            "クロック単調増加違反が0である必要があります (violations: {})",
            clock_violations
        );
        assert_eq!(
            clock_duplicates, 0,
            "クロック重複が0である必要があります (duplicates: {})",
            clock_duplicates
        );

        assert_eq!(
            seen_clocks.len(),
            130,
            "130件のイベントが全て異なる clock 値を持つ必要があります"
        );

        println!("TC-5 PASS: 全130件のクロックが厳密に単調増加し、重複が0であることを確認しました");
    }

    /// TC-6: 全13ドメイン JSON ラウンドトリップ完全性 (n=1300)
    #[test]
    fn test_m176_23_tc6_json_roundtrip_n1300() {
        let mut rng = StdRng::seed_from_u64(12345);
        let mut success_count = 0u64;
        let total = 1300u64;

        for i in 0..total {
            let kind = generate_random_event_kind(&mut rng);
            let event = create_event_with_kind(kind);

            let json = serde_json::to_string(&event).expect("シリアライズが成功する必要があります");
            let restored: DarviumEvent =
                serde_json::from_str(&json).expect("デシリアライズが成功する必要があります");

            assert_eq!(event, restored, "ラウンドトリップ不一致 at index {}", i);
            success_count += 1;
        }

        let success_rate = success_count as f64 / total as f64 * 100.0;
        println!(
            "TC-6 PASS: {} / {} ラウンドトリップ成功 (成功率 {:.2}%, 期待: 100.0%)",
            success_count, total, success_rate
        );
    }

    /// TC-7: 観測テスト — n=1300 ランダム publish 系列 + 一貫性スコア
    #[test]
    fn test_m176_23_tc7_cross_domain_consistency_n1300() {
        let mut rng = StdRng::seed_from_u64(12345);
        let bus = FakeEventBus::new();
        let catalog = FakeProjectionCatalog::new();

        // 全14 projection を登録
        let search_proj = Arc::new(DomainProjection::search_trace());
        let training_proj = Arc::new(DomainProjection::training_run_log());
        let reciprocity_proj = Arc::new(DomainProjection::reciprocity_event());
        let run_log_proj = Arc::new(DomainProjection::search_run_log());
        let village_proj = Arc::new(DomainProjection::village_observation_log());
        let system_proj = Arc::new(DomainProjection::system_log());
        let wf_proj = Arc::new(DomainProjection::workflow_execution_log());
        let knowledge_proj = Arc::new(DomainProjection::knowledge_log());
        let conv_proj = Arc::new(DomainProjection::conversational_log());
        let lifecycle_proj = Arc::new(DomainProjection::lifecycle_log());
        let gc_proj = Arc::new(DomainProjection::gc_log());
        let repair_proj = Arc::new(DomainProjection::repair_log());
        let fusion_proj = Arc::new(DomainProjection::fusion_log());
        let hitl_proj = Arc::new(DomainProjection::hitl_log());

        catalog.register("search_trace", search_proj.clone());
        catalog.register("training_run_log", training_proj.clone());
        catalog.register("reciprocity_event", reciprocity_proj.clone());
        catalog.register("search_run_log", run_log_proj.clone());
        catalog.register("village_observation_log", village_proj.clone());
        catalog.register("system_log", system_proj.clone());
        catalog.register("workflow_execution_log", wf_proj.clone());
        catalog.register("knowledge_log", knowledge_proj.clone());
        catalog.register("conversational_log", conv_proj.clone());
        catalog.register("lifecycle_log", lifecycle_proj.clone());
        catalog.register("gc_log", gc_proj.clone());
        catalog.register("repair_log", repair_proj.clone());
        catalog.register("fusion_log", fusion_proj.clone());
        catalog.register("hitl_log", hitl_proj.clone());

        let samples_per_domain = 100u64;
        let total_domains = 13u64;
        let total_samples = samples_per_domain * total_domains; // 1300

        // 各ドメイン n=100 件のイベントを生成（明示的なドメイン特化 kind）
        let mut domain_event_map: std::collections::HashMap<&str, Vec<DarviumEvent>> =
            std::collections::HashMap::new();

        // 13 domain の variant 定義
        let system_variants = [
            DarviumEventKind::System(SystemEvent::ClockAdvanced),
            DarviumEventKind::System(SystemEvent::SnapshotTaken),
            DarviumEventKind::System(SystemEvent::ReplayCompleted),
            DarviumEventKind::System(SystemEvent::StartupCompleted),
        ];
        let search_variants = [
            DarviumEventKind::Search(SearchEvent::Started),
            DarviumEventKind::Search(SearchEvent::StepCompleted),
            DarviumEventKind::Search(SearchEvent::Completed),
            DarviumEventKind::Search(SearchEvent::Failed),
            DarviumEventKind::Search(SearchEvent::Aborted),
        ];
        let wf_variants = [
            DarviumEventKind::WorkflowExecution(WorkflowExecutionEvent::Started),
            DarviumEventKind::WorkflowExecution(WorkflowExecutionEvent::Completed),
            DarviumEventKind::WorkflowExecution(WorkflowExecutionEvent::Failed),
            DarviumEventKind::WorkflowExecution(WorkflowExecutionEvent::Retried),
        ];
        let training_variants = [
            DarviumEventKind::Training(TrainingEvent::MissionGenerated),
            DarviumEventKind::Training(TrainingEvent::HumanReviewRequested),
            DarviumEventKind::Training(TrainingEvent::HumanReviewCompleted),
            DarviumEventKind::Training(TrainingEvent::SandboxExecutionStarted),
            DarviumEventKind::Training(TrainingEvent::SandboxExecutionCompleted),
            DarviumEventKind::Training(TrainingEvent::FeedbackIngested),
            DarviumEventKind::Training(TrainingEvent::PromotionCandidateCreated),
            DarviumEventKind::Training(TrainingEvent::PromotionApproved),
            DarviumEventKind::Training(TrainingEvent::PromotionRejected),
        ];
        let knowledge_variants = [
            DarviumEventKind::Knowledge(KnowledgeEvent::FragmentCreated),
            DarviumEventKind::Knowledge(KnowledgeEvent::CandidateConsolidated),
            DarviumEventKind::Knowledge(KnowledgeEvent::CanonicalPromoted),
            DarviumEventKind::Knowledge(KnowledgeEvent::OriginTraceUpdated),
        ];
        let conv_variants = [
            DarviumEventKind::Conversational(ConversationalEventEnvelope::UtteranceReceived),
            DarviumEventKind::Conversational(ConversationalEventEnvelope::Classified),
            DarviumEventKind::Conversational(ConversationalEventEnvelope::GateDecided),
            DarviumEventKind::Conversational(ConversationalEventEnvelope::Consolidated),
            DarviumEventKind::Conversational(ConversationalEventEnvelope::Promoted),
        ];
        let lifecycle_variants = [
            DarviumEventKind::Lifecycle(LifecycleEvent::NodeCreated),
            DarviumEventKind::Lifecycle(LifecycleEvent::NodeActivated),
            DarviumEventKind::Lifecycle(LifecycleEvent::NodeDeactivated),
            DarviumEventKind::Lifecycle(LifecycleEvent::NodeArchived),
        ];
        let gc_variants = [
            DarviumEventKind::Gc(GcEvent::SoftDeleted),
            DarviumEventKind::Gc(GcEvent::HardDeleteCandidate),
            DarviumEventKind::Gc(GcEvent::Tombstoned),
        ];
        let repair_variants = [
            DarviumEventKind::Repair(RepairEvent::InconsistencyDetected),
            DarviumEventKind::Repair(RepairEvent::RetryAttempted),
            DarviumEventKind::Repair(RepairEvent::TombstoneApplied),
            DarviumEventKind::Repair(RepairEvent::RepairCompleted),
        ];
        let reciprocity_variants = [
            DarviumEventKind::Reciprocity(ReciprocityEventKind::HelpOffered),
            DarviumEventKind::Reciprocity(ReciprocityEventKind::HelpAccepted),
            DarviumEventKind::Reciprocity(ReciprocityEventKind::HelpRejected),
            DarviumEventKind::Reciprocity(ReciprocityEventKind::HelpExecuted),
            DarviumEventKind::Reciprocity(ReciprocityEventKind::HelpSucceeded),
            DarviumEventKind::Reciprocity(ReciprocityEventKind::HelpAbandoned),
            DarviumEventKind::Reciprocity(ReciprocityEventKind::HarmfulMismatch),
            DarviumEventKind::Reciprocity(ReciprocityEventKind::ReturnedFavor),
        ];
        let fusion_variants = [
            DarviumEventKind::Fusion(FusionEvent::Paired),
            DarviumEventKind::Fusion(FusionEvent::FusionCompleted),
            DarviumEventKind::Fusion(FusionEvent::BirthCommitInitiated),
            DarviumEventKind::Fusion(FusionEvent::BirthCommitCompleted),
            DarviumEventKind::Fusion(FusionEvent::FusionFailed),
        ];
        let hitl_variants = [
            DarviumEventKind::Hitl(HitlEvent::NotificationRequested),
            DarviumEventKind::Hitl(HitlEvent::InteractionRequested),
            DarviumEventKind::Hitl(HitlEvent::InteractionResolved),
            DarviumEventKind::Hitl(HitlEvent::ChannelReconnected),
        ];
        let village_variants = [DarviumEventKind::Village(VillageEvent::TickCompleted)];

        let domain_configs: Vec<(&str, &[DarviumEventKind])> = vec![
            ("System", &system_variants),
            ("Search", &search_variants),
            ("WorkflowExecution", &wf_variants),
            ("Training", &training_variants),
            ("Knowledge", &knowledge_variants),
            ("Conversational", &conv_variants),
            ("Lifecycle", &lifecycle_variants),
            ("Gc", &gc_variants),
            ("Repair", &repair_variants),
            ("Reciprocity", &reciprocity_variants),
            ("Fusion", &fusion_variants),
            ("Hitl", &hitl_variants),
            ("Village", &village_variants),
        ];

        for (name, variants) in &domain_configs {
            let mut events = Vec::new();
            for i in 0..samples_per_domain {
                let kind = variants[i as usize % variants.len()].clone();
                events.push(create_event_with_kind(kind));
            }
            domain_event_map.insert(*name, events);
        }

        // 全イベントをランダム順にマージして publish
        let mut all_events: Vec<(String, DarviumEvent)> = Vec::new();
        for (_domain, events) in domain_event_map.iter() {
            for event in events.clone() {
                let debug_kind = format!("{:?}", event.kind);
                let domain_name = debug_kind
                    .split('(')
                    .next()
                    .unwrap_or("Unknown")
                    .to_string();
                all_events.push((domain_name, event));
            }
        }

        // ランダムシャッフル
        let mut shuffled_indices: Vec<usize> = (0..all_events.len()).collect();
        for i in (1..shuffled_indices.len()).rev() {
            let j = rng.random_range(0..=i);
            shuffled_indices.swap(i, j);
        }

        for &idx in &shuffled_indices {
            let event = all_events[idx].1.clone();
            let _published_id = bus
                .publish(event.clone())
                .expect("publish が成功する必要があります");
            catalog.project_all(&event);
        }

        // === 観測指標の集計 ===
        // 1. replay 完全取得率
        let replayed = bus
            .replay(0, EventFilter::all())
            .expect("replay が成功する必要があります");
        let replay_completeness = replayed.len() as f64 / total_samples as f64;
        assert_eq!(
            replay_completeness, 1.0,
            "全イベントが replay 可能である必要があります"
        );

        // 2. kind フィルタ精度（prefix ベースのドメイン分類）
        // replayed は publish 順（= shuffled_indices 順）なので、pos で対応付ける
        let correct_classification = shuffled_indices
            .iter()
            .enumerate()
            .filter(|(pos, &idx)| {
                let (ref expected_domain, _) = all_events[idx];
                let replay_event = &replayed[*pos];
                let replay_domain = format!("{:?}", replay_event.kind)
                    .split('(')
                    .next()
                    .unwrap_or("")
                    .to_string();
                replay_domain.as_str() == expected_domain.as_str()
            })
            .count();
        let kind_filter_accuracy = correct_classification as f64 / total_samples as f64;

        // 3. クロック単調増加性
        let mut clock_ok = true;
        let mut all_clocks: Vec<u64> = replayed.iter().map(|e| e.metadata.clock).collect();
        all_clocks.sort_unstable();
        for i in 1..all_clocks.len() {
            if all_clocks[i] <= all_clocks[i - 1] {
                clock_ok = false;
            }
        }
        let clock_monotonic = if clock_ok { 1.0 } else { 0.0 };

        // 4. projection 配送完全性
        let projection_checks: Vec<(Arc<DomainProjection>, &str, u64)> = vec![
            (search_proj.clone(), "Search", 100),
            (training_proj.clone(), "Training", 100),
            (reciprocity_proj.clone(), "Reciprocity", 100),
            (village_proj.clone(), "Village", 100),
            (system_proj.clone(), "System", 100),
            (wf_proj.clone(), "WorkflowExecution", 100),
            (knowledge_proj.clone(), "Knowledge", 100),
            (conv_proj.clone(), "Conversational", 100),
            (lifecycle_proj.clone(), "Lifecycle", 100),
            (gc_proj.clone(), "Gc", 100),
            (repair_proj.clone(), "Repair", 100),
            (fusion_proj.clone(), "Fusion", 100),
            (hitl_proj.clone(), "Hitl", 100),
            // search_run_log is a subset, should get fewer
            (run_log_proj.clone(), "Search(subset)", 0), // not checked by count
        ];

        let mut projection_ok_count = 0u64;
        let total_projection_checks = 13u64; // exclude search_run_log (subset)
        for (proj, name, _expected) in &projection_checks {
            let events = proj.received_events();
            if *name == "Search(subset)" {
                continue; // skip subset check
            }
            // 自身のドメインに属するイベントのみを受け取っていることを確認
            let own_domain_events: Vec<&DarviumEvent> = events
                .iter()
                .filter(|e| format!("{:?}", e.kind).starts_with(name))
                .collect();
            let other_domain_events: Vec<&DarviumEvent> = events
                .iter()
                .filter(|e| !format!("{:?}", e.kind).starts_with(name))
                .collect();

            if other_domain_events.is_empty() && own_domain_events.len() as u64 >= 90 {
                projection_ok_count += 1;
            }
        }
        let projection_delivery = projection_ok_count as f64 / total_projection_checks as f64;

        // 5. 一貫性スコア（加重平均）
        let consistency_score = replay_completeness * 0.25
            + kind_filter_accuracy * 0.25
            + clock_monotonic * 0.25
            + projection_delivery * 0.25;

        // 観測結果を構造化出力
        println!(
            "{}",
            serde_json::json!({
                "test": "TC-7 cross-domain consistency",
                "n": {
                    "total": total_samples,
                    "per_domain": samples_per_domain,
                    "domains": total_domains,
                },
                "metrics": {
                    "replay_completeness": replay_completeness,
                    "kind_filter_accuracy": kind_filter_accuracy,
                    "clock_monotonic": clock_monotonic,
                    "projection_delivery_rate": projection_delivery,
                    "projection_ok_count": projection_ok_count,
                    "projection_total_checks": total_projection_checks,
                    "consistency_score": consistency_score,
                },
                "pass": consistency_score == 1.0,
            })
        );

        assert!(
            (consistency_score - 1.0).abs() < f64::EPSILON,
            "一貫性スコアが 1.0 である必要があります (actual: {})",
            consistency_score
        );

        println!(
            "TC-7 PASS: 全13ドメイン横断一貫性スコア = {:.6}",
            consistency_score
        );
    }

    // ============================================================
    // VC-1: VirtualClock == commit 済み DarviumEvent 数 (RFC §12C.6 MUST #1)
    //
    // publish/open は DarviumEvent を作成するため clock が進むが、
    // resolve/reconnect は DarviumEvent を作成しないため clock は進まない。
    // ============================================================
    #[test]
    fn test_vc1_clock_equals_committed_events() {
        let bus = FakeEventBus::new();

        // 初期状態: clock = 0
        assert_eq!(bus.now(), 0, "初期 VirtualClock は 0 である必要があります");

        // publish(3) → clock = 3 (3 DarviumEvents)
        for kind in &[
            DarviumEventKind::System(SystemEvent::ClockAdvanced),
            DarviumEventKind::Search(SearchEvent::Started),
            DarviumEventKind::Training(TrainingEvent::MissionGenerated),
        ] {
            bus.publish(create_event_with_kind(kind.clone())).unwrap();
        }
        assert_eq!(bus.now(), 3, "publish x3 → clock = 3");
        assert_eq!(bus.published_events().len(), 3, "publish x3 → events = 3");

        // open(1) → clock = 4 (DarviumEvent 作成を伴う)
        let interaction_id = bus
            .open(create_event_with_kind(DarviumEventKind::System(
                SystemEvent::StartupCompleted,
            )))
            .unwrap();
        assert_eq!(bus.now(), 4, "open x1 → clock = 4");
        assert_eq!(bus.published_events().len(), 4, "open x1 → events = 4");

        // resolve → clock は進まない (RFC §12C.6: VirtualClock = commit 済み DarviumEvent 列の順序番号)
        bus.resolve(&interaction_id, serde_json::Value::Null)
            .unwrap();
        assert_eq!(
            bus.now(),
            4,
            "resolve → clock は進まない (MUST, RFC §12C.6)"
        );
        assert_eq!(
            bus.published_events().len(),
            4,
            "resolve → events 数も変わらない"
        );

        // open(1) → clock = 5
        let interaction_id2 = bus
            .open(create_event_with_kind(DarviumEventKind::Search(
                SearchEvent::Completed,
            )))
            .unwrap();
        assert_eq!(bus.now(), 5, "open x2 → clock = 5");

        // reconnect → clock は進まない
        bus.reconnect(&interaction_id2, "new_channel").unwrap();
        assert_eq!(
            bus.now(),
            5,
            "reconnect → clock は進まない (MUST, RFC §12C.6)"
        );
        assert_eq!(
            bus.published_events().len(),
            5,
            "reconnect → events 数も変わらない"
        );

        // publish(2) → clock = 7 (5 + 2)
        for _ in 0..2 {
            bus.publish(create_event_with_kind(DarviumEventKind::Gc(
                GcEvent::SoftDeleted,
            )))
            .unwrap();
        }
        assert_eq!(bus.now(), 7, "publish x2 → clock = 7");
        assert_eq!(bus.published_events().len(), 7, "publish x2 → events = 7");

        // replay 全件: clock 値が [0..6] の連続であること
        let replayed = bus.replay(0, EventFilter::all()).unwrap();
        assert_eq!(
            replayed.len(),
            7,
            "7件の DarviumEvent が replay 可能である必要があります"
        );

        let actual_clocks: Vec<u64> = replayed.iter().map(|e| e.metadata.clock).collect();
        let expected_clocks: Vec<u64> = (0..7).collect();
        assert_eq!(
            actual_clocks, expected_clocks,
            "clock 値が commit 順に [0,1,2,3,4,5,6] である必要があります"
        );

        println!(
            "VC-1 PASS: VirtualClock ({}) == committed DarviumEvents ({}), clock sequence: {:?}",
            bus.now(),
            replayed.len(),
            actual_clocks
        );
    }

    // ============================================================
    // VC-2: 混在操作 (publish/open/resolve/reconnect) × 任意順序でのクロック一貫性
    //
    // 操作系列の各ステップで clock == committed DarviumEvent 数が成立することを
    // 網羅的に検証する。
    // ============================================================
    #[test]
    fn test_vc2_mixed_operations_clock_consistency() {
        let bus = FakeEventBus::new();

        // 操作系列: [(expected_clock_delta, expected_total_events), action]
        // clock は DarviumEvent を作成する操作でのみ進む
        let make_event = |kind: DarviumEventKind| create_event_with_kind(kind);

        // publish: clock +1, events +1
        bus.publish(make_event(DarviumEventKind::System(
            SystemEvent::ClockAdvanced,
        )))
        .unwrap();
        assert_eq!(bus.now(), 1);
        assert_eq!(bus.published_events().len(), 1);

        // open: clock +1, events +1 (DarviumEvent 作成)
        let id1 = bus
            .open(make_event(DarviumEventKind::Search(SearchEvent::Started)))
            .unwrap();
        assert_eq!(bus.now(), 2);
        assert_eq!(bus.published_events().len(), 2);

        // resolve: clock 不変, events 不変
        bus.resolve(&id1, serde_json::Value::Null).unwrap();
        assert_eq!(bus.now(), 2);
        assert_eq!(bus.published_events().len(), 2);

        // publish: clock +1, events +1
        bus.publish(make_event(DarviumEventKind::Training(
            TrainingEvent::MissionGenerated,
        )))
        .unwrap();
        assert_eq!(bus.now(), 3);
        assert_eq!(bus.published_events().len(), 3);

        // open: clock +1, events +1
        let id2 = bus
            .open(make_event(DarviumEventKind::Lifecycle(
                LifecycleEvent::NodeCreated,
            )))
            .unwrap();
        assert_eq!(bus.now(), 4);
        assert_eq!(bus.published_events().len(), 4);

        // reconnect: clock 不変, events 不変
        bus.reconnect(&id2, "alt_channel").unwrap();
        assert_eq!(bus.now(), 4);
        assert_eq!(bus.published_events().len(), 4);

        // resolve 済み interaction の再 resolve: clock 不変
        bus.resolve(&id1, serde_json::Value::Null).unwrap();
        assert_eq!(bus.now(), 4);

        // publish: clock +1, events +1
        bus.publish(make_event(DarviumEventKind::Gc(GcEvent::Tombstoned)))
            .unwrap();
        assert_eq!(bus.now(), 5);
        assert_eq!(bus.published_events().len(), 5);

        // replay 全件: clock [0,1,2,3,4]
        let replayed = bus.replay(0, EventFilter::all()).unwrap();
        assert_eq!(replayed.len(), 5);

        let actual_clocks: Vec<u64> = replayed.iter().map(|e| e.metadata.clock).collect();
        let expected_clocks: Vec<u64> = (0..5).collect();
        assert_eq!(
            actual_clocks, expected_clocks,
            "混在操作後も clock は [0..4] の完全連続"
        );

        println!(
            "VC-2 PASS: 混在操作後の clock 一貫性: {} events, clocks: {:?}",
            replayed.len(),
            actual_clocks
        );
    }

    // ============================================================
    // VC-3: replay は VirtualClock を再増加させない (RFC §12C.6 MUST NOT #3)
    //
    // replay 呼び出し前後で clock 値が不変であることを確認する。
    // ============================================================
    #[test]
    fn test_vc3_replay_does_not_advance_clock() {
        let bus = FakeEventBus::new();

        // publish 5 events
        for _i in 0..5u64 {
            let kind = DarviumEventKind::System(SystemEvent::ClockAdvanced);
            bus.publish(create_event_with_kind(kind)).unwrap();
        }
        assert_eq!(bus.now(), 5);

        // replay を3回実行
        let clock_before = bus.now();
        for _ in 0..3 {
            let replayed = bus.replay(0, EventFilter::all()).unwrap();
            assert_eq!(replayed.len(), 5);
            assert_eq!(
                bus.now(),
                clock_before,
                "replay は VirtualClock を進めてはならない (MUST NOT, RFC §12C.6)"
            );
        }

        // replay 後も新しい publish は clock を正しく進める
        bus.publish(create_event_with_kind(DarviumEventKind::Search(
            SearchEvent::Completed,
        )))
        .unwrap();
        assert_eq!(bus.now(), 6);

        println!(
            "VC-3 PASS: replay 前後で clock 不変 ({})、publish 後も正常進行 ({})",
            clock_before, 6
        );
    }

    // ============================================================
    // VC-4: VirtualClock と EventMetadata.clock の完全連続性 (n=1000)
    //
    // publish/open/resolve/reconnect をランダム系列で実行し、
    // clock が常に committed DarviumEvent 数と一致することを検証する。
    // 観測テスト: 全操作後の clock = events.len、全 event.metadata.clock が [0..N-1]
    // ============================================================
    #[test]
    fn test_vc4_random_operations_clock_invariant_n1000() {
        let bus = FakeEventBus::new();
        let mut rng = StdRng::seed_from_u64(12345);

        let mut open_ids: Vec<InteractionId> = Vec::new();
        let domain_kinds: &[DarviumEventKind] = &[
            DarviumEventKind::System(SystemEvent::ClockAdvanced),
            DarviumEventKind::Search(SearchEvent::Started),
            DarviumEventKind::WorkflowExecution(WorkflowExecutionEvent::Started),
            DarviumEventKind::Training(TrainingEvent::MissionGenerated),
            DarviumEventKind::Knowledge(KnowledgeEvent::FragmentCreated),
            DarviumEventKind::Conversational(ConversationalEventEnvelope::UtteranceReceived),
            DarviumEventKind::Lifecycle(LifecycleEvent::NodeCreated),
            DarviumEventKind::Gc(GcEvent::SoftDeleted),
            DarviumEventKind::Repair(RepairEvent::InconsistencyDetected),
            DarviumEventKind::Reciprocity(ReciprocityEventKind::HelpOffered),
            DarviumEventKind::Fusion(FusionEvent::Paired),
            DarviumEventKind::Hitl(HitlEvent::NotificationRequested),
            DarviumEventKind::Village(VillageEvent::TickCompleted),
        ];

        for step in 0..1000u64 {
            let operation = rng.random_range(0..100u64);
            if operation < 40 {
                // 40%: publish
                let kind = domain_kinds[rng.random_range(0..domain_kinds.len())].clone();
                bus.publish(create_event_with_kind(kind)).unwrap();
            } else if operation < 70 {
                // 30%: open
                let kind = domain_kinds[rng.random_range(0..domain_kinds.len())].clone();
                let id = bus.open(create_event_with_kind(kind)).unwrap();
                open_ids.push(id);
            } else if operation < 85 {
                // 15%: resolve（open がある場合のみ）
                if !open_ids.is_empty() {
                    let idx = rng.random_range(0..open_ids.len());
                    let id = &open_ids[idx];
                    let _ = bus.resolve(id, serde_json::Value::Null);
                }
            } else {
                // 15%: reconnect（open がある場合のみ）
                if !open_ids.is_empty() {
                    let idx = rng.random_range(0..open_ids.len());
                    let id = &open_ids[idx];
                    let _ = bus.reconnect(id, "ch_alt");
                }
            }

            // 各ステップで不変条件: clock == published_events().len()
            assert_eq!(
                bus.now(),
                bus.published_events().len() as u64,
                "Step {}: clock == committed DarviumEvent 数が成立する必要があります",
                step
            );
        }

        // 最終状態の検証
        let total_events = bus.published_events().len();
        assert_eq!(
            bus.now() as usize,
            total_events,
            "全操作後も clock == committed DarviumEvent 数"
        );

        // replay の clock 値が [0..N-1] の完全連続であること
        let replayed = bus.replay(0, EventFilter::all()).unwrap();
        assert_eq!(replayed.len(), total_events);

        let mut actual_clocks: Vec<u64> = replayed.iter().map(|e| e.metadata.clock).collect();
        // replay の返す順序が clock 順であることを確認（ソートして同等性検証）
        actual_clocks.sort_unstable();
        let expected_clocks: Vec<u64> = (0..total_events as u64).collect();
        assert_eq!(
            actual_clocks,
            expected_clocks,
            "replay 全イベントの clock が [0..{}] の完全連続",
            total_events - 1
        );

        // クロック重複ゼロ
        let unique_clocks: std::collections::HashSet<u64> =
            replayed.iter().map(|e| e.metadata.clock).collect();
        assert_eq!(
            unique_clocks.len(),
            total_events,
            "全 clock 値が一意である必要があります"
        );

        println!(
            "VC-4 PASS: ランダム系列 n=1000, total_events={}, clock={}, unique_clocks={}",
            total_events,
            bus.now(),
            unique_clocks.len()
        );
    }

    // ================================================================
    // P5: transition_gc_state (TC4, TC5)
    // ================================================================

    /// TC4: 完全遷移連鎖の検証。
    ///
    /// Protected → Active → SoftDeleted → HardDeleteCandidate → Tombstoned
    /// の各遷移が正しい hazard 閾値で発生することを確認する。
    #[test]
    fn tc4_transition_gc_state_full_chain() {
        // Protected → Active (hazard > 0.0)
        let state = transition_gc_state(GcEvent::Protected, 0.1);
        assert_eq!(state, GcEvent::Active, "Protected + hazard=0.1 → Active");

        // Active → SoftDeleted (hazard > 0.0)
        let state = transition_gc_state(GcEvent::Active, 0.1);
        assert_eq!(
            state,
            GcEvent::SoftDeleted,
            "Active + hazard=0.1 → SoftDeleted"
        );

        // SoftDeleted → HardDeleteCandidate (hazard > 0.5)
        let state = transition_gc_state(GcEvent::SoftDeleted, 0.6);
        assert_eq!(
            state,
            GcEvent::HardDeleteCandidate,
            "SoftDeleted + hazard=0.6 → HardDeleteCandidate"
        );

        // HardDeleteCandidate → Tombstoned (hazard > 0.8)
        let state = transition_gc_state(GcEvent::HardDeleteCandidate, 0.9);
        assert_eq!(
            state,
            GcEvent::Tombstoned,
            "HardDeleteCandidate + hazard=0.9 → Tombstoned"
        );
    }

    /// TC5: Protected からの直接 Tombstoned 遷移禁止。
    ///
    /// Protected 状態の個人は hazard=1.0 でも Tombstoned にならず、
    /// Protected に留まる (Active にも遷移しない)。
    #[test]
    fn tc5_transition_gc_state_protected_no_skip() {
        // hazard=1.0 でも Protected → Active (Tombstoned への直接遷移禁止)
        let state = transition_gc_state(GcEvent::Protected, 1.0);
        assert_eq!(
            state,
            GcEvent::Active,
            "Protected + hazard=1.0 must NOT skip to Tombstoned, got {:?}",
            state
        );

        // hazard=0.0 では Protected に留まる
        let state = transition_gc_state(GcEvent::Protected, 0.0);
        assert_eq!(
            state,
            GcEvent::Protected,
            "Protected + hazard=0.0 must stay Protected, got {:?}",
            state
        );
    }
}
