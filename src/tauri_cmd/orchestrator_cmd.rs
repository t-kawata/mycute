/// Orchestrator 操作用 Tauri コマンド。
///
/// フロントエンドの OrchestratorOverlay から呼び出される:
/// - `create_orchestrator_session`: MockOrchestrator を初期化し、状態を TauriState に保持する
/// - `orchestrator_process`: テキストをオーケストレーターに送信し、応答を返す
use crate::mode::cl::main_of_cl::TauriState;
use crate::orchestrator::mock::MockOrchestrator;
use crate::orchestrator::{OrchestratorError, OrchestratorInput, OrchestratorOutput};
use crate::types::TauriEvent;
use tauri::{command, AppHandle, Emitter, State};

/// オーケストレーターセッションを開始する。
///
/// MockOrchestrator を生成し、TauriState に保持する。
/// フロントエンドはオーバーレイ表示時にこのコマンドを呼び出す。
#[command]
pub async fn create_orchestrator_session(
    state: State<'_, TauriState>,
) -> Result<(), String> {
    let mut guard = state.orchestrator.lock();
    *guard = Some(Box::new(MockOrchestrator::new()));
    log::debug!("Orchestrator session created");
    Ok(())
}

/// テキストをオーケストレーターに送信し、応答を返す。
///
/// フロントエンドは音声認識結果の確定テキストをこのコマンドに渡す。
/// 戻り値の `OrchestratorOutput` は frontend で表示される。
#[command]
pub async fn orchestrator_process(
    state: State<'_, TauriState>,
    app_handle: AppHandle,
    text: String,
    session_id: String,
) -> Result<OrchestratorOutput, String> {
    let input = OrchestratorInput {
        raw_text: text,
        session_id,
    };

    // orchestrator を Mutex から取り出し、ロックを解放してから process() を呼び出す
    // （parking_lot::MutexGuard は Send ではないため、await を跨げない）
    let mut orch = {
        let mut guard = state.orchestrator.lock();
        guard.take().ok_or_else(|| {
            "Orchestrator not initialized. Call create_orchestrator_session first.".to_string()
        })?
    };

    let output = orch.process(&input).await.map_err(|e| match e {
        OrchestratorError::EmptyInput => "Input text is empty".to_string(),
        OrchestratorError::PipelineFailed(msg) => format!("Pipeline failed: {}", msg),
        OrchestratorError::Internal(msg) => format!("Internal error: {}", msg),
    })?;

    // 処理済みの orchestrator を状態に戻す
    state.orchestrator.lock().replace(orch);

    // 応答イベントをフロントエンドに送信する
    let _ = app_handle.emit(
        TauriEvent::OrchestratorResponse.as_str(),
        &output.response_text,
    );

    if output.task_completed {
        let _ = app_handle.emit(
            TauriEvent::OrchestratorTaskCompleted.as_str(),
            true,
        );
    }

    Ok(output)
}

/// オーケストレーターセッションを破棄する。
///
/// フロントエンドはオーバーレイクローズ時にこのコマンドを呼び出す。
#[command]
pub async fn destroy_orchestrator_session(
    state: State<'_, TauriState>,
) -> Result<(), String> {
    let mut guard = state.orchestrator.lock();
    *guard = None;
    log::debug!("Orchestrator session destroyed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orchestrator_error_conversion() {
        let err = OrchestratorError::EmptyInput;
        assert_eq!(
            match err {
                OrchestratorError::EmptyInput => "Input text is empty".to_string(),
                _ => unreachable!(),
            },
            "Input text is empty"
        );
    }
}
