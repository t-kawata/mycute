# REST API 経由での STT エンジン切り替え実装計画

## 背景と目的
現在、MYCUTE では「言語（ロケール）の切り替え」が REST API (`POST /mycute/lang`) 経由で可能であり、これが WebSocket (SSE) 経由でフロントエンドまで到達し、リアルタイムに UI の言語が切り替わる仕組みになっています。
ユーザーの理解の通り、これと**「全く同じ構造」**を横展開することで、REST API 経由での「音声認識エンジン（STT Engine: OS / OpenAI）の切り替え」を安全かつクリーンに実装できます。

## 設計の原則
1. **既存の仕組みの横展開**: 言語切り替え (`LocaleChanged`) で実績のある WebSockets / Tauri 間のイベント駆動アーキテクチャをそのまま踏襲します。
2. **状態の一元管理**: 設定変更は REST API (RT) で受け付け、CL (メインプロセス) に通知し、CLが自身の状態を更新したのちに Tauri フロントエンド (Vue) へ伝播させます。
3. **副作用（再起動）の適切な配置**: フロントエンド側でイベントを受け取った際に、既存の Tauri コマンド (`switch_stt_engine`) を叩いてバックエンド側（RT / recognizer等）の実働エンジンを再構成します。

## 具体的な実装ステップ（修正すべきファイル群）

### 1. 内部イベント定義の追加 (`src/types.rs`)
- `EventKind` enum に `SttEngineChanged(SttEngine)` を追加します。
- `TauriEvent` enum に `AppSttEngineChanged` を追加し、定数 `EVENT_APP_STT_ENGINE_CHANGED` にマッピングします。
- イベント発火用の Payload 構造体 `AppSttEngineChangedPayload { engine: SttEngine }` を追加します。

### 2. 定数の追加 (`src/constants.rs`)
- `pub const EVENT_APP_STT_ENGINE_CHANGED: &str = "app-stt-engine-changed";` を追加します。
- Web フロントエンド側 (`web/src/consts/generated_constants.ts`) にも同定数を追加します。（※実際は自動生成スクリプトがある場合はそれに従います）

### 3. REST API のリクエスト/レスポンス構造体定義 (`src/mode/rt/rtreq/mycute_req.rs` & `rtres/mycute_res.rs`)
- `SetSttEngineReq { engine: String }` 等の Payload 定義を追加します。
- `SetSttEngineRes { message: String }` 等のレスポンス定義を追加します。

### 4. REST API ハンドラの実装 (`src/mode/rt/rthandler/mycute_handler.rs`)
- `POST /mycute/stt_engine` エンドポイントを新設します。
- リクエストを受け取り、`ConfigManager` 上の `stt_engine` を更新します。
- `EventKind::SttEngineChanged(engine)` を `event_tx` （WebSocket用ブロードキャスト）に投げる処理を実装します。

### 5. CL側でのイベント中継とTauriへのEmit (`src/mode/cl/main_of_cl.rs`)
- `run_ws_message_loop` 内のイベント受信部に `EventKind::SttEngineChanged(engine)` のマッチアームを追加します。
- 受信時、CLプロセス自身のメモリ状態 (`manager`) を更新する必要があれば更新します（※現状STT Engineは設定側に持ち込まれているため ConfigMgr のみに留まる可能性あり）。
- Tauri IPC 経由でフロントエンドへ `TauriEvent::AppSttEngineChanged` を Emit（送信）します。

### 6. フロントエンドでの受信と状態反映 (`web/src/App.vue`)
- `initApp()` 関数内に、`EVENT_APP_STT_ENGINE_CHANGED` を監視するリスナーを追加します。
- 通知を受け取ったら、Piniaストア (`mainStore.setSttEngine()`) を呼び出しフロントエンド側のアプリ設定状態を更新します。
- （注: `mainStore.setSttEngine()` 内部で、バックエンドの実働エンジンを切り替えるTauriコマンド `invoke('switch_stt_engine')` が既存で実装済みのため、これで自動的にOS/OpenAIの認識器がスワップされます）。

## リスクと検証事項 (Verification Plan)
- **再帰呼び出しの防止**: `mainStore.setSttEngine` は内部でTauri Invoke (`switch_stt_engine`) を呼んでいます。API側から呼ばれた際に無限ループに陥らないよう、状態が変わったときのみ Invoke するようにガードを設けるか、フローの整理が必要です。
- **設定ファイルの永続化の有無**: 現状、設定画面からの変更では `stt_engine` はアプリ再起動時にリセットされる仕様（`#[serde(skip)]`）のようですが、これで問題ないか（APIからの変更も揮発的でよいか）の確認が必要です。
