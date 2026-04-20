#![cfg_attr(windows, windows_subsystem = "windows")]
//! mycute - Real-time Japanese STT Input Tool
//!
//! - cl: Client/Server (Tauri/Axum)
//! - am: Auto Migration

use mycute::config;
use mycute::enums::mode::MODE_CL;
use mycute::enums::Mode;
use mycute::mode::am;
use mycute::mode::cl;
use mycute::mode::og;
use mycute::utils::init::AppInit;
use std::env;
#[cfg(target_os = "macos")]
use mycute::utils::macos_permissions::{handle_macos_prelaunch_checks, PrelaunchAction};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();

    #[cfg(target_os = "macos")]
    {
        // バージョン更新による権限リセット要求をチェック
        match handle_macos_prelaunch_checks() {
            Ok(PrelaunchAction::Restart) => {
                // 再起動（Spawn）が成功した場合は、このプロセス（親）は終了する
                return Ok(());
            }
            Ok(PrelaunchAction::Continue) => {}
            Err(e) => {
                // ロガー初期化前のため、標準エラー出力に詳細を出す
                eprintln!("<MacOSPermissions> Pre-launch check error: {:?}", e);
            }
        }
    }

    let m: &str = if args.len() < 2 {
        MODE_CL
    } else {
        args[1].as_str()
    };

    if m == "-h" || m == "--help" {
        println!("{}", Mode::help());
        return Ok(());
    }

    if m == "-v" || m == "--version" {
        println!("{}", config::VERSION);
        return Ok(());
    }

    if !Mode::is_valid(&m) {
        eprintln!("Invalid mode: {}", m);
        eprintln!("{}", Mode::help());
        anyhow::bail!("Invalid mode: {}", m);
    }

    let mode = Mode::from_str(&m).ok_or_else(|| anyhow::anyhow!("Invalid mode: {}", m))?;

    let mode_args =
        std::iter::once(args[0].clone()).chain(args[args.len().min(2)..].iter().cloned());

    let result = match mode {
        Mode::AM => {
            let (flgs, _hc) = AppInit::<am::AMFlgs>::from_args(mode_args)
                .with_logger()
                .init()
                .map_err(|e| anyhow::anyhow!("Failed to init am mode: {}", e))?;
            am::main_of_am(flgs)
        }
        Mode::CL => {
            let (flgs, hc) = AppInit::<cl::CLFlgs>::from_args(mode_args)
                .with_logger()
                .init()
                .map_err(|e| anyhow::anyhow!("Failed to init cl mode: {}", e))?;
            cl::main_of_cl(flgs, hc)
        }
        Mode::OG => {
            let (flgs, _hc) = AppInit::<og::OGFlgs>::from_args(mode_args)
                .with_logger()
                .init()
                .map_err(|e| anyhow::anyhow!("Failed to init og mode: {}", e))?;
            og::main_of_og(flgs)
        }
    };

    if let Err(e) = result {
        log::error!("Execution failed: {:?}", e);
        eprintln!("Error: {}", e);
        anyhow::bail!("Execution failed: {}", e);
    }

    Ok(())
}
