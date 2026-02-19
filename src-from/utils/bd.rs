use anyhow::{Context, Result};
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, QuerySelect, FromQueryResult, sea_query::Expr};
use crate::entities::bds;
use crate::utils::crypto::verify_hash;

#[derive(FromQueryResult)]
struct BdsHash {
    hash: String,
}

/// DBから有効なハッシュを取得し、検証を行う
/// CPU負荷が高いため内部で spawn_blocking を使用する
pub async fn is_valid_bd(conn: &DatabaseConnection, bd: String) -> Result<bool> {
    if bd.is_empty() {
        return Ok(false);
    }

    // 1. DBから有効なハッシュレコードを取得
    let hashes: Vec<String> = bds::Entity::find()
        .select_only()
        .column(bds::Column::Hash)
        .filter(Expr::col(bds::Column::BgnAt).lte(Expr::current_timestamp()))
        .filter(Expr::col(bds::Column::EndAt).gte(Expr::current_timestamp()))
        .into_model::<BdsHash>()
        .all(conn)
        .await
        .context("Failed to fetch BD hashes from DB")?
        .into_iter()
        .map(|r| r.hash)
        .collect();

    if hashes.is_empty() {
        return Ok(false);
    }

    // 2. ハッシュ検証（ブロッキング処理）
    tokio::task::spawn_blocking(move || {
        hashes.iter().any(|h| verify_hash(&bd, h).unwrap_or(false))
    })
    .await
    .context("Failed to join blocking task in is_valid_bd")
}