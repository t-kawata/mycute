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
#[cfg(target_os = "macos")]
use mycute::utils::macos_permissions::{handle_macos_prelaunch_checks, PrelaunchAction};
use std::env;

/// Windows で親プロセスのコンソールに接続し、log 出力をターミナルに表示できるようにする。
///
/// `windows_subsystem = "windows"` により GUI アプリとしてビルドされたプロセスは
/// 標準でコンソールを持たず、`stdout` への出力が失われる。
/// `AttachConsole` を呼び出すことで親プロセス（ターミナル）のコンソールに接続し、
/// `SetStdHandle` で stdout/stderr のハンドルを設定する。
#[cfg(windows)]
mod win_console {
    #[link(name = "kernel32")]
    extern "system" {
        fn AttachConsole(dwProcessId: u32) -> i32;
        fn SetStdHandle(nStdHandle: u32, hHandle: *mut std::ffi::c_void) -> i32;
        fn CreateFileA(
            lpFileName: *const u8,
            dwDesiredAccess: u32,
            dwShareMode: u32,
            lpSecurityAttributes: *mut std::ffi::c_void,
            dwCreationDisposition: u32,
            dwFlagsAndAttributes: u32,
            hTemplateFile: *mut std::ffi::c_void,
        ) -> *mut std::ffi::c_void;
    }

    const ATTACH_PARENT_PROCESS: u32 = 0xFFFFFFFF;
    const STD_OUTPUT_HANDLE: u32 = 0xFFFFFFF5; // -11
    const STD_ERROR_HANDLE: u32 = 0xFFFFFFF4; // -12
    const GENERIC_WRITE: u32 = 0x40000000;
    const FILE_SHARE_WRITE: u32 = 0x00000002;
    const OPEN_EXISTING: u32 = 3;

    pub fn attach_to_parent_console() {
        unsafe {
            // 親プロセス（ターミナル／cargo tauri dev）のコンソールに接続を試みる
            if AttachConsole(ATTACH_PARENT_PROCESS) == 0 {
                // 失敗＝ターミナルから起動されていない（Explorer からの直接起動等）
                // この場合は何もせず、従来通りコンソール無しで動作する
                return;
            }
            // コンソールの出力バッファへのハンドルを取得
            let con_handle = CreateFileA(
                b"CONOUT$\0".as_ptr() as *const u8,
                GENERIC_WRITE,
                FILE_SHARE_WRITE,
                std::ptr::null_mut(),
                OPEN_EXISTING,
                0,
                std::ptr::null_mut(),
            );
            if con_handle.is_null() || con_handle == (-1isize) as *mut std::ffi::c_void {
                return;
            }
            // stdout / stderr をコンソール出力にリダイレクト
            SetStdHandle(STD_OUTPUT_HANDLE, con_handle);
            SetStdHandle(STD_ERROR_HANDLE, con_handle);
        }
    }
}

fn main() -> anyhow::Result<()> {
    #[cfg(windows)]
    win_console::attach_to_parent_console();
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
