#[cfg(target_os = "macos")]
use crate::constants::{MYCUTE_VERSION, SETTING_KEY_LAST_RUN_VERSION};
use crate::mycute_settings::ConfigManager;
#[cfg(target_os = "macos")]
use cocoa::base::{id, nil};
#[cfg(target_os = "macos")]
use cocoa::foundation::{NSBundle, NSString};
#[cfg(target_os = "macos")]
use std::ffi::CStr;
#[cfg(target_os = "macos")]
use std::process::Command;

/// MacOS のアクセシビリティ権限（TCC）を必要に応じてリセットします。
/// アプリケーションのバージョンが以前の実行時から変更されている場合に実行されます。
pub async fn check_and_reset_macos_permissions(config_mgr: &ConfigManager) -> anyhow::Result<()> {
    #[cfg(target_os = "macos")]
    {
        let current_version = MYCUTE_VERSION;

        // DB から前回の実行バージョンを取得
        let last_version_val = config_mgr
            .get_value_from_db(SETTING_KEY_LAST_RUN_VERSION)
            .await?;
        let last_version = last_version_val.and_then(|v| v.as_str().map(|s| s.to_string()));

        log::debug!(
            "<MacOSPermissions> Current version: {}, Last run version: {:?}",
            current_version,
            last_version
        );

        if last_version.as_deref() != Some(current_version) {
            log::info!("<MacOSPermissions> Version change detected (or first run). Checking Bundle Identifier...");

            // 動的に Bundle Identifier を取得
            if let Some(bundle_id) = get_bundle_identifier() {
                log::info!("<MacOSPermissions> Detected Bundle Identifier: {}. Resetting Accessibility permissions for macOS TCC...", bundle_id);

                let status = Command::new("tccutil")
                    .arg("reset")
                    .arg("Accessibility")
                    .arg(&bundle_id)
                    .status();

                match status {
                    Ok(s) if s.success() => {
                        log::info!("<MacOSPermissions> Successfully reset Accessibility permissions for {}.", bundle_id);
                    }
                    Ok(s) => {
                        log::warn!("<MacOSPermissions> tccutil reset failed with status: {}. This might be expected if the app was not yet registered.", s);
                    }
                    Err(e) => {
                        log::error!("<MacOSPermissions> Failed to execute tccutil: {}", e);
                    }
                }
            } else {
                log::warn!("<MacOSPermissions> Could not detect Bundle Identifier. Skipping TCC reset. (This is normal in development mode)");
            }

            // バージョン情報を更新
            config_mgr
                .set_value_to_db(
                    SETTING_KEY_LAST_RUN_VERSION,
                    serde_json::Value::String(current_version.to_string()),
                )
                .await?;
            log::debug!(
                "<MacOSPermissions> Updated last_run_version to {} in DB.",
                current_version
            );
        } else {
            log::debug!("<MacOSPermissions> No version change detected. Skipping TCC reset.");
        }
    }

    #[cfg(not(target_os = "macos"))]
    {
        // MacOS 以外では何もしない
        let _ = config_mgr;
    }

    Ok(())
}

/// 実行中のアプリケーションの Bundle Identifier を動的に取得します。
#[cfg(target_os = "macos")]
fn get_bundle_identifier() -> Option<String> {
    unsafe {
        let bundle: id = NSBundle::mainBundle();
        if bundle == nil {
            return None;
        }
        let bundle_id = bundle.bundleIdentifier();
        if bundle_id == nil {
            return None;
        }
        let c_str = CStr::from_ptr(bundle_id.UTF8String());
        Some(c_str.to_string_lossy().into_owned())
    }
}
