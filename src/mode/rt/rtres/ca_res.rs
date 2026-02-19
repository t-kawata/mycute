use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct RegisterCaTokenRes {
    #[schema(example = true)]
    pub success: bool,
    #[schema(example = "CA Token registered successfully.")]
    pub message: String,
}
