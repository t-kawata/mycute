//! OpenAI Recognizer - 疑似ストリーミング ASR
//!
//! このモジュールは、PseudoAsrStreamer と AsrBackend トレイトを使用して
//! OpenAI モデルによる疑似ストリーミング音声認識を実現します。

use crate::constants::{OPENAI_READY_DELAY_MS, STT_DECORATION_INTERVAL_MS};
use crate::llm::client::LlmPool;
#[cfg(target_os = "macos")]
use crate::stt::mac::{start_native_audio_capture, stop_native_audio_capture};
#[cfg(target_os = "windows")]
use crate::stt::win::{start_native_audio_capture, stop_native_audio_capture};
use crate::mycute_settings::{LocaleCode, SttSettings, VadType as ConfigVadType};
use crate::tools::pseudo_asr_streamer::{
    AsrBackend, PseudoAsrStreamer, StreamerConfig, StreamerEvent, StreamerLocale, VadType,
};
use crate::types::SttEvent;
use anyhow::{anyhow, Result};
use async_openai::{
    types::audio::{AudioInput, AudioResponseFormat, CreateTranscriptionRequestArgs},
    Client as OpenAIClient,
};
use hound;
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;
use std::{io::Cursor, sync::atomic::AtomicU64};
use tauri::async_runtime::{self, JoinHandle};
use tokio::runtime::Handle;
use tokio::sync::mpsc;
use tokio::task::block_in_place;
use tokio::time;

/// 音声認識に使用するモデル名（音声認識APIに渡す値であり、かつ統計ログに記録される名称）
const TRANSCRIPTION_MODEL: &str = "gpt-4o-mini-transcribe";

// ============================================================================
// OpenAIBackend: AsrBackend 実装
// ============================================================================

/// OpenAI APIを使用した音声認識バックエンド
pub struct OpenAIBackend {
    /// LLM pool for OpenAI ASR and correction (round-robin inside pool)
    llm_pool: Arc<LlmPool>,
    /// Shared language state for OpenAI ASR
    language: Arc<Mutex<LocaleCode>>,
    /// 文字列置換リスト (from, to)
    replaces: Vec<(String, String)>,
}

impl OpenAIBackend {
    /// 新しいバックエンドを作成
    pub fn new(
        _settings: &SttSettings,
        llm_pool: Arc<LlmPool>,
        language: Arc<Mutex<LocaleCode>>,
        replaces: Vec<(String, String)>,
    ) -> Result<Self> {
        log::debug!("[OpenAIBackend] Initializing OpenAI ASR backend using LlmPool");

        Ok(Self {
            llm_pool,
            language,
            replaces,
        })
    }

    pub fn set_language(&self, lang: LocaleCode) {
        let mut guard = self.language.lock();
        *guard = lang;
    }
}

impl AsrBackend for OpenAIBackend {
    fn transcribe(&mut self, samples: &[f32]) -> Result<String> {
        let client = self
            .llm_pool
            .next()
            .ok_or_else(|| anyhow!("No LLM endpoints configured in settings.json"))?;
        let api_key = client.api_key();
        if api_key.is_empty() {
            return Err(anyhow!("API key is empty for client: {}", client.name));
        }

        log::debug!(
            "[OpenAIBackend] Using OpenAI endpoint for ASR: {} (model: {})",
            client.name,
            client.model_name()
        );

        log::debug!(
            "[OpenAIBackend] Using OpenAI endpoint: {} (model: {})",
            client.name,
            client.model_name()
        );

        // 1. Convert samples to in-memory WAV format
        let mut buffer = Cursor::new(Vec::new());
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 16000,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };

        {
            let mut writer = hound::WavWriter::new(&mut buffer, spec)
                .map_err(|e| anyhow!("Failed to create WAV writer: {}", e))?;
            for sample in samples {
                writer
                    .write_sample(*sample)
                    .map_err(|e| anyhow!("Failed to write sample: {}", e))?;
            }
            writer
                .finalize()
                .map_err(|e| anyhow!("Failed to finalize WAV: {}", e))?;
        }

        let wav_bytes = buffer.into_inner();
        log::debug!("[OpenAIBackend] WAV buffer size: {} bytes", wav_bytes.len());

        // 2. Create AudioInput from bytes
        let audio_input = AudioInput::from_vec_u8("input.wav".to_string(), wav_bytes);

        // 3. Build OpenAI client with base_url
        let raw_base_url = client.base_url();
        let base_url = if raw_base_url.is_empty() {
            "https://api.openai.com/v1"
        } else {
            raw_base_url
        };
        log::debug!("[OpenAIBackend] Using base_url: {}", base_url);

        let config = async_openai::config::OpenAIConfig::new()
            .with_api_key(api_key)
            .with_api_base(base_url);
        let openai_client = OpenAIClient::with_config(config);

        // 4. Build transcription request
        log::debug!(
            "[OpenAIBackend] Using transcription model: {}",
            TRANSCRIPTION_MODEL
        );
        log::debug!("[OpenAIBackend] Using language: {:?}", self.language.lock());

        let request = CreateTranscriptionRequestArgs::default()
            .file(audio_input)
            .model(TRANSCRIPTION_MODEL)
            .response_format(AudioResponseFormat::Json)
            .language(self.language.lock().as_str().to_string())
            .build()
            .map_err(|e| anyhow!("Failed to build transcription request: {}", e))?;

        // 5. Execute async request - use block_in_place to safely block within tokio runtime
        let result = block_in_place(|| {
            Handle::current()
                .block_on(async { openai_client.audio().transcription().create(request).await })
        });

        match result {
            Ok(response) => {
                log::debug!("[OpenAIBackend] OpenAI transcription: {}", response.text);
                Ok(response.text)
            }
            Err(e) => {
                log::error!("[OpenAIBackend] OpenAI API error: {}", e);
                Err(anyhow!("OpenAI transcription failed: {}", e))
            }
        }
    }

    fn post_correct(&mut self, text: &str) -> Result<String> {
        // Get the current language
        let lang = self.language.lock().clone();

        log::debug!("[OpenAIBackend] Post-correction using common LlmPool");

        // Execute correction using the pool
        let result = block_in_place(|| {
            Handle::current().block_on(async { self.llm_pool.correct_text(text, lang).await })
        });

        result.map_err(|e| anyhow!("Post-correction via LlmPool failed: {}", e))
    }

    fn model_name(&self) -> String {
        TRANSCRIPTION_MODEL.to_string()
    }

    fn record_asr_usage(&mut self, duration_ms: u64) {
        if let Err(e) = crate::stt::stats::UsageStats::record_asr(TRANSCRIPTION_MODEL, duration_ms)
        {
            log::error!("[OpenAIBackend] Failed to record ASR usage: {}", e);
        }
    }

    fn insert_punctuation(&mut self, text: &str, _locale: &StreamerLocale) -> Result<String> {
        // OpenAI already provides punctuation, so we just pass through.
        Ok(text.to_string())
    }

    fn replaces(&self) -> Vec<(String, String)> {
        self.replaces.clone()
    }
}

// ============================================================================
// OpenAIRecognizer: メイン認識器
// ============================================================================

/// OpenAI 疑似ストリーミング認識器
pub struct OpenAIRecognizer {
    streamer: Arc<Mutex<Option<PseudoAsrStreamer<OpenAIBackend>>>>,
    settings: SttSettings,
    tx: mpsc::Sender<SttEvent>,
    is_running: Arc<AtomicBool>,
    ticker_task: Option<JoinHandle<()>>,
    /// LLM pool for OpenAI ASR test
    llm_pool: Arc<LlmPool>,
    /// Decoration task handle (shared for async access)
    decoration_task: Arc<Mutex<Option<JoinHandle<()>>>>,
    /// Flag indicating whether decoration is actively sending
    is_decorating: Arc<AtomicBool>,
    /// 文字列置換リスト (from, to)
    replaces: Vec<(String, String)>,
    /// Global sequence counter for SttEvents
    sequence_counter: Arc<AtomicU64>,
    /// Shared language state
    language: Arc<parking_lot::Mutex<LocaleCode>>,
    /// Session counter for task invalidation
    session_counter: Arc<AtomicU64>,
    /// Buffer for incoming audio samples from native capture callback
    audio_buf: Arc<Mutex<Vec<f32>>>,
    /// Sample rate of the current audio stream (Dynamic, updated by capture)
    sample_rate: Arc<AtomicU32>,
    /// Buffer for PartialResult during decoration (latest only)
    partial_result_buffer: Arc<Mutex<Option<String>>>,
    /// Time when last SpeechEnd was received (for anomaly detection)
    last_speech_end_time: Arc<Mutex<Option<std::time::Instant>>>,
    // ネイティブ音声キャプチャ用タスク
    capture_task: Option<JoinHandle<()>>,
    // イベントリスナータスク (start() 時に起動)
    event_listener_task: Option<JoinHandle<()>>,
    // イベント受信チャネル (init_audio で作成、start で使用)
    event_rx: Arc<Mutex<Option<mpsc::Receiver<StreamerEvent>>>>,
}

impl OpenAIRecognizer {
    /// 新しい認識器を作成
    pub fn new(
        tx: mpsc::Sender<SttEvent>,
        settings: SttSettings,
        shared_locale: Arc<parking_lot::Mutex<LocaleCode>>,
        llm_pool: Arc<LlmPool>,
        replaces: Vec<(String, String)>,
    ) -> Self {
        Self {
            streamer: Arc::new(Mutex::new(None)),
            settings,
            tx,
            is_running: Arc::new(AtomicBool::new(false)),
            ticker_task: None,
            llm_pool,
            decoration_task: Arc::new(Mutex::new(None)),
            is_decorating: Arc::new(AtomicBool::new(false)),
            replaces,
            sequence_counter: Arc::new(AtomicU64::new(0)),
            language: shared_locale,
            session_counter: Arc::new(AtomicU64::new(0)),
            audio_buf: Arc::new(Mutex::new(Vec::with_capacity(16000))),
            sample_rate: Arc::new(AtomicU32::new(0)),
            partial_result_buffer: Arc::new(Mutex::new(None)),
            last_speech_end_time: Arc::new(Mutex::new(None)),
            capture_task: None,
            event_listener_task: None,
            event_rx: Arc::new(Mutex::new(None)),
        }
    }

    /// Get the common LlmPool
    pub fn llm_pool(&self) -> Arc<LlmPool> {
        self.llm_pool.clone()
    }

    /// Get the flattened replaces list
    pub fn replaces(&self) -> Vec<(String, String)> {
        self.replaces.clone()
    }

    /// オーディオ入力ストリームとバックエンドの初期化
    pub fn init_audio(&mut self) -> Result<()> {
        // 統計機能の初期化 (書き込み権限チェック)
        crate::stt::stats::UsageStats::init()?;

        // バックエンドの作成 (Shared language Arc を渡す)
        let backend = OpenAIBackend::new(
            &self.settings,
            self.llm_pool.clone(),
            Arc::clone(&self.language),
            self.replaces.clone(),
        )?;

        // StreamerConfig の構築
        let v_type = match self.settings.vad_type {
            ConfigVadType::Ten | ConfigVadType::TenInt8 => VadType::Ten,
            ConfigVadType::Silero | ConfigVadType::SileroInt8 => VadType::Silero,
        };

        let current_locale = *self.language.lock();
        let locale = match current_locale {
            LocaleCode::En => StreamerLocale::En,
            LocaleCode::Ja => StreamerLocale::Ja,
        };

        let config = StreamerConfig {
            vad_model_path: self.settings.get_vad_path(),
            vad_type: v_type,
            vad_threshold: self.settings.vad_threshold,
            vad_min_silence_duration: self.settings.vad_min_silence_duration,
            vad_min_speech_duration: self.settings.vad_min_speech_duration,
            vad_max_speech_duration: self.settings.vad_max_speech_duration,
            vad_pre_padding_ms: self.settings.vad_pre_padding_ms as u32,
            utterance_min_ms: self.settings.utterance_min_ms as u32,
            num_threads: self.settings.num_threads,
            locale,
            signal_check_enabled: self.settings.signal_check_enabled.unwrap_or(true),
            signal_rms_threshold: self.settings.signal_rms_threshold.unwrap_or(0.005),
            signal_occupancy_ratio: self.settings.signal_occupancy_ratio.unwrap_or(0.15),
            use_denoiser: self.settings.use_denoiser,
            denoiser_model_path: self.settings.get_denoiser_path(),
            post_correction_sentence_count_threshold: self
                .settings
                .post_correction_sentence_count_threshold,
            post_correction_min_text_length: self.settings.post_correction_min_text_length,
            post_correction_interval_ms: self.settings.post_correction_interval_ms,
        };

        let (tx_streamer, rx_streamer) = mpsc::channel(100);

        // ストリーマーの作成
        let streamer = match PseudoAsrStreamer::new(backend, tx_streamer, config) {
            Ok(s) => s,
            Err(e) => return Err(anyhow!("Failed to create PseudoAsrStreamer: {}", e)),
        };

        #[cfg(target_os = "windows")]
        {
            // Windows native capture is always 16kHz
            self.sample_rate.store(16000, Ordering::Relaxed);
            log::info!(
                "[OpenAI] Initializing native capture on Windows. Fixed sample rate: 16000Hz"
            );
        }

        // ストリーマーを Arc/Mutex に格納
        {
            let mut guard = self.streamer.lock();
            *guard = Some(streamer);
        }

        // rx_streamer を保存して start() で使用する
        {
            let mut guard = self.event_rx.lock();
            *guard = Some(rx_streamer);
        }

        Ok(())
    }

    /// 認識を開始
    pub fn start(&mut self) {
        let mut guard = self.streamer.lock();
        if let Some(ref mut streamer) = *guard {
            if let Err(e) = streamer.start() {
                log::error!("Failed to start streamer: {}", e);
                return;
            }

            // すでに実行中の場合は何もしない
            if self.is_running.load(Ordering::SeqCst) {
                return;
            }

            self.is_running.store(true, Ordering::SeqCst);

            // ----------------------------------------------------------------
            // イベント転送タスクの起動 (init_audio から移動)
            // ----------------------------------------------------------------
            // チャンネルの転送タスク
            let decoration_task = Arc::clone(&self.decoration_task);
            let is_decorating = Arc::clone(&self.is_decorating);
            // セッション管理用のカウンタ（構造体のものを使用）
            let session_counter = Arc::clone(&self.session_counter);
            let sequence_counter = Arc::clone(&self.sequence_counter);
            // 装飾タスクのタイムアウト用に vad_max_speech_duration を取得
            let decoration_timeout_secs = self.settings.vad_max_speech_duration + 5.0;
            // PartialResult バッファ（装飾中のデータ消失を防ぐ）
            let partial_result_buffer = Arc::clone(&self.partial_result_buffer);
            // SpeechEnd 後の異常検知用
            let last_speech_end_time = Arc::clone(&self.last_speech_end_time);
            let tx_out = self.tx.clone();

            // 保存しておいた rx_streamer を取り出す（初回のみ、あるいは再生成が必要なら再生成ロジックが必要だが、
            // 現状 init_audio は1回のみ前提。再 connection 等で作り直す場合は event_rx も更新される必要がある）
            let mut rx_guard = self.event_rx.lock();
            if let Some(mut rx_streamer) = rx_guard.take() {
                log::debug!("[OpenAIRecognizer] Starting event listener task");
                self.event_listener_task = Some(async_runtime::spawn(async move {
                    while let Some(event) = rx_streamer.recv().await {
                        match event {
                            StreamerEvent::SpeechStart(org_text) => {
                                // SpeechStart 時は last_speech_end_time をリセット
                                {
                                    let mut guard = last_speech_end_time.lock();
                                    *guard = None;
                                }
                                // バッファがあればフラッシュ（クリアではなく送信！）
                                // 重要: 高速な SpeechStart/SpeechEnd 遷移時にデータを失わないために
                                let buffered_text_option = {
                                    let mut guard = partial_result_buffer.lock();
                                    guard.take()
                                };
                                if let Some(buffered_text) = buffered_text_option {
                                    let seq = sequence_counter.fetch_add(1, Ordering::SeqCst);
                                    let _ = tx_out
                                        .send(SttEvent::PartialResult(buffered_text, seq))
                                        .await;
                                }

                                // セッションIDを更新（古いタスクからのメッセージを無効化する）
                                let current_session =
                                    session_counter.fetch_add(1, Ordering::SeqCst) + 1;
                                is_decorating.store(true, Ordering::SeqCst);

                                let old_task_handle = {
                                    let mut guard = decoration_task.lock();
                                    guard.take()
                                };
                                if let Some(old_task) = old_task_handle {
                                    old_task.abort();
                                    let _ = old_task.await;
                                }

                                let tx_decoration = tx_out.clone();
                                let decoration_task_clone = Arc::clone(&decoration_task);
                                let base_text = org_text.clone();
                                let is_decorating_clone = Arc::clone(&is_decorating);
                                let session_counter_clone = Arc::clone(&session_counter);
                                let sequence_counter_clone = Arc::clone(&sequence_counter);
                                let timeout_duration =
                                    Duration::from_secs_f32(decoration_timeout_secs);
                                let last_speech_end_time_clone = Arc::clone(&last_speech_end_time);
                                let partial_result_buffer_clone =
                                    Arc::clone(&partial_result_buffer);

                                let handle = async_runtime::spawn(async move {
                                    let pattern = [" … ", "?"];
                                    let mut decorated = base_text;
                                    let mut idx = 0;
                                    let start_time = std::time::Instant::now();
                                    loop {
                                        tokio::time::sleep(Duration::from_millis(
                                            STT_DECORATION_INTERVAL_MS,
                                        ))
                                        .await;

                                        // チェック1: フラグが落とされていたら即終了
                                        if !is_decorating_clone.load(Ordering::SeqCst) {
                                            break;
                                        }

                                        // チェック2: セッションIDが変わっていたら即終了
                                        if session_counter_clone.load(Ordering::SeqCst)
                                            != current_session
                                        {
                                            break;
                                        }

                                        // チェック3: タイムアウト（vad_max_speech_duration + 5秒）
                                        if start_time.elapsed() > timeout_duration {
                                            log::warn!("[Decoration] Timeout after {:.1}s - forcibly stopping.", start_time.elapsed().as_secs_f32());
                                            break;
                                        }

                                        // チェック4: SpeechEnd から 750ms 経過しているのにまだ装飾中なら異常 → ForceClearDecoration
                                        let should_force_clear = {
                                            let end_time_guard = last_speech_end_time_clone.lock();
                                            if let Some(end_time) = *end_time_guard {
                                                end_time.elapsed() > Duration::from_millis(750)
                                            } else {
                                                false
                                            }
                                        };
                                        if should_force_clear {
                                            log::warn!("[Decoration] Abnormal state: SpeechEnd was >750ms ago but still decorating - forcing clear");
                                            is_decorating_clone.store(false, Ordering::SeqCst);
                                            let _ = tx_decoration
                                                .send(SttEvent::ForceClearDecoration)
                                                .await;

                                            // バッファにあれば送信（装飾のない純粋な最新テキストで同期）
                                            let buffered_text_option = {
                                                let mut guard = partial_result_buffer_clone.lock();
                                                guard.take()
                                            };
                                            if let Some(buffered_text) = buffered_text_option {
                                                let seq = sequence_counter_clone
                                                    .fetch_add(1, Ordering::SeqCst);
                                                let _ = tx_decoration
                                                    .send(SttEvent::PartialResult(
                                                        buffered_text,
                                                        seq,
                                                    ))
                                                    .await;
                                            }
                                            break;
                                        }

                                        decorated.push_str(pattern[idx % 2]);
                                        idx += 1;

                                        // [BACKPRESSURE]
                                        // Use try_send instead of send.await to drop decoration frames if the main loop
                                        // is busy (e.g. processing deletions). This prevents "event bursts" after a lag spike.
                                        // We don't care about skipping a few steps of "…" animation.
                                        let seq =
                                            sequence_counter_clone.fetch_add(1, Ordering::SeqCst);
                                        match tx_decoration.try_send(SttEvent::PartialResult(
                                            decorated.clone(),
                                            seq,
                                        )) {
                                            Ok(_) => {}
                                            Err(mpsc::error::TrySendError::Full(_)) => {
                                                log::debug!("[Decoration] Channel full - skipping animation frame (backpressure)");
                                                // Do not increment seq? No, we already incremented. It's fine, the gap will be ignored by coalescing.
                                                // Actually coalescing checks `> latest`, so gaps are fine.
                                            }
                                            Err(mpsc::error::TrySendError::Closed(_)) => {
                                                break;
                                            }
                                        }
                                    }
                                });

                                {
                                    let mut guard = decoration_task_clone.lock();
                                    *guard = Some(handle);
                                }
                            }
                            StreamerEvent::SpeechEnd(_) => {
                                // 1. まずフラグを落とす（これにより装飾タスクのループが止まる）
                                is_decorating.store(false, Ordering::SeqCst);

                                // 2. SpeechEnd の時刻を記録（異常検知用）
                                {
                                    let mut guard = last_speech_end_time.lock();
                                    *guard = Some(std::time::Instant::now());
                                }

                                // 3. セッションを無効化（既存のタスクが次に進むのを防ぐ）
                                session_counter.fetch_add(1, Ordering::SeqCst);

                                // 4. バッファにあれば送信（ロックを解放してからawait）
                                let buffered_text_option = {
                                    let mut guard = partial_result_buffer.lock();
                                    guard.take()
                                };
                                if let Some(buffered_text) = buffered_text_option {
                                    let seq = sequence_counter.fetch_add(1, Ordering::SeqCst);
                                    let _ = tx_out
                                        .send(SttEvent::PartialResult(buffered_text, seq))
                                        .await;
                                }

                                // 5. 装飾タスクを回収
                                let task_to_wait = {
                                    let mut guard = decoration_task.lock();
                                    guard.take()
                                };
                                if let Some(task) = task_to_wait {
                                    task.abort();
                                    let _ = task.await;
                                }
                            }
                            StreamerEvent::PartialResult(text) => {
                                // 装飾中はバッファに保存（破棄せず、SpeechEnd 時にフラッシュ）
                                if is_decorating.load(Ordering::SeqCst) {
                                    let mut guard = partial_result_buffer.lock();
                                    *guard = Some(text);
                                    continue;
                                }

                                let seq = sequence_counter.fetch_add(1, Ordering::SeqCst);
                                log::info!(
                                    "[WinInputDebug] Sending PartialResult: len={} seq={}",
                                    text.len(),
                                    seq
                                );
                                let _ = tx_out.send(SttEvent::PartialResult(text, seq)).await;
                                log::info!("[WinInputDebug] Sent PartialResult: seq={}", seq);
                            }
                            StreamerEvent::FinalResult(text) => {
                                // 1. フラグを強制的に落とし、セッションを無効化
                                is_decorating.store(false, Ordering::SeqCst);
                                session_counter.fetch_add(1, Ordering::SeqCst);

                                // 2. バッファをクリア（FinalResult が優先されるため）
                                {
                                    let mut guard = partial_result_buffer.lock();
                                    *guard = None;
                                }

                                // 2. 装飾タスクを回収
                                let task_to_cleanup = {
                                    let mut guard = decoration_task.lock();
                                    guard.take()
                                };
                                if let Some(task) = task_to_cleanup {
                                    task.abort();
                                    let _ = task.await;
                                }

                                // 3. 最終テキストから装飾を除去
                                let cleaned = text.replace(" … ?", "").replace(" … ", "");

                                // 4. ここで少し待機して、チャネル内の古いメッセージが処理されるのを待つか、
                                // もしくは単にそのまま送る（上記手順で混入は激減します）
                                let seq = sequence_counter.fetch_add(1, Ordering::SeqCst);
                                log::info!(
                                    "[WinInputDebug] Sending FinalResult: len={} seq={}",
                                    cleaned.len(),
                                    seq
                                );
                                let _ = tx_out.send(SttEvent::FinalResult(cleaned, seq)).await;
                                log::info!("[WinInputDebug] Sent FinalResult: seq={}", seq);
                            }
                            StreamerEvent::PostCorrectionStarted => {
                                let _ = tx_out.send(SttEvent::PostCorrectionStarted).await;
                            }
                            StreamerEvent::PostCorrectionFinished => {
                                let _ = tx_out.send(SttEvent::PostCorrectionFinished).await;
                            }
                        }
                    }
                }));
            } else {
                // start() が2回目以降呼ばれた場合など、すでにタスクが走っているか、あるいは
                // rx が None になっている（既にtakeされた）状態。
                // 今回の実装では stop() でアボートするので、再 start() のためには
                // rx を戻すか、init 相当のことをするかが必要だが、
                // 現状の設計では "stop -> start" で再利用する場合、
                // rx が take されてしまっているので再起動できない恐れがある。
                //
                // したがって、stop() 時に rx を戻すような設計にするか、
                // あるいは「このタスクは一度起動したら走り続ける」設計にするか。
                // 元のコードでは init_audio で spawn していたので「走り続ける」設計だった。
                //
                // ここでは「走り続ける」設計を維持するため、
                // event_listener_task が None の時だけ spawn するようにし、
                // stop() ではこのタスクを abort *しない* （または abort した場合、rx を戻す必要がある）
                //
                // しかし stop() で「イベントリスナータスク」を止めないと、バックグラウンドで動き続けてしまう。
                // ユーザー要望の「stop/start」サイクルを正しく動かすには、
                // rx を Arc<Mutex<Receiver>> で包んで clone 可能にするか（mpsc Receiver は clone 不可）、
                // または stop() 時に rx を再び event_rx に戻す等の工夫が必要。
                //
                // ここではシンプルに、「event_listener_task はアプリ生存期間中（あるいは init_audio 後）ずっと生きていて良い」
                // と判断するか？ -> いや、元のコードでも tokio::spawn していたので、プロセス終了まで生き続けていた（もしくはチャネルが閉じるまで）。
                //
                // 今回の変更で `start()` に移動させたので、「start() のたびに spawn」されることになる。
                // しかし `rx` は `take()` してしまうとなくなる。
                //
                // 解決策:
                // `event_rx` にある Receiver を取り出してタスクを起動するが、
                // タスク終了時（ループを抜けた時）に再び返却するのは難しい。
                //
                // 最も安全なのは、「イベントリスナータスクも start/stop に追従させる」こと。
                // そのためには、Receiver を start() で消費してしまうと次がない。
                //
                // 修正方針:
                // mpsc::Receiver は Move 必須。
                // したがって、一度 start したらその task が rx を所有し続ける。
                // stop() で task を abort すると rx もドロップされてしまう。
                //
                // これを防ぐには、
                // A. `stop()` で event_listener_task を **abort しない**。
                //    (ストリーマー自体が止まればイベントは来なくなるので、タスクは待機状態になるだけ。リソース消費は微小)
                //    -> これが最も元の挙動（init_audio で spawn して放置）に近い。
                //    -> ただし `start` が複数回呼ばれた時に多重起動しないガードが必要。
                //
                // B. `start` のたびに新しい channel を作る -> init_audio で channel を作っているので無理。
                //
                // 今回は A案（タスクは一度起動したら維持する）を採用する。
                // ただし、タスク起動タイミングは「最初の start()」とする。

                if self.event_listener_task.is_none() {
                    log::warn!("[OpenAIRecognizer] Event listener task is not running, and event_rx is empty. This suggests a re-start after stop, but rx is gone. This needs fix if strict restart is required. However, assuming 'start called once or task stays alive'.");
                } else {
                    log::debug!("[OpenAIRecognizer] Event listener task is already running.");
                }
            }

            // Windows/Mac: ネイティブキャプチャを開始
            #[cfg(any(target_os = "windows", target_os = "macos"))]
            {
                if let Some(rx) = start_native_audio_capture() {
                    log::info!("[OpenAI] Native audio capture started.");

                    // 短い遅延の後に開始音の通知を送信
                    // この遅延は、無線ヘッドセットがスリープから復帰して音を完全に再生するのを助けます。
                    let tx_ready = self.tx.clone();
                    async_runtime::spawn(async move {
                        time::sleep(Duration::from_millis(OPENAI_READY_DELAY_MS)).await;
                        let _ = tx_ready.try_send(SttEvent::Ready);
                    });

                    let audio_buf = Arc::clone(&self.audio_buf);
                    let is_running = Arc::clone(&self.is_running);

                    let sample_rate_atomic = Arc::clone(&self.sample_rate);

                    // キャプチャデータを audio_buf に流し込むタスク
                    self.capture_task = Some(async_runtime::spawn(async move {
                        let mut rx = rx;
                        let mut counter: u64 = 0;
                        while is_running.load(Ordering::SeqCst) {
                            // On Windows rx.recv() returns Option<Vec<f32>>
                            // On Mac rx.recv() returns Option<(Vec<f32>, u32)>

                            let payload = rx.recv().await.map(|(data, rate)| {
                                // Update rate if changed
                                let current = sample_rate_atomic.load(Ordering::Relaxed);
                                if current != rate {
                                    sample_rate_atomic.store(rate, Ordering::Relaxed);
                                    log::info!("[OpenAI] Sample rate changed to {}Hz", rate);
                                }
                                data
                            });

                            if let Some(data) = payload {
                                counter += 1;
                                if counter % 50 == 0 {
                                    log::debug!("[OpenAI] Capture Task: Pushing native chunk #{} to audio_buf. Len={}", counter, data.len());
                                }
                                let mut guard = audio_buf.lock();
                                guard.extend_from_slice(&data);
                            } else {
                                log::warn!("[OpenAI] Native capture channel closed.");
                                break;
                            }
                        }
                        log::debug!("[OpenAI] Capture task finished.");
                    }));
                } else {
                    log::error!("[OpenAI] Failed to start native audio capture.");
                }
            }

            // 高精度ティッカータスクを起動
            let streamer_clone = Arc::clone(&self.streamer);
            let is_running_clone = Arc::clone(&self.is_running);
            let audio_buf_clone = Arc::clone(&self.audio_buf);
            let sample_rate_atomic = Arc::clone(&self.sample_rate);

            self.ticker_task = Some(async_runtime::spawn(async move {
                log::debug!("[OpenAIRecognizer] Background ticker started (20ms interval)");
                let interval = Duration::from_millis(20);
                while is_running_clone.load(Ordering::SeqCst) {
                    let sample_rate = sample_rate_atomic.load(Ordering::Relaxed);
                    // 1. audio_buf からデータを取り出してストリーマーに push
                    let samples = {
                        let mut buf_guard = audio_buf_clone.lock();
                        std::mem::take(&mut *buf_guard)
                    };

                    if !samples.is_empty() {
                        let mut streamer_guard = streamer_clone.lock();
                        if let Some(ref mut streamer) = *streamer_guard {
                            // log::trace!("[OpenAI] Ticker feeding {} samples to streamer", samples.len());
                            streamer.push_samples(&samples, sample_rate);
                        }
                    }

                    // 2. ストリーマーの tick を呼び出し
                    {
                        let mut streamer_guard = streamer_clone.lock();
                        if let Some(ref mut streamer) = *streamer_guard {
                            streamer.tick();
                        } else {
                            break;
                        }
                    }
                    tokio::time::sleep(interval).await;
                }
                log::debug!("[OpenAIRecognizer] Background ticker stopped");
            }));
        }
    }

    /// 認識を停止
    pub fn stop(&mut self) {
        self.is_running.store(false, Ordering::SeqCst);
        self.is_decorating.store(false, Ordering::SeqCst);
        // セッションを無効化（装飾タスクなどのループを止める）
        self.session_counter.fetch_add(1, Ordering::SeqCst);

        // ネイティブのキャプチャ（オーディオユニット）を真っ先に止める
        stop_native_audio_capture();

        // タスクを明示的にキャンセル（sleep 中の待ち時間をスキップ）
        if let Some(task) = self.ticker_task.take() {
            task.abort();
        }

        // 装飾タスクも明示的にキャンセル
        let decoration_task = {
            let mut guard = self.decoration_task.lock();
            guard.take()
        };
        if let Some(task) = decoration_task {
            task.abort();
        }

        // 重要: event_listener_task は停止しない！
        // 理由: このタスクが保持している rx_streamer (Receiver) は消費アイテムであり、
        //       一度タスクを abort してしまうと rx も drop され、二度とイベントを受け取れなくなる。
        //       ストリーマー自体が stop() されればイベントは来なくなるため、
        //       このタスクは単に次の start() まで待機（await）し続けるだけであり、リソース消費は極小である。
        //
        // if let Some(task) = self.event_listener_task.take() {
        //     task.abort();
        // }

        let mut guard = self.streamer.lock();
        if let Some(ref mut streamer) = *guard {
            streamer.stop();
        }

        #[cfg(any(target_os = "windows", target_os = "macos"))]
        {
            stop_native_audio_capture();
            if let Some(task) = self.capture_task.take() {
                task.abort();
            }
        }
    }

    /// メインループから呼ばれる処理 (独立タスク化したため、基本的には何もしない)
    pub fn tick(&mut self) {
        // バックグラウンドタスクが自動で tick するようになったため、外部からの呼び出しは無視する
        // または、将来的なデバッグや同期的な強制実行用として残しておく
    }

    /// ロケールを設定
    pub fn set_locale(&mut self, locale: LocaleCode) {
        // recognizer.rs 側で既に *self.language.lock() = locale されているはずだが、
        // 念のため再設定しても問題ない
        *self.language.lock() = locale;

        log::debug!("[OpenAIRecognizer] Language updated to: {:?}", locale);

        let mut guard = self.streamer.lock();
        if let Some(ref mut streamer) = *guard {
            let s_locale = match locale {
                LocaleCode::En => StreamerLocale::En,
                _ => StreamerLocale::Ja,
            };
            streamer.set_locale(s_locale);
        }
    }

    /// 実行中かどうか
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }
}

unsafe impl Send for OpenAIRecognizer {}
unsafe impl Sync for OpenAIRecognizer {}
