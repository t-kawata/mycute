use crate::constants::{
    ERR_INVALID_PUBKEY, ERR_NO_OWNER_KEY, ERR_OWNER_MODE, ERR_SIGNING, ERR_TARGET_ERROR,
    ERR_TARGET_RESPONSE, ERR_TARGET_UNREACHABLE, PATH_IDENTITIES_PUBKEY,
    ST_BAD_GATEWAY, ST_INTERNAL_SERVER_ERROR, ST_UNAUTHORIZED, ED448_SIGNATURE_BYTES_LEN,
};
use crate::mode::rt::client::secure_client::SecureClient;
use crate::mode::rt::owner_secrets::{OWNER_PUB_KEY_HEX, OWNER_SECRET_BLOBS};
use crate::mode::rt::rtres::errs_res::ApiError;
use crate::mycute_settings::ConfigManager;
use crate::utils::crypto::Ed448RawKeyPair;
use crate::utils::time;
use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use argon2::{
    password_hash::{PasswordHasher, SaltString},
    Argon2,
};
use ed448_goldilocks::{curve::ExtendedPoint, Scalar};
use std::str;
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

    // 1. ターゲットの公開鍵を取得 (HTTP GET /v1/identities/pubkey)
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

    // 3. 署名とCAトークンの生成
    let (ca_token, _, _) =
        generate_ca_token_core(config_manager, &target_pubkey_hex, expire_hours).await?;

    Ok(ca_token)
}

pub async fn generate_ca_token_manual(
    config_manager: Arc<ConfigManager>,
    pubkey_hex: String,
    expire_hours: u32,
) -> Result<String, ApiError> {
    log::info!(
        "<Owner> generate_ca_token_manual: PubKey={}, Expire={}h",
        pubkey_hex,
        expire_hours
    );

    let (ca_token, _, _) = generate_ca_token_core(config_manager, &pubkey_hex, expire_hours).await?;

    Ok(ca_token)
}

/// CAトークン生成のコアロジック。
/// 返り値: (ca_token文字列, 有効期限UnixTS, 有効期限RFC3339)
pub async fn generate_ca_token_core(
    config_manager: Arc<ConfigManager>,
    target_pubkey_hex: &str,
    expire_hours: u32,
) -> Result<(String, u64, String), ApiError> {
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

    // Hex -> Bytes
    let target_pubkey_bytes = hex::decode(target_pubkey_hex).map_err(|e| {
        ApiError::new_system(
            ST_INTERNAL_SERVER_ERROR,
            ERR_INVALID_PUBKEY,
            format!("Invalid hex pubkey: {}", e),
        )
    })?;

    // 2. 有効期限の計算
    let now = time::now_utc();
    let expire_at = now + chrono::Duration::hours(expire_hours as i64);
    let expire_at_ts = expire_at.timestamp() as u64;

    let mut sign_payload = Vec::new();
    sign_payload.extend_from_slice(&target_pubkey_bytes);
    sign_payload.extend_from_slice(&expire_at_ts.to_be_bytes());

    // 3. 署名 (Ed448)
    let signature = owner_key_pair.sign(&sign_payload).map_err(|e| {
        ApiError::new_system(
            ST_INTERNAL_SERVER_ERROR,
            ERR_SIGNING,
            format!("Failed to sign CA token: {}", e),
        )
    })?;
    let signature_hex = hex::encode(&signature.signature);

    let ca_token = format!("{}.{}", signature_hex, expire_at_ts);

    Ok((ca_token, expire_at_ts, expire_at.to_rfc3339()))
}

pub async fn deactivate_owner(config_manager: Arc<ConfigManager>) -> Result<(), ApiError> {
    log::info!("<Owner> deactivate_owner requested via API. Clearing Anchor Secret Key from memory...");
    {
        let mut guard = config_manager.owner_key.write();
        *guard = None;
    }
    log::info!("<Owner> Deactivated. Anchor Secret Key cleared.");
    Ok(())
}

pub async fn activate_owner(
    config_manager: Arc<ConfigManager>,
    passphrase: &str,
) -> Result<(), ApiError> {
    log::info!("<Owner> activate_owner requested via API. Attempting to decrypt Anchor Secret Key...");

    let argon2 = Argon2::default();
    let mut decrypted_secret_bytes = None;

    for (i, blob) in OWNER_SECRET_BLOBS.iter().enumerate() {
        if blob.len() < 1 {
            continue;
        }
        let salt_len = blob[0] as usize;
        if blob.len() < 1 + salt_len + 12 {
            continue;
        }

        let salt_bytes = &blob[1..1 + salt_len];
        let nonce_bytes = &blob[1 + salt_len..1 + salt_len + 12];
        let ciphertext = &blob[1 + salt_len + 12..];

        let salt_str = match str::from_utf8(salt_bytes) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let salt = match SaltString::from_b64(salt_str) {
            Ok(s) => s,
            Err(_) => continue,
        };

        let password_hash = match argon2.hash_password(passphrase.as_bytes(), &salt) {
            Ok(h) => h,
            Err(_) => continue,
        };

        let key_bytes: &argon2::password_hash::Output = match &password_hash.hash {
            Some(h) => h,
            None => continue,
        };

        let key_array: [u8; 32] = match key_bytes.as_bytes().try_into() {
            Ok(k) => k,
            Err(_) => continue,
        };

        let cipher = Aes256Gcm::new(&key_array.into());
        let nonce = Nonce::from_slice(nonce_bytes);

        if let Ok(plaintext) = cipher.decrypt(nonce, ciphertext) {
            if plaintext.len() == ED448_SIGNATURE_BYTES_LEN {
                decrypted_secret_bytes = Some(plaintext);
                log::info!("Owner Secret Key decrypted successfully with blob #{}", i + 1);
                break;
            }
        }
    }

    if let Some(secret_bytes_vec) = decrypted_secret_bytes {
        let secret_bytes_arr: [u8; ED448_SIGNATURE_BYTES_LEN] =
            secret_bytes_vec.try_into().unwrap_or_else(|_| {
                panic!("Already checked length against ED448_SIGNATURE_BYTES_LEN")
            });

        let secret_scalar = Scalar::from_bytes_mod_order_wide(&secret_bytes_arr);
        let public_point = ExtendedPoint::generator() * &secret_scalar;
        let public_bytes = public_point.compress();
        let public_hex = hex::encode(public_bytes.0);

        if public_hex != OWNER_PUB_KEY_HEX {
            log::error!("CRITICAL: Decrypted key does not match the hardcoded Anchor Public Key!");
            return Err(ApiError::new_system(
                ST_UNAUTHORIZED,
                ERR_OWNER_MODE,
                "Decrypted key mismatch",
            ));
        }

        // メモリ上に保持
        {
            let mut guard = config_manager.owner_key.write();
            *guard = Some(Ed448RawKeyPair {
                secret: secret_scalar,
                public: public_bytes.0,
            });
        }
        log::info!("<Owner> Activated. You have Root Authority.");
        Ok(())
    } else {
        log::error!("<Owner> Invalid owner passphrase.");
        Err(ApiError::new_system(
            ST_UNAUTHORIZED,
            ERR_OWNER_MODE,
            "Invalid passphrase",
        ))
    }
}
