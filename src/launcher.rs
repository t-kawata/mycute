#![cfg_attr(windows, windows_subsystem = "windows")]
//! mycute-server-launcher
//!
//! 依存関係を一切持たない軽量なランチャーバイナリ。
//! 実際のサーバーロジック（core）と共有ライブラリ（dylib/dll）を内包し、
//! 実行時にバイナリと同一ディレクトリに展開して起動する。

use std::env;
use std::fs;
use std::process::Command;
use mycute::utils::init::{CommonFlgs, LogLevel, setup_logging};

// ============================================================
// Embedded Resources (Self-Extraction)
// ============================================================

#[cfg(target_os = "macos")]
static LIB_SHERPA_BYTES: &[u8] = include_bytes!("../target/release/libsherpa-onnx-c-api.dylib");
#[cfg(target_os = "macos")]
static LIB_ONNX_BYTES: &[u8] = include_bytes!("../target/release/libonnxruntime.1.17.1.dylib");
#[cfg(target_os = "macos")]
static CORE_BIN_BYTES: &[u8] = include_bytes!("../target/release/mycute-server-core");

#[cfg(target_os = "windows")]
static LIB_SHERPA_BYTES: &[u8] = include_bytes!("../target/release/sherpa-onnx-c-api.dll");
#[cfg(target_os = "windows")]
static LIB_ONNX_BYTES: &[u8] = include_bytes!("../target/release/onnxruntime.dll");
#[cfg(target_os = "windows")]
static LIB_SPEECH_HELPER_BYTES: &[u8] = include_bytes!("../target/release/SpeechHelper.dll");
#[cfg(target_os = "windows")]
static LIB_VC_140_BYTES: &[u8] = include_bytes!("../native/win/libs/vcruntime140.dll");
#[cfg(target_os = "windows")]
static LIB_VC_140_1_BYTES: &[u8] = include_bytes!("../native/win/libs/vcruntime140_1.dll");
#[cfg(target_os = "windows")]
static LIB_MSVCP_140_BYTES: &[u8] = include_bytes!("../native/win/libs/msvcp140.dll");
#[cfg(target_os = "windows")]
static CORE_BIN_BYTES: &[u8] = include_bytes!("../target/release/mycute-server-core.exe");

const CORE_BIN_NAME: &str = if cfg!(target_os = "windows") {
    "__mycute-server-core.exe"
} else {
    ".__mycute-server-core"
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // バイナリディレクトリにログを出力する（昇格実行でstdoutが見えない場合も残る）
    let exe_path = env::current_exe()?;
    let bin_dir = exe_path.parent().ok_or("Failed to get binary directory")?.to_path_buf();
    let launcher_log = bin_dir.join("mycute-launcher.log");

    let common = CommonFlgs {
        log_level: LogLevel::Debug,
        output: launcher_log.to_string_lossy().to_string(),
        home: None,
    };
    let _ = setup_logging(&common);

    let result = run_launcher();

    // Exit status: GUIが終了状態を読み取れるようにする
    // "0" = 成功, "1:エラーメッセージ" = 失敗
    let status_path = bin_dir.join("launcher-exit.status");
    match &result {
        Ok(_) => {
            let _ = fs::write(&status_path, "0");
            log::info!("<Launcher> Completed successfully.");
        }
        Err(e) => {
            let msg = format!("1:{}", e);
            let _ = fs::write(&status_path, &msg);
            log::error!("CRITICAL: Launcher failure: {:?}", e);
            eprintln!("Launcher Error: {}", e);
        }
    }

    result
}

fn run_launcher() -> Result<(), Box<dyn std::error::Error>> {
    let exe_path = env::current_exe()?;
    let bin_dir = exe_path.parent().ok_or("Failed to get binary directory")?;

    // 展開するファイルのリスト
    let mut files = Vec::new();

    #[cfg(target_os = "macos")]
    {
        files.push(("libsherpa-onnx-c-api.dylib", LIB_SHERPA_BYTES));
        files.push(("libonnxruntime.1.17.1.dylib", LIB_ONNX_BYTES));
    }

    #[cfg(target_os = "windows")]
    {
        files.push(("sherpa-onnx-c-api.dll", LIB_SHERPA_BYTES));
        files.push(("onnxruntime.dll", LIB_ONNX_BYTES));
        files.push(("SpeechHelper.dll", LIB_SPEECH_HELPER_BYTES));
        files.push(("vcruntime140.dll", LIB_VC_140_BYTES));
        files.push(("vcruntime140_1.dll", LIB_VC_140_1_BYTES));
        files.push(("msvcp140.dll", LIB_MSVCP_140_BYTES));
    }

    // コアバイナリを追加
    files.push((CORE_BIN_NAME, CORE_BIN_BYTES));

    // ファイル展開
    for (name, bytes) in files {
        let path = bin_dir.join(name);
        // Note: 開発中やアップデート時に確実に最新のバイナリ・ライブラリを使用するため、
        // 既存のファイルがあり、かつサイズが異なる場合のみ再展開を行う。
        // サイズが同一であれば、既に最新であるとみなしてスキップする。
        if path.exists() {
            if let Ok(meta) = fs::metadata(&path) {
                if meta.len() == bytes.len() as u64 {
                    log::info!("<Launcher> File already up-to-date: {:?} (size: {}). Skipping.", name, meta.len());
                    continue;
                }
            }
            let _ = fs::remove_file(&path);
        }
        log::info!("<Launcher> [V-FIX-03] Refreshing file: {:?} (size: {})...", path, bytes.len());
        // 書き込みに失敗しても致命的ではない（DLLは既存版でも動作可能）。
        // ただしコアバイナリの書き込み失敗は起動不能を意味するのでエラーにする。
        if let Err(e) = fs::write(&path, bytes) {
            if name == CORE_BIN_NAME {
                return Err(format!("Failed to write core binary {:?}: {}", path, e).into());
            }
            log::warn!("<Launcher> Failed to refresh {:?}: {}. Using existing file.", name, e);
        }

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Err(e) = fs::set_permissions(&path, fs::Permissions::from_mode(0o755)) {
                log::warn!("<Launcher> Failed to set permissions on {:?}: {}", path, e);
            }
        }
    }

    // コアバイナリを起動
    let core_path = bin_dir.join(CORE_BIN_NAME);
    let args: Vec<String> = env::args().skip(1).collect();

    log::info!("<Launcher> Forwarding args: {:?} to core binary...", args);
    let mut child = Command::new(core_path).args(args).spawn()?;

    let status = child.wait()?;

    if !status.success() {
        return Err(format!("Core binary exited with status: {}", status).into());
    }

    Ok(())
}
