use crate::config::settings::Env;
use crate::constants::{SSE_CHANNEL_CAPACITY, SSE_HEARTBEAT_INTERVAL};
use crate::cuber::config::CuberConfig;
use crate::cuber::service::CuberService;
use crate::migration::{Migrator, MigratorTrait};
use crate::mode::rt::client::secure_client::SecureClient;
use crate::mode::rt::req_map;
use crate::mode::rt::rtbl::identities_bl::ensure_node_identity;
use crate::mode::rt::rtbl::{cleaner, periodic_store};
use crate::mycute_settings::ConfigManager;
use crate::myproxy::server::start_proxy_server;
use crate::myproxy::ssl::loader::load_certs;
use crate::myproxy::ssl::setup::create_certs_if_missing;
use crate::types::{EventKind, InternalEvent, WsClientRole};
use crate::utils::db::get_db;
use crate::utils::init::{CommonFlgs, HasCommonFlgs, LogLevel};
use crate::utils::rotation_bl;
use crate::utils::s3client::S3Client;
use clap::Parser;
use dashmap::DashMap;
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::broadcast;

/// サーバー（RT）モード専用の引数構造体
#[derive(Debug, Parser, Serialize)]
#[command(override_usage = "mycute-server [OPTIONS]", ignore_errors = true)]
pub struct RTFlgs {
    #[command(flatten)]
    pub common: CommonFlgs,

    /// 運命共同体モード用の親プロセスID監視オプション
    /// 指定されたPIDのプロセスが消失した場合、自己終了します (Fate-Sharing)。
    #[arg(
        long = "parent-pid",
        help = "PID of the parent process to monitor (Fate-Sharing)"
    )]
    pub parent_pid: Option<u32>,

    /// バックエンドサーバー起動時のDBオートマイグレーションをスキップする
    #[arg(
        long = "skip-rt-migration",
        help = "Skip DB auto-migration when starting the RT backend server"
    )]
    pub skip_rt_migration: bool,

    /// ログの出力先 (stdout, /path/to/file)
    /// GUIからの起動時は、manage_backend_serverによって適切なパスが設定されます。
    #[arg(short = 'o', long = "output", help = "Destination of log output")]
    pub output: Option<String>,
}

impl HasCommonFlgs for RTFlgs {
    fn common_flgs(&self) -> &CommonFlgs {
        &self.common
    }
}

pub async fn main_of_rt(
    config_manager: Arc<ConfigManager>,
    hc: Arc<reqwest::Client>,
    flgs: RTFlgs,
) {
    log::debug!("[Trace] main_of_rt started.");

    // ==============================
    // my_base_url 必須バリデーション (先頭)
    // ==============================
    config_manager.settings.read().validate_my_base_url();

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
    // CA証明書の自動セットアップ (Phase 8.9)
    // ==============================
    // サーバー起動時に自動的に証明書の状態を確認し、必要であれば生成・インストールを行う。
    // settings.json に既に存在する場合は何もせずに SetupStatus::Existing を返す。
    // この処理は管理者権限で動作しているサーバープロセスが行うべき。
    log::debug!("[Trace] Ensuring CA certificates...");

    if let Err(e) = create_certs_if_missing(&config_manager).await {
        log::error!("CRITICAL: Failed to setup CA certificates at boot: {}", e);
        // [Fate-Sharing] 致命的エラーのためサーバープロセスを終了させる。
        // これにより、監視しているクライアントも終了し、不整合な状態を防ぐ。
        std::process::exit(1);
    }

    // ==============================
    // Node Identity (Ed448) Setup
    // ==============================
    log::debug!("[Trace] Ensuring Node Identity...");
    if let Err(e) = ensure_node_identity(&config_manager) {
        log::error!("CRITICAL: Failed to ensure Node Identity: {}", e);
        std::process::exit(1);
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
    let db_pools = get_db(&env, &LogLevel::Debug)
        .await
        .map_err(|e| {
            eprintln!("Failed to create DB: {}", e);
            std::process::exit(1);
        })
        .expect("Verified above");

    // ==============================
    // [Live] ConfigManager を DB 付きで再初期化
    // ==============================
    // プロセス単体での起動を考慮し、ここで DB 付きの Live インスタンスに差し替える。
    let config_manager = Arc::new(ConfigManager::new_live(db_pools.clone()));

    log::debug!("DB created successfully and ConfigManager upgraded to Live.");

    // ==============================
    // [Live] DB 接続の同期
    // ==============================
    // main_of_cl ですでに DB 付きの ConfigManager になっているはずだが、
    // RT 起動時に念のため DB 内容との同期（および移行チェック）を行う。
    config_manager
        .ensure_initialized_with_db()
        .await
        .expect("Failed to sync settings with DB");

    // ==============================
    // [Auto Migration] Startup Check
    // ==============================
    // [DB] マイグレーション実行
    // ==============================
    if !flgs.skip_rt_migration {
        log::debug!("[Trace] Running DB Migrations...");
        let rw_conn = db_pools
            .get_rw()
            .expect("Failed to get RW connection for migration");
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
    let conn = db_pools.rw.clone();
    log::debug!("[Trace] Checking Key Rotation...");
    if let Err(e) = rotation_bl::check_and_rotate_keys(config_manager.clone(), &conn).await {
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

    let db_arc = Arc::new(db_pools);

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
    let ws_clients: Arc<DashMap<String, WsClientRole>> = Arc::new(DashMap::new());

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
