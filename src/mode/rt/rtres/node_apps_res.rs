use serde::Serialize;
use utoipa::ToSchema;
use crate::utils::pkg_bl::AppTrustInfo;

// ============================================================
// App Info Item (Node Version)
// ============================================================
#[derive(serde::Serialize, serde::Deserialize, ToSchema, Clone)]
pub struct AppInfoNodeItemRes {
    pub installed_at: String,
    // 信用情報 (共通部品)
    pub trust: AppTrustInfo,
}

// ============================================================
// App Info (Node Version)
// ============================================================
#[derive(Serialize, ToSchema)]
pub struct AppInfoNodeRes {
    pub info: AppInfoNodeItemRes,
}

// ============================================================
// Verify Result (Node Version)
// ============================================================
#[derive(Serialize, ToSchema)]
pub struct VerifyAppNodeRes {
    pub trust: AppTrustInfo,
}


// ============================================================
// Advertise Result (Node Version)
// ============================================================
#[derive(Serialize, ToSchema)]
pub struct AdvertiseAppNodeRes {
    pub success: bool,
    pub advertised_nodes: u32,
}

// ============================================================
// Discover Result (Node Version)
// ============================================================
#[derive(Serialize, ToSchema)]
pub struct DiscoverAppNodeRes {
    pub items: Vec<crate::mode::rt::rtres::ca_apps_res::DiscoverAppItemCaRes>,
}

// ============================================================
// Vote Result (Node Version)
// ============================================================
#[derive(Serialize, ToSchema)]
pub struct VoteAppNodeRes {
    pub success: bool,
}
