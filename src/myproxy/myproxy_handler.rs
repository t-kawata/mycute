#![allow(deprecated)]
use std::sync::Arc;
use tauri::{Builder, Runtime};
use tauri::http::Response;
use crate::constants::{
    MYCUTE_SDK_FILENAME, MYCUTE_SW_FILENAME, MYCUTE_ORIGIN,
    MYCUTE_SCHEME_HTTP, MYCUTE_SCHEME_HTTPS,
    HEADER_X_IS_MYCUTE,
    SCHEME_PREFIX_HTTP, SCHEME_PREFIX_HTTPS, DOMAIN_LOCALHOST
};
use crate::myproxy::utils::extract_original_host;
// use std::io::Read; // reqwest::blocking::Response implements Read

/// 本プロキシシステムがサポートする通信スキームの定義。
#[derive(Debug, Clone, Copy)]
enum MyProxyScheme {
    /// HTTP (mycute://) -> http://
    Http,
    /// HTTPS (mycutes://) -> https://
    Https,
}

impl MyProxyScheme {
    /// WebView からのリクエストを捕捉するためのカスタムスキーム名。
    fn scheme(&self) -> &'static str {
        match self {
            Self::Http => MYCUTE_SCHEME_HTTP,
            Self::Https => MYCUTE_SCHEME_HTTPS,
        }
    }

    /// 実際のネットワークリクエストを投げる際の本来のプロトコルのプレフィックス。
    fn target_prefix(&self) -> &'static str {
        match self {
            Self::Http => SCHEME_PREFIX_HTTP,
            Self::Https => SCHEME_PREFIX_HTTPS,
        }
    }

    /// すべてのスキームバリエーションを返す。
    fn all() -> &'static [Self] {
        &[Self::Http, Self::Https]
    }
}

/// 近代的なデスクトップブラウザの User-Agent リスト（優先順位順）。
/// 1番目から順に試行し、拒否された場合は次のエージェントでリトライします。
/// 
/// 【重要: メンテナンス上の注意】
/// ここで定義されている User-Agent は最新の安定版ブラウザを元にした固定値です。
/// 時代が進むにつれ、これらのバージョンは「古いブラウザ」と判定されるリスクがあります。
/// 大手サイト等で表示に支障が出始めた場合、まずここにある User-Agent 文字列を
/// その時点での最新のモダンブラウザのものへ更新することを検討してください。
const USER_AGENTS: &[&str] = &[
    // iOS 17 Safari
    "Mozilla/5.0 (iPhone; CPU iPhone OS 17_2 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.2 Mobile/15E148 Safari/605.1.15",
    // Android Chrome (Pixel 8)
    "Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/121.0.6167.143 Mobile Safari/537.36",
    // iOS Chrome
    "Mozilla/5.0 (iPhone; CPU iPhone OS 17_2 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) CriOS/121.0.6167.138 Mobile/15E148 Safari/604.1",
    // Modern Android (Generic)
    "Mozilla/5.0 (Linux; Android 10; K) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/121.0.0.0 Mobile Safari/537.36",
];

/// 本プロキシシステムの核心部。
/// HTTP (mycute://) と HTTPS (mycutes://) の両スキームをキャッチし、
/// 実際のネットワークリクエストへ転送します。
pub fn setup_proxy<R: Runtime>(builder: Builder<R>, sw_port: u16, hc: Arc<reqwest::blocking::Client>) -> Builder<R> {
    let mut builder = builder;
    for scheme in MyProxyScheme::all() {
        builder = register_protocol(builder, *scheme, sw_port, hc.clone());
    }
    builder
}

fn register_protocol<R: Runtime>(builder: Builder<R>, proxy_scheme: MyProxyScheme, sw_port: u16, hc: Arc<reqwest::blocking::Client>) -> Builder<R> {
    let scheme = proxy_scheme.scheme();
    let target_prefix = proxy_scheme.target_prefix();

    builder.register_uri_scheme_protocol(scheme, move |_ctx, request| {
        let uri = request.uri().to_string();
        log::debug!("[ProxyTrace] Received request: {} {}", request.method(), uri);
        
        // ターゲットURLの決定ロジック (Phase 8 Refactoring)
        // 1. まずは従来のスキーム置換を試みる (mycute:// -> http://)
        //    これにより "http://example.com.mc.shyme.net/..." のような形になる可能性がある
        let mut target_url = uri.replace(&format!("{}://", scheme), target_prefix);
        
        // Double-hyphen encoding: "." -> "--" before resolving
        // This is primarily for outgoing request construction, but here we are receiving a request
        // that *already* has the suffix. Wait, register_protocol handles requests to "mycute://..."
        // which are intercepted by Tauri.
        // It seems the user wants the *outgoing* requests (handled by the proxy logic if acting as a full proxy)
        // or the transformation logic here to support this.
        
        // In the context of "mycute://google.com...", the URI is "mycute://google.com...". 
        // We replace "mycute://" with "http://".
        // If the goal is that when *we* (the app) construct a URL to be loaded by WebView,
        // we should encode it.
        // BUT, `register_protocol` handles the request *from* WebView.
        // If WebView loads "https://google--com.mc.shyme.net", it comes here?
        // NO. `register_protocol` handles `mycute://` scheme. 
        // The SOCKS/HTTP proxy handles `http(s)://...`.
        
        // Ah, the user instruction was for the entire system.
        // If this file handles the custom protocol `mycute://`, it might need adjustment if it interacts with the suffix logic.
        // Lines 84-96 check for the suffix.
        // If the incoming URI is `mycute://google--com.mc.shyme.net/...`
        // Then `target_url` becomes `http://google--com.mc.shyme.net/...`
        // Then `extract_original_host` is called.
        // `extract_original_host` now handles `--` -> `.`, so it returns `google.com`.
        // Then `url_obj.set_host` sets it to `google.com`.
        // So `target_url` becomes `http://google.com/...`.
        // This looks correct for the decoding side.

        // 2. ホスト名にサフィックスが含まれているか確認し、あれば除去する (Domain Suffix Proxy)
        //    target_url は "https://example.com.mc.shyme.net/path" のようになっている想定
        if let Ok(mut url_obj) = reqwest::Url::parse(&target_url) {
            if let Some(host_str) = url_obj.host_str() {
                let original_host = extract_original_host(host_str);
                // サフィックスを除去した真のホスト名を設定
                if url_obj.set_host(Some(&original_host)).is_ok() {
                    let old_url = target_url.clone();
                    target_url = url_obj.to_string();
                    log::trace!("[ProxyTrace] URL transformed: {} -> {}", old_url, target_url);
                }
            }
        }

        // OPTIONSリクエストのハンドリング
        // プリフライトリクエストには即座に許可応答を返す
        if request.method() == "OPTIONS" {
            log::trace!("[ProxyTrace] Handling OPTIONS request internally.");
            return Response::builder()
                .status(200)
                .header("Access-Control-Allow-Origin", "*")
                .header("Access-Control-Allow-Methods", "GET, POST, OPTIONS, PUT, DELETE, PATCH")
                .header("Access-Control-Allow-Headers", "*")
                .header("Access-Control-Max-Age", "86400")
                .body(Vec::new())
                .unwrap();
        }

        // SDKおよびService Workerのエイリアス処理
        // SDK/SW へのリクエストをローカルサーバーへ転送
        // ドメインに関係なく、パスの末尾が SDK/SW ファイル名であればインターセプトする
        if target_url.ends_with(&format!("/{}", MYCUTE_SDK_FILENAME)) || target_url.ends_with(&format!("/{}", MYCUTE_SW_FILENAME)) {
            log::trace!("[ProxyTrace] Intercepting SDK/SW request: {}", target_url);
            let filename = if target_url.ends_with(MYCUTE_SDK_FILENAME) { MYCUTE_SDK_FILENAME } else { MYCUTE_SW_FILENAME };
            let local_url = format!("{}{}:{}/{}", SCHEME_PREFIX_HTTP, DOMAIN_LOCALHOST, sw_port, filename);
            
            if let Ok(res) = hc.get(&local_url).send() {
                if let Ok(bytes) = res.bytes() {
                    let mut builder = Response::builder()
                        .status(200)
                        .header("Content-Type", "application/javascript")
                        .header("Access-Control-Allow-Origin", "*");
                    
                    // Service Worker への特権付与
                    if filename == MYCUTE_SW_FILENAME {
                        builder = builder.header("Service-Worker-Allowed", "/");
                    }
                    
                    log::trace!("[ProxyTrace] Served SDK/SW from local server.");
                    return builder.body(bytes.to_vec()).unwrap();
                }
            }
            // Fallback to error if local fetch fails
            log::error!("[ProxyTrace] Failed to fetch local asset: {}", local_url);
            return Response::builder().status(500).body(b"Failed to load bundled asset".to_vec()).unwrap();
        }

        let method_str = request.method().as_str();
        let reqwest_method = reqwest::Method::from_bytes(method_str.as_bytes())
            .unwrap_or(reqwest::Method::GET);
        
        let body_data = request.body().clone();
        let mut last_error = None;

        // --- 優先順位付き User-Agent によるリトライループ ---
        for (i, &ua) in USER_AGENTS.iter().enumerate() {
            log::trace!("[ProxyTrace] Attempt {} with UA: {:.30}...", i + 1, ua);
            let mut req_builder = hc.request(reqwest_method.clone(), &target_url).body(body_data.clone());
            
            // 1. 元のリクエストからヘッダーをコピー
            for (name, value) in request.headers() {
                 let name_str = name.as_str();
                 if name_str.eq_ignore_ascii_case("Host") 
                    || name_str.eq_ignore_ascii_case("Origin") 
                    || name_str.eq_ignore_ascii_case("User-Agent") 
                    || name_str.starts_with("Sec-") {
                     continue;
                 }
                 if let Ok(val) = reqwest::header::HeaderValue::from_bytes(value.as_bytes()) {
                      req_builder = req_builder.header(name_str, val);
                 }
            }
            
            // 2. ブラウザとしての正当性を高めるためのヘッダー偽装
            req_builder = req_builder
                .header("User-Agent", ua)
                .header("Origin", MYCUTE_ORIGIN)
                .header(HEADER_X_IS_MYCUTE, "1")
                .header("Accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7")
                .header("Accept-Language", "ja,en-US;q=0.9,en;q=0.8")
                .header("Sec-Fetch-Site", "none")
                .header("Sec-Fetch-Mode", "navigate")
                .header("Sec-Fetch-User", "?1")
                .header("Sec-Fetch-Dest", "document")
                .header("Upgrade-Insecure-Requests", "1");

            // Host ヘッダーの明示的な書き換え
            if let Ok(url_obj) = reqwest::Url::parse(&target_url) {
                if let Some(host) = url_obj.host_str() {
                    req_builder = req_builder.header("Host", host);
                }
            }

            // リクエスト実行 (同期)
            log::trace!("[ProxyTrace] Sending request...");
            let result: Result<reqwest::blocking::Response, reqwest::Error> = req_builder.send();
            
            match result {
                Ok(res) => {
                    let status = res.status().as_u16();
                    log::debug!("[ProxyTrace] Received response status: {}", status);
                    
                    if status == 403 {
                        log::warn!("[ProxyTrace] Access denied (403). Retrying...");
                        continue;
                    }

                    let mut origin_headers = Vec::new();
                    for (name, value) in res.headers() {
                         if let Ok(val_str) = value.to_str() {
                             origin_headers.push((name.as_str().to_string(), val_str.to_string()));
                         }
                    }

                    use crate::myproxy::utils::{process_proxy_headers, rewrite_html};
                    let processed_headers = process_proxy_headers(origin_headers, sw_port);

                    let mut response_builder = Response::builder().status(status);
                    let mut is_html = false;

                    for (k, v) in processed_headers {
                        if k.to_lowercase() == "content-type" && v.contains("text/html") {
                            is_html = true;
                        }
                        response_builder = response_builder.header(k, v);
                    }

                    log::trace!("[ProxyTrace] Reading body (is_html: {})...", is_html);
                    let mut body = res.bytes().unwrap_or_default().to_vec();
                    log::trace!("[ProxyTrace] Body read complete (size: {} bytes).", body.len());

                    if is_html && request.method() == "GET" {
                         log::trace!("[ProxyTrace] Injecting SDK script...");
                         body = rewrite_html(body);
                    }

                    response_builder = response_builder.header("Content-Length", body.len().to_string());
                    return response_builder.body(body).unwrap();
                },
                Err(e) => {
                    log::error!("[ProxyTrace] Request failed to {}: {}", target_url, e);
                    if e.is_timeout() {
                        log::error!("[ProxyTrace] Error type: Timeout");
                    } else if e.is_connect() {
                        log::error!("[ProxyTrace] Error type: Connection Error");
                    } else if e.is_request() {
                        log::error!("[ProxyTrace] Error type: Request Error");
                    } else {
                        log::error!("[ProxyTrace] Error type: Other ({:?})", e);
                    }
                    last_error = Some(e);
                    continue;
                }
            }
        }

        let err_msg = if let Some(e) = last_error {
            format!("Proxy Error: {}", e)
        } else {
            "Proxy Error: Access denied by all candidates".to_string()
        };
        
        log::error!("[ProxyTrace] All attempts failed: {}", err_msg);

        Response::builder()
            .status(500)
            .header("Content-Type", "text/plain")
            .body(err_msg.into_bytes())
            .unwrap()
    })
}

#[cfg(test)]
mod tests {
    use super::MyProxyScheme;

    #[test]
    fn test_url_replace() {
        let scheme_https = MyProxyScheme::Https;
        let uri = format!("{}://google.com/search?q=test", scheme_https.scheme());
        let target = uri.replace(&format!("{}://", scheme_https.scheme()), scheme_https.target_prefix());
        assert_eq!(target, "https://google.com/search?q=test");

        let scheme_http = MyProxyScheme::Http;
        let uri_http = format!("{}://example.com", scheme_http.scheme());
        let target_http = uri_http.replace(&format!("{}://", scheme_http.scheme()), scheme_http.target_prefix());
        assert_eq!(target_http, "http://example.com");
    }
}
