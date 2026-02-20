use crate::{
    mode::rt::{
        rtbl::replaces_bl,
        rterr::rterr,
        rtreq::replaces_req::{
            ActivateReplacesReq, CreateReplacesReq, ImportReplacesReq, SearchReplacesReq,
            UpdateReplacesReq,
        },
        rtres::{
            errs_res::ApiError,
            replaces_res::{
                ActivateReplacesRes, CreateReplacesRes, ExportReplacesRes, GetReplacesRes,
                SearchReplacesRes, UpdateReplacesRes,
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
use uuid::Uuid;

const TAG: &str = "v1 Replace";

const SEARCH_DESC: &str = r#"
### ⚫︎ 概要
- 自身が所属する置換辞書セットを検索します。

### ⚫︎ 権限
- USR のみが使用可能

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `keyword` | string | - | 名前または説明文の部分一致検索キーワード |
| `is_active` | boolean | - | 有効/無効状態によるフィルタリング |
| `limit` | number | gte=1, lte=100, default=25 | 最大取得件数 |
| `offset` | number | gte=0, default=0 | オフセット位置 |

### ⚫︎ Response
| KEY | TYPE | DESCRIPTION |
| --- | --- | --- |
| `list` | array | 辞書セットのリスト |
| `total` | number | 総件数 |
"#;
#[utoipa::path(
    tag = TAG,
    post,
    security(("api_jwt_token" = [])),
    path = "/v1/replaces/search",
    summary = "辞書セットを検索する",
    description = SEARCH_DESC,
    request_body = SearchReplacesReq,
    responses(
        (status = 200, description = "Success", body = SearchReplacesRes),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 422, description = "Validation Error", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn search_replaces(
    ju: JwtUsr,
    ids: JwtIDs,
    Extension(db): Extension<Arc<DbPools>>,
    Json(req): Json<SearchReplacesReq>,
) -> Result<Json<SearchReplacesRes>, ApiError> {
    ju.allow_roles(&[JwtRole::USR])?;
    req.validate().map_err(|e| {
        ApiError::new_system(StatusCode::BAD_REQUEST, "VALIDATION_ERROR", e.to_string())
    })?;

    let conn = db.get_ro_for_rt()?;
    let (list, total) = replaces_bl::search_replaces(conn, ids.apx_id, ids.vdr_id, req)
        .await
        .map_err(|e| {
            ApiError::new_system(
                StatusCode::INTERNAL_SERVER_ERROR,
                rterr::ERR_DATABASE,
                e.to_string(),
            )
        })?;

    Ok(Json(SearchReplacesRes { list, total }))
}

const GET_DESC: &str = r#"
### ⚫︎ 概要
- 指定された ID の置換辞書セットの詳細情報を取得します。

### ⚫︎ 権限
- USR のみが使用可能

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `id` | string | UUID | 置換辞書セットID |

### ⚫︎ Response
| KEY | TYPE | DESCRIPTION |
| --- | --- | --- |
| `replace` | object | 辞書セットの詳細情報 |
| `items_count` | number | 含まれるアイテムの総数 |
"#;
#[utoipa::path(
    tag = TAG,
    post,
    security(("api_jwt_token" = [])),
    path = "/v1/replaces/get/{id}",
    summary = "辞書セット詳細を取得する",
    description = GET_DESC,
    params(
        ("id" = String, Path, description = "Replaces UUID")
    ),
    responses(
        (status = 200, description = "Success", body = GetReplacesRes),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 404, description = "Not Found", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn get_replaces(
    ju: JwtUsr,
    ids: JwtIDs,
    Path(id): Path<Uuid>,
    Extension(db): Extension<Arc<DbPools>>,
) -> Result<Json<GetReplacesRes>, ApiError> {
    ju.allow_roles(&[JwtRole::USR])?;

    let conn = db.get_ro_for_rt()?;
    let res = replaces_bl::get_replaces(conn, ids.apx_id, ids.vdr_id, id)
        .await
        .map_err(|e| {
            ApiError::new_system(
                StatusCode::INTERNAL_SERVER_ERROR,
                rterr::ERR_DATABASE,
                e.to_string(),
            )
        })?;

    match res {
        Some((replace, items_count)) => Ok(Json(GetReplacesRes {
            replace,
            items_count,
        })),
        None => Err(ApiError::new_system(
            StatusCode::NOT_FOUND,
            "NOT_FOUND",
            format!("Replaces {} not found", id),
        )),
    }
}

const CREATE_DESC: &str = r#"
### ⚫︎ 概要
- 新しい置換辞書セットを作成します。作成直後は非アクティブ状態となります。

### ⚫︎ 権限
- USR のみが使用可能

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `name` | string | required, max=100 | 辞書セット名 |
| `description` | string | max=500 | 説明文 |

### ⚫︎ Response
| KEY | TYPE | DESCRIPTION |
| --- | --- | --- |
| `id` | string | 作成された辞書セットのUUID |
"#;
#[utoipa::path(
    tag = TAG,
    post,
    security(("api_jwt_token" = [])),
    path = "/v1/replaces/create",
    summary = "辞書セットを作成する",
    description = CREATE_DESC,
    request_body = CreateReplacesReq,
    responses(
        (status = 200, description = "Success", body = CreateReplacesRes),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 422, description = "Validation Error", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn create_replaces(
    ju: JwtUsr,
    ids: JwtIDs,
    Extension(db): Extension<Arc<DbPools>>,
    Extension(_cm): Extension<Arc<crate::stt_config::ConfigManager>>,
    Json(req): Json<CreateReplacesReq>,
) -> Result<Json<CreateReplacesRes>, ApiError> {
    ju.allow_roles(&[JwtRole::USR])?;
    req.validate().map_err(|e| {
        ApiError::new_system(StatusCode::BAD_REQUEST, "VALIDATION_ERROR", e.to_string())
    })?;

    let conn = db.get_rw_for_rt()?;
    let id = replaces_bl::create_replaces(conn, ids.apx_id, ids.vdr_id, req)
        .await
        .map_err(|e| {
            ApiError::new_system(
                StatusCode::INTERNAL_SERVER_ERROR,
                rterr::ERR_DATABASE,
                e.to_string(),
            )
        })?;

    Ok(Json(CreateReplacesRes { id }))
}

const UPDATE_DESC: &str = r#"
### ⚫︎ 概要
- 指定された置換辞書セットの名前や説明を更新します。

### ⚫︎ 権限
- USR のみが使用可能

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `id` | string | UUID, Path | 置換辞書セットID |
| `name` | string | optional, max=100 | 辞書セット名 |
| `description` | string | optional, max=500 | 説明文 |

### ⚫︎ Response
| KEY | TYPE | DESCRIPTION |
| --- | --- | --- |
| `id` | string | 更新された辞書セットのUUID |
"#;
#[utoipa::path(
    tag = TAG,
    post,
    security(("api_jwt_token" = [])),
    path = "/v1/replaces/update/{id}",
    summary = "辞書セットを更新する",
    description = UPDATE_DESC,
    params(
        ("id" = String, Path, description = "Replaces UUID")
    ),
    request_body = UpdateReplacesReq,
    responses(
        (status = 200, description = "Success", body = UpdateReplacesRes),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 404, description = "Not Found", body = ApiError),
        (status = 422, description = "Validation Error", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn update_replaces(
    ju: JwtUsr,
    ids: JwtIDs,
    Path(id): Path<Uuid>,
    Extension(db): Extension<Arc<DbPools>>,
    Extension(cm): Extension<Arc<crate::stt_config::ConfigManager>>,
    Json(req): Json<UpdateReplacesReq>,
) -> Result<Json<UpdateReplacesRes>, ApiError> {
    ju.allow_roles(&[JwtRole::USR])?;
    req.validate().map_err(|e| {
        ApiError::new_system(StatusCode::BAD_REQUEST, "VALIDATION_ERROR", e.to_string())
    })?;

    let conn = db.get_rw_for_rt()?;
    replaces_bl::update_replaces(conn, ids.apx_id, ids.vdr_id, id, req)
        .await
        .map_err(|e| {
            ApiError::new_system(
                StatusCode::INTERNAL_SERVER_ERROR,
                rterr::ERR_DATABASE,
                e.to_string(),
            )
        })?;

    // アクティブな場合はキャッシュをリロード
    if cm.is_active_replace_set(&id) {
        if let Err(e) = cm.reload_replaces(conn).await {
            log::error!("Failed to reload replaces cache: {}", e);
        }
    }

    Ok(Json(UpdateReplacesRes { id }))
}

const DELETE_DESC: &str = r#"
### ⚫︎ 概要
- 指定された置換辞書セットを削除します。

### ⚫︎ 権限
- USR のみが使用可能

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `id` | string | UUID, Path | 置換辞書セットID |

### ⚫︎ Response
| KEY | TYPE | DESCRIPTION |
| --- | --- | --- |
| - | - | 成功時は空のレスポンスを返却します |
"#;
#[utoipa::path(
    tag = TAG,
    post,
    security(("api_jwt_token" = [])),
    path = "/v1/replaces/delete/{id}",
    summary = "辞書セットを削除する",
    description = DELETE_DESC,
    params(
        ("id" = String, Path, description = "Replaces UUID")
    ),
    responses(
        (status = 200, description = "Success"),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 404, description = "Not Found", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn delete_replaces(
    ju: JwtUsr,
    ids: JwtIDs,
    Path(id): Path<Uuid>,
    Extension(db): Extension<Arc<DbPools>>,
    Extension(cm): Extension<Arc<crate::stt_config::ConfigManager>>,
) -> Result<Json<()>, ApiError> {
    ju.allow_roles(&[JwtRole::USR])?;

    let conn = db.get_rw_for_rt()?;
    replaces_bl::delete_replaces(conn, ids.apx_id, ids.vdr_id, id)
        .await
        .map_err(|e| {
            ApiError::new_system(
                StatusCode::INTERNAL_SERVER_ERROR,
                rterr::ERR_DATABASE,
                e.to_string(),
            )
        })?;

    // アクティブな場合はキャッシュをリロード
    if cm.is_active_replace_set(&id) {
        if let Err(e) = cm.reload_replaces(conn).await {
            log::error!("Failed to reload replaces cache: {}", e);
        }
    }

    Ok(Json(()))
}

const ACTIVATE_DESC: &str = r#"
### ⚫︎ 概要
- 指定された置換辞書セットの有効/無効状態を切り替えます。

### ⚫︎ 権限
- USR のみが使用可能

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `id` | string | UUID, Path | 置換辞書セットID |
| `is_active` | boolean | required | 有効化する場合は true, 無効化する場合は false |

### ⚫︎ Response
| KEY | TYPE | DESCRIPTION |
| --- | --- | --- |
| `id` | string | 操作された辞書セットのUUID |
| `is_active` | boolean | 変更後の有効状態 |
"#;
#[utoipa::path(
    tag = TAG,
    post,
    security(("api_jwt_token" = [])),
    path = "/v1/replaces/activate/{id}",
    summary = "辞書セットを有効化/無効化する",
    description = ACTIVATE_DESC,
    params(
        ("id" = String, Path, description = "Replaces UUID")
    ),
    request_body = ActivateReplacesReq,
    responses(
        (status = 200, description = "Success", body = ActivateReplacesRes),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 404, description = "Not Found", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn activate_replaces(
    ju: JwtUsr,
    ids: JwtIDs,
    Path(id): Path<Uuid>,
    Extension(db): Extension<Arc<DbPools>>,
    Extension(cm): Extension<Arc<crate::stt_config::ConfigManager>>,
    Json(req): Json<ActivateReplacesReq>,
) -> Result<Json<ActivateReplacesRes>, ApiError> {
    ju.allow_roles(&[JwtRole::USR])?;

    let conn = db.get_rw_for_rt()?;

    // 備考: 1つを有効化することが他を無効化することを意味する場合、そのロジックはBLに置くべき。
    // 現時点では、単純な切り替えロジックが BL の `set_replace_set_active` に実装されている。
    // もし厳密な排他的有効化が必要な場合（一度に1つのみアクティブにするなど）は、
    // より複雑なロジックが必要になる。
    // ここでは複数のセットが同時にアクティブであることを許可し、マージされるものと想定する。
    // 排他的にしたい場合は、ユーザーが他を無効化する必要がある。

    replaces_bl::set_replace_set_active(
        conn,
        ids.apx_id as u32,
        ids.vdr_id as u32,
        id,
        req.is_active,
    )
    .await
    .map_err(|e| {
        ApiError::new_system(
            StatusCode::INTERNAL_SERVER_ERROR,
            rterr::ERR_DATABASE,
            e.to_string(),
        )
    })?;

    // キャッシュのリロード
    if let Err(e) = cm.reload_replaces(conn).await {
        log::error!("Failed to reload replaces cache: {}", e);
    }

    Ok(Json(ActivateReplacesRes {
        id,
        is_active: req.is_active,
    }))
}

const EXPORT_DESC: &str = r#"
### ⚫︎ 概要
- 指定された置換辞書セットとその中に含まれるすべてのアイテムを、一括エクスポート用の形式で取得します。

### ⚫︎ 権限
- USR のみが使用可能

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `id` | string | UUID, Path | 置換辞書セットID |

### ⚫︎ Response
| KEY | TYPE | DESCRIPTION |
| --- | --- | --- |
| `replace` | object | 辞書セットの詳細情報 |
| `items` | array | 含まれるアイテムのリスト |
"#;
#[utoipa::path(
    tag = TAG,
    post,
    security(("api_jwt_token" = [])),
    path = "/v1/replaces/export/{id}",
    summary = "辞書セットをエクスポートする",
    description = EXPORT_DESC,
    params(
        ("id" = String, Path, description = "Replaces UUID")
    ),
    responses(
        (status = 200, description = "Success", body = ExportReplacesRes),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 404, description = "Not Found", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn export_replaces(
    ju: JwtUsr,
    ids: JwtIDs,
    Path(id): Path<Uuid>,
    Extension(db): Extension<Arc<DbPools>>,
) -> Result<Json<ExportReplacesRes>, ApiError> {
    ju.allow_roles(&[JwtRole::USR])?;

    let conn = db.get_ro_for_rt()?;
    let res = replaces_bl::export_replaces(conn, ids.apx_id, ids.vdr_id, id)
        .await
        .map_err(|e| {
            ApiError::new_system(
                StatusCode::INTERNAL_SERVER_ERROR,
                rterr::ERR_DATABASE,
                e.to_string(),
            )
        })?;

    Ok(Json(res))
}

const IMPORT_DESC: &str = r#"
### ⚫︎ 概要
- エクスポートされた形式のデータを受け取り、新しい置換辞書セットとしてアイテムを含めて一括インポートします。

### ⚫︎ 権限
- USR のみが使用可能

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `name` | string | required, max=100 | 新しい辞書セット名 |
| `description` | string | max=500 | 説明文 |
| `items` | array | required, min_length=1 | インポートするアイテムのリスト |

### ⚫︎ Response
| KEY | TYPE | DESCRIPTION |
| --- | --- | --- |
| `id` | string | 作成された辞書セットのUUID |
"#;
#[utoipa::path(
    tag = TAG,
    post,
    security(("api_jwt_token" = [])),
    path = "/v1/replaces/import",
    summary = "辞書セットをインポートする",
    description = IMPORT_DESC,
    request_body = ImportReplacesReq,
    responses(
        (status = 200, description = "Success", body = CreateReplacesRes),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 422, description = "Validation Error", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn import_replaces(
    ju: JwtUsr,
    ids: JwtIDs,
    Extension(db): Extension<Arc<DbPools>>,
    Extension(cm): Extension<Arc<crate::stt_config::ConfigManager>>,
    Json(req): Json<ImportReplacesReq>,
) -> Result<Json<CreateReplacesRes>, ApiError> {
    ju.allow_roles(&[JwtRole::USR])?;
    req.validate().map_err(|e| {
        ApiError::new_system(StatusCode::BAD_REQUEST, "VALIDATION_ERROR", e.to_string())
    })?;

    let conn = db.get_rw_for_rt()?;
    let id = replaces_bl::import_replaces(conn, ids.apx_id, ids.vdr_id, req)
        .await
        .map_err(|e| {
            ApiError::new_system(
                StatusCode::INTERNAL_SERVER_ERROR,
                rterr::ERR_DATABASE,
                e.to_string(),
            )
        })?;

    // キャッシュのリロード
    // インポートはトランザクションを使用する。ここでの conn はプールからの RW コネクション。
    // import_replaces は内部でトランザクションを開始する。
    // import_replaces<C: TransactionTrait> となっているため、トランザクションを開始可能なオブジェクトを期待している。
    // DatabaseConnection は TransactionTrait を実装している。
    if let Err(e) = cm.reload_replaces(conn).await {
        log::error!("Failed to reload replaces cache: {}", e);
    }

    Ok(Json(CreateReplacesRes { id }))
}
