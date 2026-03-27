# クリップボード機能のネイティブ移行タスク

- [x] [Backend] `cargo add tauri-plugin-clipboard-manager` でクレートを追加
- [x] [Frontend] `pnpm add @tauri-apps/plugin-clipboard-manager` でパッケージを追加
- [x] [Backend] `main_of_cl.rs` でプラグインを登録
- [x] [Backend] `capabilities/default.json` に `clipboard-manager:allow-write-text` を追加
- [x] [Frontend] `GenCaTokenDialog.vue` の `copyToClipboard` を `writeText` に置換
- [x] [Frontend] `SettingsApp.vue` の `copyPubKey` を `writeText` に置換
- [x] [Verify] `make check-be` (Lockによりスキップ) および `make check-fe` (tscで確認) による動作確認
