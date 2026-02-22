//! VadProcessor - Sherpa-ONNX ベースの共通 VAD プロセッサ
//!
//! OpenAI モードおよび OS モード (Mac/Win) で共通して使用可能な
//! 高精度な音声区間検出 (VAD) 機能を提供します。

use std::ffi::CString;
use std::mem;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use anyhow::{anyhow, Result};
use sherpa_rs_sys as sys;

/// 内部処理用サンプリングレート (16kHz)
pub const VAD_SAMPLE_RATE: i32 = 16000;
/// Silero VAD 推奨ウィンドウサイズ (16kHz 時、32ms 相当)
pub const SILERO_VAD_WINDOW_SIZE: usize = 512;
/// TEN VAD 推奨ウィンドウサイズ (16kHz 時、16ms 相当)
pub const TEN_VAD_WINDOW_SIZE: usize = 256;

/// VAD モデルの種類
#[derive(Debug, Clone, Copy)]
pub enum VadType {
    /// Silero VAD (高精度、やや重い)
    Silero,
    /// TEN VAD (軽量)
    Ten,
}

/// VAD プロセッサの設定
#[derive(Debug, Clone)]
pub struct VadConfig {
    pub vad_type: VadType,
    pub model_path: String,
    pub threshold: f32,
    pub min_silence_duration: f32,
    pub min_speech_duration: f32,
    pub max_speech_duration: f32,
    pub num_threads: i32,
}

/// Sherpa-ONNX VAD を用いて発話状態を管理するプロセッサ
pub struct VadProcessor {
    vad: *const sys::SherpaOnnxVoiceActivityDetector,
    is_speaking: Arc<AtomicBool>,
    window_size: usize,
}

unsafe impl Send for VadProcessor {}
unsafe impl Sync for VadProcessor {}

impl VadProcessor {
    /// 新しい VadProcessor を作成します。
    pub fn new(config: VadConfig, is_speaking: Arc<AtomicBool>) -> Result<Self> {
        let c_model = CString::new(config.model_path.as_str())?;
        let c_provider = CString::new("cpu")?;

        let mut vad_config: sys::SherpaOnnxVadModelConfig = unsafe { mem::zeroed() };
        let window_size = match config.vad_type {
            VadType::Ten => {
                vad_config.ten_vad.model = c_model.as_ptr();
                vad_config.ten_vad.threshold = config.threshold;
                vad_config.ten_vad.min_silence_duration = config.min_silence_duration;
                vad_config.ten_vad.min_speech_duration = config.min_speech_duration;
                vad_config.ten_vad.window_size = TEN_VAD_WINDOW_SIZE as i32;
                TEN_VAD_WINDOW_SIZE
            }
            VadType::Silero => {
                vad_config.silero_vad.model = c_model.as_ptr();
                vad_config.silero_vad.threshold = config.threshold;
                vad_config.silero_vad.min_silence_duration = config.min_silence_duration;
                vad_config.silero_vad.min_speech_duration = config.min_speech_duration;
                vad_config.silero_vad.window_size = SILERO_VAD_WINDOW_SIZE as i32;
                SILERO_VAD_WINDOW_SIZE
            }
        };

        vad_config.sample_rate = VAD_SAMPLE_RATE;
        vad_config.num_threads = config.num_threads;
        vad_config.provider = c_provider.as_ptr();
        vad_config.debug = 0;

        let vad = unsafe {
            sys::SherpaOnnxCreateVoiceActivityDetector(&vad_config, config.max_speech_duration)
        };

        if vad.is_null() {
            return Err(anyhow!("Failed to create SherpaOnnxVoiceActivityDetector"));
        }

        Ok(Self {
            vad,
            is_speaking,
            window_size,
        })
    }

    /// 音声サンプルを入力し、VAD 状態を更新します。
    /// 渡されるデータは 16kHz モノラル f32 である必要があります。
    /// かつ、window_size（512 または 256）に適合している必要があります。
    pub fn accept_waveform(&self, samples: &[f32]) {
        if self.vad.is_null() || samples.is_empty() {
            return;
        }

        unsafe {
            sys::SherpaOnnxVoiceActivityDetectorAcceptWaveform(
                self.vad,
                samples.as_ptr(),
                samples.len() as i32,
            );

            // 発話検知状態を取得してアトミックフラグを更新
            let detected = sys::SherpaOnnxVoiceActivityDetectorDetected(self.vad) == 1;
            self.is_speaking.store(detected, Ordering::SeqCst);
        }
    }

    /// 現在の状態をリセットします。
    pub fn reset(&self) {
        if !self.vad.is_null() {
            unsafe {
                sys::SherpaOnnxVoiceActivityDetectorReset(self.vad);
            }
        }
        self.is_speaking.store(false, Ordering::SeqCst);
    }

    /// 期待されるウィンドウサイズを返します。
    pub fn window_size(&self) -> usize {
        self.window_size
    }

    /// 現在の発話状態を返します。
    pub fn is_speaking(&self) -> bool {
        self.is_speaking.load(Ordering::SeqCst)
    }
}

impl Drop for VadProcessor {
    fn drop(&mut self) {
        if !self.vad.is_null() {
            unsafe {
                sys::SherpaOnnxDestroyVoiceActivityDetector(self.vad);
            }
        }
    }
}
