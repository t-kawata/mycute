use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct SearchIdentitiesCaRes {
    pub total: u64,
    pub items: Vec<IdentityItemCaRes>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct IdentityItemCaRes {
    pub id: i32,
    pub apx_id: i32,
    pub vdr_id: i32,
    pub public_key: String,
    pub info: Option<serde_json::Value>,
    pub verified_at: Option<String>,
    pub expire_at: Option<String>,
    pub is_candidate: bool,
    #[schema(example = "L1")]
    pub identity_layer: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize, ToSchema, Clone)]
pub struct GetIdentityCaRes {
    pub id: i32,
    pub apx_id: i32,
    pub vdr_id: i32,
    pub public_key: String,
    pub info: Option<serde_json::Value>,
    pub verified_at: Option<String>,
    pub expire_at: Option<String>,
    pub is_candidate: bool,
    #[schema(example = "L1")]
    pub identity_layer: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Serialize, Deserialize, ToSchema, Clone)]
pub struct EntryIdentityCaRes {
    pub success: bool,
    pub created_at: String,

    /// Initial Budget Tickets for voting (15 balance each by default)
    /// List of JSON strings, each containing { node_pubkey, forum_id, initial_credits, issued_at, signature ... }
    #[schema(example = json!(["ticket_json_1...", "ticket_json_2..."]))]
    pub tickets: Vec<String>,

    /// CA Token (if the node is an appointed APX/VDR)
    #[schema(example = "ca_token_hex...")]
    pub ca_token: Option<String>,

    /// CA Public Key (Hex)
    #[schema(example = "ca_pubkey_hex...")]
    pub ca_pubkey: String,

    /// CA 自身の公式ベースURL。
    #[schema(example = "https://ca.example.com")]
    pub ca_base_url: String,

    /// 削除された（無効になった）フォーラムIDのリスト
    #[schema(example = json!(["uuid1", "uuid2"]))]
    pub deleted_forum_ids: Vec<String>,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct VerifyIdentityCaRes {
    pub id: i32,
    pub public_key: String,
    pub verified_at: String,
    pub expire_at: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct DeleteIdentityCaRes {
    pub id: i32,
    pub deleted: bool,
}

#[derive(Serialize, Deserialize, ToSchema, Clone)]
pub struct SyncIdentityCaRes {
    /// 検証済みのアイデンティティ情報
    pub identity: GetIdentityCaRes,

    /// CA による開発者公開鍵への署名 (L2)
    #[schema(example = "signature_hex...")]
    pub signature: Option<String>,

    /// オーナーによる CA 公開鍵への署名 (L1)
    #[schema(example = "ca_token_hex...")]
    pub ca_token: Option<String>,

    /// CA の公開鍵 (L2署名の検証に必要)
    #[schema(example = "ca_pubkey_hex...")]
    pub ca_pubkey: String,

    /// CA 自身の公式ベースURL。
    #[schema(example = "https://ca.example.com")]
    pub ca_base_url: String,
}
#[derive(Serialize, Deserialize, ToSchema)]
pub struct ApplyIdentityCaRes {
    pub success: bool,
    pub message: String,
}
