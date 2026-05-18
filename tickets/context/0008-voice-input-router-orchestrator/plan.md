# 実装計画: 音声入力ルーティング基盤 (Voice Input Orchestrator)

> チケット #8 | approved: 2026-05-18 | plan策定: 2026-05-18

## 要件の再確認

1. **既存の音声入力インプット(Altダブルタップ)のフロントエンドは一切変更しない**
2. **バックエンドSTT機構のみ共通化・抽象化する**
3. **Ctrl+Alt(Control+Option)の順序非依存検出 + 150msクールダウン**
4. **新規 Vue オーバーレイ(OrchestratorOverlay)を既存 OverlayView.vue のデザインを踏襲して作成**
5. **Orchestrator トレイト + MockOrchestrator(3ラリーで完了通知)**
6. **Tap-to-Talk: 押下で録音開始、再押下で送信**
7. **閉じるボタンのみでセッション終了 (自動クローズ/タイムアウトなし)**
8. **すべての関数は `Result` を返しエラーを握りつぶさない**

## 変更ファイル一覧

### バックエンド (Rust)

| ファイル | 種別 | 変更内容 |
|---------|------|---------|
| `src/types.rs` | 修正 | `HotkeyAction::OrchestratorInput` 追加 |
| `src/mycute_settings.rs` | 修正 | `HotkeyConfig` に `orchestrator_input` フィールド追加 |
| `src/hotkey_mac.rs` | 修正 | `FLAGS_CHANGED` 検出部にControl+Option状態遷移チェック追加 |
| `src/hotkey_win.rs` | 修正 | rdev + GetAsyncKeyState にCtrl+Alt検出追加 |
| `src/hotkey_win_hook.rs` | 修正 | `check_combo_hotkey` にCtrl+Alt検出追加 |
| `src/tauri_cmd/system.rs` | 修正 | `HotkeyAction::OrchestratorInput` ハンドラ追加 |
| `src/tauri_cmd/recording.rs` | 修正 | `start_orchestrator_recording` / `stop_orchestrator_recording` コマンド追加 |
| `src/constants.rs` | 修正 | Orchestrator用Tauriイベント定数追加 |
| `src/stt/recognizer.rs` | 修正 | STT抽象インターフェース抽出(最小限の共通化) |
| `src/orchestrator/mod.rs` | **新規** | `Orchestrator` トレイト、`OrchestratorInput/Output`、`OrchestratorError` |
| `src/orchestrator/mock.rs` | **新規** | `MockOrchestrator` (rally_count, session_id自動生成, エコーバック) |

### フロントエンド (Vue/Quasar)

| ファイル | 種別 | 変更内容 |
|---------|------|---------|
| `web/src/components/tools/OrchestratorOverlay.vue` | **新規** | Orchestrator用オーバーレイ(OverlayView.vue踏襲) |
| `web/src/components/effects/ThinkingAnimation.vue` | **新規** | 処理中アニメーション(独立コンポーネント、後で差し替え可能) |
| `web/src/layouts/MainLayout.vue` | 修正 | OrchestratorOverlay の登録 |

## Boy Scout 改善（スコープ外の翻訳可能性修正）

- `src/hotkey_mac.rs`: 無名ブロックコメントで区切られた責務を関数抽出する
- `src/hotkey_win.rs`: `handle_event()` のAlt分岐を `process_alt_key_event()` などに関数抽出
- `src/tauri_cmd/system.rs`: `HotkeyAction` match の各アームを handler 関数に切り出し

## 実装手順

### Step 1: 型定義と設定
- `HotkeyAction::OrchestratorInput` 追加 (`src/types.rs`)
- `HotkeyConfig` に `orchestrator_input` 追加 (`src/mycute_settings.rs`)
- Orchestrator用Tauriイベント定数追加 (`src/constants.rs`)

### Step 2: Orchestrator トレイト + MockOrchestrator
- `src/orchestrator/mod.rs` 作成（`OrchestratorInput`、`OrchestratorOutput`、`OrchestratorError`、`Orchestrator` トレイト）
- `src/orchestrator/mock.rs` 作成（`MockOrchestrator`: session_id自動生成、rally_count管理、[Orchestrator Echo]エコーバック、rally_count==3で完了通知、空テキストでEmptyInputエラー）

### Step 3: ホットキー検出（3ファイル同時）
- `hotkey_mac.rs`: CGEventFlagsChanged 内でControl+Option両方立ったら → `OrchestratorInput` 送信
- `hotkey_win.rs` / `hotkey_win_hook.rs`: `CURRENT_MODIFIERS == (MOD_CTRL | MOD_ALT)` 検出 → `OrchestratorInput` 送信

### Step 4: Tauri コマンド・イベントルーティング
- `start_orchestrator_recording` / `stop_orchestrator_recording` コマンド作成
  - 既存の `SpeechRecognizer` を開始/停止し、テキストを取得
  - 取得したテキストを `MockOrchestrator.process()` に渡す
  - 結果を Tauri イベントでフロントエンドに送信
- `system.rs` の `HotkeyAction::OrchestratorInput` ハンドラ追加

### Step 5: Orchestrator オーバーレイ
- `OrchestratorOverlay.vue` 作成: OverlayView.vue の構造・スタイルを継承
  - 対話履歴表示（スクロール可能）
  - ユーザー発話: 小さめ文字 + 半透明グレー背景
  - AI応答: 背景なし + マークダウン表示
  - 閉じるボタン
  - STTリアルタイム文字起こし表示
- `ThinkingAnimation.vue` 作成: シンプルな浮遊球アニメーション（独立コンポーネント）
- マークダウンレンダリングOSSライブラリ選定・組み込み

### Step 6: 結合とテスト
- 全ユニットテスト作成・実行
- 既存テスト全通過確認
- `make check-be` / `make check-fe` でビルド確認

## 物理的レビュー方法

1. `run-quality-checks.js` を変更ファイルに対して実行
2. 翻訳可能性grep:
   - `fn \w+` で名詞始まり関数がないか確認
   - 1文字変数・汎用名(`data`, `info`, `tmp`)がないか確認
   - ハードコードされた数値リテラルがないか確認
3. `make test` 全テスト通過の確認
4. 手動テスト: Ctrl+Alt → オーバーレイ表示 → 録音 → エコーバック → 3ラリーで完了通知 → 閉じる

## リスク

| リスク | 影響 | 対策 |
|-------|------|------|
| Ctrl+Alt が何らかのOS機能と競合 | 起動不能 | 150msクールダウンで緩和。ユーザー設定で変更可能にしておく |
| 既存STT抽象化で回帰 | 既存音声入力が動かなくなる | 既存テスト完全通過を条件に結合。既存モジュールには触れずラッパー的に抽象化 |
| 2つのオーバーレイ同時表示 | 画面競合 | 排他制御: OrchestratorOverlay表示中は既存Overlay非表示 |
