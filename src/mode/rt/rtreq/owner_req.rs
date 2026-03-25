use crate::mode::rt::rterr::rterr::*;
use garde::Validate;
use serde::Deserialize;
use utoipa::ToSchema;

#[derive(Deserialize, Validate, ToSchema)]
pub struct AssignCaReq {
    #[garde(custom(url_err))]
    #[garde(custom(required_simple_err(1, 2048)))]
    pub target_url: String, // e.g. "http://192.168.1.10:8080"
    
    #[garde(custom(range_err(Some(1u32), Some(87600u32))))]
    pub expire_hours: u32,
}

#[derive(Debug, Deserialize, Validate, ToSchema)]
pub struct ActivateOwnerReq {
    /// オーナーパスフレーズ
    #[garde(custom(required_simple_err(1, 255)))]
    pub passphrase: String,
}

#[derive(Deserialize, Validate, ToSchema)]
pub struct GenCaTokenReq {
    /// ターゲットノードの公開鍵 (Hex)
    #[garde(custom(required_simple_err(114, 114)))]
    pub pubkey_hex: String,

    /// 有効期限 (時間)
    #[garde(custom(range_err(Some(1u32), Some(87600u32))))]
    pub expire_hours: u32,
}
