#[cfg(target_os = "macos")]
use crate::config::settings::Env;
#[cfg(target_os = "macos")]
use crate::constants::{MYCUTE_VERSION, SETTING_KEY_LAST_RUN_VERSION};
#[cfg(target_os = "macos")]
use crate::mycute_settings::ConfigManager;
#[cfg(target_os = "macos")]
use crate::utils::db::get_db;
#[cfg(target_os = "macos")]
use crate::utils::init::LogLevel;
#[cfg(target_os = "macos")]
use crate::utils::my_path::get_mycute_home;
#[cfg(target_os = "macos")]
use std::process::Command;
#[cfg(target_os = "macos")]
use std::thread;
#[cfg(target_os = "macos")]
use std::time::Duration;
#[cfg(target_os = "macos")]
use std::path::PathBuf;

// --- macOS 固有のネイティブ API とフレームワーク ---
#[cfg(target_os = "macos")]
use core_foundation::base::TCFType;
#[cfg(target_os = "macos")]
use core_foundation::boolean::CFBoolean;
#[cfg(target_os = "macos")]
use core_foundation::dictionary::CFDictionary;
#[cfg(target_os = "macos")]
use core_foundation::string::CFString;

#[cfg(target_os = "macos")]
#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    /// アクセシビリティ権限を確認し、必要に応じてプロンプトを表示する。
    fn AXIsProcessTrustedWithOptions(options: core_foundation::dictionary::CFDictionaryRef) -> bool;
    /// プロンプト表示オプションの定数キー。
    static kAXTrustedCheckOptionPrompt: core_foundation::string::CFStringRef;
}

#[cfg(target_os = "macos")]
const BUNDLE_ID: &str = "com.shyme.mycute";
#[cfg(target_os = "macos")]
const APP_BUNDLE_PATH: &str = "/Applications/mycute.app";

/// lsregister ツールの既知の配置候補地。
/// OS のバージョン（Intel/Silicon）やディレクトリ構造の変化に対応するため網羅的に定義。
#[cfg(target_os = "macos")]
const LSREGISTER_CANDIDATES: &[&str] = &[
    "/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister",
    "/System/Library/Frameworks/CoreServices.framework/Versions/A/Frameworks/LaunchServices.framework/Versions/A/Support/lsregister",
    "/System/Library/Frameworks/CoreServices.framework/Versions/A/Frameworks/LaunchServices.framework/Support/lsregister",
    "/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Versions/A/Support/lsregister",
    "/System/Library/Frameworks/CoreServices.framework/Versions/Current/Frameworks/LaunchServices.framework/Support/lsregister",
    "/System/Library/Frameworks/CoreServices.framework/Versions/Current/Frameworks/LaunchServices.framework/Versions/Current/Support/lsregister",
];

/// MacOS 起動時の事前チェック結果を示す列挙型
pub enum PrelaunchAction {
    Continue,
    Restart,
}

/// MacOS のエントリポイントから呼び出される事前チェック処理。
#[cfg(target_os = "macos")]
pub fn handle_macos_prelaunch_checks() -> anyhow::Result<PrelaunchAction> {
    let args: Vec<String> = std::env::args().collect();
    
    // 1. 再起動ループ防止フラグのチェック
    if args.iter().any(|arg| arg == "--macos-resetted") {
        eprintln!("<MacOSPermissions> Found --macos-resetted flag. Requesting fresh prompt...");
        request_accessibility_permission_prompt();
        return Ok(PrelaunchAction::Continue);
    }

    // 2. バージョンチェック
    let current_version = MYCUTE_VERSION;
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build()?;

    let needs_reset = rt.block_on(async {
        let home = get_mycute_home(None);
        let config_mgr = ConfigManager::new_bootstrap(Some(home))?;
        let storage_settings = config_mgr.settings.read().storage.clone();
        let db_env = Env::from_settings(&storage_settings);
        let pools = get_db(&db_env, &LogLevel::Info).await?;
        let conn = pools.get_rw()?;

        let last_version_val = config_mgr
            .get_value_from_db_with_conn(conn, SETTING_KEY_LAST_RUN_VERSION)
            .await?;
        let last_version = last_version_val.and_then(|v| v.as_str().map(|s| s.to_string()));

        eprintln!("<MacOSPermissions> Version comparison: Current={}, Last={:?}", current_version, last_version);

        Ok::<bool, anyhow::Error>(last_version.as_deref() != Some(current_version))
    })?;

    if needs_reset {
        eprintln!("<MacOSPermissions> Version change detected. Initiating cleanup and restart...");
        
        // 3. 徹底洗浄
        run_extended_cleanups();

        // 4. バージョン更新
        rt.block_on(async {
            let home = get_mycute_home(None);
            let config_mgr = ConfigManager::new_bootstrap(Some(home))?;
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

        // 猶予
        thread::sleep(Duration::from_secs(2));

        // 再起動
        respawn_self(&args)?;
        return Ok(PrelaunchAction::Restart);
    }

    Ok(PrelaunchAction::Continue)
}

/// 徹底的なクリーンアップ（システムツールのパスを動的に解決）
#[cfg(target_os = "macos")]
fn run_extended_cleanups() {
    // 1. 属性消去
    if let Some(xattr_path) = find_system_command("xattr") {
        eprintln!("<MacOSPermissions> Cleaning attributes: {:?} -cr {}", xattr_path, APP_BUNDLE_PATH);
        let _ = Command::new(xattr_path).arg("-cr").arg(APP_BUNDLE_PATH).output();
    }

    // 2. TCC リセット
    run_tcc_reset();

    // 3. Launch Services 再登録 (ゾンビ排除の要)
    if let Some(ls_reg_path) = find_lsregister_path() {
        eprintln!("<MacOSPermissions> Forcing LS registration: {:?} -f {}", ls_reg_path, APP_BUNDLE_PATH);
        let _ = Command::new(ls_reg_path).arg("-f").arg(APP_BUNDLE_PATH).output();
    } else {
        eprintln!("<MacOSPermissions> Warning: lsregister not found. Skipping LS registration.");
    }
}

/// tccutil reset を実行
#[cfg(target_os = "macos")]
fn run_tcc_reset() {
    if let Some(tccutil_path) = find_system_command("tccutil") {
        eprintln!("<MacOSPermissions> Resetting TCC via {:?}", tccutil_path);
        let _ = Command::new(&tccutil_path).arg("reset").arg("All").arg(BUNDLE_ID).output();
        let _ = Command::new(&tccutil_path).arg("reset").arg("Accessibility").arg(BUNDLE_ID).output();
    }
}

/// アクセシビリティ許可プロンプトの強制要求
#[cfg(target_os = "macos")]
pub fn request_accessibility_permission_prompt() {
    unsafe {
        let key = CFString::wrap_under_get_rule(kAXTrustedCheckOptionPrompt);
        let value = CFBoolean::true_value();
        let options = CFDictionary::from_CFType_pairs(&[(key.as_CFType(), value.as_CFType())]);
        
        eprintln!("<MacOSPermissions> Requesting Accessibility prompt...");
        AXIsProcessTrustedWithOptions(options.as_concrete_TypeRef());
    }
}

/// 再起動
#[cfg(target_os = "macos")]
fn respawn_self(original_args: &[String]) -> anyhow::Result<()> {
    let mut args = original_args.to_vec();
    if !args.is_empty() {
        args.remove(0);
    }
    args.push("--macos-resetted".to_string());

    if let Some(open_path) = find_system_command("open") {
        eprintln!("<MacOSPermissions> Respawning via {:?} -b {}", open_path, BUNDLE_ID);
        let mut cmd = Command::new(open_path);
        cmd.arg("-b").arg(BUNDLE_ID);
        if !args.is_empty() {
            cmd.arg("--args");
            for arg in args {
                cmd.arg(arg);
            }
        }
        cmd.spawn()?;
    }
    Ok(())
}

/// システムコマンドを標準ディレクトリ群から探索
#[cfg(target_os = "macos")]
fn find_system_command(name: &str) -> Option<PathBuf> {
    let dirs = ["/usr/bin", "/bin", "/usr/sbin", "/sbin"];
    for dir in dirs {
        let path = PathBuf::from(dir).join(name);
        if path.exists() {
            return Some(path);
        }
    }
    None
}

/// lsregister ツールの所在を複数の候補地から探索
#[cfg(target_os = "macos")]
fn find_lsregister_path() -> Option<PathBuf> {
    for p in LSREGISTER_CANDIDATES {
        let path = PathBuf::from(p);
        if path.exists() {
            return Some(path);
        }
    }
    None
}
