use crate::constants::{
    ERR_ENCRYPT, ERR_INVALID_CA_TOKEN, ERR_SAVE, ST_BAD_REQUEST, ST_INTERNAL_SERVER_ERROR,
};
use crate::mode::rt::{
    rtbl::identities_bl,
    rtreq::ca_req::RegisterCaTokenReq,
    rtres::{ca_res::RegisterCaTokenRes, errs_res::ApiError},
};
use crate::mycute_settings::ConfigManager;
use crate::utils::{crypto, time};
use std::sync::Arc;

pub async fn register_ca_token(
    config_manager: Arc<ConfigManager>,
    req: RegisterCaTokenReq,
) -> Result<RegisterCaTokenRes, ApiError> {
    // Get My Public Key
    let my_pub_hex = identities_bl::get_pubkey(config_manager.clone())
        .await
        .map_err(|e| {
            ApiError::new_system(
                ST_INTERNAL_SERVER_ERROR,
                ERR_INVALID_CA_TOKEN,
                format!("Failed to get my pubkey: {}", e),
            )
        })?;

    let valid = identities_bl::verify_ca_token(&my_pub_hex, &req.ca_token, time::now_ts_ms() as u64);

    if !valid {
        return Err(ApiError::new_system(
            ST_BAD_REQUEST,
            ERR_INVALID_CA_TOKEN,
            "Invalid CA Token signature or expired (Owner verification failed).",
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
            format!("Failed to encrypt CA token: {}", e),
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

    Ok(RegisterCaTokenRes {
        success: true,
        message:
            "CA Token registered successfully. You are now authorized as L3 (Official Citizen)."
                .to_string(),
    })
}
