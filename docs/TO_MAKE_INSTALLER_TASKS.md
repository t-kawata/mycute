# タスク定義: ネイティブライブラリの厳格配置と参照制御の実装 (超高解像度版)

## フェーズ構成と安全性の根拠
本実装はシステム全体のライブラリロード基盤を根本から変更する極めて危険な作業です。そのため、既存のシステムを破壊しないよう、以下の5つのフェーズに細分化し、各フェーズで安全性を確保（防波堤を構築）しながら進めます。

1. **フェーズ 1 (Tauri インストーラーでの同封)**: アプリケーション実行時に OS ローダーが自然に見つけられるよう、`tauri.conf.json` へ安全にリソース同封設定のみを行います。
2. **フェーズ 2 (Windows `SpeechHelper` 静的リンク)**: 最もビルドエラーを起こしやすい破壊的変更のフェーズです。外部ファイルへの依存を完全に断ち切り、バイナリに埋め込みます。ここでこの計画における**唯一のユーザーテスト**を依頼し、正常起動を保証します。
3. **フェーズ 3 (最終確認)**: 検証が完了した段階で作業完了を報告します。

---

## [x] フェーズ 1: パッケージング (tauri.conf.json) と MacOS RPATH の安全な配置設定
- [x] 1. [tauri.conf.json](file:///Users/kawata/shyme/mycute/tauri.conf.json) を開く。
- [x] 2. `"bundle"` セクション内に `"resources"` をオブジェクト（マッピング形式）として作成する。
- [x] 3. マッピングに macOS 用 `"target/release/libsherpa-onnx-c-api.dylib": "libsherpa-onnx-c-api.dylib"` を追加する。
- [x] 4. マッピングに macOS 用 `"target/release/libonnxruntime.dylib": "libonnxruntime.dylib"` を追加する。
- [x] 5. マッピングに Windows 用 `"target/release/libsherpa-onnx-c-api.dll": "libsherpa-onnx-c-api.dll"` を追加する。
- [x] 6. マッピングに Windows 用 `"target/release/onnxruntime.dll": "onnxruntime.dll"` を追加する。
- [x] 7. [build.rs](file:///Users/kawata/shyme/mycute/build.rs) を開く。
- [x] 8. `target_os == "macos"` ブロック内を探す。
- [x] 9. macOS 用のビルド時に、Tauri がライブラリを配置する `Contents/Resources` ディレクトリを検索パス（RPATH）に含めるため、`println!("cargo:rustc-link-arg=-Wl,-rpath,@loader_path/../Resources");` を追加する。
- [x] 10. `lbug` (C++) のビルドエラー回避のため、`Makefile` に `CFLAGS`/`CXXFLAGS` を追加し `-mmacosx-version-min=10.15` を強制する修正を恒久化した。
- [x] 11. エージェント側で `make installer` を実行し、DMGパッケージの生成成功（Exit code 0）を確認した。

## [ ] フェーズ 2: Windows `SpeechHelper` 静的リンク化 と 【検証依頼】
- [ ] 12. [native/cs/SpeechHelper/SpeechHelper.csproj](file:///Users/kawata/shyme/mycute/native/cs/SpeechHelper/SpeechHelper.csproj) を開く。
- [ ] 13. `<NativeLib>Shared</NativeLib>` を `<NativeLib>Static</NativeLib>` に書き換える。
- [ ] 14. [build.rs](file:///Users/kawata/shyme/mycute/build.rs) を開く。
- [ ] 15. `target_os == "windows"` ブロック内の `cargo:rustc-link-lib=SpeechHelper` を `cargo:rustc-link-lib=static=SpeechHelper` に置換。
- [ ] 16. 静的リンク時に必須となる .NET Native AOT のランタイムライブラリをリンクする指示 (`cargo:rustc-link-lib=static=bootstrapper`, `cargo:rustc-link-lib=static=Runtime` 等) を追加。
- [ ] 17. 同じく [build.rs](file:///Users/kawata/shyme/mycute/build.rs) 内にある、`SpeechHelper.dll` を `target_path` にコピーしているロジック（`std::fs::copy` 部分）を特定して完全に削除する。
- [ ] 18. [src/stt/win.rs](file:///Users/kawata/shyme/mycute/src/stt/win.rs) を開く。
- [ ] 19. FFI 宣言領域にある `#[link(name = "SpeechHelper")]` を見つける。
- [ ] 20. これを `#[link(name = "SpeechHelper", kind = "static")]` に変更する。
- [ ] 21. エージェント側で `make check` やソース構造の整合性を再度確認する。
- [ ] 22. **【ユーザー検証依頼】**：ここで作業を完全に停止する。
- [ ] 23. ユーザー様に「Windows環境でのビルド（`SpeechHelper.lib` の生成確認と、ランタイム起因のリンクエラー解消）」を依頼する。
- [ ] 24. C# ライブラリの静的リンクは高確率でエラーが発生するため、ユーザー様からのエラーログをもとに [build.rs](file:///Users/kawata/shyme/mycute/build.rs) を修正し、コンパイルが通り `SpeechHelper.dll` 無しで完全起動するまでこのサイクルを繰り返す。

## [ ] フェーズ 3: 最終報告
- [ ] 25. **【最終報告】**：すべてのフェーズが完了したことをユーザー様に報告する。
