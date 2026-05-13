use crate::stt::SpeechRecognizer;
use crate::mycute_settings::LocaleCode;
use parking_lot::Mutex;
use std::sync::Arc;

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
        }
    }

    pub fn start_recording(&mut self, mode: InputMode) {
        self.pending_flush = false;
        self.state = AppState::Recording;
        self.input_mode = mode;
        self.current_text.clear();
        self.last_stt_seq = 0;
        self.is_post_correcting = false;
        if mode == InputMode::Buffered {
            self.buffer.clear();
        }
        self.recognizer.lock().start();
    }

    pub fn stop_recording(&mut self) {
        self.recognizer.lock().stop();
        self.state = AppState::Idle;
        self.current_text.clear();
        self.last_stt_seq = 0;
        self.is_post_correcting = false;
        self.pending_flush = false;
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
