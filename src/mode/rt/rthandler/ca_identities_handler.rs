use crate::{
    mode::rt::{
        rtbl::ca_identities_bl,
        rtreq::ca_identities_req::{
            ApplyIdentityCaReq, EntryIdentityCaReq, SearchIdentitiesCaReq, VerifyIdentityCaReq,
        },
        rtres::{
            ca_identities_res::{
                ApplyIdentityCaRes, DeleteIdentityCaRes, EntryIdentityCaRes, GetIdentityCaRes,
                SearchIdentitiesCaRes, SyncIdentityCaRes, VerifyIdentityCaRes,
            },
            errs_res::ApiError,
        },
        rtutils::db_for_rt::DbPoolsExt,
    },
    stt_config::ConfigManager,
    utils::{
        db::DbPools,
        jwt::{JwtIDs, JwtRole, JwtUsr},
    },
    TAG_MACRO_P2P_STRICT,
};
use axum::{extract::Path, Extension, Json};
use garde::Validate;
use std::sync::Arc;

macro_rules! TAG_NAME {
    () => {
        "v1 CA Identity"
    };
}
const TAG: &str = TAG_NAME!();
const TAG_P2P_STRICT: &str = concat!(TAG_NAME!(), " ", TAG_MACRO_P2P_STRICT!());

// ============================================================
// Search (CA Side)
// ============================================================
const SEARCH_DESC: &str = r#"
### ⚫︎ 概要
- [CA Side Endpoint]
- アイデンティティを検索する。
- 複数の条件やページングをサポートしており、主に管理（VDR以上）用途で使用される。

### ⚫︎ 権限
- **USR** のみが使用可能。（CA の USR がスタッフとして扱われるものとする）

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `public_key` | string (hex) | optional | 公開鍵による前方一致検索 |
| `include_isolated` | boolean | optional (default: false) | 未所属（apx_id=0, vdr_id=0）の候補者レコードを含めるか |
| `limit` | number | required (max: 25) | 取得件数 |
| `offset` | number | required | スキップ件数 |
"#;
#[utoipa::path(
    tag = TAG,
    post,
    security(("api_jwt_token" = [])),
    path = "/ca/identities/search",
    summary = "アイデンティティを検索する (CA Side)。",
    description = SEARCH_DESC,
    request_body = SearchIdentitiesCaReq,
    responses(
        (status = 200, description = "Success", body = SearchIdentitiesCaRes),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 422, description = "Validation Error", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn search_identities_ca(
    ju: JwtUsr,
    ids: JwtIDs,
    Extension(db): Extension<Arc<DbPools>>,
    Extension(config_manager): Extension<Arc<ConfigManager>>,
    Json(req): Json<SearchIdentitiesCaReq>,
) -> Result<Json<SearchIdentitiesCaRes>, ApiError> {
    ju.allow_roles(&[JwtRole::USR])?;
    req.validate().map_err(|e| ApiError::from_garde(e))?;
    let conn = db.get_ro_for_rt()?;
    let res = ca_identities_bl::search_identities(conn, &ju, &ids, req, config_manager).await?;
    Ok(Json(res))
}

// ============================================================
// Get (CA Side)
// ============================================================
const GET_DESC: &str = r#"
### ⚫︎ 概要
- [CA Side Endpoint]
- 指定された公開鍵 (`pubkey`) に紐づくアイデンティティの詳細情報を取得する。

### ⚫︎ 権限
- **USR** のみが使用可能。（CA の USR がスタッフとして扱われるものとする）

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `pubkey` | string (hex/path) | required | 対象の公開鍵 |
"#;
#[utoipa::path(
    tag = TAG,
    get,
    security(("api_jwt_token" = [])),
    path = "/ca/identities/get/{pubkey}",
    summary = "アイデンティティ情報を取得する (CA Side)。",
    description = GET_DESC,
    params(
        ("pubkey" = String, Path),
    ),
    responses(
        (status = 200, description = "Success", body = GetIdentityCaRes),
        (status = 404, description = "Not Found", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn get_identity_ca(
    ju: JwtUsr,
    ids: JwtIDs,
    Extension(db): Extension<Arc<DbPools>>,
    Extension(config_manager): Extension<Arc<ConfigManager>>,
    Path(pubkey): Path<String>,
) -> Result<Json<GetIdentityCaRes>, ApiError> {
    ju.allow_roles(&[JwtRole::USR])?;
    let conn = db.get_ro_for_rt()?;
    let res = ca_identities_bl::get_identity(conn, &ids, pubkey, config_manager).await?;
    Ok(Json(res))
}

// ============================================================
// Entry (Register @ CA)
// ============================================================
const ENTRY_DESC: &str = r#"
### ⚫︎ 概要
- [CA Side Endpoint]
- 新しいアイデンティティ（Ed448 公開鍵）を CA システムにエントリーする。
- この時点では「候補者（Candidate）」として登録され、署名は付与されない。
- Node が初回起動時に自身を CA に知らしめるために使用される。

### ⚫︎ 権限
- **Public**: 全ての Node からのアクセスを許可。

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `public_key` | string (hex) | required (114 chars) | 登録する Ed448 公開鍵 |
| `info` | object (json) | optional | ノードのプロフィール情報（名称、メタデータ等） |

### ⚫︎ Response
| KEY | TYPE | DESCRIPTION |
| --- | --- | --- |
| `success` | boolean | 登録成否 |
| `created_at` | string | 登録日時 |
"#;
#[utoipa::path(
    tag = TAG_P2P_STRICT,
    post,
    path = "/ca/identities/entry",
    summary = "アイデンティティを CA にエントリーする (Public)。",
    description = ENTRY_DESC,
    request_body = EntryIdentityCaReq,
    responses(
        (status = 200, description = "Success", body = EntryIdentityCaRes),
        (status = 409, description = "Conflict", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn entry_identity_ca(
    Extension(db): Extension<Arc<DbPools>>,
    Extension(config_manager): Extension<Arc<ConfigManager>>,
    Json(req): Json<EntryIdentityCaReq>,
) -> Result<Json<EntryIdentityCaRes>, ApiError> {
    req.validate().map_err(|e| ApiError::from_garde(e))?;
    let conn = db.get_rw_for_rt()?;
    let res = ca_identities_bl::entry_identity_ca(conn, req, config_manager).await?;
    Ok(Json(res))
}

// ============================================================
// Apply (Public @ CA)
// ============================================================
const APPLY_DESC: &str = r#"
### ⚫︎ 概要
- [CA Side Endpoint]
- 登録済みのアイデンティティに対して、正式な検証（審査）を申請する。
- 申請後、CA 管理者による審査が行われる。

### ⚫︎ 権限
- **Public**: エントリー済みの Node 自身が申請を行う。

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `public_key` | string (hex) | required (114 chars) | 申請対象の公開鍵 |
| `contact_email` | string | required (email) | 連絡先メールアドレス |
| `info` | object (json) | optional | 補足情報 |
| `expire_seconds` | number | required | 希望する有効期間（秒） |
"#;
#[utoipa::path(
    tag = TAG_P2P_STRICT,
    post,
    path = "/ca/identities/apply",
    summary = "アイデンティティの検証を申請する (Public)。",
    description = APPLY_DESC,
    request_body = ApplyIdentityCaReq,
    responses(
        (status = 200, description = "Success", body = ApplyIdentityCaRes),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn apply_identity_ca(
    Extension(db): Extension<Arc<DbPools>>,
    Extension(config_manager): Extension<Arc<ConfigManager>>,
    Json(req): Json<ApplyIdentityCaReq>,
) -> Result<Json<ApplyIdentityCaRes>, ApiError> {
    req.validate().map_err(|e| ApiError::from_garde(e))?;
    let conn = db.get_rw_for_rt()?;
    let res = ca_identities_bl::apply_identity_ca(conn, req, config_manager).await?;
    Ok(Json(res))
}

// ============================================================
// Verify (CA Side)
// ============================================================
const VERIFY_DESC: &str = r#"
### ⚫︎ 概要
- [CA Side Endpoint]
- アイデンティティを検証（承認）し、CA による証明書を発行する。
- 承認されると Node は、当該CA内において正式なアイデンティティとして振る舞うことが可能になる。

### ⚫︎ 権限
- **USR** のみが使用可能。（CA の USR がスタッフとして扱われるものとする）

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `pubkey` | string (hex/path) | required | 対象アイデンティティの公開鍵 |
| `signature` | string (hex) | required (228 chars) | CA の秘密鍵による対象公開鍵への署名 |
"#;
#[utoipa::path(
    tag = TAG,
    put,
    security(("api_jwt_token" = [])),
    path = "/ca/identities/verify/{pubkey}",
    summary = "アイデンティティを検証（承認）し、CA による証明書を発行する (CA Side)。",
    description = VERIFY_DESC,
    params(
        ("pubkey" = String, Path),
    ),
    request_body = VerifyIdentityCaReq,
    responses(
        (status = 200, description = "Success", body = VerifyIdentityCaRes),
        (status = 404, description = "Not Found", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn verify_identity_ca(
    ju: JwtUsr,
    ids: JwtIDs,
    Extension(db): Extension<Arc<DbPools>>,
    Extension(config_manager): Extension<std::sync::Arc<ConfigManager>>,
    Path(pubkey): Path<String>,
    Json(req): Json<VerifyIdentityCaReq>,
) -> Result<Json<VerifyIdentityCaRes>, ApiError> {
    ju.allow_roles(&[JwtRole::VDR, JwtRole::USR])?;
    req.validate().map_err(|e| ApiError::from_garde(e))?;
    let conn = db.get_rw_for_rt()?;
    let res = ca_identities_bl::verify_identity_ca(conn, config_manager, &ids, pubkey, req).await?;
    Ok(Json(res))
}

// ============================================================
// Sync (Public @ CA)
// ============================================================
const SYNC_DESC: &str = r#"
### ⚫︎ 概要
- [CA Side Endpoint]
- 検証済みアイデンティティ（証明書チェーン一式）を取得し同期する。
- 外部ノードが、対象ノードの正当性を検証するための「信頼の証」として使用する。

### ⚫︎ 審査状況とレスポンス
- **200 OK**: 検証（審査）が完了しており、署名入りのデータが返却される。
- **202 Accepted (Pending)**: 申請は受理されているが、審査が未完了（署名なし）。この場合、署名やトークンは含まれない。

### ⚫︎ 権限
- **Public**: 第三者による検証を可能にするため、完全に公開されている。

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `pubkey` | string (hex/path) | required | アイデンティティ特定のための公開鍵 |
"#;
#[utoipa::path(
    tag = TAG_P2P_STRICT,
    get,
    path = "/ca/identities/sync/{pubkey}",
    summary = "検証済みアイデンティティを取得し同期する (Public)。",
    description = SYNC_DESC,
    params(
        ("pubkey" = String, Path),
    ),
    responses(
        (status = 200, description = "Success", body = SyncIdentityCaRes),
        (status = 202, description = "Pending - Verification is still in progress", body = ApiError),
        (status = 404, description = "Not Found", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn sync_identity_ca(
    Extension(db): Extension<Arc<DbPools>>,
    Extension(config_manager): Extension<Arc<ConfigManager>>,
    Path(pubkey): Path<String>,
) -> Result<Json<SyncIdentityCaRes>, ApiError> {
    let conn = db.get_ro_for_rt()?;
    let res = ca_identities_bl::sync_identity_ca(conn, config_manager, pubkey).await?;
    Ok(Json(res))
}

// ============================================================
// Delete (CA Side)
// ============================================================
const DELETE_DESC: &str = r#"
### ⚫︎ 概要
- [CA Side Endpoint]
- 指定されたアイデンティティをシステムから削除（無効化）する。

### ⚫︎ 権限
- **USR** のみが使用可能。（CA の USR がスタッフとして扱われるものとする）

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `pubkey` | string (hex/path) | required | 削除対象アイデンティティの公開鍵 |
"#;
#[utoipa::path(
    tag = TAG,
    delete,
    security(("api_jwt_token" = [])),
    path = "/ca/identities/delete/{pubkey}",
    summary = "アイデンティティを削除する (CA Side)。",
    description = DELETE_DESC,
    params(
        ("pubkey" = String, Path),
    ),
    responses(
        (status = 200, description = "Success", body = DeleteIdentityCaRes),
        (status = 404, description = "Not Found", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn delete_identity_ca(
    ju: JwtUsr,
    ids: JwtIDs,
    Extension(db): Extension<Arc<DbPools>>,
    Extension(config_manager): Extension<Arc<ConfigManager>>,
    Path(pubkey): Path<String>,
) -> Result<Json<DeleteIdentityCaRes>, ApiError> {
    ju.allow_roles(&[JwtRole::USR])?;
    let conn = db.get_rw_for_rt()?;
    let res = ca_identities_bl::delete_identity_ca(conn, &ju, &ids, pubkey, config_manager).await?;
    Ok(Json(res))
}
