//! CGEvent を使用したキーボード入力のインジェクション。
//!
//! このモジュールは、Unicode サポートを備えた macOS の CGEvent ベースのキーボードシミュレーションを使用して、
//! アクティブなアプリケーションにテキストを挿入する機能を提供します。

#[link(name = "CoreGraphics", kind = "framework")]
#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {}

use crate::constants::{DELETION_COOLDOWN_MS_MAC, DELETION_WEIGHT_MS_MAC, KEY_DELAY_MS_MAC};
use std::ffi::c_void;
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};

/// キー削除完了のデッドライン（期限）のグローバルリスト。
/// 進行中の全ての削除操作が論理的に完了するまで、タイピングをブロックするために使用されます。
static DELETION_DEADLINES: Mutex<Vec<Instant>> = Mutex::new(Vec::new());

/// 全てのキーボード入力操作をシリアル化するためのグローバルロック。
/// これにより、一度に進行できるinput_diff/type_text/send_backspaces操作は1つだけであり、
/// 競合状態を防ぎます。
static INPUT_LOCK: Mutex<()> = Mutex::new(());

/// 全ての保留中の削除デッドラインが経過するまで待機します。
/// この関数は過去のデッドラインのガベージコレクションを行い、残っているものがあればブロックします。
fn wait_for_deletion_completion() {
    loop {
        {
            let mut deadlines = DELETION_DEADLINES.lock().unwrap();
            let now = Instant::now();
            // ガベージコレクション: 過去のデッドラインを全て削除
            deadlines.retain(|&deadline| deadline > now);
            if deadlines.is_empty() {
                return; // 全ての削除が完了。タイピングを続行可能。
            }
            log::debug!(
                "[KeyboardInjector] Waiting for {} deletion deadline(s) to complete...",
                deadlines.len()
            );
        }
        // 短時間スリープして再チェック
        thread::sleep(Duration::from_millis(10));
    }
}

/// 明確さのためのCGKeyCode型エイリアス。
pub type CGKeyCode = u16;

/// キー押下をシミュレートするためのキーボードインジェクター。
pub struct KeyboardInjector;

impl KeyboardInjector {
    /// プロセスがアクセシビリティ権限を持っているか確認します。
    /// 許可されている場合は true、そうでない場合は false を返します。
    pub fn is_authorized() -> bool {
        unsafe {
            // ApplicationServices の AXIsProcessTrusted を使用
            extern "C" {
                fn AXIsProcessTrusted() -> bool;
            }
            AXIsProcessTrusted()
        }
    }

    /// Unicode サポートを完備したキーイベントをシミュレートして、指定されたテキストを入力します。
    /// このメソッドは、日本語/Unicode 入力を適切に行うために CGEventKeyboardSetUnicodeString を使用します。
    /// これはパブリックなエントリポイントであり、グローバルロックを取得します。
    pub fn type_text(text: &str) {
        let _lock = INPUT_LOCK.lock().unwrap();
        Self::type_text_inner(text);
    }

    /// type_text の内部実装（ロック取得なし）。
    fn type_text_inner(text: &str) {
        // 入力前に保留中の削除が全て完了するまで待機
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

            // CGEventKeyboardSetUnicodeString 用にテキストを UTF-16 に変換
            let utf16: Vec<u16> = text.encode_utf16().collect();

            // チャンクごとに処理（CGEvent には1イベントあたり約20文字の制限があるため）
            const CHUNK_SIZE: usize = 16;

            for chunk in utf16.chunks(CHUNK_SIZE) {
                // ソースを指定してキーダウンイベントを作成
                let event_down = CGEventCreateKeyboardEvent(source, 0, true);
                if event_down.is_null() {
                    continue;
                }

                // Unicode 文字列を設定
                CGEventKeyboardSetUnicodeString(event_down, chunk.len() as u64, chunk.as_ptr());

                // キーダウンイベントをポスト
                CGEventPost(0, event_down);
                CFRelease(event_down as *mut c_void);

                // キーダウン後の待機（OS/アプリが押下を認識するための時間）
                thread::sleep(Duration::from_millis(KEY_DELAY_MS_MAC));

                // キーアップイベントを作成してポスト（長押し挙動を防ぐために極めて重要）
                let event_up = CGEventCreateKeyboardEvent(source, 0, false);
                if !event_up.is_null() {
                    CGEventKeyboardSetUnicodeString(event_up, chunk.len() as u64, chunk.as_ptr());
                    CGEventPost(0, event_up);
                    CFRelease(event_up as *mut c_void);
                }

                // キーアップ後の待機（OS/アプリが処理を完了するための時間）
                thread::sleep(Duration::from_millis(KEY_DELAY_MS_MAC));
            }

            if !source.is_null() {
                CFRelease(source as *mut c_void);
            }
        }
    }

    /// バックスペースキーを送信して文字を削除します。
    /// count: 削除する UTF-8 文字数 (text.chars().count() を使用)
    /// これはパブリックなエントリポイントであり、グローバルロックを取得します。
    pub fn send_backspaces(count: usize) {
        if count == 0 {
            return;
        }
        let _lock = INPUT_LOCK.lock().unwrap();
        Self::send_backspaces_inner(count);
    }

    /// send_backspaces の内部実装（ロック取得なし）。
    fn send_backspaces_inner(count: usize) {
        if count == 0 {
            return;
        }

        // この削除バッチのデッドラインを計算して登録します。
        // 各バックスペースにはダウンとアップの両方のイベントがあるため、KEY_DELAY_MS を2倍します。
        // また、文字数に応じた動的なクールダウン（セトリング時間）を加算します。
        let dynamic_cooldown = DELETION_COOLDOWN_MS_MAC + (count as u64 * DELETION_WEIGHT_MS_MAC);
        let estimated_duration_ms = (count as u64 * KEY_DELAY_MS_MAC * 2) + dynamic_cooldown;
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
                // キーダウン
                let event_down = CGEventCreateKeyboardEvent(source, BACKSPACE_KEYCODE, true);
                if !event_down.is_null() {
                    CGEventPost(0, event_down);
                    CFRelease(event_down as *mut c_void);
                }

                // キーダウン後の待機（OS/アプリが押下を認識するための時間）
                thread::sleep(Duration::from_millis(KEY_DELAY_MS_MAC));

                // キーアップ
                let event_up = CGEventCreateKeyboardEvent(source, BACKSPACE_KEYCODE, false);
                if !event_up.is_null() {
                    CGEventPost(0, event_up);
                    CFRelease(event_up as *mut c_void);
                }

                // キーアップ後の待機（OS/アプリが削除を完了するための時間）
                thread::sleep(Duration::from_millis(KEY_DELAY_MS_MAC));
            }

            if !source.is_null() {
                CFRelease(source as *mut c_void);
            }
        }
    }

    /// 旧テキストと新テキストを比較し、増分でテキストを入力します。
    /// これによりバックスペースとタイピングを最小限に抑え、スムーズなエクスペリエンスを提供します。
    /// このメソッドは完全にシリアル化されています。一度に実行できる input_diff は1つだけです。
    pub fn input_diff(old_text: &str, new_text: &str) {
        // グローバルロックを取得して、全ての入力操作をシリアル化
        let _lock = INPUT_LOCK.lock().unwrap();

        log::debug!(
            "[KeyboardInjector] input_diff: \"{}\" -> \"{}\"",
            old_text,
            new_text
        );
        // 共通プレフィックスの長さを算出（文字単位）
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

        // old_text の末尾から削除する必要のある文字数を計算
        let delete_count = old_chars.len() - common_prefix_chars;
        if delete_count > 0 {
            log::debug!("[KeyboardInjector] delete_count: {}", delete_count);
            Self::send_backspaces_inner(delete_count);

            // [WAIT_ALPHA] 動的な削除クールダウン
            // 大規模な削除の後に OS/IME が処理を完了できるよう、タイピング開始前に待機します。
            // 安定性のため、delete_count に比例した十分なセトリング時間を確保します。
            let dynamic_cooldown =
                DELETION_COOLDOWN_MS_MAC + (delete_count as u64 * DELETION_WEIGHT_MS_MAC);
            thread::sleep(Duration::from_millis(dynamic_cooldown));
        }

        // new_text の入力が必要な部分を抽出
        let type_string: String = new_chars[common_prefix_chars..].iter().collect();
        if !type_string.is_empty() {
            log::debug!("[KeyboardInjector] type_string: \"{}\"", type_string);
            Self::type_text_inner(&type_string);
        }
    }

    /// Cmd+C (コピー) キー送信。
    pub fn send_cmd_c() {
        Self::send_cmd_key(8); // C のキーコード
    }

    /// Cmd+V (ペースト) キー送信。
    pub fn send_cmd_v() {
        Self::send_cmd_key(9); // V のキーコード
    }

    /// Cmd+キーの組み合わせを送信。
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

            // Cmd と一緒にキーダウン
            let event_down = CGEventCreateKeyboardEvent(std::ptr::null_mut(), keycode, true);
            if !event_down.is_null() {
                CGEventSetFlags(event_down, CMD_FLAG);
                CGEventPost(0, event_down);
                CFRelease(event_down as *mut c_void);
            }

            thread::sleep(Duration::from_millis(10));

            // キーアップ
            let event_up = CGEventCreateKeyboardEvent(std::ptr::null_mut(), keycode, false);
            if !event_up.is_null() {
                CGEventPost(0, event_up);
                CFRelease(event_up as *mut c_void);
            }

            thread::sleep(Duration::from_millis(10));
        }
    }
}
