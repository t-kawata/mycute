use crate::mode::cl::main_of_cl::TauriState;
use crate::mycute_manager::InputMode;
use tauri::State;

#[tauri::command]
pub async fn start_recording(
    mode: InputMode,
    state: State<'_, TauriState>,
) -> Result<(), String> {
    let mut mgr = state.manager.lock();
    mgr.start_recording(mode);
    Ok(())
}

#[tauri::command]
pub async fn stop_recording(
    state: State<'_, TauriState>,
) -> Result<(), String> {
    let mut mgr = state.manager.lock();
    mgr.stop_recording();
    Ok(())
}
