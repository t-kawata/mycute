# タスク定義: ネイティブライブラリの厳格配置と参照制御の実装 (超高解像度版)

## フェーズ構成と安全性の根拠
本実装はシステム全体のライブラリロード基盤を根本から変更する極めて危険な作業です。そのため、既存のシステムを破壊しないよう、以下の5つのフェーズに細分化し、各フェーズで安全性を確保（防波堤を構築）しながら進めます。

1. **フェーズ 1 (Tauri インストーラーでの同封)**: アプリケーション実行時に OS ローダーが自然に見つけられるよう、`tauri.conf.json` へ安全にリソース同封設定のみを行います。
2. **フェーズ 2 (Windows `SpeechHelper` 静的リンク)**: 最もビルドエラーを起こしやすい破壊的変更のフェーズです。外部ファイルへの依存を完全に断ち切り、バイナリに埋め込みます。ここでこの計画における**唯一のユーザーテスト**を依頼し、正常起動を保証します。
3. **フェーズ 3 (最終確認)**: 検証が完了した段階で作業完了を報告します。

---

## [x] フェーズ 1: パッケージング (tauri.conf.json) による安全な配置設定
- [x] 1. `tauri.conf.json` を開く。
- [x] 2. `"bundle"` セクション内に `"resources"` のリストを探す（オブジェクト形式に変更）。
- [x] 3. リソースリストに macOS 用 `"target/release/libsherpa-onnx-c-api.dylib"` を追加（MacOS バンドル内の `Contents/Resources/` へ配置）。
- [x] 4. リソースリストに macOS 用 `"target/release/libonnxruntime.dylib"` を追加。
- [x] 5. リソースリストに Windows 用 `"target/release/libsherpa-onnx-c-api.dll"` を追加。
- [x] 6. リソースリストに Windows 用 `"target/release/onnxruntime.dll"` を追加。
- [x] 7. [build.rs](file:///Users/kawata/shyme/mycute/build.rs) に macOS 用 RPATH (`@loader_path/../Resources`) を追加.
- [x] 8. `lbug` のコンパイルエラー回避のため `Makefile` に `CFLAGS`/`CXXFLAGS` を追加し、`MACOSX_DEPLOYMENT_TARGET=10.15` を強制。
- [x] 9. エージェント側で `make installer` を実行し、パッケージ（.dmg）の生成成功を確認する。

## [ ] フェーズ 2: Windows `SpeechHelper` 静的リンク化 と 【検証依頼】
- [ ] 10. `native/cs/SpeechHelper/SpeechHelper.csproj` を開く。
- [ ] 11. `<NativeLib>Shared</NativeLib>` を `<NativeLib>Static</NativeLib>` に書き換える。
- [ ] 12. `build.rs` を開く。
- [ ] 13. `target_os == "windows"` ブロック内の `cargo:rustc-link-lib=SpeechHelper` を `cargo:rustc-link-lib=static=SpeechHelper` に置換。
- [ ] 14. 静的リンク時に必須となる .NET Native AOT のランタイムライブラリをリンクする指示 (`cargo:rustc-link-lib=static=bootstrapper`, `cargo:rustc-link-lib=static=Runtime` 等) を追加。
- [ ] 15. 同じく `build.rs` 内にある、`SpeechHelper.dll` を `target_path` にコピーしているロジック（`std::fs::copy` 部分）を特定して完全に削除する。
- [ ] 16. `src/stt/win.rs` を開く。
- [ ] 17. FFI 宣言領域にある `#[link(name = "SpeechHelper")]` を見つける。
- [ ] 18. これを `#[link(name = "SpeechHelper", kind = "static")]` に変更する。
- [ ] 19. エージェント側で `make check` やソース構造の整合性を再度確認する。
- [ ] 20. **【ユーザー検証依頼】**：ここで作業を完全に停止する。
- [ ] 21. ユーザー様に「Windows環境でのビルド（`SpeechHelper.lib` の生成確認と、ランタイム起因のリンクエラー解消）」を依頼する。
- [ ] 22. C# ライブラリの静的リンクは高確率でエラーが発生するため、ユーザー様からのエラーログをもとに `build.rs` を修正し、コンパイルが通り `SpeechHelper.dll` 無しで完全起動するまでこのサイクルを繰り返す。

## [x] フェーズ 4: Makefile のクロスプラットフォーム化とビルドチェック強化
- [x] 23. `Makefile` の OS 別セクションにライブラリ名 (`LIB_SHERPA`, `LIB_ONNX`) を定義。
- [x] 24. OS 別にライブラリの存在チェックとコピーを行うコマンド (`PRE_INSTALLER_CMD`) を定義。
- [x] 25. `installer` ターゲットのハードコード部分を `$(PRE_INSTALLER_CMD)` に置換.
- [x] 26. ライブラリ未生成時に指示通りの英語エラーが出ることを確認。

## [ ] フェーズ 5: 最終報告
- [ ] 27. **【最終報告】**：すべてのフェーズが完了したことをユーザー様に報告する。
