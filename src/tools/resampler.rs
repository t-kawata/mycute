//! resampler.rs - 音声データのサンプリングレートを変換する共通ツール
//!
//! rubato クレートを使用して、高品質な Sinc 補間によるリサンプリングを提供します。

use anyhow::Result;
use rubato::{
    Resampler as RubatoResampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType,
    WindowFunction,
};

#[derive(Debug)]
pub enum ResamplerError {
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

/// リサンプラーの共通トレイト
pub trait InternalResampler: Send {
    /// 音声サンプルをリサンプリングします。
    /// 入力データが不十分な場合、内部に残存データとして保持され、次回の呼び出しで使用されます。
    fn process(&mut self, input: &[f32]) -> Result<Vec<f32>, ResamplerError>;

    /// 内部の残存データをクリアします。
    fn reset(&mut self);
}

pub struct SincResampler {
    inner: SincFixedIn<f32>,
    residual: Vec<f32>,
    input_rate: u32,
    output_rate: u32,
}

impl SincResampler {
    /// 新しい SincResampler を作成します。
    pub fn new(input_rate: u32, output_rate: u32) -> Result<Self, ResamplerError> {
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
            input_rate,
            output_rate,
        })
    }

    pub fn input_rate(&self) -> u32 {
        self.input_rate
    }

    pub fn output_rate(&self) -> u32 {
        self.output_rate
    }
}

impl InternalResampler for SincResampler {
    fn process(&mut self, input: &[f32]) -> Result<Vec<f32>, ResamplerError> {
        if input.is_empty() {
            return Ok(Vec::new());
        }

        // 前回の残存データと今回の入力を結合
        let mut all_samples = std::mem::take(&mut self.residual);
        all_samples.extend_from_slice(input);

        let mut output = Vec::new();
        let mut offset = 0;

        while offset < all_samples.len() {
            let frames_needed = self.inner.input_frames_next();
            if offset + frames_needed > all_samples.len() {
                break;
            }

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

        // 処理しきれなかった分を残存データとして保持
        self.residual = all_samples[offset..].to_vec();

        Ok(output)
    }

    fn reset(&mut self) {
        self.residual.clear();
    }
}
