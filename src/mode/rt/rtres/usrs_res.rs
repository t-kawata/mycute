use crate::entities::usrs;
use crate::utils::db::datetime_to_str;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct AuthUsrRes {
    pub token: String,
}

// ============================================================
// Search
// ============================================================
#[derive(Serialize, ToSchema)]
pub struct SearchUsrsRes {
    pub usrs: Vec<SearchUsrsResItem>,
}

#[derive(Serialize, ToSchema)]
pub struct SearchUsrsResItem {
    pub id: u32,
    pub apx_id: u32,
    pub vdr_id: u32,
    pub name: String,
    pub email: String,
    pub bgn_at: String,
    pub end_at: String,
    pub r#type: u8,
}

impl From<usrs::Model> for SearchUsrsResItem {
    fn from(m: usrs::Model) -> Self {
        Self {
            id: m.id as u32,
            apx_id: m.apx_id.unwrap_or(0) as u32,
            vdr_id: m.vdr_id.unwrap_or(0) as u32,
            name: m.name,
            email: m.email,
            bgn_at: datetime_to_str(m.bgn_at),
            end_at: datetime_to_str(m.end_at),
            r#type: m.r#type as u8,
        }
    }
}

// ============================================================
// Get
// ============================================================
#[derive(Serialize, ToSchema)]
pub struct GetUsrRes {
    pub id: u32,
    pub apx_id: u32,
    pub vdr_id: u32,
    pub name: String,
    pub email: String,
    pub bgn_at: String,
    pub end_at: String,
    pub r#type: u8,
}

impl From<usrs::Model> for GetUsrRes {
    fn from(m: usrs::Model) -> Self {
        Self {
            id: m.id as u32,
            apx_id: m.apx_id.unwrap_or(0) as u32,
            vdr_id: m.vdr_id.unwrap_or(0) as u32,
            name: m.name,
            email: m.email,
            bgn_at: datetime_to_str(m.bgn_at),
            end_at: datetime_to_str(m.end_at),
            r#type: m.r#type as u8,
        }
    }
}

// ============================================================
// Create
// ============================================================
#[derive(Serialize, ToSchema)]
pub struct CreateUsrRes {
    pub id: u32,
}

// ============================================================
// Update
// ============================================================
#[derive(Serialize, ToSchema)]
pub struct UpdateUsrRes {
    pub id: u32,
}

// ============================================================
// Delete
// ============================================================
#[derive(Serialize, ToSchema)]
pub struct DeleteUsrRes {
    pub id: u32,
}

// ============================================================
// Hire
// ============================================================
#[derive(Serialize, ToSchema)]
pub struct HireUsrRes {
    pub id: u32,
}

// ============================================================
// Dehire
// ============================================================
#[derive(Serialize, ToSchema)]
pub struct DehireUsrRes {
    pub id: u32,
}
