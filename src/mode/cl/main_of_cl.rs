use crate::config::settings::Env;
use crate::constants::{
    APP_NAME, APP_STATE_IDLE, APP_STATUS_STOPPED, IP_LOCALHOST, LMGW_CLIENT_JWT_AID, LMGW_CLIENT_JWT_EMAIL,
    LMGW_CLIENT_JWT_EXPIRE_HOURS, LMGW_CLIENT_JWT_UID, LMGW_CLIENT_JWT_VID,
    MYCUTE_SDK_FILENAME, MYCUTE_VERSION, PATH_MYCUTE_WS,
    PROTOCOL_WS, SETTING_KEY_LAST_RUN_VERSION, SSE_TIMEOUT_DURATION, WINDOW_HEIGHT,
    WINDOW_LABEL_MAIN, WINDOW_WIDTH, DEFAULT_LLM_MODEL,
};
use crate::hotkey::HotkeyMonitor;
use crate::input::clipboard;
use crate::input::keyboard::KeyboardInjector;
use crate::llm::client::LmgwClient;
use crate::migration::{Migrator, MigratorTrait};
use crate::mode::cl::sw_server;
use crate::mode::rt::rtbl::replaces_bl;
use crate::mycute_manager::{AppState as MgrAppState, InputMode, MycuteManager};
use crate::mycute_settings::{ConfigManager, WindowPositionMode};
use crate::myproxy::setup_proxy;
use crate::myproxy::ssl::load_certs;
use crate::myproxy::ssl::setup::create_certs_if_missing;
use crate::stt::stats::UsageStats;
use crate::stt::SpeechRecognizer;
use crate::tauri_cmd;
#[cfg(target_os = "macos")]
use crate::hotkey_mac;
#[cfg(windows)]
use crate::hotkey_win;
use crate::time::now;
use crate::tools::audio;
use crate::tools::text_cleanup::cleanup_final_text;
use crate::types::{
    AppCaStatusChangedPayload, AppErrorPayload, AppLicensesChangedPayload,
    AppLmgwProvidersChangedPayload, AppLocaleChangedPayload, AppOwnerStatusChangedPayload,
    AppStatePayload, AppStatusPayload, AppSttEngineChangedPayload, EventKind, SttEvent, SttPayload,
    SttUpdatePayload, TargetPlatform, TauriEvent, WsClientMessage, WsClientRole, WsServerMessage,
};
use crate::utils::auth::{self, BackendProcessGuard};
use crate::utils::db::{get_db, DbPools};
use crate::utils::init::{CommonFlgs, HasCommonFlgs, LogLevel, SharedHttpClients};
use crate::utils::mod_dl;
use crate::utils::my_path::{get_log_dir, get_mycute_home};
use crate::utils::process as proc_utils;
use crate::utils::singleton::acquire_lock;
use anyhow::{anyhow, Context, Result};
use clap::Parser;
use tokio::runtime::Builder;
use futures_util::{SinkExt, StreamExt};
use parking_lot::Mutex;
#[cfg(windows)]
use proc_utils::CommandExtSafe;
#[cfg(windows)]
use std::process::Command;
use serde::Serialize;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tauri::{
    async_runtime::{self},
    window::Color,
    Emitter, LogicalPosition, Manager, RunEvent, WebviewWindowBuilder,
};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Debug, Parser, Serialize)]
#[command(override_usage = "mycute cl [OPTIONS]", ignore_errors = true)]
pub struct CLFlgs {
    #[command(flatten)]
    pub common: CommonFlgs,
}

impl HasCommonFlgs for CLFlgs {
    fn common_flgs(&self) -> &CommonFlgs {
        &self.common
    }
}

pub struct TauriState {
    pub manager: Arc<Mutex<MycuteManager>>,
    pub config_mgr: Arc<ConfigManager>,
    /// LMGW 専用クライアント。補正・要約・ホットキー LLM 処理に使用する。
    pub lmgw_client: Arc<LmgwClient>,
    pub backend_guard: Arc<Mutex<Option<BackendProcessGuard>>>,
    /// 再起動ハンドオフ中に退避した BackendProcessGuard を保持する。
    /// Exit ハンドラはこちらを優先して確認し、存在すれば kill せずに破棄する。
    pub restart_guard_holder: Arc<Mutex<Option<BackendProcessGuard>>>,
    pub hc: Arc<reqwest::Client>,
    pub is_hotkey_active: Arc<AtomicBool>,
    pub is_shutting_down: Arc<AtomicBool>,
}

pub fn main_of_cl(flgs: CLFlgs, hc: SharedHttpClients) -> Result<()> {
    // MYCUTE_HOME を絶対パスで確定させる
    let home_dir = get_mycute_home(flgs.common.home.clone());
    // [Bootstrap] とりあえず設定の読み込みとディレクトリ作成のみを行う
    let config_mgr = Arc::new(ConfigManager::new_bootstrap(flgs.common.home.clone())?);

    // ==============================
    // [Main GUI Flow]
    // ==============================
    log::info!("Starting Main GUI process");

    // ==============================
    // my_base_url 必須バリデーション (先頭)
    // ==============================
    config_mgr.settings.read().validate_my_base_url()?;
    log::info!("Starting mycute Client (GUI) mode");

    // ==============================
    // [GUI/Server 分離リソース初期化]
    // ==============================
    // 1. オーディオプレイヤーの事前初期化 (デバイスを一回だけ開く)
    if let Err(e) = audio::init() {
        log::error!("Audio system failed to initialize: {}", e);
    }

    // 2. Model Download Check
    // GUIモードの場合は、モデルがないと音声認識が動かないためここでチェック・ダウンロードする
    log::info!("Checking AI models...");
    let rt = Builder::new_current_thread()
        .enable_all()
        .build()
        .context("Failed to create temp runtime for model check")?;

    if let Err(e) = rt.block_on(mod_dl::ensure_models(&config_mgr)) {
        log::error!("Failed to download models: {}", e);
        // モデルがないと致命的なので終了
        return Err(anyhow!("Failed to download models: {}", e));
    }

    if let Err(e) = config_mgr.validate_models() {
        log::error!("Model validation failed: {}", e);
        return Err(anyhow!("Model validation failed: {}", e));
    }

    // 2つ目のGUIウィンドウの起動を防止する
    if let Err(e) = acquire_lock(&format!("{}-app.lock", APP_NAME)) {
        return Err(anyhow!(
            "Application lock failed: Another instance of {} is already running: {}",
            APP_NAME,
            e
        ));
    }

    // ==============================
    // rustls 暗号プロバイダーの初期化 (ring をデフォルトに設定)
    // ==============================
    // 依存関係（reqwest, sea-orm 等）に複数の暗号プロバイダー（ring, aws-lc-rs）が
    // 含まれている場合、rustls 0.23 以降では明示的な初期化が必須となる。
    // 本プロジェクトでは、プロキシサーバーや各クライアントで ring を標準として採用する。
    rustls::crypto::ring::default_provider()
        .install_default()
        .map_err(|_| anyhow!("Failed to install rustls crypto provider"))?;

    // ==============================
    // DB 初期化（同期ラッパー内で非同期実行）
    // ==============================
    let db_pools = init_client_db(&config_mgr).map_err(|e| {
        log::error!("Failed to initialize client DB: {}", e);
        e
    })?;

    // ==============================
    // [Live] ConfigManager を DB 付きで再初期化・完全移行
    // ==============================
    // ここで生成される config_mgr が「唯一の正真な」インスタンスとなる。
    // DB 接続が含まれているため、これ以降の save_db() は全て有効。
    let mut live = ConfigManager::new_live(db_pools.clone(), flgs.common.home.clone())?;
    // Boot用(config_mgr)で既にロード済みの辞書データを引き継ぐ (DB二重アクセス防止)
    live.replaces = config_mgr.replaces.clone();
    live.replaces_active_ids = config_mgr.replaces_active_ids.clone();
    let config_mgr = Arc::new(live);

    // DBから最新の設定をメモリにロード (初回起動時は Example から投入)
    {
        let rt_ld = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .context("Failed to create runtime for config load")?;
        rt_ld
            .block_on(config_mgr.ensure_initialized_with_db())
            .context("Failed to ensure settings initialization in DB")?;
        // macOS: 次回起動時のバージョン不一致による tccutil reset を防止するため、
        // 現在のバージョンを DB に書き込んでおく（ベストエフォート）
        let _ = rt_ld.block_on(config_mgr.set_value_to_db(
            SETTING_KEY_LAST_RUN_VERSION,
            serde_json::Value::String(MYCUTE_VERSION.to_string()),
        ));
    }

    // ==============================
    // SSL証明書のセットアップ
    // ==============================
    // 重要: 証明書のセットアップをここで行う (Direct HTTPS)
    // バックエンドサーバー任せにすると、macOSの「信頼ダイアログ」が表示されない(Headless制限)ため、
    // GUIプロセスであるここで実行し、ユーザーに明示的に承認(パスワード入力)させる。
    // また、DB 接続が確立された後（save_db が有効な状態）で実行する必要がある。
    {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .context("Failed to create temp runtime for SSL check")?;

        if let Err(e) = rt.block_on(create_certs_if_missing(&config_mgr)) {
            log::error!(
                "Failed to setup SSL certificates in Client: {}. Proxy might fail.",
                e
            );
        }
    }

    // ==============================
    // SSL証明書の確認
    // ==============================
    let server_config = check_ssl_certificates(&config_mgr)?;

    // ==============================
    // 統計データのパスを設定
    // ==============================
    UsageStats::set_base_path(home_dir.clone());

    // バックエンドプロセスをクリーンアップするためのガード
    let backend_guard: Arc<Mutex<Option<BackendProcessGuard>>> = Arc::new(Mutex::new(None));

    // シャットダウン信号（CancellationToken）の初期化
    let shutdown_token = CancellationToken::new();

    let is_shutting_down = Arc::new(AtomicBool::new(false));

    // ==============================
    // サーバーマネージャーロジック (監視・起動)
    // ==============================
    manage_backend_server(
        &config_mgr,
        &db_pools,
        home_dir.clone(),
        backend_guard.clone(),
        shutdown_token.clone(),
        is_shutting_down.clone(),
    )?;

    let (stt_tx, stt_rx) = mpsc::channel(100);

    let settings = config_mgr.settings.read();
    let stt_engine = settings.stt_engine.clone();
    let current_locale = settings.locale.clone();
    let stt_settings = settings.stt.clone();
    let hotkey_config = settings.hotkeys.clone();
    let replaces = config_mgr.replaces.clone();
    // CL が LMGW にアクセスするための JWT を生成する。
    // RT とは別プロセスで動作するため、共有設定の rt_skey を使って CL 側で独自に JWT を生成する。
    let rt_port = settings.server.rt_port;
    let rt_skey = settings.server.rt_skey.clone();
    drop(settings);

    // CL → LMGW 通信用 JWT の生成
    // CL と RT は同一バイナリの別プロセスとして動作するため、
    // RT が生成した JWT を直接参照することはできない。
    // 代わりに、共有の rt_skey から CL 専用のシステム JWT を生成して使用する。
    let lmgw_jwt = crate::utils::jwt::generate_token_for_usr(
        &rt_skey,
        LMGW_CLIENT_JWT_AID,
        LMGW_CLIENT_JWT_VID,
        LMGW_CLIENT_JWT_UID,
        LMGW_CLIENT_JWT_EMAIL.to_string(),
        LMGW_CLIENT_JWT_EXPIRE_HOURS,
    ).unwrap_or_else(|e| {
        log::error!("Failed to generate LmgwClient JWT (fallback to empty string): {}", e);
        String::new()
    });

    // LmgwClient の初期化
    // LMGW (Bifrost Proxy) は RT サーバーの PATH_LMGW_OPENAI_V1 パスに接続する。
    let lmgw_client = Arc::new(LmgwClient::new(rt_port, &lmgw_jwt, DEFAULT_LLM_MODEL));

    // SpeechRecognizer の初期化
    let recognizer = match SpeechRecognizer::new(
        stt_tx,
        stt_engine,
        current_locale,
        Some(stt_settings),
        lmgw_client.clone(),
        replaces,
    ) {
        Ok(r) => Arc::new(Mutex::new(r)),
        Err(e) => {
            anyhow::bail!("Failed to initialize SpeechRecognizer: {}", e);
        }
    };

    // MycuteManager の初期化
    let manager = Arc::new(Mutex::new(MycuteManager::new(
        recognizer.clone(),
        current_locale,
    )));

    // HotkeyMonitor はここでは初期化のみ行い、start() はログイン後に system.rs で実行される
    let _hotkey_monitor = HotkeyMonitor::new(hotkey_config);

    // ==============================
    // Tauri アプリケーションの初期化
    // ==============================
    // 1. TauriState の初期化
    let state = TauriState {
        manager: manager.clone(),
        config_mgr: config_mgr.clone(),
        lmgw_client: lmgw_client.clone(),
        backend_guard: backend_guard.clone(),
        restart_guard_holder: Arc::new(Mutex::new(None)),
        hc: hc.async_hc.clone(),
        is_hotkey_active: Arc::new(AtomicBool::new(false)),
        is_shutting_down: is_shutting_down.clone(),
    };
    // restart_guard_holder は TauriState が .manage(state) で移動する前にクローンする
    let restart_guard_holder_for_exit = state.restart_guard_holder.clone();
    // 2. Tauri アプリケーションの構築
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            tauri_cmd::get_settings,
            tauri_cmd::save_settings,
            tauri_cmd::switch_stt_engine,
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
            tauri_cmd::prepare_restart,
            tauri_cmd::abort_restart,
            tauri_cmd::enable_hotkey_standby,
            tauri_cmd::disable_hotkey_standby,
            tauri_cmd::toggle_always_on_top,
            tauri_cmd::reset_application,
        ]);

    let (builder, sw_port) = configure_myproxy(builder, &config_mgr, &hc);

    // server_config を Tauri setup 内で使用するためにクローン
    let server_config_for_tauri = server_config;
    let config_mgr_for_ready = config_mgr.clone();

    let app = builder
        .setup(move |app| {
            // [Windows / WebView2]
            // WebView2 (Chromium) は起動オプションでのフラグ指定を公式にサポートしています。
            #[allow(unused_mut)]
            let mut browser_args = "--enable-features=msWebView2EnableDraggableRegions".to_string();

            // Windowの生成（プログラム制御）- 初期化
            let webview_url = if cfg!(debug_assertions) {
                // web/quasar.config.ts の 9000 のポート設定に合わせる
                match "http://localhost:9000".parse::<tauri::Url>() {
                    Ok(url) => tauri::WebviewUrl::External(url),
                    Err(_) => tauri::WebviewUrl::default(),
                }
            } else {
                tauri::WebviewUrl::default()
            };

            let window_builder = WebviewWindowBuilder::new(app, WINDOW_LABEL_MAIN, webview_url)
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
                .context("failed to create main window")?;

            // SDK ホスティングサーバー（Static Web Server）の起動 (GUI ロール)
            {
                let app_handle = app.handle().clone();
                let config_mgr_for_sw = config_mgr.clone();
                let hc_for_sw = hc.async_hc.clone();
                async_runtime::spawn(async move {
                    if let Err(e) = sw_server::run_sw_server(sw_port, app_handle, config_mgr_for_sw, hc_for_sw).await {
                        log::error!("CRITICAL: SW server failed to start: {}", e);
                    }
                });
            }

            // 1. WebView の下地を完全に透明にする (RGBA: 0,0,0,0)
            let _ = window.set_background_color(Some(Color(0, 0, 0, 0)));

            // 2. ウィンドウの影を無効にする
            let _ = window.set_shadow(false);

            // デッドロック回避 (Windows): WebView2ウィンドウ生成時、メッセージループの無いスレッドでブロックする問題がある。
            // 運命共同体（Fate-sharing）を防ぎ、UIハング時でも音声入力が機能するようSTTイベントループを独立した非同期タスクとして分離・先行起動する。
            spawn_stt_event_bridge(app.handle().clone(), manager.clone(), stt_rx);

            // バックエンドからのWebSocket（状態変更など）を受け取る独立タスク
            spawn_ws_event_bridge(
                app.handle().clone(),
                config_mgr.clone(),
                manager.clone(),
            );

            // バックグラウンドタスク: ホットキーハンドラ
            // enable_hotkey_standby コマンド内で実行されるため、ここでは起動しない

            Ok(())
        })
        .build(tauri::generate_context!())
        .context("error while building tauri application")?;

    // UI初期化の多重実行を防ぐフラグ
    let is_ui_initialized = Arc::new(AtomicBool::new(false));

    // シャットダウン信号の監視タスクを起動
    let handle_for_shutdown = app.handle().clone();
    let token_for_shutdown = shutdown_token.clone();
    tauri::async_runtime::spawn(async move {
        token_for_shutdown.cancelled().await;
        log::info!("Shutdown signal received. Exiting GUI (Graceful)...");
        handle_for_shutdown.exit(1);
    });

    let backend_guard_for_exit = backend_guard.clone();
    let is_shutting_down_for_exit = is_shutting_down.clone();
    app.run(move |handle, event| {
        match event {
            RunEvent::Ready => {
                // エンジン準備完了
                if !is_ui_initialized.load(Ordering::SeqCst) {
                    is_ui_initialized.store(true, Ordering::SeqCst);

                    log::info!("Tauri RunEvent::Ready triggered. Initializing UI...");
                    let handle_clone = handle.clone();
                    let config_mgr_clone = config_mgr_for_ready.clone();

                    // 非同期タスクとしてUIセットアップを実行（デッドロック回避）
                    tauri::async_runtime::spawn(async move {
                        if let Err(e) = setup_main_window(handle_clone.clone(), config_mgr_clone) {
                            log::error!("CRITICAL ERROR: Failed to setup UI: {}", e);
                            handle_clone.exit(1);
                        }
                    });
                }
            }
            RunEvent::ExitRequested { .. } | RunEvent::Exit => {
                log::info!("Tauri shutdown detected. Cleaning up backend processes...");
                // シャットダウンプロセスに入ったことをフラグに記録し、Watchdog の自爆を抑制する
                is_shutting_down_for_exit.store(true, Ordering::SeqCst);

                // 再起動ハンドオフ用に退避されたガードがあれば、kill せずに破棄する
                if let Some(detached_guard) = restart_guard_holder_for_exit.lock().take() {
                    log::info!(
                        "Detached backend guard (PID: {}) found. Dropping without kill (restart handover).",
                        detached_guard.pid
                    );
                    drop(detached_guard); // skip_kill_on_drop = true → 何も殺さない
                } else {
                    // 通常終了: ガードを Drop してバックエンドを道連れ死させる
                    let mut guard = backend_guard_for_exit.lock();
                    if let Some(bg) = guard.take() {
                        log::info!("Backend guard found. Dropping now.");
                        drop(bg); // ここで stdin クローズとポートクリーンアップが実行される
                    }
                }
            }
            _ => {}
        }
    });

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
                        // 最終補正レイヤー開始: オーバーレイに補正状態を通知する
                        {
                            let mut mgr = manager.lock();
                            if mgr.state == MgrAppState::Recording && !mgr.is_post_correcting {
                                mgr.is_post_correcting = true;
                            }
                        }
                        let _ = handle.emit(
                            TauriEvent::AppStatus.as_str(),
                            AppStatusPayload {
                                status: "correcting".to_string(),
                            },
                        );
                    }
                    SttEvent::PostCorrectionFinished => {
                        let mut mgr = manager.lock();
                        if mgr.is_post_correcting {
                            mgr.is_post_correcting = false;

                            if mgr.pending_flush {
                                mgr.pending_flush = false;
                                let flush_text = mgr.build_flush_text();

                                if !flush_text.is_empty() {
                                    // ペースト中は Manager ロックを解放する
                                    drop(mgr);
                                    let paste_ok = clipboard::save_paste_and_restore(&flush_text);
                                    let mut mgr = manager.lock();
                                    if paste_ok {
                                        mgr.stop_recording();
                                    }
                                    drop(mgr);
                                } else {
                                    mgr.stop_recording();
                                    drop(mgr);
                                }

                                audio::play_commit_sound();
                                let _ = handle.emit(TauriEvent::SttCommit.as_str(), ());
                                let _ = handle.emit(
                                    TauriEvent::AppState.as_str(),
                                    AppStatePayload {
                                        state: APP_STATE_IDLE.to_string(),
                                    },
                                );

                                #[cfg(target_os = "macos")]
                                hotkey_mac::set_recording_active(false);
                                #[cfg(windows)]
                                hotkey_win::set_recording_active(false);
                            } else {
                                let _ = handle.emit(
                                    TauriEvent::SttPartial.as_str(),
                                    SttPayload {
                                        text: mgr.current_text.clone(),
                                        seq: mgr.last_stt_seq,
                                    },
                                );
                                let _ = handle.emit(
                                    TauriEvent::AppStatus.as_str(),
                                    AppStatusPayload {
                                        status: String::new(),
                                    },
                                );
                            }
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

/// WSイベントブリッジを独立したタスクとして起動する。
/// バックエンド(RT)からのWebSocketイベント(言語変更等)を受信し、フロントエンドに通知する。
fn spawn_ws_event_bridge(
    handle: tauri::AppHandle,
    config_mgr: Arc<ConfigManager>,
    manager: Arc<Mutex<MycuteManager>>,
    // LMGW 移行後は WS イベントのライブアップデートに LmgwClient は不要なため、引数を削除した。
) {
    async_runtime::spawn(async move {
        // WS URLの組み立て
        let rt_port = config_mgr.settings.read().server.rt_port;
        let ws_url = format!(
            "{}://{}:{}{}",
            PROTOCOL_WS, IP_LOCALHOST, rt_port, PATH_MYCUTE_WS
        );
        let client_id = Uuid::new_v4().to_string();
        run_ws_client_loop(handle, manager, config_mgr, ws_url, client_id).await;
    });
}

async fn run_ws_client_loop(
    handle: tauri::AppHandle,
    manager: Arc<Mutex<MycuteManager>>,
    config_mgr: Arc<ConfigManager>,
    ws_url: String,
    client_id: String,
) {
    let mut retry_delay = Duration::from_millis(500);
    let max_retry_delay = Duration::from_millis(32000);
    let mut last_seq = 0;

    loop {
        let ws_stream = match connect_ws(&ws_url, &mut retry_delay, max_retry_delay).await {
            Some(stream) => stream,
            None => continue,
        };

        let (mut write, mut read) = ws_stream.split();

        if perform_ws_handshake(&mut write, &mut read, &client_id).await {
            retry_delay = Duration::from_secs(1); // 成功したらリセット
            run_ws_message_loop(
                &mut write,
                &mut read,
                &handle,
                &manager,
                &config_mgr,
                &mut last_seq,
            )
            .await;
        }

        log::warn!(
            "<Events> WS Connection lost or timed out. Retrying in {} seconds...",
            retry_delay.as_secs()
        );
        tokio::time::sleep(retry_delay).await;

        // 指数バックオフ
        retry_delay *= 2;
        if retry_delay > max_retry_delay {
            retry_delay = max_retry_delay;
        }
    }
}

async fn connect_ws(
    ws_url: &str,
    retry_delay: &mut Duration,
    max_retry_delay: Duration,
) -> Option<
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
> {
    log::info!("<Events> Connecting to WS at {}", ws_url);
    match tokio_tungstenite::connect_async(ws_url).await {
        Ok((stream, _)) => Some(stream),
        Err(e) => {
            log::error!("<Events> Failed to connect to WS: {}", e);
            tokio::time::sleep(*retry_delay).await;
            *retry_delay += Duration::from_millis(500);
            if *retry_delay > max_retry_delay {
                *retry_delay = max_retry_delay;
            }
            None
        }
    }
}

async fn perform_ws_handshake<W, R>(write: &mut W, read: &mut R, client_id: &str) -> bool
where
    W: futures_util::Sink<Message> + Unpin,
    R: futures_util::Stream<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
{
    log::info!("<Events> WS Connection opened. Waiting for Handshake Request...");
    match tokio::time::timeout(Duration::from_secs(5), read.next()).await {
        Ok(Some(Ok(Message::Text(text)))) => {
            if let Ok(WsServerMessage::HandshakeRequest { challenge }) = serde_json::from_str(&text)
            {
                let resp = WsClientMessage::HandshakeResponse {
                    challenge,
                    client_id: client_id.to_string(),
                    client_role: WsClientRole::CL,
                };
                if let Ok(resp_str) = serde_json::to_string(&resp) {
                    if write.send(Message::Text(resp_str.into())).await.is_ok() {
                        log::info!("<Events> WS Handshake Response sent.");
                        return true;
                    } else {
                        log::error!("<Events> Failed to send WS Handshake Response.");
                    }
                }
            } else {
                log::error!("<Events> Failed to parse Handshake Request: {}", text);
            }
        }
        _ => {
            log::error!("<Events> Handshake failed or timed out.");
        }
    }
    false
}

async fn run_ws_message_loop<W, R>(
    write: &mut W,
    read: &mut R,
    handle: &tauri::AppHandle,
    manager: &Arc<Mutex<MycuteManager>>,
    config_manager: &Arc<ConfigManager>,
    last_seq: &mut u64,
) where
    W: futures_util::Sink<Message> + Unpin,
    R: futures_util::Stream<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin,
{
    while let Ok(Some(Ok(msg))) = tokio::time::timeout(SSE_TIMEOUT_DURATION, read.next()).await {
        match msg {
            Message::Text(text) => {
                if let Ok(WsServerMessage::Event(internal_event)) =
                    serde_json::from_str::<WsServerMessage>(&text)
                {
                    if internal_event.seq > *last_seq || internal_event.seq == 0 {
                        *last_seq = internal_event.seq;
                        match internal_event.kind {
                            EventKind::Heartbeat => {
                                log::debug!("<Events> Received Heartbeat.");
                            }
                            EventKind::LocaleChanged(locale) => {
                                log::info!("<Events> Received LocaleChanged: {:?}", locale);
                                // CL自身のロケール設定を更新
                                {
                                    let mut mgr = manager.lock();
                                    mgr.set_locale(locale.clone());
                                }
                                let _ = handle.emit(
                                    TauriEvent::AppLocaleChanged.as_str(),
                                    AppLocaleChangedPayload { locale },
                                );
                            }
                            EventKind::SttEngineChanged(engine) => {
                                log::info!("<Events> Received SttEngineChanged: {:?}", engine);
                                // CL自身のSTTエンジン設定を更新
                                {
                                    let mut settings = config_manager.settings.write();
                                    settings.stt_engine = engine.clone();
                                }
                                let _ = handle.emit(
                                    TauriEvent::AppSttEngineChanged.as_str(),
                                    AppSttEngineChangedPayload { engine },
                                );
                            }
                            EventKind::OwnerStatusChanged(is_active) => {
                                log::info!("<Events> Received OwnerStatusChanged: {}", is_active);
                                let _ = handle.emit(
                                    TauriEvent::AppOwnerStatusChanged.as_str(),
                                    AppOwnerStatusChangedPayload { is_active },
                                );
                            }
                            EventKind::CaStatusChanged(ca_token) => {
                                log::info!("<Events> Received CaStatusChanged: {:?}", ca_token);
                                let _ = handle.emit(
                                    TauriEvent::AppCaStatusChanged.as_str(),
                                    AppCaStatusChangedPayload { ca_token },
                                );
                            }
                            EventKind::LicensesChanged(licenses) => {
                                log::info!(
                                    "<Events> Received LicensesChanged: {} licenses",
                                    licenses.len()
                                );
                                let _ = handle.emit(
                                    TauriEvent::AppLicensesChanged.as_str(),
                                    AppLicensesChangedPayload { licenses },
                                );
                            }
                            EventKind::LmgwProvidersChanged(providers) => {
                                log::info!("<Events> Received LmgwProvidersChanged");
                                let _ = handle.emit(
                                    TauriEvent::AppLmgwProvidersChanged.as_str(),
                                    AppLmgwProvidersChangedPayload { providers },
                                );
                            }
                            EventKind::SystemMessage(msg) => {
                                log::info!("<Events> Received SystemMessage: {}", msg);
                            }
                        }
                    } else {
                        log::debug!(
                            "<Events> Ignored old or duplicate event: seq {}",
                            internal_event.seq
                        );
                    }
                } else {
                    log::warn!("<Events> Failed to parse WS message: {}", text);
                }
            }
            Message::Close(_) => {
                log::info!("<Events> WS Stream closed normally.");
                break;
            }
            Message::Ping(data) => {
                let _ = write.send(Message::Pong(data)).await;
            }
            _ => {}
        }
    }
}

/// メインウィンドウの初期設定やSDKの注入などをまとめて行う。
fn setup_main_window(
    handle: tauri::AppHandle,
    config_mgr: Arc<ConfigManager>,
) -> Result<(), String> {
    // 1. メインウィンドウへの SDK 注入
    inject_sdk_to_main_window(&handle)?;

    // 2. メインウィンドウの表示位置最適化
    optimize_main_window_position(&handle, config_mgr)?;

    // 3. すべての準備（位置確定）が整ったら、表示する。
    if let Some(main_win) = handle.get_webview_window(WINDOW_LABEL_MAIN) {
        main_win
            .show()
            .map_err(|e| format!("Failed to show main window: {}", e))?;
    }

    Ok(())
}

/// MyProxy（カスタムプロトコルプロキシ）のセットアップを行う。
/// 外部サイトの iframe 表示制限を回避するための透過型プロキシを構成する。
fn configure_myproxy(
    builder: tauri::Builder<tauri::Wry>,
    config_mgr: &ConfigManager,
    hc: &SharedHttpClients,
) -> (tauri::Builder<tauri::Wry>, u16) {
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

    (builder, sw_port)
}

/// メインウィンドウに対して MYCUTE SDK を自動注入する。
fn inject_sdk_to_main_window(handle: &tauri::AppHandle) -> Result<(), String> {
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
        main_win
            .eval(&init_script)
            .map_err(|e| format!("Failed to inject SDK to main window: {}", e))?;
        let platform = TargetPlatform::current();
        main_win
            .eval(&format!(
                "window.__MYCUTE_PLATFORM__ = '{}';",
                platform.as_str()
            ))
            .map_err(|e| format!("Failed to inject platform info to main window: {}", e))?;
    } else {
        return Err("Main window not found for SDK injection".to_string());
    }

    Ok(())
}

/// メインウィンドウの表示位置をモニターの有効領域に合わせて最適化する。
fn optimize_main_window_position(
    handle: &tauri::AppHandle,
    config_mgr: Arc<ConfigManager>,
) -> Result<(), String> {
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
            main_win
                .set_position(LogicalPosition { x, y })
                .map_err(|e| format!("Failed to set position for main window: {}", e))?;
        } else {
            return Err("Main window not found for position optimization".to_string());
        }
        log::info!(
            "Window positioned at logical: ({}, {}), mode: {:?}",
            x,
            y,
            pos_cfg.mode
        );
    }

    Ok(())
}

// -----------------------------------------------------------------------
// Private Helpers
// -----------------------------------------------------------------------

/// GUI/Headless モード共通: プロキシサーバー用の SSL 証明書をロードします。
pub fn check_ssl_certificates(
    config_mgr: &ConfigManager,
) -> Result<Option<tokio_rustls::rustls::ServerConfig>> {
    log::info!("Checking SSL certificates...");
    match load_certs(config_mgr) {
        Ok(config) => {
            log::info!("SSL certificates loaded successfully.");
            Ok(Some(config))
        }
        Err(e) => {
            let err_msg = e.to_string();
            if err_msg.contains("not found") {
                log::warn!("SSL certificates not found yet. Proxy server will be skipped (Headless process should generate them).");
                Ok(None)
            } else {
                anyhow::bail!(
                    "CRITICAL: SSL certificates exist but failed to load: {}. Please check settings.json.",
                    err_msg
                );
            }
        }
    }
}

/// クライアント起動時のDB初期化（マイグレーション含む）と辞書 (replaces) の初期化を行います。
fn init_client_db(config_mgr: &ConfigManager) -> anyhow::Result<DbPools> {
    log::info!("Initializing DB connection and running auto-migrations...");
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("Failed to create runtime for client db init")?;

    rt.block_on(async {
        // DB Pools の初期化 (Settings からパス等を取得)
        let storage_settings = config_mgr.settings.read().storage.clone();
        let db_env = Env::from_settings(&storage_settings);
        let pools = get_db(&db_env, &LogLevel::Info)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to initialize DbPools for Client: {}", e))?;

        // Client はローカル SQLite (RW) を使用
        let conn = pools
            .get_rw()
            .map_err(|e| anyhow::anyhow!("Failed to get DB connection for Client: {}", e))?;

        // 0. Auto Migration の実行
        log::info!("Running Auto-Migration from CL Mode...");
        if let Err(e) = Migrator::up(conn, None).await {
            log::error!(
                "CRITICAL: Failed to apply auto-migrations in CL mode: {}",
                e
            );
            anyhow::bail!("Migration failed: {}", e);
        }

        // 1. デフォルトデータのシーディング (初回のみ)
        if let Err(e) = replaces_bl::seed_default_replaces(conn).await {
            log::error!("Failed to seed default replaces: {}", e);
        }

        // 2. アクティブな辞書を DB からメモリにリロード
        config_mgr
            .reload_replaces(conn)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to load replaces from DB: {}", e))?;

        Ok(pools)
    })
}

/// GUI モードにおいて、バックエンドサーバーの存在確認と自動起動（必要時）を管理します。
fn manage_backend_server(
    config_mgr: &Arc<ConfigManager>,
    _db_pools: &DbPools,
    home_dir: PathBuf,
    backend_guard: Arc<Mutex<Option<BackendProcessGuard>>>,
    shutdown_token: CancellationToken,
    is_shutting_down: Arc<AtomicBool>,
) -> Result<()> {
    // [Fate-Sharing] 子プロセス監視 (Watchdog)
    // サーバープロセス (backend_guard) が何らかの理由で終了した場合、クライアントも道連れ終了する。
    // これがないと、「サーバーが死んでるのにGUIだけ残る」ゾンビ状態になる。
    let watchdog_guard = backend_guard.clone();
    let config_mgr_for_watchdog = config_mgr.clone();
    let is_server_ready = Arc::new(AtomicBool::new(false));
    let is_server_ready_for_watchdog = is_server_ready.clone();
    let token_for_watchdog = shutdown_token.clone();
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_secs(3));

            let (pid_opt, rt_port) = {
                let guard = watchdog_guard.lock();
                let p = guard.as_ref().map(|g| g.pid);
                let port = config_mgr_for_watchdog.settings.read().server.rt_port;
                (p, port)
            };

            // 1. プロセスレベルの監視 (PIDがある場合)
            if let Some(pid) = pid_opt {
                let is_alive = {
                    #[cfg(unix)]
                    {
                        let res = unsafe { libc::kill(pid as i32, 0) };
                        res == 0
                            || std::io::Error::last_os_error().raw_os_error().unwrap_or(0)
                                == libc::EPERM
                    }
                    #[cfg(windows)]
                    {
                        let output = Command::new("tasklist")
                            .args(&["/FI", &format!("PID eq {}", pid), "/NH"])
                            .hide_window_if_windows()
                            .output();
                        match output {
                            Ok(o) => String::from_utf8_lossy(&o.stdout).contains(&pid.to_string()),
                            Err(_) => false,
                        }
                    }
                    #[cfg(not(any(unix, windows)))]
                    {
                        true
                    }
                };

                if !is_alive {
                    if is_shutting_down.load(Ordering::Relaxed) {
                        log::info!("Backend process (PID: {}) died during planned shutdown. Watchdog ignoring.", pid);
                        return;
                    }
                    log::error!(
                        "CRITICAL: Backend process (PID: {}) died. Client exiting (Fate-Sharing).",
                        pid
                    );
                    token_for_watchdog.cancel();
                    return;
                }
            }

            // 2. ネットワークレベルの監視 (疎通確認)
            // 初回のハンドシェイクが完了するまでは、ポートの疎通確認を行わない（起動のラグによる誤検知を防ぐ）
            if is_server_ready_for_watchdog.load(Ordering::Relaxed) {
                let server_addr = format!("{}:{}", IP_LOCALHOST, rt_port);
                let parsed_addr = match server_addr.parse::<SocketAddr>() {
                    Ok(a) => a,
                    Err(e) => {
                        log::error!("Failed to parse server address {}: {}", server_addr, e);
                        token_for_watchdog.cancel();
                        return;
                    }
                };

                if TcpStream::connect_timeout(&parsed_addr, Duration::from_secs(1)).is_err() {
                    if is_shutting_down.load(Ordering::Relaxed) {
                        log::info!("Backend server at {} unreachable during planned shutdown. Watchdog ignoring.", server_addr);
                        return;
                    }
                    log::error!("CRITICAL: Backend server at {} is unreachable. Client exiting (Fate-Sharing).", server_addr);
                    token_for_watchdog.cancel();
                    return;
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
        // 既存サーバーの PID を特定して監視対象に加える
        if let Some(pid) = proc_utils::find_pids_by_port(rt_port).first().cloned() {
            log::info!("Monitoring existing backend server (PID: {})", pid);

            // ============================================================
            // [Fate-Sharing Fail-safe] 道連れ清掃ポートの登録
            // ============================================================
            // RT 終了時に「確実に」ポートを解放させたいプロセスがある場合、
            // そのポート番号を以下のリストに追加してください。
            // これにより、CL 終了時に RT プロセスの生存確認とポートの強制解放が行われます。
            // ============================================================
            *backend_guard.lock() = Some(BackendProcessGuard::new(
                pid,
                None,
                None,
                config_mgr.clone(),
            ));

            // 再起動ハンドオフ後、新 CL が既存 RT を adopt したことを通知する。
            // RT 側の restart_mode を解除し、新 CL の PID で Fate-Sharing 監視を再開させる。
            let current_pid = std::process::id();
            let complete_url = format!(
                "http://{IP_LOCALHOST}:{rt_port}/api/v1/internal/complete-restart"
            );
            match reqwest::blocking::Client::new()
                .post(&complete_url)
                .json(&serde_json::json!({ "new_cl_pid": current_pid }))
                .send()
            {
                Ok(resp) if resp.status().is_success() => {
                    log::info!(
                        "Restart handover: new CL (PID: {}) adopted by RT successfully.",
                        current_pid
                    );
                }
                Ok(resp) => {
                    log::warn!(
                        "Restart handover: RT returned non-success for complete-restart: {}",
                        resp.status()
                    );
                }
                Err(e) => {
                    log::warn!(
                        "Restart handover: Failed to notify complete-restart to RT: {}",
                        e
                    );
                }
            }
        }
    } else {
        log::info!(
            "Backend server not found at {}. Initiating auto-elevation spawn...",
            server_addr
        );

        // 昇格された特権サーバーを起動
        // 引数: "cl" (GUIモードとして自分自身を再起動し、内生バックエンドを特権モードで立ち上げる)
        // 追加: "--parent-pid", "<PID>" (運命共同体モード: 親プロセス監視用)

        let current_pid = std::process::id().to_string();
        let home_path_str = home_dir.to_string_lossy().to_string();

        let mut args = vec![
            "--parent-pid",
            Box::leak(current_pid.into_boxed_str()),
            "--skip-rt-migration",
            "--home",
            Box::leak(home_path_str.into_boxed_str()),
        ];

        // どのような権限で起動すべきか判定
        let needs_elevation = config_mgr.needs_elevation_for_cert();
        let now_dt = now();
        let timestamp = now_dt.format("%Y%m%d_%H%M%S").to_string();
        let server_log_filename = format!("{}_server_{}.log", APP_NAME, timestamp);

        let log_file_path = if cfg!(windows) {
            let log_dir = get_log_dir(&home_dir);
            log_dir.join(&server_log_filename)
        } else {
            // macOS/Linux は 一時ディレクトリを利用（BackendProcessGuard で削除される）
            std::env::temp_dir().join(&server_log_filename)
        };

        // macOS/Linux も含め、子プロセスへの明示的な出力先指定を行う
        let log_path_str = log_file_path.to_string_lossy().to_string();
        args.push("--output");
        args.push(Box::leak(log_path_str.into_boxed_str())); // 長期間生存する引数として保持

        let spawn_res = if needs_elevation {
            log::info!("Certificate needs registration or OS trust. Initiating elevated spawn...");
            auth::spawn_elevated_server(&args, &log_file_path)
        } else {
            log::info!("Certificate is valid and trusted. Starting with normal privilege.");
            auth::spawn_normal_server(&args, &log_file_path)
        };

        match spawn_res {
            Ok((pid, stdin)) => {
                log::info!("Backend server process spawned. PID: {}", pid);
                if pid > 0 {
                    let log_to_guard = if cfg!(windows) {
                        Some(log_file_path.clone())
                    } else {
                        None
                    };

                    *backend_guard.lock() = Some(BackendProcessGuard::new(
                        pid,
                        log_to_guard,
                        stdin,
                        config_mgr.clone(),
                    ));

                    // macOS/Windows 両方でログパイプ処理を有効化
                    {
                        let log_target = log_file_path.clone();
                        thread::spawn(move || {
                            let mut position: u64 = 0;
                            // 監視開始
                            loop {
                                if let Ok(mut file) = File::open(&log_target) {
                                    if let Ok(_) = file.seek(SeekFrom::Start(position)) {
                                        let reader = BufReader::new(file);
                                        for line in reader.lines() {
                                            if let Ok(l) = line {
                                                println!("[Backend] {}", l);
                                                position += (l.len() as u64) + 1u64;
                                            }
                                        }
                                    }
                                }
                                thread::sleep(Duration::from_millis(200));
                            }
                        });
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to spawn backend server: {}", e);
                anyhow::bail!("Failed to spawn backend server: {}. Please ensure you have sufficient privileges.", e);
            }
        }
    }

    // 起動または接続後の最終確認（心中ロジックを含む同期）
    let mut retry = 0;
    while !check_health() && retry < 60 {
        // 最大30秒待機 (バイナリ展開などを考慮)
        log::info!("Waiting for backend server to be ready ({})...", retry);
        thread::sleep(Duration::from_millis(500));
        retry += 1;
    }

    if !check_health() {
        log::error!(
            "CRITICAL: Backend server failed to start or bind port {} within timeout.",
            rt_port
        );
        return Err(anyhow::anyhow!(
            "Backend server failed to start within timeout."
        ));
    }

    // ウォッチドッグにサーバー起動完了を通知し、厳密なネットワーク監視を開始させる
    is_server_ready.store(true, Ordering::Relaxed);

    Ok(())
}
