use utoipa::{IntoParams, ToSchema};
use serde::Deserialize;
use garde::Validate;
use crate::mode::rt::rterr::rterr::*;

// ============================================================
// Create
// ============================================================
#[derive(Deserialize, IntoParams, Validate, ToSchema)]
pub struct CreateBdHashReq {
    #[serde(default)]
    #[garde(custom(required_simple_err(1, 10000)))]
    pub bd: String,
}

// ============================================================
// Check
// ============================================================
#[derive(Deserialize, IntoParams, Validate, ToSchema)]
pub struct CheckBdHashReq {
    #[serde(default)]
    #[garde(custom(required_simple_err(1, 10000)))]
    pub bd: String,
}
