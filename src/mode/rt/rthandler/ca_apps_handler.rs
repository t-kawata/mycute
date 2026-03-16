use crate::entities::apps;
use crate::mode::rt::rtutils::db_for_rt::DbPoolsExt;
use crate::mycute_settings::ConfigManager;
use crate::{
    constants::{ERR_APP_NOT_FOUND, ERR_DB, ST_INTERNAL_SERVER_ERROR, ST_NOT_FOUND},
    mode::rt::{
        client::secure_client::SecureClient,
        rtbl::ca_apps_bl,
        rtreq::ca_apps_req::{AdvertiseAppCaReq, DiscoverAppCaReq, VoteAppCaReq},
        rtres::{
            ca_apps_res::{AdvertiseAppCaRes, DiscoverAppCaRes, VoteAppCaRes},
            errs_res::ApiError,
        },
    },
    utils::{
        db::DbPools,
        jwt::{JwtRole, JwtUsr},
    },
    TAG_MACRO_P2P_STRICT,
};
use axum::{Extension, Json};
use garde::Validate;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use std::sync::Arc;

macro_rules! TAG_NAME {
    () => {
        "v1 CA Apps"
    };
}
const TAG_P2P_STRICT: &str = concat!(TAG_NAME!(), " ", TAG_MACRO_P2P_STRICT!());

// ============================================================
// Advertise & Discover (Skeleton)
// ============================================================
const ADVERTISE_DESC: &str = r#"
### ⚫︎ 概要
- アプリを CA ネットワークを介して P2P 上に公開 (広告) する (Skeleton)。

### ⚫︎ 権限
- **USR** のみ実行可能。

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `app_id` | string (UUID) | required | 広告対象のアプリ ID |

### ⚫︎ Response
| TYPE | DESCRIPTION |
| --- | --- |
| Object (`AdvertiseAppCaRes`) | 広告の結果 (ノード数など) |
"#;
#[utoipa::path(
    tag = TAG_P2P_STRICT,
    post,
    security(("api_jwt_token" = [])),
    path = "/ca/apps/advertise",
    summary = "アプリをCAネットワークに広告する (CA Skeleton)。",
    description = ADVERTISE_DESC,
    request_body = AdvertiseAppCaReq,
    responses(
        (status = 200, description = "Success", body = AdvertiseAppCaRes),
        (status = 500, description = "Not Implemented", body = ApiError)
    )
)]
pub async fn advertise_app_ca(
    ju: JwtUsr,
    Json(req): Json<AdvertiseAppCaReq>,
) -> Result<Json<AdvertiseAppCaRes>, ApiError> {
    ju.allow_roles(&[JwtRole::VDR, JwtRole::USR])?;
    if let Err(e) = req.validate() {
        return Err(ApiError::from_garde(e));
    }
    let res = ca_apps_bl::advertise_app_ca(req).await?;
    Ok(Json(res))
}

const DISCOVER_DESC: &str = r#"
### ⚫︎ 概要
- 指定したアプリを保持しているノードを CA に問い合わせる (Skeleton)。
- 複数のアプリ ID 指定、または名前による曖昧検索をサポート。

### ⚫︎ 権限
- **USR / VDR**: のみ実行可能。

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `app_ids` | array (string) | optional | 検索対象 hungry アプリ ID リスト |
| `query` | string | optional | 名前による曖昧検索クエリ |

### ⚫︎ Response
| TYPE | DESCRIPTION |
| --- | --- |
| Object (`DiscoverAppCaRes`) | 検索の結果 (アプリごとのノードリスト) |
"#;
#[utoipa::path(
    tag = TAG_P2P_STRICT,
    post,
    security(("api_jwt_token" = [])),
    path = "/ca/apps/discover",
    summary = "アプリを持っているノードをCAに問い合わせる (CA Skeleton)。",
    description = DISCOVER_DESC,
    request_body = DiscoverAppCaReq,
    responses(
        (status = 200, description = "Success", body = DiscoverAppCaRes),
        (status = 500, description = "Not Implemented", body = ApiError)
    )
)]
pub async fn discover_app_ca(
    ju: JwtUsr,
    Json(req): Json<DiscoverAppCaReq>,
) -> Result<Json<DiscoverAppCaRes>, ApiError> {
    ju.allow_roles(&[JwtRole::VDR, JwtRole::USR])?;
    if let Err(e) = req.validate() {
        return Err(ApiError::from_garde(e));
    }
    let res = ca_apps_bl::discover_app_ca(req).await?;
    Ok(Json(res))
}

// ============================================================
// Vote App
// ============================================================
const VOTE_DESC: &str = r#"
### ⚫︎ 概要
- アプリケーションに対して投票（支持/不支持）を行う。
- 署名（Identityによる署名）とチケット（予算証明）によって正当性を検証する。
- 1人1票（Quadratic Voting）。
- 投票内容は暗号化されて保存される。
- 変更時は上書き、0を指定すると投票取り消し。

### ⚫︎ 権限
- **Public**: JWTなしでアクセス可能。リクエスト内の署名によって権限を確認する。

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `node_pubkey` | string (Hex) | required | 投票者の公開鍵 |
| `app_id` | string (UUID) | required | 対象アプリID |
| `vote` | number | 0 ~ 15 | 投票値 (0は取り消し) |
| `timestamp` | string | required | リクエスト日時 |
| `tickets` | array | required | 予算を証明するチケットリスト |
| `signature` | string (Hex) | required | リクエスト全体の署名 |

### ⚫︎ Response
| TYPE | DESCRIPTION |
| --- | --- |
| Object (`VoteProvisionalCaRes`) | 投票の受理結果 (再計算は非同期、または手動) |
"#;
#[utoipa::path(
    tag = TAG_P2P_STRICT,
    post,
    path = "/ca/apps/vote",
    summary = "アプリに投票する (CA Trust)",
    description = VOTE_DESC,
    request_body = VoteAppCaReq,
    responses(
        (status = 200, description = "Success", body = VoteAppCaRes),
        (status = 403, description = "Forbidden (Low Level)", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn vote_app_ca(
    Extension(db): Extension<Arc<DbPools>>,
    Extension(config_manager): Extension<Arc<ConfigManager>>,
    Extension(client): Extension<Arc<SecureClient>>,
    Json(req): Json<VoteAppCaReq>,
) -> Result<Json<VoteAppCaRes>, ApiError> {
    if let Err(e) = req.validate() {
        return Err(ApiError::from_garde(e));
    }

    let conn = db.get_rw_for_rt()?;

    // UUID -> ID Lookup
    let uuid_val = uuid::Uuid::parse_str(&req.app_id)
        .unwrap_or_default()
        .as_bytes()
        .to_vec();

    let app_opt = apps::Entity::find()
        .filter(apps::Column::GlobalAppId.eq(uuid_val))
        .one(conn)
        .await
        .map_err(|e: sea_orm::DbErr| {
            ApiError::new_system(ST_INTERNAL_SERVER_ERROR, ERR_DB, e.to_string())
        })?;

    let app = app_opt
        .ok_or_else(|| ApiError::new_system(ST_NOT_FOUND, ERR_APP_NOT_FOUND, "App not found."))?;

    let res = ca_apps_bl::vote_app_ca(&db, &client, app.id, req, config_manager).await?;

    Ok(Json(res))
}
