分析の結果、現在のコードがクラッシュ（`trace trap`）している最大の原因は、**`analyzer.start(inputSequence:)` の挙動に関する誤解と、それに関連するライフサイクル管理の破綻**にあります。

### クラッシュの根本原因：`analyzer.start` が即座に終了している

ログにある `[Tahoe] Analyzer.start() returned normally` は、本来起こってはいけない現象です。
ドキュメントおよび WWDC 2025 のセッション  によれば、`start(inputSequence:)` は **「非同期的に解析を開始し、入力ストリームが終了するまで待機（サスペンド）する」** ものではなく、**「解析プロセスをスケジュールして、即座に制御を戻す（Returns immediately）」** という設計になっています 。[1][2][3]

しかし、現在のコードでは以下のような矛盾が生じています。

1.  **早期終了**: `analyzer.start()` が即座にリターンした直後、TaskGroup 内のタスクが終了し始めます。
2.  **リソースの解放**: `analyzer.start()` が完了したとみなされると、背後で動いている解析エンジンや `transcriber` の接続が不安定になる、あるいは解放の準備に入ります。
3.  **不正アクセス**: その状態で `transcriber.results` をループ（`for try await`）しようとした瞬間に、無効なメモリ領域へのアクセスが発生し、`trace trap` (SIGTRAP) でクラッシュします 。[4]

### 修正が必要なポイント

1.  **`analyzer.start` を待機させない**: `analyzer.start` は呼び出し後に即座に戻るため、その後に **`analyzer.finalizeAndFinish()`** や **`analyzer.cancelAndFinishNow()`** を呼び出して、解析が完了するまで明示的に待機（await）する必要があります 。[2]
2.  **`result.text` の扱い**: エンジニアのコメントにある通り、`result.text` は `AttributedString` で正解です 。ただし、`String(result.text.characters)` で変換する際、`result` 自体が解析終了により無効化されているとクラッシュします。[2]
3.  **オーディオフォーマットの不整合**: ログに `44100.0Hz` とありますが、`SpeechAnalyzer` は特定のフォーマット（通常 16kHz や 48kHz）を好む場合があります。`SpeechAnalyzer.bestAvailableAudioFormat(compatibleWith:)` を使用して、システムが推奨するフォーマットで `prepareToAnalyze` を行うのが安全です 。[2]

### 修正案（抜粋）

以下の構成に変更することで、解析プロセスを正しく維持できます。

```swift
// Analyzer プロセス側の Task
group.addTask {
    do {
        // 1. 開始（これは即座に戻る）
        try await analyzer.start(inputSequence: stream)
        
        // 2. 解析が続いている間、ここで待機する必要がある
        // isRunning が false になるまで待機
        while self.isRunning && !Task.isCancelled {
            try await Task.sleep(nanoseconds: 100_000_000)
        }
        
        // 3. 終了処理を await することで、プロセスを安全に閉じる
        await analyzer.cancelAndFinishNow()
        print("[Tahoe] Analyzer finished and cleaned up")
    } catch {
        print("[Tahoe] Analyzer error: \(error)")
    }
}
```

また、`trace trap` を防ぐため、`resultCallback` を呼ぶ前に `text` が空でないか、`resultCallback` 自体が nil になっていないかをより厳密にチェックすることをお勧めします。

[1](https://developer.apple.com/documentation/speech/speechanalyzer)
[2](https://developer.apple.com/videos/play/wwdc2025/277/)
[3](https://developer.apple.com/documentation/speech/speechanalyzer/start(inputsequence:))
[4](https://community.toggl.com/t/app-crashes-on-launch-macos-26-beta-4/2830)
[5](https://developer.apple.com/documentation/speech/speechtranscriber)
[6](https://www.reddit.com/r/apple/comments/1lef13s/apples_new_transcription_apis_blow_past_whisper/)
[7](https://www.macrumors.com/2025/06/18/apple-transcription-api-faster-than-whisper/)
[8](https://zenn.dev/explaza/articles/d0e658c7862eac)
[9](https://gigazine.net/gsc_news/en/20250619-apple-speech-analyzer/)
[10](https://www.reddit.com/r/techsupport/comments/1cxi1bj/various_crashes_due_to_texttospeech_on_macos/)
[11](https://www.macstories.net/stories/hands-on-how-apples-new-speech-apis-outpace-whisper-for-lightning-fast-transcription/)
[12](https://applech2.com/archives/20250616-macos-26-tahoe-and-ios-26-speechanalyzer-api.html)
[13](https://discussions.apple.com/thread/255684326)
[14](https://gist.github.com/auramagi/9c040c2233dfe71c24c76942e186f788)
[15](https://forums.developer.apple.com/forums/thread/665843)