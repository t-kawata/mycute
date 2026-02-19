use sea_orm::{DatabaseConnection, EntityTrait, QueryFilter, ColumnTrait, Set, ActiveModelTrait};
use crate::{
    utils::{crypto::{encrypt, decrypt}, jwt::{JwtUsr, JwtIDs, JwtRole, generate_token_for_vdr}},
    mode::rt::{rtres::{errs_res::ApiError, cryptos_res::{EncryptRes, DecryptRes, CreateVdrTokenRes, GetVdrTokenRes}}, rterr::rterr},
    entities::{cryptos, usrs},
};
use crate::constants::{ST_BAD_REQUEST, ST_INTERNAL_SERVER_ERROR, ST_FORBIDDEN, ST_NOT_FOUND};
use regex::Regex;

// ============================================================
// Encrypt
// ============================================================
pub async fn encrypt_text(crypto_key: &str, text: String) -> Result<EncryptRes, ApiError> {
    if text.is_empty() {
        return Err(ApiError::new_system(ST_BAD_REQUEST, rterr::ERR_INVALID_REQUEST, "Empty text."));
    }
    let data = encrypt(&text, crypto_key)
        .map_err(|e| ApiError::new_system(ST_BAD_REQUEST, rterr::ERR_INVALID_REQUEST, format!("Failed: {}", e)))?;
    Ok(EncryptRes { data })
}

// ============================================================
// Decrypt
// ============================================================
pub async fn decrypt_text(crypto_key: &str, text: String) -> Result<DecryptRes, ApiError> {
    if text.is_empty() {
        return Err(ApiError::new_system(ST_BAD_REQUEST, rterr::ERR_INVALID_REQUEST, "Empty text."));
    }
    let data = decrypt(&text, crypto_key)
        .map_err(|e| ApiError::new_system(ST_BAD_REQUEST, rterr::ERR_INVALID_REQUEST, format!("Failed: {}", e)))?;
    Ok(DecryptRes { data })
}

// ============================================================
// Create VDR Token
// ============================================================
pub async fn create_vdr_token(
    conn: &DatabaseConnection,
    ju: &JwtUsr,
    ids: &JwtIDs,
    skey: &str,
    crypto_key: &str,
    key: String,
    apx_id: u32,
    vdr_id: u32,
) -> Result<CreateVdrTokenRes, ApiError> {
    // Role check (APX only)
    ju.allow_roles(&[JwtRole::APX])?;
    // Key validation: 半角英数字とハイフンとアンダーバーのみの5文字以上、50文字以下
    let re = Regex::new("^[a-zA-Z0-9-_]{5,50}$").unwrap();
    if !re.is_match(&key) {
        return Err(ApiError::new_system(ST_BAD_REQUEST, rterr::ERR_INVALID_REQUEST, "Invalid key."));
    }
    // IDS check: APX can only create tokens for their own VDRs
    if ids.apx_id != apx_id {
        return Err(ApiError::new_system(ST_FORBIDDEN, rterr::ERR_AUTH, "Invalid APX ID."));
    }
    // VDR check
    let usr = usrs::Entity::find()
        .filter(usrs::Column::ApxId.eq(apx_id))
        .filter(usrs::Column::Id.eq(vdr_id))
        .one(conn)
        .await
        .map_err(|e| ApiError::new_system(ST_INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::new_system(ST_NOT_FOUND, rterr::ERR_NOT_FOUND, "VDR not found."))?;
    // Generate 100-year token (876000 hours)
    let token = generate_token_for_vdr(skey, apx_id, vdr_id, usr.email, 876000).map_err(|e| ApiError::new_system(ST_INTERNAL_SERVER_ERROR, rterr::ERR_UNEXPECTED, format!("Failed to generate token: {}", e)))?;
    // Encrypt token
    let encrypted_token = encrypt(&token, crypto_key).map_err(|e| ApiError::new_system(ST_INTERNAL_SERVER_ERROR, rterr::ERR_UNEXPECTED, format!("Failed to encrypt token: {}", e)))?;
    // Check existence and ownership protection
    let existing = cryptos::Entity::find()
        .filter(cryptos::Column::Key.eq(&key))
        .filter(cryptos::Column::ApxId.eq(Some(apx_id)))
        .filter(cryptos::Column::VdrId.eq(Some(vdr_id)))
        .one(conn)
        .await
        .map_err(|e| ApiError::new_system(ST_INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, format!("Database error: {}", e)))?;
    if let Some(record) = existing {
        // Ownership check: Must match both apx_id and vdr_id
        if record.apx_id != Some(apx_id as i32) || record.vdr_id != Some(vdr_id as i32) {
            return Err(ApiError::new_system(ST_FORBIDDEN, rterr::ERR_AUTH, "This key is already owned by another entity. Ownership theft is prohibited."));
        }
        // Update existing record
        let mut active: cryptos::ActiveModel = record.into();
        active.value = Set(encrypted_token.clone());
        active.update(conn).await.map_err(|e| ApiError::new_system(ST_INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, format!("Failed to update crypto: {}", e)))?;
    } else {
        // Insert new record
        let model = cryptos::ActiveModel {
            key: Set(key.clone()),
            value: Set(encrypted_token.clone()),
            apx_id: Set(Some(apx_id as i32)),
            vdr_id: Set(Some(vdr_id as i32)),
            ..Default::default()
        };
        model.insert(conn).await.map_err(|e| ApiError::new_system(ST_INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, format!("Failed to save crypto: {}", e)))?;
    }
    Ok(CreateVdrTokenRes {
        key,
        value: encrypted_token,
    })
}

// ============================================================
// Get VDR Token
// ============================================================
pub async fn get_vdr_token(
    conn: &DatabaseConnection,
    key: String,
) -> Result<GetVdrTokenRes, ApiError> {
    // Key validation
    let re = Regex::new("^[a-zA-Z0-9-_]{5,50}$").unwrap();
    if !re.is_match(&key) {
        return Err(ApiError::new_system(ST_BAD_REQUEST, rterr::ERR_INVALID_REQUEST, "Invalid key."));
    }

    let crypto = cryptos::Entity::find()
        .filter(cryptos::Column::Key.eq(key))
        .one(conn)
        .await
        .map_err(|e| ApiError::new_system(ST_INTERNAL_SERVER_ERROR, rterr::ERR_DATABASE, format!("Database error: {}", e)))?
        .ok_or_else(|| ApiError::new_system(ST_NOT_FOUND, rterr::ERR_NOT_FOUND, "Record not found."))?;

    Ok(GetVdrTokenRes {
        key: crypto.key,
        value: crypto.value,
    })
}
