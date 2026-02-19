use serde::{Deserialize, Serialize};
use uuid::Uuid;
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SearchReplaceItemsRes {
    pub list: Vec<ReplaceItemDetail>,
    pub total: u64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ReplaceItemDetail {
    pub id: i32,
    #[schema(value_type = String)]
    pub replace_id: Uuid,
    pub key: String, // "Replace After" text
    pub texts: Vec<String>, // "Replace Before" texts
    pub rank: i32,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateReplaceItemRes {
    pub id: i32,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UpdateReplaceItemRes {
    pub id: i32,
}
