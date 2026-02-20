//! ChatModel ビジネスロジック
//!
//! CRUD順序: Search -> Get -> Create -> Update -> Delete

use crate::constants::{ST_INTERNAL_SERVER_ERROR, ST_NOT_FOUND};
use crate::entities::chat_models;
use crate::mode::rt::rterr::rterr;
use crate::mode::rt::rtreq::chat_models_req::{
    CreateChatModelReq, SearchChatModelsReq, UpdateChatModelReq,
};
use crate::mode::rt::rtres::chat_models_res::{
    ChatModelRes, CreateChatModelRes, DeleteChatModelRes, SearchChatModelsRes, UpdateChatModelRes,
};
use crate::mode::rt::rtres::errs_res::ApiError;
use crate::utils::jwt::{JwtIDs, JwtUsr};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, ModelTrait,
    QueryFilter, QuerySelect, Select, Set,
};

// ============================================================
// Private Helper for Search and Get
// ============================================================
/// 権限に基づいた共通のクエリベースを作成する
/// ChatModel は USR のみが使用可能であり、apx_id と vdr_id でパーティショニングされる
fn find_chat_models_base(_ju: &JwtUsr, ids: &JwtIDs) -> Select<chat_models::Entity> {
    log::debug!(
        "<ChatModelsBl> find_chat_models_base: Filter apx_id: {}, vdr_id: {}",
        ids.apx_id,
        ids.vdr_id
    );
    chat_models::Entity::find()
        .filter(chat_models::Column::ApxId.eq(ids.apx_id as i32))
        .filter(chat_models::Column::VdrId.eq(ids.vdr_id as i32))
}

// ============================================================
// Search
// ============================================================
pub async fn search_chat_models(
    conn: &DatabaseConnection,
    ju: &JwtUsr,
    ids: &JwtIDs,
    req: SearchChatModelsReq,
) -> Result<SearchChatModelsRes, ApiError> {
    // --------------------------------
    // 1. クエリの基本形を取得
    // --------------------------------
    log::debug!("<ChatModelsBl> search_chat_models: Constructing base query.");
    let mut query = find_chat_models_base(ju, ids);
    // --------------------------------
    // 2. 検索条件（LIKE検索）
    // --------------------------------
    if let Some(ref name) = req.name {
        if !name.is_empty() {
            log::debug!(
                "<ChatModelsBl> search_chat_models: Filter by name: {}",
                name
            );
            query = query.filter(chat_models::Column::Name.contains(name));
        }
    }
    if let Some(ref provider) = req.provider {
        if !provider.is_empty() {
            log::debug!(
                "<ChatModelsBl> search_chat_models: Filter by provider: {}",
                provider
            );
            query = query.filter(chat_models::Column::Provider.contains(provider));
        }
    }
    if let Some(ref model) = req.model {
        if !model.is_empty() {
            log::debug!(
                "<ChatModelsBl> search_chat_models: Filter by model: {}",
                model
            );
            query = query.filter(chat_models::Column::Model.contains(model));
        }
    }
    if let Some(ref base_url) = req.base_url {
        if !base_url.is_empty() {
            log::debug!(
                "<ChatModelsBl> search_chat_models: Filter by base_url: {}",
                base_url
            );
            query = query.filter(chat_models::Column::BaseUrl.contains(base_url));
        }
    }
    // --------------------------------
    // 3. データの取得
    // --------------------------------
    log::debug!(
        "<ChatModelsBl> search_chat_models: Fetching records. limit: {}, offset: {}",
        req.limit,
        req.offset
    );
    let models = query
        .offset(req.offset as u64)
        .limit(req.limit as u64)
        .all(conn)
        .await
        .map_err(|e| {
            ApiError::new_system(
                ST_INTERNAL_SERVER_ERROR,
                rterr::ERR_DATABASE,
                format!("Search query error: {}", e),
            )
        })?;
    log::debug!(
        "<ChatModelsBl> search_chat_models: Found {} records.",
        models.len()
    );
    // --------------------------------
    // 4. DBデータのレスポンス用変換
    // --------------------------------
    let data = models.into_iter().map(ChatModelRes::from).collect();
    // --------------------------------
    // 5. 最終レスポンス
    // --------------------------------
    Ok(SearchChatModelsRes { data })
}

// ============================================================
// Get
// ============================================================
pub async fn get_chat_model(
    conn: &DatabaseConnection,
    ju: &JwtUsr,
    ids: &JwtIDs,
    chat_model_id: i32,
) -> Result<ChatModelRes, ApiError> {
    // --------------------------------
    // 1. クエリの基本形を取得
    // --------------------------------
    log::debug!(
        "<ChatModelsBl> get_chat_model: Fetching chat_model: {}",
        chat_model_id
    );
    let query = find_chat_models_base(ju, ids);
    // --------------------------------
    // 2. レコードの取得
    // --------------------------------
    let model = query
        .filter(chat_models::Column::Id.eq(chat_model_id))
        .one(conn)
        .await
        .map_err(|e| {
            ApiError::new_system(
                ST_INTERNAL_SERVER_ERROR,
                rterr::ERR_DATABASE,
                format!("Fetch chat_model error: {}", e),
            )
        })?
        .ok_or_else(|| {
            ApiError::new_system(
                ST_NOT_FOUND,
                rterr::ERR_INVALID_REQUEST,
                "Chat model not found.",
            )
        })?;
    log::debug!(
        "<ChatModelsBl> get_chat_model: Found chat_model: {}",
        chat_model_id
    );
    Ok(ChatModelRes::from(model))
}

// ============================================================
// Create
// ============================================================
pub async fn create_chat_model(
    conn: &DatabaseConnection,
    _ju: &JwtUsr,
    ids: &JwtIDs,
    req: CreateChatModelReq,
) -> Result<CreateChatModelRes, ApiError> {
    log::debug!("<ChatModelsBl> create_chat_model: Creating new chat_model.");
    // --------------------------------
    // 1. ActiveModel 作成
    // --------------------------------
    // Note: Go版では APIキーを暗号化しているが、初期実装ではまず基本的な CRUD を確立するため、
    // 暗号化は後のステップで追加する。ここではプレーンテキストで保存。
    let active = chat_models::ActiveModel {
        name: Set(req.name),
        provider: Set(req.provider),
        model: Set(req.model),
        base_url: Set(req.base_url.unwrap_or_default()),
        api_key: Set(req.api_key),
        max_tokens: Set(req.max_tokens),
        temperature: Set(req.temperature),
        apx_id: Set(ids.apx_id as i32),
        vdr_id: Set(ids.vdr_id as i32),
        ..Default::default()
    };
    // --------------------------------
    // 2. 保存
    // --------------------------------
    let model = active.insert(conn).await.map_err(|e| {
        ApiError::new_system(
            ST_INTERNAL_SERVER_ERROR,
            rterr::ERR_DATABASE,
            format!("Insert chat_model error: {}", e),
        )
    })?;
    log::debug!(
        "<ChatModelsBl> create_chat_model: Success. ID: {}",
        model.id
    );
    // --------------------------------
    // 3. 最終レスポンス
    // --------------------------------
    Ok(CreateChatModelRes { id: model.id })
}

// ============================================================
// Update
// ============================================================
pub async fn update_chat_model(
    conn: &DatabaseConnection,
    ju: &JwtUsr,
    ids: &JwtIDs,
    chat_model_id: i32,
    req: UpdateChatModelReq,
) -> Result<UpdateChatModelRes, ApiError> {
    log::debug!(
        "<ChatModelsBl> update_chat_model: Fetching target chat_model: {}",
        chat_model_id
    );
    // --------------------------------
    // 1. クエリの基本形を取得して存在確認
    // --------------------------------
    let model = find_chat_models_base(ju, ids)
        .filter(chat_models::Column::Id.eq(chat_model_id))
        .one(conn)
        .await
        .map_err(|e| {
            ApiError::new_system(
                ST_INTERNAL_SERVER_ERROR,
                rterr::ERR_DATABASE,
                format!("Fetch chat_model error: {}", e),
            )
        })?
        .ok_or_else(|| {
            ApiError::new_system(
                ST_NOT_FOUND,
                rterr::ERR_INVALID_REQUEST,
                "Chat model not found.",
            )
        })?;
    log::debug!(
        "<ChatModelsBl> update_chat_model: Found target chat_model. Processing field updates."
    );
    // --------------------------------
    // 2. 更新用 ActiveModel の準備
    // --------------------------------
    let mut active = model.into_active_model();
    // --------------------------------
    // 3. 各フィールドの更新 (存在する場合のみ)
    // --------------------------------
    if let Some(name) = req.name {
        if !name.is_empty() {
            active.name = Set(name);
        }
    }
    if let Some(provider) = req.provider {
        if !provider.is_empty() {
            active.provider = Set(provider);
        }
    }
    if let Some(model_name) = req.model {
        if !model_name.is_empty() {
            active.model = Set(model_name);
        }
    }
    if let Some(base_url) = req.base_url {
        if !base_url.is_empty() {
            active.base_url = Set(base_url);
        }
    }
    if let Some(api_key) = req.api_key {
        if !api_key.is_empty() {
            // Note: 暗号化は後のステップで追加
            active.api_key = Set(api_key);
        }
    }
    if let Some(max_tokens) = req.max_tokens {
        if max_tokens > 0 {
            active.max_tokens = Set(max_tokens);
        }
    }
    if let Some(temperature) = req.temperature {
        active.temperature = Set(temperature);
    }
    // --------------------------------
    // 4. 保存
    // --------------------------------
    log::debug!("<ChatModelsBl> update_chat_model: Saving changes to DB.");
    active.update(conn).await.map_err(|e| {
        ApiError::new_system(
            ST_INTERNAL_SERVER_ERROR,
            rterr::ERR_DATABASE,
            format!("Update chat_model error: {}", e),
        )
    })?;
    log::debug!("<ChatModelsBl> update_chat_model: Success.");
    // --------------------------------
    // 5. 最終レスポンス
    // --------------------------------
    Ok(UpdateChatModelRes { success: true })
}

// ============================================================
// Delete
// ============================================================
pub async fn delete_chat_model(
    conn: &DatabaseConnection,
    ju: &JwtUsr,
    ids: &JwtIDs,
    chat_model_id: i32,
) -> Result<DeleteChatModelRes, ApiError> {
    log::debug!(
        "<ChatModelsBl> delete_chat_model: Fetching target chat_model: {}",
        chat_model_id
    );
    // --------------------------------
    // 1. クエリの基本形を取得して存在確認
    // --------------------------------
    let model = find_chat_models_base(ju, ids)
        .filter(chat_models::Column::Id.eq(chat_model_id))
        .one(conn)
        .await
        .map_err(|e| {
            ApiError::new_system(
                ST_INTERNAL_SERVER_ERROR,
                rterr::ERR_DATABASE,
                format!("Fetch chat_model error: {}", e),
            )
        })?
        .ok_or_else(|| {
            ApiError::new_system(
                ST_NOT_FOUND,
                rterr::ERR_INVALID_REQUEST,
                "Chat model not found.",
            )
        })?;
    log::debug!("<ChatModelsBl> delete_chat_model: Found target chat_model. Deleting.");
    // --------------------------------
    // 2. 削除の実行
    // --------------------------------
    model.delete(conn).await.map_err(|e| {
        ApiError::new_system(
            ST_INTERNAL_SERVER_ERROR,
            rterr::ERR_DATABASE,
            format!("Delete chat_model error: {}", e),
        )
    })?;
    log::debug!("<ChatModelsBl> delete_chat_model: Success.");
    // --------------------------------
    // 3. 最終レスポンス
    // --------------------------------
    Ok(DeleteChatModelRes { success: true })
}
