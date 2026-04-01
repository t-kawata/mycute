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

/// CA がユーザーにライセンスを発行するためのリクエスト。
#[derive(Deserialize, Validate, ToSchema)]
pub struct GenLicenseReq {
    /// ライセンス付与対象ユーザーの Ed448 公開鍵 (Hex)
    #[garde(custom(required_simple_err(114, 114)))]
    #[schema(example = "abcd1234...")]
    pub pubkey_hex: String,

    /// ライセンスの有効期限（時間）
    #[garde(custom(range_err(Some(1u32), Some(87600u32))))]
    #[schema(example = 720)]
    pub expire_hours: u32,

    /// 権限内容 (JSON オブジェクト)
    /// 省略した場合はデフォルト {"all": true} が使用される。
    #[garde(skip)]
    #[schema(example = json!({"all": true}))]
    pub permissions: Option<serde_json::Value>,
}
