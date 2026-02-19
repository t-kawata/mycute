use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, Set, ActiveModelTrait};
use crate::constants::{ERR_INSUFFICIENT_FUNDS, ST_BAD_GATEWAY, ST_BAD_REQUEST, ST_INTERNAL_SERVER_ERROR};
use std::sync::Arc;
use std::io::Cursor;
use crate::{
    entities::{apps, identities, tickets, verifications},
    mode::rt::{
        rtreq::{
            node_apps_req::{DiscoverAppNodeReq, AdvertiseAppNodeReq, VoteAppNodeReq},
            ca_apps_req::{DiscoverAppCaReq, AdvertiseAppCaReq, VoteAppCaReq},
        },
        rtres::{
            errs_res::ApiError,
            node_apps_res::{AppInfoNodeRes, AppInfoNodeItemRes, AdvertiseAppNodeRes, DiscoverAppNodeRes, VoteAppNodeRes, VerifyAppNodeRes},
            ca_apps_res::{DiscoverAppCaRes, AdvertiseAppCaRes, VoteAppCaRes},
        },
        rtbl::{identities_bl, blacklists_bl::{CrimeEvidence, CrimeDetail, add_to_blacklist, report_crime_broadcast}},
        rterr::rterr,
        rtutils::{apps_types::AppLayer, voting::{format_vote_payload, format_vote_receipt_payload}},
    },
    utils::{
        crypto,
        jwt::{JwtUsr, JwtIDs},
        time,
        pkg_bl::{self, AppTrustInfo, MyCuteManifest},
        db::DbPools
    },
    stt_config::{ConfigManager, MyRemPayload},
    constants::{
        APP_BUILD_TEMP_DIR_PREFIX, ERR_IO, ERR_INVALID_ZIP, ERR_EXTRACT_ZIP, APP_MANIFEST_FILENAME,
        ERR_READ_MANIFEST, ERR_PARSE_MANIFEST, APP_BUILD_FILE_EXTENSION, APP_BUILD_DEFAULT_FILENAME,
        APP_BUILD_DIST_DIRNAME, ERR_BUILD_FAILED, ERR_READ_OUTPUT, MYCUTE_DATA_DIRNAME, MYCUTE_APPS_DIRNAME,
        APP_INSTALL_TEMP_DIR_PREFIX, APP_INSTALL_PACKAGE_FILENAME, APP_VERIFY_TEMP_DIR_PREFIX,
        APP_VERIFY_PACKAGE_FILENAME, APP_TEMP_EXTRACT_DIRNAME,
        ERR_WRITE_PKG, ERR_EXTRACT_FAILED, ERR_REMOVE_OLD, ERR_INSTALL_IO,
        ERR_CA_UNREACHABLE,
        ERR_CA_ERROR, ERR_INVALID_CA_RESPONSE, APP_BUILD_WORK_DIRNAME,
        PATH_CA_APPS_DISCOVER, PATH_CA_APPS_ADVERTISE, PATH_CA_APPS_VOTE,
        ERR_SIGN, ERR_SAVE, ED448_KEY_BYTES_LEN, ED448_SIGNATURE_BYTES_LEN,
    },
    mode::rt::client::secure_client::SecureClient,
};
use crate::mode::rt::rtutils::db_for_rt::DbPoolsExt;
use sha3::{Shake256, digest::{Update, ExtendableOutput, XofReader}};
use std::path::{Path, PathBuf};

// ヘルパー: ノードに関連する全ての検証レコードを取得
pub async fn get_all_my_verifications(
    conn: &DatabaseConnection,
    my_pub_hex: &str,
) -> Result<Vec<verifications::Model>, ApiError> {
    verifications::Entity::find()
        .filter(verifications::Column::NodePubkey.eq(my_pub_hex))
        .all(conn)
        .await
        .map_err(|e: sea_orm::DbErr| ApiError::new_system(ST_INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, e.to_string()))
}

// ============================================================
// アプリのビルド (Node)
// ============================================================
pub async fn build_app_node(
    conn: &DatabaseConnection,
    _ju: &JwtUsr,
    _ids: &JwtIDs,
    zip_data: Vec<u8>,
    original_filename: String,
    config_manager: Arc<ConfigManager>,
) -> Result<(String, Vec<u8>), ApiError> {
    log::info!("<Apps> build_app (Node): zip_len={} bytes, filename={}", zip_data.len(), original_filename);

    // 1. 一時作業ディレクトリの作成
    let temp_dir = tempfile::Builder::new()
        .prefix(APP_BUILD_TEMP_DIR_PREFIX)
        .tempdir()
        .map_err(|e| ApiError::new_system(ST_INTERNAL_SERVER_ERROR, ERR_IO, format!("Failed to create temp dir: {}", e)))?;
    let temp_path = temp_dir.path();
    
    // 2. ZIPの展開
    let work_dir = temp_path.join(APP_BUILD_WORK_DIRNAME);
    std::fs::create_dir_all(&work_dir).ok();

    let reader = Cursor::new(zip_data);
    let mut zip = zip::ZipArchive::new(reader).map_err(|e| ApiError::new_system(ST_BAD_REQUEST, ERR_INVALID_ZIP, format!("Invalid zip: {}", e)))?;
    zip.extract(&work_dir).map_err(|e| ApiError::new_system(ST_BAD_REQUEST, ERR_EXTRACT_ZIP, format!("Failed to extract zip: {}", e)))?;

    // 3. 資格情報の準備 (Identity & Multi-CA Verifications)
    let key_pair = config_manager.get_node_keypair()?;
    let my_pub_hex = hex::encode(key_pair.public);

    let ver_recs = get_all_my_verifications(conn, &my_pub_hex).await?;
    
    let mut app_verifications = Vec::new();
    for v in ver_recs {
        // [重要] 署名とCAトークンが存在する「検証済み」レコードのみを抽出
        if let (Some(sig), Some(tok), Some(_ver_at), Some(exp_at)) = (v.signature, v.ca_token, v.verified_at, v.expire_at) {
            app_verifications.push(pkg_bl::AppVerification {
                ca_public_key: v.ca_pubkey,
                signature: sig,
                ca_token: tok,
                expire_at: time::to_ts(exp_at),
            });
        }
    }

    let credentials = if !app_verifications.is_empty() {
        Some(pkg_bl::IdentityCredentials {
            key_pair,
            verifications: app_verifications,
        })
    } else {
        log::warn!("No valid verifications found. Building unverified.");
        None
    };

    // 4. マニフェストの読み込みと出力ファイル名の決定
    let manifest_path = work_dir.join(APP_MANIFEST_FILENAME);
    if !manifest_path.exists() {
        return Err(ApiError::new_system(ST_BAD_REQUEST, rterr::ERR_INVALID_REQUEST, format!("{} not found in zip root.", APP_MANIFEST_FILENAME)));
    }
    let manifest_content = std::fs::read_to_string(&manifest_path).map_err(|_| ApiError::new_system(ST_BAD_REQUEST, ERR_READ_MANIFEST, "Failed to read manifest."))?;
    let manifest: MyCuteManifest = serde_json::from_str(&manifest_content).map_err(|_| ApiError::new_system(ST_BAD_REQUEST, ERR_PARSE_MANIFEST, "Failed to parse manifest."))?;
    
    let output_filename = if !original_filename.is_empty() {
        let path = Path::new(&original_filename);
        path.with_extension(APP_BUILD_FILE_EXTENSION).file_name().unwrap_or(std::ffi::OsStr::new(APP_BUILD_DEFAULT_FILENAME)).to_string_lossy().to_string()
    } else {
        format!("{}.{}.{}", manifest.name, manifest.global_app_version, APP_BUILD_FILE_EXTENSION)
    };

    // 5. ビルド実行
    let dist_dir = temp_path.join(APP_BUILD_DIST_DIRNAME);
    std::fs::create_dir_all(&dist_dir).ok();
    let output_path = dist_dir.join(&output_filename);
    pkg_bl::build_package(&config_manager, &work_dir, &output_path, credentials).map_err(|e: anyhow::Error| ApiError::new_system(ST_INTERNAL_SERVER_ERROR, ERR_BUILD_FAILED, e.to_string()))?;

    // 6. 成果物の読み込み
    let binary_data = std::fs::read(&output_path).map_err(|e| ApiError::new_system(ST_INTERNAL_SERVER_ERROR, ERR_READ_OUTPUT, e.to_string()))?;

    Ok((output_filename, binary_data))
}

// ============================================================
// 定数
// ============================================================
// パス: ~/.mycute/apps/
fn get_apps_root() -> PathBuf {
    let home = dirs::home_dir().expect("Home directory not found");
    home.join(MYCUTE_DATA_DIRNAME).join(MYCUTE_APPS_DIRNAME)
}

// ヘルパー: ディレクトリのコピー
fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    if !dst.exists() {
        std::fs::create_dir_all(dst)?;
    }
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dst_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&entry.path(), &dst_path)?;
        } else {
            std::fs::copy(entry.path(), dst_path)?;
        }
    }
    Ok(())
}
// ============================================================
// アプリのインストール (Node)
// ============================================================
pub async fn install_app_file_node(
    conn: &DatabaseConnection,
    _ju: &JwtUsr,
    ids: &JwtIDs,
    package_data: Vec<u8>,
    config_manager: Arc<ConfigManager>,
) -> Result<AppInfoNodeRes, ApiError> {
    log::info!("<Apps> install_app_file_node: size={} bytes", package_data.len());
    
    // 1. パッケージの展開とマニフェスト取得 (共通プロセス)
    let (
        _temp_dir,
        manifest,
        app_verification,
    ) = extract_package_to_temp(
        &package_data,
        APP_INSTALL_TEMP_DIR_PREFIX,
        APP_INSTALL_PACKAGE_FILENAME,
        &config_manager,
    ).await?;
    let temp_path = _temp_dir.path();
    let extract_dir = temp_path.join(APP_TEMP_EXTRACT_DIRNAME);
        
    // 5. インストール先へ移動
    let apps_root = get_apps_root();
    let install_dir = apps_root.join(&manifest.global_app_id);
    
    if install_dir.exists() {
        std::fs::remove_dir_all(&install_dir).map_err(|e| ApiError::new_system(ST_INTERNAL_SERVER_ERROR, ERR_REMOVE_OLD, e.to_string()))?;
    }
    std::fs::create_dir_all(&apps_root).ok();
    
    if let Err(_) = std::fs::rename(&extract_dir, &install_dir) {
        copy_dir_recursive(&extract_dir, &install_dir).map_err(|e| ApiError::new_system(ST_INTERNAL_SERVER_ERROR, ERR_INSTALL_IO, e.to_string()))?;
    }

    // 6. DBへの登録
    let existing = apps::Entity::find()
        .filter(apps::Column::ApxId.eq(ids.apx_id as i32))
        .filter(apps::Column::VdrId.eq(ids.vdr_id as i32))
        .filter(apps::Column::GlobalAppId.eq(uuid::Uuid::parse_str(&manifest.global_app_id).unwrap_or_default().as_bytes().to_vec()))
        .one(conn)
        .await
        .map_err(|e: sea_orm::DbErr| ApiError::new_system(ST_INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, e.to_string()))?;

    let now = time::now();
    
    let mut hasher = Shake256::default();
    hasher.update(&package_data);
    let mut xof = hasher.finalize_xof();
    let mut hash_output = [0u8; 32];
    XofReader::read(&mut xof, &mut hash_output);
    let package_hash = hex::encode(hash_output);

    let manifest_json = serde_json::to_value(&manifest).ok();
    let verifications_json = serde_json::to_value(&manifest.verifications).ok();
    let results_cache_json = serde_json::to_value(&app_verification.verifications).ok();
    let dev_pub_key = manifest.dev_public_key.clone();

    if let Some(record) = existing {
        let mut active: apps::ActiveModel = record.into();
        active.global_app_version = Set(manifest.global_app_version.clone());
        active.global_app_hash = Set(package_hash);
        active.name = Set(manifest.name.clone());
        active.author = Set(Some(manifest.author.clone()));
        active.install_path = Set(Some(install_dir.to_string_lossy().to_string()));
        active.updated_at = Set(now);
        active.dev_public_key = Set(dev_pub_key.clone());
        active.manifest_data = Set(manifest_json.clone());
        active.verifications = Set(verifications_json.clone());
        active.verification_results_cache = Set(results_cache_json.clone());
        active.update(conn).await.map_err(|e: sea_orm::DbErr| ApiError::new_system(ST_INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, e.to_string()))?;
    } else {
        let full_identity = if let Some(pub_key) = &manifest.dev_public_key {
            identities::Entity::find()
                .filter(identities::Column::PublicKey.eq(pub_key.clone()))
                .one(conn)
                .await
                .map_err(|e: sea_orm::DbErr| ApiError::new_system(ST_INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, e.to_string()))?
        } else {
            None
        };

        let model = apps::ActiveModel {
            apx_id: Set(ids.apx_id as i32),
            vdr_id: Set(ids.vdr_id as i32),
            identity_id: Set(full_identity.as_ref().map(|i| i.id).unwrap_or(0)),
            global_app_id: Set(uuid::Uuid::parse_str(&manifest.global_app_id).unwrap_or_default().as_bytes().to_vec()),
            global_app_version: Set(manifest.global_app_version.clone()),
            global_app_hash: Set(package_hash),
            name: Set(manifest.name.clone()),
            layer: Set(AppLayer::Local.to_string()),
            created_at: Set(now),
            updated_at: Set(now),
            dev_public_key: Set(dev_pub_key.clone()),
            manifest_data: Set(manifest_json.clone()),
            verifications: Set(verifications_json.clone()),
            verification_results_cache: Set(results_cache_json.clone()),
            author: Set(Some(manifest.author.clone())),
            install_path: Set(Some(install_dir.to_string_lossy().to_string())),
            ..Default::default()
        };
        
        model.insert(conn).await.map_err(|e: sea_orm::DbErr| ApiError::new_system(ST_INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, e.to_string()))?;
    }

      // 7. レスポンスの構築
    let trust = AppTrustInfo::from_manifest(
        manifest.clone(),
        app_verification,
    );
    
    Ok(AppInfoNodeRes {
        info: AppInfoNodeItemRes {
            installed_at: now.to_string(),
            trust,
        }
    })
}

// ============================================================
// アプリの検証 (Node)
// ============================================================
pub async fn verify_app_node(
    _conn: &DatabaseConnection,
    _ju: &JwtUsr,
    _ids: &JwtIDs,
    package_data: Vec<u8>,
    config_manager: Arc<ConfigManager>,
) -> Result<VerifyAppNodeRes, ApiError> {
    // 1. パッケージの展開とマニフェスト取得 (共通プロセス)
    let (
        _temp_dir,
        manifest,
        app_verification,
    ) = extract_package_to_temp(
        &package_data,
        APP_VERIFY_TEMP_DIR_PREFIX,
        APP_VERIFY_PACKAGE_FILENAME,
        &config_manager,
    ).await?;

    // 5. 信用情報の構築 (インストールは行わない)
    let trust = AppTrustInfo::from_manifest(
        manifest,
        app_verification,
    );

    Ok(VerifyAppNodeRes { trust })
}


// ============================================================
// アプリの発見 (CAへのプロキシ)
// ============================================================
pub async fn discover_app_node(client: &SecureClient, req: DiscoverAppNodeReq) -> Result<DiscoverAppNodeRes, ApiError> {
    log::debug!("<Apps> discover_app_node: Proxying to CA: {} for IDs: {:?}, query: {:?}", req.ca_base_url, req.app_ids, req.query);
    
    let url = format!("{}{}", req.ca_base_url.trim_end_matches('/'), PATH_CA_APPS_DISCOVER);
    
    let ca_req = DiscoverAppCaReq {
        app_ids: req.app_ids,
        query: req.query,
    };

    let res = client.post(&url, &ca_req)
        .await
        .map_err(|e| ApiError::new_system(ST_BAD_GATEWAY, ERR_CA_UNREACHABLE, e.to_string()))?;

    if !res.status().is_success() {
        let err_text = res.text().await.unwrap_or_default();
        return Err(ApiError::new_system(ST_BAD_GATEWAY, ERR_CA_ERROR, err_text));
    }

    let ca_res: DiscoverAppCaRes = res.json().await.map_err(|e| ApiError::new_system(ST_BAD_GATEWAY, ERR_INVALID_CA_RESPONSE, e.to_string()))?;

    Ok(DiscoverAppNodeRes { items: ca_res.items })
}

// ============================================================
// アプリの広告 (CAへのプロキシ)
// ============================================================
pub async fn advertise_app_node(client: &SecureClient, req: AdvertiseAppNodeReq) -> Result<AdvertiseAppNodeRes, ApiError> {
    log::debug!("<Apps> advertise_app_node: Proxying to CA: {} for app: {}", req.ca_base_url, req.app_id);
    
    let url = format!("{}{}", req.ca_base_url.trim_end_matches('/'), PATH_CA_APPS_ADVERTISE);
    
    let ca_req = AdvertiseAppCaReq {
        app_id: req.app_id,
    };

    let res = client.post(&url, &ca_req)
        .await
        .map_err(|e| ApiError::new_system(ST_BAD_GATEWAY, ERR_CA_UNREACHABLE, e.to_string()))?;

    if !res.status().is_success() {
        let err_text = res.text().await.unwrap_or_default();
        return Err(ApiError::new_system(ST_BAD_GATEWAY, ERR_CA_ERROR, err_text));
    }

    let ca_res: AdvertiseAppCaRes = res.json().await.map_err(|e| ApiError::new_system(ST_BAD_GATEWAY, ERR_INVALID_CA_RESPONSE, e.to_string()))?;

    Ok(AdvertiseAppNodeRes { 
        success: ca_res.success,
        advertised_nodes: ca_res.advertised_nodes 
    })
}

// ============================================================
// アプリへの投票 (CAへのプロキシ)
// ============================================================
pub async fn vote_app_node(
    db: &DbPools,
    client: &SecureClient,
    req: VoteAppNodeReq,
    config_manager: Arc<ConfigManager>,
) -> Result<VoteAppNodeRes, ApiError> {
    let conn = db.get_ro_for_rt()?;
    log::debug!("<Apps> vote_app_node: Processing vote for app: {}, value: {}, ca: {}", req.app_id, req.vote, req.ca_base_url);

    // 1. ノードのアイデンティティと鍵を取得
    let my_keypair = config_manager.get_node_keypair()?;
    let my_pub_hex = hex::encode(my_keypair.public);

    // 2. my_rem の読み込み（Multi-CA 形式）
    let mut payload = {
        let _settings = config_manager.settings.read();
        config_manager.load_my_rem_payload(&my_keypair)?
    };

    // 3. 対象 CA のエントリを取得
    let ca_entry = payload.ca_entries.get_mut(&req.ca_base_url)
        .ok_or_else(|| ApiError::new_system(
            ST_BAD_REQUEST, 
            rterr::ERR_NOT_FOUND, 
            format!("You have not performed 'Entry' with CA '{}'. Please complete the Entry process.", req.ca_base_url)
        ))?;

    // 4. 対象フォーラムの財布を取得
    let forum_state = ca_entry.forum_states.get_mut(&req.forum_id)
        .ok_or_else(|| ApiError::new_system(
            ST_BAD_REQUEST, 
            rterr::ERR_NOT_FOUND, 
            format!("You do not have a wallet for Forum '{}'. Please perform 'Entry' to receive a ticket for this forum.", req.forum_id)
        ))?;

    // 5. 既存の投票数を取得して差分を計算
    let n_old = forum_state.votes.get(&req.app_id).cloned().unwrap_or(0);
    let n_new = req.vote;
    let diff = n_new - n_old;

    // 6. 予算チェック（増票時のみ）
    if diff > 0 && forum_state.balance < diff {
        return Err(ApiError::new_system(
            ST_BAD_REQUEST, 
            ERR_INSUFFICIENT_FUNDS, 
            format!("Insufficient balance in Forum '{}'. You need {} more to vote. Your current balance is {}.", req.forum_id, diff - forum_state.balance, forum_state.balance)
        ));
    }

    // 7. チケットの収集 (該当フォーラムのみ - 単一チケット前提)
    let forum_uuid_bytes = uuid::Uuid::parse_str(&req.forum_id)
        .map_err(|_| ApiError::new_system(ST_BAD_REQUEST, rterr::ERR_INVALID_REQUEST, "Invalid forum_id."))?
        .as_bytes()
        .to_vec();

    let ticket_record = tickets::Entity::find()
        .filter(tickets::Column::CaBaseUrl.eq(&req.ca_base_url))
        .filter(tickets::Column::ForumId.eq(forum_uuid_bytes))
        .one(conn)
        .await
        .map_err(|e| ApiError::new_system(ST_INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, e.to_string()))?
        .ok_or_else(|| ApiError::new_system(ST_BAD_REQUEST, rterr::ERR_NOT_FOUND, "Ticket not found for this forum."))?;
    
    let ticket_json = ticket_record.ticket_data;

    // 8. vote_allocated の計算 (BudgetFraud 検証用)
    let ticket_payload: identities_bl::TicketPayload = serde_json::from_value(ticket_json.clone()).map_err(|e| ApiError::new_system(ST_INTERNAL_SERVER_ERROR, rterr::ERR_INVALID_REQUEST, format!("Invalid ticket format: {}", e)))?;
    
    let new_balance = forum_state.balance - diff;
    let vote_allocated = ticket_payload.initial_balance - new_balance;

    if vote_allocated < 0 {
         return Err(ApiError::new_system(ST_INTERNAL_SERVER_ERROR, rterr::ERR_INVALID_REQUEST, "Calculated vote_allocated is negative. Data inconsistency detected."));
    }

    // 9. リクエストの構築と署名
    // 数値のタイムスタンプを使用
    let timestamp = time::now_ts_ms() as i64;
    let ca_payload_str = format_vote_payload(&req.app_id, req.vote, vote_allocated, &timestamp.to_string(), &ticket_json)?;
    
    let sig_struct = my_keypair.sign(ca_payload_str.as_bytes()).map_err(|e| ApiError::new_system(ST_INTERNAL_SERVER_ERROR, ERR_SIGN, e.to_string()))?;
    let signature_hex = hex::encode(sig_struct.signature);

    let ca_req = VoteAppCaReq {
        node_pubkey: my_pub_hex,
        app_id: req.app_id.clone(),
        forum_id: req.forum_id.clone(),
        vote: req.vote,
        timestamp,
        vote_allocated,
        ticket: ticket_json,
        signature: signature_hex.clone(),
    };

    // 9. CAへ送信
    let url = format!("{}{}", req.ca_base_url.trim_end_matches('/'), PATH_CA_APPS_VOTE);
    let res = client.post(&url, &ca_req)
        .await
        .map_err(|e| ApiError::new_system(ST_BAD_GATEWAY, ERR_CA_UNREACHABLE, e.to_string()))?;

    if !res.status().is_success() {
        let err_text = res.text().await.unwrap_or_default();
        return Err(ApiError::new_system(ST_BAD_GATEWAY, ERR_CA_ERROR, err_text));
    }

    let ca_res: VoteAppCaRes = res.json().await.map_err(|e| ApiError::new_system(ST_BAD_GATEWAY, ERR_INVALID_CA_RESPONSE, e.to_string()))?;

    // 9.5 CaVoteAllocatedFraud 検証
    // CAからのレスポンスを検証し、vote_allocated の改ざんがないか確認する。
    // 検証内容:
    // 1. CA署名の検証: ca_pubkey (Ticketから取得) を使用して ca_res.signature を検証。
    //    ペイロード: "vote_allocated:{},timestamp:{}"
    // 2. vote_allocated の一致確認: req.vote_allocated == ca_res.vote_allocated
    
    let ca_pubkey_hex = ticket_record.ca_pubkey;
    let ca_pub_bytes = hex::decode(&ca_pubkey_hex).unwrap_or_default();
    let mut ca_pub_arr = [0u8; ED448_KEY_BYTES_LEN];
    if ca_pub_bytes.len() == ED448_KEY_BYTES_LEN {
        ca_pub_arr.copy_from_slice(&ca_pub_bytes);
    } // else invalid key, but we proceed to fail verification

    let ca_res_payload = format_vote_receipt_payload(ca_res.vote_allocated, ca_res.timestamp, &signature_hex);
    let ca_sig_bytes = hex::decode(&ca_res.signature).unwrap_or_default();
    let mut ca_sig_arr = [0u8; ED448_SIGNATURE_BYTES_LEN];
    if ca_sig_bytes.len() == ED448_SIGNATURE_BYTES_LEN {
        ca_sig_arr.copy_from_slice(&ca_sig_bytes);
    }
    let ca_sig_struct = crypto::Ed448Signature { signature: ca_sig_arr };
    
    let sig_valid = crypto::verify_signature(&ca_pub_arr, ca_res_payload.as_bytes(), &ca_sig_struct).unwrap_or(false);
    let amount_match = ca_res.vote_allocated == vote_allocated; // Nodeが計算した vote_allocated と一致するか

    if !sig_valid || !amount_match {
        log::warn!("<Apps> CaVoteAllocatedFraud detected from CA {}. SigValid: {}, AmountMatch: {} (Exp: {}, Got: {})", req.ca_base_url, sig_valid, amount_match, vote_allocated, ca_res.vote_allocated);
        
        let evidence = CrimeEvidence {
            detail: CrimeDetail::CaVoteAllocatedFraud {
                forum_id: req.forum_id.clone(),
                original_vote_allocated: vote_allocated, 
                tampered_vote_allocated: ca_res.vote_allocated,
                original_signature: signature_hex.clone(),
                original_payload: hex::encode(ca_payload_str.as_bytes()),
                ca_base_url: req.ca_base_url.clone(),
            },
            target_pubkey: ca_pubkey_hex.clone(), // CAに対する告発
            observed_at: crate::utils::time::now_ts_ms() as i64,
            signature: ca_res.signature.clone(), // CA不正の証拠はこの署名
            signed_payload: hex::encode(ca_res_payload.as_bytes()),
        };
        
        // 証拠保存
        if let Err(e) = add_to_blacklist(db, evidence.clone()).await {
             log::error!("<Apps> Failed to save fraud evidence to local DB: {}", e);
        }

        // ブロードキャスト
        let db_clone = Arc::new(db.clone());
        let config_clone = config_manager.clone();
        let client_clone = Arc::new(client.clone());
        let ev = evidence;

        tokio::spawn(async move {
            if let Err(e) = report_crime_broadcast(&db_clone, &config_clone, &client_clone, &ev, None).await {
                log::warn!("<Apps> Failed to broadcast CaVoteAllocatedFraud report: {}", e);
            }
        });

        return Err(ApiError::new_system(ST_BAD_GATEWAY, ERR_INVALID_CA_RESPONSE, "CA Fraud detected: Invalid signature or vote_allocated mismatch."));
    }

    // 10. コミット: ForumState の更新
    
    // 対象フォーラムの財布を再取得して更新（borrow checker 対策）
    {
        // ここでの unwrap は安全（上記でチェック済み）
        let ca_entry_mut = payload.ca_entries.get_mut(&req.ca_base_url).unwrap();
        let forum_state_mut = ca_entry_mut.forum_states.get_mut(&req.forum_id).unwrap();
        
        forum_state_mut.balance = new_balance;
        
        // 投票マップを更新（0なら削除）
        if n_new == 0 {
            forum_state_mut.votes.remove(&req.app_id);
        } else {
            forum_state_mut.votes.insert(req.app_id.clone(), n_new);
        }
    }

    // 11. my_rem 全体を保存
    {
        let mut settings = config_manager.settings.write();
        let encrypted = config_manager.encode_my_rem_payload(&payload, &my_keypair)?;
        settings.my_rem = Some(encrypted);
    }
    config_manager.save().map_err(|e| ApiError::new_system(ST_INTERNAL_SERVER_ERROR, ERR_SAVE, e))?;

    if diff > 0 {
        log::info!("<Apps> Vote success for CA '{}'. Allocated {} balance. Remaining: {}", req.ca_base_url, diff, new_balance);
    } else {
        log::info!("<Apps> Vote success for CA '{}'. Refunded {} balance. Remaining: {}", req.ca_base_url, -diff, new_balance);
    }

    Ok(VoteAppNodeRes { 
        success: true,
    })
}

/// Entry 処理専用の my_rem ロードヘルパー。
/// 空の ca_states でもエラーとせず、そのまま返す（Entry 時にデータを追加するため）。
pub fn load_my_rem_payload_for_entry(
    rem_enc: &str,
    crypto_key: &str,
    keypair: &crypto::Ed448KeyValuePair,
) -> Result<MyRemPayload, String> {
    let rem_dec = crypto::decrypt(rem_enc, crypto_key).map_err(|e| format!("Decrypt failed: {}", e))?;
    
    let parts: Vec<&str> = rem_dec.splitn(2, ':').collect();
    if parts.len() != 2 {
        return Err("Invalid my_rem format".to_string());
    }
    let json_str = parts[0];
    let sig_hex = parts[1];

    // 署名検証
    let sig_bytes = hex::decode(sig_hex).map_err(|_| "Invalid signature hex".to_string())?;
    if sig_bytes.len() != ED448_SIGNATURE_BYTES_LEN {
        return Err("Invalid signature length".to_string());
    }
    let mut sig_arr = [0u8; ED448_SIGNATURE_BYTES_LEN];
    sig_arr.copy_from_slice(&sig_bytes);
    let sig_struct = crypto::Ed448Signature { signature: sig_arr };
    
    if !crypto::verify_signature(&keypair.public, json_str.as_bytes(), &sig_struct).unwrap_or(false) {
        return Err("Signature verification failed".to_string());
    }

    // パース（空でも OK）
    let payload: MyRemPayload = serde_json::from_str(json_str).map_err(|e| format!("Parse failed: {}", e))?;
    Ok(payload)
}

// ============================================================
// 内部ヘルパー
// ============================================================

/// アップロードされたパッケージデータを一時的に展開し、マニフェストを返す内部関数。
/// 戻り値の TempDir を保持し続ける限り、ファイルは維持される。
async fn extract_package_to_temp(
    package_data: &[u8],
    dir_prefix: &str,
    pkg_filename: &str,
    config_manager: &ConfigManager,
) -> Result<(tempfile::TempDir, MyCuteManifest, pkg_bl::AppVerificationResults), ApiError> {
    // 1. 一時作業ディレクトリの作成
    let temp_dir = tempfile::Builder::new()
        .prefix(dir_prefix)
        .tempdir()
        .map_err(|e| ApiError::new_system(ST_INTERNAL_SERVER_ERROR, ERR_IO, format!("Failed to create temp dir: {}", e)))?;
    let temp_path = temp_dir.path();

    // 2. パッケージの保存
    let pkg_path = temp_path.join(pkg_filename);
    std::fs::write(&pkg_path, package_data).map_err(|e| ApiError::new_system(ST_INTERNAL_SERVER_ERROR, ERR_WRITE_PKG, format!("Failed to write package: {}", e)))?;

    // 3. 展開先ディレクトリ
    let extract_dir = temp_path.join(APP_TEMP_EXTRACT_DIRNAME);
    std::fs::create_dir_all(&extract_dir).ok();

    // 4. 展開と検証
    let (manifest, app_verification) = pkg_bl::extract_package(config_manager, &pkg_path, &extract_dir).map_err(|e| {
        ApiError::new_system(ST_BAD_REQUEST, ERR_EXTRACT_FAILED, format!("Package extraction failed: {}", e))
    })?;

    Ok((temp_dir, manifest, app_verification))
}
