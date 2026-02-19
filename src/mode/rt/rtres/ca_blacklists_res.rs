use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use crate::mode::rt::rtbl::blacklists_bl::CrimeEvidence;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ReportBlacklistCaRes {
    pub success: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SyncBlacklistCaRes {
    pub items: Vec<CrimeEvidence>,
}
