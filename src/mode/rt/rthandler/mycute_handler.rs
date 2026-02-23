use crate::mode::rt::rtbl::mycute_bl;
use crate::mode::rt::rtreq::mycute_req::SetLangReq;
use crate::mode::rt::rtres::errs_res::ApiError;
use crate::mode::rt::rtres::mycute_res::{MyCuteHomeDirRes, MyCuteVersionRes, SetLangRes};
use crate::stt_config::ConfigManager;
use crate::types::{EventKind, InternalEvent, LocaleCode};
use crate::utils::time;

use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::Response,
    Extension, Json,
};
use dashmap::DashMap;
use futures_util::StreamExt;
use std::{sync::Arc, time::Duration};
use tokio::sync::broadcast;

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
- MYCUTE の現在の言語 (ロケール) を設定し、SSE イベントをブロードキャストします。

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
    log::debug!("<MyCute> Setting language to: {}", payload.locale);

    let locale = LocaleCode::from_str(&payload.locale);

    // 1. ConfigManager のオンメモリのロケールを更新する (ファイルへは保存しない)
    {
        let mut settings = config_manager.settings.write();
        settings.locale = locale.clone();
    }

    // 2. SSE イベントを送信
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
pub async fn ws_events_handler(
    ws: WebSocketUpgrade,
    Extension(event_tx): Extension<Arc<broadcast::Sender<InternalEvent>>>,
    Extension(ws_clients): Extension<Arc<DashMap<String, ()>>>,
) -> Response {
    ws.on_upgrade(move |socket| handle_ws_socket(socket, event_tx, ws_clients))
}

async fn handle_ws_socket(
    mut socket: WebSocket,
    event_tx: Arc<broadcast::Sender<InternalEvent>>,
    ws_clients: Arc<DashMap<String, ()>>,
) {
    log::info!("<Events> WS Client connected. Starting handshake...");

    // 1. ハンドシェイク要求 (チャレンジ) の送信
    let challenge = uuid::Uuid::new_v4().to_string();
    let handshake_req = crate::types::WsServerMessage::HandshakeRequest {
        challenge: challenge.clone(),
    };
    if let Ok(msg_str) = serde_json::to_string(&handshake_req) {
        if let Err(e) = socket.send(Message::Text(msg_str.into())).await {
            log::error!("<Events> Failed to send WS handshake request: {}", e);
            return;
        }
    }

    // 2. ハンドシェイク応答の待機 (タイムアウト付き)
    let client_id: String;
    match tokio::time::timeout(Duration::from_secs(5), socket.next()).await {
        Ok(Some(Ok(msg))) => match msg {
            Message::Text(text) => {
                if let Ok(crate::types::WsClientMessage::HandshakeResponse {
                    challenge: resp_chal,
                    client_id: cid,
                }) = serde_json::from_str(&text)
                {
                    if resp_chal == challenge {
                        log::info!("<Events> WS Handshake successful. client_id: {}", cid);
                        client_id = cid;
                        // ハンドシェイク成功時のみマップに登録
                        ws_clients.insert(client_id.clone(), ());
                    } else {
                        log::warn!("<Events> WS Handshake failed: Invalid challenge.");
                        return;
                    }
                } else {
                    log::warn!("<Events> WS Handshake failed: Unparseable message.");
                    return;
                }
            }
            _ => {
                log::warn!("<Events> WS Handshake failed: Non-text message received.");
                return;
            }
        },
        Ok(_) => {
            log::warn!("<Events> WS Handshake failed: Socket closed or error.");
            return;
        }
        Err(_) => {
            log::warn!("<Events> WS Handshake timeout.");
            return;
        }
    }

    // 3. ブロードキャストの受信と WebSocket への転送ループ
    //    15秒間隔のハートビート送信を統合
    let mut rx = event_tx.subscribe();
    let mut heartbeat_interval = tokio::time::interval(crate::constants::SSE_HEARTBEAT_INTERVAL);
    loop {
        tokio::select! {
            result = rx.recv() => {
                match result {
                    Ok(event) => {
                        let ws_msg = crate::types::WsServerMessage::Event(event);
                        if let Ok(msg_str) = serde_json::to_string(&ws_msg) {
                            if socket.send(Message::Text(msg_str.into())).await.is_err() {
                                log::warn!("<Events> WS client disconnected during send. client_id: {}", client_id);
                                break;
                            }
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(count)) => {
                        log::warn!("<Events> WS Client lagged by {} messages. client_id: {}", count, client_id);
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        break;
                    }
                }
            }
            _ = heartbeat_interval.tick() => {
                // サーバーからのハートビート送信（クライアント側のタイムアウト検知に利用される）
                let hb_event = InternalEvent {
                    seq: 0,
                    kind: EventKind::Heartbeat,
                };
                let ws_msg = crate::types::WsServerMessage::Event(hb_event);
                if let Ok(msg_str) = serde_json::to_string(&ws_msg) {
                    if socket.send(Message::Text(msg_str.into())).await.is_err() {
                        log::warn!("<Events> WS client disconnected during heartbeat. client_id: {}", client_id);
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
    ws_clients.remove(&client_id);
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
        (status = 200, description = "Current WS Status", body = crate::types::WsStatusRes)
    )
)]
pub async fn get_ws_status(
    Extension(ws_clients): Extension<Arc<DashMap<String, ()>>>,
) -> Result<Json<crate::types::WsStatusRes>, ApiError> {
    let active_clients = ws_clients.len();
    let is_connected = active_clients > 0;

    Ok(Json(crate::types::WsStatusRes {
        is_connected,
        active_clients,
    }))
}
