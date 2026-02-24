//! Windows Native STT Backend using C# SpeechHelper.dll (Native AOT)
//!
//! This module provides speech recognition on Windows by dynamically loading
//! the SpeechHelper.dll built from the C# project.

use crate::stt_config::LocaleCode;
use crate::stt_config::SttSettings;
use crate::tools::post_correction_processor::{
    PostCorrectionBackend, PostCorrectionConfig, PostCorrectionProcessor, ProcessorOutput,
    SttModelType,
};
use crate::tools::punctuation_machine::PunctuationMachine;
use crate::tools::resampler::{InternalResampler, SincResampler};
use crate::tools::vad_processor::{VadConfig, VadProcessor, VAD_SAMPLE_RATE};
use crate::types::SttEvent;
use std::ffi::{c_char, c_int, CStr, CString};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::{self, Sender, UnboundedReceiver, UnboundedSender};

// Link to the C# dynamic library (DLL)
// The actual linking configuration is handled in build.rs
#[link(name = "SpeechHelper")] // Defaults to dylib on Windows if not specified
extern "C" {
    fn speech_helper_init(speech_timeout_sec: f64) -> c_int;
    fn speech_helper_set_result_callback(callback: extern "C" fn(*const c_char, c_int));
    fn speech_helper_set_error_callback(callback: extern "C" fn(*const c_char));
    fn speech_helper_set_ready_callback(callback: extern "C" fn());
    fn speech_helper_set_audio_data_callback(callback: Option<extern "C" fn(*const f32, u32, u32)>);
    fn speech_helper_start_capture() -> c_int;
    fn speech_helper_stop_capture();
    fn speech_helper_start(locale: *const c_char) -> c_int;
    fn speech_helper_stop();
    fn speech_helper_cleanup();
    fn speech_helper_tick();
}

// Global channel for sending events from callbacks (same pattern as Swift backend)
lazy_static::lazy_static! {
    static ref WIN_GLOBAL_TX: Mutex<Option<Sender<SttEvent>>> = Mutex::new(None);
    static ref WIN_GLOBAL_SEQ: AtomicU64 = AtomicU64::new(0);
    static ref WIN_GLOBAL_PUNCH: Mutex<Option<PunctuationMachine>> = Mutex::new(None);
    static ref WIN_CURRENT_LOCALE: Mutex<LocaleCode> = Mutex::new(LocaleCode::Ja);
    static ref WIN_AUDIO_SENDER: Mutex<Option<UnboundedSender<(Vec<f32>, u32)>>> = Mutex::new(None);
}

// Debug counter for Rust side
static WIN_DEBUG_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Callback function for receiving raw audio data from C#
extern "C" fn win_audio_data_callback(samples: *const f32, count: u32, sample_rate: u32) {
    if samples.is_null() || count == 0 {
        return;
    }

    if let Ok(mut guard) = WIN_AUDIO_SENDER.lock() {
        let mut failed = false;
        if let Some(ref tx) = *guard {
            // Safety: The pointer is valid for 'count' elements as guaranteed by C# side (GC pinned)
            let slice = unsafe { std::slice::from_raw_parts(samples, count as usize) };
            let data = slice.to_vec();

            // Debug logging periodically
            let counter = WIN_DEBUG_COUNTER.fetch_add(1, Ordering::Relaxed);
            if counter % 100 == 0 {
                let max_amp = data.iter().fold(0.0f32, |acc, &x| acc.max(x.abs()));
                log::debug!(
                    "[Win] Received native audio: {} samples, rate={}, max_amp={:.6}",
                    count,
                    sample_rate,
                    max_amp
                );
            }

            // Send data to Rust aggregator
            if let Err(e) = tx.send((data, sample_rate)) {
                // Log only on error if needed, but avoid spamming
                if counter % 100 == 0 {
                    log::warn!(
                        "[Win] Failed to send audio data to channel: {}. Sender cleared.",
                        e
                    );
                }
                failed = true;
            }
        }
        if failed {
            *guard = None;
        }
    }
}

/// Start native audio capture and return a receiver for audio data
pub fn start_native_audio_capture() -> Option<UnboundedReceiver<(Vec<f32>, u32)>> {
    log::debug!("[Win] Starting native audio capture...");
    let (tx, rx) = mpsc::unbounded_channel();

    if let Ok(mut guard) = WIN_AUDIO_SENDER.lock() {
        *guard = Some(tx);
    }

    unsafe {
        speech_helper_set_audio_data_callback(Some(win_audio_data_callback));
        let ret = speech_helper_start_capture();
        if ret != 0 {
            log::error!("[Win] Failed to start audio capture: {}", ret);
            return None;
        }
    }

    Some(rx)
}

/// Stop native audio capture
pub fn stop_native_audio_capture() {
    log::debug!("[Win] Stopping native audio capture...");
    unsafe {
        speech_helper_stop_capture();
        speech_helper_set_audio_data_callback(None);
    }

    if let Ok(mut guard) = WIN_AUDIO_SENDER.lock() {
        *guard = None;
    }
}

/// Callback function for receiving ready signal from C#
extern "C" fn win_ready_callback() {
    log::debug!("[Win] Received READY signal from native layer");

    if let Ok(guard) = WIN_GLOBAL_TX.lock() {
        if let Some(ref tx) = *guard {
            let _ = tx.try_send(SttEvent::Ready);
        }
    }
}

/// Callback function for receiving transcription results from C#
extern "C" fn win_result_callback(text: *const c_char, is_final: c_int) {
    if text.is_null() {
        return;
    }

    if let Ok(guard) = WIN_GLOBAL_TX.lock() {
        if let Some(ref tx) = *guard {
            let c_str = unsafe { CStr::from_ptr(text) };
            if let Ok(s) = c_str.to_str() {
                // No punctuation here (Moved to Ticker with Semantic Anchoring)
                let processed_text = s.to_string();

                log::debug!(
                    "[Win] Received text: {}, final: {}",
                    processed_text,
                    is_final
                );
                let seq = WIN_GLOBAL_SEQ.fetch_add(1, Ordering::SeqCst);
                let event = if is_final != 0 {
                    SttEvent::FinalResult(processed_text, seq)
                } else {
                    SttEvent::PartialResult(processed_text, seq)
                };
                if let Err(e) = tx.try_send(event) {
                    log::error!("[Win] Failed to send event: {}", e);
                }
            }
        }
    }
}

/// Callback function for receiving errors from C#
extern "C" fn win_error_callback(error: *const c_char) {
    if error.is_null() {
        return;
    }

    if let Ok(guard) = WIN_GLOBAL_TX.lock() {
        if let Some(ref tx) = *guard {
            let c_str = unsafe { CStr::from_ptr(error) };
            if let Ok(s) = c_str.to_str() {
                // "COMPLETED:" で始まるメッセージは正常終了通知なので、エラーではなく INFO ログとして扱う
                if s.starts_with("COMPLETED:") {
                    log::info!("[Win] Native session completed: {}", s);
                    return;
                }

                log::error!("[Win] Received error: {}", s);
                let _ = tx.try_send(SttEvent::Error(s.to_string()));
            }
        }
    }
}

/// Windows Speech Recognition Backend
pub struct WinSpeechBackend {
    is_running: Arc<AtomicBool>,
    locale: Arc<parking_lot::Mutex<LocaleCode>>,
    /// Post correction processor (shared with ticker task)
    post_correction_processor: Arc<parking_lot::Mutex<Option<PostCorrectionProcessor>>>,
    /// 発話状態
    is_speaking: Arc<AtomicBool>,
    /// 共通 VAD プロセッサ
    vad_processor: Arc<parking_lot::Mutex<Option<VadProcessor>>>,
    /// VAD設定
    stt_settings: Option<SttSettings>,
    /// Internal receiver for raw events (shared with ticker task)
    rx_raw: Arc<parking_lot::Mutex<Option<mpsc::Receiver<SttEvent>>>>,
    /// Sender for final app events
    tx_app: Sender<SttEvent>,
    /// Background ticker task handle
    ticker_task: Option<tokio::task::JoinHandle<()>>,
    /// リサンプラー (動的レート -> 16kHz VAD用)
    resampler: Arc<parking_lot::Mutex<Option<SincResampler>>>,
}

impl WinSpeechBackend {
    /// Create a new Windows speech backend.
    pub fn new(
        tx: Sender<SttEvent>,
        shared_locale: Arc<parking_lot::Mutex<LocaleCode>>,
        backend: Option<Arc<dyn PostCorrectionBackend>>,
        pc_config: Option<PostCorrectionConfig>,
        replaces: Vec<(String, String)>,
        stt_settings: Option<SttSettings>,
    ) -> Result<Self, String> {
        // Create internal channel for raw events
        let (tx_raw, rx_raw) = mpsc::channel(100);

        // 共有される発話状態の初期化
        let is_speaking = Arc::new(AtomicBool::new(false));

        // Store internal sender globally for callbacks
        if let Ok(mut guard) = WIN_GLOBAL_TX.lock() {
            *guard = Some(tx_raw);
        }

        // Initialize processor if backend is provided
        // IMPORTANT: Use UseOnlineModel for Windows OS speech engine
        // The OS engine sends cumulative (all-history) text and may backtrack/rewrite past content.
        let post_correction_processor = if let (Some(b), Some(c)) = (backend, pc_config) {
            log::debug!("[Win] PostCorrectionProcessor enabled with UseOnlineModel");
            Some(PostCorrectionProcessor::with_model_type(
                b,
                c,
                SttModelType::UseOnlineModel,
                replaces,
                is_speaking.clone(),
            ))
        } else {
            log::debug!("[Win] PostCorrectionProcessor disabled (no backend/config)");
            None
        };

        // Initialize PunctuationMachine globally if not already
        if let Ok(mut guard) = WIN_GLOBAL_PUNCH.lock() {
            if guard.is_none() {
                match PunctuationMachine::new() {
                    Ok(pm) => *guard = Some(pm),
                    Err(e) => log::warn!("[Win] Failed to init PunctuationMachine: {}", e),
                }
            }
        }

        // Set initial global locale
        if let Ok(mut guard) = WIN_CURRENT_LOCALE.lock() {
            *guard = locale;
        }

        log::debug!("[Win] Initializing speech helper (Dynamic Link)...");

        // Initialize Native AOT library
        unsafe {
            use crate::constants::SPEECH_TIMEOUT_SEC;
            let result = speech_helper_init(SPEECH_TIMEOUT_SEC);
            if result != 0 {
                return Err(format!("speech_helper_init failed with code: {}", result));
            }

            // Set callbacks
            speech_helper_set_result_callback(win_result_callback);
            speech_helper_set_error_callback(win_error_callback);
            speech_helper_set_ready_callback(win_ready_callback);
        }

        log::debug!("[Win] SpeechHelper initialized successfully");

        Ok(Self {
            is_running: Arc::new(AtomicBool::new(false)),
            locale: shared_locale,
            post_correction_processor: Arc::new(parking_lot::Mutex::new(post_correction_processor)),
            is_speaking,
            vad_processor: Arc::new(parking_lot::Mutex::new(None)),
            stt_settings,
            rx_raw: Arc::new(parking_lot::Mutex::new(Some(rx_raw))),
            tx_app: tx,
            ticker_task: None,
            resampler: Arc::new(parking_lot::Mutex::new(None)),
        })
    }

    /// Start speech recognition
    pub fn start(&mut self) {
        if self.is_running.load(Ordering::SeqCst) {
            log::debug!("[Win] Speech recognition already running");
            return;
        }

        self.is_running.store(true, Ordering::SeqCst);

        // Reset processor state at session start to avoid stale data/flags
        {
            let mut proc_guard = self.post_correction_processor.lock();
            if let Some(ref mut proc) = *proc_guard {
                proc.reset();
            }
        }

        let c_locale = CString::new(self.locale.as_str()).unwrap();

        unsafe {
            let result = speech_helper_start(c_locale.as_ptr());
            if result != 0 {
                log::error!("[Win] speech_helper_start failed with code: {}", result);
                self.is_running.store(false, Ordering::SeqCst);

                // Send error event
                if let Ok(guard) = WIN_GLOBAL_TX.lock() {
                    if let Some(ref tx) = *guard {
                        let _ = tx.try_send(SttEvent::Error(format!(
                            "Windows speech recognition failed to start (code: {})",
                            result
                        )));
                    }
                }
                return;
            } else {
                log::debug!(
                    "[Win] Speech recognition started (locale: {:?})",
                    self.locale
                );
            }
        }

        // Initialize VadProcessor if settings and models are available
        if let Some(settings) = &self.stt_settings {
            let mut vp_guard = self.vad_processor.lock();
            if vp_guard.is_none() {
                use crate::stt_config::VadType as SttConfigVadType;
                use crate::tools::vad_processor::VadType as CommonVadType;

                let vad_type = match settings.vad_type {
                    SttConfigVadType::Silero | SttConfigVadType::SileroInt8 => {
                        CommonVadType::Silero
                    }
                    SttConfigVadType::Ten | SttConfigVadType::TenInt8 => CommonVadType::Ten,
                };

                let vad_config = VadConfig {
                    vad_type,
                    model_path: settings.get_vad_path(),
                    threshold: settings.vad_threshold,
                    min_silence_duration: settings.vad_min_silence_duration,
                    min_speech_duration: settings.vad_min_speech_duration,
                    max_speech_duration: settings.vad_max_speech_duration,
                    num_threads: settings.num_threads,
                };

                log::debug!(
                    "[Win] Initializing VadProcessor for OS mode (type: {:?})",
                    vad_type
                );

                match VadProcessor::new(vad_config, self.is_speaking.clone()) {
                    Ok(vp) => {
                        *vp_guard = Some(vp);
                    }
                    Err(e) => {
                        log::error!("[Win] Failed to initialize VadProcessor: {}", e);
                    }
                }
            }
        }

        // Start native audio capture to feed VadProcessor
        let mut rx_audio = start_native_audio_capture();

        // Spawn autonomous background ticker
        let is_running = self.is_running.clone();
        let rx_raw = self.rx_raw.clone();
        let processor = self.post_correction_processor.clone();
        let vad_processor = self.vad_processor.clone();
        let tx_app = self.tx_app.clone();
        let resampler = self.resampler.clone();

        // [Defensive] Drain any old events left in the channel from previous session
        {
            let mut rx_guard = rx_raw.lock();
            if let Some(ref mut rx) = *rx_guard {
                let mut drained = 0;
                while let Ok(_) = rx.try_recv() {
                    drained += 1;
                }
                if drained > 0 {
                    log::warn!(
                        "[Win] Drained {} stale events from previous session",
                        drained
                    );
                }
            }
        }

        self.ticker_task = Some(tokio::spawn(async move {
            let interval = tokio::time::Duration::from_millis(50);
            log::debug!("[Win] Background ticker started");
            let mut last_processed_seq = 0u64;
            let mut watermark_len: usize = 0;
            let mut current_raw_char_count: usize = 0;
            let mut current_seq: u64 = 0;

            // Timeout tracking
            let mut last_received_time = tokio::time::Instant::now();
            let mut last_processed_text: Option<String> = None;
            let mut processed_timeout_seq: Option<u64> = None; // ONE-SHOT trigger tracking
            let timeout_duration = tokio::time::Duration::from_millis(1000);

            loop {
                if !is_running.load(Ordering::SeqCst) {
                    break;
                }

                // Collect audio data for VAD
                if let Some(ref mut rx) = rx_audio {
                    while let Ok((samples, rate)) = rx.try_recv() {
                        let mut res_guard = resampler.lock();

                        // Check if resampler needs (re)initialization
                        let needs_init = match *res_guard {
                            Some(ref res) => res.input_rate() != rate,
                            None => true,
                        };

                        if needs_init && rate > 0 {
                            log::info!(
                                "[Win] (Re)initializing resampler: {}Hz -> {}Hz",
                                rate,
                                VAD_SAMPLE_RATE
                            );
                            *res_guard = SincResampler::new(rate, VAD_SAMPLE_RATE as u32).ok();
                        }

                        // Apply resampling if rate doesn't match VAD_SAMPLE_RATE
                        let samples_to_process = if rate != VAD_SAMPLE_RATE as u32 {
                            if let Some(ref mut res) = *res_guard {
                                res.process(&samples).unwrap_or(samples)
                            } else {
                                samples
                            }
                        } else {
                            samples
                        };

                        let vp_guard = vad_processor.lock();
                        if let Some(ref vp) = *vp_guard {
                            vp.accept_waveform(&samples_to_process);
                        }
                    }
                }

                // Collect events from internal channel (drop guard before processing)
                let raw_events: Vec<SttEvent> = {
                    let mut rx_guard = rx_raw.lock();
                    if let Some(ref mut rx) = *rx_guard {
                        let mut collected = Vec::new();
                        while let Ok(event) = rx.try_recv() {
                            collected.push(event);
                        }
                        collected
                    } else {
                        Vec::new()
                    }
                };

                // --- Coalescing: Keep only the latest STT result, but keep all other control events ---
                let mut latest_stt: Option<SttEvent> = None;
                let mut control_events = Vec::new();
                let mut has_new_stt_event = false;

                for event in raw_events {
                    match event {
                        SttEvent::PartialResult(_, seq) | SttEvent::FinalResult(_, seq) => {
                            has_new_stt_event = true;
                            if seq >= last_processed_seq {
                                let is_newer = if let Some(ref current) = latest_stt {
                                    match current {
                                        SttEvent::PartialResult(_, s)
                                        | SttEvent::FinalResult(_, s) => seq >= *s,
                                        _ => true,
                                    }
                                } else {
                                    true
                                };
                                if is_newer {
                                    latest_stt = Some(event);
                                } else {
                                    log::debug!("[Win] Coalescing: dropping older seq={}", seq);
                                }
                            } else {
                                log::debug!(
                                    "[Win] Discarding stale seq={} (last={})",
                                    seq,
                                    last_processed_seq
                                );
                            }
                        }
                        other => control_events.push(other),
                    }
                }

                // 1. Process control events first (Stopped, Error, etc.)

                for event in control_events {
                    let _ = tx_app.try_send(event);
                }

                if has_new_stt_event {
                    last_received_time = tokio::time::Instant::now();
                }

                // Determine if we should process:
                // 1. New STT event available
                // 2. Timeout triggered (no new event for > 1000ms, AND we have unprocessed pending text)

                let is_timeout = latest_stt.is_none()
                    && last_received_time.elapsed() >= timeout_duration
                    && processed_timeout_seq != Some(last_processed_seq); // Check if already triggered for this seq

                let event_to_process = if let Some(evt) = latest_stt {
                    Some(evt)
                } else if is_timeout {
                    // Check if we need to force-flush pending text with punctuation
                    if let Some(ref last_text) = last_processed_text {
                        // Re-process the last known text with timeout flag
                        // Use strict sequence consistency: reuse last_processed_seq
                        // Note: We treat this as a PartialResult to prompt update
                        log::debug!(
                            "[Win] Timeout triggered (>1000ms). Re-processing for punctuation."
                        );

                        // Update timestamp (optional but good for safety)
                        last_received_time = tokio::time::Instant::now();
                        // Mark this sequence as PROCESSED for timeout
                        processed_timeout_seq = Some(last_processed_seq);

                        Some(SttEvent::PartialResult(
                            last_text.clone(),
                            last_processed_seq,
                        ))
                    } else {
                        None
                    }
                } else {
                    None
                };

                // 2. Process latest STT result (or Timeout replay) with WATERMARK synchronization
                if let Some(event) = event_to_process {
                    let (raw_text, seq, is_final_event) = match event {
                        SttEvent::PartialResult(t, s) => (t, s, false),
                        SttEvent::FinalResult(t, s) => (t, s, true),
                        _ => unreachable!(),
                    };

                    // Update last processed text reference
                    last_processed_text = Some(raw_text.clone());

                    let raw_char_count = raw_text.chars().count();
                    log::debug!(
                        "[Win] Processing seq={} (raw_chars={}, watermark={})",
                        seq,
                        raw_char_count,
                        watermark_len
                    );
                    last_processed_seq = seq;

                    // ===========================================================
                    // WATERMARK LOGIC: Extract only the UNCONFIRMED slice
                    // ===========================================================
                    // If the engine backtracks (raw_char_count < watermark_len),
                    // we simply wait until it grows past the watermark again.
                    // We NEVER reset watermark to 0 because the text before it
                    // has already been committed and typed into the app.
                    if raw_char_count < watermark_len {
                        log::warn!("[Win] Engine backtracked! raw_chars={} < watermark={}. Waiting for engine to move forward.", raw_char_count, watermark_len);
                    }

                    // ===========================================================
                    // SEMANTIC-ANCHORED WINDOWING SYSTEM
                    // ===========================================================
                    // Instead of feeding pure raw text, we construct a "Semantic Window"
                    // anchored by the last confirmed punctuation.
                    // This ensures the PunctuationMachine sees valid sentence context.

                    let confirmed_text: String = raw_text.chars().take(watermark_len).collect();

                    // Search for the last "Semantic Anchor" (Sentence End) in the confirmed history
                    // We look for [。？！？!]
                    let anchor_char_pos = confirmed_text
                        .chars()
                        .collect::<Vec<_>>()
                        .iter()
                        .rposition(|c| ['。', '？', '！', '!', '?'].contains(c))
                        .map(|i| i + 1) // Start AFTER the punctuation
                        .unwrap_or(0); // If none found, start from 0

                    let context_slice: String = raw_text
                        .chars()
                        .skip(anchor_char_pos)
                        .take(watermark_len - anchor_char_pos)
                        .collect();
                    let raw_unconfirmed: String = raw_text.chars().skip(watermark_len).collect();

                    // Apply punctuation to the unconfirmed part, using the confirmed part as context
                    let unconfirmed_slice = {
                        let locale = WIN_CURRENT_LOCALE
                            .lock()
                            .map(|g| *g)
                            .unwrap_or(LocaleCode::Ja);
                        if let Ok(punch_guard) = WIN_GLOBAL_PUNCH.lock() {
                            if let Some(ref machine) = *punch_guard {
                                // insert_with_context returns ONLY the punctuated 'target' part
                                // ALLOW TERMINAL PUNCTUATION if timeout triggered
                                machine
                                    .insert_with_context(
                                        &raw_unconfirmed,
                                        &context_slice,
                                        &locale,
                                        is_timeout,
                                    )
                                    .unwrap_or_else(|e| {
                                        log::warn!("[Win] Punctuation failed: {}", e);
                                        raw_unconfirmed.to_string()
                                    })
                            } else {
                                raw_unconfirmed.to_string()
                            }
                        } else {
                            raw_unconfirmed.to_string()
                        }
                    };

                    log::debug!("[Win] Windowing: Anchor={}, Context='{}', RawTarget='{}' -> Punctuated='{}'", 
                        anchor_char_pos, context_slice, raw_unconfirmed, unconfirmed_slice);

                    let output = {
                        let mut proc_guard = processor.lock();
                        if let Some(ref mut proc) = *proc_guard {
                            proc.process_input(&unconfirmed_slice)
                        } else {
                            None
                        }
                    };

                    if let Some(output) = output {
                        log::debug!("[Win] Processor Output: {:?}", output);
                        match output {
                            ProcessorOutput::Partial(corrected) => {
                                // ===========================================================
                                // [Windows専用] 句読点装飾（Partial のみ）
                                // ===========================================================
                                // これは「リアルタイム表示を読みやすくするための装飾」であり、
                                // 正確な句読点挿入ではありません。あくまでユーザー体験向上のための
                                // Windows 専用の処理です。
                                //
                                // 【重要】Final（LLM補正結果）には絶対に適用しないこと。理由:
                                // 1. LLM は文脈を完璧に理解して句読点を打つため、品質が高い
                                // 2. 装飾を被せると二重句読点「。。」などが発生する恐れがある
                                // 3. Final は「システムの最終決定」であり、純粋な LLM 出力であるべき
                                // ===========================================================
                                let decorated = corrected.clone(); // NO Decoration (Punctuation handled upstream)
                                let _ = tx_app.try_send(SttEvent::PartialResult(decorated, seq));
                            }
                            ProcessorOutput::Final(corrected) => {
                                // ===========================================================
                                // ADVANCE WATERMARK: Mark current raw_text as consumed
                                // ===========================================================
                                // 【重要】Final は LLM の補正結果なので、句読点装飾は一切行わない。
                                // LLM が打った句読点がそのまま最終出力となる。
                                watermark_len = raw_char_count;
                                log::debug!("[Win] Watermark advanced to {}", watermark_len);
                                let _ = tx_app.try_send(SttEvent::FinalResult(corrected, seq));
                            }
                        }
                    } else {
                        // Passthrough (processor is None)
                        let guard = processor.lock();
                        let has_processor = guard.is_some();
                        if !has_processor {
                            if is_final_event {
                                let _ = tx_app.try_send(SttEvent::FinalResult(raw_text, seq));
                            } else {
                                // ===========================================================
                                // [Windows専用] Passthrough 時も Partial には装飾を適用
                                // ===========================================================
                                let decorated = raw_text.clone(); // NO Decoration
                                let _ = tx_app.try_send(SttEvent::PartialResult(decorated, seq));
                            }
                        }
                    }

                    // Keep track of current state for background triggers
                    current_raw_char_count = raw_char_count;
                    current_seq = seq;
                }

                // 3. Try execute pending correction (triggered by silence/timer)
                let (ready_to_correct, text_to_correct, backend) = {
                    let mut proc_guard = processor.lock();
                    if let Some(ref mut proc) = *proc_guard {
                        if proc.check_and_start_silence_timer() {
                            (true, proc.get_text_to_correct(), Some(proc.backend.clone()))
                        } else {
                            (false, String::new(), None)
                        }
                    } else {
                        (false, String::new(), None)
                    }
                };

                if ready_to_correct {
                    if let Some(be) = backend {
                        let _ = tx_app.try_send(SttEvent::PostCorrectionStarted);
                        log::info!(
                            "[Win] Executing pending correction: \"{}\"",
                            text_to_correct
                        );
                        match be.post_correct(&text_to_correct).await {
                            Ok(corrected) => {
                                let output = {
                                    let mut proc_guard = processor.lock();
                                    if let Some(ref mut proc) = *proc_guard {
                                        Some(proc.commit_correction(&corrected))
                                    } else {
                                        None
                                    }
                                };
                                let _ = tx_app.try_send(SttEvent::PostCorrectionFinished);

                                if let Some(ProcessorOutput::Final(final_text)) = output {
                                    log::info!(
                                        "[Win] Pending correction EXECUTED: \"{}\"",
                                        final_text
                                    );
                                    watermark_len = current_raw_char_count;
                                    let _ = tx_app
                                        .try_send(SttEvent::FinalResult(final_text, current_seq));
                                }
                            }
                            Err(e) => {
                                log::error!("[Win] Post correction failed: {}", e);
                                let _ = tx_app.try_send(SttEvent::PostCorrectionFinished);
                            }
                        }
                    }
                }

                // (removed unreferenced output block)

                tokio::time::sleep(interval).await;
            }
            log::debug!("[Win] Background ticker stopped");
        }));
    }

    /// Stop speech recognition
    pub fn stop(&mut self) {
        if !self.is_running.load(Ordering::SeqCst) {
            return;
        }

        self.is_running.store(false, Ordering::SeqCst);
        WIN_GLOBAL_SEQ.store(0, Ordering::SeqCst);

        // ネイティブのキャプチャ（オーディオユニット）を真っ先に止める
        stop_native_audio_capture();

        // Reset processor state at session stop
        {
            let mut proc_guard = self.post_correction_processor.lock();
            if let Some(ref mut proc) = *proc_guard {
                proc.reset();
            }
        }

        // Abort background ticker task
        if let Some(task) = self.ticker_task.take() {
            task.abort();
        }

        unsafe {
            speech_helper_stop();
        }

        log::info!("[Win] stop() called.");
        log::debug!("[Win] Speech recognition stopped");
    }

    /// Update locale for next session
    pub fn set_locale(&mut self, locale: LocaleCode) {
        *self.locale.lock() = locale;
        if let Ok(mut guard) = WIN_CURRENT_LOCALE.lock() {
            *guard = locale;
        }
    }

    /// Cleanup resources
    pub fn cleanup(&self) {
        unsafe {
            speech_helper_cleanup();
        }
    }

    /// Tick (for message pump if needed)
    /// Note: Event processing is now handled by autonomous background task.
    /// This method only calls the native message pump.
    pub fn tick(&mut self) {
        if !self.is_running.load(Ordering::SeqCst) {
            return;
        }

        // Only call native message pump; event processing is now autonomous
        unsafe {
            speech_helper_tick();
        }
    }
}

impl Drop for WinSpeechBackend {
    fn drop(&mut self) {
        self.stop();
        self.cleanup();
        if let Ok(mut guard) = WIN_GLOBAL_TX.lock() {
            *guard = None;
        }
    }
}
