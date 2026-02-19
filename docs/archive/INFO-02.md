担当技術者の方が「APIセットに含まれない」と仰っている状況は、.NET 10 の標準ライブラリ（BCL）だけを参照している、あるいは **WinRT のメタデータ（投影）が AOT コンパイル時に正しく解決されていない** ことに起因します。
この問題を完全に解決するための「技術仕様まとめ」を作成しました。そのまま担当者の方へお渡しください。
***
## 【技術共有】ContinuousRecognitionSession の参照不可問題と解決策
### 1. 現象の正体
`ContinuousRecognitionSession` は `Windows.Media.SpeechRecognition` 名前空間に属する **WinRT API** です。これは .NET 10 の標準的なクラスライブラリ（`System.*` など）には含まれず、Windows OS のメタデータから動的に投影されるクラスです。
デスクトップアプリ（特に Native AOT を使用する場合）では、コンパイラが「どの DLL にこの型があるか」を静的に特定できず、参照不可（存在しない）と判定されます 。 [learn.microsoft](https://learn.microsoft.com/en-us/windows/apps/desktop/modernize/desktop-to-uwp-enhance)
### 2. 技術的な解決手順（修正案）
#### A. プロジェクトファイルの TFM 修正
通常の `net10.0` ではなく、Windows の型情報を明示的に含むターゲット（TFM）にする必要があります。
```xml
<TargetFramework>net10.0-windows10.0.19041.0</TargetFramework>
```
#### B. CsWinRT による静的バインディングの生成
`PublishAot` が有効な場合、実行時の動的な型解決ができないため、**NuGet パッケージ `Microsoft.Windows.CsWinRT`** を導入して、コンパイル時に C# 用のラッパークラスを生成させる必要があります 。 [learn.microsoft](https://learn.microsoft.com/ja-jp/windows/apps/windows-app-sdk/release-notes/windows-app-sdk-1-6)
#### C. プロジェクトファイルの完全な構成例
以下の設定により、API セットが解決され、コンパイルエラーが解消されます。
```xml
<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <TargetFramework>net10.0-windows10.0.19041.0</TargetFramework>
    <PublishAot>true</PublishAot>
    <ImplicitUsings>enable</ImplicitUsings>
    <Nullable>enable</Nullable>
    <!-- AOT環境でWinRTを正常に動作させるためのフラグ -->
    <CsWinRTAotOptimizerEnabled>true</CsWinRTAotOptimizerEnabled>
  </PropertyGroup>
  <ItemGroup>
    <!-- APIセット(投影)を生成するためのライブラリ -->
    <PackageReference Include="Microsoft.Windows.CsWinRT" Version="2.1.1" />
  </ItemGroup>
</Project>
```
### 3. 注意が必要な「デスクトップアプリの制限」
この API は本来 UWP 用であるため、デスクトップアプリから呼び出す際は以下の 2 点が必須です：
1.  **マイク権限の宣言**: アプリがパッケージ化（MSIX）されていない場合でも、Windows の「プライバシー設定」でデスクトップアプリのマイクアクセスが許可されている必要があります 。 [learn.microsoft](https://learn.microsoft.com/en-us/uwp/api/windows.media.speechrecognition.speechcontinuousrecognitionsession?view=winrt-26100)
2.  **非同期メソッドの変換**: `StartAsync()` などの戻り値は `IAsyncOperation` です。C# 側で `using System.Runtime.InteropServices.WindowsRuntime;` を追加し、`.AsTask()` で `Task` に変換して await する必要があります 。 [weblog.west-wind](https://weblog.west-wind.com/posts/2025/Mar/24/Using-WindowsMedia-SpeechRecognition-in-WPF)
### 4. 推奨される確認アクション
1.  **キャッシュ破棄**: `.csproj` 書き換え後、必ず `obj` と `bin` フォルダを手動削除してからビルドしてください。
2.  **型名の確認**: `System.Speech.Recognition`（旧デスクトップ用）と混同していないか、必ず `Windows.Media.SpeechRecognition` をインポートしているか確認してください。
この構成を適用すれば、API セットが正しくプロジェクトに含まれ、`ContinuousRecognitionSession` へのアクセスが可能になります。
Collapse














