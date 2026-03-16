use crate::{
    constants::{
        ED448_KEY_BYTES_LEN, ED448_SIGNATURE_BYTES_LEN, ERR_APP_NOT_FOUND, ERR_INSUFFICIENT_FUNDS,
        ERR_INVALID_KEY, ERR_INVALID_PUBKEY, ERR_INVALID_SIG, ERR_SIGN, ERR_SIG_FAIL,
        ERR_TICKET_PARSE, KEY_TICKET_SIGNATURE, ST_BAD_REQUEST, ST_FORBIDDEN,
        ST_INTERNAL_SERVER_ERROR, ST_NOT_FOUND,
    },
    entities::{apps, ca_vote_allocated_summaries, ca_vote_item_summaries},
    mode::rt::rtbl::{
        blacklists_bl::{add_to_blacklist, report_crime_broadcast, CrimeDetail, CrimeEvidence},
        identities_bl,
    },
    mode::rt::rtutils::voting::{format_vote_payload, format_vote_receipt_payload},
    mode::rt::{
        client::secure_client::SecureClient,
        rterr::rterr,
        rtreq::ca_apps_req::{AdvertiseAppCaReq, DiscoverAppCaReq, VoteAppCaReq},
        rtres::{
            ca_apps_res::{
                AdvertiseAppCaRes, DiscoverAppCaRes, DiscoverAppItemCaRes, VoteAppCaRes,
            },
            errs_res::ApiError,
        },
        rtutils::db_for_rt::DbPoolsExt,
    },
    mycute_settings::ConfigManager,
    utils::{
        crypto::{verify_signature, Ed448Signature},
        db::DbPools,
        time,
    },
};
use chrono::Utc;
use hex;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DbErr, EntityTrait, IntoActiveModel, ModelTrait, QueryFilter,
    QuerySelect, Set, TransactionError, TransactionTrait,
};
use std::sync::Arc;
use uuid::Uuid;

// ------------------------------------------------------------
// アプリへの投票
// ------------------------------------------------------------
pub async fn vote_app_ca(
    db: &DbPools,
    client: &SecureClient,
    app_id: i32,
    req: VoteAppCaReq,
    config_manager: Arc<ConfigManager>,
) -> Result<VoteAppCaRes, ApiError> {
    let conn = db.get_rw_for_rt()?;

    // 1. リクエスト署名の検証 (権限/意図の証明)
    let payload_str = format_vote_payload(
        &req.app_id,
        req.vote,
        req.vote_allocated,
        &req.timestamp.to_string(),
        &req.ticket,
    )?;

    let node_pub_bytes = hex::decode(&req.node_pubkey)
        .map_err(|e| ApiError::new_system(ST_BAD_REQUEST, ERR_INVALID_PUBKEY, e.to_string()))?;
    if node_pub_bytes.len() != ED448_KEY_BYTES_LEN {
        return Err(ApiError::new_system(
            ST_BAD_REQUEST,
            ERR_INVALID_PUBKEY,
            "Invalid node public key length.",
        ));
    }
    let mut node_pub_arr = [0u8; ED448_KEY_BYTES_LEN];
    node_pub_arr.copy_from_slice(&node_pub_bytes);

    let sig_bytes = hex::decode(&req.signature)
        .map_err(|e| ApiError::new_system(ST_BAD_REQUEST, ERR_INVALID_SIG, e.to_string()))?;
    if sig_bytes.len() != ED448_SIGNATURE_BYTES_LEN {
        return Err(ApiError::new_system(
            ST_BAD_REQUEST,
            ERR_INVALID_SIG,
            "Invalid signature length.",
        ));
    }
    let mut sig_arr = [0u8; ED448_SIGNATURE_BYTES_LEN];
    sig_arr.copy_from_slice(&sig_bytes);
    let sig_struct = Ed448Signature { signature: sig_arr };

    if !verify_signature(&node_pub_arr, payload_str.as_bytes(), &sig_struct).unwrap_or(false) {
        return Err(ApiError::new_system(
            ST_FORBIDDEN,
            ERR_SIG_FAIL,
            "Invalid request signature.",
        ));
    }

    // 2. チケットの検証 (予算の証明)
    let ticket: &serde_json::Value = &req.ticket;
    let t_sig_hex = ticket
        .get(KEY_TICKET_SIGNATURE)
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if t_sig_hex.is_empty() {
        return Err(ApiError::new_system(
            ST_BAD_REQUEST,
            ERR_TICKET_PARSE,
            "Ticket signature missing.",
        ));
    }

    let ticket_payload: identities_bl::TicketPayload = serde_json::from_value(ticket.clone())
        .map_err(|e| {
            ApiError::new_system(
                ST_BAD_REQUEST,
                ERR_TICKET_PARSE,
                format!("Failed to parse ticket payload: {}", e),
            )
        })?;

    if ticket_payload.node_pubkey != req.node_pubkey {
        return Err(ApiError::new_system(
            ST_FORBIDDEN,
            ERR_INVALID_KEY,
            "Ticket belongs to another node.",
        ));
    }
    if ticket_payload.forum_id != req.forum_id {
        return Err(ApiError::new_system(
            ST_FORBIDDEN,
            ERR_INVALID_KEY,
            format!(
                "Ticket forum_id '{}' does not match request forum_id '{}'.",
                ticket_payload.forum_id, req.forum_id
            ),
        ));
    }

    // チケット内の発行者(CA)公開鍵をデコード
    let t_ca_pub_bytes = hex::decode(&ticket_payload.ca_pubkey).map_err(|_| {
        ApiError::new_system(
            ST_BAD_REQUEST,
            ERR_INVALID_KEY,
            "Invalid ca_pubkey hex in ticket.",
        )
    })?;
    if t_ca_pub_bytes.len() != ED448_KEY_BYTES_LEN {
        return Err(ApiError::new_system(
            ST_BAD_REQUEST,
            ERR_INVALID_KEY,
            "Invalid ca_pubkey length in ticket.",
        ));
    }
    let mut t_ca_pub_arr = [0u8; ED448_KEY_BYTES_LEN];
    t_ca_pub_arr.copy_from_slice(&t_ca_pub_bytes);

    // チケットの署名対象ペイロードを再構築 (Canonical Serialization)
    let ticket_payload_str = ticket_payload.to_canonical_json()?;

    // チケットに対する CA の署名を検証
    let t_sig_bytes = hex::decode(t_sig_hex).map_err(|_| {
        ApiError::new_system(
            ST_BAD_REQUEST,
            ERR_TICKET_PARSE,
            "Invalid ticket signature hex.",
        )
    })?;
    if t_sig_bytes.len() != ED448_SIGNATURE_BYTES_LEN {
        return Err(ApiError::new_system(
            ST_BAD_REQUEST,
            ERR_TICKET_PARSE,
            "Invalid ticket signature length.",
        ));
    }
    let mut t_sig_arr = [0u8; ED448_SIGNATURE_BYTES_LEN];
    t_sig_arr.copy_from_slice(&t_sig_bytes);
    let t_sig_struct = Ed448Signature {
        signature: t_sig_arr,
    };

    if !verify_signature(&t_ca_pub_arr, ticket_payload_str.as_bytes(), &t_sig_struct)
        .unwrap_or(false)
    {
        return Err(ApiError::new_system(
            ST_FORBIDDEN,
            ERR_SIG_FAIL,
            "Invalid ticket signature (CA trust check failed).",
        ));
    }

    // --- トランザクション開始 (クロージャー方式) ---
    // lock_exclusive() を使用して同時実行時の競合を防ぎます。
    let (res_timestamp, res_sig_hex, final_vote_allocated) = conn.transaction::<_, (i64, String, i32), ApiError>(|txn| {
        let req = req.clone();
        let ticket_payload = ticket_payload.clone();
        let db = db.clone();
        let client = client.clone();
        let config_manager = config_manager.clone();

        Box::pin(async move {
            let now_naive = Utc::now().naive_utc();
            let now_ts_ms = time::to_ts(now_naive) as i64;
            let forum_id_uuid: uuid::Uuid = Uuid::parse_str(&req.forum_id).map_err(|e| ApiError::new_system(ST_BAD_REQUEST, ERR_INVALID_KEY, format!("Invalid forum_id uuid: {}", e)))?;
            let forum_id_bytes: Vec<u8> = forum_id_uuid.as_bytes().to_vec();

            // 3. アプリの存在確認
            let _app = apps::Entity::find_by_id(app_id)
                .one(txn)
                .await
                .map_err(|e| ApiError::new_system(ST_INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, e.to_string()))?
                .ok_or_else(|| ApiError::new_system(ST_NOT_FOUND, ERR_APP_NOT_FOUND, "App not found."))?;

            // 4. 既存の個別投票 (Item Summary) を取得
            // ca_vote_item_summaries から (Node, Forum, App) で検索: Vec<u8> expected
            let existing_item_opt = ca_vote_item_summaries::Entity::find()
                .filter(ca_vote_item_summaries::Column::NodePubkey.eq(&req.node_pubkey))
                .filter(ca_vote_item_summaries::Column::ForumId.eq(forum_id_bytes.clone()))
                .filter(ca_vote_item_summaries::Column::AppId.eq(&req.app_id))
                .lock_exclusive()
                .one(txn)
                .await
                .map_err(|e| ApiError::new_system(ST_INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, e.to_string()))?;

            let old_vote = existing_item_opt.as_ref().map(|r| r.vote_allocated).unwrap_or(0);
            
            // 5. BudgetFraud 検証 (Allocated Summary を使用): Uuid expected
            let summary_opt = ca_vote_allocated_summaries::Entity::find()
                .filter(ca_vote_allocated_summaries::Column::NodePubkey.eq(&req.node_pubkey))
                .filter(ca_vote_allocated_summaries::Column::ForumId.eq(forum_id_uuid))
                .lock_exclusive()
                .one(txn)
                .await
                .map_err(|e| ApiError::new_system(ST_INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, e.to_string()))?;

            let current_total_allocated = summary_opt.as_ref().map(|s| s.vote_allocated).unwrap_or(0);

            // 差分計算: 新しい総割り当て量 = 現在の総量 - 今回のアプリへの旧投票 + 今回のアプリへの新投票
            let new_vote = req.vote;
            let expected_vote_allocated = current_total_allocated - old_vote + new_vote;

            // 送信者となるノードが申告してきた vote_allocated と、CAが計算した vote_allocated が一致しない場合、
            // BudgetFraud としてブラックリストに登録し、ブロードキャストする
            if req.vote_allocated != expected_vote_allocated {
                let err_msg = format!("Budget Fraud detected: Node reported {}, but CA calculated {}. (Old:{}, New:{}, Base:{})", 
                    req.vote_allocated, expected_vote_allocated, old_vote, new_vote, current_total_allocated);
                log::warn!("<Apps> {}", err_msg);

                // 証拠保全 & ブロードキャスト (BudgetFraud)
                // トランザクションがロールバックされてもブラックリスト登録を維持するため、
                // また全体のレスポンスタイムを短縮するため、非同期タスクとして実行します。
                let evidence = CrimeEvidence {
                    detail: CrimeDetail::BudgetFraud {
                        forum_id: req.forum_id.clone(),
                        reported_vote_allocated: req.vote_allocated,
                        expected_vote_allocated,
                        timestamp: req.timestamp,
                        ca_base_url: ticket_payload.ca_base_url.to_string(),
                    },
                    target_pubkey: req.node_pubkey.clone(),
                    observed_at: now_ts_ms,
                    signature: req.signature.clone(),
                    signed_payload: format_vote_payload(&req.app_id, req.vote, req.vote_allocated, &req.timestamp.to_string(), &req.ticket).unwrap_or_default().as_bytes().to_vec().iter().map(|b| format!("{:02x}", b)).collect::<String>(),
                };

                // 非同期で実行 (トランザクション外)
                let db_clone = Arc::new(db.clone());
                let config_clone = config_manager.clone();
                let client_clone = Arc::new(client.clone());
                let evidence_clone = evidence.clone();

                tokio::spawn(async move {
                    if let Err(e) = add_to_blacklist(&db_clone, evidence_clone.clone()).await {
                        log::error!("<Apps> Failed to save fraud evidence: {}", e);
                    }
                    if let Err(e) = report_crime_broadcast(&db_clone, &config_clone, &client_clone, &evidence_clone, None).await {
                        log::warn!("<Apps> Broadcast failed: {}", e);
                    }
                });

                return Err(ApiError::new_system(ST_FORBIDDEN, ERR_INSUFFICIENT_FUNDS, err_msg));
            }

            if expected_vote_allocated > ticket_payload.initial_balance {
                return Err(ApiError::new_system(ST_FORBIDDEN, ERR_INSUFFICIENT_FUNDS, format!("Insufficient funds. Allocated {} exceeds Initial {}", expected_vote_allocated, ticket_payload.initial_balance)));
            }

            // 6. レスポンス生成 (CA署名)
            let ca_keypair = config_manager.get_node_keypair()?;
            let res_timestamp = now_ts_ms;
            let res_payload = format_vote_receipt_payload(req.vote_allocated, res_timestamp, &req.signature);
            let res_sig_struct = ca_keypair.sign(res_payload.as_bytes()).map_err(|e| ApiError::new_system(ST_INTERNAL_SERVER_ERROR, ERR_SIGN, e.to_string()))?;
            let res_sig_hex = hex::encode(res_sig_struct.signature);

            // 7. DB更新: ca_vote_item_summaries
            if req.vote == 0 {
                if let Some(record) = existing_item_opt {
                    record.delete(txn).await.map_err(|e: DbErr| ApiError::new_system(ST_INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, e.to_string()))?;
                }
            } else {
                let mut active_item = if let Some(record) = existing_item_opt {
                    record.into_active_model()
                } else {
                    ca_vote_item_summaries::ActiveModel {
                        id: Set(Uuid::new_v4().as_bytes().to_vec()),
                        node_pubkey: Set(req.node_pubkey.clone()),
                        forum_id: Set(forum_id_bytes.clone()),
                        app_id: Set(req.app_id.clone()),
                        created_at: Set(now_naive),
                        ..Default::default()
                    }
                };
                active_item.vote_allocated = Set(req.vote);
                active_item.node_timestamp = Set(req.timestamp);
                active_item.node_signature = Set(req.signature.clone());
                active_item.ca_timestamp = Set(res_timestamp);
                active_item.ca_signature = Set(res_sig_hex.clone());
                active_item.updated_at = Set(now_naive);
                active_item.save(txn).await.map_err(|e: DbErr| ApiError::new_system(ST_INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, e.to_string()))?;
            }

            // 8. DB更新: ca_vote_allocated_summaries
            if let Some(s) = summary_opt {
                let mut active: ca_vote_allocated_summaries::ActiveModel = s.into_active_model();
                active.vote_allocated = Set(req.vote_allocated);
                active.node_timestamp = Set(req.timestamp);
                active.node_signature = Set(req.signature.clone());
                active.ca_timestamp = Set(res_timestamp);
                active.ca_signature = Set(res_sig_hex.clone());
                active.updated_at = Set(now_naive);
                
                active.update(txn).await.map_err(|e: DbErr| ApiError::new_system(ST_INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, format!("Failed to update vote allocated summary: {}", e)))?;
            } else {
                let active = ca_vote_allocated_summaries::ActiveModel {
                    id: Set(Uuid::new_v4()),
                    node_pubkey: Set(req.node_pubkey.clone()),
                    forum_id: Set(forum_id_uuid),
                    vote_allocated: Set(req.vote_allocated),
                    node_timestamp: Set(req.timestamp),
                    node_signature: Set(req.signature.clone()),
                    ca_timestamp: Set(res_timestamp),
                    ca_signature: Set(res_sig_hex.clone()),
                    created_at: Set(now_naive),
                    updated_at: Set(now_naive),
                };
                active.insert(txn).await.map_err(|e: DbErr| ApiError::new_system(ST_INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, format!("Failed to insert vote allocated summary: {}", e)))?;
            }

            Ok((res_timestamp, res_sig_hex, req.vote_allocated))
        })
    }).await.map_err(|e| match e {
        TransactionError::Connection(db_err) => ApiError::new_system(ST_INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, db_err.to_string()),
        TransactionError::Transaction(api_err) => api_err,
    })?;

    Ok(VoteAppCaRes {
        success: true,
        vote_allocated: final_vote_allocated,
        timestamp: res_timestamp,
        signature: res_sig_hex,
    })
}

// ============================================================
// 広告と発見 (スケルトン)
// ============================================================
pub async fn advertise_app_ca(_req: AdvertiseAppCaReq) -> Result<AdvertiseAppCaRes, ApiError> {
    // TODO: 実際のCA間 P2P ネットワークへの広告ロジックをここに実装する
    Ok(AdvertiseAppCaRes {
        success: true,
        advertised_nodes: 0,
    })
}

pub async fn discover_app_ca(req: DiscoverAppCaReq) -> Result<DiscoverAppCaRes, ApiError> {
    log::debug!(
        "<Apps> discover_app_ca (Skeleton) called. IDs: {:?}, query: {:?}",
        req.app_ids,
        req.query
    );

    // スケルトン実装:
    // app_ids または q に基づいて、モックのアプリ情報を返す。
    let mut items = Vec::new();

    if let Some(ids) = req.app_ids {
        for id in ids {
            items.push(DiscoverAppItemCaRes {
                app_id: id.clone(),
                name: format!("App-{}", id),
                nodes: vec![
                    "http://node1.example.mycute".to_string(),
                    "http://node2.example.mycute".to_string(),
                ],
            });
        }
    } else if let Some(query) = req.query {
        items.push(DiscoverAppItemCaRes {
            app_id: "mock-uuid-for-query".to_string(),
            name: format!("Search Result for: {}", query),
            nodes: vec!["http://node-search.example.mycute".to_string()],
        });
    }

    Ok(DiscoverAppCaRes { items })
}
