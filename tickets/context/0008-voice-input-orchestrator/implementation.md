# 実装サマリー: 音声入力オーケストレーター基盤 (Ticket #8)

## 変更ファイル一覧

| ファイル | 種別 | 内容 |
|----------|------|------|
| `src/orchestrator/mod.rs` | 修正 | `OrchestratorOutput` に `Serialize`/`Deserialize` 導出追加 |
| `src/orchestrator/mock.rs` | 新規 | `MockOrchestrator` 構造体 + 7 テスト |
| `src/tauri_cmd/mod.rs` | 修正 | `orchestrator_cmd` モジュール追加 |
| `src/tauri_cmd/orchestrator_cmd.rs` | 新規 | 3 Tauri コマンド (create/destroy session, process) |
| `src/mode/cl/main_of_cl.rs` | 修正 | TauriState に orchestrator 追加 + コマンド登録 |
| `src/hotkey_mac.rs` | 修正 | Ctrl+Option 同時押し検出 (FLAGS_CHANGED) |
| `src/hotkey_win_hook.rs` | 修正 | Ctrl+Alt 同時押し検出 (WH_KEYBOARD_LL) |
| `web/src/stores/orchestrator-store.ts` | 新規 | Pinia ストア (状態: isVisible, isProcessing, messages) |
| `web/src/components/effects/ThinkingAnimation.vue` | 新規 | CSS ドットアニメーション |
| `web/src/components/tools/OrchestratorOverlay.vue` | 新規 | チャット形式オーバーレイ |
| `web/src/layouts/MainLayout.vue` | 修正 | OrchestratorOverlay 登録 |

## アーキテクチャ

```
Hotkey (Ctrl+Alt/Option)
  → HotkeyMonitor (macOS: CGEventTap / Windows: WH_KEYBOARD_LL)
  → HotkeyAction::OrchestratorInput
  → TauriEvent::OrchestratorDisplay を emit
  → Frontend が受信 → create_orchestrator_session → overlay 表示
  → STT final text → orchestrator_process コマンド
  → MockOrchestrator が処理 → OrchestratorOutput 返却
  → TauriEvent::OrchestratorResponse を emit → overlay に表示
```

## テスト結果

- 12 orchestrator 関連テスト: すべて PASS
- 全 80 テスト: PASS
- Frontend build: 成功
- Backend build: 成功

## 品質チェック

新規コードの指摘は mock.rs のテスト内 `.unwrap()` のみ（テストコードとして許容範囲）。
既存コードの問題（1文字変数、unsafe、デバッグ出力等）は今回のスコープ外。
