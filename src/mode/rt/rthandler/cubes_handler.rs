use crate::{
    mode::rt::{
        rtbl::cubes_bl,
        rtreq::cubes_req::{
            AbsorbCubeReq, CreateCubeReq, DeleteCubeReq, ExportCubeReq, GenKeyCubeReq,
            ImportCubeReq, MemifyCubeReq, QueryCubeReq, ReKeyCubeReq, SearchCubesReq,
        },
        rtres::{
            cubes_res::{
                AbsorbCubeRes, CreateCubeRes, CubeRes, DeleteCubeRes, ExportCubeRes, GenKeyCubeRes,
                ImportCubeRes, MemifyCubeRes, QueryCubeRes, ReKeyCubeRes, SearchCubesRes,
            },
            errs_res::ApiError,
        },
        rtutils::db_for_rt::DbPoolsExt,
    },
    utils::{
        db::DbPools,
        jwt::{JwtIDs, JwtRole, JwtUsr},
    },
};
use axum::{
    extract::{Extension, Path, Query},
    response::IntoResponse,
    Json,
};
use garde::Validate;
use std::sync::Arc;

const TAG: &str = "v1 Cubes";

// ============================================================
// Search
// ============================================================
const SEARCH_DESC: &str = r#"
### ⚫︎ 概要
- 利用可能なCubeを検索する。
- ユーザーは自分が所有するCubeのみ、または権限があるものを参照可能。

### ⚫︎ 権限
- USR ロールのみが可能

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `name` | string | max=50 | 名前検索 |
| `description` | string | max=255 | 説明検索 |
| `limit` | number | range=(1, 25) | 取得数 (Max 25) |
| `offset` | number | range=(0, ) | オフセット |
"#;
#[utoipa::path(
    post,
    path = "/cubes/search",
    tag = TAG,
    description = SEARCH_DESC,
    request_body = SearchCubesReq,
    responses(
        (status = 200, description = "Success", body = SearchCubesRes),
        (status = 400, description = "Bad Request", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError),
    )
)]
pub async fn search_cubes(
    ju: JwtUsr,
    ids: JwtIDs,
    Extension(db): Extension<Arc<DbPools>>,
    Json(req): Json<SearchCubesReq>,
) -> Result<impl IntoResponse, ApiError> {
    log::debug!("<Cubes> search_cubes called by user_id: {}", ju.usr_id);
    ju.allow_roles(&[JwtRole::USR])?;
    req.validate().map_err(|e| ApiError::from_garde(e))?;
    let conn = db.get_ro_for_rt()?;
    let res = cubes_bl::search_cubes(conn, &ju, &ids, req).await?;
    Ok(Json(res))
}

// ============================================================
// Get
// ============================================================
const GET_DESC: &str = r#"
### ⚫︎ 概要
- 特定のCubeの詳細を取得する。

### ⚫︎ 権限
- USR ロールのみが可能

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `id` | number | required | Cube ID |
"#;
#[utoipa::path(
    get,
    path = "/cubes/get/{cube_id}",
    tag = TAG,
    description = GET_DESC,
    params(
        ("cube_id" = i32, Path, description = "Cube ID")
    ),
    responses(
        (status = 200, description = "Success", body = CubeRes),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 404, description = "Not Found", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError),
    )
)]
pub async fn get_cube(
    ju: JwtUsr,
    ids: JwtIDs,
    Extension(db): Extension<Arc<DbPools>>,
    Path(cube_id): Path<i32>,
) -> Result<impl IntoResponse, ApiError> {
    log::debug!("<Cubes> get_cube called for cube_id: {}", cube_id);
    ju.allow_roles(&[JwtRole::USR])?;
    let conn = db.get_ro_for_rt()?;
    let res = cubes_bl::get_cube(conn, &ju, &ids, cube_id).await?;
    Ok(Json(res))
}

// ============================================================
// Create
// ============================================================
const CREATE_DESC: &str = r#"
### ⚫︎ 概要
- 新しいCubeを作成する。

### ⚫︎ 権限
- USR ロールのみが可能

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `name` | string | required, max=50 | Cube名 |
| `description` | string | max=255 | 説明 |
| `embedding_provider` | string | required, max=50 | 埋め込みプロバイダー |
| `embedding_model` | string | required, max=100 | 埋め込みモデル |
| `embedding_dimension` | number | required | 次元数 |
| `embedding_api_key` | string | required | APIキー |
| `embedding_base_url` | string | max=255 | BaseURL (Optional) |
"#;
#[utoipa::path(
    post,
    path = "/cubes/create",
    tag = TAG,
    description = CREATE_DESC,
    request_body = CreateCubeReq,
    responses(
        (status = 200, description = "Success", body = CreateCubeRes),
        (status = 400, description = "Bad Request", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError),
    )
)]
pub async fn create_cube(
    ju: JwtUsr,
    ids: JwtIDs,
    Extension(db): Extension<Arc<DbPools>>,
    Json(req): Json<CreateCubeReq>,
) -> Result<impl IntoResponse, ApiError> {
    log::debug!("<Cubes> create_cube called");
    ju.allow_roles(&[JwtRole::USR])?;
    req.validate().map_err(|e| ApiError::from_garde(e))?;
    let conn = db.get_rw_for_rt()?;
    let res = cubes_bl::create_cube(conn, &ju, &ids, req).await?;
    Ok(Json(res))
}

// ============================================================
// Update
// ============================================================

// ============================================================
// Delete
// ============================================================
const DELETE_DESC: &str = r#"
### ⚫︎ 概要
- Cubeを削除する。論理削除。

### ⚫︎ 権限
- USR ロールのみが可能

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `cube_id` | number | required | Cube ID |
"#;
#[utoipa::path(
    delete,
    path = "/cubes/delete",
    tag = TAG,
    description = DELETE_DESC,
    request_body = DeleteCubeReq, // actually Query params but utoipa checks generic
    params(
        DeleteCubeReq
    ),
    responses(
        (status = 200, description = "Success", body = DeleteCubeRes),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 404, description = "Not Found", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError),
    )
)]
pub async fn delete_cube(
    ju: JwtUsr,
    ids: JwtIDs,
    Extension(db): Extension<Arc<DbPools>>,
    Query(req): Query<DeleteCubeReq>,
) -> Result<impl IntoResponse, ApiError> {
    log::debug!("<Cubes> delete_cube called for cube_id: {}", req.cube_id);
    ju.allow_roles(&[JwtRole::USR])?;
    let conn = db.get_rw_for_rt()?;
    let res = cubes_bl::delete_cube(conn, &ju, &ids, req.cube_id).await?;
    Ok(Json(res))
}

// ============================================================
// Absorb
// ============================================================
const ABSORB_DESC: &str = r#"
### ⚫︎ 概要
- ファイルをアップロードして知識を取り込む。

### ⚫︎ 権限
- USR ロールのみが可能

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `cube_id` | number | required | Cube ID |
"#;
#[utoipa::path(
    put,
    path = "/cubes/absorb",
    tag = TAG,
    description = ABSORB_DESC,
    request_body = AbsorbCubeReq,
    responses(
        (status = 200, description = "Success", body = AbsorbCubeRes),
        (status = 400, description = "Bad Request", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError),
    )
)]
pub async fn absorb_cube(
    ju: JwtUsr,
    _ids: JwtIDs,
    Extension(_db): Extension<Arc<DbPools>>,
    Json(req): Json<AbsorbCubeReq>, // strict struct
) -> Result<impl IntoResponse, ApiError> {
    log::debug!("<Cubes> absorb_cube called for cube_id: {}", req.cube_id);
    ju.allow_roles(&[JwtRole::USR])?;
    req.validate().map_err(|e| ApiError::from_garde(e))?;

    Ok(Json(AbsorbCubeRes {
        input_tokens: 100,
        output_tokens: 50,
        absorb_limit: 9900,
    }))
}

// ============================================================
// Query
// ============================================================
const QUERY_DESC: &str = r#"
### ⚫︎ 概要
- Cubeに対して質問を行う。

### ⚫︎ 権限
- USR ロールのみが可能

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `cube_id` | number | required | Cube ID |
| `memory_group` | string | required | メモリグループ |
| `text` | string | required | 質問内容 |
| `type` | number | required | 検索タイプ (1-11) |
| `chat_model_id` | number | required | チャットモデルID |
"#;
#[utoipa::path(
    post,
    path = "/cubes/query",
    tag = TAG,
    description = QUERY_DESC,
    request_body = QueryCubeReq,
    responses(
        (status = 200, description = "Success", body = QueryCubeRes),
        (status = 400, description = "Bad Request", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError),
    )
)]
pub async fn query_cube(
    ju: JwtUsr,
    _ids: JwtIDs,
    Extension(_db): Extension<Arc<DbPools>>,
    Json(req): Json<QueryCubeReq>,
) -> Result<impl IntoResponse, ApiError> {
    log::debug!("<Cubes> query_cube called for cube_id: {}", req.cube_id);
    ju.allow_roles(&[JwtRole::USR])?;
    req.validate().map_err(|e| ApiError::from_garde(e))?;

    Ok(Json(QueryCubeRes {
        answer: Some("This is a dummy answer.".to_string()),
        chunks: Some("chunk1\nchunk2".to_string()),
        summaries: None,
        graph: None,
        input_tokens: 150,
        output_tokens: 75,
        query_limit: 999,
    }))
}

// ============================================================
// Memify
// ============================================================
const MEMIFY_DESC: &str = r#"
### ⚫︎ 概要
- Cubeの自己組織化（整理・最適化）を行う。

### ⚫︎ 権限
- USR ロールのみが可能

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `cube_id` | number | required | Cube ID |
| `memory_group` | string | required | メモリグループ |
| `epochs` | number | optional | エポック数 |
| `chat_model_id` | number | required | チャットモデルID |
"#;
#[utoipa::path(
    put,
    path = "/cubes/memify",
    tag = TAG,
    description = MEMIFY_DESC,
    request_body = MemifyCubeReq,
    responses(
        (status = 200, description = "Success", body = MemifyCubeRes),
        (status = 400, description = "Bad Request", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError),
    )
)]
pub async fn memify_cube(
    ju: JwtUsr,
    _ids: JwtIDs,
    Extension(_db): Extension<Arc<DbPools>>,
    Json(req): Json<MemifyCubeReq>,
) -> Result<impl IntoResponse, ApiError> {
    log::debug!("<Cubes> memify_cube called for cube_id: {}", req.cube_id);
    ju.allow_roles(&[JwtRole::USR])?;
    req.validate().map_err(|e| ApiError::from_garde(e))?;

    Ok(Json(MemifyCubeRes {
        input_tokens: 500,
        output_tokens: 200,
        memify_limit: 50,
    }))
}

// ============================================================
// GenKey
// ============================================================
const GENKEY_DESC: &str = r#"
### ⚫︎ 概要
- Cubeのエクスポート用暗号化鍵を生成する。

### ⚫︎ 権限
- USR ロールのみが可能

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `permissions` | object | required | 権限設定 (JSON) |
| `expire_at` | string | optional | 有効期限 |
"#;
#[utoipa::path(
    post,
    path = "/cubes/genkey",
    tag = TAG,
    description = GENKEY_DESC,
    request_body = GenKeyCubeReq,
    responses(
        (status = 200, description = "Success", body = GenKeyCubeRes),
        (status = 400, description = "Bad Request", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError),
    )
)]
pub async fn genkey_cube(
    ju: JwtUsr,
    _ids: JwtIDs,
    Extension(_db): Extension<Arc<DbPools>>,
    Json(req): Json<GenKeyCubeReq>,
) -> Result<impl IntoResponse, ApiError> {
    log::debug!("<Cubes> genkey_cube called");
    ju.allow_roles(&[JwtRole::USR])?;
    req.validate().map_err(|e| ApiError::from_garde(e))?;

    Ok(Json(GenKeyCubeRes {
        key: "dummy-generated-key".to_string(),
    }))
}

// ============================================================
// ReKey
// ============================================================
const REKEY_DESC: &str = r#"
### ⚫︎ 概要
- Cubeの鍵を再設定（ローテーション）する。

### ⚫︎ 権限
- USR ロールのみが可能

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `cube_id` | number | required | Cube ID |
| `key` | string | required | 新しい鍵 |
"#;
#[utoipa::path(
    post,
    path = "/cubes/rekey",
    tag = TAG,
    description = REKEY_DESC,
    request_body = ReKeyCubeReq,
    // Removed Path param as ID is in body now? strict alignment says yes body has cube_id
    responses(
        (status = 200, description = "Success", body = ReKeyCubeRes),
        (status = 400, description = "Bad Request", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError),
    )
)]
pub async fn rekey_cube(
    ju: JwtUsr,
    _ids: JwtIDs,
    Extension(_db): Extension<Arc<DbPools>>,
    // Path(id) removed in favor of strict body struct mapping
    Json(req): Json<ReKeyCubeReq>,
) -> Result<impl IntoResponse, ApiError> {
    log::debug!("<Cubes> rekey_cube called for cube_id: {}", req.cube_id);
    ju.allow_roles(&[JwtRole::USR])?;
    req.validate().map_err(|e| ApiError::from_garde(e))?;

    Ok(Json(ReKeyCubeRes { success: true }))
}

// ============================================================
// Export
// ============================================================
const EXPORT_DESC: &str = r#"
### ⚫︎ 概要
- Cubeをエクスポートする。

### ⚫︎ 権限
- USR ロールのみが可能

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `id` | number | required | Cube ID |
"#;
#[utoipa::path(
    get,
    path = "/cubes/export",
    tag = TAG,
    description = EXPORT_DESC,
    params(
        ExportCubeReq
    ),
    responses(
        (status = 200, description = "Success", body = ExportCubeRes),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError),
    )
)]
pub async fn export_cube(
    ju: JwtUsr,
    _ids: JwtIDs,
    Extension(_db): Extension<Arc<DbPools>>,
    Query(req): Query<ExportCubeReq>,
) -> Result<impl IntoResponse, ApiError> {
    log::debug!("<Cubes> export_cube called for cube_id: {}", req.cube_id);
    ju.allow_roles(&[JwtRole::USR])?;

    Ok(Json(ExportCubeRes {}))
}

// ============================================================
// Import
// ============================================================
const IMPORT_DESC: &str = r#"
### ⚫︎ 概要
- Cubeをインポートする。

### ⚫︎ 権限
- USR ロールのみが可能

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `key` | string | required | 鍵 |
| `name` | string | required | Cube名 |
| `description` | string | required | 説明 |
| `embedding_api_key` | string | required | 埋め込みAPIキー |
"#;
#[utoipa::path(
    post,
    path = "/cubes/import",
    tag = TAG,
    description = IMPORT_DESC,
    request_body = ImportCubeReq,
    responses(
        (status = 200, description = "Success", body = ImportCubeRes),
        (status = 400, description = "Bad Request", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError),
    )
)]
pub async fn import_cube(
    ju: JwtUsr,
    _ids: JwtIDs,
    Extension(_db): Extension<Arc<DbPools>>,
    Json(_req): Json<ImportCubeReq>,
) -> Result<impl IntoResponse, ApiError> {
    log::debug!("<Cubes> import_cube called");
    ju.allow_roles(&[JwtRole::USR])?;

    Ok(Json(ImportCubeRes {
        id: 100,
        uuid: "550e8400-e29b-41d4-a716-446655440000".to_string(),
    }))
}
