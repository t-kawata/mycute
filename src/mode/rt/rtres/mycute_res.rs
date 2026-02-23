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
