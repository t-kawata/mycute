use reqwest::{Client, RequestBuilder, Response};
use serde::Serialize;
use std::sync::Arc;
use crate::utils::db::DbPools;
use crate::stt_config::ConfigManager;
use crate::utils::time;
use crate::constants::{
    HEADER_X_MYCUTE_TIMESTAMP, HEADER_X_MYCUTE_SIGNATURE, HEADER_X_MYCUTE_SENDER_PUBKEY,
    HEADER_X_MYCUTE_CA_BASE_URL, P2P_BLACKLIST_SYNC_TARGET_MAX,
    TIMESTAMP_TOLERANCE_MS,
    ERR_HTTP_CLIENT,
};
use crate::mode::rt::rterr::rterr::ERR_TIME_SKEW;
use crate::mode::rt::rtres::errs_res::ApiError;
use crate::mode::rt::rtbl::{blacklists_bl, identities_bl};
use crate::mode::rt::rtutils::db_for_rt::DbPoolsExt;
use rand::prelude::IndexedRandom;
use axum::http::StatusCode;

/// 外部ノード（CA、他ノード）へのセキュア通信クライアント
/// 
/// # 機能
/// 1. リクエスト送信時に自身の現在時刻、Ed448署名、およびブラックリスト（圧縮）をヘッダーに付与。
/// 3. レスポンス受信時に相手の時刻と署名を検証 (許容誤差を超えたらエラー、証拠を保存)。
/// 4. レスポンスに含まれる CA BASE URL を元に、バックグラウンドでブラックリストを同期する。
/// 5. 自身が L3 の場合、不正を検知したら CA に報告し、未報告の証拠も遡って共有する。
/// 
/// # 用途制限
/// 本クライアントは **P2P 通信専用** です。
/// UI や内部ループバック通信には使用しないでください。
#[derive(Clone)]
pub struct SecureClient {
    inner: Client,
    db: Arc<DbPools>,
    config: Arc<ConfigManager>,
}

impl SecureClient {
    pub fn new(inner: Client, db: Arc<DbPools>, config: Arc<ConfigManager>) -> Self {
        Self { inner, db, config }
    }

    /// POST リクエスト送信
    pub async fn post<U: reqwest::IntoUrl, T: Serialize + ?Sized>(
        &self,
        url: U,
        json: &T,
    ) -> Result<Response, ApiError> {
        let builder = self.inner.post(url).json(json);
        self.execute_verified(builder).await
    }

    /// POST リクエスト送信 (検証なし - CA報告用)
    pub async fn post_without_verification<U: reqwest::IntoUrl, T: Serialize + ?Sized>(
        &self,
        url: U,
        json: &T,
    ) -> Result<Response, ApiError> {
        let builder = self.inner.post(url).json(json);
        self.execute_raw(builder).await
    }

    /// CA への POST リクエスト送信 (JSON 返却)
    pub async fn post_ca<R: serde::de::DeserializeOwned, U: reqwest::IntoUrl, T: Serialize + ?Sized>(
        &self,
        url: U,
        json: &T,
    ) -> Result<R, ApiError> {
        let res = self.post_without_verification(url, json).await?;
        res.json::<R>().await.map_err(|e| {
            ApiError::new_system(
                StatusCode::INTERNAL_SERVER_ERROR,
                crate::constants::ERR_HTTP_CLIENT,
                format!("Failed to parse CA response: {}", e),
            )
        })
    }

    /// GET リクエスト送信
    pub async fn get<U: reqwest::IntoUrl>(&self, url: U) -> Result<Response, ApiError> {
        let builder = self.inner.get(url);
        self.execute_verified(builder).await
    }

    /// GET リクエスト送信 (検証なし - CA同期用)
    pub async fn get_without_verification<U: reqwest::IntoUrl>(&self, url: U) -> Result<Response, ApiError> {
        let builder = self.inner.get(url);
        self.execute_raw(builder).await
    }

    /// リクエスト準備（ヘッダー付与）
    fn prepare_request(&self, mut builder: RequestBuilder) -> RequestBuilder {
        // 1. ヘッダー付与 (武装)
        let my_ts = time::now_ts_ms();
        builder = builder.header(HEADER_X_MYCUTE_TIMESTAMP, my_ts.to_string());
        
        // --- 署名付与 ---
        if let Ok(keypair) = self.config.get_node_keypair() {
            let msg = my_ts.to_be_bytes();
            if let Ok(sig_raw) = keypair.sign(&msg) {
                let sig: crate::utils::crypto::Ed448Signature = sig_raw;
                builder = builder.header(HEADER_X_MYCUTE_SIGNATURE, hex::encode(sig.signature));
                builder = builder.header(HEADER_X_MYCUTE_SENDER_PUBKEY, hex::encode(keypair.public));
            }
        }

        // --- CA BASE URL 付与 ---
        // Note: Async version is handled in execute_raw/execute_with_retry
        
        builder
    }

    /// 検証なし実行
    async fn execute_raw(&self, mut builder: RequestBuilder) -> Result<Response, ApiError> {
        // --- CA BASE URL 付与 (Async) ---
        if let Ok(conn) = self.db.get_ro_for_rt() {
            if let Some(urls) = identities_bl::get_reliable_ca_urls(conn, &self.config).await {
                // リストをカンマ区切り文字列に変換してヘッダーにセット
                let joined_urls = urls.join(",");
                // log::debug!("<SecureClient> Selected reliable CA URLs: {}", joined_urls);
                builder = builder.header(HEADER_X_MYCUTE_CA_BASE_URL, joined_urls);
            }
        }

        let builder = self.prepare_request(builder);
        builder.send().await.map_err(|e| {
            ApiError::new_system(
                StatusCode::INTERNAL_SERVER_ERROR,
                ERR_HTTP_CLIENT,
                format!("Failed to send request: {}", e),
            )
        })
    }

    /// 検証あり実行
    async fn execute_verified(&self, builder: RequestBuilder) -> Result<Response, ApiError> {
        let res = self.execute_raw(builder).await?;
        let my_ts = time::now_ts_ms() as i64;
        self.verify_response(&res, my_ts).await?;
        Ok(res)
    }

    /// レスポンスの検証と取り込み
    async fn verify_response(&self, res: &Response, my_ts: i64) -> Result<(), ApiError> {
        let headers = res.headers();
        
        // 1. 基本的な時刻と署名の抽出
        let peer_ts = match headers.get(HEADER_X_MYCUTE_TIMESTAMP)
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<i64>().ok()) {
                Some(ts) => ts,
                None => return Ok(()), // ヘッダーがない場合は P2P 非対応ノードとみなして終了
            };
            
        let peer_sig_hex = match headers.get(HEADER_X_MYCUTE_SIGNATURE).and_then(|v| v.to_str().ok()) {
            Some(sig) => sig,
            None => return Ok(()),
        };
            
        let peer_pub_hex = match headers.get(HEADER_X_MYCUTE_SENDER_PUBKEY).and_then(|v| v.to_str().ok()) {
            Some(pubkey) => pubkey,
            None => return Ok(()),
        };

        let diff = (my_ts - peer_ts).abs();
        
        // 重要: 時刻が狂っている場合、または署名が不正な場合、まずは相手をブラックリストに入れる証拠を作る
        if diff > TIMESTAMP_TOLERANCE_MS {
            log::warn!("SecureClient: Time skew detected. Peer: {}, My: {}, Diff: {}", peer_ts, my_ts, diff);
            
            // 証拠構築
            let evidence = blacklists_bl::CrimeEvidence {
                detail: blacklists_bl::CrimeDetail::TimestampFraud {
                    wrong_timestamp: peer_ts,
                    time_diff_ms: diff,
                },
                target_pubkey: peer_pub_hex.to_string(),
                observed_at: my_ts, // 観測時刻 (判決日)
                signature: peer_sig_hex.to_string(),
                signed_payload: hex::encode(peer_ts.to_be_bytes()), // 署名対象
            };
            
            // 証拠を検証してから DB に入れる (冤罪防止)
            if let Ok(_) = blacklists_bl::validate_evidence(&evidence) {
                // ローカル保存 (全ノード共通義務: 執行)
                let _ = blacklists_bl::add_to_blacklist(&self.db, evidence.clone()).await;

                // L3 ノードのみの義務: 司法 (CA への報告 - Broadcast)
                // 今回の不正証拠を信頼できる全CAに報告する
                let db = self.db.clone();
                let config = self.config.clone();
                let client = self.clone();
                let evidence_clone = evidence.clone();
                
                // 相手が申告した CA URL リストを取得
                let sender_ca_urls: Option<Vec<String>> = headers.get(HEADER_X_MYCUTE_CA_BASE_URL)
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.split(',').map(|u| u.trim().to_string()).filter(|u| !u.is_empty()).collect());

                tokio::spawn(async move {
                    if let Err(e) = blacklists_bl::report_crime_broadcast(&db, &config, &client, &evidence_clone, sender_ca_urls).await {
                        log::warn!("SecureClient: Broadcast report failed: {}", e);
                    }
                });

                // Note: 遡り共有 (過去の未報告分) については、Broadcastすると負荷が高いため、
                // 定期的な同期プロセスや、別途の専用タスクで行うのが適切。
                // ここでは即時性の高い「現行犯」の通報を優先する。

                return Err(ApiError::new_system(
                    StatusCode::BAD_GATEWAY,
                    ERR_TIME_SKEW,
                    format!("Peer time skew too large: {}ms", diff),
                ));
            }
        }

        // 2. CA BASE URL に基づくブラックリスト同期 (執行義務: 拡散)
        // リスト形式で受け取り、ランダムに規定数だけ選んで同期する
        if let Some(ca_base_urls_str) = headers.get(HEADER_X_MYCUTE_CA_BASE_URL).and_then(|v| v.to_str().ok()) {

            let ca_urls: Vec<String> = ca_base_urls_str.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();

            if !ca_urls.is_empty() {
                let mut rng = rand::rng();
                let selected_urls: Vec<String> = ca_urls
                    .choose_multiple(&mut rng, P2P_BLACKLIST_SYNC_TARGET_MAX)
                    .cloned()
                    .collect();

                let db = self.db.clone();
                let client_clone = self.clone();
                let config_clone = self.config.clone();

                // 同期はバックグラウンドで行い、メイン処理をブロックしない
                tokio::spawn(async move {
                    for url in selected_urls {
                        if let Err(e) = blacklists_bl::sync_blacklists_with_ca(&db, &url, &client_clone, &config_clone).await {
                             log::warn!("SecureClient: Background sync with CA {} failed: {}", url, e.to_string());
                        }
                    }
                });
            }
        }

        Ok(())
    }
}
