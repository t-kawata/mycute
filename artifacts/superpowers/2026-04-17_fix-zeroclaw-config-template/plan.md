# 実装計画書: ZeroClaw 設定テンプレートの修正と警告の抑制 (修正版)

ZeroClaw 起動時に出力される「Unknown config key ignored」という警告を解消するため、`src/zeroclaw/installer.rs` 内の設定テンプレートを最新のスキーマおよび提供された資料に基づいて修正します。

## 目的
- `config.toml` 生成時の警告（api_key, api_url, audit, estop, resource_limits, workspace_dir）を排除する。
- 設定項目を正しい階層構造（`security` セクションへのネスト）に整理する。
- 無視される項目（`workspace_dir`）を削除する。

## ユーザー承認が必要な事項
- `workspace_dir` を TOML から削除しますが、これは資料の通り環境変数やバイナリ位置から自動解決されるため、動作に支障はありません。
- **`api_key` へのダミー値設定**: 空文字列による警告を避けるため、コメントアウトではなく `"YOUR_API_KEY_HERE"` というダミー値を設定します。環境変数 `ZEROCLAW_API_KEY` を設定している場合はそちらが優先されます。

## 変更内容

### 1. [installer.rs](file:///Users/kawata/shyme/mycute/src/zeroclaw/installer.rs) の修正

#### [MODIFY] `ZEROCLAW_CONFIG_TEMPLATE` の更新
- **`api_key`**: `api_key = ""` を **`api_key = "YOUR_API_KEY_HERE"`** に変更します。
- **`api_url`**: 資料に基づきトップレベルに維持します。
- **`workspace_dir`**: スキーマでスキップされているため、テンプレートから削除します。
- **セキュリティ設定の整理**:
    - `[estop]` セクションを `[security.estop]` に変更し、内容をネストします。
    - `[resource_limits]` セクションを `[security.resources]` に変更し、内容をネストします。
    - `[audit]` セクションを `[security.audit]` に変更し、内容をネストします。
    - これらを既存の `[security]` セクションの定義箇所（132行目付近）に集約し、重複を排除します。

## 検証計画

### 自動テスト
- `cargo check` を実行し、コード自体にコンパイルエラーがないことを確認します。
- `make check-be` を使用してプロジェクト全体のビルドを確認します。

### 手動検証
1. 現行の `~/.mycute/zeroclaw/config.toml` をバックアップまたは削除します。
2. アプリケーションを再起動し、新しい `config.toml` が正常に生成されることを確認します。
3. ZeroClaw 起動時のログを確認し、「Unknown config key ignored」の警告が消失していることを確認します。
4. ZeroClaw が正常に Bifrost (api_url) と通信できているか、動作確認を行います。

## 自己レビュー
- **Blocker**: なし。
- **Major**: なし。
- **Minor/Nit**: テンプレート内の日本語コメントの整合性を維持します。
