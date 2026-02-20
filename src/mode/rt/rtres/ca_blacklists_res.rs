use crate::mode::rt::rtbl::blacklists_bl::CrimeEvidence;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ReportBlacklistCaRes {
    pub success: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SyncBlacklistCaRes {
    pub items: Vec<CrimeEvidence>,
}
