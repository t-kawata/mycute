use crate::mycute_settings::LlmEndpoint;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MyCuteVersionRes {
    #[schema(example = "v0.1.0")]
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MyCuteHomeDirRes {
    #[schema(example = "/Users/username/.mycute")]
    pub home_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SetLangRes {
    #[schema(example = "Language updated successfully")]
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SetSttEngineRes {
    #[schema(example = "STT engine updated successfully")]
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SetLlmsRes {
    #[schema(example = "LLM settings updated successfully")]
    pub message: String,
}

/// GET /mycute/llms のレスポンス型。バックエンドの LLM 設定一覧を返す。
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct GetMycuteLlmsRes {
    pub llms: Vec<LlmEndpoint>,
}


/// GET /mycute/catoken/verify のレスポンス型。検証結果を返す。
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VerifyCaTokenRes {
    pub success: bool,
    #[schema(example = "CA Token is valid")]
    pub message: String,
    /// 署名が正当な場合に、トークン内に含まれている CA 公開鍵を返す
    pub ca_pubkey: Option<String>,
    /// トークンの有効期限（Unix TS）
    pub expire_at: Option<u64>,
}
