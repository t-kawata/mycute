//! Common types used throughout the voice dictation tool.

use crate::constants::*;
use crate::mode::rt::rtres::mycute_res::LicenseSummary;
use crate::mycute_settings::SttEngine;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub enum SttEvent {
    /// 部分的な認識結果（表示用）
    PartialResult(String, u64),
    /// 最終的な認識結果
    FinalResult(String, u64),
    /// 開始
    Started,
    /// エラー
    Error(String),
    /// 停止
    Stopped,
    /// 装飾タスクの異常状態による表示の強制クリーンアップ（認識は継続）
    ForceClearDecoration,
    /// 録音準備完了（ハードウェア開放完了）
    Ready,
    /// 最終補正レイヤー処理開始（入力ロック用）
    PostCorrectionStarted,
    /// 最終補正レイヤー処理完了（入力ロック解除用）
    PostCorrectionFinished,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TargetPlatform {
    #[serde(rename = "macos")]
    Macos,
    #[serde(rename = "windows")]
    Windows,
}

impl TargetPlatform {
    pub fn as_str(&self) -> &'static str {
        match self {
            TargetPlatform::Macos => "macos",
            TargetPlatform::Windows => "windows",
        }
    }

    pub fn current() -> Self {
        if cfg!(target_os = "macos") {
            TargetPlatform::Macos
        } else {
            TargetPlatform::Windows
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HotkeyAction {
    Start,
    Commit, // 実際のホットキーとしては存在しないが、内部で使用される
    Correct,
    Summarize,
    BufferStart,
    BufferFlush,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum LocaleCode {
    #[default]
    Ja,
    En,
}

impl LocaleCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            LocaleCode::Ja => "ja",
            LocaleCode::En => "en",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            LocaleCode::Ja => "Japanese",
            LocaleCode::En => "English",
        }
    }

    /// Returns the language token for Sherpa01 multilingual models (e.g., "<JA>", "<EN>")
    pub fn sherpa01_language_token(&self) -> &'static str {
        match self {
            LocaleCode::Ja => "<JA>",
            LocaleCode::En => "<EN>",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "en" | "en-us" | "english" => LocaleCode::En,
            "ja" | "japanese" => LocaleCode::Ja,
            _ => {
                log::warn!("Unknown locale '{}', falling back to default.", s);
                LocaleCode::default()
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LlmAction {
    #[serde(rename = "correct")]
    Correct,
    #[serde(rename = "summarize")]
    Summarize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InputMode {
    #[serde(rename = "realtime")]
    RealTime,
    #[serde(rename = "buffered")]
    Buffered,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppState {
    Idle,
    Recording,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TauriEvent {
    SttPartial,
    SttFinal,
    SttUpdate,
    SttCommit,
    AppStatus,
    AppError,
    AppState,
    AppLocaleChanged,
    AppSttEngineChanged,
    // AppLlmsChanged は LMGW 移行に伴い廃止済み
    AppOwnerStatusChanged,
    AppCaStatusChanged,
    AppLicensesChanged,
}

impl TauriEvent {
    pub fn as_str(&self) -> &'static str {
        match self {
            TauriEvent::SttPartial => EVENT_STT_PARTIAL,
            TauriEvent::SttFinal => EVENT_STT_FINAL,
            TauriEvent::SttUpdate => EVENT_STT_UPDATE,
            TauriEvent::SttCommit => EVENT_STT_COMMIT,
            TauriEvent::AppStatus => EVENT_APP_STATUS,
            TauriEvent::AppError => EVENT_APP_ERROR,
            TauriEvent::AppState => EVENT_APP_STATE,
            TauriEvent::AppLocaleChanged => EVENT_APP_LOCALE_CHANGED,
            TauriEvent::AppSttEngineChanged => EVENT_APP_STT_ENGINE_CHANGED,
            TauriEvent::AppOwnerStatusChanged => EVENT_APP_OWNER_STATUS_CHANGED,
            TauriEvent::AppCaStatusChanged => EVENT_APP_CA_STATUS_CHANGED,
            TauriEvent::AppLicensesChanged => EVENT_APP_LICENSES_CHANGED,
        }
    }
}

/// Callback function type for receiving raw audio data from native layer.
/// Args:
/// - samples: Pointer to the float array of audio samples.
/// - count: Number of samples in the array.
/// - sample_rate: Sampling rate of the audio data.
pub type AudioDataCallback =
    unsafe extern "C" fn(samples: *const f32, count: u32, sample_rate: u32);

// ============================================================
// Tauri Event Payloads
// ============================================================

/// Payload for `stt-partial` and `stt-final` events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SttPayload {
    pub text: String,
    pub seq: u64,
}

/// Payload for `stt-update` event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SttUpdatePayload {
    pub text: String,
}

/// Payload for `app-status` event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppStatusPayload {
    pub status: String,
}

/// Payload for `app-error` event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppErrorPayload {
    pub error: String,
}

/// Payload for `app-state` event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppStatePayload {
    pub state: String,
}

/// Payload for `app-locale-changed` event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppLocaleChangedPayload {
    pub locale: LocaleCode,
}

/// Payload for `app-stt-engine-changed` event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSttEngineChangedPayload {
    pub engine: SttEngine,
}

// AppLlmsChangedPayload は LMGW 移行に伴い廃止済み

/// Payload for `app-owner-status-changed` event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppOwnerStatusChangedPayload {
    pub is_active: bool,
}

/// Payload for `app-ca-status-changed` event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppCaStatusChangedPayload {
    pub ca_token: Option<String>,
}

/// Payload for `app-licenses-changed` event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppLicensesChangedPayload {
    pub licenses: Vec<LicenseSummary>,
}

// ============================================================
// RT-CL Process IPC Events (SSE)
// ============================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum EventKind {
    LocaleChanged(LocaleCode),
    SttEngineChanged(SttEngine),
    // LlmsChanged は LMGW 移行に伴い廃止済み
    OwnerStatusChanged(bool),
    CaStatusChanged(Option<String>),
    LicensesChanged(Vec<LicenseSummary>),
    SystemMessage(String),
    Heartbeat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InternalEvent {
    pub seq: u64,
    pub kind: EventKind,
}

// ============================================================
// RT-CL Process IPC Events (WebSocket)
// ============================================================

/// サーバー(RT)からクライアント(CL)へのメッセージ
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum WsServerMessage {
    Event(InternalEvent),
    HandshakeRequest { challenge: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WsClientRole {
    #[serde(rename = "cl")]
    CL,
    #[serde(rename = "unknown")]
    Unknown,
}

impl Default for WsClientRole {
    fn default() -> Self {
        WsClientRole::Unknown
    }
}

/// クライアント(CL)からサーバー(RT)へのメッセージ
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum WsClientMessage {
    HandshakeResponse {
        challenge: String,
        /// クライアントが自身を識別するための UUID。再接続時も同一の値を維持する。
        client_id: String,
        #[serde(default)]
        client_role: WsClientRole,
    },
}

/// WebSocket 疎通状態確認APIのレスポンス
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct WsStatusRes {
    pub is_cl_connected: bool,
    pub active_clients: usize,
}
