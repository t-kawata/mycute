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

fn main() {
    let args: Vec<String> = env::args().collect();

    #[cfg(target_os = "macos")]
    {
        use mycute::utils::macos_permissions::{handle_macos_prelaunch_checks, PrelaunchAction};
        // バージョン更新による権限リセット要求をチェック
        if let Ok(action) = handle_macos_prelaunch_checks() {
            if matches!(action, PrelaunchAction::Restart) {
                // 再起動（Spawn）が成功した場合は、このプロセス（親）は終了する
                std::process::exit(0);
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
        std::process::exit(0);
    }

    if m == "-v" || m == "--version" {
        println!("{}", config::VERSION);
        std::process::exit(0);
    }

    if !Mode::is_valid(&m) {
        eprintln!("Invalid mode: {}", m);
        eprintln!("{}", Mode::help());
        std::process::exit(1);
    }

    let mode = Mode::from_str(&m).expect("Invalid mode");

    let mode_args =
        std::iter::once(args[0].clone()).chain(args[args.len().min(2)..].iter().cloned());

    let result = match mode {
        Mode::AM => {
            let (flgs, _hc) = AppInit::<am::AMFlgs>::from_args(mode_args)
                .with_logger()
                .init()
                .expect("Failed to init am mode.");
            am::main_of_am(flgs)
        }
        Mode::CL => {
            let (flgs, hc) = AppInit::<cl::CLFlgs>::from_args(mode_args)
                .with_logger()
                .init()
                .expect("Failed to init cl mode.");
            cl::main_of_cl(flgs, hc)
        }
        Mode::OG => {
            let (flgs, _hc) = AppInit::<og::OGFlgs>::from_args(mode_args)
                .with_logger()
                .init()
                .expect("Failed to init og mode.");
            og::main_of_og(flgs)
        }
    };

    if let Err(e) = result {
        log::error!("Execution failed: {:?}", e);
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
