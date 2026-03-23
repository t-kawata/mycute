#![cfg_attr(windows, windows_subsystem = "windows")]
//! mycute-server-launcher
//!
//! 依存関係を一切持たない軽量なランチャーバイナリ。
//! 実際のサーバーロジック（core）と共有ライブラリ（dylib/dll）を内包し、
//! 実行時にバイナリと同一ディレクトリに展開して起動する。

use std::env;
use std::fs;
use std::process::Command;

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
static LIB_VC_140_BYTES: &[u8] = include_bytes!("../target/release/vcruntime140.dll");
#[cfg(target_os = "windows")]
static LIB_VC_140_1_BYTES: &[u8] = include_bytes!("../target/release/vcruntime140_1.dll");
#[cfg(target_os = "windows")]
static LIB_MSVCP_140_BYTES: &[u8] = include_bytes!("../target/release/msvcp140.dll");
#[cfg(target_os = "windows")]
static CORE_BIN_BYTES: &[u8] = include_bytes!("../target/release/mycute-server-core.exe");

const CORE_BIN_NAME: &str = if cfg!(target_os = "windows") {
    "__mycute-server-core.exe"
} else {
    ".__mycute-server-core"
};

fn main() {
    if let Err(e) = run_launcher() {
        eprintln!("CRITICAL: Launcher failure: {}", e);
        std::process::exit(1);
    }
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
        if !path.exists() {
            // 注意: ログライブラリを入れていないので標準出力に書く（ランチャーなのでシンプルに）
            println!("Extracting native dependency: {}...", name);
            fs::write(&path, bytes)?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if name == CORE_BIN_NAME {
                    let mut perms = fs::metadata(&path)?.permissions();
                    perms.set_mode(0o755);
                    fs::set_permissions(&path, perms)?;
                }
            }
        }
    }

    // コアバイナリを起動
    let core_path = bin_dir.join(CORE_BIN_NAME);
    let args: Vec<String> = env::args().skip(1).collect();

    let mut child = Command::new(core_path).args(args).spawn()?;

    let status = child.wait()?;

    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }

    Ok(())
}
