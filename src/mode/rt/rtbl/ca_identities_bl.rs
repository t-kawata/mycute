use crate::constants::{ST_BAD_REQUEST, ST_CONFLICT, ST_INTERNAL_SERVER_ERROR, ST_NOT_FOUND};
use crate::{
    constants::{
        APX_ID_ISOLATED, ED448_KEY_BYTES_LEN, ED448_PUBKEY_HEX_LEN, ERR_CA_PUBKEY, ERR_DECRYPT,
        ERR_EXPECTED_RECORD, ERR_INVALID_SIG, ERR_SIGN, ERR_VERIFICATION_PENDING,
        IS_CANDIDATE_FALSE, IS_CANDIDATE_TRUE, KEY_TICKET_SIGNATURE, VDR_ID_ISOLATED,
    },
    entities::{forums, identities, verifications},
    mode::rt::{
        rtbl::identities_bl::{self},
        rterr::rterr,
        rtreq::ca_identities_req::{
            ApplyIdentityCaReq, EntryIdentityCaReq, SearchIdentitiesCaReq, VerifyIdentityCaReq,
        },
        rtres::{
            ca_identities_res::{
                ApplyIdentityCaRes, DeleteIdentityCaRes, EntryIdentityCaRes, GetIdentityCaRes,
                IdentityItemCaRes, SearchIdentitiesCaRes, SyncIdentityCaRes, VerifyIdentityCaRes,
            },
            errs_res::ApiError,
        },
    },
    mycute_settings::ConfigManager,
    utils::crypto,
    utils::jwt::{JwtIDs, JwtRole, JwtUsr},
    utils::time,
};
use axum::http::StatusCode;
use chrono::Duration;
use sea_orm::{
    prelude::Expr, ActiveModelTrait, ColumnTrait, Condition, DatabaseConnection, EntityTrait,
    PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
};
use std::sync::Arc;

// ============================================================
// 内部ヘルパー
// ============================================================
fn find_ca_identities_base(apx_id: i32, vdr_id: i32) -> sea_orm::Select<identities::Entity> {
    identities::Entity::find()
        .filter(identities::Column::ApxId.eq(apx_id))
        .filter(identities::Column::VdrId.eq(vdr_id))
}

fn get_my_ca_pubkey_hex(config_manager: &ConfigManager) -> Result<String, ApiError> {
    let ca_keypair = config_manager.get_node_keypair()?;
    Ok(hex::encode(ca_keypair.public))
}

// ============================================================
// 公開ロジック
// ============================================================

pub async fn search_identities(
    conn: &DatabaseConnection,
    _ju: &JwtUsr,
    ids: &JwtIDs,
    req: SearchIdentitiesCaReq,
    config_manager: Arc<ConfigManager>,
) -> Result<SearchIdentitiesCaRes, ApiError> {
    log::debug!(
        "<CaIdentities> search_identities: apx {}, vdr {}",
        ids.apx_id,
        ids.vdr_id
    );

    // 自分自身の CA 公開鍵を取得
    let ca_pubkey = get_my_ca_pubkey_hex(&config_manager)?;

    let mut query = identities::Entity::find();
    let mut condition = Condition::any().add(
        Condition::all()
            .add(identities::Column::ApxId.eq(ids.apx_id as i32))
            .add(identities::Column::VdrId.eq(ids.vdr_id as i32)),
    );

    if req.include_isolated {
        condition = condition.add(
            Condition::all()
                .add(identities::Column::ApxId.eq(APX_ID_ISOLATED as i32))
                .add(identities::Column::VdrId.eq(VDR_ID_ISOLATED as i32)),
        );
    }

    query = query.filter(condition);

    let total = query.clone().count(conn).await.map_err(|e| {
        ApiError::new_system(ST_INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, e.to_string())
    })?;

    let items = query
        .order_by_desc(identities::Column::Id)
        .offset(Some(req.offset as u64))
        .limit(Some(req.limit as u64))
        .all(conn)
        .await
        .map_err(|e| {
            ApiError::new_system(ST_INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, e.to_string())
        })?;

    // 各identityに対してverificationデータを取得
    let mut result_items = Vec::new();
    for m in items {
        let verification = verifications::Entity::find()
            .filter(verifications::Column::NodePubkey.eq(&m.public_key))
            .filter(verifications::Column::CaPubkey.eq(&ca_pubkey))
            .one(conn)
            .await
            .map_err(|e| {
                ApiError::new_system(ST_INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, e.to_string())
            })?;

        let (verified_at, expire_at, is_candidate, identity_layer) = if let Some(v) = verification {
            let layer = identities_bl::determine_layer(
                &config_manager,
                &m.public_key,
                &ca_pubkey,
                &config_manager
                    .settings
                    .read()
                    .server
                    .my_base_url
                    .clone()
                    .unwrap_or_default(),
                v.signature.as_deref(),
                v.ca_token.as_deref(),
                v.expire_at.map(|d| time::to_ts(d) as u64),
                time::now_ts_ms() as u64,
            );
            (
                v.verified_at.map(|d| d.to_string()),
                v.expire_at.map(|d| d.to_string()),
                v.is_candidate != 0,
                format!("{:?}", layer),
            )
        } else {
            (None, None, false, "L1".to_string())
        };

        result_items.push(IdentityItemCaRes {
            id: m.id,
            apx_id: m.apx_id,
            vdr_id: m.vdr_id,
            public_key: m.public_key,
            info: m.info,
            verified_at,
            expire_at,
            is_candidate,
            identity_layer,
            created_at: m.created_at.to_string(),
            updated_at: m.updated_at.to_string(),
        });
    }

    Ok(SearchIdentitiesCaRes {
        total,
        items: result_items,
    })
}

pub async fn get_identity(
    conn: &DatabaseConnection,
    ids: &JwtIDs,
    pubkey: String,
    config_manager: Arc<ConfigManager>,
) -> Result<GetIdentityCaRes, ApiError> {
    // 自分自身の CA 公開鍵を取得
    let ca_pubkey = get_my_ca_pubkey_hex(&config_manager)?;

    let identity = find_ca_identities_base(ids.apx_id as i32, ids.vdr_id as i32)
        .filter(identities::Column::PublicKey.eq(pubkey))
        .one(conn)
        .await
        .map_err(|e| {
            ApiError::new_system(ST_INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, e.to_string())
        })?
        .ok_or_else(|| {
            ApiError::new_system(ST_NOT_FOUND, rterr::ERR_NOT_FOUND, "Identity not found.")
        })?;

    // verifications テーブルから検証データを取得
    let verification = verifications::Entity::find()
        .filter(verifications::Column::NodePubkey.eq(&identity.public_key))
        .filter(verifications::Column::CaPubkey.eq(&ca_pubkey))
        .one(conn)
        .await
        .map_err(|e| {
            ApiError::new_system(ST_INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, e.to_string())
        })?;

    let (verified_at, expire_at, is_candidate, identity_layer) = if let Some(v) = verification {
        let layer = identities_bl::determine_layer(
            &config_manager,
            &identity.public_key,
            &ca_pubkey,
            &config_manager
                .settings
                .read()
                .server
                .my_base_url
                .clone()
                .unwrap_or_default(),
            v.signature.as_deref(),
            v.ca_token.as_deref(),
            v.expire_at.map(|d| time::to_ts(d) as u64),
            time::now_ts_ms() as u64,
        );
        (
            v.verified_at.map(|d| d.to_string()),
            v.expire_at.map(|d| d.to_string()),
            v.is_candidate != 0,
            format!("{:?}", layer),
        )
    } else {
        (None, None, false, "L1".to_string())
    };

    Ok(GetIdentityCaRes {
        id: identity.id,
        apx_id: identity.apx_id,
        vdr_id: identity.vdr_id,
        public_key: identity.public_key,
        info: identity.info,
        verified_at,
        expire_at,
        is_candidate,
        identity_layer,
        created_at: identity.created_at.to_string(),
        updated_at: identity.updated_at.to_string(),
    })
}

pub async fn entry_identity_ca(
    conn: &DatabaseConnection,
    req: EntryIdentityCaReq,
    config_manager: Arc<ConfigManager>,
) -> Result<EntryIdentityCaRes, ApiError> {
    if req.public_key.len() != ED448_PUBKEY_HEX_LEN
        || !req.public_key.chars().all(|c| c.is_ascii_hexdigit())
    {
        return Err(ApiError::new_system(
            ST_BAD_REQUEST,
            rterr::ERR_INVALID_REQUEST,
            "Invalid public_key.",
        ));
    }

    // 0. PoP (Proof of Possession) の検証
    // PoP は公開鍵自体を署名する。
    // 検証: Signature(秘密鍵, 公開鍵バイト列)

    // 公開鍵をデコード
    let pub_bytes = hex::decode(&req.public_key).map_err(|_| {
        ApiError::new_system(
            ST_BAD_REQUEST,
            rterr::ERR_INVALID_REQUEST,
            "Invalid public_key hex.",
        )
    })?;
    let mut pub_arr = [0u8; ED448_KEY_BYTES_LEN];
    pub_arr.copy_from_slice(&pub_bytes);

    // 署名をデコード
    let sig_bytes = hex::decode(&req.signature).map_err(|_| {
        ApiError::new_system(ST_BAD_REQUEST, ERR_INVALID_SIG, "Invalid signature hex.")
    })?;
    let mut sig_struct = crypto::Ed448Signature::default();
    sig_struct.signature.copy_from_slice(&sig_bytes);

    // クライアントの公開鍵バイト列の署名を検証
    let valid_pop = crypto::verify_signature(&pub_arr, &pub_bytes, &sig_struct).unwrap_or(false);

    if !valid_pop {
        return Err(ApiError::new_system(
            ST_BAD_REQUEST,
            ERR_INVALID_SIG,
            "Invalid PoP signature.",
        ));
    }

    // 1. 署名用の CA 鍵ペアを取得
    let ca_keypair = config_manager.get_node_keypair()?;
    let ca_pubkey_hex = hex::encode(ca_keypair.public);

    // トランザクション内で必要な値をクローン
    let pubkey_in = req.public_key.clone();
    let info_in = req.info.clone();
    let ca_pubkey_hex_in = ca_pubkey_hex.clone();

    // トランザクション開始
    let identity = conn
        .transaction::<_, identities::Model, ApiError>(|txn| {
            Box::pin(async move {
                // 2. 既存の確認または新規作成
                let existing = identities::Entity::find()
                    .filter(identities::Column::PublicKey.eq(&pubkey_in))
                    .one(txn)
                    .await
                    .map_err(|e| {
                        ApiError::new_system(
                            ST_INTERNAL_SERVER_ERROR,
                            rterr::ERR_DATABASE,
                            e.to_string(),
                        )
                    })?;

                let (identity, _created_new) = if let Some(ext) = existing {
                    // 冪等性: 既存のレコードを使用
                    (ext, false)
                } else {
                    // 新規作成: identities テーブルにはプロファイル情報のみ
                    let model = identities::ActiveModel {
                        apx_id: Set(APX_ID_ISOLATED as i32),
                        vdr_id: Set(VDR_ID_ISOLATED as i32),
                        public_key: Set(pubkey_in.clone()),
                        info: Set(info_in.map(sea_orm::JsonValue::from)),
                        ..Default::default()
                    };
                    let res = model.insert(txn).await.map_err(|e| {
                        ApiError::new_system(
                            ST_INTERNAL_SERVER_ERROR,
                            rterr::ERR_DATABASE,
                            e.to_string(),
                        )
                    })?;

                    // verifications テーブルに検証レベルを記録 (L1: PoP)
                    let ver_model = verifications::ActiveModel {
                        node_pubkey: Set(pubkey_in),
                        ca_pubkey: Set(ca_pubkey_hex_in),
                        is_candidate: Set(IS_CANDIDATE_FALSE as i8),
                        ..Default::default()
                    };
                    ver_model.insert(txn).await.map_err(|e| {
                        ApiError::new_system(
                            ST_INTERNAL_SERVER_ERROR,
                            rterr::ERR_DATABASE,
                            e.to_string(),
                        )
                    })?;

                    (res, true)
                };
                Ok(identity)
            })
        })
        .await?;

    // 3. チケット生成 (Delta Entry)
    // ノードが持っていないID、または持っているが更新が必要な（タイムスタンプが古い）レコードのみを抽出する Condition を構築
    let mut sync_cond = Condition::any();
    let mut deleted_forum_ids = Vec::new();

    if let Some(existing) = req.existing_forums {
        let mut ids = Vec::new();
        let mut update_check = Condition::any();

        for f in existing {
            if let Ok(u) = uuid::Uuid::parse_str(&f.id) {
                let id_bytes = u.as_bytes().to_vec();
                ids.push(id_bytes.clone());

                // 個別の更新チェック: (id = id_bytes AND updated_at > node_ts)
                if let Ok(node_ts) = chrono::NaiveDateTime::parse_from_str(
                    &f.updated_at,
                    crate::constants::DATE_FORMAT_STANDARD,
                ) {
                    update_check = update_check.add(
                        Condition::all()
                            .add(forums::Column::Id.eq(id_bytes))
                            .add(forums::Column::UpdatedAt.gt(node_ts)),
                    );
                }
            }
        }

        // 条件 A: リストにない ID は（有効であれば）全て取得
        if !ids.is_empty() {
            sync_cond = sync_cond.add(forums::Column::Id.is_not_in(ids.clone()));

            // 削除済みフォーラムの特定：送られてきた ID の中で論理削除されているものを取得
            let deleted_in_db = forums::Entity::find()
                .filter(forums::Column::Id.is_in(ids))
                .filter(forums::Column::DeletedAt.is_not_null())
                .all(conn)
                .await
                .map_err(|e| {
                    ApiError::new_system(
                        ST_INTERNAL_SERVER_ERROR,
                        rterr::ERR_DATABASE,
                        e.to_string(),
                    )
                })?;

            for d in deleted_in_db {
                // d.id is already Uuid
                deleted_forum_ids.push(d.id.to_string());
            }
        }
        // 条件 B: リストにある ID でも更新があれば（有効であれば）取得
        sync_cond = sync_cond.add(update_check);
    } else {
        // existing が None なら全件取得 (絞り込みなし)
        sync_cond = sync_cond.add(Expr::cust("1=1"));
    }

    // 最終的なクエリ条件： (同期対象) AND (削除されていない)
    let final_cond = Condition::all()
        .add(sync_cond)
        .add(forums::Column::DeletedAt.is_null());

    let all_forums = forums::Entity::find()
        .filter(final_cond)
        .order_by_desc(forums::Column::CreatedAt)
        .all(conn)
        .await
        .map_err(|e| {
            ApiError::new_system(ST_INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, e.to_string())
        })?;

    // CA 自身のベースURLを取得
    let (my_base_url, ca_token) = {
        let s = config_manager.settings.read();
        let url = s
            .server
            .my_base_url
            .clone()
            .expect("my_base_url must be configured at startup validation");

        let token = if let Some(enc) = &s.my_cat {
            // my_cat を復号
            let crypto_key = s.server.rt_crypto_key.clone();
            Some(crypto::decrypt(enc, &crypto_key).map_err(|e| {
                ApiError::new_system(
                    ST_INTERNAL_SERVER_ERROR,
                    ERR_DECRYPT,
                    format!("Failed to decrypt my CA token: {}", e),
                )
            })?)
        } else {
            None
        };

        (url, token)
    };

    let mut tickets = Vec::new();
    // Issued At (Unix Milliseconds)
    let issued_at_ts = time::to_ts(identity.created_at) as u64;

    for forum in all_forums {
        let forum_id_uuid = forum.id;
        let forum_id_str = forum_id_uuid.to_string();

        // デジタルチケットのペイロードを作成 (Canonical Serialization)
        let ticket_payload_struct = identities_bl::TicketPayload {
            node_pubkey: req.public_key.clone(),
            initial_balance: forum.initial_balance,
            issued_at: issued_at_ts,
            ca_pubkey: ca_pubkey_hex.clone(),
            forum_id: forum_id_str,
            forum_name: forum.name,
            forum_desc: Some(forum.description),
            ca_base_url: my_base_url.clone(),
        };

        // 署名用文字列を生成 (ここで順序が保証される)
        let payload_canonical_json = ticket_payload_struct.to_canonical_json()?;

        // チケットに署名
        let sig_struct = ca_keypair
            .sign(payload_canonical_json.as_bytes())
            .map_err(|e| ApiError::new_system(ST_INTERNAL_SERVER_ERROR, ERR_SIGN, e.to_string()))?;
        let sig_hex = hex::encode(sig_struct.signature);

        // 最終的なチケット JSON (署名を含む)
        // 配布用 JSON オブジェクトの構築
        let mut ticket_map = serde_json::to_value(&ticket_payload_struct).map_err(|e| {
            ApiError::new_system(ST_INTERNAL_SERVER_ERROR, "ERR_TICKET_GEN", e.to_string())
        })?;
        if let Some(obj) = ticket_map.as_object_mut() {
            obj.insert(
                KEY_TICKET_SIGNATURE.to_string(),
                serde_json::Value::String(sig_hex),
            );
        }

        tickets.push(ticket_map.to_string());
    }

    Ok(EntryIdentityCaRes {
        success: true,
        created_at: identity.created_at.to_string(),
        tickets,
        ca_token,
        ca_pubkey: ca_pubkey_hex,
        ca_base_url: my_base_url,
        deleted_forum_ids,
    })
}

pub async fn apply_identity_ca(
    conn: &DatabaseConnection,
    req: ApplyIdentityCaReq,
    config_manager: Arc<ConfigManager>,
) -> Result<ApplyIdentityCaRes, ApiError> {
    // 自分自身の CA 公開鍵を取得
    let ca_pubkey = get_my_ca_pubkey_hex(&config_manager)?;

    let existing = identities::Entity::find()
        .filter(identities::Column::PublicKey.eq(&req.public_key))
        .one(conn)
        .await
        .map_err(|e| {
            ApiError::new_system(ST_INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, e.to_string())
        })?;

    // トランザクション内で必要な値をクローン
    let pubkey_in = req.public_key.clone();
    let info_in = req.info.clone();
    let expire_seconds_in = req.expire_seconds;

    // トランザクション開始
    conn.transaction::<_, (), ApiError>(|txn| {
        Box::pin(async move {
            if let Some(identity) = existing {
                // 既に存在する場合
                // verifications テーブルから検証状態を確認
                let verification = verifications::Entity::find()
                    .filter(verifications::Column::NodePubkey.eq(&pubkey_in))
                    .filter(verifications::Column::CaPubkey.eq(&ca_pubkey))
                    .one(txn)
                    .await
                    .map_err(|e| {
                        ApiError::new_system(
                            ST_INTERNAL_SERVER_ERROR,
                            rterr::ERR_DATABASE,
                            e.to_string(),
                        )
                    })?;

                if let Some(v) = &verification {
                    // 期限情報を取得
                    let renew_window_days = {
                        let s = config_manager.settings.read();
                        s.server.ca_renew_window_days as i64
                    };
                    let now = time::now();

                    let is_expired_or_near = if let Some(expire_at) = v.expire_at {
                        now >= expire_at - Duration::days(renew_window_days)
                    } else {
                        false
                    };

                    // 期限切れまたは期限切れ近い場合以外は再申請を許可しない
                    if v.verified_at.is_some() && !is_expired_or_near {
                        return Err(ApiError::new_system(
                            ST_CONFLICT,
                            rterr::ERR_DUPLICATE,
                            "Already verified.",
                        ));
                    }
                    // 既に申請中または申請完了の場合は再申請を許可しない
                    if v.is_candidate != 0 && v.verified_at.is_none() {
                        return Err(ApiError::new_system(
                            ST_CONFLICT,
                            rterr::ERR_DUPLICATE,
                            "Already applied and pending.",
                        ));
                    }
                }

                // identities の info を更新
                let mut active: identities::ActiveModel = identity.into();
                active.info = Set(info_in.map(sea_orm::JsonValue::from));
                active.update(txn).await.map_err(|e| {
                    ApiError::new_system(
                        ST_INTERNAL_SERVER_ERROR,
                        rterr::ERR_DATABASE,
                        e.to_string(),
                    )
                })?;

                // verifications の is_candidate を更新
                if let Some(v) = verification {
                    let mut ver_active: verifications::ActiveModel = v.into();
                    ver_active.is_candidate = Set(IS_CANDIDATE_TRUE as i8);
                    ver_active.applied_expire_seconds = Set(expire_seconds_in as i64);
                    ver_active.update(txn).await.map_err(|e| {
                        ApiError::new_system(
                            ST_INTERNAL_SERVER_ERROR,
                            rterr::ERR_DATABASE,
                            e.to_string(),
                        )
                    })?;
                }
            } else {
                // まだ存在しない場合
                // identities テーブルにプロファイル情報を追加
                let model = identities::ActiveModel {
                    apx_id: Set(APX_ID_ISOLATED as i32),
                    vdr_id: Set(VDR_ID_ISOLATED as i32),
                    public_key: Set(pubkey_in),
                    info: Set(info_in.map(sea_orm::JsonValue::from)),
                    ..Default::default()
                };
                model.insert(txn).await.map_err(|e| {
                    ApiError::new_system(
                        ST_INTERNAL_SERVER_ERROR,
                        rterr::ERR_DATABASE,
                        e.to_string(),
                    )
                })?;

                // verifications テーブルに候補として登録
                let ver_model = verifications::ActiveModel {
                    node_pubkey: Set(req.public_key), // ここは req を使って OK (最後なので)
                    ca_pubkey: Set(ca_pubkey),
                    is_candidate: Set(IS_CANDIDATE_TRUE as i8),
                    applied_expire_seconds: Set(expire_seconds_in as i64),
                    ..Default::default()
                };
                ver_model.insert(txn).await.map_err(|e| {
                    ApiError::new_system(
                        ST_INTERNAL_SERVER_ERROR,
                        rterr::ERR_DATABASE,
                        e.to_string(),
                    )
                })?;
            }
            Ok(())
        })
    })
    .await?;

    Ok(ApplyIdentityCaRes {
        success: true,
        message: "Applied successfully.".to_string(),
    })
}

pub async fn verify_identity_ca(
    conn: &DatabaseConnection,
    config_manager: Arc<ConfigManager>,
    ids: &JwtIDs,
    pubkey: String,
    req: VerifyIdentityCaReq,
) -> Result<VerifyIdentityCaRes, ApiError> {
    let identity = find_ca_identities_base(ids.apx_id as i32, ids.vdr_id as i32)
        .filter(identities::Column::PublicKey.eq(&pubkey))
        .one(conn)
        .await
        .map_err(|e| {
            ApiError::new_system(ST_INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, e.to_string())
        })?
        .ok_or_else(|| ApiError::new_system(ST_NOT_FOUND, rterr::ERR_NOT_FOUND, "Not found."))?;

    // 自分自身の CA 公開鍵を取得
    let ca_pubkey = get_my_ca_pubkey_hex(&config_manager)?;

    // verifications テーブルから申請時の希望期限を取得
    let verification = verifications::Entity::find()
        .filter(verifications::Column::NodePubkey.eq(&pubkey))
        .filter(verifications::Column::CaPubkey.eq(&ca_pubkey))
        .one(conn)
        .await
        .map_err(|e| {
            ApiError::new_system(ST_INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, e.to_string())
        })?
        .ok_or_else(|| {
            ApiError::new_system(
                ST_INTERNAL_SERVER_ERROR,
                ERR_EXPECTED_RECORD,
                "Verification record not found. Please apply first.",
            )
        })?;

    let now = time::now();
    let expire_at = now + Duration::seconds(verification.applied_expire_seconds);

    // 検証レベルと CA トークンの決定
    // 1. 自身の my_cat (CA トークン) を持っているか確認
    let my_cat_enc = {
        let s = config_manager.settings.read();
        s.my_cat.clone()
    };

    let ca_token_to_imprint = if let Some(enc) = my_cat_enc {
        // my_cat を復号
        let crypto_key = {
            let s = config_manager.settings.read();
            s.server.rt_crypto_key.clone()
        };
        let decrypted = crypto::decrypt(&enc, &crypto_key).map_err(|e| {
            ApiError::new_system(
                ST_INTERNAL_SERVER_ERROR,
                ERR_DECRYPT,
                format!("Failed to decrypt my CA token: {}", e),
            )
        })?;

        // CA トークンあり -> L3 (公認市民)
        Some(decrypted)
    } else {
        // CA トークンなし -> L2 (検証済みピア)
        None
    };

    // レスポンス用の値を退避
    let res_id = identity.id;
    let res_pubkey = identity.public_key.clone();

    // トランザクション開始
    conn.transaction::<_, (), ApiError>(|txn| {
        Box::pin(async move {
            let mut active: identities::ActiveModel = identity.into();
            active.updated_at = Set(now);
            active.update(txn).await.map_err(|e| {
                ApiError::new_system(ST_INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, e.to_string())
            })?;

            // verifications テーブルを更新
            let mut ver_active: verifications::ActiveModel = verification.into();
            ver_active.signature = Set(Some(req.signature));
            ver_active.ca_token = Set(ca_token_to_imprint);
            ver_active.is_candidate = Set(IS_CANDIDATE_FALSE as i8);
            ver_active.verified_at = Set(Some(now));
            ver_active.expire_at = Set(Some(expire_at));
            ver_active.updated_at = Set(now);
            ver_active.update(txn).await.map_err(|e| {
                ApiError::new_system(ST_INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, e.to_string())
            })?;
            Ok(())
        })
    })
    .await?;

    Ok(VerifyIdentityCaRes {
        id: res_id,
        public_key: res_pubkey,
        verified_at: now.to_string(),
        expire_at: expire_at.to_string(),
    })
}

pub async fn sync_identity_ca(
    conn: &DatabaseConnection,
    config_manager: Arc<ConfigManager>,
    pubkey: String,
) -> Result<SyncIdentityCaRes, ApiError> {
    let ca_pubkey = identities_bl::get_pubkey(config_manager.clone())
        .await
        .map_err(|e| {
            ApiError::new_system(ST_INTERNAL_SERVER_ERROR, ERR_CA_PUBKEY, e.to_string())
        })?;

    let identity = identities::Entity::find()
        .filter(identities::Column::PublicKey.eq(&pubkey))
        .one(conn)
        .await
        .map_err(|e| {
            ApiError::new_system(ST_INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, e.to_string())
        })?
        .ok_or_else(|| ApiError::new_system(ST_NOT_FOUND, rterr::ERR_NOT_FOUND, "Not found."))?;

    // verifications テーブルから検証データを取得
    let verification = verifications::Entity::find()
        .filter(verifications::Column::NodePubkey.eq(&pubkey))
        .filter(verifications::Column::CaPubkey.eq(&ca_pubkey))
        .one(conn)
        .await
        .map_err(|e| {
            ApiError::new_system(ST_INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, e.to_string())
        })?;

    // 検証レコードがない、または署名がない場合は 202 Accepted (Pending) を返す
    let v = verification.ok_or_else(|| {
        ApiError::new_system(
            StatusCode::ACCEPTED,
            ERR_VERIFICATION_PENDING,
            "Verification is still pending (no record).",
        )
    })?;

    if v.signature.is_none() {
        return Err(ApiError::new_system(
            StatusCode::ACCEPTED,
            ERR_VERIFICATION_PENDING,
            "Verification is still pending (no signature).",
        ));
    }

    let verified_at = v.verified_at.map(|d| d.to_string());
    let expire_at = v.expire_at.map(|d| d.to_string());
    let is_candidate = v.is_candidate != 0;
    let signature = v.signature;
    let ca_token = v.ca_token;
    let ca_base_url = config_manager
        .settings
        .read()
        .server
        .my_base_url
        .clone()
        .unwrap_or_default();

    let layer = identities_bl::determine_layer(
        &config_manager,
        &pubkey,
        &ca_pubkey,
        &ca_base_url,
        signature.as_deref(),
        ca_token.as_deref(),
        v.expire_at.map(|d| time::to_ts(d) as u64), // signature has been moved? No, signature is cloned or Copy? String is Clone.
        time::now_ts_ms() as u64,
    );
    let identity_layer = format!("{:?}", layer);

    Ok(SyncIdentityCaRes {
        identity: GetIdentityCaRes {
            id: identity.id,
            apx_id: identity.apx_id,
            vdr_id: identity.vdr_id,
            public_key: identity.public_key,
            info: identity.info,
            verified_at,
            expire_at,
            is_candidate,
            identity_layer,
            created_at: identity.created_at.to_string(),
            updated_at: identity.updated_at.to_string(),
        },
        signature,
        ca_token,
        ca_pubkey,
        ca_base_url,
    })
}

pub async fn delete_identity_ca(
    conn: &DatabaseConnection,
    ju: &JwtUsr,
    ids: &JwtIDs,
    pubkey: String,
    config_manager: Arc<ConfigManager>,
) -> Result<DeleteIdentityCaRes, ApiError> {
    // 自分自身の CA 公開鍵を取得
    let ca_pubkey = get_my_ca_pubkey_hex(&config_manager)?;

    let identity = identities::Entity::find()
        .filter(identities::Column::PublicKey.eq(pubkey))
        .one(conn)
        .await
        .map_err(|e| {
            ApiError::new_system(ST_INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, e.to_string())
        })?
        .ok_or_else(|| ApiError::new_system(ST_NOT_FOUND, rterr::ERR_NOT_FOUND, "Not found."))?;

    let is_authorized = match ju.role() {
        JwtRole::APX => {
            (identity.apx_id == ids.apx_id as i32) || (identity.apx_id == 0 && identity.vdr_id == 0)
        }
        JwtRole::VDR | JwtRole::USR => {
            identity.apx_id == ids.apx_id as i32 && identity.vdr_id == ids.vdr_id as i32
        }
        _ => false,
    };

    if !is_authorized {
        return Err(ApiError::new_system(
            ST_NOT_FOUND,
            rterr::ERR_NOT_FOUND,
            "Not found.",
        ));
    }

    // レスポンス用の値を退避
    let res_id = identity.id;

    // トランザクション開始
    conn.transaction::<_, (), ApiError>(|txn| {
        Box::pin(async move {
            identities::Entity::delete_by_id(identity.id)
                .exec(txn)
                .await
                .map_err(|e| {
                    ApiError::new_system(
                        ST_INTERNAL_SERVER_ERROR,
                        rterr::ERR_DATABASE,
                        e.to_string(),
                    )
                })?;

            // 自身の verification レコードも削除
            verifications::Entity::delete_many()
                .filter(verifications::Column::NodePubkey.eq(&identity.public_key))
                .filter(verifications::Column::CaPubkey.eq(&ca_pubkey))
                .exec(txn)
                .await
                .map_err(|e| {
                    ApiError::new_system(
                        ST_INTERNAL_SERVER_ERROR,
                        rterr::ERR_DATABASE,
                        e.to_string(),
                    )
                })?;
            Ok(())
        })
    })
    .await?;

    Ok(DeleteIdentityCaRes {
        id: res_id,
        deleted: true,
    })
}
