use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct OwnerStatusRes {
    pub is_active: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AssignCaRes {
    /// 発行された CA トークン (signature.expire_ts)
    pub ca_token: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct GenCaTokenRes {
    /// 発行された CA トークン (signature.expire_ts)
    pub ca_token: String,
}
