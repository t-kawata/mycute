//! Windows Hotkey monitoring using rdev.
//!
//! This module provides hotkey monitoring for Windows platform.

use crate::constants::{HOTKEY_DOUBLE_TAP_MAX_MS, HOTKEY_DOUBLE_TAP_MIN_MS};
use crate::mycute_settings::HotkeyConfig;
use crate::types::HotkeyAction;
use rdev::{listen, Event, EventType, Key};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicU8, Ordering};
use tokio::sync::mpsc;

// Modifier bit flags
const MOD_ALT: u8 = 1 << 0;
const MOD_CTRL: u8 = 1 << 1;
const MOD_SHIFT: u8 = 1 << 2;
const MOD_WIN: u8 = 1 << 3;

// === Alt キー検出: GetAsyncKeyState ポーリング ===
// WH_KEYBOARD_LL は mycute フォーカス時に OS からコールバックが呼ばれないため使用不能。
// GetAsyncKeyState はフォーカスに依存せずキー状態を取得できる。
type DWORD = u32;

const VK_MENU: u16 = 0x12;
const WM_KEYDOWN: u32 = 0x0100;
const WM_KEYUP: u32 = 0x0101;
const WM_SYSKEYDOWN: u32 = 0x0104;
const WM_SYSKEYUP: u32 = 0x0105;

#[link(name = "user32")]
extern "system" {
    fn GetAsyncKeyState(v_key: i32) -> i16;
}

#[link(name = "kernel32")]
extern "system" {
    fn GetCurrentThreadId() -> DWORD;
}

// Track modifier states (bitmask)
static CURRENT_MODIFIERS: AtomicU8 = AtomicU8::new(0);
static LAST_ALT_PRESS_TIME: AtomicU64 = AtomicU64::new(0);
static MONITORING_ACTIVE: AtomicBool = AtomicBool::new(true);
static LISTENER_SPAWNED: AtomicBool = AtomicBool::new(false);
static PENDING_ALT_START: AtomicBool = AtomicBool::new(false);
static PENDING_ALT_FLUSH: AtomicBool = AtomicBool::new(false);
static RECORDING_ACTIVE: AtomicBool = AtomicBool::new(false);

// Raw WH_KEYBOARD_LL フックのライフサイクル管理
static RAW_HOOK_ACTIVE: AtomicBool = AtomicBool::new(false);
static RAW_HOOK_THREAD_ID: AtomicU32 = AtomicU32::new(0);

// Global sender for hotkey actions
lazy_static::lazy_static! {
    static ref HOTKEY_SENDER: std::sync::Mutex<Option<std::sync::mpsc::SyncSender<HotkeyAction>>> = std::sync::Mutex::new(None);
}

/// 録音中フラグを設定する（system.rs から呼び出す）
pub fn set_recording_active(active: bool) {
    RECORDING_ACTIVE.store(active, Ordering::SeqCst);
    if !active {
        // 録音終了時は保留中の Flush フラグをクリアする
        PENDING_ALT_FLUSH.store(false, Ordering::SeqCst);
    }
}

/// ホットキー監視を停止/一時停止する (Windows rdev の制限回避 + 終了処理)
pub fn stop_monitoring() {
    MONITORING_ACTIVE.store(false, Ordering::SeqCst);

    // 送信側チャンネルを明示的に破棄し、ハンドラーループを終了させる
    if let Ok(mut guard) = HOTKEY_SENDER.lock() {
        *guard = None;
    }

    // Alt ポーリングスレッドは MONITORING_ACTIVE フラグを見て自律終了する
}

/// Convert rdev Key to a string representation
fn key_to_string(key: Key) -> Option<&'static str> {
    match key {
        Key::KeyA => Some("KeyA"),
        Key::KeyB => Some("KeyB"),
        Key::KeyC => Some("KeyC"),
        Key::KeyD => Some("KeyD"),
        Key::KeyE => Some("KeyE"),
        Key::KeyF => Some("KeyF"),
        Key::KeyG => Some("KeyG"),
        Key::KeyH => Some("KeyH"),
        Key::KeyI => Some("KeyI"),
        Key::KeyJ => Some("KeyJ"),
        Key::KeyK => Some("KeyK"),
        Key::KeyL => Some("KeyL"),
        Key::KeyM => Some("KeyM"),
        Key::KeyN => Some("KeyN"),
        Key::KeyO => Some("KeyO"),
        Key::KeyP => Some("KeyP"),
        Key::KeyQ => Some("KeyQ"),
        Key::KeyR => Some("KeyR"),
        Key::KeyS => Some("KeyS"),
        Key::KeyT => Some("KeyT"),
        Key::KeyU => Some("KeyU"),
        Key::KeyV => Some("KeyV"),
        Key::KeyW => Some("KeyW"),
        Key::KeyX => Some("KeyX"),
        Key::KeyY => Some("KeyY"),
        Key::KeyZ => Some("KeyZ"),
        Key::Num1 => Some("Key1"),
        Key::Num2 => Some("Key2"),
        Key::Num3 => Some("Key3"),
        Key::Num4 => Some("Key4"),
        Key::Num5 => Some("Key5"),
        Key::Num6 => Some("Key6"),
        Key::Num7 => Some("Key7"),
        Key::Num8 => Some("Key8"),
        Key::Num9 => Some("Key9"),
        Key::Num0 => Some("Key0"),
        _ => None,
    }
}

struct HotkeyDef {
    key: String,
    modifiers: u8,
}

impl HotkeyDef {
    fn matches(&self, key: &str, current_modifiers: u8) -> bool {
        self.key == key && self.modifiers == current_modifiers
    }
}

/// Active hotkey configuration
struct ActiveHotkeys {
    correct: HotkeyDef,
    summarize: HotkeyDef,
}

impl ActiveHotkeys {
    fn from_config(config: &HotkeyConfig) -> Self {
        Self {
            correct: parse_hotkey(&config.correct),
            summarize: parse_hotkey(&config.summarize),
        }
    }
}

fn parse_hotkey(keys: &[String]) -> HotkeyDef {
    let mut modifiers = 0;
    let mut key = String::new();

    for k in keys {
        match k.as_str() {
            "Option" | "Alt" => modifiers |= MOD_ALT,
            "Control" | "Ctrl" => modifiers |= MOD_CTRL,
            "Shift" => modifiers |= MOD_SHIFT,
            "Command" | "Meta" | "Win" | "Windows" => modifiers |= MOD_WIN,
            s if s.starts_with("Key") => key = s.to_string(),
            _ => log::warn!("Unknown key/modifier in config: {}", k),
        }
    }
    HotkeyDef { key, modifiers }
}

lazy_static::lazy_static! {
    static ref ACTIVE_HOTKEYS: std::sync::Mutex<Option<ActiveHotkeys>> = std::sync::Mutex::new(None);
}

// === Raw WH_KEYBOARD_LL フックの実装 ===

/// Alt キー処理（rdev と GetAsyncKeyState ポーリングの両方から呼び出される）。
/// rdev を経由しないため、AttachThreadInput 問題の影響を受けない。
fn handle_raw_alt_event(msg: u32) {
    if !MONITORING_ACTIVE.load(Ordering::SeqCst) {
        return;
    }

    match msg {
        WM_KEYDOWN | WM_SYSKEYDOWN => {
            let old_mods = CURRENT_MODIFIERS.fetch_or(MOD_ALT, Ordering::SeqCst);
            if (old_mods & MOD_ALT) == 0 {
                // Recording 中は BufferFlush を保留し、KeyRelease で発火する
                if RECORDING_ACTIVE.load(Ordering::SeqCst) {
                    PENDING_ALT_FLUSH.store(true, Ordering::SeqCst);
                    return;
                }

                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64;
                let last = LAST_ALT_PRESS_TIME.load(Ordering::SeqCst);
                let diff = now.saturating_sub(last);
                log::debug!(
                    "[RawHook] Alt Down: diff={}, last={}, pending_start={}, recording={}",
                    diff, last, PENDING_ALT_START.load(Ordering::SeqCst),
                    RECORDING_ACTIVE.load(Ordering::SeqCst)
                );
                if diff > HOTKEY_DOUBLE_TAP_MIN_MS && diff < HOTKEY_DOUBLE_TAP_MAX_MS {
                    PENDING_ALT_START.store(true, Ordering::SeqCst);
                    LAST_ALT_PRESS_TIME.store(0, Ordering::SeqCst);
                } else {
                    LAST_ALT_PRESS_TIME.store(now, Ordering::SeqCst);
                }
            }
        }
        WM_KEYUP | WM_SYSKEYUP => {
            CURRENT_MODIFIERS.fetch_and(!MOD_ALT, Ordering::SeqCst);

            let pending_start = PENDING_ALT_START.load(Ordering::SeqCst);
            let pending_flush = PENDING_ALT_FLUSH.load(Ordering::SeqCst);
            log::debug!(
                "[RawHook] Alt Up: pending_start={}, pending_flush={}, recording={}",
                pending_start, pending_flush, RECORDING_ACTIVE.load(Ordering::SeqCst)
            );

            // 保留されていた Start アクションを Alt 解放時に発動する
            if PENDING_ALT_START.swap(false, Ordering::SeqCst) {
                if !MONITORING_ACTIVE.load(Ordering::SeqCst) {
                    return;
                }
                if let Ok(guard) = HOTKEY_SENDER.lock() {
                    if let Some(ref sender) = *guard {
                        let _ = sender.try_send(HotkeyAction::Start);
                    }
                }
            }

            // Recording 中の BufferFlush を Alt 解放時に発火する
            if PENDING_ALT_FLUSH.swap(false, Ordering::SeqCst) {
                if !MONITORING_ACTIVE.load(Ordering::SeqCst) {
                    return;
                }
                if let Ok(guard) = HOTKEY_SENDER.lock() {
                    if let Some(ref sender) = *guard {
                        let _ = sender.try_send(HotkeyAction::BufferFlush);
                    }
                }
            }
        }
        _ => {}
    }
}

/// GetAsyncKeyState による Alt キーポーリングループ。
/// WH_KEYBOARD_LL は mycute フォーカス時にコールバックが呼ばれないことが確認されたため
/// （v0.24.38 診断結果）、ポーリング方式に完全移行。フォーカスに影響されない。
fn alt_polling_thread() {
    log::info!("[AltPoll] Alt key polling thread started");
    RAW_HOOK_THREAD_ID.store(unsafe { GetCurrentThreadId() }, Ordering::SeqCst);
    RAW_HOOK_ACTIVE.store(true, Ordering::SeqCst);

    let mut prev_alt_down = false;

    while MONITORING_ACTIVE.load(Ordering::SeqCst) {
        unsafe {
            let alt_state = GetAsyncKeyState(VK_MENU as i32);
            let is_alt_down = (alt_state as u16 & 0x8000u16) != 0;

            if is_alt_down && !prev_alt_down {
                handle_raw_alt_event(WM_SYSKEYDOWN);
            } else if !is_alt_down && prev_alt_down {
                handle_raw_alt_event(WM_SYSKEYUP);
            }

            prev_alt_down = is_alt_down;
        }

        std::thread::sleep(std::time::Duration::from_millis(15));
    }

    RAW_HOOK_THREAD_ID.store(0, Ordering::SeqCst);
    RAW_HOOK_ACTIVE.store(false, Ordering::SeqCst);
    log::info!("[AltPoll] Alt key polling thread stopped");
}

/// Monitors for global hotkey events.
pub struct HotkeyMonitor {
    config: HotkeyConfig,
}

impl HotkeyMonitor {
    /// Create a new hotkey monitor with the given configuration.
    pub fn new(config: HotkeyConfig) -> Self {
        Self { config }
    }

    /// Start monitoring hotkeys in a separate thread.
    /// Returns a receiver for hotkey actions.
    pub fn start(self) -> mpsc::Receiver<HotkeyAction> {
        let (async_tx, async_rx) = mpsc::channel::<HotkeyAction>(10);
        let (sync_tx, sync_rx) = std::sync::mpsc::sync_channel::<HotkeyAction>(10);

        // Store the sender globally
        {
            let mut guard = HOTKEY_SENDER.lock().unwrap();
            *guard = Some(sync_tx);
        }

        // Store active hotkeys
        {
            let mut guard = ACTIVE_HOTKEYS.lock().unwrap();
            *guard = Some(ActiveHotkeys::from_config(&self.config));
        }

        // Bridge sync to async
        let async_tx_clone = async_tx.clone();
        std::thread::spawn(move || {
            while let Ok(action) = sync_rx.recv() {
                let _ = async_tx_clone.blocking_send(action);
            }
        });

        // Set monitoring active explicitly (in case it was disabled)
        MONITORING_ACTIVE.store(true, Ordering::SeqCst);

        // Start the rdev listener thread ONLY if not already started
        if !LISTENER_SPAWNED.swap(true, Ordering::SeqCst) {
            log::info!("Starting Windows hotkey listener thread (first time)");
            std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(100));

                if let Err(e) = listen(move |event: Event| {
                    handle_event(event);
                }) {
                    log::error!("Failed to start rdev listener: {:?}", e);
                }
            });
        } else {
            log::info!("Windows hotkey listener thread already running. Updated config/sender and resumed.");
        }

        // Start the Alt polling thread ONLY if not already running
        if !RAW_HOOK_ACTIVE.swap(true, Ordering::SeqCst) {
            log::info!("Starting Alt key polling thread");
            std::thread::spawn(move || {
                alt_polling_thread();
            });
        } else {
            log::info!("Alt key polling thread already running");
        }

        async_rx
    }
}

fn handle_event(event: Event) {
    match event.event_type {
        EventType::KeyPress(key) => {
            // Check if monitoring is active
            if !MONITORING_ACTIVE.load(Ordering::SeqCst) {
                return;
            }

            // Update modifiers
            match key {
                Key::Alt | Key::AltGr => {
                    // AltPoll: GetAsyncKeyState ポーリングスレッドも別経路で動作。
                    // rdev の Alt イベントは AttachThreadInput の問題によりフォーカス時に欠落するため、
                    // 両経路を共存させ二重発火はタイミング差により防止する。
                    let old_mods = CURRENT_MODIFIERS.fetch_or(MOD_ALT, Ordering::SeqCst);
                    if (old_mods & MOD_ALT) == 0 {
                        // Recording 中は BufferFlush を保留し、KeyRelease で発火する
                        if RECORDING_ACTIVE.load(Ordering::SeqCst) {
                            PENDING_ALT_FLUSH.store(true, Ordering::SeqCst);
                            return;
                        }

                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_millis() as u64;
                        let last = LAST_ALT_PRESS_TIME.load(Ordering::SeqCst);
                        let diff = now.saturating_sub(last);
                        log::debug!(
                            "[AltDiag] KeyPress: diff={}, last={}, pending_start={}, recording={}",
                            diff, last, PENDING_ALT_START.load(Ordering::SeqCst),
                            RECORDING_ACTIVE.load(Ordering::SeqCst)
                        );
                        if diff > HOTKEY_DOUBLE_TAP_MIN_MS && diff < HOTKEY_DOUBLE_TAP_MAX_MS {
                            PENDING_ALT_START.store(true, Ordering::SeqCst);
                            LAST_ALT_PRESS_TIME.store(0, Ordering::SeqCst);
                        } else {
                            LAST_ALT_PRESS_TIME.store(now, Ordering::SeqCst);
                        }
                    }
                    return;
                }
                Key::ControlLeft | Key::ControlRight => {
                    CURRENT_MODIFIERS.fetch_or(MOD_CTRL, Ordering::SeqCst);
                    return;
                }
                Key::ShiftLeft | Key::ShiftRight => {
                    CURRENT_MODIFIERS.fetch_or(MOD_SHIFT, Ordering::SeqCst);
                    return;
                }
                Key::MetaLeft | Key::MetaRight => {
                    CURRENT_MODIFIERS.fetch_or(MOD_WIN, Ordering::SeqCst);
                    return;
                }
                _ => {}
            }

            // Check for hotkeys
            if let Some(key_str) = key_to_string(key) {
                let current_mods = CURRENT_MODIFIERS.load(Ordering::SeqCst);

                // Only process hotkeys if at least one modifier is pressed,
                // or if needed (though usually hotkeys have modifiers).
                // Our parsing logic requires specific modifiers, so 0 modifiers is also a valid state if config says so.

                let action = {
                    let guard = ACTIVE_HOTKEYS.lock().unwrap();
                    if let Some(ref hotkeys) = *guard {
                        if hotkeys.correct.matches(key_str, current_mods) {
                            Some(HotkeyAction::Correct)
                        } else if hotkeys.summarize.matches(key_str, current_mods) {
                            Some(HotkeyAction::Summarize)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                };

                if let Some(action) = action {
                    if let Ok(guard) = HOTKEY_SENDER.lock() {
                        if let Some(ref sender) = *guard {
                            let _ = sender.try_send(action);
                        }
                    }
                    return;
                }
            }

            // Control/Meta held = shortcut in use → no commit
            let current_mods = CURRENT_MODIFIERS.load(Ordering::SeqCst);
            if (current_mods & (MOD_CTRL | MOD_WIN)) != 0 {
                return;
            }
        }
        EventType::KeyRelease(key) => {
            // Update modifiers
            match key {
                Key::Alt | Key::AltGr => {
                    CURRENT_MODIFIERS.fetch_and(!MOD_ALT, Ordering::SeqCst);

                    let pending_start = PENDING_ALT_START.load(Ordering::SeqCst);
                    let pending_flush = PENDING_ALT_FLUSH.load(Ordering::SeqCst);
                    log::debug!(
                        "[AltDiag] KeyRelease: pending_start={}, pending_flush={}, recording={}",
                        pending_start, pending_flush, RECORDING_ACTIVE.load(Ordering::SeqCst)
                    );

                    // 保留されていた Start アクションを Alt キーが離された瞬間に発動する
                    if PENDING_ALT_START.swap(false, Ordering::SeqCst) {
                        if !MONITORING_ACTIVE.load(Ordering::SeqCst) {
                            return;
                        }
                        if let Ok(guard) = HOTKEY_SENDER.lock() {
                            if let Some(ref sender) = *guard {
                                let _ = sender.try_send(HotkeyAction::Start);
                            }
                        }
                    }

                    // Recording 中は BufferFlush を Alt 解放時に発火する
                    if PENDING_ALT_FLUSH.swap(false, Ordering::SeqCst) {
                        if !MONITORING_ACTIVE.load(Ordering::SeqCst) {
                            return;
                        }
                        if let Ok(guard) = HOTKEY_SENDER.lock() {
                            if let Some(ref sender) = *guard {
                                let _ = sender.try_send(HotkeyAction::BufferFlush);
                            }
                        }
                    }
                }
                Key::ControlLeft | Key::ControlRight => {
                    CURRENT_MODIFIERS.fetch_and(!MOD_CTRL, Ordering::SeqCst);
                }
                Key::ShiftLeft | Key::ShiftRight => {
                    CURRENT_MODIFIERS.fetch_and(!MOD_SHIFT, Ordering::SeqCst);
                }
                Key::MetaLeft | Key::MetaRight => {
                    CURRENT_MODIFIERS.fetch_and(!MOD_WIN, Ordering::SeqCst);
                }
                _ => {}
            }
        }
        EventType::ButtonPress(_) => {
            // 自動コミット廃止により何もしない
        }
        _ => {}
    }
}
