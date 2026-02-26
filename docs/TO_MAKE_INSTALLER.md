# Makefile のクロスプラットフォーム化とビルドチェック強化

`make installer` コマンドが Windows でも適切に動作するようにし、かつ `target/release/deps/` に必要なライブラリが存在しない場合に分かりやすい日本語のエラーメッセージを表示して終了するように修正しました。

## Proposed Changes

### [Component Name] Makefile

#### [MODIFY] [Makefile](file:///Users/kawata/shyme/mycute/Makefile)

- `OS_Detection` ブロックで、OS ごとにネイティブライブラリ名（`LIB_SHERPA`, `LIB_ONNX`）を定義しました。
- `INSTALLER_RESOURCES_CONFIG` を変数を使用して共通化し、DRY な記述にしました。
- `installer` ターゲットにおいて、OS ごとのシェル（PowerShell / POSIX bash）に対応したライブラリ存在チェックとコピーロジックを実装しました。

## Verification Plan

### Automated Tests
- macOS 環境で `make installer` を実行し、ライブラリがない場合にエラーが出るか、あれば正しくコピーされるかを確認（検証済）。
- Windows 環境（ユーザー環境）でのビルド成功を確認（今後実施）。

### Manual Verification
1. `target/release/deps/` 内のライブラリを一時的にリネームして `make installer` を実行し、指示通りの日本語エラーが出ることを確認（検証済）。
2. リリースビルド後に実行して、正しく構築が進むことを確認。
