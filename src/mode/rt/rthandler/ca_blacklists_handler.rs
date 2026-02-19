use std::sync::Arc;
use axum::{Extension, Json};
use garde::Validate;
use crate::{
    TAG_MACRO_P2P_STRICT,
    mode::rt::{
        rtreq::ca_blacklists_req::{ReportBlacklistCaReq, SyncBlacklistCaReq},
        rtres::{ca_blacklists_res::{ReportBlacklistCaRes, SyncBlacklistCaRes}, errs_res::ApiError},
        rtbl::ca_blacklists_bl,
    },
    utils::db::DbPools,
};

macro_rules! TAG_NAME { () => { "v1 CA Blacklist" }; }
const TAG_P2P_STRICT: &str = concat!(TAG_NAME!(), " ", TAG_MACRO_P2P_STRICT!());

// ============================================================
// Report (CA Side)
// ============================================================
const REPORT_CA_DESC: &str = r#"
### ⚫︎ 概要
- [CA Side Endpoint]
- 犯罪証拠 (Crime Evidence) を報告し、対象の公開鍵をブラックリストに登録する。
- **P2P プロトコルによる通信**を想定しており、P2P ヘッダーによる署名検証が必須。

### ⚫︎ 権限
- **Public**: P2P プロトコル通信として、全てのノードからのアクセスを許可。
"#;

#[utoipa::path(
    tag = TAG_P2P_STRICT,
    post,
    path = "/ca/blacklists/report",
    summary = "不正証拠を報告する (CA P2P Strict)",
    description = REPORT_CA_DESC,
    request_body = ReportBlacklistCaReq,
    responses(
        (status = 200, description = "Success", body = ReportBlacklistCaRes),
        (status = 400, description = "Bad Request", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn report_blacklist_ca(
    Extension(db): Extension<Arc<DbPools>>,
    Json(req): Json<ReportBlacklistCaReq>,
) -> Result<Json<ReportBlacklistCaRes>, ApiError> {
    req.validate().map_err(|e| ApiError::from_garde(e))?;
    let res = ca_blacklists_bl::report_blacklist_ca(&db, req).await?;
    Ok(Json(res))
}

// ============================================================
// Sync (CA Side)
// ============================================================
const SYNC_CA_DESC: &str = r#"
### ⚫︎ 概要
- [CA Side Endpoint]
- ブラックリスト情報を P2P 上で同期する。
- **P2P プロトコルによる通信**を想定しており、P2P ヘッダーによる署名検証が必須。

### ⚫︎ 権限
- **Public**: 全てのノードからの同期リクエストを許可。
"#;

#[utoipa::path(
    tag = TAG_P2P_STRICT,
    get,
    path = "/ca/blacklists/sync",
    summary = "ブラックリストを同期する (CA P2P Strict)",
    description = SYNC_CA_DESC,
    params(
        SyncBlacklistCaReq,
    ),
    responses(
        (status = 200, description = "Success", body = SyncBlacklistCaRes),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn sync_blacklists_ca(
    Extension(db): Extension<Arc<DbPools>>,
    axum::extract::Query(req): axum::extract::Query<SyncBlacklistCaReq>,
) -> Result<Json<SyncBlacklistCaRes>, ApiError> {
    req.validate().map_err(|e| ApiError::from_garde(e))?;
    let res = ca_blacklists_bl::sync_blacklists_ca(&db, req).await?;
    Ok(Json(res))
}
