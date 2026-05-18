# 実装計画: 音声入力オーケストレーター基盤 (Voice Input Orchestrator)

> チケット #8 | approved: 2026-05-18 | plan策定: 2026-05-18 | 補強: 2026-05-18

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
| `src/constants.rs` | 修正 | Orchestrator用Tauriイベント定数追加（`EVENT_ORCHESTRATOR_DISPLAY`、`EVENT_ORCHESTRATOR_TEXT`、`EVENT_ORCHESTRATOR_RESPONSE`、`EVENT_ORCHESTRATOR_TASK_COMPLETED`） |
| `src/stt/recognizer.rs` | 修正 | `SpeechRecognizer::create_orchestrator_instance()` ファクトリメソッド追加（既存コンストラクタには触れず、オーケストレーター専用の生成経路を新設） |
| `src/lib.rs` | 修正 | `pub mod orchestrator;` 追加 |
| `src/orchestrator/mod.rs` | **新規** | `OrchestratorError`（thiserror使用）、`OrchestratorInput`、`OrchestratorOutput`、`Orchestrator` トレイト（`#[async_trait]`、`Send + Sync`） |
| `src/orchestrator/mock.rs` | **新規** | `MockOrchestrator`（rally_count, session_id自動生成, エコーバック） |

### フロントエンド (Vue/Quasar)

| ファイル | 種別 | 変更内容 |
|---------|------|---------|
| `web/src/stores/orchestrator-store.ts` | **新規** | オーケストレーターセッション状態管理（録音状態 `isRecording`、会話履歴 `messages[]`、オーケストレーター応答 `currentResponse`、オーバーレイ表示 `isOverlayVisible`） |
| `web/src/components/tools/OrchestratorOverlay.vue` | **新規** | Orchestrator用オーバーレイ(OverlayView.vue踏襲) |
| `web/src/components/effects/ThinkingAnimation.vue` | **新規** | 処理中アニメーション(独立コンポーネント、後で差し替え可能) |
| `web/src/layouts/MainLayout.vue` | 修正 | OrchestratorOverlay の登録 + 排他制御（オーケストレーターオーバーレイ表示中は既存Overlay非表示） |

## Boy Scout 改善（スコープ外の翻訳可能性修正）

- `src/hotkey_mac.rs`: 無名ブロックコメントで区切られた責務を関数抽出する。CGEventFlags のマジックナンバー（`0x00080000`, `0x00040000`）は既存の `K_CG_EVENT_FLAG_MASK_ALTERNATE` / `K_CG_EVENT_FLAG_MASK_CONTROL` 定数を使用済みのため追加対応不要
- `src/hotkey_win.rs`: `handle_event()` のAlt分岐を `process_alt_key_event()` などに関数抽出
- `src/tauri_cmd/system.rs`: `HotkeyAction` match の各アームを handler 関数に切り出し

## 実装手順

### Step 0: 事前調査（OverlayView.vue 構造分析）
- `web/src/components/tools/OverlayView.vue` のテンプレート構造・スタイル定義（背景、フォント `MPLUSRounded1c`、`backdrop-filter: blur`、scale + border-radius アニメーション）を分析し、OrchestratorOverlay に流用するパーツを特定する
- マークダウンレンダリングライブラリ候補を調査し選定する（候補: `markdown-it` / `vue-markdown` / `remark` + `rehype`）。本チケットでは最小限の表示でよいため、**`markdown-it` を第一候補**とする（Vue との組み合わせ実績が豊富）

### Step 1: 型定義と設定
- `HotkeyAction::OrchestratorInput` 追加 (`src/types.rs`)
- `HotkeyConfig` に `orchestrator_input` 追加 (`src/mycute_settings.rs`)
- Orchestrator用Tauriイベント定数追加 (`src/constants.rs`)
- `cargo check` でビルド確認

### Step 2: Orchestrator トレイト + MockOrchestrator
- `src/orchestrator/mod.rs` 作成
  - `OrchestratorError`（thiserror で `EmptyInput`、`PipelineFailed(String)`、`Internal(String)` を定義）
  - `OrchestratorInput { raw_text, session_id }`
  - `OrchestratorOutput { response_text, task_completed }`
  - `Orchestrator` トレイト（`#[async_trait]`、`Send + Sync`、`async fn process()`）
- `src/orchestrator/mock.rs` 作成
  - `MockOrchestrator { session_id, rally_count }`
  - 初回 process(): UUID自動生成、rally_count=1、`[Orchestrator Echo]: {入力}` をエコーバック
  - 2回目以降: rally_count インクリメント、エコーバック
  - rally_count==3: 完了メッセージ追記 + `task_completed: true`
  - rally_count>=3 でさらに発話: rally_count=0 にリセット
  - 空テキスト: `Err(OrchestratorError::EmptyInput)`
- **ユニットテスト（spec Test Plan 準拠）**:
  - MockOrchestrator がエコーバックを返すこと
  - rally_count が正しくインクリメントされること
  - rally_count==3 で完了メッセージ + task_completed==true
  - rally_count>=3 → リセット
  - 空テキスト → `Err(EmptyInput)`
  - session_id が初回呼び出し時に自動生成され、同一セッション内で不変であること
- `cargo test` で全テスト通過確認

### Step 3: ホットキー検出（3ファイル同時）
- `hotkey_mac.rs`: CGEventFlagsChanged イベント内で Control フラグと Option フラグが同時に立ったことを検出 → 150ms クールダウン通過後 `OrchestratorInput` 送信
- `hotkey_win.rs`: rdev + GetAsyncKeyState 経路で `CURRENT_MODIFIERS == (MOD_CTRL | MOD_ALT)` 検出
- `hotkey_win_hook.rs`: WH_KEYBOARD_LL の `check_combo_hotkey` 処理に Ctrl+Alt 検出追加
- **ユニットテスト（spec Test Plan 準拠）**:
  - Ctrl+Alt 同時押し → OrchestratorInput 発火（macOS/Windows 両方）
  - Ctrl のみ → 発火しない
  - Alt のみ → 発火しない
  - 順序逆転（Alt→Ctrl）でも発火すること
  - クールダウン150ms未満 → 発火しない
  - 片方のキーを離したら再度両方押すまで発火しない
- `cargo check` でビルド確認（各OS固有コードは `#[cfg]` ガード）

### Step 4: Tauri コマンド・イベントルーティング
- `SpeechRecognizer::create_orchestrator_instance()` を実装: 既存コンストラクタをラップし、オーケストレーター用の独立した `mpsc::channel` で `SpeechRecognizer` インスタンスを生成するファクトリ
- `start_orchestrator_recording` / `stop_orchestrator_recording` コマンド作成
  - オーケストレーター専用の `SpeechRecognizer` インスタンスを開始/停止
  - 取得したテキストを `MockOrchestrator.process()` に渡す
  - 結果を Tauri イベントでフロントエンドに送信
- `system.rs` の `HotkeyAction::OrchestratorInput` ハンドラ追加
  - 初回押下 → 録音開始 + オーバーレイ表示イベント送信
  - 2回目押下 → 録音停止 + テキスト送信 + オーケストレーター呼び出し
- 排他制御: オーケストレーター録音中は既存の音声入力インプット(Altダブルタップ)を受付禁止にする（マネージャーレベルで制御）
- `cargo check --all-targets` でビルド確認

### Step 5: フロントエンド
1. `web/src/stores/orchestrator-store.ts` 作成
   - 状態: `isRecording`、`isProcessing`、`messages[]`、`isOverlayVisible`
   - アクション: `startSession()`、`stopRecording()`、`addMessage()`、`closeOverlay()`
2. `web/src/components/effects/ThinkingAnimation.vue` 作成
   - 独立コンポーネント（後で差し替え可能な最小実装）
   - 画面中央に浮遊する球のアニメーション（CSSアニメーションで実現）
3. `OrchestratorOverlay.vue` 作成
   - OverlayView.vue から継承する要素:
     - コンテナ構造 + `backdrop-filter: blur` の半透明ガラス背景
     - フォント `MPLUSRounded1c`、scale + border-radius アニメーション
     - 閉じるボタン
   - OrchestratorOverlay 固有の要素:
     - 対話履歴表示（スクロール可能）、`v-for` でメッセージ一覧を描画
     - ユーザー発話: `class="__mycute-orchestrator-user-msg"` 小さめ文字 + 半透明グレー背景
     - AI応答: `class="__mycute-orchestrator-ai-msg"` 背景なし + `v-html` でマークダウンレンダリング
     - 録音中インジケーター + 部分的認識結果のリアルタイム表示
     - 処理中は `ThinkingAnimation` を表示
4. `web/src/layouts/MainLayout.vue` 修正
   - OrchestratorOverlay のインポートと追加
   - 排他制御: `v-show="orchestratorStore.isOverlayVisible"` ＋ 既存 OverlayView と同時表示されない制御
5. `make check-fe` でビルド確認

### Step 6: 結合とテスト
- 全ユニットテスト作成・実行
  - **キーバインド検出テスト**: 上記 Step 3 の観点
  - **オーバーレイコンポーネントテスト**: 起動→表示・録音開始、発話確定→テキスト送信、応答表示→マークダウン、スクロール、閉じる、処理中アニメーション表示/非表示
  - **オーケストレーターユニットテスト**: Step 2 で実施済み、リグレッション確認
- 既存テスト全通過確認
- `make check-be` / `make check-fe` でビルド確認

## 物理的レビュー方法

1. `run-quality-checks.js` を変更ファイルに対して実行
2. 翻訳可能性grep:
   - `fn \w+` で名詞始まり関数がないか確認
   - 1文字変数・汎用名(`data`, `info`, `tmp`)がないか確認
   - ハードコードされた数値リテラル（150msクールダウン等）が名前付き定数になっているか確認
3. `make test` 全テスト通過の確認
4. 手動テスト: Ctrl+Alt → オーバーレイ表示 → 録音 → `[Orchestrator Echo]` エコーバック → 3ラリーで完了通知 → 閉じるボタンでセッション終了
5. 既存音声入力インプット(Altダブルタップ)が従来通り動作することを確認

## リスク

| リスク | 影響 | 対策 |
|-------|------|------|
| Ctrl+Alt が何らかのOS機能と競合 | 起動不能 | 150msクールダウンで緩和。ユーザー設定で変更可能にしておく |
| 既存STT抽象化で回帰 | 既存音声入力が動かなくなる | 既存テスト完全通過を条件に結合。既存コンストラクタには触れず `create_orchestrator_instance()` を新設して影響範囲を限定 |
| 2つのオーバーレイ同時表示 | 画面競合 | 排他制御を orchestrator-store と MainLayout.vue の2層で担保。`v-show` の条件に「他方のオーバーレイが非表示であること」を含める |
| SpeechRecognizer のインスタンス競合（同時録音） | 録音失敗 | オーケストレーター録音中は既存マネージャーの録音開始をブロックする排他フラグを `MycuteManager` に追加。逆方向も同様 |
| `HotkeyConfig` に `orchestrator_input` 追加時の `#[serde(default)]` 付け忘れ | アプリ起動不能（settings.json の deserialize 失敗） | 新フィールドに `#[serde(default = "default_orchestrator_input")]` を必ず付与。`Default` impl にも追記。実装時のコードレビューで確認 |
