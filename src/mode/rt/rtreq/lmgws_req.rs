use garde::Validate;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct ManageLmgwProviderReq {
    #[garde(skip)]
    pub provider_name: String,
    #[garde(skip)]
    pub config_json: String,
}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct SaveLmgwProvidersReq {
    #[garde(dive)]
    pub providers: Vec<ManageLmgwProviderReq>,
}
