use crate::constants::{
    ERR_INVALID_PUBKEY, ERR_NO_OWNER_KEY, ERR_SIGNING, ERR_TARGET_ERROR, ERR_TARGET_RESPONSE,
    ERR_TARGET_UNREACHABLE, PATH_IDENTITIES_PUBKEY, ST_BAD_GATEWAY, ST_INTERNAL_SERVER_ERROR,
};
use crate::mode::rt::client::secure_client::SecureClient;
use crate::mode::rt::rtres::errs_res::ApiError;
use crate::stt_config::ConfigManager;
use crate::utils::time;
use std::sync::Arc;

pub async fn assign_ca(
    config_manager: Arc<ConfigManager>,
    client: &SecureClient,
    target_url: String,
    expire_hours: u32,
) -> Result<String, ApiError> {
    log::info!(
        "<Owner> assign_ca: Target={}, Expire={}h",
        target_url,
        expire_hours
    );

    // 1. 自身の Owner Key を取得 (メモリから)
    let owner_key_pair = {
        let guard = config_manager.owner_key.read();
        guard.as_ref().cloned().ok_or_else(|| {
            ApiError::new_system(
                ST_BAD_GATEWAY,
                ERR_NO_OWNER_KEY,
                "Owner key not present in memory.",
            )
        })?
    };

    // 2. ターゲットの公開鍵を取得 (HTTP GET /v1/identities/pubkey)
    // URLの末尾スラッシュを除去してパスを結合
    let target_api_url = format!(
        "{}{}",
        target_url.trim_end_matches('/'),
        PATH_IDENTITIES_PUBKEY
    );

    let resp = client.get(&target_api_url).await.map_err(|e| {
        ApiError::new_system(
            ST_BAD_GATEWAY,
            ERR_TARGET_UNREACHABLE,
            format!("Failed to connect to target: {}", e),
        )
    })?;

    if !resp.status().is_success() {
        return Err(ApiError::new_system(
            ST_BAD_GATEWAY,
            ERR_TARGET_ERROR,
            format!("Target returned error: {}", resp.status()),
        ));
    }

    let target_pubkey_hex: String = resp.json().await.map_err(|e| {
        ApiError::new_system(
            ST_BAD_GATEWAY,
            ERR_TARGET_RESPONSE,
            format!("Failed to parse target response: {}", e),
        )
    })?;

    // Hex -> Bytes
    let target_pubkey_bytes = hex::decode(&target_pubkey_hex).map_err(|e| {
        ApiError::new_system(
            ST_BAD_GATEWAY,
            ERR_INVALID_PUBKEY,
            format!("Target returned invalid hex pubkey: {}", e),
        )
    })?;

    // 46. 有効期限の計算
    let now = time::now_utc();
    let expire_at = now + chrono::Duration::hours(expire_hours as i64);
    let expire_at_ts = expire_at.timestamp() as u64;

    let mut sign_payload = Vec::new();
    sign_payload.extend_from_slice(&target_pubkey_bytes);
    sign_payload.extend_from_slice(&expire_at_ts.to_be_bytes());

    // 署名 (Ed448)
    let signature = owner_key_pair.sign(&sign_payload).map_err(|e| {
        ApiError::new_system(
            ST_INTERNAL_SERVER_ERROR,
            ERR_SIGNING,
            format!("Failed to sign CA token: {}", e),
        )
    })?;
    let signature_hex = hex::encode(&signature.signature);

    // 4. 結果の整形 (CA Appointment Certificate)
    let result = format!(
        "=== MYCUTE CA APPOINTMENT ===\n\
        Target: {}\n\
        Target PubKey: {}\n\
        Expire At: {} (Unix: {})\n\
        -----------------------------\n\
        [CA TOKEN]\n\
        {}.{}\n\
        -----------------------------\n\
        INSTRUCTIONS:\n\
        1. Copy the CA TOKEN above (Signature.ExpireEpoch).\n\
        2. Hand it over to the CA administrator.\n\
        3. CA admin must register this token to activate CA status.\n\
        =============================",
        target_url,
        target_pubkey_hex,
        expire_at.to_rfc3339(),
        expire_at_ts,
        signature_hex,
        expire_at_ts
    );

    Ok(result)
}
