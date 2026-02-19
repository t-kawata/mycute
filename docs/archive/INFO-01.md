.NET 10 環境において、デスクトップアプリ（DLL）から `ContinuousRecognitionSession` を呼び出す際のコンパイルエラーおよび実装上の問題について、修正方針をまとめました。担当者への共有用としてご活用ください。
## ContinuousRecognitionSession 解決ガイド
### 1. 根本的な原因
デスクトップアプリ（Win32/.NET）から WinRT API である `Windows.Media.SpeechRecognition` 名前空間にアクセスする際、以下の 2 点がボトルネックとなっています。
- **TFM (Target Framework Moniker) の不足**: `net10.0` だけでは、Windows 固有の WinRT 型情報が解決されません。
- **Native AOT との互換性**: `PublishAot` が有効な場合、WinRT の動的な型解決がトリミング（削除）対象となり、「型が存在しない」というエラーを誘発します 。 [learn.microsoft](https://learn.microsoft.com/en-us/windows/apps/desktop/modernize/desktop-to-uwp-enhance)
***
### 2. プロジェクトファイル (.csproj) の修正
Native AOT を維持したまま WinRT API を呼び出すには、`CsWinRT` パッケージの導入と正しい TFM の指定が必須です。
```xml
<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <!-- 1. TFMをWindows 10/11固有のバージョンに変更 (WinRT APIの解決に必須) -->
    <TargetFramework>net10.0-windows10.0.19041.0</TargetFramework>
    
    <RuntimeIdentifiers>win-x64</RuntimeIdentifiers>
    <ImplicitUsings>enable</ImplicitUsings>
    <Nullable>enable</Nullable>
    
    <!-- 2. Native AOT設定 -->
    <PublishAot>true</PublishAot>
    
    <!-- 3. CsWinRTのAOT最適化を有効化 -->
    <CsWinRTAotOptimizerEnabled>true</CsWinRTAotOptimizerEnabled>
  </PropertyGroup>
  <ItemGroup>
    <!-- 4. WinRT APIのプロジェクション生成に必要なパッケージを追加 -->
    <PackageReference Include="Microsoft.Windows.CsWinRT" Version="2.1.1" />
  </ItemGroup>
</Project>
```
***
### 3. 実装上のチェックリスト
DLL 側で音声認識ロジックを再実装する際の技術的ポイントです。
| 項目 | 内容 |
| :--- | :--- |
| **名前空間の確認** | `System.Speech` ではなく `Windows.Media.SpeechRecognition` を使用しているか確認  [learn.microsoft](https://learn.microsoft.com/en-us/archive/msdn-magazine/2014/december/voice-recognition-speech-recognition-with-net-desktop-applications)。 |
| **非同期処理** | `StartAsync()` などの WinRT 非同期メソッドは、`.AsTask()` を使用して .NET の `Task` に変換する  [weblog.west-wind](https://weblog.west-wind.com/posts/2025/Mar/24/Using-WindowsMedia-SpeechRecognition-in-WPF)。 |
| **マイク権限** | アプリのマニフェストまたは Windows 設定でマイクアクセスが許可されている必要がある。 |
| **AOT互換性** | `CsWinRT` を導入することで、コンパイル時に必要な型情報が静的に生成され、`PublishAot` 下でも動作可能になる  [learn.microsoft](https://learn.microsoft.com/ja-jp/windows/apps/windows-app-sdk/release-notes/windows-app-sdk-1-6)。 |
***
### 4. 推奨されるトラブルシューティング手順
1. **キャッシュ削除**: `.csproj` 修正後、必ず `bin` および `obj` フォルダを削除してリビルドしてください。
2. **AOT の切り分け**: まず `<PublishAot>false</PublishAot>` でコンパイルが通ることを確認してください。
   - 通る場合：原因は AOT による型トリミングです。`CsWinRT` の設定を見直してください。
   - 通らない場合：原因は TFM 指定ミスか SDK の欠如です。
3. **ランタイムの確認**: DLL 実行環境に `.NET Desktop Runtime` がインストールされているか確認してください 。 [youtube](https://www.youtube.com/watch?v=g9pUURVI8mE)
以上の設定を適用することで、コメントアウトされていた `ContinuousRecognitionSession` の呼び出しを正常に復元できるはずです。