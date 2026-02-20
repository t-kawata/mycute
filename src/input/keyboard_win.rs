//! Windows Keyboard input injection using winapi.
//!
//! This module provides functionality to inject text into the active application
//! using Windows SendInput API with Unicode support.

use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};

/// Delay between key presses in milliseconds.
/// Delay between key presses in milliseconds.
use crate::constants::KEY_DELAY_MS;

/// Cooldown after deletion for app-side UI update (α).
/// Reduced from 250ms: the new Down-Wait-Up-Wait protocol provides
/// sufficient pacing, so we only need a short buffer for final UI sync.
use crate::constants::DELETION_COOLDOWN_MS;

/// Global list of deletion completion deadlines.
/// Used to block typing until all pending deletions are logically complete.
static DELETION_DEADLINES: Mutex<Vec<Instant>> = Mutex::new(Vec::new());

/// Global lock for serializing all keyboard input operations.
/// This ensures that only one input_diff/type_text/send_backspaces operation
/// can be in progress at any time, preventing race conditions.
static INPUT_LOCK: Mutex<()> = Mutex::new(());

/// Wait until all pending deletion deadlines have passed.
/// This function performs garbage collection on expired deadlines and blocks if any remain.
fn wait_for_deletion_completion() {
    loop {
        {
            let mut deadlines = DELETION_DEADLINES.lock().unwrap();
            let now = Instant::now();
            // Garbage collection: remove all deadlines that are in the past
            deadlines.retain(|&deadline| deadline > now);
            if deadlines.is_empty() {
                return; // All deletions complete, proceed with typing
            }
            log::debug!(
                "[KeyboardInjector] Waiting for {} deletion deadline(s) to complete...",
                deadlines.len()
            );
        }
        // Sleep briefly and re-check
        thread::sleep(Duration::from_millis(10));
    }
}

// --- Windows API structures for 64-bit ---

#[repr(C)]
struct KeybdInput {
    w_vk: u16,
    w_scan: u16,
    dw_flags: u32,
    time: u32,
    dw_extra_info: usize,
}

#[repr(C)]
struct Input {
    input_type: u32,
    _pad: u32,           // Alignment padding for 64-bit
    ki: KeybdInput,      // 24 bytes (includes internal pad before usize)
    _union_pad: [u8; 8], // 32 - 24 = 8 bytes to make union 32 bytes. Total structure: 40 bytes.
}

const INPUT_KEYBOARD: u32 = 1;
const KEYEVENTF_UNICODE: u32 = 0x0004;
const KEYEVENTF_KEYUP: u32 = 0x0002;
const VK_CONTROL: u16 = 0x11;
const VK_V: u16 = 0x56;
const VK_BACK: u16 = 0x08;

#[link(name = "user32")] // Link against user32.dll for SendInput
extern "system" {
    fn SendInput(c_inputs: u32, p_inputs: *const Input, cb_size: i32) -> u32;
}

/// Helper to manage typing mode state in hotkey monitor.
/// This serves the same purpose as Mac's MYCUTE_EVENT_ID - preventing self-triggered commits.
struct TypingGuard;
impl TypingGuard {
    fn new() -> Self {
        crate::hotkey::set_typing_mode(true);
        Self
    }
}
impl Drop for TypingGuard {
    fn drop(&mut self) {
        crate::hotkey::set_typing_mode(false);
    }
}

/// CGKeyCode type alias for compatibility.
pub type CGKeyCode = u16;

/// Keyboard injector for simulating key presses.
pub struct KeyboardInjector;

impl KeyboardInjector {
    /// Check if the process has accessibility permission.
    /// On Windows, this always returns true.
    pub fn is_authorized() -> bool {
        true
    }

    /// Type the given text by simulating key events with full Unicode support.
    /// This method processes text in chunks, matching Mac's CGEventKeyboardSetUnicodeString behavior.
    /// This is the PUBLIC entry point - acquires the global lock.
    pub fn type_text(text: &str) {
        let _lock = INPUT_LOCK.lock().unwrap();
        let _guard = TypingGuard::new();
        Self::type_text_inner(text);
    }

    /// Internal implementation of type_text (no lock acquisition).
    fn type_text_inner(text: &str) {
        // Wait for any pending deletions to complete before typing
        wait_for_deletion_completion();

        // Convert to UTF-16 for Windows API
        let utf16: Vec<u16> = text.encode_utf16().collect();
        if utf16.is_empty() {
            return;
        }

        // Process in chunks (matching Mac's CHUNK_SIZE approach)
        const CHUNK_SIZE: usize = 16;

        for chunk in utf16.chunks(CHUNK_SIZE) {
            // Send each character in the chunk with Down + Up events
            // Using Down-Wait-Up-Wait protocol for stability
            for &code_unit in chunk {
                use std::mem::size_of;

                // Key down with Unicode character
                let mut input_down: Input = unsafe { std::mem::zeroed() };
                input_down.input_type = INPUT_KEYBOARD;
                input_down.ki.w_vk = 0; // Must be 0 for KEYEVENTF_UNICODE
                input_down.ki.w_scan = code_unit;
                input_down.ki.dw_flags = KEYEVENTF_UNICODE;

                log::info!("[WinInputDebug] sending DOWN U+{:04X}", code_unit);

                unsafe {
                    let sent = SendInput(1, &input_down, size_of::<Input>() as i32);
                    if sent != 1 {
                        log::error!(
                            "[WinInputDebug] SendInput failed for char down U+{:04X}: sent {}/1. Err: {:?}",
                            code_unit, sent, std::io::Error::last_os_error()
                        );
                    } else {
                        log::info!(
                            "[WinInputDebug] SendInput success for char down U+{:04X}",
                            code_unit
                        );
                    }
                }

                // Wait after key down (allows OS/app to register the press)
                thread::sleep(Duration::from_millis(KEY_DELAY_MS));

                // Key up
                let mut input_up: Input = unsafe { std::mem::zeroed() };
                input_up.input_type = INPUT_KEYBOARD;
                input_up.ki.w_vk = 0;
                input_up.ki.w_scan = code_unit;
                input_up.ki.dw_flags = KEYEVENTF_UNICODE | KEYEVENTF_KEYUP;

                log::info!("[WinInputDebug] sending UP U+{:04X}", code_unit);

                unsafe {
                    let sent = SendInput(1, &input_up, size_of::<Input>() as i32);
                    if sent != 1 {
                        log::error!(
                            "[WinInputDebug] SendInput failed for char up U+{:04X}: sent {}/1. Err: {:?}",
                            code_unit, sent, std::io::Error::last_os_error()
                        );
                    } else {
                        log::info!(
                            "[WinInputDebug] SendInput success for char up U+{:04X}",
                            code_unit
                        );
                    }
                }

                // Wait after key up (allows OS/app to complete processing)
                thread::sleep(Duration::from_millis(KEY_DELAY_MS));
            }
        }
    }

    /// Send backspace key presses to delete characters.
    /// count: number of UTF-8 characters to delete (use text.chars().count())
    /// This is the PUBLIC entry point - acquires the global lock.
    pub fn send_backspaces(count: usize) {
        if count == 0 {
            return;
        }
        let _lock = INPUT_LOCK.lock().unwrap();
        let _guard = TypingGuard::new();
        Self::send_backspaces_inner(count);
    }

    /// Internal implementation of send_backspaces (no lock acquisition).
    fn send_backspaces_inner(count: usize) {
        if count == 0 {
            return;
        }

        // Calculate and register the deadline for this deletion batch
        // We multiply KEY_DELAY_MS by 2 because each backspace has both Down and Up events.
        // We also add a dynamic cooldown (Dynamic alpha) proportional to the count.
        let dynamic_cooldown = DELETION_COOLDOWN_MS + (count as u64 * 2);
        let estimated_duration_ms = (count as u64 * KEY_DELAY_MS * 2) + dynamic_cooldown;
        let deadline = Instant::now() + Duration::from_millis(estimated_duration_ms);
        {
            let mut deadlines = DELETION_DEADLINES.lock().unwrap();
            deadlines.push(deadline);
            log::debug!(
                "[KeyboardInjector] Registered deletion deadline: {} chars, {}ms from now",
                count,
                estimated_duration_ms
            );
        }

        use std::mem::size_of;

        // Process one backspace at a time using Down-Wait-Up-Wait protocol
        for _ in 0..count {
            // Backspace down
            let mut input_down: Input = unsafe { std::mem::zeroed() };
            input_down.input_type = INPUT_KEYBOARD;
            input_down.ki.w_vk = VK_BACK;

            unsafe {
                let sent = SendInput(1, &input_down, size_of::<Input>() as i32);
                if sent != 1 {
                    log::error!(
                        "[KeyboardInjector] SendInput failed for backspace down: sent {}/1",
                        sent
                    );
                }
            }

            // Wait after key down (allows OS/app to register the press)
            thread::sleep(Duration::from_millis(KEY_DELAY_MS));

            // Backspace up
            let mut input_up: Input = unsafe { std::mem::zeroed() };
            input_up.input_type = INPUT_KEYBOARD;
            input_up.ki.w_vk = VK_BACK;
            input_up.ki.dw_flags = KEYEVENTF_KEYUP;

            unsafe {
                let sent = SendInput(1, &input_up, size_of::<Input>() as i32);
                if sent != 1 {
                    log::error!(
                        "[KeyboardInjector] SendInput failed for backspace up: sent {}/1",
                        sent
                    );
                }
            }

            // Wait after key up (allows OS/app to complete the deletion)
            thread::sleep(Duration::from_millis(KEY_DELAY_MS));
        }
    }

    /// Type text incrementally by comparing with the previous string.
    /// This minimizes backspaces and typing for a smoother experience.
    /// This method is fully serialized - only one input_diff can run at a time.
    pub fn input_diff(old_text: &str, new_text: &str) {
        log::info!("[WinInputDebug] input_diff start. waiting for lock...");
        // Acquire global lock to serialize all input operations
        let _lock = INPUT_LOCK.lock().unwrap();
        log::info!("[WinInputDebug] input_diff lock acquired.");
        let _guard = TypingGuard::new(); // Keep flag ON throughout the entire diff process

        log::debug!(
            "[KeyboardInjector] input_diff: \"{}\" -> \"{}\"",
            old_text,
            new_text
        );

        // Find common prefix length (in characters)
        let mut common_prefix_chars = 0;
        let old_chars: Vec<char> = old_text.chars().collect();
        let new_chars: Vec<char> = new_text.chars().collect();

        for (oc, nc) in old_chars.iter().zip(new_chars.iter()) {
            if oc == nc {
                common_prefix_chars += 1;
            } else {
                break;
            }
        }
        log::debug!(
            "[KeyboardInjector] common_prefix_chars: {}",
            common_prefix_chars
        );

        // Calculate how many characters to delete from the end of old_text
        let delete_count = old_chars.len() - common_prefix_chars;
        if delete_count > 0 {
            log::debug!("[KeyboardInjector] delete_count: {}", delete_count);
            Self::send_backspaces_inner(delete_count);

            // [WAIT_ALPHA] Dynamic Deletion Cooldown
            // Give OS/IME a moment to process the massive deletion before we start typing.
            // Proportional to delete_count for stability with large deletions.
            let dynamic_cooldown = DELETION_COOLDOWN_MS + (delete_count as u64 * 2);
            thread::sleep(Duration::from_millis(dynamic_cooldown));
        }

        // Calculate what part of new_text needs to be typed
        let type_string: String = new_chars[common_prefix_chars..].iter().collect();
        if !type_string.is_empty() {
            log::debug!("[KeyboardInjector] type_string: \"{}\"", type_string);
            Self::type_text_inner(&type_string);
        }
    }

    /// Send Cmd+C (Copy) keystroke - on Windows this is Ctrl+C
    pub fn send_cmd_c() {
        Self::send_ctrl_key(0x43); // C key
    }

    /// Send Cmd+V (Paste) keystroke - on Windows this is Ctrl+V
    pub fn send_cmd_v() {
        Self::send_ctrl_key(VK_V);
    }

    /// Send Ctrl+key combination.
    fn send_ctrl_key(keycode: CGKeyCode) {
        let _lock = INPUT_LOCK.lock().unwrap();
        let _guard = TypingGuard::new();
        use std::mem::size_of;

        let mut inputs: [Input; 4] = unsafe { std::mem::zeroed() };

        // Ctrl down
        inputs[0].input_type = INPUT_KEYBOARD;
        inputs[0].ki.w_vk = VK_CONTROL;

        // Key down
        inputs[1].input_type = INPUT_KEYBOARD;
        inputs[1].ki.w_vk = keycode;

        // Key up
        inputs[2].input_type = INPUT_KEYBOARD;
        inputs[2].ki.w_vk = keycode;
        inputs[2].ki.dw_flags = KEYEVENTF_KEYUP;

        // Ctrl up
        inputs[3].input_type = INPUT_KEYBOARD;
        inputs[3].ki.w_vk = VK_CONTROL;
        inputs[3].ki.dw_flags = KEYEVENTF_KEYUP;

        unsafe {
            SendInput(4, inputs.as_ptr(), size_of::<Input>() as i32);
        }

        thread::sleep(Duration::from_millis(10));
    }
}
