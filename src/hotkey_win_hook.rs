//! WH_KEYBOARD_LL によるカスタム低レベルキーボードフック。
//!
//! rdev よりも上位のフックチェーンに割り込み、MYCUTE のホットキーと一致する
//! イベントをブロックする（戻り値 1 により OS/他アプリへの到達を阻止する）。
//! hotkey_win.rs と同一の atomic フラグを共有し、二重発火を防止する。

use crate::constants::{HOTKEY_DOUBLE_TAP_MAX_MS, HOTKEY_DOUBLE_TAP_MIN_MS};
use crate::hotkey_win::{
    ACTIVE_HOTKEYS, HOTKEY_SENDER, MOD_ALT, MOD_CTRL, MOD_SHIFT, MOD_WIN, VK_MENU,
    CURRENT_MODIFIERS, LAST_ALT_PRESS_TIME,
    PENDING_ALT_FLUSH, PENDING_ALT_START, RECORDING_ACTIVE,
};
use crate::types::HotkeyAction;
use std::ffi::c_void;
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicU32, AtomicU64, Ordering};
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

/// SendInput の dw_extra_info に設定するマーカー値（自己イベント識別用）。
/// ASCII "MYCU" に相当し、他アプリからの偶然の一致を避ける。
const MYCUTE_EVENT_TAG: usize = 0x4D594355;

/// 仮想キーコード: Shift
const VK_SHIFT: u32 = 0x10;
/// 仮想キーコード: 左 Win
const VK_LWIN: u32 = 0x5B;
/// 仮想キーコード: 右 Win
const VK_RWIN: u32 = 0x5C;

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

/// SendInput 用 KeybdInput 構造体（hotkey_win_hook 内で使用）。
/// keyboard_win.rs と重複するが、依存関係の循環を避けるため別途定義する。
#[repr(C)]
struct KeybdInput {
    w_vk: u16,
    w_scan: u16,
    dw_flags: u32,
    time: u32,
    dw_extra_info: usize,
}

/// SendInput 用 Input 構造体（64ビット用パディング付き）。
#[repr(C)]
struct Input {
    input_type: u32,
    _pad: u32,
    ki: KeybdInput,
    _union_pad: [u8; 8],
}

/// SendInput の入力種別: キーボード
const INPUT_KEYBOARD: u32 = 1;
/// キーイベントフラグ: キー解放
const KEYEVENTF_KEYUP: u32 = 0x0002;

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

    fn SendInput(
        c_inputs: u32,
        p_inputs: *const Input,
        cb_size: i32,
    ) -> u32;
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
/// フックが有効であるべきかどうか（disable 後は再インストールしないためのガード）
static HOOK_SHOULD_BE_ACTIVE: AtomicBool = AtomicBool::new(false);

/// プロセス内の Alt DOWN がこのフックによってブロックされたかどうか。
/// ブロックした DOWN に対応する UP も確実にブロックするために使用する。
static HOOK_ALT_DOWN_BLOCKED: AtomicBool = AtomicBool::new(false);
/// Alt キーリピート検出用ガード（同一スレッド=アトミックで十分）
static HOOK_ALT_REPEAT: AtomicBool = AtomicBool::new(false);

/// BufferFlush 重複送信防止用の前回送信時刻（ミリ秒）。
const BUFFER_FLUSH_DEDUP_MS: u64 = 50;
static LAST_BUFFER_FLUSH_TIME: AtomicU64 = AtomicU64::new(0);

/// Ctrl+Alt 同時押しの重複送信防止用フラグ
static ORCHESTRATOR_COMBO_ACTIVE: AtomicBool = AtomicBool::new(false);
/// OrchestratorInput 誤発火防止クールダウン（ミリ秒）
const ORCHESTRATOR_COOLDOWN_MS: u64 = 150;
/// 前回の OrchestratorInput 発火時刻（ミリ秒）
static ORCHESTRATOR_LAST_FIRE_MS: AtomicU64 = AtomicU64::new(0);

// ─── 公開 API ─────────────────────────────────────────────────────────

/// 別スレッドで WH_KEYBOARD_LL フックを開始する。
/// SetWindowsHookExW の成否はスレッド内でしか検出できないため、この関数は
/// 常に Ok(()) を返す。実際の失敗は HOOK_ACTIVE フラグで検出し、
/// check_hook_health() による定期監視でリカバリする。
pub fn start_hook() -> Result<(), String> {
    if HOOK_ACTIVE.load(Ordering::SeqCst) {
        log::debug!("WH_KEYBOARD_LL hook is already active.");
        return Ok(());
    }

    HOOK_SHOULD_BE_ACTIVE.store(true, Ordering::SeqCst);
    spawn_hook_thread();
    Ok(())
}

/// フックスレッドを起動する内部関数。
fn spawn_hook_thread() {
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
}

/// WH_KEYBOARD_LL フックを停止する。
/// メッセージポンプスレッドに WM_QUIT をポストし、スレッド終了まで待たない。
pub fn stop_hook() {
    HOOK_ACTIVE.store(false, Ordering::SeqCst);
    HOOK_SHOULD_BE_ACTIVE.store(false, Ordering::SeqCst);

    // グローバルフラグも後続のイベントでブロックしないためにクリア
    HOOK_ALT_DOWN_BLOCKED.store(false, Ordering::SeqCst);

    let tid = HOOK_THREAD_ID.swap(0, Ordering::SeqCst);
    if tid != 0 {
        unsafe {
            PostThreadMessageW(tid, WM_QUIT, 0, 0);
        }
    }
}

/// WH_KEYBOARD_LL フックの健全性を確認し、無効かつ有効であるべき状態なら
/// 再インストールを試みる。ホットキーハンドラループから定期的に呼び出される。
pub fn check_hook_health() {
    if HOOK_SHOULD_BE_ACTIVE.load(Ordering::SeqCst) && !HOOK_ACTIVE.load(Ordering::SeqCst) {
        log::warn!(
            "WH_KEYBOARD_LL hook health check failed: hook is not active. \
             Attempting reinstall..."
        );
        spawn_hook_thread();
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

    // 自己生成イベント（dw_extra_info に MYCUTE_EVENT_TAG が設定されている）は
    // ブロックせずに通過させる。これにより SendInput が生成したイベントが
    // 再びこのフックに到達した際の無限ループを防止する。
    if kb.dw_extra_info == MYCUTE_EVENT_TAG {
        return CallNextHookEx(ptr::null_mut(), n_code, w_param, l_param);
    }

    match w_param as u32 {
        WM_KEYDOWN | WM_SYSKEYDOWN => {
            if kb.vk_code == VK_MENU as u32 {
                return process_alt_down();
            }

            // Alt 修飾ありのホットキーコンボをチェック
            if (kb.flags & LLKHF_ALTDOWN) != 0 {
                CURRENT_MODIFIERS.fetch_or(MOD_ALT, Ordering::SeqCst);
                update_orchestrator_combo_state();
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
                VK_SHIFT | VK_LWIN | VK_RWIN => {
                    track_other_modifier(kb.vk_code, false);
                }
                _ => {}
            }
        }

        _ => {}
    }

    CallNextHookEx(ptr::null_mut(), n_code, w_param, l_param)
}

// ─── Alt キー処理 ─────────────────────────────────────────────────────

/// Alt KEY_DOWN を処理する。
/// ダブルタップ検出時のみイベントをブロックし、シングルタップは通過させる。
unsafe fn process_alt_down() -> isize {
    if is_alt_repeat() {
        return CallNextHookEx(ptr::null_mut(), HC_ACTION, 0, 0);
    }

    CURRENT_MODIFIERS.fetch_or(MOD_ALT, Ordering::SeqCst);
    update_orchestrator_combo_state();

    // Ctrl+Alt 同時押しの場合はダブルタップ処理をスキップする
    if ORCHESTRATOR_COMBO_ACTIVE.load(Ordering::SeqCst) {
        LAST_ALT_PRESS_TIME.store(0, Ordering::SeqCst);
        return CallNextHookEx(ptr::null_mut(), HC_ACTION, 0, 0);
    }

    if is_double_tap_detected() {
        // ダブルタップ確定: 録音中なら Flush、非録音なら Start
        if RECORDING_ACTIVE.load(Ordering::SeqCst) {
            PENDING_ALT_FLUSH.store(true, Ordering::SeqCst);
        } else {
            PENDING_ALT_START.store(true, Ordering::SeqCst);
        }
        LAST_ALT_PRESS_TIME.store(0, Ordering::SeqCst);
        HOOK_ALT_DOWN_BLOCKED.store(true, Ordering::SeqCst);
        inject_alt_up();
        return 1;
    }

    // シングルタップ: 押下時刻を記録し、イベントを通過させる
    LAST_ALT_PRESS_TIME.store(current_time_ms(), Ordering::SeqCst);
    HOOK_ALT_DOWN_BLOCKED.store(false, Ordering::SeqCst);
    CallNextHookEx(ptr::null_mut(), HC_ACTION, 0, 0)
}

/// Alt キーのオートリピートかどうかを判定する。
fn is_alt_repeat() -> bool {
    HOOK_ALT_REPEAT.swap(true, Ordering::SeqCst)
}

/// ダブルタップ条件を満たすか判定する。
fn is_double_tap_detected() -> bool {
    let now = current_time_ms();
    let last = LAST_ALT_PRESS_TIME.load(Ordering::SeqCst);
    let diff = now.saturating_sub(last);
    diff > HOTKEY_DOUBLE_TAP_MIN_MS as u64
        && diff < HOTKEY_DOUBLE_TAP_MAX_MS as u64
}

/// SendInput で Alt UP イベントを強制注入する。
/// これにより、WH_KEYBOARD_LL のブロックをすり抜けた Alt キーが
/// フラッシュ先アプリでメニュー等を起動するのを防止する。
unsafe fn inject_alt_up() {
    let mut input: Input = std::mem::zeroed();
    input.input_type = INPUT_KEYBOARD;
    input.ki.w_vk = VK_MENU;
    input.ki.dw_flags = KEYEVENTF_KEYUP;
    input.ki.dw_extra_info = MYCUTE_EVENT_TAG;
    SendInput(1, &input, std::mem::size_of::<Input>() as i32);
}

/// Alt KEY_UP を処理する。
/// 保留中のアクションがあれば送信し、ブロックした DOWN に対応する UP もブロックする。
unsafe fn process_alt_up() -> isize {
    HOOK_ALT_REPEAT.store(false, Ordering::SeqCst);

    CURRENT_MODIFIERS.fetch_and(!MOD_ALT, Ordering::SeqCst);
    update_orchestrator_combo_state();

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
        0x11 | 0xA2 | 0xA3 => MOD_CTRL,
        VK_SHIFT => MOD_SHIFT,
        VK_LWIN | VK_RWIN => MOD_WIN,
        _ => return,
    };
    if is_down {
        CURRENT_MODIFIERS.fetch_or(bit, Ordering::SeqCst);
        update_orchestrator_combo_state();
    } else {
        CURRENT_MODIFIERS.fetch_and(!bit, Ordering::SeqCst);
        update_orchestrator_combo_state();
    }
}

/// 現在の修飾子が Control+Alt 同時押し状態か確認し、遷移時に OrchestratorInput を送信する。
/// 150ms のクールダウンを持ち、誤発火を防止する。
fn update_orchestrator_combo_state() {
    let mods = CURRENT_MODIFIERS.load(Ordering::SeqCst);
    let both_held = (mods & (MOD_CTRL | MOD_ALT)) == (MOD_CTRL | MOD_ALT);
    if both_held && !ORCHESTRATOR_COMBO_ACTIVE.swap(true, Ordering::SeqCst) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let last = ORCHESTRATOR_LAST_FIRE_MS.load(Ordering::SeqCst);
        if now.saturating_sub(last) > ORCHESTRATOR_COOLDOWN_MS {
            ORCHESTRATOR_LAST_FIRE_MS.store(now, Ordering::SeqCst);
            log::debug!("[OrchestratorCombo] Ctrl+Alt detected");
            send_action(HotkeyAction::OrchestratorInput);
        } else {
            // クールダウン中の再発火: フラグを戻して次回の検出を可能にする
            ORCHESTRATOR_COMBO_ACTIVE.store(false, Ordering::SeqCst);
        }
    } else if !both_held {
        ORCHESTRATOR_COMBO_ACTIVE.store(false, Ordering::SeqCst);
    }
}

/// 共有送信者経由でホットキーアクションを送信する（非ブロッキング）。
fn send_action(action: HotkeyAction) {
    // BufferFlush の重複送信ガード（WH_KEYBOARD_LL フックと rdev/GetAsyncKeyState の
    // 両経路から同時に送信された場合の保護）
    if let HotkeyAction::BufferFlush = action {
        let now = current_time_ms();
        let last = LAST_BUFFER_FLUSH_TIME.load(Ordering::SeqCst);
        if now.saturating_sub(last) < BUFFER_FLUSH_DEDUP_MS {
            return;
        }
        LAST_BUFFER_FLUSH_TIME.store(now, Ordering::SeqCst);
    }

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
