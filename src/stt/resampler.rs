//! Audio resampling abstraction layer.
//!
//! This module provides an abstract `AudioResampler` trait and concrete implementations
//! for converting audio sample rates. The abstraction allows swapping implementations
//! without changing the consuming code.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────┐
//! │       AudioResampler Trait      │  ← Abstract interface
//! └─────────────────────────────────┘
//!                  ▲
//!                  │ implements
//! ┌────────────────┴────────────────┐
//! │         SincResampler           │  ← Concrete: rubato 0.16.x
//! └─────────────────────────────────┘
//! ```

use rubato::{
    Resampler as RubatoResampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType,
    WindowFunction,
};

/// Target sample rate for Sherpa-ONNX (16kHz)
pub const TARGET_SAMPLE_RATE: u32 = 16000;

/// Error type for resampling operations.
#[derive(Debug)]
pub enum ResamplerError {
    /// Failed to create resampler with given parameters
    CreationFailed(String),
    /// Resampling process failed
    ProcessFailed(String),
    /// Invalid configuration
    InvalidConfig(String),
}

impl std::fmt::Display for ResamplerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResamplerError::CreationFailed(msg) => write!(f, "Resampler creation failed: {}", msg),
            ResamplerError::ProcessFailed(msg) => write!(f, "Resampling failed: {}", msg),
            ResamplerError::InvalidConfig(msg) => write!(f, "Invalid config: {}", msg),
        }
    }
}

impl std::error::Error for ResamplerError {}

/// Abstract trait for audio resampling.
///
/// Implementations of this trait can use different resampling algorithms
/// (e.g., sinc interpolation, linear interpolation, FFT-based).
/// The trait is designed to be swappable without changing consuming code.
pub trait AudioResampler: Send {
    /// Process input samples and produce resampled output.
    ///
    /// # Arguments
    /// * `input` - Mono audio samples at the input sample rate
    ///
    /// # Returns
    /// Resampled mono audio samples at the target rate
    fn process(&mut self, input: &[f32]) -> Result<Vec<f32>, ResamplerError>;

    /// Get the number of input frames required for the next process call.
    fn _input_frames_next(&self) -> usize;

    /// Get the input sample rate.
    fn _input_sample_rate(&self) -> u32;

    /// Get the output sample rate.
    fn _output_sample_rate(&self) -> u32;

    /// Reset the internal state (useful when starting a new audio stream).
    fn _reset(&mut self);
}

/// High-quality sinc interpolation resampler using rubato.
///
/// This implementation uses `SincFixedIn` from rubato 0.16.x which provides
/// high-quality resampling with configurable sinc filter parameters.
pub struct SincResampler {
    inner: SincFixedIn<f32>,
    _input_rate: u32,
    _output_rate: u32,
    _chunk_size: usize,
    residual: Vec<f32>,
}

impl SincResampler {
    /// Create a new sinc resampler.
    ///
    /// # Arguments
    /// * `input_rate` - Input sample rate (e.g., 44100, 48000)
    /// * `output_rate` - Output sample rate (e.g., 16000)
    /// * `chunk_size` - Number of input frames per processing chunk
    ///
    /// # Returns
    /// A configured `SincResampler` or an error if creation fails.
    pub fn new(
        input_rate: u32,
        output_rate: u32,
        chunk_size: usize,
    ) -> Result<Self, ResamplerError> {
        if input_rate == 0 || output_rate == 0 {
            return Err(ResamplerError::InvalidConfig(
                "Sample rates must be non-zero".to_string(),
            ));
        }

        if chunk_size == 0 {
            return Err(ResamplerError::InvalidConfig(
                "Chunk size must be non-zero".to_string(),
            ));
        }

        let resample_ratio = output_rate as f64 / input_rate as f64;

        // High-quality sinc interpolation parameters
        let params = SincInterpolationParameters {
            sinc_len: 256,
            f_cutoff: 0.95,
            interpolation: SincInterpolationType::Linear,
            oversampling_factor: 256,
            window: WindowFunction::BlackmanHarris2,
        };

        let inner = SincFixedIn::<f32>::new(
            resample_ratio,
            2.0, // max_resample_ratio_relative
            params,
            chunk_size,
            1, // 1 channel (mono)
        )
        .map_err(|e| ResamplerError::CreationFailed(format!("{:?}", e)))?;

        log::debug!(
            "[Resampler] SincResampler created: {} Hz -> {} Hz (ratio: {:.4}, chunk: {})",
            input_rate,
            output_rate,
            resample_ratio,
            chunk_size
        );

        Ok(Self {
            inner,
            _input_rate: input_rate,
            _output_rate: output_rate,
            _chunk_size: chunk_size,
            residual: Vec::new(),
        })
    }

    /// Create a resampler configured for Sherpa-ONNX (16kHz output).
    pub fn new_for_sherpa(input_rate: u32) -> Result<Self, ResamplerError> {
        Self::new(input_rate, TARGET_SAMPLE_RATE, 1024)
    }
}

impl AudioResampler for SincResampler {
    fn process(&mut self, input: &[f32]) -> Result<Vec<f32>, ResamplerError> {
        if input.is_empty() {
            return Ok(Vec::new());
        }

        // Combine residual with new input
        let mut all_samples = std::mem::take(&mut self.residual);
        all_samples.extend_from_slice(input);

        let mut output = Vec::new();
        let frames_needed = self.inner.input_frames_next();

        // Process complete chunks
        let mut offset = 0;
        while offset + frames_needed <= all_samples.len() {
            let chunk = &all_samples[offset..offset + frames_needed];

            // rubato 0.16.x uses Vec<Vec<f32>> format - outer vec is channels, inner is samples
            let input_vecs = vec![chunk.to_vec()];

            match self.inner.process(&input_vecs, None) {
                Ok(result) => {
                    if !result.is_empty() && !result[0].is_empty() {
                        output.extend_from_slice(&result[0]);
                    }
                }
                Err(e) => {
                    return Err(ResamplerError::ProcessFailed(format!("{:?}", e)));
                }
            }

            offset += frames_needed;
        }

        // Save remaining samples for next call
        self.residual = all_samples[offset..].to_vec();

        Ok(output)
    }

    fn _input_frames_next(&self) -> usize {
        self.inner.input_frames_next()
    }

    fn _input_sample_rate(&self) -> u32 {
        self._input_rate
    }

    fn _output_sample_rate(&self) -> u32 {
        self._output_rate
    }

    fn _reset(&mut self) {
        self.inner.reset();
        self.residual.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sinc_resampler_creation() {
        let resampler = SincResampler::new(48000, 16000, 1024);
        assert!(resampler.is_ok());

        let r = resampler.unwrap();
        assert_eq!(r._input_sample_rate(), 48000);
        assert_eq!(r._output_sample_rate(), 16000);
    }

    #[test]
    fn test_sinc_resampler_invalid_params() {
        assert!(SincResampler::new(0, 16000, 1024).is_err());
        assert!(SincResampler::new(48000, 0, 1024).is_err());
        assert!(SincResampler::new(48000, 16000, 0).is_err());
    }

    #[test]
    fn test_resampler_process() {
        let mut resampler = SincResampler::new(48000, 16000, 512).unwrap();

        // Generate 1 second of silence at 48kHz
        let input: Vec<f32> = vec![0.0; 48000];

        let output = resampler.process(&input).unwrap();

        // Should produce approximately 16000 samples (some variance due to filter delay)
        assert!(output.len() > 15000 && output.len() < 17000);
    }
}
