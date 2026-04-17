# ウォークスルー: ZeroClaw 設定テンプレートの修正

ZeroClaw 起動時に出力されていた `Unknown config key ignored` 警告を解消するため、`src/zeroclaw/installer.rs` 内の `ZEROCLAW_CONFIG_TEMPLATE` を最新のスキーマに準拠するよう修正しました。

## 実施内容

### 1. 設定構造の修正
- **セクションのネスト**: `[estop]`, `[resource_limits]`, `[audit]` セクションを、トップレベルから `[security]` セクションの下へと移動しました（例: `[security.audit]`）。
- **名称の変更**: `[resource_limits]` をスキーマ上のフィールド名に合わせて `[security.resources]` に変更しました。

### 2. 項目の整理と警告の抑制
- **不要項目の削除**: スキーマで `#[serde(skip)]` となっている `workspace_dir` をテンプレートから削除しました。
- **ダミー値の設定**: `api_key` が空文字列であることによる警告を防ぐため、`"YOUR_API_KEY_HERE"` というダミー値をデフォルトで設定しました。

### 3. 検証
- **ビルド確認**: `make check-be` を実行し、コンパイルエラーがないことを確認しました。
- **設定ファイルの退避**: 既存の `~/.mycute/zeroclaw/config.toml` を `config.toml.bak` にリネームしました。次回アプリケーション起動時に、修正後のテンプレートから新しい設定ファイルが生成されます。

## 修正後のテンプレート構造（抜粋）

```toml
[security]
enabled = false

[security.audit]
log_path = "logs/audit.log"
# ...

[security.estop]
# ...

[security.resources]
max_memory_mb = 4096
# ...
```

## 次のステップ
1. アプリケーションを起動してください。
2. `~/.mycute/zeroclaw/config.toml` が新しく生成されていることを確認してください。
3. ログに `Unknown config key ignored` 警告が出力されないことを確認してください。
