use serde::Deserialize;
use garde::Validate;
use utoipa::{IntoParams, ToSchema};
use crate::mode::rt::rterr::rterr::*;

// ============================================================
// Search
// ============================================================
#[derive(Deserialize, IntoParams, Validate, ToSchema)]
pub struct SearchCubesReq {
    #[schema(example = "Research")]
    #[garde(custom(length_simple_err(0, 50)))]
    pub name: Option<String>,

    #[schema(example = "Description search")]
    #[garde(custom(length_simple_err(0, 255)))]
    pub description: Option<String>,

    #[schema(default = 25, example = 25)]
    #[garde(custom(range_err(Some(1u16), Some(25u16))))]
    pub limit: Option<u16>,

    #[schema(default = 0, example = 0)]
    #[garde(custom(range_err(Some(0u16), None)))]
    pub offset: Option<u16>,
}

// ============================================================
// Create
// ============================================================
#[derive(Deserialize, Validate, ToSchema)]
pub struct CreateCubeReq {
    #[schema(example = "Physics Research")]
    #[serde(default)]
    #[garde(custom(required_simple_err(1, 50)))]
    pub name: String,

    #[schema(example = "Cube for physics papers")]
    #[garde(custom(length_simple_err(0, 255)))]
    pub description: Option<String>,

    #[schema(example = "openai")]
    #[serde(default)]
    #[garde(custom(required_simple_err(1, 50)))]
    pub embedding_provider: String,

    #[schema(example = "text-embedding-3-small")]
    #[serde(default)]
    #[garde(custom(required_simple_err(1, 100)))]
    pub embedding_model: String,

    #[schema(example = 1536)]
    #[serde(default)]
    #[garde(custom(range_err(Some(1i32), None)))]
    pub embedding_dimension: i32,

    #[schema(example = "sk-proj-...")]
    #[serde(default)]
    #[garde(custom(required_simple_err(1, 1024)))]
    pub embedding_api_key: String,

    #[schema(example = "https://api.openai.com/v1")]
    #[garde(custom(length_simple_err(0, 255)))]
    pub embedding_base_url: Option<String>,
}

// ============================================================
// Absorb
// ============================================================
#[derive(Deserialize, Validate, ToSchema)]
pub struct AbsorbCubeReq {
    #[schema(example = 1)]
    #[garde(custom(range_err(Some(1u32), None)))]
    pub cube_id: u32,

    #[schema(example = "group1")]
    #[garde(custom(length_simple_err(0, 64)))]
    pub memory_group: Option<String>,

    #[schema(example = "Content text...")]
    #[garde(custom(length_simple_err(0, 1000000)))] // Large limit for content
    pub content: Option<String>,

    #[schema(example = 512)]
    #[garde(custom(range_err(Some(25i32), None)))]
    pub chunk_size: Option<i32>,

    #[schema(example = 16)]
    #[garde(custom(range_err(Some(0i32), None)))]
    pub chunk_overlap: Option<i32>,

    #[schema(example = 1)]
    #[garde(custom(range_err(Some(1u32), None)))]
    pub chat_model_id: Option<u32>,

    #[schema(example = false)]
    #[garde(skip)]
    pub stream: Option<bool>,

    #[schema(example = false)]
    #[garde(skip)]
    pub as_json: Option<bool>,

    #[schema(example = false)]
    #[garde(skip)]
    pub is_en: Option<bool>,

    #[schema(example = 30.0)]
    #[garde(custom(range_err(Some(1.0f64), None)))]
    pub half_life_days: Option<f64>,

    #[schema(example = 0.1)]
    #[garde(custom(range_err(Some(0.0f64), Some(1.0f64))))]
    pub prune_threshold: Option<f64>,

    #[schema(example = 72.0)]
    #[garde(custom(range_err(Some(0.0f64), None)))]
    pub min_survival_protection_hours: Option<f64>,

    #[schema(example = 5)]
    #[garde(custom(range_err(Some(1i32), None)))]
    pub mdl_k_neighbors: Option<i32>,
}

// ============================================================
// Query
// ============================================================
#[derive(Deserialize, Validate, ToSchema)]
pub struct QueryCubeReq {
    #[schema(example = 1)]
    #[garde(custom(range_err(Some(1u32), None)))]
    pub cube_id: u32,

    #[schema(example = "legal_expert")]
    #[garde(custom(length_simple_err(0, 64)))]
    pub memory_group: Option<String>,

    #[schema(example = "Question?")]
    #[garde(custom(length_simple_err(0, 10000)))]
    pub text: Option<String>,

    #[schema(example = 1)]
    #[garde(custom(range_err(Some(1u8), Some(11u8))))]
    pub r#type: Option<u8>,

    #[schema(example = 3)]
    #[garde(custom(range_err(Some(0i32), None)))]
    pub summary_topk: Option<i32>,

    #[schema(example = 3)]
    #[garde(custom(range_err(Some(0i32), None)))]
    pub chunk_topk: Option<i32>,

    #[schema(example = 3)]
    #[garde(custom(range_err(Some(0i32), None)))]
    pub entity_topk: Option<i32>,

    #[schema(example = 0)]
    #[garde(custom(range_err(Some(0u8), Some(2u8))))]
    pub fts_type: Option<u8>,

    #[schema(example = 0)]
    #[garde(custom(range_err(Some(0i32), None)))]
    pub fts_topk: Option<i32>,

    #[schema(example = 0.3)]
    #[garde(custom(range_err(Some(0.0f64), Some(1.0f64))))]
    pub thickness_threshold: Option<f64>,

    #[schema(example = 2)]
    #[garde(custom(range_err(Some(0u8), Some(2u8))))]
    pub conflict_resolution_stage: Option<u8>,

    #[schema(example = 1)]
    #[garde(custom(range_err(Some(1u32), None)))]
    pub chat_model_id: Option<u32>,

    #[schema(example = false)]
    #[garde(skip)]
    pub stream: Option<bool>,

    #[schema(example = false)]
    #[garde(skip)]
    pub as_json: Option<bool>,

    #[schema(example = false)]
    #[garde(skip)]
    pub is_en: Option<bool>,
}

// ============================================================
// Memify
// ============================================================
#[derive(Deserialize, Validate, ToSchema)]
pub struct MemifyCubeReq {
    #[schema(example = 1)]
    #[garde(custom(range_err(Some(1u32), None)))]
    pub cube_id: u32,

    #[schema(example = "group1")]
    #[garde(custom(length_simple_err(0, 64)))]
    pub memory_group: Option<String>,

    #[schema(example = 10)]
    #[garde(custom(range_err(Some(0i32), None)))]
    pub epochs: Option<i32>,

    #[schema(example = false)]
    #[garde(skip)]
    pub prioritize_unknowns: Option<bool>,

    #[schema(example = 2)]
    #[garde(custom(range_err(Some(0u8), Some(2u8))))]
    pub conflict_resolution_stage: Option<u8>,

    #[schema(example = 1)]
    #[garde(custom(range_err(Some(1u32), None)))]
    pub chat_model_id: Option<u32>,

    #[schema(example = false)]
    #[garde(skip)]
    pub stream: Option<bool>,

    #[schema(example = false)]
    #[garde(skip)]
    pub as_json: Option<bool>,

    #[schema(example = false)]
    #[garde(skip)]
    pub is_en: Option<bool>,
}

// ============================================================
// GenKey
// ============================================================
// In Go this is Multipart with 'permissions' JSON string.
// In Rust strict rules, we use Request structs. Only Absorb/Import use multipart for files usually.
// But GenKey requires .cube upload? Go: `file formData file true`.
// Wait, `GenKeyCube` in Go takes a FILE? 
// "Exportされた.cubeファイルをアップロードして鍵を発行" -> Yes.
// For now, we define the Struct part (permissions).
#[derive(Deserialize, Validate, ToSchema)]
pub struct GenKeyCubeReq {
    #[schema(example = "{}")]
    #[garde(skip)]
    pub permissions: String, // JSON string

    #[schema(example = "2026-12-31T23:59:59")]
    #[garde(skip)] // Datetime validation needed? Go uses ISO8601
    pub expire_at: Option<String>,
}

// ============================================================
// ReKey
// ============================================================
#[derive(Deserialize, Validate, ToSchema)]
pub struct ReKeyCubeReq {
    #[schema(example = 1)]
    #[garde(custom(range_err(Some(1u32), None)))]
    pub cube_id: u32,

    #[schema(example = "new-secret-key...")]
    #[garde(custom(length_simple_err(1, 1024)))]
    pub key: String,
}

// ============================================================
// Import
// ============================================================
// Go: Multipart form data.
#[derive(Deserialize, Validate, ToSchema)]
pub struct ImportCubeReq {
    #[schema(example = "key...")]
    #[garde(custom(length_simple_err(1, 1024)))]
    pub key: String,

    #[schema(example = "My Imported Cube")]
    #[garde(custom(length_simple_err(0, 50)))]
    pub name: Option<String>,

    #[schema(example = "Description...")]
    #[garde(custom(length_simple_err(0, 255)))]
    pub description: Option<String>,

    #[schema(example = "sk-proj-...")]
    #[garde(custom(length_simple_err(0, 1024)))]
    pub embedding_api_key: Option<String>,
}

// ============================================================
// Delete (Query)
// ============================================================
#[derive(Deserialize, Validate, ToSchema, IntoParams)]
pub struct DeleteCubeReq {
    #[schema(example = 1)]
    #[garde(custom(range_err(Some(1u32), None)))]
    pub cube_id: u32,
}

// ============================================================
// Export (Query)
// ============================================================
#[derive(Deserialize, Validate, ToSchema, IntoParams)]
pub struct ExportCubeReq {
    #[schema(example = 1)]
    #[garde(custom(range_err(Some(1u32), None)))]
    pub cube_id: u32,
}
