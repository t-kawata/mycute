use std::sync::Arc;
use axum::{Extension, Json, extract::{Path}};
use garde::Validate;
use crate::{
    mode::rt::{
        rtreq::forums_req::{CreateForumReq, SearchForumsReq, UpdateForumReq},
        rtres::{errs_res::ApiError, forums_res::{SearchForumsRes, GetForumRes, CreateForumRes, UpdateForumRes, DeleteForumRes}},
        rtbl::forums_bl,
        rtutils::db_for_rt::DbPoolsExt,
    },
    utils::{db::DbPools, jwt::{JwtUsr, JwtRole}},
};

const TAG: &str = "v1 Forum";

const SEARCH_DESC: &str = r#"
### ⚫︎ 概要
- 全てのフォーラムを検索する。

### ⚫︎ 権限
- パブリック（誰でもアクセス可能）

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `keyword` | string | optional | 検索キーワード（名称または説明分に含まれるもの） |
| `limit` | number | optional, 1-100 | 取得件数（デフォルト: 20） |
| `offset` | number | optional | 開始位置 |

### ⚫︎ Response
| KEY | TYPE | DESCRIPTION |
| --- | --- | --- |
| `total` | number | ヒットした総件数 |
| `forums` | array | フォーラムのリスト |
| `forums.id` | string | フォーラムID (UUID) |
| `forums.name` | string | フォーラム名 |
| `forums.description` | string | フォーラムの説明 |
| `forums.initial_balance` | number | 初期残高 |
| `forums.created_at` | string | 作成日時 |
| `forums.updated_at` | string | 更新日時 |
"#;
#[utoipa::path(
    tag = TAG,
    post,
    path = "/forums/search",
    summary = "フォーラムを検索する",
    description = SEARCH_DESC,
    request_body = SearchForumsReq,
    responses(
        (status = 200, description = "Success", body = SearchForumsRes),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn search_forums(
    Extension(db): Extension<Arc<DbPools>>,
    Json(req): Json<SearchForumsReq>,
) -> Result<Json<SearchForumsRes>, ApiError> {
    let conn = db.get_ro_for_rt()?;
    let res = forums_bl::search_forums(conn, req).await?;
    Ok(Json(res))
}

const GET_DESC: &str = r#"
### ⚫︎ 概要
- 指定されたIDのフォーラムを取得する。

### ⚫︎ 権限
- パブリック（誰でもアクセス可能）

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `id` | string | required, uuid | フォーラムID |

### ⚫︎ Response
| KEY | TYPE | DESCRIPTION |
| --- | --- | --- |
| `id` | string | フォーラムID (UUID) |
| `name` | string | フォーラム名 |
| `description` | string | フォーラムの説明 |
| `initial_balance` | number | 初期残高 |
| `created_at` | string | 作成日時 |
| `updated_at` | string | 更新日時 |
"#;
#[utoipa::path(
    tag = TAG,
    get,
    path = "/forums/get/{id}",
    summary = "フォーラムを取得する",
    description = GET_DESC,
    params(
        ("id" = String, Path, description = "Forum UUID")
    ),
    responses(
        (status = 200, description = "Success", body = GetForumRes),
        (status = 404, description = "Not Found", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn get_forum(
    Path(id): Path<String>,
    Extension(db): Extension<Arc<DbPools>>,
) -> Result<Json<GetForumRes>, ApiError> {
    let conn = db.get_ro_for_rt()?;
    let res = forums_bl::get_forum(conn, id).await?;
    Ok(Json(res))
}

const CREATE_DESC: &str = r#"
### ⚫︎ 概要
- 新しいフォーラムを作成する。

### ⚫︎ 権限
- USR ロールのみが可能

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `name` | string | required, 1-255 | フォーラム名 |
| `description` | string | required, 1-1000 | フォーラムの説明 |
| `initial_balance` | number | required, 2-30 | 初期残高 |

### ⚫︎ Response
| KEY | TYPE | DESCRIPTION |
| --- | --- | --- |
| `id` | string | フォーラムID (UUID) |
| `name` | string | フォーラム名 |
| `description` | string | フォーラムの説明 |
| `initial_balance` | number | 初期残高 |
| `created_at` | string | 作成日時 |
| `updated_at` | string | 更新日時 |
"#;
#[utoipa::path(
    tag = TAG,
    post,
    security(("api_jwt_token" = [])),
    path = "/forums/create",
    summary = "フォーラムを作成する",
    description = CREATE_DESC,
    request_body = CreateForumReq,
    responses(
        (status = 200, description = "Success", body = CreateForumRes),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 400, description = "Bad Request", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn create_forum(
    ju: JwtUsr,
    Extension(db): Extension<Arc<DbPools>>,
    Json(req): Json<CreateForumReq>,
) -> Result<Json<CreateForumRes>, ApiError> {
    ju.allow_roles(&[JwtRole::USR])?;
    req.validate().map_err(|e| ApiError::from_garde(e))?;

    let conn = db.get_rw_for_rt()?;
    let res = forums_bl::create_forum(conn, req).await?;
    Ok(Json(res))
}

const UPDATE_DESC: &str = r#"
### ⚫︎ 概要
- フォーラムの情報を更新する。

### ⚫︎ 権限
- USR ロールのみが可能

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `id` | string | required, uuid | フォーラムID |
| `name` | string | optional, 1-255 | フォーラム名 |
| `description` | string | optional | フォーラムの説明 |

### ⚫︎ Response
| KEY | TYPE | DESCRIPTION |
| --- | --- | --- |
| `id` | string | フォーラムID (UUID) |
| `name` | string | フォーラム名 |
| `description` | string | フォーラムの説明 |
| `initial_balance` | number | 初期残高 |
| `created_at` | string | 作成日時 |
| `updated_at` | string | 更新日時 |
"#;
#[utoipa::path(
    tag = TAG,
    put,
    security(("api_jwt_token" = [])),
    path = "/forums/update/{id}",
    summary = "フォーラムを更新する",
    description = UPDATE_DESC,
    request_body = UpdateForumReq,
    params(
        ("id" = String, Path, description = "Forum UUID")
    ),
    responses(
        (status = 200, description = "Success", body = UpdateForumRes),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 400, description = "Bad Request", body = ApiError),
        (status = 404, description = "Not Found", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn update_forum(
    ju: JwtUsr,
    Path(id): Path<String>,
    Extension(db): Extension<Arc<DbPools>>,
    Json(req): Json<UpdateForumReq>,
) -> Result<Json<UpdateForumRes>, ApiError> {
    ju.allow_roles(&[JwtRole::USR])?;
    req.validate().map_err(|e| ApiError::from_garde(e))?;

    let conn = db.get_rw_for_rt()?;
    let res = forums_bl::update_forum(conn, id, req).await?;
    Ok(Json(res))
}

const DELETE_DESC: &str = r#"
### ⚫︎ 概要
- フォーラムを削除する。

### ⚫︎ 権限
- USR ロールのみが可能

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `id` | string | required, uuid | フォーラムID |

### ⚫︎ Response
| KEY | TYPE | DESCRIPTION |
| --- | --- | --- |
| `id` | string | 削除されたフォーラムID |
"#;
#[utoipa::path(
    tag = TAG,
    delete,
    security(("api_jwt_token" = [])),
    path = "/forums/delete/{id}",
    summary = "フォーラムを削除する",
    description = DELETE_DESC,
    params(
        ("id" = String, Path, description = "Forum UUID")
    ),
    responses(
        (status = 200, description = "Success", body = DeleteForumRes),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 404, description = "Not Found", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn delete_forum(
    ju: JwtUsr,
    Path(id): Path<String>,
    Extension(db): Extension<Arc<DbPools>>,
) -> Result<Json<DeleteForumRes>, ApiError> {
    ju.allow_roles(&[JwtRole::USR])?;

    let conn = db.get_rw_for_rt()?;
    let res = forums_bl::delete_forum(conn, id).await?;
    Ok(Json(res))
}
