use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct RegisterCaTokenRes {
    #[schema(example = true)]
    pub success: bool,
    #[schema(example = "CA Cert registered successfully.")]
    pub message: String,
    pub ca_token: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct UnregisterCaTokenRes {
    #[schema(example = true)]
    pub success: bool,
    #[schema(example = "CA Cert unregistered successfully.")]
    pub message: String,
}

#[derive(Serialize, ToSchema)]
pub struct CaStatusRes {
    pub ca_token: Option<String>,
}
