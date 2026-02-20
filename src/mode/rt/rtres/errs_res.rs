use crate::constants::{ST_INTERNAL_SERVER_ERROR, ST_UNPROCESSABLE_ENTITY};
use axum::http::StatusCode;
use axum::{
    response::{IntoResponse, Response},
    Json,
};
use sea_orm::{DbErr, TransactionError};
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorDetail {
    /// エラー箇所 (例: "email" / "system")
    #[schema(example = "email")]
    pub field: String,

    /// エラーコード (例: "E0001")
    #[schema(example = "E0001")]
    pub code: String,

    /// 人間が読めるメッセージ
    #[schema(example = "Invalid email address.")]
    pub message: String,
}

#[derive(Debug, Serialize, ToSchema)] // OpenAPIドキュメント生成とシリアライズ用
pub struct ApiError {
    /// HTTPステータスコード (例: 500)
    #[schema(example = 500)]
    pub status: u16,

    /// 構造化されたエラー詳細のリスト
    pub errors: Vec<ErrorDetail>,
}

impl ApiError {
    /// 単一のメッセージを持つApiErrorを生成 (field="system")
    pub fn new_system(
        status: StatusCode,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            status: status.as_u16(),
            errors: vec![ErrorDetail {
                field: "system".to_string(),
                code: code.into(),
                message: message.into(),
            }],
        }
    }

    /// 複数の構造化エラーを持つApiErrorを生成
    pub fn new_many(status: StatusCode, errors: Vec<ErrorDetail>) -> Self {
        Self {
            status: status.as_u16(),
            errors,
        }
    }

    /// garde の Report から構造化されたエラー詳細を持つ ApiError を生成
    pub fn from_garde(report: garde::Report) -> Self {
        let mut errors = Vec::new();
        for (path, error) in report.iter() {
            let full_msg = error.to_string();
            // 「|」で分割を試みる
            let (code, message) = if let Some((c, m)) = full_msg.split_once('|') {
                (c.trim().to_string(), m.trim().to_string())
            } else {
                // 「|」が含まれない場合は、デフォルトのバリデーションエラーコードを使用
                (
                    crate::mode::rt::rterr::rterr::ERR_VALIDATION.to_string(),
                    full_msg,
                )
            };
            errors.push(ErrorDetail {
                field: path.to_string(),
                code,
                message,
            });
        }
        Self::new_many(ST_UNPROCESSABLE_ENTITY, errors)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = StatusCode::from_u16(self.status).unwrap_or(ST_INTERNAL_SERVER_ERROR);
        (status, Json(self)).into_response()
    }
}

impl std::fmt::Display for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(first) = self.errors.first() {
            write!(f, "[{}] {}", first.code, first.message)
        } else {
            write!(f, "ApiError(status={})", self.status)
        }
    }
}

impl From<TransactionError<ApiError>> for ApiError {
    fn from(e: TransactionError<ApiError>) -> Self {
        match e {
            TransactionError::Connection(db_err) => ApiError::new_system(
                ST_INTERNAL_SERVER_ERROR,
                crate::mode::rt::rterr::rterr::ERR_DATABASE,
                format!("Database error: {}", db_err),
            ),
            TransactionError::Transaction(api_err) => api_err,
        }
    }
}

impl From<DbErr> for ApiError {
    fn from(e: DbErr) -> Self {
        ApiError::new_system(
            ST_INTERNAL_SERVER_ERROR,
            crate::mode::rt::rterr::rterr::ERR_DATABASE,
            format!("Database error: {}", e),
        )
    }
}
