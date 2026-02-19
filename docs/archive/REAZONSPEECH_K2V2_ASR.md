# 問題意識
現在使用している sherpa-onnx-streaming-zipformer-ar_en_id_ja_ru_th_vi_zh-2025-02-10 は、非常に精度が低い。特に日本語の精度は低くて、時々中国語が混ざってしまう。よって真のリアルタイムストリーミング音声認識にはならないが、`reazonspeech-k2-v2 / sherpa-onnx-zipformer-ja-reazonspeech` を使用することにより、疑似ストリーミング音声認識を実装することにする。

# `reazonspeech-k2-v2 / sherpa-onnx-zipformer-ja-reazonspeech` を使って「VAD＋オフラインASRで疑似ストリーミングし、INTERIM と FINAL を出す」最小構成のイメージを Rust で書きます。
`sherpa-rs` という Rust バインディングと `cpal` を使う想定です（型名・関数名は sherpa-onnx の C++/Python API にかなり寄せた擬似コードですが、構造はこのまま移植できます）。[1][2]

## Cargo.toml（イメージ）

```toml
[dependencies]
cpal = "0.15"
sherpa-rs = "0.1"   # 仮のクレート名・バージョン（実際のものに合わせてください）
anyhow = "1"
crossbeam-channel = "0.5"
```

## 初期化部分（ASR と VAD）

```rust
use anyhow::Result;
use crossbeam_channel::{unbounded, Receiver};
use std::time::{Duration, Instant};

// sherpa-rs の API 名は仮です。実際のクレートに合わせて読み替えてください。
use sherpa_rs::{
    OfflineRecognizer, OfflineRecognizerConfig, OfflineStream,
    VadModel, VadConfig,
};

fn create_offline_recognizer() -> Result<OfflineRecognizer> {
    let cfg = OfflineRecognizerConfig {
        // reazonspeech-k2-v2 の ONNX 一式に合わせて設定
        encoder: "models/reazonspeech-k2-v2/encoder.onnx".into(),
        decoder: "models/reazonspeech-k2-v2/decoder.onnx".into(),
        joiner:  "models/reazonspeech-k2-v2/joiner.onnx".into(),
        tokens:  "models/reazonspeech-k2-v2/tokens.txt".into(),
        num_threads: 4,
        debug: false,
    };

    Ok(OfflineRecognizer::new(cfg)?)
}

fn create_vad() -> Result<VadModel> {
    let cfg = VadConfig {
        // Silero-VAD の ONNX など
        model_path: "models/silero-vad.onnx".into(),
        sample_rate: 16000,
        frame_length_ms: 10,          // 10ms/frame
        min_speech_duration_ms: 300,  // 0.3秒以上で開始判定
        min_silence_duration_ms: 500, // 0.5秒以上で終了判定
        max_speech_duration_ms: 20000,
        // ほかのパラメータも実APIに合わせて
    };
    Ok(VadModel::new(cfg)?)
}
```

## マイク入力スレッド（cpal）

```rust
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

fn spawn_mic_thread() -> Receiver<Vec<f32>> {
    let (tx, rx) = unbounded::<Vec<f32>>();

    std::thread::spawn(move || {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .expect("no input device available");

        let mut supported_configs_range = device
            .supported_input_configs()
            .expect("error while querying configs");
        let supported_config = supported_configs_range
            .find(|c| c.sample_format() == cpal::SampleFormat::F32)
            .expect("no f32 config");
        let config = supported_config.with_max_sample_rate().config();

        let err_fn = |err| eprintln!("an error occurred on stream: {}", err);

        let tx2 = tx.clone();
        let stream = device
            .build_input_stream(
                &config,
                move | &[f32], _| {
                    // data はフレームの塊。ここでは 10ms ごとに切り出すイメージで、
                    // とりあえずそのままチャネルに投げる。
                    let buf = data.to_vec();
                    let _ = tx2.send(buf);
                },
                err_fn,
                None,
            )
            .expect("failed to build input stream");

        stream.play().expect("failed to play stream");

        // スレッドは stream ライフタイム中動き続ける
        loop {
            std::thread::sleep(Duration::from_secs(1));
        }
    });

    rx
}
```

## メインループ（VAD＋疑似ストリーミング ASR）

```rust
fn asr_offline_recognize(
    recognizer: &OfflineRecognizer,
    samples: &[f32],
) -> Result<String> {
    // 1 チャンク分をまるごと認識
    let mut stream = OfflineStream::new(recognizer)?;
    stream.accept_waveform(16000, samples);  // 16kHz 前提
    recognizer.decode(&mut stream)?;
    let result = recognizer.get_result(&stream)?;
    Ok(result.text)
}

fn main() -> Result<()> {
    let recognizer = create_offline_recognizer()?;
    let mut vad = create_vad()?;
    let mic_rx = spawn_mic_thread();

    let mut current_chunk: Vec<f32> = Vec::new();
    let mut base_text = String::new();
    let mut last_interim = Instant::now();
    let interim_interval = Duration::from_millis(500); // 0.5秒ごとに中間更新

    println!("Start pseudo-streaming ASR (k2-v2 + VAD).");

    loop {
        let frame = mic_rx.recv()?; // ここでは「ある程度のサンプル数」の塊

        // VAD に渡す（内部で 10ms ごとに分割する想定）
        vad.accept_waveform(&frame);

        if vad.is_speech_detected() {
            // 発話中: チャンクをためる
            current_chunk.extend_from_slice(&frame);

            // 一定間隔ごとに中間認識
            if last_interim.elapsed() >= interim_interval {
                last_interim = Instant::now();

                if !current_chunk.is_empty() {
                    // 中間結果: いまのチャンクをそのままオフライン認識
                    if let Ok(text) = asr_offline_recognize(&recognizer, &current_chunk) {
                        // これまでの確定テキスト + 現チャンクの暫定
                        print!(
                            "\r[INTERIM] {}{}",
                            base_text,
                            text
                        );
                        use std::io::Write;
                        std::io::stdout().flush().ok();
                    }
                }
            }
        }

        // just_ended_speech の実装は VAD 側 API に依存します
        if vad.just_ended_speech() && !current_chunk.is_empty() {
            // 発話終了: 最終認識
            if let Ok(final_text) = asr_offline_recognize(&recognizer, &current_chunk) {
                if !base_text.is_empty() {
                    base_text.push(' ');
                }
                base_text.push_str(&final_text);

                println!("\n[FINAL] {}", base_text);
            }

            current_chunk.clear();
            last_interim = Instant::now();
        }
    }
}
```

## 実装時のポイント

- VAD API
  - 実際の sherpa-onnx VAD（Silero）には `is_speech_detected`・`is_silence_detected`・`reset` など、もう少し細かい状態管理 API があるはずなので、そこに合わせて `just_ended_speech()` のロジックを書き換えてください。[3][4]
- 中間結果の頻度
  - `interim_interval` を短くしすぎると、そのたびに `OfflineRecognizer` がフルパスを回すので CPU が厳しくなります。0.3〜0.7秒くらいが現実的な範囲です。[5]
- レイテンシ感
  - 「真のストリーミング」と違い、文頭側は VAD が「speech start」を検知するまで多少遅れて出ますが、**喋り続けている間にテキストが伸びていく**体感はかなり SFSpeechRecognizer に近づきます。[6][5]

この骨格を、手元の `sherpa-rs` の具体的な型名・関数名に合わせて書き換えれば、そのまま「VAD＋k2-v2 で疑似ストリーミング・INTERIM/FINAL 出し」の土台になります。

情報源
[1] k2-fsa/sherpa-onnx https://github.com/k2-fsa/sherpa-onnx
[2] sherpa-onnx https://pypi.org/project/sherpa-onnx/
[3] k2-fsa/sherpa-onnx - Speech not detected by silero vad https://github.com/k2-fsa/sherpa-onnx/issues/1084
[4] csukuangfj/sherpa-onnx-apk at main - Hugging Face https://huggingface.co/csukuangfj/sherpa-onnx-apk/tree/main
[5] Paraformer models - sherpa-onnx https://k2-fsa.github.io/sherpa/onnx/pretrained_models/offline-paraformer/paraformer-models.html
[6] sherpa 1.3 documentation https://k2-fsa.github.io/sherpa/
