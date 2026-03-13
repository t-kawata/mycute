use crate::stt_config::LlmEndpoint;
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

