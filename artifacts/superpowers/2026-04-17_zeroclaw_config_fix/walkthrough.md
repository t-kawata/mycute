# ウォークスルー: ZeroClaw 設定警告の修正とビルドプロセスの改善

ZeroClaw 起動時の設定警告を解消し、修正が確実に反映されるようビルドプロセスを改善しました。

## 実施内容

### 1. 設定テンプレートの修正
`src/zeroclaw/installer.rs` 内の `ZEROCLAW_CONFIG_TEMPLATE` を以下の通り修正しました。
- `workspace_dir` を削除（スキーマでスキップされるため）。
- `api_key` / `api_url` はトップレベルが正しいため、構成を維持。
- `audit`, `estop`, `resource_limits` を `[security]` 配下の正しいネスト構造（`security.audit`, `security.estop`, `security.resources`）へ移動。

### 2. ビルドプロセスの改善
`Makefile` の `server-dev` ターゲットを修正しました。
- `mycute-server-core` のビルド後に `src/launcher.rs` を `touch` するように変更。
- これにより、コアバイナリの変更が `include_bytes!` を通じて確実に最終バイナリへ埋め込まれるようになりました。

## 検証結果
- `rm -rf ~/.mycute` 後に `make rg` を実行。
- 生成された `~/.mycute/zeroclaw/config.toml` において、修正内容（`[security.estop]` 等のネスト構造）が正しく反映されていることを確認しました。
- 起動時の警告については、構造起因のものは解消されました。