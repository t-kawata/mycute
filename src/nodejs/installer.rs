use std::fs;
use std::io::Cursor;
#[cfg(target_os = "macos")]
use std::process::Command;
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use tar::Archive;
use zip::ZipArchive;
use crate::nodejs::assets::{get_node_asset, ArchiveFormat};
use crate::nodejs::error::NodeError;
use crate::nodejs::NODE_JS_DIRNAME;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

pub fn install(home: &Path) -> Result<PathBuf> {
    let asset = get_node_asset().ok_or_else(|| {
        NodeError::UnsupportedPlatform("No Node.js asset available for this platform".to_string())
    })?;

    let install_dir = home.join(NODE_JS_DIRNAME).join(asset.version);
    
    // すでに展開済みの場合はスキップ
    if install_dir.exists() {
        log::info!("Node.js {} is already installed at {:?}", asset.version, install_dir);
        return Ok(install_dir);
    }

    log::info!("Installing Node.js {} to {:?}", asset.version, install_dir);
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
    
    // 最初のディレクトリレベルをスキップするために個別エントリを処理
    for entry in archive.entries().context("Failed to read tar entries")? {
        let mut entry = entry.context("Failed to get tar entry")?;
        let path = entry.path().context("Failed to get entry path")?.to_path_buf();
        
        // トップレベルディレクトリを除去
        let components: Vec<_> = path.components().skip(1).collect();
        if components.is_empty() {
            continue;
        }
        let stripped_path: PathBuf = components.iter().collect();
        let dest = target.join(stripped_path);
        
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

        // トップレベルディレクトリを除去
        let components: Vec<_> = outpath.components().skip(1).collect();
        if components.is_empty() {
            continue;
        }
        let stripped_path: PathBuf = components.iter().collect();
        let dest = target.join(stripped_path);

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
        // binディレクトリ内のファイルや、拡張子がないファイルなどは実行権限を付与
        let is_bin = path.to_string_lossy().contains("/bin/");
        let no_ext = path.extension().is_none();
        
        if is_bin || no_ext {
            if let Ok(metadata) = fs::metadata(path) {
                let mut perms = metadata.permissions();
                perms.set_mode(0o755);
                let _ = fs::set_permissions(path, perms);
            }
        }
    }
    Ok(())
}

/// macOS の隔離属性 (Quarantine flag) を再帰的に除去します。
/// これにより、ゲートキーパーによる実行ブロックを回避します。
#[cfg(target_os = "macos")]
fn remove_quarantine_flag(install_dir: &Path) -> Result<()> {
    log::info!("Removing macOS quarantine flag from {:?}", install_dir);
    
    let status = Command::new("xattr")
        .arg("-rc")
        .arg(install_dir)
        .status()
        .context("Failed to execute xattr command")?;

    if !status.success() {
        log::warn!("xattr command failed with status: {:?}", status);
        // 致命的なエラーとはせず、警告に留めます（実行自体は試みるため）
    }

    Ok(())
}
