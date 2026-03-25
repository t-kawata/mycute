use crate::mode::rt::rterr::rterr::*;
use garde::Validate;
use serde::Deserialize;
use utoipa::ToSchema;

#[derive(Deserialize, Validate, ToSchema)]
pub struct RegisterCaTokenReq {
    #[garde(custom(required_simple_err(1, 2048)))]
    #[schema(example = "sig_hex.expire_ts...")]
    pub ca_token: String,
}
