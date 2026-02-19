use crate::stt::SpeechRecognizer;
use crate::stt_config::LocaleCode;
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
        }
    }

    pub fn start_recording(&mut self, mode: InputMode) {
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
