use crate::constants::ST_INTERNAL_SERVER_ERROR;
use crate::{
    entities::apps,
    mode::rt::{
        rterr::rterr,
        rtres::{
            errs_res::ApiError,
            pub_apps_res::{AppInfoPubItemRes, ListAppsPubRes},
        },
    },
};
use sea_orm::{DatabaseConnection, EntityTrait};

// ============================================================
// 公開アプリ一覧の取得 (Public)
// ============================================================
pub async fn list_apps_pub(conn: &DatabaseConnection) -> Result<ListAppsPubRes, ApiError> {
    log::debug!("<PubApps> list_apps_pub called.");

    let apps_records = apps::Entity::find().all(conn).await.map_err(|e| {
        ApiError::new_system(ST_INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, e.to_string())
    })?;

    let items = apps_records
        .into_iter()
        .map(|a| {
            // マニフェストデータの正規化 (Struct -> JSON) を通じて、
            // 署名検証時と同一のシリアライズ結果を保証する。
            let manifest_val = a.manifest_data.clone();
            let normalized_manifest = manifest_val
                .and_then(|v| {
                    let m: Result<crate::utils::pkg_bl::MyCuteManifest, _> =
                        serde_json::from_value(v);
                    m.ok()
                })
                .and_then(|m| serde_json::to_value(m).ok());

            AppInfoPubItemRes {
                global_app_id: a.global_app_id.to_string(),
                global_app_version: a.global_app_version,
                name: a.name,
                author: a.author.unwrap_or_default(),
                description: normalized_manifest
                    .as_ref()
                    .and_then(|m| m.get("description"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string()),
                dev_public_key: a.dev_public_key,
                verifications: a.verifications,
                verification_results_cache: a.verification_results_cache,
                manifest_data: normalized_manifest,
                created_at: a.created_at.to_string(),
                updated_at: a.updated_at.to_string(),
            }
        })
        .collect();

    Ok(ListAppsPubRes { items })
}
