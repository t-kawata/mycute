use crate::{
    mode::rt::{
        rtreq::ca_blacklists_req::{ReportBlacklistCaReq, SyncBlacklistCaReq},
        rtres::{ca_blacklists_res::{ReportBlacklistCaRes, SyncBlacklistCaRes}, errs_res::ApiError},
        rtbl::blacklists_bl::{self, CrimeEvidence},
    },
    utils::db::DbPools,
};

// ============================================================
// CA Blacklist Logic
// ============================================================

/// 不正証拠を受け付ける (CA)
/// 1. 証拠の検証
/// 2. DB に保存
pub async fn report_blacklist_ca(
    db: &DbPools,
    req: ReportBlacklistCaReq,
) -> Result<ReportBlacklistCaRes, ApiError> {
    log::info!("<Blacklist> Processing report_blacklist_ca. Target: {}", req.evidence.target_pubkey);

    // 1. 検証
    blacklists_bl::validate_evidence(&req.evidence)?;

    // 2. 保存
    blacklists_bl::add_to_blacklist(db, req.evidence).await?;

    Ok(ReportBlacklistCaRes { success: true })
}

/// ブラックリストを提供する (CA)
/// 1. DB から指定 ID 以降のデータを取得して返す
pub async fn sync_blacklists_ca(
    db: &DbPools,
    req: SyncBlacklistCaReq,
) -> Result<SyncBlacklistCaRes, ApiError> {
    // req.since_ts is i64 (Timestamp ms)
    let records: Vec<crate::entities::blacklists::Model> = blacklists_bl::get_blacklists_since(db, req.since_ts).await?;
    
    let items = records.into_iter()
        .filter_map(|r| serde_json::from_str::<CrimeEvidence>(&r.evidence_json).ok())
        .collect::<Vec<_>>();

    Ok(SyncBlacklistCaRes { items })
}
