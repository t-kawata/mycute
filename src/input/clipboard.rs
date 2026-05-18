//! クリップボード操作モジュール（クロスプラットフォーム対応）
//!
//! `arboard` クレートを使用し、Mac / Windows の両方でクリップボードを操作します。
//! 従来の `pbcopy` / `pbpaste` コマンド依存を排除し、ネイティブAPIを直接利用します。

use arboard::{Clipboard, Error as ArboardError};
use crate::input::keyboard::KeyboardInjector;
use std::sync::Mutex;

/// クリップボード排他制御用Mutex。
///
/// `save_paste_and_restore`, `get_selected_text`, `replace_selected_text` は
/// ホットキーハンドラスレッドとSTTイベントループの2スレッドから呼ばれる。
/// 退避→セット→ペースト→復元のシーケンスが複数スレッドでインターリーブされないよう、
/// 全クリップボード操作をこの Mutex で直列化する。
static CLIPBOARD_LOCK: Mutex<()> = Mutex::new(());

/// ペースト後のOS反映待機時間（ミリ秒）
///
/// Windows: SendInput(Ctrl+V) は対象アプリに非同期で配送される。
/// アプリがビジーで50ms以内にペーストを処理できない場合、
/// クリップボード復元後に処理が行われ、誤った内容が貼り付けられる。
/// そのためWindowsでは余裕を持った待機時間を設定する。
#[cfg(target_os = "windows")]
const PASTE_DELAY_MS: u64 = 200;
#[cfg(not(target_os = "windows"))]
const PASTE_DELAY_MS: u64 = 50;

/// 現在のクリップボード内容を取得する（ロックなし内部関数）。
/// クリップボードが空またはテキスト以外の場合は空文字列を返す。
fn get_clipboard_inner() -> Result<String, String> {
    let mut clip = Clipboard::new().map_err(|e| format!("Failed to open clipboard: {}", e))?;
    match clip.get_text() {
        Ok(text) => Ok(text),
        Err(ArboardError::ContentNotAvailable) => Ok(String::new()),
        Err(e) => Err(format!("Failed to get clipboard text: {}", e)),
    }
}

/// 現在のクリップボード内容を取得する（スレッドセーフ）。
/// クリップボードが空またはテキスト以外の場合は空文字列を返す。
pub fn get_clipboard() -> Result<String, String> {
    let _lock = CLIPBOARD_LOCK.lock().unwrap();
    get_clipboard_inner()
}

/// クリップボードにテキストを設定する（ロックなし内部関数）。
fn set_clipboard_inner(text: &str) -> Result<(), String> {
    let mut clip = Clipboard::new().map_err(|e| format!("Failed to open clipboard: {}", e))?;
    clip.set_text(text).map_err(|e| format!("Failed to set clipboard text: {}", e))
}

/// クリップボードにテキストを設定する（スレッドセーフ）。
pub fn set_clipboard(text: &str) -> Result<(), String> {
    let _lock = CLIPBOARD_LOCK.lock().unwrap();
    set_clipboard_inner(text)
}

/// 選択中のテキストを Cmd+C / Ctrl+C で取得する（スレッドセーフ）。
/// 選択されていなければ空文字列を返す。
pub fn get_selected_text() -> Result<String, String> {
    let _lock = CLIPBOARD_LOCK.lock().unwrap();

    // 現在のクリップボード内容を退避
    let saved = get_clipboard_inner().unwrap_or_default();

    // クリップボードをクリア
    set_clipboard_inner("")?;

    // Cmd+C / Ctrl+C を送信してコピー
    KeyboardInjector::send_cmd_c();

    // コピー処理がOSに反映されるまでわずかに待機
    std::thread::sleep(std::time::Duration::from_millis(PASTE_DELAY_MS));

    // 新しいクリップボード内容（＝選択されていたテキスト）を取得
    let selected = get_clipboard_inner().unwrap_or_default();

    // 何も選択されていなかった場合はクリップボードを元に戻す
    if selected.is_empty() {
        let _ = set_clipboard_inner(&saved);
    }

    Ok(selected)
}

/// 現在のクリップボード内容を退避し、指定テキストをクリップボード経由でペーストする。
/// ペースト後、退避した内容を復元する。戻り値はペースト操作の成否。
/// 複数スレッドからの同時呼び出しに対し、Mutex で排他制御を行う。
///
/// 復元前にクリップボードの内容を確認し、外部プロセスによって変更されていた場合は
/// 復元をスキップする。これにより、(1)対象アプリがペーストを処理する前に元の内容で
/// 上書きしてしまう問題と、(2)ユーザーがペースト後に別の内容をコピーした場合に
/// それを消してしまう問題の両方を防ぐ。
pub fn save_paste_and_restore(text: &str) -> bool {
    let _lock = CLIPBOARD_LOCK.lock().unwrap();
    let saved = get_clipboard_inner().unwrap_or_default();
    if let Err(e) = set_clipboard_inner(text) {
        log::error!("Failed to set clipboard for paste: {}", e);
        false
    } else {
        KeyboardInjector::send_cmd_v();
        std::thread::sleep(std::time::Duration::from_millis(PASTE_DELAY_MS));

        // クリップボードの内容を確認し、まだ自分が設定した内容なら復元する。
        // 外部で変更されていた場合は上書きせずスキップする（ユーザーの意図を優先）。
        let current = get_clipboard_inner().unwrap_or_default();
        if current == text {
            if let Err(e) = set_clipboard_inner(&saved) {
                log::warn!("Failed to restore clipboard after paste: {}", e);
            }
        } else if current != saved {
            log::debug!(
                "Clipboard changed externally after paste (was: ours, now: {:?}), restore skipped",
                current.chars().take(20).collect::<String>()
            );
        }
        true
    }
}

/// 選択中のテキストを指定テキストで差し替え、クリップボードを復元する（スレッドセーフ）。
/// 退避→セット→ペースト→復元の順で動作し、呼び出し後もクリップボードの内容を保持する。
///
/// 復元前にクリップボードの内容を確認し、外部プロセスによって変更されていた場合は
/// 復元をスキップする。
pub fn replace_selected_text(text: &str) -> Result<(), String> {
    let _lock = CLIPBOARD_LOCK.lock().unwrap();
    let saved = get_clipboard_inner().unwrap_or_default();

    set_clipboard_inner(text)?;
    KeyboardInjector::send_cmd_v();
    std::thread::sleep(std::time::Duration::from_millis(PASTE_DELAY_MS));

    let current = get_clipboard_inner().unwrap_or_default();
    if current == text {
        let _ = set_clipboard_inner(&saved);
    } else if current != saved {
        log::debug!(
            "Clipboard changed externally after replace (was: ours, now: {:?}), restore skipped",
            current.chars().take(20).collect::<String>()
        );
    }
    Ok(())
}
