use axum::http::StatusCode;
use axum::{
    body::Body,
    extract::{
        ws::{Message, WebSocket},
        Query, WebSocketUpgrade,
    },
    http::header,
    response::IntoResponse,
    routing::{get, post},
    Extension, Router,
};

use axum::body::Bytes;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use std::net::SocketAddr;
use tauri::AppHandle;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message as TungsteniteMessage};
use tower_http::cors::{Any, CorsLayer};

use crate::constants::{
    MYCUTE_SDK_FILENAME, MYCUTE_SSE_PROXY_PATH, MYCUTE_SW_FILENAME, MYCUTE_WS_PROXY_PATH,
    PATH_OSCA_CERT_DOWNLOAD, PATH_PROXY_LEAK_CSP, PATH_PROXY_LEAK_SW, ST_BAD_GATEWAY,
    ST_INTERNAL_SERVER_ERROR, ST_NOT_FOUND, ST_OK,
};
use crate::mode::rt::rthandler::mycute_proxy_leaks_handler::{
    create_csp_leak_report, create_sw_leak_report,
};

use crate::mycute_settings::ConfigManager;
use base64::{engine::general_purpose, Engine as _};
use std::sync::Arc;

// コンパイル時に SDK と Service Worker をバイナリに埋め込む
// 相対パスは src/mode/cl/sw_server.rs から見た位置
const MYCUTE_SDK_JS: &[u8] = include_bytes!("../../../sdk-ts/dist/mycute_sdk.js");
const MYCUTE_SW_JS: &[u8] = include_bytes!("../../../sdk-ts/dist/mycute_sw.js");

#[derive(Deserialize)]
struct ProxyQuery {
    target: String,
}

/// Static Web ホスティングサーバーを起動する。
/// 指定されたポートで SDK と Service Worker を配信します。
/// また、プロキシリークの診断レポートを受け取るエンドポイントも提供します。
pub async fn run_sw_server(
    port: u16,
    app_handle: AppHandle,
    config_manager: Arc<ConfigManager>,
    hc: Arc<reqwest::Client>,
) -> anyhow::Result<()> {
    log::info!("Starting Static Web (SW) hosting server on port {}", port);
    log::info!("Bundled SDK and Service Worker are ready for delivery.");

    // CORS 設定
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let config_manager_clone = config_manager.clone();

    // ハンドラの定義
    let app = Router::new()
        .route(
            &format!("/{}", MYCUTE_SDK_FILENAME),
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "application/javascript")],
                    Bytes::from_static(MYCUTE_SDK_JS),
                )
            }),
        )
        .route(
            &format!("/{}", MYCUTE_SW_FILENAME),
            get(|| async {
                (
                    [(header::CONTENT_TYPE, "application/javascript")],
                    Bytes::from_static(MYCUTE_SW_JS),
                )
            }),
        )
        // Mobile OSCA (Proxy CA) Download Endpoint
        .route(
            PATH_OSCA_CERT_DOWNLOAD,
            get(move || async move {
                let settings = config_manager_clone.settings.read();
                if let Some(osca_cert_b64) = &settings.osca_certificate {
                    match general_purpose::STANDARD.decode(osca_cert_b64) {
                        Ok(bytes) => {
                            // PEM形式で返す
                            let disp = format!(
                                "attachment; filename=\"{}\"",
                                PATH_OSCA_CERT_DOWNLOAD.trim_start_matches('/')
                            );
                            let disp_val = match disp.parse::<header::HeaderValue>() {
                                Ok(v) => v,
                                Err(_) => header::HeaderValue::from_static("attachment"),
                            };

                            (
                                [
                                    (
                                        header::CONTENT_TYPE,
                                        header::HeaderValue::from_static("application/x-pem-file"),
                                    ),
                                    (header::CONTENT_DISPOSITION, disp_val),
                                ],
                                Body::from(bytes),
                            )
                                .into_response()
                        }
                        Err(e) => {
                            log::error!("Failed to decode OSCA cert for download: {}", e);
                            (
                                ST_INTERNAL_SERVER_ERROR,
                                "Failed to decode OSCA certificate",
                            )
                                .into_response()
                        }
                    }
                } else {
                    (ST_NOT_FOUND, "OSCA Certificate not generated yet").into_response()
                }
            }),
        )
        // プロキシパス
        .route(MYCUTE_WS_PROXY_PATH, get(ws_proxy_handler))
        .route(MYCUTE_SSE_PROXY_PATH, get(sse_proxy_handler))
        // 診断レポートパス (憲法遵守: v1/mycute_proxy_leak)
        .route(PATH_PROXY_LEAK_CSP, post(create_csp_leak_report))
        .route(PATH_PROXY_LEAK_SW, post(create_sw_leak_report))
        // 共通コンポーネント
        .layer(cors)
        .layer(Extension(hc))
        .layer(Extension(app_handle)); // ハンドラに AppHandle を注入

    // サーバー起動
    // 【意図的な外部公開】
    // 3911 (SWポート) は、同一ネットワーク上のモバイル端末等の外部デバイスから、
    // ルート証明書 (OSCA) をダウンロード可能にする必要があるため、
    // 127.0.0.1 ではなく 0.0.0.0 にバインドします。
    //
    // 補足:
    // 証明書の配布URL（例: `http://[LAN-IP]:3911/mycute-osca.pem`）は、
    // RT側の `/v1/osca/url` エンドポイントや、GUI上のQRコード等を通じて
    // 外部デバイスへ提供されます。これらのデバイスがLAN経由で本サーバーへ
    // 直接アクセスしてファイルをDLできるよう、ネットワークインターフェースを限定しません。
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to bind SW server to port {}: {}", port, e))?;

    axum::serve(listener, app)
        .await
        .map_err(|e| anyhow::anyhow!("SW server error: {}", e))?;

    Ok(())
}

// --- WebSocket Proxy Handler ---

async fn ws_proxy_handler(
    Query(query): Query<ProxyQuery>,
    ws: WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws_socket(socket, query.target))
}

async fn handle_ws_socket(client_socket: WebSocket, target_url: String) {
    log::info!("WS Proxy: Connecting to target: {}", target_url);

    let (server_socket, _) = match connect_async(&target_url).await {
        Ok(v) => v,
        Err(e) => {
            log::error!("WS Proxy: Connection failed to {}: {}", target_url, e);
            return;
        }
    };

    let (mut client_sender, mut client_receiver): (
        futures_util::stream::SplitSink<WebSocket, Message>,
        futures_util::stream::SplitStream<WebSocket>,
    ) = client_socket.split();
    let (mut server_sender, mut server_receiver) = server_socket.split();

    // Client -> Server
    let client_to_server = async {
        // Explicitly annotate stream item type if possible, or just used loop convention
        while let Some(msg) = client_receiver.next().await {
            let msg: Message = match msg {
                Ok(m) => m,
                Err(_) => break,
            };

            let t_msg = match msg {
                Message::Text(t) => TungsteniteMessage::Text(t.to_string().into()),
                Message::Binary(b) => TungsteniteMessage::Binary(b.to_vec().into()),
                Message::Ping(p) => TungsteniteMessage::Ping(p.to_vec().into()),
                Message::Pong(p) => TungsteniteMessage::Pong(p.to_vec().into()),
                Message::Close(_) => TungsteniteMessage::Close(None),
            };

            let send_result: Result<(), tokio_tungstenite::tungstenite::Error> =
                server_sender.send(t_msg).await;
            if send_result.is_err() {
                break;
            }
        }
    };

    // Server -> Client
    let server_to_client = async {
        while let Some(msg) = server_receiver.next().await {
            let msg: TungsteniteMessage = match msg {
                Ok(m) => m,
                Err(_) => break,
            };

            let a_msg = match msg {
                TungsteniteMessage::Text(t) => Message::Text(t.to_string().into()),
                TungsteniteMessage::Binary(b) => Message::Binary(b.to_vec().into()),
                TungsteniteMessage::Ping(p) => Message::Ping(p.to_vec().into()),
                TungsteniteMessage::Pong(p) => Message::Pong(p.to_vec().into()),
                TungsteniteMessage::Close(_) => Message::Close(None),
                TungsteniteMessage::Frame(_) => continue,
            };

            let send_result: Result<(), axum::Error> = client_sender.send(a_msg).await;
            if send_result.is_err() {
                break;
            }
        }
    };

    tokio::select! {
        _ = client_to_server => {},
        _ = server_to_client => {},
    }
    log::info!("WS Proxy: Connection closed for {}", target_url);
}

// --- SSE Proxy Handler ---

async fn sse_proxy_handler(
    Query(query): Query<ProxyQuery>,
    hc: Extension<Arc<reqwest::Client>>,
) -> impl IntoResponse {
    log::info!("SSE Proxy: Connecting to target: {}", query.target);

    match hc.get(&query.target).send().await {
        Ok(res) => {
            let res_status_u16 = res.status().as_u16();
            let status = StatusCode::from_u16(res_status_u16).unwrap_or(ST_OK);
            let headers = res.headers().clone();

            let mut response_headers = header::HeaderMap::new();
            for (k, v) in headers.iter() {
                // Convert reqwest HeaderName to axum HeaderName
                let name = match header::HeaderName::from_bytes(k.as_str().as_bytes()) {
                    Ok(n) => n,
                    Err(_) => continue,
                };

                // Convert reqwest HeaderValue to axum HeaderValue
                let value = match header::HeaderValue::from_bytes(v.as_bytes()) {
                    Ok(v) => v,
                    Err(_) => continue,
                };

                response_headers.insert(name, value);
            }

            // 安全なヘッダー解析
            let ct_val = match "text/event-stream".parse::<header::HeaderValue>() {
                Ok(v) => v,
                Err(_) => header::HeaderValue::from_static("text/event-stream"),
            };
            let cc_val = match "no-cache".parse::<header::HeaderValue>() {
                Ok(v) => v,
                Err(_) => header::HeaderValue::from_static("no-cache"),
            };

            response_headers.insert(header::CONTENT_TYPE, ct_val);
            response_headers.insert(header::CACHE_CONTROL, cc_val);

            let stream = res.bytes_stream().map(|result| {
                match result {
                    Ok(bytes) => Ok(bytes), // reqwest::bytes::Bytes -> axum::body::Bytes (compatible)
                    Err(e) => Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
                }
            });

            (status, response_headers, Body::from_stream(stream)).into_response()
        }
        Err(e) => {
            log::error!("SSE Proxy: Request failed: {}", e);
            (ST_BAD_GATEWAY, format!("Proxy Error: {}", e)).into_response()
        }
    }
}
