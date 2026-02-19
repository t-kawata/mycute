use serde::Deserialize;
use garde::Validate;
use utoipa::ToSchema;
use crate::mode::rt::rterr::rterr::url_err;

#[derive(Deserialize, Validate, ToSchema)]
pub struct EntryIdentityNodeReq {
    /// CA のベースURL (例: "http://canode.example.com")
    #[schema(example = "http://localhost:8080")]
    #[serde(default)]
    #[garde(custom(url_err))]
    pub ca_base_url: String,

    /// プロフィール情報 (JSON)
    #[schema(example = json!({"name": "mycute-user"}))]
    #[serde(default)]
    #[garde(skip)]
    pub info: Option<serde_json::Value>,
}

#[derive(Deserialize, Validate, ToSchema)]
pub struct SyncIdentityNodeReq {
    /// CA のベースURL (例: "http://canode.example.com")
    #[schema(example = "http://localhost:8080")]
    #[serde(default)]
    #[garde(custom(url_err))]
    pub ca_base_url: String,
}

#[derive(Deserialize, Validate, ToSchema)]
pub struct ApplyIdentityNodeReq {
    /// CA のベースURL (例: "http://canode.example.com")
    #[schema(example = "http://localhost:8080")]
    #[serde(default)]
    #[garde(custom(url_err))]
    pub ca_base_url: String,

    /// 連絡先メールアドレス
    #[schema(example = "dev@example.com")]
    #[serde(default)]
    #[garde(custom(crate::mode::rt::rterr::rterr::email_err))]
    pub contact_email: String,

    /// プロフィール情報 (JSON)
    #[schema(example = json!({"name": "Alice"}))]
    #[serde(default)]
    #[garde(skip)]
    pub info: Option<serde_json::Value>,

    /// 検証を希望する有効期間 (秒)
    /// 1時間(3600)〜10年(315360000)の範囲
    #[schema(example = 604800)]
    #[garde(custom(crate::mode::rt::rterr::rterr::range_err(Some(3600u64), Some(315360000u64))))]
    pub expire_seconds: u64,
}
