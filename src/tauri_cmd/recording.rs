use crate::mode::cl::main_of_cl::TauriState;
use crate::mycute_manager::InputMode;
use crate::tools::audio;
use std::time::Duration;
use tauri::State;
use tokio::time::timeout;

#[tauri::command]
pub async fn start_recording(mode: InputMode, state: State<'_, TauriState>) -> Result<(), String> {
    let mut mgr = state.manager.lock();
    mgr.start_recording(mode);
    drop(mgr);

    #[cfg(target_os = "macos")]
    crate::hotkey_mac::set_recording_active(true);
    #[cfg(windows)]
    crate::hotkey_win::set_recording_active(true);

    Ok(())
}

#[tauri::command]
pub async fn stop_recording(state: State<'_, TauriState>) -> Result<(), String> {
    let mut mgr = state.manager.lock();
    mgr.stop_recording();
    drop(mgr);

    #[cfg(target_os = "macos")]
    crate::hotkey_mac::set_recording_active(false);
    #[cfg(windows)]
    crate::hotkey_win::set_recording_active(false);

    Ok(())
}

/// 録音を停止し、確定テキストを返す（オーケストレーター用）。
///
/// `stop_recording` と異なり、録音結果のテキストを戻り値として返す。
/// STTパイプラインの完了（FinalResult / 補正）を待ってからテキストを取得するため、
/// 後半だけが欠落する問題を防止する。
/// フロントエンドはこのテキストを `orchestrator_process` に渡す。
#[tauri::command]
pub async fn stop_orchestrator_recording(state: State<'_, TauriState>) -> Result<String, String> {
    let rx = {
        let mut mgr = state.manager.lock();
        mgr.request_flush()
    };

    // STTパイプラインの完了を非同期で待つ（安全のため10秒でタイムアウト）
    let text = timeout(Duration::from_secs(10), rx).await
        .map_err(|_| {
            log::warn!("Orchestrator flush timed out after 10 seconds");
            "STT flush timed out".to_string()
        })?
        .map_err(|_| {
            log::error!("Orchestrator flush channel closed without sending text");
            "Failed to receive flush text".to_string()
        })?;

    // Manager の状態を Idle に戻す。
    // request_flush() は recognizer.stop() を呼ぶが state を変更しないため、
    // 従来の音声入力（Option 2回）が state == Idle のガードでブロックされるのを防ぐ。
    {
        let mut mgr = state.manager.lock();
        mgr.stop_recording();
    }

    #[cfg(target_os = "macos")]
    crate::hotkey_mac::set_recording_active(false);
    #[cfg(windows)]
    crate::hotkey_win::set_recording_active(false);

    log::debug!("Orchestrator recording stopped, text length: {}", text.len());
    Ok(text)
}

/// 録音終了・コミット音を再生する。
#[tauri::command]
pub async fn play_commit_sound() -> Result<(), String> {
    audio::play_commit_sound();
    Ok(())
}
