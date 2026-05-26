//! Kernel-specific error types.

use declorch_types::error::DeclorchError;
use thiserror::Error;

/// Kernel error type wrapping DeclorchError with kernel-specific context.
#[derive(Error, Debug)]
pub enum KernelError {
    /// A wrapped DeclorchError.
    #[error(transparent)]
    Declorch(#[from] DeclorchError),

    /// The kernel failed to boot.
    #[error("Boot failed: {0}")]
    BootFailed(String),
}

/// Alias for kernel results.
pub type KernelResult<T> = Result<T, KernelError>;
