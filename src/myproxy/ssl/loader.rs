use crate::constants::PEM_BEGIN;
use crate::mycute_settings::ConfigManager;
use anyhow::Result;
use base64::{engine::general_purpose, Engine as _};
use tokio_rustls::rustls::ServerConfig;

/// CLモード用: リソースから証明書をロードします。
/// 生成やインストールは一切行いません。証明書がない場合はエラーを返します。
pub fn load_certs(config_manager: &ConfigManager) -> Result<ServerConfig> {
    let (cert_b64, key_b64) = {
        let settings = config_manager.settings.read();
        (
            settings.proxy_certificate.clone(),
            settings.proxy_private_key.clone(),
        )
    };

    if let (Some(cert_str), Some(key_str)) = (cert_b64, key_b64) {
        log::info!("Loading existing SSL certificates from settings...");
        load_server_config(&cert_str, &key_str)
    } else {
        anyhow::bail!("SSL certificates not found. Please run 'mycute ca' with administrator privileges to setup certificates first.");
    }
}

fn load_server_config(cert_input: &str, key_input: &str) -> Result<ServerConfig> {
    // 入力が PEM か Base64 エンコードされた PEM かを判定。
    let cert_pem = if cert_input.trim().starts_with(PEM_BEGIN) {
        cert_input.to_string()
    } else {
        String::from_utf8(general_purpose::STANDARD.decode(cert_input)?)?
    };

    let key_pem = if key_input.trim().starts_with(PEM_BEGIN) {
        key_input.to_string()
    } else {
        String::from_utf8(general_purpose::STANDARD.decode(key_input)?)?
    };

    let mut cert_cursor = std::io::Cursor::new(cert_pem.as_bytes());
    let mut key_cursor = std::io::Cursor::new(key_pem.as_bytes());

    let certs = rustls_pemfile::certs(&mut cert_cursor).collect::<Result<Vec<_>, _>>()?;

    let private_key = rustls_pemfile::private_key(&mut key_cursor)?
        .ok_or_else(|| anyhow::anyhow!("No private key found"))?;

    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, private_key)?;

    Ok(config)
}
