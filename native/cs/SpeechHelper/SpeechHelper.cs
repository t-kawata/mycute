using System;
using System.Text;
using System.Linq;
using System.Threading.Tasks;
using System.Runtime.InteropServices;
using Windows.Media.SpeechRecognition;
using Windows.Globalization;
using Windows.Media.Audio;
using Windows.Media.Render;
using Windows.Media.MediaProperties;
using Windows.Media;
using Windows.Media.Capture;
using Windows.Foundation;
using System.Runtime.InteropServices.WindowsRuntime;
using WinRT;

// 名前空間は Rust からは隠蔽されますが、C# 内部での管理用です。
namespace Mycute.WindowsBackend
{
    // We don't use [ComImport] interface directly to avoid ComWrapper issues in NativeAOT.
    // Instead we interact via manual IUnknown QueryInterface/GetDelegateForFunctionPointer if needed,
    // or just use manual VTable access.
    // GUID: 5B0D3235-4DBA-4D44-865E-8F1D0E4FD04D

    /// <summary>
    /// macOS 版 SpeechHelper.swift と同等の機能を提供する Windows 用ネイティブヘルパー。
    /// Native AOT により、Rust からは C の動的ライブラリとして見えます。
    /// </summary>
    public static partial class SpeechHelper
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

        // typedef void (*AudioDataCallback)(const float *samples, uint32_t count, uint32_t sample_rate);
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void AudioDataCallback(IntPtr samples, uint count, uint sampleRate);

        // typedef void (*ReadyCallback)();
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void ReadyCallback();

        // GC によって回収されないように静的フィールドで保持
        private static SpeechResultCallback? _resultCallback;
        private static SpeechErrorCallback? _errorCallback;
        private static AudioDataCallback? _audioDataCallback;
        private static ReadyCallback? _readyCallback;
        private static bool _hasNotifiedReady = false;
        private static double _speechTimeoutSec = 30.0; // Default fallback

        // 音声認識のセッション状態
        private static SpeechRecognizer? _recognizer;
        private static SpeechContinuousRecognitionSession? _session;
        private static bool _isRunning = false;

        // AudioGraph 状態
        private static AudioGraph? _audioGraph;
        private static AudioDeviceInputNode? _deviceInputNode;
        private static AudioFrameOutputNode? _frameOutputNode;
        private static bool _isCapturing = false;
        private static object _audioLock = new object();

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
        public static int Init(double speechTimeoutSec)
        {
            // Initialize COM wrappers for Native AOT WinRT support
            WinRT.ComWrappersSupport.InitializeComWrappers();

            _speechTimeoutSec = speechTimeoutSec;
            Console.WriteLine($"[Win/SpeechHelper] Initialized (Timeout: {_speechTimeoutSec}s).");
            return 0; // Success
        }

        /// <summary>
        /// Windows 音声入力の設定状態をチェックし、不足をビットマスクで返す。
        /// 戻り値: 0 = 正常, bit0 = 音声認識モデル未インストール, bit1 = 音声認識プライバシーOFF, bit2 = マイク権限なし
        /// </summary>
        [UnmanagedCallersOnly(EntryPoint = "speech_helper_check_health")]
        public static int CheckHealth()
        {
            return Task.Run(async () =>
            {
                int result = 0;

                // 1. 音声認識モデルがインストールされているか
                try
                {
                    var supported = SpeechRecognizer.SupportedTopicLanguages;
                    bool hasJapanese = supported.Any(l =>
                        l.LanguageTag.StartsWith("ja-", StringComparison.OrdinalIgnoreCase) ||
                        l.LanguageTag.Equals("ja", StringComparison.OrdinalIgnoreCase));
                    bool hasEnglish = supported.Any(l =>
                        l.LanguageTag.StartsWith("en-", StringComparison.OrdinalIgnoreCase) ||
                        l.LanguageTag.Equals("en", StringComparison.OrdinalIgnoreCase));
                    if (!hasJapanese && !hasEnglish)
                    {
                        result |= 1; // モデル未インストール
                    }
                }
                catch (Exception ex)
                {
                    Console.WriteLine($"[Win/SpeechHelper] HealthCheck: model detection failed: {ex.Message}");
                    result |= 1; // 検出失敗 = 利用不可とみなす
                }

                // 2. 音声認識プライバシートグルが ON か
                // CompileConstraintsAsync が失敗した場合、プライバシートグルが OFF と判断する
                try
                {
                    var recognizer = new SpeechRecognizer(new Language("ja-JP"));
                    var compilationResult = await recognizer.CompileConstraintsAsync();
                    if (compilationResult.Status != SpeechRecognitionResultStatus.Success)
                    {
                        result |= 2; // プライバシートグル OFF
                    }
                    recognizer.Dispose();
                }
                catch (Exception ex)
                {
                    Console.WriteLine($"[Win/SpeechHelper] HealthCheck: privacy check failed: {ex.Message}");
                    result |= 2; // 検出失敗 = 利用不可とみなす
                }

                // 3. マイク権限が付与されているか
                try
                {
                    var settings = new AudioGraphSettings(AudioRenderCategory.Speech);
                    var graphResult = await AudioGraph.CreateAsync(settings);
                    if (graphResult.Status == AudioGraphCreationStatus.Success)
                    {
                        var inputResult = await graphResult.Graph.CreateDeviceInputNodeAsync(MediaCategory.Speech);
                        if (inputResult.Status == AudioDeviceNodeCreationStatus.AccessDenied)
                        {
                            result |= 4; // マイク権限なし
                        }
                        graphResult.Graph.Dispose();
                    }
                    else
                    {
                        result |= 4; // AudioGraph 作成失敗 = マイク利用不可
                    }
                }
                catch (Exception ex)
                {
                    Console.WriteLine($"[Win/SpeechHelper] HealthCheck: mic permission check failed: {ex.Message}");
                    result |= 4;
                }

                return result;
            }).GetAwaiter().GetResult();
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
        /// 準備完了通知用コールバックの登録
        /// </summary>
        [UnmanagedCallersOnly(EntryPoint = "speech_helper_set_ready_callback")]
        public static void SetReadyCallback(IntPtr callbackPtr)
        {
            if (callbackPtr != IntPtr.Zero)
            {
                _readyCallback = Marshal.GetDelegateForFunctionPointer<ReadyCallback>(callbackPtr);
            }
            else
            {
                _readyCallback = null;
            }
        }

        // ---------------------------------------------------------
        // IME Control API
        // ---------------------------------------------------------

        [LibraryImport("user32.dll")]
        private static partial IntPtr GetForegroundWindow();

        [LibraryImport("user32.dll", EntryPoint = "SendMessageW")]
        private static partial IntPtr SendMessage(IntPtr hWnd, uint Msg, IntPtr wParam, IntPtr lParam);

        [LibraryImport("imm32.dll")]
        private static partial IntPtr ImmGetDefaultIMEWnd(IntPtr hWnd);

        // WM_IME_CONTROL 定数およびサブコマンド
        private const uint WM_IME_CONTROL              = 0x0283;
        private static readonly IntPtr IMC_GETOPENSTATUS = new IntPtr(0x0005);
        private static readonly IntPtr IMC_SETOPENSTATUS = new IntPtr(0x0006);
        private static readonly IntPtr IME_OFF           = new IntPtr(0);
        private static readonly IntPtr IME_ON            = new IntPtr(1);

        private static bool _previousImeState = false;

        [UnmanagedCallersOnly(EntryPoint = "speech_helper_disable_ime")]
        public static void DisableIme()
        {
            try
            {
                IntPtr hWnd = GetForegroundWindow();
                if (hWnd == IntPtr.Zero)
                {
                    Console.WriteLine("[Win/SpeechHelper] DisableIme: GetForegroundWindow returned Zero. No active window found.");
                    return;
                }

                IntPtr hImeWnd = ImmGetDefaultIMEWnd(hWnd);
                if (hImeWnd == IntPtr.Zero)
                {
                    Console.WriteLine($"[Win/SpeechHelper] DisableIme: ImmGetDefaultIMEWnd returned Zero for hWnd=0x{hWnd:X}. Target app may not have an IME window (e.g., is IME-unaware).");
                    _previousImeState = false;
                    return;
                }

                // 現在のIME状態を取得して保存
                IntPtr currentStatus = SendMessage(hImeWnd, WM_IME_CONTROL, IMC_GETOPENSTATUS, IntPtr.Zero);
                _previousImeState = (currentStatus != IntPtr.Zero);
                Console.WriteLine($"[Win/SpeechHelper] DisableIme: hWnd=0x{hWnd:X}, hImeWnd=0x{hImeWnd:X}, currentImeOpen={_previousImeState}");

                if (_previousImeState)
                {
                    // IMEを無効化 (0 = OFF)
                    SendMessage(hImeWnd, WM_IME_CONTROL, IMC_SETOPENSTATUS, IME_OFF);
                    Console.WriteLine("[Win/SpeechHelper] IME disabled via WM_IME_CONTROL. (Previous state was ON)");
                }
                else
                {
                    Console.WriteLine("[Win/SpeechHelper] IME was already OFF. No change needed.");
                }
            }
            catch (Exception ex)
            {
                Console.WriteLine($"[Win/SpeechHelper] DisableIme error: {ex.Message}");
            }
        }

        [UnmanagedCallersOnly(EntryPoint = "speech_helper_restore_ime")]
        public static void RestoreIme()
        {
            try
            {
                IntPtr hWnd = GetForegroundWindow();
                if (hWnd == IntPtr.Zero)
                {
                    Console.WriteLine("[Win/SpeechHelper] RestoreIme: GetForegroundWindow returned Zero.");
                    return;
                }

                IntPtr hImeWnd = ImmGetDefaultIMEWnd(hWnd);
                if (hImeWnd == IntPtr.Zero)
                {
                    Console.WriteLine($"[Win/SpeechHelper] RestoreIme: ImmGetDefaultIMEWnd returned Zero for hWnd=0x{hWnd:X}.");
                    return;
                }

                Console.WriteLine($"[Win/SpeechHelper] RestoreIme: hWnd=0x{hWnd:X}, hImeWnd=0x{hImeWnd:X}, restoring to previousImeOpen={_previousImeState}");

                if (_previousImeState)
                {
                    // IMEを元のON状態に戻す (1 = ON)
                    SendMessage(hImeWnd, WM_IME_CONTROL, IMC_SETOPENSTATUS, IME_ON);
                    Console.WriteLine("[Win/SpeechHelper] IME restored to ON via WM_IME_CONTROL.");
                }
                else
                {
                    Console.WriteLine("[Win/SpeechHelper] IME was originally OFF. No restore needed.");
                }
            }
            catch (Exception ex)
            {
                Console.WriteLine($"[Win/SpeechHelper] RestoreIme error: {ex.Message}");
            }
        }

        [UnmanagedCallersOnly(EntryPoint = "speech_helper_start_capture")]
        public static int StartCapture()
        {
            if (_isCapturing) return 0;
            _hasNotifiedReady = false;
            Task.Run(async () => await StartCaptureAsync());
            return 0;
        }

        [UnmanagedCallersOnly(EntryPoint = "speech_helper_stop_capture")]
        public static void StopCapture()
        {
            StopCaptureInternal();
        }

        /// <summary>
        /// 音声データコールバックの登録
        /// </summary>
        [UnmanagedCallersOnly(EntryPoint = "speech_helper_set_audio_data_callback")]
        public static void SetAudioDataCallback(IntPtr callbackPtr)
        {
            if (callbackPtr != IntPtr.Zero)
            {
                Console.WriteLine($"[Win/SpeechHelper] SetAudioDataCallback: Registering callback at 0x{callbackPtr.ToString("X")}");
                _audioDataCallback = Marshal.GetDelegateForFunctionPointer<AudioDataCallback>(callbackPtr);
                Console.WriteLine("[Win/SpeechHelper] AudioData callback registered successfully.");
            }
            else
            {
                Console.WriteLine("[Win/SpeechHelper] SetAudioDataCallback: Received NULL pointer.");
                _audioDataCallback = null;
            }
        }



        /// <summary>
        /// 音声認識の開始 (Swift: speech_helper_start L318, TahoeSession.start L29)
        /// </summary>
        [UnmanagedCallersOnly(EntryPoint = "speech_helper_start")]
        public static int Start(IntPtr localePtr)
        {
            if (_isRunning) return 0;
            _hasNotifiedReady = false;

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
            StopInternal();
        }

        private static void StopInternal()
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
            StopInternal();
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

        private static async Task StartCaptureAsync()
        {
            try
            {
                lock (_audioLock)
                {
                    if (_isCapturing) return;
                    _isCapturing = true;
                }

                _debugFrameCounter = 0; // セッション開始時にリセットし、初回エラーを確実にログ出力する
                Console.WriteLine("[Win/SpeechHelper] Starting audio capture...");

                // 1. AudioGraph の作成
                var settings = new AudioGraphSettings(AudioRenderCategory.Speech);
                var result = await AudioGraph.CreateAsync(settings);
                if (result.Status != AudioGraphCreationStatus.Success)
                {
                    ReportError($"AudioGraph creation failed: {result.Status}");
                    lock (_audioLock) _isCapturing = false;
                    return;
                }
                _audioGraph = result.Graph;

                // 2. デバイス入力ノードの作成
                var inputResult = await _audioGraph.CreateDeviceInputNodeAsync(MediaCategory.Speech);
                if (inputResult.Status != AudioDeviceNodeCreationStatus.Success)
                {
                    ReportError($"DeviceInputNode creation failed: {inputResult.Status}");
                    lock (_audioLock) _isCapturing = false;
                    return;
                }
                _deviceInputNode = inputResult.DeviceInputNode;

                // 3. フレーム出力ノードの作成 (16kHz Mono Float)
                var outProps = AudioEncodingProperties.CreatePcm(16000, 1, 32);
                outProps.Subtype = MediaEncodingSubtypes.Float;
                _frameOutputNode = _audioGraph.CreateFrameOutputNode(outProps);

                // 4. 結線
                _deviceInputNode.AddOutgoingConnection(_frameOutputNode);

                // 5. イベント購読
                _audioGraph.QuantumStarted += OnAudioQuantumStarted;

                // 6. 開始
                _audioGraph.Start();
                Console.WriteLine("[Win/SpeechHelper] Audio capture started (16kHz Mono Float).");
            }
            catch (Exception ex)
            {
                ReportError($"StartCaptureAsync Failed: {ex.Message}");
                // エラー時はクリーンアップ
                StopCaptureInternal();
            }
        }

        private static void StopCaptureInternal()
        {
            lock (_audioLock)
            {
                if (!_isCapturing) return;
                _isCapturing = false;

                try
                {
                    // Clean up nodes first
                    if (_deviceInputNode != null)
                    {
                        try { _deviceInputNode.Dispose(); } catch { }
                        _deviceInputNode = null;
                    }

                    if (_frameOutputNode != null)
                    {
                        try { _frameOutputNode.Dispose(); } catch { }
                        _frameOutputNode = null;
                    }

                    // Clean up graph last
                    if (_audioGraph != null)
                    {
                        try
                        {
                            _audioGraph.Stop();
                            _audioGraph.QuantumStarted -= OnAudioQuantumStarted;
                            _audioGraph.Dispose();
                        }
                        catch (Exception ex)
                        {
                            Console.WriteLine($"[Win/SpeechHelper] Graph dispose error: {ex.Message}");
                        }
                        _audioGraph = null;
                    }
                }
                catch (Exception ex)
                {
                    Console.WriteLine($"[Win/SpeechHelper] StopCaptureInternal generic error: {ex.Message}");
                }

                Console.WriteLine("[Win/SpeechHelper] Audio capture stopped.");
            }
        }

        // デバッグ用カウンタ
        private static int _debugFrameCounter = 0;

        private static unsafe void OnAudioQuantumStarted(AudioGraph sender, object args)
        {
            try
            {
                if (!_isCapturing) return;

                if (_audioDataCallback == null)
                {
                    if (_debugFrameCounter % 100 == 0) Console.WriteLine("[Win/SpeechHelper] Callback is NULL, skipping data.");
                    _debugFrameCounter++;
                    return;
                }

                var frame = _frameOutputNode?.GetFrame();
                if (frame == null) return;

                using (AudioBuffer buffer = frame.LockBuffer(AudioBufferAccessMode.Read))
                using (IMemoryBufferReference reference = buffer.CreateReference())
                {
                    // Native AOT workaround for IMemoryBufferByteAccess:
                    // Avoid using implicit RCW or WinRT.CastExtensions.As<T>() for non-WinRT COM interfaces.
                    // Instead, we query interface manually from the underlying IUnknown pointer.

                    // 1. Get IUnknown pointer from the WinRT object wrapper
                    var objRef = ((IWinRTObject)reference).NativeObject;
                    IntPtr thisPtr = objRef.ThisPtr; // This is the IMemoryBufferReference* (IInspectable*)

                    // 2. Define IMemoryBufferByteAccess IID
                    // 5B0D3235-4DBA-4D44-865E-8F1D0E4FD04D
                    Guid iid = new Guid("5B0D3235-4DBA-4D44-865E-8F1D0E4FD04D");

                    // 3. QueryInterface (IUnknown method 0) - handled by Marshal
                    int hr = Marshal.QueryInterface(thisPtr, in iid, out nint bufferByteAccessPtr);
                    if (hr != 0 || bufferByteAccessPtr == IntPtr.Zero)
                    {
                        if (_debugFrameCounter % 100 == 0) Console.WriteLine($"[Win/SpeechHelper] QueryInterface failed. HR: {hr:X}");
                        _debugFrameCounter++;
                        return;
                    }

                    try
                    {
                        // 4. Call GetBuffer manually via VTable
                        // IUnknown layout: [0]QueryInterface, [1]AddRef, [2]Release
                        // IMemoryBufferByteAccess layout: + [3]GetBuffer

                        // void GetBuffer(out byte* buffer, out uint capacity);

                        void** vtable = *(void***)bufferByteAccessPtr;
                        var getBufferFunc = (delegate* unmanaged[Stdcall]<IntPtr, byte**, uint*, int>)(vtable[3]);

                        byte* dataInBytes = null;
                        uint capacityInBytes = 0;

                        int result = getBufferFunc(bufferByteAccessPtr, &dataInBytes, &capacityInBytes);

                        if (result != 0) // S_OK
                        {
                            if (_debugFrameCounter % 100 == 0) Console.WriteLine($"[Win/SpeechHelper] GetBuffer failed. HR: {result:X}");
                            _debugFrameCounter++;
                            return;
                        }

                        if (dataInBytes != null)
                        {
                            // Use the actual VALID data length from the buffer property, not the raw capacity.
                            // Capacity might be larger (e.g. system allocation alignment) and contain garbage/zeros at the end.
                            uint validLength = buffer.Length;
                            uint sampleCount = validLength / 4; // Float is 4 bytes

                            // Safety check: ensure we don't read past capacity
                            if (validLength > capacityInBytes)
                            {
                                validLength = capacityInBytes;
                                sampleCount = validLength / 4;
                            }

                            if (sampleCount > 0 && _audioDataCallback != null)
                            {
                                // Call the delegate.
                                // Argument 1: IntPtr samples (ptr to float array)
                                // Argument 2: uint count
                                // Argument 3: uint sampleRate (16000)
                                _audioDataCallback((IntPtr)dataInBytes, (uint)sampleCount, 16000);
                                if (!_hasNotifiedReady && _readyCallback != null)
                                {
                                    _hasNotifiedReady = true;
                                    _readyCallback();
                                }
                            }
                        }
                    }
                    catch (Exception ex)
                    {
                        if (_debugFrameCounter % 100 == 0) Console.WriteLine($"[Win/SpeechHelper] COM Access error: {ex.Message}");
                    }
                    finally
                    {
                        // No explicit Release needed for IntPtr obtained via simple pointer arithmetic or QueryInterface if managed by Marshal?
                        // Actually Marshal.QueryInterface increments ref count, so we MUST Release.
                        // But for 'thisPtr' from (IWinRTObject)reference it is managed by RCW.
                        // The 'bufferByteAccessPtr' was from QueryInterface, so Release it.
                        if (bufferByteAccessPtr != IntPtr.Zero)
                        {
                            Marshal.Release(bufferByteAccessPtr);
                        }
                    }
                }
            }
            catch (Exception ex)
            {
                // Swallow exceptions during shutdown/race conditions
                // Expected: ObjectDisposedException, COMException (0x8000000B), etc.
                if (_debugFrameCounter % 100 == 0) Console.WriteLine($"[Win/SpeechHelper] Quantum error (ignored): {ex.Message}");
            }
        }




        private static async Task StartAsync(string localeStr)
        {
            try
            {
                lock (_lockObj)
                {
                    _accumulatedText.Clear();
                    _isRunning = true;
                }

                Console.WriteLine($"[Win/SpeechHelper] Configuring recognizer for request: '{localeStr}'...");

                // システムで利用可能な言語を確認して最適なものを選択
                // Swift: N/A (macOS は Locale(identifier: "ja") で通るが WinRT は厳密な場合がある)
                var supportedLanguages = SpeechRecognizer.SupportedTopicLanguages;
                Language? selectedLanguage = null;

                Console.WriteLine($"[Win/SpeechHelper] Supported Languages: {string.Join(", ", supportedLanguages.Select(l => l.LanguageTag))}");

                // 1. 完全一致チェック
                selectedLanguage = supportedLanguages.FirstOrDefault(l => l.LanguageTag.Equals(localeStr, StringComparison.OrdinalIgnoreCase));

                // 2. 接頭辞チェック (例: "ja" -> "ja-JP")
                if (selectedLanguage == null)
                {
                    selectedLanguage = supportedLanguages.FirstOrDefault(l => l.LanguageTag.StartsWith(localeStr + "-", StringComparison.OrdinalIgnoreCase));
                }

                // 3. 逆方向チェック (例: "ja-JP" リクエストに対して "ja" がある場合 - 稀)
                if (selectedLanguage == null && localeStr.Contains("-"))
                {
                    var baseCode = localeStr.Split('-')[0];
                    selectedLanguage = supportedLanguages.FirstOrDefault(l => l.LanguageTag.Equals(baseCode, StringComparison.OrdinalIgnoreCase));
                }

                // 見つからなかった場合
                if (selectedLanguage == null)
                {
                    Console.WriteLine($"[Win/SpeechHelper] Warning: Requested locale '{localeStr}' not found in supported languages.");
                    // フォールバックせずにエラーにするか、デフォルトを使うか。
                    // ここではエラーにせず、new Language(localeStr) を試させる（WinRTが内部で解決する可能性に期待）
                    // しかし、既存コードで失敗しているので、リストの先頭（英語など）を使うよりは、
                    // 明示的にエラーをスローしてユーザーに環境不備を伝えるべきですが、
                    // 今回は「ja-JPがあるはずなのにjaで失敗した」ケースなので、上記のマッチングで解決するはず。

                    selectedLanguage = new Language(localeStr);
                }
                else
                {
                    Console.WriteLine($"[Win/SpeechHelper] Resolved locale '{localeStr}' -> '{selectedLanguage.LanguageTag}'");
                }

                _recognizer = new SpeechRecognizer(selectedLanguage);

                // 制約設定: 自由形式のディクテーション (Swift: DictationTranscriber L47 相当)
                var constraint = new SpeechRecognitionTopicConstraint(SpeechRecognitionScenario.Dictation, "Dictation");
                _recognizer.Constraints.Add(constraint);

                // --- Timeout Symmetry Configuration ---
                // Mac (Tahoe) defaults to ~30s. Mac (Classic) will be manually limited to 30s.
                // To ensure cross-platform experience symmetry, we explicitly set Windows timeouts to 30s.
                // Default Windows InitialSilenceTimeout is 5s, which is too short.
                // (Value is injected from Rust via Initialize)
                _recognizer.Timeouts.InitialSilenceTimeout = TimeSpan.FromSeconds(_speechTimeoutSec);
                _recognizer.Timeouts.EndSilenceTimeout = TimeSpan.FromSeconds(0.15); // Default is fine, keep it snappy
                _recognizer.Timeouts.BabbleTimeout = TimeSpan.FromSeconds(0); // Disable babble timeout logic if possible

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

                // Notify ready immediately as the session is started
                if (!_hasNotifiedReady && _readyCallback != null)
                {
                    _hasNotifiedReady = true;
                    _readyCallback();
                }
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
        private static void OnResultGenerated(SpeechContinuousRecognitionSession sender, SpeechContinuousRecognitionResultGeneratedEventArgs args)
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

        private static void OnSessionCompleted(SpeechContinuousRecognitionSession sender, SpeechContinuousRecognitionCompletedEventArgs args)
        {
            Console.WriteLine($"[Win/SpeechHelper] Session completed. Status: {args.Status}");

            // Notify Rust about completion so it can trigger auto-commit if needed.
            // This ensures symmetry with Mac's loop-exit behavior.
            // We use a special error message prefix "COMPLETED:" to distinguish from errors.
            ReportError($"COMPLETED:{args.Status}");
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
                try
                {
                    _resultCallback(pinnedNullTerm.AddrOfPinnedObject(), isFinal ? 1 : 0);
                }
                finally
                {
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
            StopCaptureInternal();
            lock (_lockObj)
            {
                _isRunning = false;
                if (_session != null)
                {
                    // WinRT types do not implement IDisposable; just release the reference.
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
