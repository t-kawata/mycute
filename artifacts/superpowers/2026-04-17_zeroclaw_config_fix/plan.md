# ZeroClaw 設定警告の修正およびビルドプロセスの改善計画

ZeroClaw の設定警告（`installer.rs`）の修正を確実に反映させるため、`Makefile` のビルド依存関係を改善します。

## ユーザーレビューが必要な項目
- `Makefile` の `server-dev` ターゲットを修正し、埋め込みバイナリの更新を確実にします。

## 修正内容

### [Component] Build System

#### [MODIFY] [Makefile](file:///Users/kawata/shyme/mycute/Makefile)
- `server-dev` ターゲットにおいて、`mycute-server-core` のビルド後に `src/launcher.rs` を `touch` する順序を厳密化し、確実に最新のコアバイナリが埋め込まれるようにします。

### [Component] ZeroClaw Installer (継続)

#### [MODIFY] [installer.rs](file:///Users/kawata/shyme/mycute/src/zeroclaw/installer.rs)
- 前回の修正（`api_key`, `api_url` のトップレベル復帰、セキュリティ項目のネスト化）が完了していることを確認します。

## 検証計画

### 自動テスト
- `make check-be` を実行し、ビルドエラーがないことを確認します。

### 手動確認
1. `rm -rf ~/.mycute` を実行。
2. `make rg` を実行。
3. `~/.mycute/zeroclaw/config.toml` を開き、修正したテンプレート（`[security.resources]` 等）が反映されていることを確認します。