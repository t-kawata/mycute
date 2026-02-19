use serde::Serialize;
use utoipa::ToSchema;

// ============================================================
// App Info Item (Public Version)
// ============================================================
#[derive(Serialize, ToSchema, Clone)]
pub struct AppInfoPubItemRes {
    pub global_app_id: String,
    pub global_app_version: String,
    pub name: String,
    pub author: String,
    pub description: Option<String>,
    pub dev_public_key: Option<String>,
    /// [証拠]: 開発者が提供した生の検証情報のリスト
    pub verifications: Option<serde_json::Value>,
    /// [結果キャッシュ]: ノードが検証した結果の詳細リスト
    pub verification_results_cache: Option<serde_json::Value>,
    pub manifest_data: Option<serde_json::Value>,
    pub created_at: String,
    pub updated_at: String,
}

// ============================================================
// App List (Public Version)
// ============================================================
#[derive(Serialize, ToSchema)]
pub struct ListAppsPubRes {
    pub items: Vec<AppInfoPubItemRes>,
}
