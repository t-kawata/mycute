pub mod clipboard;

#[cfg(target_os = "macos")]
pub mod keyboard_mac;
#[cfg(target_os = "macos")]
pub use keyboard_mac as keyboard;

#[cfg(target_os = "windows")]
pub mod keyboard_win;
#[cfg(target_os = "windows")]
pub use keyboard_win as keyboard;
