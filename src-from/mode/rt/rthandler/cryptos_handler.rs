use std::sync::Arc;
use axum::{Extension, Json, extract::{Path, Query}};
use garde::Validate;
use crate::{
    mode::rt::{
        rtreq::cryptos_req::{EncryptReq, DecryptReq},
        rtres::{errs_res::ApiError, cryptos_res::{EncryptRes, DecryptRes, CreateVdrTokenRes, GetVdrTokenRes}},
        rtbl::cryptos_bl,
        rtutils::db_for_rt::DbPoolsExt,
    },
    utils::{db::DbPools, jwt::{JwtUsr, JwtIDs, JwtConfig}}
};

const TAG: &str = "v1 Crypto";

// ============================================================
// Encrypt
// ============================================================
const ENCRYPT_DESC: &str = r#"
### ⚫︎ 概要
- 指定された文字列を AES-256-GCM で暗号化する。
- 暗号化には環境変数 `RT_CRYPTO_KEY` が使用される。

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `text` | string | required, max=10000 | 暗号化する文字列 |

### ⚫︎ 権限
- 特になし（パブリック）
"#;
#[utoipa::path(
    tag = TAG,
    get,
    path = "/crypto/enc",
    summary = "文字列を暗号化する。",
    description = ENCRYPT_DESC,
    params(EncryptReq),
    responses(
        (status = 200, description = "Success", body = EncryptRes),
        (status = 400, description = "Bad Request", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn encrypt_handler(
    Extension(jwt_config): Extension<Arc<JwtConfig>>,
    Query(req): Query<EncryptReq>,
) -> Result<Json<EncryptRes>, ApiError> {
    req.validate().map_err(ApiError::from_garde)?;
    let res = cryptos_bl::encrypt_text(&jwt_config.crypto_key, req.text).await?;
    Ok(Json(res))
}

// ============================================================
// Decrypt
// ============================================================
const DECRYPT_DESC: &str = r#"
### ⚫︎ 概要
- 指定された暗号化文字列を AES-256-GCM で復号化する。
- 復号化には環境変数 `RT_CRYPTO_KEY` が使用される。

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `text` | string | required, max=10000 | 復号化する文字列（16進エンコード） |

### ⚫︎ 権限
- 特になし（パブリック）
"#;
#[utoipa::path(
    tag = TAG,
    get,
    path = "/crypto/dec",
    summary = "文字列を復号化する。",
    description = DECRYPT_DESC,
    params(DecryptReq),
    responses(
        (status = 200, description = "Success", body = DecryptRes),
        (status = 400, description = "Bad Request", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn decrypt_handler(
    Extension(jwt_config): Extension<Arc<JwtConfig>>,
    Query(req): Query<DecryptReq>,
) -> Result<Json<DecryptRes>, ApiError> {
    req.validate().map_err(ApiError::from_garde)?;
    let res = cryptos_bl::decrypt_text(&jwt_config.crypto_key, req.text).await?;
    Ok(Json(res))
}

// ============================================================
// Create VDR Token
// ============================================================
const CREATE_VDR_TOKEN_DESC: &str = r#"
### ⚫︎ 概要
- VDR用の100年間の有効期限を持つ JWT トークンを生成し、暗号化してデータベースに保存する。
- 既存のキーがある場合は、値を更新（upsert）する。

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `key` | string | required, length=50, regex=^[a-zA-Z0-9-_]{50}$ | 検索用のユニークキー |
| `apx_id` | number | required | 所属 APX ID |
| `vdr_id` | number | required | 対象 VDR ID |

### ⚫︎ 権限
- APX: 自分の配下の VDR に対してのみトークンを生成可能

### ⚫︎ 注意点
- `key` は半角英数字、ハイフン、アンダーバーのみの50文字である必要がある。
"#;
#[utoipa::path(
    tag = TAG,
    put,
    security(("api_jwt_token" = [])),
    path = "/crypto/vdr/{key}/{apx_id}/{vdr_id}",
    summary = "VDR用の100年トークンを生成・暗号化して保存する。",
    description = CREATE_VDR_TOKEN_DESC,
    params(
        ("key" = String, Path, description = "半角英数字とハイフンとアンダーバーのみの50文字"),
        ("apx_id" = u32, Path),
        ("vdr_id" = u32, Path),
    ),
    responses(
        (status = 200, description = "Success", body = CreateVdrTokenRes),
        (status = 400, description = "Bad Request", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 403, description = "Forbidden", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn create_vdr_token_handler(
    ju: JwtUsr,
    ids: JwtIDs,
    Extension(db): Extension<Arc<DbPools>>,
    Extension(jwt_config): Extension<Arc<JwtConfig>>,
    Path((key, apx_id, vdr_id)): Path<(String, u32, u32)>,
) -> Result<Json<CreateVdrTokenRes>, ApiError> {
    let conn = db.get_rw_for_rt()?;
    let res = cryptos_bl::create_vdr_token(conn, &ju, &ids, &jwt_config.skey, &jwt_config.crypto_key, key, apx_id, vdr_id).await?;
    Ok(Json(res))
}

// ============================================================
// Get VDR Token
// ============================================================
const GET_VDR_TOKEN_DESC: &str = r#"
### ⚫︎ 概要
- キーを指定して、保存されている暗号化された VDR トークンを取得する。

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `key` | string | required, length=50, regex=^[a-zA-Z0-9-_]{50}$ | 検索用のユニークキー |

### ⚫︎ 権限
- 特になし（パブリック）
"#;
#[utoipa::path(
    tag = TAG,
    get,
    path = "/crypto/vdr/{key}",
    summary = "VDR用の100年トークンを取得する。",
    description = GET_VDR_TOKEN_DESC,
    params(
        ("key" = String, Path, description = "半角英数字とハイフンとアンダーバーのみの50文字"),
    ),
    responses(
        (status = 200, description = "Success", body = GetVdrTokenRes),
        (status = 400, description = "Bad Request", body = ApiError),
        (status = 404, description = "Not Found", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn get_vdr_token_handler(
    Extension(db): Extension<Arc<DbPools>>,
    Path(key): Path<String>,
) -> Result<Json<GetVdrTokenRes>, ApiError> {
    let conn = db.get_ro_for_rt()?;
    let res = cryptos_bl::get_vdr_token(conn, key).await?;
    Ok(Json(res))
}
