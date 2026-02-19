use crate::mode::cl::main_of_cl::TauriState;
use crate::stt_config::LocaleCode;
use tauri::State;

#[tauri::command]
pub async fn set_locale(
    locale: LocaleCode,
    state: State<'_, TauriState>,
) -> Result<(), String> {
    let mut mgr = state.manager.lock();
    mgr.set_locale(locale);
    Ok(())
}
