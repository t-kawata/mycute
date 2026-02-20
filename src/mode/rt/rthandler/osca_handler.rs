use crate::mode::rt::{rtbl::osca_bl, rtres::errs_res::ApiError, rtres::osca_res::GetOscaUrlRes};
use axum::{Extension, Json};

const TAG: &str = "v1 OSCA";

// ============================================================
// Get OSCA URL
// ============================================================
const GET_OSCA_URL_DESC: &str = r#"
### ⚫︎ 概要
- モバイル端末等で OSCA (プロキシ用CA) 証明書をダウンロードするための URL を取得する
- サーバーのローカル IP アドレスと、SW サーバーのポート番号から URL を自動生成する
- 例: `http://192.168.1.10:8889/mycute-osca.pem` (PATH_OSCA_CERT_DOWNLOAD)

### ⚫︎ 権限
- 特になし（パブリック）
"#;
#[utoipa::path(
    tag = TAG,
    get,
    path = "/osca/url",
    summary = "OSCA証明書ダウンロード用URLを取得する。",
    description = GET_OSCA_URL_DESC,
    responses(
        (status = 200, description = "Success", body = GetOscaUrlRes),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn get_osca_url(
    Extension(sw_port): Extension<u16>,
) -> Result<Json<GetOscaUrlRes>, ApiError> {
    let res = osca_bl::get_osca_url(sw_port).await?;
    Ok(Json(res))
}
