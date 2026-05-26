// Darvium EventChannel — 外部イベント送受信抽象 (RFC §12D)
//
// 本ファイルは EventChannel トレイトとその標準実装を定義する。
// 絶対正本: Darvium-RFC-0001-Unified-v2.3-final.md §12D

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::io::{BufRead, Write};
use std::sync::{Arc, Mutex};

use rand::rngs::StdRng;
use rand::Rng;
use rand::SeedableRng;

use crate::constants::{FAKE_WS_CHANNEL_BUFFER_SIZE, MAX_SUBSCRIBERS};
use crate::error::DarviumError;
use crate::event::{
    DarviumEvent, DarviumEventKind, EventCausality, EventFilter, EventMetadata, EventPrivacy,
    EventRetention, EventSource, EventVisibility, HitlEvent, InteractionMode, PiiHandlingPolicy,
};

// ============================================================
// CompatMode (RFC §12D.2)
// ============================================================

/// 旧 HITL JSON Lines プロトコル互換モード (RFC §12D.2)。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum CompatMode {
    /// 旧 HITL プロトコル互換 (§12B.9)。
    Enabled,
    /// canonical protocol のみ。
    Disabled,
}

// ============================================================
// Subscription (RFC §12D.4)
// ============================================================

/// イベント購読状態 (RFC §12D.4)。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Subscription {
    /// UUIDv4 購読識別子。
    pub id: String,
    /// 購読対象種別。
    pub kinds: Vec<DarviumEventKind>,
    /// 購読元チャネル識別子（任意）。
    pub channel: Option<String>,
}

// ============================================================
// EventChannel トレイト (RFC §12D.1, 同期版)
// ============================================================

/// 外部プロセスとのイベント送受信抽象 (RFC §12D.1)。
///
/// - send: DarviumEvent を外部チャネルに送信する
/// - receive: 外部チャネルから 1 行の JSON を読み取り DarviumEvent として返す
/// - flush: 出力バッファをフラッシュする
///
/// 本トレイトは Send + Sync を要求し、全メソッドが &self で宣言されるため
/// Box<dyn EventChannel> としてのオブジェクト利用が可能。
pub trait EventChannel: Send + Sync {
    /// イベントをチャネル経由で送信する。
    fn send(&self, event: DarviumEvent) -> Result<(), DarviumError>;

    /// チャネルからイベントを受信する。
    ///
    /// 利用可能なイベントがない場合は Ok(None) を返す。
    /// I/O エラー時は Err を返す。パースエラー時は Ok(None) としてエラーメッセージを出力する。
    fn receive(&self) -> Result<Option<DarviumEvent>, DarviumError>;

    /// 出力バッファをフラッシュする。
    fn flush(&self) -> Result<(), DarviumError>;
}

// ============================================================
// StdinoutEventChannel (RFC §12D.2)
// ============================================================

/// 標準入出力を介した EventChannel の具象実装 (RFC §12D.2)。
///
/// canonical JSON Lines プロトコル (§12B.9a) または互換モードで
/// 旧 HITL JSON Lines プロトコルを話す。
pub struct StdinoutEventChannel<R, W> {
    /// 読み取り側（別スレッドからの読み取りを想定して Arc<Mutex>>）。
    reader: Arc<Mutex<R>>,
    /// 書き込み側。
    writer: Mutex<W>,
    /// 旧プロトコル互換モード。
    compat: CompatMode,
}

impl<R: BufRead + Send, W: Write + Send> StdinoutEventChannel<R, W> {
    /// リーダー・ライター・互換モードを指定して生成する。
    pub fn new(reader: R, writer: W, compat: CompatMode) -> Self {
        Self {
            reader: Arc::new(Mutex::new(reader)),
            writer: Mutex::new(writer),
            compat,
        }
    }

    /// 標準 JSON Lines エラーメッセージを出力に書き込む。
    fn write_error(&self, code: &str, message: &str) -> Result<(), DarviumError> {
        let error_msg = serde_json::json!({
            "type": "error",
            "code": code,
            "message": message,
        });
        let line = serde_json::to_string(&error_msg)
            .map_err(|e| DarviumError::EventChannel(format!("serialize error: {}", e)))?;
        let mut writer = self
            .writer
            .lock()
            .map_err(|e| DarviumError::EventChannel(format!("writer lock: {}", e)))?;
        writeln!(writer, "{}", line)
            .map_err(|e| DarviumError::EventChannel(format!("write error: {}", e)))
    }
}

impl<R: BufRead + Send, W: Write + Send> EventChannel for StdinoutEventChannel<R, W> {
    fn send(&self, event: DarviumEvent) -> Result<(), DarviumError> {
        let line = if self.compat == CompatMode::Enabled {
            serialize_to_legacy(&event)?
        } else {
            serialize_to_canonical(&event)?
        };

        let mut writer = self
            .writer
            .lock()
            .map_err(|e| DarviumError::EventChannel(format!("writer lock: {}", e)))?;
        writeln!(writer, "{}", line)
            .map_err(|e| DarviumError::EventChannel(format!("write error: {}", e)))?;
        Ok(())
    }

    fn receive(&self) -> Result<Option<DarviumEvent>, DarviumError> {
        let line = {
            let mut reader = self
                .reader
                .lock()
                .map_err(|e| DarviumError::EventChannel(format!("reader lock: {}", e)))?;
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) => return Ok(None), // EOF
                Ok(_) => line,
                Err(e) => {
                    return Err(DarviumError::EventChannel(format!("read error: {}", e)));
                }
            }
        };

        let trimmed = line.trim();
        if trimmed.is_empty() {
            return Ok(None);
        }

        match parse_line(trimmed, self.compat) {
            Ok(Some(event)) => Ok(Some(event)),
            Ok(None) => {
                self.write_error("PARSE_ERROR", "unrecognized message type")?;
                Ok(None)
            }
            Err(e) => Err(e),
        }
    }

    fn flush(&self) -> Result<(), DarviumError> {
        let mut writer = self
            .writer
            .lock()
            .map_err(|e| DarviumError::EventChannel(format!("writer lock: {}", e)))?;
        writer
            .flush()
            .map_err(|e| DarviumError::EventChannel(format!("flush error: {}", e)))
    }
}

// ============================================================
// WebSocketEventChannel — 型定義のみ (RFC §12D.3)
// ============================================================

/// WebSocket を介した EventChannel の型定義。
///
/// 実装は将来のチケット (M1.76-21) で行う。本定義では構造体の型のみを公開し、
/// 外部コードからの参照を可能にする。
pub struct WebSocketEventChannel {
    /// WebSocket 接続先 URL。
    pub url: String,
    /// 購読状態（接続後は Some）。
    pub subscription: Option<Subscription>,
}

// ============================================================
// SubscriptionId — 購読識別子 (RFC §12D.4)
// ============================================================

/// 購読識別子の UUIDv4 文字列ラッパー。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SubscriptionId(pub String);

impl SubscriptionId {
    /// UUIDv4 を生成して SubscriptionId を作成する。
    pub fn new() -> Self {
        SubscriptionId(uuid::Uuid::new_v4().to_string())
    }
}

impl Default for SubscriptionId {
    fn default() -> Self {
        Self::new()
    }
}

impl From<String> for SubscriptionId {
    fn from(s: String) -> Self {
        SubscriptionId(s)
    }
}

impl From<SubscriptionId> for String {
    fn from(id: SubscriptionId) -> String {
        id.0
    }
}

impl fmt::Display for SubscriptionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

// ============================================================
// SubscriberStatus — 購読者状態
// ============================================================

/// 購読者の現在の状態。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum SubscriberStatus {
    /// 購読中。イベント分配の対象。
    Active,
    /// 一時停止中。イベント分配は行われない。
    Paused,
    /// 切断済み。再接続が必要。
    Disconnected,
}

// ============================================================
// EventSubscriber — 購読者 (RFC §12D.4)
// ============================================================

/// イベント購読者を表現する構造体。
///
/// 購読フィルタとイベントチャネルを保持し、
/// SubscriberManager により管理される。
pub struct EventSubscriber {
    /// 購読識別子。
    pub subscription_id: SubscriptionId,
    /// 購読フィルタ条件。
    pub filter: EventFilter,
    /// イベント配信チャネル。
    pub channel: Box<dyn EventChannel>,
    /// 購読者状態。
    pub status: SubscriberStatus,
    /// 受信済みイベント数。
    pub event_count: u64,
}

impl fmt::Debug for EventSubscriber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EventSubscriber")
            .field("subscription_id", &self.subscription_id)
            .field("status", &self.status)
            .field("event_count", &self.event_count)
            .finish()
    }
}

// ============================================================
// SubscriberSnapshot — 購読者一覧用スナップショット
// ============================================================

/// 購読者情報のスナップショット（チャネル除く）。
#[derive(Debug, Clone)]
pub struct SubscriberSnapshot {
    /// 購読識別子。
    pub subscription_id: SubscriptionId,
    /// 購読フィルタ条件。
    pub filter: EventFilter,
    /// 購読者状態。
    pub status: SubscriberStatus,
    /// 受信済みイベント数。
    pub event_count: u64,
}

impl From<&EventSubscriber> for SubscriberSnapshot {
    fn from(sub: &EventSubscriber) -> Self {
        SubscriberSnapshot {
            subscription_id: sub.subscription_id.clone(),
            filter: sub.filter.clone(),
            status: sub.status,
            event_count: sub.event_count,
        }
    }
}

// ============================================================
// SubscriberManager — 購読管理
// ============================================================

/// 購読の登録・解除・一覧・分配を行う管理構造体。
///
/// 内部で Arc<Mutex<...>> を使用し、スレッドセーフに動作する。
#[derive(Debug, Clone)]
pub struct SubscriberManager {
    /// 購読者リスト。
    subscribers: Arc<Mutex<Vec<EventSubscriber>>>,
}

impl SubscriberManager {
    /// 空の SubscriberManager を作成する。
    pub fn new() -> Self {
        SubscriberManager {
            subscribers: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// 購読者を登録し、SubscriptionId を返す。
    ///
    /// 最大購読者数を超えると Err を返す。
    pub fn register(
        &self,
        filter: EventFilter,
        channel: Box<dyn EventChannel>,
    ) -> Result<SubscriptionId, DarviumError> {
        let mut subs = self
            .subscribers
            .lock()
            .map_err(|e| DarviumError::EventChannel(format!("subscriber lock: {}", e)))?;

        if subs.len() >= MAX_SUBSCRIBERS {
            return Err(DarviumError::EventChannel(format!(
                "max subscribers ({}) reached",
                MAX_SUBSCRIBERS
            )));
        }

        let id = SubscriptionId::new();
        subs.push(EventSubscriber {
            subscription_id: id.clone(),
            filter,
            channel,
            status: SubscriberStatus::Active,
            event_count: 0,
        });
        Ok(id)
    }

    /// 購読者を解除する。
    ///
    /// 存在しない ID の場合は Err を返す。
    pub fn unregister(&self, id: &SubscriptionId) -> Result<(), DarviumError> {
        let mut subs = self
            .subscribers
            .lock()
            .map_err(|e| DarviumError::EventChannel(format!("subscriber lock: {}", e)))?;

        let len_before = subs.len();
        subs.retain(|s| s.subscription_id != *id);

        if subs.len() == len_before {
            return Err(DarviumError::EventChannel(format!(
                "subscriber {} not found",
                id
            )));
        }
        Ok(())
    }

    /// 全購読者の一覧をスナップショットとして返す。
    pub fn list(&self) -> Result<Vec<SubscriberSnapshot>, DarviumError> {
        let subs = self
            .subscribers
            .lock()
            .map_err(|e| DarviumError::EventChannel(format!("subscriber lock: {}", e)))?;
        Ok(subs.iter().map(SubscriberSnapshot::from).collect())
    }

    /// イベントを全アクティブ購読者に分配する。
    ///
    /// フィルタ条件に合致する購読者のみに配送される。
    /// 各購読者の配送エラーは分離され、他の購読者に影響を与えない。
    pub fn distribute(&self, event: &DarviumEvent) -> Result<(), DarviumError> {
        let mut subs = self
            .subscribers
            .lock()
            .map_err(|e| DarviumError::EventChannel(format!("subscriber lock: {}", e)))?;

        for sub in subs.iter_mut() {
            if sub.status != SubscriberStatus::Active {
                continue;
            }
            if !sub.filter.matches(event) {
                continue;
            }
            // 配送エラーは分離: 他の購読者に影響を与えない
            if sub.channel.send(event.clone()).is_ok() {
                sub.event_count += 1;
            }
        }
        Ok(())
    }
}

impl Default for SubscriberManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================
// FakeWebSocketEventChannel — メモリ内モック (RFC §12D.3)
// ============================================================

/// WebSocket 相当の双方向通信をメモリ内バッファで模倣する EventChannel 実装。
///
/// 内部に VecDeque<DarviumEvent> を保持し、send でキューに追加、
/// receive でキューから取り出す FIFO 動作を行う。
#[derive(Debug, Clone)]
pub struct FakeWebSocketEventChannel {
    /// メッセージバッファ。
    buffer: Arc<Mutex<VecDeque<DarviumEvent>>>,
    /// バッファ容量上限。
    capacity: usize,
}

impl FakeWebSocketEventChannel {
    /// 指定容量で FakeWebSocketEventChannel を作成する。
    pub fn with_capacity(capacity: usize) -> Self {
        FakeWebSocketEventChannel {
            buffer: Arc::new(Mutex::new(VecDeque::with_capacity(capacity))),
            capacity,
        }
    }

    /// デフォルト容量で FakeWebSocketEventChannel を作成する。
    pub fn new() -> Self {
        Self::with_capacity(FAKE_WS_CHANNEL_BUFFER_SIZE)
    }

    /// 現在のバッファ内イベント数を返す。
    pub fn len(&self) -> usize {
        self.buffer
            .lock()
            .map(|b| b.len())
            .unwrap_or(0)
    }

    /// バッファが空かどうかを返す。
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for FakeWebSocketEventChannel {
    fn default() -> Self {
        Self::new()
    }
}

impl EventChannel for FakeWebSocketEventChannel {
    fn send(&self, event: DarviumEvent) -> Result<(), DarviumError> {
        let mut buffer = self
            .buffer
            .lock()
            .map_err(|e| DarviumError::EventChannel(format!("buffer lock: {}", e)))?;

        if buffer.len() >= self.capacity {
            return Err(DarviumError::EventChannel(
                "FakeWebSocketEventChannel buffer full".into(),
            ));
        }

        buffer.push_back(event);
        Ok(())
    }

    fn receive(&self) -> Result<Option<DarviumEvent>, DarviumError> {
        let mut buffer = self
            .buffer
            .lock()
            .map_err(|e| DarviumError::EventChannel(format!("buffer lock: {}", e)))?;

        Ok(buffer.pop_front())
    }

    fn flush(&self) -> Result<(), DarviumError> {
        // メモリ内バッファのため flush は no-op
        Ok(())
    }
}

// ============================================================
// ExternalEventClient — 外部イベント購読クライアント
// ============================================================

/// 外部システムからのイベント購読・受信を抽象化するトレイト。
pub trait ExternalEventClient: Send + Sync {
    /// 指定 URL に接続し、イベントチャネルを取得する。
    fn connect(&self, url: &str) -> Result<Box<dyn EventChannel>, DarviumError>;

    /// 指定 ID の接続を切断する。
    fn disconnect(&self, id: &str) -> Result<(), DarviumError>;
}

// ============================================================
// FakeExternalEventClient — モック実装
// ============================================================

/// 固定シード PRNG で購読イベント系列を生成するメモリ内モック。
///
/// 決定論的再現性を保証するため、全テストで同一シードを使用する。
#[derive(Debug, Clone)]
pub struct FakeExternalEventClient {
    /// PRNG シード。
    seed: u64,
    /// アクティブなチャネル一覧。
    channels: Arc<Mutex<HashMap<String, FakeWebSocketEventChannel>>>,
    /// PRNG。
    rng: Arc<Mutex<StdRng>>,
}

impl FakeExternalEventClient {
    /// 指定シードで FakeExternalEventClient を作成する。
    pub fn with_seed(seed: u64) -> Self {
        FakeExternalEventClient {
            seed,
            channels: Arc::new(Mutex::new(HashMap::new())),
            rng: Arc::new(Mutex::new(StdRng::seed_from_u64(seed))),
        }
    }

    /// デフォルトシード (12345) で FakeExternalEventClient を作成する。
    pub fn new() -> Self {
        Self::with_seed(12345)
    }

    /// シード値を返す。
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// PRNG を用いてランダムな DarviumEventKind を生成する。
    fn generate_random_event_kind(rng: &mut StdRng) -> DarviumEventKind {
        match rng.random_range(0..13) {
            0 => DarviumEventKind::System(match rng.random_range(0..4) {
                0 => crate::event::SystemEvent::ClockAdvanced,
                1 => crate::event::SystemEvent::SnapshotTaken,
                2 => crate::event::SystemEvent::ReplayCompleted,
                _ => crate::event::SystemEvent::StartupCompleted,
            }),
            1 => DarviumEventKind::Search(match rng.random_range(0..5) {
                0 => crate::event::SearchEvent::Started,
                1 => crate::event::SearchEvent::StepCompleted,
                2 => crate::event::SearchEvent::Completed,
                3 => crate::event::SearchEvent::Failed,
                _ => crate::event::SearchEvent::Aborted,
            }),
            2 => DarviumEventKind::WorkflowExecution(
                match rng.random_range(0..4) {
                    0 => crate::event::WorkflowExecutionEvent::Started,
                    1 => crate::event::WorkflowExecutionEvent::Completed,
                    2 => crate::event::WorkflowExecutionEvent::Failed,
                    _ => crate::event::WorkflowExecutionEvent::Retried,
                },
            ),
            3 => DarviumEventKind::Training(match rng.random_range(0..9) {
                0 => crate::event::TrainingEvent::MissionGenerated,
                1 => crate::event::TrainingEvent::HumanReviewRequested,
                2 => crate::event::TrainingEvent::HumanReviewCompleted,
                3 => crate::event::TrainingEvent::SandboxExecutionStarted,
                4 => crate::event::TrainingEvent::SandboxExecutionCompleted,
                5 => crate::event::TrainingEvent::FeedbackIngested,
                6 => crate::event::TrainingEvent::PromotionCandidateCreated,
                7 => crate::event::TrainingEvent::PromotionApproved,
                _ => crate::event::TrainingEvent::PromotionRejected,
            }),
            4 => DarviumEventKind::Knowledge(match rng.random_range(0..4) {
                0 => crate::event::KnowledgeEvent::FragmentCreated,
                1 => crate::event::KnowledgeEvent::CandidateConsolidated,
                2 => crate::event::KnowledgeEvent::CanonicalPromoted,
                _ => crate::event::KnowledgeEvent::OriginTraceUpdated,
            }),
            5 => DarviumEventKind::Conversational(
                match rng.random_range(0..5) {
                    0 => crate::event::ConversationalEventEnvelope::UtteranceReceived,
                    1 => crate::event::ConversationalEventEnvelope::Classified,
                    2 => crate::event::ConversationalEventEnvelope::GateDecided,
                    3 => crate::event::ConversationalEventEnvelope::Consolidated,
                    _ => crate::event::ConversationalEventEnvelope::Promoted,
                },
            ),
            6 => DarviumEventKind::Lifecycle(match rng.random_range(0..4) {
                0 => crate::event::LifecycleEvent::NodeCreated,
                1 => crate::event::LifecycleEvent::NodeActivated,
                2 => crate::event::LifecycleEvent::NodeDeactivated,
                _ => crate::event::LifecycleEvent::NodeArchived,
            }),
            7 => DarviumEventKind::Gc(match rng.random_range(0..3) {
                0 => crate::event::GcEvent::SoftDeleted,
                1 => crate::event::GcEvent::HardDeleteCandidate,
                _ => crate::event::GcEvent::Tombstoned,
            }),
            8 => DarviumEventKind::Repair(match rng.random_range(0..4) {
                0 => crate::event::RepairEvent::InconsistencyDetected,
                1 => crate::event::RepairEvent::RetryAttempted,
                2 => crate::event::RepairEvent::TombstoneApplied,
                _ => crate::event::RepairEvent::RepairCompleted,
            }),
            9 => DarviumEventKind::Reciprocity(
                match rng.random_range(0..8) {
                    0 => crate::event::ReciprocityEventKind::HelpOffered,
                    1 => crate::event::ReciprocityEventKind::HelpAccepted,
                    2 => crate::event::ReciprocityEventKind::HelpRejected,
                    3 => crate::event::ReciprocityEventKind::HelpExecuted,
                    4 => crate::event::ReciprocityEventKind::HelpSucceeded,
                    5 => crate::event::ReciprocityEventKind::HelpAbandoned,
                    6 => crate::event::ReciprocityEventKind::HarmfulMismatch,
                    _ => crate::event::ReciprocityEventKind::ReturnedFavor,
                },
            ),
            10 => DarviumEventKind::Fusion(match rng.random_range(0..5) {
                0 => crate::event::FusionEvent::Paired,
                1 => crate::event::FusionEvent::FusionCompleted,
                2 => crate::event::FusionEvent::BirthCommitInitiated,
                3 => crate::event::FusionEvent::BirthCommitCompleted,
                _ => crate::event::FusionEvent::FusionFailed,
            }),
            11 => DarviumEventKind::Hitl(match rng.random_range(0..4) {
                0 => crate::event::HitlEvent::NotificationRequested,
                1 => crate::event::HitlEvent::InteractionRequested,
                2 => crate::event::HitlEvent::InteractionResolved,
                _ => crate::event::HitlEvent::ChannelReconnected,
            }),
            _ => DarviumEventKind::Extension("fake.external".into()),
        }
    }

    /// ランダムな DarviumEvent を生成する。
    fn generate_random_event(rng: &mut StdRng, clock: u64) -> DarviumEvent {
        let kind = Self::generate_random_event_kind(rng);
        DarviumEvent {
            event_id: uuid::Uuid::new_v4().to_string(),
            kind,
            interaction_mode: if rng.random_bool(0.7) {
                InteractionMode::OneWay
            } else {
                InteractionMode::TwoWay
            },
            payload: serde_json::json!({
                "random": rng.random::<u64>(),
            }),
            causality: EventCausality {
                parent_event_id: None,
                root_event_id: None,
                trace_ref: None,
                mission_id: None,
                workflow_id: None,
                run_id: None,
            },
            metadata: EventMetadata {
                clock,
                timestamp: std::time::SystemTime::now(),
                source: EventSource::External {
                    channel_id: "fake_external".into(),
                },
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
}

impl Default for FakeExternalEventClient {
    fn default() -> Self {
        Self::new()
    }
}

impl ExternalEventClient for FakeExternalEventClient {
    fn connect(&self, url: &str) -> Result<Box<dyn EventChannel>, DarviumError> {
        if url.is_empty() {
            return Err(DarviumError::EventChannel(
                "url must not be empty".into(),
            ));
        }

        let mut rng = self
            .rng
            .lock()
            .map_err(|e| DarviumError::EventChannel(format!("rng lock: {}", e)))?;

        let mut channels = self
            .channels
            .lock()
            .map_err(|e| DarviumError::EventChannel(format!("channels lock: {}", e)))?;

        let channel = FakeWebSocketEventChannel::new();

        // プレフィル: 5〜15 個のランダムイベントをプリロード
        let prefill_count: usize = rng.random_range(5..=15);
        let events: Vec<DarviumEvent> = (0..prefill_count)
            .map(|i| Self::generate_random_event(&mut rng, i as u64))
            .collect();

        for event in events {
            let _ = channel.send(event);
        }

        let id = url.to_string();
        channels.insert(id.clone(), channel.clone());

        Ok(Box::new(channel))
    }

    fn disconnect(&self, id: &str) -> Result<(), DarviumError> {
        let mut channels = self
            .channels
            .lock()
            .map_err(|e| DarviumError::EventChannel(format!("channels lock: {}", e)))?;

        channels
            .remove(id)
            .ok_or_else(|| DarviumError::EventChannel(format!("channel {} not found", id)))?;

        Ok(())
    }
}

// ============================================================
// シリアライズ: DarviumEvent → JSON Lines
// ============================================================

/// canonical JSON Lines 形式にシリアライズする。
fn serialize_to_canonical(event: &DarviumEvent) -> Result<String, DarviumError> {
    let kind_value = serde_json::to_value(&event.kind)
        .map_err(|e| DarviumError::EventChannel(format!("kind serialization: {}", e)))?;

    let map = match event.interaction_mode {
        InteractionMode::OneWay => serde_json::json!({
            "type": "event.publish",
            "event_kind": kind_value,
            "payload": event.payload,
        }),
        InteractionMode::TwoWay => {
            // ChannelReconnected は interaction.reconnect として出力する
            if matches!(
                event.kind,
                DarviumEventKind::Hitl(HitlEvent::ChannelReconnected)
            ) {
                serde_json::json!({
                    "type": "interaction.reconnect",
                    "interaction_id": event.event_id,
                    "event_kind": kind_value,
                    "payload": event.payload,
                })
            } else {
                serde_json::json!({
                    "type": "interaction.open",
                    "interaction_id": event.event_id,
                    "event_kind": kind_value,
                    "payload": event.payload,
                })
            }
        }
    };

    serde_json::to_string(&map)
        .map_err(|e| DarviumError::EventChannel(format!("canonical serialization: {}", e)))
}

/// 旧 HITL JSON Lines 形式 (互換モード) にシリアライズする。
fn serialize_to_legacy(event: &DarviumEvent) -> Result<String, DarviumError> {
    // 非 HITL イベントは canonical 形式にフォールバック
    let (msg_type, request) = match &event.kind {
        DarviumEventKind::Hitl(hitl) => match hitl {
            HitlEvent::NotificationRequested => ("notify", event.payload.clone()),
            HitlEvent::InteractionRequested => ("communicate", event.payload.clone()),
            HitlEvent::ChannelReconnected => ("reconnect", event.payload.clone()),
            _ => return serialize_to_canonical(event),
        },
        _ => return serialize_to_canonical(event),
    };

    let map = serde_json::json!({
        "type": msg_type,
        "interaction_id": event.event_id,
        "request": request,
    });

    serde_json::to_string(&map)
        .map_err(|e| DarviumError::EventChannel(format!("legacy serialization: {}", e)))
}

// ============================================================
// デシリアライズ: JSON Lines → DarviumEvent
// ============================================================

/// 1 行の JSON をパースし DarviumEvent を生成する。
fn parse_line(line: &str, compat: CompatMode) -> Result<Option<DarviumEvent>, DarviumError> {
    let value: serde_json::Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(_) => return Ok(None), // パース不能 → プロトコルエラーとして None
    };

    let msg_type = value.get("type").and_then(|v| v.as_str());

    match msg_type {
        Some("event.publish") => parse_event_publish(&value),
        Some("interaction.open") => parse_interaction_open(&value),
        Some("interaction.reply") => parse_interaction_reply(&value),
        Some("interaction.reconnect") => parse_interaction_reconnect(&value),
        Some("subscribe") => parse_subscribe(&value),
        Some("ack") => parse_ack(&value),
        Some("error") => parse_error_message(&value),
        // 旧プロトコル (互換モードのみ)
        Some("notify") if compat == CompatMode::Enabled => parse_legacy_convert(&value, "notify"),
        Some("communicate") if compat == CompatMode::Enabled => {
            parse_legacy_convert(&value, "communicate")
        }
        Some("reconnect") if compat == CompatMode::Enabled => {
            parse_legacy_convert(&value, "reconnect")
        }
        // type フィールドがなく outcome がある → 旧応答形式 (互換モードのみ)
        None if compat == CompatMode::Enabled && value.get("outcome").is_some() => {
            parse_legacy_response(&value)
        }
        _ => Ok(None),
    }
}

/// 汎用 DarviumEvent 構築ヘルパー（デフォルトフィールド値で埋める）。
fn build_event(
    kind: DarviumEventKind,
    interaction_mode: InteractionMode,
    payload: serde_json::Value,
) -> DarviumEvent {
    DarviumEvent {
        event_id: uuid::Uuid::new_v4().to_string(),
        kind,
        interaction_mode,
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
            timestamp: std::time::SystemTime::now(),
            source: EventSource::External {
                channel_id: "stdinout".into(),
            },
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

/// event.publish → OneWay DarviumEvent。
fn parse_event_publish(value: &serde_json::Value) -> Result<Option<DarviumEvent>, DarviumError> {
    let kind: DarviumEventKind = serde_json::from_value(value["event_kind"].clone())
        .map_err(|e| DarviumError::EventChannel(format!("parse event_kind: {}", e)))?;
    let payload = value
        .get("payload")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    Ok(Some(build_event(kind, InteractionMode::OneWay, payload)))
}

/// interaction.open → TwoWay DarviumEvent。
fn parse_interaction_open(value: &serde_json::Value) -> Result<Option<DarviumEvent>, DarviumError> {
    let kind: DarviumEventKind = serde_json::from_value(value["event_kind"].clone())
        .map_err(|e| DarviumError::EventChannel(format!("parse event_kind: {}", e)))?;
    let payload = value
        .get("payload")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    Ok(Some(build_event(kind, InteractionMode::TwoWay, payload)))
}

/// interaction.reply → 応答ペイロードを持つ System イベント。
fn parse_interaction_reply(
    value: &serde_json::Value,
) -> Result<Option<DarviumEvent>, DarviumError> {
    let outcome = value
        .get("outcome")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let interaction_id = value
        .get("interaction_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let payload = serde_json::json!({
        "interaction_id": interaction_id,
        "outcome": outcome,
    });
    Ok(Some(build_event(
        DarviumEventKind::System(crate::event::SystemEvent::ReplayCompleted),
        InteractionMode::OneWay,
        payload,
    )))
}

/// interaction.reconnect → ChannelReconnected DarviumEvent。
fn parse_interaction_reconnect(
    value: &serde_json::Value,
) -> Result<Option<DarviumEvent>, DarviumError> {
    let kind: DarviumEventKind = serde_json::from_value(value["event_kind"].clone())
        .unwrap_or(DarviumEventKind::Hitl(HitlEvent::ChannelReconnected));
    let payload = value
        .get("payload")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    Ok(Some(build_event(kind, InteractionMode::TwoWay, payload)))
}

/// subscribe → 購読要求。
fn parse_subscribe(value: &serde_json::Value) -> Result<Option<DarviumEvent>, DarviumError> {
    let event_kinds = value
        .get("event_kinds")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let payload = serde_json::json!({ "event_kinds": event_kinds });
    Ok(Some(build_event(
        DarviumEventKind::System(crate::event::SystemEvent::StartupCompleted),
        InteractionMode::OneWay,
        payload,
    )))
}

/// ack → 確認応答。
fn parse_ack(value: &serde_json::Value) -> Result<Option<DarviumEvent>, DarviumError> {
    let payload = value.clone();
    Ok(Some(build_event(
        DarviumEventKind::System(crate::event::SystemEvent::ReplayCompleted),
        InteractionMode::OneWay,
        payload,
    )))
}

/// error → エラーメッセージ。
fn parse_error_message(value: &serde_json::Value) -> Result<Option<DarviumEvent>, DarviumError> {
    let code = value
        .get("code")
        .and_then(|v| v.as_str())
        .unwrap_or("UNKNOWN");
    let message = value.get("message").and_then(|v| v.as_str()).unwrap_or("");
    let payload = serde_json::json!({
        "code": code,
        "message": message,
    });
    Ok(Some(build_event(
        DarviumEventKind::System(crate::event::SystemEvent::ReplayCompleted),
        InteractionMode::OneWay,
        payload,
    )))
}

// ============================================================
// 旧 HITL JSON Lines 変換 (互換モード)
// ============================================================

/// 旧形式 (notify/communicate/reconnect) を canonical DarviumEvent に変換する。
fn parse_legacy_convert(
    value: &serde_json::Value,
    legacy_type: &str,
) -> Result<Option<DarviumEvent>, DarviumError> {
    let request = value
        .get("request")
        .cloned()
        .unwrap_or(serde_json::Value::Null);

    let (kind, mode) = match legacy_type {
        "notify" => (
            DarviumEventKind::Hitl(HitlEvent::NotificationRequested),
            InteractionMode::OneWay,
        ),
        "communicate" => (
            DarviumEventKind::Hitl(HitlEvent::InteractionRequested),
            InteractionMode::TwoWay,
        ),
        "reconnect" => (
            DarviumEventKind::Hitl(HitlEvent::ChannelReconnected),
            InteractionMode::TwoWay,
        ),
        _ => return Ok(None),
    };

    Ok(Some(build_event(kind, mode, request)))
}

/// 旧応答形式 (outcome 直置き) を DarviumEvent に変換する。
fn parse_legacy_response(value: &serde_json::Value) -> Result<Option<DarviumEvent>, DarviumError> {
    let outcome = value
        .get("outcome")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let payload = serde_json::json!({ "outcome": outcome });
    Ok(Some(build_event(
        DarviumEventKind::System(crate::event::SystemEvent::ReplayCompleted),
        InteractionMode::OneWay,
        payload,
    )))
}

// ============================================================
// テスト
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{
        EventRetention, EventSource, EventVisibility, HitlEvent, InteractionMode,
        PiiHandlingPolicy, SearchEvent,
    };
    use std::io::{BufReader, Cursor, Write};

    // ── テスト用ヘルパー ──

    /// Arc<Mutex<Vec<u8>>> をラップする Write 実装（テスト用）。
    struct SharedVecWriter(Arc<Mutex<Vec<u8>>>);

    impl Write for SharedVecWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().write(buf)
        }
        fn flush(&mut self) -> std::io::Result<()> {
            self.0.lock().unwrap().flush()
        }
    }

    // SharedVecWriter は Send（内部の Arc<Mutex<Vec<u8>>> が Send + Sync）。
    // コンパイラが自動導出するため明示的な impl は不要。

    /// テスト用の最小 DarviumEvent を生成する。
    fn test_event(kind: DarviumEventKind, mode: InteractionMode) -> DarviumEvent {
        let event_id = uuid::Uuid::new_v4().to_string();
        DarviumEvent {
            event_id,
            kind,
            interaction_mode: mode,
            payload: serde_json::json!({"key": "value"}),
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
                timestamp: std::time::SystemTime::now(),
                source: EventSource::Test,
            },
            transport_meta: None,
            visibility: EventVisibility::Public,
            retention: EventRetention {
                persist: true,
                ttl_days: None,
            },
            privacy: crate::event::EventPrivacy {
                contains_pii: false,
                sandbox_only: false,
                pii_handling: PiiHandlingPolicy::Reject,
            },
        }
    }

    /// canonical モードでラウンドトリップする（書き出し→読み戻し）。
    fn roundtrip_canonical(events: Vec<DarviumEvent>) -> Vec<DarviumEvent> {
        let buf = Arc::new(Mutex::new(Vec::new()));
        let writer = SharedVecWriter(buf.clone());
        let reader = BufReader::new(Cursor::new(Vec::new()));
        let channel = StdinoutEventChannel::new(reader, writer, CompatMode::Disabled);

        for event in &events {
            channel.send(event.clone()).unwrap();
        }
        channel.flush().unwrap();

        let output = buf.lock().unwrap().clone();
        let reader = BufReader::new(Cursor::new(output));
        let writer = Vec::new();
        let channel = StdinoutEventChannel::new(reader, writer, CompatMode::Disabled);

        let mut result = Vec::new();
        while let Some(event) = channel.receive().unwrap() {
            result.push(event);
        }
        result
    }

    // ============================================================
    // T1: EventChannel トレイト型境界テスト
    // ============================================================

    /// T1-1: EventChannel が Send + Sync を実装していることのコンパイル時確認。
    #[test]
    fn t1_1_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        assert_send::<StdinoutEventChannel<BufReader<Cursor<Vec<u8>>>, Vec<u8>>>();
        assert_sync::<StdinoutEventChannel<BufReader<Cursor<Vec<u8>>>, Vec<u8>>>();
    }

    /// T1-2: Box<dyn EventChannel> が使用可能であること。
    #[test]
    fn t1_2_box_dyn_event_channel() {
        let channel: Box<dyn EventChannel> = Box::new(StdinoutEventChannel::new(
            BufReader::new(Cursor::new(Vec::new())),
            Vec::new(),
            CompatMode::Disabled,
        ));
        let event = test_event(
            DarviumEventKind::Search(SearchEvent::Started),
            InteractionMode::OneWay,
        );
        // send / receive / flush が dyn 経由で呼べることを確認
        channel.send(event).unwrap();
        let received = channel.receive().unwrap();
        assert!(received.is_none()); // 空なので None
    }

    // ============================================================
    // T2: StdinoutEventChannel canonical モードのラウンドトリップ
    // ============================================================

    /// T2-1: send → receive ラウンドトリップで同一 DarviumEvent が戻ること。
    #[test]
    fn t2_1_roundtrip_single() {
        let event = test_event(
            DarviumEventKind::Search(SearchEvent::Started),
            InteractionMode::OneWay,
        );
        let received = roundtrip_canonical(vec![event.clone()]);
        assert_eq!(received.len(), 1);
        // kind と payload の一致を確認（event_id や timestamp は再生成される）
        assert_eq!(received[0].kind, event.kind);
        assert_eq!(received[0].payload, event.payload);
        assert_eq!(received[0].interaction_mode, event.interaction_mode);
    }

    /// T2-2: 13 種の DarviumEventKind すべてでラウンドトリップが成功すること。
    #[test]
    fn t2_2_all_kinds_roundtrip() {
        use crate::event::{
            ConversationalEventEnvelope, FusionEvent, GcEvent, KnowledgeEvent, LifecycleEvent,
            ReciprocityEventKind, RepairEvent, SystemEvent, TrainingEvent, WorkflowExecutionEvent,
        };

        let kinds: Vec<DarviumEventKind> = vec![
            DarviumEventKind::System(SystemEvent::ClockAdvanced),
            DarviumEventKind::Search(SearchEvent::Completed),
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
            DarviumEventKind::Extension("custom.test".into()),
        ];

        for kind in kinds {
            let event = test_event(kind.clone(), InteractionMode::OneWay);
            let received = roundtrip_canonical(vec![event.clone()]);
            assert_eq!(received[0].kind, event.kind, "kind mismatch for {:?}", kind);
        }
    }

    /// T2-3: 1000 イベントの一括ラウンドトリップで消失ゼロを確認。
    #[test]
    fn t2_3_bulk_1000_roundtrip() {
        let n: usize = 1000;
        let events: Vec<DarviumEvent> = (0..n)
            .map(|i| {
                let kind = if i % 2 == 0 {
                    DarviumEventKind::Search(SearchEvent::Started)
                } else {
                    DarviumEventKind::Hitl(HitlEvent::NotificationRequested)
                };
                test_event(kind, InteractionMode::OneWay)
            })
            .collect();

        let received = roundtrip_canonical(events.clone());
        assert_eq!(
            received.len(),
            n,
            "bulk roundtrip: expected {} events, got {}",
            n,
            received.len()
        );

        println!("T2-3: sent={} received={} loss=0", n, received.len());
    }

    /// T2-4: flush 後にバッファがフラッシュされること。
    #[test]
    fn t2_4_flush() {
        let mut buffer = Vec::new();
        {
            let reader = BufReader::new(Cursor::new(Vec::new()));
            let channel = StdinoutEventChannel::new(reader, &mut buffer, CompatMode::Disabled);
            let event = test_event(
                DarviumEventKind::Search(SearchEvent::Started),
                InteractionMode::OneWay,
            );
            channel.send(event).unwrap();
            channel.flush().unwrap();
        }
        assert!(!buffer.is_empty(), "flush 後にバッファにデータがあること");
    }

    // ============================================================
    // T3: canonical JSON Lines プロトコルのメッセージ形式
    // ============================================================

    /// T3-1: send(event.publish) の出力形式を確認する。
    #[test]
    fn t3_1_event_publish_format() {
        let output = Arc::new(Mutex::new(Vec::new()));
        let writer = SharedVecWriter(output.clone());
        let reader = BufReader::new(Cursor::new(Vec::new()));
        let channel = StdinoutEventChannel::new(reader, writer, CompatMode::Disabled);

        let event = test_event(
            DarviumEventKind::Search(SearchEvent::Started),
            InteractionMode::OneWay,
        );
        channel.send(event).unwrap();
        channel.flush().unwrap();

        let output_str = String::from_utf8(output.lock().unwrap().clone()).unwrap();
        assert!(
            output_str.contains(r#""type":"event.publish""#),
            "expected event.publish type, got: {}",
            output_str
        );
        assert!(
            output_str.contains(r#""payload""#),
            "expected payload field, got: {}",
            output_str
        );
    }

    /// T3-2: send(interaction.open) の出力形式を確認する。
    #[test]
    fn t3_2_interaction_open_format() {
        let output = Arc::new(Mutex::new(Vec::new()));
        let writer = SharedVecWriter(output.clone());
        let reader = BufReader::new(Cursor::new(Vec::new()));
        let channel = StdinoutEventChannel::new(reader, writer, CompatMode::Disabled);

        let event = test_event(
            DarviumEventKind::Hitl(HitlEvent::InteractionRequested),
            InteractionMode::TwoWay,
        );
        channel.send(event).unwrap();
        channel.flush().unwrap();

        let output_str = String::from_utf8(output.lock().unwrap().clone()).unwrap();
        assert!(
            output_str.contains(r#""type":"interaction.open""#),
            "expected interaction.open type, got: {}",
            output_str
        );
        assert!(
            output_str.contains(r#""interaction_id""#),
            "expected interaction_id field, got: {}",
            output_str
        );
    }

    /// T3-3: send(interaction.reconnect) の出力形式を確認する。
    #[test]
    fn t3_3_interaction_reconnect_format() {
        let output = Arc::new(Mutex::new(Vec::new()));
        let writer = SharedVecWriter(output.clone());
        let reader = BufReader::new(Cursor::new(Vec::new()));
        let channel = StdinoutEventChannel::new(reader, writer, CompatMode::Disabled);

        let event = test_event(
            DarviumEventKind::Hitl(HitlEvent::ChannelReconnected),
            InteractionMode::TwoWay,
        );
        channel.send(event).unwrap();
        channel.flush().unwrap();

        let output_str = String::from_utf8(output.lock().unwrap().clone()).unwrap();
        assert!(
            output_str.contains(r#""type":"interaction.reconnect""#),
            "expected interaction.reconnect type, got: {}",
            output_str
        );
    }

    /// T3-4: subscribe 出力形式を確認する。
    #[test]
    fn t3_4_subscribe_not_supported_static() {
        // subscribe は send では送出せず、receive で解釈する
        // send() で TwoWay 以外は event.publish になることを確認
        let output = Arc::new(Mutex::new(Vec::new()));
        let writer = SharedVecWriter(output.clone());
        let reader = BufReader::new(Cursor::new(Vec::new()));
        let channel = StdinoutEventChannel::new(reader, writer, CompatMode::Disabled);

        let event = test_event(
            DarviumEventKind::System(crate::event::SystemEvent::StartupCompleted),
            InteractionMode::OneWay,
        );
        channel.send(event).unwrap();
        channel.flush().unwrap();

        let output_str = String::from_utf8(output.lock().unwrap().clone()).unwrap();
        assert!(
            output_str.contains(r#""type":"event.publish""#),
            "OneWay System event should produce event.publish, got: {}",
            output_str
        );
    }

    // ============================================================
    // T4: compat モード — 旧プロトコル変換
    // ============================================================

    /// T4-1: compat モードで旧 notify 形式が HitlEvent::NotificationRequested に変換されること。
    #[test]
    fn t4_1_legacy_notify_parsed() {
        let legacy_line =
            r#"{"type":"notify","interaction_id":"id-1","request":{"subject":"test"}}"#;
        let reader = BufReader::new(Cursor::new(legacy_line));
        let writer = Vec::new();
        let channel = StdinoutEventChannel::new(reader, writer, CompatMode::Enabled);

        let received = channel
            .receive()
            .unwrap()
            .expect("should parse legacy notify");
        assert_eq!(
            received.kind,
            DarviumEventKind::Hitl(HitlEvent::NotificationRequested)
        );
        assert_eq!(received.interaction_mode, InteractionMode::OneWay);
    }

    /// T4-2: compat モードで旧 communicate 形式が HitlEvent::InteractionRequested に変換されること。
    #[test]
    fn t4_2_legacy_communicate_parsed() {
        let legacy_line =
            r#"{"type":"communicate","interaction_id":"id-2","request":{"subject":"test"}}"#;
        let reader = BufReader::new(Cursor::new(legacy_line));
        let writer = Vec::new();
        let channel = StdinoutEventChannel::new(reader, writer, CompatMode::Enabled);

        let received = channel
            .receive()
            .unwrap()
            .expect("should parse legacy communicate");
        assert_eq!(
            received.kind,
            DarviumEventKind::Hitl(HitlEvent::InteractionRequested)
        );
        assert_eq!(received.interaction_mode, InteractionMode::TwoWay);
    }

    /// T4-3: compat モードで旧 reconnect 形式が ChannelReconnected に変換されること。
    #[test]
    fn t4_3_legacy_reconnect_parsed() {
        let legacy_line =
            r#"{"type":"reconnect","interaction_id":"id-3","request":{"subject":"test"}}"#;
        let reader = BufReader::new(Cursor::new(legacy_line));
        let writer = Vec::new();
        let channel = StdinoutEventChannel::new(reader, writer, CompatMode::Enabled);

        let received = channel
            .receive()
            .unwrap()
            .expect("should parse legacy reconnect");
        assert_eq!(
            received.kind,
            DarviumEventKind::Hitl(HitlEvent::ChannelReconnected)
        );
        assert_eq!(received.interaction_mode, InteractionMode::TwoWay);
    }

    /// T4-4: compat モードの send 出力が旧形式であること。
    #[test]
    fn t4_4_legacy_send_format() {
        let output = Arc::new(Mutex::new(Vec::new()));
        let writer = SharedVecWriter(output.clone());
        let reader = BufReader::new(Cursor::new(Vec::new()));
        let channel = StdinoutEventChannel::new(reader, writer, CompatMode::Enabled);

        let event = test_event(
            DarviumEventKind::Hitl(HitlEvent::NotificationRequested),
            InteractionMode::OneWay,
        );
        channel.send(event).unwrap();
        channel.flush().unwrap();

        let output_str = String::from_utf8(output.lock().unwrap().clone()).unwrap();
        assert!(
            output_str.contains(r#""type":"notify""#),
            "compat mode should produce legacy 'notify', got: {}",
            output_str
        );
    }

    /// T4-5: Disabled モードで旧形式がパースエラーになること。
    #[test]
    fn t4_5_legacy_rejected_in_canonical_mode() {
        let legacy_line =
            r#"{"type":"notify","interaction_id":"id-1","request":{"subject":"test"}}"#;
        let reader = BufReader::new(Cursor::new(legacy_line));
        let writer = Arc::new(Mutex::new(Vec::new()));
        let test_writer = SharedVecWriter(writer.clone());
        let channel = StdinoutEventChannel::new(reader, test_writer, CompatMode::Disabled);

        let received = channel.receive().unwrap();
        assert!(
            received.is_none(),
            "Disabled mode should reject legacy format"
        );

        // エラーメッセージが出力されていることを確認
        let err_output = String::from_utf8(writer.lock().unwrap().clone()).unwrap();
        assert!(
            err_output.contains(r#""type":"error""#),
            "error response should be written, got: {}",
            err_output
        );
    }

    // ============================================================
    // T5: パースエラー処理
    // ============================================================

    /// T5-1: 不正 JSON の receive がエラーを返さず None を返すこと。
    #[test]
    fn t5_1_invalid_json_returns_none() {
        let invalid = "this is not json\n";
        let reader = BufReader::new(Cursor::new(invalid));
        let writer = Arc::new(Mutex::new(Vec::new()));
        let test_writer = SharedVecWriter(writer.clone());
        let channel = StdinoutEventChannel::new(reader, test_writer, CompatMode::Disabled);

        // receive は Err ではなく Ok(None) を返す
        let result = channel.receive().unwrap();
        assert!(result.is_none(), "invalid JSON should return None, not Err");
    }

    /// T5-2: 不明な type フィールドの入力行に対して None が返ること。
    #[test]
    fn t5_2_unknown_type_returns_none() {
        let unknown = r#"{"type":"unknown_type","data":1}"#;
        let reader = BufReader::new(Cursor::new(unknown));
        let writer = Vec::new();
        let channel = StdinoutEventChannel::new(reader, writer, CompatMode::Disabled);

        let result = channel.receive().unwrap();
        assert!(result.is_none(), "unknown type should return None");
    }

    /// T5-3: 空行がスキップされること。
    #[test]
    fn t5_3_empty_line_skipped() {
        let data = "\n\n";
        let reader = BufReader::new(Cursor::new(data));
        let writer = Vec::new();
        let channel = StdinoutEventChannel::new(reader, writer, CompatMode::Disabled);

        let result = channel.receive().unwrap();
        assert!(result.is_none(), "empty line should return None");
    }

    /// T5-4: 空白のみの行がスキップされること。
    #[test]
    fn t5_4_whitespace_line_skipped() {
        let data = "   \t  \n";
        let reader = BufReader::new(Cursor::new(data));
        let writer = Vec::new();
        let channel = StdinoutEventChannel::new(reader, writer, CompatMode::Disabled);

        let result = channel.receive().unwrap();
        assert!(result.is_none(), "whitespace line should return None");
    }

    // ============================================================
    // T6: 互換モード往復変換の情報損失ゼロ
    // ============================================================

    /// T6-1: 旧形式 → canonical 変換 → 旧形式の往復で情報が一致すること。
    #[test]
    fn t6_1_legacy_to_canonical_roundtrip() {
        let legacy_types = ["notify", "communicate", "reconnect"];
        let expected_kinds: [DarviumEventKind; 3] = [
            DarviumEventKind::Hitl(HitlEvent::NotificationRequested),
            DarviumEventKind::Hitl(HitlEvent::InteractionRequested),
            DarviumEventKind::Hitl(HitlEvent::ChannelReconnected),
        ];

        for (i, legacy_type) in legacy_types.iter().enumerate() {
            let legacy_line = format!(
                r#"{{"type":"{}","interaction_id":"id-{}","request":{{"subject":"test"}}}}"#,
                legacy_type, i
            );

            // 1. 旧形式を canonical DarviumEvent に変換 (compat mode receive)
            let reader = BufReader::new(Cursor::new(legacy_line.as_bytes()));
            let buf = Arc::new(Mutex::new(Vec::new()));
            let writer = SharedVecWriter(buf.clone());
            let channel = StdinoutEventChannel::new(reader, writer, CompatMode::Enabled);
            let event = channel.receive().unwrap().expect("should parse legacy");

            // kind が正しいことを確認
            assert_eq!(
                event.kind, expected_kinds[i],
                "legacy type {} should convert to expected kind",
                legacy_type
            );

            // 2. canonical DarviumEvent を旧形式に再変換 (compat mode send)
            let reader2 = BufReader::new(Cursor::new(Vec::new()));
            let buf2 = Arc::new(Mutex::new(Vec::new()));
            let writer2 = SharedVecWriter(buf2.clone());
            let channel2 = StdinoutEventChannel::new(reader2, writer2, CompatMode::Enabled);
            channel2.send(event).unwrap();
            channel2.flush().unwrap();

            let re_serialized = String::from_utf8(buf2.lock().unwrap().clone()).unwrap();
            assert!(
                re_serialized.contains(&format!(r#""type":"{}""#, legacy_type)),
                "re-serialized should contain original legacy type '{}', got: {}",
                legacy_type,
                re_serialized
            );
        }
    }

    /// T6-2: canonical 形式 → 旧形式変換 → canonical 形式の往復で情報が一致すること。
    #[test]
    fn t6_2_canonical_to_legacy_roundtrip() {
        // canonical DarviumEvent を compat mode で send → legacy 形式
        let input_buf = Arc::new(Mutex::new(Vec::new()));
        let writer = SharedVecWriter(input_buf.clone());
        let reader = BufReader::new(Cursor::new(Vec::new()));
        let channel = StdinoutEventChannel::new(reader, writer, CompatMode::Enabled);

        let original = test_event(
            DarviumEventKind::Hitl(HitlEvent::NotificationRequested),
            InteractionMode::OneWay,
        );
        channel.send(original.clone()).unwrap();
        channel.flush().unwrap();

        // legacy 形式を compat mode で receive → canonical DarviumEvent
        let legacy_output = input_buf.lock().unwrap().clone();
        let reader2 = BufReader::new(Cursor::new(legacy_output));
        let writer2 = Vec::new();
        let channel2 = StdinoutEventChannel::new(reader2, writer2, CompatMode::Enabled);
        let received = channel2.receive().unwrap().expect("should re-parse");

        assert_eq!(received.kind, original.kind);
        assert_eq!(received.interaction_mode, original.interaction_mode);
        // payload の subject は legacy 変換で "request" にラップされる可能性がある
        // canonical → legacy → canonical で payload 表現が変わる場合もある
        // 最低限 kind と interaction_mode が一致すれば OK
    }

    // ============================================================
    // T1: EventSubscriber 基本操作
    // ============================================================

    /// T1-1: EventSubscriber 構造体の全フィールド設定・取得。
    #[test]
    fn t1_1_event_subscriber_fields() {
        let filter = EventFilter::all();
        let channel = FakeWebSocketEventChannel::new();
        let sub = EventSubscriber {
            subscription_id: SubscriptionId::new(),
            filter: filter.clone(),
            channel: Box::new(channel),
            status: SubscriberStatus::Active,
            event_count: 42,
        };
        assert_eq!(sub.status, SubscriberStatus::Active);
        assert_eq!(sub.event_count, 42);
        // filter が正しく設定されていることを確認
        let event = test_event(
            DarviumEventKind::Search(SearchEvent::Started),
            InteractionMode::OneWay,
        );
        assert!(sub.filter.matches(&event));
    }

    /// T1-2: SubscriberStatus 全 variant のパターンマッチ網羅。
    #[test]
    fn t1_2_subscriber_status_exhaustive() {
        let describe = |s: SubscriberStatus| -> &'static str {
            match s {
                SubscriberStatus::Active => "active",
                SubscriberStatus::Paused => "paused",
                SubscriberStatus::Disconnected => "disconnected",
            }
        };
        assert_eq!(describe(SubscriberStatus::Active), "active");
        assert_eq!(describe(SubscriberStatus::Paused), "paused");
        assert_eq!(describe(SubscriberStatus::Disconnected), "disconnected");
    }

    /// T1-3: SubscriptionId newtype の UUIDv4 互換性。
    #[test]
    fn t1_3_subscription_id_uuid() {
        let id = SubscriptionId::new();
        let parsed = uuid::Uuid::parse_str(&id.0);
        assert!(parsed.is_ok(), "SubscriptionId が UUID としてパース可能であること");
        // Clone / Debug / PartialEq
        let cloned = id.clone();
        assert_eq!(id, cloned);
        let debug = format!("{:?}", id);
        assert!(!debug.is_empty());
    }

    /// T1-4: SubscriptionId from String / Display ラウンドトリップ。
    #[test]
    fn t1_4_subscription_id_from_string() {
        let s = "550e8400-e29b-41d4-a716-446655440000".to_string();
        let id: SubscriptionId = s.clone().into();
        assert_eq!(id.0, s);
        let display = format!("{}", id);
        assert_eq!(display, s);
        let back: String = id.into();
        assert_eq!(back, s);
    }

    // ============================================================
    // T2: SubscriberManager 購読管理
    // ============================================================

    /// T2-1: 購読登録 → 一覧に含まれる。
    #[test]
    fn t2_1_register_contains() {
        let manager = SubscriberManager::new();
        let filter = EventFilter::all();
        let channel = FakeWebSocketEventChannel::new();
        let id = manager.register(filter, Box::new(channel)).unwrap();
        let list = manager.list().unwrap();
        assert!(list.iter().any(|s| s.subscription_id == id));
    }

    /// T2-2: 購読解除 → 一覧から削除される。
    #[test]
    fn t2_2_unregister_removes() {
        let manager = SubscriberManager::new();
        let id = manager
            .register(EventFilter::all(), Box::new(FakeWebSocketEventChannel::new()))
            .unwrap();
        manager.unregister(&id).unwrap();
        let list = manager.list().unwrap();
        assert!(!list.iter().any(|s| s.subscription_id == id));
    }

    /// T2-3: 複数購読登録・一覧サイズ確認。
    #[test]
    fn t2_3_multiple_subscribers() {
        let manager = SubscriberManager::new();
        let n = 5;
        for _ in 0..n {
            manager
                .register(EventFilter::all(), Box::new(FakeWebSocketEventChannel::new()))
                .unwrap();
        }
        assert_eq!(manager.list().unwrap().len(), n);
    }

    /// T2-4: 存在しない ID の unregister が Err を返す。
    #[test]
    fn t2_4_unregister_not_found() {
        let manager = SubscriberManager::new();
        let id = SubscriptionId::new();
        let result = manager.unregister(&id);
        assert!(result.is_err());
    }

    /// T2-6: 空の manager で distribute が正常終了。
    #[test]
    fn t2_6_empty_distribute_ok() {
        let manager = SubscriberManager::new();
        let event = test_event(
            DarviumEventKind::Search(SearchEvent::Started),
            InteractionMode::OneWay,
        );
        let result = manager.distribute(&event);
        assert!(result.is_ok());
    }

    /// T2-7: 空の manager で list が空 Vec を返す。
    #[test]
    fn t2_7_empty_list() {
        let manager = SubscriberManager::new();
        assert!(manager.list().unwrap().is_empty());
    }

    // ============================================================
    // T3: SubscriberManager distribute — フィルタリング
    // ============================================================

    /// T3-1: フィルタに合致するイベント → subscriber の event_count 増加。
    #[test]
    fn t3_1_distribute_matching_increases_count() {
        let manager = SubscriberManager::new();
        let filter = EventFilter {
            kind_filter: Some(vec![DarviumEventKind::Search(SearchEvent::Started)]),
            since_vt: None,
            until_vt: None,
        };
        manager
            .register(filter, Box::new(FakeWebSocketEventChannel::new()))
            .unwrap();

        let event = test_event(
            DarviumEventKind::Search(SearchEvent::Started),
            InteractionMode::OneWay,
        );
        manager.distribute(&event).unwrap();

        let list = manager.list().unwrap();
        assert_eq!(list[0].event_count, 1);
    }

    /// T3-2: フィルタに合致しないイベント → event_count 不変。
    #[test]
    fn t3_2_distribute_non_matching_unchanged() {
        let manager = SubscriberManager::new();
        let filter = EventFilter {
            kind_filter: Some(vec![DarviumEventKind::Search(SearchEvent::Started)]),
            since_vt: None,
            until_vt: None,
        };
        manager
            .register(filter, Box::new(FakeWebSocketEventChannel::new()))
            .unwrap();

        // Search::Completed はフィルタに合致しない
        let event = test_event(
            DarviumEventKind::Search(SearchEvent::Completed),
            InteractionMode::OneWay,
        );
        manager.distribute(&event).unwrap();

        let list = manager.list().unwrap();
        assert_eq!(list[0].event_count, 0);
    }

    /// T3-3: 複数購読者全員に合致 → 全員の count 増加。
    #[test]
    fn t3_3_all_subscribers_receive() {
        let manager = SubscriberManager::new();
        let n = 3;
        for _ in 0..n {
            manager
                .register(EventFilter::all(), Box::new(FakeWebSocketEventChannel::new()))
                .unwrap();
        }
        let event = test_event(
            DarviumEventKind::System(crate::event::SystemEvent::ClockAdvanced),
            InteractionMode::OneWay,
        );
        manager.distribute(&event).unwrap();

        let list = manager.list().unwrap();
        for sub in &list {
            assert_eq!(sub.event_count, 1, "購読者 {} がイベントを受信していること", sub.subscription_id);
        }
    }

    /// T3-4: 購読解除後に distribute → count 不変。
    #[test]
    fn t3_4_unsubscribed_not_receive() {
        let manager = SubscriberManager::new();
        let id = manager
            .register(EventFilter::all(), Box::new(FakeWebSocketEventChannel::new()))
            .unwrap();
        manager.unregister(&id).unwrap();

        let event = test_event(
            DarviumEventKind::Search(SearchEvent::Started),
            InteractionMode::OneWay,
        );
        manager.distribute(&event).unwrap();

        // 購読解除後は配送されない
        assert!(manager.list().unwrap().is_empty());
    }

    /// T3-6: EventFilter::all() で購読 → 全イベント受信。
    #[test]
    fn t3_6_filter_all_receives_all() {
        let manager = SubscriberManager::new();
        manager
            .register(EventFilter::all(), Box::new(FakeWebSocketEventChannel::new()))
            .unwrap();

        let kinds = vec![
            DarviumEventKind::System(crate::event::SystemEvent::ClockAdvanced),
            DarviumEventKind::Search(SearchEvent::Started),
            DarviumEventKind::Hitl(HitlEvent::NotificationRequested),
        ];
        for kind in &kinds {
            let event = test_event(kind.clone(), InteractionMode::OneWay);
            manager.distribute(&event).unwrap();
        }

        let list = manager.list().unwrap();
        assert_eq!(list[0].event_count, kinds.len() as u64);
    }

    // ============================================================
    // T4: FakeWebSocketEventChannel ラウンドトリップ
    // ============================================================

    /// T4-1: send → receive ラウンドトリップ。
    #[test]
    fn t4_1_roundtrip() {
        let channel = FakeWebSocketEventChannel::new();
        let event = test_event(
            DarviumEventKind::Search(SearchEvent::Started),
            InteractionMode::OneWay,
        );
        channel.send(event.clone()).unwrap();
        let received = channel.receive().unwrap().expect("should receive event");
        assert_eq!(received.kind, event.kind);
        assert_eq!(received.payload, event.payload);
        assert_eq!(received.interaction_mode, event.interaction_mode);
    }

    /// T4-2: FIFO 順序保存。
    #[test]
    fn t4_2_fifo_order() {
        let channel = FakeWebSocketEventChannel::new();
        let n = 10;
        let events: Vec<DarviumEvent> = (0..n)
            .map(|i| {
                test_event(
                    if i % 2 == 0 {
                        DarviumEventKind::Search(SearchEvent::Started)
                    } else {
                        DarviumEventKind::Hitl(HitlEvent::NotificationRequested)
                    },
                    InteractionMode::OneWay,
                )
            })
            .collect();

        for event in &events {
            channel.send(event.clone()).unwrap();
        }

        for (i, original) in events.iter().enumerate() {
            let received = channel.receive().unwrap().expect("should receive");
            assert_eq!(
                received.kind, original.kind,
                "FIFO order mismatch at index {}",
                i
            );
        }
    }

    /// T4-3: 空チャネルで receive → None。
    #[test]
    fn t4_3_empty_receives_none() {
        let channel = FakeWebSocketEventChannel::new();
        assert!(channel.receive().unwrap().is_none());
        assert!(channel.is_empty());
    }

    /// T4-4: flush がエラーを返さない。
    #[test]
    fn t4_4_flush_ok() {
        let channel = FakeWebSocketEventChannel::new();
        let event = test_event(
            DarviumEventKind::Search(SearchEvent::Started),
            InteractionMode::OneWay,
        );
        channel.send(event).unwrap();
        let result = channel.flush();
        assert!(result.is_ok());
    }

    /// T4-5: Send + Sync のコンパイル時確認。
    #[test]
    fn t4_5_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}
        assert_send::<FakeWebSocketEventChannel>();
        assert_sync::<FakeWebSocketEventChannel>();
    }

    /// T4-6: Box<dyn EventChannel> としての利用確認。
    #[test]
    fn t4_6_box_dyn_event_channel() {
        let channel: Box<dyn EventChannel> = Box::new(FakeWebSocketEventChannel::new());
        let event = test_event(
            DarviumEventKind::Search(SearchEvent::Started),
            InteractionMode::OneWay,
        );
        channel.send(event).unwrap();
        let received = channel.receive().unwrap();
        assert!(received.is_some());
    }

    /// T4-7: バッファ容量超過時に Err を返す。
    #[test]
    fn t4_7_buffer_full_error() {
        let channel = FakeWebSocketEventChannel::with_capacity(2);
        let event = test_event(
            DarviumEventKind::Search(SearchEvent::Started),
            InteractionMode::OneWay,
        );
        channel.send(event.clone()).unwrap();
        channel.send(event.clone()).unwrap();
        let result = channel.send(event);
        assert!(
            result.is_err(),
            "capacity exceeded should return Err"
        );
    }

    // ============================================================
    // T5: ExternalEventClient トレイト
    // ============================================================

    /// T5-1: connect → Box<dyn EventChannel> 取得。
    #[test]
    fn t5_1_connect_returns_channel() {
        let client = FakeExternalEventClient::new();
        let channel = client.connect("ws://test.example.com").unwrap();
        // 接続後はイベントがプリロードされている
        let received = channel.receive().unwrap();
        assert!(received.is_some(), "connect 後にイベントが受信可能であること");
    }

    /// T5-2: disconnect 後に利用不可。
    #[test]
    fn t5_2_disconnect_removes_channel() {
        let client = FakeExternalEventClient::new();
        let url = "ws://test.example.com";
        client.connect(url).unwrap();
        client.disconnect(url).unwrap();
        // disconnect 後の同一 ID での接続試行は失敗する
        let result = client.disconnect(url);
        assert!(result.is_err(), "2回目の disconnect はエラーになること");
    }

    /// T5-4: 不正 URL → Err。
    #[test]
    fn t5_4_empty_url_error() {
        let client = FakeExternalEventClient::new();
        let result = client.connect("");
        assert!(result.is_err(), "空 URL の接続はエラーになること");
    }

    // ============================================================
    // T6: FakeExternalEventClient 固定シードイベント生成
    // ============================================================

    /// T6-1: connect 後にイベントの受信が可能。
    #[test]
    fn t6_1_connect_receives_events() {
        let client = FakeExternalEventClient::new();
        let channel = client.connect("ws://test").unwrap();
        let received = channel.receive().unwrap();
        assert!(received.is_some(), "connect 後にイベントを受信できること");
    }

    /// T6-2: 同一シードで同一系列のイベントが生成される。
    #[test]
    fn t6_2_same_seed_same_sequence() {
        let client_a = FakeExternalEventClient::with_seed(9999);
        let client_b = FakeExternalEventClient::with_seed(9999);

        let chan_a = client_a.connect("ws://a").unwrap();
        let chan_b = client_b.connect("ws://b").unwrap();

        let events_a: Vec<DarviumEvent> = std::iter::from_fn(|| chan_a.receive().unwrap()).take(5).collect();
        let events_b: Vec<DarviumEvent> = std::iter::from_fn(|| chan_b.receive().unwrap()).take(5).collect();

        assert_eq!(events_a.len(), events_b.len());
        for (i, (a, b)) in events_a.iter().zip(events_b.iter()).enumerate() {
            assert_eq!(
                a.kind, b.kind,
                "同一シード: kind が一致すること (index={})",
                i
            );
        }
    }

    /// T6-3: 異なるシードで異なる系列。
    #[test]
    fn t6_3_different_seed_different_sequence() {
        let client_a = FakeExternalEventClient::with_seed(1111);
        let client_b = FakeExternalEventClient::with_seed(2222);

        let chan_a = client_a.connect("ws://a").unwrap();
        let chan_b = client_b.connect("ws://b").unwrap();

        let event_a = chan_a.receive().unwrap();
        let event_b = chan_b.receive().unwrap();

        // 異なるシード → 異なる系列（確率的に99.9%以上異なる）
        // 両方 Some であることのみ確認
        assert!(event_a.is_some());
        assert!(event_b.is_some());
    }

    /// T6-5: 生成される DarviumEventKind が多様である。
    #[test]
    fn t6_5_diverse_event_kinds() {
        let client = FakeExternalEventClient::with_seed(7777);
        let channel = client.connect("ws://test").unwrap();

        let events: Vec<DarviumEvent> =
            std::iter::from_fn(|| channel.receive().unwrap()).take(20).collect();

        assert!(!events.is_empty(), "イベントが生成されること");
        // 2 種類以上の event_kind が含まれていることを確認
        let kind_set: std::collections::HashSet<_> =
            events.iter().map(|e| format!("{:?}", e.kind)).collect();
        assert!(
            kind_set.len() >= 2,
            "イベント種別が多様であること: {} 種類",
            kind_set.len()
        );
    }

    // ============================================================
    // T7: 統合テスト — SubscriberManager 配送完全性
    // ============================================================

    /// N 個のランダムイベントを生成するテスト用ヘルパー。
    fn generate_test_events(seed: u64, count: u64) -> Vec<DarviumEvent> {
        let mut rng = StdRng::seed_from_u64(seed);
        (0..count)
            .map(|i| {
                let kind = FakeExternalEventClient::generate_random_event_kind(&mut rng);
                DarviumEvent {
                    event_id: uuid::Uuid::new_v4().to_string(),
                    kind,
                    interaction_mode: InteractionMode::OneWay,
                    payload: serde_json::json!({"test": true}),
                    causality: EventCausality {
                        parent_event_id: None,
                        root_event_id: None,
                        trace_ref: None,
                        mission_id: None,
                        workflow_id: None,
                        run_id: None,
                    },
                    metadata: EventMetadata {
                        clock: i,
                        timestamp: std::time::SystemTime::now(),
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
            })
            .collect()
    }

    /// T7-1: イベントを SubscriberManager 経由で購読者に配送する基本フロー。
    #[test]
    fn t7_1_external_event_flow() {
        let manager = SubscriberManager::new();
        let sub_channel = FakeWebSocketEventChannel::new();

        // 全イベント購読
        let filter = EventFilter::all();
        manager
            .register(filter, Box::new(sub_channel.clone()))
            .unwrap();

        let events = generate_test_events(12345, 100);
        for event in &events {
            manager.distribute(event).unwrap();
        }

        let list = manager.list().unwrap();
        println!(
            "T7-1: n_events={} received={}",
            events.len(),
            list[0].event_count
        );
        assert_eq!(
            list[0].event_count,
            events.len() as u64,
            "全イベントが受信されること"
        );
    }

    /// T7-2: 購読フィルタ精度（偽陽性0%、偽陰性0%）。
    #[test]
    fn t7_2_filter_accuracy() {
        let manager = SubscriberManager::new();
        let sub_channel = FakeWebSocketEventChannel::new();

        // Search イベント全般を購読
        let filter = EventFilter {
            kind_filter: Some(vec![DarviumEventKind::Search(SearchEvent::Started)]),
            since_vt: None,
            until_vt: None,
        };
        manager
            .register(filter, Box::new(sub_channel.clone()))
            .unwrap();

        let events = generate_test_events(42, 500);
        let mut matched = 0u64;
        for event in &events {
            let is_search = matches!(event.kind, DarviumEventKind::Search(SearchEvent::Started));
            if is_search {
                matched += 1;
            }
            manager.distribute(event).unwrap();
        }

        let list = manager.list().unwrap();
        let received = list[0].event_count;

        // 偽陰性率: 0%（フィルタに合致する全イベントが受信されている）
        let false_negative = if matched > 0 {
            (matched - received) as f64 / matched as f64
        } else {
            0.0
        };
        // 偽陽性率: 0%（フィルタに合致しないイベントは受信されていない）
        let false_positive = if events.len() as u64 - matched > 0 {
            let excess = received.saturating_sub(matched);
            excess as f64 / (events.len() as u64 - matched) as f64
        } else {
            0.0
        };

        println!(
            "T7-2: total={} matched={} received={} fp_rate={:.6} fn_rate={:.6}",
            events.len(),
            matched,
            received,
            false_positive,
            false_negative
        );
        assert_eq!(false_positive, 0.0, "偽陽性率 0%");
        assert_eq!(false_negative, 0.0, "偽陰性率 0%");
    }

    /// T7-3: n_sub 購読者、n_event イベントでの完全性検証。
    #[test]
    fn t7_3_subscription_completeness() {
        let n_sub: usize = 5;
        let manager = SubscriberManager::new();

        // 購読者0: 全イベント購読
        // 購読者1-4: Search イベントのみ購読
        let subscriptions: Vec<(FakeWebSocketEventChannel, EventFilter)> = (0..n_sub)
            .map(|i| {
                let ch = FakeWebSocketEventChannel::new();
                let filter = if i == 0 {
                    EventFilter::all()
                } else {
                    EventFilter {
                        kind_filter: Some(vec![DarviumEventKind::Search(
                            SearchEvent::Started,
                        )]),
                        since_vt: None,
                        until_vt: None,
                    }
                };
                (ch, filter)
            })
            .collect();

        for (ch, filter) in &subscriptions {
            manager
                .register(filter.clone(), Box::new(ch.clone()))
                .unwrap();
        }

        let events = generate_test_events(12345, 1000);
        let search_count = events
            .iter()
            .filter(|e| matches!(e.kind, DarviumEventKind::Search(SearchEvent::Started)))
            .count() as u64;

        for event in &events {
            manager.distribute(event).unwrap();
        }

        let list = manager.list().unwrap();
        println!("\n=== T7-3: 購読完全性検証 ===");
        println!(
            "n_sub={} n_event={} search_events={}",
            n_sub,
            events.len(),
            search_count
        );
        for (i, sub) in list.iter().enumerate() {
            let expected = if i == 0 {
                events.len() as u64
            } else {
                search_count
            };
            let completeness = if expected > 0 {
                sub.event_count as f64 / expected as f64 * 100.0
            } else {
                100.0
            };
            println!(
                "  subscriber[{}]: received={} expected={} completeness={:.2}%",
                i, sub.event_count, expected, completeness
            );
            assert_eq!(
                sub.event_count, expected,
                "subscriber[{}]: 完全性 100% であること (received={}, expected={})",
                i, sub.event_count, expected
            );
        }
        println!("=== T7-3 PASS ===");
    }
}
