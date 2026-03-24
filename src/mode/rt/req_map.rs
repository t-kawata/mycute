use crate::cuber::service::CuberService;
use crate::mode::rt::client::secure_client::SecureClient;
use crate::mode::rt::middleware::p2p_clock_sync_enforcement_middleware;
use crate::mode::rt::rthandler::bds_handler::*;
use crate::mode::rt::rthandler::ca_apps_handler::*;
use crate::mode::rt::rthandler::ca_blacklists_handler::*;
use crate::mode::rt::rthandler::ca_handler::*;
use crate::mode::rt::rthandler::ca_identities_handler::*;
use crate::mode::rt::rthandler::chat_models_handler::*;
use crate::mode::rt::rthandler::cryptos_handler::*;
use crate::mode::rt::rthandler::cubes_handler::*;
use crate::mode::rt::rthandler::forums_handler::*;
use crate::mode::rt::rthandler::health_handler::*;
use crate::mode::rt::rthandler::mycute_handler::*;
use crate::mode::rt::rthandler::mycute_proxy_leaks_handler::*;
use crate::mode::rt::rthandler::node_apps_handler::*;
use crate::mode::rt::rthandler::node_blacklists_handler::*;
use crate::mode::rt::rthandler::node_identities_handler::*;
use crate::mode::rt::rthandler::osca_handler::*;
use crate::mode::rt::rthandler::owner_handler::*;
use crate::mode::rt::rthandler::pub_apps_handler::*;
use crate::mode::rt::rthandler::replace_items_handler::*;
use crate::mode::rt::rthandler::replaces_handler::*;
use crate::mode::rt::rthandler::usrs_handler::*;
use crate::mycute_settings::ConfigManager;
use crate::types::InternalEvent;
use crate::utils::jwt::JwtConfig;
use crate::{config::VERSION, utils::cors::cors_layer, utils::db::DbPools};
use axum::{Extension, Router};
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::broadcast;
use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::Modify;
use utoipa::OpenApi;
use utoipa_axum::{router::OpenApiRouter, routes};
use utoipa_swagger_ui::SwaggerUi;

// ==============================
// セキュリティアドオン作成
// ==============================
struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        // componentsがNoneの場合に備えて取り出す
        let components = openapi.components.as_mut().unwrap();
        components.add_security_scheme(
            "api_jwt_token", // この名前を後で参照
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("JWT") // 任意でフォーマットを指定
                    .build(),
            ),
        );
    }
}

// ==============================
// Swagger 共通定義
// ==============================
#[derive(OpenApi)]
#[openapi(modifiers(&SecurityAddon), info(
    title = "MYCUTE",
    version = VERSION,
    description = "## API概要\nMYCUTE REST APIを定義する。\nURL最大長のリスクを避ける為、検索は query parameter ではなく body json を使用する。\n検索は POST にて行う。",
))]
pub(crate) struct ApiDoc;

pub static RUNTIME_OPENAPI: std::sync::OnceLock<utoipa::openapi::OpenApi> =
    std::sync::OnceLock::new();

// ==============================
// Route & Handler を設定
// ==============================
fn app_routes() -> OpenApiRouter {
    OpenApiRouter::new()
        // Owner 関連
        .routes(routes!(assign_ca))
        .routes(routes!(activate_owner))
        .routes(routes!(deactivate_owner))
        .routes(routes!(get_owner_status))
        // 中央認証局、開発者、一般ユーザ、および共通エンドポイント
        .routes(routes!(create_bd_hash))
        .routes(routes!(check_bd_hash))
        .routes(routes!(auth_usr))
        .routes(routes!(search_usrs))
        .routes(routes!(get_usr))
        .routes(routes!(create_usr))
        .routes(routes!(update_usr))
        .routes(routes!(delete_usr))
        .routes(routes!(hire_usr))
        .routes(routes!(dehire_usr))
        // Replaces
        .routes(routes!(search_replaces))
        .routes(routes!(get_replaces))
        .routes(routes!(create_replaces))
        .routes(routes!(update_replaces))
        .routes(routes!(delete_replaces))
        .routes(routes!(activate_replaces))
        .routes(routes!(export_replaces))
        .routes(routes!(import_replaces))
        // Replace Items
        .routes(routes!(search_replace_items))
        .routes(routes!(create_replace_item))
        .routes(routes!(update_replace_item))
        .routes(routes!(delete_replace_item))
        // Forums
        .routes(routes!(search_forums))
        .routes(routes!(get_forum))
        .routes(routes!(create_forum))
        .routes(routes!(update_forum))
        .routes(routes!(delete_forum))
        .routes(routes!(encrypt_handler))
        .routes(routes!(decrypt_handler))
        .routes(routes!(create_vdr_token_handler))
        .routes(routes!(get_vdr_token_handler))
        .routes(routes!(search_chat_models))
        .routes(routes!(create_chat_model))
        .routes(routes!(get_chat_model))
        .routes(routes!(update_chat_model))
        .routes(routes!(delete_chat_model))
        .routes(routes!(search_cubes))
        .routes(routes!(create_cube))
        .routes(routes!(get_cube))
        .routes(routes!(delete_cube))
        .routes(routes!(absorb_cube))
        .routes(routes!(query_cube))
        .routes(routes!(memify_cube))
        .routes(routes!(export_cube))
        .routes(routes!(import_cube))
        .routes(routes!(genkey_cube))
        .routes(routes!(rekey_cube))
        .routes(routes!(create_csp_leak_report))
        .routes(routes!(create_sw_leak_report))
        .routes(routes!(get_osca_url))
        .routes(routes!(check_health))
        // MYCUTE
        .routes(routes!(get_mycute_version))
        .routes(routes!(get_mycute_home_dir))
        .routes(routes!(set_mycute_lang))
        .routes(routes!(set_mycute_stt_engine))
        .routes(routes!(get_mycute_llms))
        .routes(routes!(set_mycute_llms))
        .routes(routes!(subscribe_ws_events))
        .routes(routes!(get_ws_status))
        // CA Identities
        .routes(routes!(search_identities_ca))
        .routes(routes!(get_identity_ca))
        .routes(routes!(entry_identity_ca))
        .routes(routes!(apply_identity_ca))
        .routes(routes!(sync_identity_ca))
        .routes(routes!(verify_identity_ca))
        .routes(routes!(delete_identity_ca))
        // Node Identities
        .routes(routes!(entry_identity_node))
        .routes(routes!(apply_identity_node))
        .routes(routes!(sync_identity_node))
        .routes(routes!(get_pubkey_node))
        // CA Apps
        .routes(routes!(advertise_app_ca))
        .routes(routes!(discover_app_ca))
        .routes(routes!(vote_app_ca))
        // Node Apps
        .routes(routes!(build_app_node))
        .routes(routes!(install_app_file_node))
        .routes(routes!(verify_app_node))
        .routes(routes!(advertise_app_node))
        .routes(routes!(discover_app_node))
        .routes(routes!(vote_app_node))
        // Pub Apps
        .routes(routes!(list_apps_pub))
        // CA Blacklists
        .routes(routes!(report_blacklist_ca))
        .routes(routes!(sync_blacklists_ca))
        // Node Blacklists
        .routes(routes!(report_blacklist_node))
        .routes(routes!(sync_blacklists_node))
        // CA Transparency
        .routes(routes!(get_ca_status))
        .routes(routes!(register_ca_token_ca))
}

// ==============================
// リクエストマッピング
// ==============================
pub fn map_request(
    cors: bool,
    db: Arc<DbPools>,
    rt_skey: &str,
    rt_crypto_key: &str,
    cuber_service: Arc<CuberService>,
    sw_port: u16,
    config_manager: Arc<ConfigManager>,
    hc: Arc<reqwest::Client>,
    secure_client: Arc<SecureClient>,
    event_tx: broadcast::Sender<InternalEvent>,
    ws_clients: Arc<DashMap<String, crate::types::WsClientRole>>,
) -> Router {
    log::debug!("Mapping requests.");

    log::info!("[req_map] Registering all API endpoints including dynamic Owner routes.");

    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .nest("/v1", app_routes())
        .split_for_parts();

    // Middleware がパス判定に使うために、構築済み OpenAPI を保存
    let _ = RUNTIME_OPENAPI.set(api.clone());
    let mut app = Router::new()
        .merge(router)
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", api))
        // ------------------------------------------------------------
        // 1. ミドルウェア層 (内側)
        // ------------------------------------------------------------
        .layer(axum::middleware::from_fn(
            p2p_clock_sync_enforcement_middleware::p2p_clock_sync_enforcement_middleware,
        ))
        // ------------------------------------------------------------
        // 2. 共通 Extension 層 (外側＝ミドルウェアより先に実行される)
        // ------------------------------------------------------------
        .layer(Extension(db.clone()))
        .layer(Extension(hc.clone()))
        .layer(Extension(secure_client))
        .layer(Extension(cuber_service))
        .layer(Extension(Arc::new(JwtConfig {
            skey: rt_skey.to_string(),
            crypto_key: rt_crypto_key.to_string(),
        })))
        .layer(Extension(sw_port))
        .layer(Extension(config_manager))
        .layer(Extension(ws_clients))
        .layer(Extension(Arc::new(event_tx)));

    if cors {
        app = app.layer(cors_layer());
    }
    app
}
