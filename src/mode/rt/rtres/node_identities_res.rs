use serde::{Serialize, Deserialize};
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct EntryIdentityNodeRes {
    pub success: bool,
    pub created_at: String,
}

#[derive(Serialize, ToSchema)]
pub struct SyncIdentityNodeRes {
    /// 検証済みのアイデンティティ情報
    pub identity: GetIdentityNodeRes,

    /// CA による開発者公開鍵への署名 (L2)
    #[schema(example = "signature_hex...")]
    pub signature: Option<String>,

    /// オーナーによる CA 公開鍵への署名 (L1)
    #[schema(example = "ca_token_hex...")]
    pub ca_token: Option<String>,

    /// CA の公開鍵 (L2署名の検証に必要)
    #[schema(example = "ca_pubkey_hex...")]
    pub ca_pubkey: String,
}

#[derive(Serialize, ToSchema)]
pub struct GetIdentityNodeRes {
    pub id: i32,
    pub apx_id: i32,
    pub vdr_id: i32,
    pub public_key: String,
    pub info: Option<serde_json::Value>,
    pub verified_at: Option<String>,
    pub expire_at: Option<String>,
    pub is_candidate: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, ToSchema)]
pub struct GetPubKeyNodeRes {
    pub public_key: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ApplyIdentityNodeRes {
    pub success: bool,
    pub message: String,
}
