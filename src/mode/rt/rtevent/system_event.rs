use serde::{Serialize, Deserialize};
use super::proxy_leak::ProxyLeakPayload;

/// Mycute OS システムイベント定義 (MycuteEventBus Protocol)
/// Rust Kernel (Brain) -> Frontend Apps (Organs)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", content = "payload", rename_all = "kebab-case")]
pub enum MycuteSystemEvent {
    /// プロキシ漏洩警告
    #[serde(rename = "mycute://kernel/proxy-leak")]
    ProxyLeak(ProxyLeakPayload),

    // Future expansions:
    // #[serde(rename = "mycute://kernel/stt-status")]
    // SttStatus(SttStatusPayload),
}
