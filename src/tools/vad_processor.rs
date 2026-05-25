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
        // 非ASCIIパスを含む場合でも SherpaOnnx がファイルを開けるよう、
        // Windows では 8.3 短縮名（ASCII only）に変換する
        let model_path = resolve_ascii_path(&config.model_path);
        let c_model = CString::new(model_path)?;
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

/// Windows 環境で、パスに非 ASCII 文字が含まれる場合に 8.3 短縮名を取得する。
///
/// SherpaOnnx の C API (fopen) が ANSI コードページを使用するため、日本語などの
/// マルチバイト文字を含むパスを正しく扱えない。以下の2段階で対処する：
///
/// 1. GetShortPathNameW で ASCII only の 8.3 短縮名に変換する
/// 2. 8.3 名が無効な場合、%PROGRAMDATA% 下にモデルファイルをコピーし、
///    ASCII-only が保証されたパスで SherpaOnnx に渡す
#[cfg(windows)]
fn resolve_ascii_path(path: &str) -> String {
    // Phase 1: 8.3 短縮名の取得を試行
    if let Some(short) = try_get_short_path(path) {
        if short.is_ascii() {
            log::debug!(
                "[VadProcessor] Resolved ASCII-safe path via GetShortPathNameW: {} -> {}",
                path, short
            );
            return short;
        }
    }

    // Phase 2: 8.3 名が無効なボリューム向けに %PROGRAMDATA% 下へキャッシュコピー
    log::warn!(
        "[VadProcessor] 8.3 short name unavailable. Falling back to PROGRAMDATA cache copy."
    );
    copy_to_ascii_cache(path)
}

/// GetShortPathNameW で 8.3 短縮名を取得する。
/// 失敗時または 8.3 名が無効な場合は None を返す。
#[cfg(windows)]
fn try_get_short_path(path: &str) -> Option<String> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use winapi::um::fileapi::GetShortPathNameW;

    let wide: Vec<u16> = OsStr::new(path)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();

    let mut buf = vec![0u16; 260];
    let len = unsafe {
        GetShortPathNameW(wide.as_ptr(), buf.as_mut_ptr(), buf.len() as u32)
    } as usize;

    if len > 0 && len <= buf.len() {
        // len は null 終端を含むので -1 が実際の文字列長
        Some(String::from_utf16_lossy(&buf[..len - 1]))
    } else {
        None
    }
}

/// モデルファイルを %PROGRAMDATA%\mycute\vad-models\ にコピーし、
/// ASCII-only が保証されたパスを返す。
///
/// %PROGRAMDATA% は全 Windows ロケールで ASCII パス (C:\ProgramData) が保証されている。
/// コピーは初回のみ実行され、2回目以降はキャッシュが存在すればスキップされる。
#[cfg(windows)]
fn copy_to_ascii_cache(original_path: &str) -> String {
    let program_data = match std::env::var("PROGRAMDATA") {
        Ok(p) => p,
        Err(_) => {
            log::warn!(
                "[VadProcessor] PROGRAMDATA env var not set, \
                 cannot create ASCII-safe model cache"
            );
            return original_path.to_string();
        }
    };

    let cache_dir = std::path::Path::new(&program_data)
        .join("mycute")
        .join("vad-models");
    if !cache_dir.exists() {
        if let Err(e) = std::fs::create_dir_all(&cache_dir) {
            log::warn!(
                "[VadProcessor] Failed to create cache dir {}: {}",
                cache_dir.display(),
                e
            );
            return original_path.to_string();
        }
    }

    let original = std::path::Path::new(original_path);
    let filename = match original.file_name() {
        Some(f) => f,
        None => return original_path.to_string(),
    };
    let cache_path = cache_dir.join(filename);

    if !cache_path.exists() {
        if let Err(e) = std::fs::copy(original_path, &cache_path) {
            log::warn!(
                "[VadProcessor] Failed to copy model to cache: {}",
                e
            );
            return original_path.to_string();
        }
        log::info!(
            "[VadProcessor] Created ASCII-safe model cache: {} -> {}",
            original_path,
            cache_path.display()
        );
    }

    cache_path.to_string_lossy().to_string()
}

#[cfg(not(windows))]
fn resolve_ascii_path(path: &str) -> String {
    path.to_string()
}
