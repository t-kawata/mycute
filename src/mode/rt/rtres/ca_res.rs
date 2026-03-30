use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct RegisterCaTokenRes {
    #[schema(example = true)]
    pub success: bool,
    #[schema(example = "CA Cert registered successfully.")]
    pub message: String,
}
