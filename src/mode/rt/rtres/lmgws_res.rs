use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use serde_json::Value;

// ============================================================
// Config (構成設定)
// ============================================================

/// Bifrost クライアント設定
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LmgwClientConfigRes {
    /// 過剰リクエストをドロップするか否か
    pub drop_excess_requests: Option<bool>,
    /// ロギングを有効にするか否か
    pub enable_logging: Option<bool>,
    /// 許可するオリジンのリスト
    pub allowed_origins: Option<Vec<String>>,
}

/// 構成取得レスポンス（Bifrost GET /api/config のレスポンスをラップする）
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GetLmgwConfigRes {
    /// クライアント設定
    pub client: Option<LmgwClientConfigRes>,
    /// ConfigStore が有効かどうか
    pub config_store_enabled: Option<bool>,
}

/// 構成更新レスポンス
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateLmgwConfigRes {
    /// 操作結果メッセージ
    pub message: String,
}

// ============================================================
// ProxyConfig (プロキシ設定)
// ============================================================

/// プロキシ設定レスポンス（Bifrost GET /api/proxy-config のレスポンスをラップする）
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LmgwProxyConfigRes {
    /// プロキシサーバーURL
    pub url: Option<String>,
    /// プロキシ認証ユーザー名
    pub username: Option<String>,
}

/// プロキシ設定更新レスポンス
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateLmgwProxyConfigRes {
    /// 操作結果メッセージ
    pub message: String,
}

// ============================================================
// Providers (プロバイダー管理)
// ============================================================

/// プロバイダー個別情報
/// Bifrost の ListProvidersResponse / ProviderResponse の共通フィールドに対応する。
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LmgwProviderRes {
    /// プロバイダー識別名（表示名）
    pub name: String,
    /// プロバイダー種別（"openai", "anthropic" 等）
    pub provider: String,
    /// プロバイダーの説明
    pub description: Option<String>,
    /// 現在のステータス（"active", "inactive" 等）
    pub status: Option<String>,
}

/// プロバイダー検索レスポンス
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SearchLmgwProvidersRes {
    /// プロバイダー一覧
    pub providers: Vec<LmgwProviderRes>,
    /// 総件数
    pub total: usize,
}

/// プロバイダー取得レスポンス（Get）
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GetLmgwProviderRes {
    /// プロバイダー情報
    pub provider: LmgwProviderRes,
}

/// プロバイダー作成レスポンス
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateLmgwProviderRes {
    /// 作成されたプロバイダー情報
    pub provider: LmgwProviderRes,
}

/// プロバイダー更新レスポンス
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UpdateLmgwProviderRes {
    /// 更新後のプロバイダー情報
    pub provider: LmgwProviderRes,
}

/// プロバイダー削除レスポンス
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DeleteLmgwProviderRes {
    /// 操作結果メッセージ
    pub message: String,
}

// ============================================================
// Keys (API キー管理)
// ============================================================

/// API キー情報
/// Bifrost GET /api/keys のレスポンス要素に対応する。
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LmgwKeyRes {
    /// キーID（Bifrost内部ID）
    pub id: String,
    /// 所属プロバイダー名
    pub provider: Option<String>,
    /// キーの別名（ニックネーム）
    pub nickname: Option<String>,
    /// キーが有効かどうか
    pub active: bool,
    /// キー文字列（マスクされた状態で返ることがある）
    pub value: Option<String>,
}

/// キー検索レスポンス
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SearchLmgwKeysRes {
    /// APIキー一覧
    pub keys: Vec<LmgwKeyRes>,
    /// 総件数
    pub total: usize,
}

// ============================================================
// Models (モデル情報)
// ============================================================

/// モデル個別情報
/// Bifrost GET /api/models のレスポンス要素に対応する。
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LmgwModelRes {
    /// モデルID
    pub id: String,
    /// 所属プロバイダー名
    pub provider: Option<String>,
    /// モデル所有者
    pub owned_by: Option<String>,
}

/// モデル検索レスポンス
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SearchLmgwModelsRes {
    /// モデル一覧
    pub models: Vec<LmgwModelRes>,
    /// 総件数
    pub total: usize,
}

/// モデルパラメーター検索レスポンス
/// Bifrost は additionalProperties 形式（動的なキーと値）を返すため、
/// serde_json::Value で柔軟に受け取る。
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SearchLmgwModelParametersRes {
    /// パラメーター定義マップ（キー: パラメーター名, 値: 定義オブジェクト）
    pub parameters: Value,
}

/// ベースモデル検索レスポンス
/// Bifrost は additionalProperties 形式を返すため、serde_json::Value で受け取る。
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SearchLmgwBaseModelsRes {
    /// ベースモデル定義マップ
    pub models: Value,
}
