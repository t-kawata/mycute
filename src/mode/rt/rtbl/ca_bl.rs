use crate::constants::{
    ERR_ENCRYPT, ERR_INVALID_CA_TOKEN, ERR_SAVE, ST_BAD_REQUEST, ST_INTERNAL_SERVER_ERROR,
};
use crate::mode::rt::{
    rtbl::identities_bl,
    rtreq::ca_req::RegisterCaTokenReq,
    rtres::{ca_res::{RegisterCaTokenRes, UnregisterCaTokenRes}, errs_res::ApiError},
};
use crate::mycute_settings::ConfigManager;
use crate::utils::{crypto, time};
use std::sync::Arc;

pub async fn register_ca_token(
    config_manager: Arc<ConfigManager>,
    req: RegisterCaTokenReq,
) -> Result<RegisterCaTokenRes, ApiError> {
    // Get My Public Key
    let my_pub_hex = identities_bl::get_my_node_pubkey(config_manager.clone())
        .await
        .map_err(|e| {
            ApiError::new_system(
                ST_INTERNAL_SERVER_ERROR,
                ERR_INVALID_CA_TOKEN,
                format!("Failed to get my pubkey: {}", e),
            )
        })?;

    let res = identities_bl::verify_ca_token(&req.ca_token, time::now_ts_ms() as u64);

    if let Some(token_pubkey) = res {
        if token_pubkey != my_pub_hex {
            return Err(ApiError::new_system(
                ST_BAD_REQUEST,
                ERR_INVALID_CA_TOKEN,
                format!(
                    "CA Cert is for another node. Cert pubkey: {}, My pubkey: {}",
                    token_pubkey, my_pub_hex
                ),
            ));
        }
    } else {
        return Err(ApiError::new_system(
            ST_BAD_REQUEST,
            ERR_INVALID_CA_TOKEN,
            "Invalid CA Cert signature or expired (Owner verification failed).",
        ));
    }

    // 2. Encrypt Token
    let crypto_key = {
        let s = config_manager.settings.read();
        s.server.rt_crypto_key.clone()
    };

    let encrypted_token = crypto::encrypt(&req.ca_token, &crypto_key).map_err(|e| {
        ApiError::new_system(
            ST_INTERNAL_SERVER_ERROR,
            ERR_ENCRYPT,
            format!("Failed to encrypt CA cert: {}", e),
        )
    })?;

    // 3. Save to Settings
    {
        let mut w = config_manager.settings.write();
        w.my_cat = Some(encrypted_token);
    }
    config_manager.save_db().await.map_err(|e| {
        ApiError::new_system(
            ST_INTERNAL_SERVER_ERROR,
            ERR_SAVE,
            format!("Failed to save settings: {}", e),
        )
    })?;

    // 4. Return Success
    let (payload, _) = identities_bl::parse_ca_token_raw(&req.ca_token).map_err(|e| {
        ApiError::new_system(
            ST_INTERNAL_SERVER_ERROR,
            ERR_INVALID_CA_TOKEN,
            format!("Failed to parse CA cert for response: {}", e),
        )
    })?;

    Ok(RegisterCaTokenRes {
        success: true,
        message:
            "CA Cert registered successfully. You are now authorized as a Central Authority (Trust Anchor)."
                .to_string(),
        ca_token: Some(req.ca_token),
        permissions: Some(payload.permissions),
    })
}

pub async fn unregister_ca_token(
    config_manager: Arc<ConfigManager>,
) -> Result<UnregisterCaTokenRes, ApiError> {
    // 1. Check Owner Mode
    if !config_manager.is_owner_active() {
        return Err(ApiError::new_system(
            axum::http::StatusCode::FORBIDDEN,
            "UNAUTHORIZED",
            "Owner mode must be active to unregister CA token.".to_string(),
        ));
    }

    // 2. Clear from Settings
    {
        let mut w = config_manager.settings.write();
        w.my_cat = None;
    }
    
    config_manager.save_db().await.map_err(|e| {
        ApiError::new_system(
            ST_INTERNAL_SERVER_ERROR,
            ERR_SAVE,
            format!("Failed to save settings: {}", e),
        )
    })?;

    Ok(UnregisterCaTokenRes {
        success: true,
        message: "CA Cert unregistered successfully.".to_string(),
    })
}

pub async fn get_ca_status(config_manager: Arc<ConfigManager>) -> Option<String> {
    let my_cat = {
        let s = config_manager.settings.read();
        s.my_cat.clone()
    };

    let encrypted_ca_token = match my_cat {
        Some(t) => t,
        None => return None,
    };

    // 1. Decrypt Token
    let crypto_key = {
        let s = config_manager.settings.read();
        s.server.rt_crypto_key.clone()
    };

    let ca_token = match crypto::decrypt(&encrypted_ca_token, &crypto_key) {
        Ok(t) => t,
        Err(e) => {
            log::error!("<CA> Failed to decrypt stored CA token: {}", e);
            return None;
        }
    };

    // 2. Verify Token
    let res = identities_bl::verify_ca_token(&ca_token, time::now_ts_ms() as u64);

    if let Some(token_pubkey) = res {
        // 3. Match with My Public Key
        let my_pub_hex = match identities_bl::get_my_node_pubkey(config_manager.clone()).await {
            Ok(p) => p,
            Err(e) => {
                log::error!("<CA> Failed to get my pubkey for status check: {}", e);
                return None;
            }
        };

        if token_pubkey == my_pub_hex {
            return Some(ca_token);
        } else {
            log::warn!(
                "<CA> Stored CA token pubkey mismatch. token: {}, my: {}",
                token_pubkey,
                my_pub_hex
            );
        }
    }

    None
}
