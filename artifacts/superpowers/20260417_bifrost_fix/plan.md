# Bifrost 起動失敗の原因調査報告と修正計画

## 調査結果 (Root Cause Analysis)

`mycute.exe` を実行した際、バックエンドサーバー（RTモード）が Bifrost を起動しようとしますが、Bifrost がポート 3912 で応答せず、タイムアウトが発生していました。

直接 Bifrost バイナリ（`bifrost-http.exe`）を実行してログを確認したところ、以下のエラーが出力されていました：
`{"level":"error","message":"failed to bootstrap server: failed to load config failed to unmarshal schema: invalid character 'U' in string escape code"}`

### 原因の詳細
`src/bifrost/installer.rs` で生成される `config.json` において、Windows の絶対パスがそのまま埋め込まれています。

```json
"config": {
  "path": "C:\Users\user\.mycute\bifrost\config.sqlite"
}
```

JSON 規格では `\` はエスケープ文字であり、`C:\Users` の `\U` が不正な Unicode エスケープシーケンスとみなされています。正しくは `C:\\Users` または `C:/Users` と記述する必要があります。

## 修正案 (Proposed Changes)

### [Component] Bifrost Installer

#### [MODIFY] [installer.rs](file:///c:/Users/user/shyme/mycute/src/bifrost/installer.rs)

`generate_config_json` 関数において、パス文字列を置換する前にバックスラッシュをスラッシュに置換するか、ダブルバックスラッシュにエスケープする処理を追加します。

```rust
// 修正イメージ
let db_path_str = db_path.to_string_lossy().replace("\\", "/");
let log_path_str = log_path.to_string_lossy().replace("\\", "/");
```

> [!NOTE]
> JSON において Windows のパスを扱う場合、スラッシュ `/` に置換しても Go や Rust の標準ライブラリは正しく解釈できるため、この方法が最も簡潔で安全です。

## 検証計画 (Verification Plan)

### 手動検証
1. 修正後、`make build` を実行してバイナリを再生成する。
2. `C:\Users\user\.mycute\bifrost\config.json` を削除し、再度 `mycute.exe` を実行する。
3. `config.json` が正しくエスケープされていること、および Bifrost が正常に起動することを確認する。

## オープンクエスチョン

- 特になし。

---
報告は以上です。指示をお待ちします。
