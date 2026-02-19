use std::path::{Path, PathBuf};
use std::fs;
use crate::constants::{
    MYCUTE_DATA_DIRNAME,
    MYCUTE_LOG_DIRNAME,
    DB_DEFAULT_DIRNAME,
    MYCUTE_S3_DIRNAME,
    MYCUTE_DL_DIRNAME,
    MYCUTE_APPS_DIRNAME,
};

/// MYCUTE のルートディレクトリ（MYCUTE_HOME）を解決し、
/// 必要なサブディレクトリを自動生成した上で返却します。
/// 
/// 引数 `forced_home` が指定されている場合はそれを最優先します（コマンドライン引数由来）。
/// 指定されていない場合は、OS標準のホームディレクトリ直下の `.mycute` を絶対パスで返します。
pub fn get_mycute_home(forced_home: Option<String>) -> PathBuf {
    let mut home = if let Some(h) = forced_home {
        PathBuf::from(h)
    } else {
        // デフォルト: ~/.mycute
        let base_home = if cfg!(target_os = "windows") {
            std::env::var("USERPROFILE").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("C:\\"))
        } else {
            std::env::var("HOME").map(PathBuf::from).unwrap_or_else(|_| PathBuf::from("/tmp"))
        };
        base_home.join(MYCUTE_DATA_DIRNAME)
    };

    // 絶対パスに変換
    if !home.is_absolute() {
        if let Ok(abs) = fs::canonicalize(&home) {
            home = abs;
        } else {
            // canonicalize はファイルが存在しないと失敗するため、カレントディレクトリベースで結合
            if let Ok(cwd) = std::env::current_dir() {
                home = cwd.join(home);
            }
        }
    }

    // 必須ディレクトリの作成
    ensure_directories(&home);

    home
}

/// 必要なディレクトリ構造を強制的に作成します。
fn ensure_directories(home: &Path) {
    let dirs = [
        home,
        &home.join(MYCUTE_LOG_DIRNAME),
        &home.join(DB_DEFAULT_DIRNAME),
        &home.join(MYCUTE_S3_DIRNAME),
        &home.join(MYCUTE_DL_DIRNAME),
        &home.join(MYCUTE_APPS_DIRNAME),
    ];

    for dir in dirs {
        if !dir.exists() {
            if let Err(e) = fs::create_dir_all(dir) {
                eprintln!("CRITICAL: Failed to create directory {:?}: {}", dir, e);
            }
        }
    }
}

pub fn get_log_dir(home: &Path) -> PathBuf {
    home.join(MYCUTE_LOG_DIRNAME)
}

pub fn get_db_dir(home: &Path) -> PathBuf {
    home.join(DB_DEFAULT_DIRNAME)
}
