use crate::{
    mode::rt::rterr::rterr::*,
    mycute_settings::SttEngine,
    types::LocaleCode,
};
use garde::Validate;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct SetLangReq {
    #[garde(skip)]
    #[schema(value_type = String, example = "en")]
    pub locale: LocaleCode,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct SetSttEngineReq {
    #[garde(skip)]
    #[schema(value_type = String, example = "os")]
    pub engine: SttEngine,
}

// SetLlmsReq / LlmEndpointReq は LMGW 移行に伴い廃止済み

// ============================================================
// CA任命証検証リクエスト
// ============================================================
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct VerifyCaTokenReq {
    /// 検証対象の CA任命証
    #[garde(custom(required_simple_err(1, 1000)))]
    pub ca_token: String,
}

// ============================================================
// ライセンス管理リクエスト
// ============================================================

/// POST /mycute/license/register: ライセンスを自身に登録する。
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct RegisterLicenseReq {
    /// 登録するライセンス文字列 (base64(payload).sig_hex)
    #[garde(custom(required_simple_err(1, 10000)))]
    pub license: String,
}

/// POST /mycute/license/unregister: 登録済みライセンスを削除する。
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct UnregisterLicenseReq {
    /// 削除対象のライセンス識別子 (LicenseSummary.id)
    #[garde(custom(required_simple_err(1, 64)))]
    pub id: String,
}

/// POST /mycute/license/verify: ライセンスの妥当性を検証する（登録不要）。
#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct VerifyLicenseReq {
    /// 検証対象のライセンス文字列
    #[garde(custom(required_simple_err(1, 10000)))]
    pub license: String,
}
