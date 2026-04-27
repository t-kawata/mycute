use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ManageLmgwProviderRes {
    pub provider_name: String,
    pub config_json: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct GetLmgwProvidersRes {
    pub providers: Vec<ManageLmgwProviderRes>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SaveLmgwProvidersRes {
    pub success: bool,
}
