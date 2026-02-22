use crate::constants::*;
use crate::hotkey::HotkeyMonitor;
use crate::input::clipboard;
use crate::mode::cl::main_of_cl::TauriState;
use crate::mycute_manager::{AppState as MgrAppState, InputMode};
use crate::tools::audio;
use crate::types::{AppStatePayload, HotkeyAction, ShowSnackbarPayload, TauriEvent};
use indexmap::IndexMap;
use std::sync::atomic::Ordering;
use tauri::{command, Emitter, Manager, State};

#[command]
pub async fn check_server_health(state: State<'_, TauriState>) -> Result<bool, String> {
    let rt_port = state.config_mgr.settings.read().server.rt_port;
    // Health check url
    let url = format!(
        "{}{}:{}{}",
        SCHEME_PREFIX_HTTP, IP_LOCALHOST, rt_port, PATH_HEALTH
    );
    let client = reqwest::Client::new();

    match client.get(&url).send().await {
        Ok(res) => {
            if res.status().is_success() {
                Ok(true)
            } else {
                Ok(false)
            }
        }
        Err(_) => Ok(false),
    }
}
#[command]
pub async fn force_shutdown(app_handle: tauri::AppHandle) {
    log::error!("Force shutdown requested from frontend.");
    app_handle.exit(1);
}

#[command]
pub async fn enable_hotkey_standby(
    state: State<'_, TauriState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    // 二重起動防止
    if state
        .is_hotkey_active
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        log::debug!("Hotkey standby is already active.");
        return Ok(());
    }

    log::info!("Activating Hotkey Standby mode...");

    // HotkeyMonitor::start() がプラットフォームごとの詳細（再開か新規起動か）を内部で処理する
    let settings = state.config_mgr.settings.read();
    let hotkey_config = settings.hotkeys.clone();
    drop(settings);

    let hotkey_monitor = HotkeyMonitor::new(hotkey_config);
    let mut hk_rx = hotkey_monitor.start();

    let manager_for_hk = state.manager.clone();
    let handle_for_hk = app_handle.clone();
    let llm_pool_for_hk = state.llm_pool.clone();
    let config_mgr_for_hk = state.config_mgr.clone();

    // ホットキーハンドラループを生成する
    // NOTE: stop_monitoring() で送信側 (HOTKEY_SENDER) が破棄されると、
    // このループの recv() が None を返し、タスクは正常に終了する。

    tauri::async_runtime::spawn(async move {
        log::info!("Hotkey handler bridge loop started.");
        while let Some(action) = hk_rx.recv().await {
            match action {
                HotkeyAction::Start => {
                    let mut mgr = manager_for_hk.lock();
                    if mgr.state == MgrAppState::Idle {
                        mgr.start_recording(InputMode::RealTime);
                        let _ = handle_for_hk.emit(
                            TauriEvent::AppState.as_str(),
                            AppStatePayload {
                                state: APP_STATE_RECORDING.to_string(),
                            },
                        );
                    }
                }
                HotkeyAction::Commit => {
                    let mut mgr = manager_for_hk.lock();
                    if mgr.state == MgrAppState::Recording {
                        mgr.stop_recording();
                        // コミット音と同期してオーバーレイを消去
                        audio::play_commit_sound();
                        let _ = handle_for_hk.emit(TauriEvent::SttCommit.as_str(), ());
                        let _ = handle_for_hk.emit(
                            TauriEvent::AppState.as_str(),
                            AppStatePayload {
                                state: APP_STATE_IDLE.to_string(),
                            },
                        );
                    }
                }
                HotkeyAction::Correct => {
                    // AI文章補正: 選択テキストをLLMで補正し、差し替える
                    if llm_pool_for_hk.is_empty() {
                        log::error!("No LLM endpoints configured in settings.json");
                    } else {
                        log::debug!("Correcting selected text...");
                        let selected = clipboard::get_selected_text().unwrap_or_default();
                        if selected.is_empty() {
                            log::error!("No text selected for correction");
                        } else {
                            let pool = llm_pool_for_hk.clone();
                            let config_mgr_inner = config_mgr_for_hk.clone();
                            let locale = manager_for_hk.lock().locale;
                            let handle_inner = handle_for_hk.clone();
                            let _ = handle_for_hk.emit(
                                TauriEvent::ShowSnackbar.as_str(),
                                ShowSnackbarPayload {
                                    message: MSG_CORRECTING.to_string(),
                                },
                            );
                            tokio::spawn(async move {
                                match pool.correct_text(&selected, locale).await {
                                    Ok(corrected) => {
                                        // ユーザー定義の置換ルールをAI出力にも適用
                                        let replaces = config_mgr_inner.replaces.read().clone();
                                        let final_text = apply_replaces(&corrected, &replaces);
                                        if let Err(e) =
                                            clipboard::replace_selected_text(&final_text)
                                        {
                                            log::error!(
                                                "Failed to replace text after correction: {}",
                                                e
                                            );
                                            let _ = handle_inner.emit(
                                                TauriEvent::ShowSnackbar.as_str(),
                                                ShowSnackbarPayload {
                                                    message: MSG_CORRECTION_FAILED.to_string(),
                                                },
                                            );
                                        } else {
                                            log::debug!("Text corrected successfully");
                                            let _ = handle_inner.emit(
                                                TauriEvent::ShowSnackbar.as_str(),
                                                ShowSnackbarPayload {
                                                    message: MSG_CORRECTED.to_string(),
                                                },
                                            );
                                        }
                                    }
                                    Err(e) => {
                                        log::error!("Text correction failed: {}", e);
                                        let _ = handle_inner.emit(
                                            TauriEvent::ShowSnackbar.as_str(),
                                            ShowSnackbarPayload {
                                                message: MSG_CORRECTION_FAILED.to_string(),
                                            },
                                        );
                                    }
                                }
                            });
                        }
                    }
                }
                HotkeyAction::Summarize => {
                    // AI要約: 選択テキストをLLMで要約し、差し替える
                    if llm_pool_for_hk.is_empty() {
                        log::error!("No LLM endpoints configured in settings.json");
                    } else {
                        log::debug!("Summarizing selected text...");
                        let selected = clipboard::get_selected_text().unwrap_or_default();
                        if selected.is_empty() {
                            log::error!("No text selected for summarization");
                        } else {
                            let pool = llm_pool_for_hk.clone();
                            let config_mgr_inner = config_mgr_for_hk.clone();
                            let locale = manager_for_hk.lock().locale;
                            let handle_inner = handle_for_hk.clone();
                            let _ = handle_for_hk.emit(
                                TauriEvent::ShowSnackbar.as_str(),
                                ShowSnackbarPayload {
                                    message: MSG_SUMMARIZING.to_string(),
                                },
                            );
                            tokio::spawn(async move {
                                match pool.summarize_text(&selected, locale).await {
                                    Ok(summarized) => {
                                        // ユーザー定義の置換ルールをAI出力にも適用
                                        let replaces = config_mgr_inner.replaces.read().clone();
                                        let final_text = apply_replaces(&summarized, &replaces);
                                        if let Err(e) =
                                            clipboard::replace_selected_text(&final_text)
                                        {
                                            log::error!(
                                                "Failed to replace text after summarization: {}",
                                                e
                                            );
                                            let _ = handle_inner.emit(
                                                TauriEvent::ShowSnackbar.as_str(),
                                                ShowSnackbarPayload {
                                                    message: MSG_SUMMARIZATION_FAILED.to_string(),
                                                },
                                            );
                                        } else {
                                            log::debug!("Text summarized successfully");
                                            let _ = handle_inner.emit(
                                                TauriEvent::ShowSnackbar.as_str(),
                                                ShowSnackbarPayload {
                                                    message: MSG_SUMMARIZED.to_string(),
                                                },
                                            );
                                        }
                                    }
                                    Err(e) => {
                                        log::error!("Text summarization failed: {}", e);
                                        let _ = handle_inner.emit(
                                            TauriEvent::ShowSnackbar.as_str(),
                                            ShowSnackbarPayload {
                                                message: MSG_SUMMARIZATION_FAILED.to_string(),
                                            },
                                        );
                                    }
                                }
                            });
                        }
                    }
                }
                HotkeyAction::ToggleLocale => {
                    // 言語切替: 日本語⇔英語のトグル
                    let mut mgr = manager_for_hk.lock();
                    let new_locale = if mgr.locale == crate::stt_config::LocaleCode::Ja {
                        crate::stt_config::LocaleCode::En
                    } else {
                        crate::stt_config::LocaleCode::Ja
                    };
                    config_mgr_for_hk.settings.write().locale = new_locale;
                    mgr.set_locale(new_locale);
                    let msg = format!("{}{}", MSG_PREFIX_LANGUAGE, new_locale.display_name());
                    log::debug!("{}", msg);
                    let _ = handle_for_hk.emit(
                        TauriEvent::ShowSnackbar.as_str(),
                        ShowSnackbarPayload {
                            message: msg.clone(),
                        },
                    );
                    let _ = handle_for_hk.emit(
                        TauriEvent::AppState.as_str(),
                        AppStatePayload { state: msg },
                    );
                }
                _ => {
                    log::debug!("Hotkey received but unhandled in cl mode: {:?}", action);
                }
            }
        }
        log::info!("Hotkey handler bridge loop ended.");
    });

    Ok(())
}

#[command]
pub async fn toggle_always_on_top(
    app_handle: tauri::AppHandle,
    always_on_top: bool,
) -> Result<(), String> {
    log::debug!("<Window> Toggling Always on Top: {}", always_on_top);
    if let Some(window) = app_handle.get_webview_window(WINDOW_LABEL_MAIN) {
        window
            .set_always_on_top(always_on_top)
            .map_err(|e: tauri::Error| {
                log::error!("Failed to set always on top: {}", e);
                e.to_string()
            })?;
        Ok(())
    } else {
        Err("Main window not found".to_string())
    }
}

#[command]
pub async fn disable_hotkey_standby(state: State<'_, TauriState>) -> Result<(), String> {
    // アクティブ時のみ無効化処理を行う
    if state
        .is_hotkey_active
        .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        log::debug!("Hotkey standby is not active.");
        return Ok(());
    }

    log::info!("Disabling Hotkey Standby mode...");

    #[cfg(target_os = "macos")]
    {
        crate::hotkey_mac::stop_monitoring();
    }

    #[cfg(windows)]
    {
        crate::hotkey_win::stop_monitoring();
    }

    Ok(())
}

/// VPモードと同一のテキスト置換ロジック（最長一致優先）
/// settings.json の replaces マップ（置換後 -> [置換前の候補]）を適用する
fn apply_replaces(text: &str, replaces_map: &IndexMap<String, Vec<String>>) -> String {
    // "置換前" 文字列の長い順にソート（最長一致優先）するためにフラット化してソート
    let mut flat_replaces: Vec<(&str, &str)> = Vec::new();
    for (after, befores) in replaces_map {
        for before in befores {
            flat_replaces.push((before, after));
        }
    }
    flat_replaces.sort_by(|a, b| b.0.len().cmp(&a.0.len()));

    let mut result = text.to_string();
    for (from, to) in flat_replaces {
        if !from.is_empty() {
            result = result.replace(from, to);
        }
    }
    result
}
