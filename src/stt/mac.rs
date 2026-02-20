//! macOS Native STT Backend using Swift SpeechHelper library
//!
//! This module provides speech recognition on macOS by calling into
//! the Swift helper library via C FFI.
//! Supports both Classic (SFSpeechRecognizer) and Tahoe (macOS 15+) modes.

use crate::stt_config::{LocaleCode, SttEngine};
use crate::tools::post_correction_processor::{
    PostCorrectionBackend, PostCorrectionConfig, PostCorrectionProcessor, ProcessorOutput,
    SttModelType,
};
use crate::types::SttEvent;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;

#[derive(Debug, Clone, Copy, PartialEq)]
enum InternalMacEngine {
    Tahoe,
    Classic,
}

// Link to the Swift helper library
#[link(name = "SpeechHelper")]
extern "C" {
    fn speech_helper_init(speech_timeout_sec: f64) -> i32;
    #[allow(dead_code)]
    fn speech_helper_request_authorization() -> i32;
    fn speech_helper_set_result_callback(callback: extern "C" fn(*const c_char, i32));
    fn speech_helper_set_error_callback(callback: extern "C" fn(*const c_char));
    fn speech_helper_set_ready_callback(callback: extern "C" fn());
    // Native Capture (OpenAI Mode)
    fn speech_helper_set_audio_data_callback(callback: Option<extern "C" fn(*const f32, i32, i32)>);
    fn speech_helper_start_capture() -> i32;
    fn speech_helper_stop_capture();

    // Classic Mode
    fn speech_helper_start(locale: *const c_char) -> i32;
    fn speech_helper_stop();
    fn speech_helper_cleanup();
    fn speech_helper_tick();

    // Tahoe API (macOS 15+)
    fn tahoe_helper_init(locale: *const c_char, speech_timeout_sec: f64) -> i32;
    fn tahoe_helper_start(locale: *const c_char) -> i32;
    fn tahoe_helper_stop();
}

// Global channel for sending events from callbacks (same pattern as Windows backend)
lazy_static::lazy_static! {
    static ref MAC_GLOBAL_TX: std::sync::Mutex<Option<mpsc::Sender<SttEvent>>> = std::sync::Mutex::new(None);
    static ref MAC_GLOBAL_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    static ref MAC_AUDIO_SENDER: std::sync::Mutex<Option<mpsc::UnboundedSender<(Vec<f32>, u32)>>> = std::sync::Mutex::new(None);
}

// Debug counter for Rust side
static MAC_DEBUG_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Callback function for receiving raw audio data from Swift
extern "C" fn mac_audio_data_callback(samples: *const f32, count: i32, sample_rate: i32) {
    if samples.is_null() || count <= 0 {
        return;
    }

    if let Ok(guard) = MAC_AUDIO_SENDER.lock() {
        if let Some(ref tx) = *guard {
            // Safety: The pointer is valid for 'count' elements as guaranteed by Swift side
            let slice = unsafe { std::slice::from_raw_parts(samples, count as usize) };
            let data = slice.to_vec();
            // sample_rate comes as i32 from C, convert to u32
            let rate = sample_rate as u32;

            // Debug logging periodically
            let counter = MAC_DEBUG_COUNTER.fetch_add(1, Ordering::Relaxed);
            if counter % 100 == 0 {
                let max_amp = data.iter().fold(0.0f32, |acc, &x| acc.max(x.abs()));
                log::debug!(
                    "[Mac] Received native audio: {} samples, rate={}, max_amp={:.6}",
                    count,
                    rate,
                    max_amp
                );
            }

            // Send data to Rust aggregator
            if let Err(e) = tx.send((data, rate)) {
                if counter % 100 == 0 {
                    log::error!("[Mac] Failed to send audio data to channel: {}", e);
                }
            }
        }
    }
}

/// Callback function for receiving transcription results from Swift
extern "C" fn result_callback(text: *const c_char, is_final: i32) {
    if text.is_null() {
        return;
    }

    if let Ok(guard) = MAC_GLOBAL_TX.lock() {
        if let Some(ref tx) = *guard {
            let c_str = unsafe { CStr::from_ptr(text) };
            if let Ok(s) = c_str.to_str() {
                log::debug!("[Mac] Received text: {}, final: {}", s, is_final);
                let seq = MAC_GLOBAL_SEQ.fetch_add(1, Ordering::SeqCst);
                let event = if is_final != 0 {
                    SttEvent::FinalResult(s.to_string(), seq)
                } else {
                    SttEvent::PartialResult(s.to_string(), seq)
                };
                // Use try_send to avoid blocking in async runtime
                match tx.try_send(event) {
                    Ok(()) => {
                        log::debug!("[Mac] >>> EVENT SENT to internal channel (seq={})", seq);
                    }
                    Err(e) => {
                        log::error!("[Mac] Failed to send event: {}", e);
                    }
                }
            }
        }
    }
}

/// Callback function for receiving errors from Swift
extern "C" fn error_callback(error: *const c_char) {
    if error.is_null() {
        return;
    }

    if let Ok(guard) = MAC_GLOBAL_TX.lock() {
        if let Some(ref tx) = *guard {
            let c_str = unsafe { CStr::from_ptr(error) };
            if let Ok(s) = c_str.to_str() {
                // "COMPLETED:" で始まるメッセージは正常終了通知なので、エラーではなく INFO ログとして扱う
                if s.starts_with("COMPLETED:") {
                    log::info!("[Mac] Native session completed: {}", s);
                    return;
                }

                log::info!("[MAC-TRC] Received native ERROR: {}", s);
                // 自律終了メッセージの場合、ここではイベント送信のみ行い、
                // 再生は最終的に呼ばれる stop() 側に任せる（二重再生防止）

                // Use try_send to avoid blocking in async runtime
                match tx.try_send(SttEvent::Error(s.to_string())) {
                    Ok(()) => log::debug!("[MAC-TRC] Error event sent to internal channel"),
                    Err(e) => log::error!("[MAC-TRC] Failed to send Error event: {}", e),
                }
            }
        }
    }
}

/// Callback function for receiving ready signal from Swift
extern "C" fn mac_ready_callback() {
    log::info!("[MAC-TRC] Received READY signal from native layer");

    if let Ok(guard) = MAC_GLOBAL_TX.lock() {
        if let Some(ref tx) = *guard {
            // Use try_send to avoid blocking in async runtime
            let _ = tx.try_send(SttEvent::Ready);
        }
    }
}

/// Start native audio capture and return a receiver for audio data
pub fn start_native_audio_capture() -> Option<mpsc::UnboundedReceiver<(Vec<f32>, u32)>> {
    log::debug!("[Mac] Starting native audio capture...");
    let (tx, rx) = mpsc::unbounded_channel();

    if let Ok(mut guard) = MAC_AUDIO_SENDER.lock() {
        *guard = Some(tx);
    }

    unsafe {
        speech_helper_set_audio_data_callback(Some(mac_audio_data_callback));
        let ret = speech_helper_start_capture();
        if ret != 0 {
            log::error!("[Mac] Failed to start audio capture: {}", ret);
            return None;
        }
    }

    Some(rx)
}

/// Stop native audio capture
pub fn stop_native_audio_capture() {
    log::debug!("[Mac] Stopping native audio capture...");
    unsafe {
        speech_helper_stop_capture();
        speech_helper_set_audio_data_callback(None);
    }

    if let Ok(mut guard) = MAC_AUDIO_SENDER.lock() {
        *guard = None;
    }
}

/// macOS Speech Recognition Backend
pub struct MacSpeechBackend {
    is_running: Arc<AtomicBool>,
    internal_engine: InternalMacEngine,
    locale: LocaleCode,
    /// Post correction processor (shared with ticker task)
    post_correction_processor: Arc<tokio::sync::Mutex<Option<PostCorrectionProcessor>>>,
    /// Internal receiver for raw events (shared with ticker task)
    rx_raw: Arc<parking_lot::Mutex<Option<mpsc::Receiver<SttEvent>>>>,
    /// Sender for final app events
    tx_app: mpsc::Sender<SttEvent>,
    /// Background ticker task handle
    ticker_task: Option<tokio::task::JoinHandle<()>>,
}

impl MacSpeechBackend {
    /// Create a new macOS speech backend.
    pub fn new(
        tx: mpsc::Sender<SttEvent>,
        _engine: SttEngine,
        locale: LocaleCode,
        backend: Option<Arc<dyn PostCorrectionBackend>>,
        config: Option<PostCorrectionConfig>,
        replaces: Vec<(String, String)>,
    ) -> Result<Self, String> {
        // Create internal channel for raw events
        let (tx_raw, rx_raw) = mpsc::channel(100);
        log::debug!("[Mac] Created internal channel (capacity=100)");

        // Store internal sender globally for callbacks
        if let Ok(mut guard) = MAC_GLOBAL_TX.lock() {
            *guard = Some(tx_raw);
            log::debug!("[Mac] Set MAC_GLOBAL_TX to new sender");
        } else {
            log::error!("[Mac] Failed to lock MAC_GLOBAL_TX!");
        }

        // Initialize processor if backend is provided
        // IMPORTANT: Use UseOnlineModel for native OS speech engines (Tahoe, Classic)
        // These engines send cumulative (all-history) text and may backtrack/rewrite past content.
        let post_correction_processor = if let (Some(b), Some(c)) = (backend, config) {
            log::debug!("[Mac] PostCorrectionProcessor enabled with UseOnlineModel");
            Some(PostCorrectionProcessor::with_model_type(
                b,
                c,
                SttModelType::UseOnlineModel,
                replaces,
            ))
        } else {
            log::debug!("[Mac] PostCorrectionProcessor disabled (no backend/config)");
            None
        };

        let mut internal_engine = InternalMacEngine::Classic;

        // Initialize Swift helper
        unsafe {
            use crate::constants::SPEECH_TIMEOUT_SEC;
            let result = speech_helper_init(SPEECH_TIMEOUT_SEC);
            if result != 0 {
                let msg = match result {
                    -10 => {
                        "macOS 15.0 or later is required for the selected speech engine features."
                            .to_string()
                    }
                    -13 => "Microphone permission denied.".to_string(),
                    _ => format!("Failed to initialize speech helper (Error: {})", result),
                };
                return Err(msg);
            }

            // Set callbacks
            speech_helper_set_result_callback(result_callback);
            speech_helper_set_error_callback(error_callback);
            speech_helper_set_ready_callback(mac_ready_callback);

            // Try Tahoe detection (macOS 15+)
            let c_locale = CString::new(locale.as_str()).unwrap();
            let tahoe_result = tahoe_helper_init(c_locale.as_ptr(), SPEECH_TIMEOUT_SEC);
            if tahoe_result == 0 {
                log::info!("[Mac] Tahoe engine initialized successfully (macOS 15+ detected)");
                internal_engine = InternalMacEngine::Tahoe;
            } else {
                log::info!(
                    "[Mac] Tahoe initialization skipped (code={}). Falling back to Classic engine.",
                    tahoe_result
                );
                // Already initialized speech_helper above, so just keep Classic
            }
        }

        log::debug!(
            "[Mac] SpeechHelper initialized successfully (Internal: {:?})",
            internal_engine
        );

        Ok(Self {
            is_running: Arc::new(AtomicBool::new(false)),
            internal_engine,
            locale,
            post_correction_processor: Arc::new(tokio::sync::Mutex::new(post_correction_processor)),
            rx_raw: Arc::new(parking_lot::Mutex::new(Some(rx_raw))),
            tx_app: tx,
            ticker_task: None,
        })
    }

    /// Start speech recognition
    pub fn start(&mut self) {
        if self.is_running.load(Ordering::SeqCst) {
            log::debug!("[Mac] Speech recognition already running");
            return;
        }

        self.is_running.store(true, Ordering::SeqCst);

        let c_locale = CString::new(self.locale.as_str()).unwrap();

        unsafe {
            let result = if self.internal_engine == InternalMacEngine::Tahoe {
                tahoe_helper_start(c_locale.as_ptr())
            } else {
                speech_helper_start(c_locale.as_ptr())
            };

            if result != 0 {
                self.handle_error(result);
                self.is_running.store(false, Ordering::SeqCst);
                return;
            } else {
                log::debug!(
                    "[Mac] Speech recognition started (internal: {:?}, locale: {:?})",
                    self.internal_engine,
                    self.locale
                );
            }
        }

        // Spawn autonomous background ticker
        let is_running = self.is_running.clone();
        let rx_raw = self.rx_raw.clone();
        let processor = self.post_correction_processor.clone();
        let tx_app = self.tx_app.clone();

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
                        "[Mac] Drained {} stale events from previous session",
                        drained
                    );
                }
            }
        }

        self.ticker_task = Some(tokio::spawn(async move {
            let interval = tokio::time::Duration::from_millis(50);
            log::debug!("[Mac] Background ticker started");
            let mut last_processed_seq = 0u64;
            // ===========================================================
            // WATERMARK: Tracks how many CHARACTERS from the engine's
            // cumulative output have been consumed by a Final commit.
            // This prevents re-feeding already-committed text to the processor.
            // ===========================================================
            let mut watermark_len: usize = 0;

            loop {
                if !is_running.load(Ordering::SeqCst) {
                    break;
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

                for event in raw_events {
                    match event {
                        SttEvent::PartialResult(_, seq) | SttEvent::FinalResult(_, seq) => {
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
                                    log::debug!("[Mac] Coalescing: dropping older seq={}", seq);
                                }
                            } else {
                                log::debug!(
                                    "[Mac] Discarding stale seq={} (last={})",
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
                    log::info!("[MAC-TRC] Ticker: Forwarding control event: {:?}", event);
                    let _ = tx_app.try_send(event);
                }

                // 2. Process latest STT result with WATERMARK synchronization
                if let Some(event) = latest_stt {
                    let (raw_text, seq, is_final_event) = match event {
                        SttEvent::PartialResult(t, s) => (t, s, false),
                        SttEvent::FinalResult(t, s) => (t, s, true),
                        _ => unreachable!(),
                    };

                    let raw_char_count = raw_text.chars().count();
                    log::debug!(
                        "[Mac] Processing seq={} (raw_chars={}, watermark={})",
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
                        log::warn!("[Mac] Engine backtracked! raw_chars={} < watermark={}. Waiting for engine to move forward.", raw_char_count, watermark_len);
                    }

                    // Extract the unconfirmed slice (characters after watermark)
                    let unconfirmed_slice: String = raw_text.chars().skip(watermark_len).collect();
                    log::debug!(
                        "[Mac] Unconfirmed slice: '{}' (len={})",
                        unconfirmed_slice,
                        unconfirmed_slice.chars().count()
                    );

                    let output = {
                        let mut proc_guard = processor.lock().await;
                        if let Some(ref mut proc) = *proc_guard {
                            if is_final_event {
                                // Feed unconfirmed slice as final state AND trigger commit
                                if !unconfirmed_slice.is_empty() {
                                    // Pre-check for correction (Predictive)
                                    let will_trigger =
                                        proc.should_trigger_correction(Some(&unconfirmed_slice));
                                    if will_trigger {
                                        let _ = tx_app.try_send(SttEvent::PostCorrectionStarted);
                                    }

                                    let _ = proc.process_input(&unconfirmed_slice).await;
                                    let res = proc.force_commit().await;

                                    if will_trigger {
                                        let _ = tx_app.try_send(SttEvent::PostCorrectionFinished);
                                    }
                                    res
                                } else {
                                    // Just force commit whatever is there
                                    let will_trigger = proc.should_trigger_correction(None);
                                    if will_trigger {
                                        let _ = tx_app.try_send(SttEvent::PostCorrectionStarted);
                                    }

                                    let res = proc.force_commit().await;

                                    if will_trigger {
                                        let _ = tx_app.try_send(SttEvent::PostCorrectionFinished);
                                    }
                                    res
                                }
                            } else if !unconfirmed_slice.is_empty() {
                                // Partial: check if adding this text will trigger a full correction (Predictive)
                                let will_trigger =
                                    proc.should_trigger_correction(Some(&unconfirmed_slice));
                                if will_trigger {
                                    let _ = tx_app.try_send(SttEvent::PostCorrectionStarted);
                                }

                                let res = proc.process_input(&unconfirmed_slice).await;

                                if will_trigger {
                                    let _ = tx_app.try_send(SttEvent::PostCorrectionFinished);
                                }
                                res
                            } else {
                                // No new content to process
                                None
                            }
                        } else {
                            None
                        }
                    };

                    if let Some(output) = output {
                        log::debug!("[Mac] Processor Output: {:?}", output);
                        match output {
                            ProcessorOutput::Partial(corrected) => {
                                let _ = tx_app.try_send(SttEvent::PartialResult(corrected, seq));
                            }
                            ProcessorOutput::Final(corrected) => {
                                // ===========================================================
                                // ADVANCE WATERMARK: Mark current raw_text as consumed
                                // ===========================================================
                                watermark_len = raw_char_count;
                                log::debug!("[Mac] Watermark advanced to {}", watermark_len);
                                let _ = tx_app.try_send(SttEvent::FinalResult(corrected, seq));
                            }
                        }
                    } else {
                        // Passthrough (processor is None)
                        let guard = processor.lock().await;
                        let has_processor = guard.is_some();
                        if !has_processor {
                            if is_final_event {
                                let _ = tx_app.try_send(SttEvent::FinalResult(raw_text, seq));
                            } else {
                                let _ = tx_app.try_send(SttEvent::PartialResult(raw_text, seq));
                            }
                        }
                    }
                }

                tokio::time::sleep(interval).await;
            }
            log::debug!("[Mac] Background ticker stopped");
        }));
    }

    /// Handle FFI error codes
    fn handle_error(&self, code: i32) {
        let msg = match code {
            -10 => "macOS 15.0 or later is required for Tahoe engine.".to_string(),
            -11 => "Speech model is not installed. Downloading in background...".to_string(),
            -12 => "Hardware does not support Tahoe engine (Neural Engine required).".to_string(),
            -13 => "Microphone permission denied.".to_string(),
            _ => format!("Failed to start speech recognition (Error: {})", code),
        };

        log::debug!("[Mac] {}", msg);

        // Send error event
        if let Ok(guard) = MAC_GLOBAL_TX.lock() {
            if let Some(ref tx) = *guard {
                let _ = tx.try_send(SttEvent::Error(msg));
            }
        }
    }

    /// Stop speech recognition
    pub fn stop(&mut self) {
        if !self.is_running.load(Ordering::SeqCst) {
            return;
        }

        self.is_running.store(false, Ordering::SeqCst);
        MAC_GLOBAL_SEQ.store(0, Ordering::SeqCst);

        // Abort background ticker task
        if let Some(task) = self.ticker_task.take() {
            task.abort();
        }

        unsafe {
            if self.internal_engine == InternalMacEngine::Tahoe {
                tahoe_helper_stop();
            } else {
                speech_helper_stop();
            }
        }

        log::debug!("[MAC-TRC] stop() called by host.");
        log::debug!("[Mac] Speech recognition stopped");
    }

    /// Update locale for next session
    pub fn set_locale(&mut self, locale: LocaleCode) {
        self.locale = locale;
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

impl Drop for MacSpeechBackend {
    fn drop(&mut self) {
        self.stop();
        self.cleanup();
        if let Ok(mut guard) = MAC_GLOBAL_TX.lock() {
            *guard = None;
        }
    }
}
