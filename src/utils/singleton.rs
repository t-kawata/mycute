use crate::constants::APP_NAME;
use fd_lock::RwLock;
use std::env;
use std::fs::{self, File};
use std::io;
use std::path::PathBuf;

// グローバルなロックガードを保持して、プロセス終了までロックを維持する
// main関数で確保したガードをここに格納する
// LockFileEx/flock はファイル記述子が閉じられるまで有効。
// RwLockWriteGuard を保持し続けることで、File が Drop されるのを防ぐ。
static mut GLOBAL_LOCK: Option<fd_lock::RwLockWriteGuard<'static, File>> = None;
// release_lock() で Box を回収してドロップするために生ポインタを保持する
static mut GLOBAL_LOCK_PTR: *mut fd_lock::RwLock<File> = std::ptr::null_mut();

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
    // Box::into_raw で生ポインタを取得し、release_lock() で回収・ドロップできるようにする。
    let lock = Box::new(RwLock::new(file));
    let lock_ptr = Box::into_raw(lock);
    // Safety: lock_ptr は acquire_lock が成功した後、release_lock() が呼ばれるまで
    // 有効であり続ける。
    let lock_ref: &'static mut RwLock<File> = unsafe { &mut *lock_ptr };

    match lock_ref.try_write() {
        Ok(guard) => {
            // ロック取得成功。
            // ガードを static 変数に保存してプロセス終了まで維持する。
            unsafe {
                GLOBAL_LOCK_PTR = lock_ptr;
                GLOBAL_LOCK = Some(guard);
            }
            Ok(())
        }
        Err(e) => {
            // ロック取得失敗: Box を回収してドロップ
            unsafe {
                let _ = Box::from_raw(lock_ptr);
            }
            if e.kind() == io::ErrorKind::WouldBlock {
                Err(format!(
                    "Another instance of {} is already running.",
                    APP_NAME
                ))
            } else {
                Err(format!(
                    "Failed to acquire lock for {}: {} (kind: {:?}, os error: {})",
                    APP_NAME,
                    e,
                    e.kind(),
                    e.raw_os_error().unwrap_or(0)
                ))
            }
        }
    }
}

/// アプリケーションのロックを解放します。
///
/// `acquire_lock` で開かれたロックファイルの OS ハンドルを閉じます。
///
/// Windows では開いているファイルハンドルがディレクトリ削除を妨げるため、
/// `reset_and_exit` などで MYCUTE_HOME を削除する直前に呼び出す必要があります。
/// macOS では不要ですが、呼び出しても安全です。
///
/// 複数回の呼び出しは安全です（2回目以降は何も行いません）。
pub fn release_lock() {
    // static mut への mutable reference を避けるため addr_of_mut! を使用する
    // （Rust 2024 互換性: static_mut_refs 警告の抑制）
    unsafe {
        // 1. 先にガードをドロップ（OS のファイルロックを解放）
        let guard = (*(std::ptr::addr_of_mut!(GLOBAL_LOCK))).take();
        if let Some(g) = guard {
            drop(g);
        }
        // 2. 続いて RwLock + File をドロップ（ファイルハンドルを閉じる）
        let lock_ptr = *(std::ptr::addr_of_mut!(GLOBAL_LOCK_PTR));
        if !lock_ptr.is_null() {
            let _ = Box::from_raw(lock_ptr);
            *(std::ptr::addr_of_mut!(GLOBAL_LOCK_PTR)) = std::ptr::null_mut();
        }
    }
}
