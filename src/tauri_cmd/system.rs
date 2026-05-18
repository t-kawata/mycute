use crate::constants::{
    APP_STATE_IDLE, APP_STATE_RECORDING, IP_LOCALHOST, PATH_HEALTH, SCHEME_PREFIX_HTTP,
    WINDOW_LABEL_MAIN,
};
use crate::hotkey::HotkeyMonitor;
use crate::input::clipboard;
use crate::mode::cl::main_of_cl::TauriState;
use crate::mycute_manager::{AppState as MgrAppState, InputMode};
use crate::tools::audio;
use crate::types::{AppOverlayVisibilityPayload, AppStatePayload, HotkeyAction, TauriEvent};
#[cfg(windows)]
use crate::types::WindowsHealthPayload;
use indexmap::IndexMap;
use std::path::Path;
use std::sync::atomic::Ordering;
use tauri::{command, Emitter, Manager, State};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, TransactionTrait};
use crate::entities::prelude::*;
use crate::entities::settings::Column as SettingsColumn;
use crate::mycute_settings::Settings as MySettings;
use crate::stt::stats::UsageStats;
use serde_json::json;
#[cfg(windows)]
use crate::stt::win as stt_win;
#[cfg(target_os = "macos")]
use crate::hotkey_mac;
#[cfg(windows)]
use crate::hotkey_win;
#[cfg(windows)]
use crate::hotkey_win_hook;
#[cfg(windows)]
use crate::utils::process::{
    kill_process_by_image_name, wait_for_process_exit,
};
use std::time::Duration;

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

    // ★ Bifrost クリーンアップのためにプロバイダー名を DB 削除前に保存する
    let provider_names: Vec<String> = LmgwProviders::find()
        .all(db)
        .await
        .map_err(|e| format!("Failed to read providers before reset: {}", e))?
        .into_iter()
        .map(|p| p.provider_name)
        .collect();

    let provider_count = provider_names.len();
    if provider_count > 0 {
        log::info!("Found {} provider(s) to clean up from Bifrost after reset.", provider_count);
    }

    do_reset_db(&db).await.map_err(|e| e.to_string())?;

    // ★ Bifrost 側のプロバイダー設定も削除する（ベストエフォート）
    let mut bifrost_cleanup_ok = true;
    if provider_count > 0 {
        let rt_port = state.config_mgr.settings.read().server.rt_port;
        let url = format!("http://{IP_LOCALHOST}:{rt_port}/api/v1/internal/bifrost/clear-providers");
        let body = json!({ "provider_names": provider_names });

        match state.hc.post(&url).json(&body).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    log::info!("Bifrost providers cleaned up after reset.");
                } else {
                    bifrost_cleanup_ok = false;
                    log::warn!(
                        "Bifrost cleanup returned non-success status: {}",
                        resp.status()
                    );
                }
            }
            Err(e) => {
                bifrost_cleanup_ok = false;
                log::warn!("Failed to request Bifrost cleanup after reset: {}", e);
            }
        }
    }

    if bifrost_cleanup_ok {
        log::info!("Application reset completed successfully.");
    } else {
        log::warn!("Application reset completed (DB cleared, but Bifrost cleanup failed — leftover provider configs may remain in Bifrost).");
    }
    Ok(())
}

async fn do_reset_db(db: &sea_orm::DatabaseConnection) -> Result<(), sea_orm::DbErr> {
    db.transaction::<_, (), sea_orm::DbErr>(|txn| {
        Box::pin(async move {
            Apps::delete_many().exec(txn).await?;
            Bds::delete_many().exec(txn).await?;
            Blacklists::delete_many().exec(txn).await?;
            CaVoteAllocatedSummaries::delete_many().exec(txn).await?;
            CaVoteItemSummaries::delete_many().exec(txn).await?;
            LmgwProviders::delete_many().exec(txn).await?;
            Cryptos::delete_many().exec(txn).await?;
            CubeContributors::delete_many().exec(txn).await?;
            CubeLineages::delete_many().exec(txn).await?;
            CubeModelStats::delete_many().exec(txn).await?;
            Cubes::delete_many().exec(txn).await?;
            Exports::delete_many().exec(txn).await?;
            Forums::delete_many().exec(txn).await?;
            Identities::delete_many().exec(txn).await?;
            ReplaceItems::delete_many().exec(txn).await?;
            Replaces::delete_many().exec(txn).await?;
            Settings::delete_many()
                .filter(SettingsColumn::Key.is_not_in(MySettings::protected_settings_keys()))
                .exec(txn).await?;
            SttHistories::delete_many().exec(txn).await?;
            Tickets::delete_many().exec(txn).await?;
            Usrs::delete_many().exec(txn).await?;
            Verifications::delete_many().exec(txn).await?;
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

    // WH_KEYBOARD_LL フックを起動し、MYCUTE ホットキーの OS/他アプリへの到達を阻止する
    #[cfg(windows)]
    if let Err(e) = hotkey_win_hook::start_hook() {
        log::error!(
            "Critical: WH_KEYBOARD_LL hook failed to install: {}. \
             Alt blocking will be unavailable.",
            e
        );
    }

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
                            // 未確認の設定不足があればフロントエンドに通知（ダイアログは非ブロッキング）
                            let health = crate::stt::win::get_health_check_result();
                            if health != 0 && !crate::stt::win::is_health_check_acknowledged() {
                                let _ = handle_for_hk.emit(
                                    TauriEvent::WinHealthCheck.as_str(),
                                    WindowsHealthPayload { issues: health },
                                );
                            }
                            stt_win::disable_ime();
                        }
                        mgr.start_recording(InputMode::Buffered);
                        #[cfg(target_os = "macos")]
                        hotkey_mac::set_recording_active(true);
                        #[cfg(windows)]
                        hotkey_win::set_recording_active(true);
                        let _ = handle_for_hk.emit(
                            TauriEvent::AppState.as_str(),
                            AppStatePayload {
                                state: APP_STATE_RECORDING.to_string(),
                            },
                        );
                        // Option/Alt 2回押し: ウィンドウを最前面に表示し、オーバーレイを開く
                        // pin_during_voice が true の場合のみ最前面固定する
                        if let Some(window) = handle_for_hk.get_webview_window(WINDOW_LABEL_MAIN) {
                            let _ = window.show();
                            let _ = window.unminimize();
                            if mgr.pin_during_voice {
                                let _ = window.set_always_on_top(true);
                            }
                        }
                        let _ = handle_for_hk.emit(
                            TauriEvent::AppOverlayVisibility.as_str(),
                            AppOverlayVisibilityPayload { visible: true },
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
                HotkeyAction::BufferFlush => {
                    let should_flush_now = {
                        let mut mgr = manager_for_hk.lock();
                        if mgr.is_post_correcting || mgr.is_stt_pending {
                            mgr.pending_flush = true;
                            false
                        } else {
                            true
                        }
                    };

                    if should_flush_now {
                        let flush_text = {
                            let mgr = manager_for_hk.lock();
                            mgr.build_flush_text()
                        };
                        if !flush_text.is_empty() {
                            clipboard::save_paste_and_restore(&flush_text);
                        }
                        let mut mgr = manager_for_hk.lock();
                        if mgr.state == MgrAppState::Recording {
                            #[cfg(windows)]
                            {
                                stt_win::restore_ime();
                            }
                            mgr.stop_recording();
                            audio::play_commit_sound();
                            let _ = handle_for_hk.emit(TauriEvent::SttCommit.as_str(), ());
                            let _ = handle_for_hk.emit(
                                TauriEvent::AppState.as_str(),
                                AppStatePayload {
                                    state: APP_STATE_IDLE.to_string(),
                                },
                            );
                        }
                        #[cfg(target_os = "macos")]
                        hotkey_mac::set_recording_active(false);
                        #[cfg(windows)]
                        hotkey_win::set_recording_active(false);
                        // Flush: オーバーレイを閉じ、最前面固定を解除（pin_during_voice が有効だった場合のみ）
                        let _ = handle_for_hk.emit(
                            TauriEvent::AppOverlayVisibility.as_str(),
                            AppOverlayVisibilityPayload { visible: false },
                        );
                        if let Some(window) = handle_for_hk.get_webview_window(WINDOW_LABEL_MAIN) {
                            if mgr.pin_during_voice {
                                let _ = window.set_always_on_top(false);
                            }
                        }
                    }
                }
            }
        }
        log::info!("Hotkey handler bridge loop ended.");
    });

    // WH_KEYBOARD_LL フックの定期健全性チェック（15秒間隔）
    #[cfg(windows)]
    {
        tauri::async_runtime::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(15));
            // 最初の即時 tick を消費し、次の tick から 15 秒間隔にする
            interval.tick().await;
            loop {
                interval.tick().await;
                hotkey_win_hook::check_hook_health();
            }
        });
    }

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
pub async fn get_pin_during_voice(state: State<'_, TauriState>) -> Result<bool, String> {
    Ok(state.manager.lock().pin_during_voice)
}

#[command]
pub async fn set_pin_during_voice(
    state: State<'_, TauriState>,
    pin: bool,
) -> Result<(), String> {
    state.manager.lock().pin_during_voice = pin;
    log::info!("<Voice> Pin during voice input: {}", pin);
    Ok(())
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
        hotkey_win_hook::stop_hook();
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

/// 再起動ハンドオフの事前準備を行う Tauri コマンド。
///
/// 1. `BackendProcessGuard` をデタッチして Drop 時の kill を抑止する
/// 2. ガードを `restart_guard_holder` に退避する
/// 3. RT に再起動モードへの移行を HTTP 通知する（ベストエフォート）
///
/// フロントエンドの `restartMycute()` → `relaunch()` の直前に呼ばれる。
#[command]
pub async fn prepare_restart(state: State<'_, TauriState>) -> Result<(), String> {
    // 1. BackendProcessGuard をデタッチして退避
    let mut guard = state
        .backend_guard
        .lock()
        .take()
        .ok_or_else(|| "No backend process to prepare for restart.".to_string())?;

    let pid = guard.pid;
    guard.detach();
    *state.restart_guard_holder.lock() = Some(guard);

    log::info!("Restart handover: BackendProcessGuard (PID: {}) detached and stored.", pid);

    // 2. RT に再起動モードへの移行を通知 (ベストエフォート)
    let rt_port = state.config_mgr.settings.read().server.rt_port;
    let url = format!("http://{IP_LOCALHOST}:{rt_port}/api/v1/internal/prepare-restart");
    match state.hc.post(&url).send().await {
        Ok(resp) => {
            if resp.status().is_success() {
                log::info!("Restart handover: RT notified successfully.");
            } else {
                log::warn!(
                    "Restart handover: RT returned non-success status: {}",
                    resp.status()
                );
            }
        }
        Err(e) => {
            // RT が応答しなくても、relaunch 後の新 CL が既存 RT を発見して
            // manage_backend_server で引き継ぐ通常パスで動作継続可能
            log::warn!(
                "Restart handover: Failed to notify RT (will auto-detect): {}",
                e
            );
        }
    }

    Ok(())
}

/// 再起動ハンドオフを中断する Tauri コマンド。
///
/// `prepare_restart` の呼び出し後に `relaunch()` が失敗した場合、
/// フロントエンドがこのコマンドを呼び出して状態を復元する。
#[command]
pub async fn abort_restart(state: State<'_, TauriState>) -> Result<(), String> {
    // 1. 退避したガードを復元
    let mut guard = state
        .restart_guard_holder
        .lock()
        .take()
        .ok_or_else(|| "No guard in restart holder to abort.".to_string())?;

    let pid = guard.pid;
    guard.reattach();
    *state.backend_guard.lock() = Some(guard);

    log::info!("Restart handover aborted: BackendProcessGuard (PID: {}) restored.", pid);

    // 2. RT に再起動モード解除を通知 (ベストエフォート)
    let rt_port = state.config_mgr.settings.read().server.rt_port;
    let url = format!("http://{IP_LOCALHOST}:{rt_port}/api/v1/internal/abort-restart");
    let _ = state.hc.post(&url).send().await;

    Ok(())
}

/// 指定されたディレクトリをリトライ＋OSフォールバックで削除する。
///
/// 1. `std::fs::remove_dir_all` を遅延 [0, 200, 500, 1000]ms で最大4回リトライ
/// 2. 全失敗時は OS の削除コマンドにフォールバック:
///    - Unix: `rm -rf --`
///    - Windows: `cmd /c rmdir /s /q`
/// 3. 削除完了を確認し、残存していればエラーを返す
///
/// 存在しないパスに対しては Ok(()) を返す（べき等）。
async fn delete_home_directory(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }

    // macOS では rmdir がカレントディレクトリに対して EBUSY を返すため回避する
    // Windows でも同様にカレントディレクトリにロックがかかる場合がある
    let _ = std::env::set_current_dir(std::env::temp_dir());

    // リトライ遅延: 過渡的なファイルロック競合を避けるための待機時間
    const DELETE_RETRY_DELAYS_MS: &[u64] = &[0, 200, 500, 1000];
    let mut last_error: Option<std::io::Error> = None;

    for (attempt, &delay_ms) in DELETE_RETRY_DELAYS_MS.iter().enumerate() {
        if delay_ms > 0 {
            tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
        }
        match std::fs::remove_dir_all(path) {
            Ok(()) => {
                last_error = None;
                break;
            }
            Err(e) => {
                log::warn!(
                    "Attempt {} to delete directory {:?} failed: {}",
                    attempt + 1,
                    path,
                    e,
                );
                last_error = Some(e);
            }
        }
    }

    // remove_dir_all が全て失敗した場合、フォールバックとして OS の削除コマンドを試行
    if let Some(ref _err) = last_error {
        #[cfg(unix)]
        {
            log::warn!("All remove_dir_all attempts failed. Trying rm -rf fallback...");
            match std::process::Command::new("rm")
                .arg("-rf")
                .arg("--")
                .arg(path)
                .status()
            {
                Ok(status) if status.success() => {
                    last_error = None;
                    log::info!("rm -rf fallback succeeded.");
                }
                Ok(status) => {
                    log::error!("rm -rf fallback failed with exit code: {:?}", status.code());
                }
                Err(rm_err) => {
                    log::error!("rm -rf fallback error: {}", rm_err);
                }
            }
        }
        #[cfg(windows)]
        {
            log::warn!("All remove_dir_all attempts failed. Trying rmdir fallback...");
            match std::process::Command::new("cmd")
                .args(["/c", "rmdir", "/s", "/q"])
                .arg(path)
                .status()
            {
                Ok(status) if status.success() => {
                    last_error = None;
                    log::info!("rmdir fallback succeeded.");
                }
                Ok(status) => {
                    log::error!("rmdir fallback failed with exit code: {:?}", status.code());
                }
                Err(rm_err) => {
                    log::error!("rmdir fallback error: {}", rm_err);
                }
            }
        }
    }

    // 最終的に削除できなかった場合はエラーを返す
    if let Some(e) = last_error {
        return Err(format!(
            "Failed to delete directory {:?} after all attempts: {}",
            path, e
        ));
    }

    // 削除完了の確認
    if path.exists() {
        return Err(format!(
            "Directory {:?} still exists after all removal attempts.",
            path
        ));
    }

    log::info!("Directory {:?} deleted successfully.", path);
    Ok(())
}

/// Windows: アプリロックファイルを解放してからディレクトリを削除する。
///
/// macOS では開いているファイルの削除が可能であるため不要だが、
/// Windows ではファイルハンドルが削除を妨げるため明示的に解放する必要がある。
#[cfg(windows)]
async fn release_and_delete(path: &Path) -> Result<(), String> {
    crate::utils::singleton::release_lock();
    log::info!("Singleton lock released.");
    delete_home_directory(path).await
}

/// Windows: MYCUTE_HOME 削除用コマンドをクリップボードにコピーする。
///
/// アプリ終了後にユーザーが手動で実行する rmdir コマンドを生成し、
/// クリップボードにセットする。コマンド文字列も戻り値として返す。
/// フロントエンドが Windows かどうかを判定してこのコマンドを呼ぶ想定。
#[command]
pub async fn copy_cleanup_cmd_for_windows(
    state: State<'_, TauriState>,
) -> Result<String, String> {
    let home_dir = &state.config_mgr.home_dir;
    let path_str = home_dir
        .to_str()
        .ok_or_else(|| "Invalid UTF-8 in home directory path".to_string())?;
    let cmd = format!("rmdir /s /q \"{}\"", path_str);
    clipboard::set_clipboard(&cmd)
        .map_err(|e| format!("Failed to copy to clipboard: {}", e))?;
    log::info!("Cleanup command copied to clipboard: {}", cmd);
    Ok(cmd)
}

/// 完全初期化して終了: DBリセット → 全バックエンド終了 → MYCUTE_HOME削除 → アプリ終了
///
/// 以下の順序で安全に実行する:
/// 1. DB の論理リセット（do_reset_db）
/// 2. Bifrost プロバイダーのクリーンアップ
/// 3. DB コネクションプールの明示的解放
/// 4. シャットダウンフラグをセット（watchdog 競合抑制）
/// 5. バックエンドプロセス kill (BackendProcessGuard Drop)
/// 6. 全バックエンドポートの強制解放
/// 7. Windows: イメージ名ベースの taskkill 追い打ち
/// 8. プロセス終了の待機（Windows: wait_for_process_exit / 他: 固定待機）
/// 9. アプリロック解放 → MYCUTE_HOME ディレクトリ削除（delete_home_directory）
/// 10. app_handle.exit(0) でクリーンシャットダウン
#[command]
#[cfg_attr(windows, allow(unreachable_code))]
pub async fn reset_and_exit(
    state: State<'_, TauriState>,
    app_handle: tauri::AppHandle,
) -> Result<(), String> {
    log::warn!("RESET_AND_EXIT: Starting complete reset and exit...");

    // Windows: 完全初期化＋終了は未サポート
    // 理由: taskkill の特権不足と mandatory file locking によりプロセス強制終了と
    // ディレクトリ削除が確実に完了できないため。macOS では問題なく動作する。
    #[cfg(windows)]
    {
        let _ = (&state, &app_handle);
        log::warn!("RESET_AND_EXIT: Reset and exit is not supported on Windows.");
        return Err("This feature is not supported on Windows.".to_string());
    }

    // ---- Phase 1: 使用統計の削除（ベストエフォート） ----
    if let Err(e) = UsageStats::delete_all_stats() {
        log::error!("RESET_AND_EXIT: Failed to delete usage stats: {}", e);
    }

    // ---- Phase 2: DB リセット ----
    let pools = {
        let guard = state.config_mgr.db_pools.read();
        guard
            .as_ref()
            .cloned()
            .ok_or_else(|| "RESET_AND_EXIT: Database not initialized.".to_string())?
    };
    let db = pools
        .get_rw()
        .map_err(|e| format!("RESET_AND_EXIT: Failed to get DB connection: {}", e))?;

    // Bifrost クリーンアップのためにプロバイダー名を DB 削除前に保存
    let provider_names: Vec<String> = LmgwProviders::find()
        .all(db)
        .await
        .map_err(|e| {
            format!("RESET_AND_EXIT: Failed to read providers before reset: {}", e)
        })?
        .into_iter()
        .map(|p| p.provider_name)
        .collect();

    let provider_count = provider_names.len();
    if provider_count > 0 {
        log::info!(
            "RESET_AND_EXIT: Found {} provider(s) to clean up from Bifrost.",
            provider_count
        );
    }

    do_reset_db(&db)
        .await
        .map_err(|e| format!("RESET_AND_EXIT: DB reset failed: {}", e))?;

    // ---- Phase 3: Bifrost プロバイダークリーンアップ（ベストエフォート） ----
    if provider_count > 0 {
        let rt_port = state.config_mgr.settings.read().server.rt_port;
        let url = format!(
            "http://{IP_LOCALHOST}:{rt_port}/api/v1/internal/bifrost/clear-providers"
        );
        let body = json!({ "provider_names": provider_names });
        match state.hc.post(&url).json(&body).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    log::info!("RESET_AND_EXIT: Bifrost providers cleaned up.");
                } else {
                    log::warn!(
                        "RESET_AND_EXIT: Bifrost cleanup returned non-success status: {}",
                        resp.status()
                    );
                }
            }
            Err(e) => {
                log::warn!(
                    "RESET_AND_EXIT: Failed to request Bifrost cleanup: {}",
                    e
                );
            }
        }
    }

    // ---- Phase 4: DB コネクションの明示的解放 ----
    drop(pools);
    *state.config_mgr.db_pools.write() = None;
    log::info!("RESET_AND_EXIT: DB connections closed.");

    // ---- Phase 5: シャットダウンフラグを先に立てる ----
    // バックエンドを kill する前に is_shutting_down を true にすることで、
    // watchdog による Fate-Sharing 自爆を抑制する（並行タスクとの競合防止）
    state.is_shutting_down.store(true, Ordering::SeqCst);
    log::info!("RESET_AND_EXIT: Shutdown flag set. Backend watchdog suppressed.");

    // ---- Phase 6: バックエンドプロセスの終了 ----
    if let Some(guard) = state.backend_guard.lock().take() {
        let pid = guard.pid;
        drop(guard); // BackendProcessGuard の Drop によりバックエンド kill
        log::info!("RESET_AND_EXIT: Backend process (PID: {}) terminated.", pid);
    }

    // ---- Phase 7: 全バックエンドポートの強制解放（念のため） ----
    state.config_mgr.cleanup_all_backend_ports("reset_and_exit");

    // ---- Phase 7.5: Windows: イメージ名ベースの追い打ち（orphan 対策） ----
    // ポートベースのクリーンアップが netstat の競合等で取りこぼした場合の安全網。
    // macOS ではこのブロックはコンパイル自体されないため影響ゼロ。
    #[cfg(windows)]
    {
        log::info!("RESET_AND_EXIT: Image-name based cleanup: bifrost-http.exe, zeroclaw.exe");
        let _ = kill_process_by_image_name("bifrost-http.exe");
        let _ = kill_process_by_image_name("zeroclaw.exe");
    }

    // ---- Phase 8: プロセス終了の待機 ----
    // Windows: Bifrost/ZeroClaw の終了をポーリングで確認し、全終了後に進む
    #[cfg(windows)]
    {
        // Bifrost と ZeroClaw の終了を最大5秒間ポーリングする
        let target_pids: Vec<u32> = ["bifrost-http.exe", "zeroclaw.exe"]
            .iter()
            .filter_map(|name| crate::utils::process::find_pid_by_image_name(name))
            .collect();

        if !target_pids.is_empty() {
            log::info!(
                "RESET_AND_EXIT: Waiting for processes to exit: {:?}",
                target_pids
            );
            let alive = wait_for_process_exit(&target_pids, Duration::from_millis(5000));
            if !alive.is_empty() {
                log::warn!(
                    "RESET_AND_EXIT: Processes still alive after wait: {:?}",
                    alive
                );
            }
        } else {
            log::info!("RESET_AND_EXIT: No Bifrost/ZeroClaw processes found. Proceeding.");
        }

        // 強制終了後の OS ファイルハンドル解放を待つための追加待機
        tokio::time::sleep(Duration::from_millis(1000)).await;
    }
    #[cfg(not(windows))]
    {
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    // ---- Phase 9: アプリロック解放 → MYCUTE_HOME ディレクトリ削除 ----
    // Windows: release_lock() でアプリロックファイルハンドルを閉じてから削除する
    // macOS: 開いているファイルの削除が可能なため不要
    let home_dir = state.config_mgr.home_dir.clone();
    #[cfg(windows)]
    {
        if let Err(e) = release_and_delete(&home_dir).await {
            log::error!("RESET_AND_EXIT: Sync cleanup failed: {}", e);
        }
    }
    #[cfg(not(windows))]
    {
        delete_home_directory(&home_dir).await?;
    }

    // ---- Phase 10: アプリ終了 ----
    log::warn!("RESET_AND_EXIT: Exiting application.");
    app_handle.exit(0);

    Ok(())
}

/// Windows 音声入力のヘルスチェックを確認済みとしてマークする（フロントエンドがダイアログを閉じた時に呼ぶ）
#[command]
pub fn acknowledge_windows_health() {
    #[cfg(windows)]
    crate::stt::win::acknowledge_health_check();
    log::debug!("[HealthCheck] Acknowledged by user.");
}

/// Windows 設定画面 (`ms-settings:` URI) を開く
#[command]
pub fn open_windows_settings(_uri: String) -> Result<(), String> {
    #[cfg(windows)]
    {
        std::process::Command::new("cmd")
            .args(["/c", "start", "", &uri])
            .spawn()
            .map_err(|e| format!("Failed to open Windows settings: {}", e))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // -------------------------------------------------------------------------
    // delete_home_directory tests (cross-platform)
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn test_delete_home_directory_empty_dir() {
        let dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let path = dir.path().to_path_buf();
        assert!(path.exists(), "Temp dir should exist");

        let result = delete_home_directory(&path).await;
        assert!(result.is_ok(), "Empty dir should be deletable: {:?}", result);
        // delete_home_directory で削除済みのため、TempDir の drop はエラーを無視する
    }

    #[tokio::test]
    async fn test_delete_home_directory_with_files() {
        let dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        // ファイルとサブディレクトリを作成
        let file_path = dir.path().join("test_file.txt");
        std::fs::write(&file_path, "hello").expect("Failed to write file");
        let sub_dir = dir.path().join("subdir");
        std::fs::create_dir(&sub_dir).expect("Failed to create subdir");
        let sub_file = sub_dir.join("nested.txt");
        std::fs::write(&sub_file, "nested").expect("Failed to write nested file");

        let path = dir.path().to_path_buf();
        let result = delete_home_directory(&path).await;
        assert!(result.is_ok(), "Dir with files should be deletable: {:?}", result);
    }

    #[tokio::test]
    async fn test_delete_home_directory_nonexistent_path() {
        let path = Path::new("C:\\THIS_PATH_SHOULD_NOT_EXIST_12345_TEST");
        let result = delete_home_directory(path).await;
        assert!(result.is_ok(), "Non-existent path should return Ok: {:?}", result);
    }

    #[tokio::test]
    async fn test_delete_home_directory_readonly_file() {
        let dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let file_path = dir.path().join("readonly.txt");
        std::fs::write(&file_path, "readonly").expect("Failed to write file");
        // Windows: readonly 属性を設定
        #[cfg(windows)]
        {
            let _ = std::process::Command::new("attrib")
                .args(&["+R", &file_path.to_string_lossy()])
                .status();
        }
        let path = dir.path().to_path_buf();
        let result = delete_home_directory(&path).await;
        assert!(result.is_ok(), "Dir with readonly file should be deletable: {:?}", result);
    }

    // -------------------------------------------------------------------------
    // Windows-only: release_and_delete tests
    // -------------------------------------------------------------------------

    #[cfg(windows)]
    #[tokio::test]
    async fn test_release_and_delete_simple() {
        let dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let path = dir.path().to_path_buf();
        // release_and_delete は singleton lock の release を試みる
        // ロックを取得していなくても問題なく動作するはず
        let result = release_and_delete(&path).await;
        assert!(result.is_ok(), "release_and_delete should succeed: {:?}", result);
    }

    // -------------------------------------------------------------------------
    // Additional edge case tests for delete_home_directory
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn test_delete_home_directory_deeply_nested() {
        let dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        // 深いネスト構造を作成: a/b/c/d/e/f/g/h/i/j/
        let nested = dir.path().join("a/b/c/d/e/f/g/h/i/j");
        std::fs::create_dir_all(&nested).expect("Failed to create nested dirs");
        let file = nested.join("deep_file.txt");
        std::fs::write(&file, "deep").expect("Failed to write file");

        let path = dir.path().to_path_buf();
        let result = delete_home_directory(&path).await;
        assert!(result.is_ok(), "Deeply nested dir should be deletable: {:?}", result);
    }

    #[tokio::test]
    async fn test_delete_home_directory_unicode_names() {
        let dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        // Unicode ファイル名を含むディレクトリ
        let unicode_file = dir.path().join("日本語ファイル名.txt");
        std::fs::write(&unicode_file, "日本語").expect("Failed to write unicode file");
        let emoji_file = dir.path().join("🎉test🎊.txt");
        std::fs::write(&emoji_file, "emoji").expect("Failed to write emoji file");

        let path = dir.path().to_path_buf();
        let result = delete_home_directory(&path).await;
        assert!(result.is_ok(), "Dir with unicode names should be deletable: {:?}", result);
    }

    #[tokio::test]
    async fn test_delete_home_directory_idempotent() {
        let dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let path = dir.path().to_path_buf();

        // 1回目: 削除成功
        let result1 = delete_home_directory(&path).await;
        assert!(result1.is_ok(), "First deletion should succeed: {:?}", result1);

        // 2回目: 既に削除済みでも OK（べき等）
        let result2 = delete_home_directory(&path).await;
        assert!(result2.is_ok(), "Second deletion (idempotent) should succeed: {:?}", result2);
    }

    #[tokio::test]
    async fn test_delete_home_directory_hidden_files() {
        let dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        // 隠しファイル（ドットファイル）
        let hidden = dir.path().join(".hidden_config");
        std::fs::write(&hidden, "secret").expect("Failed to write hidden file");
        // Windows: 隠しファイル属性を設定
        #[cfg(windows)]
        {
            let _ = std::process::Command::new("attrib")
                .args(&["+H", &hidden.to_string_lossy()])
                .status();
        }

        let path = dir.path().to_path_buf();
        let result = delete_home_directory(&path).await;
        assert!(result.is_ok(), "Dir with hidden files should be deletable: {:?}", result);
    }

    // -------------------------------------------------------------------------
    // Windows-only integration test: 実在プロセス kill → ディレクトリ削除
    // -------------------------------------------------------------------------

    /// 軽量 Integration Test: ping.exe を起動し、kill_process_by_image_name で kill した後、
    /// ディレクトリ削除が正常に動作することを検証する。
    #[cfg(windows)]
    #[tokio::test]
    async fn test_integration_kill_real_process_and_delete() {
        use crate::utils::process::CommandExtSafe;
        use std::process::Command;

        // ping をバックグラウンド起動
        // ping -n 10 127.0.0.1 は約9秒間稼働する
        let child: std::process::Child = match Command::new("cmd")
            .args(&["/c", "start", "/b", "ping", "-n", "10", "127.0.0.1"])
            .hide_window_if_windows()
            .spawn()
        {
            Ok(c) => c,
            Err(e) => {
                log::warn!("Skipping: could not start ping: {}", e);
                return;
            }
        };
        let pid = child.id();
        std::thread::sleep(Duration::from_millis(500));

        // kill_process_by_image_name で kill
        let kill_result = crate::utils::process::kill_process_by_image_name("ping.exe");
        assert!(kill_result.is_ok(), "kill_process_by_image_name should succeed");

        // wait_for_process_exit で終了確認
        let alive = crate::utils::process::wait_for_process_exit(
            &[pid],
            Duration::from_millis(5000),
        );
        assert!(alive.is_empty(), "ping.exe (PID: {}) should be killed", pid);

        // ディレクトリ削除が可能であることを確認
        let dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let file_path = dir.path().join("after_kill.txt");
        std::fs::write(&file_path, "data").expect("Failed to write file");
        let path = dir.path().to_path_buf();
        let result = delete_home_directory(&path).await;
        assert!(result.is_ok(), "Directory deletion after kill should succeed: {:?}", result);
    }

}
