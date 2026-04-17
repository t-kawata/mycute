# タスクリスト: ZeroClaw 設定テンプレートの修正

- [ ] `src/zeroclaw/installer.rs` の修正
    - [ ] `workspace_dir` の削除
    - [ ] `api_key` へのダミー値設定 (`YOUR_API_KEY_HERE`)
    - [ ] `[security]` セクションへの各種設定（estop, resources, audit）の集約
    - [ ] `resource_limits` から `resources` への名称変更
- [ ] ビルド確認 (`make check-be`)
- [ ] 設定ファイル生成の再試行とログ確認
