use serde::{Deserialize, Serialize};
use garde::Validate;
use utoipa::ToSchema;
use crate::mode::rt::rterr::rterr::*;

// ============================================================
// Discover App (Search)
// ============================================================
#[derive(Deserialize, Serialize, Validate, ToSchema, Clone)]
pub struct DiscoverAppCaReq {
    /// アプリIDリスト (UUID)
    #[schema(example = r#"["uuid1", "uuid2"]"#)]
    #[garde(skip)]
    pub app_ids: Option<Vec<String>>,

    /// 名前による曖昧検索クエリ
    #[schema(example = "my-app")]
    #[garde(skip)]
    pub query: Option<String>,
}

// ============================================================
// Advertise App (Create/Action)
// ============================================================
#[derive(Deserialize, Serialize, Validate, ToSchema, Clone)]
pub struct AdvertiseAppCaReq {
    /// アプリID (UUID)
    #[schema(example = "uuid...")]
    #[garde(custom(required_simple_err(36, 36)))]
    pub app_id: String,
}

// ============================================================
// Vote App (Update/Create)
// ============================================================
#[derive(Deserialize, Serialize, Validate, ToSchema, Clone)]
pub struct VoteAppCaReq {
    /// Node Public Key (Hex)
    #[schema(example = "pubkey_hex...")]
    #[garde(custom(required_simple_err(114, 114)))]
    pub node_pubkey: String,

    /// Global App UUID
    #[schema(example = "uuid...")]
    #[garde(custom(required_simple_err(36, 36)))]
    pub app_id: String,

    /// Forum UUID
    #[schema(example = "uuid...")]
    #[garde(custom(required_simple_err(36, 36)))]
    pub forum_id: String,
    
    /// Vote Value (0 to 15)
    /// 0 = Cancel Vote
    #[schema(example = 15)]
    #[garde(custom(range_err(Some(0i32), Some(15i32))))]
    pub vote: i32,

    /// Timestamp (Unix Milliseconds)
    #[schema(example = 1709251200000i64)]
    #[garde(skip)] // ロジック側で検証
    pub timestamp: i64,

    /// Vote Allocated (Cumulative total of votes cast by this node in this forum)
    /// このノードがこのフォーラムで消費した投票コストの累計（今回の投票を含む）。
    /// 不正検知（BudgetFraud）のための重要な検証値。
    #[schema(example = 15)]
    #[garde(skip)] // ロジック側で検証
    pub vote_allocated: i32,

    /// Ticket (Proof of Budget)
    /// 投票権を示す単一のチケット。
    #[schema(example = r#"{"node_pubkey": "...", ...}"#)]
    #[garde(skip)]
    pub ticket: serde_json::Value,

    /// Signature of the request payload
    #[schema(example = "sig_hex...")]
    #[garde(custom(required_simple_err(228, 228)))]
    pub signature: String,
}
