//
use crate::mode::rt::rtbl::mycute_bl;
use crate::mode::rt::rterr::rterr::ERR_UNEXPECTED;
use crate::mode::rt::rtreq::mycute_req::{SetLangReq, SetLlmsReq, SetSttEngineReq};
use crate::mode::rt::rtres::errs_res::ApiError;
use crate::mode::rt::rtres::mycute_res::{
    GetMycuteLlmsRes, MyCuteHomeDirRes, MyCuteVersionRes, SetLangRes, SetLlmsRes, SetSttEngineRes,
};
use crate::stt_config::{ConfigManager, LlmEndpoint};
use crate::types::{
    EventKind, InternalEvent, WsClientMessage, WsClientRole, WsServerMessage, WsStatusRes,
};
use crate::utils::time;
use axum::http::StatusCode;
use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::Response,
    Extension, Json,
};
use dashmap::DashMap;
use futures_util::StreamExt;
use std::{sync::Arc, time::Duration};
use tokio::sync::broadcast::{self, error::RecvError};

const TAG: &str = "v1 MYCUTE";

// ============================================================
// Version
// ============================================================
const VERSION_DESC: &str = r#"
### ⚫︎ 概要
- MYCUTE の現在のバージョンを取得します。

### ⚫︎ 権限
- パブリック（認証不要）。誰でもアクセス可能です。

### ⚫︎ Response
| KEY | TYPE | DESCRIPTION |
| --- | --- | --- |
| `version` | string | MYCUTE のバージョン (例: v0.1.0) |
"#;
#[utoipa::path(
    tag = TAG,
    get,
    path = "/mycute/version",
    summary = "MYCUTE のバージョンを取得する。",
    description = VERSION_DESC,
    responses(
        (status = 200, description = "Success", body = MyCuteVersionRes),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn get_mycute_version() -> Result<Json<MyCuteVersionRes>, ApiError> {
    log::debug!("<MyCute> Fetching version.");
    let res = mycute_bl::get_version().await;
    Ok(Json(res))
}

// ============================================================
// Home Directory
// ============================================================
const HOME_DIR_DESC: &str = r#"
### ⚫︎ 概要
- MYCUTE が使用しているホームディレクトリの絶対パスを取得します。

### ⚫︎ 権限
- パブリック（認証不要）。誰でもアクセス可能です。

### ⚫︎ Response
| KEY | TYPE | DESCRIPTION |
| --- | --- | --- |
| `home_dir` | string | ホームディレクトリの絶対パス |
"#;
#[utoipa::path(
    tag = TAG,
    get,
    path = "/mycute/home_dir",
    summary = "MYCUTE のホームディレクトリを取得する。",
    description = HOME_DIR_DESC,
    responses(
        (status = 200, description = "Success", body = MyCuteHomeDirRes),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn get_mycute_home_dir() -> Result<Json<MyCuteHomeDirRes>, ApiError> {
    log::debug!("<MyCute> Fetching home directory.");
    let res = mycute_bl::get_home_dir().await;
    Ok(Json(res))
}

// ============================================================
// Language Setting
// ============================================================
const SET_LANG_DESC: &str = r#"
### ⚫︎ 概要
- MYCUTE の現在の言語 (ロケール) を設定し、内部イベントをブロードキャストします。

### ⚫︎ 権限
- パブリック（認証不要）。誰でもアクセス可能です。

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `locale` | string | required | 言語コード (例: "en", "ja") |

### ⚫︎ Response
| KEY | TYPE | DESCRIPTION |
| --- | --- | --- |
| `message` | string | 処理結果のメッセージ |
"#;
#[utoipa::path(
    tag = TAG,
    post,
    path = "/mycute/lang",
    summary = "MYCUTE の言語設定を更新する。",
    description = SET_LANG_DESC,
    request_body(content = SetLangReq, description = "設定する言語コード", content_type = "application/json"),
    responses(
        (status = 200, description = "Success", body = SetLangRes),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn set_mycute_lang(
    Extension(config_manager): Extension<Arc<ConfigManager>>,
    Extension(event_tx): Extension<Arc<broadcast::Sender<InternalEvent>>>,
    Json(payload): Json<SetLangReq>,
) -> Result<Json<SetLangRes>, ApiError> {
    log::debug!("<MyCute> Setting language to: {:?}", payload.locale);

    let locale = payload.locale;

    // 1. ConfigManager のオンメモリのロケールを更新する (ファイルへは保存しない)
    {
        let mut settings = config_manager.settings.write();
        settings.locale = locale.clone();
    }

    // 2. 内部イベントをブロードキャスト (WebSocket経由でフロントに中継される)
    // シーケンス番号として UNIX タイムスタンプ(ms) を利用
    let seq = time::now_ts_ms();

    let event = InternalEvent {
        seq,
        kind: EventKind::LocaleChanged(locale),
    };

    let _ = event_tx.send(event); // リスナーがいなくてもエラーは無視

    Ok(Json(SetLangRes {
        message: "Language updated successfully".to_string(),
    }))
}

// ============================================================
// STT Engine Setting
// ============================================================
const SET_STT_ENGINE_DESC: &str = r#"
### ⚫︎ 概要
- MYCUTE の現在の STT エンジンを設定し、内部イベントをブロードキャストします。

### ⚫︎ 権限
- パブリック（認証不要）。誰でもアクセス可能です。

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `engine` | string | required | STTエンジン (例: "os", "openai") |

### ⚫︎ Response
| KEY | TYPE | DESCRIPTION |
| --- | --- | --- |
| `message` | string | 処理結果のメッセージ |
"#;
#[utoipa::path(
    tag = TAG,
    post,
    path = "/mycute/stt_engine",
    summary = "MYCUTE の STT エンジン設定を更新する。",
    description = SET_STT_ENGINE_DESC,
    request_body(content = SetSttEngineReq, description = "設定する STT エンジン", content_type = "application/json"),
    responses(
        (status = 200, description = "Success", body = SetSttEngineRes),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn set_mycute_stt_engine(
    Extension(config_manager): Extension<Arc<ConfigManager>>,
    Extension(event_tx): Extension<Arc<broadcast::Sender<InternalEvent>>>,
    Json(payload): Json<SetSttEngineReq>,
) -> Result<Json<SetSttEngineRes>, ApiError> {
    log::debug!("<MyCute> Setting STT engine to: {:?}", payload.engine);

    let engine = payload.engine;

    // 1. ConfigManager のオンメモリの STT エンジンを更新する (ファイルへは保存しない)
    {
        let mut settings = config_manager.settings.write();
        settings.stt_engine = engine.clone();
    }

    // 2. 内部イベントをブロードキャスト (WebSocket経由でフロントに中継される)
    // シーケンス番号として UNIX タイムスタンプ(ms) を利用
    let seq = time::now_ts_ms();

    let event = InternalEvent {
        seq,
        kind: EventKind::SttEngineChanged(engine),
    };

    let _ = event_tx.send(event); // リスナーがいなくてもエラーは無視

    Ok(Json(SetSttEngineRes {
        message: "STT engine updated successfully".to_string(),
    }))
}

// ============================================================
// Ws Events
// ============================================================
const WS_DESC: &str = r#"
### 総則
- WebSocket による各種イベントの購読と双方向のハンドシェイクを行います。
- 接続時にサーバーからチャレンジが送信され、クライアントが応答した時点で接続完了とみなされます。
"#;

#[utoipa::path(
    tag = TAG,
    get,
    path = "/mycute/events/ws",
    summary = "マイキュートイベントを WebSocket で購読する。",
    description = WS_DESC,
    responses(
        (status = 200, description = "WebSocket Upgraded successfully")
    )
)]
pub async fn subscribe_ws_events(
    ws: WebSocketUpgrade,
    Extension(event_tx): Extension<Arc<broadcast::Sender<InternalEvent>>>,
    Extension(ws_clients): Extension<Arc<DashMap<String, WsClientRole>>>,
) -> Response {
    ws.on_upgrade(move |socket| handle_ws_socket(socket, event_tx, ws_clients))
}

async fn handle_ws_socket(
    mut socket: WebSocket,
    event_tx: Arc<broadcast::Sender<InternalEvent>>,
    ws_clients: Arc<DashMap<String, WsClientRole>>,
) {
    log::info!("<Events> WS Client connected. Starting handshake...");

    // 1. ハンドシェイク要求 (チャレンジ) の送信
    let challenge = uuid::Uuid::new_v4().to_string();
    if !send_handshake_challenge(&mut socket, &challenge).await {
        return;
    }

    // 2. ハンドシェイク応答の待機 (タイムアウト付き)
    let client_id =
        match wait_for_handshake_response(&mut socket, &challenge, ws_clients.clone()).await {
            Some(id) => id,
            None => return,
        };

    // 3. ブロードキャストの受信と WebSocket への転送ループ
    broadcast_to_ws_loop(&mut socket, event_tx, ws_clients, &client_id).await;
}

// 1. ハンドシェイク要求 (チャレンジ) の送信
async fn send_handshake_challenge(socket: &mut WebSocket, challenge: &str) -> bool {
    let handshake_req = WsServerMessage::HandshakeRequest {
        challenge: challenge.to_string(),
    };
    if let Ok(msg_str) = serde_json::to_string(&handshake_req) {
        if let Err(e) = socket.send(Message::Text(msg_str.into())).await {
            log::error!("<Events> Failed to send WS handshake request: {}", e);
            return false;
        }
    }
    true
}

// 2. ハンドシェイク応答の待機 (タイムアウト付き)
async fn wait_for_handshake_response(
    socket: &mut WebSocket,
    expected_challenge: &str,
    ws_clients: Arc<DashMap<String, WsClientRole>>,
) -> Option<String> {
    match tokio::time::timeout(Duration::from_secs(5), socket.next()).await {
        Ok(Some(Ok(msg))) => match msg {
            Message::Text(text) => {
                if let Ok(WsClientMessage::HandshakeResponse {
                    challenge: resp_challenge,
                    client_id: resp_client_id,
                    client_role: resp_client_role,
                }) = serde_json::from_str(&text)
                {
                    if resp_challenge == expected_challenge {
                        log::info!(
                            "<Events> WS Handshake successful. client_id: {}, role: {:?}",
                            resp_client_id,
                            resp_client_role
                        );
                        // ハンドシェイク成功時のみマップに登録
                        ws_clients.insert(resp_client_id.clone(), resp_client_role);
                        return Some(resp_client_id);
                    } else {
                        log::warn!("<Events> WS Handshake failed: Invalid challenge.");
                    }
                } else {
                    log::warn!("<Events> WS Handshake failed: Unparseable message.");
                }
            }
            _ => log::warn!("<Events> WS Handshake failed: Non-text message received."),
        },
        Ok(_) => log::warn!("<Events> WS Handshake failed: Socket closed or error."),
        Err(_) => log::warn!("<Events> WS Handshake timeout."),
    }
    None
}

// 3. ブロードキャストの受信と WebSocket への転送ループ
async fn broadcast_to_ws_loop(
    socket: &mut WebSocket,
    event_tx: Arc<broadcast::Sender<InternalEvent>>,
    ws_clients: Arc<DashMap<String, WsClientRole>>,
    client_id: &str,
) {
    let mut rx = event_tx.subscribe();
    loop {
        tokio::select! {
            result = rx.recv() => {
                match result {
                    Ok(event) => {
                        let ws_msg = WsServerMessage::Event(event);
                        if let Ok(msg_str) = serde_json::to_string(&ws_msg) {
                            if socket.send(Message::Text(msg_str.into())).await.is_err() {
                                log::warn!("<Events> WS client disconnected during send. client_id: {}", client_id);
                                break;
                            }
                        }
                    }
                    Err(RecvError::Lagged(count)) => {
                        log::warn!("<Events> WS Client lagged by {} messages. client_id: {}", count, client_id);
                    }
                    Err(RecvError::Closed) => {
                        break;
                    }
                }
            }
            msg = socket.next() => {
                // クライアントからの予期せぬ切断やレスポンスをハンドリング
                if let Some(Ok(Message::Close(_))) | None = msg {
                    log::info!("<Events> WS Client disconnected. client_id: {}", client_id);
                    break;
                }
            }
        }
    }

    // 接続ループを抜けたら確実にマップから削除
    ws_clients.remove(client_id);
    log::info!(
        "<Events> WS client removed from map. client_id: {}. Active: {}",
        client_id,
        ws_clients.len()
    );
}

// ============================================================
// Ws Status
// ============================================================
const WS_STATUS_DESC: &str = r#"
### 総則
- 現在の WebSocket 接続確立状態（ハンドシェイクが完了しているか）を返します。
"#;

#[utoipa::path(
    tag = TAG,
    get,
    path = "/mycute/events/ws/status",
    summary = "WebSocket 接続状態を確認する。",
    description = WS_STATUS_DESC,
    responses(
        (status = 200, description = "Current WS Status", body = WsStatusRes)
    )
)]
pub async fn get_ws_status(
    Extension(ws_clients): Extension<Arc<DashMap<String, WsClientRole>>>,
) -> Result<Json<WsStatusRes>, ApiError> {
    let active_clients = ws_clients.len();
    let is_cl_connected = ws_clients
        .iter()
        .any(|entry| *entry.value() == WsClientRole::CL);

    Ok(Json(WsStatusRes {
        is_cl_connected,
        active_clients,
    }))
}

// ============================================================
// Mycute Settings (GET)
// ============================================================
const GET_MYCUTE_LLMS_DESC: &str = r#"
### ⚫︎ 概要
- MYCUTE の現在の LLM エンドポイント設定一覧を取得します。
- フロントエンドはアプリ起動時にこのエンドポイントを呼び出し、バックエンドの設定を Source of Truth として初期化します。

### ⚫︎ 権限
- パブリック（認証不要）。誰でもアクセス可能です。

### ⚫︎ Response
| KEY | TYPE | DESCRIPTION |
| --- | --- | --- |
| `llms` | array | 現在の LLM エンドポイント設定一覧 |
"#;
#[utoipa::path(
    tag = TAG,
    get,
    path = "/mycute/llms/get",
    summary = "MYCUTE の現在の LLM 設定を取得する。",
    description = GET_MYCUTE_LLMS_DESC,
    responses(
        (status = 200, description = "Success", body = GetMycuteLlmsRes)
    )
)]
pub async fn get_mycute_llms(
    Extension(config_manager): Extension<Arc<ConfigManager>>,
) -> Result<Json<GetMycuteLlmsRes>, ApiError> {
    let llms = {
        let settings = config_manager.settings.read();
        settings.llms.clone()
    };
    log::debug!("<MyCute> get_mycute_llms: {} llms found", llms.len());
    Ok(Json(GetMycuteLlmsRes { llms }))
}

// ============================================================
// LLM Settings (POST)
// ============================================================
const SET_LLMS_DESC: &str = r#"
### ⚫︎ 概要
- MYCUTE の LLM エンドポイント設定を更新し、settings.json へ永続保存した後、内部イベントをブロードキャストします。
- フロントエンドは入力後 500ms のデバウンス後にこのエンドポイントを呼び出します。

### ⚫︎ 権限
- パブリック（認証不要）。誰でもアクセス可能です。

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `llms` | array | required | LlmEndpointReq の配列（0件で全クリア可能） |

### ⚫︎ Response
| KEY | TYPE | DESCRIPTION |
| --- | --- | --- |
| `message` | string | 処理結果のメッセージ |
"#;
#[utoipa::path(
    tag = TAG,
    post,
    path = "/mycute/llms/set",
    summary = "MYCUTE の LLM 設定を更新する。",
    description = SET_LLMS_DESC,
    request_body(content = SetLlmsReq, description = "設定する LLM エンドポイント一覧", content_type = "application/json"),
    responses(
        (status = 200, description = "Success", body = SetLlmsRes),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn set_mycute_llms(
    Extension(config_manager): Extension<Arc<ConfigManager>>,
    Extension(event_tx): Extension<Arc<broadcast::Sender<InternalEvent>>>,
    Json(payload): Json<SetLlmsReq>,
) -> Result<Json<SetLlmsRes>, ApiError> {
    log::debug!("<MyCute> set_mycute_llms: {} endpoints", payload.llms.len());

    // リクエストの LlmEndpointReq を LlmEndpoint に変換（From実装を利用）
    let llms: Vec<LlmEndpoint> = payload.llms.into_iter().map(LlmEndpoint::from).collect();

    // 1. ConfigManager のオンメモリ設定を更新する
    {
        let mut settings = config_manager.settings.write();
        settings.llms = llms.clone();
    }

    // 2. settings.json へ永続保存する（言語・エンジンと異なりファイル保存が必須）
    config_manager.save_db().await.map_err(|e| {
        log::error!("<MyCute> Failed to save settings to DB: {}", e);
        ApiError::new_system(StatusCode::INTERNAL_SERVER_ERROR, ERR_UNEXPECTED, e.to_string())
    })?;

    // 3. 内部イベントをブロードキャスト（WebSocket経由でフロントに中継される）
    let seq = time::now_ts_ms();
    let event = InternalEvent {
        seq,
        kind: EventKind::LlmsChanged(llms),
    };
    let _ = event_tx.send(event); // リスナーがいなくてもエラーは無視

    log::debug!("<MyCute> LLM settings saved and event broadcasted.");
    Ok(Json(SetLlmsRes {
        message: "LLM settings updated successfully".to_string(),
    }))
}
