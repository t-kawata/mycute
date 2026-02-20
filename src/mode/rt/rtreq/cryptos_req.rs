use crate::mode::rt::rterr::rterr::*;
use garde::Validate;
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};

#[derive(Deserialize, IntoParams, Validate, ToSchema)]
pub struct EncryptReq {
    #[serde(default)]
    #[garde(custom(required_simple_err(1, 10000)))]
    pub text: String,
}

#[derive(Deserialize, IntoParams, Validate, ToSchema)]
pub struct DecryptReq {
    #[serde(default)]
    #[garde(custom(required_simple_err(1, 10000)))]
    pub text: String,
}
