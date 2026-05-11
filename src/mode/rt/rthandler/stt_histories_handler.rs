use crate::entities::stt_histories;
use crate::mode::rt::rtres::errs_res::ApiError;
use crate::mode::rt::rtutils::db_for_rt::DbPoolsExt;
use crate::utils::db::DbPools;
use axum::{Extension, Json};
use sea_orm::{EntityTrait, QueryOrder};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;

const TAG: &str = "v1 STT";

// ============================================================
// Response 構造体
// ============================================================

/// 音声認識履歴の1件分のデータを表す。
#[derive(Serialize, Deserialize, ToSchema)]
pub struct SttHistoryItem {
    /// レコードの一意識別子（自動採番）
    pub id: i32,
    /// 認識されたテキスト本文
    pub text: String,
    /// レコード作成日時（ISO 8601 文字列）
    pub created_at: String,
}

/// STT 履歴一覧取得のレスポンスを表す。
#[derive(Serialize, ToSchema)]
pub struct GetSttHistoryRes {
    /// 履歴データの配列（新しい順）
    pub histories: Vec<SttHistoryItem>,
}

/// STT 履歴全件削除のレスポンスを表す。
#[derive(Serialize, ToSchema)]
pub struct DeleteSttHistoryRes {
    /// 削除が成功したかどうか
    pub success: bool,
}

// ============================================================
// GET /stt/history
// ============================================================

const GET_STT_HISTORY_DESC: &str = r#"
### ⚫︎ 概要
- 音声認識（STT）の履歴を全件、新しい順に取得します。
- データベースに保存されている全てのレコードが返却されます。
- 最大保存件数は50件で、古いものから自動的に削除されます。

### ⚫︎ 権限
- パブリック（認証不要）。ローカルアプリケーション内からのみアクセス可能です。

### ⚫︎ Response
| KEY | TYPE | DESCRIPTION |
| --- | --- | --- |
| `histories` | array | 履歴データの配列 |
| `histories[].id` | number | レコードの一意識別子 |
| `histories[].text` | string | 認識されたテキスト本文 |
| `histories[].created_at` | string | レコード作成日時（ISO 8601） |
"#;

/// STT認識履歴を全件取得する（新しい順）。
#[utoipa::path(
    get,
    path = "/stt/history",
    tag = TAG,
    summary = "STT 認識履歴を全件取得する。",
    description = GET_STT_HISTORY_DESC,
    responses(
        (status = 200, description = "Success", body = GetSttHistoryRes),
        (status = 500, description = "Internal Server Error", body = ApiError),
    )
)]
pub async fn get_stt_history(
    Extension(db): Extension<Arc<DbPools>>,
) -> Result<Json<GetSttHistoryRes>, ApiError> {
    let conn = db.get_ro_for_rt()?;

    let models = stt_histories::Entity::find()
        .order_by_desc(stt_histories::Column::CreatedAt)
        .all(conn)
        .await
        .map_err(|e| {
            log::error!("Failed to fetch STT history: {}", e);
            ApiError::from(e)
        })?;

    let histories: Vec<SttHistoryItem> = models
        .into_iter()
        .map(|m| SttHistoryItem {
            id: m.id,
            text: m.text,
            created_at: m.created_at.to_string(),
        })
        .collect();

    Ok(Json(GetSttHistoryRes { histories }))
}

// ============================================================
// DELETE /stt/history
// ============================================================

const DELETE_STT_HISTORY_DESC: &str = r#"
### ⚫︎ 概要
- 音声認識（STT）の履歴を全て削除します。
- この操作は元に戻せません。

### ⚫︎ 権限
- パブリック（認証不要）。ローカルアプリケーション内からのみアクセス可能です。

### ⚫︎ Response
| KEY | TYPE | DESCRIPTION |
| --- | --- | --- |
| `success` | boolean | 削除が成功したかどうか |
"#;

/// STT認識履歴を全件削除する。
#[utoipa::path(
    delete,
    path = "/stt/history",
    tag = TAG,
    summary = "STT 認識履歴を全件削除する。",
    description = DELETE_STT_HISTORY_DESC,
    responses(
        (status = 200, description = "Success", body = DeleteSttHistoryRes),
        (status = 500, description = "Internal Server Error", body = ApiError),
    )
)]
pub async fn delete_stt_history(
    Extension(db): Extension<Arc<DbPools>>,
) -> Result<Json<DeleteSttHistoryRes>, ApiError> {
    let conn = db.get_rw_for_rt()?;

    stt_histories::Entity::delete_many()
        .exec(conn)
        .await
        .map_err(|e| {
            log::error!("Failed to delete STT history: {}", e);
            ApiError::from(e)
        })?;

    Ok(Json(DeleteSttHistoryRes { success: true }))
}
