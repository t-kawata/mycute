use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::mode::rt::rtres::replace_items_res::ReplaceItemDetail;

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct SearchReplacesRes {
    pub list: Vec<ReplacesListItem>,
    pub total: u64,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ReplacesListItem {
    #[schema(value_type = String)]
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct GetReplacesRes {
    pub replace: ReplacesDetail,
    // アイテムは別途取得するか、含めるか？
    // 通常、GetReplacesはメタデータのみを返し、アイテムには別途エンドポイントを用意する。
    // しかし、利便性のためにアイテム数を含めることにする。
    pub items_count: u64,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ReplacesDetail {
    #[schema(value_type = String)]
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CreateReplacesRes {
    #[schema(value_type = String)]
    pub id: Uuid,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct UpdateReplacesRes {
    #[schema(value_type = String)]
    pub id: Uuid,
}

#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ActivateReplacesRes {
    #[schema(value_type = String)]
    pub id: Uuid,
    pub is_active: bool,
}


#[derive(Debug, Serialize, Deserialize, utoipa::ToSchema)]
pub struct ExportReplacesRes {
    // エクスポートされるJSON構造
    pub replace: ReplacesDetail,
    pub items: Vec<ReplaceItemDetail>,
}
