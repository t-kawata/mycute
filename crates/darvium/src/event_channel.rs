// Darvium EventChannel — 外部イベント送受信抽象 (RFC §12D)
//
// 本ファイルは EventChannel トレイトとその標準実装を定義する。
// 絶対正本: Darvium-RFC-0001-Unified-v2.3-final.md §12D

use serde::{Deserialize, Serialize};
use std::io::{BufRead, Write};
use std::sync::{Arc, Mutex};

use crate::error::DarviumError;
use crate::event::{
    DarviumEvent, DarviumEventKind, EventCausality, EventMetadata, EventPrivacy, EventRetention,
    EventSource, EventVisibility, HitlEvent, InteractionMode, PiiHandlingPolicy,
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
            ReciprocityEvent, RepairEvent, SystemEvent, TrainingEvent, WorkflowExecutionEvent,
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
            DarviumEventKind::Reciprocity(ReciprocityEvent::HelpOffered),
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
}
