# 「CA任命証の登録」機能の実装計画

「CA任命証の登録」機能を `SettingsApp.vue` に実装し、自身のノードが CA（認証局）として動作する状態（`isCaActive`）をフロントエンドとバックエンドで常に同期する仕組みを構築します。

## ユーザーレビューが必要な事項

> [!IMPORTANT]
> `isCaActive` の同期には、既存のオーナーモードや言語設定と同様に、バックエンドからのイベント通知（Tauri Event）を使用します。これにより、API 経由での設定変更が即座に UI に反映される一貫した挙動を実現します。

## 提案される変更点

### 1. バックエンド（Rust）: ステータス同期基盤の拡張

バックエンドの状態変化をフロントエンドに通知するための定数と型を定義します。

#### [MODIFY] [constants.rs](file:///Users/kawata/shyme/mycute/src/constants.rs)
- `EVENT_APP_CA_STATUS_CHANGED` 定数を追加（`"app-ca-status-changed"`）。

#### [MODIFY] [types.rs](file:///Users/kawata/shyme/mycute/src/types.rs)
- `TauriEvent` 列挙型に `AppCaStatusChanged` を追加。
- `AppCaStatusChangedPayload` 構造体を追加。
- `EventKind` 列挙型に `CaStatusChanged(bool)` を追加。

#### [MODIFY] [ca_handler.rs](file:///Users/kawata/shyme/mycute/src/mode/rt/rthandler/ca_handler.rs)
- `register_ca_token_ca`: 登録成功後、`InternalEvent` を使用して `CaStatusChanged(true)` を `event_tx` に送信します。
- `get_ca_status`: 以下のロジックに基づき、実際のステータス（true/false）を返すように修正します。
    1. 設定（DB）から暗号化された `my_cat` を取得。
    2. 存在しない場合は `false`。
    3. 存在する場合、`rt_crypto_key` を用いて復号。
    4. 復号されたトークンを `identities_bl::verify_ca_token` で検証。
    5. トークン内の公開鍵が自身の公開鍵（`my_pub`）と一致し、かつ署名が有効で期限内であれば `true`、それ以外は `false` を返します。

#### [MODIFY] [main_of_cl.rs](file:///Users/kawata/shyme/mycute/src/mode/cl/main_of_cl.rs)
- `run_ws_message_loop` 内のメッセージループで `EventKind::CaStatusChanged` をハンドルし、Tauri の `AppCaStatusChanged` イベントとしてフロントエンドへ Emit する処理を追加します。

---

### 2. フロントエンド: Store と API の実装

#### [MODIFY] [rtres.ts](file:///Users/kawata/shyme/mycute/web/src/models/rtres.ts)
- `CaStatusRes` インターフェースを追加（`{ is_active: boolean }`）。

#### [MODIFY] [rest.ts](file:///Users/kawata/shyme/mycute/web/src/utils/rest.ts)
- `getCaStatus` および `registerCaToken` API 関数を追加します。

#### [MODIFY] [main-store.ts](file:///Users/kawata/shyme/mycute/web/src/stores/main-store.ts)
- state に `isCaActive` と `isRegisterCaTokenDialogOpen` を追加。
- actions に `setIsCaActive`, `setIsRegisterCaTokenDialogOpen`, `fetchCaStatus` を追加。

---

### 3. フロントエンド: UI と 同期処理の実装

#### [MODIFY] [App.vue](file:///Users/kawata/shyme/mycute/web/src/App.vue)
- `initApp` 内で `EVENT_APP_CA_STATUS_CHANGED` のリスナーを登録し、`mainStore.isCaActive` を更新する処理を追加します。
- 起動時に `mainStore.fetchCaStatus()` を呼び出し、初期状態をバックエンドと同期します。
- 新しい `RegisterCaTokenDialog` コンポーネントを template に追加します。

#### [MODIFY] [SettingsApp.vue](file:///Users/kawata/shyme/mycute/web/src/apps/SettingsApp.vue)
- 「CA任命証の発行」と「CA任命証の検証」の間に「CA任命証の登録」の `q-item` を追加します。
- デザイン（アイコン、配色、ラベル構成）は既存の検証機能と一貫性を持たせます。

#### [NEW] [RegisterCaTokenDialog.vue](file:///Users/kawata/shyme/mycute/web/src/components/dialogs/RegisterCaTokenDialog.vue)
- `VerifyCaTokenDialog.vue` をベースに作成します。
- トークンの入力、検証結果の表示、および最終的な「登録実行」ボタンを設けます。
- 登録成功時には `isCaActive` がバックエンドからのイベントにより自動的に true に更新されるため、UI も連動して変化します。

#### [MODIFY] [I18n (ja-JP/index.ts, en-US/index.ts)](file:///Users/kawata/shyme/mycute/web/src/i18n/ja-JP/index.ts)
- 新しい機能（登録ボタン、ダイアログタイトル、成功・失敗メッセージ等）に必要な翻訳キーを追加します。

## オープンクエスチョン

- 特になし。既存のデザインと同期ロジックを忠実に再現します。

## 検証計画

### 自動テスト
- `make check-all` によるビルド確認。

### 手動検証
1. フロントエンドから CA トークンを入力し、登録を実行。
2. バックエンドで `settings.my_cat` が保存されることを確認。
3. イベントがフロントエンドに到達し、`isCaActive` が true になることを確認。
4. プロセス再起動後も属性が維持（同期）されていることを確認。