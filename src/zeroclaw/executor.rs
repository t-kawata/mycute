use std::path::{Path, PathBuf};
use std::process::Command;
use crate::zeroclaw::assets::ZEROCLAW_VERSION;
use crate::zeroclaw::ZEROCLAW_DIRNAME;

#[cfg(windows)]
const EXE_ZEROCLAW: &str = "zeroclaw.exe";
#[cfg(unix)]
const EXE_ZEROCLAW: &str = "zeroclaw";

pub struct ZeroClawManager {
    root_dir: PathBuf,
}

impl ZeroClawManager {
    pub fn new(home: &Path) -> Self {
        Self {
            root_dir: home.join(ZEROCLAW_DIRNAME).join(ZEROCLAW_VERSION),
        }
    }

    /// zeroclaw コマンドの絶対パスを取得
    pub fn get_zeroclaw_path(&self) -> PathBuf {
        self.root_dir.join(EXE_ZEROCLAW)
    }

    /// ZeroClaw の Command オブジェクトを作成します。
    pub fn command(&self) -> Command {
        Command::new(self.get_zeroclaw_path())
    }
}
