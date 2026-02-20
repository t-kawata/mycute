use crate::{
    entities::forums,
    mode::rt::{
        rterr::rterr,
        rtreq::forums_req::{CreateForumReq, SearchForumsReq, UpdateForumReq},
        rtres::{
            errs_res::ApiError,
            forums_res::{
                CreateForumRes, DeleteForumRes, GetForumRes, SearchForumsRes, SearchForumsResItem,
                UpdateForumRes,
            },
        },
    },
    utils::time,
};
use axum::http::StatusCode;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, EntityTrait, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, Set,
};
use uuid::Uuid;

pub async fn search_forums<C>(conn: &C, req: SearchForumsReq) -> Result<SearchForumsRes, ApiError>
where
    C: ConnectionTrait,
{
    let mut query = forums::Entity::find().filter(forums::Column::DeletedAt.is_null());

    if !req.keyword.is_empty() {
        query = query.filter(
            Condition::any()
                .add(forums::Column::Name.contains(&req.keyword))
                .add(forums::Column::Description.contains(&req.keyword)),
        );
    }

    let total = query
        .clone()
        .count(conn)
        .await
        .map_err(|e: sea_orm::DbErr| {
            ApiError::new_system(
                StatusCode::INTERNAL_SERVER_ERROR,
                rterr::ERR_DATABASE,
                e.to_string(),
            )
        })?;

    let models: Vec<forums::Model> = query
        .order_by_desc(forums::Column::CreatedAt)
        .offset(Some(req.offset as u64))
        .limit(Some(req.limit as u64))
        .all(conn)
        .await
        .map_err(|e: sea_orm::DbErr| {
            ApiError::new_system(
                StatusCode::INTERNAL_SERVER_ERROR,
                rterr::ERR_DATABASE,
                e.to_string(),
            )
        })?;

    let forums = models.into_iter().map(SearchForumsResItem::from).collect();
    Ok(SearchForumsRes { total, forums })
}

pub async fn get_forum<C>(conn: &C, id: String) -> Result<GetForumRes, ApiError>
where
    C: ConnectionTrait,
{
    let uuid = Uuid::parse_str(&id).map_err(|_| {
        ApiError::new_system(
            StatusCode::BAD_REQUEST,
            rterr::ERR_INVALID_REQUEST,
            "Invalid verify UUID".to_string(),
        )
    })?;

    let model = forums::Entity::find_by_id(uuid)
        .one(conn)
        .await
        .map_err(|e| {
            ApiError::new_system(
                StatusCode::INTERNAL_SERVER_ERROR,
                rterr::ERR_DATABASE,
                e.to_string(),
            )
        })?
        .ok_or_else(|| {
            ApiError::new_system(
                StatusCode::NOT_FOUND,
                rterr::ERR_NOT_FOUND,
                "Forum not found".to_string(),
            )
        })?;

    Ok(GetForumRes::from(model))
}

pub async fn create_forum<C>(conn: &C, req: CreateForumReq) -> Result<CreateForumRes, ApiError>
where
    C: ConnectionTrait,
{
    let now = time::now();
    let uuid = Uuid::new_v4();
    let active_model = forums::ActiveModel {
        id: Set(uuid),
        name: Set(req.name),
        description: Set(req.description),
        initial_balance: Set(req.initial_balance),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };

    let model = active_model.insert(conn).await.map_err(|e| {
        ApiError::new_system(
            StatusCode::INTERNAL_SERVER_ERROR,
            rterr::ERR_DATABASE,
            e.to_string(),
        )
    })?;

    Ok(CreateForumRes::from(model))
}

pub async fn update_forum<C>(
    conn: &C,
    id: String,
    req: UpdateForumReq,
) -> Result<UpdateForumRes, ApiError>
where
    C: ConnectionTrait,
{
    let uuid = Uuid::parse_str(&id).map_err(|_| {
        ApiError::new_system(
            StatusCode::BAD_REQUEST,
            rterr::ERR_INVALID_REQUEST,
            "Invalid UUID".to_string(),
        )
    })?;

    let model = forums::Entity::find_by_id(uuid)
        .one(conn)
        .await
        .map_err(|e| {
            ApiError::new_system(
                StatusCode::INTERNAL_SERVER_ERROR,
                rterr::ERR_DATABASE,
                e.to_string(),
            )
        })?
        .ok_or_else(|| {
            ApiError::new_system(
                StatusCode::NOT_FOUND,
                rterr::ERR_NOT_FOUND,
                "Forum not found".to_string(),
            )
        })?;

    let mut active_model: forums::ActiveModel = model.into();
    let now = time::now();

    if let Some(name) = req.name {
        active_model.name = Set(name);
    }
    if let Some(description) = req.description {
        active_model.description = Set(description);
    }

    active_model.updated_at = Set(now);

    let model = active_model.update(conn).await.map_err(|e| {
        ApiError::new_system(
            StatusCode::INTERNAL_SERVER_ERROR,
            rterr::ERR_DATABASE,
            e.to_string(),
        )
    })?;

    Ok(UpdateForumRes::from(model))
}

pub async fn delete_forum<C>(conn: &C, id: String) -> Result<DeleteForumRes, ApiError>
where
    C: ConnectionTrait,
{
    let uuid = Uuid::parse_str(&id).map_err(|_| {
        ApiError::new_system(
            StatusCode::BAD_REQUEST,
            rterr::ERR_INVALID_REQUEST,
            "Invalid UUID".to_string(),
        )
    })?;

    // 削除対象が存在するか確認（既に論理削除されている場合も除外）
    let model = forums::Entity::find()
        .filter(forums::Column::Id.eq(uuid))
        .filter(forums::Column::DeletedAt.is_null())
        .one(conn)
        .await
        .map_err(|e| {
            ApiError::new_system(
                StatusCode::INTERNAL_SERVER_ERROR,
                rterr::ERR_DATABASE,
                e.to_string(),
            )
        })?
        .ok_or_else(|| {
            ApiError::new_system(
                StatusCode::NOT_FOUND,
                rterr::ERR_NOT_FOUND,
                "Forum not found".to_string(),
            )
        })?;

    let mut active_model: forums::ActiveModel = model.into();
    let now = time::now();
    active_model.deleted_at = Set(Some(now));
    active_model.updated_at = Set(now);

    active_model.update(conn).await.map_err(|e| {
        ApiError::new_system(
            StatusCode::INTERNAL_SERVER_ERROR,
            rterr::ERR_DATABASE,
            e.to_string(),
        )
    })?;

    Ok(DeleteForumRes { id })
}
