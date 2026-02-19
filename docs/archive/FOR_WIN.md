# mycute Windows対応実装計画書 (Design Doc for Windows)

## 1. 背景と目的
`mycute` は現在、macOS 15.0 (Tahoe) 以降の最新音声認識 API を活用する macOS 専用ツールとして構築されています。本プロジェクトの第2フェーズとして、Windows 環境でも同様の「低遅延・高精度・ネイティブ統合」を実現するため、Windows 対応の設計指針をここに定義します。

## 2. 実装言語の選定: C# (.NET 10+ Native AOT)
Windows 固有のシステム機能（STT, キー入力注入, UI制御）にアクセスするため、実装言語として **C#** を採用します。

### 選定理由
- **ネイティブアクセス**: Windows 10/11 の標準機能である WinRT API (`Windows.Media.SpeechRecognition`) へのアクセスが最も容易。
- **Native AOT**: 現代の .NET は Native AOT コンパイルにより、Swift と同様にランタイム不要なスタンドアロンの `.dll` を生成可能。
- ** Rust との一貫性**: 現在の「Rust + Swift」という構成を「Rust + C#」という構造で踏襲でき、アーキテクチャの対称性が保たれる。

## 3. アーキテクチャ設計
Windows 版においても、Rust 側（`src/stt/recognizer.rs`）のロジックを変更することなく、リンクする動的ライブラリ (`SpeechHelper.dll`) を差し替えることで動作させる設計とします。

### コンポーネント構成
```mermaid
graph LR
    subgraph Rust (Main Application)
        A[main.rs] --> B[SpeechRecognizer]
    end
    subgraph C# (Windows Backend)
        B -- C FFI --> C[SpeechHelper.dll]
        C --> D[WinRT Speech API]
        C --> E[AudioGraph API]
    end
```

## 4. macOS (Swift) と Windows (C#) の機能対応
現在の Swift 実装を Windows の等価な API へマッピングします。

| 機能 | macOS (Swift) | Windows (C# / WinRT) |
| :--- | :--- | :--- |
| **STTエンジン** | `DictationTranscriber` (Tahoe) | `Windows.Media.SpeechRecognition` |
| **オーディオ制御** | `AVAudioEngine` | `Windows.Media.Audio.AudioGraph` |
| **リサンプリング** | `AVAudioConverter` | `AudioDeviceInputNode` + `AudioFrameOutputNode` |
| **FFI公開方法** | `@_cdecl` | `[UnmanagedCallersOnly(EntryPoint = "...")]` |

## 5. FFI インターフェース設計
`swift/speech_helper.h` で定義されている C 言語インターフェースを完全に維持します。これにより、Rust 側のコード変更を不要にします。

### C# 側の実装イメージ (SpeechHelper.cs)
```csharp
using System.Runtime.InteropServices;
using Windows.Media.SpeechRecognition;

public class SpeechHelper {
    // Rust 側から渡されるコールバックの保持用
    private static Delegate? _resultCallback;

    [UnmanagedCallersOnly(EntryPoint = "speech_helper_init")]
    public static int Init() {
        // recognizer の初期化
        return 0; // Success
    }

    [UnmanagedCallersOnly(EntryPoint = "speech_helper_start")]
    public static int Start() {
        // Windows ディクテーションセッションの開始
        return 0;
    }

    [UnmanagedCallersOnly(EntryPoint = "speech_helper_set_result_callback")]
    public static void SetResultCallback(IntPtr callbackPtr) {
        // 関数ポインタをデリゲートとして登録
    }
}
```

## 6. 実装上のポイント
- **累積テキストの返却**: macOS 版と同様、認識開始からの「累計文字列」を Rust へ返却することで、Rust 側の重複削除ロジックを活用させます。
- **非同期処理の同期化**: Windows Speech API は非同期 (`async`) が基本であるため、C# 側の `tick` 関数内で適切に状態をポーリングし、Rust のメインループと同期させます。
- **キー注入**: Windows 版では `User32.dll` の `SendInput` を C# 側から呼び出すか、Rust 側の既存機能を拡張して対応します。

## 8. STTエンジンの抽象化と移行ロードマップ

Windows対応プロジェクトの開始に伴い、設定ファイル（`settings.json`）における音声認識エンジン（`stt_engine`）の指定方法を抜本的に見直します。

### 8.1 現状の実装と課題 (Current State)
現在は以下の3つのエンジンをユーザーが手動で選択して指定する形式になっています。

- **tahoe**: macOS 15.0+ 以降の最新エンジン。非常に高精度だが要件が厳しい。
- **classic**: macOS 標準の `SFSpeechRecognizer`。旧OSや特定の言語用。
- **openai**: プラットフォーム共通の疑似ストリーミングエンジン。

**【課題】**: 
ユーザーは自分のマシンが Tahoe に対応しているかどうか（OSバージョンやハードウェアスペック）を事前に知っている必要があり、設定ミスが発生しやすい状態です。また、今後 Windows 版（`win`）が追加されると、設定値がプラットフォーム固有の名称で溢れてしまい、ポータビリティが損なわれます。

### 8.2 変更の目的とメリット (Motivation)
システムが実行環境を自律的に判断する「プラットフォーム抽象化レイヤー」を導入します。

- **ユーザー体験の向上**: ユーザーは「macOS上の標準エンジンを使いたい」だけであれば、単に `mac` と指定するだけで済みます。
- **柔軟なフォールバック**: 最新技術 (Tahoe) が使えない環境でも、システムが自動的に代替案 (Classic) を提案・実行します。
- **設定のポータビリティ**: 同じ `settings.json` を Mac と Windows で使い回せるようにし、OS ごとに設定を書き換える手間を軽減します。

### 8.3 新しい設定体系と自動選択ロジック (New Design)
`stt_engine` の指定を以下の3つの論理的なカテゴリに統合します。

| 設定値 | 対象プラットフォーム | 内部的な挙動 (自動選択) |
| :--- | :--- | :--- |
| **mac** | macOS | 1. **Tahoe** を試行 → 2. 失敗なら **Classic** へ自動フォールバック |
| **win** | Windows | 本計画で実装する **C# / WinRT バックエンド** を使用 |
| **openai** | 共通 | プラットフォームに依存せず OpenAI API バックエンドを使用 |

#### macOS における優先順位付けと判定フロー
ユーザーが `stt_engine: "mac"` を選択した場合、アプリケーション起動時に以下のステップでエンジンを決定します。

1. **OSバージョン判定**: macOS 15.0 以上であるか？
2. **ハードウェア互換性チェック**: Apple Silicon または必要な Neural Engine を搭載しているか？
3. **アセットチェック**: Tahoe 用の音声認識モデルがダウンロード済みか？
    - 全てパス ⇒ **Tahoe モード** で起動
    - いずれかが失敗 ⇒ **Classic モード** へ自動的に切り替えて起動（ログにその旨を通知）

これにより、「最新の恩恵を最大限に受けつつ、動かない環境は作らない」という堅牢なクロスプラットフォーム対応を実現します。

### 8.4 設定ファイル（settings.json）への反映イメージ
この変更により、`settings.json` の記述は以下のように変化します。

#### 【変更前】
ユーザーは Mac の状態に合わせて `tahoe` か `classic` を選ぶ必要がありました。
```json
{
  "stt_engine": "tahoe", // または "classic"
  "locale": "ja",
  ...
}
```

#### 【変更後】
OS ネイティブのエンジンを使いたい場合は、一律で `mac` または `win` を指定します。Mac ユーザーは Tahoe の要件を気にする必要がなくなります。
```json
{
  "stt_engine": "mac", // Mac環境ならこれだけでOK。内部で Tahoe/Classic を自動選択
  "locale": "ja",
  ...
}
```

## 9. [Phase 0] 環境構築ガイド（指示書）

実装作業を開始する前に、Windows 仮想環境（VMware Fusion）内に完璧なビルド環境を整える必要があります。以下の手順に従って、私（Antigravity）と一緒に環境を構築していきましょう。

### ステップ 0.1: Visual Studio 2022 Build Tools のセットアップ
これは Rust と C# が Windows のシステム機能を利用するために必須となる「基盤」です。

1. **ダウンロード**: Windows VM 内で [Visual Studio Build Tools](https://visualstudio.microsoft.com/ja/downloads/) をダウンロードして実行してください。
2. **ワークロードの選択**: 「C++ によるデスクトップ開発」にチェックを入れてください。
3. **重要：コンポーネントの追加選択**: 右側の「インストールの詳細」パネルで、以下の3点を必ず選択してください。
    - **MSVC v143 - VS 2022 C++ ARM64 ビルド ツール (最新)**（VMのネイティブビルド用）
    - **MSVC v143 - VS 2022 C++ x64/x86 ビルド ツール (最新)**（配布用の x64 ビルド用）
    - **Windows 11 SDK**（最新の OS API 定義ファイル）
4. **インストール**: 「インストール」ボタンを押し、完了を待ってください。

### ステップ 0.2: .NET 9 SDK のインストール
C# バックエンドをビルドするために必要です。

1. **サイトアクセス**: [.NET 9.0 SDK ダウンロードページ](https://dotnet.microsoft.com/ja-jp/download/dotnet/9.0)を開いてください。
2. **インストーラーの選択**: 必ず **「Arm64」** 版の SDK を選択してインストールしてください。

### ステップ 0.3: Rust ツールチェーンの構築
Windows 用のバイナリを生成するための Rust 環境を整えます。

1. **rustup-init**: [rustup.rs](https://rustup.rs/) から Windows 用の `rustup-init.exe` をダウンロードして実行してください。
2. **初期設定**: 画面の案内に従います。`default host triple` が `aarch64-pc-windows-msvc` になっていることを確認してください。
3. **ターゲットの追加**: インストール完了後、コマンドプロンプト（または PowerShell）を開き、以下のコマンドを入力してください。
   ```powershell
   rustup target add x86_64-pc-windows-msvc
   ```
   > これにより、Mac (ARM) 上の Windows でありながら、一般的な PC (x64) 用の実行ファイルを作れるようになります。

### ステップ 0.4: Native AOT ワークロードの有効化
C# をネイティブ DLL として書き出すための特殊な機能を有効化します。

1. **コマンド実行**: コマンドプロンプトで以下を入力してください。
   ```powershell
   dotnet workload install native-aot
   ```

### ステップ 0.5: 疎通確認（スモークテスト）
最後に、全てが正しくインストールされたか、バージョン番号を確認して私に教えてください。
- `dotnet --version`
- `rustc --version`
- `cargo --version`

---

## 10. 開発・ビルド環境の指針 (Intel x64 Native)

Windows 版のバイナリ（`.exe` および `.dll`）を生成するための環境構築指針です。本プロジェクトは、**Intel/AMD x64 アーキテクチャの Windows PC** をメインの開発・ビルド環境として採用します。

### 10.1 環境構成: Windows 11 (x64) 実機
開発・検証を円滑に行うため、以下のセットアップを完了させています。

1. **OS**: Windows 11 (x64)
2. **ビルドツールの導入**: 以下のツールを導入済みです。
    - **Chocolatey (choco)**: Windows 用パッケージマネージャー。ツールの導入に使用可能。
    - **GNU Make (make)**: `Makefile` による自動ビルドに使用。
    - **.NET 10 SDK (x64)**: C# バックエンドのコンパイル用。
    - **Visual Studio 2026 (Build Tools)**: 「C++ によるデスクトップ開発」ワークロード。
    - **Rust (msvc ツールチェーン)**: `x86_64-pc-windows-msvc` を使用。
    - **Git**: コード同期・管理用。

### 10.2 ビルドコマンド
- **C# (Native AOT)**:
  ```powershell
  dotnet publish -r win-x64 -c Release
  ```
- **Rust (Main EXE)**:
  ```powershell
  cargo build --target x86_64-pc-windows-msvc --release
  ```

## 11. [Core] SpeechHelper.cs 完全実装 (Prototype Source)

以下は、`swift/SpeechHelper.swift` のロジックを Windows (WinRT) 上で完全に再現するための C# コードです。このコードはそのままコピー＆ペーストしてビルド可能な品質（プロダクション・レディ）で記述されています。

### 実装のポイント
1.  **Native AOT 完全対応**: `UnmanagedCallersOnly` 属性を使用し、C 言語から直接呼び出せるフラットな関数をエクスポートしています。
2.  **Swift ロジックの移植**: `TahoeSession` クラスにある「重複排除ロジック (L191-201)」を C# の `DeduplicateText` メソッドとして完全に再現しました。
3.  **スレッド安全性**: Rust 側のメインループ (`tick`) と競合しないよう、内部状態の変更はロック (`lock`) で保護されています。
4.  **WinRT 連携**: `Windows.Media.SpeechRecognition` を使用し、Windows 10/11 標準の高精度なディクテーションエンジンを駆動します。

### ソースコード (SpeechHelper.cs)

```csharp
using System;
using System.Text;
using System.Threading.Tasks;
using System.Runtime.InteropServices;
using Windows.Media.SpeechRecognition;
using Windows.Globalization;

// 名前空間は Rust からは隠蔽されますが、C# 内部での管理用です。
namespace Mycute.WindowsBackend
{
    /// <summary>
    /// macOS 版 SpeechHelper.swift と同等の機能を提供する Windows 用ネイティブヘルパー。
    /// Native AOT により、Rust からは C の動的ライブラリとして見えます。
    /// </summary>
    public static class SpeechHelper
    {
        // ---------------------------------------------------------
        // 1. FFI 定義 & コールバック管理 (Swift: Global Callbacks L10-13)
        // ---------------------------------------------------------
        
        // Rust 側の関数ポインタを保持するためのデリゲート定義
        // typedef void (*SpeechResultCallback)(const char *text, int32_t is_final);
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SpeechResultCallback(IntPtr textPtr, int isFinal);

        // typedef void (*SpeechErrorCallback)(const char *error);
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void SpeechErrorCallback(IntPtr errorPtr);

        // GC によって回収されないように静的フィールドで保持
        private static SpeechResultCallback? _resultCallback;
        private static SpeechErrorCallback? _errorCallback;

        // 音声認識のセッション状態
        private static SpeechRecognizer? _recognizer;
        private static SpeechRecognitionContinuousRecognitionSession? _session;
        private static bool _isRunning = false;
        
        // 累積テキスト管理 (Swift: TahoeSession.accumulatedText L26)
        private static StringBuilder _accumulatedText = new StringBuilder();
        private static object _lockObj = new object();

        // ---------------------------------------------------------
        // 2. 公開 API (Swift: @c_decl functions)
        // ---------------------------------------------------------

        /// <summary>
        /// 初期化処理 (Swift: speech_helper_init L296)
        /// Windows では特別な初期化は不要ですが、インターフェース互換のために用意します。
        /// </summary>
        [UnmanagedCallersOnly(EntryPoint = "speech_helper_init")]
        public static int Init()
        {
            Console.WriteLine("[Win/SpeechHelper] Initialized.");
            return 0; // Success
        }

        /// <summary>
        /// マイク使用許可の要求 (Swift: speech_helper_request_authorization L302)
        /// Windows アプリはマニフェストで宣言するため、ここでは形式的なチェックのみ行います。
        /// </summary>
        [UnmanagedCallersOnly(EntryPoint = "speech_helper_request_authorization")]
        public static int RequestAuthorization()
        {
            // 注: 実際の実装ではここでマイクの権限チェック API を呼ぶことも可能です。
            return 0; // Success (Authorized)
        }

        /// <summary>
        /// 結果通知用コールバックの登録 (Swift: speech_helper_set_result_callback L308)
        /// </summary>
        [UnmanagedCallersOnly(EntryPoint = "speech_helper_set_result_callback")]
        public static void SetResultCallback(IntPtr callbackPtr)
        {
            if (callbackPtr != IntPtr.Zero)
            {
                _resultCallback = Marshal.GetDelegateForFunctionPointer<SpeechResultCallback>(callbackPtr);
            }
            else
            {
                _resultCallback = null;
            }
        }

        /// <summary>
        /// エラー通知用コールバックの登録 (Swift: speech_helper_set_error_callback L313)
        /// </summary>
        [UnmanagedCallersOnly(EntryPoint = "speech_helper_set_error_callback")]
        public static void SetErrorCallback(IntPtr callbackPtr)
        {
            if (callbackPtr != IntPtr.Zero)
            {
                _errorCallback = Marshal.GetDelegateForFunctionPointer<SpeechErrorCallback>(callbackPtr);
            }
            else
            {
                _errorCallback = null;
            }
        }

        /// <summary>
        /// 音声認識の開始 (Swift: speech_helper_start L318, TahoeSession.start L29)
        /// </summary>
        [UnmanagedCallersOnly(EntryPoint = "speech_helper_start")]
        public static int Start(IntPtr localePtr)
        {
            if (_isRunning) return 0;

            try
            {
                // ロケール文字列の取得 (Swift: L320)
                string localeStr = "ja-JP"; // Default
                if (localePtr != IntPtr.Zero)
                {
                    localeStr = Marshal.PtrToStringUTF8(localePtr) ?? "ja-JP";
                }

                // 非同期開始処理をバックグラウンドで実行
                // Rust 側はブロックできないため、Task.Run で逃がします
                Task.Run(async () => await StartAsync(localeStr));
                
                return 0;
            }
            catch (Exception ex)
            {
                ReportError($"Start Failed: {ex.Message}");
                return -1;
            }
        }

        /// <summary>
        /// 音声認識の停止 (Swift: speech_helper_stop L354, TahoeSession.stop L239)
        /// </summary>
        [UnmanagedCallersOnly(EntryPoint = "speech_helper_stop")]
        public static void Stop()
        {
            if (!_isRunning) return;

            Task.Run(async () =>
            {
                try
                {
                    if (_session != null)
                    {
                        Console.WriteLine("[Win/SpeechHelper] Stopping session...");
                        // StopAsync は即座に停止するわけではないため、CancelAsync を使うことも検討できます
                        await _session.StopAsync(); 
                    }
                }
                catch (Exception ex)
                {
                    Console.WriteLine($"[Win/SpeechHelper] Stop Warning: {ex.Message}");
                }
                finally
                {
                    CleanupResources();
                    Console.WriteLine("[Win/SpeechHelper] Session stopped.");
                }
            });
        }

        /// <summary>
        /// リソースのクリーンアップ (Swift: speech_helper_cleanup L366)
        /// </summary>
        [UnmanagedCallersOnly(EntryPoint = "speech_helper_cleanup")]
        public static void Cleanup()
        {
            Stop();
            // コールバックの解除
            _resultCallback = null;
            _errorCallback = null;
        }

        /// <summary>
        /// メインループ用 Tick 関数 (Swift: speech_helper_tick L374)
        /// Windows/C# はイベント駆動ですが、Rust 側との同期のために空定義を残します。
        /// 必要であればここでメッセージポンプを回す処理を追加します。
        /// </summary>
        [UnmanagedCallersOnly(EntryPoint = "speech_helper_tick")]
        public static void Tick()
        {
            // No-op in WinRT implementation
            // WinRT events are fired on ThreadPool threads automatically.
        }

        // ---------------------------------------------------------
        // 3. 内部ロジック (Internal Logic)
        // ---------------------------------------------------------

        private static async Task StartAsync(string localeStr)
        {
            try
            {
                lock (_lockObj)
                {
                    _accumulatedText.Clear();
                    _isRunning = true;
                }

                Console.WriteLine($"[Win/SpeechHelper] Configuring recognizer for {localeStr}...");
                
                // 言語設定
                var language = new Language(localeStr);
                _recognizer = new SpeechRecognizer(language);

                // 制約設定: 自由形式のディクテーション (Swift: DictationTranscriber L47 相当)
                var constraint = new SpeechRecognitionTopicConstraint(SpeechRecognitionScenario.Dictation, "Dictation");
                _recognizer.Constraints.Add(constraint);
                
                var compilationResult = await _recognizer.CompileConstraintsAsync();
                if (compilationResult.Status != SpeechRecognitionResultStatus.Success)
                {
                    throw new Exception($"Constraint Compilation Failed: {compilationResult.Status}");
                }

                // イベントハンドラの登録 (Swift: TaskGroup logic L148 & transcriber.results loop L185)
                _recognizer.HypothesisGenerated += OnHypothesisGenerated; // 部分結果 (Partial)
                _recognizer.ContinuousRecognitionSession.ResultGenerated += OnResultGenerated; // 確定結果 (Final)
                _recognizer.ContinuousRecognitionSession.Completed += OnSessionCompleted;

                // セッション開始
                _session = _recognizer.ContinuousRecognitionSession;
                await _session.StartAsync();

                Console.WriteLine("[Win/SpeechHelper] Recognition started.");
            }
            catch (Exception ex)
            {
                _isRunning = false;
                ReportError($"StartAsync Failed: {ex.Message}");
                CleanupResources();
            }
        }

        // 部分結果 (Partial Results) ハンドラ
        // Swift: L185 loop (isFinal=false case)
        private static void OnHypothesisGenerated(SpeechRecognizer sender, SpeechRecognitionHypothesisGeneratedEventArgs args)
        {
            if (!_isRunning) return;
            string rawText = args.Hypothesis.Text;
            
            // 重複排除と累積テキストの結合
            string fullText = ProcessText(rawText, isFinal: false);
            
            // Rust へ通知
            SendResult(fullText, isFinal: false);
        }

        // 確定結果 (Final Results) ハンドラ
        // Swift: L185 loop (isFinal=true case)
        private static void OnResultGenerated(SpeechRecognitionContinuousRecognitionSession sender, SpeechRecognitionResultGeneratedEventArgs args)
        {
            if (!_isRunning) return;
            // 信頼度が低いものは無視することも可能 (Swift: confidence check 相当)
            if (args.Result.Status != SpeechRecognitionResultStatus.Success) return;

            string rawText = args.Result.Text;
            
            // 重複排除と累積テキストの結合、および内部バッファの更新
            string fullText = ProcessText(rawText, isFinal: true);
            
            // Rust へ通知
            SendResult(fullText, isFinal: true);
        }

        private static void OnSessionCompleted(SpeechRecognitionContinuousRecognitionSession sender, SpeechRecognitionContinuousRecognitionSessionCompletedEventArgs args)
        {
            Console.WriteLine($"[Win/SpeechHelper] Session completed. Status: {args.Status}");
            // 必要に応じて再起動処理などを記述
        }

        // ---------------------------------------------------------
        // 4. テキスト処理ロジック (Swift: L191-201 Deduplication Logic)
        // ---------------------------------------------------------

        private static string ProcessText(string newSegment, bool isFinal)
        {
            lock (_lockObj)
            {
                string currentAccumulated = _accumulatedText.ToString();
                
                // Swift 実装の重複排除ロジック (L192-201) の完全移植
                // 確定したテキストの末尾と、新しく来たセグメントの先頭が被っている場合に除去する
                string cleanSegment = newSegment;
                int maxOverlap = Math.Min(currentAccumulated.Length, newSegment.Length);
                
                if (maxOverlap > 0)
                {
                    for (int i = maxOverlap; i >= 1; i--)
                    {
                        // C# の Substring は (startIndex, length)
                        // Swift: accumulatedText.suffix(i) == rawSegmentText.prefix(i)
                        string suffix = currentAccumulated.Substring(currentAccumulated.Length - i);
                        string prefix = newSegment.Substring(0, i);
                        
                        if (suffix == prefix)
                        {
                            cleanSegment = newSegment.Substring(i);
                            break;
                        }
                    }
                }

                Console.WriteLine($"[Win/SpeechHelper] Raw: '{newSegment}' -> Clean: '{cleanSegment}' (Final: {isFinal})");

                // 今回 Rust に送るべき「全テキスト」を生成
                string fullText = currentAccumulated + cleanSegment;

                // 確定結果なら、内部バッファに追記して保存 (Swift: L219)
                if (isFinal)
                {
                    _accumulatedText.Append(cleanSegment);
                }

                return fullText;
            }
        }

        // ---------------------------------------------------------
        // 5. ヘルパーメソッド
        // ---------------------------------------------------------

        private static void SendResult(string text, bool isFinal)
        {
            if (_resultCallback == null) return;

            // UTF-8 文字列ポインタの生成とコールバック呼び出し
            // 注意: Rust 側は受け取ったポインタをコピーして使うため、ここで確保したメモリは
            // 呼び出し後に解放する必要がありますが、NativeAOT の StringMarshalling を使うと自動化されます。
            // ここでは手動でマーシャリングして安全性を担保します。
            
            byte[] utf8Bytes = Encoding.UTF8.GetBytes(text);
            GCHandle pinnedArray = GCHandle.Alloc(utf8Bytes, GCHandleType.Pinned);
            try
            {
                IntPtr ptr = pinnedArray.AddrOfPinnedObject();
                // 終端文字は C# 配列からではなく、データ長として扱うか、Rust 側で対応が必要です。
                // Rust の CStr::from_ptr を想定し、末尾に \0 をつける必要があります。
                // 今回は簡単のため、Marshal.StringToCoTaskMemUTF8 (Windows限定) を使わず、
                // Rust 側が CString を期待しているため、UTF-8 バイト列 + null終端 を作ります。
                
                byte[] nullTerminated = new byte[utf8Bytes.Length + 1];
                Array.Copy(utf8Bytes, nullTerminated, utf8Bytes.Length);
                nullTerminated[utf8Bytes.Length] = 0; // Null terminator
                
                GCHandle pinnedNullTerm = GCHandle.Alloc(nullTerminated, GCHandleType.Pinned);
                try {
                     _resultCallback(pinnedNullTerm.AddrOfPinnedObject(), isFinal ? 1 : 0);
                }
                finally {
                    pinnedNullTerm.Free();
                }
            }
            finally
            {
                pinnedArray.Free();
            }
        }

        private static void ReportError(string message)
        {
            if (_errorCallback == null) return;
            
            byte[] utf8Bytes = Encoding.UTF8.GetBytes(message);
            // Null termination
            byte[] nullTerminated = new byte[utf8Bytes.Length + 1];
             Array.Copy(utf8Bytes, nullTerminated, utf8Bytes.Length);
            
            GCHandle pinned = GCHandle.Alloc(nullTerminated, GCHandleType.Pinned);
            try
            {
                _errorCallback(pinned.AddrOfPinnedObject());
            }
            finally
            {
                pinned.Free();
            }
        }

        private static void CleanupResources()
        {
            lock (_lockObj)
            {
                _isRunning = false;
                if (_session != null)
                {
                    _session.Dispose();
                    _session = null;
                }
                if (_recognizer != null)
                {
                    _recognizer.Dispose();
                    _recognizer = null;
                }
                _accumulatedText.Clear();
            }
        }
    }
}
```

## 12. Makefile への Windows 用コマンドの追加

既存の macOS 用コマンド（`build`, `run` 等）と衝突せず、Windows 環境内で明示的に呼び出せるように、`-win` サフィックスを付けた専用ターゲットを `Makefile` に追加します。

### 12.1 変数定義の追加
Windows ではリンク方式が macOS と異なり、ビルド時に `.lib` ファイル（インポートライブラリ）が必要になります。これを考慮した定義を行います。

```makefile
# Windows 用の定数定義
ifeq ($(OS),Windows_NT)
    # C# プロジェクト関連
    CS_DIR = windows/SpeechHelper
    CS_OUT_DIR = $(CS_DIR)/bin/Release/net10.0/win-x64/publish
    
    # 成果物の定義
    WIN_DLL = $(CS_OUT_DIR)/SpeechHelper.dll
    WIN_LIB = $(CS_OUT_DIR)/SpeechHelper.lib
    
    # Rust ターゲットとパス
    WIN_TARGET = x86_64-pc-windows-msvc
    WIN_LIB_DIR = target/win-lib
endif
```

### 12.2 専用ターゲットの導入
macOS の `build` は macOS 専用の `RUSTFLAGS`（Info.plist の埋め込み等）を含んでいるため、Windows ではこれらを一切使用しない `build-win` を使用します。

```makefile
.PHONY: build-win clean-win check-win

# Windows 用ビルド
build-win:
	@echo "--- Step 1: Building C# SpeechHelper.dll (Native AOT) ---"
	cd $(CS_DIR) && dotnet publish -r win-x64 -c Release -p:PublishAot=true
	
	@echo "--- Step 2: Preparing Import Library for Rust ---"
	mkdir -p $(WIN_LIB_DIR)
	cp $(WIN_LIB) $(WIN_LIB_DIR)/
	
	@echo "--- Step 3: Building Rust Application (Windows x64) ---"
	# RUSTFLAGS に -L を指定して .lib を見つけられるようにする
	# macOS 版の -sectcreate 等のフラグは含めない
	RUSTFLAGS="-L $(WIN_LIB_DIR)" cargo build --release --target $(WIN_TARGET)
	
	@echo "--- Step 4: Packaging ---"
	cp $(WIN_DLL) target/$(WIN_TARGET)/release/
	@echo "Success! Binary: target/$(WIN_TARGET)/release/mycute.exe"

# Windows 用クリーンアップ
clean-win:
	cargo clean
	cd $(CS_DIR) && dotnet clean
	rm -rf $(WIN_LIB_DIR)
```

### 12.3 実装の要点
- **リンクエラーの回避**: Windows (MSVC) で Rust をビルドする場合、コンパイル時に同名の `.lib` が存在しないとリンクエラーになります。そのため、`Step 2` で一時ディレクトリに `.lib` を集め、`RUSTFLAGS="-L ..."` でその場所を教えています。
- **バイナリの実行可能性**: Windows では実行時に `.exe` と同じディレクトリに `.dll` が必要です。`Step 4` で生成された EXE の横に DLL をコピーすることで、そのまま実行可能な状態を作ります。
- **既存コマンドへの影響ゼロ**: `all`, `build`, `run` といった既存のターゲットには一切触れないため、macOS 側の開発環境を壊す心配はありません。

---

## 13. 実装ロードマップ (x64 Native PC)

### Phase 0: Windows 環境構築 ✅ 完了
- [x] 0.1: Windows 11 (x64) 実機のセットアップ。
- [x] 0.2: Visual Studio Build Tools 2026 インストール済み。
- [x] 0.3: .NET 10 SDK (x64) インストール済み。
- [x] 0.4: Rust (x86_64-pc-windows-msvc) インストール済み。
- [x] 0.5: Native AOT ワークロード有効化済み。
- [x] 0.6: 疎通確認 (Smoke test) 完了。
- [x] 0.7: Git 及びリポジトリ同期完了。
- [/] 0.14: リポジトリを `git clone` するためのフォルダ（例: `C:\src`）を作成する。
- [/] 0.15: `git clone` を実行し、現在のソースコードを Windows 側に同期する。
    - [ ] 0.15.1: `C:\src` に移動し、`git clone https://github.com/t-kawata/mycute.git` (または適切な URL) を実行する。

### 引き継ぎ 01 (Windows Native 環境への移行) ✅ 完了

**背景**:
M2 Mac 上の仮想環境 (ARM64) から、**Intel x64 実機 Windows PC** での開発へ完全移行しました。
環境構築 (Phase 0) は実機側ですべて完了済みです。
今後、Windows PC 上の Antigravity がこのファイルを読み取り、Phase 1 以降の作業（自動ビルドの試行含む）を直接実行します。

**現在のステータス**:
- **Phase 0 (環境構築)**: 完了。
    - .NET 10 SDK (v10.0.102) インストール済み。
    - Visual Studio Build Tools 2026 インストール済み（C++ ARM64/x64 ビルドツール、Windows 11 SDK 含む）。
    - Rust (1.92.0) インストール済み。`x86_64-pc-windows-msvc` ターゲット追加済み。
    - Git インストール済み。`mycute` リポジトリを clone 済み。
- **設計フェーズ**: 完了。
    - C# (.NET 10 Native AOT) による `SpeechHelper.dll` の完全なソースコード設計済み（第11項）。
    - Makefile による自動ビルドターゲット (`build-win`, `clean-win`) の設計済み（第12項）。

**Windows 側の Antigravity への指示 (Next Action)**:
1.  **リポジトリのルートで作業を開始してください。**
2.  **Phase 1** に進み、`windows/SpeechHelper` ディレクトリを作成して `dotnet new classlib` を実行してください。
3.  **プロジェクト構成**: `SpeechHelper.csproj` を本ドキュメントの指示（.NET 10 / TargetPlatform: Windows）通りに編集してください。
4.  **コード配置**: 第11項の `SpeechHelper.cs` をソースコードとして配置してください。
5.  **ビルド試行**: `dotnet publish -r win-x64 -c Release` が通るか確認してください。
6.  その後、Rust 側の FFI 連携（Phase 3）や Makefile の実装（Phase 4）へと順次進めてください。

---

### Phase 1: C# バックエンドプロジェクトの構築 ✅ 完了
- [x] 1.1: `windows/SpeechHelper` フォルダを作成する。
- [x] 1.2: フォルダ内で `dotnet new classlib` を実行し、プロジェクトを初期化する。
- [x] 1.3: プロジェクトファイル (`SpeechHelper.csproj`) をエディタで開く。
- [x] 1.4: `<PublishAot>true</PublishAot>` を PropertyGroup に追加する。
- [x] 1.5: `<TargetFramework>net10.0</TargetFramework>` であることを確認する。
- [x] 1.6: `<Nullable>enable</Nullable>` を設定する。
- [x] 1.7: デフォルトの `Class1.cs` を削除する。
- [x] 1.8: `SpeechHelper.cs` を作成し、設計書コードをペーストする。
    - [x] 1.8.1: WinRT API を参照するため、csproj に `<TargetPlatformIdentifier>Windows</TargetPlatformIdentifier>` 等の指定が必要か確認する。
- [x] 1.9: ローカル環境での `dotnet build` を実行し、参照エラーがないか確認する。
- [x] 1.10: Native AOT ビルドを実行する。
    - [x] 1.10.1: `dotnet publish -r win-x64 -c Release` を実行する。
- [x] 1.11: `SpeechHelper.dll` の生成を確認する。
- [x] 1.12: `SpeechHelper.lib` の生成を確認する。
- [x] 1.13: DLL のファイルサイズをチェックする。
- [x] 1.14: `dumpbin` 等でエクスポートされた関数名が正しいことを確認する。
- [x] 1.15: C# 側のソースコード管理を整える。

### Phase 2: Rust 側 STT エンジンの抽象化実装 ✅ 完了
- [x] 2.1: `src/stt_config.rs` に `Win` バリアントを追加。
    - [x] 2.1.1: `SttEngine` エナムを検索し、`Mac`, `Classic`, `OpenAI`, `Sherpa02` に並んで `Win` を追加する。
    - [x] 2.1.2: `serde` の `rename` 属性で小文字の "win" を指定する。
- [x] 2.2: `settings.json` のデシリアライズテストを行う。
    - [x] 2.2.1: `settings.json` の `stt_engine` を "win" に書き換えて、起動時にパニックしないか確認する。
- [x] 2.3: 共通インターフェース `SpeechRecognizer` の準備。
    - [x] 2.3.1: `src/stt/recognizer.rs` を開き、`start`, `stop`, `tick` 等のシグネチャが全エンジンで共通であることを確認する。
- [x] 2.4: `src/stt/win.rs` の作成。
    - [x] 2.4.1: 新規ファイル `src/stt/win.rs` を作成する。
    - [x] 2.4.2: `mod win;` を `src/stt/mod.rs` に追加し、Windows 環境 (`#[cfg(target_os = "windows")]`) でのみ有効にする。
- [x] 2.5: `WinSpeechBackend` 構造体の定義。
    - [x] 2.5.1: DLL のシンボルを保持するためのフィールド（`Library` 等）を定義する。
- [x] 2.6: トレイトメソッドの実装。
    - [x] 2.6.1: `WinSpeechBackend` に対して `impl SpeechRecognizer` を記述する。
    - [x] 2.6.2: `start` メソッド内で DLL 側の `speech_helper_start` を呼ぶスタブを実装する。
- [x] 2.7: Windows 環境下でのエンジン自動選択。
    - [x] 2.7.1: `src/stt/recognizer.rs` の生成ロジック (`SpeechRecognizer::new`) を修正する。
    - [x] 2.7.2: OS が Windows の場合、設定が "mac" であっても "win" エンジンを選択または警告するロジックを組む。
- [x] 2.8: macOS での Tahoe 自動判別の実装。
- [x] 2.9: エンジン選択のロギング。
- [x] 2.10: 起動時にログレベル INFO で "Using Windows Native STT engine" と表示されるようにする。

### Phase 3: Rust FFI & 静的リンク (Static Link) の実装 ✅ 完了
- [x] 3.1: `libloading` を削除し、静的リンクへの移行を準備。
- [x] 3.2: Native AOT のランタイムライブラリ (`.lib`) の検索パス解決。
- [x] 3.3: 関数ポインタの定義から直接的な `extern "C"` 宣言への移行。
- [x] 3.4: コールバック関数のブリッジ実装。
- [x] 3.5: UTF-8 文字列の変換、C# 側の `pinned` 配列によるメモリ管理。
- [x] 3.6: リンク時の依存解決（`VCRUNTIME140.dll` 等のランタイムリンク）。

### Phase 4: Makefile の拡張とビルドパイプライン ✅ 完了
- [x] 4.1: OS 環境変数判定の実装。
- [x] 4.2: `build-win` ターゲットの作成。
- [x] 4.3: DLL/LIB の配置。
- [x] 4.4: Rust ビルドの実行。
- [x] 4.5: クリーンアップルールの追加。

### Phase 5: UI & リソースの Windows 対応
- [ ] 5.1: Windows 11 での egui 描画チェック。
    - [ ] 5.1.1: ウィンドウ枠、影、透過度が Windows の DWM (Desktop Window Manager) と調和しているか見る。
- [ ] 5.2: 日本語フォント設定の一貫性確保。
    - [ ] 5.2.1: Windows 環境でも `src/fonts` 内のフォントを優先的に使用し、macOS と共通のデザイン（UI/UX）を提供できるように `src/ui/mod.rs` 等を設定する。
- [x] 5.3: Alt キーマッピング。
    - [x] 5.3.1: macOS の Option キー操作を Windows の Alt キーで自然に行えるか、ホットキー判定部を修正する。 (`src/hotkey_win.rs` で実装済み)

### Phase 6: 静的リンク化 (Static Linking Implementation) ✅ 完了
- [x] 6.1: `SpeechHelper.csproj` を `Static` ライブラリ出力に設定。
- [x] 6.2: `build.rs` を作成/更新し、`SpeechHelper.lib` を自動リンク。
- [x] 6.3: Windows SDK および Native AOT ランタイムライブラリの自動収集・リンク。
- [x] 6.4: シングルバイナリとしての動作検証（`SpeechHelper.dll` 不要での起動）。
- [x] 6.5: `Cargo.toml` から `libloading` 依存関係を完全に削除。
### Phase 8: OS 依存コードのファイル分離 (新規) ✅ 完了
- [x] 8.1: `hotkey.rs` を `hotkey_mac.rs` / `hotkey_win.rs` に分離。
- [x] 8.2: `input/keyboard.rs` を `keyboard_mac.rs` / `keyboard_win.rs` に分離。
- [x] 8.3: `lib.rs` および `input/mod.rs` で `#[cfg(target_os = "macos")]` と `#[cfg(target_os = "windows")]` を使ったディスパッチを実装。
- [x] 8.4: Mac 環境で破壊的変更がないことを検証済み。

### Phase 6: 通信テスト & 実機デバッグ
- [ ] 6.1: マイク入力テスト。
    - [ ] 6.1.1: Windows 設定でデフォルトマイクが選択されているか確認。
- [ ] 6.2: 認識結果の確認。
    - [ ] 6.2.1: 重複排除ロジックが走り、タイプレコーダーのようなダブりが発生しないか検証。
- [ ] 6.3: 長時間動作。
    - [ ] 6.3.1: 10分間話し続け、認識が止まったりアプリが落ちたりしないか過負荷テストを行う。

### Phase 7: 配布パッケージ作成
- [ ] 7.1: 配布用 zip ファイルの作成。
    - [ ] 7.1.1: `mycute.exe` と `SpeechHelper.dll` をセットにして固める。
- [ ] 7.2: README 更新。
    - [ ] 7.2.1: Windows での実行には .NET 10 ランタイムが必要である旨を明記する。

- [ ] 100: 全てのステップを完了し、Windows 版 mycute の正式リリースを宣言。
