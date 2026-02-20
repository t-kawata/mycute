use crate::constants::ST_INTERNAL_SERVER_ERROR;
use crate::entities::bds;
use crate::mode::rt::rterr::rterr;
use crate::mode::rt::rtreq::bds_req::{CheckBdHashReq, CreateBdHashReq};
use crate::mode::rt::rtres::bds_res::{CheckBdHashRes, CreateBdHashRes};
use crate::mode::rt::rtres::errs_res::ApiError;
use crate::mode::rt::rtutils::db_for_rt::DbPoolsExt;
use crate::utils::bd::is_valid_bd;
use crate::utils::crypto::get_hash_with_cost;
use crate::utils::db::{str_to_datetime, DbPools};
use anyhow::Result;
use axum::{extract::Query, Extension, Json};
use garde::Validate;
use sea_orm::{
    ActiveModelTrait,
    ActiveValue::{self, Set},
};
use std::sync::Arc;
use tokio::task;

const TAG: &str = "v1 BD";

// ============================================================
// BDハッシュを作成
// ============================================================
const CREATE_BD_HASH_DESC: &str = r#"
### ⚫︎ 概要
- BDハッシュを作成する。

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `bd` | string | required | BD文字列 |
"#;
#[utoipa::path(
    tag = TAG,
    get,
    path = "/bds/create",
    summary = "BDハッシュを作成する。",
    description = CREATE_BD_HASH_DESC,
    params(CreateBdHashReq),
    responses(
        (status = 200, description = "Success", body = CreateBdHashRes),
        (status = 400, description = "Bad Request", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn create_bd_hash(
    Query(req): Query<CreateBdHashReq>,
    Extension(db): Extension<Arc<DbPools>>,
) -> Result<Json<CreateBdHashRes>, ApiError> {
    // --------------------------------
    // バリデーション
    // --------------------------------
    req.validate().map_err(ApiError::from_garde)?;
    log::debug!("Generating BD hash for '{}'", req.bd);
    // --------------------------------
    // BDハッシュの生成
    // --------------------------------
    // 所有権をスレッドに渡すためクローン
    let bd = req.bd.clone();
    // CPU負荷の高い処理を専用スレッドに投げる
    let hash = task::spawn_blocking(move || -> Result<String> { get_hash_with_cost(&bd, 10) })
        .await
        .map_err(|e| {
            ApiError::new_system(
                ST_INTERNAL_SERVER_ERROR,
                rterr::ERR_UNEXPECTED,
                format!("Join error: {}", e),
            )
        })?
        .map_err(|e| {
            ApiError::new_system(
                ST_INTERNAL_SERVER_ERROR,
                rterr::ERR_UNEXPECTED,
                format!("Failed to generate BD hash: {}", e),
            )
        })?;
    // --------------------------------
    // DBに保存
    // --------------------------------
    let conn = db.get_rw_for_rt()?;
    let bgn_at_str = "2026-01-01T00:00:00";
    let end_at_str = "2100-12-31T23:59:59";
    log::debug!("Inserting BD hash into DB...");
    let new_bds = bds::ActiveModel {
        hash: Set(hash.clone()),
        bgn_at: str_to_datetime(bgn_at_str).unwrap_or(ActiveValue::NotSet),
        end_at: str_to_datetime(end_at_str).unwrap_or(ActiveValue::NotSet),
        ..Default::default()
    };
    let result = new_bds.insert(conn).await;
    match result {
        Ok(_) => {
            log::debug!("BD hash saved successfully.");
        }
        Err(e) => {
            return Err(ApiError::new_system(
                ST_INTERNAL_SERVER_ERROR,
                rterr::ERR_DATABASE,
                format!("Failed to save BD hash: {}", e),
            ));
        }
    }
    // --------------------------------
    // 最終レスポンス
    // --------------------------------
    Ok(Json(CreateBdHashRes { hash }))
}

// ============================================================
// BDハッシュを検証
// ============================================================
const CHECK_BD_HASH_DESC: &str = r#"
### ⚫︎ 概要
- BDハッシュを検証する。

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `bd` | string | required | BD文字列 |
"#;
#[utoipa::path(
    tag = TAG,
    get,
    path = "/bds/check",
    summary = "BDハッシュを検証する。",
    description = CHECK_BD_HASH_DESC,
    params(CheckBdHashReq),
    responses(
        (status = 200, description = "Success", body = CheckBdHashRes),
        (status = 400, description = "Bad Request", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn check_bd_hash(
    Query(req): Query<CheckBdHashReq>,
    Extension(db): Extension<Arc<DbPools>>,
) -> Result<Json<CheckBdHashRes>, ApiError> {
    // --------------------------------
    // バリデーション
    // --------------------------------
    req.validate().map_err(ApiError::from_garde)?;
    log::debug!("Checking BD hash for '{}'", req.bd);
    // --------------------------------
    // BDの検証
    // --------------------------------
    let conn = db.get_ro_for_rt()?;
    let is_valid = is_valid_bd(conn, req.bd.clone()).await.map_err(|e| {
        ApiError::new_system(
            ST_INTERNAL_SERVER_ERROR,
            rterr::ERR_DATABASE,
            format!("BD verification error: {}", e),
        )
    })?;
    // --------------------------------
    // 最終レスポンス
    // --------------------------------
    Ok(Json(CheckBdHashRes { ok: is_valid }))
}
