use crate::{
    mode::rt::{
        rtbl::replace_items_bl,
        rterr::rterr,
        rtreq::replace_items_req::{
            CreateReplaceItemReq, SearchReplaceItemsReq, UpdateReplaceItemReq,
        },
        rtres::{
            errs_res::ApiError,
            replace_items_res::{
                CreateReplaceItemRes, SearchReplaceItemsRes, UpdateReplaceItemRes,
            },
        },
        rtutils::db_for_rt::DbPoolsExt,
    },
    utils::{
        db::DbPools,
        jwt::{JwtIDs, JwtRole, JwtUsr},
    },
};
use axum::{extract::Path, http::StatusCode, Extension, Json};
use garde::Validate;
use std::sync::Arc;

const TAG: &str = "v1 Replace Item";

const SEARCH_DESC: &str = r#"
### ⚫︎ 概要
- 指定された辞書セットに含まれる置換アイテムを検索します。

### ⚫︎ 権限
- USR のみが使用可能

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `replace_id` | string | UUID, required | 対象の置換辞書セットID |
| `keyword` | string | - | 置換前/後テキストの部分一致キーワード |
| `limit` | number | gte=1, lte=200, default=100 | 最大取得件数 |
| `offset` | number | gte=0, default=0 | オフセット位置 |

### ⚫︎ Response
| KEY | TYPE | DESCRIPTION |
| --- | --- | --- |
| `list` | array | 置換アイテムの詳細リスト |
| `total` | number | 総件数 |
"#;
#[utoipa::path(
    tag = TAG,
    post,
    security(("api_jwt_token" = [])),
    path = "/v1/replaces_items/search",
    summary = "辞書アイテムを検索する",
    description = SEARCH_DESC,
    request_body = SearchReplaceItemsReq,
    responses(
        (status = 200, description = "Success", body = SearchReplaceItemsRes),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 422, description = "Validation Error", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn search_replace_items(
    ju: JwtUsr,
    ids: JwtIDs,
    Extension(db): Extension<Arc<DbPools>>,
    Json(req): Json<SearchReplaceItemsReq>,
) -> Result<Json<SearchReplaceItemsRes>, ApiError> {
    ju.allow_roles(&[JwtRole::USR])?;
    req.validate().map_err(|e| {
        ApiError::new_system(StatusCode::BAD_REQUEST, "VALIDATION_ERROR", e.to_string())
    })?;

    let conn = db.get_ro_for_rt()?;
    let (list, total) = replace_items_bl::search_items(conn, ids.apx_id, ids.vdr_id, req)
        .await
        .map_err(|e| {
            ApiError::new_system(
                StatusCode::INTERNAL_SERVER_ERROR,
                rterr::ERR_DATABASE,
                e.to_string(),
            )
        })?;

    Ok(Json(SearchReplaceItemsRes { list, total }))
}

const CREATE_DESC: &str = r#"
### ⚫︎ 概要
- 指定された辞書セットに新しい置換アイテムを追加します。

### ⚫︎ 権限
- USR のみが使用可能

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `replace_id` | string | UUID, required | 対象の置換辞書セットID |
| `key` | string | required, 1-200 | 置換後の文字列 |
| `texts` | array | required, min=1 | 置換前の文字列（候補）のリスト |
| `rank` | number | - | ソート順序（昇順） |

### ⚫︎ Response
| KEY | TYPE | DESCRIPTION |
| --- | --- | --- |
| `id` | number | 作成された置換アイテムのID |
"#;
#[utoipa::path(
    tag = TAG,
    post,
    security(("api_jwt_token" = [])),
    path = "/v1/replaces_items/create",
    summary = "辞書アイテムを作成する",
    description = CREATE_DESC,
    request_body = CreateReplaceItemReq,
    responses(
        (status = 200, description = "Success", body = CreateReplaceItemRes),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 422, description = "Validation Error", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn create_replace_item(
    ju: JwtUsr,
    ids: JwtIDs,
    Extension(db): Extension<Arc<DbPools>>,
    Extension(cm): Extension<Arc<crate::mycute_settings::ConfigManager>>,
    Json(req): Json<CreateReplaceItemReq>,
) -> Result<Json<CreateReplaceItemRes>, ApiError> {
    ju.allow_roles(&[JwtRole::USR])?;
    req.validate().map_err(|e| {
        ApiError::new_system(StatusCode::BAD_REQUEST, "VALIDATION_ERROR", e.to_string())
    })?;

    let conn = db.get_rw_for_rt()?;
    let id = replace_items_bl::create_item(conn, ids.apx_id, ids.vdr_id, req.clone())
        .await
        .map_err(|e| {
            ApiError::new_system(
                StatusCode::INTERNAL_SERVER_ERROR,
                rterr::ERR_DATABASE,
                e.to_string(),
            )
        })?;

    // アクティブな場合はキャッシュをリロード
    if cm.is_active_replace_set(&req.replace_id) {
        if let Err(e) = cm.reload_replaces(conn).await {
            log::error!("Failed to reload replaces cache: {}", e);
        }
    }

    Ok(Json(CreateReplaceItemRes { id }))
}

const UPDATE_DESC: &str = r#"
### ⚫︎ 概要
- 指定された置換アイテムの情報を更新します。

### ⚫︎ 権限
- USR のみが使用可能

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `id` | number | Path, required | 置換アイテムID |
| `key` | string | optional, 1-200 | 置換後の文字列 |
| `texts` | array | optional | 置換前の文字列（候補）のリスト |
| `rank` | number | optional | ソート順序（昇順） |

### ⚫︎ Response
| KEY | TYPE | DESCRIPTION |
| --- | --- | --- |
| `id` | number | 更新された置換アイテムのID |
"#;
#[utoipa::path(
    tag = TAG,
    post,
    security(("api_jwt_token" = [])),
    path = "/v1/replaces_items/update/{id}",
    summary = "辞書アイテムを更新する",
    description = UPDATE_DESC,
    params(
        ("id" = i32, Path, description = "ReplaceItem ID")
    ),
    request_body = UpdateReplaceItemReq,
    responses(
        (status = 200, description = "Success", body = UpdateReplaceItemRes),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 404, description = "Not Found", body = ApiError),
        (status = 422, description = "Validation Error", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn update_replace_item(
    ju: JwtUsr,
    ids: JwtIDs,
    Path(id): Path<i32>,
    Extension(db): Extension<Arc<DbPools>>,
    Extension(cm): Extension<Arc<crate::mycute_settings::ConfigManager>>,
    Json(req): Json<UpdateReplaceItemReq>,
) -> Result<Json<UpdateReplaceItemRes>, ApiError> {
    ju.allow_roles(&[JwtRole::USR])?;
    req.validate().map_err(|e| {
        ApiError::new_system(StatusCode::BAD_REQUEST, "VALIDATION_ERROR", e.to_string())
    })?;

    let conn = db.get_rw_for_rt()?;
    replace_items_bl::update_item(conn, ids.apx_id, ids.vdr_id, id, req)
        .await
        .map_err(|e| {
            ApiError::new_system(
                StatusCode::INTERNAL_SERVER_ERROR,
                rterr::ERR_DATABASE,
                e.to_string(),
            )
        })?;

    // アクティブな場合はキャッシュをリロード
    let item = replace_items_bl::get_item_by_id(conn, id)
        .await
        .map_err(|e| {
            ApiError::new_system(
                StatusCode::INTERNAL_SERVER_ERROR,
                rterr::ERR_DATABASE,
                e.to_string(),
            )
        })?;
    if let Some(m) = item {
        if cm.is_active_replace_set(&m.replace_id) {
            if let Err(e) = cm.reload_replaces(conn).await {
                log::error!("Failed to reload replaces cache: {}", e);
            }
        }
    }

    Ok(Json(UpdateReplaceItemRes { id }))
}

const DELETE_DESC: &str = r#"
### ⚫︎ 概要
- 指定された置換アイテムを削除します。

### ⚫︎ 権限
- USR のみが使用可能

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `id` | number | Path, required | 置換アイテムID |

### ⚫︎ Response
| KEY | TYPE | DESCRIPTION |
| --- | --- | --- |
| - | - | 成功時は空のレスポンスを返却します |
"#;
#[utoipa::path(
    tag = TAG,
    post,
    security(("api_jwt_token" = [])),
    path = "/v1/replaces_items/delete/{id}",
    summary = "辞書アイテムを削除する",
    description = DELETE_DESC,
    params(
        ("id" = i32, Path, description = "ReplaceItem ID")
    ),
    responses(
        (status = 200, description = "Success"),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 404, description = "Not Found", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn delete_replace_item(
    ju: JwtUsr,
    ids: JwtIDs,
    Path(id): Path<i32>,
    Extension(db): Extension<Arc<DbPools>>,
    Extension(cm): Extension<Arc<crate::mycute_settings::ConfigManager>>,
) -> Result<Json<()>, ApiError> {
    ju.allow_roles(&[JwtRole::USR])?;

    let conn = db.get_rw_for_rt()?;
    replace_items_bl::delete_item(conn, ids.apx_id, ids.vdr_id, id)
        .await
        .map_err(|e| {
            ApiError::new_system(
                StatusCode::INTERNAL_SERVER_ERROR,
                rterr::ERR_DATABASE,
                e.to_string(),
            )
        })?;

    // アクティブな場合はキャッシュをリロード
    let item = replace_items_bl::get_item_by_id(conn, id)
        .await
        .map_err(|e| {
            ApiError::new_system(
                StatusCode::INTERNAL_SERVER_ERROR,
                rterr::ERR_DATABASE,
                e.to_string(),
            )
        })?;
    if let Some(m) = item {
        if cm.is_active_replace_set(&m.replace_id) {
            if let Err(e) = cm.reload_replaces(conn).await {
                log::error!("Failed to reload replaces cache: {}", e);
            }
        }
    }

    Ok(Json(()))
}
