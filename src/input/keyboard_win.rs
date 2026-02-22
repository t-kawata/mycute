//! winapi を使用した Windows キーボード入力のインジェクション。
//!
//! このモジュールは、Unicode サポートを備えた Windows SendInput API を使用して、
//! アクティブなアプリケーションにテキストを挿入する機能を提供します。

use crate::constants::{DELETION_COOLDOWN_MS_WIN, KEY_DELAY_MS_WIN};
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

// --- 64ビット用 Windows API 構造体 ---

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
    _pad: u32,           // 64ビット用のアライメントパディング
    ki: KeybdInput,      // 24バイト (usize の前の内部パディングを含む)
    _union_pad: [u8; 8], // union を 32 バイトにするための 8 バイト。構造体合計: 40 バイト。
}

const INPUT_KEYBOARD: u32 = 1;
const KEYEVENTF_UNICODE: u32 = 0x0004;
const KEYEVENTF_KEYUP: u32 = 0x0002;
const VK_CONTROL: u16 = 0x11;
const VK_V: u16 = 0x56;
const VK_BACK: u16 = 0x08;

#[link(name = "user32")] // SendInput のために user32.dll を規定
extern "system" {
    fn SendInput(c_inputs: u32, p_inputs: *const Input, cb_size: i32) -> u32;
}

/// ホットキーモニターでのタイピングモード状態を管理するためのヘルパー。
/// これは Mac の MYCUTE_EVENT_ID と同じ目的（自己トリガーによるコミットの防止）を果たします。
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

/// 互換性のための CGKeyCode 型エイリアス。
pub type CGKeyCode = u16;

/// キー押下をシミュレートするためのキーボードインジェクター。
pub struct KeyboardInjector;

impl KeyboardInjector {
    /// プロセスがアクセシビリティ権限を持っているか確認します。
    /// Windows では、これは常に true を返します。
    pub fn is_authorized() -> bool {
        true
    }

    /// Unicode サポートを完備したキーイベントをシミュレートして、指定されたテキストを入力します。
    /// このメソッドは、Mac の CGEventKeyboardSetUnicodeString の挙動に合わせて、テキストをチャンクで処理します。
    /// これはパブリックなエントリポイントであり、グローバルロックを取得します。
    pub fn type_text(text: &str) {
        let _lock = INPUT_LOCK.lock().unwrap();
        let _guard = TypingGuard::new();
        Self::type_text_inner(text);
    }

    /// type_text の内部実装（ロック取得なし）。
    fn type_text_inner(text: &str) {
        // 入力前に保留中の削除が全て完了するまで待機
        wait_for_deletion_completion();

        // Windows API 用に UTF-16 に変換
        let utf16: Vec<u16> = text.encode_utf16().collect();
        if utf16.is_empty() {
            return;
        }

        // チャンクごとに処理（Mac の CHUNK_SIZE アプローチに合わせる）
        const CHUNK_SIZE: usize = 16;

        for chunk in utf16.chunks(CHUNK_SIZE) {
            // チャンク内の各文字を、ダウン + アップイベントで送信
            // 安定性のために Down-Wait-Up-Wait プロトコルを使用
            for &code_unit in chunk {
                use std::mem::size_of;

                // Unicode 文字によるキーダウン
                let mut input_down: Input = unsafe { std::mem::zeroed() };
                input_down.input_type = INPUT_KEYBOARD;
                input_down.ki.w_vk = 0; // KEYEVENTF_UNICODE の場合は 0 である必要がある
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

                // キーダウン後の待機（OS/アプリが押下を認識するための時間）
                thread::sleep(Duration::from_millis(KEY_DELAY_MS_WIN));

                // キーアップ
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

                // キーアップ後の待機（OS/アプリが処理を完了するための時間）
                thread::sleep(Duration::from_millis(KEY_DELAY_MS_WIN));
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
        let _guard = TypingGuard::new();
        Self::send_backspaces_inner(count);
    }

    /// send_backspaces の内部実装（ロック取得なし）。
    fn send_backspaces_inner(count: usize) {
        if count == 0 {
            return;
        }

        // この削除バッチのデッドラインを計算して登録します。
        // 各バックスペースにはダウンとアップの両方のイベントがあるため、KEY_DELAY_MS を2倍します。
        // また、文字数に応じた動的なクールダウン（Dynamic α）を加算します。
        let dynamic_cooldown = DELETION_COOLDOWN_MS_WIN + (count as u64 * 2);
        let estimated_duration_ms = (count as u64 * KEY_DELAY_MS_WIN * 2) + dynamic_cooldown;
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

        // Down-Wait-Up-Wait プロトコルを使用して、1つずつバックスペースを処理
        for _ in 0..count {
            // バックスペース・ダウン
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

            // キーダウン後の待機（OS/アプリが押下を認識するための時間）
            thread::sleep(Duration::from_millis(KEY_DELAY_MS_WIN));

            // バックスペース・アップ
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

            // キーアップ後の待機（OS/アプリが削除を完了するための時間）
            thread::sleep(Duration::from_millis(KEY_DELAY_MS_WIN));
        }
    }

    /// 旧テキストと新テキストを比較し、増分でテキストを入力します。
    /// これによりバックスペースとタイピングを最小限に抑え、スムーズなエクスペリエンスを提供します。
    /// このメソッドは完全にシリアル化されています。一度に実行できる input_diff は1つだけです。
    pub fn input_diff(old_text: &str, new_text: &str) {
        log::info!("[WinInputDebug] input_diff start. waiting for lock...");
        // 全ての入力操作をシリアル化するためのグローバルロックを取得
        let _lock = INPUT_LOCK.lock().unwrap();
        log::info!("[WinInputDebug] input_diff lock acquired.");
        let _guard = TypingGuard::new(); // diff プロセス全体でフラグを ON に維持

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
            // 安定性のため、delete_count に比例した時間を確保します。
            let dynamic_cooldown = DELETION_COOLDOWN_MS_WIN + (delete_count as u64 * 2);
            thread::sleep(Duration::from_millis(dynamic_cooldown));
        }

        // new_text の入力が必要な部分を抽出
        let type_string: String = new_chars[common_prefix_chars..].iter().collect();
        if !type_string.is_empty() {
            log::debug!("[KeyboardInjector] type_string: \"{}\"", type_string);
            Self::type_text_inner(&type_string);
        }
    }

    /// Cmd+C (コピー) キー送信 - Windows では Ctrl+C
    pub fn send_cmd_c() {
        Self::send_ctrl_key(0x43); // C キー
    }

    /// Cmd+V (ペースト) キー送信 - Windows では Ctrl+V
    pub fn send_cmd_v() {
        Self::send_ctrl_key(VK_V);
    }

    /// Ctrl+キーの組み合わせを送信。
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
