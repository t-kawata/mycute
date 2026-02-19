//! Utility functions for mycute

use std::ffi::CStr;

/// C 文字列を Rust の String に変換するユーティリティ
pub fn cstr_to_string(ptr: *const std::os::raw::c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    unsafe { CStr::from_ptr(ptr).to_string_lossy().into_owned() }
}

/// 日本語のフィラー（「えーと」「あの」など）を除去する
pub fn remove_fillers(text: &str) -> String {
    let fillers = [
        "えーと",
        "えっと",
        "あのー",
        "あの",
        "うーん",
        "そのー",
        "まぁ",
    ];
    let mut result = text.to_string();
    for filler in fillers {
        result = result.replace(filler, "");
    }
    result
}
