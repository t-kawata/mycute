use crate::{
    mode::rt::rterr::rterr::*,
    mycute_settings::{LlmEndpoint, SttEngine},
    types::LocaleCode,
};
use garde::Validate;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct SetLangReq {
    #[garde(skip)]
    #[schema(value_type = String, example = "en")]
    pub locale: LocaleCode,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct SetSttEngineReq {
    #[garde(skip)]
    #[schema(value_type = String, example = "os")]
    pub engine: SttEngine,
}

// ============================================================
// LLM設定変更リクエスト
// ============================================================

/// LLM設定の更新リクエスト。llms を空配列にすることで全件削除も可能。
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct SetLlmsReq {
    /// LlmEndpointReq の配列。0件（空配列）は合法（全LLMクリア）。
    #[garde(dive)]
    pub llms: Vec<LlmEndpointReq>,
}

/// 1件の LLM エンドポイント設定。すべてのフィールドに custom バリデーションを適用。
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct LlmEndpointReq {
    #[serde(default)]
    #[garde(custom(required_simple_err(1, 100)))]
    pub name: String,
    #[serde(default)]
    #[garde(custom(url_err))]
    pub base_url: String,
    /// APIキーは任意項目（`None` または省略可）
    #[garde(skip)]
    pub api_key: Option<String>,
    #[serde(default)]
    #[garde(custom(required_simple_err(1, 200)))]
    pub model: String,
}

impl From<LlmEndpointReq> for LlmEndpoint {
    fn from(req: LlmEndpointReq) -> Self {
        LlmEndpoint {
            name: req.name,
            base_url: req.base_url,
            api_key: req.api_key,
            model: req.model,
        }
    }
}


// ============================================================
// CAトークン検証リクエスト
// ============================================================
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct VerifyCaTokenReq {
    /// 検証対象の CA トークン
    #[garde(custom(required_simple_err(1, 1000)))]
    pub ca_token: String,
}
