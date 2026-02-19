use crate::stt_config::Settings;
use crate::mode::cl::main_of_cl::TauriState;
use tauri::State;

#[tauri::command]
pub async fn get_settings(state: State<'_, TauriState>) -> Result<Settings, String> {
    let settings = state.config_mgr.settings.read().clone();
    Ok(settings)
}

#[tauri::command]
pub async fn save_settings(
    settings: Settings,
    state: State<'_, TauriState>,
) -> Result<(), String> {
    // 1. Update in-memory settings and save to disk
    {
        let mut guard = state.config_mgr.settings.write();
        *guard = settings.clone();
    }
    if let Err(e) = state.config_mgr.save() {
        return Err(format!("Failed to save settings: {}", e));
    }

    // 2. Reflect to active manager/recognizer
    let mgr = state.manager.lock();
    let mut rec = mgr.recognizer.lock();
    let _ = rec.update_config(
        settings.stt_engine,
        settings.locale,
        Some(settings.stt),
        settings.llms
    );

    Ok(())
}
