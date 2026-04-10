use cfg_if::cfg_if;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveFormat {
    TarGz,
    Zip,
}

pub struct NodeAsset {
    pub bytes: &'static [u8],
    pub format: ArchiveFormat,
    pub version: &'static str,
}

cfg_if! {
    if #[cfg(all(target_os = "macos", target_arch = "aarch64"))] {
        const NODE_BYTES: &[u8] = include_bytes!("node-v25.9.0-darwin-arm64.tar.gz");
        const FORMAT: ArchiveFormat = ArchiveFormat::TarGz;
    } else if #[cfg(all(target_os = "linux", target_arch = "x86_64"))] {
        // Build環境での cmake 不在を考慮し、Linux版も .tar.gz に変換して埋め込みます
        const NODE_BYTES: &[u8] = include_bytes!("node-v25.9.0-linux-x64.tar.gz");
        const FORMAT: ArchiveFormat = ArchiveFormat::TarGz;
    } else if #[cfg(target_os = "windows")] {
        const NODE_BYTES: &[u8] = include_bytes!("node-v25.9.0-win-x64.zip");
        const FORMAT: ArchiveFormat = ArchiveFormat::Zip;
    } else {
        const NODE_BYTES: &[u8] = &[];
        const FORMAT: ArchiveFormat = ArchiveFormat::TarGz; // Dummy
    }
}

pub const NODE_VERSION: &str = "v25.9.0";

pub fn get_node_asset() -> Option<NodeAsset> {
    if NODE_BYTES.is_empty() {
        None
    } else {
        Some(NodeAsset {
            bytes: NODE_BYTES,
            format: FORMAT,
            version: NODE_VERSION,
        })
    }
}
