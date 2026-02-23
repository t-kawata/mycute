//! Global hotkey monitoring using CGEventTap.
//!
//! This module intercepts global keyboard events and blocks Option+S/C
//! from propagating to other applications.

#[link(name = "CoreGraphics", kind = "framework")]
#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {}

use crate::stt_config::HotkeyConfig;
use crate::types::HotkeyAction;
use std::ffi::c_void;
use std::ptr;
use tokio::sync::mpsc;

// CoreGraphics types
type CGEventRef = *mut c_void;
type CGEventTapProxy = *mut c_void;
type CGEventType = u32;
type CGEventFlags = u64;
type CGKeyCode = u16;

// Event types
const K_CG_EVENT_KEY_DOWN: CGEventType = 10;
const K_CG_EVENT_KEY_UP: CGEventType = 11;
const K_CG_EVENT_FLAGS_CHANGED: CGEventType = 12;

// Mouse events
const K_CG_EVENT_LEFT_MOUSE_DOWN: CGEventType = 1;
const K_CG_EVENT_RIGHT_MOUSE_DOWN: CGEventType = 3;
const K_CG_EVENT_OTHER_MOUSE_DOWN: CGEventType = 25;

// Event flags
const K_CG_EVENT_FLAG_MASK_ALTERNATE: CGEventFlags = 0x00080000; // Option key
const K_CG_EVENT_FLAG_MASK_CONTROL: CGEventFlags = 0x00040000; // Control key

// Key codes
const K_VK_C: CGKeyCode = 8;
const K_VK_H: CGKeyCode = 4;
const K_VK_M: CGKeyCode = 46;
const K_VK_L: CGKeyCode = 37;
const K_VK_B: CGKeyCode = 11;
const K_VK_F: CGKeyCode = 3;
const K_VK_J: CGKeyCode = 38;
const K_VK_U: CGKeyCode = 32;

// CFRunLoop constants

extern "C" {
    fn CGEventTapCreate(
        tap: u32,
        place: u32,
        options: u32,
        events_of_interest: u64,
        callback: extern "C" fn(
            CGEventTapProxy,
            CGEventType,
            CGEventRef,
            *mut c_void,
        ) -> CGEventRef,
        user_info: *mut c_void,
    ) -> *mut c_void;

    fn CFMachPortCreateRunLoopSource(
        allocator: *const c_void,
        port: *mut c_void,
        order: i64,
    ) -> *mut c_void;

    fn CFRunLoopGetCurrent() -> *mut c_void;
    fn CFRunLoopAddSource(rl: *mut c_void, source: *mut c_void, mode: *const c_void);
    fn CFRunLoopRun();
    fn CGEventGetFlags(event: CGEventRef) -> CGEventFlags;
    fn CGEventGetIntegerValueField(event: CGEventRef, field: u32) -> i64;
    fn CGEventTapEnable(tap: *mut c_void, enable: bool);
    fn CFRunLoopStop(rl: *mut c_void);
}

// Keyboard virtual key code field
const K_CG_KEYBOARD_EVENT_KEYCODE: u32 = 9;

struct ActiveHotkeys {
    correct_key: CGKeyCode,
    correct_flags: CGEventFlags,
    summarize_key: CGKeyCode,
    summarize_flags: CGEventFlags,
    toggle_locale_key: CGKeyCode,
    toggle_locale_flags: CGEventFlags,
    buffer_start_key: CGKeyCode,
    buffer_start_flags: CGEventFlags,
    buffer_flush_key: CGKeyCode,
    buffer_flush_flags: CGEventFlags,
    settings_key: CGKeyCode,
    settings_flags: CGEventFlags,
    help_key: CGKeyCode,
    help_flags: CGEventFlags,
    usage_stats_key: CGKeyCode,
    usage_stats_flags: CGEventFlags,
}

// Global active hotkeys
static mut ACTIVE_HOTKEYS: ActiveHotkeys = ActiveHotkeys {
    correct_key: K_VK_H,
    correct_flags: K_CG_EVENT_FLAG_MASK_ALTERNATE,
    summarize_key: K_VK_M,
    summarize_flags: K_CG_EVENT_FLAG_MASK_ALTERNATE,
    toggle_locale_key: K_VK_L,
    toggle_locale_flags: K_CG_EVENT_FLAG_MASK_ALTERNATE,
    buffer_start_key: K_VK_B,
    buffer_start_flags: K_CG_EVENT_FLAG_MASK_ALTERNATE,
    buffer_flush_key: K_VK_F,
    buffer_flush_flags: K_CG_EVENT_FLAG_MASK_ALTERNATE,
    settings_key: K_VK_J,
    settings_flags: K_CG_EVENT_FLAG_MASK_ALTERNATE,
    help_key: K_VK_C,
    help_flags: K_CG_EVENT_FLAG_MASK_ALTERNATE,
    usage_stats_key: K_VK_U,
    usage_stats_flags: K_CG_EVENT_FLAG_MASK_ALTERNATE,
};

// ホットキーアクション用のグローバル送信者 (初期化時に設定)
static mut HOTKEY_SENDER: Option<std::sync::mpsc::SyncSender<HotkeyAction>> = None;
// Ctrl+Key の組み合わせを確実に検出するために Control キーの状態を追跡する
static mut CONTROL_KEY_DOWN: bool = false;
static mut OPTION_KEY_DOWN: bool = false;
static mut LAST_OPTION_PRESS_TIME: u128 = 0;
// バッファモードがアクティブなときに自動コミットを抑制するためのフラグ
static mut BUFFER_MODE_ACTIVE: bool = false;
// アプリケーションが現在キーボード入力を注入しているかどうかを追跡するためのフラグ（対称性のため）
static mut IS_TYPING: bool = false;
// 停止用のグローバルランループ参照
static mut RUN_LOOP: Option<*mut c_void> = None;

/// バッファモードがアクティブかどうかを設定する（キー/マウスイベントでの自動コミットを抑制する）
pub fn set_buffer_mode(active: bool) {
    unsafe {
        BUFFER_MODE_ACTIVE = active;
    }
}

/// アプリケーションが現在キーボード入力を注入しているかどうかを設定する
pub fn set_typing_mode(active: bool) {
    unsafe {
        IS_TYPING = active;
    }
}

/// Callback function for CGEventTap
extern "C" fn event_tap_callback(
    _proxy: CGEventTapProxy,
    event_type: CGEventType,
    event: CGEventRef,
    _user_info: *mut c_void,
) -> CGEventRef {
    // Marker used by KeyboardInjector to identify self-generated events
    const MYCUTE_EVENT_ID: i64 = 0x4D594355;
    // Field ID for event source user data
    const K_CG_EVENT_SOURCE_USER_DATA: u32 = 42;

    unsafe {
        // Track Control key state from FLAGS_CHANGED events
        if event_type == K_CG_EVENT_FLAGS_CHANGED {
            let flags = CGEventGetFlags(event);
            CONTROL_KEY_DOWN = (flags & K_CG_EVENT_FLAG_MASK_CONTROL) != 0;

            let is_option_down = (flags & K_CG_EVENT_FLAG_MASK_ALTERNATE) != 0;
            if is_option_down && !OPTION_KEY_DOWN {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis();

                let diff = now.saturating_sub(LAST_OPTION_PRESS_TIME);
                if diff > 10 && diff < 500 {
                    if let Some(ref sender) = HOTKEY_SENDER {
                        let _ = sender.try_send(HotkeyAction::Start);
                    }
                    LAST_OPTION_PRESS_TIME = 0;
                } else {
                    LAST_OPTION_PRESS_TIME = now;
                }
            }
            OPTION_KEY_DOWN = is_option_down;

            return event;
        }

        // Ignore events generated by our own KeyboardInjector
        let user_data = CGEventGetIntegerValueField(event, K_CG_EVENT_SOURCE_USER_DATA);
        if user_data == MYCUTE_EVENT_ID {
            return event;
        }

        // Skip if Control is held (for Ctrl+C, Ctrl+Z, etc.)
        if CONTROL_KEY_DOWN {
            return event;
        }

        // Check for specific hotkeys
        let flags = CGEventGetFlags(event);
        let keycode = CGEventGetIntegerValueField(event, K_CG_KEYBOARD_EVENT_KEYCODE) as CGKeyCode;

        if event_type == K_CG_EVENT_KEY_DOWN {
            LAST_OPTION_PRESS_TIME = 0; // 他のキーが押された場合はダブルクリック判定をリセット
            let mut action = None;

            // Simple bitmask check for flags. We check if the required flags are present.
            // Note: We only check the specific masks we care about.
            if (flags & ACTIVE_HOTKEYS.correct_flags) == ACTIVE_HOTKEYS.correct_flags
                && keycode == ACTIVE_HOTKEYS.correct_key
            {
                action = Some(HotkeyAction::Correct);
            } else if (flags & ACTIVE_HOTKEYS.summarize_flags) == ACTIVE_HOTKEYS.summarize_flags
                && keycode == ACTIVE_HOTKEYS.summarize_key
            {
                action = Some(HotkeyAction::Summarize);
            } else if (flags & ACTIVE_HOTKEYS.toggle_locale_flags)
                == ACTIVE_HOTKEYS.toggle_locale_flags
                && keycode == ACTIVE_HOTKEYS.toggle_locale_key
            {
                action = Some(HotkeyAction::ToggleLocale);
            } else if (flags & ACTIVE_HOTKEYS.buffer_start_flags)
                == ACTIVE_HOTKEYS.buffer_start_flags
                && keycode == ACTIVE_HOTKEYS.buffer_start_key
            {
                action = Some(HotkeyAction::BufferStart);
            } else if (flags & ACTIVE_HOTKEYS.buffer_flush_flags)
                == ACTIVE_HOTKEYS.buffer_flush_flags
                && keycode == ACTIVE_HOTKEYS.buffer_flush_key
            {
                action = Some(HotkeyAction::BufferFlush);
            } else if (flags & ACTIVE_HOTKEYS.settings_flags) == ACTIVE_HOTKEYS.settings_flags
                && keycode == ACTIVE_HOTKEYS.settings_key
            {
                action = Some(HotkeyAction::Settings);
            } else if (flags & ACTIVE_HOTKEYS.help_flags) == ACTIVE_HOTKEYS.help_flags
                && keycode == ACTIVE_HOTKEYS.help_key
            {
                action = Some(HotkeyAction::Help);
            } else if (flags & ACTIVE_HOTKEYS.usage_stats_flags) == ACTIVE_HOTKEYS.usage_stats_flags
                && keycode == ACTIVE_HOTKEYS.usage_stats_key
            {
                action = Some(HotkeyAction::UsageStats);
            }

            if let Some(action) = action {
                if let Some(ref sender) = HOTKEY_SENDER {
                    let _ = sender.try_send(action);
                }
                // Block the specific hotkey event from propagating
                return ptr::null_mut();
            }
        }

        // Trigger commit on any key down or mouse down while allowing it to pass through
        // But skip if it's one of our defined hotkeys (those were already blocked above)
        // Also skip if buffer mode is active (user is working elsewhere while voice input)
        if event_type == K_CG_EVENT_LEFT_MOUSE_DOWN
            || event_type == K_CG_EVENT_RIGHT_MOUSE_DOWN
            || event_type == K_CG_EVENT_OTHER_MOUSE_DOWN
        {
            LAST_OPTION_PRESS_TIME = 0; // マウスクリックでもリセット
        }

        if !BUFFER_MODE_ACTIVE
            && (event_type == K_CG_EVENT_KEY_DOWN
                || event_type == K_CG_EVENT_LEFT_MOUSE_DOWN
                || event_type == K_CG_EVENT_RIGHT_MOUSE_DOWN
                || event_type == K_CG_EVENT_OTHER_MOUSE_DOWN)
        {
            if let Some(ref sender) = HOTKEY_SENDER {
                let _ = sender.try_send(HotkeyAction::Commit);
            }
        }

        event
    }
}

/// ホットキー監視ランループを停止する。
pub fn stop_monitoring() {
    unsafe {
        if let Some(rl) = RUN_LOOP {
            log::info!("Stopping hotkey monitoring run loop...");
            CFRunLoopStop(rl);
            RUN_LOOP = None;
        } else {
            log::warn!("Attempted to stop hotkey monitoring but no run loop was active.");
        }
        // 送信側チャンネルを明示的に破棄し、ハンドラーループを終了させる
        HOTKEY_SENDER = None;
    }
}

/// CGEventTap を使用してグローバルホットキーイベントを監視する。
pub struct HotkeyMonitor {
    config: HotkeyConfig,
}

impl HotkeyMonitor {
    /// 指定された設定で新しいホットキーモニターを作成する。
    pub fn new(config: HotkeyConfig) -> Self {
        Self { config }
    }

    /// 別スレッドでホットキーの監視を開始する。
    /// ホットキーアクションのレシーバーを返す。
    pub fn start(self) -> mpsc::Receiver<HotkeyAction> {
        let (async_tx, async_rx) = mpsc::channel::<HotkeyAction>(10);

        // Parse key config
        let (correct_key, correct_flags) = parse_hotkey(&self.config.correct);
        let (summarize_key, summarize_flags) = parse_hotkey(&self.config.summarize);
        let (toggle_locale_key, toggle_locale_flags) = parse_hotkey(&self.config.toggle_locale);
        let (buffer_start_key, buffer_start_flags) = parse_hotkey(&self.config.buffer_start);
        let (buffer_flush_key, buffer_flush_flags) = parse_hotkey(&self.config.buffer_flush);
        let (settings_key, settings_flags) = parse_hotkey(&self.config.settings);
        let (help_key, help_flags) = parse_hotkey(&self.config.help);
        let (usage_stats_key, usage_stats_flags) = parse_hotkey(&self.config.usage_stats);

        // Store active hotkeys
        unsafe {
            ACTIVE_HOTKEYS = ActiveHotkeys {
                correct_key,
                correct_flags,
                summarize_key,
                summarize_flags,
                toggle_locale_key,
                toggle_locale_flags,
                buffer_start_key,
                buffer_start_flags,
                buffer_flush_key,
                buffer_flush_flags,
                settings_key,
                settings_flags,
                help_key,
                help_flags,
                usage_stats_key,
                usage_stats_flags,
            };
        }

        // Create a sync channel for the callback
        let (sync_tx, sync_rx) = std::sync::mpsc::sync_channel::<HotkeyAction>(10);

        // コールバック用に送信者をグローバルに保存
        unsafe {
            HOTKEY_SENDER = Some(sync_tx);
        }

        // 同期から非同期へのブリッジ
        let async_tx_clone = async_tx.clone();
        std::thread::spawn(move || {
            while let Ok(action) = sync_rx.recv() {
                let _ = async_tx_clone.blocking_send(action);
            }
        });

        // イベントタップスレッドを開始する (tokio の初期化を待つための遅延付き)
        std::thread::spawn(move || {
            // メインイベントループの開始を待機
            std::thread::sleep(std::time::Duration::from_millis(100));

            unsafe {
                // Events we're interested in: keyboard and mouse down events
                let events_of_interest: u64 = (1 << K_CG_EVENT_KEY_DOWN)
                    | (1 << K_CG_EVENT_KEY_UP)
                    | (1 << K_CG_EVENT_FLAGS_CHANGED)
                    | (1 << K_CG_EVENT_LEFT_MOUSE_DOWN)
                    | (1 << K_CG_EVENT_RIGHT_MOUSE_DOWN)
                    | (1 << K_CG_EVENT_OTHER_MOUSE_DOWN);

                // Create the event tap
                // kCGSessionEventTap = 1, kCGHeadInsertEventTap = 0, kCGEventTapOptionDefault = 0
                let tap = CGEventTapCreate(
                    1, // kCGSessionEventTap
                    0, // kCGHeadInsertEventTap
                    0, // kCGEventTapOptionDefault (can modify events)
                    events_of_interest,
                    event_tap_callback,
                    ptr::null_mut(),
                );

                if tap.is_null() {
                    log::error!("Failed to create CGEventTap. Make sure Accessibility permission is granted.");
                    return;
                }

                log::debug!("CGEventTap created successfully");

                // Create run loop source
                let source = CFMachPortCreateRunLoopSource(ptr::null(), tap, 0);
                if source.is_null() {
                    log::error!("Failed to create run loop source");
                    return;
                }

                // Get kCFRunLoopCommonModes
                extern "C" {
                    static kCFRunLoopCommonModes: *const c_void;
                }

                // Add to current run loop
                let run_loop = CFRunLoopGetCurrent();
                RUN_LOOP = Some(run_loop); // Store for stopping
                CFRunLoopAddSource(run_loop, source, kCFRunLoopCommonModes);

                // Enable the tap
                CGEventTapEnable(tap, true);

                log::debug!("Hotkey monitoring started (CGEventTap)");

                // Run the loop
                CFRunLoopRun();
            }
        });

        async_rx
    }
}

/// Helper to parse hotkey strings like ["Option", "KeyS"] into (KeyCode, Flags)
fn parse_hotkey(keys: &[String]) -> (CGKeyCode, CGEventFlags) {
    let mut flags = 0;
    let mut keycode = 0;

    for key in keys {
        match key.as_str() {
            "Option" => flags |= K_CG_EVENT_FLAG_MASK_ALTERNATE,
            "Control" => flags |= K_CG_EVENT_FLAG_MASK_CONTROL,
            "Command" => flags |= 0x00100000,
            "Shift" => flags |= 0x00020000,
            "KeyA" => keycode = 0,
            "KeyS" => keycode = 1,
            "KeyD" => keycode = 2,
            "KeyF" => keycode = 3,
            "KeyH" => keycode = 4,
            "KeyG" => keycode = 5,
            "KeyZ" => keycode = 6,
            "KeyX" => keycode = 7,
            "KeyC" => keycode = 8,
            "KeyV" => keycode = 9,
            "KeyB" => keycode = 11,
            "KeyQ" => keycode = 12,
            "KeyW" => keycode = 13,
            "KeyE" => keycode = 14,
            "KeyR" => keycode = 15,
            "KeyY" => keycode = 16,
            "KeyT" => keycode = 17,
            "Key1" => keycode = 18,
            "Key2" => keycode = 19,
            "Key3" => keycode = 20,
            "Key4" => keycode = 21,
            "Key6" => keycode = 22,
            "Key5" => keycode = 23,
            "KeyM" => keycode = 46,
            "KeyL" => keycode = 37,
            "KeyJ" => keycode = 38,
            "KeyU" => keycode = 32,
            _ => {}
        }
    }
    (keycode, flags)
}
