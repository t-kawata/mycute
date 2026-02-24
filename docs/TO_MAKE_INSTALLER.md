# 実装計画書: ネイティブライブラリの厳格配置と参照制御

## 目的
実行ファイルやカレントディレクトリ等の曖昧なパスからネイティブライブラリ（DLL / dylib）がロードされることを防ぎ、すべての外部ライブラリ依存を `~/.mycute/lib` ディレクトリへ厳格に集約・制御する。
また、Windows 用の SpeechHelper は実行ファイルへ静的リンク（埋め込み）し、単一の成果物とする。

## 変更内容
### 1. `MYCUTE_HOME` ディレクトリ構造の拡張とデプロイ制御
`~/.mycute/lib` を新たにアプリケーションの必須ディレクトリとして定義し、ビルド時に同封されたライブラリを初回起動時に自動展開するようにする。

#### [MODIFY] [tauri.conf.json](file:///Users/kawata/shyme/mycute/tauri.conf.json)
- `bundle > resources` に、ビルド後に [target](file:///Users/kawata/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/sherpa-rs-sys-0.6.8/build.rs#57-72) ディレクトリに生成される動的ライブラリを追加する。
  - **Windows**: `libsherpa-onnx-c-api.dll`, `onnxruntime.dll`
  - **macOS**: `libsherpa-onnx-c-api.dylib`, `libonnxruntime.dylib`
- これにより、各 OS のインストーラーに適切なバイナリが自動同封される。

#### [MODIFY] [constants.rs](file:///Users/kawata/shyme/mycute/src/constants.rs)
- `MYCUTE_LIB_DIRNAME: &str = "lib";` の追加。

#### [MODIFY] [my_path.rs](file:///Users/kawata/shyme/mycute/src/utils/my_path.rs)
- [ensure_directories](file:///Users/kawata/shyme/mycute/src/utils/my_path.rs#42-62) 関数に `home.join(MYCUTE_LIB_DIRNAME)` の作成処理を追加。
- `get_lib_dir` ヘルパー関数の追加。

#### [NEW] [lib_deploy.rs](file:///Users/kawata/shyme/mycute/src/utils/lib_deploy.rs)
- `deploy_native_libs(app_handle, lib_dir)` の実装。
- `app_handle.path().resource_dir()` から同封されたライブラリを特定し、`~/.mycute/lib` へコピーする。
- 既にファイルが存在し、かつバージョンやサイズが一致する場合はスキップし、起動速度を確保する。

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

### 3. 外部ライブラリ (sherpa-onnx等) の動的ロードパスの強制
`sherpa-rs` や ONNX Runtime のような外部動的ライブラリが、OS のデフォルトサーチパスからではなく、**例外なく `~/.mycute/lib` からのみロードされるように** プログラム起動の最初期段階で OS のローダーパスを書き換える。

#### [MODIFY] [main_of_cl.rs](file:///Users/kawata/shyme/mycute/src/mode/cl/main_of_cl.rs)
1. **デプロイ**: [main_of_cl](file:///Users/kawata/shyme/mycute/src/mode/cl/main_of_cl.rs#117-434) の初期段階で `lib_deploy::deploy_native_libs` を呼び出し、同封ライブラリを `~/.mycute/lib` へ展開。
2. **パス強制**: 次に `native_lib::force_native_lib_path` を呼び出し、OS 固有の API を用いてライブラリ検索パスを `~/.mycute/lib` に強制固定する。

#### [NEW] [native_lib.rs](file:///Users/kawata/shyme/mycute/src/utils/native_lib.rs)
- `force_native_lib_path(lib_dir)` を実装し、特定のライブラリのみを `~/.mycute/lib` から絶対パスでピンポイントにロードする。
- **安全性**: プロセス全体の検索パスを変更せず、対象ライブラリのみを強制指定することで、OS 標準ライブラリ等への副作用を完全に排除する。

```rust
use std::path::Path;

/// 特定のネイティブライブラリを ~/.mycute/lib から絶対パスで強制的にロードする。
/// これにより、OS のデフォルト検索パスを汚染することなく、特定のライブラリのみを厳格に制御する。
pub fn force_native_lib_path(lib_dir: &Path) {
    let libs = if cfg!(windows) {
        vec!["libsherpa-onnx-c-api.dll", "onnxruntime.dll"]
    } else {
        vec!["libsherpa-onnx-c-api.dylib", "libonnxruntime.dylib"]
    };

    for lib in libs {
        let lib_path = lib_dir.join(lib);
        if !lib_path.exists() {
            log::warn!("Native library not found at: {:?}", lib_path);
            continue;
        }

        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;
            extern "system" {
                fn LoadLibraryExW(lpLibFileName: *const u16, hFile: isize, dwFlags: u32) -> isize;
            }
            const LOAD_WITH_ALTERED_SEARCH_PATH: u32 = 0x00000008;
            
            let mut path_u16: Vec<u16> = lib_path.as_os_str().encode_wide().collect();
            path_u16.push(0);
            
            let handle = unsafe { LoadLibraryExW(path_u16.as_ptr(), 0, LOAD_WITH_ALTERED_SEARCH_PATH) };
            if handle == 0 {
                log::error!("CRITICAL: Failed to LoadLibraryExW {:?}", lib_path);
            } else {
                log::info!("Pre-loaded native library (Windows): {:?}", lib_path);
            }
        }

        #[cfg(target_os = "macos")]
        {
            let path_cstr = std::ffi::CString::new(lib_path.to_str().unwrap()).unwrap();
            let handle = unsafe { libc::dlopen(path_cstr.as_ptr(), libc::RTLD_NOW | libc::RTLD_GLOBAL) };
            if handle.is_null() {
                log::error!("CRITICAL: Failed to dlopen {:?}", lib_path);
            } else {
                log::info!("Pre-loaded native library (macOS): {:?}", lib_path);
            }
        }
    }
}
```

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
2. **パス強制の検証 (macOS / Windows 共通)**
   - アプリケーション起動前に、わざと別の場所に古い `libsherpa-onnx-c-api.dylib` や `onnxruntime.dll` を配置し、環境変数などに設定する。
   - `~/.mycute/lib/` に正規の ONNX ランタイムを配置して起動。
   - 実行時のプロセスエクスプローラー（または `lsof` / `Process Explorer`）で、ロードされているモジュールの物理パスが確実に `~/.mycute/lib/` 以下のファイルのみであることを確認。これ以外の場所からライブラリが読み込まれていれば失敗とする。
