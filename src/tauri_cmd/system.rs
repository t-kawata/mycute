use crate::constants::{
    APP_STATE_IDLE, APP_STATE_RECORDING, IP_LOCALHOST, PATH_HEALTH, SCHEME_PREFIX_HTTP,
    WINDOW_LABEL_MAIN,
};
use crate::hotkey::HotkeyMonitor;
use crate::input::clipboard;
use crate::mode::cl::main_of_cl::TauriState;
use crate::mycute_manager::{AppState as MgrAppState, InputMode};
use crate::tools::audio;
use crate::types::{AppStatePayload, HotkeyAction, TauriEvent};
use indexmap::IndexMap;
use std::sync::atomic::Ordering;
use tauri::{command, Emitter, Manager, State};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, TransactionTrait};
use crate::entities::prelude::*;
use crate::entities::settings::Column as SettingsColumn;
use crate::mycute_settings::Settings as MySettings;
use crate::stt::stats::UsageStats;
#[cfg(windows)]
use crate::stt::win as stt_win;
#[cfg(target_os = "macos")]
use crate::hotkey_mac;
#[cfg(windows)]
use crate::hotkey_win;

#[command]
pub async fn reset_application(state: State<'_, TauriState>, _app_handle: tauri::AppHandle) -> Result<(), String> {
    log::info!("Starting application reset process...");
    
    // 1. Delete all usage stats files
    if let Err(e) = UsageStats::delete_all_stats() {
        log::error!("Failed to delete usage stats: {}", e);
        // We continue even if stats deletion fails
    }

    let pools = {
        let pools_guard = state.config_mgr.db_pools.read();
        pools_guard.as_ref().cloned().ok_or("Database not initialized")?
    };
    let db = pools.get_rw().map_err(|e: anyhow::Error| e.to_string())?;
    
    do_reset_db(&db).await.map_err(|e| e.to_string())?;

    log::info!("Application reset completed successfully.");
    Ok(())
}

async fn do_reset_db(db: &sea_orm::DatabaseConnection) -> Result<(), sea_orm::DbErr> {
    db.transaction::<_, (), sea_orm::DbErr>(|txn| {
        Box::pin(async move {
            Apps::delete_many().exec(txn).await?;
            Badges::delete_many().exec(txn).await?;
            Bds::delete_many().exec(txn).await?;
            Belongs::delete_many().exec(txn).await?;
            Blacklists::delete_many().exec(txn).await?;
            BurnedKeys::delete_many().exec(txn).await?;
            CaVoteAllocatedSummaries::delete_many().exec(txn).await?;
            CaVoteItemSummaries::delete_many().exec(txn).await?;
            ChatModels::delete_many().exec(txn).await?;
            Cryptos::delete_many().exec(txn).await?;
            CubeContributors::delete_many().exec(txn).await?;
            CubeLineages::delete_many().exec(txn).await?;
            CubeModelStats::delete_many().exec(txn).await?;
            Cubes::delete_many().exec(txn).await?;
            Exports::delete_many().exec(txn).await?;
            Flushes::delete_many().exec(txn).await?;
            Forums::delete_many().exec(txn).await?;
            Identities::delete_many().exec(txn).await?;
            Jobs::delete_many().exec(txn).await?;
            MatchStatuses::delete_many().exec(txn).await?;
            Matches::delete_many().exec(txn).await?;
            Payments::delete_many().exec(txn).await?;
            Payouts::delete_many().exec(txn).await?;
            Points::delete_many().exec(txn).await?;
            Pools::delete_many().exec(txn).await?;
            ReplaceItems::delete_many().exec(txn).await?;
            Replaces::delete_many().exec(txn).await?;
            Settings::delete_many()
                .filter(SettingsColumn::Key.is_not_in(MySettings::protected_settings_keys()))
                .exec(txn).await?;
            Tickets::delete_many().exec(txn).await?;
            UsrBadges::delete_many().exec(txn).await?;
            Usrs::delete_many().exec(txn).await?;
            Verifications::delete_many().exec(txn).await?;
            Works::delete_many().exec(txn).await?;
            Ok(())
        })
    }).await.map_err(|e| match e {
        sea_orm::TransactionError::Connection(e) => e,
        sea_orm::TransactionError::Transaction(e) => e,
    })?;
    Ok(())
}
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
    let lmgw_client_for_hk = state.lmgw_client.clone();
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
                        #[cfg(windows)]
                        {
                            stt_win::disable_ime();
                        }
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
                    // 【解説】これは特定のホットキー（例：Option+Cなど）によるアクションではありません。
                    // プラットフォーム固有のホットキー実装（hotkey_mac.rs / hotkey_win.rs）において、
                    // 録音中に「登録済みホットキー以外」のキー入力やマウスクリックが検知された際、
                    // 自動的にこの Commit アクションが発行されるようになっています。
                    // つまり、「ユーザーが何か他の操作を始めたら、今の音声認識を確定させる」という自動トリガーの受取口です。
                    let mut mgr = manager_for_hk.lock();
                    if mgr.state == MgrAppState::Recording {
                        #[cfg(windows)]
                        {
                            stt_win::restore_ime();
                        }
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
                    // AI文章補正: 選択テキストを LMGW 経由で LLM により補正し、差し替える
                    log::debug!("Correcting selected text via LMGW...");
                    let selected = clipboard::get_selected_text().unwrap_or_default();
                    if selected.is_empty() {
                        log::error!("No text selected for correction");
                    } else {
                        let client = lmgw_client_for_hk.clone();
                        let config_mgr_inner = config_mgr_for_hk.clone();
                        let locale = manager_for_hk.lock().locale;
                        tokio::spawn(async move {
                            match client.correct_text(&selected, locale).await {
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
                                    } else {
                                        log::debug!("Text corrected successfully via LMGW.");
                                    }
                                }
                                Err(e) => {
                                    log::error!("Text correction via LMGW failed: {}", e);
                                }
                            }
                        });
                    }
                }
                HotkeyAction::Summarize => {
                    // AI要約: 選択テキストを LMGW 経由で LLM により要約し、差し替える
                    log::debug!("Summarizing selected text via LMGW...");
                    let selected = clipboard::get_selected_text().unwrap_or_default();
                    if selected.is_empty() {
                        log::error!("No text selected for summarization");
                    } else {
                        let client = lmgw_client_for_hk.clone();
                        let config_mgr_inner = config_mgr_for_hk.clone();
                        let locale = manager_for_hk.lock().locale;
                        tokio::spawn(async move {
                            match client.summarize_text(&selected, locale).await {
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
                                    } else {
                                        log::debug!("Text summarized successfully via LMGW.");
                                    }
                                }
                                Err(e) => {
                                    log::error!("Text summarization via LMGW failed: {}", e);
                                }
                            }
                        });
                    }
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
        hotkey_mac::stop_monitoring();
    }

    #[cfg(windows)]
    {
        hotkey_win::stop_monitoring();
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
