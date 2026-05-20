use crate::constants::APP_NAME;

#[cfg(windows)]
use std::ffi::c_void;
#[cfg(not(windows))]
use std::env;
#[cfg(not(windows))]
use std::path::PathBuf;

#[cfg(not(windows))]
use fd_lock::RwLock;
#[cfg(not(windows))]
use std::fs::{self, File};
#[cfg(not(windows))]
use std::io;

// ============================================================
// 静的ストレージ: ロックハンドル/ガードをプロセス終了まで保持
// ============================================================

/// Windows: Named Mutex のハンドル（CloseHandle で解放。プロセス終了時にはOSが自動解放）
#[cfg(windows)]
static mut GLOBAL_MUTEX_HANDLE: *mut c_void = std::ptr::null_mut();

/// Unix: LockFileEx (flock) 用 — RwLockWriteGuard を保持し File の Drop を防ぐ
#[cfg(not(windows))]
static mut GLOBAL_LOCK: Option<fd_lock::RwLockWriteGuard<'static, File>> = None;
#[cfg(not(windows))]
static mut GLOBAL_LOCK_PTR: *mut fd_lock::RwLock<File> = std::ptr::null_mut();

// ============================================================
// Windows: Named Mutex API (kernel32)
// ============================================================
#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    fn CreateMutexW(
        lpMutexAttributes: *mut c_void,
        bInitialOwner: i32,
        lpName: *const u16,
    ) -> *mut c_void;

    fn WaitForSingleObject(
        hHandle: *mut c_void,
        dwMilliseconds: u32,
    ) -> u32;

    fn CloseHandle(
        hObject: *mut c_void,
    ) -> i32;
}

#[cfg(windows)]
const WAIT_OBJECT_0: u32 = 0x00000000;
#[cfg(windows)]
const WAIT_ABANDONED: u32 = 0x00000080;
#[cfg(windows)]
const WAIT_TIMEOUT: u32 = 0x00000102;

/// アプリケーションの単一インスタンス性を保証するロックを取得する
///
/// Windows: Named Mutex (CreateMutexW) を使用。カーネルオブジェクトなので
/// プロセス終了時にOSが自動解放し、スタブファイル問題が発生しない。
/// Unix: fd_lock (flock) を使用。Unix の flock もプロセス終了時にカーネルが解放する。
///
/// # Arguments
/// - `lock_name`: ロック名（例: "mycute.lock", "mycute-app.lock"）
///
/// # Returns
/// - `Ok(())`: ロック取得成功。このプロセスが唯一のインスタンス。
/// - `Err(String)`: ロック取得失敗。既に別のインスタンスが起動中。
pub fn acquire_lock(lock_name: &str) -> Result<(), String> {
    #[cfg(windows)]
    {
        acquire_lock_windows(lock_name)
    }

    #[cfg(not(windows))]
    {
        acquire_lock_unix(lock_name)
    }
}

/// Windows: Named Mutex によるシングルトンロック獲得
#[cfg(windows)]
fn acquire_lock_windows(lock_name: &str) -> Result<(), String> {
    // UTF-16 null終端文字列に変換
    let mut wide: Vec<u16> = lock_name.encode_utf16().collect();
    wide.push(0);

    // ミューテックスを作成またはオープン
    // bInitialOwner=FALSE: 所有権を即時取得せず、後続の WaitForSingleObject に委ねる
    let handle = unsafe { CreateMutexW(std::ptr::null_mut(), 0, wide.as_ptr()) };

    if handle.is_null() {
        return Err(format!(
            "Failed to create singleton mutex for {} (kernel error).",
            APP_NAME
        ));
    }

    // 非ブロッキング try-wait (0ms)
    match unsafe { WaitForSingleObject(handle, 0) } {
        WAIT_OBJECT_0 => {
            // 新規作成または無競合での獲得成功
            log::debug!("Singleton mutex acquired: {}", lock_name);
            unsafe {
                GLOBAL_MUTEX_HANDLE = handle;
            }
            Ok(())
        }
        WAIT_ABANDONED => {
            // 前所有者がクラッシュ — このスレッドが所有権を継承（成功扱い）
            log::warn!(
                "Stale mutex detected (previous owner of '{}' crashed). Ownership inherited.",
                lock_name
            );
            unsafe {
                GLOBAL_MUTEX_HANDLE = handle;
            }
            Ok(())
        }
        WAIT_TIMEOUT => {
            // 他プロセスが生きて所有中
            unsafe { CloseHandle(handle); }
            Err(format!(
                "Another instance of {} is already running.",
                APP_NAME
            ))
        }
        other => {
            // 予期しない戻り値
            unsafe { CloseHandle(handle); }
            Err(format!(
                "Unexpected WaitForSingleObject result ({}) for {}",
                other, APP_NAME
            ))
        }
    }
}

/// Unix: fd_lock (flock) によるシングルトンロック獲得
#[cfg(not(windows))]
fn acquire_lock_unix(lock_name: &str) -> Result<(), String> {
    let home = env::var("HOME")
        .or_else(|_| env::var("APPDATA").or_else(|_| env::var("USERPROFILE")))
        .map_err(|_| "Could not determine home directory".to_string())?;

    let lock_dir = PathBuf::from(home).join(format!(".{}", APP_NAME));

    if !lock_dir.exists() {
        fs::create_dir_all(&lock_dir)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }

    let lock_path = lock_dir.join(lock_name);

    log::debug!("Acquiring singleton lock at: {:?}", lock_path);

    let file = File::options()
        .read(true)
        .write(true)
        .create(true)
        .open(&lock_path)
        .map_err(|e| format!("Failed to open lock file: {}", e))?;

    let lock = Box::new(RwLock::new(file));
    let lock_ptr = Box::into_raw(lock);
    // Safety: lock_ptr は acquire_lock 成功後、release_lock() が呼ばれるまで有効
    let lock_ref: &'static mut RwLock<File> = unsafe { &mut *lock_ptr };

    match lock_ref.try_write() {
        Ok(guard) => {
            unsafe {
                GLOBAL_LOCK_PTR = lock_ptr;
                GLOBAL_LOCK = Some(guard);
            }
            Ok(())
        }
        Err(e) => {
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
/// Windows: CloseHandle() で Named Mutex のハンドルを閉じる。
/// Unix: ロックガードをドロップし、ファイルハンドルを閉じる。
///
/// `reset_and_exit` などで MYCUTE_HOME を削除する直前に呼び出す必要があります。
/// （Windows では開いているファイルハンドルがディレクトリ削除を妨げるため。
///   macOS では不要ですが、呼び出しても安全です。）
///
/// 複数回の呼び出しは安全です（2回目以降は何も行いません）。
pub fn release_lock() {
    #[cfg(windows)]
    unsafe {
        let handle_ptr = std::ptr::addr_of_mut!(GLOBAL_MUTEX_HANDLE);
        let handle = *handle_ptr;
        if !handle.is_null() {
            CloseHandle(handle);
            *handle_ptr = std::ptr::null_mut();
            log::debug!("Singleton mutex released.");
        }
    }

    #[cfg(not(windows))]
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

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    use std::thread;
    #[cfg(windows)]
    use std::time::Duration;

    // ================================================================
    // 基本テスト（両プラットフォーム）
    // ================================================================

    /// acquire → release → 再acquire が正常に動作する
    #[test]
    fn test_acquire_release_cycle() {
        let name = format!("test-cycle-{}.lock", std::process::id());
        let r1 = acquire_lock(&name);
        assert!(r1.is_ok(), "First acquire should succeed: {:?}", r1);
        release_lock();
        let r2 = acquire_lock(&name);
        assert!(r2.is_ok(), "Re-acquire after release should succeed: {:?}", r2);
        release_lock();
    }

    /// release_lock の二重呼び出しが panic しない
    #[test]
    fn test_double_release() {
        let name = format!("test-double-{}.lock", std::process::id());
        let r = acquire_lock(&name);
        assert!(r.is_ok());
        release_lock();
        release_lock(); // 2回目も panic しないこと
    }

    /// acquire_lock なしで release_lock を呼んでも安全（null handle / null ptr）
    #[test]
    fn test_release_without_acquire() {
        release_lock(); // panic しないこと
        release_lock(); // 再度呼んでも安全
    }

    // ================================================================
    // Windows 専用テスト: Named Mutex の競合・継承
    // ================================================================

    #[cfg(windows)]
    #[test]
    fn test_windows_mutex_contention() {
        let name = format!("test-contention-{}.lock", std::process::id());

        // 別スレッドで Named Mutex を生成し所有（bInitialOwner=TRUE）、200ms保持
        let n2 = name.clone();
        let thread = thread::spawn(move || {
            let mut wide: Vec<u16> = n2.encode_utf16().collect();
            wide.push(0);
            // bInitialOwner=1: 作成と同時に所有権を獲得
            let h = unsafe { CreateMutexW(std::ptr::null_mut(), 1, wide.as_ptr()) };
            assert!(!h.is_null(), "Thread must create mutex successfully");
            // スレッド終了時にミューテックス所有権が放棄され、カーネルが
            // WAIT_ABANDONED 状態にする（CloseHandle 不要：意図的なリーク）
            thread::sleep(Duration::from_millis(200));
        });
        thread::sleep(Duration::from_millis(50));

        // 別スレッドが所有中 → WAIT_TIMEOUT → Err("Another instance")
        let r1 = acquire_lock(&name);
        assert!(r1.is_err(), "Held by another thread → should fail: {:?}", r1);

        // スレッド終了を待ち、mutex が ABANDONED 状態になるのを待つ
        thread.join().unwrap();
        thread::sleep(Duration::from_millis(50));

        // WAIT_ABANDONED → acquire_lock_windows は Ok として扱う
        let r2 = acquire_lock(&name);
        assert!(r2.is_ok(), "Abandoned mutex → should succeed: {:?}", r2);
        release_lock();
    }

    // ================================================================
    // Unix 専用テスト: ロックファイルの作成確認
    // ================================================================

    #[cfg(not(windows))]
    #[test]
    fn test_unix_lock_file_creation() {
        let name = format!("test-file-{}.lock", std::process::id());
        let r = acquire_lock(&name);
        assert!(r.is_ok(), "acquire_lock should succeed: {:?}", r);

        let home = env::var("HOME").expect("HOME must be set");
        let lock_path = PathBuf::from(home)
            .join(format!(".{}", APP_NAME))
            .join(&name);
        assert!(lock_path.exists(), "Lock file should exist on Unix: {:?}", lock_path);

        release_lock();
    }
}
