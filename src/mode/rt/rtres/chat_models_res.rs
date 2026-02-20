use crate::entities::chat_models;
use crate::utils::time::datetime_to_str;
use serde::Serialize;
use utoipa::ToSchema;

// ============================================================
// Common
// ============================================================
#[derive(Serialize, ToSchema)]
pub struct ChatModelRes {
    #[schema(example = 1)]
    pub id: i32,
    #[schema(example = "My GPT-4")]
    pub name: String,
    #[schema(example = "openai")]
    pub provider: String,
    #[schema(example = "gpt-4")]
    pub model: String,
    #[schema(example = "https://api.openai.com/v1")]
    pub base_url: String,
    #[schema(example = 2048)]
    pub max_tokens: i32,
    #[schema(example = 0.7)]
    pub temperature: f64,
    #[schema(example = "2026-01-08T00:00:00")]
    pub created_at: String,
    #[schema(example = "2026-01-08T00:00:00")]
    pub updated_at: String,
}

impl From<chat_models::Model> for ChatModelRes {
    fn from(m: chat_models::Model) -> Self {
        Self {
            id: m.id,
            name: m.name,
            provider: m.provider,
            model: m.model,
            base_url: m.base_url,
            max_tokens: m.max_tokens,
            temperature: m.temperature,
            created_at: datetime_to_str(&m.created_at),
            updated_at: datetime_to_str(&m.updated_at),
        }
    }
}

// ============================================================
// Search
// ============================================================
#[derive(Serialize, ToSchema)]
pub struct SearchChatModelsRes {
    pub data: Vec<ChatModelRes>,
}

// ============================================================
// Create
// ============================================================
#[derive(Serialize, ToSchema)]
pub struct CreateChatModelRes {
    #[schema(example = 1)]
    pub id: i32,
}

// ============================================================
// Update
// ============================================================
#[derive(Serialize, ToSchema)]
pub struct UpdateChatModelRes {
    #[schema(example = true)]
    pub success: bool,
}

// ============================================================
// Delete
// ============================================================
#[derive(Serialize, ToSchema)]
pub struct DeleteChatModelRes {
    #[schema(example = true)]
    pub success: bool,
}
