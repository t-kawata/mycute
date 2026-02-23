use crate::constants::{
    DB_DEFAULT_DIRNAME, MYCUTE_APPS_DIRNAME, MYCUTE_DATA_DIRNAME, MYCUTE_DL_DIRNAME,
    MYCUTE_LOG_DIRNAME, MYCUTE_MODELS_DIRNAME, MYCUTE_S3_DIRNAME,
};
use std::fs;
use std::path::{Path, PathBuf};

/// MYCUTE のルートディレクトリ（MYCUTE_HOME）を解決し、
/// 必要なサブディレクトリを自動生成した上で返却します。
///
pub fn get_mycute_home() -> PathBuf {
    let mut home = {
        // デフォルト: ~/.mycute
        let base_home = if cfg!(target_os = "windows") {
            std::env::var("USERPROFILE")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("C:\\"))
        } else {
            std::env::var("HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("/tmp"))
        };
        base_home.join(MYCUTE_DATA_DIRNAME)
    };

    // 絶対パスに変換
    if !home.is_absolute() {
        if let Ok(abs) = fs::canonicalize(&home) {
            home = abs;
        } else if let Ok(cwd) = std::env::current_dir() {
            // canonicalize は実体がないと失敗するため、カレントディレクトリベースで結合
            home = cwd.join(home);
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
        &home.join(MYCUTE_MODELS_DIRNAME),
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
