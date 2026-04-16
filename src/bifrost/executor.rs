use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use crate::bifrost::assets::BIFROST_VERSION;
use crate::bifrost::BIFROST_DIRNAME;

#[cfg(windows)]
const EXE_BIFROST: &str = "bifrost-http.exe";
#[cfg(unix)]
const EXE_BIFROST: &str = "bifrost-http";

pub struct BifrostManager {
    root_dir: PathBuf,
    install_dir: PathBuf,
}

impl BifrostManager {
    pub fn new(home: &Path) -> Self {
        let root_dir = home.join(BIFROST_DIRNAME);
        Self {
            install_dir: root_dir.join(BIFROST_VERSION),
            root_dir,
        }
    }

    /// bifrost コマンドの絶対パスを取得
    pub fn get_bifrost_path(&self) -> PathBuf {
        self.install_dir.join(EXE_BIFROST)
    }

    /// Bifrost の Command オブジェクトを作成します。
    pub fn command(&self) -> Command {
        Command::new(self.get_bifrost_path())
    }

    /// Bifrost を「運命共同体」として起動します。
    pub fn spawn(&self, host: &str, port: u16) -> std::io::Result<Child> {
        let mut cmd = self.command();
        
        // 引数の設定 (共通ディレクトリを app-dir に指定)
        cmd.arg("-app-dir").arg(&self.root_dir)
           .arg("-host").arg(host)
           .arg("-port").arg(port.to_string());

        // Fate-Sharing の核心:
        // 親プロセス (RT) と標準入力をパイプで繋ぐ。
        // これにより、RT がパニックや強制終了等で消滅した際、
        // OS によりパイプが閉じられたことを Bifrost 側が検知可能になる。
        cmd.stdin(Stdio::piped())
           .stdout(Stdio::inherit()) // ログは RT の標準出力へ
           .stderr(Stdio::inherit());

        log::info!("Spawning Bifrost: {:?} with app-dir {:?}", self.get_bifrost_path(), self.root_dir);
        cmd.spawn()
    }
}
