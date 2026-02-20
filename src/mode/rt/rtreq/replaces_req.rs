use crate::mode::rt::rterr::rterr::*;
use crate::mode::rt::rtreq::replace_items_req::ImportReplaceItem;
use garde::Validate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Validate, utoipa::ToSchema)]
pub struct SearchReplacesReq {
    #[garde(skip)]
    pub keyword: Option<String>,
    #[garde(skip)]
    pub is_active: Option<bool>,

    #[garde(custom(range_err(Some(1u64), Some(100u64))))]
    #[serde(default = "default_limit")]
    pub limit: u64,
    #[garde(skip)]
    #[serde(default)]
    pub offset: u64,
}

fn default_limit() -> u64 {
    25
}

#[derive(Debug, Serialize, Deserialize, Validate, utoipa::ToSchema)]
pub struct CreateReplacesReq {
    #[garde(custom(required_simple_err(1, 100)))]
    pub name: String,
    #[garde(inner(custom(length_simple_err(0, 500))))]
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Validate, utoipa::ToSchema)]
pub struct UpdateReplacesReq {
    #[garde(inner(custom(length_simple_err(1, 100))))]
    pub name: Option<String>,
    #[garde(inner(custom(length_simple_err(0, 500))))]
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Validate, utoipa::ToSchema)]
pub struct ActivateReplacesReq {
    #[garde(skip)]
    pub is_active: bool,
}

#[derive(Debug, Serialize, Deserialize, Validate, utoipa::ToSchema)]
pub struct ImportReplacesReq {
    #[garde(custom(required_simple_err(1, 100)))]
    pub name: String,
    #[garde(inner(custom(length_simple_err(0, 500))))]
    pub description: Option<String>,

    #[garde(length(min = 1))]
    pub items: Vec<ImportReplaceItem>,
}
