use garde::Validate;
use serde::Deserialize;
use utoipa::ToSchema;

#[derive(Deserialize, Validate, ToSchema)]
pub struct RegisterCaTokenReq {
    #[garde(length(min = 1))]
    #[schema(example = "sig_hex.expire_ts...")]
    pub token: String,
}
