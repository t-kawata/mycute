use std::env;
use std::process::Command;

pub fn spawn_elevated_server(args: &[&str]) -> anyhow::Result<u32> {
    #[cfg(target_os = "macos")]
    {
        spawn_elevated_server_macos(args)
    }
    #[cfg(target_os = "linux")]
    {
        spawn_elevated_server_linux(args)
    }
    #[cfg(target_os = "windows")]
    {
        spawn_elevated_server_windows(args)
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        Ok(0) // モバイル環境では何もしない
    }
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

            // クリーンナップ: 中間ログファイルを削除
            if let Some(path) = &self.log_path {
                if path.exists() {
                    log::info!("Cleaning up intermediate log file: {:?}", path);
                    let _ = std::fs::remove_file(path);
                }
            }
        }
    }
}

// -----------------------------------------------------------------------------------------
// MACOS の実装
// -----------------------------------------------------------------------------------------
// -----------------------------------------------------------------------------------------
// MACOS の実装
// -----------------------------------------------------------------------------------------
#[cfg(target_os = "macos")]
fn spawn_elevated_server_macos(args: &[&str]) -> anyhow::Result<u32> {
    let exe = env::current_exe()?;
    let exe_str = exe.to_string_lossy();

    // GUIアプリからの起動ではTERMが設定されていなかったり、
    // 逆にVSCodeのターミナルから起動した場合はTERMがあったりと不安定。
    // 特権昇格が必要な場面では、常に osascript を使用して GUI パスワードプロンプトを出す方が堅牢。
    // 特に "sudo: a password is required" エラーを避けるためには対話的な入力が必須。

    log::info!("Requesting elevation via osascript (GUI prompt).");
    let mut args_str = String::new();
    for arg in args {
        args_str.push_str(&format!("'{}' ", arg.replace("'", "'\\''")));
    }
    let log_file = "/tmp/mycute_server.log";
    // バックグラウンドで実行し、PIDを出力する
    // 注意: do shell script は標準出力を返すので、コマンド自体の出力はリダイレクトし、
    // 最後に PID ($!) だけを echo して取得する。
    let cmd_str = format!(
        "'{}' {} > {} 2>&1 & echo $!",
        exe_str.replace("'", "'\\''"),
        args_str,
        log_file
    );
    let script = format!(
        "do shell script \"{}\" with administrator privileges",
        cmd_str.replace("\"", "\\\"")
    );

    let output = Command::new("osascript").arg("-e").arg(&script).output()?;
    if !output.status.success() {
        // キャンセルされた場合などもここに来る
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
fn spawn_elevated_server_linux(args: &[&str]) -> anyhow::Result<u32> {
    use std::path::Path;
    let exe = env::current_exe()?;

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

    // Linuxでは pkexec は認証までブロックする可能性があるため、spawn で実行したい。
    // pkexec がコマンドを起動する。
    // 認証成功を確認したほうが良いか？
    let child = cmd.spawn()?;
    Ok(child.id())
}

// -----------------------------------------------------------------------------------------
// WINDOWS の実装
// -----------------------------------------------------------------------------------------
#[cfg(target_os = "windows")]
fn spawn_elevated_server_windows(args: &[&str]) -> anyhow::Result<u32> {
    // Windowsでは Powershell の Start-Process -Verb RunAs を使用する
    let exe = env::current_exe()?;
    let exe_str = exe.to_string_lossy();

    let mut args_str = String::new();

    // 引数リストの構築
    // バックエンド側で --output 引数を受け取ってファイル書き込みを行うため、
    // PowerShellレベルでのリダイレクト（-RedirectStandardOutput）は不要かつ、
    // -Verb RunAs と併用できないため削除する。
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
