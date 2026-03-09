use crate::config::settings::Env;
use crate::constants::{ED448_SIGNATURE_BYTES_LEN, SSE_CHANNEL_CAPACITY, SSE_HEARTBEAT_INTERVAL};
use crate::cuber::config::CuberConfig;
use crate::cuber::service::CuberService;
use crate::migration::{Migrator, MigratorTrait};
use crate::mode::rt::client::secure_client::SecureClient;
use crate::mode::rt::owner_secrets::{OWNER_PUB_KEY_HEX, OWNER_SECRET_BLOBS};
use crate::mode::rt::req_map;
use crate::mode::rt::rtbl::identities_bl::ensure_node_identity;
use crate::mode::rt::rtbl::{cleaner, periodic_store};
use crate::myproxy::server::start_proxy_server;
use crate::myproxy::ssl::loader::load_certs;
use crate::myproxy::ssl::setup::create_certs_if_missing;
use crate::stt_config::ConfigManager;
use crate::types::{EventKind, InternalEvent};
use crate::utils::crypto::Ed448RawKeyPair;
use crate::utils::db::get_db;
use crate::utils::init::LogLevel;
use crate::utils::mod_dl;
use crate::utils::s3client::S3Client;
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use argon2::{
    password_hash::{PasswordHasher, SaltString},
    Argon2,
};
use dashmap::DashMap;
use ed448_goldilocks::{curve::ExtendedPoint, Scalar};
use std::sync::Arc;
use tokio::sync::broadcast;

pub async fn main_of_rt(
    config_manager: Arc<ConfigManager>,
    _env_legacy: Env,
    owner_passphrase: Option<String>,
    hc: Arc<reqwest::Client>,
    skip_rt_migration: bool,
) {
    log::debug!("[Trace] main_of_rt started.");

    // ==============================
    // my_base_url 必須バリデーション (先頭)
    // ==============================
    config_manager
        .settings
        .read()
        .validate_my_base_url(owner_passphrase.is_some());

    // ==============================
    // Model Download Check
    // ==============================
    log::info!("Checking AI models...");
    if let Err(e) = mod_dl::ensure_models(&config_manager).await {
        log::error!("Failed to download models: {}", e);
        std::process::exit(1);
    }
    if let Err(e) = config_manager.validate_models() {
        log::error!("Model validation failed: {}", e);
        std::process::exit(1);
    }

    // ==============================
    // 設定のスナップショット取得
    // ==============================
    let (sset, stset) = {
        let s = config_manager.settings.read();
        (s.server.clone(), s.storage.clone())
    };

    // settings.json から DB 環境情報を生成
    let env = Env::from_settings(&stset);
    log::debug!("[Trace] Settings loaded and Env derived.");

    let rt_port = sset.rt_port;
    let cors_on_rt = sset.cors_on_rt;
    let rt_skey = sset.rt_skey.clone();

    log::debug!("Starting server with settings from settings.json");
    log::debug!("RT_PORT: {}", rt_port);
    log::debug!("CORS_ON_RT: {}", cors_on_rt);

    // ==============================
    // [Owner Mode] 起動時復号チェック (Phase 1.5)
    // ==============================
    log::debug!("[Trace] Checking Owner Passphrase...");
    if let Some(pass) = owner_passphrase {
        log::info!("Owner Mode requested. Attempting to decrypt Anchor Secret Key...");

        let argon2 = Argon2::default();
        let mut decrypted_secret_bytes = None;

        for (i, blob) in OWNER_SECRET_BLOBS.iter().enumerate() {
            // Parse Blob: [SaltLen(1) | Salt(...) | Nonce(12) | Ciphertext(Var)]
            if blob.len() < 1 {
                continue;
            }
            let salt_len = blob[0] as usize;
            if blob.len() < 1 + salt_len + 12 {
                continue;
            }

            let salt_bytes = &blob[1..1 + salt_len];
            let nonce_bytes = &blob[1 + salt_len..1 + salt_len + 12];
            let ciphertext = &blob[1 + salt_len + 12..];

            let salt_str = match std::str::from_utf8(salt_bytes) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let salt = match SaltString::from_b64(salt_str) {
                Ok(s) => s,
                Err(_) => continue,
            };

            let password_hash = match argon2.hash_password(pass.as_bytes(), &salt) {
                Ok(h) => h,
                Err(_) => continue,
            };

            let key_bytes: &argon2::password_hash::Output = match &password_hash.hash {
                Some(h) => h,
                None => continue,
            };

            let key_array: [u8; 32] = match key_bytes.as_bytes().try_into() {
                Ok(k) => k,
                Err(_) => continue,
            };

            let cipher = Aes256Gcm::new(&key_array.into());
            let nonce = Nonce::from_slice(nonce_bytes);

            if let Ok(plaintext) = cipher.decrypt(nonce, ciphertext) {
                if plaintext.len() == ED448_SIGNATURE_BYTES_LEN {
                    decrypted_secret_bytes = Some(plaintext);
                    log::info!(
                        "Owner Secret Key decrypted successfully with blob #{}",
                        i + 1
                    );
                    break;
                }
            }
        }

        if let Some(secret_bytes_vec) = decrypted_secret_bytes {
            // Ed448 キーとして検証とロード
            // Fix: TryInto to convert Vec<u8> to [u8; ED448_SIGNATURE_BYTES_LEN]
            let secret_bytes_arr: [u8; ED448_SIGNATURE_BYTES_LEN] =
                secret_bytes_vec.try_into().expect("Length checked above");

            let secret_scalar = Scalar::from_bytes_mod_order_wide(&secret_bytes_arr);
            let public_point = ExtendedPoint::generator() * &secret_scalar;
            let public_bytes = public_point.compress();
            let public_hex = hex::encode(public_bytes.0);

            if public_hex != OWNER_PUB_KEY_HEX {
                log::error!(
                    "CRITICAL: Decrypted key does not match the hardcoded Anchor Public Key!"
                );
                log::error!("Expected: {}", OWNER_PUB_KEY_HEX);
                log::error!("Actual:   {}", public_hex);
                std::process::exit(1);
            }

            // メモリ上に保持
            {
                let mut guard = config_manager.owner_key.write();
                *guard = Some(Ed448RawKeyPair {
                    secret: secret_scalar,
                    public: public_bytes.0,
                });
            }
            log::info!("[Owner Mode] Activated. You have Root Authority.");
        } else {
            log::error!(
                "CRITICAL: Invalid owner passphrase. Failed to decrypt any of the 15 secret blobs."
            );
            log::error!("Access Denied.");
            std::process::exit(1);
        }
    } else {
        log::info!("Running in Standard Mode (No Owner Privileges).");
    }

    // ==============================
    // CA証明書の自動セットアップ (Phase 8.9)
    // ==============================
    // サーバー起動時に自動的に証明書の状態を確認し、必要であれば生成・インストールを行う。
    // settings.json に既に存在する場合は何もせずに SetupStatus::Existing を返す。
    // この処理は管理者権限で動作しているサーバープロセスが行うべき。
    log::debug!("[Trace] Ensuring CA certificates...");
    if let Err(e) = create_certs_if_missing(&config_manager) {
        log::error!("CRITICAL: Failed to setup CA certificates at boot: {}", e);
        // [Fate-Sharing] 致命的エラーのためサーバープロセスを終了させる。
        // これにより、監視しているクライアントも終了し、不整合な状態を防ぐ。
        std::process::exit(1);
    }

    // ==============================
    // Node Identity (Ed448) Setup
    // ==============================
    // オーナーモードの場合は、埋め込みキーペアを使用するため、ノード固有のアイデンティティ生成をスキップする。
    let is_owner = config_manager.owner_key.read().is_some();
    if !is_owner {
        log::debug!("[Trace] Ensuring Node Identity...");
        if let Err(e) = ensure_node_identity(&config_manager) {
            log::error!("CRITICAL: Failed to ensure Node Identity: {}", e);
            std::process::exit(1);
        }
    } else {
        log::info!(
            "Owner Mode detected. Using Anchor Identity; skipping Node Identity generation."
        );
    }

    // ==============================
    // HTTPSプロキシサーバーの起動 (Phase 8.20)
    // ==============================
    log::debug!("[Trace] Loading SSL certificates for Proxy...");
    let server_config = match load_certs(&config_manager) {
        Ok(config) => config,
        Err(e) => {
            log::error!("CRITICAL: Failed to load SSL certificates: {}", e);
            std::process::exit(1);
        }
    };

    let sw_port = sset.sw_port;
    log::debug!("[Trace] Starting Proxy Server on port {}...", sw_port);
    let (proxy_addr, proxy_future) =
        start_proxy_server(config_manager.clone(), sw_port, server_config, hc.clone())
            .await
            .expect("Failed to initialize proxy server binding");

    // バックグラウンドでサーバーを走行させる
    tokio::spawn(proxy_future);
    log::info!("MyProxy Direct HTTPS Server started on {}", proxy_addr);

    // ==============================
    // s3clientの初期化
    // ==============================
    log::debug!("[Trace] Initializing S3Client...");
    let s3c = S3Client::new(
        &stset.s3_access_key,
        &stset.s3_secret_access_key,
        &stset.s3_region,
        &stset.s3_bucket,
        &stset.s3_local_dir,
        &stset.s3_down_dir,
        stset.s3_use_local,
    )
    .await;

    let s3_client = match s3c {
        Ok(s) => {
            log::debug!("S3Client created successfully.");
            Arc::new(s)
        }
        Err(e) => {
            eprintln!("Failed to create s3client: {}", e);
            std::process::exit(1);
        }
    };

    // ==============================
    // DB接続
    // ==============================
    log::debug!("[Trace] Connecting to Database...");
    let db_result = get_db(&env, &LogLevel::Debug).await;

    let db = match db_result {
        Ok(db) => {
            log::debug!("DB created successfully.");
            db
        }
        Err(e) => {
            eprintln!("Failed to create DB: {}", e);
            std::process::exit(1);
        }
    };

    // ==============================
    // [Auto Migration] Startup Check
    // ==============================
    // アプリ起動時に自動的にマイグレーション(UP)を適用する。
    // シングルトンロックにより、多重起動時の競合は防止されている前提。
    if !skip_rt_migration {
        log::info!("Checking for pending migrations...");
        let rw_conn = db
            .get_rw()
            .expect("Failed to get RW connection for migration");
        log::debug!("[Trace] Applying DB Migrations...");
        if let Err(e) = Migrator::up(rw_conn, None).await {
            log::error!("CRITICAL: Failed to apply auto-migrations: {}", e);
            // DB不整合を防ぐため、マイグレーション失敗時は起動を中止する
            std::process::exit(1);
        }
        log::info!("Auto-migration completed successfully.");
    } else {
        log::info!("Skipping Auto-migration as instructed by parent process/caller.");
    }

    // ==============================
    // Key Rotation (Phase 3.6)
    // ==============================
    // DB接続直後にキーローテーションの必要性をチェックする。
    // もしローテーションが実行された場合、config_manager内のrt_crypto_keyも更新されるため、
    // 以降の処理（req_map等）には新しいキーが渡される必要がある。
    // rw.clone() returns DatabaseConnection which is cheap to clone (Arc inside)
    let conn = db.rw.clone();
    log::debug!("[Trace] Checking Key Rotation...");
    if let Err(e) =
        crate::utils::rotation_bl::check_and_rotate_keys(config_manager.clone(), &conn).await
    {
        log::error!("CRITICAL: Key Rotation Failed: {}", e);
        // 暗号化状態の不整合を防ぐため、失敗時は即死させる
        std::process::exit(1);
    }

    // 再取得（キーが更新された可能性があるため）
    // Note: sset (ServerSettings) is a COPY from the beginning. We need to refresh it.
    let (sset, cset) = {
        let s = config_manager.settings.read();
        (s.server.clone(), s.cuber.clone())
    };
    let rt_crypto_key = sset.rt_crypto_key.clone(); // Refreshed key

    // ==============================
    // CuberServiceの初期化
    // ==============================
    log::debug!("[Trace] Initializing CuberService...");
    let cuber_config = CuberConfig::from_settings(&cset, &stset);
    let cuber_service = match CuberService::new(cuber_config, Arc::clone(&s3_client)).await {
        Ok(s) => Arc::new(s),
        Err(e) => {
            eprintln!("Failed to initialize CuberService: {:?}", e);
            std::process::exit(1);
        }
    };

    let db_arc = Arc::new(db);

    // ==============================
    // SecureClient の初期化 (Arc 共有)
    // ==============================
    let secure_client = Arc::new(SecureClient::new(
        hc.as_ref().clone(),
        db_arc.clone(),
        config_manager.clone(),
    ));

    // ==============================
    // 定期クリーナータスクの開始 (Background)
    // ==============================
    cleaner::start_cleaner_task(db_arc.clone());

    // ==============================
    // 定期的な情報保持タスクの開始 (Background)
    // ==============================
    periodic_store::start_periodical_store_task(db_arc.clone(), config_manager.clone());

    // ==============================
    // SSE / WS イベントブロードキャスターと状態管理の初期化
    // ==============================
    let (event_tx, _event_rx) = broadcast::channel::<InternalEvent>(SSE_CHANNEL_CAPACITY);
    // WS接続管理マップ：ハンドシェイク成功済みの接続のみを UUID をキーとして管理
    let ws_clients: Arc<DashMap<String, crate::types::WsClientRole>> = Arc::new(DashMap::new());

    // ハートビート定期送信タスクの起動
    let hb_tx = event_tx.clone();
    tokio::spawn(async move {
        let mut interval_timer = tokio::time::interval(SSE_HEARTBEAT_INTERVAL);
        loop {
            interval_timer.tick().await;
            let _ = hb_tx.send(InternalEvent {
                seq: 0,
                kind: EventKind::Heartbeat,
            });
        }
    });

    // ==============================
    // Axum リクエストマッピングと起動
    // ==============================
    let router = req_map::map_request(
        cors_on_rt,
        db_arc,
        &rt_skey,
        &rt_crypto_key,
        cuber_service,
        sset.sw_port,
        config_manager.clone(),
        hc,
        secure_client,
        event_tx,
        ws_clients,
    );
    log::debug!("Starting RT server on port {}...", rt_port);
    log::debug!("[Trace] Binding TCP Listener on port {}...", rt_port);
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{rt_port}"))
        .await
        .expect("Failed to bind listener.");

    axum::serve(listener, router)
        .await
        .expect("Failed to serve.");
}
