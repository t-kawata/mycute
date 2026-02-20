use crate::mode::rt::rterr::rterr::*;
use garde::Validate;
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};

// ============================================================
// Auth
// ============================================================
#[derive(Deserialize, IntoParams, Validate, ToSchema)]
pub struct AuthUsrReq {
    #[schema(example = "user@example.com")]
    #[serde(default)]
    #[garde(custom(required_simple_err(1, 100)))]
    pub email: String,

    #[schema(example = "password123")]
    #[serde(default)]
    #[garde(custom(required_simple_err(1, 100)))]
    pub password: String,

    #[serde(default = "default_expire")]
    #[schema(default = 24)]
    #[garde(skip)]
    pub expire: Option<u32>,
}

fn default_expire() -> Option<u32> {
    Some(24)
}

// ============================================================
// Search
// ============================================================
#[derive(Deserialize, IntoParams, Validate, ToSchema)]
pub struct SearchUsrsReq {
    #[schema(example = "山田 太郎")]
    #[serde(default)]
    #[garde(custom(length_simple_err(0, 50)))]
    pub name: String,

    #[schema(example = "user@example.com")]
    #[serde(default)]
    #[garde(custom(email_err))]
    #[garde(custom(length_simple_err(0, 50)))]
    pub email: String,

    #[schema(example = "2026-01-01T00:00:00")]
    #[serde(default)]
    #[garde(custom(required_simple_err(1, 100)))]
    #[garde(custom(datetime_err))]
    pub bgn_at: String,

    #[schema(example = "2026-01-01T00:00:00")]
    #[serde(default)]
    #[garde(custom(required_simple_err(1, 100)))]
    #[garde(custom(datetime_err))]
    pub end_at: String,

    #[schema(default = 10)]
    #[garde(custom(range_err(Some(1u16), Some(25u16))))]
    pub limit: u16,

    #[schema(default = 0)]
    #[garde(custom(range_err(Some(0u16), None)))]
    pub offset: u16,
}

// ============================================================
// Create
// ============================================================
#[derive(Deserialize, Validate, ToSchema)]
pub struct CreateUsrReq {
    #[schema(example = "APX Kawata")]
    #[serde(default)]
    #[garde(custom(required_simple_err(1, 50)))]
    #[garde(custom(length_simple_err(0, 50)))]
    pub name: String,

    #[schema(example = "user01@shyme.net")]
    #[serde(default)]
    #[garde(custom(required_simple_err(1, 50)))]
    #[garde(custom(email_err))]
    #[garde(custom(length_simple_err(0, 50)))]
    pub email: String,

    #[schema(example = "password123")]
    #[serde(default)]
    #[garde(custom(required_simple_err(1, 100)))]
    pub password: String,

    #[schema(example = "2026-01-01T00:00:00")]
    #[serde(default)]
    #[garde(custom(required_simple_err(1, 100)))]
    #[garde(custom(datetime_err))]
    pub bgn_at: String,

    #[schema(example = "2100-12-31T23:59:59")]
    #[serde(default)]
    #[garde(custom(required_simple_err(1, 100)))]
    #[garde(custom(datetime_err))]
    pub end_at: String,

    #[schema(example = 1)]
    #[serde(rename = "type")]
    #[garde(inner(custom(range_err(Some(1u8), Some(2u8)))))]
    pub usr_type: Option<u8>,

    #[schema(example = 1000)]
    #[garde(inner(custom(range_err(Some(0u32), None))))]
    pub base_point: Option<u32>,

    #[schema(example = 0.1)]
    #[garde(inner(custom(range_err(Some(0.0f64), None))))]
    pub belong_rate: Option<f64>,

    #[schema(example = 5)]
    #[garde(inner(custom(range_err(Some(0u32), None))))]
    pub max_works: Option<u32>,

    #[schema(example = 3)]
    #[garde(inner(custom(range_err(Some(0u32), None))))]
    pub flush_days: Option<u32>,

    #[schema(example = 0.25)]
    #[garde(inner(custom(range_err(Some(0.0f64), None))))]
    pub rate: Option<f64>,

    #[schema(example = 0.05)]
    #[garde(inner(custom(range_err(Some(0.0f64), None))))]
    pub flush_fee_rate: Option<f64>,
}

// ============================================================
// Update
// ============================================================
#[derive(Deserialize, Validate, ToSchema)]
pub struct UpdateUsrReq {
    #[schema(example = "APX Kawata")]
    #[garde(inner(custom(length_simple_err(0, 50))))]
    pub name: Option<String>,

    #[schema(example = "user01@shyme.net")]
    #[garde(inner(custom(email_err)))]
    #[garde(inner(custom(length_simple_err(0, 50))))]
    pub email: Option<String>,

    #[schema(example = "newpassword123")]
    #[garde(skip)]
    pub password: Option<String>,

    #[schema(example = "2026-01-01T00:00:00")]
    #[garde(inner(custom(datetime_err)))]
    pub bgn_at: Option<String>,

    #[schema(example = "2100-12-31T23:59:59")]
    #[garde(inner(custom(datetime_err)))]
    pub end_at: Option<String>,

    #[schema(example = 1)]
    #[serde(rename = "type")]
    #[garde(inner(custom(range_err(Some(1u8), Some(2u8)))))]
    pub usr_type: Option<u8>,

    #[schema(example = 1000)]
    #[garde(inner(custom(range_err(Some(0u32), None))))]
    pub base_point: Option<u32>,

    #[schema(example = 0.1)]
    #[garde(inner(custom(range_err(Some(0.0f64), None))))]
    pub belong_rate: Option<f64>,

    #[schema(example = 5)]
    #[garde(inner(custom(range_err(Some(0u32), None))))]
    pub max_works: Option<u32>,

    #[schema(example = 3)]
    #[garde(inner(custom(range_err(Some(0u32), None))))]
    pub flush_days: Option<u32>,

    #[schema(example = 0.25)]
    #[garde(inner(custom(range_err(Some(0.0f64), None))))]
    pub rate: Option<f64>,

    #[schema(example = 0.05)]
    #[garde(inner(custom(range_err(Some(0.0f64), None))))]
    pub flush_fee_rate: Option<f64>,
}
