use crate::stt_config::ConfigManager;
use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use fastcert::ca::CertificateAuthority;
use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair, Issuer};
use crate::constants::{
    MYCUTE_PROXY_SUFFIX, MYCUTE_OSCA_TEMP_DIR_PREFIX, ENV_OSCAROOT,
    DOMAIN_LOCALHOST
};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[derive(Debug, serde::Serialize)]
pub enum SetupStatus {
    Created,
    Existing,
    Updated,
}

pub fn create_certs_if_missing(config_manager: &ConfigManager) -> Result<SetupStatus> {
    log::info!("Checking SSL certificates...");

    // 1. 設定から既存のサーバー証明書と Root OSCA を取得
    let (cert_b64, key_b64, osca_cert_b64, osca_key_b64, osca_expire_b64) = {
        let settings = config_manager.settings.read();
        (
            settings.proxy_certificate.clone(),
            settings.proxy_private_key.clone(),
            settings.osca_certificate.clone(),
            settings.osca_private_key.clone(),
            settings.osca_expire.clone(),
        )
    };

    // 有効期限のチェック
    let is_expired = if let Some(expire_str) = osca_cert_b64.as_ref().and_then(|_| osca_key_b64.as_ref()).and_then(|_| osca_expire_b64.as_ref()) {
        if let Ok(expire_dt) = chrono::DateTime::parse_from_rfc3339(expire_str) {
            let now = chrono::Utc::now();
            if expire_dt < now {
                 log::warn!("Root CA has expired at {}. Forcing re-generation...", expire_str);
                 true
            } else {
                 false
            }
        } else {
            log::warn!("Invalid OSCA expiration format in settings. Forcing re-generation...");
            true
        }
    } else if osca_cert_b64.is_some() && osca_key_b64.is_some() {
        // 設定に証明書はあるが期限情報がない場合
        log::info!("OSCA certificate found but expiration date is missing in settings.");
        false // とりあえず続行（後で抽出する）
    } else {
        false
    };

    // 既にすべての証明書が揃っており、かつ期限切れでないか確認
    if cert_b64.is_some() && key_b64.is_some() && !is_expired {
        if let (Some(c_b64), Some(_)) = (osca_cert_b64.clone(), osca_key_b64.clone()) {
            log::info!("SSL certificates already exist in settings. Ensuring trust...");
            
            // 既存のOSCA証明書をシステムに再登録（信頼修復）する
            if let Ok(c_pem) = String::from_utf8(general_purpose::STANDARD.decode(&c_b64).unwrap_or_default()) {
                let temp_dir = std::env::temp_dir().join(format!("{}{}", MYCUTE_OSCA_TEMP_DIR_PREFIX, uuid::Uuid::new_v4()));
                if !temp_dir.exists() {
                    let _ = std::fs::create_dir_all(&temp_dir);
                }
                
                let ca = CertificateAuthority::new(temp_dir.clone());
                if let Ok(_) = std::fs::write(ca.cert_path(), &c_pem) {
                    std::env::set_var(crate::constants::ENV_OSCAROOT, temp_dir.to_string_lossy().to_string());
                    
                    #[cfg(target_os = "macos")]
                    {
                        if osca_expire_b64.is_none() {
                            if let Ok(expire_str) = extract_cert_expiration(&ca.cert_path()) {
                                log::info!("Extracted OSCA expiration from existing cert: {}", expire_str);
                                {
                                    let mut settings = config_manager.settings.write();
                                    settings.osca_expire = Some(expire_str);
                                }
                                if let Err(e) = config_manager.save() {
                                    log::error!("Failed to save settings with osca_expire: {}", e);
                                }
                            }
                        }
                        let _ = ensure_macos_osca_trust(&ca);
                    }

                    #[cfg(not(target_os = "macos"))]
                    if let Err(e) = fastcert::ca::install() {
                        log::warn!("Failed to reinstall Root OSCA: {}", e);
                    }
                }
                let _ = std::fs::remove_dir_all(&temp_dir);
            }
        }

        return Ok(SetupStatus::Existing);
    }

    log::info!("Preparing SSL certificates for {}...", MYCUTE_PROXY_SUFFIX);

    // 2. 標準的な一時ディレクトリを使用して OSCA 作業領域を確保
    let temp_dir = std::env::temp_dir().join(format!("{}{}", MYCUTE_OSCA_TEMP_DIR_PREFIX, uuid::Uuid::new_v4()));
    if !temp_dir.exists() {
        std::fs::create_dir_all(&temp_dir).context("Failed to create temporary OSCA directory")?;
    }

    // fastcert の OSCA インスタンス
    let mut ca = CertificateAuthority::new(temp_dir.clone());
    
    // 3. Root OSCA の取得または生成
    let (ca_cert_pem, ca_key_pem) = if let (Some(c_b64), Some(k_b64)) = (osca_cert_b64.clone(), osca_key_b64.clone()) {
        log::info!("Loading existing Root OSCA from settings and reinstalling...");
        let c_pem = String::from_utf8(general_purpose::STANDARD.decode(c_b64)?)?;
        let k_pem = String::from_utf8(general_purpose::STANDARD.decode(k_b64)?)?;
        
        std::fs::write(ca.cert_path(), &c_pem)?;
        std::fs::write(ca.key_path(), &k_pem)?;
        (c_pem, k_pem)
    } else {
        log::info!("Generating new Root OSCA...");
        ca.create_ca().context("Failed to create Root OSCA")?;
        ca.save().context("Failed to save Root OSCA")?;
        
        let c_pem = std::fs::read_to_string(ca.cert_path())?;
        let k_pem = std::fs::read_to_string(ca.key_path())?;
        
        // 設定に保存
        {
            let mut settings = config_manager.settings.write();
            settings.osca_certificate = Some(general_purpose::STANDARD.encode(&c_pem));
            settings.osca_private_key = Some(general_purpose::STANDARD.encode(&k_pem));
            
            // 有効期限を抽出して保存
            match extract_cert_expiration(&ca.cert_path()) {
                Ok(expire_str) => {
                    log::info!("OSCA Expiration detected: {}", expire_str);
                    settings.osca_expire = Some(expire_str);
                },
                Err(e) => log::error!("Failed to extract OSCA expiration: {}", e),
            }
        }
        (c_pem, k_pem)
    };

    // 4. OSCA をシステムトラストストアにインストール
    std::env::set_var(ENV_OSCAROOT, temp_dir.to_string_lossy().to_string());
    
    log::info!("Ensuring Root OSCA is installed to system trust store...");
    
    // Linux/Windows: fastcert (mkcert logic) に任せる
    #[cfg(not(target_os = "macos"))]
    if let Err(e) = fastcert::ca::install() {
        log::warn!("Failed to install Root OSCA (Permission denied?): {}", e);
    }

    // macOS: fastcert::install は Login キーチェーンに (信頼なしで) 入れる可能性があるためスキップし、
    // 明示的に System キーチェーンに "trustRoot" として登録するコマンドのみを実行する。
    // これにより "Always Trust" が確実に適用され、重複も防ぐ。
    // macOS: 明示的に Login キーチェーンに "trustRoot" として登録する。
    #[cfg(target_os = "macos")]
    {
        let _ = ensure_macos_osca_trust(&ca);
    }
    
    // 5. サーバー証明書の生成
    let wildcard_domain = format!("*{}", MYCUTE_PROXY_SUFFIX);
    log::info!("Generating server certificate for {}...", wildcard_domain);
    let server_key_pair = KeyPair::generate().context("Failed to generate server key pair")?;
    let ca_key_pair = KeyPair::from_pem(&ca_key_pem).context("Failed to parse OSCA key")?;
    let issuer = Issuer::from_ca_cert_pem(&ca_cert_pem, &ca_key_pair).context("Failed to create Issuer from OSCA")?;
    
    let mut params = CertificateParams::new(vec![wildcard_domain.clone(), DOMAIN_LOCALHOST.to_string()])?;
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, &wildcard_domain);
    params.distinguished_name = dn;

    let cert = params.signed_by(&server_key_pair, &issuer).context("Failed to sign server certificate")?;
    let cert_pem = cert.pem();
    let server_key_pem = server_key_pair.serialize_pem();
    
    // 6. 設定に保存 (Base64)
    {
        let mut settings = config_manager.settings.write();
        settings.proxy_certificate = Some(general_purpose::STANDARD.encode(&cert_pem));
        settings.proxy_private_key = Some(general_purpose::STANDARD.encode(&server_key_pem));
    }
    if let Err(e) = config_manager.save() {
        log::error!("Failed to save settings with new certificates: {}", e);
    }

    // 7. パーミッション修正 (644)
    // 一般ユーザー(Client)が読み取れるように rw-r--r-- に設定する
    #[cfg(unix)]
    {
        if let Err(e) = std::fs::set_permissions(&config_manager.path, std::fs::Permissions::from_mode(0o644)) {
             log::warn!("Failed to set 644 permission to settings.json at {:?}: {}", config_manager.path, e);
        }
    }

    // 一時ディレクトリのクリーンアップ
    let _ = std::fs::remove_dir_all(&temp_dir);
    
    Ok(SetupStatus::Created)
}

/// macOS固有のルート証明書信頼設定フロー
#[cfg(target_os = "macos")]
fn ensure_macos_osca_trust(ca: &CertificateAuthority) -> Result<()> {
    log::info!("Ensuring MacOS Root OSCA trust in User Keychain...");
    
    let cert_path = ca.cert_path().to_string_lossy().to_string();
    
    // 1. 古い証明書の削除 (Common Nameでマッチング)
    let mut cn = String::new();
    if let Ok(output) = std::process::Command::new("openssl")
        .args(&["x509", "-in", &cert_path, "-noout", "-subject", "-nameopt", "RFC2253"])
        .output() 
    {
        let subject = String::from_utf8_lossy(&output.stdout);
        if let Some(start) = subject.find("CN=") {
            let rest = &subject[start + 3..];
            let end = rest.find([',', '\n']).unwrap_or(rest.len());
            cn = rest[..end].trim().to_string();
        }
    }
    
    if !cn.is_empty() {
        log::debug!("Cleaning up existing certificates with CN: {}", cn);
        let _ = std::process::Command::new("security")
            .args(&["delete-certificate", "-c", &cn])
            .output();
    }

    // 2. 信頼済み証明書として追加 (User Login Keychain)
    log::info!("Installing OSCA to User Keychain: {}", cert_path);
    let keychain_path = std::env::var("HOME").unwrap_or_default() + "/Library/Keychains/login.keychain-db";

    let output = std::process::Command::new("security")
        .args(&["add-trusted-cert", "-d", "-r", "trustRoot", "-k"])
        .arg(&keychain_path)
        .arg(&cert_path)
        .output()?;

    if output.status.success() {
        log::info!("Successfully ensured Root OSCA trust via 'security' command.");
        Ok(())
    } else {
        let err = String::from_utf8_lossy(&output.stderr);
        log::warn!("Primary security command failed, retrying without explicit keychain path: {}", err);
        
        let output2 = std::process::Command::new("security")
            .args(&["add-trusted-cert", "-d", "-r", "trustRoot"])
            .arg(&cert_path)
            .output()?;

        if output2.status.success() {
            log::info!("Successfully ensured Root OSCA trust (default keychain).");
            Ok(())
        } else {
            let err2 = String::from_utf8_lossy(&output2.stderr);
            anyhow::bail!("Failed to trust OSCA on MacOS: {}", err2);
        }
    }
}

/// 証明書ファイルから有効期限 (Not After) を抽出し、RFC3339形式で返す
fn extract_cert_expiration(cert_path: &std::path::Path) -> Result<String> {
    log::debug!("Extracting expiration from {:?}", cert_path);
    
    // openssl x509 -noout -enddate -dateopt iso_8601 -in <path>
    // 出力例: notAfter=2036-02-02T11:42:35Z
    let output = std::process::Command::new("openssl")
        .args(&["x509", "-noout", "-enddate", "-dateopt", "iso_8601", "-in"])
        .arg(cert_path)
        .output()?;

    if output.status.success() {
        let text = String::from_utf8_lossy(&output.stdout);
        if let Some(pos) = text.find('=') {
            return Ok(text[pos + 1..].trim().to_string());
        }
    }
    
    // フォールバック: 標準形式 (notAfter=Feb  2 12:42:35 2036 GMT) をパース
    let output = std::process::Command::new("openssl")
        .args(&["x509", "-noout", "-enddate", "-in"])
        .arg(cert_path)
        .output()?;
        
    if output.status.success() {
        let text = String::from_utf8_lossy(&output.stdout);
        if let Some(pos) = text.find('=') {
            let raw_date = text[pos + 1..].trim();
            // %b %e %H:%M:%S %Y %Z -> Feb  2 12:42:35 2036 GMT
            if let Ok(dt) = chrono::DateTime::parse_from_str(raw_date, "%b %e %H:%M:%S %Y %Z") {
                return Ok(dt.to_rfc3339());
            }
            // GMTなしの場合のパース
            let raw_no_gmt = raw_date.replace(" GMT", "");
            if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(&raw_no_gmt, "%b %e %H:%M:%S %Y") {
                 let dt_utc = chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(dt, chrono::Utc);
                 return Ok(dt_utc.to_rfc3339());
            }
        }
    }

    anyhow::bail!("Failed to extract expiration date from certificate")
}
