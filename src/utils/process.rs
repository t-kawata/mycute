use std::process::{Child, Command};
use crate::constants::WIN_CREATE_NO_WINDOW;

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
