//! クリップボード操作モジュール（クロスプラットフォーム対応）
//!
//! `arboard` クレートを使用し、Mac / Windows の両方でクリップボードを操作します。
//! 従来の `pbcopy` / `pbpaste` コマンド依存を排除し、ネイティブAPIを直接利用します。

use arboard::{Clipboard, Error as ArboardError};
use crate::input::keyboard::KeyboardInjector;

/// 現在のクリップボード内容を取得する。
/// クリップボードが空またはテキスト以外の場合は空文字列を返す。
pub fn get_clipboard() -> Result<String, String> {
    let mut clip = Clipboard::new().map_err(|e| format!("Failed to open clipboard: {}", e))?;
    // テキスト取得に失敗した場合（画像等の非テキストデータ）は空文字を返す
    match clip.get_text() {
        Ok(text) => Ok(text),
        Err(ArboardError::ContentNotAvailable) => Ok(String::new()),
        Err(e) => Err(format!("Failed to get clipboard text: {}", e)),
    }
}

/// クリップボードにテキストを設定する。
pub fn set_clipboard(text: &str) -> Result<(), String> {
    let mut clip = Clipboard::new().map_err(|e| format!("Failed to open clipboard: {}", e))?;
    clip.set_text(text).map_err(|e| format!("Failed to set clipboard text: {}", e))
}

/// 選択中のテキストを Cmd+C / Ctrl+C で取得する。
/// 選択されていなければ空文字列を返す。
pub fn get_selected_text() -> Result<String, String> {
    // 現在のクリップボード内容を退避
    let saved = get_clipboard().unwrap_or_default();

    // クリップボードをクリア
    set_clipboard("")?;

    // Cmd+C / Ctrl+C を送信してコピー
    KeyboardInjector::send_cmd_c();

    // コピー処理がOSに反映されるまでわずかに待機
    std::thread::sleep(std::time::Duration::from_millis(50));

    // 新しいクリップボード内容（＝選択されていたテキスト）を取得
    let selected = get_clipboard().unwrap_or_default();

    // 何も選択されていなかった場合はクリップボードを元に戻す
    if selected.is_empty() {
        let _ = set_clipboard(&saved);
    }

    Ok(selected)
}

/// 選択中のテキストを差し替える（Cmd+V / Ctrl+V）。
pub fn replace_selected_text(text: &str) -> Result<(), String> {
    // クリップボードに差し替えたいテキストをセット
    set_clipboard(text)?;

    // Cmd+V / Ctrl+V で貼り付け
    KeyboardInjector::send_cmd_v();

    Ok(())
}
