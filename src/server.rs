#![cfg_attr(windows, windows_subsystem = "windows")]
//! mycute-server - Standalone Server Binary
//!
//! GUI（Tauri）に依存しない独立したサーバーバイナリのエントリポイント。
//! `make server` でビルドされ、`dist/mac(win)/` に配置される。

use anyhow::Result;
use clap::Parser;
use mycute::config;
use mycute::config::settings::Env; // Env を追加
use mycute::constants::LOCK_FILE_SERVER;
use mycute::migration::{Migrator, MigratorTrait};
use mycute::mode::cl::check_ssl_certificates;
use mycute::mode::rt::main_of_rt::{main_of_rt, RTFlgs};
use mycute::mycute_settings::ConfigManager;
use mycute::utils::db::get_db;
use mycute::utils::init::{AppInit, HasCommonFlgs};
use mycute::utils::singleton;
use std::env;
#[cfg(unix)]
use std::io;
use std::iter;
use std::process;
#[cfg(target_os = "windows")]
use std::process::Command;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tokio::runtime::Runtime;

// ============================================================
// Native Library Embedding (Self-Extraction)
// ============================================================

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        let m = &args[1];
        if m == "-h" || m == "--help" {
            // RT モード特化の Clap ヘルプを表示
            <RTFlgs as Parser>::parse_from(
                iter::once(&args[0]).chain(iter::once(&"---help".to_string())),
            );
            process::exit(0);
        }
        if m == "-v" || m == "--version" {
            println!("{}", config::VERSION);
            process::exit(0);
        }
    }

    let head = args[0].clone();
    let tail = &args[1..];
    let args_chain = iter::once(head.clone()).chain(tail.iter().cloned());

    // 1. サーバー用ロックの取得 (mycute.lock)
    if let Err(e) = singleton::acquire_lock(LOCK_FILE_SERVER) {
        log::error!("Server lock failed: {}", e);
        eprintln!("Error: {}", e);
        process::exit(1);
    }

    // 2. SSL等の基本初期化 (最優先で rustls プロバイダを固定)
    let _ = rustls::crypto::ring::default_provider().install_default();

    // 3. AppInit / Logger 初期化
    let (flgs, hc) = AppInit::<RTFlgs>::from_args(args_chain)
        .with_logger()
        .init()
        .expect("Failed to init server mode.");

    // 4. SSL証明書の確認
    let config_mgr_bootstrap = Arc::new(ConfigManager::new_bootstrap());
    let _server_config = check_ssl_certificates(&config_mgr_bootstrap)?;

    // Runtimeの初期化 (DB初期化とマイグレーションのために早めに作成)
    let rt = Runtime::new().expect("Failed to create runtime for server");

    // DB 初期化
    let db_pools = rt.block_on(async {
        let env = {
            let s = config_mgr_bootstrap.settings.read();
            Env::from_settings(&s.storage)
        };
        let common = flgs.common_flgs();
        let pools = get_db(&env, &common.log_level)
            .await
            .expect("Failed to create DB pools");

        // サーバー起動時はマイグレーションを実行
        Migrator::up(&pools.rw, None)
            .await
            .expect("Failed to run migrations");

        pools
    });

    // 正真な ConfigManager インスタンスの作成
    let config_mgr = Arc::new(ConfigManager::new_live(db_pools));

    // 親プロセス監視（Fate-Sharing）
    if let Some(ppid) = flgs.parent_pid {
        log::info!("Starting Watchdog for Parent PID: {}", ppid);
        thread::spawn(move || loop {
            thread::sleep(Duration::from_secs(3));
            let is_alive = {
                #[cfg(unix)]
                {
                    let res = unsafe { libc::kill(ppid as i32, 0) };
                    res == 0
                        || io::Error::last_os_error().raw_os_error().unwrap_or(0) == libc::EPERM
                }
                #[cfg(windows)]
                {
                    let output = Command::new("tasklist")
                        .args(&["/FI", &format!("PID eq {}", ppid), "/NH"])
                        .output();
                    match output {
                        Ok(o) => String::from_utf8_lossy(&o.stdout).contains(&ppid.to_string()),
                        Err(_) => false,
                    }
                }
                #[cfg(not(any(unix, windows)))]
                {
                    true
                }
            };
            if !is_alive {
                log::warn!(
                    "Parent process (PID: {}) is gone. Shutting down server (Fate-Sharing).",
                    ppid
                );
                process::exit(0);
            }
        });
    }

    log::info!("Starting main_of_rt...");
    let rt = Runtime::new().expect("Failed to create runtime for server");
    rt.block_on(async move {
        main_of_rt(config_mgr, hc.async_hc.clone(), flgs).await;
    });

    println!("Mycute Server is running. Press Ctrl+C to stop.");
    loop {
        thread::sleep(Duration::from_secs(3600));
    }
}
