//! Windows Hotkey monitoring using rdev and GetAsyncKeyState polling.
//!
//! Two paths exist to detect Alt key events:
//! - rdev listener thread: works when the mycute window does NOT have focus.
//! - GetAsyncKeyState polling thread: works when the mycute window DOES have focus.
//! Double-fire prevention is achieved via shared atomic flags.

use crate::constants::{HOTKEY_DOUBLE_TAP_MAX_MS, HOTKEY_DOUBLE_TAP_MIN_MS};
use crate::mycute_settings::HotkeyConfig;
use crate::types::HotkeyAction;
use rdev::{listen, Event, EventType, Key};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};
use tokio::sync::mpsc;

// Modifier bit flags
pub(crate) const MOD_ALT: u8 = 1 << 0;
pub(crate) const MOD_CTRL: u8 = 1 << 1;
pub(crate) const MOD_SHIFT: u8 = 1 << 2;
pub(crate) const MOD_WIN: u8 = 1 << 3;

pub(crate) const VK_MENU: u16 = 0x12;
pub(crate) const VK_CONTROL: u16 = 0x11;

#[link(name = "user32")]
extern "system" {
    fn GetAsyncKeyState(v_key: i32) -> i16;
}

// Track modifier states (bitmask)
pub(crate) static CURRENT_MODIFIERS: AtomicU8 = AtomicU8::new(0);
pub(crate) static LAST_ALT_PRESS_TIME: AtomicU64 = AtomicU64::new(0);
pub(crate) static MONITORING_ACTIVE: AtomicBool = AtomicBool::new(true);
static LISTENER_SPAWNED: AtomicBool = AtomicBool::new(false);
pub(crate) static PENDING_ALT_START: AtomicBool = AtomicBool::new(false);
pub(crate) static PENDING_ALT_FLUSH: AtomicBool = AtomicBool::new(false);
pub(crate) static RECORDING_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Ctrl+Alt コンボ検出の前回発火時刻（ミリ秒）。hotkey_win_hook と共有する。
pub(crate) static ORCHESTRATOR_LAST_FIRE_MS: AtomicU64 = AtomicU64::new(0);
/// OrchestratorInput 誤発火防止クールダウン（ミリ秒）
pub(crate) const ORCHESTRATOR_COOLDOWN_MS: u64 = 150;
/// Ctrl+Alt コンボ検出の上昇エッジ検出フラグ（hotkey_win_hook と共有）
pub(crate) static ORCHESTRATOR_COMBO_ACTIVE: AtomicBool = AtomicBool::new(false);

// GetAsyncKeyState ポーリングスレッドのライフサイクル管理
static POLLING_ACTIVE: AtomicBool = AtomicBool::new(false);

// Global sender for hotkey actions
lazy_static::lazy_static! {
    pub(crate) static ref HOTKEY_SENDER: std::sync::Mutex<Option<std::sync::mpsc::SyncSender<HotkeyAction>>> = std::sync::Mutex::new(None);
}

/// 録音中フラグを設定する（system.rs から呼び出す）
pub fn set_recording_active(active: bool) {
    RECORDING_ACTIVE.store(active, Ordering::SeqCst);
    if !active {
        // 録音終了時は保留中の Flush フラグをクリアする
        PENDING_ALT_FLUSH.store(false, Ordering::SeqCst);
    }
}

/// ホットキー監視を停止/一時停止する。
/// MONITORING_ACTIVE を false に設定し、ポーリングスレッドを終了させる。
pub fn stop_monitoring() {
    MONITORING_ACTIVE.store(false, Ordering::SeqCst);

    // 送信側チャンネルを明示的に破棄し、ハンドラーループを終了させる
    if let Ok(mut guard) = HOTKEY_SENDER.lock() {
        *guard = None;
    }
    // ポーリングスレッドは MONITORING_ACTIVE のチェックにより自律終了する
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

pub(crate) struct HotkeyDef {
    pub(crate) key: String,
    pub(crate) modifiers: u8,
}

impl HotkeyDef {
    pub(crate) fn matches(&self, key: &str, current_modifiers: u8) -> bool {
        self.key == key && self.modifiers == current_modifiers
    }
}

/// Active hotkey configuration
pub(crate) struct ActiveHotkeys {
    pub(crate) correct: HotkeyDef,
    pub(crate) summarize: HotkeyDef,
}

impl ActiveHotkeys {
    pub(crate) fn from_config(config: &HotkeyConfig) -> Self {
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
    pub(crate) static ref ACTIVE_HOTKEYS: std::sync::Mutex<Option<ActiveHotkeys>> = std::sync::Mutex::new(None);
}

/// GetAsyncKeyState ポーリングスレッド。
///
/// フォーカス時に rdev が Alt イベントを取得できない問題の対策として、
/// 専用スレッドで GetAsyncKeyState をポーリングし、Alt キーの押下/解放を検出する。
/// rdev と同一の atomic フラグを共有して二重発火を防止する。
fn alt_monitor_thread() {
    log::info!("[AltMonitor] GetAsyncKeyState polling started");
    let mut alt_was_pressed = false;
    let mut ctrl_was_pressed = false;

    while MONITORING_ACTIVE.load(Ordering::SeqCst) {
        let state = unsafe { GetAsyncKeyState(VK_MENU as i32) };
        let is_pressed = (state as u16 & 0x8000u16) != 0;

        // --- Ctrl state ---
        let ctrl_state = unsafe { GetAsyncKeyState(VK_CONTROL as i32) };
        let ctrl_is_pressed = (ctrl_state as u16 & 0x8000u16) != 0;

        if is_pressed && !alt_was_pressed {
            // --- Key DOWN transition ---
            let old_mods = CURRENT_MODIFIERS.fetch_or(MOD_ALT, Ordering::SeqCst);
            if (old_mods & MOD_ALT) == 0 {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64;
                let last = LAST_ALT_PRESS_TIME.load(Ordering::SeqCst);
                let diff = now.saturating_sub(last);
                log::debug!(
                    "[AltMonitor] Alt Down: diff={}, last={}, pending_start={}, recording={}",
                    diff, last, PENDING_ALT_START.load(Ordering::SeqCst),
                    RECORDING_ACTIVE.load(Ordering::SeqCst)
                );
                if diff > HOTKEY_DOUBLE_TAP_MIN_MS && diff < HOTKEY_DOUBLE_TAP_MAX_MS {
                    // ダブルタップ確定: 録音中なら Flush、非録音なら Start
                    if RECORDING_ACTIVE.load(Ordering::SeqCst) {
                        PENDING_ALT_FLUSH.store(true, Ordering::SeqCst);
                    } else {
                        PENDING_ALT_START.store(true, Ordering::SeqCst);
                    }
                    LAST_ALT_PRESS_TIME.store(0, Ordering::SeqCst);
                } else {
                    LAST_ALT_PRESS_TIME.store(now, Ordering::SeqCst);
                }
            }
        } else if !is_pressed && alt_was_pressed {
            // --- Key UP transition ---
            CURRENT_MODIFIERS.fetch_and(!MOD_ALT, Ordering::SeqCst);

            let pending_start = PENDING_ALT_START.load(Ordering::SeqCst);
            let pending_flush = PENDING_ALT_FLUSH.load(Ordering::SeqCst);
            log::debug!(
                "[AltMonitor] Alt Up: pending_start={}, pending_flush={}, recording={}",
                pending_start, pending_flush, RECORDING_ACTIVE.load(Ordering::SeqCst)
            );

            if PENDING_ALT_START.swap(false, Ordering::SeqCst) {
                if !MONITORING_ACTIVE.load(Ordering::SeqCst) {
                    alt_was_pressed = is_pressed;
                    std::thread::sleep(std::time::Duration::from_millis(15));
                    continue;
                }
                if let Ok(guard) = HOTKEY_SENDER.lock() {
                    if let Some(ref sender) = *guard {
                        let _ = sender.try_send(HotkeyAction::Start);
                    }
                }
            }

            if PENDING_ALT_FLUSH.swap(false, Ordering::SeqCst) {
                if !MONITORING_ACTIVE.load(Ordering::SeqCst) {
                    alt_was_pressed = is_pressed;
                    std::thread::sleep(std::time::Duration::from_millis(15));
                    continue;
                }
                if let Ok(guard) = HOTKEY_SENDER.lock() {
                    if let Some(ref sender) = *guard {
                        let _ = sender.try_send(HotkeyAction::BufferFlush);
                    }
                }
            }
        }

        // --- Ctrl transition handling ---
        if ctrl_is_pressed && !ctrl_was_pressed {
            CURRENT_MODIFIERS.fetch_or(MOD_CTRL, Ordering::SeqCst);
            check_orchestrator_combo();
        } else if !ctrl_is_pressed && ctrl_was_pressed {
            CURRENT_MODIFIERS.fetch_and(!MOD_CTRL, Ordering::SeqCst);
        }

        alt_was_pressed = is_pressed;
        ctrl_was_pressed = ctrl_is_pressed;
        std::thread::sleep(std::time::Duration::from_millis(15));
    }

    POLLING_ACTIVE.store(false, Ordering::SeqCst);
    log::info!("[AltMonitor] GetAsyncKeyState polling stopped");
}

/// Control+Alt 同時押しを検出し、OrchestratorInput を送信する。
/// hotkey_win_hook と同一の共有フラグ/クールダウンを参照し、二重発火を防止する。
pub(crate) fn check_orchestrator_combo() {
    let mods = CURRENT_MODIFIERS.load(Ordering::SeqCst);
    let both_held = (mods & (MOD_CTRL | MOD_ALT)) == (MOD_CTRL | MOD_ALT);
    if both_held && !ORCHESTRATOR_COMBO_ACTIVE.swap(true, Ordering::SeqCst) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let last = ORCHESTRATOR_LAST_FIRE_MS.load(Ordering::SeqCst);
        if now.saturating_sub(last) > ORCHESTRATOR_COOLDOWN_MS {
            ORCHESTRATOR_LAST_FIRE_MS.store(now, Ordering::SeqCst);
            log::debug!("[OrchestratorCombo] Ctrl+Alt detected");
            if let Ok(guard) = HOTKEY_SENDER.lock() {
                if let Some(ref sender) = *guard {
                    let _ = sender.try_send(HotkeyAction::OrchestratorInput);
                }
            }
        } else {
            ORCHESTRATOR_COMBO_ACTIVE.store(false, Ordering::SeqCst);
        }
    } else if !both_held {
        ORCHESTRATOR_COMBO_ACTIVE.store(false, Ordering::SeqCst);
    }
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

        // Start the GetAsyncKeyState Alt polling thread ONLY if not already running
        if !POLLING_ACTIVE.swap(true, Ordering::SeqCst) {
            log::info!("Starting GetAsyncKeyState Alt polling thread");
            std::thread::spawn(move || {
                alt_monitor_thread();
            });
        } else {
            log::info!("GetAsyncKeyState Alt polling thread already running");
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
                    // Polling: GetAsyncKeyState による Alt 監視スレッドも別スレッドで動作。
                    // rdev の Alt イベントはフォーカス時に欠落するため、
                    // 両経路を共存させ二重発火は atomic フラグにより防止する。
                    let old_mods = CURRENT_MODIFIERS.fetch_or(MOD_ALT, Ordering::SeqCst);
                    if (old_mods & MOD_ALT) == 0 {
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
                            // ダブルタップ確定: 録音中なら Flush、非録音なら Start
                            if RECORDING_ACTIVE.load(Ordering::SeqCst) {
                                PENDING_ALT_FLUSH.store(true, Ordering::SeqCst);
                            } else {
                                PENDING_ALT_START.store(true, Ordering::SeqCst);
                            }
                            LAST_ALT_PRESS_TIME.store(0, Ordering::SeqCst);
                        } else {
                            LAST_ALT_PRESS_TIME.store(now, Ordering::SeqCst);
                        }
                    }
                    return;
                }
                Key::ShiftLeft | Key::ShiftRight => {
                    CURRENT_MODIFIERS.fetch_or(MOD_SHIFT, Ordering::SeqCst);
                    return;
                }
                Key::ControlLeft | Key::ControlRight => {
                    CURRENT_MODIFIERS.fetch_or(MOD_CTRL, Ordering::SeqCst);
                    check_orchestrator_combo();
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
                Key::ShiftLeft | Key::ShiftRight => {
                    CURRENT_MODIFIERS.fetch_and(!MOD_SHIFT, Ordering::SeqCst);
                }
                Key::ControlLeft | Key::ControlRight => {
                    CURRENT_MODIFIERS.fetch_and(!MOD_CTRL, Ordering::SeqCst);
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
