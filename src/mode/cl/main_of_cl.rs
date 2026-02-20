use crate::constants::*;
use crate::hotkey::HotkeyMonitor;
use crate::input::keyboard::KeyboardInjector;
use crate::llm::client::LlmPool;
use crate::mycute_manager::{AppState as MgrAppState, InputMode, MycuteManager};
use crate::stt::SpeechRecognizer;
use crate::stt_config::{ConfigManager, WindowPositionMode};
use crate::tauri_cmd;
use crate::tools::audio;
use crate::tools::text_cleanup::cleanup_final_text;
use crate::types::{
    AppErrorPayload, AppStatusPayload, SttEvent, SttPayload, SttUpdatePayload, TargetPlatform,
    TauriEvent,
};
use crate::utils::db::get_db;
use crate::utils::init::{CommonFlgs, HasCommonFlgs, LogLevel, SharedHttpClients};
use clap::Parser;
use parking_lot::Mutex;
use serde::Serialize;
#[cfg(unix)]
use std::io;
#[cfg(windows)]
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::net::TcpStream;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tauri::{
    async_runtime::{self, spawn},
    window::Color,
    Emitter, LogicalPosition, Manager, WebviewWindowBuilder, WindowEvent,
};
use tokio::time::sleep;

#[cfg(windows)]
use std::fs;
#[cfg(windows)]
use std::fs::File;

use crate::config::settings::{AppRole, Env, DEFAULT_SKEY};
#[cfg(windows)]
use crate::constants::APP_NAME;
use crate::constants::{
    IP_LOCALHOST, LOCK_FILE_APP, LOCK_FILE_SERVER, MYCUTE_SDK_FILENAME, POST_CORRECTION_DECORATION,
    WINDOW_HEIGHT, WINDOW_WIDTH,
};
use crate::enums::Mode;
use crate::mode::cl::sw_server;
use crate::mode::rt::main_of_rt;
use crate::mode::rt::rtbl::replaces_bl;
use crate::myproxy::setup_proxy;
use crate::myproxy::ssl::load_certs;
use crate::myproxy::ssl::setup::create_certs_if_missing;
use crate::stt::stats::UsageStats;
use crate::utils::auth::{spawn_elevated_server, BackendProcessGuard};
#[cfg(windows)]
use crate::utils::my_path::get_log_dir;
use crate::utils::my_path::get_mycute_home;
use crate::utils::singleton;
use anyhow::Result;
use reqwest::Client as ReqwestClient;
#[cfg(windows)]
use std::process::Command;
use std::process::{self};
use tokio::runtime::Runtime;
use tokio::sync::mpsc;

#[derive(Debug, Parser, Serialize)]
#[command(override_usage = "mycute cl [OPTIONS]", ignore_errors = true)]
pub struct CLFlgs {
    #[command(flatten)]
    pub common: CommonFlgs,

    #[arg(
        short = 's',
        help = "Path to settings.json (default: ~/.mycute/settings.json)"
    )]
    pub settings: Option<String>,

    #[arg(
        short = 'r',
        default_value = "gui",
        help = "Role to run: gui=GUI+Helper (default), headless=Server Only"
    )]
    pub role: AppRole,

    /// 運命共同体モード用の親プロセスID監視オプション
    /// 指定されたPIDのプロセスが消失した場合、自己終了します (Fate-Sharing)。
    #[arg(
        long = "parent-pid",
        help = "PID of the parent process to monitor (Fate-Sharing)"
    )]
    pub parent_pid: Option<u32>,

    /// オーナーモード起動用のパスフレーズ
    /// これが指定された場合、サーバーはオーナー秘密鍵の復号を試み、成功すれば特権モードで起動します。
    #[arg(long = "owner", help = "Passphrase to activate Owner Mode")]
    pub owner_passphrase: Option<String>,
}

impl HasCommonFlgs for CLFlgs {
    fn common_flgs(&self) -> &CommonFlgs {
        &self.common
    }
}

pub struct TauriState {
    pub manager: Arc<Mutex<MycuteManager>>,
    pub config_mgr: Arc<ConfigManager>,
    pub llm_pool: Arc<LlmPool>,
    pub backend_guard: Arc<Mutex<Option<BackendProcessGuard>>>,
    pub hc: Arc<ReqwestClient>,
    pub is_hotkey_active: Arc<AtomicBool>,
}
use crate::utils::mod_dl;

pub fn main_of_cl(flgs: CLFlgs, hc: SharedHttpClients) -> Result<()> {
    // MYCUTE_HOME を絶対パスで確定させる
    let home_dir = get_mycute_home(flgs.common.home.clone());
    let home_dir_str = home_dir.to_string_lossy().to_string();
    let config_mgr = Arc::new(ConfigManager::new(
        flgs.common.home.clone(),
        flgs.settings.clone(),
    ));

    // ==============================
    // my_base_url 必須バリデーション (先頭)
    // ==============================
    config_mgr
        .settings
        .read()
        .validate_my_base_url(flgs.owner_passphrase.is_some());

    // ==============================
    // Model Download Check
    // ==============================
    // CLIモードでもGUIモードでも、モデルがないと動かないのでここでチェック・ダウンロードする
    {
        log::info!("Checking AI models...");
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create temp runtime for model check");

        if let Err(e) = rt.block_on(mod_dl::ensure_models(&config_mgr)) {
            log::error!("Failed to download models: {}", e);
            // モデルがないと致命的なので終了
            return Err(anyhow::anyhow!("Failed to download models: {}", e));
        }

        if let Err(e) = config_mgr.validate_models() {
            log::error!("Model validation failed: {}", e);
            return Err(anyhow::anyhow!("Model validation failed: {}", e));
        }
    }

    // オーディオプレイヤーの事前初期化 (デバイスを一回だけ開く)
    audio::init();

    let role = flgs.role;
    log::info!("Starting mycute Client (GUI) mode with role: {:?}", role);

    // ==============================
    // 多重起動防止 (Singleton Lock) — ロール別
    // ==============================
    // SSL証明書セットアップ（パスワードダイアログ）よりも前にロックを取得する。
    // これにより、既に起動中の場合はダイアログを表示せず即座に終了できる。
    match role {
        AppRole::GUI => {
            // GUIプロセスは内部的に Headless サーバーを子プロセスとして生成するため、
            // サーバー用ロック（mycute.lock）はここでは取得しない。
            // 代わりに、GUI専用のロック（mycute-app.lock）を使用して
            // 2つ目のGUIウィンドウの起動を防止する。
            if let Err(e) = singleton::acquire_lock(LOCK_FILE_APP) {
                log::error!("Application lock failed: {}", e);
                eprintln!("Error: {}", e);
                process::exit(1);
            }
        }
        AppRole::Headless => {
            // Headless（サーバー）モードでは、サーバー用ロックを取得する。
            // main_of_rt に到達する前にここで阻止することで、
            // SSL証明書ダイアログの表示や、スレッド内パニック後にプロセスが残る問題を回避する。
            if let Err(e) = singleton::acquire_lock(LOCK_FILE_SERVER) {
                log::error!("Server lock failed: {}", e);
                eprintln!("Error: {}", e);
                process::exit(1);
            }
        }
    }

    // rustls 暗号プロバイダーの初期化 (ring を明示的に使用)
    //
    // AWS SDK (S3 クライアント) が aws-lc-rs を、プロキシ TLS が ring を使用するため
    // 両方のプロバイダーが依存関係に含まれている。rustls はどちらを使うか自動判定できないため、
    // ring を明示的にデフォルトとしてインストールする。
    //
    // 注意: S3 クライアントが削除されても TLS 機能が壊れないよう、ring は明示的な依存として維持すること。
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    // 重要: 証明書のセットアップをここで行う (Direct HTTPS / Phase 8.21)
    // バックエンドサーバー任せにすると、macOSの「信頼ダイアログ」が表示されない(Headless制限)ため、
    // GUIプロセスであるここで実行し、ユーザーに明示的に承認(パスワード入力)させる。
    if let Err(e) = create_certs_if_missing(&config_mgr) {
        log::error!(
            "Failed to setup SSL certificates in Client: {}. Proxy might fail.",
            e
        );
        // ここで失敗しても、サーバー側で再試行される可能性にかけて続行するが、ダイアログは出ないかもしれない。
    }

    // 重要: 証明書の確認を最初期に行う
    // 他のコンポーネントを初期化する前に、証明書が存在するかチェックする
    // これにより、証明書がない場合は即座にエラーを返して終了できる
    let server_config = match load_certs(&config_mgr) {
        Ok(config) => {
            log::info!("SSL certificates loaded successfully.");
            Some(config)
        }
        Err(e) => {
            // エラーを詳細にチェック
            let err_msg = e.to_string();
            if err_msg.contains("not found") {
                log::warn!("SSL certificates not found yet. Proxy server will be skipped (Headless process should generate them).");
            } else {
                log::error!("CRITICAL: SSL certificates exist but failed to load: {}. Please check settings.json.", err_msg);
            }
            None
        }
    };

    // settings.json にサーバー関連の設定がない場合、デフォルトで埋めて保存する
    {
        let settings = config_mgr.settings.write();
        let changed = false;
        if settings.server.rt_skey.is_empty() || settings.server.rt_skey == DEFAULT_SKEY {
            // 秘密鍵がデフォルトのままなら再生成する等の処理をここに入れることも可能
        }
        // 他のフィールドのチェック...
        if changed {
            match config_mgr.save() {
                Ok(_) => log::info!("Settings saved successfully."),
                Err(e) => {
                    anyhow::bail!("Failed to save settings: {}", e);
                }
            }
        }
    }

    // 統計データのパスを設定
    UsageStats::set_base_path(home_dir.clone());

    // ========================================================================
    // Replaces (Business Logic) Initialization for Client
    // ========================================================================
    // Client モードでも辞書 (replaces) をロードする必要があるため、
    // ここで DB に接続し、初期データのシーディングとメモリへのロードを行う。
    // ※ GUI プロセスも独自の DB 接続を持つ (SQLite は複数プロセスからの接続が可能)
    {
        log::info!("Initializing DB connection for Client Replaces...");
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to create runtime for client db init");

        rt.block_on(async {
            // DB Pools の初期化 (Settings からパス等を取得)
            let storage_settings = config_mgr.settings.read().storage.clone();
            let db_env = Env::from_settings(&storage_settings);
            match get_db(&db_env, &LogLevel::Info).await {
                Ok(pools) => {
                    // Client はローカル SQLite (RW) を使用
                    match pools.get_rw() {
                        Ok(conn) => {
                            // 1. デフォルトデータのシーディング (初回のみ)
                            if let Err(e) = replaces_bl::seed_default_replaces(conn).await {
                                log::error!("Failed to seed default replaces: {}", e);
                                // シーディング失敗は致命的ではないがログに残す
                            }

                            // 2. アクティブな辞書を DB からメモリ (config_mgr.replaces) にロード
                            if let Err(e) = config_mgr.reload_replaces(conn).await {
                                log::error!("Failed to load replaces from DB: {}", e);
                            }
                        }
                        Err(e) => log::error!("Failed to get DB connection for Client: {}", e),
                    }
                }
                Err(e) => log::error!("Failed to initialize DbPools for Client: {}", e),
            }
        });
    }

    // バックエンドプロセスをクリーンアップするためのガード
    let backend_guard: Arc<Mutex<Option<BackendProcessGuard>>> = Arc::new(Mutex::new(None));

    // -----------------------------------------------------------------------
    // [特権分離] サーバーマネージャーロジック
    // クライアントモード（GUI）として動作する場合、バックエンドサーバーの稼働を確実にする
    // -----------------------------------------------------------------------
    // クライアントの起動 (GUI ロール)
    if role == AppRole::GUI {
        // [Fate-Sharing] 子プロセス監視 (Watchdog)
        // サーバープロセス (backend_guard) が何らかの理由で終了した場合、クライアントも道連れ終了する。
        // これがないと、「サーバーが死んでるのにGUIだけ残る」ゾンビ状態になる。
        let watchdog_guard = backend_guard.clone();
        thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_secs(3));
                let pid_opt = {
                    let guard = watchdog_guard.lock();
                    guard.as_ref().map(|g| g.pid)
                };

                if let Some(pid) = pid_opt {
                    // 子プロセスが生存しているか確認
                    let is_alive = {
                        #[cfg(unix)]
                        {
                            let res = unsafe { libc::kill(pid as i32, 0) };
                            if res == 0 {
                                true
                            } else {
                                let err = io::Error::last_os_error().raw_os_error().unwrap_or(0);
                                if err == libc::EPERM {
                                    true
                                } else {
                                    false
                                }
                            }
                        }

                        #[cfg(windows)]
                        {
                            let output = Command::new("tasklist")
                                .args(&["/FI", &format!("PID eq {}", pid), "/NH"])
                                .output();
                            match output {
                                Ok(o) => {
                                    String::from_utf8_lossy(&o.stdout).contains(&pid.to_string())
                                }
                                Err(_) => false,
                            }
                        }

                        #[cfg(not(any(unix, windows)))]
                        {
                            true
                        }
                    };

                    if !is_alive {
                        log::error!("CRITICAL: Backend server (PID: {}) died unexpectedy. Client shutting down (Fate-Sharing).", pid);
                        process::exit(1);
                    }
                }
            }
        });

        let rt_port = config_mgr.settings.read().server.rt_port;
        let server_addr = format!("{}:{}", IP_LOCALHOST, rt_port);

        // 接続確認用ヘルパー
        let check_health = || -> bool { TcpStream::connect(&server_addr).is_ok() };

        if check_health() {
            log::info!("Backend server detected at {}. Connecting...", server_addr);
        } else {
            log::info!("Backend server not found. Initiating auto-elevation spawn...");

            // 昇格された特権サーバーを起動
            // 引数: "cl", "-r", "headless" (GUIモード, Role=Headless)
            // 追加: "--parent-pid", "<PID>" (運命共同体モード: 親プロセス監視用)
            // 注意: ここで "cl" "-r" "headless" を指定することで main_of_cl -> role=Headless -> main_of_rt と遷移する
            let cl_mode_str = Mode::CL.as_str();
            let server_role_str = AppRole::Headless.as_arg();
            let current_pid = process::id().to_string();

            let mut args = vec![
                cl_mode_str,
                "-r",
                server_role_str,
                "--parent-pid",
                &current_pid,
                "--home",
                &home_dir_str, // 重要：絶対パス化された MYCUTE_HOME を確実に引き継ぐ
            ];

            // 以前の -s オプションも残すが、基本は --home で十分なはず
            let settings_path_str = config_mgr.path.to_string_lossy().to_string();
            args.push("-s");
            args.push(&settings_path_str);

            #[cfg(windows)]
            {
                // [Windows Trace] ログパイプ実装 (擬似)
                // MYCUTE_HOME/log/mycute_server_YYYYMMDD_HHMMSS.log を出力する
                let log_dir = get_log_dir(&home_dir);
                let now_dt = crate::utils::time::now();
                let timestamp = now_dt.format("%Y%m%d_%H%M%S").to_string();
                let server_log_filename = format!("{}_server_{}.log", APP_NAME, timestamp);
                let log_path = log_dir.join(&server_log_filename);
                let log_path_str = log_path.to_string_lossy().to_string();

                args.push("--output");
                args.push(&log_path_str);
                log::info!("Backend logging pipe (timestamped): {}", log_path_str);

                // パスをスレッドに渡すために保持
                let log_target = log_path.clone();

                if let Some(pass) = &flgs.owner_passphrase {
                    args.push("--owner");
                    args.push(pass);
                }

                match spawn_elevated_server(&args) {
                    Ok(pid) => {
                        log::info!("Elevated server process spawned. PID: {}", pid);
                        if pid > 0 {
                            *backend_guard.lock() =
                                Some(BackendProcessGuard::new(pid, Some(log_path.clone())));

                            // [Log Tailing Thread]
                            // Windowsでは直接パイプできないため、ファイル経由でログをリアルタイム取得して表示する
                            thread::spawn(move || {
                                // ファイルが生成されるまで少し待機
                                thread::sleep(Duration::from_millis(500));
                                let mut position = 0;
                                if let Ok(meta) = fs::metadata(&log_target) {
                                    position = meta.len();
                                }

                                loop {
                                    if let Ok(mut file) = File::open(&log_target) {
                                        if let Ok(_) = file.seek(SeekFrom::Start(position)) {
                                            let reader = BufReader::new(file);
                                            for line in reader.lines() {
                                                if let Ok(l) = line {
                                                    println!("[Backend] {}", l);
                                                    position += l.len() as u64 + 1;
                                                }
                                            }
                                            if let Ok(meta) = fs::metadata(&log_target) {
                                                position = meta.len();
                                            }
                                        }
                                    }
                                    thread::sleep(Duration::from_millis(200));
                                }
                            });
                        }
                        log::info!("Proceeding to launch Tauri immediately.");
                    }
                    Err(e) => {
                        log::error!("Failed to spawn elevated server: {}", e);
                        anyhow::bail!("Failed to spawn elevated server: {}. Please ensure you have administrator privileges.", e);
                    }
                }
            }

            #[cfg(not(windows))]
            {
                if let Some(pass) = &flgs.owner_passphrase {
                    args.push("--owner");
                    args.push(pass);
                }

                match spawn_elevated_server(&args) {
                    Ok(pid) => {
                        log::info!("Elevated server process spawned. PID: {}", pid);
                        if pid > 0 {
                            *backend_guard.lock() = Some(BackendProcessGuard::new(pid, None));
                        }
                        log::info!("Proceeding to launch Tauri immediately.");
                    }
                    Err(e) => {
                        log::error!("Failed to spawn elevated server: {}", e);
                        anyhow::bail!("Failed to spawn elevated server: {}. Please ensure you have administrator privileges.", e);
                    }
                }
            }
        }
    }

    // サーバーの起動 (Headless only)
    // GUIモードの場合は spawn_elevated_server で外部プロセスとして起動しているので、
    // ここで内部的に起動してはいけない（ポート競合する）。
    if role == AppRole::Headless {
        // [Fate-Sharing] 親プロセス監視 (Watchdog)
        // 引数で parent_pid が指定されている場合、そのプロセスが消失したらサーバー自身も終了する
        if let Some(ppid) = flgs.parent_pid {
            log::info!("Starting Watchdog for Parent PID: {}", ppid);
            thread::spawn(move || {
                loop {
                    thread::sleep(Duration::from_secs(3));

                    let is_alive = {
                        #[cfg(unix)]
                        {
                            // Unix: kill(pid, 0) で生存確認。0シグナルはプロセスチェックのみを行う。
                            // 成功すればプロセスは存在する。権限エラー(EPERM)も存在はする。ESRCHなら存在しない。
                            let res = unsafe { libc::kill(ppid as i32, 0) };
                            if res == 0 {
                                true
                            } else {
                                // errno チェックは簡易的に省略し、killが失敗したら死んだとみなす（厳密にはEPERMは生きてる）
                                // しかし自分の親なのでEPERMにはならないはず（親がrootで子が非特権ならあり得るが逆はどうか）
                                // 今回は 親(User) -> 子(Root) なので、RootからUserへはkill(0)送れるはず。
                                let err = io::Error::last_os_error().raw_os_error().unwrap_or(0);
                                if err == libc::EPERM {
                                    true // 生きてるが権限がない
                                } else {
                                    false // おそらく死んでる (ESRCH)
                                }
                            }
                        }

                        #[cfg(windows)]
                        {
                            // Windows: OpenProcess で SYNCHRONIZE 権限で開き、WaitForSingleObject で状態を見る
                            // 簡易実装として tasklist コマンド等を叩く手もあるが、依存を増やしたくないため
                            // ここでは外部コマンド "tasklist" を使用する（パフォーマンスは数秒に1回なら許容）
                            // tasklist /FI "PID eq <ppid>"
                            let output = Command::new("tasklist")
                                .args(&["/FI", &format!("PID eq {}", ppid), "/NH"])
                                .output();

                            match output {
                                Ok(o) => {
                                    let s = String::from_utf8_lossy(&o.stdout);
                                    // PIDが含まれていれば生きているとみなす
                                    s.contains(&ppid.to_string())
                                }
                                Err(_) => false, // コマンド失敗は異常だが、死んだとみなす
                            }
                        }
                        #[cfg(not(any(unix, windows)))]
                        {
                            true // その他のOSでは監視しない
                        }
                    };

                    if !is_alive {
                        log::warn!("Parent process (PID: {}) is gone. Shutting down server (Fate-Sharing).", ppid);
                        process::exit(0);
                    }
                }
            });
        }

        let config_mgr_for_server = config_mgr.clone();
        let env_for_server = Env::from_settings(&config_mgr.settings.read().storage);

        // Spawn server in a separate thread with its own runtime to avoid conflict with Tauri's main loop
        let owner_pass_clone = flgs.owner_passphrase.clone();
        let hc_for_server = hc.async_hc.clone(); // Clone BEFORE move
        thread::spawn(move || {
            let rt = Runtime::new().expect("Failed to create runtime for server");
            rt.block_on(async move {
                main_of_rt(
                    config_mgr_for_server,
                    env_for_server,
                    owner_pass_clone,
                    hc_for_server,
                )
                .await;
            });
        });

        // Headless ロール（Server Only）の場合はここで Tauri を起動せず、待機に回る
        if role == AppRole::Headless {
            println!("Mycute Headless Server is running. Press Ctrl+C to stop.");
            // サーバーが止まるまで無限待機
            loop {
                // これは専用スレッドまたはTauri起動前のメインスレッドなので、シンプルに保ちます。
                // ただし、同期関数内の無限ループはすべてをブロックします。
                // 待ってください、ロールが Server の場合、以下で Tauri を実行していないのでは？
                // 元のコードは Tauri ビルダーにフォールスルーしていました。
                // Server Only の場合、Tauri を実行すべきではありません。
                thread::sleep(Duration::from_secs(3600));
            }
        }
    }

    let (stt_tx, stt_rx) = mpsc::channel(100);

    let settings = config_mgr.settings.read();
    let stt_engine = settings.stt_engine.clone();
    let current_locale = settings.locale;
    let stt_settings = settings.stt.clone();
    let llm_endpoints = settings.llms.clone();
    let hotkey_config = settings.hotkeys.clone();
    let replaces = config_mgr.replaces.clone();

    // 手動でのソートは SpeechRecognizer 内部で処理される
    // SpeechRecognizer::new は IndexMap を直接受け取るように変更された

    drop(settings);

    // LLMプール初期化
    let llm_pool = Arc::new(LlmPool::new(&llm_endpoints));

    let recognizer = match SpeechRecognizer::new(
        stt_tx,
        stt_engine,
        current_locale,
        Some(stt_settings),
        llm_pool.clone(),
        replaces,
    ) {
        Ok(r) => Arc::new(Mutex::new(r)),
        Err(e) => {
            anyhow::bail!("Failed to initialize SpeechRecognizer: {}", e);
        }
    };

    let manager = Arc::new(Mutex::new(MycuteManager::new(
        recognizer.clone(),
        current_locale,
    )));

    // HotkeyMonitor はここでは初期化のみ行い、start() はログイン後に system.rs で実行される
    let _hotkey_monitor = HotkeyMonitor::new(hotkey_config);

    let state = TauriState {
        manager: manager.clone(),
        config_mgr: config_mgr.clone(),
        llm_pool: llm_pool.clone(),
        backend_guard: backend_guard.clone(),
        hc: hc.async_hc.clone(),
        is_hotkey_active: Arc::new(AtomicBool::new(false)),
    };

    // 2. Tauri アプリケーションの構築
    let builder = tauri::Builder::default()
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            tauri_cmd::get_settings,
            tauri_cmd::save_settings,
            tauri_cmd::execute_llm,
            tauri_cmd::start_recording,
            tauri_cmd::stop_recording,
            tauri_cmd::set_locale,
            tauri_cmd::get_usage_stats,
            tauri_cmd::get_hotkey_list,
            tauri_cmd::log_from_frontend,
            tauri_cmd::set_clipboard,
            tauri_cmd::get_clipboard,
            tauri_cmd::check_server_health,
            tauri_cmd::force_shutdown,
            tauri_cmd::enable_hotkey_standby,
            tauri_cmd::disable_hotkey_standby,
            tauri_cmd::toggle_overlay_visibility,
            tauri_cmd::set_overlay_visibility,
        ]);

    // -----------------------------------------------------------------------------------
    // 【MyProxy: カスタムプロトコルによるウェブ表示基盤の導入】
    //
    // [背景と目的]
    // MYCUTE は単なるアプリではなく、OS層として機能することを目指しています。
    // そのため、Google等の外部サイトを「外部ブラウザ」に逃がすのではなく、アプリ内の iframe で
    // シームレスに完結させる必要があります（OS on OS 体験）。
    //
    // [課題]
    // しかし、多くのウェブサイトは X-Frame-Options や Content-Security-Policy (CSP) を設定しており、
    // セキュリティ上の制限から iframe 内での表示を拒否します。
    //
    // [解決策: 透過型プロキシ]
    // この制限を突破するため、Tauri のカスタム URI スキーム機能を利用したプロキシを導入しました。
    // - mycute://  => http:// への転送を担う
    // - mycutes:// => https:// への転送を担う
    //
    // [動作原理]
    // 1. フロントエンド（WebFrame.vue）は `mycutes://www.google.com` のような形式でリクエストを投げます。
    // 2. Tauri 側で登録された `myproxy` モジュールがこのリクエストを横取りし、Rust の `reqwest` で実際のサイトへアクセスします。
    // 3. 取得したレスポンスから、iframe 表示を阻害するヘッダー（X-Frame-Options 等）を Rust 側で剥離します。
    // 4. また、User-Agent をデスクトップブラウザに偽装することで、互換性を確保します。
    //    (定義場所: src/myproxy/myproxy_handler.rs)
    // 5. 最後に、加工したレスポンスをフロントエンドに返すことで、あらゆるサイトを安全かつ統合的に表示可能にします。
    //
    // [メンテナンス上の注意]
    // User-Agent の偽装定義（src/myproxy/myproxy_handler.rs）は固定値リストを使用しています。
    // 時代が進むにつれ、これらのバージョン番号は「古いブラウザ」と判定されるようになるため、
    // 定期的に最新のモダンブラウザの User-Agent に更新し続ける必要があります。
    // -----------------------------------------------------------------------------------
    let sw_port = config_mgr.settings.read().server.sw_port; // プロキシサーバーの設定
    let builder = setup_proxy(builder, sw_port, hc.blocking_hc.clone());

    // server_config を Tauri setup 内で使用するためにクローン
    let server_config_for_tauri = server_config;

    builder
        .setup(move |app| {
            // [Windows / WebView2]
            // WebView2 (Chromium) は起動オプションでのフラグ指定を公式にサポートしています。
            #[allow(unused_mut)]
            let mut browser_args = "--enable-features=msWebView2EnableDraggableRegions".to_string();

            // Windowの生成（プログラム制御）- 初期化
            let window_builder =
                WebviewWindowBuilder::new(app, WINDOW_LABEL_MAIN, tauri::WebviewUrl::default())
                    .title("")
                    .inner_size(WINDOW_WIDTH, WINDOW_HEIGHT)
                    .resizable(false)
                    .fullscreen(false)
                    .decorations(false)
                    .transparent(true)
                    .visible(false)
                    .additional_browser_args(&browser_args);

            // Direct Hosting Architecture (Phase 8.20)
            // プロキシサーバーの本体は Headless サーバープロセス側で動作します。
            // GUI側での重複起動は避けます。
            if server_config_for_tauri.is_some() {
                log::info!(
                    "System Server (Direct HTTPS) should be running via background process."
                );
            } else {
                log::warn!(
                    "System Server (Direct HTTPS) may not be available (SSL config missing)."
                );
            }

            let window = window_builder
                .build()
                .expect("failed to create main window");

            // SDK ホスティングサーバー（Static Web Server）の起動 (GUI ロール)
            if role == AppRole::GUI {
                let app_handle = app.handle().clone();
                let config_mgr_for_sw = config_mgr.clone();
                let hc_for_sw = hc.async_hc.clone();
                async_runtime::spawn(async move {
                    sw_server::run_sw_server(sw_port, app_handle, config_mgr_for_sw, hc_for_sw)
                        .await;
                });
            }

            // 1. WebView の下地を完全に透明にする (RGBA: 0,0,0,0)
            let _ = window.set_background_color(Some(Color(0, 0, 0, 0)));

            // 2. ウィンドウの影を無効にする
            let _ = window.set_shadow(false);

            // Windows の場合、このメインスレッドのタイミングで表示を行わないとウィンドウが出現しない。
            #[cfg(target_os = "windows")]
            {
                let _ = window.show();
            }

            // デッドロック回避 (Windows): WebView2ウィンドウ生成時、メッセージループの無いスレッドでブロックする問題がある。
            // 運命共同体（Fate-sharing）を防ぎ、UIハング時でも音声入力が機能するようSTTイベントループを独立した非同期タスクとして分離・先行起動する。
            spawn_stt_event_bridge(app.handle().clone(), manager.clone(), stt_rx);

            // バックグラウンドタスク 2: 追加のウィンドウ（オーバーレイ、スナックバー）生成
            // ここでブロックが発生しても、タスク1（STT）は影響を受けない
            let app_handle_for_ui = app.handle().clone();
            let config_mgr_for_ui = config_mgr.clone();

            async_runtime::spawn(async move {
                setup_extra_ui_windows(app_handle_for_ui, config_mgr_for_ui).await;
            }); // tauri::async_runtime::spawn for window init end

            // バックグラウンドタスク: ホットキーハンドラ
            // enable_hotkey_standby コマンド内で実行されるため、ここでは起動しない

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    Ok(())
}

/// STT イベントブリッジを独立したタスクとして起動する。
/// 音声認識結果を受信し、キーボード入力の注入やオーバーレイUIへの通知を制御する。
fn spawn_stt_event_bridge(
    handle: tauri::AppHandle,
    manager: Arc<Mutex<MycuteManager>>,
    mut stt_rx: mpsc::Receiver<SttEvent>,
) {
    async_runtime::spawn(async move {
        let mut injected_text = String::new();
        let mut pending_event: Option<SttEvent> = None;

        loop {
            // 1. 優先度付きでイベントを取得 (待避中のイベントがあればそれを優先)
            let mut event = if let Some(e) = pending_event.take() {
                e
            } else if let Some(e) = stt_rx.recv().await {
                e
            } else {
                break; // チャンネルが閉じた場合
            };

            // 2. セッション開始時に強制リセット
            if matches!(event, SttEvent::Started) {
                injected_text.clear();
                // オーバーレイ全文表示用バッファもリセット
                manager.lock().buffer.clear();
            }

            // 3. 連続する文字入力イベントを効率化のために統合する
            let mut latest_text = None;
            let mut latest_seq = 0;
            let mut is_final = false;

            loop {
                match event {
                    SttEvent::PartialResult(text, seq) => {
                        if seq >= latest_seq {
                            latest_text = Some(text);
                            latest_seq = seq;
                        }
                    }
                    SttEvent::FinalResult(text, seq) => {
                        if seq >= latest_seq {
                            latest_text = Some(text);
                            latest_seq = seq;
                        }
                        is_final = true;
                    }
                    SttEvent::Started => {
                        injected_text.clear();
                    }
                    SttEvent::Stopped => {
                        injected_text.clear();
                        // オーバーレイ全文表示用バッファもリセット
                        manager.lock().buffer.clear();
                        // コミット音と同期してオーバーレイ消去イベントを発信
                        audio::play_commit_sound();
                        let _ = handle.emit(TauriEvent::SttCommit.as_str(), ());
                        let _ = handle.emit(
                            TauriEvent::AppStatus.as_str(),
                            AppStatusPayload {
                                status: APP_STATUS_STOPPED.to_string(),
                            },
                        );
                    }
                    SttEvent::Error(msg) => {
                        // エラー時もコミット音と同期してオーバーレイ消去イベントを発信
                        audio::play_commit_sound();
                        let _ = handle.emit(TauriEvent::SttCommit.as_str(), ());
                        let _ = handle.emit(
                            TauriEvent::AppError.as_str(),
                            AppErrorPayload { error: msg },
                        );
                    }
                    SttEvent::Ready => {
                        // 録音準備完了・開始
                        audio::play_ready_sound();
                    }
                    SttEvent::PostCorrectionStarted => {
                        // 重要な入力制御イベントなので、即座に処理（副作用を実行）して、ループは継続
                        let mut mgr = manager.lock();
                        if mgr.state == MgrAppState::Recording && !mgr.is_post_correcting {
                            // 装飾情報を付与
                            let decoration = POST_CORRECTION_DECORATION;
                            KeyboardInjector::type_text(decoration);

                            // 内部状態と物理的な打鍵内容を同期 (重要!)
                            mgr.current_text.push_str(decoration);
                            injected_text = mgr.current_text.clone();

                            let _ = handle.emit(
                                TauriEvent::SttPartial.as_str(),
                                SttPayload {
                                    text: mgr.current_text.clone(),
                                    seq: mgr.last_stt_seq,
                                },
                            );
                        }
                    }
                    SttEvent::PostCorrectionFinished => {
                        let mut mgr = manager.lock();
                        if mgr.is_post_correcting {
                            mgr.is_post_correcting = false;
                            let _ = handle.emit(
                                TauriEvent::SttPartial.as_str(),
                                SttPayload {
                                    text: mgr.current_text.clone(),
                                    seq: mgr.last_stt_seq,
                                },
                            );
                        }
                    }
                    _ => {}
                }

                // 次のイベントがあれば取り出す。文字入力以外の重要なイベントなら、統合を止めて次へ回す
                if let Ok(next) = stt_rx.try_recv() {
                    if matches!(
                        next,
                        SttEvent::PartialResult(..) | SttEvent::FinalResult(..)
                    ) {
                        event = next;
                        continue;
                    } else {
                        // 重要な制御イベント（Start/Stop/Error）が来たので、
                        // 次のループで処理するために保持してブレイクする。
                        pending_event = Some(next);
                        break;
                    }
                }
                break;
            }

            // 統合されたテキストがあれば入力を実行
            if let Some(mut text) = latest_text {
                let mut mgr = manager.lock();
                if latest_seq >= mgr.last_stt_seq {
                    mgr.last_stt_seq = latest_seq;

                    // 不自然な句読点や断片を除去したり、「ですか」の「？」を補強したりする
                    // if is_final {
                    let cleaned = cleanup_final_text(&text);
                    if cleaned != text {
                        log::debug!("[Cleanup] '{}' -> '{}'", text, cleaned);
                        text = cleaned;
                    }
                    // }

                    mgr.current_text = text.clone();

                    // オーバーレイ用: 確定済みバッファ + 最新スライスを連結した全文を送信
                    let overlay_full_text = format!("{}{}", mgr.buffer, text);
                    let _ = handle.emit(
                        TauriEvent::SttUpdate.as_str(),
                        SttUpdatePayload {
                            text: overlay_full_text,
                        },
                    );

                    if mgr.state == MgrAppState::Recording && mgr.input_mode == InputMode::RealTime
                    {
                        KeyboardInjector::input_diff(&injected_text, &text);
                        injected_text = text.clone();
                    }

                    if is_final {
                        // 確定したスライスをバッファに蓄積（次回以降の全文合成に使用）
                        mgr.buffer.push_str(&text);
                        injected_text.clear();
                    }

                    drop(mgr);
                    let tauri_evt = if is_final {
                        TauriEvent::SttFinal
                    } else {
                        TauriEvent::SttPartial
                    };
                    let _ = handle.emit(
                        tauri_evt.as_str(),
                        SttPayload {
                            text,
                            seq: latest_seq,
                        },
                    );
                }
            }
        } // End of outer loop (stt_rx.recv)
    });
}

/// 追加のUIウィンドウ（オーバーレイ、スナックバー）やメインウィンドウの初期設定をまとめて行う。
/// デッドロック回避のため、非同期タスク内から呼び出される。
async fn setup_extra_ui_windows(handle: tauri::AppHandle, config_mgr: Arc<ConfigManager>) {
    // OS/WebView2の初期化が安定し、メインウィンドウの描画が完了するまでの安全マージン
    tokio::time::sleep(Duration::from_millis(1500)).await;

    // 1. オーバーレイウィンドウの生成
    setup_overlay_window(&handle, config_mgr.clone()).await;

    // 2. スナックバーウィンドウの生成
    setup_snackbar_window(&handle).await;

    // 3. メインウィンドウへの SDK 注入
    inject_sdk_to_main_window(&handle).await;

    // 4. メインウィンドウの表示位置最適化
    optimize_main_window_position(&handle, config_mgr).await;

    // 5. すべての準備（位置確定）が整ったら、表示する。 (Plan A)
    // macOS では位置のジャンプを防ぐため、最適化が完了したこのタイミングで表示する。
    #[cfg(target_os = "macos")]
    if let Some(main_win) = handle.get_webview_window(WINDOW_LABEL_MAIN) {
        let _ = main_win.show();
    }
}

/// オーバーレイウィンドウの適切な表示座標（論理座標）を算出する。
/// 保存された相対座標をモニターの絶対座標に変換し、画面外ロスト時の救済措置も行う。
fn calculate_overlay_bounds(
    handle: &tauri::AppHandle,
    config_mgr: &ConfigManager,
) -> (f64, f64, f64, f64) {
    // ============================================================
    // Tauri サブウィンドウの生成: オーバーレイ & スナックバー
    // ============================================================

    // --- オーバーレイウィンドウ ---
    // 認識中のテキストを表示する透明ウィンドウ。
    // マウス透過を有効にし、背後のウィンドウ操作を妨げない。
    let initial_overlay_state = {
        let settings = config_mgr.settings.read();
        settings.overlay_state.clone()
    };

    // =============================================================
    // 起動時の座標復元ロジック
    // 【設計方針】
    // 全ての値（位置・サイズ）を「論理ピクセル」で管理する。
    // 位置はメインディスプレイ基準の相対値、サイズは論理絶対値。
    // Tauri の WebviewWindowBuilder にはそのまま論理値を渡し、
    // OS側が各ディスプレイのスケールに応じた物理描画を処理する。
    // =============================================================

    // 論理座標をそのまま使用（ビルダーに渡す用）。
    let mut overlay_x = initial_overlay_state.x as f64;
    let mut overlay_y = initial_overlay_state.y as f64;
    let overlay_w = initial_overlay_state.width;
    let overlay_h = initial_overlay_state.height;

    // メインディスプレイ情報を取得し、保存された相対座標を絶対論理座標に変換。
    if let Some(primary_m) = handle.primary_monitor().ok().flatten().or_else(|| {
        handle
            .available_monitors()
            .ok()
            .and_then(|m| m.first().cloned())
    }) {
        let m_scale = primary_m.scale_factor();
        let m_logical_pos = primary_m.position().to_logical::<f64>(m_scale);

        // メインディスプレイの論理座標に、保存値（論理相対）を加算。
        overlay_x = m_logical_pos.x + initial_overlay_state.x as f64;
        overlay_y = m_logical_pos.y + initial_overlay_state.y as f64;

        // ロスト防止: 算出した論理座標がいずれかのモニターの表示領域内に完全に収まっているか確認。
        let mut is_fully_visible = false;
        if let Ok(available_monitors) = handle.available_monitors() {
            for m in &available_monitors {
                let s = m.scale_factor();
                let m_logical_pos = m.position().to_logical::<f64>(s);
                let m_logical_size = m.size().to_logical::<f64>(s);

                // 各モニターの論理座標系で「完全包含」を判定
                let contains_x = overlay_x >= m_logical_pos.x
                    && (overlay_x + overlay_w) <= (m_logical_pos.x + m_logical_size.width);
                let contains_y = overlay_y >= m_logical_pos.y
                    && (overlay_y + overlay_h) <= (m_logical_pos.y + m_logical_size.height);

                if contains_x && contains_y {
                    is_fully_visible = true;
                    break; // 完全に収まっているモニターが1つでもあればOK
                }
            }
        }

        // はみ出している場合、プライマリーモニターの中央へ強制リセット（救済措置）。
        if !is_fully_visible {
            log::warn!("Overlay position is partially or completely out of bounds. Rescuing to primary monitor center.");
            let m_logical_size = primary_m.size().to_logical::<f64>(m_scale);

            // 中央座標を算出。算出した座標がマイナス（画面外）にならないよう max(0.0) で保護
            overlay_x = m_logical_pos.x + ((m_logical_size.width - overlay_w) / 2.0).max(0.0);
            overlay_y = m_logical_pos.y + ((m_logical_size.height - overlay_h) / 2.0).max(0.0);
        }

        log::info!(
            "Overlay logical position: ({}, {}), logical size: {}x{}",
            overlay_x,
            overlay_y,
            overlay_w,
            overlay_h
        );
    }

    (overlay_x, overlay_y, overlay_w, overlay_h)
}

/// オーバーレイウィンドウを生成し、背景透過や座標永続化イベントをセットアップする。
async fn setup_overlay_window(handle: &tauri::AppHandle, config_mgr: Arc<ConfigManager>) {
    // 座標計算ロジックを独立した関数にオフロード
    let (x, y, w, h) = calculate_overlay_bounds(handle, &config_mgr);

    let overlay_window = WebviewWindowBuilder::new(
        handle,
        WINDOW_LABEL_OVERLAY,
        tauri::WebviewUrl::App(WINDOW_URL_OVERLAY.into()),
    )
    .title("")
    .inner_size(w, h)
    .position(x, y)
    .resizable(true)
    .decorations(false)
    .transparent(true)
    .always_on_top(true)
    .visible(true)
    .build()
    .expect("Failed to create overlay window");

    let _ = overlay_window.set_background_color(Some(Color(0, 0, 0, 0)));
    let _ = overlay_window.set_shadow(false);
    let _ = overlay_window.set_ignore_cursor_events(false);

    // 状態をemit
    let is_visible = overlay_window.is_visible().unwrap_or(false);
    let _ = handle.emit(EVENT_OVERLAY_VISIBILITY, is_visible);
    log::info!(
        "Event emitted: {} ({})",
        EVENT_OVERLAY_VISIBILITY,
        is_visible
    );

    // ウィンドウイベントによる座標保存（デバウンス対応）
    let config_mgr_for_events = config_mgr.clone();
    let pending_save_task = Arc::new(Mutex::new(None::<tauri::async_runtime::JoinHandle<()>>));
    let handle_for_events = handle.clone();
    let overlay_ready = Arc::new(std::sync::atomic::AtomicBool::new(true));

    overlay_window.on_window_event(move |event| {
        if !overlay_ready.load(std::sync::atomic::Ordering::Relaxed) {
            return;
        }

        let mut state_changed = false;
        match event {
            WindowEvent::Moved(pos) => {
                if let Some(overlay_win) =
                    handle_for_events.get_webview_window(WINDOW_LABEL_OVERLAY)
                {
                    let win_scale = overlay_win.scale_factor().unwrap_or(1.0);
                    let logical_pos = pos.to_logical::<f64>(win_scale);

                    if let Some(primary_m) = handle_for_events.primary_monitor().ok().flatten() {
                        let m_scale = primary_m.scale_factor();
                        let m_logical_pos = primary_m.position().to_logical::<f64>(m_scale);

                        let mut settings = config_mgr_for_events.settings.write();
                        settings.overlay_state.x = (logical_pos.x - m_logical_pos.x) as i32;
                        settings.overlay_state.y = (logical_pos.y - m_logical_pos.y) as i32;
                        state_changed = true;
                    }
                }
            }
            WindowEvent::Resized(size) => {
                if let Some(overlay_win) =
                    handle_for_events.get_webview_window(WINDOW_LABEL_OVERLAY)
                {
                    let win_scale = overlay_win.scale_factor().unwrap_or(1.0);
                    let logical_size = size.to_logical::<f64>(win_scale);

                    let mut settings = config_mgr_for_events.settings.write();
                    settings.overlay_state.width = logical_size.width;
                    settings.overlay_state.height = logical_size.height;
                    state_changed = true;
                }
            }
            _ => {}
        }

        if state_changed {
            let config_mgr_inner = config_mgr_for_events.clone();
            let pending_lock = pending_save_task.clone();
            *pending_lock.lock() = Some(spawn(async move {
                sleep(Duration::from_millis(500)).await;
                if let Err(e) = config_mgr_inner.save() {
                    log::error!("Failed to save overlay state: {}", e);
                } else {
                    log::debug!("Overlay state saved to disk (debounced).");
                }
            }));
        }
    });
}

/// スナックバーウィンドウ（通知用）を生成する。
async fn setup_snackbar_window(handle: &tauri::AppHandle) {
    let snackbar_window = WebviewWindowBuilder::new(
        handle,
        WINDOW_LABEL_SNACKBAR,
        tauri::WebviewUrl::App(WINDOW_URL_SNACKBAR.into()),
    )
    .title("")
    .inner_size(350.0, 80.0)
    .resizable(false)
    .decorations(false)
    .transparent(true)
    .always_on_top(true)
    .visible(false)
    .build()
    .expect("Failed to create snackbar window");

    let _ = snackbar_window.set_background_color(Some(Color(0, 0, 0, 0)));
    let _ = snackbar_window.set_shadow(false);
}

/// メインウィンドウに対して MYCUTE SDK を自動注入する。
async fn inject_sdk_to_main_window(handle: &tauri::AppHandle) {
    let init_script = format!(
        r#"
        (function() {{
            if (!window.location.protocol.startsWith("mycute")) return;
            const script = document.createElement("script");
            script.src = "/{}";
            script.type = "module";
            script.async = false;
            document.head.appendChild(script);
        }})();
    "#,
        MYCUTE_SDK_FILENAME
    );

    if let Some(main_win) = handle.get_webview_window(WINDOW_LABEL_MAIN) {
        let _ = main_win.eval(&init_script);
        let platform = TargetPlatform::current();
        let _ = main_win.eval(&format!(
            "window.__MYCUTE_PLATFORM__ = '{}';",
            platform.as_str()
        ));
    }
}

/// メインウィンドウの表示位置をモニターの有効領域に合わせて最適化する。
async fn optimize_main_window_position(handle: &tauri::AppHandle, config_mgr: Arc<ConfigManager>) {
    if let Ok(Some(monitor)) = handle.primary_monitor() {
        let scale = monitor.scale_factor();

        #[cfg(not(target_os = "windows"))]
        let (m_logical_pos, m_logical_size) = {
            (
                monitor.position().to_logical::<f64>(scale),
                monitor.size().to_logical::<f64>(scale),
            )
        };

        #[cfg(target_os = "windows")]
        let (m_logical_pos, m_logical_size) = {
            let work_area = monitor.work_area();
            (
                work_area.position.to_logical::<f64>(scale),
                work_area.size.to_logical::<f64>(scale),
            )
        };

        let win_w = WINDOW_WIDTH;
        let win_h = WINDOW_HEIGHT;
        let settings = config_mgr.settings.read();
        let pos_cfg = &settings.window_position;

        let (x, y) = match pos_cfg.mode {
            WindowPositionMode::TopLeft => {
                let x = m_logical_pos.x + pos_cfg.left as f64;
                let y = m_logical_pos.y + pos_cfg.top as f64;
                (x, y)
            }
            WindowPositionMode::BottomLeft => {
                let x = m_logical_pos.x + pos_cfg.left as f64;
                let y = m_logical_pos.y + m_logical_size.height - win_h - pos_cfg.bottom as f64;
                (x, y)
            }
            WindowPositionMode::TopRight => {
                let x = m_logical_pos.x + m_logical_size.width - win_w - pos_cfg.right as f64;
                let y = m_logical_pos.y + pos_cfg.top as f64;
                (x, y)
            }
            WindowPositionMode::BottomRight => {
                let x = m_logical_pos.x + m_logical_size.width - win_w - pos_cfg.right as f64;
                let y = m_logical_pos.y + m_logical_size.height - win_h - pos_cfg.bottom as f64;
                (x, y)
            }
        };

        if let Some(main_win) = handle.get_webview_window(WINDOW_LABEL_MAIN) {
            let _ = main_win.set_position(LogicalPosition { x, y });
        }
        log::info!(
            "Window positioned at logical: ({}, {}), mode: {:?}",
            x,
            y,
            pos_cfg.mode
        );
    }
}
