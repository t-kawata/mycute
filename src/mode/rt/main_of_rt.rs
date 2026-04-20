use crate::bifrost;
use crate::config::settings::Env;
use crate::constants::{IP_LOCALHOST, SSE_CHANNEL_CAPACITY, SSE_HEARTBEAT_INTERVAL};
use crate::cuber::config::CuberConfig;
use crate::cuber::service::CuberService;
use crate::migration::{Migrator, MigratorTrait};
use crate::mode::rt::client::secure_client::SecureClient;
use crate::mode::rt::req_map;
use crate::mode::rt::rtbl::identities_bl;
use crate::mode::rt::rtbl::{cleaner, periodic_store};
use crate::mycute_settings::ConfigManager;
use crate::myproxy::server::start_proxy_server;
use crate::myproxy::ssl::loader::load_certs;
use crate::myproxy::ssl::setup::create_certs_if_missing;
use crate::nodejs::{self, NodeManager};
use crate::types::{EventKind, InternalEvent, WsClientRole};
use crate::utils::db::get_db;
use crate::utils::init::{CommonFlgs, HasCommonFlgs, LogLevel};
use crate::utils::process::{self as proc_utils, ChildProcessGuard};
use crate::utils::rotation_bl;
use crate::utils::s3client::S3Client;
use crate::zeroclaw;
use anyhow::Context;
use clap::Parser;
use dashmap::DashMap;
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;

/// サーバー（RT）モード専用の引数構造体
#[derive(Debug, Parser, Serialize)]
#[command(override_usage = "mycute-server [OPTIONS]")]
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
    shutdown_token: CancellationToken,
) -> anyhow::Result<()> {
    log::debug!("[Trace] main_of_rt started.");

    // ============================================================
    // [Fate-Sharing Registry] 運命共同体プロセスの登録所
    // ============================================================
    // 今後、RT によって起動される子プロセス（バイナリ等）が増える場合は、
    // そのプロセスの PID をこの child_pids に追加してください。
    // 親プロセス（CL）が消失した際、ここに登録された全てのプロセスが道連れ kill されます。
    // ============================================================
    let child_pids = Arc::new(parking_lot::Mutex::new(Vec::<u32>::new()));

    // ============================================================
    // [Cleanup] 既存プロセスのクリーンアップとポート解放確認
    // ============================================================
    // 起動時に、主要なポート（RT_PORT, BIFROST_PORT, ZEROCLAW_PORT）を占有している
    // 既存のプロセスを特定して強制終了し、OSがポートを完全に解放するのを待ちます。
    // ============================================================
    config_manager.cleanup_all_backend_ports("Startup");

    log::info!("Backend parsed flags: {:?}", flgs);
    let home_dir = crate::utils::my_path::get_mycute_home(flgs.common.home.clone());

    // ==============================
    // [Runtime Dependency] Node.js セットアップ
    // ==============================
    log::info!("Setting up Node.js runtime for backend...");
    if let Err(e) = nodejs::install(&home_dir) {
        log::error!("CRITICAL: Failed to setup Node.js runtime: {}", e);
        return Err(anyhow::anyhow!("Failed to setup Node.js runtime: {}", e));
    }
    let node_manager = Arc::new(NodeManager::new(&home_dir));

    // ==============================
    // [Runtime Dependency] Bifrost セットアップ & 起動 (最優先)
    // ==============================
    // ZeroClaw が Bifrost を経由するため、まず Bifrost を起動して準備完了を待つ必要がある。
    log::info!("Setting up Bifrost runtime for backend...");
    let _bifrost_install = match bifrost::install(&home_dir) {
        Ok(res) => res,
        Err(e) => {
            log::error!("CRITICAL: Failed to setup Bifrost runtime: {}", e);
            return Err(anyhow::anyhow!("Failed to setup Bifrost runtime: {}", e));
        }
    };
    let bifrost_manager = bifrost::BifrostManager::new(&home_dir);

    // ServerSettings からポートを取得（RT_PORT, SW_PORT と同様の扱い）
    let bifrost_port = config_manager.settings.read().server.bifrost_port;
    let bifrost_child = match bifrost_manager.spawn(IP_LOCALHOST, bifrost_port) {
        Ok(child) => child,
        Err(e) => {
            log::error!("CRITICAL: Failed to spawn Bifrost process: {}", e);
            return Err(anyhow::anyhow!("Failed to spawn Bifrost process: {}", e));
        }
    };
    child_pids.lock().push(bifrost_child.id()); // 運命共同体に登録
    let bifrost_guard = ChildProcessGuard::new(bifrost_child, "Bifrost");

    // Bifrost の準備完了を待機 (HTTP ポーリング)
    log::info!(
        "Waiting for Bifrost to be ready on port {}...",
        bifrost_port
    );
    let mut ready = false;
    for i in 0..50 {
        // 最大10秒程度待機
        match hc
            .get(format!(
                "http://{}:{}/api/providers",
                IP_LOCALHOST, bifrost_port
            ))
            .send()
            .await
        {
            Ok(res) if res.status().is_success() => {
                log::info!("Bifrost is ready!");
                ready = true;
                break;
            }
            _ => {
                log::debug!("Bifrost is not ready yet (attempt {})...", i + 1);
                tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            }
        }
    }
    if !ready {
        log::error!("CRITICAL: Bifrost failed to start within timeout.");
        // bifrost_guard の Drop により自動的に終了されるが、明示的な exit で即座に反応
        return Err(anyhow::anyhow!("Bifrost failed to start within timeout."));
    }

    // Bifrost の子プロセスを非同期で管理 (ゾンビ化防止 & 早期終了検知)
    // 監視用スレッドへは PID のみ渡し、ガード自体はメインスレッドで保持する
    let _bifrost_pid = bifrost_guard.id();
    tokio::spawn(async move {
        // 注: ここで wait() するためには Child へのアクセスが必要だが、
        // Fate-Sharing の核心は「標準入力パイプのクローズ」にあるため、
        // ガードが Drop されることで子が自死する流れをメインとする。
        // もし子が先に死んだ場合は、API 経由のヘルスチェック等で検知可能。
    });

    // ==============================
    // [Runtime Dependency] ZeroClaw セットアップ & 起動
    // ==============================
    log::info!("Setting up ZeroClaw runtime for backend...");
    let zeroclaw_port = config_manager.settings.read().server.zeroclaw_port;
    let zeroclaw_install = match zeroclaw::install(&home_dir, bifrost_port, zeroclaw_port) {
        Ok(res) => res,
        Err(e) => {
            log::error!("CRITICAL: Failed to setup ZeroClaw runtime: {}", e);
            return Err(anyhow::anyhow!("Failed to setup ZeroClaw runtime: {}", e));
        }
    };

    let zeroclaw_manager =
        zeroclaw::ZeroClawManager::new(zeroclaw_install.install_dir, zeroclaw_install.root_dir);

    let zeroclaw_child = match zeroclaw_manager.spawn(zeroclaw_port) {
        Ok(child) => child,
        Err(e) => {
            log::error!("CRITICAL: Failed to spawn ZeroClaw process: {}", e);
            return Err(anyhow::anyhow!("Failed to spawn ZeroClaw process: {}", e));
        }
    };
    child_pids.lock().push(zeroclaw_child.id()); // 運命共同体に登録
    let _zeroclaw_guard = ChildProcessGuard::new(zeroclaw_child, "ZeroClaw");

    // ZeroClaw の子プロセス監視 (Fate-Sharing)
    // ChildProcessGuard がこのスコープに留まる限り、RT 終了時に道連れ kill される

    spawn_fate_sharing_monitor(flgs.parent_pid, child_pids.clone(), config_manager.clone(), shutdown_token.clone());

    // ==============================
    // my_base_url 必須バリデーション (先頭)
    // ==============================
    config_manager.settings.read().validate_my_base_url()?;

    // ==============================
    // 1. DB接続 & ConfigManager ライブ化 (最優先)
    // ==============================
    // 永続化（保存）を伴う初期化処理を確実に行うため、まず DB 接続を確立し、
    // ConfigManager を DB アクセス可能な "Live" インスタンスにアップグレードします。
    log::debug!("[Trace] Connecting to Database for initial setup...");
    let stset_boot = config_manager.settings.read().storage.clone();
    let env = Env::from_settings(&stset_boot);

    let db_pools = match get_db(&env, &LogLevel::Debug).await {
        Ok(pools) => pools,
        Err(e) => {
            log::error!("CRITICAL: Failed to create DB: {}", e);
            return Err(anyhow::anyhow!("Failed to create DB: {}", e));
        }
    };

    // ConfigManager を Live インスタンスに差し替える
    let config_manager = Arc::new(ConfigManager::new_live(
        db_pools.clone(),
        flgs.common.home.clone(),
    )?);
    log::debug!("DB connection established. ConfigManager upgraded to Live.");

    // DB 接続の同期・初期化チェック
    // GUI 側で初期化済みのケースが多いが、念のため ensure_initialized_with_db を実行し、
    // DB が空なら初期値を投入、あれば最新化を行う。
    log::debug!("Ensuring settings are initialized in DB...");
    config_manager
        .ensure_initialized_with_db()
        .await
        .context("Failed to ensure settings initialization in DB")?;

    // ==============================
    // 2. [Auto Migration] Startup Check
    // ==============================
    if !flgs.skip_rt_migration {
        log::debug!("[Trace] Running DB Migrations...");
        let rw_conn = db_pools
            .get_rw()
            .context("Failed to get RW connection for migration")?;
        if let Err(e) = Migrator::up(rw_conn, None).await {
            log::error!("CRITICAL: Failed to apply auto-migrations: {}", e);
            return Err(anyhow::anyhow!("Failed to apply auto-migrations: {}", e));
        }
        log::info!("Auto-migration completed successfully.");
    }

    // ==============================
    // 3. Node Identity (Ed448) Setup
    // ==============================
    // DBが準備できたので、ノードのアイデンティティを確立（未存在なら作成・保存）する。
    log::debug!("[Trace] Ensuring Node Identity...");
    if let Err(e) = identities_bl::ensure_node_identity_async(&config_manager).await {
        log::error!("CRITICAL: Failed to ensure Node Identity: {}", e);
        return Err(anyhow::anyhow!("Failed to ensure Node Identity: {}", e));
    }

    // ==============================
    // 4. CA証明書の自動セットアップ
    // ==============================
    log::debug!("[Trace] Ensuring CA certificates...");
    if let Err(e) = create_certs_if_missing(&config_manager).await {
        log::error!("CRITICAL: Failed to setup CA certificates at boot: {}", e);
        return Err(anyhow::anyhow!("Failed to setup CA certificates at boot: {}", e));
    }

    // ==============================
    // 5. HTTPSプロキシサーバーの起動
    // ==============================
    log::debug!("[Trace] Loading SSL certificates for Proxy...");
    let server_config = match load_certs(&config_manager) {
        Ok(config) => config,
        Err(e) => {
            log::error!("CRITICAL: Failed to load SSL certificates: {}", e);
            return Err(anyhow::anyhow!("Failed to load SSL certificates: {}", e));
        }
    };

    let sset = config_manager.settings.read().server.clone();
    let sw_port = sset.sw_port;
    log::debug!("[Trace] Starting Proxy Server on port {}...", sw_port);
    let (proxy_addr, proxy_future) =
        start_proxy_server(config_manager.clone(), sw_port, server_config, hc.clone())
            .await
            .context("Failed to initialize proxy server binding")?;

    tokio::spawn(proxy_future);
    log::info!("MyProxy Direct HTTPS Server started on {}", proxy_addr);

    // ==============================
    // 各種コンポーネント用スナップショット取得 & 初期化
    // ==============================
    let stset = {
        let s = config_manager.settings.read();
        s.storage.clone()
    };

    // s3client の初期化
    log::debug!("[Trace] Initializing S3Client...");
    let s3_client = match S3Client::new(
        &stset.s3_access_key,
        &stset.s3_secret_access_key,
        &stset.s3_region,
        &stset.s3_bucket,
        &stset.s3_local_dir,
        &stset.s3_down_dir,
        stset.s3_use_local,
    )
    .await
    {
        Ok(s) => Arc::new(s),
        Err(e) => {
            log::error!("Failed to create s3client: {}", e);
            return Err(anyhow::anyhow!("Failed to create s3client: {}", e));
        }
    };

    // Key Rotation
    // GUI による管理下にある場合は、既に行われているためスキップする。
    if flgs.parent_pid.is_none() {
        let conn = db_pools.rw.clone();
        log::debug!("[Trace] Checking Key Rotation (Headless mode)...");
        if let Err(e) = rotation_bl::check_and_rotate_keys(config_manager.clone(), &conn).await {
            log::error!("CRITICAL: [V-FIX-01] Key Rotation Failed: {}", e);
            return Err(anyhow::anyhow!("[V-FIX-01] Key Rotation Failed: {}", e));
        }
    } else {
        log::debug!("Parent PID set. Skipping Key Rotation check to avoid DB lock.");
    }

    // 取得し直し（ローテーションによる更新反映）
    let (sset, cset) = {
        let s = config_manager.settings.read();
        (s.server.clone(), s.cuber.clone())
    };
    let rt_crypto_key = sset.rt_crypto_key.clone();
    let rt_skey = sset.rt_skey.clone();
    let rt_port = sset.rt_port;
    let cors_on_rt = sset.cors_on_rt;

    // CuberService の初期化
    log::debug!("[Trace] Initializing CuberService...");
    let cuber_config = CuberConfig::from_settings(&cset, &stset);
    let cuber_service = match CuberService::new(cuber_config, Arc::clone(&s3_client)).await {
        Ok(s) => Arc::new(s),
        Err(e) => {
            log::error!("Failed to initialize CuberService: {:?}", e);
            return Err(anyhow::anyhow!("Failed to initialize CuberService: {:?}", e));
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
        node_manager,
    );
    log::debug!("Starting RT server on port {}...", rt_port);
    log::debug!("[Trace] Binding TCP Listener on port {}...", rt_port);
    let listener = tokio::net::TcpListener::bind(format!("{IP_LOCALHOST}:{rt_port}"))
        .await
        .context("Failed to bind listener.")?;

    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_token.cancelled_owned())
        .await
        .map_err(|e| anyhow::anyhow!("Failed to serve: {}", e))?;

    Ok(())
}

/// OSネイティブの運命共同体監視スレッドを起動します。
fn spawn_fate_sharing_monitor(
    parent_pid: Option<u32>,
    child_pids: Arc<parking_lot::Mutex<Vec<u32>>>,
    config_mgr: Arc<ConfigManager>,
    shutdown_token: CancellationToken,
) {
    if let Some(pid) = parent_pid {
        let config_mgr_for_stdin = config_mgr.clone();
        let config_mgr_for_monitor = config_mgr.clone();
        let child_pids_for_stdin = child_pids.clone();
        let child_pids_for_monitor = child_pids.clone();
        let token_for_stdin = shutdown_token.clone();
        let token_for_monitor = shutdown_token.clone();
        // 1. パイプによるEOF監視 (主に normal privileges 用)
        // [Windows] 特権昇格時 (Start-Process -Verb RunAs) は stdin パイプが接続されず、
        // read() が即座に EOF を返して誤終了してしまう。
        // そのため、親プロセスから環境変数で「パイプが有効である」と指示された場合のみ
        // stdin 監視を開始する。
        #[cfg(target_os = "windows")]
        let watch_stdin = std::env::var("MYCUTE_WATCH_STDIN").unwrap_or_default() == "1";

        // [Mac / Linux] 既存の挙動（無条件監視）で正常に動作しているため、
        // 余計な副作用を避けるべく、従来通り無条件に監視を開始する。
        #[cfg(not(target_os = "windows"))]
        let watch_stdin = true;

        if watch_stdin {
            std::thread::spawn(move || {
                use std::io::Read;
                let mut buf = [0; 8];
                loop {
                    match std::io::stdin().read(&mut buf) {
                        Ok(0) => {
                            log::error!("CRITICAL: Stdin pipe closed (EOF). Parent process {} is dead. Exiting now.", pid);
                            // exit 前に道連れプロセスを確実に kill する
                            let pids = child_pids_for_stdin.lock().clone();
                            proc_utils::kill_pids(&pids);
                            // ポートベースのクリーンアップも実行（フェイルセーフ）
                            config_mgr_for_stdin.cleanup_all_backend_ports("Fate-Sharing");
                            token_for_stdin.cancel();
                            return;
                        }
                        Ok(_) => {} // 入力は無視
                        Err(e) => {
                            log::error!("CRITICAL: Stdin read error: {}. Exiting now.", e);
                            let pids = child_pids_for_stdin.lock().clone();
                            proc_utils::kill_pids(&pids);
                            config_mgr_for_stdin.cleanup_all_backend_ports("Fate-Sharing");
                            token_for_stdin.cancel();
                            return;
                        }
                    }
                }
            });
        }

        // 2. カーネルレベル等での明示的な親PID監視 (主に特権昇格用Fallback)
        std::thread::spawn(move || {
            #[cfg(target_os = "macos")]
            monitor_mac_kqueue(pid, child_pids_for_monitor, config_mgr_for_monitor, token_for_monitor);

            #[cfg(not(target_os = "macos"))]
            monitor_process_polling(pid, child_pids_for_monitor, config_mgr_for_monitor, token_for_monitor);
        });
    }
}

#[cfg(target_os = "macos")]
fn monitor_mac_kqueue(
    pid: u32,
    child_pids_for_monitor: Arc<parking_lot::Mutex<Vec<u32>>>,
    config_mgr: Arc<ConfigManager>,
    shutdown_token: CancellationToken,
) {
    unsafe {
        let kq = libc::kqueue();
        if kq < 0 {
            log::error!("kqueue failed. Falling back to polling.");
            monitor_process_polling(pid, child_pids_for_monitor, config_mgr, shutdown_token);
            return;
        }

        let mut changes = [libc::kevent {
            ident: pid as usize,
            filter: libc::EVFILT_PROC,
            flags: libc::EV_ADD | libc::EV_RECEIPT,
            fflags: libc::NOTE_EXIT,
            data: 0,
            udata: std::ptr::null_mut(),
        }];

        let res = libc::kevent(
            kq,
            changes.as_ptr(),
            1,
            changes.as_mut_ptr(),
            1,
            std::ptr::null(),
        );

        if res < 0 || (changes[0].flags & libc::EV_ERROR) != 0 {
            log::error!("kevent registration failed. Falling back to polling.");
            libc::close(kq);
            monitor_process_polling(pid, child_pids_for_monitor, config_mgr, shutdown_token);
            return;
        }

        let mut events = [libc::kevent {
            ident: 0,
            filter: 0,
            flags: 0,
            fflags: 0,
            data: 0,
            udata: std::ptr::null_mut(),
        }];

        let _ = libc::kevent(
            kq,
            std::ptr::null(),
            0,
            events.as_mut_ptr(),
            1,
            std::ptr::null(),
        );

        log::error!(
            "CRITICAL: Parent process {} exited (kqueue NOTE_EXIT). Exiting now.",
            pid
        );
        // exit 前に道連れプロセスを確実に kill する
        let pids = child_pids_for_monitor.lock().clone();
        proc_utils::kill_pids(&pids);
        config_mgr.cleanup_all_backend_ports("Fate-Sharing");
        libc::close(kq);
        shutdown_token.cancel();
    }
}

fn monitor_process_polling(
    _pid: u32,
    child_pids: Arc<parking_lot::Mutex<Vec<u32>>>,
    config_mgr: Arc<ConfigManager>,
    shutdown_token: CancellationToken,
) {
    loop {
        #[cfg(unix)]
        {
            if unsafe { libc::kill(_pid as i32, 0) } != 0 {
                if std::io::Error::last_os_error().raw_os_error().unwrap_or(0) != libc::EPERM {
                    log::error!(
                        "CRITICAL: Parent process {} is no longer running. Exiting now.",
                        _pid
                    );
                    // exit 前に道連れプロセスを確実に kill する
                    let pids = child_pids.lock().clone();
                    proc_utils::kill_pids(&pids);
                    config_mgr.cleanup_all_backend_ports("Fate-Sharing");
                    shutdown_token.cancel();
                    return;
                }
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(1000));
    }
}
