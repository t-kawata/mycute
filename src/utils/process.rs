use std::process::{Child, Command};
use std::net::TcpListener;
use std::time::{Duration, Instant};
use crate::constants::{IP_LOCALHOST, WIN_CREATE_NO_WINDOW};

/// 子プロセスのライフサイクルを管理するガード構造体。
///
/// この構造体がドロップされると、関連付けられた子プロセスを強制終了（kill）します。
/// また、標準入力がパイプされている場合は、明示的にクローズすることで
/// 子プロセス側の自死（Fate-Sharing）を促します。
pub struct ChildProcessGuard {
    child: Child,
    name: String,
}

impl ChildProcessGuard {
    pub fn new(child: Child, name: &str) -> Self {
        Self {
            child,
            name: name.to_string(),
        }
    }

    /// 子プロセスの PID を取得します
    pub fn id(&self) -> u32 {
        self.child.id()
    }

    /// 標準入力を明示的にクローズします
    pub fn close_stdin(&mut self) {
        let _ = self.child.stdin.take();
    }
}

/// Windows専用のプロセス生成フラグを安全に設定するためのトレイト。
/// 非Windows環境では何もしない（no-op）ため、コードの可読性が向上します。
pub trait CommandExtSafe {
    fn creation_flags_if_windows(&mut self, flags: u32) -> &mut Self;
    fn hide_window_if_windows(&mut self) -> &mut Self;
}

impl CommandExtSafe for Command {
    fn creation_flags_if_windows(&mut self, _flags: u32) -> &mut Self {
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            self.creation_flags(_flags);
        }
        self
    }

    /// Windows環境でのみコンソールウィンドウを非表示にするフラグを適用します。
    fn hide_window_if_windows(&mut self) -> &mut Self {
        self.creation_flags_if_windows(WIN_CREATE_NO_WINDOW)
    }
}

impl Drop for ChildProcessGuard {
    fn drop(&mut self) {
        let pid = self.child.id();
        log::info!("<Fate-Sharing> Terminating child process: {} (PID: {})", self.name, pid);

        // まず標準入力をドロップしてクローズを試みる (自死の誘導)
        let _ = self.child.stdin.take();

        // 物理的な kill
        #[cfg(unix)]
        {
            use std::process::Command;
            // 猶予なしの kill -9
            let _ = Command::new("kill")
                .arg("-9")
                .arg(pid.to_string())
                .spawn();
        }

        #[cfg(windows)]
        {
            use std::process::Command;
            // 強制終了 /F
            let _ = Command::new("taskkill")
                .arg("/F")
                .arg("/PID")
                .arg(pid.to_string())
                .spawn();
        }

        // Child 構造体自体の終了待機は行わない（ゾンビにならないよう OS に任せるか
        // または wait() を非同期で回しているタスクに任せる）
        let _ = self.child.kill();
    }
}

/// 指定されたポートを使用しているプロセスの PID をすべて取得します。
pub fn find_pids_by_port(port: u16) -> Vec<u32> {
    let mut pids = Vec::new();

    #[cfg(unix)]
    {
        // macOS/Linux: lsof -t -iTCP:port -sTCP:LISTEN
        let output = Command::new("lsof")
            .args(&["-t", "-iTCP", &format!(":{}", port), "-sTCP:LISTEN"])
            .output();

        if let Ok(output) = output {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    if let Ok(pid) = line.trim().parse::<u32>() {
                        pids.push(pid);
                    }
                }
            }
        }
    }

    #[cfg(windows)]
    {
        // Windows: netstat -ano | findstr :port | findstr LISTENING
        let cmd = format!("netstat -ano | findstr :{} | findstr LISTENING", port);
        let output = Command::new("cmd")
            .args(&["/C", &cmd])
            .hide_window_if_windows()
            .output();

        if let Ok(output) = output {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    // netstat output format:
                    //   TCP    0.0.0.0:3910           0.0.0.0:0              LISTENING       1234
                    if let Some(pid_str) = line.split_whitespace().last() {
                        if let Ok(pid) = pid_str.parse::<u32>() {
                            pids.push(pid);
                        }
                    }
                }
            }
        }
    }

    pids
}

/// 指定されたポートを使用しているプロセスをすべて強制終了し、
/// ポートが完全に解放される（bind可能になる）まで最大2秒間待機します。
pub fn kill_process_on_port(port: u16) {
    let my_pid = std::process::id();
    let pids = find_pids_by_port(port);

    if !pids.is_empty() {
        for pid in pids {
            if pid == my_pid {
                continue;
            }

            log::warn!("<Process> Found stale process {} listening on port {}. Terminating...", pid, port);

            #[cfg(unix)]
            {
                match Command::new("kill").arg("-9").arg(pid.to_string()).status() {
                    Ok(s) if s.success() => log::info!("<Process> Successfully sent KILL signal to process {}.", pid),
                    Ok(s) => log::error!("<Process> Failed to kill process {}: exit code {:?}", pid, s.code()),
                    Err(e) => log::error!("<Process> Error executing kill for {}: {}", pid, e),
                }
            }

            #[cfg(windows)]
            {
                match Command::new("taskkill")
                    .arg("/F")
                    .arg("/PID")
                    .arg(pid.to_string())
                    .hide_window_if_windows()
                    .status() {
                    Ok(s) if s.success() => log::info!("<Process> Successfully terminated process {}.", pid),
                    Ok(s) => log::error!("<Process> Failed to taskkill process {}: exit code {:?}", pid, s.code()),
                    Err(e) => log::error!("<Process> Error executing taskkill for {}: {}", pid, e),
                }
            }
        }
    }

    // ポートが実際に解放されるのを待機する（OSカーネルによるソケットクリーンアップ待ち）
    if !wait_for_port_release(port, 2000) {
        log::error!("<Process> Port {} remains unavailable even after kill attempt. Subsequent bind may fail.", port);
    }
}

/// 複数のポートに対して一括でクリーンアップ（プロセス終了と解放待ち）を実行します。
/// 
/// `tag` はログの識別子として使用されます（例: "Startup", "Fate-Sharing"）。
pub fn kill_processes_on_ports(ports: &[u16], tag: &str) {
    if ports.is_empty() {
        return;
    }
    log::info!("<{}> Starting proactive cleanup for ports: {:?}", tag, ports);
    
    let mut handles = Vec::new();
    for &port in ports {
        handles.push(std::thread::spawn(move || {
            kill_process_on_port(port);
        }));
    }

    for handle in handles {
        let _ = handle.join();
    }
    log::info!("<{}> Cleanup completed. Ports are ready for binding.", tag);
}

/// 指定されたポートが bind 可能（解放済み）になるまで待機します。
/// 
/// `find_pids_by_port` (lsof) だけでは、プロセス終了直後の TIME_WAIT 状態などを
/// 検知できない場合があるため、実際に bind を試みることで確実性を高めます。
pub fn wait_for_port_release(port: u16, timeout_ms: u64) -> bool {
    let start = Instant::now();
    let timeout = Duration::from_millis(timeout_ms);
    let addr = format!("{}:{}", IP_LOCALHOST, port);

    while start.elapsed() < timeout {
        // 実際に bind してみる（成功すれば即座にクローズされる）
        match TcpListener::bind(&addr) {
            Ok(_) => {
                log::debug!("<Process> Port {} is now verified as free and bindable.", port);
                return true;
            }
            Err(e) => {
                // Address already in use 以外（Permission Denied等）でも、
                // 現状 bind できないという事実が重要なのでリトライを続ける。
                log::trace!("<Process> Port {} is not yet bindable: {}. Retrying...", port, e);
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    }
    false
}


/// 指定された複数の PID をすべて強制終了します。
/// 指定された PID リストに対して一括で強制終了（SIGKILL / taskkill /F）を実行します。
/// spawn() ではなく status() を使用することで、シグナルの送信完了を待機し、確実性を高めます。
pub fn kill_pids(pids: &[u32]) {
    if pids.is_empty() {
        return;
    }
    log::info!("<Process> Starting direct kill for PIDs: {:?}", pids);

    for &pid in pids {
        if pid == 0 {
            continue;
        }
        log::info!("<Process> Directly killing process {}...", pid);

        #[cfg(unix)]
        {
            use std::process::Command;
            match Command::new("kill").arg("-9").arg(pid.to_string()).status() {
                Ok(s) if s.success() => log::info!("<Process> Successfully killed process {}.", pid),
                Ok(s) => log::warn!("<Process> Failed to kill process {}: exit code {:?}", pid, s.code()),
                Err(e) => log::error!("<Process> Error executing kill for {}: {}", pid, e),
            }
        }

        #[cfg(windows)]
        {
            use std::process::Command;
            match Command::new("taskkill")
                .arg("/F")
                .arg("/PID")
                .arg(pid.to_string())
                .hide_window_if_windows()
                .status() {
                Ok(s) if s.success() => log::info!("<Process> Successfully terminated process {}.", pid),
                Ok(s) => log::warn!("<Process> Failed to taskkill process {}: exit code {:?}", pid, s.code()),
                Err(e) => log::error!("<Process> Error executing taskkill for {}: {}", pid, e),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_wait_for_port_release() {
        let port = 54321; // テスト用ポート
        let addr = format!("{}:{}", IP_LOCALHOST, port);

        // 1. ポートを一時的に占有する
        let listener = TcpListener::bind(&addr).expect("Failed to bind test port");
        
        // 2. 別スレッドで 500ms 後に解放する
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(500));
            drop(listener);
        });

        // 3. 解放を待機する
        let start = std::time::Instant::now();
        let success = wait_for_port_release(port, 2000);
        let elapsed = start.elapsed();

        assert!(success, "Port should be released");
        assert!(elapsed >= Duration::from_millis(500), "Should have waited at least 500ms");
        assert!(elapsed < Duration::from_millis(2000), "Should have finished before timeout");
        
        // 実際に bind できるか最終確認
        let _ = TcpListener::bind(&addr).expect("Port should be truly free now");
    }
}
