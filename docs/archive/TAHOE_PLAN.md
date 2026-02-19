# Tahoe Speech API Migration Plan

## 1. 目的
macOS 26.2 (Tahoe) で導入された最新の音声認識フレームワーク（`SpeechAnalyzer` / `SpeechTranscriber`）を導入し、オンデバイスでの高速かつ高精度な文字起こしを実現します。同時に、既存の安定した `SFSpeechRecognizer` 実装を「Classic」モードとして完全に維持し、設定で切り替え可能にします。

> **注意**: 実装した通り、`SpeechTranscriber` API は macOS 26.2 以降に最適化されています。

## 2. 実装の基本方針（厳格な互換性維持）
本改修の最優先事項は **「既存の安定動作を一切破壊しない」** ことです。
- `settings.json` の設定が `classic` の場合、現在と全く同じコードパス（Swiftの関数、Rustの処理ロジック）を通過するように実装します。
- 既存の Swift 関数（`speechHelperStart` 等）のロジックは変更せず、必要に応じて Tahoe 用の新しい関数（`tahoeHelperStart` 等）を別途定義します。

## 3. 具体的なソースコード改修計画

### A. 設定ファイルの拡張 (`src/config.rs`)
`Settings` 構造体に `stt_engine` フィールドを追加します。

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum SttEngine {
    Classic,
    Tahoe,
}


// [NEW] ロケールの定義
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LocaleCode {
    En,
    Ja,
}

impl LocaleCode {
    pub fn as_str(&self) -> &str {
        match self {
            Self::En => "en-US",
            Self::Ja => "ja-JP",
        }
    }
}

// Settings構造体への追加
pub struct Settings {
    pub hotkeys: HotkeyConfig,
    // ...
    #[serde(default = "default_stt_engine")]
    pub stt_engine: SttEngine,
    #[serde(default = "default_locale")]
    pub locale: LocaleCode, // 初期言語設定
}

fn default_stt_engine() -> SttEngine { SttEngine::Classic }
fn default_locale() -> LocaleCode { LocaleCode::Ja }

pub struct HotkeyConfig {
    // ...
    pub toggle_locale: Vec<String>, // ["Option", "KeyL"]
}
```

### A.1 FFI Return Codes (標準化)
RustとSwift間でやり取りするエラーコードを以下のように定義します。

| コード | 定数名 (Swift想定) | 意味 |
| :--- | :--- | :--- |
| `0` | `success` | 正常終了 |
| `-10` | `errOsVersion` | macOS 15 未満 |
| `-11` | `errModelNotReady` | 言語モデル未インストール / ダウンロード中 |
| `-12` | `errHardwareSupport`| ハードウェア(Neural Engine等)非対応 |
| `-13` | `errMicPermission` | マイク権限なし |
| `-14` | `errRecognitionFailed`| 認識プロセスの起動失敗 |

### B. Rust STT レイヤーのディスパッチロジック (`src/stt/recognizer.rs`)
`SpeechRecognizer` が設定値を見て、呼び出す FFI 関数を切り替えるようにします。
- **変更内容**:
    - FFI宣言ブロックに Tahoe 用の関数（`tahoe_helper_init`, `tahoe_helper_start`, `tahoe_helper_stop`）を追加。
    - `start()` や `stop()` メソッド内で、`if self.engine == SttEngine::Tahoe { ... } else { ... }` のように分岐させます。
- **理由**: Rust 側で明確にエンジンを分離することで、Classic モード時の副作用をゼロにするため。

```rust
#[link(name = "SpeechHelper")]
extern "C" {
    // 引数に locale を追加
    fn speech_helper_start(locale: *const c_char) -> i32;
    fn tahoe_helper_start(locale: *const c_char) -> i32;
}

impl SpeechRecognizer {
    pub fn start(&mut self, engine: SttEngine, locale: &str) {
        let c_locale = CString::new(locale).unwrap();
        unsafe {
            let result = if engine == SttEngine::Tahoe {
                tahoe_helper_start(c_locale.as_ptr())
            } else {
                speech_helper_start(c_locale.as_ptr())
            };

            if result != 0 {
                self.handle_error(result);
            }
        }
    }

    fn handle_error(&self, code: i32) {
        let msg = match code {
            -10 => "macOS 15.0 or later is required for Tahoe engine.".to_string(),
            -11 => "Speech model is not installed. Downloading in background...".to_string(),
            -12 => "Hardware does not support Tahoe engine (Neural Engine required).".to_string(),
            -13 => "Microphone permission denied.".to_string(),
            _ => format!("Failed to start speech recognition (Error: {})", code),
        };
        
        show_notification("mycute", &msg);
        
        // エラー時は自動的に Classic モードへフォールバックする等の処理も検討
        // if code == -11 || code == -12 { fallback_to_classic(); }
    }
}
```

### C. Swift ヘルパーの拡張 (`swift/SpeechHelper.swift`)
既存のコードを維持したまま、新しい Tahoe 専用ロジックを追加します。
- **変更内容**:
    - `SpeechAnalyzer` と `SpeechTranscriber` を用いた新しい認識ロジックをクラスまたは関数として実装。
    - `@_cdecl("tahoe_helper_start")` などの新しいエントリポイントを定義。
    - macOS 15 未満の環境では Tahoe 起動時にエラーを返すガードを追加。
- **理由**: `SFSpeechRecognizer` は `AVAudioEngine` のタップを利用しますが、`SpeechAnalyzer` は `AsyncSequence` 型のバッファストリームを利用するため、内部実装を分けるのが最も安全です。

```swift
// [NEW] Tahoe用グローバルステート（既存の audioEngine 等とは独立）
private var tahoeAnalyzer: Any? // SpeechAnalyzer (macOS 15+)
private var tahoeTask: Task<Void, Never>?

@_cdecl("tahoe_helper_start")
public func tahoeHelperStart(_ localePtr: UnsafePointer<CChar>?) -> Int32 {
    guard #available(macOS 26.0, *) else { return -10 }
    let localeStr = String(cString: localePtr!)
    let locale = Locale(identifier: localeStr)
    
    tahoeTask = Task {
        let transcriber = SpeechTranscriber(locale: locale, preset: .transcription)
        // AsyncSequence を利用したモダンな取得
        for try await result in transcriber.results {
            let text = String(result.text.characters)
            text.withCString { ptr in
                resultCallback?(ptr, result.isFinal ? 1 : 0)
            }
        }
    }
    return 0
}
```

## 4. Tahoe 実装の詳細（成功のための鍵）
3. **結果の取得**: `transcriber.results` (AsyncSequence) を `for await` で回し、得られたテキストを既存の `resultCallback` を通じて Rust 側に通知します。

### 4.1. 音声バッファのブリッジ (AVAudioPCMBuffer → AsyncSequence)
Tahoe APIは `AsyncSequence` を要求するため、`AVAudioEngine` のタップで得られる継続的なバッファを以下のように `AsyncStream` でラップして供給します。

```swift
// [NEW] Audio Stream Bridge
private var audioContinuation: AsyncStream<AVAudioPCMBuffer>.Continuation?

private func createAudioStream() -> AsyncStream<AVAudioPCMBuffer> {
    return AsyncStream { continuation in
        self.audioContinuation = continuation
        
        // 既存の audioEngine の物理タップをここに流し込む
        guard let engine = audioEngine else { return }
        let inputNode = engine.inputNode
        let recordingFormat = inputNode.outputFormat(forBus: 0)
        
        inputNode.installTap(onBus: 0, bufferSize: 1024, format: recordingFormat) { buffer, _ in
            self.audioContinuation?.yield(buffer)
        }
    }
}
```

### 4.1.1 終了処理の完全性
認識停止時には、`AsyncStream` を明示的に終了させる必要があります。
```swift
private func stopAudioStream() {
    audioContinuation?.finish()
    audioContinuation = nil
}
```

### 4.2. 出力結果の正規化 (Incremental → Cumulative)
Tahoeの `SpeechTranscriber` は認識された断片を流してくる場合があります。現在のRust側の `input_diff` ロジックと整合させるため、Swift側で全文を管理・正規化します。

```swift
// [NEW] 結果の正規化ロジック
tahoeTask = Task {
    var fullTranscript = ""
    let audioStream = createAudioStream()
    // analyzer.start にストリームを渡す
    try? await tahoeAnalyzer?.start(inputAudioStream: audioStream)

    // 上記の tahoeHelperStart 内で初期化された transcriber を使用
    for try await result in transcriber.results {
        // ... (正規化ロジック)
        fullTranscript = result.text 
        
        fullTranscript.withCString { ptr in
            resultCallback?(ptr, result.isFinal ? 1 : 0)
        }
    }
}
```
**理由**: このブリッジ層がないと、Rust側で「現在どこまで入力したか」の整合性が崩れ、テキストが重複したり消えたりする原因になります。

## 5. 動作確認と安全性の保証
1. **Classic モードの不変性確認**: `stt_engine: "classic"` の状態で、従来通り認識が行われることを確認します。
2. **Tahoe モードの有効性確認**: macOS 15 環境で `stt_engine: "tahoe"` に設定した際、より高速に応答が返ることを確認します。
3. **フォールバック**: Tahoe が利用できない環境（OSバージョン不足等）での適切なエラー表示を確認します。macOS 26.0 未満の環境では Classic モードでの動作が継続されます。

## 7. ランタイムでの言語切り替え機能 (Option + L)
起動中に言語を切り替えるためのトグル機能を実装します。

### 7.1. Rust側での切り替えロジック (`src/main.rs`)
`HotkeyAction::ToggleLocale` をハンドルし、通知を表示した上で内部のロケール状態を更新します。

```rust
match action {
    HotkeyAction::ToggleLocale => {
        let mut settings = config.write();
        let new_locale = if settings.locale == LocaleCode::Ja {
            LocaleCode::En
        } else {
            LocaleCode::Ja
        };
        settings.locale = new_locale;
        
        let msg = format!("Language changed to: {}", 
            if settings.locale == LocaleCode::Ja { "日本語" } else { "English" });
        show_notification("mycute", &msg);
    }
    // ...
}
```

### 7.2. 考慮点
- 言語切り替え時に現在進行中の録音がある場合は、一度安全に `stop` し、新しいロケールで再開可能な状態にします。
- 言語切り替えの通知は `notification.rs` を通じて即座にユーザーへ視覚的フィードバックを行います。

## 6. 追加の技術的考慮事項とリスク管理（徹底調査に基づく追記）
さらに完璧な実装を目指すため、以下の「最新API特有の注意点」を考慮に入れます。

### 6.1. 言語モデルアセットの管理 (`AssetInventory`)
Tahoe の認識エンジンはオンデバイスで動作しますが、初回利用時に言語モデル（学習済みデータ）のダウンロードが必要な場合があります。

```swift
// [NEW] Asset Inventory Check logic
@available(macOS 15.0, *)
private func checkAndRequestAssets(locale: Locale) async -> Bool {
    let inventory = await SpeechAnalyzer.AssetInventory.init(locale: locale)
    if inventory.isAnyStatus(.notInstalled) {
        // モデルがない場合はダウンロードをトリガー
        // 注意: 直接のダウンロード開始APIは制限されている場合があるため、
        // ユーザーにシステム設定を促すか、バックグラウンドでの進行を通知する
        return false
    }
    return true
}
```

### 6.2. 応答速度の向上 (`Preheating`)
認識開始時のレイテンシを最小化するため、初期化直後に `prepareToAnalyze(in:)` を呼び出し、エンジンを「ウォームアップ」状態にします。

```swift
// [NEW] Preheating logic
@available(macOS 15.0, *)
private func preheatTahoe(locale: Locale) {
    Task {
        let analyzer = SpeechAnalyzer(locale: locale)
        // 解析準備を整える
        try? await analyzer.prepareToAnalyze(in: .transcription)
    }
}
```

### 6.3. カスタム語彙（辞書）の非サポートへの対応
現在の Tahoe API (`SpeechTranscriber`) は、旧来の `SFSpeechRecognizer` で利用可能だった「カスタム語彙の追加（Contextual Strings）」をサポートしていません。
- **考慮点**: 専門用語などの認識精度が Classic モードに劣る可能性があるため、ユーザーが精度を優先したい場合に Classic モードへ戻せる選択肢（`settings.json` のスイッチ）が重要になります。

### 6.4. Volatile（揮発的）結果のハンドリング
Tahoe は確定前の「暫定的な結果」として Volatile 結果を返します。
- **実装方針**: ユーザーの体感速度を重視するため、Volatile 結果も積極的に Rust 側に流しますが、Classic モードとの挙動差（データの書き換え頻度）を吸収するため、Swift 側でタイミングを適切に制御します。

### 6.5. ハードウェア制約の事前診断
macOS 15 以上であっても、Intel Mac や古いハードウェアでは `SpeechTranscriber` が期待通りに動作しない、あるいは性能が極端に低い場合があります。
- **対応策**: `SpeechTranscriber.isAvailable` だけでなく、実際の初期化プロセスでのエラー（リソース不足等）をキャッチし、安全に動作を停止・通知するガードを強化します。
