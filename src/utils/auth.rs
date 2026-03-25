use crate::constants::APP_SERVER_NAME;
use anyhow::Context;
use std::env;
use std::fs::OpenOptions;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};

pub fn spawn_elevated_server(args: &[&str], log_file: &Path) -> anyhow::Result<u32> {
    #[cfg(target_os = "macos")]
    {
        spawn_elevated_server_macos(args, log_file)
    }
    #[cfg(target_os = "linux")]
    {
        spawn_elevated_server_linux(args, log_file)
    }
    #[cfg(target_os = "windows")]
    {
        spawn_elevated_server_windows(args, log_file)
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        Ok(0) // モバイル環境では何もしない
    }
}

/// バックエンドサーバーのバイナリパスを取得する
fn get_server_exe() -> anyhow::Result<std::path::PathBuf> {
    let mut exe = env::current_exe()?;
    let exe_name = exe.file_name().unwrap_or_default().to_string_lossy();

    // Tauri dev 時 (env::current_exe が mycute または tauri-runner) の場合、
    // 隣にある mycute-server を探し、なければエラーを返す（再帰起動はもう行わない）
    if !exe_name.contains("server") {
        let server_name = if cfg!(windows) {
            format!("{}.exe", APP_SERVER_NAME)
        } else {
            APP_SERVER_NAME.to_string()
        };
        let server_exe = exe.with_file_name(server_name);
        if server_exe.exists() {
            exe = server_exe;
        } else {
            anyhow::bail!("Backend server binary not found at: {:?}", server_exe);
        }
    }
    Ok(exe)
}

/// Drop時にバックエンドプロセスを強制終了するガード。
/// GUIが終了した際にゾンビプロセスが残るのを防ぐために使用される。
pub struct BackendProcessGuard {
    pub pid: u32,
    pub log_path: Option<std::path::PathBuf>,
}

impl BackendProcessGuard {
    pub fn new(pid: u32, log_path: Option<std::path::PathBuf>) -> Self {
        Self { pid, log_path }
    }
}

impl Drop for BackendProcessGuard {
    fn drop(&mut self) {
        if self.pid == 0 {
            return;
        }
        log::info!("Killing backend process (PID: {}) on exit...", self.pid);

        #[cfg(unix)]
        {
            use std::process::Command;
            let _ = Command::new("kill")
                .arg("-9")
                .arg(self.pid.to_string())
                .spawn();
        }

        #[cfg(windows)]
        {
            use std::process::Command;
            let _ = Command::new("taskkill")
                .arg("/F")
                .arg("/PID")
                .arg(self.pid.to_string())
                .spawn();
        }

        // クリーンナップ: 中間ログファイルを削除
        if let Some(path) = &self.log_path {
            if path.exists() {
                log::info!("Cleaning up intermediate log file: {:?}", path);
                let _ = std::fs::remove_file(path);
            }
        }
    }
}

// -----------------------------------------------------------------------------------------
// MACOS の実装
// -----------------------------------------------------------------------------------------
#[cfg(target_os = "macos")]
fn spawn_elevated_server_macos(args: &[&str], log_file: &Path) -> anyhow::Result<u32> {
    let exe = get_server_exe()?;
    let exe_str = exe.to_string_lossy();

    log::info!("Requesting elevation via osascript (GUI prompt).");
    let mut args_str = String::new();
    for arg in args {
        args_str.push_str(&format!("'{}' ", arg.replace("'", "'\\''")));
    }
    let log_file_str = log_file.to_string_lossy();
    // バックグラウンドで実行し、PIDを出力する
    let cmd_str = format!(
        "'{}' {} > {} 2>&1 & echo $!",
        exe_str.replace("'", "'\\''"),
        args_str,
        log_file_str
    );
    let script = format!(
        "do shell script \"{}\" with administrator privileges",
        cmd_str.replace("\"", "\\\"")
    );

    let output = Command::new("osascript").arg("-e").arg(&script).output()?;
    if !output.status.success() {
        anyhow::bail!(
            "Elevation failed (User cancelled or error): {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let pid_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let pid: u32 = pid_str
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid PID returned: {}", pid_str))?;
    log::info!("Elevated process spawned with PID: {}", pid);
    Ok(pid)
}

// -----------------------------------------------------------------------------------------
// LINUX の実装
// -----------------------------------------------------------------------------------------
#[cfg(target_os = "linux")]
fn spawn_elevated_server_linux(args: &[&str], _log_file: &Path) -> anyhow::Result<u32> {
    let exe = get_server_exe()?;

    // pkexec の有無を確認
    let has_pkexec = Command::new("which")
        .arg("pkexec")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    let mut cmd;
    if has_pkexec {
        cmd = Command::new("pkexec");
        // pkexec は通常絶対パスを要求する（current_exe は絶対パス）
        cmd.arg(exe);
    } else {
        cmd = Command::new("sudo");
        cmd.arg(exe);
    }
    cmd.args(args);

    let child = cmd.spawn()?;
    Ok(child.id())
}

// -----------------------------------------------------------------------------------------
// WINDOWS の実装
// -----------------------------------------------------------------------------------------
#[cfg(target_os = "windows")]
fn spawn_elevated_server_windows(args: &[&str], _log_file: &Path) -> anyhow::Result<u32> {
    let exe = get_server_exe()?;
    let exe_str = exe.to_string_lossy();

    let mut args_str = String::new();
    for arg in args {
        args_str.push_str(&format!(" \"{}\"", arg));
    }

    let ps_script = format!(
        "Start-Process -FilePath '{}' -ArgumentList '{}' -Verb RunAs -WindowStyle Hidden -PassThru | Select-Object -ExpandProperty Id",
        exe_str, args_str
    );

    let output = Command::new("powershell")
        .arg("-Command")
        .arg(&ps_script)
        .output()?;

    if !output.status.success() {
        anyhow::bail!(
            "Elevation failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let pid_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let pid: u32 = pid_str
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid PID: {}", pid_str))?;
    Ok(pid)
}

/// 一般ユーザー権限でバックエンドサーバーを起動する
pub fn spawn_normal_server(args: &[&str], log_file: &Path) -> anyhow::Result<u32> {
    let exe = get_server_exe()?;
    log::info!("Spawning backend server. Exe: {:?}, Args: {:?}", exe, args);
    let mut cmd = Command::new(exe);
    cmd.args(args);

    #[cfg(windows)]
    {
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

    // 標準出力と標準エラーをログファイルにリダイレクト
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_file)?;
    let file_err = file.try_clone()?;
    cmd.stdout(Stdio::from(file));
    cmd.stderr(Stdio::from(file_err));

    let child = cmd
        .spawn()
        .context("Failed to spawn normal backend server")?;
    let pid = child.id();
    log::info!("Normal process spawned with PID: {}", pid);
    Ok(pid)
}
