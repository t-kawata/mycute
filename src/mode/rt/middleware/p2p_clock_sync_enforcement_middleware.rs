use crate::constants::{
    HEADER_X_MYCUTE_CA_BASE_URL, HEADER_X_MYCUTE_SENDER_PUBKEY, HEADER_X_MYCUTE_SIGNATURE,
    HEADER_X_MYCUTE_TIMESTAMP, P2P_BLACKLIST_SYNC_TARGET_MAX, TAG_MARKER_P2P_OPTIONAL,
    TAG_MARKER_P2P_STRICT, TIMESTAMP_TOLERANCE_MS,
};
use crate::mode::rt::client::secure_client::SecureClient;
use crate::mode::rt::rtbl::{blacklists_bl, identities_bl};
use crate::mode::rt::rterr::rterr::{ERR_AUTH, ERR_BLACKLISTED, ERR_TIME_SKEW};
use crate::mode::rt::rtres::errs_res::ApiError;
use crate::mode::rt::rtutils::db_for_rt::DbPoolsExt;
use crate::stt_config::ConfigManager;
use crate::utils::crypto::Ed448Signature;
use crate::utils::db::DbPools;
use crate::utils::time;
use axum::{
    body::Body,
    http::{HeaderValue, Request, Response, StatusCode},
    middleware::Next,
    response::IntoResponse,
    Extension,
};
use rand::prelude::IndexedRandom;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::OnceLock;
use utoipa::OpenApi;

static P2P_PATH_SETS: OnceLock<(HashSet<String>, HashSet<String>)> = OnceLock::new();

/// パスリストを取得（初回のみ生成）
fn get_p2p_path_sets() -> (&'static HashSet<String>, &'static HashSet<String>) {
    let (strict, optional) = P2P_PATH_SETS.get_or_init(|| {
        let mut strict_set = HashSet::new();
        let mut optional_set = HashSet::new();
        let binding = crate::mode::rt::req_map::ApiDoc::openapi();
        let openapi = crate::mode::rt::req_map::RUNTIME_OPENAPI
            .get()
            .unwrap_or(&binding);

        for (path, path_item) in &openapi.paths.paths {
            let ops = [
                &path_item.get,
                &path_item.post,
                &path_item.put,
                &path_item.delete,
                &path_item.options,
                &path_item.head,
                &path_item.patch,
                &path_item.trace,
            ];
            for op_opt in ops {
                if let Some(operation) = op_opt {
                    if let Some(tags) = &operation.tags {
                        let prefix = path.split('{').next().unwrap_or(&path);
                        if tags
                            .iter()
                            .any(|t: &String| t.contains(TAG_MARKER_P2P_STRICT))
                        {
                            strict_set.insert(prefix.to_string());
                        } else if tags
                            .iter()
                            .any(|t: &String| t.contains(TAG_MARKER_P2P_OPTIONAL))
                        {
                            optional_set.insert(prefix.to_string());
                        }
                    }
                }
            }
        }
        (strict_set, optional_set)
    });

    (strict, optional)
}

/// P2P クロック同期強制ミドルウェア
/// ネットワーク全体で時刻が一致していることを強制し、乖離がある場合はブラックリスト化する。
/// UI（JWTロール: USR, VDR等）からのリクエストは対象外とし、バイパスする。
pub async fn p2p_clock_sync_enforcement_middleware(
    Extension(db): Extension<Arc<DbPools>>,
    Extension(config): Extension<Arc<ConfigManager>>,
    Extension(client): Extension<Arc<SecureClient>>,
    req: Request<Body>,
    next: Next,
) -> Response<Body> {
    // 1. パスセットを取得 (動的抽出済みの STRICT/OPTIONAL 一覧)
    let (strict_set, optional_set) = get_p2p_path_sets();
    let path = req.uri().path();

    // 2. セキュリティ分類に従った判定 (動的リストを使用)
    let is_strict = strict_set.iter().any(|prefix| path.starts_with(prefix));
    let is_optional = optional_set.iter().any(|prefix| path.starts_with(prefix));

    if !is_strict && !is_optional {
        // 3. それ以外の一般パスは一律バイパス (動的タグが付与されていない API)
        return next.run(req).await;
    }

    // ここから P2P 検証対象パス (STRICT or OPTIONAL)
    // 検証に必要な4つのヘッダーをここでのみ取得する (Lazy Extraction)
    let headers = req.headers();
    let h_ts = headers.get(HEADER_X_MYCUTE_TIMESTAMP);
    let h_sig = headers.get(HEADER_X_MYCUTE_SIGNATURE);
    let h_pub = headers.get(HEADER_X_MYCUTE_SENDER_PUBKEY);
    let h_ca = headers.get(HEADER_X_MYCUTE_CA_BASE_URL);

    if is_strict {
        // 2.1 STRICT カテゴリ: ヘッダーが一つでも欠けていればエラー
        if h_ts.is_none() || h_sig.is_none() || h_pub.is_none() || h_ca.is_none() {
            log::warn!(
                "P2P Middleware: Missing mandatory headers on STRICT path: {}",
                path
            );
            return ApiError::new_system(
                StatusCode::BAD_REQUEST,
                ERR_AUTH,
                "Missing mandatory P2P headers on protocol endpoint".to_string(),
            )
            .into_response();
        }
        // ヘッダーが揃っている場合は、後半の検証ロジックへ。
    } else if is_optional {
        // 2.2 OPTIONAL カテゴリ: ヘッダーが揃っていれば検証、欠けていればバイパス
        if h_ts.is_none() || h_sig.is_none() || h_pub.is_none() || h_ca.is_none() {
            return next.run(req).await;
        }
    }

    // 2. Request Inspection (入り口点検 & 自動取り込み)
    let my_ts = time::now_ts_ms() as i64;

    // 2.1 データの抽出 (存在チェック済み)
    let peer_ts_str = h_ts.unwrap().to_str().unwrap_or("0");
    let peer_sig_hex = h_sig.unwrap().to_str().unwrap_or("");
    let peer_pub_hex = h_pub.unwrap().to_str().unwrap_or("");
    let peer_ca_url = h_ca.unwrap().to_str().unwrap_or("");
    let peer_ts = peer_ts_str.parse::<i64>().unwrap_or(0);

    // 2.2 自己執行 (Self-Enforcement) & ブラックリストチェック
    // まず自分自身がブラックリストに入っていないか確認 (自殺スイッチ)
    if let Ok(keypair) = config.get_node_keypair() {
        let my_pub_hex = hex::encode(keypair.public);
        if let Ok(true) = blacklists_bl::is_blacklisted(&db, &my_pub_hex).await {
            log::warn!("P2P Clock Sync Enforcement: SELF-ENFORCEMENT ACTIVATED. I am blacklisted. Shutting down request.");
            let err_res = ApiError::new_system(
                StatusCode::FORBIDDEN,
                ERR_BLACKLISTED,
                "I am dead (Self-Enforcement).",
            )
            .into_response();
            return inject_headers(err_res, &config, &db).await;
        }
    }

    // 2.3 CA BASE URL 同期 (非同期実行)
    // 相手が提示してきた CA BASE URL を使って、その CA のブラックリストを同期する
    // リスト形式 (カンマ区切り) で受け取り、規定数だけランダムに選んで同期する
    {
        let peer_ca_url_list: Vec<String> = peer_ca_url
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        if !peer_ca_url_list.is_empty() {
            let mut rng = rand::rng();
            let selected_urls: Vec<String> = peer_ca_url_list
                .choose_multiple(&mut rng, P2P_BLACKLIST_SYNC_TARGET_MAX)
                .cloned()
                .collect();

            let db_clone = db.clone();
            let config_clone = config.clone();
            let client_clone = client.clone();

            tokio::spawn(async move {
                for url in selected_urls {
                    if let Err(e) = blacklists_bl::sync_blacklists_with_ca(
                        &db_clone,
                        &url,
                        &client_clone,
                        &config_clone,
                    )
                    .await
                    {
                        log::warn!(
                            "P2P Clock Sync Enforcement: Background sync with CA {} failed: {}",
                            url,
                            e.to_string()
                        );
                    }
                }
            });
        }
    }

    // 2.4 P2P 検証 (ヘッダーが揃っている前提)
    {
        // 2.5 送信元がブラックリストに入っていないかチェック (他殺スイッチ)
        if let Ok(true) = blacklists_bl::is_blacklisted(&db, peer_pub_hex).await {
            log::warn!(
                "P2P Clock Sync Enforcement: Disconnected request from blacklisted node: {}",
                peer_pub_hex
            );
            let err_res: axum::response::Response = ApiError::new_system(
                StatusCode::FORBIDDEN,
                ERR_BLACKLISTED,
                "You are blacklisted.",
            )
            .into_response();
            return inject_headers(err_res, &config, &db).await;
        }

        // 署名検証用の証拠構築
        let evidence = blacklists_bl::CrimeEvidence {
            detail: blacklists_bl::CrimeDetail::TimestampFraud {
                wrong_timestamp: peer_ts,
                time_diff_ms: 0, // あとで埋める（または検証用には不要な場合も）
            },
            target_pubkey: peer_pub_hex.to_string(),
            observed_at: my_ts, // 観測時刻 = 自分の現在時刻 (判決日)
            signature: peer_sig_hex.to_string(),
            signed_payload: hex::encode(peer_ts.to_be_bytes()), // 現状はタイムスタンプのBigEndian表現
        };

        if let Err(e) = blacklists_bl::check_evidence_structure(&evidence) {
            log::warn!(
                "P2P Clock Sync Enforcement: Invalid signature from {}. Error: {}",
                peer_pub_hex,
                e
            );
            let response: axum::response::Response = axum::response::IntoResponse::into_response(e);
            return inject_headers(response, &config, &db).await;
        }

        // 2.6 時刻乖離チェック
        let diff = (my_ts - peer_ts).abs();

        if diff > TIMESTAMP_TOLERANCE_MS {
            // タイムスタンプの誤差が許容範囲を超えていたら
            log::warn!(
                "P2P Clock Sync Enforcement: Time skew detected. Peer: {}, My: {}, Diff: {}",
                peer_ts,
                my_ts,
                diff
            );

            // 執行 (Enforcement): ローカル DB にブラックリスト追加
            let evidence_to_save = blacklists_bl::CrimeEvidence {
                detail: blacklists_bl::CrimeDetail::TimestampFraud {
                    wrong_timestamp: peer_ts,
                    time_diff_ms: diff,
                },
                target_pubkey: peer_pub_hex.to_string(),
                observed_at: my_ts,
                signature: peer_sig_hex.to_string(),
                signed_payload: hex::encode(peer_ts.to_be_bytes()),
            };
            let _ = blacklists_bl::add_to_blacklist(&db, evidence_to_save.clone()).await;

            // 司法 (Judgment): L3 ノードのみ CA へ報告 (Broadcast)
            {
                let db_clone = db.clone();
                let config_clone = config.clone();
                let client_clone = client.clone();
                let evidence_clone = evidence_to_save;

                // 相手が申告した CA URL リストを取得
                let sender_ca_urls: Option<Vec<String>> =
                    h_ca.and_then(|v| v.to_str().ok()).map(|s| {
                        s.split(',')
                            .map(|u| u.trim().to_string())
                            .filter(|u| !u.is_empty())
                            .collect()
                    });

                tokio::spawn(async move {
                    if let Err(e) = blacklists_bl::report_crime_broadcast(
                        &db_clone,
                        &config_clone,
                        &client_clone,
                        &evidence_clone,
                        sender_ca_urls,
                    )
                    .await
                    {
                        log::warn!("P2P Clock Sync Enforcement: Broadcast report failed: {}", e);
                    }
                });
            }

            let err_res = ApiError::new_system(
                StatusCode::FORBIDDEN,
                ERR_TIME_SKEW,
                format!("Time skew too large. Diff: {}ms", diff),
            )
            .into_response();

            return inject_headers(err_res, &config, &db).await;
        }
    }

    // 3. Inner Handler Execution
    let res = next.run(req).await;

    // 4. Response Injection (正常系レスポンスへのヘッダー付与)
    inject_headers(res, &config, &db).await
}

/// レスポンスに時刻、署名、ブラックリストヘッダーを付与するヘルパー
async fn inject_headers(
    mut res: Response<Body>,
    config: &Arc<ConfigManager>,
    db: &Arc<DbPools>,
) -> Response<Body> {
    let my_ts = time::now_ts_ms();
    let headers = res.headers_mut();

    // 自時刻の付与
    if let Ok(v) = HeaderValue::from_str(&my_ts.to_string()) {
        headers.insert(HEADER_X_MYCUTE_TIMESTAMP, v);
    }

    // 署名の付与
    if let Ok(keypair) = config.get_node_keypair() {
        let msg = my_ts.to_be_bytes();
        // anyhow::Result を介した後の型推論を補助するため、明示的に Ed448Signature を期待する
        if let Ok(sig) = keypair.sign(&msg) {
            let sig: Ed448Signature = sig;
            if let Ok(v) = HeaderValue::from_str(&hex::encode(sig.signature)) {
                headers.insert(HEADER_X_MYCUTE_SIGNATURE, v);
            }
            if let Ok(v) = HeaderValue::from_str(&hex::encode(keypair.public)) {
                headers.insert(HEADER_X_MYCUTE_SENDER_PUBKEY, v);
            }
        }
    }

    // CA BASE URL の付与 (Async)
    if let Ok(conn) = db.get_ro_for_rt() {
        if let Some(urls) = identities_bl::get_reliable_ca_urls(conn, config).await {
            // リストをカンマ区切り文字列に変換してヘッダーにセット
            let joined_urls = urls.join(",");
            if let Ok(v) = HeaderValue::from_str(&joined_urls) {
                headers.insert(HEADER_X_MYCUTE_CA_BASE_URL, v);
            }
        }
    }

    res
}
