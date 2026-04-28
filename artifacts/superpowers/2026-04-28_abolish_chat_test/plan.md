# LlmApp.vue チャットテストパネルの廃止実装計画

`web/src/apps/LlmApp.vue` に実装されている「チャットテスト」パネルを廃止し、関連するすべてのコード、状態、翻訳、アイコンをクリーンに削除します。

## ユーザーレビュー必須

> [!IMPORTANT]
> - チャットテスト機能は完全に削除されます。今後、LMGW (Bifrost) の動作確認はバックエンドのログや直接の API 呼び出し（curl等）で行う必要があります。
> - LLM 設定（APIキー管理）機能はそのまま残ります。

## 変更内容

### フロントエンド

---

#### [MODIFY] [LlmApp.vue](file:///Users/kawata/shyme/mycute/web/src/apps/LlmApp.vue)
- `<template>`:
  - `q-tab-panel` の `name="CHAT"` ブロックを削除。
  - ボトムタブバー内の `CHAT` 用ボタンを削除。
- `<script>`:
  - `currentTab` の初期値を `'CHAT'` から `'SETTINGS'` に変更。
  - チャット関連の変数 (`chatInput`, `chatScrollEl`) を削除。
  - チャット関連のメソッド (`onProviderChange`, `scrollToBottom`, `onSendChat`) を削除。
  - チャットメッセージを監視する `watch` を削除。
  - 未使用のインポート (`BotIcon`, `BrainAI1Icon`, `ChatMessage`) を削除。
- `<style>`:
  - チャットパネル専用のスタイル（`/* ===== チャットパネル ===== */` セクション）をすべて削除。

#### [MODIFY] [llm-store.ts](file:///Users/kawata/shyme/mycute/web/src/stores/llm-store.ts)
- `ChatMessage` インターフェースの定義を削除。
- チャット関連の State (`chatMessages`, `isChatStreaming`, `selectedProviderName`, `selectedModel`) を削除。
- チャットパネル専用の Computed (`configuredProviders`, `availableModels`) を削除。
- チャット関連の Action (`clearChat`) を削除。

#### [MODIFY] [ja-JP/index.ts](file:///Users/kawata/shyme/mycute/web/src/i18n/ja-JP/index.ts)
- `app.llm` 配下のチャット関連キー (`chatTab`, `chatHint`, `provider`, `model`, `messagePlaceholder`, `selectProviderHint`, `clearChat`) を削除。
- ※ `provider` が設定パネルで使われていないことを再確認済み。

#### [MODIFY] [en-US/index.ts](file:///Users/kawata/shyme/mycute/web/src/i18n/en-US/index.ts)
- `app.llm` 配下のチャット関連キーを日本語版と同期して削除。

#### [MODIFY] [MainLayout.vue](file:///Users/kawata/shyme/mycute/web/src/layouts/MainLayout.vue)
- コメントアウトされたコード内で参照されている `BotIcon` のインポートを削除（またはコメントアウト）。

## 検証計画

### 自動テスト
- `make check-fe` を実行し、フロントエンドのビルドエラーや型エラーが発生しないことを確認します。

### 手動確認
- アプリケーションを起動し、「LLM」アプリを開いた際に最初から「設定」タブが表示され、チャットタブが存在しないことを確認します。
- 設定（APIキーの追加・保存・同期）が従来通り正常に動作することを確認します。