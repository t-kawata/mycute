use std::fs;
use std::io::Cursor;
use std::process::Command;
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use tar::Archive;
use zip::ZipArchive;
use crate::zeroclaw::assets::{get_zeroclaw_asset, ArchiveFormat};
use crate::zeroclaw::error::ZeroClawError;
use crate::zeroclaw::ZEROCLAW_DIRNAME;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

pub fn install(home: &Path) -> Result<PathBuf> {
    let asset = get_zeroclaw_asset().ok_or_else(|| {
        ZeroClawError::UnsupportedPlatform("No ZeroClaw asset available for this platform".to_string())
    })?;

    let install_dir = home.join(ZEROCLAW_DIRNAME).join(asset.version);
    
    // すでに展開済みの場合はスキップ
    if install_dir.exists() {
        log::info!("ZeroClaw {} is already installed at {:?}", asset.version, install_dir);
        return Ok(install_dir);
    }

    log::info!("Installing ZeroClaw {} to {:?}", asset.version, install_dir);
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

    Ok(install_dir)
}

fn extract_tar_gz(bytes: &[u8], target: &Path) -> Result<()> {
    let tar = GzDecoder::new(Cursor::new(bytes));
    let mut archive = Archive::new(tar);
    
    // ZeroClaw は圧縮ファイル直下にバイナリがある構造のため、
    // nodejs のようにトップレベルディレクトリをスキップせずに展開
    for entry in archive.entries().context("Failed to read tar entries")? {
        let mut entry = entry.context("Failed to get tar entry")?;
        let path = entry.path().context("Failed to get entry path")?.to_path_buf();
        let dest = target.join(path);
        
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        
        entry.unpack(&dest).context(format!("Failed to unpack to {:?}", dest))?;
        
        #[cfg(unix)]
        apply_unix_permissions(&dest)?;
    }
    
    Ok(())
}

fn extract_zip(bytes: &[u8], target: &Path) -> Result<()> {
    let mut archive = ZipArchive::new(Cursor::new(bytes)).context("Failed to read zip archive")?;
    
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).context("Failed to get zip file entry")?;
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
                    fs::create_dir_all(p).context("Failed to create parent directory for zip entry")?;
                }
            }
            let mut outfile = fs::File::create(&dest).context("Failed to create file for zip entry")?;
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
        // ZeroClaw バイナリ自体に実行権限を付与
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
    log::info!("Removing macOS quarantine flag from ZeroClaw at {:?}", install_dir);
    
    let status = Command::new("xattr")
        .arg("-rc")
        .arg(install_dir)
        .status()
        .context("Failed to execute xattr command for ZeroClaw")?;

    if !status.success() {
        log::warn!("xattr command for ZeroClaw failed with status: {:?}", status);
    }

    Ok(())
}
