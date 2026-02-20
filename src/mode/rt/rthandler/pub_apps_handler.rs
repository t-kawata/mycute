use crate::{
    mode::rt::{
        rtbl::pub_apps_bl,
        rtres::{errs_res::ApiError, pub_apps_res::ListAppsPubRes},
        rtutils::db_for_rt::DbPoolsExt,
    },
    utils::db::DbPools,
    TAG_MACRO_P2P_OPTIONAL,
};
use axum::{Extension, Json};
use std::sync::Arc;

macro_rules! TAG_NAME {
    () => {
        "v1 Pub Apps"
    };
}
const TAG_P2P_OPTIONAL: &str = concat!(TAG_NAME!(), " ", TAG_MACRO_P2P_OPTIONAL!());

// ============================================================
// List Apps (Public)
// ============================================================
const LIST_DESC: &str = r#"
### ⚫︎ 概要
- ノードにインストールされているアプリの一覧を公開する。
- 認証は不要であり、誰でも閲覧可能。
- CA による情報収集にも使用される。

### ⚫︎ 権限
- **Public**: 誰でも実行可能。

### ⚫︎ Response
| TYPE | DESCRIPTION |
| --- | --- |
| Object (`ListAppsPubRes`) | アプリ一覧 |
"#;
#[utoipa::path(
    tag = TAG_P2P_OPTIONAL,
    get,
    path = "/pub/apps/list",
    summary = "インストール済みアプリ一覧をパブリックに公開する。",
    description = LIST_DESC,
    responses(
        (status = 200, description = "Success", body = ListAppsPubRes),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn list_apps_pub(
    Extension(db): Extension<Arc<DbPools>>,
) -> Result<Json<ListAppsPubRes>, ApiError> {
    let conn = db.get_ro_for_rt()?;
    let res = pub_apps_bl::list_apps_pub(conn).await?;
    Ok(Json(res))
}
