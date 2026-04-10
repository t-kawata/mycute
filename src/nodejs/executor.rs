use std::path::{Path, PathBuf};
use std::process::Command;
use crate::nodejs::assets::NODE_VERSION;
use crate::nodejs::{NODE_JS_DIRNAME, NODE_BIN_DIRNAME};
use crate::constants::MYCUTE_SCRIPTS_DIRNAME;

const BIN_NODE: &str = "node";
const BIN_NPM: &str = "npm";
const BIN_NPX: &str = "npx";

#[cfg(windows)]
const EXE_NODE: &str = "node.exe";
#[cfg(windows)]
const EXE_NPM: &str = "npm.cmd";
#[cfg(windows)]
const EXE_NPX: &str = "npx.cmd";

#[cfg(unix)]
const EXE_NODE: &str = "node";
#[cfg(unix)]
const EXE_NPM: &str = "npm";
#[cfg(unix)]
const EXE_NPX: &str = "npx";

const ENV_PATH: &str = "PATH";

pub struct NodeManager {
    home_dir: PathBuf,
    root_dir: PathBuf,
}

impl NodeManager {
    pub fn new(home: &Path) -> Self {
        Self {
            home_dir: home.to_path_buf(),
            root_dir: home.join(NODE_JS_DIRNAME).join(NODE_VERSION),
        }
    }

    /// スクプリト実行・保存の拠点ディレクトリを取得
    pub fn get_scripts_dir(&self) -> PathBuf {
        self.home_dir.join(MYCUTE_SCRIPTS_DIRNAME)
    }

    /// node コマンドの絶対パスを取得
    pub fn get_node_path(&self) -> PathBuf {
        if cfg!(target_os = "windows") {
            self.root_dir.join(EXE_NODE)
        } else {
            self.root_dir.join(NODE_BIN_DIRNAME).join(EXE_NODE)
        }
    }

    /// npm コマンドの絶対パスを取得
    pub fn get_npm_path(&self) -> PathBuf {
        if cfg!(target_os = "windows") {
            self.root_dir.join(EXE_NPM)
        } else {
            self.root_dir.join(NODE_BIN_DIRNAME).join(EXE_NPM)
        }
    }

    /// npx コマンドの絶対パスを取得
    pub fn get_npx_path(&self) -> PathBuf {
        if cfg!(target_os = "windows") {
            self.root_dir.join(EXE_NPX)
        } else {
            self.root_dir.join(NODE_BIN_DIRNAME).join(EXE_NPX)
        }
    }

    /// 指定されたコマンド名 (node, npm, npx) に対して、
    /// 正しくパスと環境変数が設定された Command オブジェクトを作成します。
    pub fn command(&self, bin_name: &str) -> Command {
        let bin_path = match bin_name {
            BIN_NODE => self.get_node_path(),
            BIN_NPM => self.get_npm_path(),
            BIN_NPX => self.get_npx_path(),
            _ => self.root_dir.join(bin_name), // 基本は root 直下と仮定
        };

        let mut cmd = Command::new(bin_path);
        
        // npm等の中で node が見つかるよう、PATH の先頭に node のディレクトリを追加
        let bin_dir = if cfg!(target_os = "windows") {
            self.root_dir.clone()
        } else {
            self.root_dir.join(NODE_BIN_DIRNAME)
        };

        // 元のロジック（修正前）により忠実な置換
        if let Some(path) = std::env::var_os(ENV_PATH) {
            let mut paths = std::env::split_paths(&path).collect::<Vec<_>>();
            paths.insert(0, bin_dir);
            if let Ok(new_path) = std::env::join_paths(paths) {
                cmd.env(ENV_PATH, new_path);
            }
        } else {
            cmd.env(ENV_PATH, bin_dir);
        }

        cmd
    }

    pub fn node(&self) -> Command {
        self.command(BIN_NODE)
    }

    pub fn npm(&self) -> Command {
        self.command(BIN_NPM)
    }

    pub fn npx(&self) -> Command {
        self.command(BIN_NPX)
    }
}
