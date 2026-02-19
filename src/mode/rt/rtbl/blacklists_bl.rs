use crate::{
    mode::rt::{
        client::secure_client::SecureClient,
        rtres::{errs_res::ApiError, ca_blacklists_res::SyncBlacklistCaRes},
        rtreq::ca_blacklists_req::{ReportBlacklistCaReq, SyncBlacklistCaReq},
        rtutils::db_for_rt::DbPoolsExt,
        rtbl::identities_bl,
    },
    utils::{crypto::{self, Ed448Signature}, db::DbPools, time},
    stt_config::ConfigManager,
    entities::blacklists,
    constants::{
        ED448_KEY_BYTES_LEN, ED448_SIGNATURE_BYTES_LEN, TIMESTAMP_TOLERANCE_MS,
        ERR_DECODE, ERR_INVALID_KEY, ERR_INVALID_SIG, ERR_SIG_FAIL, ERR_LOW_LEVEL,
        PATH_BLACKLISTS_REPORT, PATH_BLACKLISTS_SYNC,
        BLACKLIST_CLEANUP_MARGIN_HOURS, ERR_DB,
    },
};
use axum::http::StatusCode;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set, TransactionTrait,
    sea_query::{Expr, ExprTrait}, PaginatorTrait,
};
use serde::{Deserialize, Serialize};

// ============================================================
// Data Structures
// ============================================================

/// 犯罪の種別
#[derive(Debug, Clone, Copy, Serialize, Deserialize, utoipa::ToSchema, PartialEq, Eq)]
pub enum CrimeType {
    /// 時刻詐称 (Timestamp Fraud)
    TimestampFraud = 1,
    /// 予算不正 (Budget Fraud / Double Spending)
    BudgetFraud = 2,
    /// CA投票割り当て量不正 (CA Vote Allocated Fraud / Tampering)
    CaVoteAllocatedFraud = 3,
}

impl CrimeType {
    /// 刑期 (懲役時間) を取得する
    pub fn prison_term_hours(&self) -> i64 {
        match self {
            CrimeType::TimestampFraud => 72,         // 72時間
            CrimeType::BudgetFraud => 1_752_000,    // 200年 (永久追放)
            CrimeType::CaVoteAllocatedFraud => 1_752_000, // 200年 (永久追放)
        }
    }
}

/// 犯罪の具体的な証拠データ
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(tag = "type")]
pub enum CrimeDetail {
    /// 時刻詐称 (Timestamp Fraud)
    TimestampFraud {
        /// 相手が主張した「不正な」タイムスタンプ
        wrong_timestamp: i64,
        /// 自分の時刻との誤差 (ms)
        time_diff_ms: i64,
    },
    /// 予算不正 (Budget Fraud / Double Spending)
    BudgetFraud {
        /// 対象のフォーラムID
        forum_id: String,
        /// ノードが申告した割り当て量
        reported_vote_allocated: i32,
        /// CAが期待する（記録している）割り当て量
        expected_vote_allocated: i32,
        /// 証拠となる投票のタイムスタンプ
        timestamp: i64,
        /// 関連する CA の BASE URL
        ca_base_url: String,
    },
    /// CA投票割り当て量不正 (CA Vote Allocated Fraud)
    CaVoteAllocatedFraud {
        /// 対象のフォーラムID
        forum_id: String,
        /// ノードの署名付き原本における割り当て量
        original_vote_allocated: i32,
        /// CAが改竄・提示した割り当て量
        tampered_vote_allocated: i32,
        /// 原本の署名データ (Hex)
        original_signature: String,
        /// 原本の署名対象ペイロード (Hex)
        original_payload: String,
        /// 関連する CA の BASE URL
        ca_base_url: String,
    },
}

/// 犯罪の証拠 (統合構造)
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema, garde::Validate)]
pub struct CrimeEvidence {
    /// 罪の詳細 (種類と具体的なデータ)
    #[garde(skip)]
    pub detail: CrimeDetail,
    /// 不正を行ったノードの公開鍵
    #[garde(skip)]
    pub target_pubkey: String,
    /// 観測者が「正しい」と判断した時刻（非署名、判決日）
    #[garde(skip)]
    pub observed_at: i64,
    /// 署名データ (Hex)
    #[garde(skip)]
    pub signature: String,
    /// 署名対象の生メッセージ (Hex)
    #[garde(skip)]
    pub signed_payload: String,
}

impl CrimeEvidence {
    /// 犯罪種別を取得する
    pub fn crime_type(&self) -> CrimeType {
        match self.detail {
            CrimeDetail::TimestampFraud { .. } => CrimeType::TimestampFraud,
            CrimeDetail::BudgetFraud { .. } => CrimeType::BudgetFraud,
            CrimeDetail::CaVoteAllocatedFraud { .. } => CrimeType::CaVoteAllocatedFraud,
        }
    }
}

// ============================================================
// Validation Logic
// ============================================================

/// 証拠の構造的整合性と署名を検証する (罪の重さは問わない)
pub fn check_evidence_structure(evidence: &CrimeEvidence) -> Result<(), ApiError> {
    // 1. 公開鍵の形式チェック
    let pubkey_bytes = hex::decode(&evidence.target_pubkey).map_err(|e| {
        ApiError::new_system(StatusCode::BAD_REQUEST, ERR_DECODE, format!("Invalid pubkey hex: {}", e))
    })?;

    if pubkey_bytes.len() != ED448_KEY_BYTES_LEN {
        return Err(ApiError::new_system(StatusCode::BAD_REQUEST, ERR_INVALID_KEY, "Invalid pubkey length"));
    }

    let mut pub_arr = [0u8; ED448_KEY_BYTES_LEN];
    pub_arr.copy_from_slice(&pubkey_bytes);

    // 2. 署名の形式チェック
    let sig_bytes = hex::decode(&evidence.signature).map_err(|e| {
        ApiError::new_system(StatusCode::BAD_REQUEST, ERR_DECODE, format!("Invalid signature hex: {}", e))
    })?;

    if sig_bytes.len() != ED448_SIGNATURE_BYTES_LEN {
        return Err(ApiError::new_system(StatusCode::BAD_REQUEST, ERR_INVALID_SIG, "Invalid signature length"));
    }

    let mut sig_arr = [0u8; ED448_SIGNATURE_BYTES_LEN];
    sig_arr.copy_from_slice(&sig_bytes);
    let signature = Ed448Signature { signature: sig_arr };

    // 3. メッセージの一貫性検証
    let msg_raw = hex::decode(&evidence.signed_payload).map_err(|e| {
        ApiError::new_system(StatusCode::BAD_REQUEST, ERR_DECODE, format!("Invalid signed_payload hex: {}", e))
    })?;

    // 犯罪種別ごとのペイロード検証ロジック
    match &evidence.detail {
        CrimeDetail::TimestampFraud { wrong_timestamp, .. } => {
            let ts_bytes = wrong_timestamp.to_be_bytes();
            if msg_raw != ts_bytes {
                return Err(ApiError::new_system(StatusCode::BAD_REQUEST, ERR_LOW_LEVEL, "signed_payload do not match wrong_timestamp"));
            }
        },
        CrimeDetail::BudgetFraud { reported_vote_allocated, .. } => {
            // BudgetFraud: ノードが署名したペイロード(vote_allocated)が含まれていることを確認
             match String::from_utf8(msg_raw.clone()) {
                Ok(s) => {
                    // ペイロードが特定のフォーマットであることを期待 (例: "vote_allocated:123")
                    let expected = format!("vote_allocated:{}", reported_vote_allocated);
                    if !s.contains(&expected) {
                         return Err(ApiError::new_system(StatusCode::BAD_REQUEST, ERR_LOW_LEVEL, format!("signed_payload does not contain reported_vote_allocated '{}'. Payload: {}", expected, s)));
                    }
                },
                Err(_) => {
                     // バイナリの場合などは別途デコードが必要。一旦UTF-8文字列としてチェック。
                     return Err(ApiError::new_system(StatusCode::BAD_REQUEST, ERR_DECODE, "signed_payload is not valid utf8 for BudgetFraud"));
                }
            }
        },
        CrimeDetail::CaVoteAllocatedFraud { original_vote_allocated, tampered_vote_allocated, original_payload, .. } => {
            // 1. 原本(original_payload) に original_vote_allocated が含まれているか
             match String::from_utf8(hex::decode(original_payload).unwrap_or_default()) {
                Ok(s) => {
                    let expected = format!("vote_allocated:{}", original_vote_allocated);
                    if !s.contains(&expected) {
                         return Err(ApiError::new_system(StatusCode::BAD_REQUEST, ERR_LOW_LEVEL, format!("original_payload does not contain original_vote_allocated '{}'. Payload: {}", expected, s)));
                    }
                },
                Err(_) => {}
            }

            // 2. 改竄データ(signed_payload by CA) に tampered_vote_allocated が含まれているか
            match String::from_utf8(msg_raw.clone()) {
                Ok(s) => {
                    // CAのレスポンス形式に依存するが、ここでは簡易チェック
                    let expected = format!("vote_allocated:{}", tampered_vote_allocated);
                    if !s.contains(&expected) {
                         return Err(ApiError::new_system(StatusCode::BAD_REQUEST, ERR_LOW_LEVEL, format!("signed_payload (CA) does not contain tampered_vote_allocated '{}'. Payload: {}", expected, s)));
                    }
                },
                Err(_) => {
                     return Err(ApiError::new_system(StatusCode::BAD_REQUEST, ERR_DECODE, "signed_payload is not valid utf8 for CaVoteAllocatedFraud"));
                }
            }
        }
    }

    // 4. 署名検証
    let is_valid = crypto::verify_signature(&pub_arr, &msg_raw, &signature).map_err(|e| {
        ApiError::new_system(StatusCode::INTERNAL_SERVER_ERROR, ERR_SIG_FAIL, format!("Signature verification error: {}", e))
    })?;

    if !is_valid {
        return Err(ApiError::new_system(StatusCode::FORBIDDEN, ERR_INVALID_SIG, "Invalid evidence signature"));
    }

    Ok(())
}

/// 証拠が数学的に正しいか検証する (構造 + 罪の証明)
pub fn validate_evidence(evidence: &CrimeEvidence) -> Result<(), ApiError> {
    check_evidence_structure(evidence)?;

    match &evidence.detail {
        CrimeDetail::TimestampFraud { wrong_timestamp, .. } => {
            let diff = (evidence.observed_at - *wrong_timestamp).abs();
            if diff <= TIMESTAMP_TOLERANCE_MS {
                return Err(ApiError::new_system(StatusCode::BAD_REQUEST, ERR_LOW_LEVEL, format!("Time difference {}ms is within tolerance (Not a crime)", diff)));
            }
        },
        CrimeDetail::BudgetFraud { reported_vote_allocated, expected_vote_allocated, .. } => {
            if *reported_vote_allocated == *expected_vote_allocated {
                return Err(ApiError::new_system(StatusCode::BAD_REQUEST, ERR_LOW_LEVEL, "Reported and expected vote_allocated match (Not a crime)"));
            }
        },
        CrimeDetail::CaVoteAllocatedFraud { original_vote_allocated, tampered_vote_allocated, .. } => {
            if *original_vote_allocated == *tampered_vote_allocated {
                return Err(ApiError::new_system(StatusCode::BAD_REQUEST, ERR_LOW_LEVEL, "Original and tampered vote_allocated match (Not a crime)"));
            }
        }
    }

    Ok(())
}

// ============================================================
// DB Operations
// ============================================================

/// ブラックリストに証拠を追加または更新する (Upsert)
pub async fn add_to_blacklist(db: &DbPools, evidence: CrimeEvidence) -> Result<(), ApiError> {
    let conn = db.get_rw_for_rt()?;
    let evidence_json = serde_json::to_string(&evidence).map_err(|e| {
        ApiError::new_system(StatusCode::INTERNAL_SERVER_ERROR, ERR_LOW_LEVEL, format!("Failed to serialize evidence: {}", e))
    })?;

    let txn = conn.begin().await.map_err(|e| {
        ApiError::new_system(StatusCode::INTERNAL_SERVER_ERROR, ERR_DB, format!("Failed to begin transaction: {}", e))
    })?;

    let existing = blacklists::Entity::find()
        .filter(blacklists::Column::TargetPubkey.eq(&evidence.target_pubkey))
        .one(&txn)
        .await
        .map_err(|e| {
            ApiError::new_system(StatusCode::INTERNAL_SERVER_ERROR, ERR_DB, format!("Failed to find blacklist: {}", e))
        })?;

    let crime_type = evidence.crime_type();
    let prison_term = crime_type.prison_term_hours();

    if let Some(model) = existing {
        let mut active: blacklists::ActiveModel = model.into();
        active.evidence_json = Set(evidence_json);
        active.crime_type = Set(crime_type as i32);
        active.observed_at = Set(evidence.observed_at);
        active.prison_term_hours = Set(prison_term);
        active.updated_at = Set(time::now());
        active.update(&txn).await.map_err(|e| {
            ApiError::new_system(StatusCode::INTERNAL_SERVER_ERROR, ERR_DB, format!("Failed to update blacklist: {}", e))
        })?;
    } else {
        let active = blacklists::ActiveModel {
            target_pubkey: Set(evidence.target_pubkey),
            evidence_json: Set(evidence_json),
            crime_type: Set(crime_type as i32),
            observed_at: Set(evidence.observed_at),
            prison_term_hours: Set(prison_term),
            created_at: Set(time::now()),
            updated_at: Set(time::now()),
            ..Default::default()
        };
        active.insert(&txn).await.map_err(|e| {
            ApiError::new_system(StatusCode::INTERNAL_SERVER_ERROR, ERR_DB, format!("Failed to insert blacklist: {}", e))
        })?;
    }

    txn.commit().await.map_err(|e| {
        ApiError::new_system(StatusCode::INTERNAL_SERVER_ERROR, ERR_DB, format!("Failed to commit transaction: {}", e))
    })?;

    Ok(())
}

/// 期限切れのブラックリストを削除する (SQL 最適化版)
pub async fn delete_expired_blacklists(db: &DbPools) -> Result<u64, ApiError> {
    let conn = db.get_rw_for_rt()?;
    let now = time::now_ts_ms() as i64;
    let margin_ms = (BLACKLIST_CLEANUP_MARGIN_HOURS * 60 * 60 * 1000) as i64;

    let result = blacklists::Entity::delete_many()
        .filter(
            Expr::col(blacklists::Column::ObservedAt).lt(
                Expr::val(now).sub(
                    Expr::col(blacklists::Column::PrisonTermHours).mul(3_600_000).add(margin_ms)
                )
            )
        )
        .exec(conn)
        .await
        .map_err(|e| ApiError::new_system(StatusCode::INTERNAL_SERVER_ERROR, ERR_DB, format!("Failed to cleanup expired blacklists: {}", e)))?;

    if result.rows_affected > 0 {
        log::info!("<Blacklist> Cleaned up {} expired blacklist records (SQL optimized).", result.rows_affected);
    }

    Ok(result.rows_affected)
}

/// 指定された公開鍵がブラックリストに含まれているか確認する (SQL 最適化版)
pub async fn is_blacklisted(db: &DbPools, pubkey: &str) -> Result<bool, ApiError> {
    let conn = db.get_ro_for_rt()?;
    let now = time::now_ts_ms() as i64;

    let count = blacklists::Entity::find()
        .filter(blacklists::Column::TargetPubkey.eq(pubkey))
        .filter(
            Expr::val(now).sub(Expr::col(blacklists::Column::ObservedAt)).lt(
                Expr::col(blacklists::Column::PrisonTermHours).mul(3_600_000)
            )
        )
        .count(conn)
        .await
        .map_err(|e| {
            ApiError::new_system(StatusCode::INTERNAL_SERVER_ERROR, ERR_DB, format!("Failed to check blacklisted: {}", e))
        })?;

    Ok(count > 0)
}

/// 指定された時刻以降に更新されたブラックリストを取得する
pub async fn get_blacklists_since(db: &DbPools, since_ts: i64) -> Result<Vec<blacklists::Model>, ApiError> {
    let conn = db.get_ro_for_rt()?;
    blacklists::Entity::find()
        .filter(blacklists::Column::UpdatedAt.gt(time::from_ts_ms(since_ts)))
        .all(conn)
        .await
        .map_err(|e| {
            ApiError::new_system(StatusCode::INTERNAL_SERVER_ERROR, ERR_DB, format!("Failed to get blacklists since {}: {}", since_ts, e))
        })
}

/// ブラックリスト全件を取得する
pub async fn get_blacklist_all(db: &DbPools) -> Result<Vec<blacklists::Model>, ApiError> {
    let conn = db.get_ro_for_rt()?;
    blacklists::Entity::find().all(conn).await.map_err(|e| {
        ApiError::new_system(StatusCode::INTERNAL_SERVER_ERROR, ERR_DB, format!("Failed to get all blacklists: {}", e))
    })
}

// ============================================================
// Network / Sync Logic
// ============================================================

/// 不正証拠を CA に報告する (Relay)
pub async fn report_crime_to_ca(
    evidence: &CrimeEvidence,
    ca_base_url: &str,
    client: &SecureClient
) -> Result<(), ApiError> {
    let url = format!("{}{}", ca_base_url.trim_end_matches('/'), PATH_BLACKLISTS_REPORT);
    log::debug!("<Blacklist> Reporting crime to CA: {} -> target: {}", url, evidence.target_pubkey);
    
    let req = ReportBlacklistCaReq {
        evidence: evidence.clone(),
    };

    client.post_ca::<serde_json::Value, _, _>(&url, &req).await.map(|_| ()).map_err(|e| {
        ApiError::new_system(StatusCode::BAD_GATEWAY, "CA_REPORT_FAIL", format!("Failed to report to CA: {}", e))
    })
}

/// 不正証拠を信頼できるすべての CA にブロードキャストで報告する
/// * `sender_declared_urls`: 不正を行ったノードが自己申告した CA の URL リスト (X-MyCute-CA-Base-URL 由来)。
///   これを含めることで、そのノードを管轄する CA に直接通報できる。
pub async fn report_crime_broadcast(
    db: &DbPools,
    config: &ConfigManager,
    client: &SecureClient,
    evidence: &CrimeEvidence,
    sender_declared_urls: Option<Vec<String>>
) -> Result<usize, ApiError> {
    let conn = db.get_ro_for_rt()?;
    
    // 1. 自分の信頼する CA リストを取得
    let mut target_urls = identities_bl::get_reliable_ca_urls(conn, config).await.unwrap_or_default();

    // 2. 送信元が申告した CA リストを追加 (重複排除のため一旦セットへ... としたいが順序保持も兼ねて Vec で処理)
    if let Some(declared) = sender_declared_urls {
        for url in declared {
            if !target_urls.contains(&url) {
                target_urls.push(url);
            }
        }
    }
    
    // 3. 自身の Base URL を除外 (自分自身に報告しても意味がない/ループ防止)
    let my_base_url = config.settings.read().server.my_base_url.clone().unwrap_or_default();
    if !my_base_url.is_empty() {
        target_urls.retain(|u| u.trim_end_matches('/') != my_base_url.trim_end_matches('/'));
    }

    let mut success_count = 0;
    
    for url in target_urls {
         match report_crime_to_ca(evidence, &url, client).await {
            Ok(_) => {
                success_count += 1;
            },
            Err(e) => {
                log::warn!("<Blacklist> Failed to broadcast crime report to {}: {}", url, e);
            }
         }
    }
    
    if success_count == 0 {
        // 全滅してもエラーにはせず、ログに残す程度にする（ネットワーク分断の可能性もあるため）
        log::warn!("<Blacklist> Broadcast report yielded 0 successes for target: {}", evidence.target_pubkey);
    } else {
        log::info!("<Blacklist> Broadcast report succeeded to {} CAs for target: {}", success_count, evidence.target_pubkey);
    }
    
    Ok(success_count)
}

/// CA とブラックリストを同期する
pub async fn sync_blacklists_with_ca(
    db: &DbPools,
    ca_base_url: &str,
    client: &SecureClient,
    config_manager: &ConfigManager,
) -> Result<u64, ApiError> {
    let url = format!("{}{}", ca_base_url.trim_end_matches('/'), PATH_BLACKLISTS_SYNC);
    
    // 最終同期時刻を CA ごとに管理
    let mut entry = config_manager.get_ca_entry(ca_base_url).await?;
    let last_sync_ts = entry.last_blacklist_sync_ts;
    
    log::debug!("<Blacklist> Syncing blacklists with CA: {} since {}", url, last_sync_ts);

    let req = SyncBlacklistCaReq {
        since_ts: last_sync_ts,
    };

    let res: SyncBlacklistCaRes = match client.post_ca(&url, &req).await {
        Ok(r) => r,
        Err(e) => {
            log::warn!("<Blacklist> Failed to fetch from CA: {}", e);
            // 失敗してもエラーを返さず、今回の同期をスキップする等のハンドリングも考えられるが、
            // 現状はエラーとして返す
            return Err(ApiError::new_system(StatusCode::BAD_GATEWAY, "CA_SYNC_FAIL", format!("Failed to fetch from CA: {}", e)));
        }
    };

    let mut count = 0;
    for evidence in res.items {
        if let Err(e) = validate_evidence(&evidence) {
            log::warn!("<Blacklist> Received invalid evidence from CA sync: {}", e);
            continue;
        }
        if let Ok(_) = add_to_blacklist(db, evidence).await {
            count += 1;
        }
    }

    // Watermark の更新
    entry.last_blacklist_sync_ts = time::now_ts_ms() as i64;
    config_manager.set_ca_entry(ca_base_url, entry).await?;

    Ok(count)
}
