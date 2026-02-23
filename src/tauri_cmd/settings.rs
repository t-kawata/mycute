use crate::mode::cl::main_of_cl::TauriState;
use crate::stt_config::Settings;
use tauri::State;

#[tauri::command]
pub async fn get_settings(state: State<'_, TauriState>) -> Result<Settings, String> {
    let settings = state.config_mgr.settings.read().clone();
    Ok(settings)
}

#[tauri::command]
pub async fn save_settings(settings: Settings, state: State<'_, TauriState>) -> Result<(), String> {
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
    let stt_engine = settings.stt_engine.clone();
    let current_locale = settings.locale.clone();
    let stt_settings = settings.stt.clone();
    let llm_endpoints = settings.llms.clone();
    let _ = rec.update_config(
        stt_engine,
        current_locale,
        Some(stt_settings),
        llm_endpoints,
    );

    // 3. Update global LLM pool
    state.llm_pool.update_endpoints(&settings.llms);

    Ok(())
}
