//! Cuber Error Types
//!
//! Cuber モジュールで使用される構造化されたエラー定義です。
//! Go 版の `errors.New` や `fmt.Errorf` に相当するエラーを、
//! `thiserror` を用いて型安全に表現しています。

use thiserror::Error;

/// Cuber モジュールにおけるエラー型
#[derive(Error, Debug)]
pub enum CuberError {
    /// ストレージ（LadybugDB）の初期化に失敗
    #[error("Storage initialization failed: {0}")]
    StorageInitError(String),

    /// ストレージへの接続・クエリ実行に失敗
    #[error("Storage query error: {0}")]
    StorageQueryError(String),

    /// S3 クライアント操作に失敗
    #[error("S3 client error: {0}")]
    S3Error(String),

    /// 設定値の検証に失敗
    #[error("Configuration validation failed: {0}")]
    ConfigValidationError(String),

    /// Tokenizer（形態素解析器）の初期化に失敗
    #[error("Tokenizer initialization failed: {0}")]
    TokenizerInitError(String),

    /// I/O エラー
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// LLM（大規模言語モデル）関連のエラー
    #[error("Model error: {0}")]
    ModelError(String),

    /// リソースが見つからない
    #[error("Resource not found: {0}")]
    NotFound(String),

    /// 内部エラー（予期しないエラー）
    #[error("Internal error: {0}")]
    InternalError(String),
    // TODO: 将来実装予定の LadybugDB 固有のエラー型
    // TODO: 将来実装予定の LLM API 関連のエラー型
}

/// `lbug::Error` から `CuberError` への変換
impl From<lbug::Error> for CuberError {
    fn from(err: lbug::Error) -> Self {
        CuberError::StorageQueryError(format!("{:?}", err))
    }
}
