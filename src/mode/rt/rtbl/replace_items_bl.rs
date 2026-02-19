use crate::entities::{prelude::*, replace_items, replaces};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, Set, QueryOrder, PaginatorTrait, ModelTrait,
    sea_query::{Expr, Func},
};
use sea_orm::ActiveValue::NotSet;
use serde_json::json;
use crate::utils::time::now;
use crate::mode::rt::rtreq::replace_items_req::{SearchReplaceItemsReq, CreateReplaceItemReq, UpdateReplaceItemReq};
use crate::mode::rt::rtres::replace_items_res::ReplaceItemDetail;

// ===========================================
// Replace Items Logic
// ===========================================

pub async fn search_items<C>(db: &C, apx_id: u32, vdr_id: u32, req: SearchReplaceItemsReq) -> Result<(Vec<ReplaceItemDetail>, u64), DbErr>
where
    C: ConnectionTrait,
{
    // 親セットがユーザーに属していることを確認
    let _ = Replaces::find()
        .filter(replaces::Column::ApxId.eq(apx_id))
        .filter(replaces::Column::VdrId.eq(vdr_id))
        .filter(replaces::Column::Id.eq(req.replace_id))
        .one(db)
        .await?
        .ok_or(DbErr::RecordNotFound("Parent replaces not found or access denied".to_string()))?;

    let mut query = ReplaceItems::find()
        .filter(replace_items::Column::ReplaceId.eq(req.replace_id))
        .filter(replace_items::Column::ApxId.eq(apx_id as i32))
        .filter(replace_items::Column::VdrId.eq(vdr_id as i32));

    if let Some(k) = req.keyword {
         let lower_k = format!("%{}%", k.to_lowercase());
         query = query.filter(Expr::expr(Func::lower(Expr::col(replace_items::Column::Key))).like(lower_k));
    }

    let paginator = query.order_by_asc(replace_items::Column::Rank).paginate(db, req.limit);
    let total = paginator.num_items().await?;
    let items = paginator.fetch_page(req.offset).await?;

    let list = items.into_iter().map(|m| {
        let texts_vec: Vec<String> = serde_json::from_value(m.texts).unwrap_or_default();
        ReplaceItemDetail {
            id: m.id,
            replace_id: m.replace_id,
            key: m.key,
            texts: texts_vec,
            rank: m.rank,
        }
    }).collect();

    Ok((list, total))
}

pub async fn create_item<C>(db: &C, apx_id: u32, vdr_id: u32, req: CreateReplaceItemReq) -> Result<i32, DbErr>
where
    C: ConnectionTrait,
{
    // 権限チェック
    let _ = Replaces::find()
        .filter(replaces::Column::ApxId.eq(apx_id))
        .filter(replaces::Column::VdrId.eq(vdr_id))
        .filter(replaces::Column::Id.eq(req.replace_id))
        .one(db)
        .await?
        .ok_or(DbErr::RecordNotFound("Parent replaces not found".to_string()))?;

    let now = now();
    let active = replace_items::ActiveModel {
        id: NotSet,
        replace_id: Set(req.replace_id),
        apx_id: Set(apx_id as i32),
        vdr_id: Set(vdr_id as i32),
        key: Set(req.key),
        texts: Set(json!(req.texts)),
        rank: Set(req.rank),
        created_at: Set(now),
        updated_at: Set(now),
    };
    active.insert(db).await.map(|m| m.id)
}

pub async fn update_item<C>(db: &C, apx_id: u32, vdr_id: u32, id: i32, req: UpdateReplaceItemReq) -> Result<(), DbErr>
where
    C: ConnectionTrait,
{
    let item = ReplaceItems::find_by_id(id)
        .filter(replace_items::Column::ApxId.eq(apx_id as i32))
        .filter(replace_items::Column::VdrId.eq(vdr_id as i32))
        .one(db)
        .await?
        .ok_or(DbErr::RecordNotFound(format!("Item {} not found", id)))?;

    let mut active: replace_items::ActiveModel = item.into();
    if let Some(k) = req.key {
        active.key = Set(k);
    }
    if let Some(t) = req.texts {
        active.texts = Set(json!(t));
    }
    if let Some(r) = req.rank {
        active.rank = Set(r);
    }
    active.updated_at = Set(now());
    active.update(db).await?;
    Ok(())
}

pub async fn delete_item<C>(db: &C, apx_id: u32, vdr_id: u32, id: i32) -> Result<(), DbErr>
where
    C: ConnectionTrait,
{
    let item = ReplaceItems::find_by_id(id)
        .filter(replace_items::Column::ApxId.eq(apx_id as i32))
        .filter(replace_items::Column::VdrId.eq(vdr_id as i32))
        .one(db)
        .await?
        .ok_or(DbErr::RecordNotFound(format!("Item {} not found", id)))?;
    
    item.delete(db).await?;
    Ok(())
}

pub async fn get_item_by_id<C>(db: &C, id: i32) -> Result<Option<replace_items::Model>, DbErr>
where
    C: ConnectionTrait,
{
    ReplaceItems::find_by_id(id).one(db).await
}
