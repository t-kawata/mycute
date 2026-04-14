pub mod assets;
pub mod error;
pub mod executor;
pub mod installer;

pub use executor::ZeroClawManager;
pub use installer::install;
pub use assets::ZEROCLAW_VERSION;

/// ZeroClaw ランタイムがインストールされるサブディレクトリ名
pub const ZEROCLAW_DIRNAME: &str = "zeroclaw";
