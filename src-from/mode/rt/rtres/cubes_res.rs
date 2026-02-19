use serde::Serialize;
use utoipa::ToSchema;
use crate::entities::cubes;
use crate::utils::time::datetime_to_str;

// ============================================================
// Common Structures
// ============================================================

#[derive(Serialize, ToSchema)]
pub struct CubeRes {
    #[schema(example = 1)]
    pub id: i32,
    #[schema(example = "550e8400-e29b-41d4-a716-446655440000")]
    pub uuid: String,
    #[schema(example = "MyCube")]
    pub name: String,
    #[schema(example = "This is my cube")]
    pub description: String,
    #[schema(example = "openai")]
    pub embedding_provider: String,
    #[schema(example = "text-embedding-3-small")]
    pub embedding_model: String,
    #[schema(example = 1536)]
    pub embedding_dimension: i32,
    #[schema(example = "2026-01-08T00:00:00")]
    pub created_at: String,
    #[schema(example = "2026-01-08T00:00:00")]
    pub updated_at: String,
    // Add other fields from Go if present in DB model and returned
}

impl From<cubes::Model> for CubeRes {
    fn from(m: cubes::Model) -> Self {
        Self {
            id: m.id,
            uuid: m.uuid,
            name: m.name,
            description: m.description,
            embedding_provider: m.embedding_provider,
            embedding_model: m.embedding_model,
            embedding_dimension: m.embedding_dimension,
            created_at: datetime_to_str(&m.created_at),
            updated_at: datetime_to_str(&m.updated_at),
        }
    }
}

#[derive(Serialize, ToSchema)]
pub struct LineageRes {
    pub uuid: String,
    pub owner: String,
    pub exported_at: i64,
    pub exported_at_jst: String,
    pub generation: i32,
}

#[derive(Serialize, ToSchema)]
pub struct ModelStatRes {
    pub model_name: String,
    pub action_type: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
}

#[derive(Serialize, ToSchema)]
pub struct ContributorRes {
    pub contributor_name: String,
    pub model_name: String,
    pub input_tokens: i64,
    pub output_tokens: i64,
}

#[derive(Serialize, ToSchema)]
pub struct MemoryGroupStatsRes {
    pub memory_group: String,
    pub stats: Vec<ModelStatRes>,
    pub contributors: Vec<ContributorRes>,
}

#[derive(Serialize, ToSchema)]
pub struct SearchCubesResItem {
    pub cube: CubeRes,
    pub lineage: Vec<LineageRes>,
    pub memory_groups: Vec<MemoryGroupStatsRes>,
}

// ============================================================
// Search
// ============================================================
#[derive(Serialize, ToSchema)]
pub struct SearchCubesRes {
    pub data: Vec<SearchCubesResItem>,
    pub total: u64,
}

// ============================================================
// Get
// ============================================================
// Same structure as Search Item
pub type GetCubeRes = SearchCubesResItem;

// ============================================================
// Create
// ============================================================
#[derive(Serialize, ToSchema)]
pub struct CreateCubeRes {
    #[schema(example = 1)]
    pub id: i32,
    #[schema(example = "550e8400-e29b-41d4-a716-446655440000")]
    pub uuid: String,
}

// ============================================================
// Delete
// ============================================================
#[derive(Serialize, ToSchema)]
pub struct DeleteCubeRes {
    #[schema(example = true)]
    pub success: bool,
}

// ============================================================
// Absorb
// ============================================================
#[derive(Serialize, ToSchema)]
pub struct AbsorbCubeRes {
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub absorb_limit: i32,
}

// ============================================================
// Query
// ============================================================
#[derive(Serialize, ToSchema)]
pub struct QueryCubeRes {
    pub answer: Option<String>,
    pub chunks: Option<String>,
    pub summaries: Option<String>,
    pub graph: Option<Vec<String>>, // Simplified Triple for dummy
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub query_limit: i32,
}

// ============================================================
// Memify
// ============================================================
#[derive(Serialize, ToSchema)]
pub struct MemifyCubeRes {
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub memify_limit: i32,
}

// ============================================================
// GenKey
// ============================================================
#[derive(Serialize, ToSchema)]
pub struct GenKeyCubeRes {
    #[schema(example = "generated-key-string")]
    pub key: String,
}

// ============================================================
// ReKey
// ============================================================
#[derive(Serialize, ToSchema)]
pub struct ReKeyCubeRes {
    #[schema(example = true)]
    pub success: bool,
}

// ============================================================
// Export
// ============================================================
// Streaming file response, but UtioPa might need a struct placeholder or just file
#[derive(Serialize, ToSchema)]
pub struct ExportCubeRes {
    // Usually empty JSON on success if file is downloaded, or error.
}

// ============================================================
// Import
// ============================================================
#[derive(Serialize, ToSchema)]
pub struct ImportCubeRes {
    pub id: i32,
    pub uuid: String,
}
