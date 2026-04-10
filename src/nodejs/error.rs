use thiserror::Error;

#[derive(Error, Debug)]
pub enum NodeError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Archive error: {0}")]
    Archive(String),

    #[error("Extraction failed: {0}")]
    Extraction(String),

    #[error("Unsupported platform: {0}")]
    UnsupportedPlatform(String),

    #[error("Anyhow error: {0}")]
    Anyhow(#[from] anyhow::Error),

    #[error("Invalid version: {0}")]
    InvalidVersion(String),
}
