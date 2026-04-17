# ウォークスルー: Bifrost 起動エラーの修正

Windows 環境において、Bifrost が設定ファイルの読み込みに失敗し、起動が停止していた問題を修正しました。

## 実施内容

### 1. 原因の特定
Bifrost の起動ログを確認したところ、`invalid character 'U' in string escape code` というエラーが発生していました。これは、`config.json` 内に Windows のパス（`C:\Users\...`）がエスケープされずに書き込まれていたため、JSON フォーマットエラーが生じていたことが原因でした。

### 2. 修正の実施
`src/bifrost/installer.rs` の `generate_config_json` 関数を修正しました。パス文字列を JSON テンプレートに埋め込む前に、バックスラッシュ `\` を全てスラッシュ `/` に置換するように変更しました。

> [!TIP]
> Go や Rust のライブラリは、Windows 上でもスラッシュ区切りのパスを正しく解釈できるため、この修正は安全かつ効果的です。

### 3. 検証
- **ユニットテストの追加と実行**: `src/bifrost/installer.rs` に専用のテストを追加し、生成された JSON が有効であること、およびバックスラッシュが含まれていないことを確認しました。
  - テストコマンド: `cargo test bifrost::installer::tests::test_generate_config_json_escaping`
  - 結果: **Passed**
- **コンパイル確認**: `make check-be` によりバックエンド全体のビルドに問題がないことを確認しました。

## 修正後の確認方法

1. 現在インストールされている `config.json` を一度削除します（修正後のバイナリで再生成させるため）。
   - パス: `C:\Users\user\.mycute\bifrost\config.json`
2. 最新のソースコードでビルドされた `mycute.exe` を実行します。
3. `bifrost-http.exe` が正常に起動し、その後 `zeroclaw` の起動まで進むことを確認してください。

---
作業が完了しました。修正が反映されていることをご確認ください。
