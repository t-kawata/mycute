use serde::{Deserialize, Serialize};
use garde::Validate;
use utoipa::ToSchema;
use crate::mode::rt::rterr::rterr::*;

// ============================================================
// Discover App (Search)
// ============================================================
#[derive(Deserialize, Serialize, Validate, ToSchema)]
pub struct DiscoverAppNodeReq {
    /// 宛先 CA のベース URL
    #[schema(example = "http://ca.example.com")]
    #[garde(custom(url_err))]
    pub ca_base_url: String,

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
// Vote App (Update/Create)
// ============================================================
#[derive(Deserialize, Serialize, Validate, ToSchema)]
pub struct VoteAppNodeReq {
    /// 宛先 CA のベース URL
    #[schema(example = "http://ca.example.com")]
    #[garde(custom(url_err))]
    pub ca_base_url: String,

    /// Global App UUID
    #[schema(example = "uuid...")]
    #[garde(custom(required_simple_err(36, 36)))]
    pub app_id: String,

    /// Forum UUID
    #[schema(example = "uuid...")]
    #[garde(custom(required_simple_err(36, 36)))]
    pub forum_id: String,
    
    /// Vote Value (0 to 15)
    #[schema(example = 15)]
    #[garde(custom(range_err(Some(0i32), Some(15i32))))]
    pub vote: i32,
}

// ============================================================
// Build App (Action)
// ============================================================
#[allow(dead_code)]
#[derive(ToSchema)]
pub struct BuildAppNodeReq {
    #[schema(format = "binary")]
    pub zip: Vec<u8>,
}

// ============================================================
// Install App (Action)
// ============================================================
#[allow(dead_code)]
#[derive(ToSchema)]
pub struct InstallAppFileNodeReq {
    #[schema(format = "binary")]
    pub mycute: Vec<u8>,
}

// ============================================================
// Advertise App (Action)
// ============================================================
#[derive(Deserialize, Serialize, Validate, ToSchema)]
pub struct AdvertiseAppNodeReq {
    /// 宛先 CA のベース URL
    #[schema(example = "http://ca.example.com")]
    #[garde(custom(url_err))]
    pub ca_base_url: String,

    /// アプリID (UUID)
    #[schema(example = "uuid...")]
    #[garde(custom(required_simple_err(36, 36)))]
    pub app_id: String,
}

// ============================================================
// Verify App (Action)
// ============================================================
#[allow(dead_code)]
#[derive(ToSchema)]
pub struct VerifyAppNodeReq {
    #[schema(format = "binary")]
    pub mycute: Vec<u8>,
}
