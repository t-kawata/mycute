use crate::mode::rt::rtbl::mycute_bl;
use crate::mode::rt::rtres::errs_res::ApiError;
use crate::mode::rt::rtres::mycute_res::{MyCuteHomeDirRes, MyCuteVersionRes};

use axum::Json;

const TAG: &str = "v1 MYCUTE";

// ============================================================
// Version
// ============================================================
const VERSION_DESC: &str = r#"
### ⚫︎ 概要
- MYCUTE の現在のバージョンを取得します。

### ⚫︎ 権限
- パブリック（認証不要）。誰でもアクセス可能です。

### ⚫︎ Response
| KEY | TYPE | DESCRIPTION |
| --- | --- | --- |
| `version` | string | MYCUTE のバージョン (例: v0.1.0) |
"#;

#[utoipa::path(
    tag = TAG,
    get,
    path = "/mycute/version",
    summary = "MYCUTE のバージョンを取得する。",
    description = VERSION_DESC,
    responses(
        (status = 200, description = "Success", body = MyCuteVersionRes),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn get_mycute_version() -> Result<Json<MyCuteVersionRes>, ApiError> {
    log::debug!("<MyCute> Fetching version.");
    let res = mycute_bl::get_version().await;
    Ok(Json(res))
}

// ============================================================
// Home Directory
// ============================================================
const HOME_DIR_DESC: &str = r#"
### ⚫︎ 概要
- MYCUTE が使用しているホームディレクトリの絶対パスを取得します。

### ⚫︎ 権限
- パブリック（認証不要）。誰でもアクセス可能です。

### ⚫︎ Response
| KEY | TYPE | DESCRIPTION |
| --- | --- | --- |
| `home_dir` | string | ホームディレクトリの絶対パス |
"#;

#[utoipa::path(
    tag = TAG,
    get,
    path = "/mycute/home_dir",
    summary = "MYCUTE のホームディレクトリを取得する。",
    description = HOME_DIR_DESC,
    responses(
        (status = 200, description = "Success", body = MyCuteHomeDirRes),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn get_mycute_home_dir() -> Result<Json<MyCuteHomeDirRes>, ApiError> {
    log::debug!("<MyCute> Fetching home directory.");
    let res = mycute_bl::get_home_dir().await;
    Ok(Json(res))
}
