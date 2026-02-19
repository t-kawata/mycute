use axum::Json;
use utoipa::ToSchema;
use serde::Serialize;
use serde_json::{json, Value};

#[derive(Serialize, ToSchema)]
pub struct HealthStatus {
    pub status: String,
}

/// Check server health
#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Server is healthy", body = HealthStatus)
    ),
    tag = "v1 Health"
)]
pub async fn check_health() -> Json<Value> {
    Json(json!({
        "status": "ok"
    }))
}
