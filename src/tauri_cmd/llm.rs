use crate::mode::cl::main_of_cl::TauriState;
use crate::types::LlmAction;
use tauri::State;

#[tauri::command]
pub async fn execute_llm(
    action: LlmAction,
    text: String,
    state: State<'_, TauriState>,
) -> Result<String, String> {
    let pool = &state.llm_pool;
    
    // Get language from manager state (currently locked)
    let locale = {
        let mgr = state.manager.lock();
        mgr.locale
    };
    
    pool.execute(action, &text, locale).await
        .map_err(|e| format!("LLM {} failed: {}", match action {
            LlmAction::Correct => "correction",
            LlmAction::Summarize => "summarization",
        }, e))
}
