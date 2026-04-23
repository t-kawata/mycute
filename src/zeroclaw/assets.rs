use cfg_if::cfg_if;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveFormat {
    TarGz,
    Zip,
}

pub struct ZeroClawAsset {
    pub bytes: &'static [u8],
    pub format: ArchiveFormat,
    pub version: &'static str,
}

cfg_if! {
    if #[cfg(all(target_os = "macos", target_arch = "aarch64"))] {
        const ZEROCLAW_BYTES: &[u8] = include_bytes!("zeroclaw-aarch64-apple-darwin.tar.gz");
        const FORMAT: ArchiveFormat = ArchiveFormat::TarGz;
    } else if #[cfg(all(target_os = "linux", target_arch = "x86_64"))] {
        const ZEROCLAW_BYTES: &[u8] = include_bytes!("zeroclaw-x86_64-unknown-linux-gnu.tar.gz");
        const FORMAT: ArchiveFormat = ArchiveFormat::TarGz;
    } else if #[cfg(target_os = "windows")] {
        const ZEROCLAW_BYTES: &[u8] = include_bytes!("zeroclaw-x86_64-pc-windows-msvc.zip");
        const FORMAT: ArchiveFormat = ArchiveFormat::Zip;
    } else {
        const ZEROCLAW_BYTES: &[u8] = &[];
        const FORMAT: ArchiveFormat = ArchiveFormat::TarGz;
    }
}

pub const ZEROCLAW_VERSION: &str = "v0.7.3";

pub fn get_zeroclaw_asset() -> Option<ZeroClawAsset> {
    if ZEROCLAW_BYTES.is_empty() {
        None
    } else {
        Some(ZeroClawAsset {
            bytes: ZEROCLAW_BYTES,
            format: FORMAT,
            version: ZEROCLAW_VERSION,
        })
    }
}
