use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct OwnerStatusRes {
    pub is_active: bool,
}
