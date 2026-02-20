//! PseudoAsrStreamer - 擬似ストリーミング ASR オーケストレーター
//!
//! オフラインモデルをストリーミング音声認識に対応させるための
//! 完全自己完結型のパイプラインを提供します。
//!
//! ## 特徴
//! - 外部モジュール（config, resampler, types等）への依存を排除した完全なカプセル化
//! - 任意のサンプリングレート・チャンネル数からのオーディオ正規化 (16kHz Mono)
//! - VAD (Voice Activity Detection) の管理とスピーチ区間検出
//! - ウィンドウ管理とスライディング認識
//! - 差分マージとセッション履歴管理
//! - 形態素解析ベースの句読点挿入

use std::collections::VecDeque;
use std::ffi::CString;
use std::mem;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use rubato::{
    Resampler as RubatoResampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType,
    WindowFunction,
};
use sherpa_rs_sys as sys;
use tokio::sync::mpsc;

use crate::tools::post_correction_processor::{
    PostCorrectionBackend, PostCorrectionConfig, PostCorrectionProcessor, ProcessorOutput,
};

// ============================================================================
// 内部定数
// ============================================================================

/// 内部処理用サンプリングレート (16kHz)
/// 呼び出し側がこのレートを参照できるよう pub にしている。
pub const INTERNAL_TARGET_RATE: u32 = 16000;
/// ミリ秒からサンプル数への変換用
const MS_PER_SEC: usize = 1000;
/// Silero VAD 推奨ウィンドウサイズ (16kHz 時、32ms 相当)
const SILERO_VAD_WINDOW_SIZE: usize = 512;
/// TEN VAD 推奨ウィンドウサイズ (16kHz 時、16ms 相当)
const TEN_VAD_WINDOW_SIZE: usize = 256;
/// i16 から f32 への変換係数 (1.0 / 32768.0)
/// 呼び出し側（openai.rs など）でモノラル化時に使用可能。
pub const I16_TO_F32_SCALE: f32 = 1.0 / 32768.0;
/// デフォルトの信号占有率閾値 (0.0 ~ 1.0)
const DEFAULT_SIGNAL_OCCUPANCY_RATIO: f32 = 0.15;
/// デフォルトの最小 RMS 閾値
const DEFAULT_SIGNAL_RMS_THRESHOLD: f32 = 0.005;

// ============================================================================
// SpeechDenoiser: GTCRN ノイズ除去ラッパー
// ============================================================================

/// SherpaOnnxOfflineSpeechDenoiser のラッパー
struct SpeechDenoiser {
    inner: *const sys::SherpaOnnxOfflineSpeechDenoiser,
}

unsafe impl Send for SpeechDenoiser {}
unsafe impl Sync for SpeechDenoiser {}

impl SpeechDenoiser {
    fn new(model_path: &str, num_threads: i32) -> Result<Self> {
        let c_model = CString::new(model_path)?;

        // GTCRN Config 構築
        let gtcrn_config = sys::SherpaOnnxOfflineSpeechDenoiserGtcrnModelConfig {
            model: c_model.as_ptr(),
        };

        // Model Config 構築
        let model_config = sys::SherpaOnnxOfflineSpeechDenoiserModelConfig {
            gtcrn: gtcrn_config,
            num_threads,
            debug: 0,
            provider: std::ptr::null(), // "cpu" by default
        };

        // Denoiser Config 構築
        let config = sys::SherpaOnnxOfflineSpeechDenoiserConfig {
            model: model_config,
        };

        let denoiser = unsafe { sys::SherpaOnnxCreateOfflineSpeechDenoiser(&config) };
        if denoiser.is_null() {
            return Err(anyhow!("Failed to create SherpaOnnxOfflineSpeechDenoiser."));
        }

        log::debug!("[SpeechDenoiser] Initialized with model: {}", model_path);
        Ok(Self { inner: denoiser })
    }

    /// 音声データをデノイズ処理する
    /// offline API だが、ウィンドウ単位で処理するためここで呼び出し可能。
    fn run(&self, samples: &[f32], sample_rate: i32) -> Result<Vec<f32>> {
        let n_samples = samples.len() as i32;

        let result_ptr = unsafe {
            sys::SherpaOnnxOfflineSpeechDenoiserRun(
                self.inner,
                samples.as_ptr(),
                n_samples,
                sample_rate,
            )
        };

        if result_ptr.is_null() {
            return Err(anyhow!("Denoiser returned null result."));
        }

        // 結果を Vec<f32> にコピー
        let result = unsafe { &*result_ptr };
        let output_samples = if result.n > 0 && !result.samples.is_null() {
            unsafe { std::slice::from_raw_parts(result.samples, result.n as usize).to_vec() }
        } else {
            Vec::new()
        };

        // 結果オブジェクトを破棄
        unsafe { sys::SherpaOnnxDestroyDenoisedAudio(result_ptr) };

        Ok(output_samples)
    }
}

impl Drop for SpeechDenoiser {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe { sys::SherpaOnnxDestroyOfflineSpeechDenoiser(self.inner) };
        }
    }
}

// ============================================================================
// 外部依存を排除するためのローカル定義
// ============================================================================

/// 認識器の言語ロケール
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamerLocale {
    /// 日本語
    Ja,
    /// 英語
    En,
}

/// ストリーマーから出力されるイベント
#[derive(Debug, Clone)]
pub enum StreamerEvent {
    /// 発話開始
    SpeechStart(String),
    /// 発話終了
    SpeechEnd(String),
    /// 部分的な認識結果（表示用）
    PartialResult(String),
    /// 最終的な認識結果
    FinalResult(String),
    /// 補正開始（ロック通知）
    PostCorrectionStarted,
    /// 補正終了（ロック解除通知）
    PostCorrectionFinished,
}

// ============================================================================
// Resampler: 内部リサンプリングロジック
// ============================================================================

#[derive(Debug)]
enum ResamplerError {
    CreationFailed(String),
    ProcessFailed(String),
}

impl std::fmt::Display for ResamplerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResamplerError::CreationFailed(msg) => write!(f, "Resampler creation failed: {}", msg),
            ResamplerError::ProcessFailed(msg) => write!(f, "Resampler process failed: {}", msg),
        }
    }
}

impl std::error::Error for ResamplerError {}

trait InternalResampler: Send {
    fn process(&mut self, input: &[f32]) -> Result<Vec<f32>, ResamplerError>;
    fn reset(&mut self);
}

struct SincResampler {
    inner: SincFixedIn<f32>,
    residual: Vec<f32>,
}

impl SincResampler {
    fn new(input_rate: u32, output_rate: u32) -> Result<Self, ResamplerError> {
        let resample_ratio = output_rate as f64 / input_rate as f64;
        let params = SincInterpolationParameters {
            sinc_len: 256,
            f_cutoff: 0.95,
            interpolation: SincInterpolationType::Linear,
            oversampling_factor: 256,
            window: WindowFunction::BlackmanHarris2,
        };

        let inner = SincFixedIn::<f32>::new(
            resample_ratio,
            2.0,
            params,
            1024,
            1, // mono
        )
        .map_err(|e| ResamplerError::CreationFailed(format!("{:?}", e)))?;

        Ok(Self {
            inner,
            residual: Vec::new(),
        })
    }
}

impl InternalResampler for SincResampler {
    fn process(&mut self, input: &[f32]) -> Result<Vec<f32>, ResamplerError> {
        if input.is_empty() {
            return Ok(Vec::new());
        }
        let mut all_samples = std::mem::take(&mut self.residual);
        all_samples.extend_from_slice(input);
        let mut output = Vec::new();
        let frames_needed = self.inner.input_frames_next();
        let mut offset = 0;
        while offset + frames_needed <= all_samples.len() {
            let chunk = &all_samples[offset..offset + frames_needed];
            let input_vecs = vec![chunk.to_vec()];
            match self.inner.process(&input_vecs, None) {
                Ok(result) => {
                    if !result.is_empty() && !result[0].is_empty() {
                        output.extend_from_slice(&result[0]);
                    }
                }
                Err(e) => return Err(ResamplerError::ProcessFailed(format!("{:?}", e))),
            }
            offset += frames_needed;
        }
        self.residual = all_samples[offset..].to_vec();
        Ok(output)
    }

    fn reset(&mut self) {
        self.residual.clear();
    }
}

// ============================================================================
// AsrBackend: バックエンドが実装すべきトレイト
// ============================================================================

pub trait AsrBackend: Send {
    /// 音声データを渡してテキストを取得（バッチ推論）
    fn transcribe(&mut self, samples: &[f32]) -> Result<String>;
    /// 最終補正レイヤーとして、LLMを用いた補正を適用する
    fn post_correct(&mut self, text: &str) -> Result<String>;
    /// 使用しているモデル名を返す
    fn model_name(&self) -> String;
    /// 音声認識時間 (ASR) を記録する
    fn record_asr_usage(&mut self, duration_ms: u64);
    /// 句読点を挿入する (オプション)
    /// 必要ない場合はデフォルト実装でそのまま返す
    fn insert_punctuation(&mut self, text: &str, _locale: &StreamerLocale) -> Result<String> {
        Ok(text.to_string())
    }
    /// 文字列置換リストを取得
    fn replaces(&self) -> Vec<(String, String)>;
}

/// バックエンドを `PostCorrectionProcessor` から呼び出せるようにするためのラッパー
/// Arc<Mutex<B>> を保持し、呼び出し時にロックして移譲する
pub struct BackendWrapper<B>(pub Arc<Mutex<B>>);

#[async_trait]
impl<B: AsrBackend + Send + 'static> PostCorrectionBackend for BackendWrapper<B> {
    async fn post_correct(&self, text: &str) -> Result<String> {
        // ロックしてバックエンドの同期メソッドを呼ぶ
        // 注意: ここでロック時間が長引くとストリーミングに影響が出る可能性があるが、
        // post_correct 自体が重い(LLM)ため、ある程度は許容する。
        // OpenAIBackendの post_correct は内部で block_in_place を使っているので、
        // Mutexを保持したままブロックするのは好ましくないかもしれないが、
        // 現状のアーキテクチャ（PseudoAsrStreamerがbackendを所有）を維持するためにはこれしかない。
        let mut guard = self.0.lock().unwrap();
        guard.post_correct(text)
    }
}

// ============================================================================
// 設定
// ============================================================================

/// VAD (発話検知) のアルゴリズムタイプ
#[derive(Debug, Clone, PartialEq)]
pub enum VadType {
    /// Silero VAD (ONNX)
    Silero,
    /// TEN VAD (ONNX)
    Ten,
}

/// 擬似ストリーミングの動作を制御する設定項目
#[derive(Debug, Clone)]
pub struct StreamerConfig {
    /// VAD モデルファイルへの絶対パス
    pub vad_model_path: String,
    /// 使用する VAD の種類 (Silero / Ten)
    pub vad_type: VadType,
    /// 発話検知の閾値 (0.0 から 1.0)
    pub vad_threshold: f32,
    /// 発話終了とみなす無音継続時間 (秒)
    pub vad_min_silence_duration: f32,
    /// 発話開始とみなす最小音声時間 (秒)
    pub vad_min_speech_duration: f32,
    /// VADバッファの最大長さおよび強制発話終了のタイムアウト (秒)
    pub vad_max_speech_duration: f32,
    /// 発話開始前に遡って保持する音声の時間 (ミリ秒)
    pub vad_pre_padding_ms: u32,

    /// 認識に回す最小の utterance の長さ (ミリ秒)
    pub utterance_min_ms: u32,

    /// VAD およびリサンプリングに使用するスレッド数
    pub num_threads: i32,
    /// 使用する言語ロケール (文末記号や形態素解析に使用)
    pub locale: StreamerLocale,

    // ========================================
    // 信号品質チェック
    // ========================================
    /// 信号品質チェックを有効化するか
    pub signal_check_enabled: bool,
    /// 最小 RMS 閾値（音量の平均強度によるノイズ・ゲート）
    ///
    /// 【概要】
    /// 入力された音声波形の RMS (Root Mean Square: 実効値) がこの値未満の場合、その区間は「完全な無音」
    /// または「無視すべき環境ノイズ」として扱われます。VAD (音声区間検出) にデータを渡す前の前段フィルター
    /// として機能します。
    ///
    /// 【数値の意味】
    /// 数値は 0.0（無音）から 1.0（フルスケール）の範囲で設定します。
    /// 一般的なマイク入力では非常に小さな値（例: 0.001 〜 0.01）で十分に機能します。
    ///
    /// 【設定の目安と効果】
    /// - 値を「大きく」する (例: 0.05 〜 0.1):
    ///   - 効果: 判定が厳しくなり、エアコンの音や PC のファン音などの小さな環境ノイズを完全に無視できます。
    ///   - リスク: ぼそぼそと喋った時や、文末の消え入るような声がカットされてしまい、認識率が低下する可能性があります。
    /// - 値を「小さく」する (例: 0.001 〜 0.005):
    ///   - 効果: 非常に感度が良くなり、遠くの声や小さな囁き声まで VAD の判定対象に含めることができます。
    ///   - リスク: 静かな部屋でも、マイクのホワイトノイズだけで「何かが喋っている」と VAD が誤反応しやすくなります。
    ///
    /// 【推奨される調整手順】
    /// 1. まず 0.005 程度で試し、もし喋っていないのに VAD ログ（Speech START）が頻出するようなら少しずつ値を上げます。
    /// 2. 喋り始めが欠ける（認識されない）場合は、この値を下げてください。
    pub signal_rms_threshold: f32,
    /// 有意な音声の占有率閾値（窓全体に対する割合、0.0 ~ 1.0）
    pub signal_occupancy_ratio: f32,

    // ========================================
    // ノイズ除去 (GTCRN) 設定
    // ========================================
    /// ノイズ除去を有効化するか
    pub use_denoiser: bool,
    /// デノイザーモデルへのパス
    pub denoiser_model_path: String,

    // ========================================
    // 最終補正レイヤー
    // ========================================
    /// 最終補正レイヤーを起動する文数閾値
    pub post_correction_sentence_count_threshold: usize,
    /// 最終補正レイヤーを起動する最小文字数（文数条件の補助）
    pub post_correction_min_text_length: usize,
    /// 最終補正レイヤーを起動する長期沈黙時間（ミリ秒）
    pub post_correction_interval_ms: u64,
}

impl Default for StreamerConfig {
    fn default() -> Self {
        Self {
            vad_model_path: String::new(),
            vad_type: VadType::Silero,
            vad_threshold: 0.5,
            vad_min_silence_duration: 0.2,
            vad_min_speech_duration: 0.25,
            vad_max_speech_duration: 25.0,
            vad_pre_padding_ms: 100,
            utterance_min_ms: 300,
            num_threads: 4,
            locale: StreamerLocale::Ja,
            signal_check_enabled: true,
            signal_rms_threshold: DEFAULT_SIGNAL_RMS_THRESHOLD,
            signal_occupancy_ratio: DEFAULT_SIGNAL_OCCUPANCY_RATIO,
            use_denoiser: false,
            denoiser_model_path: String::new(),
            post_correction_sentence_count_threshold: 3,
            post_correction_min_text_length: 10,
            post_correction_interval_ms: 2000,
        }
    }
}

// ============================================================================
// Chunk: VAD で切り出された発話区間
// ============================================================================

/// VAD によって検出された1つの発話区間
#[derive(Debug, Clone)]
struct Chunk {
    /// 16kHz モノラル音声データ
    samples: Vec<f32>,
    /// 識別用の連番ID
    id: u64,
}

impl Chunk {
    fn new(samples: Vec<f32>, id: u64) -> Self {
        Self { samples, id }
    }
}

// ============================================================================
// UtteranceQueue: 発話区間の一時キュー
// ============================================================================

#[derive(Debug, Default)]
struct UtteranceQueue {
    /// 保持されている発話区間のリスト
    utterance: VecDeque<Chunk>,
}

impl UtteranceQueue {
    fn new() -> Self {
        Self::default()
    }

    fn push(&mut self, chunk: Chunk) {
        self.utterance.push_back(chunk);
    }

    fn clear(&mut self) {
        self.utterance.clear();
    }

    fn pop_front(&mut self) -> Option<Chunk> {
        self.utterance.pop_front()
    }
}

// ============================================================================
// PseudoAsrStreamer: メインオーケストレーター
// ============================================================================

/// 擬似ストリーミング ASR オーケストレーター
///
/// オフライン推論エンジンを包み込み、マイク入力を受け取って
/// リアルタイムな逐次認識結果を出力します。
pub struct PseudoAsrStreamer<B: AsrBackend + Send + Sync + 'static> {
    /// ストリーマーの設定項目
    config: StreamerConfig,

    /// 実際の認識処理を行うエンジン本体
    /// PostCorrectionProcessor からも参照するため、Arc<Mutex> で共有
    backend: Arc<Mutex<B>>,

    /// 最終補正レイヤープロセッサ
    /// バッファリングと補正ロジックを担当
    post_correction_processor: PostCorrectionProcessor,

    /// 外部（UIなど）へ結果を送信するためのチャンネル
    tx: mpsc::Sender<StreamerEvent>,

    /// 現在オーディオ入力・認識が動作しているかどうかのフラグ
    is_running: Arc<AtomicBool>,

    /// 外部から push_samples() で届いた音声データを一時的に蓄積するバッファ
    audio_buf: Vec<f32>,
    /// 現在のリサンプラーが対応している入力サンプリングレート
    input_sample_rate: u32,
    /// 入力レートを 16kHz に変換するためのリサンプラー
    resampler: Option<Box<dyn InternalResampler + Send>>,

    /// Sherpa-ONNX の VAD インスタンスへのポインタ
    vad: *const sys::SherpaOnnxVoiceActivityDetector,
    /// VAD に渡す前の固定長ウィンドウ切り出し用バッファ
    vad_buf: Vec<f32>,
    /// VAD モデルが要求するウィンドウサイズ (512 または 256)
    vad_window_size: usize,
    /// 直前のフレームで音声が検出されていたかどうか
    was_speech: bool,
    /// 現在の発話が開始された時刻（タイムアウト判定用）
    current_speech_start: Option<Instant>,
    /// 最後に ASR 結果（テキスト）が変化した時刻（インテリジェントタイムアウト用）
    last_asr_text_change: Instant,
    /// 最後に確定した ASR テキスト（変化検出用）
    last_asr_text: String,
    /// VAD で切り出された発話区間を複数保持するキュー
    utterance_queue: UtteranceQueue,
    /// 現在蓄積中の発話区間の音声データ
    utterance_buf: Vec<f32>,
    /// 生成したチャンクの累計数（ID用）
    utterance_id_counter: u64,
    /// 発話開始前の音声を保持しておくためのリングバッファ
    pre_padding_buf: VecDeque<f32>,
    /// プリパディングとして保持するサンプル数
    pre_padding_samples_cnt: usize,

    /// GTCRN ノイズ除去モジュール
    denoiser: Option<SpeechDenoiser>,
}

impl<B: AsrBackend + Send + Sync + 'static> PseudoAsrStreamer<B> {
    /// 新しいストリーマーを作成します。
    pub fn new(
        backend: B,
        tx: mpsc::Sender<StreamerEvent>,
        config: StreamerConfig,
    ) -> Result<Self> {
        let sample_rate = INTERNAL_TARGET_RATE as usize;
        let pre_padding_samples_cnt =
            (config.vad_pre_padding_ms as usize * sample_rate) / MS_PER_SEC;
        let vad_window_size = match config.vad_type {
            VadType::Silero => SILERO_VAD_WINDOW_SIZE,
            VadType::Ten => TEN_VAD_WINDOW_SIZE,
        };

        // PostCorrectionProcessor の初期化
        // backend を Arc<Mutex> で包み、Wrapper を通じて共有する
        let shared_backend = Arc::new(Mutex::new(backend));
        let backend_wrapper = Arc::new(BackendWrapper(shared_backend.clone()));

        let pc_config = PostCorrectionConfig {
            sentence_count_threshold: config.post_correction_sentence_count_threshold,
            min_text_length: config.post_correction_min_text_length,
            interval_ms: config.post_correction_interval_ms,
        };

        let backend_replaces = shared_backend
            .lock()
            .expect("Failed to lock backend")
            .replaces();
        let post_correction_processor =
            PostCorrectionProcessor::new(backend_wrapper, pc_config, backend_replaces);

        Ok(Self {
            config,
            backend: shared_backend,
            post_correction_processor,
            tx,
            is_running: Arc::new(AtomicBool::new(false)),
            audio_buf: Vec::with_capacity(INTERNAL_TARGET_RATE as usize),
            input_sample_rate: 0, // Will be set on first push_samples call
            resampler: None,
            vad: std::ptr::null(),
            vad_buf: Vec::with_capacity(vad_window_size),
            vad_window_size,
            was_speech: false,
            current_speech_start: None,
            last_asr_text_change: Instant::now(),
            last_asr_text: String::new(),
            utterance_queue: UtteranceQueue::new(),
            utterance_buf: Vec::new(),
            utterance_id_counter: 0,
            pre_padding_buf: VecDeque::with_capacity(pre_padding_samples_cnt),
            pre_padding_samples_cnt,
            denoiser: None,
        })
    }

    /// 外部から音声サンプルを受け取り、必要に応じてリサンプリングして内部バッファに追加します。
    ///
    /// # Arguments
    /// * `samples` - モノラル f32 音声データ
    /// * `sample_rate` - 入力のサンプリングレート (Hz)
    pub fn push_samples(&mut self, samples: &[f32], sample_rate: u32) {
        if samples.is_empty() {
            return;
        }

        // サンプルレートが変わった場合はリサンプラーを再初期化
        if sample_rate != self.input_sample_rate {
            self.input_sample_rate = sample_rate;
            if let Err(e) = self.init_resampler() {
                log::error!(
                    "[PseudoAsrStreamer] Failed to init resampler for {}Hz: {}",
                    sample_rate,
                    e
                );
                return;
            }
        }

        // リサンプリングが必要なら適用
        let resampled = if let Some(ref mut resampler) = self.resampler {
            match resampler.process(samples) {
                Ok(s) => s,
                Err(e) => {
                    log::error!("[PseudoAsrStreamer] Resampling error: {:?}", e);
                    return;
                }
            }
        } else {
            samples.to_vec()
        };

        self.audio_buf.extend(resampled);
    }

    /// リサンプラーを初期化または再初期化します。
    fn init_resampler(&mut self) -> Result<()> {
        if self.input_sample_rate == INTERNAL_TARGET_RATE || self.input_sample_rate == 0 {
            self.resampler = None;
            return Ok(());
        }

        log::debug!(
            "[PseudoAsrStreamer] Initializing resampler: {}Hz -> {}Hz",
            self.input_sample_rate,
            INTERNAL_TARGET_RATE
        );
        let resampler = SincResampler::new(self.input_sample_rate, INTERNAL_TARGET_RATE)
            .map_err(|e| anyhow!("Failed to create resampler: {:?}", e))?;

        self.resampler = Some(Box::new(resampler));
        Ok(())
    }

    /// 認識を開始し、VAD および句読点エンジンを初期設定します。
    pub fn start(&mut self) -> Result<()> {
        self.is_running.store(true, Ordering::SeqCst);
        log::debug!("Recognition started.");

        // VAD 初期化
        match self.init_vad() {
            Ok(_) => log::debug!("VAD initialized."),
            Err(e) => {
                log::error!("Failed to init VAD: {}", e);
                return Err(anyhow!("Failed to init VAD: {}", e));
            }
        }

        // デノイザー初期化
        if self.config.use_denoiser && !self.config.denoiser_model_path.is_empty() {
            match SpeechDenoiser::new(&self.config.denoiser_model_path, self.config.num_threads) {
                Ok(d) => {
                    log::debug!(
                        "[PseudoAsrStreamer] GTCRN Denoiser initialized: model={}",
                        self.config.denoiser_model_path
                    );
                    self.denoiser = Some(d);
                }
                Err(e) => {
                    log::error!("Failed to init denoiser: {}", e);
                }
            }
        }

        Ok(())
    }

    /// 認識を停止し、各種バッファおよびリソースをクリーンアップします。
    pub fn stop(&mut self) {
        self.is_running.store(false, Ordering::SeqCst);
        log::debug!("Recognition stopped.");

        // 停止時にバッファに残っているテキストがあれば強制確定
        // Async な処理を呼び出す必要があるが、stop は同期関数。
        // ここではランタイムハンドルの block_on を使うなどのハックは避け、
        // tokio::spawn で投げっぱなしにするか、プロセッサのリセットだけ行う。
        // ※ 本来は force_commit を呼びたいが、async function cannot be called here properly without await.
        //   best effort でリセットのみとするか、親が stop する前に force_commit すべき。
        //   今回は processor.reset() を呼んでクリーンにする。
        self.post_correction_processor.reset();

        // 1. オーディオバッファのクリア
        self.audio_buf.clear();
        self.pre_padding_buf.clear();
        self.vad_buf.clear();

        // 2. コンテキスト・補正バッファのクリア
        self.utterance_queue.clear();
        self.utterance_buf.clear();
        if let Some(ref mut resampler) = self.resampler {
            resampler.reset();
        }

        // 3. 内部状態フラグ・カウンタのリセット
        self.was_speech = false;
        self.utterance_id_counter = 0;
        self.input_sample_rate = 0; // 次回の push_samples でリサンプラー再初期化

        // 4. オプションコンポーネントのクリア
        self.denoiser = None;

        // VAD の破棄
        if !self.vad.is_null() {
            unsafe { sys::SherpaOnnxDestroyVoiceActivityDetector(self.vad) };
            self.vad = std::ptr::null();
        }
    }

    fn init_vad(&mut self) -> Result<()> {
        let vad_model = &self.config.vad_model_path;
        log::debug!(
            "[PseudoAsrStreamer] Initializing VAD (type: {:?}) with model: {}",
            self.config.vad_type,
            vad_model
        );

        let c_model = CString::new(vad_model.as_str())?;
        let c_provider = CString::new("cpu")?; // "cpu" by default

        let mut vad_config: sys::SherpaOnnxVadModelConfig = unsafe { mem::zeroed() };

        match self.config.vad_type {
            VadType::Ten => {
                vad_config.ten_vad.model = c_model.as_ptr();
                vad_config.ten_vad.threshold = self.config.vad_threshold;
                vad_config.ten_vad.min_silence_duration = self.config.vad_min_silence_duration;
                vad_config.ten_vad.min_speech_duration = self.config.vad_min_speech_duration;
                vad_config.ten_vad.window_size = TEN_VAD_WINDOW_SIZE as i32;
                self.vad_window_size = TEN_VAD_WINDOW_SIZE;
            }
            VadType::Silero => {
                vad_config.silero_vad.model = c_model.as_ptr();
                vad_config.silero_vad.threshold = self.config.vad_threshold;
                vad_config.silero_vad.min_silence_duration = self.config.vad_min_silence_duration;
                vad_config.silero_vad.min_speech_duration = self.config.vad_min_speech_duration;
                vad_config.silero_vad.window_size = SILERO_VAD_WINDOW_SIZE as i32;
                self.vad_window_size = SILERO_VAD_WINDOW_SIZE;
            }
        }

        vad_config.sample_rate = INTERNAL_TARGET_RATE as i32;
        vad_config.num_threads = self.config.num_threads;
        vad_config.provider = c_provider.as_ptr(); // "cpu" by default
        vad_config.debug = 0;

        log::debug!(
            "[PseudoAsrStreamer] Initializing VAD with buffer duration: {:.1}s",
            self.config.vad_max_speech_duration
        );
        let vad = unsafe {
            sys::SherpaOnnxCreateVoiceActivityDetector(
                &vad_config,
                self.config.vad_max_speech_duration,
            )
        };
        if vad.is_null() {
            return Err(anyhow!("Failed to create SherpaOnnxVoiceActivityDetector"));
        }
        self.vad = vad;

        Ok(())
    }

    /// メインループ（心拍）として機能し、オーディオの処理、VAD 判定、推論の実行を制御します。
    pub fn tick(&mut self) {
        if !self.is_running.load(Ordering::SeqCst) {
            return;
        }

        // 1. オーディオデータの処理
        let samples = self.process_audio();
        if !samples.is_empty() {
            self.vad_buf.extend(samples);
        }

        // 2. VAD 判定
        let vad_window_size = self.vad_window_size;
        while self.vad_buf.len() >= vad_window_size {
            let vad_window: Vec<f32> = self.vad_buf.drain(0..vad_window_size).collect();
            self.handle_vad(&vad_window);

            for &s in &vad_window {
                if self.pre_padding_buf.len() >= self.pre_padding_samples_cnt {
                    self.pre_padding_buf.pop_front();
                }
                self.pre_padding_buf.push_back(s);
            }
        }

        // 3. 発話区間音声を蓄積しているキューを処理する
        // 補足: process_utterance_queue は現在非同期処理（重たいLLM操作を含む）を行いますが、
        // tick メソッド自体は同期的です。そのため、ブロックするか spawn する必要があります。
        // PseudoAsrStreamer が非同期チャネルを想定して設計されている一方で、tick が同期的であるという
        // アーキテクチャ上の制約は扱いにくい点です。
        //
        // 幸い、`openai.rs` 内の `ticker_task`（非同期）から `tick` が呼び出されています。
        // しかし `PseudoAsrStreamer::tick` のシグネチャは `fn tick(&mut self)` です。
        // これをきれいに解決するため、`process_utterance_queue` 内部で `tokio::task::block_in_place` を使用するか、
        // ランタイムハンドルに依存する形をとります。
        //
        // ここでは、マルチスレッドランタイム内で実行されていることを前提として、
        // 非同期関数を呼び出す際に内部で `tokio::task::block_in_place` を使用するようにします。
        self.process_utterance_queue();
    }

    fn process_audio(&mut self) -> Vec<f32> {
        if self.audio_buf.is_empty() {
            return Vec::new();
        }

        // バッファからデータを取り出す（push_samples で既にリサンプル済み）
        mem::take(&mut self.audio_buf)
    }

    fn handle_vad(&mut self, vad_window: &[f32]) {
        if self.vad.is_null() {
            return;
        }

        let is_speech_vad = unsafe {
            sys::SherpaOnnxVoiceActivityDetectorAcceptWaveform(
                self.vad,
                vad_window.as_ptr(),
                vad_window.len() as i32,
            );
            sys::SherpaOnnxVoiceActivityDetectorDetected(self.vad) == 1
        };

        // インテリジェントタイムアウト判定:
        // 以下の条件がすべて満たされた場合のみ強制終了
        // 1. 発話開始から vad_max_speech_duration 秒以上経過
        // 2. ASR テキストがここ5秒間変化していない（意味のある発話がない）
        // 3. 現在のウィンドウのRMSが閾値未満（ノイズレベル）
        let is_intelligent_timeout = if let Some(start_time) = self.current_speech_start {
            let elapsed_since_start = start_time.elapsed().as_secs_f32();
            let elapsed_since_text_change = self.last_asr_text_change.elapsed().as_secs_f32();

            // 条件1: 最大発話時間を超過
            let time_exceeded = elapsed_since_start >= self.config.vad_max_speech_duration;

            // 条件2: ASRテキストが5秒以上変化していない
            const ASR_STAGNATION_THRESHOLD_SECS: f32 = 5.0;
            let asr_stagnant = elapsed_since_text_change >= ASR_STAGNATION_THRESHOLD_SECS;

            // 条件3: 現在のウィンドウのRMSがノイズレベル（信号が弱い）
            let rms = self.calculate_rms(vad_window);
            let is_low_signal = rms < self.config.signal_rms_threshold;

            let should_timeout = time_exceeded && asr_stagnant && is_low_signal;

            if time_exceeded {
                log::debug!(
                    "[VAD] Timeout check: elapsed={:.1}s >= max={:.1}s, asr_stagnant={} ({:.1}s), low_signal={} (RMS={:.4})",
                    elapsed_since_start,
                    self.config.vad_max_speech_duration,
                    asr_stagnant,
                    elapsed_since_text_change,
                    is_low_signal,
                    rms
                );
            }

            should_timeout
        } else {
            false
        };

        // VADが反応している、かつインテリジェントタイムアウトでない場合は「音声継続」
        if is_speech_vad && !is_intelligent_timeout {
            if !self.was_speech {
                log::debug!("Speech START.");
                // UIフリッカー防止のため、現在のバッファ内容を取得して送信
                let current_display_text = self.post_correction_processor.get_display_text();
                let _ = self
                    .tx
                    .try_send(StreamerEvent::SpeechStart(current_display_text));
                self.utterance_buf.extend(self.pre_padding_buf.iter());
                // 発話開始時刻を記録
                self.current_speech_start = Some(Instant::now());
                // ASRテキスト変化追跡もリセット
                self.last_asr_text_change = Instant::now();
                self.last_asr_text.clear();
                log::debug!(
                    "[VAD] Speech timer started (max: {:.1}s)",
                    self.config.vad_max_speech_duration
                );
            }
            self.utterance_buf.extend_from_slice(vad_window);
            self.was_speech = true;
        }
        // 「VADが沈黙を検出した」または「インテリジェントタイムアウトが発生した」場合
        else if self.was_speech {
            if is_intelligent_timeout {
                log::warn!(
                    "Speech END (Intelligent Timeout: max_duration={:.1}s exceeded, ASR stagnant, low signal).",
                    self.config.vad_max_speech_duration
                );
            } else {
                log::debug!("Speech END (VAD silence detected).");
            }

            // --- 終了処理の実行 ---
            // --- 終了処理の実行 ---
            // UIフリッカー防止のため、現在のバッファ内容を取得して送信
            let current_display_text = self.post_correction_processor.get_display_text();
            let _ = self
                .tx
                .try_send(StreamerEvent::SpeechEnd(current_display_text));

            // ASR推論、キューイング、バッファクリア等が行われる
            self.process_one_utterance();

            // 状態のリセット
            self.was_speech = false;
            self.current_speech_start = None;
            log::debug!("[VAD] Speech timer reset.");

            // ******************************************************
            // 発話終了時の強制コミット (オプションだが推奨)
            // ******************************************************
            // 沈黙が検出されたときに、未確定の「ぶら下がっている」テキストを強制的にコミットしたい場合があります。
            // しかし、通常はさらなる文脈を待つか、手動トリガーを待ちます。
            //
            // `PostCorrectionProcessor` には、より多くのテキストを待つロジックがあります。
            // しかし、ユーザーが話すのをやめた場合、コミットすべきでしょうか？
            // 設定の `silence_duration_ms_for_post_correction` は `process_input` 経由（経過時間チェック）でこれを処理します。
            // しかし、`process_input` は *新しい* テキストがある場合にのみ実行されます。
            // 沈黙が続いた場合、時間チェックをトリガーするための新しいテキストが得られない可能性があります。
            // したがって、ここで時間ベースのチェックをトリガーするか、定期的にチェックする仕組みが必要かもしれません。
            //
            // 実際、`process_utterance_queue` は発話キューをループ処理します。
            // キューが空の場合、`process_input` は呼び出されません。
            // タイムアウトコミットを確認するための独立したメカニズムが必要になる可能性があります。
            // しかし今のところはシンプルに保ち、次の入力（たとえ空やノイズであっても）または単純なロジックに依存することにします。
            // 現在の設計では、`PostCorrectionProcessor` は `should_trigger_correction` 内で `last_correction_time` をチェックします。
            // これは `process_input` が呼び出されたときにのみチェックされます。
            // 純粋に沈黙時間に基づいてコミットしたい場合は、入力がなくても `processor` をポーリングする必要があります。
            // しかし `PostCorrectionProcessor` にはまだ `tick` メソッドがありません。
            // 現状は、「沈黙後の次の入力で補正がトリガーされる」あるいは「明示的な強制コミット」という挙動を維持します。
            //
            // [判断追記 2026-01-25]
            // 純粋に沈黙時間に基づいて自動コミットする処理は「導入しない」と判断しました。
            // 理由は、ほとんどの場合、ユーザーのクリックやキーボード操作（ホットキー等）によってコミットされるのが普通であり、
            // 勝手に沈黙時間でコミットが走る必要性が低いためです。
        }
    }

    /// RMS (Root Mean Square) を計算するヘルパー関数
    /// 信号の平均音圧を計算し、ノイズ判定に使用します。
    fn calculate_rms(&self, samples: &[f32]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }
        let sum_sq: f32 = samples.iter().map(|s| s * s).sum();
        (sum_sq / samples.len() as f32).sqrt()
    }

    fn process_one_utterance(&mut self) {
        if self.utterance_buf.is_empty() {
            return;
        }

        let tmp_utterance = Chunk::new(self.utterance_buf.clone(), self.utterance_id_counter);
        // tmp_utterance の音声データのミリ秒単位の長さを計算
        let tmp_utterance_duration_ms =
            tmp_utterance.samples.len() as u64 * 1000 / INTERNAL_TARGET_RATE as u64;
        // tmp_utterance_duration_ms が vad_max_speech_duration より大きい場合は、utterance を分割する
        let max_duration_ms = (self.config.vad_max_speech_duration * 1000.0_f32) as u64;
        if tmp_utterance_duration_ms > max_duration_ms {
            // vad_max_speech_duration 単位で切り出して分割（最後に端数が残ったら、それも一つの utterance にする）
            let samples_per_segment =
                (self.config.vad_max_speech_duration * INTERNAL_TARGET_RATE as f32) as usize;
            // 分割された結果を一つ一つ utterance_queue に追加する
            for chunk_samples in tmp_utterance.samples.chunks(samples_per_segment) {
                let segment = Chunk::new(chunk_samples.to_vec(), self.utterance_id_counter);
                log::debug!(
                    "[PseudoAsrStreamer] Large utterance split: ID={}, duration={:.2}s",
                    segment.id,
                    segment.samples.len() as f32 / INTERNAL_TARGET_RATE as f32
                );
                self.utterance_queue.push(segment);
                self.utterance_id_counter += 1;
            }
            self.utterance_buf.clear();
            self.pre_padding_buf.clear();
        } else {
            self.utterance_queue.push(tmp_utterance.clone());
            log::debug!(
                "[PseudoAsrStreamer] One utterance detected: ID={}, duration={:.2}s",
                tmp_utterance.id,
                tmp_utterance_duration_ms / 1000
            );
            self.utterance_id_counter += 1;
            self.utterance_buf.clear();
            self.pre_padding_buf.clear();
        }
    }

    fn process_utterance_queue(&mut self) {
        // self.utterance_queue から全ての utterance を取り出して処理
        while let Some(utterance) = self.utterance_queue.pop_front() {
            // ========================================
            // ノイズ除去 (GTCRN) - Optional
            // ========================================
            let samples_to_recognize = if let Some(denoiser) = &self.denoiser {
                match denoiser.run(&utterance.samples, INTERNAL_TARGET_RATE as i32) {
                    Ok(denoised) => {
                        log::trace!(
                            "[PseudoAsrStreamer] Denoised window: {} samples",
                            denoised.len()
                        );
                        denoised
                    }
                    Err(e) => {
                        log::error!("[PseudoAsrStreamer] Denoiser error: {}", e);
                        utterance.samples.clone()
                    }
                }
            } else {
                utterance.samples.clone()
            };
            // ========================================
            // 信号品質フィルタ (Pre-ASR Signal Filter)
            // 全体に対して有意な音声の占有率が低い場合、ASR をスキップ。
            // これにより、端の残響による「はい」等の幻聴を防ぐ。
            // ========================================
            if self.config.signal_check_enabled {
                if !self.is_worthy_to_run_asr(&samples_to_recognize) {
                    log::debug!("[SignalFilter] ASR_SKIP: audio signal below quality threshold");
                    continue;
                }
            }
            // ========================================
            // 認識実行 (バックエンド)
            // ========================================
            log::debug!("[PseudoAsrStreamer] Recognizing one utterance.");
            let start_time = std::time::Instant::now();

            // Use local scope to release lock immediately after usage if possible
            // But we need `self.backend` which is Arc<Mutex<B>>.
            // `transcribe` is synchronous method on trait.
            let window_text = {
                let mut guard = self.backend.lock().unwrap();
                match guard.transcribe(&samples_to_recognize) {
                    Ok(text) => {
                        let duration_ms = (samples_to_recognize.len() as u64 * 1000)
                            / (INTERNAL_TARGET_RATE as u64);
                        guard.record_asr_usage(duration_ms);
                        text
                    }
                    Err(e) => {
                        log::error!("Transcription failed: {}", e);
                        continue;
                    }
                }
            };

            let duration = start_time.elapsed();
            log::debug!(
                "Raw transcription: \"{}\" (took {:.2}s)",
                window_text,
                duration.as_secs_f32()
            );
            if window_text.is_empty() {
                continue;
            }

            // ========================================
            // 句読点付与
            // ========================================
            let punctuated_text = {
                let mut guard = self.backend.lock().unwrap();
                match guard.insert_punctuation(&window_text, &self.config.locale) {
                    Ok(text) => text,
                    Err(e) => {
                        log::error!("Punctuation failed: {}", e);
                        window_text.clone()
                    }
                }
            };

            // ========================================
            // 最終補正レイヤー (PostCorrectionProcessor) への委譲
            // ========================================
            log::debug!(
                "[PseudoAsrStreamer] Feeding text to processor: \"{}\"",
                punctuated_text
            );

            // Async execution via block_in_place
            let output_option = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async {
                    // Check if correction will be triggered (Predictive)
                    let will_trigger = self
                        .post_correction_processor
                        .should_trigger_correction(Some(&punctuated_text));

                    if will_trigger {
                        let _ = self.tx.try_send(StreamerEvent::PostCorrectionStarted);
                    }

                    // Execute process_input (which calls backend.post_correct if trigger matches)
                    let result = self
                        .post_correction_processor
                        .process_input(&punctuated_text)
                        .await;

                    if will_trigger {
                        let _ = self.tx.try_send(StreamerEvent::PostCorrectionFinished);
                    }

                    result
                })
            });

            if let Some(output) = output_option {
                match output {
                    ProcessorOutput::Partial(text) => {
                        // ASRテキスト変化の追跡（インテリジェントタイムアウト用）
                        if text != self.last_asr_text {
                            self.last_asr_text = text.clone();
                            self.last_asr_text_change = Instant::now();
                        }

                        // Send partial
                        match self.tx.try_send(StreamerEvent::PartialResult(text)) {
                            Ok(_) => {}
                            Err(e) => log::error!(
                                "[PseudoAsrStreamer] Failed to send partial result: {}",
                                e
                            ),
                        }
                    }
                    ProcessorOutput::Final(text) => {
                        // ASRテキスト変化の追跡更新
                        self.last_asr_text = text.clone();
                        self.last_asr_text_change = Instant::now();

                        // Send final (this will also trigger buffer reset in processor internally)
                        match self.tx.try_send(StreamerEvent::FinalResult(text)) {
                            Ok(_) => {}
                            Err(e) => log::error!(
                                "[PseudoAsrStreamer] Failed to send final result: {}",
                                e
                            ),
                        }
                    }
                }
            }
        }
    }

    /// 使用する言語ロケールを動的に設定します。
    pub fn set_locale(&mut self, locale: StreamerLocale) {
        self.config.locale = locale;
        log::debug!("Locale set to {:?}", self.config.locale);
    }

    /// ストリーマーが現在動作中かどうかを返します。
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::SeqCst)
    }

    // 以前存在した `get_org_text` は post_correction_processor 内に隠蔽されたため削除、
    // あるいは必要なら post_correction_processor から取得する getter を追加するが、
    // 現状外部からそこまで強く依存されていないはず（UI表示用はイベントで飛ぶ）。

    /// 信号品質チェック: 全体に占める有意な音声の割合と RMS を確認。
    fn is_worthy_to_run_asr(&self, samples: &[f32]) -> bool {
        if samples.is_empty() {
            log::debug!("[SignalFilter] empty sample array -> quality_fail");
            return false;
        }

        let utterance_duration_ms = (samples.len() as f32 / INTERNAL_TARGET_RATE as f32) * 1000.0;

        // self.config.utterance_min_ms の長さを sample が満たしていないならば false を返す
        if utterance_duration_ms < self.config.utterance_min_ms as f32 {
            log::debug!("[SignalFilter] utterance_min_ms not met -> quality_fail");
            return false;
        }

        // RMS (Root Mean Square) calculation
        let sum_sq: f32 = samples.iter().map(|s| s * s).sum();
        let rms = (sum_sq / samples.len() as f32).sqrt();

        // Count samples exceeding energy threshold
        let energy_threshold = self.config.signal_rms_threshold;
        let active_samples = samples
            .iter()
            .filter(|&s| s.abs() > energy_threshold)
            .count();
        let active_duration_ms = (active_samples as f32 / INTERNAL_TARGET_RATE as f32) * 1000.0;

        // Occupancy ratio = active samples / total samples
        let occupancy_ratio = active_samples as f32 / samples.len() as f32;

        let rms_ok = rms >= self.config.signal_rms_threshold;
        let occupancy_ok = occupancy_ratio >= self.config.signal_occupancy_ratio;
        let is_worthy = rms_ok && occupancy_ok;

        // Always log detailed info (debug for fail, trace for pass)
        let log_msg = format!(
            "[SignalFilter] window={:.0}ms, RMS={:.4} ({} threshold={:.4}), active={:.0}ms, occupancy={:.1}% ({} threshold={:.1}%)",
            utterance_duration_ms,
            rms,
            if rms_ok { "OK" } else { "NG" },
            self.config.signal_rms_threshold,
            active_duration_ms,
            occupancy_ratio * 100.0,
            if occupancy_ok { "OK" } else { "NG" },
            self.config.signal_occupancy_ratio * 100.0
        );

        if is_worthy {
            log::trace!("{} -> PASS", log_msg);
        } else {
            log::debug!("{} -> FAIL", log_msg);
        }

        is_worthy
    }
}

impl<B: AsrBackend + Send + Sync + 'static> Drop for PseudoAsrStreamer<B> {
    fn drop(&mut self) {
        if !self.vad.is_null() {
            unsafe { sys::SherpaOnnxDestroyVoiceActivityDetector(self.vad) };
        }
    }
}

unsafe impl<B: AsrBackend + Send + Sync> Send for PseudoAsrStreamer<B> {}
