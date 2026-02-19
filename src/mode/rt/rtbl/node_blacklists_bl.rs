use crate::{
    mode::rt::{
        client::secure_client::SecureClient,
        rtbl::blacklists_bl,
        rtreq::node_blacklists_req::{ReportBlacklistNodeReq, SyncBlacklistNodeReq},
        rtres::{errs_res::ApiError, node_blacklists_res::{ReportBlacklistNodeRes, SyncBlacklistNodeRes}},
    },
    utils::db::DbPools,
    stt_config::ConfigManager,
};

// ============================================================
// Node Blacklist Logic (Handler Relays)
// ============================================================

/// 不正証拠を報告する (Node -> CA Relay)
/// 1. ローカル DB に保存
/// 2. CA にリレー報告
pub async fn report_blacklist_node(
    db: &DbPools,
    client: &SecureClient,
    req: ReportBlacklistNodeReq,
    ca_base_url: String, // 報告先 CA (クライアントが指定、またはデフォルト)
) -> Result<ReportBlacklistNodeRes, ApiError> {
    log::info!("<Blacklist> Processing report_blacklist_node. Target: {}", req.evidence.target_pubkey);

    // 1. ローカル保存 (検証含む)
    blacklists_bl::validate_evidence(&req.evidence)?;
    blacklists_bl::add_to_blacklist(db, req.evidence.clone()).await?;

    // 2. CA へリレー
    blacklists_bl::report_crime_to_ca(&req.evidence, &ca_base_url, client).await?;

    Ok(ReportBlacklistNodeRes { success: true })
}

/// ブラックリストを同期する (Node Sync)
/// 1. (Optional) 指定された CA と同期を実行 (Fetch & Open)
/// 2. ローカル DB から指定 ID 以降のデータを返却
pub async fn sync_blacklists_node(
    db: &DbPools,
    client: &SecureClient,
    config_manager: &ConfigManager,
    req: SyncBlacklistNodeReq,
) -> Result<SyncBlacklistNodeRes, ApiError> {
    let mut new_items_count = 0;

    // 1. CA との同期 (ca_base_url が指定されている場合)
    if !req.ca_base_url.is_empty() {
        match blacklists_bl::sync_blacklists_with_ca(db, &req.ca_base_url, client, config_manager).await {
            Ok(count) => {
                new_items_count = count;
            },
            Err(e) => {
                log::warn!("<Blacklist> Failed to sync with CA '{}': {}", req.ca_base_url, e);
                // 同期指示自体は受け付けたが中身が失敗したケース。
                // 厳密にエラーを返すべきかは要検討だが、一旦 false を返す設計も可能。
                return Ok(SyncBlacklistNodeRes { success: false, new_items_count: 0 });
            }
        }
    }

    Ok(SyncBlacklistNodeRes { success: true, new_items_count })
}
