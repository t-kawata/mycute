use std::path::{Path, PathBuf};
use std::process::Command;
use crate::bifrost::assets::BIFROST_VERSION;
use crate::bifrost::BIFROST_DIRNAME;

#[cfg(windows)]
const EXE_BIFROST: &str = "bifrost-http.exe";
#[cfg(unix)]
const EXE_BIFROST: &str = "bifrost-http";

pub struct BifrostManager {
    root_dir: PathBuf,
}

impl BifrostManager {
    pub fn new(home: &Path) -> Self {
        Self {
            root_dir: home.join(BIFROST_DIRNAME).join(BIFROST_VERSION),
        }
    }

    /// bifrost コマンドの絶対パスを取得
    pub fn get_bifrost_path(&self) -> PathBuf {
        self.root_dir.join(EXE_BIFROST)
    }

    /// Bifrost の Command オブジェクトを作成します。
    pub fn command(&self) -> Command {
        Command::new(self.get_bifrost_path())
    }
}
