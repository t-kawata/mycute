//! Keyboard input injection using CGEvent.
//!
//! This module provides functionality to inject text into the active application
//! using macOS CGEvent-based keyboard simulation with Unicode support.

#[link(name = "CoreGraphics", kind = "framework")]
#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {}

use std::ffi::c_void;
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};

/// Delay between key presses in milliseconds.
const KEY_DELAY_MS: u64 = 1;

/// Cooldown after deletion for app-side UI update (α).
/// Reduced from 250ms: the new Down-Wait-Up-Wait protocol provides
/// sufficient pacing, so we only need a short buffer for final UI sync.
const DELETION_COOLDOWN_MS: u64 = 30;

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

/// CGKeyCode type alias for clarity.
pub type CGKeyCode = u16;

/// Keyboard injector for simulating key presses.
pub struct KeyboardInjector;

impl KeyboardInjector {
    /// Check if the process has accessibility permission.
    /// Returns true if authorized, false otherwise.
    pub fn is_authorized() -> bool {
        unsafe {
            // Use AXIsProcessTrusted from ApplicationServices
            extern "C" {
                fn AXIsProcessTrusted() -> bool;
            }
            AXIsProcessTrusted()
        }
    }

    /// Type the given text by simulating key events with full Unicode support.
    /// This method uses CGEventKeyboardSetUnicodeString for proper Japanese/Unicode input.
    /// This is the PUBLIC entry point - acquires the global lock.
    pub fn type_text(text: &str) {
        let _lock = INPUT_LOCK.lock().unwrap();
        Self::type_text_inner(text);
    }

    /// Internal implementation of type_text (no lock acquisition).
    fn type_text_inner(text: &str) {
        // Wait for any pending deletions to complete before typing
        wait_for_deletion_completion();
        
        unsafe {
            extern "C" {
                fn CGEventCreateKeyboardEvent(
                    source: *mut (),
                    virtual_key: CGKeyCode,
                    key_down: bool,
                ) -> *mut ();
                fn CGEventKeyboardSetUnicodeString(
                    event: *mut (),
                    string_length: u64,
                    unicode_string: *const u16,
                );
                fn CGEventPost(tap: u32, event: *mut ());
                fn CFRelease(cf: *mut c_void);
                fn CGEventSourceCreate(state_id: i32) -> *mut ();
                fn CGEventSourceSetUserData(source: *mut (), user_data: i64);
            }

            // kCGEventSourceStateCombinedSessionState = 0
            let source = CGEventSourceCreate(0);
            const MYCUTE_EVENT_ID: i64 = 0x4D594355;
            if !source.is_null() {
                CGEventSourceSetUserData(source, MYCUTE_EVENT_ID);
            }

            // Convert text to UTF-16 for CGEventKeyboardSetUnicodeString
            let utf16: Vec<u16> = text.encode_utf16().collect();

            // Process in chunks (CGEvent has a limit of ~20 characters per event)
            const CHUNK_SIZE: usize = 16;

            for chunk in utf16.chunks(CHUNK_SIZE) {
                // Create a key down event with our source
                let event_down = CGEventCreateKeyboardEvent(source, 0, true);
                if event_down.is_null() {
                    continue;
                }

                // Set the Unicode string
                CGEventKeyboardSetUnicodeString(event_down, chunk.len() as u64, chunk.as_ptr());

                // Post the key down event
                CGEventPost(0, event_down);
                CFRelease(event_down as *mut c_void);

                // Wait after key down (allows OS/app to register the press)
                thread::sleep(Duration::from_millis(KEY_DELAY_MS));

                // Create and post a key up event (Crucial to prevent long-press behavior)
                let event_up = CGEventCreateKeyboardEvent(source, 0, false);
                if !event_up.is_null() {
                    CGEventKeyboardSetUnicodeString(event_up, chunk.len() as u64, chunk.as_ptr());
                    CGEventPost(0, event_up);
                    CFRelease(event_up as *mut c_void);
                }

                // Wait after key up (allows OS/app to complete processing)
                thread::sleep(Duration::from_millis(KEY_DELAY_MS));
            }

            if !source.is_null() {
                CFRelease(source as *mut c_void);
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
                count, estimated_duration_ms
            );
        }
        
        unsafe {
            extern "C" {
                fn CGEventCreateKeyboardEvent(
                    source: *mut (),
                    virtual_key: CGKeyCode,
                    key_down: bool,
                ) -> *mut ();
                fn CGEventPost(tap: u32, event: *mut ());
                fn CFRelease(cf: *mut c_void);
                fn CGEventSourceCreate(state_id: i32) -> *mut ();
                fn CGEventSourceSetUserData(source: *mut (), user_data: i64);
            }

            // kCGEventSourceStateCombinedSessionState = 0
            let source = CGEventSourceCreate(0);
            const MYCUTE_EVENT_ID: i64 = 0x4D594355;
            if !source.is_null() {
                CGEventSourceSetUserData(source, MYCUTE_EVENT_ID);
            }

            const BACKSPACE_KEYCODE: CGKeyCode = 0x33;

            for _ in 0..count {
                // Key down
                let event_down = CGEventCreateKeyboardEvent(source, BACKSPACE_KEYCODE, true);
                if !event_down.is_null() {
                    CGEventPost(0, event_down);
                    CFRelease(event_down as *mut c_void);
                }

                // Wait after key down (allows OS/app to register the press)
                thread::sleep(Duration::from_millis(KEY_DELAY_MS));

                // Key up
                let event_up = CGEventCreateKeyboardEvent(source, BACKSPACE_KEYCODE, false);
                if !event_up.is_null() {
                    CGEventPost(0, event_up);
                    CFRelease(event_up as *mut c_void);
                }

                // Wait after key up (allows OS/app to complete the deletion)
                thread::sleep(Duration::from_millis(KEY_DELAY_MS));
            }

            if !source.is_null() {
                CFRelease(source as *mut c_void);
            }
        }
    }

    /// Type text incrementally by comparing with the previous string.
    /// This minimizes backspaces and typing for a smoother experience.
    /// This method is fully serialized - only one input_diff can run at a time.
    pub fn input_diff(old_text: &str, new_text: &str) {
        // Acquire global lock to serialize all input operations
        let _lock = INPUT_LOCK.lock().unwrap();

        log::debug!("[KeyboardInjector] input_diff: \"{}\" -> \"{}\"", old_text, new_text);
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
        log::debug!("[KeyboardInjector] common_prefix_chars: {}", common_prefix_chars);

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

    /// Send Cmd+C (Copy) keystroke.
    pub fn send_cmd_c() {
        Self::send_cmd_key(8); // C keycode
    }

    /// Send Cmd+V (Paste) keystroke.
    pub fn send_cmd_v() {
        Self::send_cmd_key(9); // V keycode
    }

    /// Send Cmd+key combination.
    fn send_cmd_key(keycode: CGKeyCode) {
        unsafe {
            extern "C" {
                fn CGEventCreateKeyboardEvent(
                    source: *mut (),
                    virtual_key: CGKeyCode,
                    key_down: bool,
                ) -> *mut ();
                fn CGEventSetFlags(event: *mut (), flags: u64);
                fn CGEventPost(tap: u32, event: *mut ());
                fn CFRelease(cf: *mut c_void);
            }

            const CMD_FLAG: u64 = 0x00100000; // kCGEventFlagMaskCommand

            // Key down with Cmd
            let event_down = CGEventCreateKeyboardEvent(std::ptr::null_mut(), keycode, true);
            if !event_down.is_null() {
                CGEventSetFlags(event_down, CMD_FLAG);
                CGEventPost(0, event_down);
                CFRelease(event_down as *mut c_void);
            }

            thread::sleep(Duration::from_millis(10));

            // Key up
            let event_up = CGEventCreateKeyboardEvent(std::ptr::null_mut(), keycode, false);
            if !event_up.is_null() {
                CGEventPost(0, event_up);
                CFRelease(event_up as *mut c_void);
            }

            thread::sleep(Duration::from_millis(10));
        }
    }
}
