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
use crate::mycute_settings::{LlmEndpoint, LocaleCode, SttEngine, SttSettings};
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
    /// Event sender (インターセプタータスクが保持するため、ここでは Started/Stopped 用にのみ使用)
    tx: mpsc::Sender<SttEvent>,
    /// Shared locale across all components
    shared_locale: Arc<parking_lot::Mutex<LocaleCode>>,
    /// 置換辞書の共有参照 (インターセプタータスクと共有され、常に最新の状態が反映される)
    #[allow(dead_code)]
    replaces_map: Arc<parking_lot::RwLock<indexmap::IndexMap<String, Vec<String>>>>,
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
        // ================================================================
        // インターセプター層の構築:
        // 各バックエンドには tx_internal を渡し、イベントを中継タスクで
        // 傷受し、FinalResult/PartialResult のテキストに置換辞書を適用してから
        // 本来の tx (UI向け) に転送する。
        // ================================================================
        let (tx_internal, mut rx_internal) = mpsc::channel::<SttEvent>(100);

        // 置換辞書の共有参照をタスクに渡す
        let replaces_map_for_task = replaces_map.clone();
        let tx_for_task = tx.clone();

        // インターセプタータスク: 各エンジンからのイベントを受信し、テキストに置換をかけて UI へ転送
        // SpeechRecognizer::new はTokioコンテキスト外で呼ばれるため、std::thread を使用する
        std::thread::spawn(move || {
            while let Some(event) = rx_internal.blocking_recv() {
                let forwarded = match event {
                    SttEvent::FinalResult(text, seq) => {
                        let replaced = apply_replaces_from_map(&replaces_map_for_task, &text);
                        SttEvent::FinalResult(replaced, seq)
                    }
                    SttEvent::PartialResult(text, seq) => {
                        let replaced = apply_replaces_from_map(&replaces_map_for_task, &text);
                        SttEvent::PartialResult(replaced, seq)
                    }
                    // その他のイベント (Started, Stopped, Ready, Error, 補正系等) はそのまま転送
                    other => other,
                };
                if tx_for_task.blocking_send(forwarded).is_err() {
                    // UI側のチャネルが閉じた場合はタスクを終了
                    log::debug!("[Interceptor] UI channel closed, stopping intercept task.");
                    break;
                }
            }
            log::debug!("[Interceptor] Intercept task finished.");
        });

        let shared_locale = Arc::new(parking_lot::Mutex::new(locale));

        // 即時切り替えを可能にするため、選択されたエンジンに関わらずopenai_backendを初期化する
        let settings = stt_settings.clone().unwrap_or_default();
        let mut openai_recognizer = OpenAIRecognizer::new(
            tx_internal.clone(),
            settings,
            shared_locale.clone(),
            llm_pool.clone(),
        );

        // 音声の初期化（イベント受信タスクなどの起動）
        let openai_backend = if let Err(e) = openai_recognizer.init_audio() {
            log::error!("[SpeechRecognizer] Audio init failed for OpenAI engine: {}", e);
            None
        } else {
            log::info!("[SpeechRecognizer] OpenAI backend Fully Initialized (including PseudoAsrStreamer, LlmPool, and Event Rx Task). Engine is ready for instant switch.");
            Some(openai_recognizer)
        };

        // macOS ネイティブバックエンドの初期化 (常に初期化する)
        #[cfg(target_os = "macos")]
        let mac_backend = {
            // 設定が利用可能な場合、単語補正バックエンドを準備
            let (pc_backend, pc_config) = if let Some(ref settings) = stt_settings {
                // 補正用 OpenAI バックエンドを作成
                if let Ok(backend) = OpenAIBackend::new(
                    settings,
                    llm_pool.clone(),
                    shared_locale.clone(),
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
                tx_internal.clone(),
                engine.clone(),
                shared_locale.clone(),
                pc_backend,
                pc_config,
                stt_settings.clone(),
            ) {
                Ok(backend) => {
                    log::info!("[SpeechRecognizer] MacSpeechBackend Fully Initialized. OS engine is ready for instant switch.");
                    Some(backend)
                },
                Err(e) => {
                    log::error!("[SpeechRecognizer] Failed to initialize macOS backend: {}", e);
                    None
                }
            }
        };

        // Windows ネイティブバックエンドの初期化 (常に初期化する)
        #[cfg(target_os = "windows")]
        let win_backend = {
            // 単語補正用の設定を取得
            let (pc_backend, pc_config) = if !llm_pool.is_empty() {
                let dummy_settings = stt_settings.clone().unwrap_or_default();
                if let Ok(b) = OpenAIBackend::new(
                    &dummy_settings,
                    llm_pool.clone(),
                    shared_locale.clone(),
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
                tx_internal.clone(),
                shared_locale.clone(),
                pc_backend,
                pc_config,
                stt_settings.clone(),
            ) {
                Ok(backend) => {
                    log::info!("[SpeechRecognizer] WinSpeechBackend Fully Initialized. OS engine is ready for instant switch.");
                    Some(backend)
                },
                Err(e) => {
                    log::error!("[SpeechRecognizer] Failed to initialize Windows backend: {}", e);
                    None
                }
            }
        };

        Ok(Self {
            is_running: Arc::new(AtomicBool::new(false)),
            engine,
            openai_backend,
            #[cfg(target_os = "windows")]
            win_backend,
            #[cfg(target_os = "macos")]
            mac_backend,
            last_result: String::new(),
            sequence_counter: 0,
            tx,
            shared_locale,
            replaces_map,
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
                log::info!("[SpeechRecognizer] Speech recognition started (engine: OpenAI)");
            } else {
                log::error!("[SpeechRecognizer] OpenAI backend not initialized");
                self.is_running.store(false, Ordering::SeqCst);
            }
            return;
        }

        // Os エンジン: Windows ネイティブバックエンドの開始
        #[cfg(target_os = "windows")]
        if self.engine == SttEngine::Os {
            if let Some(ref mut backend) = self.win_backend {
                backend.start();
                log::info!("[SpeechRecognizer] Speech recognition started (engine: Os/Win)");
            } else {
                log::error!("[SpeechRecognizer] Windows backend not initialized");
                self.is_running.store(false, Ordering::SeqCst);
            }
            return;
        }

        // Handle macOS backend
        #[cfg(target_os = "macos")]
        if self.engine == SttEngine::Os {
            if let Some(ref mut backend) = self.mac_backend {
                backend.start();
                log::info!("[SpeechRecognizer] Speech recognition started (engine: Os/Mac)");
            } else {
                log::error!("[SpeechRecognizer] macOS backend not initialized");
                self.is_running.store(false, Ordering::SeqCst);
            }
            return;
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

        // すべてのアクティブなバックエンドを停止し、クリーンな状態を保つ
        if let Some(ref mut backend) = self.openai_backend {
            backend.stop();
        }

        #[cfg(target_os = "windows")]
        if let Some(ref mut backend) = self.win_backend {
            backend.stop();
        }

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
        *self.shared_locale.lock() = locale;

        // OpenAIバックエンドへの伝播
        if let Some(ref mut backend) = self.openai_backend {
            backend.set_locale(locale);
        }

        // Os エンジン: Windows ネイティブバックエンドへのロケール伝播
        #[cfg(target_os = "windows")]
        if let Some(ref mut backend) = self.win_backend {
            backend.set_locale(locale);
        }

        // macOSネイティブバックエンドへの伝播
        #[cfg(target_os = "macos")]
        if let Some(ref mut backend) = self.mac_backend {
            backend.set_locale(locale);
        }
    }

    pub fn set_engine(&mut self, engine: SttEngine) {
        self.engine = engine;
    }

    /// Update configuration including engine, locale, and OpenAI settings.
    pub fn update_config(
        &mut self,
        engine: SttEngine,
        locale: LocaleCode,
        stt_settings: Option<SttSettings>,
        llm_endpoints: Vec<LlmEndpoint>,
    ) -> Result<(), String> {
        let was_running = self.is_running.load(Ordering::SeqCst);
        if was_running {
            self.stop();
        }
        if self.engine != engine {
            log::info!("[SpeechRecognizer] Switching engine from {:?} to {:?}", self.engine, engine);
            self.engine = engine;
        }

        // 設定の再初期化（ロケールのシームレスな更新）
        if let Some(ref mut backend) = self.openai_backend {
            backend.set_locale(locale);
        }

        #[cfg(target_os = "macos")]
        if let Some(ref mut backend) = self.mac_backend {
            backend.set_locale(locale);

            // 補正設定の動的更新
            let (pc_backend, pc_config) = if !llm_endpoints.is_empty() {
                // 補正用 OpenAI バックエンドを現在の設定から作成
                let settings = stt_settings.clone().unwrap_or_default();
                let pool = self.openai_backend.as_ref().map(|b| b.llm_pool());

                if let Some(p) = pool {
                    if let Ok(oa_backend) = OpenAIBackend::new(&settings, p, self.shared_locale.clone()) {
                        let wrapper: Arc<dyn PostCorrectionBackend> =
                            Arc::new(BackendWrapper(Arc::new(std::sync::Mutex::new(oa_backend))));
                        let config = PostCorrectionConfig::default();
                        (Some(wrapper), Some(config))
                    } else {
                        (None, None)
                    }
                } else {
                    (None, None)
                }
            } else {
                (None, None)
            };
            backend.update_pc_config(pc_backend, pc_config);
        }
        #[cfg(target_os = "windows")]
        if let Some(ref mut backend) = self.win_backend {
            backend.set_locale(locale);
 
            // 補正設定の動的更新
            let (pc_backend, pc_config) = if !llm_endpoints.is_empty() {
                // 補正用 OpenAI バックエンドを現在の設定から作成
                let settings = stt_settings.clone().unwrap_or_default();
                let pool = self.openai_backend.as_ref().map(|b| b.llm_pool());

                if let Some(p) = pool {
                    if let Ok(oa_backend) = OpenAIBackend::new(&settings, p, self.shared_locale.clone()) {
                        let wrapper: Arc<dyn PostCorrectionBackend> =
                            Arc::new(BackendWrapper(Arc::new(std::sync::Mutex::new(oa_backend))));
                        let config = PostCorrectionConfig::default();
                        (Some(wrapper), Some(config))
                    } else {
                        (None, None)
                    }
                } else {
                    (None, None)
                }
            } else {
                (None, None)
            };
            backend.update_pc_config(pc_backend, pc_config);
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

        // Os エンジン: ネイティブバックエンドの tick
        if self.engine == SttEngine::Os {
            #[cfg(target_os = "windows")]
            if let Some(ref mut backend) = self.win_backend {
                if !self.is_running.load(Ordering::SeqCst) {
                    return;
                }
                backend.tick();
            }

            #[cfg(target_os = "macos")]
            if let Some(ref mut backend) = self.mac_backend {
                if !self.is_running.load(Ordering::SeqCst) {
                    return;
                }
                backend.tick();
            }
        }
    }
}

impl Drop for SpeechRecognizer {
    fn drop(&mut self) {
        self.stop();
        self.cleanup();
    }
}

/// インターセプタータスク用ヘルパー: 置換辞書マップからテキストに一括置換を適用する。
///
/// IndexMap<String, Vec<String>> は { "置換後" => ["置換前1", "置換前2", ...] } の形式。
/// これをフラット化し、置換前文字列の長い順（最長一致優先）にソートしてから順次置換する。
/// `RwLock::read()` のため、動的な辞書更新とも安全に共存する。
fn apply_replaces_from_map(
    replaces_map: &parking_lot::RwLock<indexmap::IndexMap<String, Vec<String>>>,
    text: &str,
) -> String {
    let map = replaces_map.read();
    if map.is_empty() {
        return text.to_string();
    }

    // IndexMap をフラット化: (before, after) のペアに展開
    let mut flat: Vec<(&str, &str)> = Vec::new();
    for (after, befores) in map.iter() {
        for before in befores {
            if !before.is_empty() {
                flat.push((before.as_str(), after.as_str()));
            }
        }
    }

    // 最長一致優先: 置換前文字列が長いものを先に適用する
    flat.sort_by(|a, b| b.0.len().cmp(&a.0.len()));

    // 順次置換を適用
    let mut result = text.to_string();
    for (from, to) in &flat {
        result = result.replace(from, to);
    }
    result
}
