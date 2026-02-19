use crate::mode::rt::rtbl::blacklists_bl::CrimeEvidence;

#[derive(Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema, garde::Validate)]
pub struct ReportBlacklistCaReq {
    #[garde(skip)]
    pub evidence: CrimeEvidence,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, utoipa::IntoParams, utoipa::ToSchema, garde::Validate)]
pub struct SyncBlacklistCaReq {
    /// 最後に同期した時刻 (ms)。これより新しい証拠を同期する。
    #[garde(skip)]
    pub since_ts: i64,
}
