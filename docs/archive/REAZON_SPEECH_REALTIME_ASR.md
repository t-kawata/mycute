2026年1月現在、Rustで「日本語と英語のバイリンガルのリアルタイム音声認識」を実装するための最短かつ最高性能な構成をまとめます。

使用するモデルは、多言語対応で真のストリーミングが可能な **`sherpa-onnx-streaming-zipformer-ar_en_id_ja_ru_th_vi_zh-2025-02-10`** です。

### 1. 必要なリソースの準備
まず、Hugging Faceから以下のファイルをダウンロードし、同一ディレクトリに配置します。

- **モデルファイル**: `encoder.onnx`, `decoder.onnx`, `joiner.onnx`
- **語彙ファイル**: `tokens.txt`
- **（任意）設定ファイル**: `bpe.model` (SentencePiece用)

### 2. Rust プロジェクトの設定
`Cargo.toml` に、`sherpa-onnx` のバインディングと音声入力用の `cpal` を追加します。

```toml
[dependencies]
# sherpa-onnxのRustバインディング
sherpa-rs = "0.1" 
# クロスプラットフォームのマイク入力
cpal = "0.15"
# サンプリングレート変換が必要な場合
rubato = "0.14" 
```

### 3. 実装の主要ステップ
実装は「認識器の初期化」「音声ストリームの生成」「マイク入力ループ」の3段階で構成されます。

#### 認識器の初期化
`OnlineRecognizerConfig` を使用して、ダウンロードしたモデルパスを指定します。

```rust
use sherpa_rs::online_asr::{OnlineRecognizer, OnlineRecognizerConfig};

let config = OnlineRecognizerConfig {
    encoder: "./models/encoder.onnx".into(),
    decoder: "./models/decoder.onnx".into(),
    joiner: "./models/joiner.onnx".into(),
    tokens: "./models/tokens.txt".into(),
    num_threads: 4, // CPUコア数に合わせて調整
    sample_rate: 16000, // モデルの想定レート
    feature_config: Default::default(),
    ..Default::default()
};
let recognizer = OnlineRecognizer::new(config);
```

#### リアルタイム認識のループ
`cpal` のコールバック内で取得した音声データを、`OnlineStream` に流し込みます。

- **サンプリングレートの整合性**: マイク入力が44.1kHzや48kHzの場合、必ず**16kHz**にリサンプリングして供給してください 。[1][2]
- **逐次出力**: `stream.accept_samples()` でデータを送り、`recognizer.decode(&mut stream)` を呼ぶことで、発話中のテキスト（中間結果）をリアルタイムに取得できます 。[3][4]

### 4. 実装における重要なTips

| 項目 | 詳細 |
| :--- | :--- |
| **ビルド設定** | 推論速度を確保するため、必ず `cargo build --release` で実行してください。 |
| **言語の自動識別** | このモデルは多言語対応ですが、日本語を優先する場合はデコーダー設定でトークンを最適化できます [5]。 |
| **VADの併用** | `sherpa-onnx` 内蔵の VAD を有効にすると、無音時のCPU負荷を下げ、誤認識を減らせます [3][6]。 |
| **エンドポイント制御** | `EndpointConfig` を調整することで、「沈黙が500ms続いたら文を確定させる」といった制御が可能です [3]。 |

### 結論
この構成（`sherpa-rs` + `2025-02-10`版 Zipformer）は、Voskよりも高精度かつ低遅延であり、ReazonSpeech v2.1 (k2-v2) 級の精度を**真のストリーミング**で享受できる、2026年時点でのベストプラクティスです。

[1](https://k2-fsa.github.io/sherpa/onnx/pretrained_models/offline-transducer/zipformer-transducer-models.html)
[2](https://dasroot.net/posts/2025/12/building-flutter-voice-assistants-local-speech-recognition/)
[3](https://github.com/k2-fsa/sherpa-onnx)
[4](https://github.com/thewh1teagle/sherpa-rs)
[5](https://huggingface.co/csukuangfj/sherpa-onnx-streaming-zipformer-ar_en_id_ja_ru_th_vi_zh-2025-02-10/tree/main)
[6](https://sourceforge.net/projects/sherpa-onnx.mirror/files/v1.12.18/sherpa-onnx-wasm-simd-1.12.18-vad-asr-zh-zipformer-ctc.tar.bz2/download)