use utoipa::ToSchema;
use serde::Serialize;

#[derive(Serialize, ToSchema)]
pub struct CreateBdHashRes {
    pub hash: String,
}

#[derive(Serialize, ToSchema)]
pub struct CheckBdHashRes {
    pub ok: bool,
}