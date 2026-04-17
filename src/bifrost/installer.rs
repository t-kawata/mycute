use crate::bifrost::assets::{get_bifrost_asset, ArchiveFormat};
use crate::bifrost::error::BifrostError;
use crate::bifrost::BIFROST_DIRNAME;
use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
#[cfg(target_os = "macos")]
use std::process::Command;
use tar::Archive;
use zip::ZipArchive;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

pub struct InstallResult {
    pub root_dir: PathBuf,
    pub install_dir: PathBuf,
}

pub fn install(home: &Path) -> Result<InstallResult> {
    let asset = get_bifrost_asset().ok_or_else(|| {
        BifrostError::UnsupportedPlatform(
            "No Bifrost asset available for this platform".to_string(),
        )
    })?;

    let root_dir = home.join(BIFROST_DIRNAME);
    let install_dir = root_dir.join(asset.version);

    // 共通ディレクトリの作成
    fs::create_dir_all(&root_dir).context("Failed to create bifrost root directory")?;

    // バイナリの展開（すでに存在しても安全性のため再展開を検討しても良いが、ここでは存在チェックでスキップ）
    if !install_dir.exists() {
        log::info!("Installing Bifrost {} to {:?}", asset.version, install_dir);
        fs::create_dir_all(&install_dir).context("Failed to create install directory")?;

        match asset.format {
            ArchiveFormat::TarGz => {
                extract_tar_gz(asset.bytes, &install_dir)?;
            }
            ArchiveFormat::Zip => {
                extract_zip(asset.bytes, &install_dir)?;
            }
        }

        #[cfg(target_os = "macos")]
        remove_quarantine_flag(&install_dir)?;
    } else {
        log::info!(
            "Bifrost {} is already installed at {:?}",
            asset.version,
            install_dir
        );
    }

    // config.json の生成 (共通ディレクトリ直下)
    generate_config_json(&root_dir)?;

    Ok(InstallResult {
        root_dir,
        install_dir,
    })
}

/// Bifrost 設定ファイルのテンプレート (JSON)。
/// $schema やクエリ制限、SQLite パス設定を定義する。
const BIFROST_CONFIG_TEMPLATE: &str = r#"{
  "$schema": "https://www.getbifrost.ai/schema",
  "client": {
    "drop_excess_requests": false,
    "enable_logging": false,
    "allowed_origins": ["*"]
  },
  "providers": {},
  "config_store": {
    "enabled": true,
    "type": "sqlite",
    "config": {
      "path": "{db_path}"
    }
  },
  "log_store": {
    "enabled": true,
    "type": "sqlite",
    "config": {
      "path": "{log_path}"
    }
  }
}"#;

fn generate_config_json(root_dir: &Path) -> Result<()> {
    let config_path = root_dir.join("config.json");
    let db_path = root_dir.join("config.sqlite");
    let log_path = root_dir.join("logs.sqlite");

    // 一貫性を保つため、常に絶対パスを使用
    // Windows のバックスラッシュが JSON 内で不正なエスケープシーケンスとして扱われるのを防ぐため、スラッシュに置換。
    let db_path_str = db_path.to_string_lossy().replace("\\", "/");
    let log_path_str = log_path.to_string_lossy().replace("\\", "/");

    // テンプレートのプレースホルダーを置換
    let config_content = BIFROST_CONFIG_TEMPLATE
        .replace("{db_path}", &db_path_str)
        .replace("{log_path}", &log_path_str);

    log::info!("Generating Bifrost config at {:?}", config_path);
    fs::write(&config_path, config_content).context("Failed to write Bifrost config.json")?;

    Ok(())
}

fn extract_tar_gz(bytes: &[u8], target: &Path) -> Result<()> {
    let tar = GzDecoder::new(Cursor::new(bytes));
    let mut archive = Archive::new(tar);

    // Bifrost もアーカイブ直下にバイナリがある構造のため、
    // トップレベルディレクトリをスキップせずに展開
    for entry in archive.entries().context("Failed to read tar entries")? {
        let mut entry = entry.context("Failed to get tar entry")?;
        let path = entry
            .path()
            .context("Failed to get entry path")?
            .to_path_buf();
        let dest = target.join(path);

        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }

        entry
            .unpack(&dest)
            .context(format!("Failed to unpack to {:?}", dest))?;

        #[cfg(unix)]
        apply_unix_permissions(&dest)?;
    }

    Ok(())
}

fn extract_zip(bytes: &[u8], target: &Path) -> Result<()> {
    let mut archive = ZipArchive::new(Cursor::new(bytes)).context("Failed to read zip archive")?;

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .context("Failed to get zip file entry")?;
        let outpath = match file.enclosed_name() {
            Some(path) => path.to_owned(),
            None => continue,
        };

        let dest = target.join(outpath);

        if file.name().ends_with('/') {
            fs::create_dir_all(&dest).context("Failed to create zip directory")?;
        } else {
            if let Some(p) = dest.parent() {
                if !p.exists() {
                    fs::create_dir_all(p)
                        .context("Failed to create parent directory for zip entry")?;
                }
            }
            let mut outfile =
                fs::File::create(&dest).context("Failed to create file for zip entry")?;
            std::io::copy(&mut file, &mut outfile).context("Failed to copy zip entry to file")?;
        }

        #[cfg(unix)]
        apply_unix_permissions(&dest)?;
    }

    Ok(())
}

#[cfg(unix)]
fn apply_unix_permissions(path: &Path) -> Result<()> {
    if path.is_file() {
        // バイナリ自体に実行権限を付与
        if let Ok(metadata) = fs::metadata(path) {
            let mut perms = metadata.permissions();
            perms.set_mode(0o755);
            let _ = fs::set_permissions(path, perms);
        }
    }
    Ok(())
}

/// macOS の隔離属性 (Quarantine flag) を再帰的に除去します。
#[cfg(target_os = "macos")]
fn remove_quarantine_flag(install_dir: &Path) -> Result<()> {
    log::info!(
        "Removing macOS quarantine flag from Bifrost at {:?}",
        install_dir
    );

    let status = Command::new("xattr")
        .arg("-rc")
        .arg(install_dir)
        .status()
        .context("Failed to execute xattr command for Bifrost")?;

    if !status.success() {
        log::warn!("xattr command for Bifrost failed with status: {:?}", status);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_generate_config_json_escaping() {
        let dir = tempdir().unwrap();
        let root_dir = dir.path();

        // 正常に生成されるか確認
        generate_config_json(root_dir).expect("Failed to generate config");

        let config_path = root_dir.join("config.json");
        assert!(config_path.exists());

        let content = std::fs::read_to_string(config_path).unwrap();
        
        // JSON としてパースできるか確認
        let json: serde_json::Value = serde_json::from_str(&content).expect("Generated JSON is invalid");

        // パスにバックスラッシュが含まれていないこと（スラッシュに置換されていること）を確認
        let db_path = json["config_store"]["config"]["path"].as_str().unwrap();
        let log_path = json["log_store"]["config"]["path"].as_str().unwrap();

        assert!(!db_path.contains('\\'), "db_path should not contain backslashes: {}", db_path);
        assert!(!log_path.contains('\\'), "log_path should not contain backslashes: {}", log_path);
        
        // パスが正しいファイル名を指しているか確認
        assert!(db_path.ends_with("config.sqlite"));
        assert!(log_path.ends_with("logs.sqlite"));
    }
}
