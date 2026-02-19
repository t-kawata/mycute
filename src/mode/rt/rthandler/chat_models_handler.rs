use std::sync::Arc;
use axum::{
    extract::{Path, Extension},
    Json,
    response::IntoResponse,
};
use garde::Validate;
use crate::{
    mode::rt::{
        rtreq::chat_models_req::{SearchChatModelsReq, CreateChatModelReq, UpdateChatModelReq},
        rtres::{
            errs_res::ApiError,
            chat_models_res::{SearchChatModelsRes, ChatModelRes, CreateChatModelRes, UpdateChatModelRes, DeleteChatModelRes}
        },
        rtbl::chat_models_bl,
        rtutils::db_for_rt::DbPoolsExt,
    },
    utils::{
        db::DbPools,
        jwt::{JwtUsr, JwtIDs, JwtRole}
    }
};

const TAG: &str = "v1 ChatModels";

// ============================================================
// Search
// ============================================================
const SEARCH_DESC: &str = r#"
### ⚫︎ 概要
- 利用可能なチャットモデルを検索する。
- USR によってのみ使用できる

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `name` | string | max=50 | モデル名検索 |
| `provider` | string | max=50 | プロバイダー検索 |
| `model` | string | max=100 | モデルID検索 |
| `base_url` | string | max=255 | BaseURL検索 |
| `limit` | number | default=10, min=1, max=100 | 取得件数 |
| `offset` | number | default=0, min=0 | 取得開始位置 |
"#;
#[utoipa::path(
    post,
    path = "/chat_models/search",
    tag = TAG,
    request_body = SearchChatModelsReq,
    description = SEARCH_DESC,
    responses(
        (status = 200, description = "Success", body = SearchChatModelsRes),
        (status = 400, description = "Bad Request", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError),
    )
)]
pub async fn search_chat_models(
    ju: JwtUsr,
    ids: JwtIDs,
    Extension(db): Extension<Arc<DbPools>>,
    Json(req): Json<SearchChatModelsReq>,
) -> Result<impl IntoResponse, ApiError> {
    log::debug!("<ChatModels> search_chat_models called by user_id: {}", ju.usr_id);
    ju.allow_roles(&[JwtRole::USR])?;
    req.validate().map_err(|e| ApiError::from_garde(e))?;
    let conn = db.get_ro_for_rt()?;
    let res = chat_models_bl::search_chat_models(conn, &ju, &ids, req).await?;
    Ok(Json(res))
}

// ============================================================
// Get
// ============================================================
const GET_DESC: &str = r#"
### ⚫︎ 概要
- 特定のチャットモデルの詳細を取得する。
- USR によってのみ使用できる

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `chat_model_id` | number | required | モデルID |
"#;
#[utoipa::path(
    get,
    path = "/chat_models/{chat_model_id}",
    tag = TAG,
    description = GET_DESC,
    params(
        ("chat_model_id" = i32, Path, description = "ChatModel ID")
    ),
    responses(
        (status = 200, description = "Success", body = ChatModelRes),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 404, description = "Not Found", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError),
    )
)]
pub async fn get_chat_model(
    ju: JwtUsr,
    ids: JwtIDs,
    Extension(db): Extension<Arc<DbPools>>,
    Path(chat_model_id): Path<i32>,
) -> Result<impl IntoResponse, ApiError> {
    log::debug!("<ChatModels> get_chat_model called for chat_model_id: {}", chat_model_id);
    ju.allow_roles(&[JwtRole::USR])?;
    let conn = db.get_ro_for_rt()?;
    let res = chat_models_bl::get_chat_model(conn, &ju, &ids, chat_model_id).await?;
    Ok(Json(res))
}

// ============================================================
// Create
// ============================================================
const CREATE_DESC: &str = r#"
### ⚫︎ 概要
- 新しいチャットモデルを登録する。
- USR によってのみ使用できる

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `name` | string | required, max=50 | モデル表示名 |
| `provider` | string | required, max=50 | プロバイダー (openai, anthropic...) |
| `model` | string | required, max=100 | モデル識別子 |
| `base_url` | string | max=255 | ベースURL (Optional) |
| `api_key` | string | required, max=1024 | APIキー |
| `max_tokens` | number | required | 最大トークン数 |
| `temperature` | number | required | 応答の多様性 (0.0 - 1.0) |
"#;
#[utoipa::path(
    post,
    path = "/chat_models",
    tag = TAG,
    description = CREATE_DESC,
    request_body = CreateChatModelReq,
    responses(
        (status = 200, description = "Success", body = CreateChatModelRes),
        (status = 400, description = "Bad Request", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError),
    )
)]
pub async fn create_chat_model(
    ju: JwtUsr,
    ids: JwtIDs,
    Extension(db): Extension<Arc<DbPools>>,
    Json(req): Json<CreateChatModelReq>,
) -> Result<impl IntoResponse, ApiError> {
    log::debug!("<ChatModels> create_chat_model called");
    ju.allow_roles(&[JwtRole::USR])?;
    req.validate().map_err(|e| ApiError::from_garde(e))?;
    let conn = db.get_rw_for_rt()?;
    let res = chat_models_bl::create_chat_model(conn, &ju, &ids, req).await?;
    Ok(Json(res))
}


// ============================================================
// Update
// ============================================================
const UPDATE_DESC: &str = r#"
### ⚫︎ 概要
- チャットモデル情報を更新する。
- USR によってのみ使用できる

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `chat_model_id` | number | required | モデルID |
| `name` | string | max=50 | （Optional） |
| `provider` | string | max=50 | （Optional） |
| `model` | string | max=100 | （Optional） |
| `base_url` | string | max=255 | （Optional） |
| `api_key` | string | max=1024 | （Optional） |
| `max_tokens` | number | min=1 | （Optional） |
| `temperature` | number | 0.0-1.0 | （Optional） |
"#;

#[utoipa::path(
    patch,
    path = "/chat_models/{chat_model_id}",
    tag = TAG,
    description = UPDATE_DESC,
    request_body = UpdateChatModelReq,
    params(
        ("chat_model_id" = i32, Path, description = "ChatModel ID")
    ),
    responses(
        (status = 200, description = "Success", body = UpdateChatModelRes),
        (status = 400, description = "Bad Request", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 404, description = "Not Found", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError),
    )
)]
pub async fn update_chat_model(
    ju: JwtUsr,
    ids: JwtIDs,
    Extension(db): Extension<Arc<DbPools>>,
    Path(chat_model_id): Path<i32>,
    Json(req): Json<UpdateChatModelReq>,
) -> Result<impl IntoResponse, ApiError> {
    log::debug!("<ChatModels> update_chat_model called for chat_model_id: {}", chat_model_id);
    ju.allow_roles(&[JwtRole::USR])?;
    req.validate().map_err(|e| ApiError::from_garde(e))?;
    let conn = db.get_rw_for_rt()?;
    let res = chat_models_bl::update_chat_model(conn, &ju, &ids, chat_model_id, req).await?;
    Ok(Json(res))
}

// ============================================================
// Delete
// ============================================================
const DELETE_DESC: &str = r#"
### ⚫︎ 概要
- チャットモデルを削除する。
- USR によってのみ使用できる

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `chat_model_id` | number | required | モデルID |
"#;
#[utoipa::path(
    delete,
    path = "/chat_models/{chat_model_id}",
    tag = TAG,
    description = DELETE_DESC,
    params(
        ("chat_model_id" = i32, Path, description = "ChatModel ID")
    ),
    responses(
        (status = 200, description = "Success", body = DeleteChatModelRes),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 404, description = "Not Found", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError),
    )
)]
pub async fn delete_chat_model(
    ju: JwtUsr,
    ids: JwtIDs,
    Extension(db): Extension<Arc<DbPools>>,
    Path(chat_model_id): Path<i32>,
) -> Result<impl IntoResponse, ApiError> {
    log::debug!("<ChatModels> delete_chat_model called for chat_model_id: {}", chat_model_id);
    ju.allow_roles(&[JwtRole::USR])?;
    let conn = db.get_rw_for_rt()?;
    let res = chat_models_bl::delete_chat_model(conn, &ju, &ids, chat_model_id).await?;
    Ok(Json(res))
}
