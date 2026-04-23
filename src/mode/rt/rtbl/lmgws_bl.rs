use std::sync::Arc;
use futures::StreamExt;
use axum::{
    body::Body,
    http::{HeaderMap, HeaderName, HeaderValue, Method, StatusCode},
    response::Response,
};
use reqwest::Client;
use crate::{
    constants::IP_LOCALHOST,
    mode::rt::{
        rtres::errs_res::ApiError,
        rterr::rterr,
    },
    mycute_settings::ConfigManager,
};

/// Bifrost HTTP API への完全透過プロキシクライアント。
///
/// # 設計方針
/// 従来の個別メソッド（get_config, search_providers 等）を全廃し、
/// 単一の proxy_lmgw_request メソッドのみを提供する。
/// これにより、Bifrost が将来エンドポイントを追加・変更しても
/// MYCUTE 側のコードを一切変更せずに即時対応が可能となる。
///
/// # 認証フロー
/// 1. MYCUTE の JWT 認証は上位のハンドラー（JwtUsr）で完結させる。
/// 2. クライアントが送ってきた Authorization ヘッダーを削除し、
///    BIFROST_AUTH_SECRET を Bearer トークンとして再注入してから転送する。
pub struct BifrostClient {
    hc: Arc<Client>,
    config_manager: Arc<ConfigManager>,
}

impl BifrostClient {
    pub fn new(hc: Arc<Client>, config_manager: Arc<ConfigManager>) -> Self {
        Self { hc, config_manager }
    }

    /// Bifrost のベース URL を組み立てる。
    /// ポートは ConfigManager の設定値から動的に取得するため、設定変更に追従できる。
    fn get_base_url(&self) -> String {
        let port = self.config_manager.settings.read().server.bifrost_port;
        format!("http://{}:{}", IP_LOCALHOST, port)
    }

    /// LMGW シークレットを取得する。
    /// 未設定の場合は空文字列で代替する（エラーにしないことで起動を止めない）。
    fn get_secret(&self) -> String {
        self.config_manager.get_lmgw_secret().unwrap_or_default()
    }

    /// Bifrost へリクエストを透過的に中継し、レスポンスをそのまま返す。
    ///
    /// # 処理の流れ
    /// 1. proxy_path と Bifrost ベース URL を連結してリクエスト先 URL を確定する。
    /// 2. クライアントからのヘッダーを転送するが、転送に不適切な hop-by-hop ヘッダー
    ///    （host, connection, transfer-encoding, te, trailer, upgrade）は除去する。
    /// 3. Authorization ヘッダーを BIFROST_AUTH_SECRET で上書きする。
    /// 4. リクエスト Body はバッファリングせずストリームのまま reqwest に渡す。
    ///    これにより動画・ファイルアップロード等の巨大なペイロードもメモリを圧迫しない。
    /// 5. Bifrost からのレスポンス（SSE ストリーム含む）をステータス・ヘッダーごと
    ///    axum::response::Response に乗せてそのままクライアントへ返す。
    pub async fn proxy_lmgw_request(
        &self,
        method: Method,
        proxy_path: &str,
        incoming_headers: HeaderMap,
        body: Body,
    ) -> Result<Response<Body>, ApiError> {
        // URL 構築: proxy_path は "/" 始まりとは限らないため、確実に "/" で繋ぐ
        let base = self.get_base_url();
        let url = if proxy_path.starts_with('/') {
            format!("{}{}", base, proxy_path)
        } else {
            format!("{}/{}", base, proxy_path)
        };
        log::debug!("<LMGW> proxy request: {} {}", method, url);

        // 転送禁止ヘッダー（hop-by-hop）の一覧
        // これらをそのまま転送すると Bifrost 側でホスト名不一致等のエラーを引き起こす
        let blocked_headers: &[&str] = &[
            "host",
            "connection",
            "transfer-encoding",
            "te",
            "trailer",
            "upgrade",
            "authorization", // 後で BIFROST_AUTH_SECRET で上書きするため先に除去
        ];

        // クライアントのヘッダーを転送用ヘッダーマップに変換
        let mut fwd_headers = reqwest::header::HeaderMap::new();
        for (name, value) in &incoming_headers {
            let name_str = name.as_str().to_lowercase();
            if blocked_headers.contains(&name_str.as_str()) {
                continue;
            }
            // ヘッダー名・値を reqwest 形式に変換（エラーは無視してスキップ）
            if let (Ok(n), Ok(v)) = (
                reqwest::header::HeaderName::from_bytes(name.as_str().as_bytes()),
                reqwest::header::HeaderValue::from_bytes(value.as_bytes()),
            ) {
                fwd_headers.insert(n, v);
            }
        }

        // BIFROST_AUTH_SECRET を Authorization ヘッダーとして注入
        let secret = self.get_secret();
        if let Ok(auth_val) = reqwest::header::HeaderValue::from_str(&format!("Bearer {}", secret)) {
            fwd_headers.insert(reqwest::header::AUTHORIZATION, auth_val);
        }

        // Body を reqwest::Body としてストリーム変換
        // http_body_util::BodyStream は Frame<Bytes> を出力するが、
        // reqwest::Body::wrap_stream は From<S::Ok> for Bytes を要求する。
        // そのため、filter_map でデータフレーム（Frame::data）からのみ Bytes を取り出し、
        // トレーラーフレームはスキップする形でストリームを変換する。
        let body_stream = reqwest::Body::wrap_stream(
            http_body_util::BodyStream::new(body)
                .filter_map(|frame_result| async move {
                    match frame_result {
                        Ok(frame) => frame.into_data().ok().map(Ok),
                        Err(e) => Some(Err(e)),
                    }
                })
        );

        // Bifrost へリクエスト送信
        let resp = self.hc
            .request(
                reqwest::Method::from_bytes(method.as_str().as_bytes())
                    .unwrap_or(reqwest::Method::GET),
                &url,
            )
            .headers(fwd_headers)
            .body(body_stream)
            .send()
            .await
            .map_err(|e| {
                log::error!("<LMGW> Failed to send request to Bifrost: {}", e);
                ApiError::new_system(StatusCode::BAD_GATEWAY, rterr::ERR_UNEXPECTED, e.to_string())
            })?;

        // ステータスコードを axum 形式に変換
        let status = StatusCode::from_u16(resp.status().as_u16())
            .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        log::debug!("<LMGW> Bifrost responded with status: {}", status);

        // レスポンスヘッダーを axum 形式に変換して転送
        // hop-by-hop ヘッダーは除去し、content-type や content-length 等のみ転送する
        let blocked_res_headers: &[&str] = &[
            "connection",
            "transfer-encoding",
            "te",
            "trailer",
            "upgrade",
        ];
        let mut res_headers = HeaderMap::new();
        for (name, value) in resp.headers() {
            let name_str = name.as_str().to_lowercase();
            if blocked_res_headers.contains(&name_str.as_str()) {
                continue;
            }
            if let (Ok(n), Ok(v)) = (
                HeaderName::from_bytes(name.as_str().as_bytes()),
                HeaderValue::from_bytes(value.as_bytes()),
            ) {
                res_headers.insert(n, v);
            }
        }

        // レスポンス Body をストリームとして axum Body に変換
        // これにより SSE（Server-Sent Events）のストリーミングが透過的に機能する
        let bytes_stream = resp.bytes_stream();
        let axum_body = Body::from_stream(bytes_stream);

        // axum::response::Response を組み立てて返す
        let mut builder = Response::builder().status(status);
        if let Some(headers_mut) = builder.headers_mut() {
            *headers_mut = res_headers;
        }
        builder.body(axum_body).map_err(|e| {
            log::error!("<LMGW> Failed to build response: {}", e);
            ApiError::new_system(StatusCode::INTERNAL_SERVER_ERROR, rterr::ERR_UNEXPECTED, e.to_string())
        })
    }
}
