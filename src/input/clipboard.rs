//! Clipboard operations using pbcopy/pbpaste.
//!
//! This module provides functions to interact with the system clipboard.

use std::process::Command;

/// Get the current clipboard content.
pub fn get_clipboard() -> Result<String, String> {
    let output = Command::new("pbpaste")
        .output()
        .map_err(|e| format!("Failed to run pbpaste: {}", e))?;

    if !output.status.success() {
        return Err("pbpaste failed".to_string());
    }

    String::from_utf8(output.stdout).map_err(|e| format!("Invalid UTF-8: {}", e))
}

/// Set the clipboard content.
pub fn set_clipboard(text: &str) -> Result<(), String> {
    use std::io::Write;
    use std::process::Stdio;

    let mut child = Command::new("pbcopy")
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to run pbcopy: {}", e))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(text.as_bytes())
            .map_err(|e| format!("Failed to write to pbcopy: {}", e))?;
    }

    child.wait().map_err(|e| format!("pbcopy failed: {}", e))?;

    Ok(())
}

/// Get selected text by sending Cmd+C and reading clipboard.
/// Returns the selected text, or empty string if nothing selected.
pub fn get_selected_text() -> Result<String, String> {
    use crate::input::keyboard::KeyboardInjector;

    // Save current clipboard
    let saved = get_clipboard().unwrap_or_default();

    // Clear clipboard
    set_clipboard("")?;

    // Send Cmd+C to copy selection
    KeyboardInjector::send_cmd_c();

    // Small delay to allow copy to complete
    std::thread::sleep(std::time::Duration::from_millis(50));

    // Read new clipboard content
    let selected = get_clipboard().unwrap_or_default();

    // Restore original clipboard if nothing was selected
    if selected.is_empty() {
        let _ = set_clipboard(&saved);
    }

    Ok(selected)
}

/// Replace selected text by writing to clipboard and sending Cmd+V.
pub fn replace_selected_text(text: &str) -> Result<(), String> {
    use crate::input::keyboard::KeyboardInjector;

    // Set clipboard to new text
    set_clipboard(text)?;

    // Send Cmd+V to paste
    KeyboardInjector::send_cmd_v();

    Ok(())
}
