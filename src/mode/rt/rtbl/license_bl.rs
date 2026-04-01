// ライセンスの生成・登録・管理・検証ロジック。
//
// # ライセンスの形式
// `{base64(payload_json)}.{sig_hex}` の2パート構造を採用する。
//
// # payload_json の内容
// ```json
// {
//   "user_pubkey":  "<対象ユーザーの公開鍵 Hex>",
//   "expire_at":    <有効期限 Unix TS ms>,
//   "permissions":  {"all": true},
//   "ca_token":     "<ca_pubkey_hex.sig_hex.expire_ms>"
// }
// ```
//
// # 信頼の鎖
// ライセンス検証時、以下の 3 ステップを経て「オーナー ➔ CA ➔ ユーザー」の信頼が確認される：
//   1. ライセンス署名を、payload 中に埋め込まれた ca_token の CA 公開鍵で検証
//   2. ca_token をオーナー公開鍵で検証（identities_bl::verify_ca_token）
//   3. ライセンスの expire_at が ca_token より後でないことを確認

use crate::constants::{
    ED448_KEY_BYTES_LEN, ED448_SIGNATURE_BYTES_LEN, ERR_DECRYPT, ERR_ENCRYPT, ERR_INVALID_CA_TOKEN,
    ERR_SAVE, ST_BAD_REQUEST, ST_INTERNAL_SERVER_ERROR,
};
use crate::mode::rt::rtbl::identities_bl;
use crate::mode::rt::rtres::errs_res::ApiError;
use crate::mode::rt::rtres::mycute_res::LicenseSummary;
use crate::mycute_settings::ConfigManager;
use crate::utils::{crypto, time};
use base64::Engine;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;

// ============================================================
// ライセンスのペイロード構造体
// ============================================================

/// ライセンスの署名対象となるペイロード。
/// Canonical JSON (BTreeMap によるキーソート) で直列化し、Ed448 で署名する。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicensePayload {
    /// ライセンス付与対象ユーザーの公開鍵 (Hex)
    pub user_pubkey: String,
    /// ライセンスの有効期限 (Unix TS ms)
    pub expire_at: u64,
    /// 権限内容の JSON 値
    pub permissions: serde_json::Value,
    /// 発行元 CA の CA トークン文字列 ({base64(payload)}.{sig_hex})
    /// 信頼の鎖を維持するために埋め込む。
    pub ca_token: String,
}

impl LicensePayload {
    /// キーをアルファベット順にソートした正規化 JSON 文字列を生成する。
    /// 署名・検証時の一意性を保証するための Canonical Serialization。
    pub fn to_canonical_json(&self) -> Result<String, ApiError> {
        let val = serde_json::to_value(self).map_err(|e| {
            ApiError::new_system(
                ST_INTERNAL_SERVER_ERROR,
                "ERR_LICENSE_GEN",
                format!("Failed to serialize license payload: {}", e),
            )
        })?;
        if let serde_json::Value::Object(map) = val {
            let sorted: std::collections::BTreeMap<_, _> = map.into_iter().collect();
            serde_json::to_string(&sorted).map_err(|e| {
                ApiError::new_system(
                    ST_INTERNAL_SERVER_ERROR,
                    "ERR_LICENSE_GEN",
                    format!("Failed to generate canonical JSON for license: {}", e),
                )
            })
        } else {
            Err(ApiError::new_system(
                ST_INTERNAL_SERVER_ERROR,
                "ERR_LICENSE_GEN",
                "License payload is not a JSON object.",
            ))
        }
    }
}

// ============================================================
// ライセンス ID 生成
// ============================================================

/// ライセンス文字列の SHA-256 ハッシュから先頭 16 文字を ID として使用する。
/// 同一のライセンス文字列は常に同一の ID になるため、登録・削除の際のキーとして機能する。
fn compute_license_id(license_raw: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(license_raw.as_bytes());
    let result = hasher.finalize();
    hex::encode(&result[..8]) // 先頭 8 bytes = 16 文字
}

// ============================================================
// ライセンス文字列のパース
// ============================================================

/// `base64(payload).sig_hex` 形式のライセンス文字列をパースして LicensePayload と署名を返す。
fn parse_license_str(
    license: &str,
) -> Result<(LicensePayload, [u8; ED448_SIGNATURE_BYTES_LEN]), ApiError> {
    let parts: Vec<&str> = license.splitn(2, '.').collect();
    if parts.len() != 2 {
        return Err(ApiError::new_system(
            ST_BAD_REQUEST,
            "ERR_LICENSE_PARSE",
            "License format must be 'base64(payload).sig_hex'.",
        ));
    }
    let payload_b64 = parts[0];
    let sig_hex = parts[1];

    // Base64 デコード
    let payload_bytes = base64::engine::general_purpose::STANDARD
        .decode(payload_b64)
        .map_err(|e| {
            ApiError::new_system(
                ST_BAD_REQUEST,
                "ERR_LICENSE_PARSE",
                format!("Failed to decode license payload base64: {}", e),
            )
        })?;

    // JSON デシリアライズ
    let payload: LicensePayload = serde_json::from_slice(&payload_bytes).map_err(|e| {
        ApiError::new_system(
            ST_BAD_REQUEST,
            "ERR_LICENSE_PARSE",
            format!("Failed to parse license payload JSON: {}", e),
        )
    })?;

    // 署名のデコード
    let sig_bytes = hex::decode(sig_hex).map_err(|e| {
        ApiError::new_system(
            ST_BAD_REQUEST,
            "ERR_LICENSE_PARSE",
            format!("Failed to decode license signature hex: {}", e),
        )
    })?;
    if sig_bytes.len() != ED448_SIGNATURE_BYTES_LEN {
        return Err(ApiError::new_system(
            ST_BAD_REQUEST,
            "ERR_LICENSE_PARSE",
            format!(
                "Invalid license signature length. Expected {}, got {}.",
                ED448_SIGNATURE_BYTES_LEN,
                sig_bytes.len()
            ),
        ));
    }
    let mut sig_arr = [0u8; ED448_SIGNATURE_BYTES_LEN];
    sig_arr.copy_from_slice(&sig_bytes);

    Ok((payload, sig_arr))
}

// ============================================================
// ライセンスの検証
// ============================================================

/// ライセンス文字列を完全に検証し、LicenseSummary を返す。
///
/// # 検証ステップ
/// 1. ライセンス文字列のパース
/// 2. 埋め込まれた ca_token をオーナー公開鍵で検証
/// 3. ライセンスの expire_at が ca_token の expire_at を超えていないことを確認
/// 4. ライセンス本体の署名を ca_token 内の CA 公開鍵で検証
pub fn verify_license_chain(license: &str) -> Result<LicenseSummary, ApiError> {
    // 1. パース
    let (payload, sig_arr) = parse_license_str(license)?;

    // 2. ca_token をオーナー公開鍵で検証
    //    verify_ca_token は Some(ca_pubkey_hex) を返せば有効、None なら無効・期限切れ
    let now_ts = time::now_ts_ms();
    let verified_ca_pubkey_hex =
        identities_bl::verify_ca_token(&payload.ca_token, now_ts).ok_or_else(|| {
            ApiError::new_system(
                ST_BAD_REQUEST,
                ERR_INVALID_CA_TOKEN,
                "The CA token embedded in the license is invalid or expired.",
            )
        })?;

    // 3. ライセンスの有効期限が CA トークンの有効期限を超えていないことを確認
    let (ca_payload, _) = identities_bl::parse_ca_token_raw(&payload.ca_token).map_err(|e| {
        ApiError::new_system(
            ST_BAD_REQUEST,
            ERR_INVALID_CA_TOKEN,
            format!("Failed to parse the embedded CA token: {}", e),
        )
    })?;
    let ca_token_expire_at = ca_payload.expire_at;

    if payload.expire_at > ca_token_expire_at {
        return Err(ApiError::new_system(
            ST_BAD_REQUEST,
            "ERR_LICENSE_EXPIRE_EXCEEDS_CA",
            format!(
                "License expire_at ({}) exceeds the CA token expire_at ({}). This is not allowed.",
                payload.expire_at, ca_token_expire_at
            ),
        ));
    }

    // 4. CA 公開鍵でライセンス自体の署名を検証
    let ca_pub_bytes = hex::decode(&verified_ca_pubkey_hex).map_err(|e| {
        ApiError::new_system(
            ST_BAD_REQUEST,
            ERR_INVALID_CA_TOKEN,
            format!("Failed to decode CA public key from ca_token: {}", e),
        )
    })?;
    if ca_pub_bytes.len() != ED448_KEY_BYTES_LEN {
        return Err(ApiError::new_system(
            ST_BAD_REQUEST,
            ERR_INVALID_CA_TOKEN,
            "CA public key has invalid length.",
        ));
    }
    let mut ca_pub_arr = [0u8; ED448_KEY_BYTES_LEN];
    ca_pub_arr.copy_from_slice(&ca_pub_bytes);

    // 署名対象は Canonical JSON のバイト列
    let canonical_json = payload.to_canonical_json()?;
    let sig_struct = crypto::Ed448Signature { signature: sig_arr };

    let is_valid =
        crypto::verify_signature(&ca_pub_arr, canonical_json.as_bytes(), &sig_struct)
            .unwrap_or(false);

    // ライセンスが現在有効か（有効期限 + 署名検証両方）
    let is_currently_valid = is_valid && payload.expire_at >= now_ts;

    if !is_valid {
        return Err(ApiError::new_system(
            ST_BAD_REQUEST,
            "ERR_LICENSE_SIG_FAIL",
            "License signature verification failed.",
        ));
    }

    Ok(LicenseSummary {
        id: compute_license_id(license),
        ca_pubkey: verified_ca_pubkey_hex,
        expire_at: payload.expire_at,
        permissions: payload.permissions,
        is_valid: is_currently_valid,
        raw: license.to_string(),
    })
}

// ============================================================
// CA によるライセンス発行
// ============================================================

/// CA として機能しているノードがユーザーにライセンスを発行する。
///
/// # 前提条件
/// - 呼び出し元ノードが有効な `my_cat` を保持していること（CAとして動作中）
///
/// # 検証内容
/// - 要求された `expire_at` が自身の CA トークンの有効期限を超えていないこと
pub async fn generate_license(
    config_manager: Arc<ConfigManager>,
    target_pubkey_hex: String,
    expire_hours: u32,
    permissions: Option<serde_json::Value>,
) -> Result<(String, u64), ApiError> {
    // 1. 自身の CA トークンを取得・復号する
    let (crypto_key, encrypted_ca_token) = {
        let s = config_manager.settings.read();
        let key = s.server.rt_crypto_key.clone();
        let cat = s.my_cat.clone().ok_or_else(|| {
            ApiError::new_system(
                ST_BAD_REQUEST,
                "ERR_NOT_A_CA",
                "This node has no CA token registered. Only CAs can issue licenses.",
            )
        })?;
        (key, cat)
    };

    let ca_token_raw = crypto::decrypt(&encrypted_ca_token, &crypto_key).map_err(|e| {
        ApiError::new_system(
            ST_INTERNAL_SERVER_ERROR,
            ERR_DECRYPT,
            format!("Failed to decrypt CA token: {}", e),
        )
    })?;

    // 2. CA トークンが有効であることを確認し、CA 有効期限を取得する
    let now_ts = time::now_ts_ms();
    identities_bl::verify_ca_token(&ca_token_raw, now_ts).ok_or_else(|| {
        ApiError::new_system(
            ST_BAD_REQUEST,
            ERR_INVALID_CA_TOKEN,
            "This node's CA token is invalid or expired. Cannot issue license.",
        )
    })?;

    let (ca_payload, _) = identities_bl::parse_ca_token_raw(&ca_token_raw).map_err(|e| {
        ApiError::new_system(
            ST_INTERNAL_SERVER_ERROR,
            ERR_INVALID_CA_TOKEN,
            format!("Failed to parse CA token: {}", e),
        )
    })?;
    let ca_token_expire_at = ca_payload.expire_at;

    // 3. ライセンスの有効期限を計算（ms 単位）
    let expire_at_ms = now_ts + (expire_hours as u64) * 3600 * 1000;

    // 4. ライセンスの有効期限が CA トークンの有効期限を超えていないことを確認
    if expire_at_ms > ca_token_expire_at {
        return Err(ApiError::new_system(
            ST_BAD_REQUEST,
            "ERR_LICENSE_EXPIRE_EXCEEDS_CA",
            format!(
                "Requested license expire ({}) exceeds this CA token's expire_at ({}). \
                 A CA cannot issue a license that outlasts itself.",
                expire_at_ms, ca_token_expire_at
            ),
        ));
    }

    // 5. 権限内容を設定する（省略時はデフォルト {"all": true}）
    let permissions = permissions.unwrap_or_else(|| serde_json::json!({"all": true}));

    // 6. ペイロードを組み立てる
    let payload = LicensePayload {
        user_pubkey: target_pubkey_hex.clone(),
        expire_at: expire_at_ms,
        permissions,
        ca_token: ca_token_raw,
    };
    let canonical_json = payload.to_canonical_json()?;

    // 7. 自身の秘密鍵でペイロードに署名する
    let kp = config_manager.get_node_keypair()?;
    let sig = kp.sign(canonical_json.as_bytes()).map_err(|e| {
        ApiError::new_system(
            ST_INTERNAL_SERVER_ERROR,
            ERR_ENCRYPT,
            format!("Failed to sign license payload: {}", e),
        )
    })?;
    let sig_hex = hex::encode(sig.signature);

    // 8. ライセンス文字列を組み立てる
    let payload_b64 = base64::engine::general_purpose::STANDARD
        .encode(canonical_json.as_bytes());
    let license_str = format!("{}.{}", payload_b64, sig_hex);

    log::debug!(
        "<License> Generated license for user_pubkey: {}..., expire_at: {}",
        &target_pubkey_hex[..8.min(target_pubkey_hex.len())],
        expire_at_ms
    );

    Ok((license_str, expire_at_ms))
}

// ============================================================
// ライセンスの登録
// ============================================================

/// ライセンスを自身の `my_lics` に登録する。
///
/// # 前提条件
/// - ライセンスが自分の公開鍵に対して発行されたものであること
/// - 完全な信頼の鎖検証に合格すること
pub async fn register_license(
    config_manager: Arc<ConfigManager>,
    license: String,
) -> Result<LicenseSummary, ApiError> {
    // 1. 完全な検証（信頼の鎖確認）
    let summary = verify_license_chain(&license)?;

    // 2. 自分の公開鍵と照合する
    let my_pub_hex = identities_bl::get_my_node_pubkey(config_manager.clone())
        .await
        .map_err(|e| {
            ApiError::new_system(
                ST_INTERNAL_SERVER_ERROR,
                "ERR_MY_PUBKEY",
                format!("Failed to get my pubkey: {}", e),
            )
        })?;

    // ペイロードを再度パースして user_pubkey を確認
    let (payload, _) = parse_license_str(&license)?;
    if payload.user_pubkey != my_pub_hex {
        return Err(ApiError::new_system(
            ST_BAD_REQUEST,
            "ERR_LICENSE_NOT_FOR_ME",
            format!(
                "This license is issued for pubkey '{}...', but my pubkey is '{}...'. \
                 Cannot register a license issued for another node.",
                &payload.user_pubkey[..8.min(payload.user_pubkey.len())],
                &my_pub_hex[..8.min(my_pub_hex.len())]
            ),
        ));
    }

    // 3. 重複登録チェック（同じ ID が既に存在する場合はスキップ）
    let new_id = compute_license_id(&license);
    {
        let s = config_manager.settings.read();
        for existing_lic in &s.my_lics {
            if compute_license_id(existing_lic) == new_id {
                return Err(ApiError::new_system(
                    ST_BAD_REQUEST,
                    "ERR_LICENSE_DUPLICATE",
                    "This license is already registered.",
                ));
            }
        }
    }

    // 4. 暗号化して保存する
    let crypto_key = {
        let s = config_manager.settings.read();
        s.server.rt_crypto_key.clone()
    };
    let encrypted_license = crypto::encrypt(&license, &crypto_key).map_err(|e| {
        ApiError::new_system(
            ST_INTERNAL_SERVER_ERROR,
            ERR_ENCRYPT,
            format!("Failed to encrypt license: {}", e),
        )
    })?;

    {
        let mut w = config_manager.settings.write();
        w.my_lics.push(encrypted_license);
    }
    config_manager.save_db().await.map_err(|e| {
        ApiError::new_system(
            ST_INTERNAL_SERVER_ERROR,
            ERR_SAVE,
            format!("Failed to save license to DB: {}", e),
        )
    })?;

    log::debug!(
        "<License> License registered. id: {}, ca: {}...",
        new_id,
        &summary.ca_pubkey[..8.min(summary.ca_pubkey.len())]
    );

    Ok(summary)
}

// ============================================================
// ライセンスの削除
// ============================================================

/// 指定した ID のライセンスを `my_lics` から削除する。
pub async fn unregister_license(
    config_manager: Arc<ConfigManager>,
    license_id: &str,
) -> Result<(), ApiError> {
    let crypto_key = {
        let s = config_manager.settings.read();
        s.server.rt_crypto_key.clone()
    };

    let mut found = false;
    {
        let mut w = config_manager.settings.write();
        let original_len = w.my_lics.len();
        w.my_lics.retain(|enc_lic| {
            // 復号してIDを計算し、一致するものを削除
            match crypto::decrypt(enc_lic, &crypto_key) {
                Ok(raw) => {
                    if compute_license_id(&raw) == license_id {
                        found = true;
                        false // = retain しない（削除する）
                    } else {
                        true // = retain する
                    }
                }
                Err(e) => {
                    log::warn!(
                        "<License> Failed to decrypt a license during unregister: {}",
                        e
                    );
                    true // 復号失敗したものは削除しない（安全側に倒す）
                }
            }
        });
        let _ = original_len; // suppress warning
    }

    if !found {
        return Err(ApiError::new_system(
            ST_BAD_REQUEST,
            "ERR_LICENSE_NOT_FOUND",
            format!("License with id '{}' not found.", license_id),
        ));
    }

    config_manager.save_db().await.map_err(|e| {
        ApiError::new_system(
            ST_INTERNAL_SERVER_ERROR,
            ERR_SAVE,
            format!("Failed to save settings after unregister: {}", e),
        )
    })?;

    log::debug!("<License> License unregistered. id: {}", license_id);
    Ok(())
}

// ============================================================
// ライセンス一覧取得
// ============================================================

/// 保持している全ライセンスをパースして LicenseSummary のリストとして返す。
/// 復号・パースに失敗したもの、また期限切れのものも返却する（is_valid で区別）。
pub async fn list_licenses(config_manager: Arc<ConfigManager>) -> Vec<LicenseSummary> {
    let (encrypted_lics, crypto_key) = {
        let s = config_manager.settings.read();
        (s.my_lics.clone(), s.server.rt_crypto_key.clone())
    };

    let mut result = Vec::new();
    for enc_lic in &encrypted_lics {
        match crypto::decrypt(enc_lic, &crypto_key) {
            Ok(raw) => {
                match verify_license_chain(&raw) {
                    Ok(summary) => result.push(summary),
                    Err(e) => {
                        // パース・検証失敗したライセンスも ID だけは返す（破損表示用）
                        log::warn!("<License> Failed to verify a stored license: {}", e);
                    }
                }
            }
            Err(e) => {
                log::warn!("<License> Failed to decrypt a stored license: {}", e);
            }
        }
    }
    result
}
