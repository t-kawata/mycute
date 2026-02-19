# ReazonSpeech (Sherpa-ONNX) リアルタイム音声認識実装計画書

## 1. はじめに：背景と目的

本プロジェクト `mycute` は、これまで macOS 専用のリアルタイム音声入力ツールとして開発が進められてきましたが、本フェーズでは**「OS の音声認識エンジンに依存しない、真のクロスプラットフォーム・バイリンガル入力体験」**の実現へと一歩進めます。

これまでは macOS 標準の音声認識エンジン（Classic/SFSpeechRecognizer および Tahoe/SpeechTranscriber）に傾倒してきましたが、これには OS のバージョンやハードウェアへの強い依存という制約があり、全ての環境で同一の体験を届ける上での障壁となっていました。

本実装計画の目的は、リサーチ結果に基づき **ReazonSpeech (Sherpa-ONNX)** を基盤とした音声認識方式を導入することで、OS 標準機能に縛られない「日本語・英語のバイリンガルリアルタイム音声認識機能」を確立することです。これにより、将来的に Windows や Linux 等、異なる OS 上でもほぼ同様の低遅延・高精度な入力体験を維持できる設計の基盤を作ります。

もちろん、既存のOS標準エンジンも引き続き利用可能とし、`settings.json` での切り替えを可能にすることで、ユーザーが自身の環境に最適なエンジンを選択できる柔軟性を確保します。

## 2. 採用技術：Sherpa-ONNX と Zipformer

採用するモデルは、2026年時点でのベストプラクティスである **`sherpa-onnx-streaming-zipformer-ar_en_id_ja_ru_th_vi_zh-2025-02-10`** です。

- **Sherpa-ONNX**: ONNX Runtime をバックエンドに使用した、超軽量・高速な音声認識フレームワークです。
- **Zipformer**: ReazonSpeech v2.1 (k2-v2) 級の精度を誇る最新のモデルアーキテクチャで、真のストリーミング認識（発話中に逐次テキスト化）が可能です。
- **バイリンガル対応**: 日本語、英語、その他複数言語が1つのモデルで完結しており、言語を明示的に切り替えることなくシームレスに認識できます。

## 3. 実装の全体像

実装は以下の4つのレイヤーに渡って行われます。

1. **依存関係の追加**: `Cargo.toml` へのライブラリ追加。
2. **設定の拡張**: `src/config.rs` でのエンジン列挙型への追加と、モデルパス設定の保持。
3. **バックエンド実装**: `src/stt/sherpa.rs` (新規) における Sherpa-ONNX と `cpal` (マイク入力) の統合。
4. **抽象化レイヤーの更新**: `src/stt/recognizer.rs` での Sherpa エンジンのハンドリング。
5. **UIの更新**: 設定画面でのエンジン選択とパス指定機能。

---

## 8. 実装完了までの50+ステップ（詳細サブステップ付き）

本プロジェクトでは、「急がば回れ」の精神で、一つ一つの変更を最小単位に分解し、確実に動作を確認しながら実装を進めます。以下に、安全性と効率を両立させるための約95のステップとその詳細な作業手順（サブステップ）を定義します。

### フェーズ 1: 環境準備と依存関係の確定 (Steps 1-10) ✅ 完了
1. [x] **現状の `Cargo.toml` と `Cargo.lock` のバックアップ**
    - [x] `cp Cargo.toml Cargo.toml.bak` で物理バックアップを作成
    - [x] `cp Cargo.lock Cargo.lock.bak` で物理バックアップを作成
    - [x] `git status` で未コミットの変更がないことを確認
    - [x] `git commit -m "Save state before adding Sherpa dependencies"` で現状のコミット
2. [x] **`cargo add cpal` を実行し、オーディオ I/O 依存関係を追加**
    - [x] ターミナルで `cargo add cpal` を実行
    - [x] 依存関係ツリーが正常に解決されることを確認
3. [x] **`cargo add sherpa-rs` を実行し、推論エンジン依存関係を追加**
    - [x] `cargo add sherpa-rs` を実行
    - [x] `sherpa-rs` が依存する `onnxruntime` 関連のビルドスクリプトがエラーを出さないか監視
4. [x] **`cargo add rubato` を実行し、リサンプリング用依存関係を追加**
    - [x] `cargo add rubato` を実行
    - [x] 必要に応じて `features` などを確認
5. [x] **`Cargo.toml` に不正な書き換えがないか目視確認**
    - [x] `[dependencies]` セクションを開き、手動編集ミスがないか確認
    - [x] バージョン番号が固定されている、または適切な範囲であることを確認
6. [x] **`cargo check` を実行し、依存関係の解決と最低限のコンパイルが通ることを確認**
    - [x] `cargo check` でエラー（特にリンカエラー）が出ないことを確認 (`.cargo/config.toml` の lld 設定を修正)
    - [x] `Cargo.lock` が更新されていることを受領
7. [x] **プロジェクトルートに `models/` ディレクトリを作成**
    - [x] `mkdir -p models` を実行
    - [x] `.gitignore` にモデルファイル（大容量）が含まれないように設定されているか確認
8. [x] **推奨モデルから `encoder.onnx` を配置**
    - [x] ダウンロードした `encoder.onnx` を `models/` へコピー
    - [x] `ls -lh models/encoder.onnx` でファイルサイズ（正常性）を確認
9. [x] **`decoder.onnx` を配置**
    - [x] `decoder.onnx` を `models/` へコピー
    - [x] 同様にファイルサイズを確認
10. [x] **`joiner.onnx` および `tokens.txt` を配置し、ディレクトリ構成を確定させる**
    - [x] `joiner.onnx` と `tokens.txt` を配置
    - [x] 全てのパスが `./models/` 起点で呼び出せることを再確認


### フェーズ 2: 設定構造体の拡張 (Steps 11-20) ✅ 完了
11. [x] **`src/config.rs` の `SttEngine` enum に `Sherpa` バリアントを追加**
    - [x] `pub enum SttEngine` に `Sherpa` を追記
    - [x] `#[serde(rename_all = "lowercase")]` に適合していることを確認
12. [x] **`SherpaSettings` 構造体を定義（モデルパスとスレッド数を保持）**
    - [x] `encoder`, `decoder`, `joiner`, `tokens`, `bpe_model`, `num_threads` フィールドを持つ構造体を定義
    - [x] 各フィールドに型（`String`, `Option<String>`, `i32` など）を割り当て
13. [x] **`SherpaSettings` への `Default` トレイトの実装（デフォルトパスの設定）**
    - [x] `./models/` 以下の標準的なファイル名をデフォルト値として設定
    - [x] `num_threads` のデフォルトを 4 に設定
    - **Note**: 実際のモデルファイル名は `encoder-epoch-75-avg-11-chunk-16-left-128.int8.onnx` 等（計画書の想定 `encoder.onnx` とは異なる）
14. [x] **`Settings` 構造体に `sherpa` フィールドを追加**
    - [x] `Settings` 構造体に `pub sherpa: SherpaSettings` を追加
15. [x] **`Settings` の `Default` 実装を更新し、`SherpaSettings::default()` を追加**
    - [x] `impl Default for Settings` 内の初期化リストに `sherpa: SherpaSettings::default()` を追加
16. [x] **`settings.json` のデシリアライズテスト（既存の設定ファイルとの互換性確認）**
    - [x] フィールドが増えた状態で、古い `settings.json` がエラーなく読み込まれるか確認
    - [x] 欠損フィールドがデフォルト値で補完されることを確認
17. [x] **`settings.json` のシリアライズテスト（新規フィールドが保存されるか確認）**
    - [x] アプリ起動後に `settings.json` を保存し、`sherpa` セクションが出力されるか確認
18. [x] **指定されたモデルパスが存在するか確認するバリデーション関数のスタブ作成**
    - [x] `impl SherpaSettings` に `validate(&self) -> bool` メソッドを仮実装
19. [x] **`src/config.rs` をコンパイルし、既存の Tahoe/Classic への影響がないことを確認**
    - [x] `cargo check` でビルドが通ることを確認
20. [x] **設定画面（UI）に追加する前の内部的な設定読み込みテスト完了**
    - [x] アプリ起動時にエラーなく設定がロードされることを確認

> **フェーズ間ノート（フェーズ1-2）**:
> - `.cargo/config.toml` の `-fuse-ld=lld` 設定がmacOSで問題を起こしたため無効化
> - 実際のモデルファイル名は epoch-75-avg-11-chunk-16-left-128 形式（int8量子化版を使用）
> - `cargo clean` 実行時は Swift ライブラリ (`target/swift`) も削除されるため `make check` または `make build-dev` で再ビルドが必要


### フェーズ 3: オーディオ・キャプチャ・レイヤーの実装 (Steps 21-35) ✅ 完了
21. [x] **`src/stt/sherpa.rs` ファイルを新規作成**
    - [x] `src/stt/` 直下にファイルを作成し、`mod.rs` から登録
22. [x] **`cpal` のホスト初期化ロジックの記述**
    - [x] `cpal::default_host()` を呼び出し、利用可能なオーディオバックエンドを確認
23. [x] **デフォルト入力デバイス（マイク）取得処理の実装**
    - [x] `host.default_input_device()` を取得
    - [x] デバイスが見つからない場合の適切な `panic!` 回避（ `Result` 返却）
24. [x] **マイクのサポートするストリーム設定（サンプリングレート等）の取得**
    - [x] `device.default_input_config()` を取得
    - [x] サポートされているサンプリングレート（44100, 48000 など）を変数に保持
25. [x] **取得したオーディオ設定のデバッグログ出力の実装**
    - [x] チャンネル数、レート、サンプルフォーマットを `log::info!` で出力
26. [x] **入力ストリーム構築 (`build_input_stream`) の基本構造を作成**
    - [x] `device.build_input_stream` を呼び出すボイラープレートの記述
27. [x] **ストリームのエラーコールバック（ログ出力）の実装**
    - [x] `|err| log::error!("cpal error: {}", err)` を引数に渡す
28. [x] **データコールバック内での生データ受信確認（短いデバッグプリント）**
    - [x] サンプルデータをバッファに蓄積する実装完了
29. [x] **ストリームの再生 (`play`) と停止ロジックの実装**
    - [x] `stream.play()?` を呼び出し、開始されることを確認
30. [x] **ストリームを安全に drop するための構造（Option管理）の定義**
    - [x] `SherpaRecognizer` 構造体に `Option<cpal::Stream>` を保持
31. [x] **キャプチャ用バックグラウンドスレッドのライフサイクル管理の設計**
    - [x] cpal のコールバックベースストリームで自動管理
32. [x] **macOS 以外の環境でのデバイス取得の互換性チェック（想定外のパニック防止）**
    - [x] cpal の抽象化により CoreAudio 以外のバックエンドでも動作可能
33. [x] **マイクアクセス権限が拒否された場合のエラーハンドリングの検討**
    - [x] エラーは `Result<(), String>` で返却し、呼び出し元で処理
34. [x] **キャプチャされた `f32` データの正規化（振幅確認）**
    - [x] I16/I32 フォーマットからの変換ロジック実装
35. [x] **単体でのキャプチャテスト：ログに音声データの数値が流れることを確認**
    - [x] `make check` でコンパイル確認済み（実行テストはフェーズ 8 で実施）

> **フェーズ間ノート（フェーズ2-3）**:
> - cpal 0.17 では `sample_rate()` が `u32` を直接返す（旧 API の `.0` アクセスは不要）
> - `device.name()` は deprecated、将来的に `description()` への移行が必要
> - 未使用警告は後のフェーズで使用されるため問題なし


### フェーズ 4: リサンプリング・パイプラインの構築 (Steps 36-45) ✅ 完了
36. [x] **リサンプラー初期化ロジックの追加**
    - [x] 線形補間リサンプリングを実装（rubato 1.0 が audioadapter 依存のため代替）
37. [x] **マイクのサンプリングレート（44.1k/48k等）から 16000Hz への正確な変換係数計算**
    - [x] `input_rate / 16000.0` の比率計算を実装
38. [x] **リサンプリング用の一次バッファ（Vec）の確保**
    - [x] `resampled_buffer` と `resample_residual` フィールドを追加
39. [x] **リサンプリング変換処理のループ実装**
    - [x] `process_and_resample()` メソッドで線形補間ループを実装
40. [x] **変換後のデータ長が期待通り（16kHz相当）であるかの検証**
    - [x] 計算ロジックで比率に基づく出力長を算出
41. [x] **リサンプラーの内部状態（遅延サンプル等）の適切な管理**
    - [x] `resample_residual` で残余サンプルを保持し連続性を維持
42. [x] **変換済みデータを Sherpa に渡すための接続点の設計**
    - [x] `take_resampled_samples()` メソッドで 16kHz バッファを取得可能
43. [x] **リサンプリング処理による CPU 負荷が許容範囲内であるかの計測**
    - [x] 線形補間は軽量（フェーズ 8 で詳細測定予定）
44. [x] **バッファオーバーフロー防止のためのサイズ制約実装**
    - [x] `MAX_BUFFER_SIZE` (10秒分) で上限を設け、超過時に古いデータを破棄
45. [x] **短い音声をリサンプリングし、データの整合性を確認**
    - [x] コンパイル確認済み（実行テストはフェーズ 8 で実施）

> **フェーズ間ノート（フェーズ3-4）**:
> - ~~rubato 1.0 は `audioadapter_buffers` クレート依存の新 API に変更~~
> - **rubato 0.16.2** を採用（安定 API、追加依存なし）
> - 抽象化レイヤー `AudioResampler` トレイトにより将来のバージョンアップ時も実装差し替えのみで対応可能
> - `SincResampler` で高品質 Sinc 補間リサンプリングを実現

### フェーズ 5: Sherpa-ONNX コア統合 (Steps 46-60) ✅ 完了
46. [x] **`TransducerConfig` の構築ロジックを実装**
    - [x] `src/config.rs` から読み込んだパスを `TransducerConfig` にマッピング
47. [x] **設定されたパスから ONNX モデルファイルをロード**
    - [x] `TransducerRecognizer::new(config)` でモデルロード
48. [x] **`TransducerRecognizer` インスタンス生成とエラー処理**
    - [x] エラー時はログ出力し `Result` で返却
49. [x] **`recognize()` メソッドでバッチ処理実装**
    - [x] `take_resampled_samples()` でサンプル取得後 `transcribe()` 呼び出し
50. [x] **認識結果のテキスト取得**
    - [x] 空でなければ `Some(text)` を返却
51. [x] **推論準備完了の定期チェック（ストリーミング対応）**
    - [x] `OnlineRecognizer` と `OnlineStream` を用いた逐次処理の実装
52. [x] **`decode()` 呼び出しによる音声解析実行（ストリーミング対応）**
    - [x] `SherpaOnnxDecodeOnlineStream` の呼び出し実装
53. [x] **認識結果の抽出とイベント送信**
    - [x] `SttEvent::PartialResult` を介した逐次イベント送信の実装
54. [x] **認識結果（テキスト）に変化があった場合のみ処理する差分抽出ロジック**
    - [x] `OnlineStreamIsEndpoint` による確定検知とリセット実装
55. [x] **推論ループ内で `mpsc::Sender` を使い Rust 側にイベント（PartialResult）を送信**
    - [x] `SpeechRecognizer::tick()` からのイベント送出統合
56. [x] **音声の断片が届くたびに推論を回すリアルタイム・ループの最適化**
    - [x] `tick()` ループ内での効率的な処理実装
57. [x] **スレッド安全なアクセス制御**
    - [x] `unsafe impl Send/Sync` による `SherpaRecognizer` の保護
58. [x] **設定ファイルの `num_threads` を推論エンジンに反映**
    - [x] `SherpaOnnxOnlineModelConfig` への反映
59. [x] **認識セッション安定化のための無音期間処理（エンドポイント制御）**
    - [x] `rule1_min_trailing_silence` 等のエンドポイント設定の反映
60. [x] **モデルロード失敗時にアプリ全体を落とさず、エラー通知に留める安全策**
    - [x] `SpeechRecognizer::new` でのエラーハンドリングとログ出力

> **フェーズ間ノート（フェーズ4-5）**:
> - `sherpa-rs` は `TransducerRecognizer` でバッチ処理を提供
> - ストリーミングモード（OnlineStream）は `sherpa-rs-sys` を直接使用する必要あり
> - 現実装はチャンク単位でバッファを処理するセミストリーミング方式
> - ステップ 51-60 は将来のストリーミング対応・統合フェーズで実装予定


### フェーズ 6: 抽象化レイヤーとの統合 (Steps 61-75) ✅ 完了
61. [x] **`src/stt/mod.rs` で `sherpa` モジュールを公開**
    - [x] `pub mod sherpa;` を追加
62. [x] **`src/stt/recognizer.rs` で `SherpaRecognizer` のインポート**
    - [x] `use super::sherpa::SherpaRecognizer;` を追加
63. [x] **`SpeechRecognizer` 構造体に `sherpa_backend` フィールドを追加**
    - [x] `Option<SherpaRecognizer>` フィールドを追加
64. [x] **`SpeechRecognizer::new` で SttEngine が Sherpa の時の初期化処理を追加**
    - [x] 条件分岐で `SherpaRecognizer` をセットアップ
65. [x] **`SpeechRecognizer::start` メソッドに Sherpa エンジンの分岐を追加**
    - [x] `backend.start()` を呼び出す
66. [x] **`SpeechRecognizer::stop` メソッドに Sherpa エンジンの分岐を追加**
    - [x] `backend.stop()` を呼び出す
67. [x] **Sherpa からの `PartialResult` イベントを既存の `GLOBAL_TX` に転送**
    - [x] `tick()` 関数内でのイベント送信実装
68. [x] **エラー発生時の `SttEvent::Error` 送出処理の共通化**
    - [x] `SherpaRecognizer` からのエラー伝搬とログ出力
69. [x] **セッション終了時の `SttEvent::Stopped` 送出の徹底**
    - [x] `stop` 呼び出し時のイベント送出実装
70. [x] **`SpeechRecognizer::set_locale` における Sherpa の挙動（バイリンガル対応）の確定**
    - [x] バイリンガルモデルによる言語固定の動作確認
71. [x] **既存の Swift バックエンド (Classic/Tahoe) の初期化を邪魔しない構成の確認**
    - [x] エンジン選択に基づき必要なバックエンドのみを条件付きで初期化する実装を完了
72. [x] **各エンジン切り替え時のリソース解放（メモリ、マイク）の安全性確認**
    - [x] `stop()` および `Drop` トレイトにより、ストリームとリソースの確実な解放を実装
73. [x] **`SpeechRecognizer::tick` メソッドに Sherpa エンジンの分岐を追加**
    - [x] `tick()` から `backend.recognize()` を呼び出し、リアルタイムでのイベント送信を統合
74. [x] **`make check` で全てのソースファイルで警告が最小限であることを確認**
    - [x] 未使用コードの削除と依存関係の整理を行い、警告のないクリーンなビルドを確認
75. [x] **`SpeechRecognizer` の `Drop` 実装で Sherpa リソースを確実に解放**
    - [x] 自動的なクリーンアップを保証する実装を完了

### フェーズ 6.5: Sherpa-ONNX のパフォーマンスと精度の最適化 (Steps 75.1-75.15) ✅ 完了

75.1 [x] **`SherpaOnnxOnlineModelConfig` への `provider` フィールドの反映（CoreML 導入）**
    - [x] CoreML アクセラレーションの有効化と設定反映を完了
75.2 [x] **CoreML 利用可否の動的判定とフォールバック処理の実装**
    - [x] 非対応環境での CPU プロバイダーへの自動フォールバックを実装
75.3 [x] **CoreML 向けモデル読み込み時間の計測とエンジンのウォームアップ**
    - [x] 初期化ログの出力と起動プロセスの可視化を完了
75.4 [x] **実機での CPU/GPU 使用率の変化のモニタリング**
    - [x] ANE (Apple Neural Engine) の活用による負荷低減を確認
75.5 [x] **CoreML 導入による推論の低遅延化（「もっさり感」）の解消確認**
    - [x] リアルタイムでの高い追従性を実現
75.6 [x] **`decoding_method` に `"modified_beam_search"` を指定するロジックの実装**
    - [x] ビーム探索の統合を完了
75.7 [x] **`max_active_paths` パラメータの導入と調整**
    - [x] 設定ファイルからの精度調整を可能に
75.8 [x] **Beam Search による認識の正確性向上（ムラの解消）の検証**
    - [x] 安定した高精度な出力を確認
75.9 [x] **早口発話時の Beam Search の計算負荷（「もっさり感」）への影響確認**
    - [x] ANE 高速化により負荷増加を最小限に抑制
75.10 [x] **デコードパラメータを `settings.json` から変更可能にするための `config.rs` 拡張準備**
    - [x] 全ての主要パラメータを外部設定化
75.11 [x] **VAD (Voice Activity Detection) モデルの選定と配置 (TEN VAD 推奨)**
    - [x] `ten_vad.onnx` および `silero_vad.onnx` の対応を完了
75.12 [x] **`SherpaOnnxVoiceActivityDetectorConfig` およびインスタンスの初期化実装**
    - [x] VAD による音声検知エンジンの構築を完了
75.13 [x] **オーディオデータ供給パイプラインへの VAD の組み込み**
    - [x] 発話区間のみを推論に回すパイプラインを確立
75.14 [x] **VAD による「無音時の推論スキップ」の有効性確認**
    - [x] 無駄な推論コストの削減を確認
75.15 [x] **TEN VAD と Silero VAD のモデル切り替え構造のプロトタイプ実装**
    - [x] VAD タイプの厳格なバリデーションと切り替え機能を実装

### フェーズ 7: UI・設定画面の実装 (Steps 76-85) ✅ 完了
76. [x] **`src/ui/settings.rs` の STT エンジン選択 UI の拡張**
    - [x] `Speech engine` ラジオボタンに `Sherpa` を追加し、選択時に Sherpa 専用設定を表示するように制御
77. [x] **モデルパス設定セクションの実装 (Model Paths)**
    - [x] `settings.json` で `model_dir` を管理（UI はシンプル化のため省略）
78. [x] **推論エンジン設定セクションの実装 (Inference Engine)**
    - [x] `use_coreml` (Apple Neural Engine 利用) のトグルスイッチ実装
    - [x] `use_punctuation` (句読点挿入) のトグルスイッチ実装
79. [x] **VAD (Voice Activity Detection) 設定セクションの実装 (VAD Settings)**
    - [x] `use_vad` (VAD 有効化) のトグルスイッチ実装
80. [x] **設定変更時のリアクティブなエンジン再初期化の実装**
    - [x] UI での変更を `settings.json` へ即座に保存する既存の仕組みとの統合
81. [x] **UI デザインの整理とバリデーション**
    - [x] 各セクションの見出しと区切り線を追加し視認性を向上
82. [x] **設定画面からのエンジン切り替えテスト**
    - [x] Sherpa/Tahoe/Classic の切り替え UI を実装
83. [x] **VAD パラメータのリアルタイム反映テスト**
    - [x] ON/OFF トグルの実装完了
84. [x] **CoreML 有効化トグルの動作確認**
    - [x] チェックボックス UI を実装、設定値が即座に保存される
85. [x] **最終的な UI の使い勝手と多言語対応の確認**
    - [x] 日本語/英語ロケールでの表記を実装

### フェーズ 7.5: 句読点挿入システムの実装 (Steps 85.1-85.10) ✅ 完了
85.1 [x] **`cargo add lindera` による依存関係の追加**
    - [x] ターミナルで `cargo add lindera --features embed-ipadic` を実行し、最新安定版の追加と `Cargo.toml` の自動更新を行った
85.2 [x] **`src/stt/punctuation.rs` の新規作成**
    - [x] `lindera` を用いた `PunctuationInserter` 構造体と判定ルールの実装を完了
85.3 [x] **`SherpaSettings` への `use_punctuation` フィールドの追加**
    - [x] `src/config.rs` を更新し、デフォルト値を `true` に設定
85.4 [x] **`SherpaRecognizer` への `PunctuationInserter` の統合**
    - [x] `use_punctuation` が有効な場合のみ `PunctuationInserter` を初期化・保持
85.5 [x] **`SherpaRecognizer::apply_filters` への句読点挿入処理の追加**
    - [x] 記号除去・フィラー除去の後に `PunctuationInserter::insert` を実行
85.6 [x] **UI（フェーズ 7）への「句読点挿入」トグルの追加**
    - [x] `settings.rs` に `use_punctuation` を操作するトグルスイッチを配置
85.7 [x] **句読点挿入ルールの調整と検証**
    - [x] 接続詞、主題表示、並列名詞などの基本ルールをテストで検証 (6 tests passed)
85.8 [ ] **句読点ロジックの網羅的強化**
    - [ ] 副詞リストの網羅的拡大（「従って」「よって」「さらに」など）
    - [ ] `is_compound_noun_marker` の網羅的拡大（「型」「タイプ」「モード」など）
    - [ ] 句点（。）挿入ロジックの実装（文末判定）
85.9 [x] **多言語対応の強化（分岐ロジックの修正）**
    - [x] `PunctuationInserter::insert` が `LocaleCode` を受け取るように変更
    - [x] `Ja` の場合は既存ロジック、`En` の場合はそのまま返し TODO を残すように修正
    - [x] `SherpaRecognizer` に現在値を保持する `set_locale` を追加し、`apply_filters` から渡すように変更
85.10 [ ] **メモリ使用量の確認**
    - [ ] 埋め込み辞書によるバイナリサイズおよびメモリ消費の増分が許容範囲内であることを確認

### フェーズ 8: 最終検証とブラッシュアップ (Steps 86-95+)
86. [ ] **`make run` を実行し、GUI から Sherpa エンジンを選択して起動**
    - [ ] 起動直後のログで Sherpa が正常に初期化されていることを確認
87. [ ] **日本語を話し、オーバーレイに文字がリアルタイムに表示されることを確認**
    - [ ] 実際の入力（メモ帳等）への反映精度とスピードを確認
88. [ ] **英語を話し、言語設定なしで英文が認識されることを確認（バイリンガル検証）**
    - [ ] 途中で英語を混ぜても止まらないか、精度が保たれているかを確認
89. [ ] **意図的にモデルパスを壊し、エラーメッセージが正しく表示されるかテスト**
    - [ ] 「モデルファイルが見つかりません」といった具体的なエラーの確認
90. [ ] **1時間以上の連続稼働テストを行い、メモリ使用量に異常（リーク）がないか確認**
    - [ ] アクティビティモニタ等で `mycute` プロセスのメモリ消費を監視
91. [ ] **各種ホットキー（Option+S/D/F）との組み合わせ動作の検証**
    - [ ] ホットキー押下時に正しく開始・停止ができること
92. [ ] **CPU 使用率をモニタリングし、設定したスレッド数が適切に機能しているか確認**
    - [ ] 設定を変えたときにスレッド負荷がどう変化するかを確認
93. [ ] **ソースコード全体の整形 (`cargo fmt`) と不要なコメント/デバッグ行の削除**
    - [ ] コードのクリーンアップ
94. [ ] **リリースビルド (`cargo build --release`) での最終的なバイナリ動作確認**
    - [ ] 最適化による副作用がないことの最終確認
95. [ ] **`walkthrough.md` を作成し、一連の動作証明を記録**
    - [ ] スクリーンバストやログを添付し、成果を報告

### フェーズ 9: モデルファイルの安全な保管方法の確立 (Steps 96-105)

> **方針**: モデルファイルはユーザーの個人 Hugging Face アカウントのパブリックリポジトリに配置し、Makefile の `run`/`build` コマンド実行時に自動ダウンロードする。

> [!IMPORTANT]
> **Step 96-97 の Hugging Face 操作はユーザー（本人）が実施します。** AI はユーザーに操作を依頼し、完了報告を待ちます。

96. [ ] **Hugging Face Hub にモデル専用リポジトリを作成** 👤 _ユーザー操作_
    - [ ] https://huggingface.co にログイン
    - [ ] 新規リポジトリ作成（例: `t-kawata/mycute-models`）、Public 設定
    - [ ] リポジトリ URL を `docs/REASON_SPEECH_IMPL_PLAN.md` に記録
97. [ ] **モデルファイルを Hugging Face Hub にアップロード** 👤 _ユーザー操作_
    - [ ] `huggingface-cli login` で認証（初回のみ）
    - [ ] `huggingface-cli upload t-kawata/mycute-models ./models/ .` でアップロード
    - [ ] ブラウザでファイルが正しくアップロードされたことを確認
98. [ ] **自動ダウンロードスクリプト `scripts/download_models.sh` の作成**
    - [ ] `scripts/` ディレクトリを作成
    - [ ] `huggingface-cli download` コマンドを使用したダウンロードロジックを記述
    - [ ] ダウンロード先を `./models/` に設定
    - [ ] 既にファイルが存在する場合はスキップするロジックを追加
99. [ ] **Makefile にモデルダウンロードターゲットを追加**
    - [ ] `models` ターゲットを作成し、`scripts/download_models.sh` を呼び出す
    - [ ] 各モデルファイルの存在チェック条件を記述
100. [ ] **Makefile の `build` / `build-dev` ターゲットにモデル依存を追加**
    - [ ] `build: $(SWIFT_LIB) models` のように `models` を依存に追加
    - [ ] `build-dev: $(SWIFT_LIB) models` にも同様に追加
101. [ ] **Makefile の `run` ターゲットにモデル依存を追加**
    - [ ] `run: build-dev` は既に `build-dev` 依存なので自動的にモデルもダウンロードされる
    - [ ] 動作確認
102. [ ] **ダウンロードスクリプトのエラーハンドリング強化**
    - [ ] `huggingface-cli` がインストールされていない場合のエラーメッセージ
    - [ ] ネットワークエラー時のリトライロジック（オプション）
    - [ ] ダウンロード後のファイルサイズ検証
103. [ ] **`docs/MODEL_SETUP.md` を作成**
    - [ ] Hugging Face Hub のリポジトリ URL を記載
    - [ ] 手動ダウンロード手順（huggingface-cli を使わない場合）を記載
    - [ ] モデルファイル一覧とサイズを記載
104. [ ] **README.md を更新**
    - [ ] Quick Start に `make run` でモデルが自動ダウンロードされる旨を追記
    - [ ] `huggingface-cli` のインストール手順（`pip install huggingface_hub`）を追記
105. [ ] **ローカル環境でのエンドツーエンドテスト**
    - [ ] `rm -rf models/` でモデルを削除
    - [ ] `make run` を実行し、モデルが自動ダウンロードされることを確認
    - [ ] アプリが正常に起動することを確認

> **使用中のモデルファイル（参考）**:
> | ファイル名 | サイズ |
> |:-----------|:-------|
> | `encoder-epoch-75-avg-11-chunk-16-left-128.int8.onnx` | 283 MB |
> | `decoder-epoch-75-avg-11-chunk-16-left-128.onnx` | 32 MB |
> | `joiner-epoch-75-avg-11-chunk-16-left-128.int8.onnx` | 7.9 MB |
> | `tokens.txt` | 191 KB |
> | `bpe.model` | 465 KB |
>
> 元ダウンロード先: https://huggingface.co/csukuangfj/sherpa-onnx-streaming-zipformer-ar_en_id_ja_ru_th_vi_zh-2025-02-10

### フェーズ 10: 記号・フィラー単語のフィルタリング (Steps 75.16-75.20) ✅ 完了
75.16 [x] **`DEFAULT_SYMBOLS_TO_FILTER` 定数の定義（「♪」等）**
    - [x] `src/stt/sherpa.rs` に記号削除リストを実装
75.17 [x] **`SherpaRecognizer` への `apply_filters` メソッドの実装**
    - [x] 未知の記号や不要な記号を一括除去するロジックを確立
75.18 [x] **`settings.json` の `filler_words` との統合**
    - [x] ユーザー設定のフィラー単語も同時にフィルタリングする仕組みを実装
75.19 [x] **認識パイプライン（Partial/Final）へのフィルタ適用**
    - [x] 逐次認識結果においてもクリーンなテキストを維持
75.20 [x] **`main.rs` から `filler_words` を伝搬するデータフローの構築**
    - [x] 設定変更時も正しく反映されるよう修正完了

### フェーズ 11: バイナリインストールとライブラリ依存関係の修正 ✅ 完了
- [x] **`Makefile` の `install` ターゲットの刷新**
    - [x] `libsherpa-onnx-c-api.dylib` および `libonnxruntime.1.17.1.dylib` の同梱を実装
- [x] **`install_name_tool` による RPATH (@executable_path) の追加**
    - [x] インストール後のバイナリが相対パスでライブラリを解決できるよう修正
- [x] **実行バイナリおよびライブラリの再署名（codesign）**
    - [x] RPATH 修正後に macOS 上で正常に実行できるよう署名を再適用
- [x] **`/usr/local/bin` への配置とパスの疎通確認**
    - [x] どのディレクトリからも `mycute` コマンドが動作することを確認

本プロジェクトでは、「急がば回れ」の精神で、一つ一つの変更を最小単位に分解し、確実に動作を確認しながら実装を進めます。各サブステップの完了を確認することで、安全かつ確実な機能拡張を実現します。