pub mod assets;
pub mod error;
pub mod executor;
pub mod installer;

pub use executor::NodeManager;
pub use installer::install;
pub use assets::NODE_VERSION;

/// Node.js ランタイムがインストールされるサブディレクトリ名
pub const NODE_JS_DIRNAME: &str = "nodejs";
/// UNIX系 OS における実行ファイルの配置ディレクトリ名
pub const NODE_BIN_DIRNAME: &str = "bin";
