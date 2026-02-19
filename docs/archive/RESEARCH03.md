詳細な分析結果です。現在のコードが `[Tahoe] Results loop started` の直後にクラッシュしている根本原因は、複数の設計上の欠陥が重層的に作用しています。

## 根本原因の特定

### **1. `transcriber.results` が存在しない可能性（最大の疑い）**

ログの流れから、最もクリティカルな問題として以下が考えられます：

```swift
for try await result in transcriber.results {
```

このコード行で **`results` プロパティが実際には `SpeechTranscriber` に存在しない**、あるいは**予期しない型**である可能性が高いです。

**理由：**
- WWDC 2025 の新しい Tahoe API では、`SpeechTranscriber` は `SpeechAnalyzer` の配下で動作するモジュール
- 結果の取得は `analyzer.results` や `analyzer.transcripts` などを通じて行われるべき設計
- 「`transcriber.results`」は直接アクセスできない可能性が**非常に高い**

### **2. `result.text.characters` の安全性問題**

```swift
let text = String(result.text.characters)
```

この行で `result` のメモリが既に不正な状態にある可能性があります。特に：
- `result` が weak reference である場合、既に解放されている
- `AttributedString.characters` の読み取り時に、背後のストレージが無効化されている

### **3. スレッドセーフティの欠如**

`transcriber.results` ループと `analyzer.start()` の相互作用において、Tahoe API の内部スレッドモデルとの競合が発生している可能性があります。

***

## **解決策：正しい API 使用法への修正**

Tahoe（macOS 26.2）の `SpeechAnalyzer` フレームワークは、結果取得の正式な方法が異なります。以下の修正が必須です。

### **修正版コード（key changes only）**

```swift
// 11. Run Analysis and Results loop in a TaskGroup
await withTaskGroup(of: Void.self) { group in
    // Sub-task: Analyzer Process
    group.addTask {
        print("[Tahoe] Analyzer.start() called")
        do {
            // analyzer.start() は結果を配信しながら実行される
            try await analyzer.start(inputSequence: stream)
            print("[Tahoe] Analyzer.start() returned normally")
        } catch {
            print("[Tahoe] Analyzer.start() error: \(error)")
        }
    }
    
    // Sub-task: Results Collection (from analyzer, NOT transcriber)
    group.addTask {
        print("[Tahoe] Results loop started")
        do {
            // 重要：transcriber.results ではなく analyzer.results を使用する
            // （または analyzer.analyzeStream() などの別メソッド）
            for try await result in analyzer.results {
                if Task.isCancelled || !self.isRunning { break }
                
                // analyzer.results の構造は異なるため、以下を確認が必要
                // 典型的には: result.transcripts[0]?.formattedString など
                let text: String
                if let transcript = result.transcripts.first {
                    text = transcript.formattedString
                } else {
                    continue
                }
                
                // isFinal の判定も analyzer.Result 型に依存
                let isFinal: Int32 = result.isFinal ? 1 : 0
                
                print("[Tahoe] Result: '\(text)' (final=\(isFinal))")
                
                await MainActor.run {
                    if let cb = resultCallback {
                        text.withCString { ptr in
                            cb(ptr, isFinal)
                        }
                    }
                }
            }
        } catch {
            print("[Tahoe] Results loop error: \(error)")
        }
        print("[Tahoe] Results loop finished")
    }
    
    // Keep group alive
    while self.isRunning && !Task.isCancelled {
        try? await Task.sleep(nanoseconds: 100_000_000)
    }
    group.cancelAll()
}
```

### **重要な確認項目**

**1. API の正式な結果取得メソッドを確認**
   ```swift
   // これらのうち実際に存在するものを使用すること
   analyzer.results              // ← 確認必須
   analyzer.transcripts          // ← 別の可能性
   transcriber.results           // ← 存在しない可能性が高い
   analyzer.start(inputSequence:) // の戻り値 ← 最も可能性あり
   ```

**2. Xcode のコード補完で確認**
   - `analyzer.` と入力して、補完候補に何が出現するかを記録
   - `transcriber.` と入力して、実際に `.results` が補完候補に出るか確認

**3. コンパイラ警告の確認**
   - `-Wall` フラグでコンパイルして、型不一致の警告がないか確認

***

## **推奨されるデバッグ手順**

1. **プロパティの存在確認**
   ```swift
   print("[Tahoe] Transcriber type: \(type(of: transcriber))")
   print("[Tahoe] Transcriber members: \(Mirror(reflecting: transcriber).children)")
   print("[Tahoe] Analyzer type: \(type(of: analyzer))")
   ```

2. **結果オブジェクトの構造確認**
   ```swift
   // 最初の結果を受け取ったら即座に型を検査
   for try await result in analyzer.results {
       print("[Tahoe] Result type: \(type(of: result))")
       print("[Tahoe] Result: \(Mirror(reflecting: result).children)")
       break // 最初の1つだけ検査
   }
   ```

3. **WWDC 2025 セッション動画の確認**
   - 「Bring advanced speech-to-text to your app with SpeechAnalyzer」の実装例コード
   - 結果取得の正式な方法がそこに記載されている可能性が高い

***

## **最も可能性の高い原因**

**`transcriber.results` を直接反復処理しようとしているが、Tahoe API ではそのプロパティが存在しないか、別の型である。その結果、未定義メモリへのアクセスが発生し、`trace trap` でクラッシュしている。**

正しい API は `analyzer` オブジェクトを通じて結果を取得する設計になっているはずです。