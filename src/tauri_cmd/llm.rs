use crate::mode::cl::main_of_cl::TauriState;
use crate::types::LlmAction;
use tauri::State;

#[tauri::command]
pub async fn execute_llm(
    action: LlmAction,
    text: String,
    state: State<'_, TauriState>,
) -> Result<String, String> {
    let client = &state.lmgw_client;

    // 言語はマネージャーの状態から取得する（ロック中）
    let locale = {
        let mgr = state.manager.lock();
        mgr.locale
    };

    client.execute(action, &text, locale).await.map_err(|e| {
        format!(
            "LLM {} failed: {}",
            match action {
                LlmAction::Correct => "correction",
                LlmAction::Summarize => "summarization",
            },
            e
        )
    })
}
