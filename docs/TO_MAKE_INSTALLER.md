# 実装計画書: ネイティブライブラリの厳格配置と参照制御

## 目的
実行ファイルやカレントディレクトリ等の曖昧なパスからネイティブライブラリ（DLL / dylib）がロードされることを防ぎ、すべての外部ライブラリ依存を**実行ファイルと同じディレクトリ（macOS の AppBundle 内や Windows の .exe と同階層）**へ厳格に集約して同封する。
これにより、OS 標準のダイナミックローダー（`dyld` や `LoadLibrary`）の安全な標準挙動を利用して確実な起動を保証する。
また、Windows 用の SpeechHelper は実行ファイルへ静的リンク（埋め込み）し、単一の成果物とする。

## 変更内容
### 1. Tauri インストーラーによるライブラリリソースの確実な配置と RPATH 設定
`tauri.conf.json` の `bundle` リソース設定において、各 OS 向けライブラリをマッピング指定し、確実に出力バンドルに含める。さらに macOS では隔離されたリソースフォルダをローダーが見つけられるよう `build.rs` を調整する。

#### [MODIFY] [tauri.conf.json](file:///Users/kawata/shyme/mycute/tauri.conf.json)
- `bundle > resources` にマッピングオブジェクトを追加し、ビルドされたライブラリをインストーラーに同封する。
  - **Windows**: `"target/release/libsherpa-onnx-c-api.dll": "libsherpa-onnx-c-api.dll"`, `"target/release/onnxruntime.dll": "onnxruntime.dll"` （ルートに同封される）
  - **macOS**: `"target/release/libsherpa-onnx-c-api.dylib": "libsherpa-onnx-c-api.dylib"`, `"target/release/libonnxruntime.dylib": "libonnxruntime.dylib"` （`$RESOURCES` すなわち `Contents/Resources/` に同封される）

#### [MODIFY] [build.rs](file:///Users/kawata/shyme/mycute/build.rs)
- macOS において、Tauri の `resources` 先である `Contents/Resources/` と実行ファイルのフォルダ（`Contents/MacOS/`）間に生じるギャップを埋めるため、Rust 実行ファイルの RPATH に `@loader_path/../Resources` を追加し、OS ローダーがリソースフォルダからもライブラリを探せるようにする。

---

### 2. Windows: `SpeechHelper` の静的リンク化
C# で書かれた Windows 向けの `SpeechHelper` を DLL 出力から Static Library (`.lib`) 出力へ変更し、Rust バイナリへ完全に埋め込む。

#### [MODIFY] native/cs/SpeechHelper/SpeechHelper.csproj
- `<NativeLib>Shared</NativeLib>` を `<NativeLib>Static</NativeLib>` に変更。
- `<SelfContained>true</SelfContained>` などの AOT プラグマを維持し、スタンドアロンな静的ライブラリを生成するように設定。

#### [MODIFY] build.rs
- `cargo:rustc-link-lib=SpeechHelper` を `cargo:rustc-link-lib=static=SpeechHelper` に変更。
- DLL ファイル (`SpeechHelper.dll`) のコピー処理を削除。
- 静的リンク時に必要となる .NET Native AOT のランタイムライブラリ群 (`bootstrapper.lib`, `Runtime.lib` など。環境によりビルドエラーを見ながら過不足なく追加) を `cargo:rustc-link-lib=static=...` として追加。

#### [MODIFY] src/stt/win.rs
- FFI 宣言ブロックにある `#[link(name = "SpeechHelper")]` を `#[link(name = "SpeechHelper", kind = "static")]` に変更。

---

### 3. 外部ライブラリ (sherpa-onnx等) の動的ロードの検証（手動ロードの廃止）
過去の計画では `LoadLibraryExW` や `dlopen` による強制ロードを検討したが、macOS において `dyld` は `main()` 実行前にコンパイル時参照の解決を試みるため、後からの介入は起動クラッシュを引き起こすことが判明した。
したがって、特別な Rust コード（`native_lib.rs` など）によるロードパスの書き換えは行わず、**OSの標準的なロード機構に完全に任せる**。
設定ファイル（`tauri.conf.json`）による同一ディレクトリへの確実なデプロイこそが、最も安全な解決策である。

---

## 開発・検証環境とクロスプラットフォーム対応戦略

### 開発環境
- 本実装は **macOS (Apple Silicon)** 環境にて実施される。
- したがって、macOS 用のビルドおよび動作検証はエージェント側で完結可能であるが、**Windows 用のバイナリ生成および実機テストはエージェント側では不可能**である。

### クロスプラットフォーム検証フロー
実装の正確性を担保するため、以下のステップでユーザーと連携して検証を進める。

1. **macOS 版の先行実装と検証**: 
   - `~/.mycute/lib` へのデプロイおよびロード制御を macOS で先行実装し、期待通り動作することを確認する。
2. **Windows 版の並行実装 (コードベース)**: 
   - `SetDllDirectoryW` や静的リンク設定など、Windows 固有のコードを Mac 上で記述する。
3. **ユーザーへの Windows テスト依頼**:
   - macOS での実装と Windows 用のコード記述が完了した段階で、ユーザーに報告する。
   - ユーザーは Windows 環境へソースを同期し、ビルド（`make windows`）および動作テスト（`~/.mycute/lib` からのロード確認）を実行する。
4. **フィードバックと修正**:
   - ユーザーからのテスト結果（ビルドログや実行時ログ）に基づき、エージェントが Mac 上で修正案を作成し、再度ユーザーに検証を依頼する。
   - このサイクルを繰り返し、Mac/Windows 両環境での足並みを揃えて完成させる。

---

## 検証計画

### 自動テスト (Automated Tests)
- `make check`: `cargo clippy` およびビルドチェックが通ることを確認。
- `make test`: すべてのユニットテストが通過することを確認。

### 手動検証 (Manual Verification)
1. **Windows ビルド検証**
   - `make windows` を実行し、C# のビルドが成功し、Rust バイナリが静的リンクの警告なしに作成されることを確認。
   - `target/release/` 配下に `SpeechHelper.dll` が存在しなくてもアプリケーションが起動し、音声入力が動作することを確認（依存関係が内包された証明）。
2. **インストーラーの検証 (macOS / Windows 共通)**
   - アプリケーションをビルド・インストール（`tauri build`）する。
   - macOS の場合は `.app` バンドル内の `Contents/MacOS/` 内に、Windows の場合はインストール先フォルダ内に、対象のネイティブライブラリが正しく同封されていることを確認する。
   - インストールしたアプリが正常起動することを確認。
