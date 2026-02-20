use crate::constants::APP_NAME;
use fd_lock::RwLock;
use std::env;
use std::fs::{self, File};
use std::path::PathBuf;

// グローバルなロックガードを保持して、プロセス終了までロックを維持する
// main関数で確保したガードをここに格納する
// LockFileEx/flock はファイル記述子が閉じられるまで有効。
// RwLockWriteGuard を保持し続けることで、File が Drop されるのを防ぐ。
static mut GLOBAL_LOCK: Option<fd_lock::RwLockWriteGuard<'static, File>> = None;

/// アプリケーションの単一インスタンス性を保証するロックを取得する
///
/// # Arguments
/// - `lock_name`: ロックファイル名（例: "mycute.lock", "mycute-app.lock"）
///
/// # Returns
/// - `Ok(())`: ロック取得成功。このプロセスが唯一のインスタンス。
/// - `Err(String)`: ロック取得失敗。既に別のインスタンスが起動中。
pub fn acquire_lock(lock_name: &str) -> Result<(), String> {
    let home = env::var("HOME")
        .or_else(|_| env::var("APPDATA").or_else(|_| env::var("USERPROFILE")))
        .map_err(|_| "Could not determine home directory".to_string())?;

    let lock_dir = PathBuf::from(home).join(format!(".{}", APP_NAME));

    // ディレクトリが存在しない場合は作成
    if !lock_dir.exists() {
        fs::create_dir_all(&lock_dir)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }

    let lock_path = lock_dir.join(lock_name);

    log::debug!("Acquiring singleton lock at: {:?}", lock_path);

    // ロックファイルを開く（作成）
    let file = File::options()
        .read(true)
        .write(true)
        .create(true)
        .open(&lock_path)
        .map_err(|e| format!("Failed to open lock file: {}", e))?;

    // 排他的書き込みロックを取得
    // try_write は即座に結果を返す。既にロックされていれば Error になる。
    // RwLock 自体をメモリリークさせて、ガードのライフタイムを 'static に延長する
    let lock = Box::new(RwLock::new(file));
    let lock_ref = Box::leak(lock);

    match lock_ref.try_write() {
        Ok(guard) => {
            // ロック取得成功。
            // ガードを static 変数に保存してプロセス終了まで維持する。
            unsafe {
                GLOBAL_LOCK = Some(guard);
            }
            Ok(())
        }
        Err(_) => Err(format!(
            "Another instance of {} is already running.",
            APP_NAME
        )),
    }
}
