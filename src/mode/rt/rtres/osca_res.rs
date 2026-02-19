use serde::Serialize;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct GetOscaUrlRes {
    #[schema(example = "http://192.168.1.10:8889/mycute-osca.pem")]
    pub osca_url: String,
}
