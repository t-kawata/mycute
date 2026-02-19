use std::sync::Arc;
use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, Set, ActiveModelTrait, TransactionTrait, QuerySelect, DbErr, Select};
use crate::constants::{ST_INTERNAL_SERVER_ERROR, ST_BAD_GATEWAY};
use axum::http::StatusCode;
use chrono::NaiveDateTime;
use uuid::Uuid;
use std::collections::HashMap;
use std::cmp::max;
use serde_json::{self, Value};
use crate::{
    entities::{identities, tickets, verifications},
    mode::rt::{
        rtreq::node_identities_req::{SyncIdentityNodeReq, EntryIdentityNodeReq, ApplyIdentityNodeReq},
        rtreq::ca_identities_req::{EntryIdentityCaReq, ApplyIdentityCaReq, ExistingForumReq},
        rtres::{
            errs_res::ApiError,
            node_identities_res::{SyncIdentityNodeRes, GetIdentityNodeRes, EntryIdentityNodeRes, ApplyIdentityNodeRes},
            ca_identities_res::{SyncIdentityCaRes, EntryIdentityCaRes, ApplyIdentityCaRes},
        },
        rterr::rterr,
        owner_secrets::OWNER_PUB_KEY_HEX,
        rtbl::{
            node_apps_bl::{load_my_rem_payload_for_entry},
            identities_bl::{self, ParsedTicket},
        },
    },
    utils::{
        jwt::{JwtUsr, JwtIDs},
        time,
        crypto::{verify_signature, Ed448Signature},
    },
    stt_config::{ConfigManager, CaEntry, ForumState, MyRemPayload},
    constants::{
        PATH_IDENTITIES_SYNC, ERR_CA_UNREACHABLE,
        ERR_CA_RESPONSE, ERR_CA_PARSE, ERR_ANCHOR_KEY, ERR_INVALID_CA_TOKEN, ERR_INVALID_CA_KEY,
        ED448_KEY_BYTES_LEN, ED448_SIGNATURE_BYTES_LEN,
        ERR_CA_TRUST_FAIL, ERR_MY_PUBKEY, ERR_INVALID_RESP, DATE_FORMAT_STANDARD,
        ERR_INVALID_SIG, ERR_SIG_FAIL, ERR_IDENTITY, PATH_CA_IDENTITIES_ENTRY, PATH_CA_IDENTITIES_APPLY, ERR_CA_CONNECT,
        ERR_TICKET_PARSE, ERR_SAVE, ERR_SIGN,
        KEY_TICKET_INITIAL_BALANCE, KEY_TICKET_CA_PUBKEY, ERR_VERIFICATION_PENDING,
        KEY_TICKET_FORUM_ID, KEY_TICKET_FORUM_NAME, KEY_TICKET_FORUM_DESC
    },
    mode::rt::client::secure_client::SecureClient,
};

// ============================================================
// Internal Helpers
// ============================================================
fn find_node_identities_base(apx_id: i32, vdr_id: i32) -> Select<identities::Entity> {
    identities::Entity::find()
        .filter(identities::Column::ApxId.eq(apx_id))
        .filter(identities::Column::VdrId.eq(vdr_id))
}

// ============================================================
// Public Logic
// ============================================================

pub async fn entry_identity_node(
    conn: &DatabaseConnection,
    req: EntryIdentityNodeReq,
    config_manager: Arc<ConfigManager>,
    client: &SecureClient,
) -> Result<EntryIdentityNodeRes, ApiError> {
    // 1. Ensure Identity & Get Pubkey and Keypair
    identities_bl::ensure_node_identity(&config_manager).map_err(|e| ApiError::new_system(ST_INTERNAL_SERVER_ERROR, ERR_IDENTITY, e.to_string()))?;
    let my_pubkey = identities_bl::get_pubkey(config_manager.clone()).await?;
    let my_keypair = config_manager.get_node_keypair()?;

    // 2. Prepare Request (Delta Entry)
    // DB の tickets テーブルから、この CA (ca_base_url) に紐づく既存の forum_id と updated_at を取得
    let existing_forums = {
        let ca_url_key = req.ca_base_url.trim_end_matches('/').to_string();
        let ticket_records: Vec<(Vec<u8>, NaiveDateTime)> = tickets::Entity::find()
            .filter(tickets::Column::CaBaseUrl.eq(&ca_url_key))
            .select_only()
            .column(tickets::Column::ForumId)
            .column(tickets::Column::UpdatedAt)
            .into_tuple()
            .all(conn)
            .await
            .map_err(|e| ApiError::new_system(ST_INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, e.to_string()))?;
        
        let mut forums = Vec::new();
        for (forum_id_bytes, updated_at) in ticket_records {
            let uuid = Uuid::from_slice(&forum_id_bytes).unwrap_or_default();
            forums.push(ExistingForumReq {
                id: uuid.to_string(),
                updated_at: updated_at.to_string(),
            });
        }
        
        if forums.is_empty() {
             None 
        } else {
             Some(forums) 
        }
    };

    // Generate PoP Signature (Sign PubKey Bytes)
    let pop_signature = {
        let key_pair = config_manager.get_node_keypair()?;
        let pub_bytes = hex::decode(&my_pubkey).map_err(|_| ApiError::new_system(ST_INTERNAL_SERVER_ERROR, ERR_MY_PUBKEY, "Invalid my pubkey hex."))?;
        let sig = key_pair.sign(&pub_bytes).map_err(|e| ApiError::new_system(ST_INTERNAL_SERVER_ERROR, ERR_SIGN, e.to_string()))?;
        hex::encode(sig.signature)
    };

    let ca_req = EntryIdentityCaReq {
        public_key: my_pubkey.clone(),
        info: req.info,
        signature: pop_signature,
        existing_forums,
    };
    let url = format!("{}{}", req.ca_base_url.trim_end_matches('/'), PATH_CA_IDENTITIES_ENTRY);
    
    let res = client.post(&url, &ca_req)
        .await
        .map_err(|e| ApiError::new_system(ST_BAD_GATEWAY, ERR_CA_CONNECT, format!("Failed to connect to CA: {}", e)))?;

    if !res.status().is_success() {
        let status = StatusCode::from_u16(res.status().as_u16()).unwrap_or(ST_INTERNAL_SERVER_ERROR);
        let text = res.text().await.unwrap_or_default();
        return Err(ApiError::new_system(status, ERR_CA_RESPONSE, format!("CA returned error: {}", text)));
    }

    let ca_res: EntryIdentityCaRes = res.json().await.map_err(|e| ApiError::new_system(ST_BAD_GATEWAY, ERR_CA_PARSE, e.to_string()))?;

    // 3. Process Tickets & Prepare Data (Validate First)
    let mut parsed_tickets = Vec::new();

    for ticket_str in &ca_res.tickets {
        let ticket_json: Value = serde_json::from_str(ticket_str).map_err(|e| ApiError::new_system(ST_BAD_GATEWAY, ERR_TICKET_PARSE, format!("Invalid ticket JSON: {}", e)))?;

        let _ = ticket_json.get(KEY_TICKET_CA_PUBKEY).and_then(|v| v.as_str()).ok_or_else(|| ApiError::new_system(ST_BAD_GATEWAY, ERR_TICKET_PARSE, "Ticket missing ca_pubkey."))?;
        let forum_id = ticket_json.get(KEY_TICKET_FORUM_ID).and_then(|v| v.as_str()).ok_or_else(|| ApiError::new_system(ST_BAD_GATEWAY, ERR_TICKET_PARSE, "Ticket missing forum_id."))?;
        let forum_name = ticket_json.get(KEY_TICKET_FORUM_NAME).and_then(|v| v.as_str()).unwrap_or("Unknown").to_string();
        let forum_desc = ticket_json.get(KEY_TICKET_FORUM_DESC).and_then(|v| v.as_str()).map(|s| s.to_string());
        let initial_balance = ticket_json.get(KEY_TICKET_INITIAL_BALANCE).and_then(|v| v.as_i64()).unwrap_or(0) as i32;

        let forum_id_str = forum_id.to_string();
        let forum_uuid_bytes = Uuid::parse_str(forum_id).unwrap_or_default().as_bytes().to_vec();

        parsed_tickets.push(ParsedTicket {
            json: ticket_json,
            forum_id_str,
            forum_uuid_bytes,
            forum_name,
            forum_desc,
            initial_balance,
        });
    }

    // 4. Execute Transaction (Atomic Block)
    let ca_base_url_for_txn = ca_res.ca_base_url.clone();
    let ca_token_for_txn = ca_res.ca_token.clone();
    let ca_pubkey_for_txn = ca_res.ca_pubkey.clone();

    conn.transaction::<_, (), ApiError>(|txn| Box::pin(async move {
        // DB Upsert loop
        let now = time::now();
        for pt in &parsed_tickets {
            let existing_ticket = tickets::Entity::find()
                .filter(tickets::Column::CaBaseUrl.eq(&ca_base_url_for_txn))
                .filter(tickets::Column::ForumId.eq(pt.forum_uuid_bytes.clone()))
                .one(txn)
                .await
                .map_err(|e| ApiError::new_system(ST_INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, e.to_string()))?;

            if let Some(record) = existing_ticket {
                let mut active: tickets::ActiveModel = record.into();
                
                if active.ticket_data.as_ref() != &pt.json {
                    active.ticket_data = Set(pt.json.clone());
                }
                if active.forum_name.as_ref() != &pt.forum_name {
                    active.forum_name = Set(pt.forum_name.clone());
                }
                if active.forum_description.as_ref() != &pt.forum_desc {
                    active.forum_description = Set(pt.forum_desc.clone());
                }
                if active.ca_token.as_ref() != &ca_token_for_txn {
                    active.ca_token = Set(ca_token_for_txn.clone());
                }

                if active.is_changed() {
                    active.updated_at = Set(now);
                    active.update(txn).await.map_err(|e| ApiError::new_system(ST_INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, e.to_string()))?;
                }
            } else {
                let ticket_model = tickets::ActiveModel {
                    ca_pubkey: Set(ca_pubkey_for_txn.clone()),
                    ca_base_url: Set(ca_base_url_for_txn.clone()),
                    forum_id: Set(pt.forum_uuid_bytes.clone()),
                    forum_name: Set(pt.forum_name.clone()),
                    forum_description: Set(pt.forum_desc.clone()),
                    ticket_data: Set(pt.json.clone()),
                    ca_token: Set(ca_token_for_txn.clone()),
                    created_at: Set(now),
                    updated_at: Set(now),
                    ..Default::default()
                };
                ticket_model.insert(txn).await.map_err(|e| ApiError::new_system(ST_INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, e.to_string()))?;
            }
        }
        
        // Cleanup Deleted Forums (Physical Delete)
        if !ca_res.deleted_forum_ids.is_empty() {
            log::info!("<NodeIdentities> Cleaning up deleted forums: {:?}", ca_res.deleted_forum_ids);
            for dfid in &ca_res.deleted_forum_ids {
                if let Ok(u) = Uuid::parse_str(dfid) {
                    let uuid_bytes = u.as_bytes().to_vec();
                    // DB 削除
                    tickets::Entity::delete_many()
                        .filter(tickets::Column::CaBaseUrl.eq(&ca_base_url_for_txn))
                        .filter(tickets::Column::ForumId.eq(uuid_bytes))
                        .exec(txn)
                        .await
                        .map_err(|e| ApiError::new_system(ST_INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, e.to_string()))?;
                }
            }
        }

        // Update my_rem (Memory) and Save (Disk)
        {
            let mut settings = config_manager.settings.write();

            // Load existing
            let crypto_key = settings.server.rt_crypto_key.clone();
            let mut payload: MyRemPayload = match &settings.my_rem {
                Some(rem_enc) => {
                    load_my_rem_payload_for_entry(rem_enc, &crypto_key, &my_keypair)
                        .unwrap_or_else(|e| {
                            log::warn!("<NodeIdentities> Failed to load existing my_rem, starting fresh: {}", e);
                            MyRemPayload::default()
                        })
                },
                None => MyRemPayload::default(),
            };

            // Update Entry
            let entry = payload.ca_entries.entry(ca_base_url_for_txn.clone()).or_insert_with(CaEntry::default);
            let now_ts = time::now_ts_ms() as i64;
            entry.last_blacklist_sync_ts = max(entry.last_blacklist_sync_ts, now_ts);

            // Update Forum States
            for pt in &parsed_tickets {
                entry.forum_states.entry(pt.forum_id_str.clone()).or_insert(ForumState {
                    balance: pt.initial_balance,
                    votes: HashMap::new(),
                });
            }

            // Cleanup Deleted Forums from my_rem
            for dfid in &ca_res.deleted_forum_ids {
                entry.forum_states.remove(dfid);
            }

            // Encrypt & Set
            let encrypted = config_manager.encode_my_rem_payload(&payload, &my_keypair)?;
            settings.my_rem = Some(encrypted);
            
            // Save to Disk (Critical: if this fails, transaction will rollback)
            // Drop lock before saving to avoid potential deadlocks if save() takes read lock
            drop(settings); 
            
            // Save
            config_manager.save().map_err(|e| ApiError::new_system(ST_INTERNAL_SERVER_ERROR, ERR_SAVE, e))?;
        }
        
        Ok(())
    })).await?;

    log::info!("<NodeIdentities> Processed tickets and updated my_rem for CA '{}' (Atomic).", ca_res.ca_base_url);

    Ok(EntryIdentityNodeRes {
        success: ca_res.success,
        created_at: ca_res.created_at,
    })
}

pub async fn apply_identity_node(
    req: ApplyIdentityNodeReq,
    config_manager: Arc<ConfigManager>,
    client: &SecureClient,
) -> Result<ApplyIdentityNodeRes, ApiError> {
    // 1. Ensure Identity & Get Pubkey
    identities_bl::ensure_node_identity(&config_manager).map_err(|e| ApiError::new_system(ST_INTERNAL_SERVER_ERROR, ERR_IDENTITY, e.to_string()))?;
    let my_pubkey = identities_bl::get_pubkey(config_manager.clone()).await?;

    // 2. Call CA
    let ca_req = ApplyIdentityCaReq {
        public_key: my_pubkey.clone(),
        contact_email: req.contact_email,
        info: req.info.clone(),
        expire_seconds: req.expire_seconds,
    };
    let url = format!("{}{}", req.ca_base_url.trim_end_matches('/'), PATH_CA_IDENTITIES_APPLY);

    log::debug!("<NodeIdentities> apply_identity_node: Requesting CA at {}", url);
    let res = client.post(&url, &ca_req)
        .await
        .map_err(|e| ApiError::new_system(ST_BAD_GATEWAY, ERR_CA_CONNECT, format!("Failed to connect to CA: {}", e)))?;

    if !res.status().is_success() {
        let status = StatusCode::from_u16(res.status().as_u16()).unwrap_or(ST_INTERNAL_SERVER_ERROR);
        let text = res.text().await.unwrap_or_default();
        return Err(ApiError::new_system(status, ERR_CA_RESPONSE, format!("CA returned error: {}", text)));
    }

    let ca_res: ApplyIdentityCaRes = res.json().await.map_err(|e| ApiError::new_system(ST_BAD_GATEWAY, ERR_CA_PARSE, format!("Failed to parse CA response: {}", e)))?;
    
    Ok(ApplyIdentityNodeRes {
        success: ca_res.success,
        message: ca_res.message,
    })
}

pub async fn sync_identity_node(
    conn: &DatabaseConnection,
    _ju: &JwtUsr,
    ids: &JwtIDs,
    req: SyncIdentityNodeReq,
    config_manager: Arc<ConfigManager>,
    client: &SecureClient,
) -> Result<SyncIdentityNodeRes, ApiError> {
    let my_pubkey_hex = identities_bl::get_pubkey(config_manager).await?;
    log::debug!("<NodeIdentities> sync_identity_node: Node Pubkey: {}", my_pubkey_hex);

    let url = format!("{}{}/{}", req.ca_base_url.trim_end_matches('/'), PATH_IDENTITIES_SYNC, my_pubkey_hex);
    log::debug!("<NodeIdentities> Fetching from CA: {}", url);

    let res = client.get(&url)
        .await
        .map_err(|e| ApiError::new_system(ST_BAD_GATEWAY, ERR_CA_UNREACHABLE, format!("Failed to contact CA: {}", e)))?;

    if res.status().as_u16() == StatusCode::ACCEPTED {
        log::info!("<NodeIdentities> Sync: Verification is still pending for {}.", my_pubkey_hex);
        return Err(ApiError::new_system(StatusCode::ACCEPTED, ERR_VERIFICATION_PENDING, "CA verification is still pending. Please wait for approval."));
    }

    if !res.status().is_success() {
        return Err(ApiError::new_system(StatusCode::from_u16(res.status().as_u16()).unwrap_or(ST_BAD_GATEWAY), ERR_CA_RESPONSE, format!("CA returned error: {}", res.status())));
    }

    let ca_res: SyncIdentityCaRes = res.json().await.map_err(|e| ApiError::new_system(ST_INTERNAL_SERVER_ERROR, ERR_CA_PARSE, format!("Failed to parse CA response: {}", e)))?;

    // Verification Logic (L1 & L2)
    if let (Some(sig_hex), Some(tok_hex)) = (&ca_res.signature, &ca_res.ca_token) {
        log::debug!("<NodeIdentities> Verifying signatures from CA...");

        let owner_pub_bytes = hex::decode(OWNER_PUB_KEY_HEX).map_err(|_| ApiError::new_system(ST_INTERNAL_SERVER_ERROR, ERR_ANCHOR_KEY, "Invalid hardcoded owner key."))?;
        let mut owner_pub_arr = [0u8; ED448_KEY_BYTES_LEN];
        owner_pub_arr.copy_from_slice(&owner_pub_bytes);

        let parts: Vec<&str> = tok_hex.split('.').collect::<Vec<&str>>();
        if parts.len() != 2 {
            return Err(ApiError::new_system(ST_INTERNAL_SERVER_ERROR, ERR_CA_PARSE, "Invalid CA token format (hex).".to_string()));
        }
        let ca_token_sig_hex = parts[0];
        let ca_expire_at_str = parts[1];
        let ca_expire_at: u64 = ca_expire_at_str.parse().map_err(|_| ApiError::new_system(ST_BAD_GATEWAY, ERR_INVALID_CA_TOKEN, "Invalid CA Token expire."))?;

        let ca_pub_bytes = hex::decode(&ca_res.ca_pubkey).map_err(|_| ApiError::new_system(ST_BAD_GATEWAY, ERR_INVALID_CA_KEY, "Invalid ca_public_key hex."))?;
        if ca_pub_bytes.len() != ED448_KEY_BYTES_LEN {
             return Err(ApiError::new_system(ST_BAD_GATEWAY, ERR_INVALID_CA_KEY, "Invalid ca_public_key length."));
        }
        let mut ca_msg = Vec::new();
        ca_msg.extend_from_slice(&ca_pub_bytes);
        ca_msg.extend_from_slice(&ca_expire_at.to_be_bytes());

        let ca_sig_bytes = hex::decode(ca_token_sig_hex).map_err(|_| ApiError::new_system(ST_BAD_GATEWAY, ERR_INVALID_CA_TOKEN, "Invalid ca_token signature hex."))?;
        let mut ca_sig_arr = [0u8; ED448_SIGNATURE_BYTES_LEN];
        ca_sig_arr.copy_from_slice(&ca_sig_bytes);
        let ca_sig_struct = Ed448Signature { signature: ca_sig_arr };

        if !verify_signature(&owner_pub_arr, &ca_msg, &ca_sig_struct).unwrap_or(false) {
             return Err(ApiError::new_system(ST_BAD_GATEWAY, ERR_CA_TRUST_FAIL, "CA Token verification failed (L1)."));
        }

        let mut my_pub_arr = [0u8; ED448_KEY_BYTES_LEN];
        let my_pub_vec = hex::decode(&my_pubkey_hex).map_err(|_| ApiError::new_system(ST_INTERNAL_SERVER_ERROR, ERR_MY_PUBKEY, "Invalid my public key hex."))?;
        my_pub_arr.copy_from_slice(&my_pub_vec);

        let expire_str = ca_res.identity.expire_at.as_ref().ok_or_else(|| ApiError::new_system(ST_BAD_GATEWAY, ERR_INVALID_RESP, "Verified identity missing expire_at."))?;
        let expire_dt = NaiveDateTime::parse_from_str(expire_str, DATE_FORMAT_STANDARD).map_err(|_| ApiError::new_system(ST_BAD_GATEWAY, ERR_INVALID_RESP, "Invalid expire_at format."))?;
        let my_expire_at = time::to_ts(expire_dt);

        let mut dev_msg = Vec::new();
        dev_msg.extend_from_slice(&my_pub_vec);
        dev_msg.extend_from_slice(&my_expire_at.to_be_bytes());

        let dev_sig_bytes = hex::decode(sig_hex).map_err(|_| ApiError::new_system(ST_BAD_GATEWAY, ERR_INVALID_SIG, "Invalid signature hex."))?;
        let mut dev_sig_arr = [0u8; ED448_SIGNATURE_BYTES_LEN];
        dev_sig_arr.copy_from_slice(&dev_sig_bytes);
        let dev_sig_struct = Ed448Signature { signature: dev_sig_arr };

        let mut ca_pub_arr = [0u8; ED448_KEY_BYTES_LEN];
        ca_pub_arr.copy_from_slice(&ca_pub_bytes);

        if !verify_signature(&ca_pub_arr, &dev_msg, &dev_sig_struct).unwrap_or(false) {
             return Err(ApiError::new_system(ST_BAD_GATEWAY, ERR_SIG_FAIL, "Dev Signature verification failed (L2)."));
        }
    }

    // === DB Update (トランザクションでラップ) ===
    let apx_id = ids.apx_id as i32;
    let vdr_id = ids.vdr_id as i32;
    let my_pubkey_hex_clone = my_pubkey_hex.clone();
    let ca_pubkey_clone = ca_res.ca_pubkey.clone();
    let signature_clone = ca_res.signature.clone();
    let ca_token_clone = ca_res.ca_token.clone();
    let ca_base_url_from_ca = ca_res.ca_base_url.clone();
    let info_clone = ca_res.identity.info.clone();
    let is_candidate = ca_res.identity.is_candidate;
    let verified_at_dt = ca_res.identity.verified_at.as_ref().and_then(|s| NaiveDateTime::parse_from_str(s, DATE_FORMAT_STANDARD).ok());
    let expire_at_dt = ca_res.identity.expire_at.as_ref().and_then(|s| NaiveDateTime::parse_from_str(s, DATE_FORMAT_STANDARD).ok());

    conn.transaction::<_, (), ApiError>(|txn| Box::pin(async move {
        let now = time::now();

        // 既存レコードの検索
        let existing = find_node_identities_base(apx_id, vdr_id)
            .filter(identities::Column::PublicKey.eq(&my_pubkey_hex_clone))
            .one(txn)
            .await
            .map_err(|e: DbErr| ApiError::new_system(ST_INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, e.to_string()))?;

        if let Some(record) = existing {
            // identities テーブルのプロファイル情報のみ更新
            let mut active: identities::ActiveModel = record.into();
            active.info = Set(info_clone.clone());
            active.updated_at = Set(now);
            active.update(txn).await.map_err(|e| ApiError::new_system(ST_INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, e.to_string()))?;

            // verifications テーブルの検証データを更新/挿入
            let existing_ver = verifications::Entity::find()
                .filter(verifications::Column::NodePubkey.eq(&my_pubkey_hex_clone))
                .filter(verifications::Column::CaBaseUrl.eq(&ca_base_url_from_ca))
                .filter(verifications::Column::CaPubkey.eq(&ca_pubkey_clone))
                .one(txn)
                .await
                .map_err(|e| ApiError::new_system(ST_INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, e.to_string()))?;

            if let Some(ver_rec) = existing_ver {
                let mut ver_active: verifications::ActiveModel = ver_rec.into();
                // Note: ca_pubkey は不変であるため更新不要
                ver_active.signature = Set(signature_clone.clone());
                ver_active.ca_token = Set(ca_token_clone.clone());
                ver_active.is_candidate = Set(if is_candidate { 1 } else { 0 });
                ver_active.verified_at = Set(verified_at_dt);
                ver_active.expire_at = Set(expire_at_dt);
                ver_active.updated_at = Set(now);
                ver_active.update(txn).await.map_err(|e| ApiError::new_system(ST_INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, e.to_string()))?;
            } else {
                let ver_model = verifications::ActiveModel {
                    node_pubkey: Set(my_pubkey_hex_clone.clone()),
                    ca_pubkey: Set(ca_pubkey_clone.clone()),
                    ca_base_url: Set(ca_base_url_from_ca.clone()),
                    signature: Set(signature_clone.clone()),
                    ca_token: Set(ca_token_clone.clone()),
                    is_candidate: Set(if is_candidate { 1 } else { 0 }),
                    verified_at: Set(verified_at_dt),
                    expire_at: Set(expire_at_dt),
                    ..Default::default()
                };
                ver_model.insert(txn).await.map_err(|e| ApiError::new_system(ST_INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, e.to_string()))?;
            }
        } else {
            // identities テーブルにプロファイル情報を挿入
            let model = identities::ActiveModel {
                apx_id: Set(apx_id),
                vdr_id: Set(vdr_id),
                public_key: Set(my_pubkey_hex_clone.clone()),
                info: Set(info_clone.clone()),
                ..Default::default()
            };
            model.insert(txn).await.map_err(|e| ApiError::new_system(ST_INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, e.to_string()))?;

            // verifications テーブルに検証データを挿入
            let ver_model = verifications::ActiveModel {
                node_pubkey: Set(my_pubkey_hex_clone),
                ca_pubkey: Set(ca_pubkey_clone.clone()),
                ca_base_url: Set(ca_base_url_from_ca),
                signature: Set(signature_clone.clone()),
                ca_token: Set(ca_token_clone.clone()),
                is_candidate: Set(if is_candidate { 1 } else { 0 }),
                verified_at: Set(verified_at_dt),
                expire_at: Set(expire_at_dt),
                ..Default::default()
            };
            ver_model.insert(txn).await.map_err(|e| ApiError::new_system(ST_INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, e.to_string()))?;
        }

        Ok(())
    })).await?;

    Ok(SyncIdentityNodeRes {
        identity: GetIdentityNodeRes {
            id: ca_res.identity.id,
            apx_id: ca_res.identity.apx_id,
            vdr_id: ca_res.identity.vdr_id,
            public_key: ca_res.identity.public_key,
            info: ca_res.identity.info,
            verified_at: ca_res.identity.verified_at,
            expire_at: ca_res.identity.expire_at,
            is_candidate: ca_res.identity.is_candidate,
            created_at: ca_res.identity.created_at,
            updated_at: ca_res.identity.updated_at,
        },
        signature: ca_res.signature,
        ca_token: ca_res.ca_token,
        ca_pubkey: ca_res.ca_pubkey,
    })
}