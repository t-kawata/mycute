# 技術リファレンス: Sherpa01 (オンライン・ストリーミング音声認識)

## 1. 目的
本ドキュメントは、2026年1月時点で `mycute` から削除された `Sherpa01` エンジンの実装詳細を保存するために作成されました。将来、より高性能なオンライン音声認識モデルが登場した際、あるいはオンデバイスでのリアルタイム認識を再導入する際の、アーキテクチャ上のテンプレートおよびリファレンスとして機能します。

---

## 2. 実装の核心ロジック
`Sherpa01` は、音声を入力しながら逐次結果を出力する「ストリーミング方式」を採用していました。

### A. オーディオ流ハンドリング
- **サンプリングレート変換**: 入力デバイスのレート（44.1kHz等）をモデルが要求する 16kHz へ `dasp` クレートや `AudioResampler` 抽象レイヤーを用いて変換。
- **バッファリング**: 変換されたサンプルは `audio_buffer` に蓄積され、一定量（チャンク）に達するたびにモデルへ投入。

### B. 推論ループ
- **非同期実行**: 音声取り込みスレッドと推論スレッド（tick）を分離。
- **状態維持**: `SherpaOnnxOnlineStream` を通じてデコーダの状態を維持しながら、新しい音声チャンクを `AcceptWaveform` で投入。
- **逐次出力**: `GetOnlineStreamResult` を呼び出すことで、文が確定する前の「中間結果（Partial Result）」を即座に取得し、UIへフィードバック。

---

## 3. 完全なソースコード (src/stt/sherpa01.rs)

```rust
//! Sherpa-ONNX based speech recognition using cpal for audio capture.
//!
//! This module provides cross-platform real-time speech-to-text transcription
//! using the sherpa-rs library and cpal for microphone input.

use super::punctuation::PunctuationInserter;
use super::resampler::{AudioResampler, SincResampler, TARGET_SAMPLE_RATE};
use crate::stt_config::{LocaleCode, Sherpa01Settings};
use crate::types::SttEvent;
use crate::utils::some::cstr_to_string;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream, StreamConfig};
use sherpa_rs_sys as sys;
use std::ffi::CString;
use std::mem;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

/// Maximum buffer size to prevent memory issues (10 seconds at 16kHz)
const MAX_BUFFER_SIZE: usize = 16000 * 10;

/// Default symbols to filter out from transcription results
const DEFAULT_SYMBOLS_TO_FILTER: &[&str] = &["♪"];

/// Sherpa-ONNX based speech recognizer with cpal audio capture.
pub struct Sherpa01Recognizer {
    /// Event sender for transcription results
    tx: mpsc::Sender<SttEvent>,
    /// Whether recognition is currently active
    is_running: Arc<AtomicBool>,
    /// Audio input stream (Option for safe drop)
    stream: Option<Stream>,
    /// Audio buffer for accumulating samples before processing
    audio_buffer: Arc<Mutex<Vec<f32>>>,
    /// Resampled audio buffer at 16kHz for Sherpa
    resampled_buffer: Arc<Mutex<Vec<f32>>>,
    /// Settings containing model paths
    settings: Sherpa01Settings,
    /// Input sample rate from the microphone
    input_sample_rate: u32,
    /// Abstracted resampler instance (created after init_audio)
    resampler: Option<Box<dyn AudioResampler>>,
    /// Sherpa-ONNX online recognizer raw pointer
    recognizer: *const sys::SherpaOnnxOnlineRecognizer,
    /// Active online stream for continuous decoding raw pointer
    online_stream: *const sys::SherpaOnnxOnlineStream,
    /// Memory-managed VAD instance
    vad: *const sys::SherpaOnnxVoiceActivityDetector,
    /// Previously finalized transcription segments
    committed_text: String,
    /// Currently active (unfinalized) transcription segment
    active_text: String,
    /// Filler words to remove from transcription
    filler_words: Vec<String>,
    /// Optional punctuation inserter for Japanese text
    punctuation_inserter: Option<PunctuationInserter>,
    /// Current recognition locale
    current_locale: LocaleCode,
    /// Enable stop-word/filler-word filtering
    use_stop_word_filter: bool,
}

impl Sherpa01Recognizer {
    /// Create a new Sherpa recognizer with the given settings.
    ///
    /// Returns `Err` if audio device initialization fails.
    pub fn new(
        tx: mpsc::Sender<SttEvent>,
        settings: Sherpa01Settings,
        filler_words: Vec<String>,
        use_stop_word_filter: bool,
        initial_locale: LocaleCode,
    ) -> Result<Self, String> {
        // Validate model files exist
        if !settings.validate() {
            return Err("Model files not found. Please run the model download script.".to_string());
        }

        // Initialize punctuation inserter if enabled
        let punctuation_inserter = if settings.use_punctuation {
            match PunctuationInserter::new() {
                Ok(inserter) => {
                    log::debug!("Punctuation insertion enabled");
                    Some(inserter)
                }
                Err(e) => {
                    log::debug!(
                        "[STT:Sherpa01] Failed to initialize punctuation inserter: {}",
                        e
                    );
                    None
                }
            }
        } else {
            None
        };

        Ok(Self {
            tx,
            is_running: Arc::new(AtomicBool::new(false)),
            stream: None,
            audio_buffer: Arc::new(Mutex::new(Vec::with_capacity(4096))),
            resampled_buffer: Arc::new(Mutex::new(Vec::with_capacity(TARGET_SAMPLE_RATE as usize))),
            settings,
            input_sample_rate: 0,
            resampler: None,
            recognizer: std::ptr::null(),
            online_stream: std::ptr::null(),
            vad: std::ptr::null(),
            committed_text: String::new(),
            active_text: String::new(),
            filler_words,
            punctuation_inserter,
            current_locale: initial_locale,
            use_stop_word_filter,
        })
    }

    /// Set the current locale for recognition processing.
    pub fn set_locale(&mut self, locale: LocaleCode) {
        log::debug!("Setting locale to: {:?}", locale);
        self.current_locale = locale;
    }

    /// Initialize the audio capture stream.
    ///
    /// This sets up the cpal input stream but does not start capturing yet.
    pub fn init_audio(&mut self) -> Result<(), String> {
        let host = cpal::default_host();

        let device = host
            .default_input_device()
            .ok_or("No input device available")?;

        #[allow(deprecated)]
        let device_name = device.name().unwrap_or_else(|_| "Unknown".to_string());
        log::debug!("Using input device: {}", device_name);

        let supported_config = device
            .default_input_config()
            .map_err(|e| format!("Failed to get input config: {}", e))?;

        let sample_format = supported_config.sample_format();

        // 1. Try 16kHz first to avoid resampling
        let mut config: StreamConfig = supported_config.clone().into();
        let original_sample_rate = config.sample_rate;
        config.sample_rate = TARGET_SAMPLE_RATE;

        log::debug!("Requesting 16kHz input...");

        let res = match sample_format {
            SampleFormat::F32 => self.build_stream_f32(
                &device,
                &config,
                Arc::clone(&self.audio_buffer),
                Arc::clone(&self.is_running),
            ),
            SampleFormat::I16 => self.build_stream_i16(
                &device,
                &config,
                Arc::clone(&self.audio_buffer),
                Arc::clone(&self.is_running),
            ),
            SampleFormat::I32 => self.build_stream_i32(
                &device,
                &config,
                Arc::clone(&self.audio_buffer),
                Arc::clone(&self.is_running),
            ),
            _ => Err(format!("Unsupported sample format: {:?}", sample_format)),
        };

        match res {
            Ok(stream) => {
                log::debug!("Successfully opened stream at 16kHz directly");
                self.input_sample_rate = TARGET_SAMPLE_RATE;
                self.stream = Some(stream);
                self.resampler = None; // 16kHz なのでリサンプラー不要
            }
            Err(e) => {
                log::debug!(
                    "[STT:Sherpa01] 16kHz request failed: {}. Falling back to default rate: {}Hz",
                    e,
                    original_sample_rate
                );
                // 2. Fallback to default rate
                self.input_sample_rate = supported_config.sample_rate();
                let config: StreamConfig = supported_config.into();

                let stream = match sample_format {
                    SampleFormat::F32 => self.build_stream_f32(
                        &device,
                        &config,
                        Arc::clone(&self.audio_buffer),
                        Arc::clone(&self.is_running),
                    )?,
                    SampleFormat::I16 => self.build_stream_i16(
                        &device,
                        &config,
                        Arc::clone(&self.audio_buffer),
                        Arc::clone(&self.is_running),
                    )?,
                    SampleFormat::I32 => self.build_stream_i32(
                        &device,
                        &config,
                        Arc::clone(&self.audio_buffer),
                        Arc::clone(&self.is_running),
                    )?,
                    _ => return Err(format!("Unsupported sample format: {:?}", sample_format)),
                };

                self.stream = Some(stream);
                self.init_resampler()?;
            }
        }

        Ok(())
    }

    /// Build input stream for f32 samples.
    fn build_stream_f32(
        &self,
        device: &cpal::Device,
        config: &StreamConfig,
        audio_buffer: Arc<Mutex<Vec<f32>>>,
        is_running: Arc<AtomicBool>,
    ) -> Result<Stream, String> {
        let channels = config.channels as usize;

        let stream = device
            .build_input_stream(
                config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    if !is_running.load(Ordering::SeqCst) {
                        return;
                    }

                    // Convert to mono by averaging channels
                    let mono_samples: Vec<f32> = if channels > 1 {
                        data.chunks(channels)
                            .map(|chunk| chunk.iter().sum::<f32>() / channels as f32)
                            .collect()
                    } else {
                        data.to_vec()
                    };

                    if let Ok(mut buffer) = audio_buffer.lock() {
                        buffer.extend(mono_samples);
                    }
                },
                |err| {
                    log::error!("cpal stream error: {}", err);
                },
                None, // No timeout
            )
            .map_err(|e| format!("Failed to build input stream: {}", e))?;

        Ok(stream)
    }

    /// Build input stream for i16 samples (convert to f32).
    fn build_stream_i16(
        &self,
        device: &cpal::Device,
        config: &StreamConfig,
        audio_buffer: Arc<Mutex<Vec<f32>>>,
        is_running: Arc<AtomicBool>,
    ) -> Result<Stream, String> {
        let channels = config.channels as usize;

        let stream = device
            .build_input_stream(
                config,
                move |data: &[i16], _: &cpal::InputCallbackInfo| {
                    if !is_running.load(Ordering::SeqCst) {
                        return;
                    }

                    // Convert i16 to f32 and to mono
                    let samples_f32: Vec<f32> =
                        data.iter().map(|&s| s as f32 / i16::MAX as f32).collect();

                    let mono_samples: Vec<f32> = if channels > 1 {
                        samples_f32
                            .chunks(channels)
                            .map(|chunk| chunk.iter().sum::<f32>() / channels as f32)
                            .collect()
                    } else {
                        samples_f32
                    };

                    if let Ok(mut buffer) = audio_buffer.lock() {
                        buffer.extend(mono_samples);
                    }
                },
                |err| {
                    log::error!("cpal stream error: {}", err);
                },
                None,
            )
            .map_err(|e| format!("Failed to build input stream: {}", e))?;

        Ok(stream)
    }

    /// Build input stream for i32 samples (convert to f32).
    fn build_stream_i32(
        &self,
        device: &cpal::Device,
        config: &StreamConfig,
        audio_buffer: Arc<Mutex<Vec<f32>>>,
        is_running: Arc<AtomicBool>,
    ) -> Result<Stream, String> {
        let channels = config.channels as usize;

        let stream = device
            .build_input_stream(
                config,
                move |data: &[i32], _: &cpal::InputCallbackInfo| {
                    if !is_running.load(Ordering::SeqCst) {
                        return;
                    }

                    // Convert i32 to f32 and to mono
                    let samples_f32: Vec<f32> =
                        data.iter().map(|&s| s as f32 / i32::MAX as f32).collect();

                    let mono_samples: Vec<f32> = if channels > 1 {
                        samples_f32
                            .chunks(channels)
                            .map(|chunk| chunk.iter().sum::<f32>() / channels as f32)
                            .collect()
                    } else {
                        samples_f32
                    };

                    if let Ok(mut buffer) = audio_buffer.lock() {
                        buffer.extend(mono_samples);
                    }
                },
                |err| {
                    log::error!("cpal stream error: {}", err);
                },
                None,
            )
            .map_err(|e| format!("Failed to build input stream: {}", e))?;

        Ok(stream)
    }

    /// Start audio capture and recognition.
    pub fn start(&mut self) -> Result<(), String> {
        if self.is_running.load(Ordering::SeqCst) {
            log::debug!("Already running");
            return Ok(());
        }

        // Initialize audio if not already done
        if self.stream.is_none() {
            self.init_audio()?;
        }

        // Clear any existing audio buffer
        if let Ok(mut buffer) = self.audio_buffer.lock() {
            buffer.clear();
        }

        self.is_running.store(true, Ordering::SeqCst);

        // Start the audio stream
        if let Some(ref stream) = self.stream {
            stream
                .play()
                .map_err(|e| format!("Failed to start stream: {}", e))?;
        }

        log::debug!(
            "[STT:Sherpa01] Started audio capture at {} Hz",
            self.input_sample_rate
        );
        Ok(())
    }

    /// Stop audio capture and recognition.
    pub fn stop(&mut self) {
        if !self.is_running.load(Ordering::SeqCst) {
            return;
        }

        self.is_running.store(false, Ordering::SeqCst);

        // Pause the stream to save power
        if let Some(ref stream) = self.stream {
            let _ = stream.pause();
        }

        // Reset the stream so it's fresh for next start
        unsafe {
            if !self.recognizer.is_null() && !self.online_stream.is_null() {
                sys::SherpaOnnxOnlineStreamReset(self.recognizer, self.online_stream);
            }
        }

        // Clear text buffers for a clean start next time
        self.committed_text.clear();
        self.active_text.clear();

        // Send stopped event
        let _ = self.tx.try_send(SttEvent::Stopped);

        log::debug!("Stopped audio capture and cleared buffers");
    }

    /// Apply filters to the transcript to remove unwanted symbols and fillers,
    /// and optionally insert punctuation text.
    fn apply_filters(&self, text: &str) -> String {
        let mut result = text.to_string();

        // 1. Script/Unicode filtering (detect Chinese hallucination etc.)
        if self.settings.use_script_filter
            && self.current_locale == LocaleCode::Ja
            && self.is_hallucination_script(&result)
        {
            log::debug!(
                "[STT:Sherpa01] Filtered out script hallucination: '{}'",
                result
            );
            return String::new();
        }

        // 2. Structural/Grammar filtering (detect noise-induced junk)
        if self.settings.use_structure_filter && self.current_locale == LocaleCode::Ja {
            if let Some(inserter) = self.punctuation_inserter.as_ref() {
                if inserter.is_structure_collapsed(&result) {
                    log::debug!(
                        "[STT:Sherpa01] Filtered out structural hallucination: '{}'",
                        result
                    );
                    return String::new();
                }
            }
        }

        // 3. Default symbol filtering
        for symbol in DEFAULT_SYMBOLS_TO_FILTER {
            result = result.replace(symbol, "");
        }

        // 4. Blacklist/Filler word filtering
        if self.use_stop_word_filter {
            for filler in &self.filler_words {
                result = result.replace(filler, "");
            }
        }

        // 5. Punctuation insertion
        if self.settings.use_punctuation {
            if let Some(ref inserter) = self.punctuation_inserter {
                if let Ok(punctuated) = inserter.insert(&result, &self.current_locale) {
                    result = punctuated;
                }
            }
        }

        result.trim().to_string()
    }

    /// Detects if the text contains non-Japanese characters that are likely hallucinations (e.g., Simplified Chinese).
    fn is_hallucination_script(&self, text: &str) -> bool {
        if text.is_empty() {
            return false;
        }

        // Check for specific common hallucinations
        let hallucinations = ["那么", "谢谢", "是什么", "为什么", "没有"];
        for h in hallucinations {
            if text.contains(h) {
                return true;
            }
        }

        // Check for Simplified Chinese specific characters
        // Check for character 么 (U+4E48) which is almost always SC.
        if text.contains('\u{4E48}') {
            return true;
        }
        // Check for 们 (U+4EEC) or 门 (U+95E8)
        if text.contains('\u{4EEC}') || text.contains('\u{95E8}') {
            return true;
        }

        false
    }

    /// Initialize the resampler for converting input rate to 16kHz.
    ///
    /// Must be called after init_audio() so input_sample_rate is known.
    /// Creates a SincResampler instance from the abstracted resampling layer.
    pub fn init_resampler(&mut self) -> Result<(), String> {
        if self.input_sample_rate == 0 {
            return Err("Input sample rate not set. Call init_audio() first.".to_string());
        }

        // If already at target rate, no resampling needed
        if self.input_sample_rate == TARGET_SAMPLE_RATE {
            log::debug!("No resampling needed (already at 16kHz)");
            self.resampler = None;
            return Ok(());
        }

        // Create SincResampler from the abstracted layer
        let resampler = SincResampler::new_for_sherpa(self.input_sample_rate)
            .map_err(|e| format!("Failed to create resampler: {}", e))?;

        log::debug!(
            "[STT:Sherpa01] SincResampler initialized: {} Hz -> {} Hz",
            self.input_sample_rate,
            TARGET_SAMPLE_RATE
        );

        self.resampler = Some(Box::new(resampler));
        Ok(())
    }

    /// Process accumulated audio samples and resample to 16kHz.
    ///
    /// Takes samples from audio_buffer, resamples using the abstracted
    /// AudioResampler, and stores in resampled_buffer.
    /// Returns the number of output samples produced.
    pub fn process_and_resample(&mut self) -> Result<usize, String> {
        // Take samples from input buffer
        let input_samples = self.take_audio_samples();
        if input_samples.is_empty() {
            return Ok(0);
        }

        // If no resampler (already at 16kHz), copy directly
        if self.resampler.is_none() {
            if let Ok(mut buf) = self.resampled_buffer.lock() {
                // Enforce max buffer size
                if buf.len() + input_samples.len() > MAX_BUFFER_SIZE {
                    let drain_count = buf.len() + input_samples.len() - MAX_BUFFER_SIZE;
                    buf.drain(..drain_count);
                }
                let count = input_samples.len();
                buf.extend(input_samples);
                return Ok(count);
            }
            return Ok(0);
        }

        // Use the abstracted resampler
        let resampler = self.resampler.as_mut().unwrap();

        let output = resampler
            .process(&input_samples)
            .map_err(|e| format!("Resampling failed: {}", e))?;

        let total_output = output.len();

        if let Ok(mut buf) = self.resampled_buffer.lock() {
            // Enforce max buffer size
            if buf.len() + total_output > MAX_BUFFER_SIZE {
                let drain_count = buf.len() + total_output - MAX_BUFFER_SIZE;
                buf.drain(..drain_count);
            }
            buf.extend(output);
        }

        Ok(total_output)
    }

    /// Take resampled audio samples from the 16kHz buffer.
    pub fn take_resampled_samples(&self) -> Vec<f32> {
        if let Ok(mut buffer) = self.resampled_buffer.lock() {
            std::mem::take(&mut *buffer)
        } else {
            Vec::new()
        }
    }

    /// Take accumulated audio samples from the buffer.
    ///
    /// Returns the samples and clears the internal buffer.
    fn take_audio_samples(&self) -> Vec<f32> {
        if let Ok(mut buffer) = self.audio_buffer.lock() {
            std::mem::take(&mut *buffer)
        } else {
            Vec::new()
        }
    }

    /// Get a reference to the settings.
    #[allow(dead_code)]
    pub fn settings(&self) -> &Sherpa01Settings {
        &self.settings
    }

    /// Initialize the Sherpa-ONNX online recognizer.
    ///
    /// Loads ONNX models from paths specified in settings.
    /// Must be called before recognition can start.
    pub fn init_recognizer(&mut self) -> Result<(), String> {
        unsafe {
            let provider_str = if self.settings.use_coreml {
                "coreml"
            } else {
                "cpu"
            };
            let provider = CString::new(provider_str).unwrap();
            let encoder = CString::new(self.settings.resolve_path(&self.settings.encoder)).unwrap();
            let decoder = CString::new(self.settings.resolve_path(&self.settings.decoder)).unwrap();
            let joiner = CString::new(self.settings.resolve_path(&self.settings.joiner)).unwrap();
            let tokens = CString::new(self.settings.resolve_path(&self.settings.tokens)).unwrap();
            let decoding_method = CString::new(self.settings.decoding_method.as_str()).unwrap();
            let model_type = CString::new("zipformer2").unwrap();
            let bpe_model_empty = CString::new("").unwrap();

            let mut model_config: sys::SherpaOnnxOnlineModelConfig = mem::zeroed();
            model_config.transducer.encoder = encoder.as_ptr();
            model_config.transducer.decoder = decoder.as_ptr();
            model_config.transducer.joiner = joiner.as_ptr();
            model_config.tokens = tokens.as_ptr();
            model_config.num_threads = self.settings.num_threads;
            model_config.provider = provider.as_ptr();
            model_config.bpe_vocab = bpe_model_empty.as_ptr();
            model_config.model_type = model_type.as_ptr();
            model_config.debug = 0;

            let mut recognizer_config: sys::SherpaOnnxOnlineRecognizerConfig = mem::zeroed();
            recognizer_config.feat_config.sample_rate = TARGET_SAMPLE_RATE as i32;
            recognizer_config.feat_config.feature_dim = 80;
            recognizer_config.model_config = model_config;
            recognizer_config.decoding_method = decoding_method.as_ptr();
            recognizer_config.max_active_paths = self.settings.max_active_paths;
            recognizer_config.hotwords_score = 1.5;
            recognizer_config.enable_endpoint = 1;
            recognizer_config.rule1_min_trailing_silence = 5.0;
            recognizer_config.rule2_min_trailing_silence = 2.0;
            recognizer_config.rule3_min_utterance_length = 30.0;

            log::debug!(
                "[STT:Sherpa01] Initializing OnlineRecognizer with provider={}...",
                provider_str
            );
            let recognizer = sys::SherpaOnnxCreateOnlineRecognizer(&recognizer_config);
            if recognizer.is_null() {
                return Err("Failed to create OnlineRecognizer".to_string());
            }

            // Create stream with language token as hotwords for multilingual model
            let language_token = self.current_locale.sherpa01_language_token();
            let hotwords = CString::new(language_token).unwrap();
            log::debug!("Setting language token: {}", language_token);
            let stream =
                sys::SherpaOnnxCreateOnlineStreamWithHotwords(recognizer, hotwords.as_ptr());
            self.recognizer = recognizer;
            self.online_stream = stream;

            // Initialize VAD if enabled
            if self.settings.use_vad {
                let mut vad_config: sys::SherpaOnnxVadModelConfig = mem::zeroed();
                let vad_model_path = CString::new(self.settings.get_vad_path()).unwrap();

                if self.settings.vad_type.starts_with("ten") {
                    vad_config.ten_vad.model = vad_model_path.as_ptr();
                    vad_config.ten_vad.threshold = self.settings.vad_threshold; // 閾値（0.0-1.0）。値を下げると感度が上がり、小さな声も拾うようになります
                    vad_config.ten_vad.min_silence_duration =
                        self.settings.vad_min_silence_duration; // この秒数以上静かだと「無音」と判定します
                    vad_config.ten_vad.min_speech_duration = self.settings.vad_min_speech_duration; // この秒数以上声が続くと「音声」と判定します
                    vad_config.ten_vad.window_size = 512; // 解析単位（サンプル数）。512が推奨値です
                } else if self.settings.vad_type.starts_with("silero") {
                    vad_config.silero_vad.model = vad_model_path.as_ptr();
                    vad_config.silero_vad.threshold = self.settings.vad_threshold; // 閾値（0.0-1.0）。値を下げると感度が上がり、小さな声も拾うようになります
                    vad_config.silero_vad.min_speech_duration =
                        self.settings.vad_min_speech_duration; // この秒数以上声が続くと「音声」と判定します
                    vad_config.silero_vad.min_silence_duration =
                        self.settings.vad_min_silence_duration; // この秒数以上静かだと「無音」と判定します
                    vad_config.silero_vad.window_size = 512; // 解析単位（サンプル数）。512が推奨値です
                } else {
                    return Err(format!(
                        "Unsupported VAD type: {}. Available: 'silero', 'ten'.",
                        self.settings.vad_type
                    ));
                }
                vad_config.sample_rate = TARGET_SAMPLE_RATE as i32;
                vad_config.num_threads = self.settings.num_threads;
                vad_config.provider = provider.as_ptr();
                vad_config.debug = 0;

                let vad = sys::SherpaOnnxCreateVoiceActivityDetector(&vad_config, 10.0);
                if !vad.is_null() {
                    self.vad = vad;
                    log::debug!(
                        "[STT:Sherpa01] VAD initialized ({})",
                        self.settings.vad_type
                    );
                } else {
                    log::error!("Failed to initialize VAD");
                }
            }

            log::debug!("OnlineRecognizer initialized successfully");
            Ok(())
        }
    }

    /// Perform recognition on accumulated resampled audio.
    ///
    /// Takes audio from the resampled buffer, runs transcription,
    /// and returns the recognized text and whether an endpoint was detected.
    pub fn recognize(&mut self) -> Result<Option<(String, bool)>, String> {
        if self.recognizer.is_null() || self.online_stream.is_null() {
            return Err("Recognizer or stream not initialized".to_string());
        }

        let samples = self.take_resampled_samples();
        // if !samples.is_empty() {
        //     log::debug!("Processing {} resampled samples", samples.len());
        // }

        unsafe {
            if !samples.is_empty() {
                let mut should_accept = true;

                // VAD filtering
                if !self.vad.is_null() {
                    sys::SherpaOnnxVoiceActivityDetectorAcceptWaveform(
                        self.vad,
                        samples.as_ptr(),
                        samples.len() as i32,
                    );
                    if sys::SherpaOnnxVoiceActivityDetectorDetected(self.vad) == 0 {
                        should_accept = false;
                    }
                }

                if should_accept {
                    sys::SherpaOnnxOnlineStreamAcceptWaveform(
                        self.online_stream,
                        TARGET_SAMPLE_RATE as i32,
                        samples.as_ptr(),
                        samples.len() as i32,
                    );
                }
            }

            // Decode as much as possible
            let mut decode_count = 0;
            while sys::SherpaOnnxIsOnlineStreamReady(self.recognizer, self.online_stream) != 0 {
                sys::SherpaOnnxDecodeOnlineStream(self.recognizer, self.online_stream);
                decode_count += 1;
            }

            if decode_count > 0 {
                log::debug!("Decoded {} chunks", decode_count);
            }

            let is_endpoint =
                sys::SherpaOnnxOnlineStreamIsEndpoint(self.recognizer, self.online_stream) != 0;

            let result_ptr =
                sys::SherpaOnnxGetOnlineStreamResult(self.recognizer, self.online_stream);

            if result_ptr.is_null() {
                if is_endpoint {
                    // Endpoint reached but no text in this segment
                    sys::SherpaOnnxOnlineStreamReset(self.recognizer, self.online_stream);
                    self.committed_text.push_str(&self.active_text);
                    self.active_text.clear();
                    return Ok(Some((self.committed_text.clone(), true)));
                }
                return Ok(None);
            }

            let raw_result = result_ptr.read();
            let text = cstr_to_string(raw_result.text as _);
            sys::SherpaOnnxDestroyOnlineRecognizerResult(result_ptr);

            if is_endpoint {
                // Endpoint reached: Move currently active text to committed
                // and clear active text before resetting stream
                self.committed_text.push_str(&text);
                self.active_text.clear();
                sys::SherpaOnnxOnlineStreamReset(self.recognizer, self.online_stream);
                // log::debug!("Endpoint detected, stream reset. Current total: '{}'", self.committed_text);
            } else {
                // Normal update
                self.active_text = text.to_owned();
            }

            let full_text =
                self.apply_filters(&format!("{}{}", self.committed_text, self.active_text));
            if full_text.is_empty() {
                Ok(None)
            } else {
                Ok(Some((full_text, is_endpoint)))
            }
        }
    }
}

unsafe impl Send for Sherpa01Recognizer {}
unsafe impl Sync for Sherpa01Recognizer {}

impl Drop for Sherpa01Recognizer {
    fn drop(&mut self) {
        self.stop();
        unsafe {
            if !self.online_stream.is_null() {
                sys::SherpaOnnxDestroyOnlineStream(self.online_stream);
            }
            if !self.recognizer.is_null() {
                sys::SherpaOnnxDestroyOnlineRecognizer(self.recognizer);
            }
            if !self.vad.is_null() {
                sys::SherpaOnnxDestroyVoiceActivityDetector(self.vad);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;

    fn get_dummy_recognizer() -> Sherpa01Recognizer {
        let (tx, _) = mpsc::channel(1);
        Sherpa01Recognizer {
            tx,
            is_running: Arc::new(AtomicBool::new(false)),
            stream: None,
            audio_buffer: Arc::new(Mutex::new(Vec::<f32>::new())),
            resampled_buffer: Arc::new(Mutex::new(Vec::<f32>::new())),
            settings: Sherpa01Settings::default(),
            input_sample_rate: 16000,
            resampler: None,
            recognizer: std::ptr::null(),
            online_stream: std::ptr::null(),
            vad: std::ptr::null(),
            committed_text: String::new(),
            active_text: String::new(),
            filler_words: Vec::new(),
            punctuation_inserter: None,
            current_locale: LocaleCode::Ja,
            use_stop_word_filter: true,
        }
    }

    #[test]
    fn test_is_hallucination_script() {
        let recognizer = get_dummy_recognizer();

        // Valid Japanese
        assert!(!recognizer.is_hallucination_script("今日は良い天気ですね"));
        assert!(!recognizer.is_hallucination_script("ありがとうございます"));

        // Chinese hallucinations
        assert!(recognizer.is_hallucination_script("那么"));
        assert!(recognizer.is_hallucination_script("是什么"));
        assert!(recognizer.is_hallucination_script("没有"));

        // Simplified Chinese specific characters (么, 们)
        assert!(recognizer.is_hallucination_script("こんにちは、什么"));
        assert!(recognizer.is_hallucination_script("我门"));
    }

    #[test]
    fn test_sherpa01_settings_validation() {
        let settings = Sherpa01Settings::default();
        // This will fail unless models are present, which is expected
        // The actual test is that it doesn't panic
        let _ = settings.validate();
    }
}
```

---

## 4. 将来の再統合（復元）手順

もし本機能を現在のコードベースに復元する場合、以下の手順に従います。

### ステップ 0: モデルファイルの準備
認識にはモデルファイルが必要です。以下のコマンドで Hugging Face リポジトリから必要なファイルを `models` ディレクトリに直接ダウンロードします。これらのファイルは、削除前の `mycute` で使用されていた特定の構成（epoch-75 モデル）です。

```bash
mkdir -p models

# ASRモデル (epoch-75)
curl -L -o models/encoder-epoch-75-avg-11-chunk-16-left-128.int8.onnx "https://huggingface.co/t-kawata/mycute/resolve/main/encoder-epoch-75-avg-11-chunk-16-left-128.int8.onnx"
curl -L -o models/decoder-epoch-75-avg-11-chunk-16-left-128.onnx "https://huggingface.co/t-kawata/mycute/resolve/main/decoder-epoch-75-avg-11-chunk-16-left-128.onnx"
curl -L -o models/joiner-epoch-75-avg-11-chunk-16-left-128.int8.onnx "https://huggingface.co/t-kawata/mycute/resolve/main/joiner-epoch-75-avg-11-chunk-16-left-128.int8.onnx"

# 語彙・BPEモデル
curl -L -o models/tokens.txt "https://huggingface.co/t-kawata/mycute/resolve/main/tokens.txt"
curl -L -o models/bpe.model "https://huggingface.co/t-kawata/mycute/resolve/main/bpe.model"
```

### ステップ 1: ファイルの復旧
1. `src/stt/sherpa01.rs` を上記コードから再作成。
2. `src/stt/mod.rs` に `pub mod sherpa01;` を追加。

### ステップ 2: 設定の再定義
`src/stt_config.rs` に以下の要素を再追加します。
- `SttEngine` 列挙型に `Sherpa01` バリアントを追加。
- `Sherpa01Settings` 構造体と `impl Default` を、本ドキュメント（あるいは過去の git 履歴）を参照して復活。
- `Settings` 構造体に `pub sherpa01: Sherpa01Settings` フィールドを追加。

### ステップ 3: 認識器ファクトリの更新
`src/stt/recognizer.rs` を編集します。
- `SpeechRecognizer` のフィールドに `sherpa01_backend: Option<Sherpa01Recognizer>` を追加。
- `new` メソッド内で `SttEngine::Sherpa01` 時の初期化ロジックを追加。
- `start`, `stop`, `tick` 等の各メソッドで `sherpa01_backend` への委譲処理を追加。

### ステップ 4: UI の復元
`src/ui/settings.rs` を編集し、エンジン選択のラジオボタンと、`Sherpa01` 固有のパラメーター（CoreML 有効化など）を調整するパネルを再追加します。

---
