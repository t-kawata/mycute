# Tahoe Voice Dictation Tool - 完全な実装設計書

## 目次
1. [概要](#概要)
2. [技術的基礎](#技術的基礎)
3. [実装必須の前提知識](#実装必須の前提知識)
4. [プロジェクト構造](#プロジェクト構造)
5. [詳細な実装ガイド](#詳細な実装ガイド)
6. [トラブルシューティング](#トラブルシューティング)

---

## 概要

### プロジェクトの目的
macOS Tahoe（26）搭載の**オンデバイスSTT（音声テキスト変換）機能**を活用し、ホットキー操作によってリアルタイムに音声をテキスト化し、アクティブアプリケーションに注入するRustネイティブツール。

### なぜ Tahoe STT を使うのか
- **Whisper.cpp より 55% 高速**（Apple Silicon最適化）
- **オンデバイス処理**（プライバシー）
- **低遅延ストリーミング**（最新API）
- **日本語対応**（Apple による高品質モデル）

### 設計哲学
**実装複雑性を最小化しながら、本当に動く機能を提供する**
- 不確実な部分は代替実装を用意
- 外部依存を最小化
- 各モジュールは独立・テスト可能

---

## 技術的基礎

### macOS Tahoe の Speech Framework

#### SpeechAnalyzer（新規 in macOS 26）

```objc
// Objective-C での標準的な用法
@import Speech;

SpeechAnalyzer *analyzer = [[SpeechAnalyzer alloc] init];
analyzer.locale = [NSLocale localeWithLocaleIdentifier:@"ja_JP"];

// 音声バッファを処理
[analyzer processAudioBuffer:audioBuffer
                   completion:^(SpeechAnalysisResult *result) {
    NSLog(@"Partial: %@", result.partialTranscription);
    NSLog(@"Final: %@", result.finalTranscription);
}];
```

#### Rust での FFI 呼び出し戦略

Apple の Speech Framework には複数のレイヤがある：

1. **高レベルAPI：SpeechRecognizer**
   - 従来の macOS で利用可能
   - Partial result に対応（iOS 16以降、macOS 13以降）
   - より安定している

2. **低レベルAPI：SpeechAnalyzer（Tahoe新規）**
   - ハードウェア直結
   - より低遅延
   - ドキュメント少ない

**実装戦略：SpeechRecognizer を主軸に、Tahoe 特有の SpeechAnalyzer は段階的統合**

---

## 実装必須の前提知識

### 1. Objective-C ↔ Rust FFI の基本

macOS ネイティブAPIを呼び出すには、`objc2` クレートを使用します。

```rust
// objc2 の基本パターン
use objc2::{class, msg_send, sel, sel_impl};
use objc2::runtime::Object;

unsafe {
    // Class を取得
    let cls = class!(SpeechRecognizer);
    
    // インスタンスを作成
    let recognizer: *mut Object = msg_send![cls, new];
    
    // メソッドを呼び出し
    let _: () = msg_send![recognizer, setLocale: locale];
}
```

### 2. objc2-foundation の利用

NSString、NSLocale、NSURLSession などの基本型は `objc2-foundation` で提供される：

```rust
use objc2_foundation::{NSString, NSLocale};

// NSString を作成
let locale_str = NSString::from_str("ja_JP");
let locale = unsafe {
    NSLocale::alloc().init_with_locale_identifier(locale_str)
};
```

### 3. メモリ管理の重要性

Objective-C のメモリ管理は自動参照カウント（ARC）で行われますが、Rust から呼び出す場合は手動で注意が必要です：

```rust
// 所有権を Rust 側で管理する場合
use objc2::rc::{Id, Shared};

let obj: Id<Object, Shared> = unsafe {
    Id::retain(recognizer)
};
// obj がスコープを出るとき自動的に release される
```

---

## プロジェクト構造

### ディレクトリレイアウト

```
mycute/
├── Cargo.toml
├── Cargo.lock
├── src/
│   ├── main.rs                 # エントリーポイント、イベントループ
│   ├── config.rs               # JSON 設定管理
│   ├── hotkey.rs               # グローバルホットキー監視
│   ├── stt/
│   │   ├── mod.rs              # STT モジュール公開インターフェース
│   │   ├── speech_recognizer.rs # SpeechRecognizer（メイン実装）
│   │   └── audio_engine.rs     # AVAudioEngine ラッパー
│   ├── ui/
│   │   ├── mod.rs
│   │   ├── popup.rs            # フローティングポップアップ
│   │   └── settings_dialog.rs  # 設定ウィンドウ
│   ├── input/
│   │   ├── mod.rs
│   │   └── keyboard.rs         # キーボード入力シミュレーション
│   ├── llm/
│   │   ├── mod.rs
│   │   ├── manager.rs          # LLM 管理・ラウンドロビン
│   │   └── client.rs           # API 呼び出し
│   ├── filler.rs               # フィラー除去
│   ├── types.rs                # 共通型定義
│   └── utils.rs                # ユーティリティ関数
└── prompts/
    ├── fix.txt
    └── summary.txt
```

### Cargo.toml（完全版）

```toml
[package]
name = "mycute"
version = "0.1.0"
edition = "2021"

[dependencies]
# CLI
clap = { version = "4.5", features = ["derive"] }

# 設定管理
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# 非同期ランタイム
tokio = { version = "1.40", features = ["full"] }

# HTTP クライアント（LLM API）
reqwest = { version = "0.12", features = ["json"] }

# ホットキー監視
rdev = "0.5"

# Objective-C FFI
objc2 = { version = "0.5", features = ["exception"] }
objc2-foundation = "0.2"
objc2-core-foundation = "0.2"
objc2-core-graphics = "0.2"
objc2-app-kit = "0.2"

# 音声処理
cpal = "0.18"  # クロスプラットフォームオーディオAPI

# ログ
log = "0.4"
env_logger = "0.11"

# ユーティリティ
uuid = { version = "1.0", features = ["v4"] }
parking_lot = "0.12"  # 高速なMutex実装

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true
```

---

## 詳細な実装ガイド

### Module 1: 設定管理（config.rs）

設定ファイルの読み込み・キャッシング・動的更新を担当します。

```rust
// src/config.rs

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::fs;
use parking_lot::RwLock;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotkeysConfig {
    pub start: Vec<String>,
    pub commit: Vec<String>,
    pub settings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub name: String,
    pub base_url: String,
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptConfig {
    pub key: String,
    pub bind: Vec<String>,
    pub file: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub hotkeys: HotkeysConfig,
    pub llms: Vec<LlmConfig>,
    pub prompts: Vec<PromptConfig>,
    #[serde(default)]
    pub filler_words: Vec<String>,
}

impl Settings {
    /// settings.json を読み込む
    pub fn load(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        if !path.exists() {
            return Err(format!("Config file not found: {:?}", path).into());
        }

        let content = fs::read_to_string(path)?;
        let mut config: Settings = serde_json::from_str(&content)?;

        // デフォルト値の設定
        if config.filler_words.is_empty() {
            config.filler_words = Self::default_filler_words();
        }

        Ok(config)
    }

    /// デフォルトフィラー一覧
    fn default_filler_words() -> Vec<String> {
        vec![
            "えー", "あのー", "えっと", "その", "あ",
            "まあ", "いえ", "んー", "うーん",
            "ほら", "見ての通り", "いわば",
        ]
        .into_iter()
        .map(|s| s.to_string())
        .collect()
    }

    /// 有効なLLMのみを取得
    pub fn enabled_llms(&self) -> Vec<&LlmConfig> {
        self.llms.iter().filter(|l| l.enabled).collect()
    }

    /// 設定を JSON 形式で保存
    pub fn save(&self, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }
}

/// スレッドセーフな設定ホルダー
pub struct ConfigManager {
    config: Arc<RwLock<Settings>>,
    path: PathBuf,
}

impl ConfigManager {
    pub fn new(path: PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let config = Settings::load(&path)?;
        Ok(ConfigManager {
            config: Arc::new(RwLock::new(config)),
            path,
        })
    }

    pub fn get<T, F>(&self, f: F) -> T
    where
        F: FnOnce(&Settings) -> T,
    {
        let cfg = self.config.read();
        f(&cfg)
    }

    pub fn reload(&self) -> Result<(), Box<dyn std::error::Error>> {
        let new_config = Settings::load(&self.path)?;
        *self.config.write() = new_config;
        Ok(())
    }

    pub fn update<F>(&self, f: F) -> Result<(), Box<dyn std::error::Error>>
    where
        F: FnOnce(&mut Settings),
    {
        {
            let mut cfg = self.config.write();
            f(&mut cfg);
        }
        let cfg = self.config.read();
        cfg.save(&self.path)?;
        Ok(())
    }
}
```

### Module 2: ホットキー監視（hotkey.rs）

グローバルホットキーを監視し、ユーザーアクションを検出します。

```rust
// src/hotkey.rs

use rdev::{listen, EventType, Key};
use std::collections::HashMap;
use tokio::sync::mpsc;
use log::debug;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyAction {
    Start,
    Commit,
    Settings,
    PromptCustom(usize), // prompt インデックス
}

pub struct HotkeyListener {
    config_hotkeys: crate::config::HotkeysConfig,
    prompt_bindings: Vec<Vec<String>>,
}

impl HotkeyListener {
    pub fn new(
        config_hotkeys: crate::config::HotkeysConfig,
        prompt_bindings: Vec<Vec<String>>,
    ) -> Self {
        HotkeyListener {
            config_hotkeys,
            prompt_bindings,
        }
    }

    /// グローバルホットキーリスナーを開始
    /// 別スレッドで実行され、イベントが mpsc チャネル経由で返される
    pub fn start(self) -> mpsc::Receiver<HotkeyAction> {
        let (tx, rx) = mpsc::channel(10);

        std::thread::spawn(move || {
            let mut modifier_state = ModifierState::new();

            if let Err(err) = listen(move |event| {
                match event.event_type {
                    EventType::KeyPress(key) => {
                        modifier_state.update_press(key);
                        
                        // 各ホットキーをチェック
                        if Self::matches(&modifier_state, &self.config_hotkeys.start) {
                            let _ = tx.blocking_send(HotkeyAction::Start);
                            debug!("Hotkey: Start");
                        } else if Self::matches(&modifier_state, &self.config_hotkeys.commit) {
                            let _ = tx.blocking_send(HotkeyAction::Commit);
                            debug!("Hotkey: Commit");
                        } else if Self::matches(&modifier_state, &self.config_hotkeys.settings) {
                            let _ = tx.blocking_send(HotkeyAction::Settings);
                            debug!("Hotkey: Settings");
                        }

                        // プロンプトホットキーをチェック
                        for (idx, binding) in self.prompt_bindings.iter().enumerate() {
                            if Self::matches(&modifier_state, binding) {
                                let _ = tx.blocking_send(HotkeyAction::PromptCustom(idx));
                                debug!("Hotkey: Prompt({})", idx);
                            }
                        }
                    }
                    EventType::KeyRelease(key) => {
                        modifier_state.update_release(key);
                    }
                    _ => {}
                }
            }) {
                eprintln!("Failed to listen to global hotkey: {}", err);
            }
        });

        rx
    }

    /// 修飾キーの状態とホットキー定義をマッチング
    fn matches(state: &ModifierState, hotkey_def: &[String]) -> bool {
        for key_name in hotkey_def {
            match key_name.as_str() {
                "Control" => {
                    if !state.is_control_pressed() {
                        return false;
                    }
                }
                "Shift" => {
                    if !state.is_shift_pressed() {
                        return false;
                    }
                }
                "Command" => {
                    if !state.is_command_pressed() {
                        return false;
                    }
                }
                "Option" => {
                    if !state.is_option_pressed() {
                        return false;
                    }
                }
                other => {
                    // 通常キー
                    if !state.is_key_pressed(other) {
                        return false;
                    }
                }
            }
        }
        true
    }
}

/// 修飾キーとキー押下状態を管理
struct ModifierState {
    control: bool,
    shift: bool,
    command: bool,
    option: bool,
    pressed_keys: HashMap<String, bool>,
}

impl ModifierState {
    fn new() -> Self {
        ModifierState {
            control: false,
            shift: false,
            command: false,
            option: false,
            pressed_keys: HashMap::new(),
        }
    }

    fn update_press(&mut self, key: Key) {
        match key {
            Key::ControlLeft | Key::ControlRight => self.control = true,
            Key::ShiftLeft | Key::ShiftRight => self.shift = true,
            Key::MetaLeft | Key::MetaRight => self.command = true, // Command = Meta on macOS
            Key::AltLeft | Key::AltRight => self.option = true,
            _ => {
                self.pressed_keys.insert(self.key_to_string(key), true);
            }
        }
    }

    fn update_release(&mut self, key: Key) {
        match key {
            Key::ControlLeft | Key::ControlRight => self.control = false,
            Key::ShiftLeft | Key::ShiftRight => self.shift = false,
            Key::MetaLeft | Key::MetaRight => self.command = false,
            Key::AltLeft | Key::AltRight => self.option = false,
            _ => {
                self.pressed_keys.remove(&self.key_to_string(key));
            }
        }
    }

    fn is_control_pressed(&self) -> bool {
        self.control
    }

    fn is_shift_pressed(&self) -> bool {
        self.shift
    }

    fn is_command_pressed(&self) -> bool {
        self.command
    }

    fn is_option_pressed(&self) -> bool {
        self.option
    }

    fn is_key_pressed(&self, key_name: &str) -> bool {
        self.pressed_keys.contains_key(key_name)
    }

    fn key_to_string(&self, key: Key) -> String {
        match key {
            Key::KeyA => "KeyA".to_string(),
            Key::KeyB => "KeyB".to_string(),
            // ... その他のキー
            Key::Return => "Return".to_string(),
            _ => format!("{:?}", key),
        }
    }
}
```

### Module 3: 音声認識（stt/mod.rs と stt/speech_recognizer.rs）

#### stt/mod.rs - 公開インターフェース

```rust
// src/stt/mod.rs

pub mod speech_recognizer;
pub mod audio_engine;

pub use speech_recognizer::SpeechRecognizer;

#[derive(Debug, Clone)]
pub enum SttEvent {
    /// 音声認識の途中結果
    PartialResult(String),
    /// 認識確定
    FinalResult(String),
    /// エラー発生
    Error(String),
}
```

#### stt/speech_recognizer.rs - 実装本体

```rust
// src/stt/speech_recognizer.rs

use objc2::{class, msg_send, sel, sel_impl};
use objc2::rc::{Id, Shared};
use objc2::runtime::Object;
use objc2_foundation::{NSLocale, NSString, NSError, NSObject};
use tokio::sync::mpsc;
use log::{debug, error};
use std::sync::{Arc, Mutex};

use super::SttEvent;
use super::audio_engine::AudioEngine;

pub struct SpeechRecognizer {
    recognizer: Id<Object, Shared>,
    audio_engine: Option<AudioEngine>,
    event_tx: Arc<Mutex<Option<mpsc::Sender<SttEvent>>>>,
}

impl SpeechRecognizer {
    /// SpeechRecognizer を初期化
    pub fn new(locale: &str) -> Result<Self, String> {
        unsafe {
            let cls = class!(SpeechRecognizer);
            if cls.is_null() {
                return Err("SpeechRecognizer class not found".to_string());
            }

            // インスタンスを作成
            let recognizer: *mut Object = msg_send![cls, new];
            if recognizer.is_null() {
                return Err("Failed to create SpeechRecognizer".to_string());
            }

            let recognizer = Id::retain(recognizer)
                .ok_or("Failed to retain SpeechRecognizer")?;

            // ロケールを設定
            let locale_id = NSString::from_str(locale);
            let locale = NSLocale::alloc()
                .init_with_locale_identifier(&locale_id);
            let _: () = msg_send![&recognizer, setLocale: locale];

            // Partial result に対応させる
            let _: () = msg_send![&recognizer, setSupportsOnDeviceRecognition: true];

            debug!("SpeechRecognizer initialized with locale: {}", locale);

            Ok(SpeechRecognizer {
                recognizer,
                audio_engine: None,
                event_tx: Arc::new(Mutex::new(None)),
            })
        }
    }

    /// 音声認識を開始
    pub async fn start(&mut self) -> Result<mpsc::Receiver<SttEvent>, String> {
        // オーディオエンジンを初期化
        let audio_engine = AudioEngine::new()
            .map_err(|e| format!("Failed to create audio engine: {}", e))?;

        self.audio_engine = Some(audio_engine);

        // イベントチャネルを作成
        let (tx, rx) = mpsc::channel(100);
        *self.event_tx.lock().unwrap() = Some(tx.clone());

        // 音声認識を開始（実際のコールバック処理は delegate を設定して行う）
        unsafe {
            let _: () = msg_send![&self.recognizer, startListening];
        }

        debug!("Speech recognition started");
        Ok(rx)
    }

    /// 音声認識を停止
    pub fn stop(&mut self) -> Result<(), String> {
        unsafe {
            let _: () = msg_send![&self.recognizer, stopListening];
        }

        self.audio_engine = None;
        debug!("Speech recognition stopped");
        Ok(())
    }

    /// Partial result を処理（コールバック から呼ばれる）
    pub(crate) fn handle_partial_result(&self, text: &str) {
        if let Some(tx) = self.event_tx.lock().unwrap().as_ref() {
            let _ = tx.blocking_send(SttEvent::PartialResult(text.to_string()));
        }
    }

    /// Final result を処理（コールバック から呼ばれる）
    pub(crate) fn handle_final_result(&self, text: &str) {
        if let Some(tx) = self.event_tx.lock().unwrap().as_ref() {
            let _ = tx.blocking_send(SttEvent::FinalResult(text.to_string()));
        }
    }

    /// エラーを処理
    pub(crate) fn handle_error(&self, error: &str) {
        error!("STT Error: {}", error);
        if let Some(tx) = self.event_tx.lock().unwrap().as_ref() {
            let _ = tx.blocking_send(SttEvent::Error(error.to_string()));
        }
    }
}

impl Drop for SpeechRecognizer {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
```

#### stt/audio_engine.rs - オーディオ入力

```rust
// src/stt/audio_engine.rs

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Stream, StreamConfig};
use std::sync::{Arc, Mutex};
use log::debug;

pub struct AudioEngine {
    stream: Option<Stream>,
    config: StreamConfig,
}

impl AudioEngine {
    /// オーディオ入力を初期化
    pub fn new() -> Result<Self, String> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or("No input device found")?;

        debug!("Using audio device: {}", device.name().unwrap_or_default());

        // 設定を取得（16kHz, mono, 16-bit PCM が理想）
        let config = device
            .supported_input_configs()
            .map_err(|e| format!("Failed to get input configs: {}", e))?
            .find(|c| {
                c.sample_rate().0 == 16000 || c.sample_rate().0 == 48000
            })
            .ok_or("No suitable input config found")?
            .with_sample_rate(cpal::SampleRate(16000));

        debug!("Audio config: {:?}", config);

        let stream_config = config.config();

        // ストリームを作成（ダミー実装：実際は SpeechRecognizer に接続）
        let stream = device
            .build_input_stream(
                &stream_config,
                move |_data: &cpal::Data, _info: &cpal::InputCallbackInfo| {
                    // ここでオーディオデータを SpeechRecognizer に渡す
                    // （実装は Tahoe API のコールバック機構に依存）
                },
                |err| eprintln!("Stream error: {}", err),
            )
            .map_err(|e| format!("Failed to build input stream: {}", e))?;

        stream.play()
            .map_err(|e| format!("Failed to play stream: {}", e))?;

        Ok(AudioEngine {
            stream: Some(stream),
            config: stream_config.clone(),
        })
    }
}

impl Drop for AudioEngine {
    fn drop(&mut self) {
        if let Some(stream) = self.stream.take() {
            let _ = stream.pause();
        }
    }
}
```

### Module 4: UI ポップアップ（ui/popup.rs）

```rust
// src/ui/popup.rs

use objc2::{class, msg_send, sel, sel_impl};
use objc2::rc::{Id, Shared};
use objc2::runtime::Object;
use objc2_foundation::{NSString, NSRect, NSPoint, NSSize};
use objc2_core_graphics::{CGDisplay, CGPoint};
use objc2_app_kit::{NSWindow, NSTextView};
use log::debug;

pub struct PopupWindow {
    window: Option<Id<NSWindow, Shared>>,
    text_view: Option<Id<NSTextView, Shared>>,
}

impl PopupWindow {
    /// カーソル周辺にポップアップを作成
    pub fn create_near_cursor() -> Result<Self, String> {
        unsafe {
            // カーソル位置を取得
            let (cursor_x, cursor_y) = Self::get_cursor_position()?;
            
            // スクリーン座標を取得
            let (screen_width, screen_height) = Self::get_screen_size()?;

            // ポップアップサイズを定義
            let popup_width: f64 = 300.0;
            let popup_height: f64 = 100.0;

            // 配置位置を決定（カーソルの下側、画面端判定）
            let (popup_x, popup_y) = Self::decide_position(
                cursor_x, cursor_y,
                screen_width, screen_height,
                popup_width, popup_height,
            );

            debug!("Creating popup at ({}, {})", popup_x, popup_y);

            // NSWindow を作成
            let window_class = class!(NSWindow);
            let window: *mut Object = msg_send![window_class, alloc];
            let rect = NSRect::new(
                NSPoint::new(popup_x, popup_y),
                NSSize::new(popup_width, popup_height),
            );

            // Window を初期化
            let window: *mut NSWindow = msg_send![window, initWithContentRect:rect styleMask:15 backing:2 defer:0];
            if window.is_null() {
                return Err("Failed to create NSWindow".to_string());
            }

            let window = Id::retain(window as *mut Object)
                .ok_or("Failed to retain NSWindow")?;

            // NSTextView を作成
            let text_view_class = class!(NSTextView);
            let text_view: *mut Object = msg_send![text_view_class, alloc];
            let text_view: *mut NSTextView = msg_send![text_view, initWithFrame:rect];
            if text_view.is_null() {
                return Err("Failed to create NSTextView".to_string());
            }

            let text_view = Id::retain(text_view as *mut Object)
                .ok_or("Failed to retain NSTextView")?;

            // テキストビューを Window に追加
            let content_view: *mut Object = msg_send![window, contentView];
            let _: () = msg_send![content_view, addSubview: text_view];

            // Window を表示
            let _: () = msg_send![&window, makeKeyAndOrderFront: std::ptr::null::<Object>()];

            Ok(PopupWindow {
                window: Some(window),
                text_view: Some(text_view),
            })
        }
    }

    /// テキストを追記（置換ではなく）
    pub fn append_text(&self, text: &str) -> Result<(), String> {
        if let Some(text_view) = &self.text_view {
            unsafe {
                let ns_text = NSString::from_str(text);
                let _: () = msg_send![text_view, insertText: ns_text];
            }
            Ok(())
        } else {
            Err("Text view not available".to_string())
        }
    }

    /// テキストを取得
    pub fn get_text(&self) -> Result<String, String> {
        if let Some(text_view) = &self.text_view {
            unsafe {
                let string: *mut Object = msg_send![text_view, string];
                let utf8: *const u8 = msg_send![string, UTF8String];
                if utf8.is_null() {
                    return Ok(String::new());
                }
                let c_str = std::ffi::CStr::from_ptr(utf8 as *const i8);
                Ok(c_str.to_string_lossy().to_string())
            }
        } else {
            Err("Text view not available".to_string())
        }
    }

    /// ウィンドウを閉じる
    pub fn close(&mut self) -> Result<(), String> {
        if let Some(window) = self.window.take() {
            unsafe {
                let _: () = msg_send![&window, close];
            }
        }
        Ok(())
    }

    // ユーティリティ関数

    fn get_cursor_position() -> Result<(f64, f64), String> {
        use objc2_core_graphics::{CGEventCreate, CGEventGetLocation};

        unsafe {
            let event = CGEventCreate(std::ptr::null_mut());
            if event.is_null() {
                return Err("Failed to get cursor event".to_string());
            }
            let location: CGPoint = msg_send![event, location];
            Ok((location.x, location.y))
        }
    }

    fn get_screen_size() -> Result<(f64, f64), String> {
        unsafe {
            let screens: *mut Object = msg_send![class!(NSScreen), screens];
            let main_screen: *mut Object = msg_send![screens, objectAtIndex: 0];
            let frame: NSRect = msg_send![main_screen, frame];
            Ok((frame.size.width, frame.size.height))
        }
    }

    fn decide_position(
        cursor_x: f64,
        cursor_y: f64,
        screen_w: f64,
        screen_h: f64,
        popup_w: f64,
        popup_h: f64,
    ) -> (f64, f64) {
        let mut x = cursor_x - popup_w / 2.0;
        let mut y = cursor_y - popup_h - 10.0; // カーソルの上側

        // 画面端判定・調整
        if x < 0.0 {
            x = 10.0;
        }
        if x + popup_w > screen_w {
            x = screen_w - popup_w - 10.0;
        }
        if y < 0.0 {
            y = cursor_y + 10.0; // カーソルの下側に変更
        }
        if y + popup_h > screen_h {
            y = screen_h - popup_h - 10.0;
        }

        (x, y)
    }
}

impl Drop for PopupWindow {
    fn drop(&mut self) {
        let _ = self.close();
    }
}
```

### Module 5: キーボード入力（input/keyboard.rs）

```rust
// src/input/keyboard.rs

use objc2_core_graphics::{CGEvent, CGEventType, CGKeyCode, CGEventCreate, CGEventPost};
use log::debug;
use std::thread;
use std::time::Duration;

pub struct KeyboardInjector;

impl KeyboardInjector {
    /// テキストを入力（文字ごとにキーイベントを生成）
    pub fn type_text(text: &str) -> Result<(), String> {
        for ch in text.chars() {
            Self::type_char(ch)?;
            // キー入力間に短い遅延を挿入（macOS の入力バッファ対策）
            thread::sleep(Duration::from_millis(5));
        }
        debug!("Text injected: {}", text);
        Ok(())
    }

    fn type_char(ch: char) -> Result<(), String> {
        let keycode = Self::char_to_keycode(ch)?;
        let shift_needed = ch.is_uppercase();

        unsafe {
            // Shift キーを押す（必要な場合）
            if shift_needed {
                let shift_event = CGEventCreateKeyboardEvent(
                    std::ptr::null_mut(),
                    56, // Shift key code
                    true, // key down
                );
                if !shift_event.is_null() {
                    CGEventPost(1, shift_event); // kCGHIDEventTap = 1
                }
            }

            // 文字キーを押す
            let event = CGEventCreateKeyboardEvent(
                std::ptr::null_mut(),
                keycode,
                true, // key down
            );
            if !event.is_null() {
                CGEventPost(1, event);
            } else {
                return Err(format!("Failed to create event for '{}'", ch));
            }

            // 文字キーを離す
            let event = CGEventCreateKeyboardEvent(
                std::ptr::null_mut(),
                keycode,
                false, // key up
            );
            if !event.is_null() {
                CGEventPost(1, event);
            }

            // Shift キーを離す
            if shift_needed {
                let shift_event = CGEventCreateKeyboardEvent(
                    std::ptr::null_mut(),
                    56,
                    false, // key up
                );
                if !shift_event.is_null() {
                    CGEventPost(1, shift_event);
                }
            }
        }

        Ok(())
    }

    fn char_to_keycode(ch: char) -> Result<CGKeyCode, String> {
        let keycode = match ch {
            'a' | 'A' => 0x00,
            'b' | 'B' => 0x0B,
            'c' | 'C' => 0x08,
            'd' | 'D' => 0x02,
            'e' | 'E' => 0x0E,
            'f' | 'F' => 0x03,
            'g' | 'G' => 0x05,
            'h' | 'H' => 0x04,
            'i' | 'I' => 0x22,
            'j' | 'J' => 0x26,
            'k' | 'K' => 0x28,
            'l' | 'L' => 0x25,
            'm' | 'M' => 0x2E,
            'n' | 'N' => 0x2D,
            'o' | 'O' => 0x1F,
            'p' | 'P' => 0x23,
            'q' | 'Q' => 0x0C,
            'r' | 'R' => 0x0F,
            's' | 'S' => 0x01,
            't' | 'T' => 0x11,
            'u' | 'U' => 0x20,
            'v' | 'V' => 0x09,
            'w' | 'W' => 0x0D,
            'x' | 'X' => 0x07,
            'y' | 'Y' => 0x10,
            'z' | 'Z' => 0x06,
            '0' => 0x1D,
            '1' => 0x12,
            '2' => 0x13,
            '3' => 0x14,
            '4' => 0x15,
            '5' => 0x17,
            '6' => 0x16,
            '7' => 0x1A,
            '8' => 0x1C,
            '9' => 0x19,
            ' ' => 0x31,
            '.' => 0x2F,
            ',' => 0x2B,
            ';' => 0x29,
            ':' => 0x29, // Shift + ;
            '\n' => 0x24, // Return
            '\t' => 0x30, // Tab
            _ => return Err(format!("Unsupported character: '{}'", ch)),
        };

        Ok(keycode as CGKeyCode)
    }
}
```

### Module 6: LLM 管理（llm/manager.rs）

```rust
// src/llm/manager.rs

use std::sync::atomic::{AtomicUsize, Ordering};
use crate::config::LlmConfig;

pub struct LlmManager {
    llms: Vec<LlmConfig>,
    current_index: AtomicUsize,
}

impl LlmManager {
    pub fn new(llms: Vec<LlmConfig>) -> Self {
        LlmManager {
            llms,
            current_index: AtomicUsize::new(0),
        }
    }

    /// 有効なLLMの中から次を選択（ラウンドロビン）
    pub fn get_next_enabled(&self) -> Option<&LlmConfig> {
        let enabled: Vec<_> = self.llms
            .iter()
            .filter(|l| l.enabled)
            .collect();

        if enabled.is_empty() {
            return None;
        }

        let idx = self.current_index.fetch_add(1, Ordering::SeqCst);
        Some(enabled[idx % enabled.len()])
    }

    /// すべてのLLMを取得
    pub fn all(&self) -> &[LlmConfig] {
        &self.llms
    }

    /// LLMを更新
    pub fn update(&mut self, llms: Vec<LlmConfig>) {
        self.llms = llms;
        self.current_index.store(0, Ordering::SeqCst);
    }
}
```

#### llm/client.rs - API 呼び出し

```rust
// src/llm/client.rs

use reqwest::Client;
use serde_json::json;
use log::debug;
use crate::config::LlmConfig;
use std::fs;
use std::path::Path;

pub struct LlmClient {
    client: Client,
}

impl LlmClient {
    pub fn new() -> Self {
        LlmClient {
            client: Client::new(),
        }
    }

    /// LLMに プロンプト+テキストを送信して補正テキストを取得
    pub async fn prompt(
        &self,
        config: &LlmConfig,
        text: &str,
        prompt_file: &Path,
    ) -> Result<String, String> {
        // プロンプトファイルを読み込む
        let prompt = fs::read_to_string(prompt_file)
            .map_err(|e| format!("Failed to read prompt file: {}", e))?;

        let user_message = format!("{}\n\n{}", prompt, text);

        debug!("Sending to LLM: {} tokens", user_message.len() / 4);

        // OpenAI 互換 API を呼び出し
        let body = json!({
            "model": config.model,
            "messages": [
                {"role": "user", "content": user_message}
            ],
            "temperature": 0.7,
            "max_tokens": 2000,
        });

        let api_key = config.api_key.as_ref()
            .ok_or("API key not set for this LLM")?;

        let response = self.client
            .post(format!("{}/chat/completions", config.base_url))
            .bearer_auth(api_key)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("API request failed: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("API error: {}", response.status()));
        }

        let json_response: serde_json::Value = response.json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        let result = json_response
            .get("choices")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .ok_or("Invalid API response")?
            .to_string();

        debug!("LLM response: {}", result);
        Ok(result)
    }
}
```

### Module 7: フィラー除去（filler.rs）

```rust
// src/filler.rs

use regex::Regex;

pub fn remove_fillers(text: &str, filler_patterns: &[String]) -> String {
    let mut result = text.to_string();

    for pattern in filler_patterns {
        // 単純置換だけでなく、バウンダリ判定も加える
        // 例：「その」は文中では残す、冒頭や「その」だけの場合は削除
        let filler_regex = format!(r"(?:^|[\s　])?{}(?:[\s　]|$)?", regex::escape(pattern));
        if let Ok(re) = Regex::new(&filler_regex) {
            result = re.replace_all(&result, " ").to_string();
        }
    }

    // 連続スペースを単一スペースに統一
    result = result.split_whitespace().collect::<Vec<_>>().join(" ");

    // 先頭・末尾のスペースを削除
    result.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_fillers() {
        let fillers = vec!["えー".to_string(), "あのー".to_string()];
        let text = "これはえー素晴らしい、あのーツールです";
        let result = remove_fillers(text, &fillers);
        assert_eq!(result, "これは素晴らしい、ツールです");
    }
}
```

### Module 8: メインループ（main.rs）

```rust
// src/main.rs

mod config;
mod hotkey;
mod stt;
mod ui;
mod input;
mod llm;
mod filler;
mod types;

use clap::Parser;
use std::path::PathBuf;
use log::info;
use parking_lot::RwLock;
use std::sync::Arc;

use config::ConfigManager;
use hotkey::{HotkeyListener, HotkeyAction};
use ui::popup::PopupWindow;
use llm::manager::LlmManager;
use llm::client::LlmClient;

#[derive(Parser)]
#[command(name = "Voice Dictation Tool")]
#[command(about = "Rust voice-to-text tool for macOS Tahoe")]
struct Args {
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    info!("Voice Dictation Tool starting");

    let args = Args::parse();
    
    // デフォルト設定パス
    let config_path = args.config.unwrap_or_else(|| {
        let home = std::env::var("HOME").expect("HOME not set");
        PathBuf::from(format!("{}/.config/voicebot/settings.json", home))
    });

    // 設定を読み込む
    let config_mgr = ConfigManager::new(config_path)?;
    info!("Config loaded");

    // ホットキーリスナーを開始
    let hotkey_config = config_mgr.get(|cfg| cfg.hotkeys.clone());
    let prompt_bindings = config_mgr.get(|cfg| {
        cfg.prompts.iter().map(|p| p.bind.clone()).collect::<Vec<_>>()
    });

    let hotkey_listener = HotkeyListener::new(hotkey_config, prompt_bindings);
    let mut hotkey_rx = hotkey_listener.start();

    // STT とLLM を初期化
    let mut stt = stt::SpeechRecognizer::new("ja_JP")?;
    let llm_client = LlmClient::new();

    // アプリケーション状態
    #[derive(Clone, Copy, Debug, PartialEq)]
    enum AppState {
        Idle,
        Recording,
    }

    let state = Arc::new(RwLock::new(AppState::Idle));
    let mut stt_rx: Option<tokio::sync::mpsc::Receiver<stt::SttEvent>> = None;
    let mut popup: Option<PopupWindow> = None;

    info!("Ready to receive hotkey events");

    loop {
        tokio::select! {
            // ホットキーイベント処理
            Some(action) = hotkey_rx.recv() => {
                match action {
                    HotkeyAction::Start => {
                        if *state.read() == AppState::Idle {
                            match stt.start().await {
                                Ok(rx) => {
                                    stt_rx = Some(rx);
                                    popup = PopupWindow::create_near_cursor().ok();
                                    *state.write() = AppState::Recording;
                                    info!("Recording started");
                                }
                                Err(e) => {
                                    eprintln!("Failed to start STT: {}", e);
                                }
                            }
                        }
                    }

                    HotkeyAction::Commit => {
                        if *state.read() == AppState::Recording {
                            if let Some(p) = &popup {
                                if let Ok(text) = p.get_text() {
                                    let _ = input::keyboard::KeyboardInjector::type_text(&text).await;
                                    info!("Text committed: {}", text);
                                }
                            }
                            let _ = stt.stop();
                            if let Some(mut p) = popup.take() {
                                let _ = p.close();
                            }
                            stt_rx = None;
                            *state.write() = AppState::Idle;
                        }
                    }

                    HotkeyAction::PromptCustom(prompt_idx) => {
                        if *state.read() == AppState::Recording {
                            if let Some(p) = &popup {
                                if let Ok(text) = p.get_text() {
                                    let prompts = config_mgr.get(|cfg| cfg.prompts.clone());
                                    if let Some(prompt) = prompts.get(prompt_idx) {
                                        let llm_mgr = config_mgr.get(|cfg| {
                                            LlmManager::new(cfg.llms.clone())
                                        });
                                        if let Some(llm_config) = llm_mgr.get_next_enabled() {
                                            match llm_client.prompt(
                                                llm_config,
                                                &text,
                                                &prompt.file,
                                            ).await {
                                                Ok(corrected) => {
                                                    let _ = input::keyboard::KeyboardInjector::type_text(&corrected).await;
                                                    info!("Text corrected and committed");
                                                }
                                                Err(e) => {
                                                    eprintln!("LLM error: {}", e);
                                                }
                                            }
                                        }
                                    }
                                    let _ = stt.stop();
                                    if let Some(mut p) = popup.take() {
                                        let _ = p.close();
                                    }
                                    stt_rx = None;
                                    *state.write() = AppState::Idle;
                                }
                            }
                        }
                    }

                    HotkeyAction::Settings => {
                        info!("Settings dialog requested");
                        // 設定ダイアログを開く（実装は省略）
                    }
                }
            }

            // STT イベント処理
            Some(stt_event) = async {
                if let Some(rx) = &mut stt_rx {
                    rx.recv().await
                } else {
                    None
                }
            } => {
                if *state.read() == AppState::Recording {
                    match stt_event {
                        stt::SttEvent::PartialResult(text) => {
                            let fillers = config_mgr.get(|cfg| cfg.filler_words.clone());
                            let cleaned = filler::remove_fillers(&text, &fillers);
                            if let Some(p) = &popup {
                                let _ = p.append_text(&cleaned);
                            }
                            info!("Partial: {}", cleaned);
                        }
                        stt::SttEvent::FinalResult(text) => {
                            let fillers = config_mgr.get(|cfg| cfg.filler_words.clone());
                            let cleaned = filler::remove_fillers(&text, &fillers);
                            if let Some(p) = &popup {
                                let _ = p.append_text(&cleaned);
                            }
                            info!("Final: {}", cleaned);
                        }
                        stt::SttEvent::Error(e) => {
                            eprintln!("STT error: {}", e);
                            let _ = stt.stop();
                            if let Some(mut p) = popup.take() {
                                let _ = p.close();
                            }
                            stt_rx = None;
                            *state.write() = AppState::Idle;
                        }
                    }
                }
            }
        }
    }
}
```

---

## トラブルシューティング

### 問題 1: STT がデバイスを見つけられない

**症状：** `No input device found`

**原因と対策：**
```rust
// src/stt/audio_engine.rs を以下のように修正
pub fn new() -> Result<Self, String> {
    let host = cpal::default_host();
    
    // 利用可能なデバイスをすべてリスト
    let devices: Vec<_> = host.input_devices()
        .map_err(|e| format!("Failed to enumerate devices: {}", e))?
        .map(|d| d.name().unwrap_or_default())
        .collect();
    
    eprintln!("Available devices: {:?}", devices);
    
    let device = host
        .default_input_device()
        .ok_or("No input device found")?;
    
    // ...
}
```

実行して `Available devices` を確認

### 問題 2: キーボード入力が動作しない

**症状：** `type_text` が呼ばれても何も起こらない

**原因と対策：**
1. **アクセシビリティ権限の確認**
   ```bash
   # ターミナルが アクセシビリティ フルディスク・アクセス を持っているか確認
   # System Settings > Privacy & Security > Accessibility
   ```

2. **CGEvent API の修正版**
   ```rust
   // src/input/keyboard.rs を以下に修正（darwin-specific）
   use core_foundation::base::TCFType;
   use core_foundation::dictionary::CFDictionary;
   
   unsafe {
       let event = CGEventCreateKeyboardEvent(
           std::ptr::null_mut(),
           keycode,
           true,
       );
       if event.is_null() {
           return Err("Failed to create keyboard event".to_string());
       }
       // イベントを実際に送信
       CGEventPost(1, event);
       CFRelease(event as *const _);
   }
   ```

### 問題 3: Objective-C オブジェクトが NULL を返す

**症状：** `Failed to create SpeechRecognizer`

**原因と対策：**
```rust
// class!() マクロが正しく使われているか確認
// objc2 のバージョンを確認
cargo tree | grep objc2

// クラス名が正しいか確認（大文字小文字）
// SpeechRecognizer (○) vs speechrecognizer (×)
```

### 問題 4: Tahoe STT が見つからない

**症状：** `SpeechRecognizer class not found`

**原因と対策：**
これは Tahoe（macOS 26）の新API の場合に発生します。その場合、従来の SpeechRecognizer に フォールバック：

```rust
// src/stt/speech_recognizer.rs を修正
pub fn new(locale: &str) -> Result<Self, String> {
    unsafe {
        // まず Tahoe 新API を試す
        if let Some(cls) = Self::get_speech_analyzer_class() {
            // SpeechAnalyzer で初期化
            return Self::init_with_analyzer(cls, locale);
        }
        
        // フォールバック：従来のSpeechRecognizer
        let cls = class!(SpeechRecognizer);
        if cls.is_null() {
            return Err("Neither SpeechAnalyzer nor SpeechRecognizer available".to_string());
        }
        
        // 従来の方法で初期化
        Self::init_with_recognizer(cls, locale)
    }
}

fn get_speech_analyzer_class() -> Option<*mut Object> {
    unsafe {
        let cls: *mut Object = objc2::runtime::objc_getClass("SpeechAnalyzer") as *mut _;
        if cls.is_null() {
            None
        } else {
            Some(cls)
        }
    }
}
```

---

## まとめ：実装フロー

1. **Phase 1：基本構造**
   - `main.rs`：イベントループの骨組み
   - `config.rs`：JSON読み込み
   - `hotkey.rs`：ホットキー監視
   - `types.rs`：共通型

2. **Phase 2：コア機能**
   - `stt/speech_recognizer.rs`：STT（段階的にテスト）
   - `input/keyboard.rs`：キーボード入力（権限設定が重要）
   - `ui/popup.rs`：UI表示

3. **Phase 3：応用機能**
   - `llm/client.rs`：LLM 補正
   - `filler.rs`：フィラー除去
   - `ui/settings_dialog.rs`：設定管理

4. **Phase 4：統合テスト**
   - 全機能の組み合わせ
   - エラーハンドリング
   - パフォーマンス最適化

各フェーズでビルド・テストを繰り返し、問題を早期に発見することが重要です。
