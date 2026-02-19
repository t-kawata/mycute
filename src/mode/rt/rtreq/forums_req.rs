use serde::Deserialize;
use utoipa::{ToSchema, IntoParams};
use garde::Validate;
use crate::mode::rt::rterr::rterr::*;

#[derive(Deserialize, IntoParams, Validate, ToSchema)]
pub struct SearchForumsReq {
    #[serde(default)]
    #[garde(custom(length_simple_err(0, 255)))]
    pub keyword: String,

    #[serde(default = "default_limit")]
    #[garde(custom(range_err(Some(1u32), Some(100u32))))]
    pub limit: u32,

    #[serde(default)]
    #[garde(skip)]
    pub offset: u32,
}

fn default_limit() -> u32 { 25 }

#[derive(Deserialize, Validate, ToSchema)]
pub struct CreateForumReq {
    #[schema(example = "General Forum")]
    #[serde(default)]
    #[garde(custom(required_simple_err(1, 255)))]
    #[garde(custom(length_simple_err(0, 255)))]
    pub name: String,
    
    #[schema(example = "Discussion for general topics")]
    #[serde(default)]
    #[garde(custom(required_simple_err(1, 1000)))]
    #[garde(custom(length_simple_err(0, 1000)))]
    pub description: String,

    #[schema(example = 15, minimum = 2, maximum = 30, default = 15)]
    #[serde(default = "default_initial_balance")]
    #[garde(custom(range_err(Some(2i32), Some(30i32))))]
    pub initial_balance: i32,
}

fn default_initial_balance() -> i32 { 15 }

#[derive(Deserialize, Validate, ToSchema)]
pub struct UpdateForumReq {
    #[schema(example = "Project Forum")]
    #[garde(inner(custom(length_simple_err(1, 255))))]
    pub name: Option<String>,
    
    #[schema(example = "Updated description")]
    #[garde(inner(custom(length_simple_err(0, 1000))))]
    pub description: Option<String>,
}
