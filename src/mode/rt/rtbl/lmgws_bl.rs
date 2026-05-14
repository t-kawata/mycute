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
    entities::{lmgw_providers, prelude::*},
    mode::rt::{
        rterr::rterr,
        rtreq::lmgws_req::SaveLmgwProvidersReq,
        rtres::{
            errs_res::ApiError,
            lmgws_res::{GetLmgwProvidersRes, ManageLmgwProviderRes},
        },
    },
    mycute_settings::ConfigManager,
    utils::crypto,
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter, Set, ModelTrait,
};
use serde_json::{json, Value};

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

    /// Bifrost にプロバイダー設定を同期する。
    ///
    /// Bifrost v1.4.24 では PUT /api/providers/{name} に重複キーバグ（500 "already exists"）があるため、
    /// DELETE で既存設定を削除してから POST で再作成するワークアラウンドを採用している。
    ///
    /// # 送信する JSON 形式
    /// Bifrost が期待する形式:
    /// ```json
    /// {"name": "openai", "provider": "openai", "keys": [...]}
    /// ```
    pub async fn sync_provider(
        &self,
        provider_name: &str,
        keys_config: &serde_json::Value,
    ) -> Result<(), ApiError> {
        let base = self.get_base_url();
        let secret = self.get_secret();

        let delete_url = format!("{}/api/providers/{}", base, provider_name);
        let post_url = format!("{}/api/providers", base);

        let auth_header = format!("Bearer {}", secret);

        // 1. DELETE 既存設定（存在しなくてもエラーにしない）
        let delete_resp = self
            .hc
            .delete(&delete_url)
            .header(reqwest::header::AUTHORIZATION, &auth_header)
            .send()
            .await
            .map_err(|e| {
                log::error!("<LMGW> DELETE request failed: {}", e);
                ApiError::new_system(
                    StatusCode::BAD_GATEWAY,
                    rterr::ERR_UNEXPECTED,
                    format!("Failed to connect to Bifrost for DELETE: {}", e),
                )
            })?;

        let delete_status = delete_resp.status();
        if !delete_status.is_success() && delete_status != reqwest::StatusCode::NOT_FOUND {
            let body = delete_resp.text().await.unwrap_or_default();
            log::error!("<LMGW> DELETE returned unexpected status {}: {}", delete_status, body);
            return Err(ApiError::new_system(
                StatusCode::BAD_GATEWAY,
                rterr::ERR_UNEXPECTED,
                format!("Bifrost DELETE failed ({}): {}", delete_status, body),
            ));
        }

        // 2. POST で正しい形式で作成
        let body = serde_json::json!({
            "name": provider_name,
            "provider": provider_name,
            "keys": keys_config.get("keys").cloned().unwrap_or(serde_json::json!([])),
        });

        let post_resp = self
            .hc
            .post(&post_url)
            .header(reqwest::header::AUTHORIZATION, &auth_header)
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                log::error!("<LMGW> POST request failed: {}", e);
                ApiError::new_system(
                    StatusCode::BAD_GATEWAY,
                    rterr::ERR_UNEXPECTED,
                    format!("Failed to connect to Bifrost for POST: {}", e),
                )
            })?;

        let post_status = post_resp.status();
        if !post_status.is_success() {
            let resp_body = post_resp.text().await.unwrap_or_default();
            log::error!("<LMGW> Bifrost returned error {}: {}", post_status, resp_body);
            return Err(ApiError::new_system(
                StatusCode::BAD_GATEWAY,
                rterr::ERR_UNEXPECTED,
                format!("Bifrost returned {}: {}", post_status, resp_body),
            ));
        }

        Ok(())
    }
}

pub async fn get_lmgw_providers(
    conn: &DatabaseConnection,
    apx_id: u32,
    vdr_id: u32,
) -> Result<GetLmgwProvidersRes, ApiError> {
    let providers = LmgwProviders::find()
        .filter(lmgw_providers::Column::ApxId.eq(apx_id as i32))
        .filter(lmgw_providers::Column::VdrId.eq(vdr_id as i32))
        .all(conn)
        .await
        .map_err(|e| {
            ApiError::new_system(
                StatusCode::INTERNAL_SERVER_ERROR,
                rterr::ERR_DATABASE,
                format!("Failed to fetch lmgw providers: {}", e),
            )
        })?;

    let mut res_providers = Vec::new();
    for p in providers {
        res_providers.push(ManageLmgwProviderRes {
            provider_name: p.provider_name,
            config_json: p.config_json,
        });
    }

    Ok(GetLmgwProvidersRes {
        providers: res_providers,
    })
}

pub async fn save_lmgw_providers(
    conn: &DatabaseConnection,
    apx_id: u32,
    vdr_id: u32,
    req: SaveLmgwProvidersReq,
    hc: Arc<Client>,
    config_manager: Arc<ConfigManager>,
) -> Result<(), ApiError> {
    let rt_crypto_key = config_manager.settings.read().server.rt_crypto_key.clone();
    if rt_crypto_key.is_empty() {
        return Err(ApiError::new_system(
            StatusCode::INTERNAL_SERVER_ERROR,
            rterr::ERR_UNEXPECTED,
            "RT_CRYPTO_KEY is empty in ServerSettings".to_string(),
        ));
    }

    let client = BifrostClient::new(hc.clone(), config_manager.clone());

    for provider_req in req.providers {
        let mut config: Value = serde_json::from_str(&provider_req.config_json).map_err(|e| {
            ApiError::new_system(
                StatusCode::BAD_REQUEST,
                rterr::ERR_VALIDATION,
                format!("Invalid config JSON: {}", e),
            )
        })?;

        let mut plaintext_config = config.clone();
        if let Some(keys) = config.get_mut("keys").and_then(|v| v.as_array_mut()) {
            let mut plaintext_keys = Vec::new();
            for key_obj in keys.iter_mut() {
                if let Some(obj) = key_obj.as_object_mut() {
                    let mut is_new = false;
                    if let Some(is_new_val) = obj.get("is_new") {
                        is_new = is_new_val.as_bool().unwrap_or(false);
                    }
                    obj.remove("is_new");

                    let val_str = obj.get("value").and_then(|v| v.as_str()).unwrap_or("");

                    let plain_val;
                    let enc_val;

                    if is_new {
                        plain_val = val_str.to_string();
                        enc_val = crypto::encrypt(&plain_val, &rt_crypto_key).map_err(|e| {
                            ApiError::new_system(
                                StatusCode::INTERNAL_SERVER_ERROR,
                                    rterr::ERR_UNEXPECTED,
                                format!("Failed to encrypt key: {}", e),
                            )
                        })?;
                    } else {
                        enc_val = val_str.to_string();
                        plain_val = crypto::decrypt(&enc_val, &rt_crypto_key).map_err(|e| {
                            ApiError::new_system(
                                StatusCode::INTERNAL_SERVER_ERROR,
                                    rterr::ERR_UNEXPECTED,
                                format!("Failed to decrypt key: {}", e),
                            )
                        })?;
                    }

                    obj.insert("value".to_string(), json!(enc_val));

                    let mut plain_obj = obj.clone();
                    plain_obj.insert("value".to_string(), json!(plain_val));
                    plaintext_keys.push(Value::Object(plain_obj));
                }
            }
            if let Some(plain_keys) = plaintext_config.get_mut("keys") {
                *plain_keys = Value::Array(plaintext_keys);
            }
        }

        let db_json_str = serde_json::to_string(&config).unwrap_or_default();
        
        let existing = LmgwProviders::find()
            .filter(lmgw_providers::Column::ApxId.eq(apx_id as i32))
            .filter(lmgw_providers::Column::VdrId.eq(vdr_id as i32))
            .filter(lmgw_providers::Column::ProviderName.eq(&provider_req.provider_name))
            .one(conn)
            .await
            .map_err(|e| {
                ApiError::new_system(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    rterr::ERR_DATABASE,
                    format!("Failed to find provider: {}", e),
                )
            })?;

        if let Some(record) = existing {
            let mut am: lmgw_providers::ActiveModel = record.into_active_model();
            am.config_json = Set(db_json_str);
            let _: lmgw_providers::Model = am.update(conn).await.map_err(|e| {
                ApiError::new_system(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    rterr::ERR_DATABASE,
                    format!("Failed to update provider: {}", e),
                )
            })?;
        } else {
            let am = lmgw_providers::ActiveModel {
                apx_id: Set(apx_id as i32),
                vdr_id: Set(vdr_id as i32),
                provider_name: Set(provider_req.provider_name.clone()),
                config_json: Set(db_json_str),
                ..Default::default()
            };
            let _: lmgw_providers::Model = am.insert(conn).await.map_err(|e| {
                ApiError::new_system(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    rterr::ERR_DATABASE,
                    format!("Failed to insert provider: {}", e),
                )
            })?;
        }

        client.sync_provider(&provider_req.provider_name, &plaintext_config).await?;
    }

    Ok(())
}

pub async fn delete_lmgw_provider(
    conn: &DatabaseConnection,
    apx_id: u32,
    vdr_id: u32,
    provider_name: &str,
    hc: Arc<Client>,
    config_manager: Arc<ConfigManager>,
) -> Result<(), ApiError> {
    // 1. DBから削除
    let existing = LmgwProviders::find()
        .filter(lmgw_providers::Column::ApxId.eq(apx_id as i32))
        .filter(lmgw_providers::Column::VdrId.eq(vdr_id as i32))
        .filter(lmgw_providers::Column::ProviderName.eq(provider_name))
        .one(conn)
        .await
        .map_err(|e| {
            ApiError::new_system(
                StatusCode::INTERNAL_SERVER_ERROR,
                rterr::ERR_DATABASE,
                format!("Failed to find provider to delete: {}", e),
            )
        })?;

    if let Some(record) = existing {
        record.delete(conn).await.map_err(|e| {
            ApiError::new_system(
                StatusCode::INTERNAL_SERVER_ERROR,
                rterr::ERR_DATABASE,
                format!("Failed to delete provider: {}", e),
            )
        })?;
    }

    // 2. Bifrost への削除リクエスト転送
    let client = BifrostClient::new(hc.clone(), config_manager.clone());
    let delete_path = format!("api/providers/{}", provider_name);
    
    // ボディは空
    client.proxy_lmgw_request(Method::DELETE, &delete_path, HeaderMap::new(), Body::empty()).await?;

    Ok(())
}

/// 起動時に DB 内の全 LLM プロバイダー設定を Bifrost に同期する。
///
/// Bifrost の config.sqlite が失われた場合や前回の実行からプロセスが再起動された場合でも、
/// DB を正本として Bifrost の状態を再構築する。
/// エラーはログに記録するのみで、呼び出し元の起動処理をブロックしない。
pub async fn sync_all_providers_to_bifrost_on_startup(
    conn: &DatabaseConnection,
    hc: Arc<Client>,
    config_manager: Arc<ConfigManager>,
) {
    let providers = match LmgwProviders::find().all(conn).await {
        Ok(p) => p,
        Err(e) => {
            log::error!("<LMGW> Failed to read providers for startup sync: {}", e);
            return;
        }
    };

    if providers.is_empty() {
        log::info!("<LMGW> No providers to sync on startup.");
        return;
    }

    let rt_crypto_key = config_manager.settings.read().server.rt_crypto_key.clone();
    if rt_crypto_key.is_empty() {
        log::error!("<LMGW> Cannot sync providers on startup: rt_crypto_key is empty");
        return;
    }

    let client = BifrostClient::new(hc, config_manager.clone());
    let mut sync_count = 0usize;

    for provider in &providers {
        let mut plaintext_config: Value = match serde_json::from_str(&provider.config_json) {
            Ok(v) => v,
            Err(e) => {
                log::error!(
                    "<LMGW> Failed to parse config_json for provider '{}': {}",
                    provider.provider_name, e
                );
                continue;
            }
        };

        // DB 内の暗号化されたキー値を復号する（Bifrost へは平文で送る必要がある）
        if let Some(keys) = plaintext_config.get_mut("keys").and_then(|v| v.as_array_mut()) {
            for key_obj in keys.iter_mut() {
                if let Some(obj) = key_obj.as_object_mut() {
                    let val_str = obj.get("value").and_then(|v| v.as_str()).unwrap_or("");
                    if val_str.is_empty() {
                        continue;
                    }
                    match crypto::decrypt(val_str, &rt_crypto_key) {
                        Ok(plain) => {
                            obj.insert("value".to_string(), json!(plain));
                        }
                        Err(e) => {
                            log::error!(
                                "<LMGW> Failed to decrypt key for provider '{}': {}",
                                provider.provider_name, e
                            );
                            // 復号できないキーは空文字にして Bifrost に送る
                            obj.insert("value".to_string(), json!(""));
                        }
                    }
                }
            }
        }

        match client.sync_provider(&provider.provider_name, &plaintext_config).await {
            Ok(_) => {
                sync_count += 1;
                log::info!(
                    "<LMGW> Synced provider '{}' to Bifrost on startup.",
                    provider.provider_name
                );
            }
            Err(e) => {
                log::error!(
                    "<LMGW> Failed to sync provider '{}' on startup: {}",
                    provider.provider_name, e
                );
            }
        }
    }

    log::info!(
        "<LMGW> Startup sync complete: {}/{} providers synced to Bifrost.",
        sync_count,
        providers.len()
    );
}

/// 指定されたプロバイダー群を Bifrost から削除する（リセット時など）。
/// エラーはログに記録するのみで、呼び出し元の処理をブロックしない。
///
/// proxy_lmgw_request は Axum の streaming Body に変換するため、
/// ファイア＆フォーゲットの DELETE 用途には不適切（ボディ未消費でコネクションが
/// プールに戻らない）。代わりに reqwest Client を直接使用する。
pub async fn delete_bifrost_providers(
    hc: Arc<Client>,
    config_manager: Arc<ConfigManager>,
    provider_names: &[String],
) {
    let port = config_manager.settings.read().server.bifrost_port;
    let base_url = format!("http://{IP_LOCALHOST}:{port}");
    let secret = config_manager.get_lmgw_secret().unwrap_or_default();
    let auth_header = format!("Bearer {}", secret);

    for name in provider_names {
        let url = format!("{}/api/providers/{}", base_url, name);
        match hc
            .delete(&url)
            .header(reqwest::header::AUTHORIZATION, &auth_header)
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status();
                // レスポンスボディを明示的に消費してコネクションをプールに戻す
                let _body = resp.text().await.unwrap_or_default();
                if status.is_success() || status == reqwest::StatusCode::NOT_FOUND {
                    log::info!("<LMGW> Deleted provider '{}' from Bifrost.", name);
                } else {
                    log::warn!(
                        "<LMGW> DELETE provider '{}' returned unexpected status {}: {}",
                        name, status, _body
                    );
                }
            }
            Err(e) => {
                log::warn!(
                    "<LMGW> Failed to delete provider '{}' from Bifrost: {}",
                    name, e
                );
            }
        }
    }
}
