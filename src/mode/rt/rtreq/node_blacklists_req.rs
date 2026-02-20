use crate::mode::rt::rtbl::blacklists_bl::CrimeEvidence;

#[derive(Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema, garde::Validate)]
pub struct ReportBlacklistNodeReq {
    #[garde(skip)]
    pub evidence: CrimeEvidence,
    /// 報告先の CA Base URL (必須)
    #[garde(skip)]
    pub ca_base_url: String,
}

#[derive(Debug, serde::Deserialize, utoipa::IntoParams, utoipa::ToSchema, garde::Validate)]
pub struct SyncBlacklistNodeReq {
    /// 同期先の CA Base URL (Optional: 指定がない場合はデフォルトCAまたはエラー?)
    /// Node側で同期する場合、どのCAから同期するかを指定する必要がある。
    /// しかし、ここでは Query Parameter として定義されている。
    /// node_apps_req.rs では ca_base_url は Body に含まれることが多いが、GETなので Query になる。
    #[garde(skip)]
    pub ca_base_url: String,
}
