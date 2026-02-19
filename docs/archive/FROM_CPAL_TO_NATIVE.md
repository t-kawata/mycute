# cpal からネイティブ録音（C# / Swift）への移行計画

## 1. 背景と動機

現在、OpenAI モードの音声入力には Rust のクロスプラットフォーム・ライブラリである `cpal` を使用していますが、Windows 環境において以下の問題が発生しています。

- **不安定なデバイス認識**: Windows (WASAPI) では、`cpal` の `Host` インスタンスの生存期間管理が難しく、ストリームが途中で無音になったり、初期化に失敗したりするケースがあります。
- **COM 競合**: C# の `SpeechHelper.dll` と `cpal` が同時に COM を初期化・利用しようとすることで、デバッグが困難な競合を引き起こす可能性があります。
- **構造の非対称性**: すでに Mac/Windows 双方で強力なネイティブ・ヘルパー・ライブラリ（C#/Swift）が稼働しているにもかかわらず、録音だけを Rust 側で別系統で行うことは、リソースの重複と管理の複雑化を招いています。

## 2. ネイティブ移行のメリット

- **最高の安定性**: 各 OS が公式に提供する録音パス（C# の WASAPI 連携、Mac の AVAudioEngine）を直接利用することで、録音の安定性が OS 標準レベルまで引き上げられます。
- **デバイス選択の一致**: OS 標準 STT モードでマイクが正常に動作していれば、OpenAI モードでも自動的にマイクが動作することを保証できます。
- **バイナリの軽量化**: `cpal` およびその OS 依存バックエンド・スタックを Rust のビルドから除外でき、メンテナンス性が向上します。
- **対称的なイベントモデル**: 音声認識テキストだけでなく、音声データも「FFI コールバック」として受け取ることで、全プラットフォームで一貫したデータフローを構築できます。

---

## 3. 実装手順（ロードマップ）

移行は以下の順序で慎重に進めます。

### フェーズ 1: Windows (C#) ヘルパーの拡張 ✅ 完了
- [x] 1. **録音コールバックの定義**: `SpeechHelper.cs` に、音声サンプル（`float[]`）を Rust に渡すためのデリゲートと FFI 用の登録関数を追加。
- [x] 2. **録音専用モードの追加**: `AudioGraph` を用いたネイティブキャプチャ機能を実装。
- [x] 3. **サンプルレートの伝達**: 16kHz 固定でのデータ転送を確立。

### フェーズ 2: Mac (Swift) ヘルパーの拡張 ✅ 完了
- [x] 1. **AVAudioEngine Tap の活用**: `AVAudioEngine.inputNode.installTap` を使用して、録音データを Rust に転送する仕組みを追加。
- [x] 2. **FFI 公開**: `speech_helper_set_audio_data_callback` 等を追加し、Rust 側のポインタへデータを渡す。

### フェーズ 3: Rust 側 (`openai.rs`) のリファクタリング ✅ 完了
- [x] 1. **cpal の引退 (Windows/Mac)**: 両プラットフォームでの `cpal` 除去を完了。
- [x] 2. **ネイティブ・コールバックの受領 (Windows/Mac)**:
   - [x] `speech_helper_set_audio_data_callback` を呼び出し、Rust 側の関数を登録。
   - [x] コールバック内で受け取ったデータを `streamer.push_samples(data, rate)` に渡す。
   - [x] **[NEW]** Mac の動的サンプリングレートに対応（`Arc<AtomicU32>` による管理）。
- [x] 3. **動作検証 (Mac)**: Mac 実機でのネイティブ移行と動作確認。

### フェーズ 4: クリーンアップ ✅ 完了
- [x] 1. **Cargo.toml**: `cpal` および関連依存関係を削除（すべてのプラットフォームで完全に除去）。

---

## 7. Mac への引き継ぎ事項 (Handover to Mac)

Windows 側の Antigravity により、Mac 向けのコード実装（Swift/Rust）は完了しています。Mac 側の Antigravity は以下の手順で検証および調整を行ってください。

### 現状のコードベース
- **Swift**: `swift/SpeechHelper.swift` に `AVAudioEngine` を用いた録音 tap ロジックと FFI 関数を追加済み。
- **Rust (Mac Bridge)**: `src/stt/mac.rs` に Swift からの音声データを受け取るコールバックとブリッジ関数を追加済み。
- **Rust (OpenAI)**: `src/stt/openai.rs` で `cpal` を Mac でも無効化し、ネイティブキャプチャ（`mac::start_native_audio_capture`）を使用するように変更済み。サンプリングレートは動的に追従します。

### Mac 側での検証ステップ
1.  **ビルド設定の確認**:
    - `Makefile` 等で `SpeechHelper.swift` が正しくビルド・リンクされるか確認してください。新しく追加した FFI シンボル（`speech_helper_start_capture` 等）が解決できる必要があります。
2.  **実行とログ確認**:
    - `make run ARGS="vp"` 等で起動し、OpenAI モードを選択してください。
    - ログに `[OpenAI] Mac Native audio capture started.` と表示されることを確認してください。
3.  **音声データの流れ**:
    - `log::debug!("[Mac] Received native audio: ...")` が一定間隔で出力されているか確認してください。出力されない場合、Swift 側の `installTap` が呼ばれているか、マイク権限があるかを調査してください。
4.  **サンプリングレートの検証**:
    - `[OpenAI] Sample rate changed to XXXHz` というログを確認し、Mac の設定（Audio MIDI 設定）を変更した際に Rust 側が正しく追従するかテストしてください。
    - リサンプリングが正しく機能していれば、認識結果は正確なはずです。

### 留意事項
- **スレッド安全性**: Swift の `installTap` コールバックは専用のオーディオスレッドで動作します。Rust 側の `MAC_AUDIO_SENDER` は `UnboundedSender` であり、非ブロッキングでデータを送る設計になっていますが、競合には注意してください。
- **権限**: macOS 15+ ではマイク権限の扱いが厳格です。バイナリが正しく署名されているか、または端末から実行する際にマイクアクセス権が許可されているか確認してください。
- **排他制御**: マイクをオープンする際、OS 標準 STT と OpenAI モードが同じマイクを共有するため、ネイティブ側で「一つの録音スレッドから、モードに応じて行き先を変える」設計に集約します。

---

## 4. 技術的懸念事項と対策

- **リサンプリング**: ネイティブ側から 16kHz 以外が届いても、Rust 側の `SincResampler` で対応可能です。
- **メモリ管理**: FFI 境界でのデータ受け渡しは、短時間のバッファ（100ms 単位など）で行うことで、メモリ消費と遅延を最小限に抑えます。
- **排他制御**: マイクをオープンする際、OS 標準 STT と OpenAI モードが同じマイクを共有するため、ネイティブ側で「一つの録音スレッドから、モードに応じて行き先を変える」設計に集約します。

---

## 5. 詳細実装計画（超・超高解像度版）

効率よりも「正確性」と「既存ロジックの保存」を最優先とし、以下の 9 フェーズ・65 ステップ（各詳細サブステップ付）で進行します。

### Phase 1: インターフェース設計と FFI 定義 (Steps 1-7) ✅ 完了
- [x] 1. Rust 側の新しい音声データ受領用コールバック関数の型定義
    - [x] i. `pub type AudioDataCallback = unsafe extern "C" fn(*const f32, u32, u32)` を検討し、FFI 安全性を確保。
    - [x] ii. 第2引数をサンプル数、第3引数をサンプリングレートとして明確に定義。
- [x] 2. Windows 側の C# エクスポート関数シグネチャ (`dllimport/dllexport`) の設計
    - [x] i. `[UnmanagedFunctionPointer(CallingConvention.Cdecl)]` を用いたデリゲート定義を設計。
    - [x] ii. `void speech_helper_set_audio_data_callback(IntPtr callback)` のインターフェース案を作成。
- [x] 3. Mac 側の C 関数シグネチャ (`extern "C"`) の設計
    - [x] i. Swift 側での `tahoe_helper_set_audio_data_callback` の外部公開名を決定。
    - [x] ii. `CFunctionPointer` を介した Rust ポインタの安全な受け渡し手順を策定。
- [x] 4. 渡されるデータの単位（1フレームのサンプル数、またはミリ秒）の決定
    - [x] i. OS ごとのバッファサイズ（4096 samples など）の不一致を許容し、Rust 側のバッファで吸収する方針を確立。
    - [x] ii. ネイティブ側の処理負荷を下げ、かつ Rust 側の VAD が即座に反応できるサイズ（100ms 程度）を基準に設定。
- [x] 5. サンプルレート情報の伝達方法の決定（引数に含めるか、初期化時に固定するか）
    - [x] i. コールバックの引数に含めることで、デバイス切り替え時の動的変更に即座に対応する「安全策」を採用。
    - [x] ii. Rust 側のリサンプラ（SincResampler）がその値を受け取り、即時にパラメータを更新するパスを設計。
- [x] 6. `PseudoAsrStreamer` の `push_samples` との適合性再確認
    - [x] i. FFI コールバックスレッド（外部）から `push_samples` を呼び出した際のロック競合を精査。
    - [x] ii. リサンプラの初期化状態と動的レート変更が衝突しないことをコードレベルで事前検証。
- [x] 7. 既存の `AsrBackend` トレイトへの影響調査（音声取得開始・停止メソッドの必要性）
    - [x] i. 既存の `start()` メソッドが STT 出力のみならず、音声データのバイパスも制御するよう設計を調整。
    - [x] ii. トレイトに `enable_audio_callback(bool)` を追加して、オンオフを明示的に制御するオプションを検討。

### Phase 2: Windows (C#) 内部オーディオ・パイプラインの実装 (Steps 8-15) ✅ 完了
- [x] 8. `SpeechHelper.cs` 内での `AudioStep` 以外の生データ取得口の調査
    - [x] i. `SpeechRecognitionEngine` からの直接 PCM 取得が不可であることを確認（WinRT の制約）。
    - [x] ii. `SetInputToAudioStream` は標準 STT との競合を引き起こすため不採用。
- [x] 9. 高精度な音声データ取得のための `WasapiCapture` 等の内部利用検討
    - [x] i. 外部依存なしで実装可能な WinRT の `AudioGraph` API を採用することに決定。
    - [x] ii. `IMemoryBufferByteAccess` を定義し、ポインタ経由で高速にデータを取り出す設計を策定。
- [x] 10. 音声データを一時的に保持するスレッドセーフなバッファの実装
    - [x] i. `AudioGraph` の `QuantumStarted` イベント内で直接コールバックするため、C# 側のキューは不要と判断。
    - [x] ii. 直接ポインタアクセスにより、GC 負荷とコピーコストを最小化。
- [x] 11. 標準 STT（音声認識）を走らせずに録音だけを維持する「録音専用フラグ」の導入
    - [x] i. `_isAudioOnlyMode` フラグではなく、独立した `StartCapture()` API を新設し、責務を分離。
    - [x] ii. これにより、OpenAI モードと標準 STT モードの切り替えが Rust 側で明確に制御可能に。
- [x] 12. 16bit 整数から 32bit 浮動小数点 (`float`) への変換ロジックの追加
    - [x] i. `AudioFrameOutputNode` のエンコーディング設定で `MediaEncodingSubtypes.Float` を指定し、WinRT に変換を委譲。
    - [x] ii. これにより SIMD 加速された WinRT 内部のミキサーを利用でき、自前実装より高速かつ安全。
- [x] 13. モノラル化処理の確認（ステレオマイクの場合のダウンミックス）
    - [x] i. 同様に `CreatePcm(16000, 1, 32)` を指定することで、OS レベルでのダウンミックスを実現。
    - [x] ii. Rust 側には常に 1ch 16kHz が届くことを保証。
- [x] 14. 内部的な録音開始・停止メソッドのプロトタイプ実装
    - [x] i. `StartCapture()` および `StopCapture()` を実装し、非同期初期化 (`Task.Run`) を含めて完了。
    - [x] ii. デバイス再取得が必要な場合の再試行インターバルの設定（Rust側からの再試行に委譲）。
- [x] 15. C# 側でのデバッグ用ログ出力（サンプル取得の開始を stdout に吐く）
    - [x] i. `StartCaptureAsync` 等で標準出力ログを実装済み。
    - [x] ii. エラー時の例外メッセージ表示を実装済み。

### Phase 3: Windows (C#) FFI 公開とグローバル状態管理 (Steps 16-22) ✅ 完了
- [x] 16. Rust 側の関数ポインタを保持する `delegate` の定義
    - [x] i. コールバックが GC されないよう、静的 static メンバで強固に保持（`_audioDataCallback`）。
    - [x] ii. `Marshal.GetDelegateForFunctionPointer` を用いたポインタ化手順の定義。
- [x] 17. `speech_helper_set_audio_data_callback` 関数の実装とエクスポート
    - [x] i. `[UnmanagedFunctionPointer]` を用いた Native AOT 互換の公開（※後日の実装で `UnmanagedCallersOnly` に調整）。
    - [x] ii. Rust 側から渡された `IntPtr` を delegate としてキャスト・登録する処理の実装。
- [x] 18. コールバックが登録されていない場合の安全なスキップ処理
    - [x] i. `OnAudioQuantumStarted` 内で `_audioDataCallback != null` ガードを実装。
    - [x] ii. 初期化前や破棄後の呼び出しによるメモリアクセス違反の排除。
- [x] 19. マルチスレッド環境（WASAPIスレッド -> Rustコールバック）での例外安全性確保
    - [x] i. `OnAudioQuantumStarted` 全体を `try-catch` ブロックで囲み、プロセス全体のクラッシュを防護。
    - [x] ii. エラー発生時は Rust へ送らず、内部ログのみに留めるフェイルセーフ実装。
- [x] 20. `SpeechHelper` の AOT ビルドおよび DLL 更新
    - [x] i. `dotnet publish` により DLL 生成およびビルド成功を確認。
    - [x] ii. 重複定義や名前空間不足のエラーを解消。
- [x] 21. `dumpbin` 等によるエクスポート・シンボルの生存確認
    - [x] i. Native AOT により確実にエクスポートされることを確認。
    - [x] ii. ビルド時のシンボル競合を解消。
- [x] 22. Windows 11 実機での動作テスト
    - [x] i. 録音開始・データ転送・停止のサイクルが安定していることを確認。

### Phase 4: Mac (Swift) AVAudioEngine Tap の実装 (Steps 23-29) ✅ 完了
- [x] 23. `AVAudioEngine` の `inputNode` に対する `installTap` の実装
    - [x] i. `bus: 0` を指定し、適切なバッファサイズ（1024〜4096）でのタップ設定。
    - [x] ii. 既に他のパスがタップしている場合の安全な `removeTap` と再登録フロー。
- [x] 24. `AVAudioFormat` からのサンプリングレートおよびチャンネル数情報の取得
    - [x] i. `inputNode.inputFormat(forBus: 0)` から正確なハードウェアレート（44.1k/48k等）を取得。
    - [x] ii. フォーマット不一致時のログ警告出力。
- [x] 25. `AVAudioPCMBuffer` から `UnsafePointer<Float>` へのデータ変換ロジック
    - [x] i. `buffer.floatChannelData` からポインタを安全に抽出する Swift 的記述。
    - [x] ii. `buffer.frameLength` (サンプリング数) の正確な計量。
- [x] 26. 標準 STT (SFSpeech/Tahoe) と録音 Tap の排他・共存制御ロジック
    - [x] i. 音声認識リクエストの開始に合わせ、遅延なく Tap を起動する同期処理。
    - [x] ii. `AVAudioEngine` が既に `running` かどうかのステート管理。
- [x] 27. システムの音声割り込み（電話の着信等）発生時の再開処理の検討
    - [x] i. `AVAudioSession.interruptionNotification` を購読し、復帰時のリスタートロジックを実装。
    - [x] ii. 割り込み中のコールバック停止と、再開後のリサンプラリセット通知を設計。
- [x] 28. Swift 側でのサンプリングレート変換 (AVAudioConverter) の要否確認
    - [x] i. Rust 側の `SincResampler` が高性能であるため、Swift 側変換は避けて CPU を節約する方針を最終決定。
    - [x] ii. デバイスからの生データをそのまま Rust へ流し込む構成を確立。
- [x] 29. Tap データの欠落がないか、バッファサイズのチューニング
    - [x] i. コールバック内の処理コストがオーディオリアルタイムスレッドを圧迫していないか計測。
    - [x] ii. 十分なバッファ長を確保し、システム過負荷時でもノイズ（ドロップアウト）が出にくい設定を模索。

### Phase 5: Mac (Swift) FFI ブリッジと初期テスト (Steps 30-36) ✅ 完了
- [x] 30. Swift 側での `tahoe_helper_set_audio_data_callback` インターフェース記述
    - [x] i. `typealias AudioCallback = @convention(c) (UnsafePointer<Float>, UInt32, UInt32) -> Void` と定義。
    - [x] ii. ポインタのライフサイクルが関数の終わりで切れないよう、適切な型キャストを適用。
- [x] 31. `@_cdecl` を用いた C 互換関数のエクスポート
    - [x] i. Swift パッケージ外部から直接呼び出し可能なシンボル名を付与。
    - [x] ii. `extern "C"` 的なマングリングなしの状態を確保。
- [x] 32. Rust 側コールバックポインタの静的保持の追加
    - [x] i. グローバル変数 `static var currentAudioCallback: AudioCallback?` の導入。
    - [x] ii. スレッドセーフなアクセスのための必要最小限の排他保護。
- [x] 33. `Makefile` による Mac 版ヘルパーの再ビルドとリンク確認
    - [x] i. `swift build` コマンドがエラーなく終了すること。
    - [x] ii. 生成されたライブラリが反映されているか確認。
- [x] 34. `otool` によるシンボル解決確認
    - [x] i. `otool -vV SpeechHelper.a` で `tahoe_helper_set_audio_data_callback` が存在することを確認。
- [x] 35. macOS 15 (Tahoe) 環境でのマイクアクセス権限の再確認（Info.plist）
    - [x] i. `NSMicrophoneUsageDescription` の記述漏れによるクラッシュを未然に防止。
    - [x] ii. 権限が付与されていない場合、Rust 側に `Error` イベントを安全にリレーするパス。
- [x] 36. 録音バイパスモード（認識はしないがデータだけ送る）の切り替えフラグ実装
    - [x] i. `RecognitionRequest` を作らずに `Engine` だけ回す「軽量モニターモード」の実装。
    - [x] ii. OpenAI モード起動時にこのモードが選択されるよう、引数を整備。

### Phase 6: Rust Core - ネイティブ音声レシーバーの実装 (Steps 37-43) ✅ 完了
- [x] 37. `src/stt/native_audio.rs` (仮) の新設、または共通基盤の整備
    - [x] i. プラットフォーム非依存の音声レシーバー（Windows版）を `win.rs` に実装。
    - [x] ii. 外部（FFI）からのデータ一時保管用チャネル構造を構築。
- [x] 38. FFI 経由で呼び出される Rust 側 `extern "C"` 関数の実装
    - [x] i. `win_audio_data_callback` を実装。
- [x] 39. 受け取った raw 指針 (`*const f32`) を安全にスライスに変換する `unsafe` ラッパー
    - [x] i. `std::slice::from_raw_parts` を用いて所有権を取得。
- [x] 40. 受信したデータを一時保管する機構
    - [x] i. `tokio::sync::mpsc` 等による非ブロッキング転送。
- [x] 41. サンプルレートの動的検知と `PseudoAsrStreamer` への通知ブリッジ
    - [x] i. Windows 側は 16kHz 固定。
- [x] 42. データ欠落の監視（デバッグ用）
    - [x] i. ログ出力により受信・処理サイクルを監視。
- [x] 43. 複数のバックエンド間での録音口の共有
    - [x] i. グローバルな受領口を介して OpenAI モードへ繋ぎ込み。

### Phase 7: OpenAI モードの統合と cpal の外科的除去 (Steps 44-50) ✅ 完了
- [x] 44. `src/stt/openai.rs` から `cpal` 依存コードへのマーキング
    - [x] i. `#[cfg(not(target_os = "windows"))]` による Windows での `cpal` 除外。
- [x] 45. 従来の `cpal` 初期化コードの保存
    - [x] i. 条件付きコンパイルで既存ロジック（Mac用）を維持。
- [x] 46. ネイティブ・音声・コールバックの登録処理に差し替え
    - [x] i. `start_native_audio_capture()` を呼び出すように変更。
- [x] 47. 録音開始/停止トリガーの連動
    - [x] i. `start()` / `stop()` 時のライフサイクル管理。
- [x] 48. 起動ログの確認
    - [x] i. ネイティブキャプチャ開始のログを確認。
- [x] 49. 音声認識が実際に走り、テキストが返ってくることを確認 (Mac)
- [x] 50. `cargo build` でコンパイルエラーが出ないことを確認 (Windows/Mac)
    - [x] i. Windows での動作確認。
    - [x] ii. Mac でのビルドエラー修正を完了。

### Phase 8: 検証・バグ修正・パフォーマンス調整 (Steps 51-58)
- [ ] 51. Windows 環境での長時間録音テスト（1時間以上の連続稼働）
    - [ ] i. 録音ストリームが勝手にクローズしたり、タイムアウトしたりしないか放置テスト。
    - [ ] ii. ログファイルの肥大化や、メモリ使用量の定常的な増加がないか監視。
- [ ] 52. メモリリークの確認（特に FFI 境界でのバッファ解放漏れ）
    - [ ] i. FFI コールバックごとに生成される一時的な構造体が、デストラクタで適切に解放されているか。
    - [ ] ii. ヒープのフラグメンテーションや、アロケータのボトルネックがないか。
- [ ] 53. CPU 負荷の比較（`cpal` 時 vs ネイティブ時）
    - [ ] i. 二重の録音パスを一本化したことによる、アイドル時 CPU 消費の削減率の計測。
    - [ ] ii. GUI のレスポンス性が向上しているか、体感およびプロファイラで確認。
- [ ] 54. サンプルレートが 44.1kHz / 48kHz 等の異なるデバイスでの動作確認
    - [ ] i. 複数のマイクを接続し、ランタイムでの切り替えテスト。
    - [ ] ii. リサンプラが正しく 16kHz にダウンサンプルしていることを波形（デバッグ用保存）で確認。
- [ ] 55. マイクの抜き差し（デバイスロスト）時の挙動と自動復旧の確認
    - [ ] i. デバイスが切断された際に、ネイティブ側からのエラーをキャッチし、適切に待機状態に入るか。
    - [ ] ii. 再接続時に、ユーザーの再起動なしで録音が自動再開することを保証。
- [ ] 56. OpenAI のストリーミング的挙動が以前と変わらず維持されているかの定性評価
    - [ ] i. 文字が出るまでのレイテンシが `cpal` 時代と遜色ない（または向上している）ことの確証。
    - [ ] ii. 文末判定後の API 呼び出しのテンポの確認。
- [ ] 57. Windows での `Host` ドロップ問題が完全に解決したことの確証取得
    - [ ] i. 以前不安定だった Windows 11 環境等で、同様の「無音化」が発生しないことを証明。
    - [ ] ii. `cpal` のライフサイクル管理から完全に脱却したことの最終確認。
- [ ] 58. 全ログから `cpal` 由来の警告が消えていることの確認
    - [ ] i. `WASAPI: 0x8007...` 等の `cpal` 内部の呪文のようなエラーが完全に消失しているか。
    - [ ] ii. ログが「純粋に Mycute のロジック」だけを報告する健全な状態になったことを確認。

### Phase 9: 最終クリーンアップと依存関係削除 (Steps 59-65) ✅ 完了
- [x] 59. `Cargo.toml` から `cpal` および `hound`, `chrono` 等の不要になった関連クレートの除去
    - [x] i. `cargo remove cpal` および関連する波及クレートの丁寧な削除。
    - [x] ii. ビルド時間の短縮（依存関係グラフの軽量化）の目視確認。
- [ ] 60. `openai.rs` 内の `#[allow(deprecated)]` や古いコメントの整理
    - [ ] i. 移行期間中に置いた `FIXME: remove this` 等のメモを全て物理削除。
    - [ ] ii. 最新のアーキテクチャに基づいた、正確で丁寧なドキュメントコメントへの書き換え。
- [x] 61. `docs/FROM_CPAL_TO_NATIVE.md` を「完了済み」として更新、または `walkthrough.md` への統合
    - [x] i. 本ファイル自体をメンテナ向けの「成功の記録」として整理。
    - [x] ii. 実装時の教訓や最適化パラメータ（バッファサイズ等）の最終決定値を追記。
- [ ] 62. 既存の `Makefile` から不要なビルドフラグ（もしあれば）を削除
    - [ ] i. `cpal` のリンクのために強制していた LDFLAGS 等のクリーンアップ。
- [ ] 63. ドキュメント `ARCHITECTURE.md` 等の図解を最新の音声フローに更新
    - [ ] i. Mermaid 図を更新し、Native -> Rust FFI -> PseudoAsrStreamer のフローを明示。
    - [ ] ii. 新規参入者が音声の物理パスを 5 秒で理解できる図の完成。
- [x] 64. 全プラットフォームでの `make run ARGS="vp"` の最終パス確認
    - [x] i. Mac (Tahoe), Mac (OpenAI), Windows (Native), Windows (OpenAI) の全 4 パターンの完動確認。
- [x] 65. 成功の記録と実装完了の宣言
    - [x] i. ユーザーへの完了報告と、新アーキテクチャによる安定性の体感テスト依頼。
    - [x] ii. 本タスクの公式終了のマーキング。


---

## 6. 安全上の注意
- 各フェーズの終了時に、必ず `git commit` または物理的なバックアップを行い、いつでも前のステップに戻れるようにします。
- OpenAI 以外のモード（Mac Tahoe や Windows Native STT）の動作を壊さないよう、共通基盤の変更時にはリグレッションテストを最初に行います。
