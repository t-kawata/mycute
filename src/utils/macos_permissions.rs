#[cfg(target_os = "macos")]
use crate::constants::{MYCUTE_VERSION, SETTING_KEY_LAST_RUN_VERSION};
#[cfg(target_os = "macos")]
use crate::mycute_settings::ConfigManager;
#[cfg(target_os = "macos")]
use crate::utils::db::get_db;
#[cfg(target_os = "macos")]
use crate::utils::my_path::get_mycute_home;
#[cfg(target_os = "macos")]
use crate::config::settings::Env;
#[cfg(target_os = "macos")]
use crate::utils::init::LogLevel;
#[cfg(target_os = "macos")]
use cocoa::base::{id, nil};
#[cfg(target_os = "macos")]
use cocoa::foundation::{NSBundle, NSString};
#[cfg(target_os = "macos")]
use std::ffi::CStr;
#[cfg(target_os = "macos")]
use std::process::Command;

/// MacOS 起動時の事前チェック結果を示す列挙型
pub enum PrelaunchAction {
    Continue,
    Restart,
}

/// MacOS のエントリポイントから呼び出される事前チェック処理。
/// バージョン変更を検知した場合、権限をリセットして自分自身を再起動します。
#[cfg(target_os = "macos")]
pub fn handle_macos_prelaunch_checks() -> anyhow::Result<PrelaunchAction> {
    // 1. 再起動ループ防止用の内部フラグチェック
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--macos-resetted") {
        log::info!("<MacOSPermissions> Found --macos-resetted flag. Skipping pre-launch checks to prevent loops.");
        return Ok(PrelaunchAction::Continue);
    }

    // 2. バージョンチェック（同期的な暫定ランタイムを使用して DB を確認）
    let current_version = MYCUTE_VERSION;
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    let (needs_reset, bundle_id) = rt.block_on(async {
        // 最小限の初期化で設定を取得
        let home = get_mycute_home(None);
        let config_mgr = ConfigManager::new_bootstrap(Some(home));
        
        // DB パス等の情報を構築
        let storage_settings = config_mgr.settings.read().storage.clone();
        let db_env = Env::from_settings(&storage_settings);
        
        // DB 接続確立（マイグレーションなどは行わない）
        let pools = get_db(&db_env, &LogLevel::Info).await?;
        let conn = pools.get_rw()?;

        // バージョン情報を取得
        let last_version_val = config_mgr
            .get_value_from_db_with_conn(conn, SETTING_KEY_LAST_RUN_VERSION)
            .await?;
        let last_version = last_version_val.and_then(|v| v.as_str().map(|s| s.to_string()));

        log::debug!(
            "<MacOSPermissions> Pre-launch version check: Current={}, Last={:?}",
            current_version,
            last_version
        );

        if last_version.as_deref() != Some(current_version) {
            if let Some(bid) = get_bundle_identifier() {
                // バージョン不一致かつ Bundle ID が取得できた場合のみリセット対象
                return Ok::<_, anyhow::Error>((true, Some(bid)));
            }
        }
        Ok((false, None))
    })?;

    if needs_reset {
        if let Some(bid) = bundle_id {
            log::info!("<MacOSPermissions> Version change detected. Resetting TCC and restarting process...");
            
            // tccutil reset 実行
            let _ = run_tcc_reset(&bid);

            // DB のバージョン情報を更新
            rt.block_on(async {
                let home = get_mycute_home(None);
                let config_mgr = ConfigManager::new_bootstrap(Some(home));
                let storage_settings = config_mgr.settings.read().storage.clone();
                let db_env = Env::from_settings(&storage_settings);
                let pools = get_db(&db_env, &LogLevel::Info).await?;
                let conn = pools.get_rw()?;

                config_mgr
                    .set_value_to_db_with_conn(
                        conn,
                        SETTING_KEY_LAST_RUN_VERSION,
                        serde_json::Value::String(current_version.to_string()),
                    )
                    .await
            })?;

            // 自分自身を再起動
            respawn_self(&args)?;
            return Ok(PrelaunchAction::Restart);
        }
    }

    Ok(PrelaunchAction::Continue)
}

/// tccutil reset Accessibility を実行します。
#[cfg(target_os = "macos")]
fn run_tcc_reset(bundle_id: &str) -> bool {
    log::info!("<MacOSPermissions> Executing: tccutil reset Accessibility {}", bundle_id);
    let output = Command::new("tccutil")
        .arg("reset")
        .arg("Accessibility")
        .arg(bundle_id)
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            if out.status.success() {
                log::info!("<MacOSPermissions> TCC reset successful: {}", stdout.trim());
                true
            } else {
                log::warn!("<MacOSPermissions> TCC reset failed: {}. Stderr: {}", out.status, stderr.trim());
                false
            }
        }
        Err(e) => {
            log::error!("<MacOSPermissions> Failed to execute tccutil: {}", e);
            false
        }
    }
}

/// 現在のプロセスを新しい PID で起動し直し、内部フラグを付与します。
#[cfg(target_os = "macos")]
fn respawn_self(original_args: &[String]) -> anyhow::Result<()> {
    let current_exe = std::env::current_exe()?;
    let mut args = original_args.to_vec();
    if !args.is_empty() {
        args.remove(0); // 最初の引数（自分自身のパス）を削除
    }
    args.push("--macos-resetted".to_string());

    log::info!("<MacOSPermissions> Respawning self: {:?} with args: {:?}", current_exe, args);
    
    // Command::spawn() は子プロセスを切り離して実行する。
    Command::new(current_exe)
        .args(args)
        .spawn()?;

    log::info!("<MacOSPermissions> Respawn successful. Exiting original process.");
    Ok(())
}

/// 実行中のアプリケーションの Bundle Identifier を動的に取得します。
#[cfg(target_os = "macos")]
fn get_bundle_identifier() -> Option<String> {
    unsafe {
        let bundle: id = NSBundle::mainBundle();
        if bundle == nil {
            log::debug!("<MacOSPermissions> NSBundle::mainBundle() returned nil.");
            return None;
        }
        
        let bundle_id = bundle.bundleIdentifier();
        if bundle_id == nil {
            log::debug!("<MacOSPermissions> bundle.bundleIdentifier() returned nil. (Binary may not be inside a .app bundle)");
            return None;
        }
        
        let c_str = CStr::from_ptr(bundle_id.UTF8String());
        let id_str = c_str.to_string_lossy().into_owned();
        log::info!("<MacOSPermissions> Detected Bundle Identifier: {}", id_str);
        Some(id_str)
    }
}
