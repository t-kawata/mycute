use serde::{Serialize, Deserialize};
use utoipa::ToSchema;


// ============================================================
// Advertise Result
// ============================================================
#[derive(Serialize, Deserialize, ToSchema)]
pub struct AdvertiseAppCaRes {
    pub success: bool,
    pub advertised_nodes: u32,
}

// ============================================================
// Discover Result
// ============================================================
#[derive(Serialize, Deserialize, ToSchema, Clone)]
pub struct DiscoverAppItemCaRes {
    pub app_id: String,
    pub name: String,
    pub nodes: Vec<String>, // List of Node URLs
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct DiscoverAppCaRes {
    pub items: Vec<DiscoverAppItemCaRes>,
}

// ============================================================
// Vote / Recalc Result
// ============================================================
#[derive(Serialize, Deserialize, ToSchema)]
pub struct VoteAppCaRes {
    pub success: bool,
    /// CAが計算・記録した累積割り当て量 (証明用)
    pub vote_allocated: i32,
    /// CAの処理時刻 (Unix Milliseconds)
    pub timestamp: i64,
    /// CAによる署名 (Hex)
    pub signature: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct VoteProvisionalCaRes {
    pub success: bool,
    pub message: String,
}

// ============================================================
// CA Status Result
// ============================================================
#[derive(Serialize, Deserialize, ToSchema)]
pub struct CaStatusCaRes {
    pub status: String,
    pub policies: String,
}
