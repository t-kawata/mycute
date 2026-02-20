use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct CreateBdHashRes {
    pub hash: String,
}

#[derive(Serialize, ToSchema)]
pub struct CheckBdHashRes {
    pub ok: bool,
}
