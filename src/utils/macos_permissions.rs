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

        log::info!(
            "<MacOSPermissions> Checking version change... Current: {}, Last Run: {:?}",
            current_version,
            last_version
        );

        if last_version.as_deref() != Some(current_version) {
            log::info!("<MacOSPermissions> Version change detected. Initiating Accessibility reset process...");

            // 動的に Bundle Identifier を取得
            if let Some(bundle_id) = get_bundle_identifier() {
                log::info!("<MacOSPermissions> Executing: tccutil reset Accessibility {}", bundle_id);

                let output = Command::new("tccutil")
                    .arg("reset")
                    .arg("Accessibility")
                    .arg(&bundle_id)
                    .output();

                match output {
                    Ok(out) => {
                        let stdout = String::from_utf8_lossy(&out.stdout);
                        let stderr = String::from_utf8_lossy(&out.stderr);
                        
                        if out.status.success() {
                            log::info!(
                                "<MacOSPermissions> Success: Accessibility permissions reset for {}. stdout: '{}', stderr: '{}'",
                                bundle_id, stdout.trim(), stderr.trim()
                            );
                        } else {
                            log::warn!(
                                "<MacOSPermissions> Failure: tccutil reset failed for {}. Status: {}, stdout: '{}', stderr: '{}'",
                                bundle_id, out.status, stdout.trim(), stderr.trim()
                            );
                        }
                    }
                    Err(e) => {
                        log::error!("<MacOSPermissions> Command Error: Failed to execute tccutil command: {}", e);
                    }
                }
            } else {
                log::warn!("<MacOSPermissions> Skip: Could not detect Bundle Identifier. (This is normal if not running as a .app bundle)");
            }

            // バージョン情報を更新
            config_mgr
                .set_value_to_db(
                    SETTING_KEY_LAST_RUN_VERSION,
                    serde_json::Value::String(current_version.to_string()),
                )
                .await?;
            log::info!("<MacOSPermissions> DB Updated: last_run_version set to {} in database.", current_version);
        } else {
            log::debug!("<MacOSPermissions> No change: version matches the last run. Skipping reset.");
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
