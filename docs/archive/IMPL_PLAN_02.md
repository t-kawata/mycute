# ReazonSpeech-K2-V2 疑似ストリーミング ASR 実装計画

> **目的**: 現在の `sherpa` エンジン（ストリーミングモデル）を `sherpa01` にリネーム（完了）し、新しい `sherpa02` エンジン（ReazonSpeech-K2-V2 疑似ストリーミングモデル）を追加する。

---

## 1. 問題意識

現在使用している `sherpa-onnx-streaming-zipformer-ar_en_id_ja_ru_th_vi_zh-2025-02-10` は、非常に精度が低い。特に日本語の精度は低くて、時々中国語が混ざってしまう。よって真のリアルタイムストリーミング音声認識にはならないが、`reazonspeech-k2-v2 / sherpa-onnx-zipformer-ja-reazonspeech` を使用することにより、疑似ストリーミング音声認識を実装することにする。

---

## 2. 現在の実装状況

### 2.1 ファイル構成

```
src/
├── config.rs          # SttEngine enum, SherpaSettings 定義
├── main.rs            # SpeechRecognizer 初期化、イベントループ
├── stt/
│   ├── mod.rs         # モジュールエクスポート
│   ├── recognizer.rs  # SpeechRecognizer（エンジン切り替えロジック）
│   ├── sherpa.rs      # SherpaRecognizer（現在のストリーミング実装）
│   ├── resampler.rs   # オーディオリサンプラー
│   └── punctuation.rs # 句読点挿入
└── ui/
    └── settings.rs    # 設定 UI（エンジン選択ラジオボタン）
```

### 2.2 現在の SttEngine enum

```rust
// src/config.rs
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum SttEngine {
    Sherpa,    // ← これを Sherpa01 にリネーム
    Tahoe,
    #[default]
    Classic,
}
```

### 2.3 現在の SherpaSettings 構造体

```rust
// src/config.rs
pub struct SherpaSettings {
    pub model_dir: Option<String>,
    pub encoder: String,      // "encoder-epoch-75-avg-11-chunk-16-left-128.int8.onnx"
    pub decoder: String,      // "decoder-epoch-75-avg-11-chunk-16-left-128.onnx"
    pub joiner: String,       // "joiner-epoch-75-avg-11-chunk-16-left-128.int8.onnx"
    pub tokens: String,       // "tokens.txt"
    pub bpe_model: Option<String>,
    pub num_threads: i32,
    pub use_coreml: bool,
    pub decoding_method: String,
    pub max_active_paths: i32,
    pub use_vad: bool,
    pub vad_type: String,     // "silero_int8"
    pub vad_model_path: Option<String>,
    pub vad_threshold: f32,
    pub vad_min_silence_duration: f32,
    pub vad_min_speech_duration: f32,
    pub use_punctuation: bool,
    pub use_script_filter: bool,
    pub use_structure_filter: bool,
}
```

### 2.4 現在の SherpaRecognizer（ストリーミング）

- `sherpa-rs` の `sys` モジュールを使用（`SherpaOnnxOnlineRecognizer`, `SherpaOnnxOnlineStream`）
- **ストリーミング認識**: 音声を受け取りながらリアルタイムでテキストを生成
- VAD は `SherpaOnnxVoiceActivityDetector` を使用
- 問題: 多言語モデルのため、日本語の精度が低く中国語が混入

### 2.5 現在の settings.json 構造

```json
{
  "stt_engine": "sherpa",
  "sherpa": {
    "model_dir": "/Users/kawata/shyme/mycute/models",
    "encoder": "encoder-epoch-75-avg-11-chunk-16-left-128.int8.onnx",
    "decoder": "decoder-epoch-75-avg-11-chunk-16-left-128.onnx",
    ...
  }
}
```

---

## 3. 変更計画

### 3.1 設定値のリネーム

| 現在の値 | 新しい値 | 説明 |
|----------|----------|------|
| `sherpa` | `sherpa01` | 既存のストリーミングモデル |
| N/A | `sherpa02` | 新規: ReazonSpeech-K2-V2 疑似ストリーミング |

### 3.2 新しい SttEngine enum

```rust
// src/config.rs
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum SttEngine {
    Sherpa01,  // 既存のストリーミングモデル
    Sherpa02,  // 新規: ReazonSpeech-K2-V2 疑似ストリーミング
    Tahoe,
    #[default]
    Classic,
}
```

### 3.3 新しい Sherpa02Settings 構造体

```rust
// src/config.rs
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Sherpa02Settings {
    pub model_dir: Option<String>,
    // ReazonSpeech-K2-V2 モデル精度選択
    // true = INT8 量子化モデル（軽量・高速）, false = FP32 通常モデル（高精度）
    pub use_quantized: bool,
    // モデルファイル名は use_quantized に基づいて自動決定
    pub tokens: String,       // "tokens.txt"
    pub num_threads: i32,
    
    // VAD 設定（疑似ストリーミングに必須）
    // vad_type: "silero", "silero_int8", "ten", "ten_int8" のいずれか（現在の sherpa と同様）
    pub vad_type: String,
    pub vad_model_path: Option<String>,  // カスタムパス（オプション）
    pub vad_threshold: f32,
    pub vad_min_silence_duration: f32,
    pub vad_min_speech_duration: f32,
    pub vad_max_speech_duration: f32,
    
    // 発話区間バッファリング設定
    // silence_tolerance: 発話中に何ms分の無音を許容するか（息継ぎで分断されない）
    pub vad_silence_tolerance_ms: u64,
    // pre_padding: 発話区間の前に何msの音声を付加するか
    pub vad_pre_padding_ms: u64,
    // post_padding: 発話区間の後に何msの音声を付加するか
    pub vad_post_padding_ms: u64,
    
    // 窓（ウィンドウ）管理設定
    // window_min_ms: 認識対象とする窓の最低長さ（サンプル数で計算）
    pub window_min_ms: u64,
    // window_max_ms: 窓の最大長さ（モデルの限界を超えないよう制限）
    pub window_max_ms: u64,
    
    // 中間結果更新間隔（ミリ秒）
    pub interim_interval_ms: u64,
    pub use_punctuation: bool,
    pub use_script_filter: bool,
}
```

### 3.4 新しい settings.json 構造

```json
{
  "stt_engine": "sherpa02",
  "sherpa": {
    // 既存のストリーミングモデル設定（sherpa01 用）
    "model_dir": "/Users/kawata/shyme/mycute/models",
    "encoder": "encoder-epoch-75-avg-11-chunk-16-left-128.int8.onnx",
    ...
  },
  "sherpa02": {
    // 新規: ReazonSpeech-K2-V2 疑似ストリーミング設定
    "model_dir": "/Users/kawata/shyme/mycute/models/sherpa-onnx-zipformer-ja-reazonspeech-2024-08-01",
    "encoder": "encoder-epoch-99-avg-1.int8.onnx",
    "decoder": "decoder-epoch-99-avg-1.onnx",
    "joiner": "joiner-epoch-99-avg-1.int8.onnx",
    "tokens": "tokens.txt",
    "num_threads": 4,
    // VAD 設定（silero, silero_int8, ten, ten_int8 のいずれか）
    "vad_type": "silero_int8",
    "vad_model_path": null,
    "vad_threshold": 0.5,
    "vad_min_silence_duration": 0.5,
    "vad_min_speech_duration": 0.3,
    "vad_max_speech_duration": 20.0,
    // 発話区間バッファリング設定
    "vad_silence_tolerance_ms": 500,
    "vad_pre_padding_ms": 200,
    "vad_post_padding_ms": 200,
    // 窓管理（最低5秒蓄積してから認識）
    "window_min_ms": 5000,
    "window_max_ms": 25000,
    // 中間結果
    "interim_interval_ms": 500,
    "use_punctuation": true,
    "use_script_filter": true
  }
}
```

---

## 4. 実装するファイル

### 4.1 [MODIFY] config.rs

1. `SttEngine` enum: `Sherpa` → `Sherpa01` にリネーム、`Sherpa02` を追加
2. `Sherpa02Settings` 構造体を新規追加
3. `Settings` 構造体: `sherpa02: Sherpa02Settings` フィールドを追加
4. 各種デフォルト関数を追加

### 4.2 [NEW] src/stt/sherpa02.rs

新しい疑似ストリーミング認識器を実装:

```rust
pub struct Sherpa02Recognizer {
    tx: mpsc::Sender<SttEvent>,
    is_running: Arc<AtomicBool>,
    audio_buffer: Arc<Mutex<Vec<f32>>>,
    settings: Sherpa02Settings,
    input_sample_rate: u32,
    resampler: Option<Box<dyn AudioResampler>>,
    // 認識器とVAD
    recognizer: Option<TransducerRecognizer>,
    vad: Option<Box<dyn Sherpa02Vad>>, // 多種モデル対応の抽象化
    
    // ウィンドウ・チャンク管理
    chunk_queue: VecDeque<Chunk>,
    ring_buffer: VecDeque<f32>, // pre_padding用
    
    // 状態管理
    was_speech: bool,
    silence_samples: usize,
    last_window_text: String, // 差分検証用
    
    current_locale: LocaleCode,
    filler_words: Vec<String>,
    use_stop_word_filter: bool,
    punctuation_inserter: Option<PunctuationInserter>,
}
```

主要メソッド:
- `new()` - 初期化
- `init_audio()` - cpal オーディオ設定
- `init_resampler()` - リサンプラー設定
- `init_recognizer()` - TransducerRecognizer と SileroVad 初期化
- `start()` - 認識開始
- `stop()` - 認識停止
- `tick()` - メインループから呼ばれる処理

### 4.3 [MODIFY] src/stt/mod.rs

```rust
pub mod punctuation;
pub mod recognizer;
pub mod resampler;
pub mod sherpa;
pub mod sherpa02;  // 新規追加

pub use recognizer::SpeechRecognizer;
```

### 4.4 [MODIFY] src/stt/recognizer.rs

エンジン切り替えロジックを更新:

```rust
// 変更前
let sherpa_backend = if engine == SttEngine::Sherpa { ... }

// 変更後
let sherpa_backend = if engine == SttEngine::Sherpa01 { ... }
let sherpa02_backend = if engine == SttEngine::Sherpa02 { ... }
```

`start()`, `stop()`, `tick()`, `set_locale()` 等で `Sherpa02` のハンドリングを追加。

### 4.5 [MODIFY] src/ui/settings.rs

設定 UI のラジオボタンを更新:

```rust
// 変更前
ui.radio_value(&mut draft.stt_engine, SttEngine::Sherpa, "Sherpa (ReazonSpeech)")

// 変更後
ui.radio_value(&mut draft.stt_engine, SttEngine::Sherpa01, "Sherpa01 (Streaming)")
ui.radio_value(&mut draft.stt_engine, SttEngine::Sherpa02, "Sherpa02 (ReazonSpeech)")
```

### 4.6 [MODIFY] settings.json

既存の `stt_engine: "sherpa"` を `stt_engine: "sherpa01"` に変更し、`sherpa02` ブロックを追加。

### 4.7 モデルダウンロード

```bash
# ReazonSpeech-K2-V2 モデルのダウンロード
cd /Users/kawata/shyme/mycute/models
wget https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-zipformer-ja-reazonspeech-2024-08-01.tar.bz2
tar xvf sherpa-onnx-zipformer-ja-reazonspeech-2024-08-01.tar.bz2
rm sherpa-onnx-zipformer-ja-reazonspeech-2024-08-01.tar.bz2

# Silero VAD モデル（まだなければ）
wget https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/silero_vad.onnx
```

---

## 5. 疑似ストリーミング ASR 詳細設計

### 5.1 アーキテクチャ比較

| 項目 | Sherpa01（現在） | Sherpa02（新規） |
|------|------------------|------------------|
| 認識器 | `SherpaOnnxOnlineRecognizer` | `sherpa_rs::TransducerRecognizer` |
| モデル | 多言語ストリーミング Zipformer | ReazonSpeech-K2-V2 (日本語専用) |
| 認識方式 | フレームごとにリアルタイム | VAD 区間ごとにオフライン認識 |
| 中間結果 | 常時更新 | `interim_interval_ms` 間隔で更新 |
| 精度 | 低い（中国語混入あり） | 高い（日本語特化） |
| レイテンシ | 低い | やや高い（VAD 遅延あり） |

### 5.2 処理フロー (1チャンク・スライディングウィンドウ)

```
[マイク入力 (cpal)]
       │
       ▼
[audio_buffer に蓄積]
       │
       ▼
[リサンプリング → 16kHz]
       │
       ├──── [ring_buffer に追加（pre_padding 用）]
       │
       ▼
[VAD に渡す (SileroVad / TenVad)]
       │
       ├── is_speech() == true ─────────────────────┐
       │                                           │
       │           [チャンク蓄積 (ChunkQueue)]        │
       │           [現在のチャンクが終了するまで待機]     │
       │                                           │
       ├── is_speech() == false ────────────────────┤
       │     (silence_tolerance_ms 経過後)           │
       │                                           │
       │           [新規チャンクを ChunkQueue に追加]    │
       │                                           │
       ▼                                           │
[窓（ウィンドウ）の形成]                                 │
       │                                           │
       ├── [A. 初期フェーズ：蓄積量 < window_min_ms]       │
       │     [すべての蓄積チャンクを結合して窓とする]      │
       │     (低レイテンシ重視：話し始めから逐次表示)      │
       │                                           │
       └── [B. 定常フェーズ：蓄積量 >= window_min_ms]      │
             [最新の window_min_ms 分の音声を抜き出す]     │
             (高精度重視：文脈を維持しつつスライド)        │
       │                                           │
       ▼                                           │
[音声認識実行 (TransducerRecognizer)]                  │
       │                                           │
       ▼                                           │
[結果の差分検証 (Differential Merge)]                  │
       │                                           │
       ├─ [前回認識した窓の結果と重複部分を比較]          │
       │  [確信できる接頭辞（Confirmed Prefix）を特定]    │
       │                                           │
       ▼                                           │
[IsFinal の判定]                                      │
       │                                           │
       ├─ [Confirmed Prefix 内に「。」「？」があるか？]  │
       │                                           │
       ├── Yes ── [FinalResult 送信] ────────────────┤
       │          [確定したチャンクを Queue から削除]     │
       │          [スライディング：次のチャンクへ]         │
       │                                           │
       └── No ─── [PartialResult 送信] ──────────────┤
                  [次のチャンクを待機、または]           │
                  [窓を1チャンク分スライドして再認識]       │
                                                   │
       └───────────────────────────────────────────┘
```

### 5.3 差分検証 (Differential Merge) ロジック

1チャンクずつスライドさせながら認識を繰り返すため、認識結果のテキストを合理的に比較・結合する必要があります。

1.  **二相性（Biphasic）ウィンドウ管理**:
    - **初期蓄積フェーズ**: 最初の 5秒（`window_min_ms`）が溜まるまでは、チャンクが増えるたびに「0秒〜現在」の全データを認識し、中間結果として表示します。これにより入力開始直後の空白時間を排除します。
    - **スライディングフェーズ**: 蓄積量が 5秒を超えた後は、常に「現在から 5秒前まで」の窓を維持し、1チャンクずつ前進させます。
2.  **最長共通接頭辞 (LCP) の特定**: 
    前回認識した「窓 N」の結果と、今回の「窓 N+1」の結果を比較し、意味的に連続する部分を特定します。
3.  **確定区間の更新**: 
    前回の結果から変化していない部分は「確定」として扱い、変化があった部分以降は「暫定（Partial）」として維持します。
4.  **スライディングと削除**: 
    「。」が検出されて `IsFinal` が送られた場合のみ、確定したテキストに対応する音声チャンクを `ChunkQueue` から削除します。

---

## 6. sherpa-rs API（実際の使用例）

```rust
use sherpa_rs::silero_vad::{SileroVad, SileroVadConfig};
use sherpa_rs::transducer::{TransducerConfig, TransducerRecognizer};

// VAD 初期化
let vad_config = SileroVadConfig {
    model: "models/silero_vad.onnx".to_string(),
    min_silence_duration: 0.5,
    min_speech_duration: 0.3,
    max_speech_duration: 20.0,
    threshold: 0.5,
    sample_rate: 16000,
    window_size: 512,
    provider: None,
    num_threads: Some(2),
    debug: false,
};
let mut vad = SileroVad::new(vad_config, 30.0)?;

// 認識器初期化
let recognizer_config = TransducerConfig {
    encoder: "models/sherpa-onnx-zipformer-ja-reazonspeech-2024-08-01/encoder-epoch-99-avg-1.int8.onnx".to_string(),
    decoder: "models/sherpa-onnx-zipformer-ja-reazonspeech-2024-08-01/decoder-epoch-99-avg-1.onnx".to_string(),
    joiner: "models/sherpa-onnx-zipformer-ja-reazonspeech-2024-08-01/joiner-epoch-99-avg-1.int8.onnx".to_string(),
    tokens: "models/sherpa-onnx-zipformer-ja-reazonspeech-2024-08-01/tokens.txt".to_string(),
    num_threads: 4,
    sample_rate: 16000,
    feature_dim: 80,
    decoding_method: "greedy_search".to_string(),
    ..Default::default()
};
let mut recognizer = TransducerRecognizer::new(recognizer_config)?;

// 認識実行
let text = recognizer.transcribe(16000, &samples);
```

### 5.4 SileroVad メソッド一覧

| メソッド | シグネチャ | 説明 |
|---------|-----------|------|
| `new` | `(config, buffer_secs) -> Result<Self>` | インスタンス作成 |
| `accept_waveform` | `(&mut self, samples: Vec<f32>)` | 音声データを受け入れ |
| `is_speech` | `(&mut self) -> bool` | 現在発話中か |
| `is_empty` | `(&mut self) -> bool` | セグメントキューが空か |
| `front` | `(&mut self) -> SpeechSegment` | 先頭セグメント取得 |
| `pop` | `(&mut self)` | 先頭セグメント削除 |
| `flush` | `(&mut self)` | 強制フラッシュ |
| `clear` | `(&mut self)` | 状態リセット |

### 5.5 TransducerRecognizer メソッド一覧

| メソッド | シグネチャ | 説明 |
|---------|-----------|------|
| `new` | `(config) -> Result<Self>` | インスタンス作成 |
| `transcribe` | `(&mut self, sample_rate: u32, samples: &[f32]) -> String` | 音声をテキストに変換 |

---

## 6. ReazonSpeech-K2-V2 モデル詳細

### 6.1 モデル仕様

| 項目 | 値 |
|------|-----|
| **モデル名** | `reazonspeech-k2-v2` / `sherpa-onnx-zipformer-ja-reazonspeech-2024-08-01` |
| **アーキテクチャ** | Character-based RNN-T (Recurrent Neural Network Transducer) |
| **エンコーダ** | Zipformer (Enhanced Transformer) |
| **パラメータ数** | 159.34M (1億5934万) |
| **学習データ** | ReazonSpeech v2.0 コーパス (35,000時間) |
| **対応言語** | 日本語のみ |
| **最大処理長** | 約30秒の音声クリップ |
| **ライセンス** | Apache License 2.0 |

### 6.2 モデルファイル構成

```
sherpa-onnx-zipformer-ja-reazonspeech-2024-08-01/
├── encoder-epoch-99-avg-1.onnx       # FP32 エンコーダ (565MB)
├── encoder-epoch-99-avg-1.int8.onnx  # INT8 量子化エンコーダ (148MB) ← 推奨
├── decoder-epoch-99-avg-1.onnx       # FP32 デコーダ (11MB)
├── decoder-epoch-99-avg-1.int8.onnx  # INT8 量子化デコーダ (2.8MB)
├── joiner-epoch-99-avg-1.onnx        # FP32 ジョイナー (10MB)
├── joiner-epoch-99-avg-1.int8.onnx   # INT8 量子化ジョイナー (2.6MB) ← 推奨
├── tokens.txt                         # トークン辞書 (45KB)
└── test_wavs/                         # テスト用音声ファイル
```

### 6.3 推奨 VAD パラメータ

| パラメータ | 推奨値 | 説明 |
|-----------|--------|------|
| `vad_type` | `silero_int8` | `silero`, `silero_int8`, `ten`, `ten_int8` のいずれか |
| `vad_threshold` | 0.5 | 標準的な値。静かな環境なら 0.3〜0.4 |
| `vad_min_silence_duration` | 0.5秒 | VAD が無音と判定する最小時間 |
| `vad_min_speech_duration` | 0.3秒 | VAD が発話と判定する最小時間 |
| `vad_max_speech_duration` | 20.0秒 | モデルの最大処理長は約30秒 |
| `vad_silence_tolerance_ms` | 500ms | 発話中の息継ぎを分断しないための無音許容時間 |
| `vad_pre_padding_ms` | 200ms | 発話区間の前に付加する音声（単語の頭が切れない） |
| `vad_post_padding_ms` | 200ms | 発話区間の後に付加する音声（単語の末尾が切れない） |
| `window_min_ms` | 5000ms | 認識ループを開始するまでの最低蓄積時間 |
| `interim_interval_ms` | 500ms | 短すぎると CPU 負荷増大（300〜700ms が現実的） |

---

## 7. 実装時の注意点

### 7.1 sherpa-rs vs 現在の実装

現在の `sherpa.rs` は `sherpa-onnx-sys` の raw C FFI を直接使用しているが、新しい `sherpa02.rs` は `sherpa-rs` の高レベル Rust API を使用する:

```rust
// 現在の sherpa.rs（Low-level FFI）
use sherpa_onnx_sys as sys;
let recognizer = sys::SherpaOnnxCreateOnlineRecognizer(&config);

// 新しい sherpa02.rs（High-level API）
use sherpa_rs::transducer::TransducerRecognizer;
let recognizer = TransducerRecognizer::new(config)?;
```

### 7.2 Cargo.toml への依存追加

```toml
[dependencies]
sherpa-rs = { version = "0.6", features = ["download-binaries"] }
```

### 7.3 スレッド安全性

`TransducerRecognizer` と `SileroVad` は `Send + Sync` を実装しているため、マルチスレッド環境で安全に使用できる。

### 7.4 発話区間の状態管理

`sherpa-rs` の `SileroVad` には `just_ended_speech()` のような API がないため、状態遷移を手動で追跡する。ただし、`is_speech() == false` になったら即座に発話終了と判定するのではなく、`silence_tolerance_ms` 相当のサンプル数が経過するまで待つ。

> **重要**: 経過時間の判定には `Instant::now()` などのシステムタイムスタンプを使用してはならない。音声データのサンプル数に基づいて計算する必要がある。16kHz の場合、1秒 = 16000 サンプル。

```rust
const SAMPLE_RATE: u32 = 16000;

let mut was_speech = false;
let mut silence_samples: usize = 0;  // 無音が連続したサンプル数

// 設定値をサンプル数に変換
let silence_tolerance_samples = (settings.vad_silence_tolerance_ms as usize * SAMPLE_RATE as usize) / 1000;
let pre_padding_samples = (settings.vad_pre_padding_ms as usize * SAMPLE_RATE as usize) / 1000;
let post_padding_samples = (settings.vad_post_padding_ms as usize * SAMPLE_RATE as usize) / 1000;

loop {
    // samples: 今回処理する音声データ（Vec<f32>）
    let is_speech = vad.is_speech();
    
    if is_speech {
        // 発話中: バッファに音声を蓄積
        silence_samples = 0;  // 無音カウンタリセット
        current_chunk.extend(&samples);
    } else if was_speech || silence_samples > 0 {
        // 無音に変わった（または無音継続中）が、すぐには終了と判定しない
        silence_samples += samples.len();
        
        // 無音の間もバッファに含める（silence_tolerance 内）
        current_chunk.extend(&samples);
        
        if silence_samples >= silence_tolerance_samples {
            // silence_tolerance 相当のサンプル数経過 → 発話終了と判定
            // post_padding 分は既に current_chunk に含まれている
            let text = recognizer.transcribe(SAMPLE_RATE, &current_chunk);
            // FINAL 結果送信
            current_chunk.clear();
            silence_samples = 0;
        }
    }
    
    was_speech = is_speech;
}
```

**ポイント**:
- `silence_samples`: 無音が連続したサンプル数をカウント（システム時刻ではない）
- 設定値 (ms) は `(ms * SAMPLE_RATE) / 1000` でサンプル数に変換
- `silence_tolerance` 内の無音も `current_chunk` に含めることで、息継ぎや短いポーズで発話が分断されない

---

## 8. 参考情報

| リソース | URL |
|---------|-----|
| k2-fsa/sherpa-onnx | https://github.com/k2-fsa/sherpa-onnx |
| thewh1teagle/sherpa-rs | https://github.com/thewh1teagle/sherpa-rs |
| sherpa-rs docs.rs | https://docs.rs/sherpa_rs |
| ReazonSpeech | https://github.com/reazon-research/ReazonSpeech |
| reazonspeech-k2-v2 (Hugging Face) | https://huggingface.co/reazon-research/reazonspeech-k2-v2 |
| Zipformer 論文 | https://arxiv.org/abs/2310.11230 |
| ReazonSpeech 論文 | https://research.reazon.jp/_static/reazonspeech_nlp2023.pdf |

---

## 9. 検証計画

### 9.1 ユニットテスト（既存）

```bash
# 既存テストの実行
cargo test --all-targets
```

現在の `sherpa.rs` には以下のテストが存在:
- `test_is_hallucination_script`
- `test_sherpa_settings_validation`

### 9.2 手動検証

1. **sherpa01 互換性テスト**
   - `settings.json` の `stt_engine` を `sherpa01` に設定
   - `make run` でアプリを起動
   - Option+S で音声入力を開始
   - 日本語を話して認識結果を確認
   - 既存動作と同じであることを確認

2. **sherpa02 新機能テスト**
   - モデルをダウンロード（上記コマンド参照）
   - `settings.json` の `stt_engine` を `sherpa02` に設定
   - `sherpa02` ブロックを追加（上記 JSON 参照）
   - `make run` でアプリを起動
   - Option+S で音声入力を開始
   - 日本語を話して認識結果を確認
   - 中間結果が 500ms 間隔で更新されることを確認
   - 発話終了後に最終結果が出力されることを確認

3. **設定 UI テスト**
   - Option+J で設定画面を開く
   - 「Sherpa01 (Streaming)」と「Sherpa02 (ReazonSpeech)」が表示されることを確認
   - エンジン切り替えが正常に動作することを確認

## 10. 実装完了までの52ステップ（詳細サブステップ付き）

本実装は極めて慎重に進める必要があるため、以下の各ステップをさらに細分化し、一歩ずつ確実に進めます。

### フェーズ1：準備と環境構築
- [x] 1. `wavs` ディレクトリの作成
    - [x] プロジェクトルート直下に `wavs/` ディレクトリを作成する
    - [x] `.gitignore` に `wavs/` を追加し、バイナリファイルがコミットされないようにする
    - [x] 書き込み権限があることを確認する
- [x] 2. モデルファイルの配置確認
    - [x] `models/sherpa-onnx-zipformer-ja-reazonspeech-2024-08-01/` 内に `encoder`, `decoder`, `joiner`, `tokens` があることを確認
    - [x] `models/` 内に `silero_vad.onnx`, `ten_vad.onnx` 等の VAD モデル (4種) があることを確認
    - [x] `settings.json` のパス指定と実際のパスが一致しているか確認
- [x] 3. `Cargo.toml` への依存追加
    - [x] `sherpa-rs` の最新安定版 (0.6以上) を `[dependencies]` に追加
    - [x] `hound` (WAVファイル保存・操作用) を追加
    - [x] `crossbeam-channel` (スレッド間通信のバックアップ用) が必要か検討し、適宜追加
- [x] 4. プロジェクト全体の正常性確認
    - [x] `cargo check --all-targets` を実行し、既存コードにエラーがないことを確認
    - [x] `cargo test` を実行し、既存のテストがパスすることを確認
    - [x] `Makefile` 等のビルドプロセスに影響がないか確認

### フェーズ2：設定・データ構造の定義
- [x] 5. `SttEngine` enum の更新
    - [x] `src/config.rs` の `SttEngine` に `Sherpa01`, `Sherpa02` を追加
    - [x] 既存の `Sherpa` variant を `Sherpa01` にリネーム（置換漏れに注意）
    - [x] `serde` の `rename_all = "lowercase"` との整合性を確認
- [x] 6. `Sherpa02Settings` 構造体の定義
    - [x] `src/config.rs` に `Sherpa02Settings` 構造体を新規作成
    - [x] VAD 関連フィールド (`vad_type`, `vad_threshold`, `silence_tolerance_ms` 等) を定義
    - [x] 窓管理フィールド (`window_min_ms`, `window_max_ms`) を定義
    - [x] パディングフィールド (`pre_padding_ms`, `post_padding_ms`) を定義
    - [x] **追加**: `use_quantized: bool` フィールドを追加（INT8/FP32 モデル切替用）
- [x] 7. `Settings` へのフィールド追加
    - [x] `Settings` 構造体に `pub sherpa02: Sherpa02Settings` を追加
    - [x] `ConfigManager` 等でこのフィールドが正しくシリアライズ・デシリアライズされるか確認
- [x] 8. デフォルト値の実装
    - [x] `impl Default for Sherpa02Settings` を実装
    - [x] 各パラメータに本計画書の「推奨値」を設定
    - [x] `settings.json` に未定義の場合でも安全に起動できるようにする
- [x] 9. `src/stt/sherpa02.rs` のスケルトン作成
    - [x] ファイルを新規作成し、必要な `use` 文を記述
    - [x] `Sherpa02Recognizer` 構造体を定義し、全フィールドを記述
    - [x] `recognizer.rs` の `SpeechRecognizer` トレイト（または同等のインターフェース）に準拠させる準備
- [x] 10. `Chunk` 構造体の定義
    - [x] 音声データ `Vec<f32>`、サンプル数、タイムスタンプ（デバッグ用）を持つ `Chunk` を定義
    - [x] チャンクの接合にパディングが含まれないことを保証する設計にする
- [x] 11. `ChunkQueue` の定義
    - [x] `VecDeque<Chunk>` をラップした構造体を定義
    - [x] 合計サンプル数を定数時間で取得できる `total_samples()` メソッドを実装

> **フェーズ1 実装メモ (2026-01-13)**:
> - `hound` は `cargo add` で最新版 v3.5.1 を追加
> - ReazonSpeech-K2-V2 モデルは Hugging Face からダウンロード後、`Makefile` の `download-models` ターゲットにも追加
> - `cargo test` は Swift リンカエラーで失敗するが、`cargo check` はパス（既知の issue）

> **フェーズ2 実装メモ (2026-01-13)**:
> - `use_quantized: bool` フィールドを追加（当初計画では個別ファイル名パスだったが、ブール値で INT8/FP32 を切り替える設計に変更）
> - `get_encoder_path()`, `get_decoder_path()`, `get_joiner_path()` メソッドで `use_quantized` に基づきファイル名を自動決定
> - VAD パスは `models/` 直下を参照（ReazonSpeech モデルディレクトリとは別）
> - **追加 (アドリブ)**: デフォルト値をユーザー要件に合わせ調整 (`tolerance: 500ms`, `padding: 200ms`, `window_min: 5000ms`)

### フェーズ3：オーディオ基盤の刷新と共通化
- [x] 12. 適応型オーディオキャプチャ（16kHz 要求）の実装
    - [x] `cpal` に対して直接 16kHz をリクエストするプロトタイプを実装
    - [x] ハードウェアが 16kHz をサポートしていない場合の自動フォールバックロジック（リサンプラー有効化）を実装
- [x] 12.1 既存エンジン（`sherpa01`）への水平展開
    - [x] `src/stt/sherpa01.rs` の `init_audio` を上記の新ロジックにリファクタリング
    - [x] 16kHz で直接初期化できた場合、CPU 負荷の高いリサンプリング処理を完全にスキップすることを確認
- [x] 13. `ring_buffer` の初期化
    - [x] `pre_padding_ms` 分の音声を保持できるサイズの `VecDeque<f32>` を用意
    - [x] バッファがいっぱいになったら古いデータを捨てるロジックを実装
- [x] 14. マイク入力の流し込み
    - [x] `cpal` のコールバックから送られてくる音声を `audio_buffer` に溜める
    - [x] 同時に `ring_buffer` にも最新の音声データをコピーする
- [x] 15. VAD へのデータ渡しのタイミング調整
    - [x] VAD モデルが要求する `window_size` (例: 512) ごとにデータを切り出す
    - [x] 不足分は次回まで保持するバッファリングロジックを実装

### フェーズ4：VAD 実装 (サンプル数主導)
- [x] 16. VAD モデルの初期化
    - [x] 4種 (Silero/TEN, FP32/INT8) のモデルロードロジックを実装
    - [x] `Sherpa02Settings::get_vad_path()` を利用し、ロケールや設定に依存しないパス解決を確認
- [x] 17. `is_speech()` 判定ループ
    - [x] `tick()` で切り出された 512 サンプルのウィンドウに対し VAD を実行
    - [x] 発話開始を検知した際、`ring_buffer` (Pre-padding) を `current_chunk` の先頭にコピーする
- [x] 18. `silence_samples` カウンタの実装
    - [x] `vad.is_speech()` が `false` の間、サンプル数 (512ずつ) を加算
    - [x] カウンタリセットのタイミングを「完全に発話が途切れたと確定した後」に設定
- [x] 19. `silence_tolerance_samples` 猶予ロジック
    - [x] `silence_samples < tolerance_samples` の間は、`current_chunk` への蓄積を継続
    - [x] 文中の短いポーズでチャンクが分断されないことをログで確認
- [x] 20. チャンクの確定と切り出し
    - [x] 猶予時間を超えた場合、`current_chunk` を `Chunk` オブジェクトとして確定
    - [x] `ChunkQueue` へ追加し、`current_chunk` をクリア、カウンタをリセット
- [x] 21. `post_padding` の考慮
    - [x] 厳密には `silence_tolerance` の一部が `post_padding` として機能するが、必要に応じて末尾をトリミング/調整
- [x] 21.1 チャンク保存デバッグ用フックの追加（サンプル単位の精度確認用）
    - [x] 確定した各 `Chunk` を WAV 保存するためのデバッグ用コードを **コメントアウト状態で** `sherpa02.rs` に埋め込む。
    - [x] **重要**: 書き出すデータは、頭(Pre-padding)と尻(Post-padding)が完全に付与された「音声認識の対象となる生データそのもの」でなければならない。
    - [x] 実装時には、パディング済みの完成データが格納されている変数がどれなのかを「明確かつ確実に」示す注釈コメントを添える。
    - [x] これにより、VADの判定によって言葉の開始や終了が欠けていないか、あるいは不要な無音が長すぎないかを「耳で正確に確認」可能にする。
- [x] 22. **【確認：チャンク抽出ログ】**
    - [x] `Chunk detected: ID={}, duration={}s` のようなログを出力
    - [x] 意図した通りに「意味のあるカタマリ」でチャンクが分かれているか目視・耳で確認

### フェーズ5：二相性（Biphasic）ウィンドウと認識実行
- [x] 23. `TransducerRecognizer` の初期化
    - [x] `use_quantized` に基づき、`encoder/decoder/joiner` のパスを自動選択してロード
    - [x] オンライン認識用の `OnlineStream` ではなく、バッチ的な `transcribe` を使用する準備
- [x] 24. 窓（ウィンドウ）形成ロジックの実装
    - [x] **A. 初期蓄積フェーズ**: 合計サンプル数 < `window_min_ms` の場合
        - [x] 蓄積されている全チャンクを結合 (`flatten()`) して認識に回す（低レイテンシ）
    - [x] **B. 定常フェーズ**: 合計サンプル数 >= `window_min_ms` の場合
        - [x] 最新の `window_min_ms` 分に相当する音声データをスライスして窓とする（高精度）
- [x] 25. 認識ループの制御 (`tick`)
    - [x] `interim_interval_ms` (例: 500ms) ごとに最新の窓を認識実行
    - [x] 認識結果が空、または前回と同じ場合は後続処理をスキップ
- [x] 25.1 窓音声保存デバッグ用フックの追加（結合後の連続性確認用）
    - [x] 認識器に渡される直前の「窓（ウィンドウ）音声（複数のチャンクを結合した状態）」を WAV 保存するためのコードを **コメントアウト状態で** 追加する。
    - [x] スライディングによって切り出された結合済みデータが格納されている変数を明示し、接合部でノイズが発生していないか等を「耳で確認」できるようにする。
    - [x] 「スライディングウィンドウとして認識に回される音声の連続性を確認したい場合は、この部分を有効化する」という趣旨の技術的な注釈を添える。
- [x] 26. `transcribe` 実行と性能計測
    - [x] 重い処理になるため、`tokio::task::spawn_blocking` を使うべきか検討
    - [x] 処理時間が `interim_interval_ms` を超えないかスループットを確認
- [x] 27. **【耳と目のテスト1：単体チャンク保存】**
    - [x] `hound` を使い、生成された個々のチャンクを `wavs/chunk_N.wav` として保存
    - [x] 発話の開始（頭欠けがないか）と終了（不自然な途切れがないか）を確認
- [x] 28. **【耳と目のテスト2：窓音声保存】**
    - [x] 認識に渡した窓全体を `wavs/window_M.wav` として保存
    - [x] チャンクの接合部でプチノイズが発生していないか、連続性を確認
- [x] 29. **【確認依頼：認識品質】**
    - [x] コンソールの認識結果と WAV ファイルを提示し、日本語特化モデルの精度をユーザーにデモ
- [x] 30. 暫定結果 (PartialResult) の送信開始
    - [x] まだ差分検証 (Phase 6) 前だが、窓の結果をそのまま `PartialResult` として投げ、UIに表示されることを確認

### フェーズ6：差分検証 (Differential Merge)
- [x] 31. `last_window_text` の管理
    - [x] 前回の認識結果をストックしておく
    - [x] 窓がクリアされたタイミングでこの変数も初期化する
- [x] 32. `diff_matching` 関数のプロトタイプ
    - [x] 文字列 A (前回) と B (今回) の重複部分を探索する
    - [x] 文頭が変化していないかを検証
- [x] 33. LCP (Longest Common Prefix) 実装
    - [x] 文字列の先頭から一致する最大長を計算
    - [x] 漢字・ひらがなの揺らぎを考慮するか検討（まずは完全一致で実装）
- [x] 34. 確定・暫定判定
    - [x] 一致した接頭辞を「確定（Confirmed）」とする
    - [x] 一致しなかった後続部分を「暫定（Partial）」としてバッファ
- [x] 35. 1チャンク・スライディング
    - [x] 認識が終わるごとに、キューの先頭（最も古い）チャンクをポップする
    - [x] 次のチャンクが来るまで待機し、新しい窓を形成する
- [x] 36. 重複フィルタリング
    - [x] 既に送信済みの「確定」文字列が、重複して画面に出ないよう送信内容を調整

### フェーズ7：IsFinal (句読点) 判断ロジック
- [x] 37. `PunctuationInserter` の初期化
    - [x] 現在のロケールに合わせて句読点挿入器をインスタンス化
- [x] 38. テキストへの句読点適用
    - [x] 認識結果の文字列を `insert_punctuation` に通す
- [x] 39. 特定文字（。 ？）の検索
    - [x] 処理後の文字列に `。` または `？` が含まれているかチェック
- [x] 40. 確定チャンクの完全消去
    - [x] 文末が検出された場合、その文に含まれる全チャンクを `ChunkQueue` から削除
    - [x] `last_window_text` もクリアし、新しい文に備える
- [x] 41. `FinalResult` イベントの送信
    - [x] 文末までの確定文字列を `SttEvent::FinalResult` として `tx` に送る
- [x] 42. `PartialResult` の維持
    - [x] 文末がない場合は、最新の全体像を `PartialResult` として送る

### フェーズ7.5：表記揺れ考慮（Lindera導入）
> [!NOTE]
> このフェーズはフェーズ7完了時点で、漢字・ひらがなの表記揺れによるバックスペースの頻発が確認され、改善が「必要」と判断された場合のみ実施します。

- [ ] 41.5. 形態素解析による「読み」の抽出
    - [ ] `Lindera` を用いて認識結果を形態素解析し、各単語の読み（かな）を取得する
- [ ] 41.6. 読みベースの LCP 計算
    - [ ] 表記が異なっても読みが一致する場合、それを同一語としてマージするロジックを実装
- [ ] 41.7. 揺れ補正による入力の安定化検証
    - [ ] 「明日」から「あした」に変換が変わっても、タイピングの打ち直しが発生しないことを確認

### フェーズ8：SpeechRecognizer 統合
- [ ] 43. モジュール登録
    - [ ] `src/stt/mod.rs` に `pub mod sherpa02;` を追加。
- [ ] 44. `recognizer.rs` のコンストラクタ拡張
    - [ ] `SttEngine` の値を見て `Sherpa02Recognizer` を作成する Match 腕を追加
- [ ] 45. トレイトメソッドの実装
    - [ ] `start()`, `stop()`, `tick()`, `cleanup()` をインターフェースに合わせて実装
- [ ] 46. ロケール同期
    - [ ] `set_locale` が呼ばれた際、内部の `current_locale` や句読点挿入器を更新
- [ ] 47. **【最終ログ確認】**
    - [ ] キーボード入力を無効化した状態で、音声入力を行い、完璧な文章がコンソールに出るか確認

### フェーズ9：UI と最終統合
- [ ] 48. 設定 UI 拡張
    - [ ] `src/ui/settings.rs` に `Sherpa02` 用のラジオボタンを追加
    - [ ] VAD 種類、窓サイズ、パディング値等を微調整できるスライダー/コンボボックスを追加
- [ ] 49. デフォルト設定の保存
    - [ ] 初回起動時に `sherpa02` の設定項目が `settings.json` に書き出されるか確認
- [ ] 50. タイピング機能の有効化
    - [ ] `main.rs` のイベントループで、`Sherpa02` からのイベントに基づいて `KeyboardInjector` を叩く
- [ ] 51. パフォーマンステスト
    - [ ] CPU使用率・メモリ使用量が安定しているか確認
    - [ ] 特に長時間（1時間以上）のアイドル/認識を繰り返し、リークがないか監視
- [ ] 52. 納品完了確認
    - [ ] 全てのステップが [x] になっていることを確認
    - [ ] `walkthrough.md` に最終結果とデモ（ログ/WAV）をまとめて記録