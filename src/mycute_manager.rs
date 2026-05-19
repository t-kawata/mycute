use crate::mycute_settings::LocaleCode;
use crate::stt::SpeechRecognizer;
use parking_lot::Mutex;
use std::sync::Arc;
use tokio::sync::oneshot;

pub use crate::types::{AppState, InputMode};

pub struct MycuteManager {
    pub recognizer: Arc<Mutex<SpeechRecognizer>>,
    pub state: AppState,
    pub input_mode: InputMode,
    pub current_text: String,
    pub buffer: String,
    pub locale: LocaleCode,
    pub last_stt_seq: u64,
    /// 最終補正レイヤーの実行中フラグ（入力ロック用）
    pub is_post_correcting: bool,
    /// PostCorrection 完了待ちフラグ（フラッシュ保留用）
    pub pending_flush: bool,
    /// STT 認識結果 API 呼び出し待ちフラグ（フラッシュ保留用）
    pub is_stt_pending: bool,
    /// 音声入力中にウィンドウを最前面に固定する設定
    pub pin_during_voice: bool,
    /// 非同期フラッシュ要求の送信側（oneshot）。
    ///
    /// `request_flush()` で設定され、STTパイプラインの完了（Stopped /
    /// SttCompleted / PostCorrectionFinished）時に `build_flush_text()`
    /// の結果が送信される。レガシー音声入力・オーケストレーター・
    /// 将来の音声機能すべてが同一の完了待ちパスを通るための共通機構。
    pub flush_tx: Option<oneshot::Sender<String>>,
}

impl MycuteManager {
    pub fn new(recognizer: Arc<Mutex<SpeechRecognizer>>, locale: LocaleCode) -> Self {
        Self {
            recognizer,
            state: AppState::Idle,
            input_mode: InputMode::RealTime,
            current_text: String::new(),
            buffer: String::new(),
            locale,
            last_stt_seq: 0,
            is_post_correcting: false,
            pending_flush: false,
            is_stt_pending: false,
            pin_during_voice: false,
            flush_tx: None,
        }
    }

    /// STTエンジンを停止し、確定テキストの非同期受信チャネルを返す。
    ///
    /// STTパイプラインが未処理のイベントを全て消費した後、
    /// `build_flush_text()` の結果が Receiver 経由で届く。
    /// レガシー音声入力の `pending_flush` 機構と同様の遅延実行を
    /// oneshot チャネルで実現し、オーケストレーターなど
    /// 複数の機能が同一の完了待ちパスを通ることを保証する。
    pub fn request_flush(&mut self) -> oneshot::Receiver<String> {
        // flush_tx を先にセットしてから recognizer.stop() を呼び出す。
        // stop() は即座に SttEvent::Stopped をイベントループに送信するため、
        // 逆順だと Stopped ハンドラ実行時に flush_tx が None のまま
        // 競合が発生する（oneshot が送信されず rx.await が永久待機する）。
        let (tx, rx) = oneshot::channel();
        self.flush_tx = Some(tx);
        self.recognizer.lock().stop();
        rx
    }

    pub fn start_recording(&mut self, mode: InputMode) {
        self.pending_flush = false;
        self.flush_tx = None;
        self.state = AppState::Recording;
        self.input_mode = mode;
        self.current_text.clear();
        self.buffer.clear();
        self.last_stt_seq = 0;
        self.is_post_correcting = false;
        self.is_stt_pending = false;
        self.recognizer.lock().start();
    }

    pub fn stop_recording(&mut self) {
        self.recognizer.lock().stop();
        self.state = AppState::Idle;
        // current_text はクリアしない — Stopped イベントハンドラが履歴保存時に
        // 最後の認識テキストを読み取れるようにする。次の start_recording() でクリアされる。
        // self.current_text.clear();
        self.last_stt_seq = 0;
        self.is_post_correcting = false;
        self.pending_flush = false;
        self.is_stt_pending = false;
    }

    /// buffer と current_text を連結してフラッシュ用の全文を構築する。
    /// current_text が既に buffer の内容を先頭に含む場合は連結せず
    /// current_text のみを返す（STTが全文送信方式の場合の重複防止）。
    pub fn build_flush_text(&self) -> String {
        if self.current_text.is_empty() {
            self.buffer.clone()
        } else if self.current_text.starts_with(&self.buffer) {
            self.current_text.clone()
        } else {
            format!("{}{}", self.buffer, self.current_text)
        }
    }

    pub fn set_locale(&mut self, locale: LocaleCode) {
        self.locale = locale;
        self.recognizer.lock().set_locale(locale);
        if self.state == AppState::Recording {
            self.recognizer.lock().stop();
            self.recognizer.lock().start();
        }
    }
}
