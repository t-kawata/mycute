use crate::{
    mode::rt::client::secure_client::SecureClient,
    mode::rt::{
        rtbl::{identities_bl, node_identities_bl},
        rtreq::node_identities_req::{
            ApplyIdentityNodeReq, EntryIdentityNodeReq, SyncIdentityNodeReq,
        },
        rtres::{
            errs_res::ApiError,
            node_identities_res::{
                ApplyIdentityNodeRes, EntryIdentityNodeRes, GetPubKeyNodeRes, SyncIdentityNodeRes,
            },
        },
        rtutils::db_for_rt::DbPoolsExt,
    },
    stt_config::ConfigManager,
    utils::{
        db::DbPools,
        jwt::{JwtIDs, JwtRole, JwtUsr},
    },
};
use axum::{response::IntoResponse, Extension, Json};
use garde::Validate;
use std::sync::Arc;

const TAG: &str = "v1 Node Identity";

// ============================================================
// Entry (Node Side)
// ============================================================
const NODE_ENTRY_DESC: &str = r#"
### ⚫︎ 概要
- [Node Side Endpoint]
- Node が自身のアイデンティティを CA に登録するためのエントリーポイント。
- 内部で指定された CA ノード (`ca_base_url`) の `/v1/ca/identities/entry` を呼び出す。
- リクエストには `ca_base_url` が必須。

### ⚫︎ 権限
- **USR**: のみ使用可能。

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `ca_base_url` | string | required, url | CA のベースURL |
| `info` | object | optional | プロフィール情報 (JSON) |

### ⚫︎ Response
| KEY | TYPE | DESCRIPTION |
| --- | --- | --- |
| `success` | boolean | 登録成否 |
| `created_at` | string | 登録日時 |
"#;
#[utoipa::path(
    tag = TAG,
    post,
    security(("api_jwt_token" = [])),
    path = "/node/identities/entry",
    summary = "CA に対してアイデンティティ登録を行う (Node Side)。",
    description = NODE_ENTRY_DESC,
    request_body = EntryIdentityNodeReq,
    responses(
        (status = 200, description = "Success", body = EntryIdentityNodeRes),
        (status = 400, description = "Bad Request", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn entry_identity_node(
    ju: JwtUsr,
    _ids: JwtIDs,
    Extension(db): Extension<Arc<DbPools>>,
    Extension(config_manager): Extension<Arc<ConfigManager>>,
    Extension(client): Extension<Arc<SecureClient>>,
    Json(req): Json<EntryIdentityNodeReq>,
) -> Result<impl IntoResponse, ApiError> {
    ju.allow_roles(&[JwtRole::USR])?;
    req.validate().map_err(|e| ApiError::from_garde(e))?;
    let conn = db.get_rw_for_rt()?;
    let res = node_identities_bl::entry_identity_node(conn, req, config_manager, &client).await?;
    Ok(Json(res))
}

// ============================================================
// Apply (Node Side)
// ============================================================
const NODE_APPLY_DESC: &str = r#"
### ⚫︎ 概要
- [Node Side Endpoint]
- Node が CA に対してアイデンティティの適用（審査）を申請するためのエントリーポイント。
- 内部で指定された CA ノード の `/v1/ca/identities/apply` を呼び出し、リクエストをリレーする。

### ⚫︎ 権限
- **USR**: のみ使用可能。

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `ca_base_url` | string | required, url | CA のベースURL |
| `contact_email` | string | required, email | 連絡先メールアドレス |
| `info` | object | optional | プロフィール情報 (JSON) |
| `expire_seconds` | number | required | 希望する有効期間 (秒) |
"#;
#[utoipa::path(
    tag = TAG,
    post,
    security(("api_jwt_token" = [])),
    path = "/node/identities/apply",
    summary = "CA に対してアイデンティティ適用申請を行う (Node Side)。",
    description = NODE_APPLY_DESC,
    request_body = ApplyIdentityNodeReq,
    responses(
        (status = 200, description = "Success", body = ApplyIdentityNodeRes),
        (status = 400, description = "Bad Request", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn apply_identity_node(
    ju: JwtUsr,
    _ids: JwtIDs,
    Extension(config_manager): Extension<Arc<ConfigManager>>,
    Extension(client): Extension<Arc<SecureClient>>,
    Json(req): Json<ApplyIdentityNodeReq>,
) -> Result<Json<ApplyIdentityNodeRes>, ApiError> {
    ju.allow_roles(&[JwtRole::USR])?;
    req.validate().map_err(|e| ApiError::from_garde(e))?;
    let res = node_identities_bl::apply_identity_node(req, config_manager, &client).await?;
    Ok(Json(res))
}

// ============================================================
// Sync Identity with CA (Fetch & Persist)
// ============================================================
const SYNC_DESC: &str = r#"
### ⚫︎ 概要
- [Node Side Endpoint]
- CA (認証局) に直接問い合わせを行い、最新の審査状況を取得して自身のアイデンティティ（証明書など）を更新・保存する。
- **審査状況のハンドリング**:
    - CA から署名入りのデータ (200 OK) が返った場合、自動的に検証を行い、成功すればローカル DB に永続化する。
    - CA から審査中を示す応答 (**202 Accepted**) が返った場合、保存処理を行わずに正常終了する。

### ⚫︎ 権限
- **USR**: のみ使用可能。

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `ca_base_url` | string | required, url | CA のベースURL（例: http://ca-node.example.com） |
"#;
#[utoipa::path(
    tag = TAG,
    post,
    security(("api_jwt_token" = [])),
    path = "/node/identities/sync",
    summary = "CAから自分のアイデンティティを取得して保存する (Fetch & Persist)。",
    description = SYNC_DESC,
    request_body = SyncIdentityNodeReq,
    responses(
        (status = 200, description = "Success", body = SyncIdentityNodeRes),
        (status = 202, description = "Pending - CA verification is still in progress", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 422, description = "Validation Error", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn sync_identity_node(
    ju: JwtUsr,
    ids: JwtIDs,
    Extension(db): Extension<Arc<DbPools>>,
    Extension(config_manager): Extension<Arc<ConfigManager>>,
    Extension(client): Extension<Arc<SecureClient>>,
    Json(req): Json<SyncIdentityNodeReq>,
) -> Result<impl IntoResponse, ApiError> {
    ju.allow_roles(&[JwtRole::USR])?;
    req.validate().map_err(|e| ApiError::from_garde(e))?;
    let conn = db.get_rw_for_rt()?;
    let res = node_identities_bl::sync_identity_node(conn, &ju, &ids, req, config_manager, &client)
        .await?;
    Ok(Json(res))
}

// ============================================================
// Get My Public Key (Node Identity / Anchor Key)
// ============================================================
const GET_PUBKEY_DESC: &str = r#"
### ⚫︎ 概要
- [Node Side Endpoint]
- 自分自身（ノード）のアイデンティティを示す Ed448 公開鍵を、Hex 形式で返却する。
- **オーナーモード**: パスフレーズから復元された Anchor Public Key を返却する。
- **標準モード**: 初回起動時に自動生成・暗号化保存された Node Identity 公開鍵を返却する。

### ⚫︎ 権限
- **Public**: 公開鍵であり数学的な信頼の起点となるため、Public に公開されている。
"#;
#[utoipa::path(
    tag = TAG,
    get,
    path = "/node/identities/pubkey",
    summary = "自身の公開鍵（Node Identity / Anchor Key）を取得する。",
    description = GET_PUBKEY_DESC,
    responses(
        (status = 200, description = "Success", body = GetPubKeyNodeRes),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn get_pubkey_node(
    Extension(config_manager): Extension<Arc<ConfigManager>>,
) -> Result<Json<GetPubKeyNodeRes>, ApiError> {
    let pubkey = identities_bl::get_pubkey(config_manager).await?;
    Ok(Json(GetPubKeyNodeRes { public_key: pubkey }))
}
