use axum::{
    body::Body,
    http::{Request, StatusCode},
    routing::get,
    Extension, Router,
};
use mycute::entities::{blacklists, identities, tickets, verifications};
use mycute::{
    constants::{
        HEADER_X_MYCUTE_CA_BASE_URL, HEADER_X_MYCUTE_SENDER_PUBKEY, HEADER_X_MYCUTE_SIGNATURE,
        HEADER_X_MYCUTE_TIMESTAMP,
    },
    mode::rt::client::secure_client::SecureClient,
    mode::rt::middleware::p2p_clock_sync_enforcement_middleware::p2p_clock_sync_enforcement_middleware,
    mode::rt::rtbl::blacklists_bl::{self, CrimeDetail, CrimeEvidence},
    stt_config::ConfigManager,
    utils::{crypto::Ed448KeyValuePair, db::DbPools, time},
};
use sea_orm::{ConnectionTrait, Database, Schema};
use std::sync::Arc;
use tower::util::ServiceExt; // for oneshot

const PATH_STRICT: &str = "/v1/ca/identities/entry";
const PATH_OPTIONAL: &str = "/v1/pub/apps/list";

async fn handler() -> &'static str {
    "OK"
}

async fn get_test_db() -> DbPools {
    let db = Database::connect("sqlite::memory:").await.unwrap();

    let builder = db.get_database_backend();
    let schema = Schema::new(builder);

    let create_table_stmts = [
        builder.build(&schema.create_table_from_entity(identities::Entity)),
        builder.build(&schema.create_table_from_entity(verifications::Entity)),
        builder.build(&schema.create_table_from_entity(tickets::Entity)),
        builder.build(&schema.create_table_from_entity(blacklists::Entity)),
    ];

    for stmt in create_table_stmts {
        db.execute(stmt).await.unwrap();
    }

    DbPools {
        rw: db.clone(),
        ro: vec![db],
        ro_index: std::sync::atomic::AtomicUsize::new(0),
    }
}

async fn get_test_config() -> Arc<ConfigManager> {
    let tmp = tempfile::NamedTempFile::new().unwrap();
    let path_str = tmp.path().to_str().unwrap().to_string();
    let config = ConfigManager::new(None, Some(path_str));

    // 署名に必要な Identity を生成して保存
    {
        let mut settings = config.settings.write();
        settings.server.rt_crypto_key = "01234567890123456789012345678901".to_string();
    }
    mycute::mode::rt::rtbl::identities_bl::ensure_node_identity(&config).unwrap();

    Arc::new(config)
}

fn init_runtime_openapi() {
    // RUNTIME_OPENAPI にテスト用のパス定義を注入する
    // これにより、middleware が /v1/ca/identities/entry を STRICT として認識するようになる
    let _ = mycute::mode::rt::req_map::RUNTIME_OPENAPI.set({
        let mut openapi = utoipa::openapi::OpenApi::default();
        let mut paths = utoipa::openapi::path::Paths::new();

        // STRICT パスの登録
        let mut strict_op = utoipa::openapi::path::Operation::new();
        strict_op.tags = Some(vec![
            "v1 CA Identity".to_string(),
            mycute::constants::TAG_MARKER_P2P_STRICT.to_string(),
        ]);

        // utoipa 5.x では PathItemType が見当たらないため、Default から構築して get にセットする
        let mut strict_item = utoipa::openapi::path::PathItem::default();
        strict_item.get = Some(strict_op);

        paths.paths.insert(PATH_STRICT.to_string(), strict_item);

        // OPTIONAL パスの登録 (/pub/apps/list)
        let mut optional_op = utoipa::openapi::path::Operation::new();
        optional_op.tags = Some(vec![
            "v1 Pub Apps".to_string(),
            mycute::constants::TAG_MARKER_P2P_OPTIONAL.to_string(),
        ]);

        let mut optional_item = utoipa::openapi::path::PathItem::default();
        optional_item.get = Some(optional_op);

        paths.paths.insert(PATH_OPTIONAL.to_string(), optional_item);

        openapi.paths = paths;
        openapi
    });
}

#[tokio::test]
async fn test_p2p_bypass_no_header() {
    init_runtime_openapi();
    let db = get_test_db().await;
    let config = get_test_config().await;
    let db_arc = Arc::new(db);
    let app = Router::new()
        .route("/v1/bypass", get(handler)) // OpenAPI に登録されていないパス
        .layer(axum::middleware::from_fn(
            p2p_clock_sync_enforcement_middleware,
        ))
        .layer(Extension(config.clone()))
        .layer(Extension(db_arc.clone()))
        .layer(Extension(Arc::new(SecureClient::new(
            reqwest::Client::new(),
            db_arc,
            config,
        ))));

    let req = Request::builder()
        .uri("/v1/bypass")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    // ミドルウェアはスルーするはずなので 200 OK
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_p2p_valid_request() {
    let db = get_test_db().await;
    let config = get_test_config().await;
    let db_arc = Arc::new(db);
    let app = Router::new()
        .route(PATH_STRICT, get(handler))
        .layer(axum::middleware::from_fn(
            p2p_clock_sync_enforcement_middleware,
        ))
        .layer(Extension(config.clone()))
        .layer(Extension(db_arc.clone()))
        .layer(Extension(Arc::new(SecureClient::new(
            reqwest::Client::new(),
            db_arc,
            config,
        ))));

    let now = time::now_ts_ms();
    let keypair = Ed448KeyValuePair::generate().unwrap();
    let sig = keypair.sign(&now.to_be_bytes()).unwrap();

    let req = Request::builder()
        .uri(PATH_STRICT)
        .header(HEADER_X_MYCUTE_TIMESTAMP, now.to_string())
        .header(HEADER_X_MYCUTE_SIGNATURE, hex::encode(sig.signature))
        .header(HEADER_X_MYCUTE_SENDER_PUBKEY, hex::encode(keypair.public))
        .header(HEADER_X_MYCUTE_CA_BASE_URL, "http://ca.example.com")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers().contains_key(HEADER_X_MYCUTE_TIMESTAMP));
    assert!(response.headers().contains_key(HEADER_X_MYCUTE_SIGNATURE));
}

#[tokio::test]
async fn test_p2p_invalid_signature() {
    let db = get_test_db().await;
    let config = get_test_config().await;
    let db_arc = Arc::new(db);
    let app = Router::new()
        .route(PATH_STRICT, get(handler))
        .layer(axum::middleware::from_fn(
            p2p_clock_sync_enforcement_middleware,
        ))
        .layer(Extension(config.clone()))
        .layer(Extension(db_arc.clone()))
        .layer(Extension(Arc::new(SecureClient::new(
            reqwest::Client::new(),
            db_arc,
            config,
        ))));

    let now = time::now_ts_ms();
    let keypair = Ed448KeyValuePair::generate().unwrap();
    // 署名対象をわざと間違える
    let sig = keypair.sign(&(now + 1).to_be_bytes()).unwrap();

    let req = Request::builder()
        .uri(PATH_STRICT)
        .header(HEADER_X_MYCUTE_TIMESTAMP, now.to_string())
        .header(HEADER_X_MYCUTE_SIGNATURE, hex::encode(sig.signature))
        .header(HEADER_X_MYCUTE_SENDER_PUBKEY, hex::encode(keypair.public))
        .header(HEADER_X_MYCUTE_CA_BASE_URL, "http://ca.example.com")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_p2p_skewed_timestamp() {
    let db = get_test_db().await;
    let config = get_test_config().await;
    let db_arc = Arc::new(db);
    let app = Router::new()
        .route(PATH_STRICT, get(handler))
        .layer(axum::middleware::from_fn(
            p2p_clock_sync_enforcement_middleware,
        ))
        .layer(Extension(config.clone()))
        .layer(Extension(db_arc.clone()))
        .layer(Extension(Arc::new(SecureClient::new(
            reqwest::Client::new(),
            db_arc,
            config,
        ))));

    let future = time::now_ts_ms() + 3_600_000; // 1 hour later
    let keypair = Ed448KeyValuePair::generate().unwrap();
    let sig = keypair.sign(&future.to_be_bytes()).unwrap();

    let req = Request::builder()
        .uri(PATH_STRICT)
        .header(HEADER_X_MYCUTE_TIMESTAMP, future.to_string())
        .header(HEADER_X_MYCUTE_SIGNATURE, hex::encode(sig.signature))
        .header(HEADER_X_MYCUTE_SENDER_PUBKEY, hex::encode(keypair.public))
        .header(HEADER_X_MYCUTE_CA_BASE_URL, "http://ca.example.com")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_p2p_self_enforcement() {
    let db = get_test_db().await;
    let config = get_test_config().await;
    let db_arc = Arc::new(db);
    let app = Router::new()
        .route(PATH_STRICT, get(handler))
        .layer(axum::middleware::from_fn(
            p2p_clock_sync_enforcement_middleware,
        ))
        .layer(Extension(config.clone()))
        .layer(Extension(db_arc.clone()))
        .layer(Extension(Arc::new(SecureClient::new(
            reqwest::Client::new(),
            db_arc.clone(),
            config.clone(),
        ))));

    // 1. 自分の Identity を取得
    let my_keypair = config.get_node_keypair().unwrap();
    let my_pub_hex = hex::encode(my_keypair.public);

    // 2. 自分をブラックリストに追加 (Self-Enforcement Trigger)
    // 署名検証（validate_evidence）をパスするために、正しい署名を持つ証拠を生成する
    let now = time::now_ts_ms();
    let sig = my_keypair.sign(&now.to_be_bytes()).unwrap();
    let evidence = CrimeEvidence {
        detail: CrimeDetail::TimestampFraud {
            wrong_timestamp: now as i64,
            time_diff_ms: 0,
        },
        target_pubkey: my_pub_hex.clone(),
        observed_at: now as i64,
        signature: hex::encode(sig.signature),
        signed_payload: hex::encode(now.to_be_bytes()),
    };
    blacklists_bl::add_to_blacklist(db_arc.as_ref(), evidence)
        .await
        .unwrap();

    // 3. 正常なリクエストを送信 (相手は白)
    let other_keypair = Ed448KeyValuePair::generate().unwrap();
    let other_sig = other_keypair.sign(&now.to_be_bytes()).unwrap();

    let req = Request::builder()
        .uri(PATH_STRICT)
        .header(HEADER_X_MYCUTE_TIMESTAMP, now.to_string())
        .header(HEADER_X_MYCUTE_SIGNATURE, hex::encode(other_sig.signature))
        .header(
            HEADER_X_MYCUTE_SENDER_PUBKEY,
            hex::encode(other_keypair.public),
        )
        .header(HEADER_X_MYCUTE_CA_BASE_URL, "http://ca.example.com")
        .body(Body::empty())
        .unwrap();

    // 4. 自分が黒なので 403 が返るはず
    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_p2p_blacklisted_peer() {
    let db = get_test_db().await;
    let config = get_test_config().await;
    let db_arc = Arc::new(db);
    let app = Router::new()
        .route(PATH_STRICT, get(handler))
        .layer(axum::middleware::from_fn(
            p2p_clock_sync_enforcement_middleware,
        ))
        .layer(Extension(config.clone()))
        .layer(Extension(db_arc.clone()))
        .layer(Extension(Arc::new(SecureClient::new(
            reqwest::Client::new(),
            db_arc.clone(),
            config,
        ))));

    // 1. 相手 (Blacklisted) の Identity 生成
    let bad_keypair = Ed448KeyValuePair::generate().unwrap();
    let bad_pub_hex = hex::encode(bad_keypair.public);
    let now = time::now_ts_ms();

    // 2. 相手をブラックリストに追加
    let sig = bad_keypair.sign(&now.to_be_bytes()).unwrap();
    let evidence_valid = CrimeEvidence {
        detail: CrimeDetail::TimestampFraud {
            wrong_timestamp: now as i64,
            time_diff_ms: 0,
        },
        target_pubkey: bad_pub_hex.clone(),
        observed_at: now as i64,
        signature: hex::encode(sig.signature),
        signed_payload: hex::encode(now.to_be_bytes()),
    };
    blacklists_bl::add_to_blacklist(db_arc.as_ref(), evidence_valid)
        .await
        .unwrap();

    // 3. 相手からのリクエスト
    let req_sig = bad_keypair.sign(&now.to_be_bytes()).unwrap();
    let req = Request::builder()
        .uri(PATH_STRICT)
        .header(HEADER_X_MYCUTE_TIMESTAMP, now.to_string())
        .header(HEADER_X_MYCUTE_SIGNATURE, hex::encode(req_sig.signature))
        .header(HEADER_X_MYCUTE_SENDER_PUBKEY, bad_pub_hex)
        .header(HEADER_X_MYCUTE_CA_BASE_URL, "http://ca.example.com")
        .body(Body::empty())
        .unwrap();

    // 4. 相手が黒なので 403 が返るはず
    let response = app.oneshot(req).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn test_p2p_strict_enforcement_missing_header() {
    init_runtime_openapi();
    let db = get_test_db().await;
    let config = get_test_config().await;
    let db_arc = Arc::new(db);
    let app = Router::new()
        .route(PATH_STRICT, get(handler)) // STRICT なパス
        .layer(axum::middleware::from_fn(
            p2p_clock_sync_enforcement_middleware,
        ))
        .layer(Extension(config.clone()))
        .layer(Extension(db_arc.clone()))
        .layer(Extension(Arc::new(SecureClient::new(
            reqwest::Client::new(),
            db_arc,
            config,
        ))));

    // ヘッダーなしでアクセス
    let req = Request::builder()
        .uri("/v1/ca/identities/entry")
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    // 署名がないため、STRICT パスでは 400 Bad Request が返るはず
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_p2p_optional_bypass() {
    init_runtime_openapi();
    let db = get_test_db().await;
    let config = get_test_config().await;
    let db_arc = Arc::new(db);
    let app = Router::new()
        .route(PATH_OPTIONAL, get(handler)) // OPTIONAL なパス
        .layer(axum::middleware::from_fn(
            p2p_clock_sync_enforcement_middleware,
        ))
        .layer(Extension(config.clone()))
        .layer(Extension(db_arc.clone()))
        .layer(Extension(Arc::new(SecureClient::new(
            reqwest::Client::new(),
            db_arc,
            config,
        ))));

    // ヘッダーなしでアクセス
    let req = Request::builder()
        .uri(PATH_OPTIONAL)
        .body(Body::empty())
        .unwrap();

    let response = app.oneshot(req).await.unwrap();
    // OPTIONAL パスなので、ヘッダーがなくても通常通り 200 OK が返るはず
    assert_eq!(response.status(), StatusCode::OK);
}
