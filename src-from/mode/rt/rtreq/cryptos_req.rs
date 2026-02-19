use serde::Deserialize;
use garde::Validate;
use utoipa::{IntoParams, ToSchema};
use crate::mode::rt::rterr::rterr::*;

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
