use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use crate::utils::process::CommandExtSafe;
use crate::bifrost::assets::BIFROST_VERSION;
use crate::bifrost::BIFROST_DIRNAME;
use crate::constants::ENV_BIFROST_AUTH_SECRET;

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
    /// lmgw_secret: RT から Bifrost への API コールに使用する Bearer トークン文字列。
    /// Bifrost の認証機能（BIFROST_AUTH_SECRET 環境変数）に透過的に注入されます。
    pub fn spawn(&self, host: &str, port: u16, lmgw_secret: &str) -> std::io::Result<Child> {
        let mut cmd = self.command();
        
        // 引数の設定 (共通ディレクトリを app-dir に指定)
        cmd.arg("-app-dir").arg(&self.root_dir)
           .arg("-host").arg(host)
           .arg("-port").arg(port.to_string());

        // LMGW シークレット注入:
        // Bifrost の認証機能を有効化し、RT だけがこのシークレットを知る構造にする。
        // これにより、Bifrost 管理 API へのアクセスはこのシークレットを持つ主体のみに制限される。
        cmd.env(ENV_BIFROST_AUTH_SECRET, lmgw_secret);

        // Fate-Sharing の核心:
        // 親プロセス (RT) と標準入力をパイプで繋ぐ。
        // これにより、RT がパニックや強制終了等で消滅した際、
        // OS によりパイプが閉じられたことを Bifrost 側が検知可能になる。
        cmd.stdin(Stdio::piped())
           .stdout(Stdio::inherit()) // ログは RT の標準出力へ
           .stderr(Stdio::inherit());

        log::info!("Spawning Bifrost: {:?} with app-dir {:?}", self.get_bifrost_path(), self.root_dir);
        cmd.hide_window_if_windows()
           .spawn()
    }
}
