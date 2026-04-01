use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct RegisterCaTokenRes {
    #[schema(example = true)]
    pub success: bool,
    #[schema(example = "CA Cert registered successfully.")]
    pub message: String,
    pub ca_token: Option<String>,
    /// 登録された CA の権限内容 (JSON)
    pub permissions: Option<serde_json::Value>,
}

#[derive(Serialize, ToSchema)]
pub struct UnregisterCaTokenRes {
    #[schema(example = true)]
    pub success: bool,
    #[schema(example = "CA Cert unregistered successfully.")]
    pub message: String,
}

#[derive(Serialize, ToSchema)]
pub struct CaStatusRes {
    pub ca_token: Option<String>,
}

/// CA によるライセンス発行レスポンス。
#[derive(Serialize, ToSchema)]
pub struct GenLicenseRes {
    /// 発行されたライセンス文字列 (base64(payload).sig_hex)
    pub license: String,
}
