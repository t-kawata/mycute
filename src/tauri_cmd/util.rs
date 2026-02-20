use crate::input::clipboard;
use crate::mode::cl::main_of_cl::TauriState;
use crate::stt::stats::UsageStats;
use crate::types::LogLevel;
use tauri::State;

#[tauri::command]
pub async fn get_usage_stats() -> Result<String, String> {
    match UsageStats::get_summary() {
        Ok(summary) => Ok(summary),
        Err(e) => Err(format!("Failed to get usage stats: {}", e)),
    }
}

#[tauri::command]
pub async fn get_hotkey_list(
    state: State<'_, TauriState>,
) -> Result<Vec<(String, Vec<String>)>, String> {
    let settings = state.config_mgr.settings.read();
    let hk = &settings.hotkeys;

    Ok(vec![
        ("Start/Stop".to_string(), hk.start.clone()),
        ("Correction".to_string(), hk.correct.clone()),
        ("Summarize".to_string(), hk.summarize.clone()),
        ("Toggle Locale".to_string(), hk.toggle_locale.clone()),
        ("Buffer Start".to_string(), hk.buffer_start.clone()),
        ("Buffer Flush".to_string(), hk.buffer_flush.clone()),
        ("Settings".to_string(), hk.settings.clone()),
        ("Help".to_string(), hk.help.clone()),
        ("Usage Stats".to_string(), hk.usage_stats.clone()),
    ])
}

#[tauri::command]
pub async fn log_from_frontend(level: LogLevel, message: String) -> Result<(), String> {
    match level {
        LogLevel::Error => log::error!(target: "frontend", "{}", message),
        LogLevel::Warn => log::warn!(target: "frontend", "{}", message),
        LogLevel::Info => log::info!(target: "frontend", "{}", message),
        LogLevel::Debug => log::debug!(target: "frontend", "{}", message),
        LogLevel::Trace => log::trace!(target: "frontend", "{}", message),
    }
    Ok(())
}

#[tauri::command]
pub async fn set_clipboard(text: String) -> Result<(), String> {
    clipboard::set_clipboard(&text)
}

#[tauri::command]
pub async fn get_clipboard() -> Result<String, String> {
    clipboard::get_clipboard()
}
