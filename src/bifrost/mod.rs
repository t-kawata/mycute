pub mod assets;
pub mod error;
pub mod executor;
pub mod installer;

pub use executor::BifrostManager;
pub use installer::install;
pub use assets::BIFROST_VERSION;

/// Bifrost ランタイムがインストールされるサブディレクトリ名
pub const BIFROST_DIRNAME: &str = "bifrost";
