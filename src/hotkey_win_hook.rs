//! WH_KEYBOARD_LL によるカスタム低レベルキーボードフック。
//!
//! rdev よりも上位のフックチェーンに割り込み、MYCUTE のホットキーと一致する
//! イベントをブロックする（戻り値 1 により OS/他アプリへの到達を阻止する）。
//! hotkey_win.rs と同一の atomic フラグを共有し、二重発火を防止する。

use crate::constants::{HOTKEY_DOUBLE_TAP_MAX_MS, HOTKEY_DOUBLE_TAP_MIN_MS};
use crate::hotkey_win::{
    ACTIVE_HOTKEYS, HOTKEY_SENDER, MOD_ALT, MOD_CTRL, MOD_SHIFT, MOD_WIN, VK_MENU,
    CURRENT_MODIFIERS, LAST_ALT_PRESS_TIME,
    MONITORING_ACTIVE, PENDING_ALT_FLUSH, PENDING_ALT_START, RECORDING_ACTIVE,
};
use crate::types::HotkeyAction;
use std::ffi::c_void;
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

// ─── Windows API 定数 ────────────────────────────────────────────────
const WH_KEYBOARD_LL: i32 = 13;
const HC_ACTION: i32 = 0;
const WM_KEYDOWN: u32 = 0x0100;
const WM_KEYUP: u32 = 0x0101;
const WM_SYSKEYDOWN: u32 = 0x0104;
const WM_SYSKEYUP: u32 = 0x0105;
const WM_QUIT: u32 = 0x0012;
const LLKHF_ALTDOWN: u32 = 0x20;

// ─── Windows API 構造体 ──────────────────────────────────────────────
#[repr(C)]
struct KBDLLHOOKSTRUCT {
    vk_code: u32,
    scan_code: u32,
    flags: u32,
    time: u32,
    dw_extra_info: usize,
}

#[repr(C)]
struct MSG {
    hwnd: *mut c_void,
    message: u32,
    w_param: usize,
    l_param: isize,
    time: u32,
    pt: POINT,
}

#[repr(C)]
struct POINT {
    x: i32,
    y: i32,
}

type HOOKPROC = unsafe extern "system" fn(i32, usize, isize) -> isize;

// ─── FFI 宣言 ─────────────────────────────────────────────────────────
#[link(name = "user32")]
extern "system" {
    fn SetWindowsHookExW(
        id_hook: i32,
        lpfn: HOOKPROC,
        hmod: *mut c_void,
        dw_thread_id: u32,
    ) -> *mut c_void;

    fn CallNextHookEx(
        hhk: *mut c_void,
        n_code: i32,
        w_param: usize,
        l_param: isize,
    ) -> isize;

    fn UnhookWindowsHookEx(hhk: *mut c_void) -> i32;

    fn GetMessageW(
        lp_msg: *mut MSG,
        h_wnd: *mut c_void,
        w_msg_filter_min: u32,
        w_msg_filter_max: u32,
    ) -> i32;

    fn TranslateMessage(lp_msg: *const MSG) -> i32;

    fn DispatchMessageW(lp_msg: *const MSG) -> isize;

    fn PostThreadMessageW(
        id_thread: u32,
        msg: u32,
        w_param: usize,
        l_param: isize,
    ) -> i32;

    fn GetModuleHandleW(lp_module_name: *const u16) -> *mut c_void;
}

#[link(name = "kernel32")]
extern "system" {
    fn GetCurrentThreadId() -> u32;
}

// ─── フックライフサイクル管理 ─────────────────────────────────────────
/// フックハンドル（Unhook 時に使用）
static HOOK_HANDLE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
/// メッセージポンプスレッドの ID（WM_QUIT 送信に使用）
static HOOK_THREAD_ID: AtomicU32 = AtomicU32::new(0);
/// フックが有効かどうか
static HOOK_ACTIVE: AtomicBool = AtomicBool::new(false);

/// プロセス内の Alt DOWN がこのフックによってブロックされたかどうか。
/// ブロックした DOWN に対応する UP も確実にブロックするために使用する。
static HOOK_ALT_DOWN_BLOCKED: AtomicBool = AtomicBool::new(false);
/// Alt キーリピート検出用ガード（同一スレッド=アトミックで十分）
static HOOK_ALT_REPEAT: AtomicBool = AtomicBool::new(false);

// ─── 公開 API ─────────────────────────────────────────────────────────

/// 別スレッドで WH_KEYBOARD_LL フックを開始する。
/// 戻り値: 開始に成功したかどうか。
pub fn start_hook() -> bool {
    if HOOK_ACTIVE.load(Ordering::SeqCst) {
        log::debug!("WH_KEYBOARD_LL hook is already active.");
        return true;
    }

    HOOK_ACTIVE.store(true, Ordering::SeqCst);

    std::thread::spawn(move || {
        unsafe {
            let hmod = GetModuleHandleW(ptr::null());
            let hook = SetWindowsHookExW(
                WH_KEYBOARD_LL,
                hook_proc,
                hmod,
                0, // dwThreadId = 0 → グローバルフック
            );

            if hook.is_null() {
                log::error!(
                    "Failed to install WH_KEYBOARD_LL hook: {}",
                    std::io::Error::last_os_error()
                );
                HOOK_ACTIVE.store(false, Ordering::SeqCst);
                return;
            }

            HOOK_HANDLE.store(hook, Ordering::SeqCst);
            HOOK_THREAD_ID.store(GetCurrentThreadId(), Ordering::SeqCst);

            log::info!("WH_KEYBOARD_LL hook installed successfully");

            // メッセージポンプ（WH_KEYBOARD_LL のコールバック配送に必須）
            let mut msg = std::mem::zeroed::<MSG>();
            while GetMessageW(&mut msg, ptr::null_mut(), 0, 0) > 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            // ループ終了 = WM_QUIT 受信。フックを解除する。
            let h = HOOK_HANDLE.swap(ptr::null_mut(), Ordering::SeqCst);
            if !h.is_null() {
                UnhookWindowsHookEx(h);
            }
            HOOK_THREAD_ID.store(0, Ordering::SeqCst);
            HOOK_ACTIVE.store(false, Ordering::SeqCst);
            log::info!("WH_KEYBOARD_LL hook removed");
        }
    });

    true
}

/// WH_KEYBOARD_LL フックを停止する。
/// メッセージポンプスレッドに WM_QUIT をポストし、スレッド終了まで待たない。
pub fn stop_hook() {
    HOOK_ACTIVE.store(false, Ordering::SeqCst);

    // グローバルフラグも後続のイベントでブロックしないためにクリア
    HOOK_ALT_DOWN_BLOCKED.store(false, Ordering::SeqCst);

    let tid = HOOK_THREAD_ID.swap(0, Ordering::SeqCst);
    if tid != 0 {
        unsafe {
            PostThreadMessageW(tid, WM_QUIT, 0, 0);
        }
    }
}

// ─── フックプロシージャ ─────────────────────────────────────────────────

/// WH_KEYBOARD_LL フックプロシージャ。
///
/// MYCUTE ホットキーが検出された場合に 1 を返す（イベントをブロック）。
/// それ以外は CallNextHookEx に委譲する。
unsafe extern "system" fn hook_proc(
    n_code: i32,
    w_param: usize,
    l_param: isize,
) -> isize {
    if n_code < HC_ACTION || !HOOK_ACTIVE.load(Ordering::SeqCst) {
        return CallNextHookEx(ptr::null_mut(), n_code, w_param, l_param);
    }

    let kb = &*(l_param as *const KBDLLHOOKSTRUCT);

    match w_param as u32 {
        WM_KEYDOWN | WM_SYSKEYDOWN => {
            if kb.vk_code == VK_MENU as u32 {
                return process_alt_down();
            }

            // Alt 修飾ありのホットキーコンボをチェック
            if (kb.flags & LLKHF_ALTDOWN) != 0 {
                CURRENT_MODIFIERS.fetch_or(MOD_ALT, Ordering::SeqCst);
                if check_combo_hotkey(kb.vk_code) {
                    return 1;
                }
            } else {
                // Alt 以外の修飾キー → 状態追跡のみ
                track_other_modifier(kb.vk_code, true);
            }
        }

        WM_KEYUP | WM_SYSKEYUP => {
            if kb.vk_code == VK_MENU as u32 {
                return process_alt_up();
            }

            // 修飾キーの解放を追跡
            match kb.vk_code {
                0x11 /* VK_CONTROL */ => track_other_modifier(kb.vk_code, false),
                0x10 /* VK_SHIFT */   => track_other_modifier(kb.vk_code, false),
                0x5B /* VK_LWIN */
                | 0x5C /* VK_RWIN */ => track_other_modifier(kb.vk_code, false),
                _ => {}
            }
        }

        _ => {}
    }

    CallNextHookEx(ptr::null_mut(), n_code, w_param, l_param)
}

// ─── Alt キー処理 ─────────────────────────────────────────────────────

/// Alt KEY_DOWN を処理する。
/// 録音中フラッシュまたはダブルタップ検出時はイベントをブロックする。
unsafe fn process_alt_down() -> isize {
    // リピートガード: キーオートリピートによる再送はブロックせず通過させる
    if HOOK_ALT_REPEAT.swap(true, Ordering::SeqCst) {
        return CallNextHookEx(ptr::null_mut(), HC_ACTION, 0, 0);
    }

    CURRENT_MODIFIERS.fetch_or(MOD_ALT, Ordering::SeqCst);

    // ── 録音中: 即フラッシュ ──
    if RECORDING_ACTIVE.load(Ordering::SeqCst) {
        PENDING_ALT_FLUSH.store(true, Ordering::SeqCst);
        HOOK_ALT_DOWN_BLOCKED.store(true, Ordering::SeqCst);
        return 1;
    }

    // ── ダブルタップ検出 ──
    let now = current_time_ms();
    let last = LAST_ALT_PRESS_TIME.load(Ordering::SeqCst);
    let diff = now.saturating_sub(last);

    if diff > HOTKEY_DOUBLE_TAP_MIN_MS as u64
        && diff < HOTKEY_DOUBLE_TAP_MAX_MS as u64
    {
        // ダブルタップ確定: 2回目の Alt 押下をブロック
        PENDING_ALT_START.store(true, Ordering::SeqCst);
        LAST_ALT_PRESS_TIME.store(0, Ordering::SeqCst);
        HOOK_ALT_DOWN_BLOCKED.store(true, Ordering::SeqCst);
        return 1;
    } else {
        LAST_ALT_PRESS_TIME.store(now, Ordering::SeqCst);
    }

    // 1回目の Alt 押下 → ブロックしない（ユーザー合意済み）
    CallNextHookEx(ptr::null_mut(), HC_ACTION, 0, 0)
}

/// Alt KEY_UP を処理する。
/// 保留中のアクションがあれば送信し、ブロックした DOWN に対応する UP もブロックする。
unsafe fn process_alt_up() -> isize {
    HOOK_ALT_REPEAT.store(false, Ordering::SeqCst);

    CURRENT_MODIFIERS.fetch_and(!MOD_ALT, Ordering::SeqCst);

    let down_was_blocked = HOOK_ALT_DOWN_BLOCKED.swap(false, Ordering::SeqCst);
    let did_start = PENDING_ALT_START.swap(false, Ordering::SeqCst);
    let did_flush = PENDING_ALT_FLUSH.swap(false, Ordering::SeqCst);

    if did_start {
        send_action(HotkeyAction::Start);
    }
    if did_flush {
        send_action(HotkeyAction::BufferFlush);
    }

    // DOWN をブロックした → UP もブロック（キーボード状態の不整合を防止）
    if down_was_blocked || did_start || did_flush {
        1
    } else {
        CallNextHookEx(ptr::null_mut(), HC_ACTION, 0, 0)
    }
}

// ─── ホットキーコンボチェック ──────────────────────────────────────────

/// 現在の修飾子状態とキーコードが MYCUTE のホットキー定義と一致するか調べ、
/// 一致した場合はアクションを送信して true を返す。
unsafe fn check_combo_hotkey(vk_code: u32) -> bool {
    let current_mods = CURRENT_MODIFIERS.load(Ordering::SeqCst);
    let key_str = match vk_code_to_str(vk_code) {
        Some(s) => s,
        None => return false,
    };

    if let Ok(guard) = ACTIVE_HOTKEYS.try_lock() {
        if let Some(ref hotkeys) = *guard {
            if hotkeys.correct.matches(key_str, current_mods) {
                send_action(HotkeyAction::Correct);
                return true;
            }
            if hotkeys.summarize.matches(key_str, current_mods) {
                send_action(HotkeyAction::Summarize);
                return true;
            }
        }
    }

    false
}

// ─── ユーティリティ ─────────────────────────────────────────────────────

/// 修飾キーのビットを設定/解除する（Ctrl/Shift/Win）。
unsafe fn track_other_modifier(vk_code: u32, is_down: bool) {
    let bit = match vk_code {
        0x11 => MOD_CTRL,           // VK_CONTROL
        0x10 => MOD_SHIFT,          // VK_SHIFT
        0x5B | 0x5C => MOD_WIN,     // VK_LWIN / VK_RWIN
        _ => return,
    };
    if is_down {
        CURRENT_MODIFIERS.fetch_or(bit, Ordering::SeqCst);
    } else {
        CURRENT_MODIFIERS.fetch_and(!bit, Ordering::SeqCst);
    }
}

/// 共有送信者経由でホットキーアクションを送信する（非ブロッキング）。
fn send_action(action: HotkeyAction) {
    if let Ok(guard) = HOTKEY_SENDER.try_lock() {
        if let Some(ref sender) = *guard {
            let _ = sender.try_send(action);
        }
    }
}

/// Windows VK コードを "KeyX" 形式の文字列に変換する。
fn vk_code_to_str(vk: u32) -> Option<&'static str> {
    match vk {
        0x41 => Some("KeyA"), 0x42 => Some("KeyB"), 0x43 => Some("KeyC"),
        0x44 => Some("KeyD"), 0x45 => Some("KeyE"), 0x46 => Some("KeyF"),
        0x47 => Some("KeyG"), 0x48 => Some("KeyH"), 0x49 => Some("KeyI"),
        0x4A => Some("KeyJ"), 0x4B => Some("KeyK"), 0x4C => Some("KeyL"),
        0x4D => Some("KeyM"), 0x4E => Some("KeyN"), 0x4F => Some("KeyO"),
        0x50 => Some("KeyP"), 0x51 => Some("KeyQ"), 0x52 => Some("KeyR"),
        0x53 => Some("KeyS"), 0x54 => Some("KeyT"), 0x55 => Some("KeyU"),
        0x56 => Some("KeyV"), 0x57 => Some("KeyW"), 0x58 => Some("KeyX"),
        0x59 => Some("KeyY"), 0x5A => Some("KeyZ"),
        0x30 => Some("Key0"), 0x31 => Some("Key1"), 0x32 => Some("Key2"),
        0x33 => Some("Key3"), 0x34 => Some("Key4"), 0x35 => Some("Key5"),
        0x36 => Some("Key6"), 0x37 => Some("Key7"), 0x38 => Some("Key8"),
        0x39 => Some("Key9"),
        _ => None,
    }
}

/// UNIX epoch からの経過ミリ秒を取得する。
fn current_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
