//! Cubes ビジネスロジック
//!
//! CRUD順序: Search -> Get -> Create -> Update -> Delete

use sea_orm::{
    DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, QuerySelect, Select,
    ActiveModelTrait, Set, ModelTrait,
};
use uuid::Uuid;
use crate::entities::cubes;
use crate::utils::jwt::{JwtUsr, JwtIDs};
use crate::mode::rt::rtreq::cubes_req::{SearchCubesReq, CreateCubeReq};
use crate::mode::rt::rtres::cubes_res::{
    SearchCubesRes, SearchCubesResItem, CubeRes, CreateCubeRes, DeleteCubeRes,
};
use crate::mode::rt::rtres::errs_res::ApiError;
use crate::constants::{ST_INTERNAL_SERVER_ERROR, ST_NOT_FOUND};
use crate::mode::rt::rterr::rterr;

// ============================================================
// Private Helper for Search and Get
// ============================================================
/// 権限に基づいた共通のクエリベースを作成する
/// Cube は USR のみが使用可能であり、usr_id でパーティショニングされる
fn find_cubes_base(
    ju: &JwtUsr,
    ids: &JwtIDs,
) -> Select<cubes::Entity> {
    log::debug!(
        "<CubesBl> find_cubes_base: Filter apx_id: {}, vdr_id: {}, usr_id: {}",
        ids.apx_id, ids.vdr_id, ju.usr_id
    );
    cubes::Entity::find()
        .filter(cubes::Column::ApxId.eq(ids.apx_id as i32))
        .filter(cubes::Column::VdrId.eq(ids.vdr_id as i32))
        .filter(cubes::Column::UsrId.eq(ju.usr_id as i32))
}

// ============================================================
// Search
// ============================================================
pub async fn search_cubes(
    conn: &DatabaseConnection,
    ju: &JwtUsr,
    ids: &JwtIDs,
    req: SearchCubesReq,
) -> Result<SearchCubesRes, ApiError> {
    // --------------------------------
    // 1. クエリの基本形を取得
    // --------------------------------
    log::debug!("<CubesBl> search_cubes: Constructing base query.");
    let mut query = find_cubes_base(ju, ids);
    // --------------------------------
    // 2. 検索条件（LIKE検索）
    // --------------------------------
    if let Some(ref name) = req.name {
        if !name.is_empty() {
            log::debug!("<CubesBl> search_cubes: Filter by name: {}", name);
            query = query.filter(cubes::Column::Name.contains(name));
        }
    }
    if let Some(ref description) = req.description {
        if !description.is_empty() {
            log::debug!("<CubesBl> search_cubes: Filter by description: {}", description);
            query = query.filter(cubes::Column::Description.contains(description));
        }
    }
    // --------------------------------
    // 3. ページネーション
    // --------------------------------
    let limit = req.limit.unwrap_or(25) as u64;
    let offset = req.offset.unwrap_or(0) as u64;
    log::debug!("<CubesBl> search_cubes: Fetching records. limit: {}, offset: {}", limit, offset);
    // --------------------------------
    // 4. データの取得
    // --------------------------------
    let models = query
        .offset(offset)
        .limit(limit)
        .all(conn)
        .await
        .map_err(|e| ApiError::new_system(
            ST_INTERNAL_SERVER_ERROR,
            rterr::ERR_DATABASE,
            format!("Search query error: {}", e),
        ))?;
    log::debug!("<CubesBl> search_cubes: Found {} records.", models.len());
    // --------------------------------
    // 5. DBデータのレスポンス用変換
    // --------------------------------
    let data: Vec<SearchCubesResItem> = models.into_iter().map(|m| {
        SearchCubesResItem {
            cube: CubeRes::from(m),
            lineage: vec![],         // TODO: LadybugDB 連携後に実装
            memory_groups: vec![],   // TODO: LadybugDB 連携後に実装
        }
    }).collect();
    let total = data.len() as u64;
    // --------------------------------
    // 6. 最終レスポンス
    // --------------------------------
    Ok(SearchCubesRes { data, total })
}

// ============================================================
// Get
// ============================================================
pub async fn get_cube(
    conn: &DatabaseConnection,
    ju: &JwtUsr,
    ids: &JwtIDs,
    cube_id: i32,
) -> Result<SearchCubesResItem, ApiError> {
    // --------------------------------
    // 1. クエリの基本形を取得
    // --------------------------------
    log::debug!("<CubesBl> get_cube: Fetching cube: {}", cube_id);
    let query = find_cubes_base(ju, ids);
    // --------------------------------
    // 2. レコードの取得
    // --------------------------------
    let model = query
        .filter(cubes::Column::Id.eq(cube_id))
        .one(conn)
        .await
        .map_err(|e| ApiError::new_system(
            ST_INTERNAL_SERVER_ERROR,
            rterr::ERR_DATABASE,
            format!("Fetch cube error: {}", e),
        ))?
        .ok_or_else(|| ApiError::new_system(
            ST_NOT_FOUND,
            rterr::ERR_INVALID_REQUEST,
            "Cube not found.",
        ))?;
    log::debug!("<CubesBl> get_cube: Found cube: {}", cube_id);
    // --------------------------------
    // 3. レスポンス変換
    // --------------------------------
    Ok(SearchCubesResItem {
        cube: CubeRes::from(model),
        lineage: vec![],         // TODO: LadybugDB 連携後に実装
        memory_groups: vec![],   // TODO: LadybugDB 連携後に実装
    })
}

// ============================================================
// Create
// ============================================================
pub async fn create_cube(
    conn: &DatabaseConnection,
    ju: &JwtUsr,
    ids: &JwtIDs,
    req: CreateCubeReq,
) -> Result<CreateCubeRes, ApiError> {
    log::debug!("<CubesBl> create_cube: Creating new cube.");
    // --------------------------------
    // 1. UUID の生成
    // --------------------------------
    let cube_uuid = Uuid::new_v4().to_string();
    log::debug!("<CubesBl> create_cube: Generated UUID: {}", cube_uuid);
    // --------------------------------
    // 2. ActiveModel 作成
    // --------------------------------
    // Note: Go版では APIキーを暗号化しているが、初期実装ではまず基本的な CRUD を確立するため、
    // 暗号化は後のステップで追加する。ここではプレーンテキストで保存。
    let active = cubes::ActiveModel {
        uuid: Set(cube_uuid.clone()),
        usr_id: Set(ju.usr_id as i32),
        name: Set(req.name),
        description: Set(req.description.unwrap_or_default()),
        embedding_provider: Set(req.embedding_provider),
        embedding_model: Set(req.embedding_model),
        embedding_dimension: Set(req.embedding_dimension),
        embedding_base_url: Set(req.embedding_base_url.unwrap_or_default()),
        embedding_api_key: Set(req.embedding_api_key),
        apx_id: Set(ids.apx_id as i32),
        vdr_id: Set(ids.vdr_id as i32),
        ..Default::default()
    };
    // --------------------------------
    // 3. 保存
    // --------------------------------
    let model = active
        .insert(conn)
        .await
        .map_err(|e| ApiError::new_system(
            ST_INTERNAL_SERVER_ERROR,
            rterr::ERR_DATABASE,
            format!("Insert cube error: {}", e),
        ))?;
    log::debug!("<CubesBl> create_cube: Success. ID: {}, UUID: {}", model.id, model.uuid);
    // --------------------------------
    // 4. TODO: LadybugDB ファイル作成 (将来実装予定)
    // --------------------------------
    // --------------------------------
    // 5. 最終レスポンス
    // --------------------------------
    Ok(CreateCubeRes {
        id: model.id,
        uuid: model.uuid,
    })
}

// ============================================================
// Update
// ============================================================
// TODO: 初期実装では基本 CRUD のみ。Update は後で追加。

// ============================================================
// Delete
// ============================================================
pub async fn delete_cube(
    conn: &DatabaseConnection,
    ju: &JwtUsr,
    ids: &JwtIDs,
    cube_id: u32,
) -> Result<DeleteCubeRes, ApiError> {
    log::debug!("<CubesBl> delete_cube: Fetching target cube: {}", cube_id);
    // --------------------------------
    // 1. クエリの基本形を取得して存在確認
    // --------------------------------
    let model = find_cubes_base(ju, ids)
        .filter(cubes::Column::Id.eq(cube_id as i32))
        .one(conn)
        .await
        .map_err(|e| ApiError::new_system(
            ST_INTERNAL_SERVER_ERROR,
            rterr::ERR_DATABASE,
            format!("Fetch cube error: {}", e),
        ))?
        .ok_or_else(|| ApiError::new_system(
            ST_NOT_FOUND,
            rterr::ERR_INVALID_REQUEST,
            "Cube not found.",
        ))?;
    log::debug!("<CubesBl> delete_cube: Found target cube. Deleting.");
    // --------------------------------
    // 2. TODO: LadybugDB ファイルの削除 (将来実装予定)
    // --------------------------------
    // --------------------------------
    // 3. 削除の実行
    // --------------------------------
    model
        .delete(conn)
        .await
        .map_err(|e| ApiError::new_system(
            ST_INTERNAL_SERVER_ERROR,
            rterr::ERR_DATABASE,
            format!("Delete cube error: {}", e),
        ))?;
    log::debug!("<CubesBl> delete_cube: Success.");
    // --------------------------------
    // 4. 最終レスポンス
    // --------------------------------
    Ok(DeleteCubeRes { success: true })
}
