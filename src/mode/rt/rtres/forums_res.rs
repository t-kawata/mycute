use crate::entities::forums;
use crate::utils::time::datetime_to_str;
use serde::Serialize;
use utoipa::ToSchema;

// ============================================================
// Search
// ============================================================
#[derive(Serialize, ToSchema)]
pub struct SearchForumsRes {
    pub total: u64,
    pub forums: Vec<SearchForumsResItem>,
}

#[derive(Serialize, ToSchema)]
pub struct SearchForumsResItem {
    pub id: String,
    pub name: String,
    pub description: String,
    pub initial_balance: i32,
    pub created_at: String,
    pub updated_at: String,
}

impl From<forums::Model> for SearchForumsResItem {
    fn from(m: forums::Model) -> Self {
        let uuid = m.id;
        Self {
            id: uuid.to_string(),
            name: m.name,
            description: m.description,
            initial_balance: m.initial_balance,
            created_at: datetime_to_str(&m.created_at),
            updated_at: datetime_to_str(&m.updated_at),
        }
    }
}

// ============================================================
// Get
// ============================================================
#[derive(Serialize, ToSchema)]
pub struct GetForumRes {
    pub id: String,
    pub name: String,
    pub description: String,
    pub initial_balance: i32,
    pub created_at: String,
    pub updated_at: String,
}

impl From<forums::Model> for GetForumRes {
    fn from(m: forums::Model) -> Self {
        let uuid = m.id;
        Self {
            id: uuid.to_string(),
            name: m.name,
            description: m.description,
            initial_balance: m.initial_balance,
            created_at: datetime_to_str(&m.created_at),
            updated_at: datetime_to_str(&m.updated_at),
        }
    }
}

// ============================================================
// Create
// ============================================================
#[derive(Serialize, ToSchema)]
pub struct CreateForumRes {
    pub id: String,
    pub name: String,
    pub description: String,
    pub initial_balance: i32,
    pub created_at: String,
    pub updated_at: String,
}

impl From<forums::Model> for CreateForumRes {
    fn from(m: forums::Model) -> Self {
        let uuid = m.id;
        Self {
            id: uuid.to_string(),
            name: m.name,
            description: m.description,
            initial_balance: m.initial_balance,
            created_at: datetime_to_str(&m.created_at),
            updated_at: datetime_to_str(&m.updated_at),
        }
    }
}

// ============================================================
// Update
// ============================================================
#[derive(Serialize, ToSchema)]
pub struct UpdateForumRes {
    pub id: String,
    pub name: String,
    pub description: String,
    pub initial_balance: i32,
    pub created_at: String,
    pub updated_at: String,
}

impl From<forums::Model> for UpdateForumRes {
    fn from(m: forums::Model) -> Self {
        let uuid = m.id;
        Self {
            id: uuid.to_string(),
            name: m.name,
            description: m.description,
            initial_balance: m.initial_balance,
            created_at: datetime_to_str(&m.created_at),
            updated_at: datetime_to_str(&m.updated_at),
        }
    }
}

// ============================================================
// Delete
// ============================================================
#[derive(Serialize, ToSchema)]
pub struct DeleteForumRes {
    pub id: String,
}
