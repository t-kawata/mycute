use cfg_if::cfg_if;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveFormat {
    TarGz,
    Zip,
}

pub struct BifrostAsset {
    pub bytes: &'static [u8],
    pub format: ArchiveFormat,
    pub version: &'static str,
}

cfg_if! {
    if #[cfg(all(target_os = "macos", target_arch = "aarch64"))] {
        const BIFROST_BYTES: &[u8] = include_bytes!("bifrost-http-darwin-arm64-v1.4.22.tar.gz");
        const FORMAT: ArchiveFormat = ArchiveFormat::TarGz;
    } else if #[cfg(all(target_os = "linux", target_arch = "x86_64"))] {
        const BIFROST_BYTES: &[u8] = include_bytes!("bifrost-http-linux-amd64-v1.4.22.tar.gz");
        const FORMAT: ArchiveFormat = ArchiveFormat::TarGz;
    } else if #[cfg(all(target_os = "windows", target_arch = "x86_64"))] {
        const BIFROST_BYTES: &[u8] = include_bytes!("bifrost-http-windows-amd64-v1.4.22.tar.gz");
        const FORMAT: ArchiveFormat = ArchiveFormat::TarGz;
    } else {
        const BIFROST_BYTES: &[u8] = &[];
        const FORMAT: ArchiveFormat = ArchiveFormat::TarGz;
    }
}

pub const BIFROST_VERSION: &str = "v1.4.22";

pub fn get_bifrost_asset() -> Option<BifrostAsset> {
    if BIFROST_BYTES.is_empty() {
        None
    } else {
        Some(BifrostAsset {
            bytes: BIFROST_BYTES,
            format: FORMAT,
            version: BIFROST_VERSION,
        })
    }
}
