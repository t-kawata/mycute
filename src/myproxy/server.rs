use crate::constants::{
    DOMAIN_LOCALHOST, HEADER_X_MYCUTE_SCHEME, MYCUTE_PROXY_PORT, MYCUTE_SDK_FILENAME,
    MYCUTE_SW_FILENAME, PROTOCOL_HTTP, PROTOCOL_HTTPS, SCHEME_PREFIX_HTTP, ST_BAD_GATEWAY, ST_OK,
};
use crate::mycute_settings::ConfigManager;
use crate::myproxy::utils::{extract_original_host, process_proxy_headers, rewrite_html};
use axum::{body::Body, extract::Request, http::StatusCode, response::Response};
use http_body_util::BodyExt;
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio_rustls::rustls::ServerConfig;
use tokio_rustls::TlsAcceptor;

// バインドされたアドレスと生成されるサーバーのFutureを返します
pub async fn start_proxy_server(
    _config_manager: Arc<ConfigManager>,
    sw_port: u16,
    server_config: ServerConfig,
    hc: Arc<reqwest::Client>,
) -> anyhow::Result<(SocketAddr, impl std::future::Future<Output = ()> + Send)> {
    // TLS設定のセットアップ (呼び出し元から供給される)
    let tls_acceptor = TlsAcceptor::from(Arc::new(server_config));

    // Direct Hosting Architecture: service_fn で直接リクエストを処理するため、Router は不要

    // ローカルインターフェース (127.0.0.1) で固定ポートにバインド
    let addr = format!(
        "{}:{}",
        crate::constants::IP_LOCALHOST,
        crate::constants::MYCUTE_PROXY_PORT
    )
    .parse::<SocketAddr>()?;

    // バインドに失敗した場合はエラー（anyhow）として返し、アプリを終了させる
    let listener = TcpListener::bind(addr).await.map_err(|e| {
        log::error!("CRITICAL: Failed to bind to fixed HTTPS port {}: {}. Ensure no other instance is running.", MYCUTE_PROXY_PORT, e);
        anyhow::anyhow!("HTTPS port {} is already in use or unavailable. Cannot proceed safely.", MYCUTE_PROXY_PORT)
    })?;
    let local_addr = listener.local_addr()?;

    log::info!("MyProxy Direct HTTPS Server initialized on {}", local_addr);

    let server_future = async move {
        // TCPリスナーからストリームを受け入れ、即座にTLSハンドシェイクを行う
        loop {
            let (tcp_stream, _remote_addr) = match listener.accept().await {
                Ok(v) => v,
                Err(e) => {
                    log::warn!("Failed to accept TCP connection: {}", e);
                    continue;
                }
            };

            let tls_acceptor = tls_acceptor.clone();
            let hc = hc.clone();

            tokio::spawn(async move {
                // TLS ハンドシェイク
                let tls_stream = match tls_acceptor.accept(tcp_stream).await {
                    Ok(s) => s,
                    Err(e) => {
                        log::warn!("TLS handshake failed: {}", e);
                        return;
                    }
                };

                // Hyper互換のストリームに変換
                let stream = TokioIo::new(tls_stream);

                // hyper::service::service_fn を使用 (handle_mitm と同じパターン)
                let hc_for_req = hc.clone();
                let service = hyper::service::service_fn(
                    move |req: axum::extract::Request<hyper::body::Incoming>| {
                        handle_inner_request(req, sw_port, hc_for_req.clone())
                    },
                );

                if let Err(e) = hyper_util::server::conn::auto::Builder::new(
                    hyper_util::rt::TokioExecutor::new(),
                )
                .serve_connection(stream, service)
                .await
                {
                    log::error!("Error serving connection: {}", e);
                }
            });
        }
    };

    Ok((local_addr, server_future))
}

const MOBILE_USER_AGENTS: &[&str] = &[
    "Mozilla/5.0 (iPhone; CPU iPhone OS 17_2 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.2 Mobile/15E148 Safari/605.1.15",
    "Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/121.0.6167.143 Mobile Safari/537.36",
    "Mozilla/5.0 (iPhone; CPU iPhone OS 17_2 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) CriOS/121.0.6167.138 Mobile/15E148 Safari/604.1",
    "Mozilla/5.0 (Linux; Android 10; K) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/121.0.0.0 Mobile Safari/537.36",
];

async fn handle_inner_request(
    req: Request<hyper::body::Incoming>,
    sw_port: u16,
    hc: Arc<reqwest::Client>,
) -> Result<Response<Body>, hyper::Error> {
    // 1. URL再構築ロジック (簡略化)
    let host_header = req
        .headers()
        .get("Host")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");
    let path = req
        .uri()
        .path_and_query()
        .map(|p| p.as_str())
        .unwrap_or("/");

    // SDK/SWファイルのインターセプトをチェック
    if path.ends_with(MYCUTE_SDK_FILENAME) || path.ends_with(MYCUTE_SW_FILENAME) {
        let filename = if path.ends_with(MYCUTE_SDK_FILENAME) {
            MYCUTE_SDK_FILENAME
        } else {
            MYCUTE_SW_FILENAME
        };
        let local_url = format!(
            "{}{}:{}/{}",
            SCHEME_PREFIX_HTTP, DOMAIN_LOCALHOST, sw_port, filename
        );

        match hc.get(&local_url).send().await {
            Ok(res) => {
                let status_u16 = res.status().as_u16();
                let status = StatusCode::from_u16(status_u16).unwrap_or(ST_OK);

                let mut builder = Response::builder().status(status);
                builder = builder.header("Access-Control-Allow-Origin", "*");
                builder = builder.header("Content-Type", "application/javascript");
                let bytes = res.bytes().await.unwrap_or_default();
                return Ok(builder
                    .body(Body::from(bytes))
                    .unwrap_or_else(|_| Response::new(Body::from("Internal Server Error"))));
            }
            Err(_) => {
                return Ok(Response::builder()
                    .status(500)
                    .body(Body::from("Failed to load local asset"))
                    .unwrap_or_else(|_| Response::new(Body::from("Internal Server Error"))));
            }
        }
    }

    // 2. ターゲットURLを決定
    let original_host = extract_original_host(host_header);

    // プロトコル決定ロジック (Plan B)
    // 1. ヘッダー X-Mycute-Origin-Scheme を確認
    // 2. なければ https をデフォルトとする (グレースフルフォールバック)
    let scheme_header = req
        .headers()
        .get(HEADER_X_MYCUTE_SCHEME)
        .and_then(|h| h.to_str().ok())
        .unwrap_or(PROTOCOL_HTTPS);

    let scheme = if scheme_header == PROTOCOL_HTTP {
        PROTOCOL_HTTP
    } else {
        PROTOCOL_HTTPS
    };

    let target_url = format!("{}://{}{}", scheme, original_host, path);

    // 3. リクエストのメタデータとボディを分離
    let (parts, body) = req.into_parts();
    let req_method_str = parts.method.as_str();
    let reqwest_method =
        reqwest::Method::from_bytes(req_method_str.as_bytes()).unwrap_or(reqwest::Method::GET);

    // 4. ボディを全読み込み (reqwest への転送用)
    let body_bytes = match body.collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            log::error!("[ProxyServer] Failed to collect request body: {}", e);
            return Ok(Response::builder()
                .status(ST_BAD_GATEWAY)
                .body(Body::from("Failed to read body"))
                .unwrap_or_else(|_| Response::new(Body::from("Bad Gateway"))));
        }
    };

    // 5. リクエストを転送
    // ヘッダーを変換 (ブラウザの挙動を模倣)
    let body_len = body_bytes.len();
    let mut builder = hc.request(reqwest_method, &target_url).body(body_bytes);

    for (k, v) in parts.headers.iter() {
        let k_str = k.as_str();
        if k_str.eq_ignore_ascii_case("host")
            || k_str.eq_ignore_ascii_case("accept-encoding")
            || k_str.eq_ignore_ascii_case("content-length")
            || k_str.eq_ignore_ascii_case("transfer-encoding")
            || k_str.eq_ignore_ascii_case(HEADER_X_MYCUTE_SCHEME)
        {
            continue;
        }

        // Origin / Referer の書き換え (汎用ロジック)
        if k_str.eq_ignore_ascii_case("origin") || k_str.eq_ignore_ascii_case("referer") {
            if let Ok(val_str) = v.to_str() {
                // 値が URL 形式であればホストを書き換える
                if let Ok(mut url) = reqwest::Url::parse(val_str) {
                    if let Some(host) = url.host_str() {
                        let original = crate::myproxy::utils::extract_original_host(host);
                        if original != host {
                            let _ = url.set_host(Some(&original));
                            // プロキシポート(58300)が含まれていれば除去 (標準ポートへ)
                            if url.port() == Some(MYCUTE_PROXY_PORT) {
                                let _ = url.set_port(None);
                            }
                            builder = builder.header(k_str, url.to_string());
                            continue;
                        }
                    }
                }
            }
        }

        builder = builder.header(k_str, v.as_bytes());
    }

    // Content-Length を明示的に設定 (411 Length Required 対策)
    builder = builder.header("Content-Length", body_len.to_string());

    // User-Agent のみモバイル版を強制 (アプリの仕様として)
    // それ以外の Accept, Sec-Fetch 等はブラウザからの値を尊重してパススルーする
    let ua = MOBILE_USER_AGENTS[0]; // TODO: ランダム化
    builder = builder.header("User-Agent", ua);

    // Hostヘッダーの修正
    builder = builder.header("Host", &original_host);

    match builder.send().await {
        Ok(res) => {
            // ステータスを変換 (reqwest/http 0.2 -> http 1.0)
            let status_u16 = res.status().as_u16();
            if status_u16 >= 400 {
                log::warn!(
                    "[ProxyServer] Upstream Error {}: {} (Method: {})",
                    status_u16,
                    target_url,
                    req_method_str
                );
            }
            let status = StatusCode::from_u16(status_u16).unwrap_or(ST_BAD_GATEWAY);

            // 処理のためにヘッダーを Vec<(String, String)> に変換
            let mut origin_headers = Vec::new();
            for (k, v) in res.headers() {
                if let Ok(val_str) = v.to_str() {
                    origin_headers.push((k.as_str().to_string(), val_str.to_string()));
                }
            }

            // ヘッダーを処理
            let processed_headers = process_proxy_headers(origin_headers, sw_port);

            let mut resp_builder = Response::builder().status(status);
            let mut is_html = false;
            let mut is_css = false;

            for (k, v) in processed_headers {
                if k.to_lowercase() == "content-type" {
                    let v_str = v.to_lowercase();
                    if v_str.contains("text/html") {
                        is_html = true;
                    } else if v_str.contains("text/css") {
                        is_css = true;
                    }
                }
                resp_builder = resp_builder.header(k, v);
            }

            // ボディの処理
            let bytes = res.bytes().await.unwrap_or_default().to_vec();
            let final_bytes = if is_html && parts.method == hyper::Method::GET {
                rewrite_html(bytes)
            } else if is_css && parts.method == hyper::Method::GET {
                // Rewrite CSS content
                if let Ok(css_str) = String::from_utf8(bytes.clone()) {
                    crate::myproxy::utils::rewrite_css_content(&css_str).into_bytes()
                } else {
                    bytes
                }
            } else {
                bytes
            };

            Ok(resp_builder
                .body(Body::from(final_bytes))
                .unwrap_or_else(|_| Response::new(Body::from("Internal Server Error"))))
        }
        Err(e) => {
            log::error!("[ProxyServer] 502 Bad Gateway to {}: {}", target_url, e);
            if e.is_timeout() {
                log::error!("[ProxyServer] Reason: Timeout");
            } else if e.is_connect() {
                log::error!("[ProxyServer] Reason: Connection failure (DNS/Firewall/Refused)");
            } else if e.is_decode() {
                log::error!("[ProxyServer] Reason: Decode error (Brotli/Gzip corruption)");
            } else {
                log::error!("[ProxyServer] Reason: Unknown ({:?})", e);
            }
            Ok(Response::builder()
                .status(ST_BAD_GATEWAY)
                .body(Body::from(format!("Proxy Error: {}", e)))
                .unwrap_or_else(|_| Response::new(Body::from("Bad Gateway"))))
        }
    }
}
