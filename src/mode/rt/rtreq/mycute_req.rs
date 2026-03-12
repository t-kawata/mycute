use crate::{stt_config::SttEngine, types::LocaleCode};
use garde::Validate;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct SetLangReq {
    #[garde(skip)]
    #[schema(value_type = String, example = "en")]
    pub locale: LocaleCode,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate, ToSchema)]
pub struct SetSttEngineReq {
    #[garde(skip)]
    #[schema(value_type = String, example = "os")]
    pub engine: SttEngine,
}
