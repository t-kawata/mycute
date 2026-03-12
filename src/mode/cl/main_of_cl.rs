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
    AppErrorPayload, AppLocaleChangedPayload, AppStatusPayload, AppSttEngineChangedPayload,
    EventKind, SttEvent, SttPayload, SttUpdatePayload, TargetPlatform, TauriEvent, WsClientMessage,
    WsClientRole, WsServerMessage,
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
    async_runtime::{self},
    window::Color,
    Emitter, LogicalPosition, Manager, RunEvent, WebviewWindowBuilder,
};
use tokio::time::sleep;

#[cfg(windows)]
use std::fs;
#[cfg(windows)]
use std::fs::File;

use crate::config::settings::{AppRole, Env};
#[cfg(windows)]
use crate::constants::APP_NAME;
use crate::constants::{
    IP_LOCALHOST, LOCK_FILE_APP, LOCK_FILE_SERVER, MYCUTE_SDK_FILENAME, POST_CORRECTION_DECORATION,
    SSE_TIMEOUT_DURATION, WINDOW_HEIGHT, WINDOW_WIDTH,
};
use crate::enums::Mode;
use crate::migration::{Migrator, MigratorTrait};
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
use futures_util::{SinkExt, StreamExt};
use reqwest::Client as ReqwestClient;
#[cfg(windows)]
use std::process::Command;
use std::process::{self};
use tokio::runtime::Runtime;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;
use uuid::Uuid;

#[derive(Debug, Parser, Serialize)]
#[command(override_usage = "mycute cl [OPTIONS]", ignore_errors = true)]
pub struct CLFlgs {
    #[command(flatten)]
    pub common: CommonFlgs,

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

    /// バックエンドサーバー起動時のDBオートマイグレーションをスキップする
    #[arg(
        long = "skip-rt-migration",
        help = "Skip DB auto-migration when starting the RT backend server"
    )]
    pub skip_rt_migration: bool,
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
    pub is_overlay_visible: Arc<AtomicBool>,
}
use crate::utils::mod_dl;

pub fn main_of_cl(flgs: CLFlgs, hc: SharedHttpClients) -> Result<()> {
    // MYCUTE_HOME を絶対パスで確定させる
    let home_dir = get_mycute_home();
    let config_mgr = Arc::new(ConfigManager::new());

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

    // ==============================
    // rustls 暗号プロバイダーの初期化 (ring をデフォルトに設定)
    // ==============================
    // 依存関係（reqwest, sea-orm 等）に複数の暗号プロバイダー（ring, aws-lc-rs）が
    // 含まれている場合、rustls 0.23 以降では明示的な初期化が必須となる。
    // 本プロジェクトでは、プロキシサーバーや各クライアントで ring を標準として採用する。
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    // ==============================
    // SSL証明書のセットアップ
    // ==============================
    // 重要: 証明書のセットアップをここで行う (Direct HTTPS)
    // バックエンドサーバー任せにすると、macOSの「信頼ダイアログ」が表示されない(Headless制限)ため、
    // GUIプロセスであるここで実行し、ユーザーに明示的に承認(パスワード入力)させる。
    if let Err(e) = create_certs_if_missing(&config_mgr) {
        log::error!(
            "Failed to setup SSL certificates in Client: {}. Proxy might fail.",
            e
        );
        // ここで失敗しても、サーバー側で再試行される可能性にかけて続行するが、ダイアログは出ないかもしれない。
    }

    // ==============================
    // SSL証明書の確認
    // ==============================
    let server_config = check_ssl_certificates(&config_mgr)?;

    // ==============================
    // データベースの初期化とマイグレーション
    // ==============================
    init_client_db(&config_mgr)?;

    // ==============================
    // 統計データのパスを設定
    // ==============================
    UsageStats::set_base_path(home_dir.clone());

    // バックエンドプロセスをクリーンアップするためのガード
    let backend_guard: Arc<Mutex<Option<BackendProcessGuard>>> = Arc::new(Mutex::new(None));

    // ==============================
    // サーバーマネージャーロジック (監視・起動)
    // ==============================
    manage_backend_server(
        role,
        &config_mgr,
        &flgs,
        home_dir.clone(),
        backend_guard.clone(),
    )?;

    // ==============================
    // サーバーの起動 (Headless ロール時の内部起動または監視)
    // ==============================
    run_headless_server(role, config_mgr.clone(), &flgs, hc.clone())?;

    let (stt_tx, stt_rx) = mpsc::channel(100);

    let settings = config_mgr.settings.read();
    let stt_engine = settings.stt_engine.clone();
    let current_locale = settings.locale.clone();
    let stt_settings = settings.stt.clone();
    let llm_endpoints = settings.llms.clone();
    let hotkey_config = settings.hotkeys.clone();
    let replaces = config_mgr.replaces.clone();

    // 手動でのソートは SpeechRecognizer 内部で処理される
    // SpeechRecognizer::new は IndexMap を直接受け取るように変更された

    drop(settings);

    // LLMプール初期化
    let llm_pool = Arc::new(LlmPool::new(&llm_endpoints));

    // SpeechRecognizer の初期化
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
        llm_pool: llm_pool.clone(),
        backend_guard: backend_guard.clone(),
        hc: hc.async_hc.clone(),
        is_hotkey_active: Arc::new(AtomicBool::new(false)),
        is_overlay_visible: Arc::new(AtomicBool::new(WINDOW_INITIAL_VISIBLE_OVERLAY)),
    };
    // 2. Tauri アプリケーションの構築
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_process::init())
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
            tauri_cmd::enable_hotkey_standby,
            tauri_cmd::disable_hotkey_standby,
            tauri_cmd::toggle_always_on_top,
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

            // デッドロック回避 (Windows): WebView2ウィンドウ生成時、メッセージループの無いスレッドでブロックする問題がある。
            // 運命共同体（Fate-sharing）を防ぎ、UIハング時でも音声入力が機能するようSTTイベントループを独立した非同期タスクとして分離・先行起動する。
            spawn_stt_event_bridge(app.handle().clone(), manager.clone(), stt_rx);

            // バックエンドからのWebSocket（状態変更など）を受け取る独立タスク
            spawn_ws_event_bridge(app.handle().clone(), config_mgr.clone(), manager.clone());

            // バックグラウンドタスク: ホットキーハンドラ
            // enable_hotkey_standby コマンド内で実行されるため、ここでは起動しない

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application");

    // ウィンドウ生成の多重実行を防ぐスレッドセーフなフラグ
    let is_extra_windows_initialized = Arc::new(AtomicBool::new(false));

    app.run(move |handle, event| {
        if let RunEvent::Ready = event {
            // イベントループが開始され、エンジンが完全に準備完了した最初のタイミング
            if !is_extra_windows_initialized.load(std::sync::atomic::Ordering::SeqCst) {
                is_extra_windows_initialized.store(true, std::sync::atomic::Ordering::SeqCst);

                log::info!(
                    "Tauri RunEvent::Ready triggered. Spawning extra UI initialization task..."
                );
                let handle_clone = handle.clone();
                let config_mgr_clone = config_mgr_for_ready.clone();

                // 非同期タスクとしてウィンドウ生成を実行する。
                // これによりメインスレッドが即座にメッセージループに復帰でき、Windows WebView2 でのデッドロックが解消される。
                tauri::async_runtime::spawn(async move {
                    // 【診断】メインウィンドウの初期化と競合を避けるため、少し待機してから生成を開始する。
                    log::info!(
                        "<Diagnostic> Waiting 2 seconds before initializing extra UI windows..."
                    );
                    sleep(Duration::from_secs(2)).await;

                    if let Err(e) = setup_main_window(handle_clone, config_mgr_clone) {
                        log::error!("CRITICAL ERROR: Failed to setup extra UI windows: {}", e);
                        // エラーが発生した場合は、アプリ全体を強制終了させて問題を顕在化させる
                        std::process::exit(1);
                    }
                });
            }
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

/// WSイベントブリッジを独立したタスクとして起動する。
/// バックエンド(RT)からのWebSocketイベント(言語変更等)を受信し、フロントエンドに通知する。
fn spawn_ws_event_bridge(
    handle: tauri::AppHandle,
    config_mgr: Arc<ConfigManager>,
    manager: Arc<Mutex<MycuteManager>>,
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
    let mut retry_delay = Duration::from_secs(1);
    let max_retry_delay = Duration::from_secs(32);
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
            *retry_delay *= 2;
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
fn check_ssl_certificates(
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
fn init_client_db(config_mgr: &ConfigManager) -> Result<()> {
    log::info!("Initializing DB connection and running auto-migrations...");
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to create runtime for client db init");

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

        Ok(())
    })
}

/// GUI モードにおいて、バックエンドサーバーの存在確認と自動起動（必要時）を管理します。
fn manage_backend_server(
    role: AppRole,
    config_mgr: &ConfigManager,
    flgs: &CLFlgs,
    home_dir: std::path::PathBuf,
    backend_guard: Arc<Mutex<Option<BackendProcessGuard>>>,
) -> Result<()> {
    if role != AppRole::GUI {
        return Ok(());
    }

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
                        // Unix: kill(pid, 0) で生存確認。0シグナルはプロセスチェックのみを行う。
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
                    log::error!(
                        "CRITICAL: Backend server (PID: {}) died unexpectedy. Client shutting down (Fate-Sharing).",
                        pid
                    );
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
        let home_dir_str = home_dir.to_string_lossy().to_string();

        let mut args = vec![
            cl_mode_str,
            "-r",
            server_role_str,
            "--parent-pid",
            &current_pid,
            "--home",
            &home_dir_str,
            "--skip-rt-migration",
        ];

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
                }
                Err(e) => {
                    log::error!("Failed to spawn elevated server: {}", e);
                    anyhow::bail!("Failed to spawn elevated server: {}. Please ensure you have administrator privileges.", e);
                }
            }
        }
    }

    Ok(())
}

/// Headless ロール（サーバーモード）の起動および親プロセス監視を管理します。
fn run_headless_server(
    role: AppRole,
    config_mgr: Arc<ConfigManager>,
    flgs: &CLFlgs,
    hc: SharedHttpClients,
) -> Result<()> {
    // サーバーの起動条件チェック: Headless ロールである必要がある
    // GUIモードの場合は spawn_elevated_server で外部プロセスとして起動しているので、
    // ここで内部的に起動してはいけない（ポート競合や二重起動の防止）。
    if role != AppRole::Headless {
        return Ok(());
    }

    // [Fate-Sharing] 親プロセス監視 (Watchdog)
    // GUI プロセスから --parent-pid で起動された場合、親が消失したことを検知して
    // サーバー自身も自律的に終了する。これにより、親がいなくなった後にサーバーが
    // 居座り続ける（ゾンビサーバー化する）のを防ぐ。
    if let Some(ppid) = flgs.parent_pid {
        log::info!("Starting Watchdog for Parent PID: {}", ppid);
        thread::spawn(move || {
            loop {
                // 3秒に一回の頻度で親の生存を確認
                thread::sleep(Duration::from_secs(3));

                let is_alive = {
                    #[cfg(unix)]
                    {
                        // Unix: kill(pid, 0) で生存確認。0シグナルはプロセスチェックのみを行う。
                        // 成功(0)すればプロセスは存在し、権限エラー(EPERM)でもプロセス自体は存在する。
                        let res = unsafe { libc::kill(ppid as i32, 0) };
                        if res == 0 {
                            true
                        } else {
                            // errno チェック: 自分より特権の高いプロセス(Root等)の確認は EPERM になる可能性があるが、
                            // 今回は GUI(User) -> Headless(Root) なので Root から User への kill(0) は成功する。
                            let err = io::Error::last_os_error().raw_os_error().unwrap_or(0);
                            if err == libc::EPERM {
                                true // 生きてるが権限の問題でシグナルを送信できない
                            } else {
                                false // ESRCH (No such process)
                            }
                        }
                    }

                    #[cfg(windows)]
                    {
                        // Windows: OpenProcess 等を使用するのが流儀だが、
                        // 依存関係を増やさないために標準の tasklist コマンドを使用する。
                        // 数秒に一回の頻度であれば、オーバーヘッドは許容範囲内。
                        let output = Command::new("tasklist")
                            .args(&["/FI", &format!("PID eq {}", ppid), "/NH"])
                            .output();

                        match output {
                            Ok(o) => {
                                let s = String::from_utf8_lossy(&o.stdout);
                                // 出力に PID が含まれていれば、まだプロセスリストに載っている（＝生きている）
                                s.contains(&ppid.to_string())
                            }
                            Err(_) => false, // コマンド実行失敗時は安全側に倒して「死んだ」とみなす可能性がある
                        }
                    }

                    #[cfg(not(any(unix, windows)))]
                    {
                        true // 未対応OSでは監視をスキップ
                    }
                };

                if !is_alive {
                    log::warn!(
                        "Parent process (PID: {}) is gone. Shutting down server (Fate-Sharing).",
                        ppid
                    );
                    // 運命共同体として正常終了
                    process::exit(0);
                }
            }
        });
    }

    // RT（リアルタイム）サーバーを別スレッドで起動
    // Tauri のメインループ（GUIイベントループ）をブロックしないように、
    // 専用の Runtime を持つ独立したスレッドで実行する。
    let config_mgr_for_server = config_mgr.clone();
    let env_for_server = Env::from_settings(&config_mgr.settings.read().storage);
    let owner_pass_clone = flgs.owner_passphrase.clone();
    let hc_for_server = hc.async_hc.clone();
    // CLモード（GUI/Headless）から直接スレッド起動される場合は、必ず init_client_db を通過済みのため
    // コンテキストとしてマイグレーションスキップを強制する。
    let skip_rt_migration = true;

    log::info!("Spawning main_of_rt in a separate thread...");
    thread::spawn(move || {
        let rt = Runtime::new().expect("Failed to create runtime for server");
        rt.block_on(async move {
            main_of_rt(
                config_mgr_for_server,
                env_for_server,
                owner_pass_clone,
                hc_for_server,
                skip_rt_migration,
            )
            .await;
        });
    });

    // サーバー単独（Headless Role）モードの場合、ここでプロセスの終了を阻止する
    // GUIモードと異なり、この後 Tauri のイベントループに入らないため、
    // サーバースレッドが動作し続けられるようメインスレッドを待機状態にする。
    if role == AppRole::Headless {
        println!("Mycute Headless Server is running. Press Ctrl+C to stop.");
        log::info!("Headless server main loop entered.");
        loop {
            // OSのシグナル待ちが理想だが、ここではシンプルに長時間スリープのループで待機する
            thread::sleep(Duration::from_secs(3600));
        }
    }

    Ok(())
}
