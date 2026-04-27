//
use crate::mode::rt::rtbl::mycute_bl;
use crate::mode::rt::rtbl::license_bl;
// ERR_UNEXPECTED \u306f set_mycute_llms \u306e\u5ec3\u6b62\u306b\u4f34\u3044\u4e0d\u8981\u306b\u306a\u3063\u305f
use crate::mode::rt::rtreq::mycute_req::{
    RegisterLicenseReq, SetLangReq, SetSttEngineReq, UnregisterLicenseReq,
    VerifyCaTokenReq, VerifyLicenseReq,
};
use crate::mode::rt::rtres::errs_res::ApiError;
use crate::mode::rt::rtres::mycute_res::{
    ListLicensesRes, MyCuteHomeDirRes, MyCuteVersionRes, RegisterLicenseRes,
    SetLangRes, SetSttEngineRes, UnregisterLicenseRes, VerifyCaTokenRes,
    VerifyLicenseRes,
};
use crate::mycute_settings::ConfigManager;
use crate::types::{
    EventKind, InternalEvent, WsClientMessage, WsClientRole, WsServerMessage, WsStatusRes,
};
use crate::utils::time;
use crate::utils::jwt::{JwtRole, JwtUsr};
// StatusCode は set_mycute_llms の廃止に伴い不要になった
use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::Response,
    Extension, Json,
};
use dashmap::DashMap;
use futures_util::StreamExt;
use garde::Validate;
use std::{sync::Arc, time::Duration};
use tokio::sync::broadcast;
use tokio::sync::broadcast::error::RecvError;

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
pub async fn get_mycute_home_dir(
    Extension(config_manager): Extension<Arc<ConfigManager>>,
) -> Result<Json<MyCuteHomeDirRes>, ApiError> {
    log::debug!("<MyCute> Fetching home directory.");
    let res = mycute_bl::get_home_dir(&config_manager.home_dir).await;
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

// get_mycute_llms / set_mycute_llms は LMGW 移行に伴い廃止済み。
// LLM プロバイダー管理は Bifrost 経由で行われる。

// ============================================================
// Verify CA Cert (CA Token)
// ============================================================
const VERIFY_CA_CERT_DESC: &str = r#"
### ⚫︎ 概要
- 外部から提供された CA任命証が、オーナーによって正しく署名されており、かつ有効期限内であるかを検証します。
- このエンドポイントは任命証単体での正当性を確認するものであり、特定のノードとの紐付けチェックは含みません。

### ⚫︎ 権限
- パブリック（認証不要）。誰でもアクセス可能です。

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `ca_token` | string | required | 検証対象の CA任命証 |

### ⚫︎ Response
| KEY | TYPE | DESCRIPTION |
| --- | --- | --- |
| `success` | boolean | 検証が成功したかどうか |
| `message` | string | 検証結果のメッセージ |
| `ca_pubkey` | string (optional) | 署名が正当な場合に抽出された CA の公開鍵 |
| `expire_at` | number (optional) | CA任命証の有効期限（Unix TS） |
"#;
#[utoipa::path(
    tag = TAG,
    post,
    path = "/mycute/catoken/verify",
    summary = "CA任命証の妥当性を検証する。",
    description = VERIFY_CA_CERT_DESC,
    request_body = VerifyCaTokenReq,
    responses(
        (status = 200, description = "Verification Result", body = VerifyCaTokenRes),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn verify_ca_token(
    Json(payload): Json<VerifyCaTokenReq>,
) -> Result<Json<VerifyCaTokenRes>, ApiError> {
    log::debug!("<MyCute> Verifying CA Cert.");
    let res = mycute_bl::verify_ca_token(&payload.ca_token).await;
    Ok(Json(res))
}

// ============================================================
// License Management
// ============================================================

// -------------------------------------------------------------------------
// List Licenses
// -------------------------------------------------------------------------
const LIST_LICENSES_DESC: &str = r#"
### ⚫︎ 概要
自身が登録しているライセンスの一覧を取得します。

### ⚫︎ アクセス権限
- パブリック（認証不要）。誰でもアクセス可能です。

### ⚫︎ Response
| KEY | TYPE | DESCRIPTION |
| --- | --- | --- |
| `licenses` | array | LicenseSummary の配列 |
"#;
#[utoipa::path(
    tag = TAG,
    get,
    path = "/mycute/license/list",
    summary = "登録済みライセンスの一覧を取得する。",
    description = LIST_LICENSES_DESC,
    responses(
        (status = 200, description = "Success", body = ListLicensesRes),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn list_licenses(
    Extension(config_manager): Extension<Arc<ConfigManager>>,
) -> Result<Json<ListLicensesRes>, ApiError> {
    log::debug!("<MyCute> Listing licenses.");
    let licenses = license_bl::list_licenses(config_manager).await;
    Ok(Json(ListLicensesRes { licenses }))
}

// -------------------------------------------------------------------------
// Register License
// -------------------------------------------------------------------------
const REGISTER_LICENSE_DESC: &str = r#"
### ⚫︎ 概要
CA から受け取ったライセンス文字列を検証し、自身の保持リスト（`my_lics`）に登録します。

登録前に以下の検証を行います：
1. ライセンス文字列のパースと署名検証（CA公開鍵による）
2. 埋め込まれた CA トークンのオーナー署名検証
3. ライセンスの有効期限が CA トークンの期限内であることの確認
4. ライセンスが自身の公開鍵に発行されたものであることの確認

### ⚫︎ アクセス権限
- **USR**: 使用可能。有効な USR トークンが必要です。

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `license` | string | required | 登録するライセンス文字列 |

### ⚫︎ Response
| KEY | TYPE | DESCRIPTION |
| --- | --- | --- |
| `success` | boolean | 登録成功したかどうか |
| `message` | string | 処理結果のメッセージ |
| `summary` | object | 登録されたライセンスのサマリー |
"#;
#[utoipa::path(
    post,
    security(("api_jwt_token" = [])),
    path = "/mycute/license/register",
    summary = "ライセンスを登録する。",
    description = REGISTER_LICENSE_DESC,
    request_body = RegisterLicenseReq,
    responses(
        (status = 200, description = "Success", body = RegisterLicenseRes),
        (status = 400, description = "Bad Request", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn register_license(
    ju: JwtUsr,
    Extension(config_manager): Extension<Arc<ConfigManager>>,
    Extension(event_tx): Extension<Arc<broadcast::Sender<InternalEvent>>>,
    Json(payload): Json<RegisterLicenseReq>,
) -> Result<Json<RegisterLicenseRes>, ApiError> {
    ju.allow_roles(&[JwtRole::USR])?;
    payload.validate().map_err(|e| ApiError::from_garde(e))?;
    log::debug!("<MyCute> Registering license.");
    let summary = license_bl::register_license(config_manager.clone(), payload.license).await?;
    log::debug!("<MyCute> License registered. id: {}", summary.id);

    // 最新のライセンス一覧をブロードキャスト
    let licenses = license_bl::list_licenses(config_manager).await;
    let _ = event_tx.send(InternalEvent {
        seq: time::now_ts_ms(),
        kind: EventKind::LicensesChanged(licenses),
    });

    Ok(Json(RegisterLicenseRes {
        success: true,
        message: "License registered successfully.".to_string(),
        summary: Some(summary),
    }))
}

// -------------------------------------------------------------------------
// Unregister License
// -------------------------------------------------------------------------
const UNREGISTER_LICENSE_DESC: &str = r#"
### ⚫︎ 概要
指定した ID のライセンスを自身の保持リストから削除します。

### ⚫︎ アクセス権限
- **USR**: 使用可能。有効な USR トークンが必要です。

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `id` | string | required | 削除対象のライセンス ID (LicenseSummary.id) |

### ⚫︎ Response
| KEY | TYPE | DESCRIPTION |
| --- | --- | --- |
| `success` | boolean | 削除成功したかどうか |
| `message` | string | 処理結果のメッセージ |
"#;
#[utoipa::path(
    post,
    security(("api_jwt_token" = [])),
    path = "/mycute/license/unregister",
    summary = "ライセンスを削除する。",
    description = UNREGISTER_LICENSE_DESC,
    request_body = UnregisterLicenseReq,
    responses(
        (status = 200, description = "Success", body = UnregisterLicenseRes),
        (status = 400, description = "Bad Request", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn unregister_license(
    ju: JwtUsr,
    Extension(config_manager): Extension<Arc<ConfigManager>>,
    Extension(event_tx): Extension<Arc<broadcast::Sender<InternalEvent>>>,
    Json(payload): Json<UnregisterLicenseReq>,
) -> Result<Json<UnregisterLicenseRes>, ApiError> {
    ju.allow_roles(&[JwtRole::USR])?;
    payload.validate().map_err(|e| ApiError::from_garde(e))?;
    log::debug!("<MyCute> Unregistering license. id: {}", payload.id);
    license_bl::unregister_license(config_manager.clone(), &payload.id).await?;
    log::debug!("<MyCute> License unregistered. id: {}", payload.id);

    // 最新のライセンス一覧をブロードキャスト
    let licenses = license_bl::list_licenses(config_manager).await;
    let _ = event_tx.send(InternalEvent {
        seq: time::now_ts_ms(),
        kind: EventKind::LicensesChanged(licenses),
    });

    Ok(Json(UnregisterLicenseRes {
        success: true,
        message: "License unregistered successfully.".to_string(),
    }))
}

// -------------------------------------------------------------------------
// Verify License
// -------------------------------------------------------------------------
const VERIFY_LICENSE_DESC: &str = r#"
### ⚫︎ 概要
任意のライセンス文字列の妥当性を検証します（登録不要）。
信頼の鎖（ライセンス ➔ CA ➔ オーナー）を完全に辿って検証します。

### ⚫︎ アクセス権限
- パブリック（認証不要）。誰でもアクセス可能です。

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `license` | string | required | 検証対象のライセンス文字列 |

### ⚫︎ Response
| KEY | TYPE | DESCRIPTION |
| --- | --- | --- |
| `success` | boolean | 検証が成功したかどうか |
| `message` | string | 検証結果のメッセージ |
| `summary` | object | 検証済みのライセンスサマリー（成功時のみ） |
"#;
#[utoipa::path(
    tag = TAG,
    post,
    path = "/mycute/license/verify",
    summary = "ライセンスの妥当性を検証する。",
    description = VERIFY_LICENSE_DESC,
    request_body = VerifyLicenseReq,
    responses(
        (status = 200, description = "Verification Result", body = VerifyLicenseRes),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn verify_license(
    Json(payload): Json<VerifyLicenseReq>,
) -> Result<Json<VerifyLicenseRes>, ApiError> {
    payload.validate().map_err(|e| ApiError::from_garde(e))?;
    log::debug!("<MyCute> Verifying license.");
    match license_bl::verify_license_chain(&payload.license) {
        Ok(summary) => {
            log::debug!("<MyCute> License verification succeeded. id: {}", summary.id);
            Ok(Json(VerifyLicenseRes {
                success: true,
                message: "License is valid.".to_string(),
                summary: Some(summary),
            }))
        }
        Err(e) => {
            log::debug!("<MyCute> License verification failed: {}", e);
            Ok(Json(VerifyLicenseRes {
                success: false,
                message: format!("License verification failed: {}", e),
                summary: None,
            }))
        }
    }
}
