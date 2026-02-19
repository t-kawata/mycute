use serde::Deserialize;
use garde::Validate;
use utoipa::ToSchema;

#[derive(Deserialize, Validate, ToSchema)]
pub struct AssignCaReq {
    #[garde(url)]
    pub target_url: String, // e.g. "http://192.168.1.10:8080"
    #[garde(range(min=1))]
    pub expire_hours: u32,
}
