use axum::{
    body::Body,
    extract::{Query, WebSocketUpgrade, ws::{WebSocket, Message}},
    http::header,
    response::IntoResponse,
    routing::{get, post},
    Router,
    Extension,
};
use axum::http::StatusCode;

use tower_http::cors::{Any, CorsLayer};
use std::net::SocketAddr;
use serde::Deserialize;
use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message as TungsteniteMessage};
use axum::body::Bytes;
use tauri::AppHandle;

use crate::constants::{
    MYCUTE_SDK_FILENAME, MYCUTE_SSE_PROXY_PATH, MYCUTE_SW_FILENAME, MYCUTE_WS_PROXY_PATH, PATH_OSCA_CERT_DOWNLOAD, PATH_PROXY_LEAK_CSP, PATH_PROXY_LEAK_SW, ST_BAD_GATEWAY, ST_INTERNAL_SERVER_ERROR, ST_NOT_FOUND, ST_OK
};
use crate::mode::rt::rthandler::mycute_proxy_leaks_handler::{create_csp_leak_report, create_sw_leak_report};

use std::sync::Arc;
use crate::stt_config::ConfigManager;
use base64::{engine::general_purpose, Engine as _};

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
pub async fn run_sw_server(port: u16, app_handle: AppHandle, config_manager: Arc<ConfigManager>, hc: Arc<reqwest::Client>) {
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
        .route(&format!("/{}", MYCUTE_SDK_FILENAME), get(|| async {
            (
                [(header::CONTENT_TYPE, "application/javascript")],
                Bytes::from_static(MYCUTE_SDK_JS),
            )
        }))
        .route(&format!("/{}", MYCUTE_SW_FILENAME), get(|| async {
            (
                [(header::CONTENT_TYPE, "application/javascript")],
                Bytes::from_static(MYCUTE_SW_JS),
            )
        }))
        // Mobile OSCA (Proxy CA) Download Endpoint
        .route(PATH_OSCA_CERT_DOWNLOAD, get(move || async move {
            let settings = config_manager_clone.settings.read();
            if let Some(osca_cert_b64) = &settings.osca_certificate {
                 match general_purpose::STANDARD.decode(osca_cert_b64) {
                     Ok(bytes) => {
                         // PEM形式で返す
                         (
                            [
                                (header::CONTENT_TYPE, header::HeaderValue::from_static("application/x-pem-file")),
                                (header::CONTENT_DISPOSITION, format!("attachment; filename=\"{}\"", PATH_OSCA_CERT_DOWNLOAD.trim_start_matches('/')).parse::<header::HeaderValue>().unwrap())
                            ],
                            Body::from(bytes)
                         ).into_response()
                     }
                     Err(e) => {
                         log::error!("Failed to decode OSCA cert for download: {}", e);
                         (ST_INTERNAL_SERVER_ERROR, "Failed to decode OSCA certificate").into_response()
                     }
                 }
            } else {
                (ST_NOT_FOUND, "OSCA Certificate not generated yet").into_response()
            }
        }))
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
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();

    axum::serve(listener, app).await.unwrap();
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

    let (mut client_sender, mut client_receiver): (futures_util::stream::SplitSink<WebSocket, Message>, futures_util::stream::SplitStream<WebSocket>) = client_socket.split();
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

            let send_result: Result<(), tokio_tungstenite::tungstenite::Error> = server_sender.send(t_msg).await;
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
    hc: Extension<Arc<reqwest::Client>>
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
            response_headers.insert(header::CONTENT_TYPE, "text/event-stream".parse().unwrap());
            response_headers.insert(header::CACHE_CONTROL, "no-cache".parse().unwrap());

            let stream = res.bytes_stream().map(|result| {
                 match result {
                     Ok(bytes) => Ok(bytes), // reqwest::bytes::Bytes -> axum::body::Bytes (compatible)
                     Err(e) => Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
                 }
            });
            
            (status, response_headers, Body::from_stream(stream)).into_response()
        },
        Err(e) => {
            log::error!("SSE Proxy: Request failed: {}", e);
            (ST_BAD_GATEWAY, format!("Proxy Error: {}", e)).into_response()
        }
    }
}
