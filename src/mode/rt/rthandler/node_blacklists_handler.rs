use crate::{
    mode::rt::{
        client::secure_client::SecureClient,
        rtbl::node_blacklists_bl,
        rtreq::node_blacklists_req::{ReportBlacklistNodeReq, SyncBlacklistNodeReq},
        rtres::{
            errs_res::ApiError,
            node_blacklists_res::{ReportBlacklistNodeRes, SyncBlacklistNodeRes},
        },
    },
    stt_config::ConfigManager,
    utils::{
        db::DbPools,
        jwt::{JwtRole, JwtUsr},
    },
};
use axum::{Extension, Json};
use garde::Validate;
use std::sync::Arc;

const TAG: &str = "v1 Node Blacklist";

// ============================================================
// Report (Node Side)
// ============================================================
const REPORT_NODE_DESC: &str = r#"
### ⚫︎ 概要
- [Node Side Endpoint]
- ローカルノードに対して犯罪証拠 (Crime Evidence) を報告する。
- 報告された証拠はローカル DB に保存され、さらに CA に対してリレー報告される。
- 主に管理 UI やローカルプロセスからの呼び出しを想定。

### ⚫︎ 権限
- **USR**: 認証済みユーザーのみ実行可能。
"#;

#[utoipa::path(
    tag = TAG,
    post,
    security(("api_jwt_token" = [])),
    path = "/node/blacklists/report",
    summary = "不正証拠を報告する (Node Relay)",
    description = REPORT_NODE_DESC,
    request_body = ReportBlacklistNodeReq,
    responses(
        (status = 200, description = "Success", body = ReportBlacklistNodeRes),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn report_blacklist_node(
    ju: JwtUsr,
    Extension(db): Extension<Arc<DbPools>>,
    Extension(client): Extension<Arc<SecureClient>>,
    // Note: Request body has no specific param for CA URL?
    // Usually ConfigManager or Default CA is used if not specified.
    // node_blacklists_bl::report_blacklist_node takes `ca_base_url`.
    // Where do we get it? From ConfigManager (default CA)? Or Request?
    // node_blacklists_req definition: Request Body only has `evidence`.
    // So we should use Primary CA from config.
    Extension(_config_manager): Extension<Arc<ConfigManager>>,
    Json(req): Json<ReportBlacklistNodeReq>,
) -> Result<Json<ReportBlacklistNodeRes>, ApiError> {
    ju.allow_roles(&[JwtRole::USR])?; // User specified USR only
    req.validate().map_err(|e| ApiError::from_garde(e))?;

    // Get CA URL from Request
    let ca_base_url = req.ca_base_url.clone();

    let res = node_blacklists_bl::report_blacklist_node(&db, &client, req, ca_base_url).await?;
    Ok(Json(res))
}

// ============================================================
// Sync (Node Side)
// ============================================================
const SYNC_NODE_DESC: &str = r#"
### ⚫︎ 概要
- [Node Side Endpoint]
- ローカルノードのブラックリスト情報を同期・取得する。
- オプションで `ca_base_url` を指定することで、CA からの同期もトリガー可能。

### ⚫︎ 権限
- **USR**: 認証済みユーザーのみ実行可能。
"#;

#[utoipa::path(
    tag = TAG,
    get,
    security(("api_jwt_token" = [])),
    path = "/node/blacklists/sync",
    summary = "ブラックリストを同期する (Node Local & Relay)",
    description = SYNC_NODE_DESC,
    params(
        SyncBlacklistNodeReq,
    ),
    responses(
        (status = 200, description = "Success", body = SyncBlacklistNodeRes),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn sync_blacklists_node(
    ju: JwtUsr,
    Extension(db): Extension<Arc<DbPools>>,
    Extension(client): Extension<Arc<SecureClient>>,
    Extension(config_manager): Extension<Arc<ConfigManager>>,
    axum::extract::Query(req): axum::extract::Query<SyncBlacklistNodeReq>,
) -> Result<Json<SyncBlacklistNodeRes>, ApiError> {
    ju.allow_roles(&[JwtRole::USR])?;
    req.validate().map_err(|e| ApiError::from_garde(e))?;
    let res = node_blacklists_bl::sync_blacklists_node(&db, &client, &config_manager, req).await?;
    Ok(Json(res))
}
