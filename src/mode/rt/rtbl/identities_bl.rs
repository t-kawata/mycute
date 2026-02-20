use crate::constants::{
    ED448_KEY_BYTES_LEN, ED448_SIGNATURE_BYTES_LEN, ERR_IDENTITY_GEN, ST_INTERNAL_SERVER_ERROR,
};
use crate::mode::rt::owner_secrets::OWNER_PUB_KEY_HEX;
use crate::{
    entities::verifications,
    mode::rt::rtres::errs_res::ApiError,
    stt_config::ConfigManager,
    utils::{
        crypto::{self, verify_signature, Ed448KeyValuePair, Ed448Signature},
        time,
    },
};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedTicket {
    pub json: Value,
    pub forum_id_str: String,
    pub forum_uuid_bytes: Vec<u8>,
    pub forum_name: String,
    pub forum_desc: Option<String>,
    pub initial_balance: i32,
}

/// 署名対象となるチケットのペイロード構造体。
/// フィールドの順序に関わらず、`to_canonical_json` を通すことで一意な正規化JSON文字列を生成する。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketPayload {
    #[serde(rename = "node_pubkey")]
    pub node_pubkey: String,
    #[serde(rename = "initial_balance")]
    pub initial_balance: i32,
    #[serde(rename = "issued_at")]
    pub issued_at: u64,
    #[serde(rename = "ca_pubkey")]
    pub ca_pubkey: String,
    #[serde(rename = "forum_id")]
    pub forum_id: String,
    #[serde(rename = "forum_name")]
    pub forum_name: String,
    #[serde(rename = "forum_desc")]
    pub forum_desc: Option<String>,
    #[serde(rename = "ca_base_url")]
    pub ca_base_url: String,
}

impl TicketPayload {
    /// 署名検証用の正規化された JSON 文字列を生成する。
    /// 一度 BTreeMap に変換し、キーのアルファベット順にソートされた JSON 文字列を返すことで、
    /// 署名時のバイト列の一意性を保証する (Canonical Serialization)。
    pub fn to_canonical_json(&self) -> Result<String, ApiError> {
        // 1. serde_json::Value (Map) に変換
        let val = serde_json::to_value(self).map_err(|e| {
            ApiError::new_system(
                ST_INTERNAL_SERVER_ERROR,
                "ERR_TICKET_GEN",
                format!("Failed to serialize ticket payload: {}", e),
            )
        })?;

        // 2. Object (Map) であれば BTreeMap に変換してソート
        if let serde_json::Value::Object(map) = val {
            let sorted_map: std::collections::BTreeMap<_, _> = map.into_iter().collect();
            serde_json::to_string(&sorted_map).map_err(|e| {
                ApiError::new_system(
                    ST_INTERNAL_SERVER_ERROR,
                    "ERR_TICKET_GEN",
                    format!("Failed to generate canonical json: {}", e),
                )
            })
        } else {
            Err(ApiError::new_system(
                ST_INTERNAL_SERVER_ERROR,
                "ERR_TICKET_GEN",
                "Ticket payload is not an object.",
            ))
        }
    }
}

#[derive(
    Debug, PartialEq, PartialOrd, Eq, Ord, Clone, Copy, serde::Serialize, serde::Deserialize,
)]
pub enum IdentityLayer {
    L1, // Anonymous (Self-signed only)
    L2, // Verified (Signed by CA, but CA is not L3)
    L3, // Trust Anchor (Signed by CA, and CA has valid Token from Owner)
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, ToSchema)]
pub struct AppVerificationDetail {
    /// CA公開鍵（どのCAによる証明か）
    pub ca_public_key: String,
    /// CAがオーナーに信頼されているか
    pub ok_ca_until: Option<u64>,
    /// 開発者がCAに信頼されているか
    pub ok_dev_until: Option<u64>,
    /// アプリのパッケージ（中身と設定）が開発者本人の署名によって封印されており、改竄されていないか（L3）
    pub ok_app_sig: bool,
}

/// 指定された検証情報から IdentityLayer を判定する。
/// インメモリキャッシュを優先し、なければ署名検証を行って結果をキャッシュする。
pub fn determine_layer(
    config_manager: &ConfigManager,
    node_pubkey_hex: &str,
    ca_pubkey_hex: &str,
    ca_base_url: &str,
    signature_hex: Option<&str>,
    ca_token_hex: Option<&str>,
    expire_at_ts: Option<u64>,
    now_ts: u64,
) -> IdentityLayer {
    // 1. インメモリキャッシュの確認
    let cache_key = (node_pubkey_hex.to_string(), ca_pubkey_hex.to_string());
    if let Some(layer_val) = config_manager.identity_layer_cache.get(&cache_key) {
        // moka::sync::Cache は get 時に内部の LRU/TTL 用の統計情報を自動更新する
        return match layer_val {
            2 => IdentityLayer::L2,
            3 => IdentityLayer::L3,
            _ => IdentityLayer::L1,
        };
    }

    // 2. キャッシュがない（または期限切れ）の場合は検証を実行
    let layer = determine_layer_no_cache(
        node_pubkey_hex,
        ca_pubkey_hex,
        ca_base_url,
        signature_hex,
        ca_token_hex,
        expire_at_ts,
        now_ts,
    );

    // 3. 結果をキャッシュに保存
    let layer_val = match layer {
        IdentityLayer::L3 => 3,
        IdentityLayer::L2 => 2,
        IdentityLayer::L1 => 1,
    };
    config_manager
        .identity_layer_cache
        .insert(cache_key, layer_val);

    layer
}

/// 内部用：キャッシュを介さずに署名検証を行い判定する
fn determine_layer_no_cache(
    node_pubkey_hex: &str,
    ca_pubkey_hex: &str,
    ca_base_url: &str,
    signature_hex: Option<&str>,
    ca_token_hex: Option<&str>,
    expire_at_ts: Option<u64>,
    now_ts: u64,
) -> IdentityLayer {
    // 1. ベリフィケーション（CAからの署名）の正等性を確認
    let ca_pub_bytes = match hex::decode(ca_pubkey_hex) {
        Ok(b) if b.len() == ED448_KEY_BYTES_LEN => b,
        _ => return IdentityLayer::L1,
    };

    let exp_at = match expire_at_ts {
        Some(t) => t,
        None => return IdentityLayer::L1,
    };

    if exp_at < now_ts {
        log::debug!(
            "<IdentityBL> Developer certificate expired for CA '{}'. Downgraded to L1.",
            ca_base_url
        );
        return IdentityLayer::L1;
    }

    // 署名の検証 (CA ➔ Node)
    let node_pub_bytes = match hex::decode(node_pubkey_hex) {
        Ok(b) if b.len() == ED448_KEY_BYTES_LEN => b,
        _ => return IdentityLayer::L1,
    };
    let mut dev_msg = Vec::new();
    dev_msg.extend_from_slice(&node_pub_bytes);
    dev_msg.extend_from_slice(&exp_at.to_be_bytes());

    let dev_sig_hex = match signature_hex {
        Some(s) => s,
        None => return IdentityLayer::L1,
    };
    let dev_sig_bytes = match hex::decode(dev_sig_hex) {
        Ok(b) if b.len() == ED448_SIGNATURE_BYTES_LEN => b,
        _ => return IdentityLayer::L1,
    };
    let mut dev_sig_arr = [0u8; ED448_SIGNATURE_BYTES_LEN];
    dev_sig_arr.copy_from_slice(&dev_sig_bytes);
    let dev_sig_struct = Ed448Signature {
        signature: dev_sig_arr,
    };

    let mut ca_pub_arr = [0u8; ED448_KEY_BYTES_LEN];
    ca_pub_arr.copy_from_slice(&ca_pub_bytes);

    if !verify_signature(&ca_pub_arr, &dev_msg, &dev_sig_struct).unwrap_or(false) {
        log::warn!(
            "<IdentityBL> Developer signature verification failed for CA '{}'!",
            ca_base_url
        );
        return IdentityLayer::L1;
    }

    // --- ここまでで L2 確定。次に CA Token 検証で L3 を判定 ----

    // 2. CA Token (Owner ➔ CA) の検証
    let ca_tok_hex = match ca_token_hex {
        Some(tok) => tok,
        None => return IdentityLayer::L2,
    };

    // Owner 公開鍵の準備
    let owner_pub_bytes = match hex::decode(OWNER_PUB_KEY_HEX) {
        Ok(b) if b.len() == ED448_KEY_BYTES_LEN => b,
        _ => return IdentityLayer::L2,
    };
    let mut owner_pub_arr = [0u8; ED448_KEY_BYTES_LEN];
    owner_pub_arr.copy_from_slice(&owner_pub_bytes);

    // CA Token パース
    let parts: Vec<&str> = ca_tok_hex.split('.').collect();
    if parts.len() != 2 {
        return IdentityLayer::L2;
    }
    let ca_token_sig_hex = parts[0];
    let ca_expire_at: u64 = parts[1].parse().unwrap_or(0);

    if ca_expire_at < now_ts {
        log::debug!(
            "<IdentityBL> CA Token expired for '{}'. Downgraded to L2.",
            ca_base_url
        );
        return IdentityLayer::L2;
    }

    let mut ca_msg = Vec::new();
    ca_msg.extend_from_slice(&ca_pub_bytes);
    ca_msg.extend_from_slice(&ca_expire_at.to_be_bytes());

    let ca_sig_bytes = match hex::decode(ca_token_sig_hex) {
        Ok(b) if b.len() == ED448_SIGNATURE_BYTES_LEN => b,
        _ => return IdentityLayer::L2,
    };
    let mut ca_sig_arr = [0u8; ED448_SIGNATURE_BYTES_LEN];
    ca_sig_arr.copy_from_slice(&ca_sig_bytes);
    let ca_sig_struct = Ed448Signature {
        signature: ca_sig_arr,
    };

    if verify_signature(&owner_pub_arr, &ca_msg, &ca_sig_struct).unwrap_or(false) {
        IdentityLayer::L3
    } else {
        log::warn!(
            "<IdentityBL> CA Token signature verification failed for '{}'!",
            ca_base_url
        );
        IdentityLayer::L2
    }
}

/// オーナー署名済みの CA トークンそのものを検証する。
pub fn verify_ca_token(ca_pubkey_hex: &str, ca_token_hex: &str, now_ts: u64) -> bool {
    let ca_pub_bytes = match hex::decode(ca_pubkey_hex) {
        Ok(b) if b.len() == ED448_KEY_BYTES_LEN => b,
        _ => return false,
    };

    let owner_pub_bytes = match hex::decode(OWNER_PUB_KEY_HEX) {
        Ok(b) if b.len() == ED448_KEY_BYTES_LEN => b,
        _ => return false,
    };
    let mut owner_pub_arr = [0u8; ED448_KEY_BYTES_LEN];
    owner_pub_arr.copy_from_slice(&owner_pub_bytes);

    let parts: Vec<&str> = ca_token_hex.split('.').collect();
    if parts.len() != 2 {
        return false;
    }
    let sig_hex = parts[0];
    let expire_at: u64 = parts[1].parse().unwrap_or(0);

    if expire_at < now_ts {
        return false;
    }

    let mut ca_msg = Vec::new();
    ca_msg.extend_from_slice(&ca_pub_bytes);
    ca_msg.extend_from_slice(&expire_at.to_be_bytes());

    let sig_bytes = match hex::decode(sig_hex) {
        Ok(b) if b.len() == ED448_SIGNATURE_BYTES_LEN => b,
        _ => return false,
    };
    let mut sig_arr = [0u8; ED448_SIGNATURE_BYTES_LEN];
    sig_arr.copy_from_slice(&sig_bytes);
    let sig_struct = Ed448Signature { signature: sig_arr };

    verify_signature(&owner_pub_arr, &ca_msg, &sig_struct).unwrap_or(false)
}

// ============================================================
// Moved from node_identities_bl.rs
// ============================================================

pub async fn get_pubkey(config_manager: Arc<ConfigManager>) -> Result<String, ApiError> {
    // 1. Check if we are in Owner Mode (Using in-memory key)
    {
        let guard = config_manager.owner_key.read();
        if let Some(key) = guard.as_ref() {
            log::debug!("<IdentityBL> get_pubkey: Returning Owner Public Key.");
            return Ok(hex::encode(key.public));
        }
    }

    // 2. Ensure identity exists (Lazy initialization for non-owner modes)
    if let Err(e) = ensure_node_identity(&config_manager) {
        log::error!("Failed to ensure node identity: {}", e);
        return Err(ApiError::new_system(
            ST_INTERNAL_SERVER_ERROR,
            ERR_IDENTITY_GEN,
            "Failed to generate node identity.",
        ));
    }

    // 3. Retrieve keypair
    let keypair = config_manager.get_node_keypair()?;
    Ok(hex::encode(keypair.public))
}

pub fn ensure_node_identity(config_manager: &ConfigManager) -> anyhow::Result<()> {
    {
        let settings = config_manager.settings.read();
        if settings.my_pub.is_some() && settings.my_sec.is_some() {
            return Ok(());
        }
    }

    log::info!("Node Identity missing. Generating new Ed448 KeyPair...");

    let keypair = Ed448KeyValuePair::generate()?;
    let pub_hex = hex::encode(keypair.public);
    let sec_hex = hex::encode(keypair.secret);

    let crypto_key = {
        let settings = config_manager.settings.read();
        settings.server.rt_crypto_key.clone()
    };

    let pub_enc = crypto::encrypt(&pub_hex, &crypto_key)?;
    let sec_enc = crypto::encrypt(&sec_hex, &crypto_key)?;

    {
        let mut settings = config_manager.settings.write();
        settings.my_pub = Some(pub_enc);
        settings.my_sec = Some(sec_enc);
    }
    config_manager
        .save()
        .map_err(|e| anyhow::anyhow!("Failed to save settings: {}", e))?;

    log::info!("Node Identity generated and saved successfully.");
    Ok(())
}

pub fn determine_layer_from_verification(
    config_manager: &ConfigManager,
    ver: &verifications::Model,
    my_pubkey_hex: &str,
) -> IdentityLayer {
    determine_layer(
        config_manager,
        my_pubkey_hex,
        &ver.ca_pubkey,
        &ver.ca_base_url,
        ver.signature.as_deref(),
        ver.ca_token.as_deref(),
        ver.expire_at.as_ref().map(|dt| time::to_ts(*dt) as u64),
        time::now_ts_ms() as u64,
    )
}

/// 信頼できる CA の URL リストをすべて返す。
/// まずインメモリキャッシュ（リスト）を確認し、なければ DB 検索を行う。
pub async fn get_reliable_ca_urls(
    conn: &DatabaseConnection,
    config_manager: &ConfigManager,
) -> Option<Vec<String>> {
    // 1. キャッシュの確認
    let cached_list = {
        let guard = config_manager.reliable_ca_cache.read();
        guard.clone()
    };

    if let Some(urls) = cached_list {
        if !urls.is_empty() {
            return Some(urls);
        }
    }

    // 2. キャッシュがない（または空）場合は DB 検索を実行
    let list = select_reliable_ca_url_from_db(conn, config_manager).await;
    if let Some(ref urls) = list {
        let mut guard = config_manager.reliable_ca_cache.write();
        *guard = Some(urls.clone());
    }
    list
}

/// インメモリキャッシュを介さず、DB から信頼できる CA の URL リストを直接取得して返す。
/// 主に `periodic_store` タスクで使用される。
pub async fn select_reliable_ca_url_from_db(
    conn: &DatabaseConnection,
    config_manager: &ConfigManager,
) -> Option<Vec<String>> {
    // 1. Check Owner Mode
    {
        let guard = config_manager.owner_key.read();
        if guard.is_some() {
            return None;
        }
    }

    let my_pubkey_hex = match config_manager.get_node_keypair() {
        Ok(kp) => hex::encode(kp.public),
        Err(_) => return None,
    };

    let all_verifications = verifications::Entity::find()
        .filter(verifications::Column::NodePubkey.eq(&my_pubkey_hex))
        .all(conn)
        .await
        .unwrap_or_default();

    let mut reliable_urls = Vec::new();
    for ver in all_verifications {
        if determine_layer_from_verification(config_manager, &ver, &my_pubkey_hex)
            == IdentityLayer::L3
        {
            reliable_urls.push(ver.ca_base_url);
        }
    }

    if reliable_urls.is_empty() {
        return None;
    }

    Some(reliable_urls)
}

/// ノード全体の最高到達信頼レベルを判定する。
pub async fn get_node_layer_for_global(
    conn: &DatabaseConnection,
    config_manager: &ConfigManager,
) -> IdentityLayer {
    // 1. Check Owner Mode
    {
        let guard = config_manager.owner_key.read();
        if guard.is_some() {
            return IdentityLayer::L3; // Owner is effectively L3+
        }
    }

    let my_pubkey_hex = match config_manager.get_node_keypair() {
        Ok(kp) => hex::encode(kp.public),
        Err(_) => return IdentityLayer::L1,
    };

    // 全ての検証レコードを取得して最高到達レベルを判定。
    let all_verifications = verifications::Entity::find()
        .filter(verifications::Column::NodePubkey.eq(&my_pubkey_hex))
        .all(conn)
        .await
        .unwrap_or_default();

    let mut highest = IdentityLayer::L1;
    for ver in all_verifications {
        let layer = determine_layer_from_verification(config_manager, &ver, &my_pubkey_hex);
        if layer == IdentityLayer::L3 {
            return IdentityLayer::L3;
        }
        if layer > highest {
            highest = layer;
        }
    }
    highest
}

/// 特定の CA の文脈において、ノードがどの信頼レベルにあるかを判定する。
pub async fn get_node_layer_for_specific_ca(
    conn: &DatabaseConnection,
    config_manager: &ConfigManager,
    ca_base_url: &str,
) -> IdentityLayer {
    // 1. Check Owner Mode
    {
        let guard = config_manager.owner_key.read();
        if guard.is_some() {
            return IdentityLayer::L3;
        }
    }

    let my_pubkey_hex = match config_manager.get_node_keypair() {
        Ok(kp) => hex::encode(kp.public),
        Err(_) => return IdentityLayer::L1,
    };

    // DB から検証レコードを取得。node_pubkey と ca_base_url の複合キーで検索。
    let verification = verifications::Entity::find()
        .filter(verifications::Column::NodePubkey.eq(&my_pubkey_hex))
        .filter(verifications::Column::CaBaseUrl.eq(ca_base_url))
        .one(conn)
        .await
        .unwrap_or(None);

    match verification {
        Some(ver) => determine_layer_from_verification(config_manager, &ver, &my_pubkey_hex),
        None => IdentityLayer::L1,
    }
}
