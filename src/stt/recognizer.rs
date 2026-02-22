//! Speech recognition using Swift helper library.
//!
//! This module provides real-time speech-to-text transcription by
//! calling into a Swift library via C FFI.
//! Supports both Classic (SFSpeechRecognizer) and Tahoe (macOS 15+) modes.
//! Also supports Sherpa-ONNX for cross-platform recognition.

#[cfg(target_os = "macos")]
use super::mac::MacSpeechBackend;
use super::openai::OpenAIRecognizer;
#[cfg(target_os = "windows")]
use super::win::WinSpeechBackend;
use crate::llm::client::LlmPool;
use crate::stt::openai::OpenAIBackend;
use crate::stt_config::{LlmEndpoint, LocaleCode, SttEngine, SttSettings};
use crate::tools::post_correction_processor::{PostCorrectionBackend, PostCorrectionConfig};
use crate::tools::pseudo_asr_streamer::BackendWrapper;
use crate::types::SttEvent;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;

/// Speech recognizer that uses Swift helper library.
pub struct SpeechRecognizer {
    is_running: Arc<AtomicBool>,
    engine: SttEngine,
    locale: LocaleCode,
    /// OpenAI backend
    openai_backend: Option<OpenAIRecognizer>,
    /// Windows backend (Windows only)
    #[cfg(target_os = "windows")]
    win_backend: Option<WinSpeechBackend>,
    /// macOS backend (macOS only)
    #[cfg(target_os = "macos")]
    mac_backend: Option<MacSpeechBackend>,
    /// Last sent transcription for deduplication
    last_result: String,
    /// Local sequence counter for Sherpa01
    sequence_counter: u64,
    /// Event sender
    tx: mpsc::Sender<SttEvent>,
}

impl SpeechRecognizer {
    /// Validates if the selected engine is compatible with the current OS and hardware.
    /// 選択されたエンジンが現在のOSおよびハードウェアと互換性があるかを検証する。
    pub fn validate_config(engine: &SttEngine) -> Result<(), String> {
        match engine {
            SttEngine::OpenAI => Ok(()),
            SttEngine::Os => {
                // ビルド時のターゲットOSにより、対応するネイティブバックエンドが含まれているため常にOK
                Ok(())
            }
        }
    }

    /// Create a new speech recognizer with the given event sender.
    ///
    /// For Sherpa01 engine, `sherpa01_settings` must be provided.
    /// For Swift engines (Classic/Tahoe), `sherpa01_settings` is not used.
    pub fn new(
        tx: mpsc::Sender<SttEvent>,
        engine: SttEngine,
        locale: LocaleCode,
        stt_settings: Option<SttSettings>,
        llm_pool: Arc<LlmPool>,
        replaces_map: Arc<parking_lot::RwLock<indexmap::IndexMap<String, Vec<String>>>>,
    ) -> Result<Self, String> {
        // 各バックエンドエンジン向けに IndexMap<String, Vec<String>> を Vec<(String, String)> にフラット化
        let mut flat_replaces = Vec::new();
        {
            let map = replaces_map.read();
            for (after, befores) in map.iter() {
                for before in befores {
                    flat_replaces.push((before.clone(), after.clone()));
                }
            }
        }
        // 置換の整合性を保つため、置換前文字列（before）の長い順にソート（最長一致優先）
        // これにより、例えば "foo" より先に "foobar" を置換対象にする挙動を維持する
        flat_replaces.sort_by(|a, b| b.0.len().cmp(&a.0.len()));

        // Initialize openai_backend
        let openai_backend = if engine == SttEngine::OpenAI {
            let settings = stt_settings.clone().unwrap_or_default();
            let mut recognizer = OpenAIRecognizer::new(
                tx.clone(),
                settings,
                locale,
                llm_pool.clone(),
                flat_replaces.clone(),
            );
            // Initialize audio
            if let Err(e) = recognizer.init_audio() {
                return Err(format!("Audio init failed for OpenAI engine: {}", e));
            } else {
                log::debug!("OpenAI backend initialized successfully");
                Some(recognizer)
            }
        } else {
            None
        };

        // macOS ネイティブバックエンドの初期化 (Os エンジン選択時)
        #[cfg(target_os = "macos")]
        let mac_backend = if engine == SttEngine::Os {
            // 設定が利用可能な場合、単語補正バックエンドを準備
            let (pc_backend, pc_config) = if let Some(ref settings) = stt_settings {
                // 補正用 OpenAI バックエンドを作成
                if let Ok(backend) = OpenAIBackend::new(
                    settings,
                    llm_pool.clone(),
                    Arc::new(parking_lot::Mutex::new(locale)),
                    flat_replaces.clone(),
                ) {
                    let wrapper: Arc<dyn PostCorrectionBackend> =
                        Arc::new(BackendWrapper(Arc::new(std::sync::Mutex::new(backend))));
                    let config = PostCorrectionConfig {
                        sentence_count_threshold: settings.post_correction_sentence_count_threshold,
                        min_text_length: settings.post_correction_min_text_length,
                        interval_ms: settings.post_correction_interval_ms,
                    };
                    (Some(wrapper), Some(config))
                } else {
                    (None, None)
                }
            } else {
                (None, None)
            };

            match MacSpeechBackend::new(
                tx.clone(),
                engine.clone(),
                locale,
                pc_backend,
                pc_config,
                flat_replaces.clone(),
                stt_settings.clone(),
            ) {
                Ok(backend) => Some(backend),
                Err(e) => {
                    return Err(format!("Failed to initialize macOS backend: {}", e));
                }
            }
        } else {
            None
        };

        // Windows ネイティブバックエンドの初期化 (Os エンジン選択時)
        #[cfg(target_os = "windows")]
        let win_backend = if engine == SttEngine::Os {
            // 単語補正用の設定を取得
            let (pc_backend, pc_config) = if !llm_pool.is_empty() {
                let dummy_settings = stt_settings.clone().unwrap_or_default();
                if let Ok(b) = OpenAIBackend::new(
                    &dummy_settings,
                    llm_pool.clone(),
                    Arc::new(parking_lot::Mutex::new(locale)),
                    flat_replaces.clone(),
                ) {
                    let wrapper: Arc<dyn PostCorrectionBackend> =
                        Arc::new(BackendWrapper(Arc::new(std::sync::Mutex::new(b))));
                    let config = if let Some(ref s) = stt_settings {
                        Some(PostCorrectionConfig {
                            sentence_count_threshold: s.post_correction_sentence_count_threshold,
                            min_text_length: s.post_correction_min_text_length,
                            interval_ms: s.post_correction_interval_ms,
                        })
                    } else {
                        None
                    };
                    (Some(wrapper), config)
                } else {
                    (None, None)
                }
            } else {
                (None, None)
            };

            match WinSpeechBackend::new(
                tx.clone(),
                locale,
                pc_backend,
                pc_config,
                flat_replaces.clone(),
                stt_settings.clone(),
            ) {
                Ok(backend) => Some(backend),
                Err(e) => {
                    return Err(format!("Failed to initialize Windows backend: {}", e));
                }
            }
        } else {
            None
        };

        Ok(Self {
            is_running: Arc::new(AtomicBool::new(false)),
            engine,
            locale,
            openai_backend,
            #[cfg(target_os = "windows")]
            win_backend,
            #[cfg(target_os = "macos")]
            mac_backend,
            last_result: String::new(),
            sequence_counter: 0,
            tx,
        })
    }

    /// Start the speech recognition.
    pub fn start(&mut self) {
        if self.is_running.load(Ordering::SeqCst) {
            log::debug!("Speech recognition already running");
            return;
        }

        self.is_running.store(true, Ordering::SeqCst);
        if let Err(e) = self.tx.try_send(SttEvent::Started) {
            log::error!("[WinInputDebug] Failed to send SttEvent::Started: {:?}", e);
        } else {
            log::info!("[WinInputDebug] Successfully sent SttEvent::Started");
        }

        // Handle OpenAI engine
        if self.engine == SttEngine::OpenAI {
            if let Some(ref mut backend) = self.openai_backend {
                backend.start();
                log::debug!("Speech recognition started (engine: OpenAI)");
            } else {
                log::error!("OpenAI backend not initialized");
                self.is_running.store(false, Ordering::SeqCst);
            }
            return;
        }

        // Os エンジン: Windows ネイティブバックエンドの開始
        #[cfg(target_os = "windows")]
        if self.engine == SttEngine::Os {
            if let Some(ref mut backend) = self.win_backend {
                backend.start();
                log::debug!("Speech recognition started (engine: Os/Win)");
            } else {
                log::error!("Windows backend not initialized");
                self.is_running.store(false, Ordering::SeqCst);
            }
            return;
        }

        // Handle macOS backend
        #[cfg(target_os = "macos")]
        if let Some(ref mut backend) = self.mac_backend {
            backend.start();
        }
    }

    /// Stop the speech recognition.
    pub fn stop(&mut self) {
        if !self.is_running.load(Ordering::SeqCst) {
            return;
        }

        self.is_running.store(false, Ordering::SeqCst);
        self.last_result.clear();
        self.sequence_counter = 0; // Reset counter on stop

        // Handle OpenAI engine
        if let Some(ref mut backend) = self.openai_backend {
            backend.stop();
        }
        log::debug!("Speech recognition stopped (OpenAI)");

        // Os エンジン: Windows ネイティブバックエンドの停止
        #[cfg(target_os = "windows")]
        if self.engine == SttEngine::Os {
            if let Some(ref mut backend) = self.win_backend {
                backend.stop();
            }
            log::debug!("Speech recognition stopped (Os/Win)");
        }

        // Handle macOS backend
        #[cfg(target_os = "macos")]
        if let Some(ref mut backend) = self.mac_backend {
            backend.stop();
        }

        // Notify frontend that we stopped
        let _ = self.tx.try_send(SttEvent::Stopped);

        log::debug!("Speech recognition stopped");
    }

    /// Update the locale for next recognition session
    pub fn set_locale(&mut self, locale: LocaleCode) {
        self.locale = locale;

        // Propagate to OpenAI backend if active
        if self.engine == SttEngine::OpenAI {
            if let Some(ref mut backend) = self.openai_backend {
                backend.set_locale(locale);
            }
        }

        // Os エンジン: Windows ネイティブバックエンドへのロケール伝播
        #[cfg(target_os = "windows")]
        if self.engine == SttEngine::Os {
            if let Some(ref mut backend) = self.win_backend {
                backend.set_locale(locale);
            }
        }

        // Propagate to macOS backend if active
        #[cfg(target_os = "macos")]
        if let Some(ref mut backend) = self.mac_backend {
            backend.set_locale(locale);
        }
    }

    /// Update the engine for next recognition session
    pub fn set_engine(&mut self, engine: SttEngine) {
        self.engine = engine;
    }

    /// Update configuration including engine, locale, and OpenAI settings.
    pub fn update_config(
        &mut self,
        engine: SttEngine,
        locale: LocaleCode,
        _stt_settings: Option<SttSettings>,
        _llm_endpoints: Vec<LlmEndpoint>,
    ) -> Result<(), String> {
        let was_running = self.is_running.load(Ordering::SeqCst);
        if was_running {
            self.stop();
        }

        self.engine = engine;
        self.locale = locale;

        // Re-initialize OpenAI if needed
        if self.engine == SttEngine::OpenAI {
            // If backend exists, we could try updating it, but for simplicity we recreate for now
            // if significant settings changed. Or just update if it exists.
            if let Some(ref mut backend) = self.openai_backend {
                backend.set_locale(locale);
                // Note: other settings in OpenAIRecognizer are not easily hot-swappable
                // without internal changes. For now, we at least update locale.
                // Re-initializing the whole backend is safer but might lose state.
            } else {
                // Should not happen if engine was OpenAI before, but handles engine switch
                let (_tx, _): (mpsc::Sender<SttEvent>, _) = mpsc::channel(1); // Placeholder, real tx is needed.
                                                                              // Re-creating the backend requires the original tx which we don't store.
                                                                              // This suggests we should recreate the SpeechRecognizer in the manager instead.
            }
        }

        if was_running {
            self.start();
        }
        Ok(())
    }

    pub fn cleanup(&self) {
        #[cfg(target_os = "macos")]
        if let Some(ref backend) = self.mac_backend {
            backend.cleanup();
        }
    }

    pub fn tick(&mut self) {
        // Handle OpenAI engine
        if self.engine == SttEngine::OpenAI {
            if let Some(ref mut backend) = self.openai_backend {
                if !self.is_running.load(Ordering::SeqCst) {
                    return;
                }
                backend.tick();
            }
            return;
        }

        // Os エンジン: Windows ネイティブバックエンドの tick
        #[cfg(target_os = "windows")]
        if self.engine == SttEngine::Os {
            if let Some(ref mut backend) = self.win_backend {
                if !self.is_running.load(Ordering::SeqCst) {
                    return;
                }
                backend.tick();
            }
            return;
        }

        // Swift engines (Classic/Tahoe)
        #[cfg(target_os = "macos")]
        if let Some(ref mut backend) = self.mac_backend {
            // DEBUG: Uncomment to verify tick is being called
            // log::trace!("[Recognizer] Calling mac_backend.tick()");
            backend.tick();
        }
    }
}

impl Drop for SpeechRecognizer {
    fn drop(&mut self) {
        self.stop();
        self.cleanup();
    }
}
