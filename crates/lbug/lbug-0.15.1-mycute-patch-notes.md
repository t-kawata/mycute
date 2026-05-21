# lbug 0.15.1 mycute カスタムパッチ

このパッチは `lbug` クレート v0.15.1 を Windows / macOS 両方で確実にビルドできるよう調整したものです。

## 変更内容

### 1. `build.rs` — zstd シンボル競合の回避

オリジナルは `zstd` を `whole-archive` でリンクしていたが、`zstd-sys` クレートとのシンボル競合（多重定義）が発生。`zstd` だけ whole-archive から外し、通常の静的リンクに変更した。

### 2. `build.rs` — macOS ビルド対応

macOS 向け CMake ビルドに `CMAKE_OSX_DEPLOYMENT_TARGET=13.3` を指定。cc クレート経由の C++ コンパイルにも `-mmacosx-version-min=13.3` を追加。

### 3. `build.rs` — Windows CRT 統一

`CMAKE_MSVC_RUNTIME_LIBRARY` を `MultiThreadedDLL`（/MD）から `MultiThreaded`（/MT）に変更。cc クレート側も `static_crt(true)` にすることで、Rust / NativeAOT の /MT 設定と統一した。

### 4. `build.rs` — yyjson のリンク追加

オリジナルでリンクが漏れていた `yyjson` を whole-archive の対象に追加。

### 5. `src/ffi.rs` — 警告抑制

C → Rust FFI バインディングから大量に出るコンパイラ警告を抑制するため、ファイル先頭に `#![allow(...)]` 3行を追加。

## 適用方法

```bash
# crates/lbug/lbug-0.15.1/ に対して以下のパッチを適用:
patch -p0 < crates/lbug/lbug-0.15.1-mycute.patch
```

## ファイル構成

- `lbug-0.15.1/` — パッチ適用済みのソース
- `lbug-0.15.1-mycute.patch` — crates.io 公開版からの差分
