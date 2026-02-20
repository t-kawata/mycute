use crate::mode::rt::rterr::rterr::*;
use garde::Validate;
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};

// ============================================================
// Search
// ============================================================
#[derive(Deserialize, IntoParams, Validate, ToSchema)]
pub struct SearchChatModelsReq {
    #[schema(example = "My GPT-4")]
    #[garde(custom(length_simple_err(0, 50)))]
    pub name: Option<String>,

    #[schema(example = "openai")]
    #[garde(custom(length_simple_err(0, 50)))]
    pub provider: Option<String>,

    #[schema(example = "gpt-4")]
    #[garde(custom(length_simple_err(0, 100)))]
    pub model: Option<String>,

    #[schema(example = "https://api.openai.com/v1")]
    #[garde(custom(length_simple_err(0, 255)))]
    pub base_url: Option<String>,

    #[schema(default = 10)]
    #[garde(custom(range_err(Some(1u16), Some(100u16))))]
    pub limit: u16,

    #[schema(default = 0)]
    #[garde(custom(range_err(Some(0u16), None)))]
    pub offset: u16,
}

// ============================================================
// Create
// ============================================================
#[derive(Deserialize, Validate, ToSchema)]
pub struct CreateChatModelReq {
    #[schema(example = "My GPT-4")]
    #[serde(default)]
    #[garde(custom(required_simple_err(1, 50)))]
    pub name: String,

    #[schema(example = "openai")]
    #[serde(default)]
    #[garde(custom(required_simple_err(1, 50)))]
    pub provider: String,

    #[schema(example = "gpt-4")]
    #[serde(default)]
    #[garde(custom(required_simple_err(1, 100)))]
    pub model: String,

    #[schema(example = "https://api.openai.com/v1")]
    #[garde(custom(length_simple_err(0, 255)))]
    pub base_url: Option<String>,

    #[schema(example = "sk-proj-...")]
    #[serde(default)]
    #[garde(custom(required_simple_err(1, 1024)))]
    pub api_key: String,

    #[schema(example = 2048)]
    #[serde(default)]
    #[garde(custom(range_err(Some(1i32), None)))]
    pub max_tokens: i32,

    #[schema(example = 0.7)]
    #[serde(default)]
    #[garde(custom(range_err(Some(0.0f64), Some(2.0f64))))]
    pub temperature: f64,
}

// ============================================================
// Update
// ============================================================
#[derive(Deserialize, Validate, ToSchema)]
pub struct UpdateChatModelReq {
    #[schema(example = "My GPT-4 Updated")]
    #[garde(custom(length_simple_err(0, 50)))]
    pub name: Option<String>,

    #[schema(example = "openai")]
    #[garde(custom(length_simple_err(0, 50)))]
    pub provider: Option<String>,

    #[schema(example = "gpt-4-turbo")]
    #[garde(custom(length_simple_err(0, 100)))]
    pub model: Option<String>,

    #[schema(example = "https://api.openai.com/v1")]
    #[garde(custom(length_simple_err(0, 255)))]
    pub base_url: Option<String>,

    #[schema(example = "sk-proj-new...")]
    #[garde(custom(length_simple_err(0, 1024)))]
    pub api_key: Option<String>,

    #[schema(example = 4096)]
    #[garde(custom(range_err(Some(1i32), None)))]
    pub max_tokens: Option<i32>,

    #[schema(example = 0.8)]
    #[garde(custom(range_err(Some(0.0f64), Some(2.0f64))))]
    pub temperature: Option<f64>,
}
