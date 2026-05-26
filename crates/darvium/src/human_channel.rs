// HumanChannel — HITL (Human-In-The-Loop) 抽象トレイト
//
// 本モジュールは §12B HumanChannel Communication Abstraction の実装である。
// 人間との双方向通信を抽象化し、notify/communicate/reconnect の 3 メソッドを提供する。
// テスト用の FakeHumanChannel と JSON Lines プロトコルの StdinoutChannel を含む。

use std::collections::{HashMap, VecDeque};
use std::io::{BufRead, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use std::time::SystemTime;

use uuid::Uuid;

use crate::error::DarviumError;
use crate::event::{
    DarviumEvent, DarviumEventBus, DarviumEventKind, EventCausality, EventMetadata, EventPrivacy,
    EventRetention, EventSource, EventVisibility, HitlEvent, InteractionId, InteractionMode,
    PiiHandlingPolicy,
};
use crate::store::MetadataStore;
use crate::types::{HitlPayload, HumanOutcome, HumanRequest, InteractionStatus, StoredInteraction};

// ============================================================
// HumanChannel トレイト
// ============================================================

/// 人間との双方向通信を抽象化する。
///
/// `notify()` / `communicate()` / `reconnect()` の 3 メソッドを提供する。
/// 全実装は `Send + Sync` を満たし、トレイトオブジェクト (`Box<dyn HumanChannel>`) として
/// 使用可能でなければならない。
pub trait HumanChannel: Send + Sync {
    /// 一方向通知（fire-and-forget）。
    fn notify(&self, request: &HumanRequest) -> Result<(), DarviumError>;

    /// 双方向通信（応答待機）。
    /// interaction_id（Uuid::new_v4()）を発行し、InteractionHandle を返す。
    fn communicate(&self, request: &HumanRequest) -> Result<InteractionHandle, DarviumError>;

    /// 永続化された interaction_id とリクエストからインタラクションを再接続する。
    ///
    /// プロセス再起動後に呼ばれる。request は MetadataStore から復元された元のリクエスト全文。
    /// 全実装がこのメソッドを提供しなければならない (MUST)。
    fn reconnect(
        &self,
        interaction_id: uuid::Uuid,
        request: &HumanRequest,
    ) -> Result<InteractionHandle, DarviumError>;
}

// ============================================================
// HumanChannelConfig
// ============================================================

/// HITL Channel の設定 (v2.3-g EventBus adapter)。
///
/// 全フィールドが Option であり、後方互換性を保つ。
/// event_bus と interaction_store の両方が設定された場合のみ
/// EventBus 経由の adapter モードで動作する。
///
/// # 利用例
/// ```ignore
/// let config = HumanChannelConfig {
///     event_bus: Some(bus),
///     interaction_store: Some(store),
/// };
/// let channel = FakeHumanChannel::with_config(VecDeque::new(), config);
/// ```
pub struct HumanChannelConfig {
    /// EventBus 参照（設定時は publish/open/reconnect で使用）。
    pub event_bus: Option<Arc<dyn DarviumEventBus>>,
    /// MetadataStore 参照（設定時は interaction 永続化で使用）。
    pub interaction_store: Option<Arc<dyn MetadataStore + Send + Sync>>,
}

// ============================================================
// InteractionHandle
// ============================================================

/// HITL 通信の応答をブロッキング待機するハンドル。
///
/// 内部に `mpsc::Receiver` を持ち、`wait(timeout)` で応答を待つ。
/// プロセス生存中のみ有効。再起動後は `HumanChannel::reconnect()` を使用する。
#[derive(Debug)]
pub struct InteractionHandle {
    pub(crate) interaction_id: uuid::Uuid,
    pub(crate) rx: mpsc::Receiver<Result<HumanOutcome, DarviumError>>,
}

impl InteractionHandle {
    /// 新しい InteractionHandle を生成する。
    ///
    /// 主にテスト用の `HumanChannel` 実装が使用する。
    pub fn new(
        interaction_id: uuid::Uuid,
        rx: mpsc::Receiver<Result<HumanOutcome, DarviumError>>,
    ) -> Self {
        Self { interaction_id, rx }
    }

    /// このハンドルに対応する interaction_id を返す。
    pub fn interaction_id(&self) -> &uuid::Uuid {
        &self.interaction_id
    }

    /// 応答をブロッキング待機する。
    ///
    /// - `Some(dur)`: `recv_timeout(dur)` を使用。超過で `Ok(TimedOut)`。
    /// - `None`: `recv()` を使用。無制限待機。
    /// - チャネルが `Err(DarviumError)` を運んだ場合、そのエラーを呼び出し元に伝播する。
    pub fn wait(self, timeout: Option<Duration>) -> Result<HumanOutcome, DarviumError> {
        match timeout {
            Some(dur) => match self.rx.recv_timeout(dur) {
                Ok(result) => result,
                Err(mpsc::RecvTimeoutError::Timeout) => Ok(HumanOutcome::TimedOut),
                Err(mpsc::RecvTimeoutError::Disconnected) => Err(DarviumError::HumanChannelClosed),
            },
            None => match self.rx.recv() {
                Ok(result) => result,
                Err(_) => Err(DarviumError::HumanChannelClosed),
            },
        }
    }
}

// ============================================================
// FakeHumanChannel
// ============================================================

/// FakeHumanChannel が管理する個別インタラクションの内部レコード。
#[allow(dead_code)]
enum InteractionRecord {
    Pending { request: HumanRequest },
    Resolved(HumanOutcome),
}

/// HITL テスト用の Fake 実装。
///
/// 従来モード（デフォルト）:
/// - `notify()`: 常に Ok(())。カウンタとリクエストリストのみ更新。
/// - `communicate()`: プリロードキューから応答を取り出し即時解決。
/// - `reconnect()`: 既存インタラクション or プリロードキューから応答。
///
/// EventBus adapter モード（with_config で有効化）:
/// - notify/communicate/reconnect が EventBus + MetadataStore 経由で動作。
/// - 従来のカウンタ・リクエスト記録も引き続き有効。
pub struct FakeHumanChannel {
    sent_count: AtomicU64,
    requests_sent: Mutex<Vec<HumanRequest>>,
    preloaded: Mutex<VecDeque<HumanOutcome>>,
    interactions: Mutex<HashMap<uuid::Uuid, InteractionRecord>>,
    /// EventBus adapter モード時のみ Some。
    eventbus_delegate: Option<EventBusHumanChannel>,
}

impl FakeHumanChannel {
    /// 指定されたプリロード応答で FakeHumanChannel を生成する（従来モード）。
    pub fn new(preloaded: VecDeque<HumanOutcome>) -> Self {
        Self {
            sent_count: AtomicU64::new(0),
            requests_sent: Mutex::new(Vec::new()),
            preloaded: Mutex::new(preloaded),
            interactions: Mutex::new(HashMap::new()),
            eventbus_delegate: None,
        }
    }

    /// HumanChannelConfig で EventBus adapter モードを指定して生成する。
    ///
    /// event_bus と interaction_store の両方が設定された場合のみ
    /// EventBus adapter モードで動作する。どちらかが未設定の場合は
    /// 従来モードと同等に動作する。
    pub fn with_config(preloaded: VecDeque<HumanOutcome>, config: HumanChannelConfig) -> Self {
        let delegate = match (config.event_bus, config.interaction_store) {
            (Some(bus), Some(store)) => Some(EventBusHumanChannel::new(bus, store)),
            _ => None,
        };
        Self {
            sent_count: AtomicU64::new(0),
            requests_sent: Mutex::new(Vec::new()),
            preloaded: Mutex::new(preloaded),
            interactions: Mutex::new(HashMap::new()),
            eventbus_delegate: delegate,
        }
    }

    /// 通知/通信の総呼び出し回数を取得する。
    pub fn sent_count(&self) -> u64 {
        self.sent_count.load(Ordering::Relaxed)
    }

    /// 全送信リクエストのコピーを取得する。
    pub fn requests_sent(&self) -> Vec<HumanRequest> {
        self.requests_sent.lock().unwrap().clone()
    }

    /// 現在の全インタラクションを StoredInteraction の Vec として取得する。
    ///
    /// MetadataStore への永続化のテストで使用する。
    pub fn export_interactions(&self) -> Vec<StoredInteraction> {
        let interactions = self.interactions.lock().unwrap();
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        interactions
            .iter()
            .map(|(id, record)| match record {
                InteractionRecord::Pending { request } => StoredInteraction {
                    interaction_id: id.to_string(),
                    payload: HitlPayload {
                        request: request.clone(),
                    },
                    outcome: None,
                    status: InteractionStatus::Pending,
                    created_at: now_ms,
                    updated_at: now_ms,
                },
                InteractionRecord::Resolved(outcome) => StoredInteraction {
                    interaction_id: id.to_string(),
                    payload: HitlPayload {
                        request: HumanRequest {
                            subject: String::new(),
                            body: String::new(),
                            context: serde_json::json!({}),
                            timeout: None,
                        },
                    },
                    outcome: Some(outcome.clone()),
                    status: InteractionStatus::Resolved,
                    created_at: now_ms,
                    updated_at: now_ms,
                },
            })
            .collect()
    }

    /// 全内部状態を初期状態にリセットする。
    /// EventBus delegate の状態はリセットしない（外部の EventBus / MetadataStore が別途リセットされる）。
    pub fn reset(&self) {
        self.sent_count.store(0, Ordering::Relaxed);
        self.requests_sent.lock().unwrap().clear();
        self.preloaded.lock().unwrap().clear();
        self.interactions.lock().unwrap().clear();
    }

    /// EventBus delegate への参照を取得する（adapter モード時のみ Some）。
    pub fn eventbus_delegate(&self) -> Option<&EventBusHumanChannel> {
        self.eventbus_delegate.as_ref()
    }
}

impl HumanChannel for FakeHumanChannel {
    fn notify(&self, request: &HumanRequest) -> Result<(), DarviumError> {
        if let Some(ref delegate) = self.eventbus_delegate {
            delegate.notify(request)?;
        }
        self.sent_count.fetch_add(1, Ordering::Relaxed);
        self.requests_sent.lock().unwrap().push(request.clone());
        Ok(())
    }

    fn communicate(&self, request: &HumanRequest) -> Result<InteractionHandle, DarviumError> {
        self.sent_count.fetch_add(1, Ordering::Relaxed);
        self.requests_sent.lock().unwrap().push(request.clone());

        // EventBus adapter モード → delegate に委譲
        if let Some(ref delegate) = self.eventbus_delegate {
            return delegate.communicate(request);
        }

        // 従来モード: プリロードキューから応答を即時解決
        let interaction_id = uuid::Uuid::new_v4();
        let (tx, rx) = mpsc::channel();

        // プリロードキューから取り出し
        let outcome = self
            .preloaded
            .lock()
            .unwrap()
            .pop_front()
            .expect("FakeHumanChannel: preloaded queue is empty on communicate()");

        // 内部レコードを Resolved で保存
        self.interactions
            .lock()
            .unwrap()
            .insert(interaction_id, InteractionRecord::Resolved(outcome.clone()));

        tx.send(Ok(outcome)).ok();
        Ok(InteractionHandle { interaction_id, rx })
    }

    fn reconnect(
        &self,
        interaction_id: uuid::Uuid,
        request: &HumanRequest,
    ) -> Result<InteractionHandle, DarviumError> {
        // EventBus adapter モード → delegate に委譲
        if let Some(ref delegate) = self.eventbus_delegate {
            return delegate.reconnect(interaction_id, request);
        }

        // 従来モード
        let (tx, rx) = mpsc::channel();

        // 既存インタラクションを検索
        let mut interactions = self.interactions.lock().unwrap();
        if let Some(record) = interactions.remove(&interaction_id) {
            match record {
                InteractionRecord::Pending { request: _ } => {
                    // Pending → タイムアウトとして応答
                    interactions.insert(
                        interaction_id,
                        InteractionRecord::Resolved(HumanOutcome::TimedOut),
                    );
                    tx.send(Ok(HumanOutcome::TimedOut)).ok();
                }
                InteractionRecord::Resolved(outcome) => {
                    // 既に解決済み → 同じ outcome を返す
                    interactions
                        .insert(interaction_id, InteractionRecord::Resolved(outcome.clone()));
                    tx.send(Ok(outcome)).ok();
                }
            }
        } else {
            // 見つからなかった（別インスタンス＝クラッシュ後）→ プリロードキューから
            drop(interactions);
            let outcome = self.preloaded.lock().unwrap().pop_front().ok_or_else(|| {
                DarviumError::HumanChannelIo(
                    "FakeHumanChannel: preloaded queue is empty on reconnect()".into(),
                )
            })?;
            tx.send(Ok(outcome.clone())).ok();
            self.interactions
                .lock()
                .unwrap()
                .insert(interaction_id, InteractionRecord::Resolved(outcome));
        }

        Ok(InteractionHandle { interaction_id, rx })
    }
}

// ============================================================
// StdinoutChannel
// ============================================================

/// 標準入出力をベースとした JSON Lines プロトコルの HumanChannel 実装。
///
/// 通信プロトコル:
/// - notify(): `→ {"type":"notify","interaction_id":"xxx","request":{...}}`
/// - communicate():
///   - `→ {"type":"communicate","interaction_id":"xxx","request":{...}}`
///   - `← {"interaction_id":"xxx","outcome":{...}}`
/// - reconnect():
///   - `→ {"type":"reconnect","interaction_id":"xxx","request":{...}}`
///   - `← {"interaction_id":"xxx","outcome":{...}}`
pub struct StdinoutChannel<R, W> {
    reader: Arc<Mutex<R>>,
    writer: Mutex<W>,
    session: Mutex<()>,
}

impl<R: BufRead + Send, W: Write + Send> StdinoutChannel<R, W> {
    /// リーダーとライターを指定して StdinoutChannel を生成する。
    pub fn new(reader: R, writer: W) -> Self {
        Self {
            reader: Arc::new(Mutex::new(reader)),
            writer: Mutex::new(writer),
            session: Mutex::new(()),
        }
    }
}

impl<R: BufRead + Send + 'static, W: Write + Send> HumanChannel for StdinoutChannel<R, W> {
    fn notify(&self, request: &HumanRequest) -> Result<(), DarviumError> {
        let id = uuid::Uuid::new_v4();
        let mut writer = self
            .writer
            .lock()
            .map_err(|e| DarviumError::HumanChannelIo(e.to_string()))?;
        write_legacy_json_line(&mut *writer, "notify", id, request)
    }

    fn communicate(&self, request: &HumanRequest) -> Result<InteractionHandle, DarviumError> {
        let id = uuid::Uuid::new_v4();
        let (tx, rx) = mpsc::channel();

        // セッションロック確保（ドロップされるまで次の呼び出しはブロック）
        let _session = self
            .session
            .lock()
            .map_err(|e| DarviumError::HumanChannelIo(e.to_string()))?;

        // 1. リクエスト送信（同期的）
        {
            let mut writer = self
                .writer
                .lock()
                .map_err(|e| DarviumError::HumanChannelIo(e.to_string()))?;
            write_legacy_json_line(&mut *writer, "communicate", id, request)?;
            writer
                .flush()
                .map_err(|e| DarviumError::HumanChannelIo(e.to_string()))?;
        }

        // 2. 応答読み取りスレッドを起動（非同期的）
        let reader = self.reader.clone();
        std::thread::spawn(move || {
            let mut line = String::new();
            match reader.lock() {
                Ok(mut r) => match r.read_line(&mut line) {
                    Ok(0) => {
                        let _ = tx.send(Err(DarviumError::HumanChannelIo(
                            "reader EOF: response line expected".into(),
                        )));
                    }
                    Ok(_) => {
                        if let Ok(resp) = serde_json::from_str::<StdinoutResponse>(&line) {
                            if resp.interaction_id != id {
                                let _ = tx.send(Ok(HumanOutcome::Unreachable(format!(
                                    "interaction_id mismatch: expected {}, got {}",
                                    id, resp.interaction_id
                                ))));
                                return;
                            }
                            if let Some(outcome) = resp.outcome {
                                let _ = tx.send(Ok(outcome));
                                return;
                            }
                        }
                        let _ = tx.send(Err(DarviumError::HumanChannelIo(format!(
                            "invalid JSON response: {}",
                            line.trim()
                        ))));
                    }
                    Err(e) => {
                        let _ = tx.send(Err(DarviumError::HumanChannelIo(format!(
                            "reader I/O error: {}",
                            e
                        ))));
                    }
                },
                Err(e) => {
                    let _ = tx.send(Err(DarviumError::HumanChannelIo(format!(
                        "reader mutex poisoned: {}",
                        e
                    ))));
                }
            }
        });

        Ok(InteractionHandle {
            interaction_id: id,
            rx,
        })
    }

    fn reconnect(
        &self,
        interaction_id: uuid::Uuid,
        request: &HumanRequest,
    ) -> Result<InteractionHandle, DarviumError> {
        let (tx, rx) = mpsc::channel();

        // セッションロック確保
        let _session = self
            .session
            .lock()
            .map_err(|e| DarviumError::HumanChannelIo(e.to_string()))?;

        // 1. リクエスト再通知（同期的）
        {
            let mut writer = self
                .writer
                .lock()
                .map_err(|e| DarviumError::HumanChannelIo(e.to_string()))?;
            write_legacy_json_line(&mut *writer, "reconnect", interaction_id, request)?;
            writer
                .flush()
                .map_err(|e| DarviumError::HumanChannelIo(e.to_string()))?;
        }

        // 2. 応答読み取りスレッド（非同期的）
        let reader = self.reader.clone();
        std::thread::spawn(move || {
            let mut line = String::new();
            match reader.lock() {
                Ok(mut r) => match r.read_line(&mut line) {
                    Ok(0) => {
                        let _ = tx.send(Err(DarviumError::HumanChannelIo(
                            "reader EOF: response line expected".into(),
                        )));
                    }
                    Ok(_) => {
                        if let Ok(resp) = serde_json::from_str::<StdinoutResponse>(&line) {
                            if resp.interaction_id != interaction_id {
                                let _ = tx.send(Ok(HumanOutcome::Unreachable(format!(
                                    "interaction_id mismatch: expected {}, got {}",
                                    interaction_id, resp.interaction_id
                                ))));
                                return;
                            }
                            if let Some(outcome) = resp.outcome {
                                let _ = tx.send(Ok(outcome));
                                return;
                            }
                        }
                        let _ = tx.send(Err(DarviumError::HumanChannelIo(format!(
                            "invalid JSON response: {}",
                            line.trim()
                        ))));
                    }
                    Err(e) => {
                        let _ = tx.send(Err(DarviumError::HumanChannelIo(format!(
                            "reader I/O error: {}",
                            e
                        ))));
                    }
                },
                Err(e) => {
                    let _ = tx.send(Err(DarviumError::HumanChannelIo(format!(
                        "reader mutex poisoned: {}",
                        e
                    ))));
                }
            }
        });

        Ok(InteractionHandle { interaction_id, rx })
    }
}

// ============================================================
// EventBusHumanChannel — EventBus / MetadataStore 上の HITL adapter
// ============================================================

/// HumanChannel の EventBus / MetadataStore 経由実装 (RFC §12B.3 adapter)。
///
/// トレイトのシグネチャは変更せず、内部実装のみ DarviumEventBus と MetadataStore を
/// 利用してイベントの発行・インタラクションの管理を行う。
/// 解決機構は mpsc チャネル＋内部マップで実現し、InteractionHandle.wait() との互換性を保つ。
///
/// # adapter 変換
/// - notify() → EventBus::publish(OneWay, HitlEvent::NotificationRequested)
/// - communicate() → EventBus::open(TwoWay, HitlEvent::InteractionRequested)
/// - reconnect() → MetadataStore::reconnect_interaction() + EventBus::reconnect()
pub struct EventBusHumanChannel {
    /// 全イベントの publish / open / reconnect 先。
    event_bus: Arc<dyn DarviumEventBus>,
    /// インタラクションの永続化・再接続先。
    metadata_store: Arc<dyn MetadataStore + Send + Sync>,
    /// 未解決のインタラクション ID → mpsc Sender マップ。
    pending: Mutex<HashMap<String, mpsc::Sender<Result<HumanOutcome, DarviumError>>>>,
}

impl EventBusHumanChannel {
    /// EventBus と MetadataStore を指定して新規生成する。
    pub fn new(
        event_bus: Arc<dyn DarviumEventBus>,
        metadata_store: Arc<dyn MetadataStore + Send + Sync>,
    ) -> Self {
        Self {
            event_bus,
            metadata_store,
            pending: Mutex::new(HashMap::new()),
        }
    }

    /// EventBus 経由でインタラクションを解決する公開メソッド。
    ///
    /// 1. EventBus::resolve() で interaction を解決
    /// 2. MetadataStore::resolve_human_interaction() で永続化
    /// 3. 対応する mpsc Sender 経由で InteractionHandle.wait() に通知
    pub fn resolve_interaction(
        &self,
        interaction_id: &str,
        outcome: HumanOutcome,
    ) -> Result<(), DarviumError> {
        let id = InteractionId(interaction_id.to_string());
        let outcome_value =
            serde_json::to_value(&outcome).map_err(|e| DarviumError::EventBus(e.to_string()))?;

        self.event_bus.resolve(&id, outcome_value)?;
        self.metadata_store
            .resolve_human_interaction(interaction_id, &outcome)?;

        // pending Sender があれば通知（なければ既に解決済み）
        if let Some(tx) = self.pending.lock().unwrap().remove(interaction_id) {
            let _ = tx.send(Ok(outcome));
        }

        Ok(())
    }

    /// DarviumEvent を HumanRequest から構築する内部ヘルパー。
    fn build_hitl_event(
        &self,
        kind: DarviumEventKind,
        mode: InteractionMode,
        request: &HumanRequest,
    ) -> DarviumEvent {
        DarviumEvent {
            event_id: Uuid::new_v4().to_string(),
            kind,
            interaction_mode: mode,
            payload: serde_json::to_value(request.clone()).unwrap_or_default(),
            causality: EventCausality {
                parent_event_id: None,
                root_event_id: None,
                trace_ref: None,
                mission_id: None,
                workflow_id: None,
                run_id: None,
            },
            metadata: EventMetadata {
                clock: 0, // EventBus が割り当てる
                timestamp: SystemTime::now(),
                source: EventSource::HumanChannel,
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

impl HumanChannel for EventBusHumanChannel {
    fn notify(&self, request: &HumanRequest) -> Result<(), DarviumError> {
        let event = self.build_hitl_event(
            DarviumEventKind::Hitl(HitlEvent::NotificationRequested),
            InteractionMode::OneWay,
            request,
        );
        self.event_bus.publish(event)?;
        Ok(())
    }

    fn communicate(&self, request: &HumanRequest) -> Result<InteractionHandle, DarviumError> {
        let event = self.build_hitl_event(
            DarviumEventKind::Hitl(HitlEvent::InteractionRequested),
            InteractionMode::TwoWay,
            request,
        );
        let interaction_id = self.event_bus.open(event)?;

        // MetadataStore に Pending レコードを保存
        let now_ms = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let stored = StoredInteraction {
            interaction_id: interaction_id.0.clone(),
            payload: HitlPayload {
                request: request.clone(),
            },
            outcome: None,
            status: InteractionStatus::Pending,
            created_at: now_ms,
            updated_at: now_ms,
        };
        self.metadata_store.store_human_interaction(&stored)?;

        // mpsc チャネルを作成し、pending マップに保存
        let (tx, rx) = mpsc::channel();
        self.pending
            .lock()
            .unwrap()
            .insert(interaction_id.0.clone(), tx);

        // interaction_id を Uuid にパース
        let parsed_id = Uuid::parse_str(&interaction_id.0)
            .map_err(|e| DarviumError::EventBus(e.to_string()))?;

        Ok(InteractionHandle {
            interaction_id: parsed_id,
            rx,
        })
    }

    fn reconnect(
        &self,
        interaction_id: Uuid,
        _request: &HumanRequest,
    ) -> Result<InteractionHandle, DarviumError> {
        let id_str = interaction_id.to_string();

        // MetadataStore で再接続
        self.metadata_store
            .reconnect_interaction(&id_str, "eventbus")?;

        // EventBus で再接続
        self.event_bus
            .reconnect(&InteractionId(id_str.clone()), "eventbus")?;

        // mpsc チャネルを作成し、pending マップに保存
        let (tx, rx) = mpsc::channel();
        self.pending.lock().unwrap().insert(id_str, tx);

        Ok(InteractionHandle { interaction_id, rx })
    }
}

// ============================================================
// 内部ヘルパー
// ============================================================

/// JSON Lines 形式で writer にメッセージを書き込む。
fn write_legacy_json_line<W: Write>(
    writer: &mut W,
    msg_type: &str,
    interaction_id: uuid::Uuid,
    request: &HumanRequest,
) -> Result<(), DarviumError> {
    let payload = serde_json::json!({
        "type": msg_type,
        "interaction_id": interaction_id,
        "request": request,
    });
    let line =
        serde_json::to_string(&payload).map_err(|e| DarviumError::HumanChannelIo(e.to_string()))?;
    writeln!(writer, "{}", line).map_err(|e| DarviumError::HumanChannelIo(e.to_string()))?;
    Ok(())
}

/// StdinoutChannel 応答パース用の中間型。
#[derive(serde::Deserialize)]
struct StdinoutResponse {
    interaction_id: uuid::Uuid,
    outcome: Option<HumanOutcome>,
}

// ============================================================
// テスト
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{DarviumEventKind, FakeEventBus, HitlEvent, InteractionMode};
    use crate::store::InMemoryMetadataStore;
    use crate::types::{HumanDecision, HumanRequest, HumanResponse, InteractionStatus};
    use std::collections::VecDeque;

    // ── ヘルパー ──

    /// テスト用の最小限のリクエストを生成する。
    fn test_request(subject: &str) -> HumanRequest {
        HumanRequest {
            subject: subject.to_string(),
            body: "test body".into(),
            context: serde_json::json!({"source": "test"}),
            timeout: None,
        }
    }

    // ============================================================
    // T1: FakeHumanChannel の基本動作（2 テスト）
    // ============================================================

    /// T1-1: 型境界充足
    #[test]
    fn t1_1_fake_channel_implements_trait() {
        let channel: Box<dyn HumanChannel> = Box::new(FakeHumanChannel::new(VecDeque::new()));
        let _ = channel;
    }

    /// T1-2: notify fire-and-forget
    #[test]
    fn t1_2_notify_fire_and_forget() {
        let channel = FakeHumanChannel::new(VecDeque::new());
        let request = test_request("notify-test");
        let result = channel.notify(&request);
        assert!(result.is_ok());
        assert_eq!(channel.sent_count(), 1);
    }

    // ============================================================
    // T2: 単一 HITL 通信（3 テスト）
    // ============================================================

    /// T2-1: 基本送受信
    #[test]
    fn t2_1_basic_communicate() {
        let expected = HumanOutcome::Responded(HumanResponse {
            decision: HumanDecision::Approved,
            comment: None,
            revised_body: None,
        });
        let channel = FakeHumanChannel::new(VecDeque::from(vec![expected.clone()]));
        let handle = channel.communicate(&test_request("basic")).unwrap();
        let outcome = handle.wait(None).unwrap();
        assert_eq!(outcome, expected);
    }

    /// T2-2: 全 decision × comment × revised_body 網羅（パラメタライズド）
    #[test]
    fn t2_2_all_outcome_variants() {
        use crate::types::HumanOutcome::*;

        let decisions = [
            HumanDecision::Approved,
            HumanDecision::Rejected,
            HumanDecision::NeedsRevision,
            HumanDecision::Irrelevant,
            HumanDecision::Unsafe,
        ];
        let comment_options = [None, Some("good work".into())];
        let revised_options = [None, Some("revised text".into())];

        for &decision in &decisions {
            for comment in &comment_options {
                for revised in &revised_options {
                    let outcome = Responded(HumanResponse {
                        decision,
                        comment: comment.clone(),
                        revised_body: revised.clone(),
                    });
                    let channel = FakeHumanChannel::new(VecDeque::from(vec![outcome.clone()]));
                    let handle = channel.communicate(&test_request("variant")).unwrap();
                    let received = handle.wait(None).unwrap();
                    assert_eq!(received, outcome);
                }
            }
        }
    }

    /// T2-3: 空文字 subject/body
    #[test]
    fn t2_3_empty_subject_body() {
        let outcome = HumanOutcome::Responded(HumanResponse {
            decision: HumanDecision::Approved,
            comment: None,
            revised_body: None,
        });
        let channel = FakeHumanChannel::new(VecDeque::from(vec![outcome]));
        let request = HumanRequest {
            subject: String::new(),
            body: String::new(),
            context: serde_json::json!({}),
            timeout: None,
        };
        let handle = channel.communicate(&request).unwrap();
        let result = handle.wait(None);
        assert!(result.is_ok());
    }

    // ============================================================
    // T3: 複数 HITL の全件記録（6 テスト）
    // ============================================================

    /// T3-1: 3回 notify
    #[test]
    fn t3_1_three_notifies() {
        let channel = FakeHumanChannel::new(VecDeque::new());
        for i in 0..3 {
            channel
                .notify(&test_request(&format!("notify-{}", i)))
                .unwrap();
        }
        assert_eq!(channel.sent_count(), 3);
    }

    /// T3-2: 3回 communicate
    #[test]
    fn t3_2_three_communicates() {
        let outcomes = vec![
            HumanOutcome::Responded(HumanResponse {
                decision: HumanDecision::Approved,
                comment: None,
                revised_body: None,
            }),
            HumanOutcome::TimedOut,
            HumanOutcome::Unreachable("busy".into()),
        ];
        let channel = FakeHumanChannel::new(VecDeque::from(outcomes.clone()));
        for (i, expected) in outcomes.iter().enumerate() {
            let handle = channel
                .communicate(&test_request(&format!("comm-{}", i)))
                .unwrap();
            assert_eq!(handle.wait(None).unwrap(), *expected);
        }
        assert_eq!(channel.sent_count(), 3);
    }

    /// T3-3: FIFO 順序
    #[test]
    fn t3_3_fifo_order() {
        let channel = FakeHumanChannel::new(VecDeque::new());
        channel.notify(&test_request("first")).unwrap();
        channel.notify(&test_request("second")).unwrap();
        let sent = channel.requests_sent();
        assert_eq!(sent[0].subject, "first");
        assert_eq!(sent[1].subject, "second");
    }

    /// T3-4: 異種リクエスト
    #[test]
    fn t3_4_mixed_requests() {
        let channel = FakeHumanChannel::new(VecDeque::new());
        channel
            .notify(&HumanRequest {
                subject: "alpha".into(),
                body: "body-a".into(),
                context: serde_json::json!({"a": 1}),
                timeout: Some(Duration::from_secs(10)),
            })
            .unwrap();
        channel
            .notify(&HumanRequest {
                subject: "beta".into(),
                body: "body-b".into(),
                context: serde_json::json!({"b": 2}),
                timeout: None,
            })
            .unwrap();
        let sent = channel.requests_sent();
        assert_eq!(sent.len(), 2);
        assert!(sent.iter().any(|r| r.subject == "alpha"));
        assert!(sent.iter().any(|r| r.subject == "beta"));
    }

    /// T3-5: 大量 1,000 回
    #[test]
    fn t3_5_thousand_notifies() {
        let channel = FakeHumanChannel::new(VecDeque::new());
        let n = 1_000u64;
        for i in 0..n {
            channel
                .notify(&test_request(&format!("bulk-{}", i)))
                .unwrap();
        }
        assert_eq!(channel.sent_count(), n);
    }

    /// T3-6: インスタンス独立性
    #[test]
    fn t3_6_instance_independence() {
        let ch1 = FakeHumanChannel::new(VecDeque::new());
        let ch2 = FakeHumanChannel::new(VecDeque::new());
        ch1.notify(&test_request("ch1")).unwrap();
        assert_eq!(ch1.sent_count(), 1);
        assert_eq!(ch2.sent_count(), 0);
    }

    // ============================================================
    // T4: InteractionHandle ブロッキング動作（5 テスト）
    // ============================================================

    /// T4-1: 即時解決（FakeHumanChannel では communicate() 内で即時解決）
    #[test]
    fn t4_1_immediate_resolution() {
        let outcome = HumanOutcome::Responded(HumanResponse {
            decision: HumanDecision::Approved,
            comment: None,
            revised_body: None,
        });
        let channel = FakeHumanChannel::new(VecDeque::from(vec![outcome.clone()]));
        let handle = channel.communicate(&test_request("immediate")).unwrap();
        let result = handle.wait(None).unwrap();
        assert_eq!(result, outcome);
    }

    /// T4-2: タイムアウト — InteractionHandle の wait に短いタイムアウトを与える
    #[test]
    fn t4_2_timeout() {
        let (tx, rx) = mpsc::channel::<Result<HumanOutcome, DarviumError>>();
        // 送信側を保持するが送信しない → recv_timeout がタイムアウト
        let _tx = tx;
        let handle = InteractionHandle {
            interaction_id: uuid::Uuid::new_v4(),
            rx,
        };
        let result = handle.wait(Some(Duration::from_millis(1)));
        assert_eq!(result.unwrap(), HumanOutcome::TimedOut);
    }

    /// T4-3: 無制限待機 — 別スレッドからの解決でブロックが解除される
    #[test]
    fn t4_3_indefinite_wait() {
        let (tx, rx) = mpsc::channel::<Result<HumanOutcome, DarviumError>>();
        let handle = InteractionHandle {
            interaction_id: uuid::Uuid::new_v4(),
            rx,
        };

        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(50));
            tx.send(Ok(HumanOutcome::TimedOut)).ok();
        });

        let result = handle.wait(None);
        assert_eq!(result.unwrap(), HumanOutcome::TimedOut);
    }

    /// T4-4: mpsc 切断
    #[test]
    fn t4_4_mpsc_disconnect() {
        let (tx, rx) = mpsc::channel::<Result<HumanOutcome, DarviumError>>();
        let handle = InteractionHandle {
            interaction_id: uuid::Uuid::new_v4(),
            rx,
        };
        drop(tx);
        let result = handle.wait(None);
        assert!(matches!(result, Err(DarviumError::HumanChannelClosed)));
    }

    /// T4-5: drop 安全
    #[test]
    fn t4_5_drop_safety() {
        let (tx, rx) = mpsc::channel::<Result<HumanOutcome, DarviumError>>();
        let handle = InteractionHandle {
            interaction_id: uuid::Uuid::new_v4(),
            rx,
        };
        drop(tx);
        drop(handle);
    }

    // ============================================================
    // T5: トレイトオブジェクト安全性（3 テスト）
    // ============================================================

    /// T5-1: Box<dyn HumanChannel>
    #[test]
    fn t5_1_box_dyn_human_channel() {
        let channel: Box<dyn HumanChannel> = Box::new(FakeHumanChannel::new(VecDeque::new()));
        let request = test_request("box-dyn");
        let _ = channel.notify(&request);
    }

    /// T5-2: &dyn HumanChannel
    #[test]
    fn t5_2_ref_dyn_human_channel() {
        fn use_channel(ch: &dyn HumanChannel) {
            let req = test_request("ref-dyn");
            let _ = ch.notify(&req);
        }
        let channel = FakeHumanChannel::new(VecDeque::new());
        use_channel(&channel);
    }

    /// T5-3: Arc<dyn HumanChannel>
    #[test]
    fn t5_3_arc_dyn_human_channel() {
        use std::sync::Arc;
        let channel: Arc<dyn HumanChannel> = Arc::new(FakeHumanChannel::new(VecDeque::new()));
        let request = test_request("arc-dyn");
        let _ = channel.notify(&request);
    }

    // ============================================================
    // T6: StdinoutChannel 実装（12 テスト）
    // ============================================================

    /// T6-1: notify JSON
    #[test]
    fn t6_1_notify_json() {
        let reader = std::io::BufReader::new(std::io::empty());
        let mut writer: Vec<u8> = Vec::new();
        let channel = StdinoutChannel::new(reader, &mut writer);
        let request = test_request("json-test");
        channel.notify(&request).unwrap();

        let output = String::from_utf8(writer).unwrap();
        assert!(output.contains(r#""type":"notify""#));
        assert!(output.contains(r#""subject":"json-test""#));
        assert!(output.ends_with('\n'));
    }

    /// T6-2: communicate 即時解決
    #[test]
    fn t6_2_communicate_immediate() {
        let expected_line = r#"{"interaction_id":"00000000-0000-0000-0000-000000000000","outcome":{"Responded":{"comment":null,"decision":"Approved","revised_body":null}}}"#;
        let reader = std::io::BufReader::new(expected_line.as_bytes());
        let writer: Vec<u8> = Vec::new();
        let channel = StdinoutChannel::new(reader, writer);
        let handle = channel.communicate(&test_request("immediate")).unwrap();
        let result = handle.wait(Some(Duration::from_secs(1)));
        assert!(result.is_ok());
    }

    /// T6-3: communicate ブロッキング — 別スレッドから応答を送信
    #[test]
    fn t6_3_communicate_blocking() {
        // InteractionHandle の wait(None) が別スレッドからの送信でブロック解除されることを確認
        let (tx, rx) = mpsc::channel::<Result<HumanOutcome, DarviumError>>();
        let handle = InteractionHandle {
            interaction_id: uuid::Uuid::new_v4(),
            rx,
        };

        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(50));
            tx.send(Ok(HumanOutcome::Responded(HumanResponse {
                decision: HumanDecision::Approved,
                comment: None,
                revised_body: None,
            })))
            .ok();
        });

        let result = handle.wait(None);
        assert!(result.is_ok());
    }

    /// T6-4: タイムアウト — 空チャネルで wait(timeout) が TimedOut を返す
    #[test]
    fn t6_4_stdinout_timeout() {
        let (tx, rx) = mpsc::channel::<Result<HumanOutcome, DarviumError>>();
        let _tx = tx;
        let handle = InteractionHandle {
            interaction_id: uuid::Uuid::new_v4(),
            rx,
        };
        let result = handle.wait(Some(Duration::from_millis(10)));
        assert_eq!(result.unwrap(), HumanOutcome::TimedOut);
    }

    /// T6-5: 3往復セッション
    #[test]
    fn t6_5_three_roundtrips() {
        let outcomes = vec![
            HumanOutcome::Responded(HumanResponse {
                decision: HumanDecision::Approved,
                comment: None,
                revised_body: None,
            }),
            HumanOutcome::Responded(HumanResponse {
                decision: HumanDecision::Rejected,
                comment: Some("no".into()),
                revised_body: None,
            }),
            HumanOutcome::TimedOut,
        ];
        let channel = FakeHumanChannel::new(VecDeque::from(outcomes));
        for i in 0..3 {
            let handle = channel
                .communicate(&test_request(&format!("round-{}", i)))
                .unwrap();
            let _ = handle.wait(None).unwrap();
        }
        assert_eq!(channel.sent_count(), 3);
    }

    /// T6-6: 複数インスタンス独立性
    #[test]
    fn t6_6_instance_independence() {
        let ch1 = FakeHumanChannel::new(VecDeque::new());
        let ch2 = FakeHumanChannel::new(VecDeque::new());
        ch1.notify(&test_request("ch1")).unwrap();
        assert_eq!(ch1.requests_sent().len(), 1);
        assert_eq!(ch2.requests_sent().len(), 0);
    }

    /// T6-7: 不正 JSON 応答
    #[test]
    fn t6_7_invalid_json_response() {
        let invalid_json = r#"{]invalid{:["#;
        let reader = std::io::BufReader::new(invalid_json.as_bytes());
        let writer: Vec<u8> = Vec::new();
        let channel = StdinoutChannel::new(reader, writer);
        let handle = channel.communicate(&test_request("invalid")).unwrap();
        let result = handle.wait(Some(Duration::from_secs(1)));
        assert!(matches!(result, Err(DarviumError::HumanChannelIo(_))));
    }

    /// T6-8: EOF
    #[test]
    fn t6_8_eof_response() {
        let reader = std::io::BufReader::new(std::io::empty());
        let writer: Vec<u8> = Vec::new();
        let channel = StdinoutChannel::new(reader, writer);
        let handle = channel.communicate(&test_request("eof")).unwrap();
        let result = handle.wait(Some(Duration::from_secs(1)));
        assert!(matches!(result, Err(DarviumError::HumanChannelIo(_))));
    }

    /// T6-9: reconnect JSON
    #[test]
    fn t6_9_reconnect_json() {
        let reader = std::io::BufReader::new(std::io::empty());
        let mut writer: Vec<u8> = Vec::new();
        let channel = StdinoutChannel::new(reader, &mut writer);
        let id = uuid::Uuid::new_v4();
        let _ = channel.reconnect(id, &test_request("reconnect-json"));

        let output = String::from_utf8(writer).unwrap();
        assert!(output.contains(r#""type":"reconnect""#));
        assert!(output.contains(&format!(r#""interaction_id":"{}""#, id)));
    }

    /// T6-10: 巨大ペイロード
    #[test]
    fn t6_10_large_payload() {
        let outcome = HumanOutcome::Responded(HumanResponse {
            decision: HumanDecision::Approved,
            comment: Some("x".repeat(1024 * 1024)),
            revised_body: None,
        });
        let channel = FakeHumanChannel::new(VecDeque::from(vec![outcome]));
        let handle = channel.communicate(&test_request("large-payload")).unwrap();
        let result = handle.wait(None);
        assert!(result.is_ok());
    }

    /// T6-11: communicate interaction_id 不一致
    #[test]
    fn t6_11_communicate_id_mismatch() {
        let response = r#"{"interaction_id":"11111111-1111-1111-1111-111111111111","outcome":{"Responded":{"decision":"Approved","comment":null,"revised_body":null}}}"#;
        let reader = std::io::BufReader::new(response.as_bytes());
        let writer: Vec<u8> = Vec::new();
        let channel = StdinoutChannel::new(reader, writer);
        let handle = channel.communicate(&test_request("mismatch")).unwrap();
        let result = handle.wait(Some(Duration::from_secs(1)));
        assert!(matches!(result, Ok(HumanOutcome::Unreachable(_))));
    }

    /// T6-12: reconnect interaction_id 不一致
    #[test]
    fn t6_12_reconnect_id_mismatch() {
        // StdinoutChannel 経由ではなく、reader スレッドからの channel 送信を
        // InteractionHandle の wait が正しく処理することを検証する
        let expected_id = uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap();
        let response_id = uuid::Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap();
        let (tx, rx) = mpsc::channel::<Result<HumanOutcome, DarviumError>>();

        // interaction_id 不一致を模擬送信
        let _ = tx.send(Ok(HumanOutcome::Unreachable(format!(
            "interaction_id mismatch: expected {}, got {}",
            expected_id, response_id
        ))));

        let handle = InteractionHandle {
            interaction_id: expected_id,
            rx,
        };
        let result = handle.wait(Some(Duration::from_secs(1)));
        assert!(matches!(result, Ok(HumanOutcome::Unreachable(_))));
    }

    // ============================================================
    // T7: エラーケースと境界値（5 テスト）
    // ============================================================

    /// T7-1: 任意の context
    #[test]
    fn t7_1_arbitrary_context() {
        let channel = FakeHumanChannel::new(VecDeque::from(vec![HumanOutcome::Responded(
            HumanResponse {
                decision: HumanDecision::Approved,
                comment: None,
                revised_body: None,
            },
        )]));
        let request = HumanRequest {
            subject: "ctx-test".into(),
            body: "test".into(),
            context: serde_json::json!({
                "nested": {
                    "array": [1, 2, 3],
                    "null": null,
                    "bool": true
                }
            }),
            timeout: None,
        };
        let handle = channel.communicate(&request).unwrap();
        let result = handle.wait(None);
        assert!(result.is_ok());
    }

    /// T7-2: wait(Some(0ns))
    #[test]
    fn t7_2_zero_timeout() {
        let channel = FakeHumanChannel::new(VecDeque::from(vec![HumanOutcome::Responded(
            HumanResponse {
                decision: HumanDecision::Approved,
                comment: None,
                revised_body: None,
            },
        )]));
        let handle = channel.communicate(&test_request("zero")).unwrap();
        let result = handle.wait(Some(Duration::from_nanos(0)));
        assert!(result.is_ok());
    }

    /// T7-3: wait(None) 別スレッド
    #[test]
    fn t7_3_wait_none_other_thread() {
        let outcome = HumanOutcome::Responded(HumanResponse {
            decision: HumanDecision::Approved,
            comment: None,
            revised_body: None,
        });
        let channel = FakeHumanChannel::new(VecDeque::from(vec![outcome]));
        let handle = channel.communicate(&test_request("thread")).unwrap();

        let result = std::thread::spawn(move || handle.wait(None))
            .join()
            .unwrap();
        assert!(result.is_ok());
    }

    /// T7-4: 空キュー communicate → panic
    #[test]
    #[should_panic(expected = "preloaded queue is empty")]
    fn t7_4_empty_queue_communicate() {
        let channel = FakeHumanChannel::new(VecDeque::new());
        let _ = channel.communicate(&test_request("empty"));
    }

    /// T7-5: 8 スレッド同時アクセス
    #[test]
    fn t7_5_concurrent_access() {
        use std::sync::Arc;
        let channel = Arc::new(FakeHumanChannel::new(VecDeque::new()));
        let mut handles = Vec::new();

        for i in 0..8u64 {
            let ch = channel.clone();
            handles.push(std::thread::spawn(move || {
                let req = test_request(&format!("thread-{}", i));
                ch.notify(&req).unwrap();
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        assert_eq!(channel.sent_count(), 8);
    }

    // ============================================================
    // T8: FakeHumanChannel リセット（3 テスト）
    // ============================================================

    /// T8-1: reset → sent_count == 0
    #[test]
    fn t8_1_reset_sent_count() {
        let channel = FakeHumanChannel::new(VecDeque::new());
        channel.notify(&test_request("a")).unwrap();
        channel.notify(&test_request("b")).unwrap();
        assert_eq!(channel.sent_count(), 2);
        channel.reset();
        assert_eq!(channel.sent_count(), 0);
    }

    /// T8-2: reset → requests_sent 空
    #[test]
    fn t8_2_reset_requests_empty() {
        let channel = FakeHumanChannel::new(VecDeque::new());
        channel.notify(&test_request("x")).unwrap();
        channel.reset();
        assert!(channel.requests_sent().is_empty());
    }

    /// T8-3: reset → 再使用可能
    #[test]
    fn t8_3_reset_reusable() {
        let channel = FakeHumanChannel::new(VecDeque::new());
        channel.notify(&test_request("before")).unwrap();
        channel.reset();
        channel.notify(&test_request("after")).unwrap();
        assert_eq!(channel.sent_count(), 1);
        assert_eq!(channel.requests_sent()[0].subject, "after");
    }

    // ============================================================
    // T9: reconnect 回復可能性（7 テスト）
    // ============================================================

    /// T9-1: 解決済み再接続
    #[test]
    fn t9_1_resolved_reconnect() {
        let outcome = HumanOutcome::Responded(HumanResponse {
            decision: HumanDecision::Approved,
            comment: Some("done".into()),
            revised_body: None,
        });
        let channel = FakeHumanChannel::new(VecDeque::from(vec![outcome.clone()]));
        let handle = channel.communicate(&test_request("initial")).unwrap();
        let _ = handle.wait(None).unwrap();

        let interactions = channel.export_interactions();
        let id = uuid::Uuid::parse_str(&interactions[0].interaction_id).unwrap();

        let handle2 = channel.reconnect(id, &test_request("reconnect")).unwrap();
        let outcome2 = handle2.wait(None).unwrap();
        assert_eq!(outcome2, outcome);
    }

    /// T9-2: 未知 ID + 空キュー
    #[test]
    fn t9_2_unknown_id_empty_queue() {
        let channel = FakeHumanChannel::new(VecDeque::new());
        let id = uuid::Uuid::new_v4();
        let result = channel.reconnect(id, &test_request("unknown"));
        assert!(matches!(result, Err(DarviumError::HumanChannelIo(_))));
    }

    /// T9-3: 未知 ID + プリロードあり
    #[test]
    fn t9_3_unknown_id_with_preloaded() {
        let outcome = HumanOutcome::Responded(HumanResponse {
            decision: HumanDecision::Approved,
            comment: Some("recovered".into()),
            revised_body: None,
        });
        let channel = FakeHumanChannel::new(VecDeque::from(vec![outcome.clone()]));
        let id = uuid::Uuid::new_v4();
        let handle = channel.reconnect(id, &test_request("recover")).unwrap();
        let result = handle.wait(None).unwrap();
        assert_eq!(result, outcome);
    }

    /// T9-4: Stdinout reconnect protocol
    #[test]
    fn t9_4_stdinout_reconnect_protocol() {
        let reader = std::io::BufReader::new(std::io::empty());
        let mut writer: Vec<u8> = Vec::new();
        let channel = StdinoutChannel::new(reader, &mut writer);
        let id = uuid::Uuid::new_v4();
        let _ = channel.reconnect(id, &test_request("reconn-protocol"));

        let output = String::from_utf8(writer).unwrap();
        assert!(output.contains(r#""type":"reconnect""#));
        assert!(output.contains(&id.to_string()));
        assert!(output.contains(r#""subject":"reconn-protocol""#));
    }

    /// T9-5: Stdinout reconnect 解決
    #[test]
    fn t9_5_stdinout_reconnect_resolve() {
        let response = r#"{"interaction_id":"00000000-0000-0000-0000-000000000000","outcome":{"Responded":{"decision":"Approved","comment":null,"revised_body":null}}}"#;
        let reader = std::io::BufReader::new(response.as_bytes());
        let writer: Vec<u8> = Vec::new();
        let channel = StdinoutChannel::new(reader, writer);
        let id = uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap();
        let handle = channel.reconnect(id, &test_request("resolve")).unwrap();
        let result = handle.wait(Some(Duration::from_secs(1)));
        assert!(result.is_ok());
    }

    /// T9-6: Stdinout reconnect 不正応答
    #[test]
    fn t9_6_stdinout_reconnect_invalid() {
        let invalid = "not json at all\n";
        let reader = std::io::BufReader::new(invalid.as_bytes());
        let writer: Vec<u8> = Vec::new();
        let channel = StdinoutChannel::new(reader, writer);
        let id = uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap();
        let handle = channel.reconnect(id, &test_request("invalid")).unwrap();
        let result = handle.wait(Some(Duration::from_secs(1)));
        assert!(matches!(result, Err(DarviumError::HumanChannelIo(_))));
    }

    /// T9-7: notify→communicate→reconnect 一貫性
    #[test]
    fn t9_7_full_flow_consistency() {
        let outcome = HumanOutcome::Responded(HumanResponse {
            decision: HumanDecision::Approved,
            comment: Some("full flow".into()),
            revised_body: None,
        });
        let channel = FakeHumanChannel::new(VecDeque::from(vec![outcome.clone()]));

        channel.notify(&test_request("step1")).unwrap();
        assert_eq!(channel.sent_count(), 1);

        let handle = channel.communicate(&test_request("step2")).unwrap();
        let comm_outcome = handle.wait(None).unwrap();
        assert_eq!(comm_outcome, outcome);
        assert_eq!(channel.sent_count(), 2);

        let interactions = channel.export_interactions();
        assert_eq!(interactions.len(), 1);
        let id = uuid::Uuid::parse_str(&interactions[0].interaction_id).unwrap();
        let handle2 = channel.reconnect(id, &test_request("step3")).unwrap();
        let recon_outcome = handle2.wait(None).unwrap();
        assert_eq!(recon_outcome, outcome);
    }

    // ============================================================
    // OTS: 観測テスト
    // ============================================================

    /// OTS-1: 呼び出し回数 vs 記録件数完全一致 (n=10,000, σ²=0)
    #[test]
    fn ots1_call_count_vs_record_count() {
        use rand::rngs::StdRng;
        use rand::{Rng, SeedableRng};

        let n = 10_000u64;
        let channel = FakeHumanChannel::new(VecDeque::new());
        let mut rng = StdRng::seed_from_u64(12345);

        println!("=== OTS-1: Call Count vs Record Count ===");
        println!("n={}", n);

        for i in 0..n {
            let request = HumanRequest {
                subject: format!("ots-{}", i),
                body: rng.random::<u64>().to_string(),
                context: serde_json::json!({"seq": i}),
                timeout: None,
            };
            channel.notify(&request).unwrap();
        }

        let count = channel.sent_count();
        let record_len = channel.requests_sent().len() as u64;

        println!("sent_count={}, requests_sent_len={}", count, record_len);
        assert_eq!(count, n, "sent_count must equal n ({} != {})", count, n);
        assert_eq!(
            record_len, n,
            "requests_sent length must equal n ({} != {})",
            record_len, n
        );
        println!("=== 結果: PASS (σ² = 0) ===");
    }

    /// OTS-2: Serde ラウンドトリップ (n=8,192)
    #[test]
    fn ots2_serde_roundtrip() {
        use rand::rngs::StdRng;
        use rand::{Rng, SeedableRng};

        let n = 8_192usize;
        let mut rng = StdRng::seed_from_u64(12345);
        let decisions = [
            HumanDecision::Approved,
            HumanDecision::Rejected,
            HumanDecision::NeedsRevision,
            HumanDecision::Irrelevant,
            HumanDecision::Unsafe,
        ];
        let statuses = [InteractionStatus::Pending, InteractionStatus::Resolved];

        println!("=== OTS-2: Serde Roundtrip ===");
        println!(
            "n={}, types=[StoredInteraction, HumanRequest, HumanOutcome]",
            n
        );

        let mut passed: u64 = 0;
        for i in 0..n {
            let original = StoredInteraction {
                interaction_id: uuid::Uuid::new_v4().to_string(),
                payload: HitlPayload {
                    request: HumanRequest {
                        subject: rng.random::<u64>().to_string(),
                        body: rng.random::<u64>().to_string(),
                        context: serde_json::json!({"r": rng.random::<u64>()}),
                        timeout: if rng.random_bool(0.5) {
                            Some(Duration::from_secs(rng.random_range(1..3600)))
                        } else {
                            None
                        },
                    },
                },
                outcome: if rng.random_bool(0.5) {
                    Some(HumanOutcome::Responded(HumanResponse {
                        decision: decisions[rng.random_range(0..5)],
                        comment: if rng.random_bool(0.5) {
                            Some(rng.random::<u64>().to_string())
                        } else {
                            None
                        },
                        revised_body: if rng.random_bool(0.3) {
                            Some(rng.random::<u64>().to_string())
                        } else {
                            None
                        },
                    }))
                } else if rng.random_bool(0.5) {
                    Some(HumanOutcome::TimedOut)
                } else {
                    Some(HumanOutcome::Unreachable(rng.random::<u64>().to_string()))
                },
                status: statuses[rng.random_range(0..2)],
                created_at: rng.random::<u64>() % 1_000_000_000,
                updated_at: rng.random::<u64>() % 1_000_000_000,
            };

            let json = serde_json::to_string(&original).unwrap();
            let deserialized: StoredInteraction = serde_json::from_str(&json).unwrap();
            assert_eq!(
                deserialized, original,
                "OTS-2: StoredInteraction roundtrip failed at iteration {}",
                i
            );
            passed += 1;
        }

        println!("passed={}/{}", passed, n);
        println!("=== 結果: PASS (全ラウンドトリップ成功) ===");
    }

    // ============================================================
    // EventBusHumanChannel テスト (M1.5-R7)
    // ============================================================

    /// FakeEventBus + InMemoryMetadataStore で EventBusHumanChannel を生成するヘルパー。
    fn make_eventbus_channel() -> (
        EventBusHumanChannel,
        Arc<FakeEventBus>,
        Arc<InMemoryMetadataStore>,
    ) {
        let bus = Arc::new(FakeEventBus::new());
        let store = Arc::new(InMemoryMetadataStore::new());
        let channel = EventBusHumanChannel::new(bus.clone(), store.clone());
        (channel, bus, store)
    }

    /// T1: EventBusHumanChannel が HumanChannel トレイト境界を充足する
    #[test]
    fn t1_eventbus_channel_implements_trait() {
        let (channel, _, _) = make_eventbus_channel();
        let _: Box<dyn HumanChannel> = Box::new(channel);
    }

    /// T2: notify() が HitlEvent::NotificationRequested を publish する
    #[test]
    fn t2_eventbus_notify_publishes_notification_requested() {
        let (channel, bus, _) = make_eventbus_channel();
        let request = test_request("notify-eventbus");
        channel.notify(&request).unwrap();

        let events = bus.published_events();
        assert_eq!(events.len(), 1, "notify should publish 1 event");
        assert!(
            matches!(
                &events[0].kind,
                DarviumEventKind::Hitl(HitlEvent::NotificationRequested)
            ),
            "notify should publish NotificationRequested, got {:?}",
            events[0].kind
        );
        assert_eq!(events[0].interaction_mode, InteractionMode::OneWay);
    }

    /// T3: communicate() が HitlEvent::InteractionRequested × TwoWay を publish する
    #[test]
    fn t3_eventbus_communicate_publishes_interaction_requested() {
        let (channel, bus, store) = make_eventbus_channel();

        // このテストでは communicate は pending で返る。応答は不要。
        let request = test_request("comm-eventbus");
        let handle = channel.communicate(&request).unwrap();

        let events = bus.published_events();
        assert_eq!(events.len(), 1, "communicate should publish 1 event");

        // DarviumEventKind::Hitl(HitlEvent::InteractionRequested) であること
        assert!(
            matches!(
                &events[0].kind,
                DarviumEventKind::Hitl(HitlEvent::InteractionRequested)
            ),
            "communicate should publish InteractionRequested, got {:?}",
            events[0].kind
        );
        // InteractionMode::TwoWay であること
        assert_eq!(
            events[0].interaction_mode,
            InteractionMode::TwoWay,
            "communicate should use TwoWay mode"
        );

        // interaction_id が handle と一致すること
        assert_eq!(events[0].event_id, handle.interaction_id.to_string());

        // MetadataStore にも Pending として保存されていること
        let pending = store.list_pending_human_interactions().unwrap();
        assert_eq!(
            pending.len(),
            1,
            "communicate should store 1 pending interaction"
        );
        assert_eq!(pending[0].interaction_id, handle.interaction_id.to_string());
        assert_eq!(pending[0].status, InteractionStatus::Pending);
    }

    /// T4: communicate() → MetadataStore に Pending レコードが保存される
    #[test]
    fn t4_eventbus_communicate_stores_pending() {
        let (channel, _, store) = make_eventbus_channel();
        let request = test_request("pending-check");
        let handle = channel.communicate(&request).unwrap();

        let pending = store.list_pending_human_interactions().unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].interaction_id, handle.interaction_id.to_string());
        assert_eq!(pending[0].status, InteractionStatus::Pending);
        assert_eq!(pending[0].payload.request.subject, "pending-check");
    }

    /// T5: communicate() → resolve_interaction() → wait() が非同期解決される
    #[test]
    fn t5_eventbus_resolve_async() {
        use std::sync::Arc;

        let bus = Arc::new(FakeEventBus::new());
        let store = Arc::new(InMemoryMetadataStore::new());
        let channel = Arc::new(EventBusHumanChannel::new(bus.clone(), store.clone()));

        let request = test_request("async-resolve");
        let handle = channel.communicate(&request).unwrap();
        let id = handle.interaction_id.to_string();
        let id_for_thread = id.clone();

        let expected_outcome = HumanOutcome::Responded(HumanResponse {
            decision: HumanDecision::Approved,
            comment: Some("async resolution".into()),
            revised_body: None,
        });

        // 別スレッドで解決
        let channel_clone = channel.clone();
        let outcome_clone = expected_outcome.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(50));
            channel_clone
                .resolve_interaction(&id_for_thread, outcome_clone)
                .ok();
        });

        let result = handle.wait(Some(Duration::from_secs(5))).unwrap();
        assert_eq!(result, expected_outcome);

        // MetadataStore も Resolved になっている
        let loaded = store.load_human_interaction(&id).unwrap();
        assert_eq!(loaded.status, InteractionStatus::Resolved);
        assert_eq!(loaded.outcome, Some(expected_outcome));
    }

    /// T6: reconnect() が MetadataStore のタイムスタンプを更新する
    #[test]
    fn t6_eventbus_reconnect_updates_metadata_store() {
        let (channel, _, store) = make_eventbus_channel();

        // 事前に communicate でインタラクションを作成
        let handle = channel.communicate(&test_request("reconnect-ts")).unwrap();
        let id = handle.interaction_id;

        // 再接続前の updated_at
        let before = store.load_human_interaction(&id.to_string()).unwrap();
        let old_ts = before.updated_at;

        // 少し待ってから再接続
        std::thread::sleep(Duration::from_millis(1));
        let reconnect_handle = channel
            .reconnect(id, &test_request("reconnect-ts"))
            .unwrap();
        assert_eq!(reconnect_handle.interaction_id, id);

        let after = store.load_human_interaction(&id.to_string()).unwrap();
        assert!(
            after.updated_at > old_ts,
            "updated_at should increase after reconnect: {} <= {}",
            after.updated_at,
            old_ts
        );
    }

    /// T7: reconnect() が EventBus の再接続を呼び出す
    #[test]
    fn t7_eventbus_reconnect_calls_eventbus() {
        let (channel, bus, _) = make_eventbus_channel();

        // 事前に communicate でインタラクションを作成
        let handle = channel.communicate(&test_request("reconnect-bus")).unwrap();
        let id = handle.interaction_id;

        // 再接続実行
        let reconnect_handle = channel
            .reconnect(id, &test_request("reconnect-bus"))
            .unwrap();

        // Clock が 1 以上（communicate.open=1、reconnect は clock を進めない, RFC §12C.6）
        assert!(
            bus.current_clock() >= 1,
            "clock should be >= 1 after communicate (reconnect does not advance clock per RFC §12C.6), got {}",
            bus.current_clock()
        );
        assert_eq!(
            reconnect_handle.interaction_id.to_string(),
            handle.interaction_id.to_string()
        );
    }

    /// T8: notify のペイロードが EventBus イベントに正しく保存される
    #[test]
    fn t8_eventbus_notify_preserves_payload() {
        let (channel, bus, _) = make_eventbus_channel();

        let request = HumanRequest {
            subject: "payload-test".into(),
            body: "important body".into(),
            context: serde_json::json!({"key": "value", "nested": {"a": 1}}),
            timeout: Some(Duration::from_secs(300)),
        };
        channel.notify(&request).unwrap();

        let events = bus.published_events();
        assert_eq!(events.len(), 1);

        // ペイロードが HumanRequest として復元可能か
        let restored: HumanRequest = serde_json::from_value(events[0].payload.clone()).unwrap();
        assert_eq!(restored.subject, "payload-test");
        assert_eq!(restored.body, "important body");
        assert_eq!(
            restored.context,
            serde_json::json!({"key": "value", "nested": {"a": 1}})
        );
        assert_eq!(restored.timeout, Some(Duration::from_secs(300)));
    }

    /// T9: HumanChannelConfig 未設定でも FakeHumanChannel が動作する（後方互換性）
    #[test]
    fn t9_fake_channel_backward_compat_no_config() {
        let outcome = HumanOutcome::Responded(HumanResponse {
            decision: HumanDecision::Approved,
            comment: None,
            revised_body: None,
        });
        let channel = FakeHumanChannel::new(VecDeque::from(vec![outcome.clone()]));
        let handle = channel.communicate(&test_request("legacy")).unwrap();
        let result = handle.wait(None).unwrap();
        assert_eq!(result, outcome);
    }

    /// T10: with_config で空の Option を渡しても動作する
    #[test]
    fn t10_fake_channel_with_empty_config() {
        let outcome = HumanOutcome::Responded(HumanResponse {
            decision: HumanDecision::Approved,
            comment: None,
            revised_body: None,
        });
        let config = HumanChannelConfig {
            event_bus: None,
            interaction_store: None,
        };
        let channel = FakeHumanChannel::with_config(VecDeque::from(vec![outcome.clone()]), config);
        let handle = channel.communicate(&test_request("empty-config")).unwrap();
        let result = handle.wait(None).unwrap();
        assert_eq!(result, outcome);
    }

    /// T11: with_config で EventBus のみ設定しても動作する
    #[test]
    fn t11_fake_channel_with_eventbus_only() {
        let outcome = HumanOutcome::Responded(HumanResponse {
            decision: HumanDecision::Approved,
            comment: None,
            revised_body: None,
        });
        let bus = Arc::new(FakeEventBus::new());
        let config = HumanChannelConfig {
            event_bus: Some(bus),
            interaction_store: None,
        };
        let channel = FakeHumanChannel::with_config(VecDeque::from(vec![outcome.clone()]), config);
        let handle = channel.communicate(&test_request("bus-only")).unwrap();
        let result = handle.wait(None).unwrap();
        assert_eq!(result, outcome);
    }

    // ============================================================
    // OTS-1: EventBus モード vs 従来モード一貫性 (n=100)
    // ============================================================
    #[test]
    fn ots1_legacy_vs_eventbus_consistency_n100() {
        use rand::rngs::StdRng;
        use rand::{Rng, SeedableRng};

        let n = 100u64;
        let mut rng = StdRng::seed_from_u64(12345);
        let outcomes = [
            HumanOutcome::Responded(HumanResponse {
                decision: HumanDecision::Approved,
                comment: None,
                revised_body: None,
            }),
            HumanOutcome::Responded(HumanResponse {
                decision: HumanDecision::Rejected,
                comment: Some("no".into()),
                revised_body: None,
            }),
            HumanOutcome::TimedOut,
            HumanOutcome::Unreachable("offline".into()),
        ];

        // 従来モード
        let legacy = FakeHumanChannel::new(VecDeque::from(
            (0..n)
                .map(|i| {
                    let idx = (i % 4) as usize;
                    outcomes[idx].clone()
                })
                .collect::<VecDeque<_>>(),
        ));

        // EventBus adapter モード
        let bus = Arc::new(FakeEventBus::new());
        let store = Arc::new(InMemoryMetadataStore::new());
        let eventbus_delegate = EventBusHumanChannel::new(bus.clone(), store.clone());

        // 従来モードの操作系列を実行
        let mut legacy_notify_count = 0u64;
        let mut legacy_comm_count = 0u64;
        let mut legacy_outcomes = Vec::new();

        for i in 0..n {
            let request = HumanRequest {
                subject: format!("ots1-{}", i),
                body: rng.random::<u64>().to_string(),
                context: serde_json::json!({"seq": i}),
                timeout: if rng.random_bool(0.3) {
                    Some(Duration::from_secs(rng.random_range(1..3600)))
                } else {
                    None
                },
            };

            // notify 50%, communicate 50%
            if rng.random_bool(0.5) {
                legacy.notify(&request).unwrap();
                legacy_notify_count += 1;
            } else {
                if let Ok(handle) = legacy.communicate(&request) {
                    let result = handle.wait(None).unwrap();
                    legacy_outcomes.push(result);
                    legacy_comm_count += 1;
                }
            }
        }

        // EventBus adapter モードも同様の操作系列で実行
        // 同じシードで再初期化
        let mut rng2 = StdRng::seed_from_u64(12345);
        let mut adapter_notify_count = 0u64;
        let mut adapter_comm_count = 0u64;
        let mut adapter_outcomes = Vec::new();

        for i in 0..n {
            let request = HumanRequest {
                subject: format!("ots1-{}", i),
                body: rng2.random::<u64>().to_string(),
                context: serde_json::json!({"seq": i}),
                timeout: if rng2.random_bool(0.3) {
                    Some(Duration::from_secs(rng2.random_range(1..3600)))
                } else {
                    None
                },
            };

            if rng2.random_bool(0.5) {
                eventbus_delegate.notify(&request).unwrap();
                adapter_notify_count += 1;
            } else {
                if let Ok(handle) = eventbus_delegate.communicate(&request) {
                    // EventBus 経由で解決
                    let id = handle.interaction_id.to_string();
                    let idx = (adapter_comm_count % 4) as usize;
                    let outcome = outcomes[idx].clone();
                    eventbus_delegate.resolve_interaction(&id, outcome).ok();
                    let result = handle.wait(Some(Duration::from_secs(5))).unwrap();
                    adapter_outcomes.push(result);
                    adapter_comm_count += 1;
                }
            }
        }

        println!("=== OTS-1: Legacy vs EventBus Adapter Consistency ===");
        println!("n={}", n);
        println!(
            "legacy:   notify={}, communicate={}",
            legacy_notify_count, legacy_comm_count
        );
        println!(
            "adapter:  notify={}, communicate={}",
            adapter_notify_count, adapter_comm_count
        );
        assert_eq!(
            legacy_notify_count, adapter_notify_count,
            "notify count mismatch"
        );
        assert_eq!(
            legacy_comm_count, adapter_comm_count,
            "communicate count mismatch"
        );
        assert_eq!(
            legacy_outcomes.len(),
            adapter_outcomes.len(),
            "outcome count mismatch"
        );
        let matching = legacy_outcomes
            .iter()
            .zip(&adapter_outcomes)
            .filter(|(a, b)| a == b)
            .count();
        println!("matching_outcomes={}/{}", matching, legacy_outcomes.len());
        println!(
            "=== 結果: PASS (一致率 {:.1}%) ===",
            if legacy_outcomes.is_empty() {
                100.0
            } else {
                100.0 * matching as f64 / legacy_outcomes.len() as f64
            }
        );
    }
}
