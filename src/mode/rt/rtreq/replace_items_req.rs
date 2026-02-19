use serde::{Deserialize, Serialize};
use garde::Validate;
use uuid::Uuid;
use utoipa::ToSchema;
use crate::mode::rt::rterr::rterr::*;

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct SearchReplaceItemsReq {
    #[garde(skip)]
    #[schema(value_type = String)]
    pub replace_id: Uuid,
    #[garde(skip)]
    pub keyword: Option<String>,
    
    #[garde(custom(range_err(Some(1u64), Some(200u64))))]
    #[serde(default = "default_limit_items")]
    pub limit: u64,
    #[garde(skip)]
    #[serde(default)]
    pub offset: u64,
}

fn default_limit_items() -> u64 { 100 }

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct CreateReplaceItemReq {
    #[garde(skip)]
    #[schema(value_type = String)]
    pub replace_id: Uuid,
    
    #[garde(custom(required_simple_err(1, 200)))]
    pub key: String,
    
    #[garde(length(min = 1))]
    #[garde(inner(custom(length_simple_err(1, 200))))]
    pub texts: Vec<String>,
    
    #[garde(skip)]
    pub rank: i32,
}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct UpdateReplaceItemReq {
    #[garde(inner(custom(length_simple_err(1, 200))))]
    pub key: Option<String>,
    
    #[garde(inner(inner(custom(length_simple_err(1, 200)))))]
    pub texts: Option<Vec<String>>,
    
    #[garde(skip)]
    pub rank: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, Validate, ToSchema)]
pub struct ImportReplaceItem {
    #[garde(custom(required_simple_err(1, 200)))]
    pub key: String,
    #[garde(length(min = 1))]
    #[garde(inner(custom(length_simple_err(1, 200))))]
    pub texts: Vec<String>,
    #[garde(skip)]
    pub rank: i32,
}
