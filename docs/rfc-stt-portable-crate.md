# RFC: ポータブル音声入力完全 crate `voice-kit`

**Status**: Draft
**Author**: Toshimi Kawata (based on MYCUTE STT implementation)
**Date**: 2026-06-11
**Version**: 1.0.0-draft

---

## 目次

1. [背景と目的](#1-背景と目的)
2. [用語定義](#2-用語定義)
3. [crate の理念と範囲](#3-crate-の理念と範囲)
4. [公開 API 設計](#4-公開-api-設計)
5. [内部アーキテクチャ](#5-内部アーキテクチャ)
6. [ネイティブライブラリのプリビルドと同封](#6-ネイティブライブラリのプリビルドと同封)
7. [モジュール構成と実装ガイド](#7-モジュール構成と実装ガイド)
8. [Cargo.toml 依存関係](#8-cargotoml-依存関係)
9. [build.rs 設計](#9-buildrs-設計)
10. [テスト戦略](#10-テスト戦略)
11. [移行ガイド（MYCUTE 側）](#11-移行ガイドmycute-側)
12. [ライセンスと配布](#12-ライセンスと配布)

---

## 1. 背景と目的

### 1.1 現状

MYCUTE プロジェクトには、以下の3バックエンドを統合した完全な音声認識（STT）システムが実装されている：

| バックエンド | 基盤技術 | 特徴 |
|---|---|---|
| **OpenAI** | GPT-4o-mini-transcribe (Whisper API) | VAD + バッファリング + 疑似ストリーミング |
| **macOS ネイティブ** | SFSpeechRecognizer / DictationTranscriber (Tahoe) | Swift FFI、オンデバイス認識 |
| **Windows ネイティブ** | Windows.Media.SpeechRecognition | C# Native AOT 静的リンク、IME制御 |

さらに、以下のパイプラインコンポーネントが組み込まれている：

- **VAD (Voice Activity Detection)**: Sherpa-ONNX (Silero / TEN)
- **GTCRN ノイズ除去**: Sherpa-ONNX
- **リサンプリング**: rubato Sinc 補間
- **句読点挿入**: Lindera 形態素解析（日本語特化）
- **LLM 事後補正**: LLM API によるテキスト補正
- **置換辞書**: 最長一致プレースホルダー置換

### 1.2 問題

現在の実装は `src/stt/` 以下に MYCUTE アプリケーションと密結合しており、以下に依存している：

- `MycuteSettings` / `SttSettings` の内部構造
- `MycuteManager` の状態管理
- `LmgwClient` の MYCUTE 固有認証機構
- `ConfigManager` の設定読み込み
- `constants.rs` の MYCUTE 全体定数
- `build.rs` の MYCUTE 固有ビルドパイプライン

### 1.3 目標

**`voice-kit` crate として完全に独立させる。** 任意の Rust プロジェクトが `cargo add voice-kit` するだけで、以下のすべてを利用可能にする：

```
音声入力開始 → VAD検出 → ノイズ除去 → リサンプリング
→ 音声認識（OpenAI / macOS / Windows）
→ 句読点挿入 → 事後補正 → テキスト出力
→ 権限リクエスト → ヘルスチェック
```

この crate の実装者は **実装のタイミングで判断に迷うことが一切ない** ように、この RFC には具体的なコードスニペット、ファイル構成、依存関係、ビルド手順がすべて含まれている。

---

## 2. 用語定義

| 用語 | 定義 |
|---|---|
| **ASR** | Automatic Speech Recognition。音声データからテキストへの変換 |
| **VAD** | Voice Activity Detection。発話区間の検出 |
| **GTCRN** | Grouped Temporal Convolutional Recurrent Network。Sherpa-ONNX のノイズ除去モデル |
| **Pseudo-Streaming ASR** | オフライン推論 API (OpenAI Whisper) を VAD で区切り、ストリーミング風に見せる方式 |
| **Watermark** | OS ネイティブ認識において、確定済みテキスト位置を追跡し差分のみを抽出する機構 |
| **Post-Correction** | LLM を用いた認識テキストの最終補正（句読点修正、表記揺れ補正等） |
| **SttModelType** | エンジンが送るテキストのセマンティクスを区分する列挙型（Offline=増分追記 / Online=上書き） |
| **Interceptor** | 全バックエンド共通でテキストに置換辞書を適用する中継層 |
| **Prebuilt Native Library** | Swift / C# コードを事前コンパイルした `.a` / `.lib` ファイル。利用側に Swift/Xcode/.NET SDK 不要にする |

---

## 3. crate の理念と範囲

### 3.1 理念

**「音声入力が必要なら、これを入れるだけ」**

利用者が書くべきコードは、設定を渡してイベントを受け取るだけ。内部のすべて（ネイティブコード呼び出し、モデル管理、OS差分、VADパイプライン）は完全に隠蔽される。

### 3.2 crate が提供するもの（隠蔽対象）

| 資産 | 隠蔽形態 |
|---|---|
| macOS Swift (`SpeechHelper.swift`) | プリビルド `libspeech_helper.a` を同封、`build.rs` で自動リンク |
| Windows C# (`SpeechHelper.cs`) | プリビルド `speech_helper.lib` + `SpeechHelper.dll` を同封 |
| Sherpa-ONNX (VAD + Denoiser) | `sherpa-rs` / `sherpa-rs-sys` 依存を内部化 |
| async-openai (Whisper API) | クレート内依存 |
| rubato (リサンプリング) | 同上 |
| lindera (句読点挿入) | 同上 |
| rodio (効果音再生) | 同上 |
| 権限リクエスト (macOS/Windows) | Swift/C# FFI が内部で実行 |
| ヘルスチェック (Windows) | C# FFI 経由でモデル/プライバシー/マイク状態をビットマスク返却 |
| IME 制御 (Windows) | 音声入力開始/終了時のIME ON/OFF 自動制御 |

### 3.3 crate が提供しないもの（利用者の責務）

| 資産 | 理由 |
|---|---|
| Sherpa-ONNX モデルファイル (`.onnx`) | サイズ（合計約 2MB〜10MB）とライセンスの観点からパス指定を受け付ける |
| 効果音 WAV ファイル (`piro.wav`, `commit.wav`) | 埋め込み済み（`include_bytes!`） |
| アプリケーションの Info.plist / AppxManifest | OS のビルド時制約。README に必要記述を明示 |

---

## 4. 公開 API 設計

### 4.1 核心理念

利用者が触る型は **5つだけ**：

1. `VoiceKit` — 唯一のエントリポイント
2. `VoiceKitConfig` — 設定（ビルダーパターン）
3. `SttEvent` — 認識イベント
4. `SttEngine` — エンジン選択
5. `LocaleCode` — 言語ロケール

### 4.2 VoiceKit — エントリポイント

利用者が実際に書くコードは以下のみ：

```rust
use voice_kit::{VoiceKit, VoiceKitConfig, SttEngine, LocaleCode, SttEvent};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 設定を構築
    let config = VoiceKitConfig::builder()
        .engine(SttEngine::Os)                      // または SttEngine::OpenAi
        .locale(LocaleCode::Ja)
        .vad_model_paths(VadModelPaths {
            silero: "/path/to/silero_vad.onnx".into(),
            ten: "/path/to/ten_vad.onnx".into(),
            gtcrn: "/path/to/gtcrn.onnx".into(),
        })
        // OpenAI を使う場合のみ必要
        .openai_config(OpenAiConfig {
            base_url: "http://127.0.0.1:3912".into(),  // OpenAI API のベースURL
            api_key: "sk-...".into(),
            model: "openai/gpt-4o-mini-transcribe".into(),
        })
        .build()?;

    // 2. 認識器を作成
    let mut vk = VoiceKit::new(config)?;

    // 3. 権限リクエスト（OSネイティブ使用時）
    if let SttEngine::Os = vk.engine() {
        if !vk.request_permissions().await? {
            eprintln!("マイク権限がありません");
            return Ok(());
        }
    }

    // 4. 開始
    vk.start().await?;

    // 5. イベントループ
    while let Some(event) = vk.next_event().await {
        match event {
            SttEvent::Ready => println!("[準備完了] 録音開始"),
            SttEvent::PartialResult(text, _seq) => println!("[途中] {}", text),
            SttEvent::FinalResult(text, _seq) => println!("[確定] {}", text),
            SttEvent::Error(msg) => eprintln!("[エラー] {}", msg),
            SttEvent::Stopped => { println!("[停止]"); break; }
            _ => {}
        }
    }

    Ok(())
}
```

### 4.3 VoiceKitConfig — 設定ビルダー

```rust
/// 音声認識の全設定
#[derive(Debug, Clone)]
pub struct VoiceKitConfig {
    /// 使用するエンジン
    pub engine: SttEngine,
    /// 言語ロケール
    pub locale: LocaleCode,

    // ---- OpenAI 設定（engine == OpenAI の場合のみ必要） ----
    pub openai_config: Option<OpenAiConfig>,

    // ---- VAD 設定 ----
    pub vad: VadConfig,

    // ---- 補正設定 ----
    pub post_correction: PostCorrectionConfig,

    // ---- 句読点設定 ----
    pub punctuation: bool,  // デフォルト true

    // ---- ノイズ除去設定 ----
    pub denoiser: DenoiserConfig,

    // ---- 信号品質フィルタ ----
    pub signal_filter: SignalFilterConfig,

    // ---- マイク設定 ----
    /// 発話タイムアウト（秒）デフォルト: 30.0
    pub speech_timeout_sec: f64,

    /// VAD モデルファイルパス群
    pub vad_model_paths: VadModelPaths,
}

impl VoiceKitConfig {
    pub fn builder() -> VoiceKitConfigBuilder { VoiceKitConfigBuilder::default() }
}

#[derive(Debug, Clone, Default)]
pub struct VoiceKitConfigBuilder {
    engine: Option<SttEngine>,
    locale: Option<LocaleCode>,
    openai_config: Option<OpenAiConfig>,
    vad: Option<VadConfig>,
    post_correction: Option<PostCorrectionConfig>,
    punctuation: Option<bool>,
    denoiser: Option<DenoiserConfig>,
    signal_filter: Option<SignalFilterConfig>,
    speech_timeout_sec: Option<f64>,
    vad_model_paths: Option<VadModelPaths>,
}

impl VoiceKitConfigBuilder {
    pub fn engine(mut self, e: SttEngine) -> Self { self.engine = Some(e); self }
    pub fn locale(mut self, l: LocaleCode) -> Self { self.locale = Some(l); self }
    pub fn openai_config(mut self, c: OpenAiConfig) -> Self { self.openai_config = Some(c); self }
    pub fn vad(mut self, v: VadConfig) -> Self { self.vad = Some(v); self }
    pub fn post_correction(mut self, p: PostCorrectionConfig) -> Self { self.post_correction = Some(p); self }
    pub fn punctuation(mut self, p: bool) -> Self { self.punctuation = Some(p); self }
    pub fn denoiser(mut self, d: DenoiserConfig) -> Self { self.denoiser = Some(d); self }
    pub fn signal_filter(mut self, s: SignalFilterConfig) -> Self { self.signal_filter = Some(s); self }
    pub fn speech_timeout_sec(mut self, t: f64) -> Self { self.speech_timeout_sec = Some(t); self }
    pub fn vad_model_paths(mut self, p: VadModelPaths) -> Self { self.vad_model_paths = Some(p); self }

    pub fn build(self) -> Result<VoiceKitConfig, VoiceKitError> {
        // バリデーション:
        // - engine == OpenAI の場合 openai_config が必須
        // - vad_model_paths が必須
        // - locale が必須
        let engine = self.engine.unwrap_or_default();
        let locale = self.locale.ok_or_else(|| {
            VoiceKitError::InvalidConfig("locale is required".into())
        })?;
        let vad_model_paths = self.vad_model_paths.ok_or_else(|| {
            VoiceKitError::InvalidConfig("vad_model_paths is required".into())
        })?;

        if engine == SttEngine::OpenAi && self.openai_config.is_none() {
            return Err(VoiceKitError::InvalidConfig(
                "openai_config is required when engine is OpenAI".into()
            ));
        }

        Ok(VoiceKitConfig {
            engine,
            locale,
            openai_config: self.openai_config,
            vad: self.vad.unwrap_or_default(),
            post_correction: self.post_correction.unwrap_or_default(),
            punctuation: self.punctuation.unwrap_or(true),
            denoiser: self.denoiser.unwrap_or_default(),
            signal_filter: self.signal_filter.unwrap_or_default(),
            speech_timeout_sec: self.speech_timeout_sec.unwrap_or(30.0),
            vad_model_paths,
        })
    }
}
```

### 4.4 公開型定義

```rust
// ============================================================================
// この節の全型定義は src/types.rs に記述する
// ============================================================================

/// 音声認識エンジン
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SttEngine {
    /// OpenAI Whisper API（疑似ストリーミング）
    OpenAI,
    /// OS ネイティブ認識（macOS: SFSpeechRecognizer / Windows: WinRT）
    #[default]
    Os,
}

/// 言語ロケール
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LocaleCode {
    #[default]
    Ja,
    En,
}

impl LocaleCode {
    /// 短縮コード（"ja", "en"）
    pub fn as_str(&self) -> &'static str {
        match self { LocaleCode::Ja => "ja", LocaleCode::En => "en" }
    }

    /// macOS/Windows ネイティブ API 用の BCP-47 タグ
    pub fn as_bcp47(&self) -> &'static str {
        match self { LocaleCode::Ja => "ja-JP", LocaleCode::En => "en-US" }
    }

    /// OpenAI API 用の ISO-639-1 コード
    pub fn as_iso639_1(&self) -> &'static str {
        match self { LocaleCode::Ja => "ja", LocaleCode::En => "en" }
    }
}

/// 認識イベント
#[derive(Debug, Clone)]
pub enum SttEvent {
    /// 部分認識結果（表示用、上書きされる可能性あり）
    PartialResult(String, u64),
    /// 確定認識結果
    FinalResult(String, u64),
    /// 認識開始
    Started,
    /// エラー
    Error(String),
    /// 認識停止
    Stopped,
    /// 録音準備完了（マイク/ハードウェア開放完了）
    Ready,
    /// LLM 事後補正 開始（フロントエンドの入力ロック用）
    PostCorrectionStarted,
    /// LLM 事後補正 完了
    PostCorrectionFinished,
    /// ASR API 呼び出し中（装飾表示中）
    SttPending,
    /// ASR API 呼び出し完了
    SttCompleted,
    /// 装飾表示の強制クリア（異常検知時）
    ForceClearDecoration,
    /// 装飾フレーム（表示用アニメーション。"…" や "?" など）
    DecorationPartial(String),
}

/// OpenAI 接続設定
#[derive(Debug, Clone)]
pub struct OpenAiConfig {
    /// OpenAI API 互換のベースURL
    pub base_url: String,
    /// API キー
    pub api_key: String,
    /// 使用モデル名（例: "openai/gpt-4o-mini-transcribe"）
    pub model: String,
}

/// VAD モデルファイルパス群
#[derive(Debug, Clone)]
pub struct VadModelPaths {
    /// Silero VAD モデルのパス（必須）
    pub silero: String,
    /// TEN VAD モデルのパス（オプションだが空文字不可。silero と同じでも可）
    pub ten: String,
    /// GTCRN ノイズ除去モデルのパス（空文字列で無効化）
    pub gtcrn: String,
}

/// VAD 設定
#[derive(Debug, Clone)]
pub struct VadConfig {
    /// VAD アルゴリズム（Silero / Ten）
    pub vad_type: VadType,
    /// 発話検知閾値 (0.0〜1.0, デフォルト 0.5)
    pub threshold: f32,
    /// 発話終了とみなす無音時間（秒, デフォルト 0.2）
    pub min_silence_duration: f32,
    /// 発話開始とみなす最小音声時間（秒, デフォルト 0.25）
    pub min_speech_duration: f32,
    /// 最大発話時間（秒, デフォルト 25.0）
    pub max_speech_duration: f32,
    /// 発話開始前に遡って保持する時間（ミリ秒, デフォルト 100）
    pub pre_padding_ms: u64,
    /// 認識対象とする最小発話長（ミリ秒, デフォルト 300）
    pub utterance_min_ms: u64,
    /// Sherpa-ONNX のスレッド数（デフォルト 4）
    pub num_threads: i32,
}

impl Default for VadConfig {
    fn default() -> Self {
        Self {
            vad_type: VadType::default(),
            threshold: 0.5,
            min_silence_duration: 0.2,
            min_speech_duration: 0.25,
            max_speech_duration: 25.0,
            pre_padding_ms: 100,
            utterance_min_ms: 300,
            num_threads: 4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VadType {
    #[default]
    Silero,
    Ten,
}

/// 事後補正設定
#[derive(Debug, Clone)]
pub struct PostCorrectionConfig {
    /// 補正を起動する文数閾値（デフォルト 3）
    pub sentence_count_threshold: usize,
    /// 補正を起動する最小文字数（デフォルト 10）
    pub min_text_length: usize,
    /// 補正実行の最小間隔（ミリ秒, デフォルト 2000）
    pub interval_ms: u64,
}

impl Default for PostCorrectionConfig {
    fn default() -> Self {
        Self {
            sentence_count_threshold: 3,
            min_text_length: 10,
            interval_ms: 2000,
        }
    }
}

/// ノイズ除去設定
#[derive(Debug, Clone)]
pub struct DenoiserConfig {
    /// ノイズ除去を有効にするか
    pub enabled: bool,
}

impl Default for DenoiserConfig {
    fn default() -> Self { Self { enabled: true } }
}

/// 信号品質フィルタ設定
#[derive(Debug, Clone)]
pub struct SignalFilterConfig {
    /// 信号品質チェックを有効にするか
    pub enabled: bool,
    /// RMS 閾値 (0.0〜1.0, デフォルト 0.005)
    pub rms_threshold: f32,
    /// 有意音声占有率閾値 (0.0〜1.0, デフォルト 0.15)
    pub occupancy_ratio: f32,
}

impl Default for SignalFilterConfig {
    fn default() -> Self {
        Self { enabled: true, rms_threshold: 0.005, occupancy_ratio: 0.15 }
    }
}

/// エラー型
#[derive(Debug, thiserror::Error)]
pub enum VoiceKitError {
    #[error("設定が不正です: {0}")]
    InvalidConfig(String),

    #[error("エンジン {engine:?} は現在のプラットフォームで利用できません: {reason}")]
    UnsupportedEngine { engine: SttEngine, reason: String },

    #[error("権限がありません: {0}")]
    PermissionDenied(String),

    #[error("初期化エラー: {0}")]
    InitError(String),

    #[error("実行時エラー: {0}")]
    RuntimeError(String),

    #[error(transparent)]
    Io(#[from] std::io::Error),
}
```

**MYCUTE からの型抽出対応表**:

| MYCUTE の型 | voice-kit での型 | 変更点 |
|---|---|---|
| `src/types.rs` の `SttEvent` | 同一（`src/types.rs`） | 変更なし |
| `src/types.rs` の `LocaleCode` | 同一（`src/types.rs`） | `sherpa01_language_token()` メソッド削除（MYCUTE Sherpa01依存） |
| `src/types.rs` の `TargetPlatform` | 削除 | 利用側に不要 |
| `src/types.rs` の `HotkeyAction` | 削除 | MYCUTE ホットキー機能のため |
| `src/mycute_settings.rs` の `SttEngine` | 同一（`src/types.rs`） | variant 名: `OpenAI` → `OpenAi` |
| `src/mycute_settings.rs` の `VadType` | 同一（`src/types.rs`） | 変更なし |
| `src/mycute_settings.rs` の `SttSettings` | 複数 Config に分解 | 1対1対応ではない。後述のマッピング表参照 |

**`SttSettings` → 各 Config へのマッピング**:

| MYCUTE (`SttSettings` フィールド) | voice-kit |
|---|---|
| `model_dir: Option<String>` | → `VadModelPaths` (絶対パス化) |
| `num_threads: i32` | → `VadConfig::num_threads` |
| `vad_type: VadType` | → `VadConfig::vad_type` |
| `vad_model_path: Option<String>` | → `VadModelPaths::{silero, ten}` |
| `vad_threshold: f32` | → `VadConfig::threshold` |
| `vad_min_silence_duration` | → `VadConfig::min_silence_duration` |
| `vad_min_speech_duration` | → `VadConfig::min_speech_duration` |
| `vad_max_speech_duration` | → `VadConfig::max_speech_duration` |
| `vad_pre_padding_ms` | → `VadConfig::pre_padding_ms` |
| `utterance_min_ms` | → `VadConfig::utterance_min_ms` |
| `use_denoiser` + `denoiser_model_path` | → `DenoiserConfig` + `VadModelPaths::gtcrn` |
| `signal_check_enabled/rms_threshold/occupancy_ratio` | → `SignalFilterConfig` |
| `post_correction_sentence_count_threshold` | → `PostCorrectionConfig::sentence_count_threshold` |
| `post_correction_min_text_length` | → `PostCorrectionConfig::min_text_length` |
| `post_correction_interval_ms` | → `PostCorrectionConfig::interval_ms` |
| （存在しない） | → `OpenAiConfig` (新規) |

### 4.5 VoiceKit 本体

```rust
// src/voice_kit.rs

use std::sync::Arc;
use tokio::sync::mpsc;

use crate::backends::openai::OpenAIRecognizer;
use crate::backends::mac::MacSpeechBackend;
use crate::backends::win::WinSpeechBackend;
use crate::config::VoiceKitConfig;
use crate::error::VoiceKitError;
use crate::recognizer::SpeechRecognizer;
use crate::types::{SttEngine, SttEvent, LocaleCode};

pub struct VoiceKit {
    recognizer: SpeechRecognizer,
    config: VoiceKitConfig,
    event_rx: mpsc::Receiver<SttEvent>,
    event_tx: mpsc::Sender<SttEvent>,
}

impl VoiceKit {
    /// 新しい VoiceKit インスタンスを作成する。
    ///
    /// この時点で以下の初期化が行われる：
    /// - macOS: Swift SpeechHelper ライブラリの初期化と Tahoe 検出
    /// - Windows: C# SpeechHelper の初期化とヘルスチェック
    /// - OpenAI: OpenAiClient の初期化と音声キャプチャの準備
    /// - 全バックエンドの VAD プロセッサ初期化
    /// - インターセプタータスク（置換辞書）の起動
    pub fn new(config: VoiceKitConfig) -> Result<Self, VoiceKitError> {
        let (event_tx, event_rx) = mpsc::channel(256);

        // 置換辞書の初期化（空）
        let replaces_map = Arc::new(parking_lot::RwLock::new(
            indexmap::IndexMap::new()
        ));

        let recognizer = SpeechRecognizer::new(
            event_tx.clone(),
            &config,
            replaces_map,
        )?;

        Ok(Self {
            recognizer,
            config,
            event_rx,
            event_tx,
        })
    }

    pub fn engine(&self) -> SttEngine { self.config.engine }

    /// マイクおよび音声認識の権限をリクエストする。
    ///
    /// macOS: SFSpeechRecognizer.requestAuthorization() でシステムダイアログを表示。
    /// Windows: AudioGraph 生成でマイク権限チェック。未許可時は guidance をログ出力。
    ///
    /// 戻り値: true = 権限あり, false = 権限なし
    pub async fn request_permissions(&self) -> Result<bool, VoiceKitError> {
        #[cfg(target_os = "macos")]
        {
            let result = unsafe {
                crate::native::mac_ffi::speech_helper_request_authorization()
            };
            return Ok(result == 0);
        }

        #[cfg(target_os = "windows")]
        {
            let health = self.recognizer.health_check();
            if (health & 4) != 0 {
                log::warn!("[voice-kit] Microphone permission is not granted. \
                    Please enable it in Windows Settings > Privacy > Microphone.");
                return Ok(false);
            }
            return Ok(true);
        }

        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        { Ok(false) }
    }

    pub async fn start(&mut self) -> Result<(), VoiceKitError> {
        self.recognizer.start();
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<(), VoiceKitError> {
        self.recognizer.stop();
        Ok(())
    }

    pub async fn next_event(&mut self) -> Option<SttEvent> {
        self.event_rx.recv().await
    }

    /// 現在のバッファ内容を強制フラッシュする。
    /// 内部で stop → テキスト収集 → start を実行する。
    pub async fn flush(&mut self) -> Result<String, VoiceKitError> {
        // MYCUTE の MycuteManager::request_flush のロジックを移植
        self.recognizer.stop();

        // 最終テキストを集約
        let mut final_text = String::new();
        // stop() 後に残っているイベントを読み切る
        while let Ok(event) = self.event_rx.try_recv() {
            match event {
                SttEvent::FinalResult(text, _) | SttEvent::PartialResult(text, _) => {
                    final_text = text;
                }
                _ => {}
            }
        }

        self.recognizer.start();
        Ok(final_text)
    }

    pub async fn set_engine(&mut self, engine: SttEngine) -> Result<(), VoiceKitError> {
        let was_running = self.recognizer.is_running();
        if was_running { self.recognizer.stop(); }
        self.recognizer.set_engine(engine);
        self.config.engine = engine;
        if was_running { self.recognizer.start(); }
        Ok(())
    }

    pub fn set_locale(&mut self, locale: LocaleCode) {
        self.recognizer.set_locale(locale);
        self.config.locale = locale;
    }

    /// 置換辞書を更新する。
    /// IndexMap<String, Vec<String>>: キー = 置換後テキスト, 値 = 置換前テキスト候補リスト
    pub fn update_replaces(&mut self, replaces: indexmap::IndexMap<String, Vec<String>>) {
        self.recognizer.update_replaces(replaces);
    }

    /// Windows: 音声入力設定のヘルスチェック結果をビットマスクで取得
    /// macOS: 常に 0
    ///
    /// bit 0: 音声認識モデル未インストール
    /// bit 1: 音声認識プライバシー OFF
    /// bit 2: マイク権限なし
    pub fn health_check(&self) -> u32 {
        self.recognizer.health_check()
    }
}
```

---

## 5. 内部アーキテクチャ

### 5.1 データフロー図

```
マイク入力（OS API）
    │
    ▼
[ネイティブオーディオキャプチャ]  ← macOS: AVAudioEngine / Windows: AudioGraph
    │  f32 PCM samples
    ▼
┌─────────────────────────────────────────────────────────┐
│                   SpeechRecognizer                       │
│                                                          │
│  ┌───────────────────────────────────────────────────┐  │
│  │             エンジン分岐 (SttEngine)                │  │
│  │                                                     │  │
│  │  OpenAI  ───→ PseudoAsrStreamer                     │  │
│  │    │           ├─ SincResampler (任意→16kHz)         │  │
│  │    │           ├─ GTCRN Denoiser (任意)              │  │
│  │    │           ├─ SignalFilter (品質チェック)         │  │
│  │    │           ├─ VadProcessor (発話区間検出)         │  │
│  │    │           ├─ OpenAI Whisper API (認識)          │  │
│  │    │           ├─ PunctuationMachine (句読点)         │  │
│  │    │           └─ PostCorrectionProcessor (補正)      │  │
│  │    │                                                     │  │
│  │    │  Os ──→ MacSpeechBackend / WinSpeechBackend        │  │
│  │    │         ├─ VadProcessor (発話区間検出)              │  │
│  │    │         ├─ SincResampler (→16kHz)                  │  │
│  │    │         ├─ Watermark 同期                           │  │
│  │    │         ├─ PunctuationMachine (句読点, Win専用)     │  │
│  │    │         └─ PostCorrectionProcessor (補正)           │  │
│  │    │                                                     │  │
│  │    └──→ Interceptor (置換辞書適用)                        │  │
│  │           └──→ tx (イベント送信)                          │  │
│  └───────────────────────────────────────────────────┘  │
│                                                          │
└─────────────────────────────────────────────────────────┘
    │  SttEvent
    ▼
利用者側イベントループ (VoiceKit::next_event())
```

### 5.2 非同期タスク構造

```
VoiceKit::start()
    │
    ├── [macOS/Windows] ネイティブキャプチャ開始
    │     └── tokio::spawn(capture_task)
    │           │ マイクデータを audio_buf (Arc<Mutex<Vec<f32>>>) に流し込む
    │
    ├── [macOS/Windows] ネイティブ認識セッション開始
    │     └── OS の認識エンジンが別スレッドで動作
    │           認識結果は OS → C FFI callback → Rust の global channel (lazy_static) に送信
    │
    ├── [全エンジン] Background Ticker (tokio::spawn)
    │     ├── OpenAI: 20ms 周期
    │     │   ├── audio_buf からデータを取り出し streamer.push_samples()
    │     │   └── streamer.tick() → VAD → 認識 → 補正
    │     └── OS: 50ms 周期
    │         ├── 内部イベントチャネルからイベントを収集
    │         ├── Coalescing (最新STTイベントのみ保持)
    │         ├── Watermark 同期で差分抽出
    │         ├── 句読点挿入 (Windows)
    │         ├── PostCorrectionProcessor に投入
    │         └── tx_app 経由でイベント送信
    │
    └── [全エンジン] Interceptor Task (std::thread::spawn)
          │ 各バックエンドからのイベントを tx_internal から受信
          │ FinalResult/PartialResult のテキストに置換辞書を適用
          └── tx (利用者向け) に転送
```

### 5.3 内部トレイト階層

```rust
// ---- AsrBackend: 音声認識バックエンドが実装すべきトレイト ----
// PseudoAsrStreamer<B: AsrBackend> がこのトレイトに依存する。
// src/pipeline/streamer.rs に定義。
pub(crate) trait AsrBackend: Send {
    /// f32 サンプルをテキストに変換（バッチ推論）
    fn transcribe(&mut self, samples: &[f32]) -> Result<String>;
    /// LLM によるテキスト事後補正
    fn post_correct(&mut self, text: &str) -> Result<String>;
    /// 使用モデル名
    fn model_name(&self) -> String;
    /// ASR 使用量の内部記録
    fn record_asr_usage(&mut self, duration_ms: u64);
    /// 句読点挿入（デフォルト実装あり）
    fn insert_punctuation(&mut self, text: &str, _locale: &StreamerLocale) -> Result<String> {
        Ok(text.to_string())
    }
}

// ---- PostCorrectionBackend: 事後補正バックエンド ----
// src/pipeline/post_correct.rs に定義。
#[async_trait]
pub(crate) trait PostCorrectionBackend: Send + Sync {
    async fn post_correct(&self, text: &str) -> Result<String>;
}

// ---- InternalResampler: リサンプラ ----
// src/pipeline/resampler.rs に定義。
pub(crate) trait InternalResampler: Send {
    fn process(&mut self, input: &[f32]) -> Result<Vec<f32>, ResamplerError>;
    fn reset(&mut self);
}
```

### 5.4 `SttModelType` — エンジン特性の区分

```rust
/// 音声認識モデルの特性を区分する列挙型。
/// この区分により PostCorrectionProcessor が「届いたテキストの意味論」を正しく理解する。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum SttModelType {
    /// オフラインモデル（OpenAI Whisper 等）
    /// 届くデータの意味論: 「新しく増えた分（差分パケット）」
    /// バッファ操作: 末尾に「追記（Append）」
    #[default]
    UseOfflineModel,

    /// オンラインモデル（Apple Tahoe, Windows OS ディクテーション等）
    /// 届くデータの意味論: 「これが未確定区間の最新状態（Live State）」
    /// バッファ操作: 未確定区間を「上書き（Overwrite）」
    UseOnlineModel,
}
```

この区分が重要である理由: OS ネイティブエンジンは常に「セッション開始からの全文」を送信する。一方 OpenAI は VAD で切り出された「区間ごとの差分」を送信する。これを混同するとテキストが重複する。

---

## 6. ネイティブライブラリのプリビルドと同封

### 6.1 macOS: libspeech_helper.a

**ソース**: MYCUTE `native/swift/SpeechHelper.swift` をそのまま使用。

**ビルド手順** (リポジトリ root に `native/swift/build.sh` として配置):

```bash
#!/bin/bash
set -e

# ターゲット: macOS 15.0 (Swift SDK 26.0 相当)
SDK_PATH=$(xcrun --sdk macosx --show-sdk-path)

# arm64
swiftc \
    -target arm64-apple-macos15.0 \
    -O -whole-module-optimization \
    -sdk "$SDK_PATH" \
    -emit-library -static \
    -o libspeech_helper_arm64.a \
    SpeechHelper.swift

# x86_64
swiftc \
    -target x86_64-apple-macos15.0 \
    -O -whole-module-optimization \
    -sdk "$SDK_PATH" \
    -emit-library -static \
    -o libspeech_helper_x86_64.a \
    SpeechHelper.swift

# ユニバーサルバイナリ
lipo -create libspeech_helper_arm64.a libspeech_helper_x86_64.a \
    -output ../../prebuilt/macos/libspeech_helper.a

rm libspeech_helper_arm64.a libspeech_helper_x86_64.a
echo "Done: ../../prebuilt/macos/libspeech_helper.a"
```

**同封場所**: `prebuilt/macos/libspeech_helper.a`

### 6.2 Windows: speech_helper.lib + SpeechHelper.dll

**ソース**: MYCUTE `native/cs/SpeechHelper/` をそのまま使用。

**ビルド手順** (リポジトリ root に `native/cs/build.ps1` として配置):

```powershell
$ErrorActionPreference = "Stop"

dotnet publish SpeechHelper/SpeechHelper.csproj `
    -c Release `
    -r win-x64 `
    -p:PublishAot=true `
    -p:NativeLib=Shared `
    -p:StripSymbols=false

$base = "SpeechHelper/bin/Release/net10.0-windows10.0.26100.0/win-x64"

Copy-Item "$base/native/speech_helper.lib" "../../prebuilt/windows/speech_helper.lib"
Copy-Item "$base/publish/SpeechHelper.dll" "../../prebuilt/windows/SpeechHelper.dll"

Write-Host "Done: prebuilt/windows/"
```

**同封場所**:
- `prebuilt/windows/speech_helper.lib`
- `prebuilt/windows/SpeechHelper.dll`

### 6.3 Cargo パッケージング設定

```toml
# Cargo.toml に追加
[package]
# ...
include = [
    "src/**/*.rs",
    "prebuilt/**/*.a",
    "prebuilt/**/*.lib",
    "prebuilt/**/*.dll",
    "native/**/*.swift",
    "native/**/*.cs",
    "native/**/*.csproj",
    "native/**/*.sh",
    "native/**/*.ps1",
    "src/wav/*.wav",
    "README.md",
]
```

### 6.4 効果音 WAV の埋め込み

```rust
// src/audio.rs
static READY_WAV: &[u8] = include_bytes!("wav/piro.wav");
static COMMIT_WAV: &[u8] = include_bytes!("wav/commit.wav");
```

これは MYCUTE の `src/tools/audio.rs` の実装をそのまま移植する。

---

## 7. モジュール構成と実装ガイド

### 7.1 全体構造

```
voice-kit/
├── Cargo.toml
├── build.rs
├── prebuilt/
│   ├── macos/libspeech_helper.a
│   └── windows/{speech_helper.lib, SpeechHelper.dll}
├── native/                           # ソースコード（参照用・再ビルド用）
│   ├── swift/
│   │   ├── SpeechHelper.swift        # MYCUTE からコピー
│   │   └── build.sh
│   └── cs/
│       ├── SpeechHelper/
│       │   ├── SpeechHelper.cs       # MYCUTE からコピー
│       │   ├── SpeechHelper.csproj   # MYCUTE からコピー
│       │   └── Check.cs             # MYCUTE からコピー
│       └── build.ps1
├── src/
│   ├── lib.rs
│   ├── voice_kit.rs
│   ├── config.rs
│   ├── types.rs
│   ├── error.rs
│   ├── constants.rs
│   ├── recognizer.rs
│   ├── backends/
│   │   ├── mod.rs
│   │   ├── openai.rs
│   │   ├── mac.rs
│   │   └── win.rs
│   ├── pipeline/
│   │   ├── mod.rs
│   │   ├── streamer.rs
│   │   ├── vad.rs
│   │   ├── denoiser.rs
│   │   ├── resampler.rs
│   │   ├── post_correct.rs
│   │   ├── punctuation.rs
│   │   └── signal_filter.rs
│   ├── native/
│   │   ├── mod.rs
│   │   ├── mac_ffi.rs
│   │   └── win_ffi.rs
│   ├── audio.rs
│   ├── lindera_util.rs
│   └── wav/
│       ├── piro.wav
│       └── commit.wav
└── README.md
```

### 7.2 `src/lib.rs`

```rust
//! # voice-kit — ポータブル音声入力完全 crate
//!
//! ## 使用方法
//!
//! ```rust,no_run
//! use voice_kit::{VoiceKit, VoiceKitConfig, SttEngine, LocaleCode, SttEvent};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = VoiceKitConfig::builder()
//!         .engine(SttEngine::Os)
//!         .locale(LocaleCode::Ja)
//!         .vad_model_paths(VadModelPaths {
//!             silero: "/path/to/silero.onnx".into(),
//!             ten: "/path/to/ten.onnx".into(),
//!             gtcrn: String::new(),
//!         })
//!         .build()?;
//!
//!     let mut vk = VoiceKit::new(config)?;
//!     vk.start().await?;
//!
//!     while let Some(event) = vk.next_event().await {
//!         println!("{:?}", event);
//!     }
//!     Ok(())
//! }
//! ```

mod audio;
mod backends;
mod config;
mod constants;
mod error;
mod lindera_util;
mod native;
mod pipeline;
mod recognizer;
mod types;
mod voice_kit;

// 公開 API
pub use config::{VoiceKitConfig, VoiceKitConfigBuilder};
pub use error::VoiceKitError;
pub use types::*;
pub use voice_kit::VoiceKit;

// 内部トレイト（crate 内のみ可視）
pub(crate) use pipeline::streamer::{AsrBackend, BackendWrapper, StreamerEvent, StreamerLocale};
pub(crate) use pipeline::post_correct::PostCorrectionBackend;
```

### 7.3 `src/constants.rs` — 内部定数

MYCUTE `src/constants.rs` から必要な定数のみを抽出する：

```rust
// ---- タイムアウト・間隔 ----
pub(crate) const SPEECH_TIMEOUT_SEC: f64 = 30.0;
pub(crate) const STT_TIMEOUT_PUNCTUATION_MS: u64 = 500;
pub(crate) const POST_CORRECTION_SILENCE_WAIT_MS: u64 = 850;
pub(crate) const STT_DECORATION_INTERVAL_MS: u64 = 180;
pub(crate) const OPENAI_READY_DELAY_MS: u64 = 250;

// ---- モデルファイル名 ----
pub(crate) const MODEL_FILENAME_SILERO_VAD: &str = "silero_vad.onnx";
pub(crate) const MODEL_FILENAME_SILERO_VAD_INT8: &str = "silero_vad.int8.onnx";
pub(crate) const MODEL_FILENAME_TEN_VAD: &str = "ten_vad.onnx";
pub(crate) const MODEL_FILENAME_TEN_VAD_INT8: &str = "ten_vad.int8.onnx";
pub(crate) const MODEL_FILENAME_GTCRN: &str = "gtcrn.onnx";
```

### 7.4 `src/recognizer.rs` — SpeechRecognizer

**移植元**: MYCUTE `src/stt/recognizer.rs`

**重要な変更点**:

1. `crate::mycute_settings::*` → `crate::types::*` と `crate::config::*`
2. `crate::llm::client::LmgwClient` → `OpenAiConfig` から内部構築した `async_openai::Client`
3. `crate::constants::*` → `crate::constants::*`
4. `SttSettings` → `VoiceKitConfig`
5. `tauri::async_runtime` → `tokio`

**実装の要点**:

```rust
// src/recognizer.rs

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::mpsc;

use crate::backends::openai::{OpenAIBackend, OpenAIRecognizer};
#[cfg(target_os = "macos")]
use crate::backends::mac::MacSpeechBackend;
#[cfg(target_os = "windows")]
use crate::backends::win::WinSpeechBackend;
use crate::config::VoiceKitConfig;
use crate::error::VoiceKitError;
use crate::pipeline::post_correct::{PostCorrectionBackend, PostCorrectionConfig as PcConfig};
use crate::pipeline::streamer::BackendWrapper;
use crate::types::{SttEngine, SttEvent, LocaleCode};

pub(crate) struct SpeechRecognizer {
    is_running: Arc<AtomicBool>,
    engine: SttEngine,
    openai_backend: Option<OpenAIRecognizer>,
    #[cfg(target_os = "windows")]
    win_backend: Option<WinSpeechBackend>,
    #[cfg(target_os = "macos")]
    mac_backend: Option<MacSpeechBackend>,
    tx: mpsc::Sender<SttEvent>,
    shared_locale: Arc<parking_lot::Mutex<LocaleCode>>,
    replaces_map: Arc<parking_lot::RwLock<indexmap::IndexMap<String, Vec<String>>>>,
}

impl SpeechRecognizer {
    pub(crate) fn new(
        tx: mpsc::Sender<SttEvent>,
        config: &VoiceKitConfig,
        replaces_map: Arc<parking_lot::RwLock<indexmap::IndexMap<String, Vec<String>>>>,
    ) -> Result<Self, VoiceKitError> {
        // ================================================================
        // インターセプター層の構築:
        // 各バックエンドには tx_internal を渡し、イベントを中継タスクで
        // 受信し、FinalResult/PartialResult のテキストに置換辞書を適用してから
        // 本来の tx（利用者向け）に転送する。
        // ================================================================
        let (tx_internal, mut rx_internal) = mpsc::channel::<SttEvent>(100);
        let replaces_map_for_task = replaces_map.clone();
        let tx_for_task = tx.clone();

        // インターセプタータスク: std::thread で駆動
        std::thread::spawn(move || {
            while let Some(event) = rx_internal.blocking_recv() {
                let forwarded = match event {
                    SttEvent::FinalResult(text, seq) => {
                        let replaced = apply_replaces(&replaces_map_for_task, &text);
                        SttEvent::FinalResult(replaced, seq)
                    }
                    SttEvent::PartialResult(text, seq) => {
                        let replaced = apply_replaces(&replaces_map_for_task, &text);
                        SttEvent::PartialResult(replaced, seq)
                    }
                    other => other,
                };
                if tx_for_task.blocking_send(forwarded).is_err() {
                    break;
                }
            }
        });

        let shared_locale = Arc::new(parking_lot::Mutex::new(config.locale));

        // ---- OpenAI バックエンド (常に初期化) ----
        let openai_backend = {
            let mut recognizer = OpenAIRecognizer::new(
                tx_internal.clone(),
                config,
                shared_locale.clone(),
            );
            match recognizer.init_audio() {
                Ok(_) => Some(recognizer),
                Err(e) => {
                    log::error!("[SpeechRecognizer] OpenAI init failed: {}", e);
                    None
                }
            }
        };

        // ---- macOS バックエンド ----
        #[cfg(target_os = "macos")]
        let mac_backend = {
            let (pc_backend, pc_config) =
                build_pc_backend(config, &shared_locale);
            match MacSpeechBackend::new(
                tx_internal.clone(),
                shared_locale.clone(),
                pc_backend,
                pc_config,
                config,
            ) {
                Ok(be) => Some(be),
                Err(e) => {
                    log::error!("[SpeechRecognizer] macOS backend init failed: {}", e);
                    None
                }
            }
        };

        // ---- Windows バックエンド ----
        #[cfg(target_os = "windows")]
        let win_backend = {
            let (pc_backend, pc_config) =
                build_pc_backend(config, &shared_locale);
            match WinSpeechBackend::new(
                tx_internal.clone(),
                shared_locale.clone(),
                pc_backend,
                pc_config,
                config,
            ) {
                Ok(be) => Some(be),
                Err(e) => {
                    log::error!("[SpeechRecognizer] Windows backend init failed: {}", e);
                    None
                }
            }
        };

        Ok(Self {
            is_running: Arc::new(AtomicBool::new(false)),
            engine: config.engine,
            openai_backend,
            #[cfg(target_os = "windows")] win_backend,
            #[cfg(target_os = "macos")] mac_backend,
            tx,
            shared_locale,
            replaces_map,
        })
    }

    pub(crate) fn start(&mut self) {
        // MYCUTE recognizer.rs の start() をそのまま移植
        // engine に応じて openai_backend / mac_backend / win_backend の .start() を呼ぶ
        if self.is_running.load(Ordering::SeqCst) { return; }
        self.is_running.store(true, Ordering::SeqCst);
        let _ = self.tx.try_send(SttEvent::Started);

        if self.engine == SttEngine::OpenAi {
            if let Some(ref mut be) = self.openai_backend { be.start(); }
            return;
        }

        #[cfg(target_os = "windows")]
        if self.engine == SttEngine::Os {
            if let Some(ref mut be) = self.win_backend { be.start(); }
            return;
        }

        #[cfg(target_os = "macos")]
        if self.engine == SttEngine::Os {
            if let Some(ref mut be) = self.mac_backend { be.start(); }
            return;
        }
    }

    pub(crate) fn stop(&mut self) {
        // MYCUTE recognizer.rs の stop() をそのまま移植
        if !self.is_running.load(Ordering::SeqCst) { return; }
        self.is_running.store(false, Ordering::SeqCst);

        if let Some(ref mut be) = self.openai_backend { be.stop(); }
        #[cfg(target_os = "windows")]
        if let Some(ref mut be) = self.win_backend { be.stop(); }
        #[cfg(target_os = "macos")]
        if let Some(ref mut be) = self.mac_backend { be.stop(); }

        let _ = self.tx.try_send(SttEvent::Stopped);
    }

    pub(crate) fn set_locale(&mut self, locale: LocaleCode) {
        *self.shared_locale.lock() = locale;
        if let Some(ref mut be) = self.openai_backend { be.set_locale(locale); }
        #[cfg(target_os = "windows")]
        if let Some(ref mut be) = self.win_backend { be.set_locale(locale); }
        #[cfg(target_os = "macos")]
        if let Some(ref mut be) = self.mac_backend { be.set_locale(locale); }
    }

    pub(crate) fn set_engine(&mut self, engine: SttEngine) { self.engine = engine; }
    pub(crate) fn is_running(&self) -> bool { self.is_running.load(Ordering::SeqCst) }

    pub(crate) fn update_replaces(&mut self, replaces: indexmap::IndexMap<String, Vec<String>>) {
        *self.replaces_map.write() = replaces;
    }

    pub(crate) fn health_check(&self) -> u32 {
        #[cfg(target_os = "windows")]
        { crate::native::win_ffi::health_check_result() }
        #[cfg(not(target_os = "windows"))]
        { 0 }
    }

    pub(crate) fn tick(&mut self) {
        // MYCUTE recognizer.rs の tick() をそのまま移植
        // engine に応じて対応するバックエンドの tick() を呼ぶ
        if !self.is_running.load(Ordering::SeqCst) { return; }
        match self.engine {
            SttEngine::OpenAi => {
                if let Some(ref mut be) = self.openai_backend { be.tick(); }
            }
            SttEngine::Os => {
                #[cfg(target_os = "windows")]
                if let Some(ref mut be) = self.win_backend { be.tick(); }
                #[cfg(target_os = "macos")]
                if let Some(ref mut be) = self.mac_backend { be.tick(); }
            }
        }
    }
}

impl Drop for SpeechRecognizer {
    fn drop(&mut self) {
        self.stop();
        #[cfg(target_os = "macos")]
        if let Some(ref be) = self.mac_backend { be.cleanup(); }
    }
}

/// 事後補正バックエンドをビルドするヘルパー（MYCUTE recognizer.rs のロジックを抽出）
fn build_pc_backend(
    config: &VoiceKitConfig,
    shared_locale: &Arc<parking_lot::Mutex<LocaleCode>>,
) -> (Option<Arc<dyn PostCorrectionBackend>>, Option<PcConfig>) {
    if let Some(ref oa_cfg) = config.openai_config {
        if let Ok(oa_backend) = OpenAIBackend::new(oa_cfg, shared_locale.clone()) {
            let wrapper: Arc<dyn PostCorrectionBackend> =
                Arc::new(BackendWrapper(Arc::new(std::sync::Mutex::new(oa_backend))));
            let pc_cfg = PcConfig {
                sentence_count_threshold: config.post_correction.sentence_count_threshold,
                min_text_length: config.post_correction.min_text_length,
                interval_ms: config.post_correction.interval_ms,
            };
            return (Some(wrapper), Some(pc_cfg));
        }
    }
    (None, None)
}

/// 置換辞書の適用（MYCUTE recognizer.rs の apply_replaces_from_map を移植）
fn apply_replaces(
    map: &parking_lot::RwLock<indexmap::IndexMap<String, Vec<String>>>,
    text: &str,
) -> String {
    let map = map.read();
    if map.is_empty() { return text.to_string(); }

    let mut flat: Vec<(&str, &str)> = Vec::new();
    for (after, befores) in map.iter() {
        for before in befores {
            if !before.is_empty() {
                flat.push((before.as_str(), after.as_str()));
            }
        }
    }
    // 最長一致優先
    flat.sort_by(|a, b| b.0.len().cmp(&a.0.len()));

    let mut result = text.to_string();
    for (from, to) in &flat {
        result = result.replace(from, to);
    }
    result
}
```

### 7.5 `src/pipeline/streamer.rs` — PseudoAsrStreamer

**移植元**: MYCUTE `src/tools/pseudo_asr_streamer.rs`

**変更点**: このファイルはほぼ完全にそのまま移植する。インポートパスだけ変更する。

```rust
// 変更するインポート:
// crate::tools::resampler::{InternalResampler, SincResampler}
//   → crate::pipeline::resampler::{InternalResampler, SincResampler}
// crate::tools::vad_processor::{VadConfig, VadProcessor, VadType, VAD_SAMPLE_RATE}
//   → crate::pipeline::vad::{VadConfig, VadProcessor, VadType, VAD_SAMPLE_RATE}
// crate::tools::post_correction_processor::{PostCorrectionBackend, PostCorrectionConfig, PostCorrectionProcessor, ProcessorOutput}
//   → crate::pipeline::post_correct::{PostCorrectionBackend, PostCorrectionConfig, PostCorrectionProcessor, ProcessorOutput}

// 以下の項目は変更せずそのまま移植:
// - VadType enum → StreamerConfig 内で使うため内部定義を維持
// - AsrBackend trait → 変更なし
// - BackendWrapper<B> → 変更なし
// - StreamerConfig → 変更なし
// - StreamerEvent enum → 変更なし
// - PseudoAsrStreamer<B> の全フィールドと全メソッド → 変更なし
// - SpeechDenoiser → 独立ファイル denoiser.rs へ抽出
```

**SpeechDenoiser は独立ファイルに抽出する**（`src/pipeline/denoiser.rs`）:

```rust
// src/pipeline/denoiser.rs
use anyhow::{anyhow, Result};
use sherpa_rs_sys as sys;
use std::ffi::CString;

pub(crate) struct SpeechDenoiser {
    inner: *const sys::SherpaOnnxOfflineSpeechDenoiser,
}

unsafe impl Send for SpeechDenoiser {}
unsafe impl Sync for SpeechDenoiser {}

impl SpeechDenoiser {
    pub(crate) fn new(model_path: &str, num_threads: i32) -> Result<Self> {
        let c_model = CString::new(model_path)?;
        let gtcrn_config = sys::SherpaOnnxOfflineSpeechDenoiserGtcrnModelConfig {
            model: c_model.as_ptr(),
        };
        let model_config = sys::SherpaOnnxOfflineSpeechDenoiserModelConfig {
            gtcrn: gtcrn_config,
            num_threads,
            debug: 0,
            provider: std::ptr::null(),
        };
        let config = sys::SherpaOnnxOfflineSpeechDenoiserConfig { model: model_config };
        let denoiser = unsafe { sys::SherpaOnnxCreateOfflineSpeechDenoiser(&config) };
        if denoiser.is_null() {
            return Err(anyhow!("Failed to create SherpaOnnxOfflineSpeechDenoiser."));
        }
        Ok(Self { inner: denoiser })
    }

    pub(crate) fn run(&self, samples: &[f32], sample_rate: i32) -> Result<Vec<f32>> {
        let result_ptr = unsafe {
            sys::SherpaOnnxOfflineSpeechDenoiserRun(
                self.inner, samples.as_ptr(), samples.len() as i32, sample_rate,
            )
        };
        if result_ptr.is_null() {
            return Err(anyhow!("Denoiser returned null result."));
        }
        let result = unsafe { &*result_ptr };
        let output = if result.n > 0 && !result.samples.is_null() {
            unsafe { std::slice::from_raw_parts(result.samples, result.n as usize).to_vec() }
        } else { Vec::new() };
        unsafe { sys::SherpaOnnxDestroyDenoisedAudio(result_ptr) };
        Ok(output)
    }
}

impl Drop for SpeechDenoiser {
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe { sys::SherpaOnnxDestroyOfflineSpeechDenoiser(self.inner) };
        }
    }
}
```

### 7.6 `src/pipeline/vad.rs` — VadProcessor

**移植元**: MYCUTE `src/tools/vad_processor.rs`

**変更点**: なし。完全にそのまま移植する。`resolve_ascii_path` の Windows 固有ロジックも維持する。

### 7.7 `src/pipeline/resampler.rs` — SincResampler

**移植元**: MYCUTE `src/tools/resampler.rs`

**変更点**: なし。完全にそのまま移植する。

### 7.8 `src/pipeline/post_correct.rs` — PostCorrectionProcessor

**移植元**: MYCUTE `src/tools/post_correction_processor.rs`

**変更点**: `crate::constants::POST_CORRECTION_SILENCE_WAIT_MS` 参照を `crate::constants::POST_CORRECTION_SILENCE_WAIT_MS` に置換するのみ。その他はそのまま移植。

### 7.9 `src/pipeline/punctuation.rs` — PunctuationMachine

**移植元**: MYCUTE `src/tools/punctuation_machine.rs`

**変更点**:
- `crate::mycute_settings::LocaleCode` → `crate::types::LocaleCode`
- `super::lindera_util` → `crate::lindera_util`

### 7.10 `src/pipeline/signal_filter.rs` — 信号品質フィルタ

MYCUTE `src/tools/pseudo_asr_streamer.rs` の `is_worthy_to_run_asr()` メソッドを独立したユーティリティ関数として抽出：

```rust
// src/pipeline/signal_filter.rs
use crate::types::SignalFilterConfig;

/// 音声信号が意味のある内容を含むかどうかを判定する。
pub(crate) fn is_worthy_to_run_asr(
    samples: &[f32],
    config: &SignalFilterConfig,
    utterance_min_ms: u64,
    sample_rate: u32,
) -> bool {
    if !config.enabled { return true; }
    if samples.is_empty() { return false; }

    let duration_ms = (samples.len() as f32 / sample_rate as f32) * 1000.0;
    if duration_ms < utterance_min_ms as f32 { return false; }

    let sum_sq: f32 = samples.iter().map(|s| s * s).sum();
    let rms = (sum_sq / samples.len() as f32).sqrt();

    let active_samples = samples.iter().filter(|&s| s.abs() > config.rms_threshold).count();
    let occupancy_ratio = active_samples as f32 / samples.len() as f32;

    rms >= config.rms_threshold && occupancy_ratio >= config.occupancy_ratio
}
```

### 7.11 `src/backends/openai.rs` — OpenAIBackend + OpenAIRecognizer

**移植元**: MYCUTE `src/stt/openai.rs`

**変更する依存**:
- `crate::constants::*` → `crate::constants::*`
- `crate::mycute_settings::*` → `crate::types::*` と `crate::config::*`
- `crate::stt::mac::start_native_audio_capture` → `crate::native::mac_ffi::*`
- `crate::stt::win::start_native_audio_capture` → `crate::native::win_ffi::*`
- `tauri::async_runtime` → `tokio::task`

**OpenAI API 呼び出しのコード**:

```rust
// OpenAIBackend::transcribe() の実装（MYCUTE openai.rs から抽出・簡略化）
fn transcribe(&mut self, samples: &[f32]) -> Result<String> {
    // 1. f32 → メモリ上 WAV (hound)
    let mut buffer = std::io::Cursor::new(Vec::new());
    let spec = hound::WavSpec {
        channels: 1, sample_rate: 16000,
        bits_per_sample: 32, sample_format: hound::SampleFormat::Float,
    };
    {
        let mut writer = hound::WavWriter::new(&mut buffer, spec)?;
        for sample in samples { writer.write_sample(*sample)?; }
        writer.finalize()?;
    }
    let wav_bytes = buffer.into_inner();

    // 2. async-openai クライアントを OpenAiConfig から構築
    let oa_config = async_openai::config::OpenAIConfig::new()
        .with_api_base(&self.openai_config.base_url)
        .with_api_key(&self.openai_config.api_key);
    let client = async_openai::Client::with_config(oa_config);

    // 3. リクエスト実行
    let audio_input = async_openai::types::audio::AudioInput::from_vec_u8(
        "input.wav".into(), wav_bytes);
    let request = async_openai::types::audio::CreateTranscriptionRequestArgs::default()
        .file(audio_input)
        .model(&self.openai_config.model)
        .language(self.language.lock().as_iso639_1().to_string())
        .build()?;

    let result = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(async {
            client.audio().transcription().create(request).await
        })
    });

    match result {
        Ok(response) => Ok(response.text),
        Err(e) => Err(anyhow::anyhow!("Transcription failed: {}", e)),
    }
}
```

**OpenAIRecognizer の構造体**:

```rust
pub(crate) struct OpenAIRecognizer {
    streamer: Arc<Mutex<Option<PseudoAsrStreamer<OpenAIBackend>>>>,
    tx: mpsc::Sender<SttEvent>,
    is_running: Arc<AtomicBool>,
    ticker_task: Option<JoinHandle<()>>,
    decoration_task: Arc<Mutex<Option<JoinHandle<()>>>>,
    is_decorating: Arc<AtomicBool>,
    sequence_counter: Arc<AtomicU64>,
    language: Arc<parking_lot::Mutex<LocaleCode>>,
    session_counter: Arc<AtomicU64>,
    audio_buf: Arc<Mutex<Vec<f32>>>,
    sample_rate: Arc<AtomicU32>,
    partial_result_buffer: Arc<Mutex<Option<String>>>,
    last_speech_end_time: Arc<Mutex<Option<std::time::Instant>>>,
    capture_task: Option<JoinHandle<()>>,
    event_listener_task: Option<JoinHandle<()>>,
    event_rx: Arc<Mutex<Option<mpsc::Receiver<StreamerEvent>>>>,
    openai_config: OpenAiConfig,
    vad_config: VadConfig,
    post_correction_config: PostCorrectionConfig,
    denoiser_config: DenoiserConfig,
    signal_filter_config: SignalFilterConfig,
    vad_model_paths: VadModelPaths,
    punctuation_enabled: bool,
}
```

**重要**: `OpenAIRecognizer::init_audio()` / `start()` / `stop()` / `tick()` は MYCUTE `src/stt/openai.rs` の実装をそのまま移植する。ただし:
- `tauri::async_runtime::spawn` → `tokio::spawn` に置換
- `crate::stt::stats::UsageStats::init()` と `UsageStats::record_asr()` は削除（crate 内部の簡易統計または削除）

### 7.12 `src/backends/mac.rs` — MacSpeechBackend

**移植元**: MYCUTE `src/stt/mac.rs`

**変更点**:
- `crate::mycute_settings::*` → `crate::types::*`, `crate::config::*`
- `crate::tools::*` → `crate::pipeline::*`
- `crate::constants::SPEECH_TIMEOUT_SEC` → `config.speech_timeout_sec`
- 外部関数宣言 (`extern "C"`) を `crate::native::mac_ffi` に移動
- `start_native_audio_capture` / `stop_native_audio_capture` は `src/backends/mac.rs` 内のプライベート関数として定義（グローバルチャネル `MAC_AUDIO_SENDER` 管理を含む）

**MacSpeechBackend の構造体**:

```rust
pub(crate) struct MacSpeechBackend {
    is_running: Arc<AtomicBool>,
    internal_engine: InternalMacEngine,
    locale: Arc<parking_lot::Mutex<LocaleCode>>,
    post_correction_processor: Arc<parking_lot::Mutex<Option<PostCorrectionProcessor>>>,
    is_speaking: Arc<AtomicBool>,
    vad_processor: Arc<parking_lot::Mutex<Option<VadProcessor>>>,
    rx_raw: Arc<parking_lot::Mutex<Option<mpsc::Receiver<SttEvent>>>>,
    tx_app: mpsc::Sender<SttEvent>,
    ticker_task: Option<tokio::task::JoinHandle<()>>,
    resampler: Arc<parking_lot::Mutex<Option<SincResampler>>>,
    config: VoiceKitConfig,  // 設定全体への参照
}
```

**実装の要点**:  `MacSpeechBackend::new()` / `start()` / `stop()` / `tick()` は MYCUTE `src/stt/mac.rs` の実装をそのまま移植する。ロジックの変更は不要。

### 7.13 `src/backends/win.rs` — WinSpeechBackend

**移植元**: MYCUTE `src/stt/win.rs`

**変更点**:
- 上記 mac.rs と同様のインポート変更
- FFI 宣言を `crate::native::win_ffi` に移動
- `start_native_audio_capture` / `stop_native_audio_capture` は `src/backends/win.rs` 内のプライベート関数として定義
- `disable_ime()` / `restore_ime()` は内部関数に
- `get_health_check_result()` / `acknowledge_health_check()` / `is_health_check_acknowledged()` は `crate::native::win_ffi` に移動

**WinSpeechBackend の構造体**:

```rust
pub(crate) struct WinSpeechBackend {
    is_running: Arc<AtomicBool>,
    locale: Arc<parking_lot::Mutex<LocaleCode>>,
    post_correction_processor: Arc<parking_lot::Mutex<Option<PostCorrectionProcessor>>>,
    is_speaking: Arc<AtomicBool>,
    vad_processor: Arc<parking_lot::Mutex<Option<VadProcessor>>>,
    rx_raw: Arc<parking_lot::Mutex<Option<mpsc::Receiver<SttEvent>>>>,
    tx_app: mpsc::Sender<SttEvent>,
    ticker_task: Option<tokio::task::JoinHandle<()>>,
    resampler: Arc<parking_lot::Mutex<Option<SincResampler>>>,
    config: VoiceKitConfig,
}
```

**実装の要点**: `WinSpeechBackend::new()` / `start()` / `stop()` / `tick()` は MYCUTE `src/stt/win.rs` の実装をそのまま移植する。ロジックの変更は不要。

### 7.14 `src/native/` — FFI バインディング

```rust
// src/native/mod.rs
#[cfg(target_os = "macos")]
pub(crate) mod mac_ffi;

#[cfg(target_os = "windows")]
pub(crate) mod win_ffi;
```

```rust
// src/native/mac_ffi.rs
use std::ffi::c_char;

#[link(name = "SpeechHelper")]
extern "C" {
    pub(crate) fn speech_helper_init(speech_timeout_sec: f64) -> i32;
    pub(crate) fn speech_helper_request_authorization() -> i32;
    pub(crate) fn speech_helper_set_result_callback(
        callback: extern "C" fn(*const c_char, i32)
    );
    pub(crate) fn speech_helper_set_error_callback(
        callback: extern "C" fn(*const c_char)
    );
    pub(crate) fn speech_helper_set_ready_callback(
        callback: extern "C" fn()
    );
    pub(crate) fn speech_helper_set_audio_data_callback(
        callback: Option<extern "C" fn(*const f32, i32, i32)>
    );
    pub(crate) fn speech_helper_start_capture() -> i32;
    pub(crate) fn speech_helper_stop_capture();
    pub(crate) fn speech_helper_start(locale: *const c_char) -> i32;
    pub(crate) fn speech_helper_stop();
    pub(crate) fn speech_helper_cleanup();
    pub(crate) fn speech_helper_tick();

    // Tahoe (macOS 15+)
    pub(crate) fn tahoe_helper_init(
        locale: *const c_char, speech_timeout_sec: f64
    ) -> i32;
    pub(crate) fn tahoe_helper_start(locale: *const c_char) -> i32;
    pub(crate) fn tahoe_helper_stop();
}
```

```rust
// src/native/win_ffi.rs
use std::ffi::{c_char, c_int};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

#[link(name = "SpeechHelper", kind = "static")]
extern "C" {
    pub(crate) fn speech_helper_init(speech_timeout_sec: f64) -> c_int;
    pub(crate) fn speech_helper_set_result_callback(
        callback: extern "C" fn(*const c_char, c_int)
    );
    pub(crate) fn speech_helper_set_error_callback(
        callback: extern "C" fn(*const c_char)
    );
    pub(crate) fn speech_helper_set_ready_callback(callback: extern "C" fn());
    pub(crate) fn speech_helper_set_audio_data_callback(
        callback: Option<extern "C" fn(*const f32, u32, u32)>
    );
    pub(crate) fn speech_helper_start_capture() -> c_int;
    pub(crate) fn speech_helper_stop_capture();
    pub(crate) fn speech_helper_start(locale: *const c_char) -> c_int;
    pub(crate) fn speech_helper_stop();
    pub(crate) fn speech_helper_cleanup();
    pub(crate) fn speech_helper_tick();
    pub(crate) fn speech_helper_disable_ime();
    pub(crate) fn speech_helper_restore_ime();
    pub(crate) fn speech_helper_check_health() -> c_int;
}

// ---- ヘルスチェック状態管理 ----
static WIN_HEALTH_CHECK: AtomicU32 = AtomicU32::new(0);
static WIN_HEALTH_CHECKED: AtomicBool = AtomicBool::new(false);

pub(crate) fn health_check_result() -> u32 {
    WIN_HEALTH_CHECK.load(Ordering::Relaxed)
}

pub(crate) fn store_health_check_result(result: u32) {
    WIN_HEALTH_CHECK.store(result, Ordering::Relaxed);
}

pub(crate) fn is_health_check_acknowledged() -> bool {
    WIN_HEALTH_CHECKED.load(Ordering::Relaxed)
}

pub(crate) fn acknowledge_health_check() {
    WIN_HEALTH_CHECKED.store(true, Ordering::Relaxed);
}
```

### 7.15 `src/audio.rs` — 効果音再生

MYCUTE `src/tools/audio.rs` をそのまま移植する。WAV ファイルは `include_bytes!` で埋め込み済み。

### 7.16 `src/lindera_util.rs` — Lindera 初期化

MYCUTE `src/tools/lindera_util.rs` をそのまま移植する。

### 7.17 MYCUTE からの移植ファイル一覧（サマリ）

| voice-kit ファイル | 移植元 MYCUTE ファイル | 変更レベル |
|---|---|---|
| `src/types.rs` | `src/types.rs` + `src/mycute_settings.rs` (一部) | 中（再構成） |
| `src/config.rs` | `src/mycute_settings.rs` (一部) | 大（新規設計） |
| `src/error.rs` | 新規 | 新規 |
| `src/constants.rs` | `src/constants.rs` (一部抽出) | 小（抽出のみ） |
| `src/recognizer.rs` | `src/stt/recognizer.rs` | 中（インポート変更） |
| `src/voice_kit.rs` | 新規 | 新規 |
| `src/backends/openai.rs` | `src/stt/openai.rs` | 中（インポート変更 + LmgwClient除去） |
| `src/backends/mac.rs` | `src/stt/mac.rs` | 中（インポート変更） |
| `src/backends/win.rs` | `src/stt/win.rs` | 中（インポート変更） |
| `src/pipeline/streamer.rs` | `src/tools/pseudo_asr_streamer.rs` | 小（Denoiser分離 + インポート変更） |
| `src/pipeline/vad.rs` | `src/tools/vad_processor.rs` | なし（そのまま） |
| `src/pipeline/denoiser.rs` | `src/tools/pseudo_asr_streamer.rs` (一部抽出) | 小（抽出のみ） |
| `src/pipeline/resampler.rs` | `src/tools/resampler.rs` | なし（そのまま） |
| `src/pipeline/post_correct.rs` | `src/tools/post_correction_processor.rs` | 小（定数参照変更のみ） |
| `src/pipeline/punctuation.rs` | `src/tools/punctuation_machine.rs` | 小（インポート変更） |
| `src/pipeline/signal_filter.rs` | `src/tools/pseudo_asr_streamer.rs` (一部抽出) | 小（抽出のみ） |
| `src/native/mac_ffi.rs` | `src/stt/mac.rs` (FFI部抽出) | 小（抽出のみ） |
| `src/native/win_ffi.rs` | `src/stt/win.rs` (FFI部抽出) | 小（抽出のみ） |
| `src/audio.rs` | `src/tools/audio.rs` | なし（そのまま） |
| `src/lindera_util.rs` | `src/tools/lindera_util.rs` | なし（そのまま） |

---

## 8. Cargo.toml 依存関係

```toml
[package]
name = "voice-kit"
version = "0.1.0"
edition = "2021"
description = "Portable, cross-platform speech-to-text crate with OS-native and OpenAI backends"
license = "MIT OR Apache-2.0"
repository = "https://github.com/..."
include = [
    "src/**/*.rs",
    "prebuilt/**/*.a",
    "prebuilt/**/*.lib",
    "prebuilt/**/*.dll",
    "native/**/*.swift",
    "native/**/*.cs",
    "native/**/*.csproj",
    "native/**/*.sh",
    "native/**/*.ps1",
    "src/wav/*.wav",
    "README.md",
]

[dependencies]
# 非同期ランタイム
tokio = { version = "1.49", features = ["full"] }

# シリアライズ
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# 同期プリミティブ
parking_lot = "0.12"
lazy_static = "1.4"

# エラーハンドリング
anyhow = "1.0"
thiserror = "2.0"

# Sherpa-ONNX (VAD + Denoiser)
sherpa-rs = "0.6"
sherpa-rs-sys = "0.6"

# 音声エンコーディング (WAV生成)
hound = "3.5"

# リサンプリング
rubato = "0.16"

# 形態素解析（日本語句読点挿入）
lindera = { version = "2.0", features = ["embed-ipadic"] }
lindera-ipadic = "2.0"

# OpenAI API クライアント
async-openai = { version = "0.36", features = ["audio", "chat-completion"] }

# コレクション
indexmap = { version = "2.13", features = ["serde"] }

# 効果音再生
rodio = "0.21"

# 非同期トレイト
async-trait = "0.1"

# ロギング
log = "0.4"

[target.'cfg(target_os = "windows")'.dependencies]
winapi = { version = "0.3", features = ["fileapi", "winbase"] }

[lib]
name = "voice_kit"
crate-type = ["lib"]
```

---

## 9. build.rs 設計

```rust
// build.rs
use std::env;
use std::path::PathBuf;

fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let prebuilt = manifest_dir.join("prebuilt");

    if target_os == "windows" {
        // ---- Windows: C# Native AOT Shared Link ----
        let win_dir = prebuilt.join("windows");
        let lib_path = win_dir.join("speech_helper.lib");
        let dll_path = win_dir.join("SpeechHelper.dll");

        if lib_path.exists() {
            println!("cargo:rustc-link-lib=SpeechHelper");
            println!("cargo:rustc-link-search=native={}", win_dir.display());
        } else {
            panic!(
                "speech_helper.lib not found at {}. \
                 Run native/cs/build.ps1 to build it.",
                win_dir.display()
            );
        }

        // DLL を OUT_DIR にコピー
        if dll_path.exists() {
            let out_dir = env::var("OUT_DIR").unwrap();
            let dest = PathBuf::from(&out_dir)
                .join("..").join("..").join("..")
                .join("SpeechHelper.dll");
            std::fs::copy(&dll_path, &dest)
                .expect("Failed to copy SpeechHelper.dll to target directory");
        }

        // Windows システムライブラリ
        for lib in &[
            "ole32", "oleaut32", "advapi32", "bcrypt", "crypt32",
            "iphlpapi", "kernel32", "mswsock", "ntdll", "secur32",
            "user32", "ws2_32",
        ] {
            println!("cargo:rustc-link-lib={}", lib);
        }
        println!("cargo:rustc-link-arg=/IGNORE:4099");

    } else if target_os == "macos" {
        // ---- macOS: Swift Static Link ----
        let mac_dir = prebuilt.join("macos");
        let lib_path = mac_dir.join("libspeech_helper.a");

        if lib_path.exists() {
            println!("cargo:rustc-link-lib=static=SpeechHelper");
            println!("cargo:rustc-link-search=native={}", mac_dir.display());
        } else {
            panic!(
                "libspeech_helper.a not found at {}. \
                 Run native/swift/build.sh to build it.",
                mac_dir.display()
            );
        }

        // Swift ランタイムライブラリパス
        if let Ok(output) = std::process::Command::new("swiftc")
            .args(&["-print-target-info"]).output()
        {
            if let Ok(stdout) = String::from_utf8(output.stdout) {
                if let Some(paths_start) = stdout.find("\"runtimeLibraryPaths\"") {
                    if let Some(list_start) = stdout[paths_start..].find('[') {
                        let list_start = paths_start + list_start;
                        if let Some(list_end) = stdout[list_start..].find(']') {
                            let list_end = list_start + list_end;
                            let paths_str = &stdout[list_start + 1..list_end];
                            for path in paths_str.split(',') {
                                let path = path.trim().trim_matches('"').trim();
                                if !path.is_empty() {
                                    println!("cargo:rustc-link-search=native={}", path);
                                }
                            }
                        }
                    }
                }
            }
        }

        // RPATH
        for rpath in &[
            "/usr/lib/swift",
            "@executable_path/",
            "@loader_path/",
        ] {
            println!("cargo:rustc-link-arg=-Wl,-rpath,{}", rpath);
        }

        // System Frameworks
        for fw in &["Foundation", "AVFoundation", "Speech", "CoreFoundation"] {
            println!("cargo:rustc-link-lib=framework={}", fw);
        }

    } else {
        println!(
            "cargo:warning=voice-kit: unsupported target OS '{}'. \
             Only OpenAI engine will be available.",
            target_os
        );
    }

    println!("cargo:rerun-if-changed=prebuilt/");
}
```

**MYCUTE の build.rs との差分**:
- `tauri_build::build()` 呼び出しを**削除**（Tauri 非依存）
- `SPEECH_HELPER_LIB_DIR` 環境変数サポートは**削除**（プリビルドライブラリが同封されているため不要）
- macOS Tauri バンドル固有の RPATH (`../Resources`, `../Frameworks`) を**削除**。利用側のアプリが必要に応じて追加する
- プリビルドライブラリ不在時は `panic!` で明示的にエラー終了

---

## 10. テスト戦略

### 10.1 ユニットテスト

```rust
// src/pipeline/resampler.rs の末尾に追加
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sinc_resampler_48k_to_16k() {
        let mut resampler = SincResampler::new(48000, 16000).unwrap();
        let input: Vec<f32> = (0..4800).map(|i| (i as f32 * 0.01).sin()).collect();
        let output = resampler.process(&input).unwrap();
        assert!(output.len() > input.len() / 4);
        assert!(output.len() < input.len() / 2);
    }

    #[test]
    fn test_resampler_reset() {
        let mut resampler = SincResampler::new(48000, 16000).unwrap();
        let input = vec![0.5f32; 2048];
        let _ = resampler.process(&input).unwrap();
        resampler.reset();
        let output = resampler.process(&input).unwrap();
        assert!(!output.is_empty());
    }

    #[test]
    fn test_pass_through_same_rate() {
        let mut resampler = SincResampler::new(16000, 16000).unwrap();
        let input = vec![1.0f32; 1024];
        let output = resampler.process(&input).unwrap();
        assert!(!output.is_empty());
    }
}
```

```rust
// src/pipeline/post_correct.rs の末尾に追加
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;

    struct MockBackend;
    #[async_trait]
    impl PostCorrectionBackend for MockBackend {
        async fn post_correct(&self, text: &str) -> Result<String> {
            Ok(format!("[OK] {}", text))
        }
    }

    #[test]
    fn test_offline_model_appends() {
        let backend = Arc::new(MockBackend);
        let is_speaking = Arc::new(AtomicBool::new(false));
        let mut proc = PostCorrectionProcessor::with_model_type(
            backend, PostCorrectionConfig::default(),
            SttModelType::UseOfflineModel, is_speaking,
        );
        let out1 = proc.process_input("hello").unwrap();
        assert!(matches!(out1, ProcessorOutput::Partial(ref s) if s == "hello"));
        let out2 = proc.process_input("world").unwrap();
        assert!(matches!(out2, ProcessorOutput::Partial(ref s) if s == "helloworld"));
    }

    #[test]
    fn test_online_model_overwrites() {
        let backend = Arc::new(MockBackend);
        let is_speaking = Arc::new(AtomicBool::new(false));
        let mut proc = PostCorrectionProcessor::with_model_type(
            backend, PostCorrectionConfig::default(),
            SttModelType::UseOnlineModel, is_speaking,
        );
        let out1 = proc.process_input("hello").unwrap();
        assert!(matches!(out1, ProcessorOutput::Partial(ref s) if s == "hello"));
        let out2 = proc.process_input("hello world").unwrap();
        assert!(matches!(out2, ProcessorOutput::Partial(ref s) if s == "hello world"));
    }
}
```

```rust
// tests/integration_test.rs (crate ルートの tests/ ディレクトリ)
use voice_kit::*;

#[test]
fn test_config_validation_openai_requires_api_key() {
    let result = VoiceKitConfig::builder()
        .engine(SttEngine::OpenAi)
        .locale(LocaleCode::Ja)
        .vad_model_paths(VadModelPaths {
            silero: "/tmp/model.onnx".into(),
            ten: "/tmp/model.onnx".into(),
            gtcrn: String::new(),
        })
        .build();
    assert!(result.is_err());
}

#[test]
fn test_config_defaults() {
    let config = VoiceKitConfig::builder()
        .engine(SttEngine::Os)
        .locale(LocaleCode::Ja)
        .vad_model_paths(VadModelPaths {
            silero: "/tmp/silero.onnx".into(),
            ten: "/tmp/ten.onnx".into(),
            gtcrn: String::new(),
        })
        .build()
        .unwrap();

    assert_eq!(config.engine, SttEngine::Os);
    assert_eq!(config.locale, LocaleCode::Ja);
    assert_eq!(config.speech_timeout_sec, 30.0);
    assert_eq!(config.post_correction.sentence_count_threshold, 3);
}
```

### 10.2 テスト実行

```bash
cargo test                          # 全テスト
cargo test --lib                    # ユニットテストのみ
cargo test --test integration_test  # 結合テストのみ
cargo test --lib -- pipeline::      # パイプライン関連のみ
```

---

## 11. 移行ガイド（MYCUTE 側）

### 11.1 Cargo.toml

voice-kit 導入後、MYCUTE の `Cargo.toml` から以下の依存を**削除**できる：

```diff
- sherpa-rs = "0.6.8"
- sherpa-rs-sys = "0.6.8"
- async-openai = { version = "0.36.1", features = ["audio", "chat-completion"] }
- rubato = "0.16"
- lindera = { version = "2.0.1", features = ["embed-ipadic"] }
- lindera-ipadic = "2.0.0"
- hound = "3.5.1"
- rodio = "0.21.1"
- async-trait = "0.1.89"
+ voice-kit = "0.1"
```

### 11.2 削除するファイル・ディレクトリ

```
src/stt/                              → 全削除
src/tools/pseudo_asr_streamer.rs      → 削除
src/tools/vad_processor.rs            → 削除
src/tools/post_correction_processor.rs → 削除
src/tools/punctuation_machine.rs      → 削除
src/tools/resampler.rs                → 削除
src/tools/audio.rs                    → 削除
src/tools/lindera_util.rs             → 削除
src/wav/piro.wav                      → 削除
src/wav/commit.wav                    → 削除
native/swift/                         → 削除
native/cs/SpeechHelper/               → 削除
src/tools/mod.rs                      → audio, lindera_util, pseudo_asr_streamer,
                                        vad_processor, post_correction_processor,
                                        punctuation_machine, resampler の行を削除
```

### 11.3 MycuteManager の変更

```rust
// 移行前
use crate::stt::recognizer::SpeechRecognizer;

// 移行後
use voice_kit::VoiceKit;

pub struct MycuteManager {
    // 移行前
    // recognizer: SpeechRecognizer,
    // stt_settings: Option<SttSettings>,
    // lmgw_client: Arc<LmgwClient>,

    // 移行後
    voice_kit: VoiceKit,
}
```

### 11.4 設定の変換

MYCUTE の `ConfigManager` が保持する `SttSettings` を voice-kit の `VoiceKitConfig` に変換するアダプタ関数を1つ書く：

```rust
impl From<&SttSettings> for voice_kit::VoiceKitConfig {
    // または独立したヘルパー関数
}

fn stt_settings_to_voice_kit_config(
    settings: &SttSettings,
    engine: SttEngine,
    locale: LocaleCode,
    openai_config: Option<OpenAiConfig>,
) -> voice_kit::VoiceKitConfig {
    voice_kit::VoiceKitConfig::builder()
        .engine(match engine {
            SttEngine::OpenAI => voice_kit::SttEngine::OpenAi,
            SttEngine::Os => voice_kit::SttEngine::Os,
        })
        .locale(match locale {
            LocaleCode::Ja => voice_kit::LocaleCode::Ja,
            LocaleCode::En => voice_kit::LocaleCode::En,
        })
        .openai_config(openai_config)
        .vad(voice_kit::VadConfig {
            vad_type: /* VadType 変換 */,
            threshold: settings.vad_threshold,
            min_silence_duration: settings.vad_min_silence_duration,
            min_speech_duration: settings.vad_min_speech_duration,
            max_speech_duration: settings.vad_max_speech_duration,
            pre_padding_ms: settings.vad_pre_padding_ms,
            utterance_min_ms: settings.utterance_min_ms,
            num_threads: settings.num_threads,
        })
        .vad_model_paths(voice_kit::VadModelPaths {
            silero: settings.resolve_path(settings.vad_type.filename()).unwrap_or_default(),
            ten: settings.resolve_path("ten_vad.onnx").unwrap_or_default(),
            gtcrn: settings.get_denoiser_path().unwrap_or_default(),
        })
        // ... その他のフィールド
        .build()
        .expect("Failed to build VoiceKitConfig from SttSettings")
}
```

---

## 12. ライセンスと配布

### 12.1 crate のライセンス

`MIT OR Apache-2.0`（Rust エコシステムの標準デュアルライセンス）

### 12.2 依存クレートのライセンス互換性

すべての依存クレートが MIT または Apache-2.0 であり、互換性の問題なし。

### 12.3 ネイティブコードのライセンス

- **Swift (`SpeechHelper.swift`)**: macOS SDK の `Speech`, `AVFoundation` フレームワークを使用。Swift コード自体は crate のライセンスに従う。
- **C# (`SpeechHelper.cs`)**: Windows SDK の `Windows.Media.SpeechRecognition` を使用。C# コード自体は crate のライセンスに従う。

### 12.4 crates.io 公開時の注意

プリビルドライブラリ（`libspeech_helper.a` 約 2MB, `SpeechHelper.dll` 約 5MB）を含むため、`cargo publish` 時のサイズ制限（10MB）に注意。超える場合は GitHub Releases 経由の配布 + `build.rs` での自動ダウンロード方式に切り替える。

---

## 付録 A: 利用者アプリの OS 権限設定

### macOS: Info.plist

```xml
<key>NSMicrophoneUsageDescription</key>
<string>音声認識のためにマイクへのアクセスが必要です</string>
<key>NSSpeechRecognitionUsageDescription</key>
<string>音声をテキストに変換するために音声認識へのアクセスが必要です</string>
```

### Windows: Package.appxmanifest

```xml
<Capabilities>
    <DeviceCapability Name="microphone"/>
</Capabilities>
```

### Windows: 音声認識のセットアップ

エンドユーザーは「Windows 設定」→「プライバシーとセキュリティ」→「音声認識」→「オンライン音声認識」を有効にする必要がある。`VoiceKit::health_check()` が未設定を検出し、bit 1 を返す。

---

## 付録 B: 非対応OSでの動作

Linux など macOS/Windows 以外の OS では:
- `SttEngine::Os` → `VoiceKitError::UnsupportedEngine` 
- `SttEngine::OpenAi` → 動作する（HTTP API 経由のため）
- マイク入力（OpenAIモード）→ 動作しない（OSネイティブキャプチャ非対応のため）

将来的な Linux 対応（cpal/PulseAudio 経由のマイク入力）は、`AsrBackend` トレイトを実装する新バックエンド追加で対応可能。

---

## 付録 C: 実装順序

1. **crate 骨組み**: `Cargo.toml`, `build.rs`, `src/lib.rs`, `src/error.rs`
2. **公開型**: `src/types.rs`, `src/config.rs`
3. **ネイティブ FFI**: `src/native/mac_ffi.rs`, `src/native/win_ffi.rs`
4. **パイプライン基盤**: `src/pipeline/resampler.rs`, `src/pipeline/vad.rs`, `src/pipeline/denoiser.rs`, `src/pipeline/signal_filter.rs`
5. **パイプライン補正**: `src/pipeline/post_correct.rs`, `src/pipeline/punctuation.rs`
6. **パイプライン統合**: `src/pipeline/streamer.rs` (`AsrBackend` トレイト, `PseudoAsrStreamer`)
7. **バックエンド**: `src/backends/openai.rs`, `src/backends/mac.rs`, `src/backends/win.rs`
8. **認識器統括**: `src/recognizer.rs`
9. **公開 API**: `src/voice_kit.rs`, `src/audio.rs`, `src/lindera_util.rs`
10. **テスト**: 各モジュール `#[cfg(test)]` + `tests/integration_test.rs`
11. **ドキュメント**: `README.md`

各ステップで `cargo check` を実行し、コンパイルが通ることを確認しながら進める。

---

## 付録 D: SttSettings の全フィールドとデフォルト値（MYCUTE からのリファレンス）

実装者が MYCUTE の `SttSettings` の型とデフォルト値を参照する必要がある場合のために、以下に完全な定義を記載する。

```rust
// MYCUTE src/mycute_settings.rs より（参考）

pub struct SttSettings {
    pub model_dir: Option<String>,         // ~/.mycute/models (ConfigManager で設定)
    pub num_threads: i32,                  // デフォルト 4

    pub vad_type: VadType,                 // デフォルト SileroInt8
    pub vad_model_path: Option<String>,    // None → vad_type.filename() から解決
    pub vad_threshold: f32,                // デフォルト 0.5
    pub vad_min_silence_duration: f32,     // デフォルト 0.2
    pub vad_min_speech_duration: f32,      // デフォルト 0.25
    pub vad_max_speech_duration: f32,      // デフォルト 25.0
    pub vad_pre_padding_ms: u64,           // デフォルト 100
    pub utterance_min_ms: u64,             // デフォルト 300
    pub window_max_ms: u64,                // デフォルト 10000
    pub use_punctuation: bool,             // デフォルト true
    pub use_script_filter: bool,           // デフォルト true
    pub use_denoiser: bool,                // デフォルト true
    pub denoiser_model_path: String,       // デフォルト "gtcrn.onnx"
    pub fuzzy_threshold: f32,              // デフォルト 0.8

    pub signal_check_enabled: Option<bool>,       // デフォルト None (→ true)
    pub signal_rms_threshold: Option<f32>,        // デフォルト None (→ 0.005)
    pub signal_occupancy_ratio: Option<f32>,      // デフォルト None (→ 0.15)

    pub post_correction_sentence_count_threshold: usize, // デフォルト 3
    pub post_correction_min_text_length: usize,          // デフォルト 10
    pub post_correction_interval_ms: u64,                // デフォルト 2000
}
```

---

## 付録 E: Swift SpeechHelper.swift の C FFI インターフェース仕様

以下は Swift 側が Rust に公開している C FFI の完全なシグネチャ一覧である。実装者はこれと一致するように `src/native/mac_ffi.rs` を記述すること。

```
// 初期化・権限
int32_t speech_helper_init(double speech_timeout_sec);
int32_t speech_helper_request_authorization();

// コールバック設定
void speech_helper_set_result_callback(void (*)(const char* text, int32_t is_final));
void speech_helper_set_error_callback(void (*)(const char* error));
void speech_helper_set_ready_callback(void (*)());
void speech_helper_set_audio_data_callback(void (*)(const float* samples, int32_t count, int32_t sample_rate));

// オーディオキャプチャ (OpenAI モード用)
int32_t speech_helper_start_capture();
void speech_helper_stop_capture();

// 音声認識セッション (Classic / Tahoe)
int32_t speech_helper_start(const char* locale);
void speech_helper_stop();
void speech_helper_cleanup();
void speech_helper_tick();

// Tahoe 専用 (macOS 15+)
int32_t tahoe_helper_init(const char* locale, double speech_timeout_sec);
int32_t tahoe_helper_start(const char* locale);
void tahoe_helper_stop();
```

## 付録 F: C# SpeechHelper.cs の C FFI インターフェース仕様

以下は C# 側が Rust に公開している `UnmanagedCallersOnly` の完全なエントリポイント一覧である。

```
// 初期化・ヘルスチェック
int speech_helper_init(double speechTimeoutSec);
int speech_helper_check_health();  // 戻り値: ビットマスク (bit0=モデル, bit1=プライバシー, bit2=マイク)

// コールバック設定
void speech_helper_set_result_callback(void (*)(const char* text, int isFinal));
void speech_helper_set_error_callback(void (*)(const char* error));
void speech_helper_set_ready_callback(void (*)());
void speech_helper_set_audio_data_callback(void (*)(const float* samples, uint count, uint sampleRate));

// オーディオキャプチャ
int speech_helper_start_capture();
void speech_helper_stop_capture();

// 音声認識セッション
int speech_helper_start(const char* locale);
void speech_helper_stop();
void speech_helper_cleanup();
void speech_helper_tick();

// IME 制御 (Windows 固有)
void speech_helper_disable_ime();
void speech_helper_restore_ime();
```
