use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use garde::Validate;
use crate::mode::rt::rterr::rterr::*;

// ============================================================
// Config (構成設定)
// ============================================================

/// 構成更新リクエスト（Bifrost PUT /api/config に転送する）
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateLmgwConfigReq {
    /// Bifrost クライアント設定
    #[garde(skip)]
    pub client: Option<LmgwClientConfigReq>,
}

/// クライアント設定（構成更新時に使用）
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct LmgwClientConfigReq {
    /// 過剰リクエストをドロップするか否か
    #[garde(skip)]
    pub drop_excess_requests: Option<bool>,
    /// ロギングを有効にするか否か
    #[garde(skip)]
    pub enable_logging: Option<bool>,
    /// 許可するオリジンのリスト
    #[garde(skip)]
    pub allowed_origins: Option<Vec<String>>,
}

// ============================================================
// ProxyConfig (プロキシ設定)
// ============================================================

/// プロキシ設定更新リクエスト（Bifrost PUT /api/proxy-config に転送する）
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
pub struct UpdateLmgwProxyConfigReq {
    /// プロキシサーバーURL
    #[garde(inner(custom(length_simple_err(1, 255))))]
    pub url: Option<String>,
    /// プロキシ認証ユーザー名
    #[garde(inner(custom(length_simple_err(1, 100))))]
    pub username: Option<String>,
    /// プロキシ認証パスワード
    #[garde(inner(custom(length_simple_err(1, 100))))]
    pub password: Option<String>,
}

// ============================================================
// Providers (プロバイダー管理)
// ============================================================

/// プロバイダー検索リクエスト（実質的にフィルター不要だが、プロジェクト規則に従い Body JSON で受ける）
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
pub struct SearchLmgwProvidersReq {
    #[garde(skip)]
    pub _unused: Option<()>,
}

/// プロバイダー作成リクエスト（Bifrost POST /api/providers に転送する）
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct CreateLmgwProviderReq {
    /// プロバイダー識別名（表示名）
    #[garde(custom(required_simple_err(1, 100)))]
    pub name: String,
    /// 使用するプロバイダー種別（"openai", "anthropic" 等）
    #[garde(custom(required_simple_err(1, 50)))]
    pub provider: String,
    /// プロバイダーの説明（任意）
    #[garde(inner(custom(length_simple_err(0, 255))))]
    pub description: Option<String>,
}

/// プロバイダー更新リクエスト（Bifrost PUT /api/providers/{provider} に転送する）
/// 注意: Bifrost は部分更新ではなく「全フィールド上書き」を要求するため、全フィールドを送る必要がある。
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
#[serde(rename_all = "camelCase")]
pub struct UpdateLmgwProviderReq {
    /// 使用するプロバイダー種別（変更する場合）
    #[garde(custom(required_simple_err(1, 50)))]
    pub provider: String,
    /// プロバイダーの説明（任意）
    #[garde(inner(custom(length_simple_err(0, 255))))]
    pub description: Option<String>,
}

// ============================================================
// Keys (API キー管理)
// ============================================================

/// キー検索リクエスト（実質的にフィルター不要だが、プロジェクト規則に従い Body JSON で受ける）
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
pub struct SearchLmgwKeysReq {
    #[garde(skip)]
    pub _unused: Option<()>,
}

// ============================================================
// Models (モデル情報)
// ============================================================

/// モデル検索リクエスト（Bifrost GET /api/models のクエリパラメータに変換する）
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
pub struct SearchLmgwModelsReq {
    /// モデル名の部分一致フィルター
    #[garde(inner(custom(length_simple_err(0, 100))))]
    pub query: Option<String>,
    /// プロバイダー名フィルター
    #[garde(inner(custom(length_simple_err(0, 50))))]
    pub provider: Option<String>,
    /// 最大返却件数（デフォルト: 5）
    #[garde(inner(custom(range_err(Some(1u32), Some(100u32)))))]
    pub limit: Option<u32>,
}

/// モデルパラメーター検索リクエスト（実質的にフィルター不要だが、プロジェクト規則に従い Body JSON で受ける）
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
pub struct SearchLmgwModelParametersReq {
    #[garde(skip)]
    pub _unused: Option<()>,
}

/// ベースモデル検索リクエスト（Bifrost GET /api/models/base のクエリパラメータに変換する）
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, Validate)]
pub struct SearchLmgwBaseModelsReq {
    /// モデル名フィルター
    #[garde(inner(custom(length_simple_err(0, 100))))]
    pub query: Option<String>,
    /// プロバイダーフィルター
    #[garde(inner(custom(length_simple_err(0, 50))))]
    pub provider: Option<String>,
    /// 最大返却件数
    #[garde(inner(custom(range_err(Some(1u32), Some(100u32)))))]
    pub limit: Option<u32>,
}
