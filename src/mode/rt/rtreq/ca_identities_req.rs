use crate::constants::{ED448_PUBKEY_HEX_LEN, ED448_SIGNATURE_HEX_LEN};
use crate::mode::rt::rterr::rterr::*;
use garde::Validate;
use serde::Deserialize;
use utoipa::ToSchema;

#[derive(Deserialize, Validate, ToSchema)]
pub struct SearchIdentitiesCaReq {
    #[schema(example = "a1b2c3d4...")]
    #[serde(default)]
    #[garde(skip)]
    pub public_key: Option<String>,

    /// 未所属 (Isolated) のレコードを含めるかどうか
    #[schema(example = false, default = false)]
    #[serde(default)]
    #[garde(skip)]
    pub include_isolated: bool,

    #[schema(example = 25, default = 25)]
    #[serde(default = "default_limit")]
    #[garde(custom(range_err(Some(1u32), Some(25u32))))]
    pub limit: u32,

    #[schema(example = 0, default = 0)]
    #[serde(default)]
    #[garde(custom(range_err(Some(0u32), None)))]
    pub offset: u32,
}

fn default_limit() -> u32 {
    25
}

#[derive(serde::Serialize, Deserialize, ToSchema)]
pub struct ExistingForumReq {
    #[schema(example = "uuid-string")]
    pub id: String,
    #[schema(example = "2026-02-13 12:00:00")]
    pub updated_at: String,
}

#[derive(serde::Serialize, Deserialize, Validate, ToSchema)]
pub struct EntryIdentityCaReq {
    /// Ed448 公開鍵 (Hex エンコード 114 文字)
    #[schema(example = "a1b2c3d4e5f6...114chars...")]
    #[serde(default)]
    #[garde(custom(required_simple_err(ED448_PUBKEY_HEX_LEN, ED448_PUBKEY_HEX_LEN)))]
    pub public_key: String,

    /// プロフィール情報 (JSON)
    #[schema(example = json!({"name": "mycute-user"}))]
    #[serde(default)]
    #[garde(skip)]
    pub info: Option<serde_json::Value>,

    /// Node による自身の公開鍵への署名 (PoP - Proof of Possession)
    #[schema(example = "sig_hex...")]
    #[serde(default)]
    #[garde(custom(required_simple_err(ED448_SIGNATURE_HEX_LEN, ED448_SIGNATURE_HEX_LEN)))]
    pub signature: String,

    /// Delta Entry 用: すでにチケットを保持しているForumの情報一覧
    /// これらと比較して更新がないフォーラムのチケットは返却されない。
    #[schema(example = json!([{"id": "uuid1", "updated_at": "2026-02-13 10:00:00"}]))]
    #[serde(default)]
    #[garde(skip)]
    pub existing_forums: Option<Vec<ExistingForumReq>>,
}

#[derive(Deserialize, Validate, ToSchema)]
pub struct VerifyIdentityCaReq {
    /// CA による署名 (対象 Identity の public_key に対する署名)
    #[schema(example = "signature_hex...")]
    #[serde(default)]
    #[garde(custom(required_simple_err(ED448_SIGNATURE_HEX_LEN, ED448_SIGNATURE_HEX_LEN)))]
    pub signature: String,
}

#[derive(serde::Serialize, Deserialize, Validate, ToSchema)]
pub struct ApplyIdentityCaReq {
    /// 検証を希望する Ed448 公開鍵 (Hex 114 chars)
    #[schema(example = "a1b2c3d4...")]
    #[serde(default)]
    #[garde(custom(required_simple_err(ED448_PUBKEY_HEX_LEN, ED448_PUBKEY_HEX_LEN)))]
    pub public_key: String,

    /// 連絡先メールアドレス
    #[schema(example = "dev@example.com")]
    #[serde(default)]
    #[garde(custom(email_err))]
    pub contact_email: String,

    /// プロフィール情報 (JSON)
    #[schema(example = json!({"name": "Alice"}))]
    #[serde(default)]
    #[garde(skip)]
    pub info: Option<serde_json::Value>,

    /// 検証を希望する有効期間 (秒)
    /// 1時間(3600)〜10年(315360000)の範囲
    #[schema(example = 604800)]
    #[garde(custom(range_err(Some(3600u64), Some(315360000u64))))]
    pub expire_seconds: u64,
}
