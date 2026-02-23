use garde::Validate;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct SetLangReq {
    #[garde(skip)]
    #[schema(example = "en")]
    pub locale: String,
}
