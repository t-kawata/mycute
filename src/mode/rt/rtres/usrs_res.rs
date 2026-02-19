use utoipa::ToSchema;
use serde::Serialize;
use rust_decimal::prelude::ToPrimitive;
use crate::entities::usrs;
use crate::utils::db::datetime_to_str;

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
    pub base_point: u32,
    pub belong_rate: f64,
    pub max_works: u32,
    pub flush_days: u32,
    pub rate: f64,
    pub flush_fee_rate: f64,
}

impl From<usrs::Model> for SearchUsrsResItem {
    fn from(m: usrs::Model) -> Self {
        Self {
            id: m.id as u32,
            apx_id: m.apx_id.unwrap_or(0),
            vdr_id: m.vdr_id.unwrap_or(0),
            name: m.name,
            email: m.email,
            bgn_at: datetime_to_str(m.bgn_at),
            end_at: datetime_to_str(m.end_at),
            r#type: m.r#type,
            base_point: m.base_point,
            belong_rate: m.belong_rate.to_f64().unwrap_or(0.0),
            max_works: m.max_works,
            flush_days: m.flush_days,
            rate: m.rate.to_f64().unwrap_or(0.0),
            flush_fee_rate: m.flush_fee_rate.to_f64().unwrap_or(0.0),
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
    pub base_point: u32,
    pub belong_rate: f64,
    pub max_works: u32,
    pub flush_days: u32,
    pub rate: f64,
    pub flush_fee_rate: f64,
}

impl From<usrs::Model> for GetUsrRes {
    fn from(m: usrs::Model) -> Self {
        Self {
            id: m.id as u32,
            apx_id: m.apx_id.unwrap_or(0),
            vdr_id: m.vdr_id.unwrap_or(0),
            name: m.name,
            email: m.email,
            bgn_at: datetime_to_str(m.bgn_at),
            end_at: datetime_to_str(m.end_at),
            r#type: m.r#type,
            base_point: m.base_point,
            belong_rate: m.belong_rate.to_f64().unwrap_or(0.0),
            max_works: m.max_works,
            flush_days: m.flush_days,
            rate: m.rate.to_f64().unwrap_or(0.0),
            flush_fee_rate: m.flush_fee_rate.to_f64().unwrap_or(0.0),
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
