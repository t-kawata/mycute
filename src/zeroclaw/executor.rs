use std::path::PathBuf;
use std::process::{Child, Command, Stdio};

#[cfg(windows)]
const EXE_ZEROCLAW: &str = "zeroclaw.exe";
#[cfg(unix)]
const EXE_ZEROCLAW: &str = "zeroclaw";

pub struct ZeroClawManager {
    /// バイナリが配置されているディレクトリ (e.g., ~/.mycute/zeroclaw/v1.0.0/)
    install_dir: PathBuf,
    /// 設定ファイル config.toml が配置されているディレクトリ (e.g., ~/.mycute/zeroclaw/)
    config_dir: PathBuf,
}

impl ZeroClawManager {
    pub fn new(install_dir: PathBuf, config_dir: PathBuf) -> Self {
        Self {
            install_dir,
            config_dir,
        }
    }

    /// zeroclaw コマンドの絶対パスを取得
    pub fn get_zeroclaw_path(&self) -> PathBuf {
        self.install_dir.join(EXE_ZEROCLAW)
    }

    /// ZeroClaw プロセスを生成します。
    /// RT との運命共同体（Fate-Sharing）を実現するため、Stdio::piped() を使用して
    /// 標準入力パイプを接続した状態で起動します。
    pub fn spawn(&self, port: u16) -> std::io::Result<Child> {
        let exe = self.get_zeroclaw_path();
        log::info!("Spawning ZeroClaw: {:?} with port: {}", exe, port);

        Command::new(exe)
            .arg("daemon")
            .arg("--config-dir")
            .arg(&self.config_dir)
            .arg("--port")
            .arg(port.to_string())
            .arg("--host")
            .arg("0.0.0.0")
            .stdin(Stdio::piped()) // RT 終了時にパイプが閉じられ、ZeroClaw も道連れに終了する
            .spawn()
    }
}
