//! Cuber Event Bus
//!
//! 非同期イベントの Pub/Sub を実現するイベントバスです。
//! Go 版 `lib/eventbus` に相当し、処理の進捗（Absorb 開始/終了、チャンク処理中など）を
//! リアルタイムに通知するために使用されます。

use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::broadcast;
use crate::utils::time;

// ============================================================
// StreamEvent
// ============================================================

/// ストリームイベントの種類
///
/// Go 版 `event_types.go` に相当します。
/// 各処理ステージでのイベントを表現します。
#[derive(Debug, Clone)]
pub enum EventType {
    // Absorb 関連
    AbsorbStart,
    AbsorbAddFileStart,
    AbsorbAddFileEnd,
    AbsorbCognifyStart,
    AbsorbCognifyEnd,
    AbsorbError,
    AbsorbEnd,

    // Query 関連
    QueryStart,
    QueryEmbedding,
    QueryVectorSearch,
    QueryFtsSearch,
    QueryGraphTraversal,
    QuerySynthesis,
    QueryEnd,

    // Memify 関連
    MemifyStart,
    MemifyRuleExtraction,
    MemifySelfReflection,
    MemifyCrystallization,
    MemifyMetabolism,
    MemifyEnd,

    // 汎用
    Progress,
    Info,
    Warning,
    Error,
}

/// ストリームイベント
///
/// クライアントへ送信されるイベントのペイロードです。
#[derive(Debug, Clone)]
pub struct StreamEvent {
    /// イベント ID（連番）
    pub id: u64,

    /// イベント種類
    pub event_type: EventType,

    /// イベントメッセージ
    pub message: String,

    /// 進捗（0-100）、進捗イベントの場合のみ使用
    pub progress: Option<u8>,

    /// 追加データ（JSON 文字列など）
    pub data: Option<String>,

    /// タイムスタンプ（Unix ミリ秒）
    pub timestamp: u64,
}

impl StreamEvent {
    /// 新しい StreamEvent を作成します。
    pub fn new(event_type: EventType, message: impl Into<String>) -> Self {
        Self {
            id: 0, // EventBus で設定される
            event_type,
            message: message.into(),
            progress: None,
            data: None,
            timestamp: time::now_ts_ms(),
        }
    }

    /// 進捗を設定します。
    pub fn with_progress(mut self, progress: u8) -> Self {
        self.progress = Some(progress);
        self
    }

    /// 追加データを設定します。
    pub fn with_data(mut self, data: impl Into<String>) -> Self {
        self.data = Some(data.into());
        self
    }
}

// ============================================================
// EventBus
// ============================================================

/// イベントバス
///
/// `tokio::sync::broadcast` を用いた Pub/Sub パターンを実装します。
/// 複数のサブスクライバーに同じイベントを配信できます。
pub struct EventBus {
    /// ブロードキャスト送信者
    sender: broadcast::Sender<StreamEvent>,

    /// イベント ID カウンター
    event_counter: AtomicU64,
}

impl EventBus {
    /// 新しい EventBus を作成します。
    ///
    /// # Arguments
    /// * `capacity` - チャネルのキャパシティ（デフォルト: 256）
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self {
            sender,
            event_counter: AtomicU64::new(0),
        }
    }

    /// デフォルトのキャパシティで EventBus を作成します。
    pub fn default() -> Self {
        Self::new(256)
    }

    /// イベントを発火（送信）します。
    ///
    /// サブスクライバーがいない場合は何も起こりません（エラーにはなりません）。
    pub fn emit(&self, mut event: StreamEvent) {
        // イベント ID を割り当て
        event.id = self.event_counter.fetch_add(1, Ordering::SeqCst);

        // 受信者がいなくても送信を試みる（エラーは無視）
        let _ = self.sender.send(event);
    }

    /// イベントを購読します。
    ///
    /// 新しい Receiver を返します。この Receiver で `.recv().await` を呼び出して
    /// イベントを受信します。
    pub fn subscribe(&self) -> broadcast::Receiver<StreamEvent> {
        self.sender.subscribe()
    }

    /// Absorb 開始イベントを発火します。
    pub fn emit_absorb_start(&self, message: impl Into<String>) {
        self.emit(StreamEvent::new(EventType::AbsorbStart, message));
    }

    /// Absorb 終了イベントを発火します。
    pub fn emit_absorb_end(&self, message: impl Into<String>) {
        self.emit(StreamEvent::new(EventType::AbsorbEnd, message));
    }

    /// Absorb エラーイベントを発火します。
    pub fn emit_absorb_error(&self, message: impl Into<String>) {
        self.emit(StreamEvent::new(EventType::AbsorbError, message));
    }

    /// Query 開始イベントを発火します。
    pub fn emit_query_start(&self, message: impl Into<String>) {
        self.emit(StreamEvent::new(EventType::QueryStart, message));
    }

    /// Query 終了イベントを発火します。
    pub fn emit_query_end(&self, message: impl Into<String>) {
        self.emit(StreamEvent::new(EventType::QueryEnd, message));
    }

    /// Memify 開始イベントを発火します。
    pub fn emit_memify_start(&self, message: impl Into<String>) {
        self.emit(StreamEvent::new(EventType::MemifyStart, message));
    }

    /// Memify 終了イベントを発火します。
    pub fn emit_memify_end(&self, message: impl Into<String>) {
        self.emit(StreamEvent::new(EventType::MemifyEnd, message));
    }

    /// 進捗イベントを発火します。
    pub fn emit_progress(&self, message: impl Into<String>, progress: u8) {
        self.emit(StreamEvent::new(EventType::Progress, message).with_progress(progress));
    }

    /// 情報イベントを発火します。
    pub fn emit_info(&self, message: impl Into<String>) {
        self.emit(StreamEvent::new(EventType::Info, message));
    }

    /// 警告イベントを発火します。
    pub fn emit_warning(&self, message: impl Into<String>) {
        self.emit(StreamEvent::new(EventType::Warning, message));
    }

    /// エラーイベントを発火します。
    pub fn emit_error(&self, message: impl Into<String>) {
        self.emit(StreamEvent::new(EventType::Error, message));
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new(256)
    }
}

// TODO: 将来実装予定の機能
// - SSE (Server-Sent Events) へのストリーミング変換
// - イベントテンプレート（25 バリエーション）の実装

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_bus_pubsub() {
        let bus = EventBus::default();

        // サブスクライバーを作成
        let mut receiver = bus.subscribe();

        // イベントを発火
        bus.emit_absorb_start("Starting absorb process");

        // イベントを受信
        let event = receiver.recv().await.expect("Should receive event");
        assert!(matches!(event.event_type, EventType::AbsorbStart));
        assert_eq!(event.message, "Starting absorb process");
    }

    #[tokio::test]
    async fn test_event_id_increment() {
        let bus = EventBus::default();
        let mut receiver = bus.subscribe();

        bus.emit_info("First");
        bus.emit_info("Second");

        let event1 = receiver.recv().await.unwrap();
        let event2 = receiver.recv().await.unwrap();

        assert_eq!(event1.id, 0);
        assert_eq!(event2.id, 1);
    }
}
