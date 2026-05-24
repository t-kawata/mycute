// Darvium Event Architecture — 型定義 (RFC §12C)
//
// 本ファイルは v2.3-g Darvium Event Architecture の全基盤型を定義する。
// 絶対正本: Darvium-RFC-0001-Unified-v2.3-final.md §12C

use serde::{Deserialize, Serialize};
use std::time::SystemTime;

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

/// GC イベントの種別 (RFC §12C.2)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GcEvent {
    /// ソフト削除された。
    SoftDeleted,
    /// ハード削除候補としてマークされた。
    HardDeleteCandidate,
    /// 墓石（tombstone）が適用された。
    Tombstoned,
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
/// RFC §15.10.6 ReciprocityEventKind の variant を DarviumEventKind 用に流用。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ReciprocityEvent {
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
    Reciprocity(ReciprocityEvent),
    /// 融合イベント。
    Fusion(FusionEvent),
    /// HITL イベント。
    Hitl(HitlEvent),
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
// テスト
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::Rng;
    use rand::SeedableRng;
    use std::time::SystemTime;

    /// ラウンドトリップサンプルサイズ。
    const ROUNDTRIP_SAMPLE_SIZE: usize = 1000;

    // ============================================================
    // TC-1: 全13 variant の DarviumEventKind トレイト実装確認
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
            DarviumEventKind::Reciprocity(ReciprocityEvent::HelpOffered),
            DarviumEventKind::Fusion(FusionEvent::Paired),
            DarviumEventKind::Hitl(HitlEvent::NotificationRequested),
            DarviumEventKind::Extension("test".to_string()),
        ];

        assert_eq!(variants.len(), 13, "DarviumEventKind は13 variant である必要があります");

        for variant in &variants {
            // Debug: パニックしないこと
            let debug_str = format!("{:?}", variant);
            assert!(!debug_str.is_empty(), "Debug 出力が空であってはなりません");

            // Clone: 複製が等価であること
            let cloned = variant.clone();
            assert_eq!(*variant, cloned, "Clone が original と等価である必要があります");

            // Serialize: シリアライズが成功すること
            let json = serde_json::to_string(variant)
                .expect("serde_json::to_string が成功する必要があります");
            assert!(!json.is_empty(), "JSON 出力が空であってはなりません");

            // Deserialize: デシリアライズが成功し、元と一致すること
            let restored: DarviumEventKind = serde_json::from_str(&json)
                .expect("serde_json::from_str が成功する必要があります");
            assert_eq!(*variant, restored, "ラウンドトリップが一致する必要があります");
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
        assert_eq!(event.kind, DarviumEventKind::System(SystemEvent::StartupCompleted));
        assert_eq!(event.interaction_mode, InteractionMode::OneWay);
        assert_eq!(event.payload, serde_json::json!({"key": "value"}));
        assert_eq!(event.causality.trace_ref, Some("trace-001".to_string()));
        assert_eq!(event.metadata.clock, 42);
        assert!(event.transport_meta.is_some());
        assert_eq!(event.transport_meta.as_ref().unwrap().ttl_seconds, Some(3600));
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
                    parent_event_id: rng.random_bool(0.3).then(|| uuid::Uuid::new_v4().to_string()),
                    root_event_id: rng.random_bool(0.1).then(|| uuid::Uuid::new_v4().to_string()),
                    trace_ref: rng.random_bool(0.5).then(|| rng.random::<u64>().to_string()),
                    mission_id: rng.random_bool(0.4).then(|| rng.random::<u64>().to_string()),
                    workflow_id: rng.random_bool(0.4).then(|| rng.random::<u64>().to_string()),
                    run_id: rng.random_bool(0.3).then(|| rng.random::<u64>().to_string()),
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

            let json = serde_json::to_string(&event)
                .expect("シリアライズが成功する必要があります");
            let restored: DarviumEvent = serde_json::from_str(&json)
                .expect("デシリアライズが成功する必要があります");

            assert_eq!(event, restored, "ラウンドトリップ不一致 at index {}", i);
            success_count += 1;
        }

        let success_rate = success_count as f64 / ROUNDTRIP_SAMPLE_SIZE as f64 * 100.0;
        println!("TC-4 PASS: {} / {} ラウンドトリップ成功 (成功率 {:.2}%)",
            success_count, ROUNDTRIP_SAMPLE_SIZE, success_rate);
    }

    // ============================================================
    // TC-5: 補助型のシリアライズ確認
    // ============================================================
    #[test]
    fn test_auxiliary_types_serialization() {
        // DeliveryMode
        let modes = [DeliveryMode::AtMostOnce, DeliveryMode::AtLeastOnce, DeliveryMode::ExactlyOnce];
        for mode in &modes {
            let json = serde_json::to_string(mode).expect("DeliveryMode シリアライズ");
            let restored: DeliveryMode = serde_json::from_str(&json).expect("DeliveryMode デシリアライズ");
            assert_eq!(*mode, restored);
        }

        // TransportMeta
        let meta = TransportMeta {
            delivery_mode: DeliveryMode::ExactlyOnce,
            reply_to: Some("chan-1".to_string()),
            ttl_seconds: None,
        };
        let json = serde_json::to_string(&meta).expect("TransportMeta シリアライズ");
        let restored: TransportMeta = serde_json::from_str(&json).expect("TransportMeta デシリアライズ");
        assert_eq!(meta, restored);

        // EventVisibility
        let visibilities = [EventVisibility::Public, EventVisibility::Protected, EventVisibility::Internal];
        for vis in &visibilities {
            let json = serde_json::to_string(vis).expect("EventVisibility シリアライズ");
            let restored: EventVisibility = serde_json::from_str(&json).expect("EventVisibility デシリアライズ");
            assert_eq!(*vis, restored);
        }

        // EventRetention
        let retention = EventRetention { persist: true, ttl_days: Some(30) };
        let json = serde_json::to_string(&retention).expect("EventRetention シリアライズ");
        let restored: EventRetention = serde_json::from_str(&json).expect("EventRetention デシリアライズ");
        assert_eq!(retention, restored);

        // EventPrivacy
        let privacy = EventPrivacy {
            contains_pii: true,
            sandbox_only: false,
            pii_handling: PiiHandlingPolicy::RedactBeforePersist,
        };
        let json = serde_json::to_string(&privacy).expect("EventPrivacy シリアライズ");
        let restored: EventPrivacy = serde_json::from_str(&json).expect("EventPrivacy デシリアライズ");
        assert_eq!(privacy, restored);

        // EventSource
        let sources = [
            EventSource::System,
            EventSource::HumanChannel,
            EventSource::Orchestrator,
            EventSource::External { channel_id: "ext-1".to_string() },
            EventSource::Test,
        ];
        for src in &sources {
            let json = serde_json::to_string(src).expect("EventSource シリアライズ");
            let restored: EventSource = serde_json::from_str(&json).expect("EventSource デシリアライズ");
            assert_eq!(*src, restored);
        }

        // EventMetadata
        let metadata = EventMetadata {
            clock: 100,
            timestamp: SystemTime::UNIX_EPOCH,
            source: EventSource::Test,
        };
        let json = serde_json::to_string(&metadata).expect("EventMetadata シリアライズ");
        let restored: EventMetadata = serde_json::from_str(&json).expect("EventMetadata デシリアライズ");
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
        let restored: EventCausality = serde_json::from_str(&json).expect("EventCausality デシリアライズ");
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
        assert_eq!(event_id, uuid_str, "EventId は UUIDv4 文字列と互換である必要があります");

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
            retention: EventRetention { persist: false, ttl_days: None },
            privacy: EventPrivacy {
                contains_pii: false,
                sandbox_only: false,
                pii_handling: PiiHandlingPolicy::Reject,
            },
        };

        // UUID パース可能な形式であること
        let parsed = uuid::Uuid::parse_str(&event.event_id);
        assert!(parsed.is_ok(), "DarviumEvent.event_id が UUID としてパース可能である必要があります");

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
            EventSource::External { channel_id: "ch".to_string() },
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
        println!("  {{\"name\":\"interaction_mode\",\"type\":\"InteractionMode\",\"optional\":false}},");
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
            ("Reciprocity", "ReciprocityEvent"),
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
            7 => DarviumEventKind::Gc(match rng.random_range(0..3) {
                0 => GcEvent::SoftDeleted,
                1 => GcEvent::HardDeleteCandidate,
                _ => GcEvent::Tombstoned,
            }),
            8 => DarviumEventKind::Repair(match rng.random_range(0..4) {
                0 => RepairEvent::InconsistencyDetected,
                1 => RepairEvent::RetryAttempted,
                2 => RepairEvent::TombstoneApplied,
                _ => RepairEvent::RepairCompleted,
            }),
            9 => DarviumEventKind::Reciprocity(match rng.random_range(0..8) {
                0 => ReciprocityEvent::HelpOffered,
                1 => ReciprocityEvent::HelpAccepted,
                2 => ReciprocityEvent::HelpRejected,
                3 => ReciprocityEvent::HelpExecuted,
                4 => ReciprocityEvent::HelpSucceeded,
                5 => ReciprocityEvent::HelpAbandoned,
                6 => ReciprocityEvent::HarmfulMismatch,
                _ => ReciprocityEvent::ReturnedFavor,
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
            3 => EventSource::External { channel_id: rng.random::<u64>().to_string() },
            _ => EventSource::Test,
        }
    }
}
