use crate::constants::{
    DOMAIN_LOCALHOST, ENV_OSCAROOT, MYCUTE_OSCA_TEMP_DIR_PREFIX, MYCUTE_PROXY_SUFFIX,
};
use crate::mycute_settings::ConfigManager;
use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine as _};
use fastcert::ca::CertificateAuthority;
use rcgen::{CertificateParams, DistinguishedName, DnType, Issuer, KeyPair};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use time;
use x509_parser::oid_registry::OID_X509_COMMON_NAME;
use x509_parser::pem;

#[derive(Debug, serde::Serialize)]
pub enum SetupStatus {
    Created,
    Existing,
    Updated,
}

pub async fn create_certs_if_missing(config_manager: &ConfigManager) -> Result<SetupStatus> {
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
    let is_expired = if let Some(expire_str) = osca_cert_b64
        .as_ref()
        .and_then(|_| osca_key_b64.as_ref())
        .and_then(|_| osca_expire_b64.as_ref())
    {
        if let Ok(expire_dt) = chrono::DateTime::parse_from_rfc3339(expire_str) {
            let now = chrono::Utc::now();
            if expire_dt < now {
                log::warn!(
                    "Root CA has expired at {}. Forcing re-generation...",
                    expire_str
                );
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
            if let Ok(c_pem) =
                String::from_utf8(general_purpose::STANDARD.decode(&c_b64).unwrap_or_default())
            {
                let temp_dir = std::env::temp_dir().join(format!(
                    "{}{}",
                    MYCUTE_OSCA_TEMP_DIR_PREFIX,
                    uuid::Uuid::new_v4()
                ));
                if !temp_dir.exists() {
                    let _ = std::fs::create_dir_all(&temp_dir);
                }

                let ca = CertificateAuthority::new(temp_dir.clone());
                if let Ok(_) = std::fs::write(ca.cert_path(), &c_pem) {
                    std::env::set_var(
                        ENV_OSCAROOT,
                        temp_dir.to_string_lossy().to_string(),
                    );

                    if osca_expire_b64.is_none() {
                        if let Ok(expire_str) = extract_cert_expiration(&ca.cert_path()) {
                            log::info!(
                                "Extracted OSCA expiration from existing cert: {}",
                                expire_str
                            );
                            {
                                let mut settings = config_manager.settings.write();
                                settings.osca_expire = Some(expire_str);
                            }
                            if let Err(e) = config_manager.save_db().await {
                                log::error!("Failed to save settings with osca_expire: {}", e);
                            }
                        }
                    }

                    // MacOS/Windows で証明書が既に信頼ストアにあるか確認。
                    // 存在しない（手動削除された）場合のみ再インストールを呼び出すことでダイアログの重複を回避。
                    let cn = get_cert_common_name(&ca.cert_path()).unwrap_or_default();
                    log::info!("Checking trust for CN: '{}'", cn);
                    let already_trusted = {
                        #[cfg(target_os = "macos")]
                        {
                            !cn.is_empty() && is_macos_osca_already_trusted(&cn)
                        }
                        #[cfg(windows)]
                        {
                            !cn.is_empty() && is_windows_osca_already_trusted(&cn)
                        }
                        #[cfg(not(any(target_os = "macos", windows)))]
                        {
                            false
                        }
                    };

                    if already_trusted {
                        log::info!("Root OSCA is already trusted (CN: {})", cn);
                    } else {
                        log::info!("Root OSCA trust not found. Enforcing reinstall...");
                        #[cfg(target_os = "macos")]
                        {
                            let _ = ensure_macos_osca_trust(&ca);
                        }
                        #[cfg(not(target_os = "macos"))]
                        if let Err(e) = fastcert::ca::install() {
                            log::warn!("Failed to reinstall Root OSCA: {}", e);
                        }
                    }
                }
                let _ = std::fs::remove_dir_all(&temp_dir);
            }
        }

        return Ok(SetupStatus::Existing);
    }

    log::info!("Preparing SSL certificates for {}...", MYCUTE_PROXY_SUFFIX);

    // 2. 標準的な一時ディレクトリを使用して OSCA 作業領域を確保
    let temp_dir = std::env::temp_dir().join(format!(
        "{}{}",
        MYCUTE_OSCA_TEMP_DIR_PREFIX,
        uuid::Uuid::new_v4()
    ));
    if !temp_dir.exists() {
        std::fs::create_dir_all(&temp_dir).context("Failed to create temporary OSCA directory")?;
    }

    // fastcert の OSCA インスタンス
    let mut ca = CertificateAuthority::new(temp_dir.clone());

    // 3. Root OSCA の取得または生成
    let (ca_cert_pem, ca_key_pem) =
        if let (Some(c_b64), Some(k_b64)) = (osca_cert_b64.clone(), osca_key_b64.clone()) {
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
                    }
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
    let issuer = Issuer::from_ca_cert_pem(&ca_cert_pem, &ca_key_pair)
        .context("Failed to create Issuer from OSCA")?;

    let mut params =
        CertificateParams::new(vec![wildcard_domain.clone(), DOMAIN_LOCALHOST.to_string()])?;
    let mut dn = DistinguishedName::new();
    dn.push(DnType::CommonName, &wildcard_domain);
    params.distinguished_name = dn;

    let cert = params
        .signed_by(&server_key_pair, &issuer)
        .context("Failed to sign server certificate")?;
    let cert_pem = cert.pem();
    let server_key_pem = server_key_pair.serialize_pem();

    // 6. 設定に保存 (Base64)
    {
        let mut settings = config_manager.settings.write();
        settings.proxy_certificate = Some(general_purpose::STANDARD.encode(&cert_pem));
        settings.proxy_private_key = Some(general_purpose::STANDARD.encode(&server_key_pem));
    }
    if let Err(e) = config_manager.save_db().await {
        log::error!("Failed to save settings with new certificates: {}", e);
    }

    // 7. パーミッション修正 (644)
    // 一般ユーザー(Client)が読み取れるように rw-r--r-- に設定する
    #[cfg(unix)]
    {
        if let Err(e) =
            std::fs::set_permissions(&config_manager.path, std::fs::Permissions::from_mode(0o644))
        {
            log::warn!(
                "Failed to set 644 permission to settings.json at {:?}: {}",
                config_manager.path,
                e
            );
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
    let cn = get_cert_common_name(&ca.cert_path()).unwrap_or_default();

    if !cn.is_empty() {
        log::debug!("Cleaning up existing certificates with CN: {}", cn);
        let _ = std::process::Command::new("security")
            .args(&["delete-certificate", "-c", &cn])
            .output();
    }

    // 2. 信頼済み証明書として追加 (User Login Keychain)
    log::info!("Installing OSCA to User Keychain: {}", cert_path);
    let keychain_path =
        std::env::var("HOME").unwrap_or_default() + "/Library/Keychains/login.keychain-db";

    let output = std::process::Command::new("security")
        .args(&["add-trusted-cert", "-d", "-r", "trustRoot", "-k"])
        .arg(&keychain_path)
        .arg(&cert_path)
        .output()?;

    if output.status.success() {
        log::info!("Successfully ensured Root OSCA trust via 'security' command.");
        return Ok(());
    }

    let err = String::from_utf8_lossy(&output.stderr);
    log::warn!(
        "Primary security command failed, retrying without explicit keychain path: {}",
        err
    );

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

/// MacOSにおいて指定された共通名（CN）の証明書が既にキーチェーンに存在し、信頼されているか確認する
fn is_macos_osca_already_trusted(cn: &str) -> bool {
    let cn = cn.trim();
    if cn.is_empty() {
        return false;
    }
    log::info!("Checking MacOS Root OSCA trust for CN: '{}'...", cn);
    
    // 特定のキーチェーンを指定せず検索することで、ログイン/システム双方を対象とする
    let output = std::process::Command::new("security")
        .args(&["find-certificate", "-c", cn])
        .output();

    match output {
        Ok(out) => {
            let found = out.status.success();
            if found {
                log::info!("Root OSCA found in keychain(s).");
            } else {
                log::info!("Root OSCA not found in keychain(s).");
            }
            found
        }
        Err(e) => {
            log::warn!("Failed to execute security command: {}", e);
            false
        }
    }
}

/// Windowsにおいて指定された共通名（CN）の証明書が既にルートストアに存在するか確認する
#[allow(dead_code)]
fn is_windows_osca_already_trusted(cn: &str) -> bool {
    log::debug!("Checking if Windows Root OSCA is already trusted: {}", cn);
    let output = std::process::Command::new("certutil")
        .args(&["-verifystore", "Root", cn])
        .output();

    match output {
        Ok(out) => out.status.success(),
        Err(e) => {
            log::warn!("Failed to execute certutil command: {}", e);
            false
        }
    }
}

/// 証明書ファイルから共通名 (Common Name) を抽出する
fn get_cert_common_name(cert_path: &std::path::Path) -> Result<String> {
    let cert_pem = std::fs::read_to_string(cert_path).context("Failed to read certificate file")?;
    let (_, pem) = pem::parse_x509_pem(cert_pem.as_bytes()).context("Failed to parse PEM")?;
    let cert = pem.parse_x509().context("Failed to parse X.509 certificate")?;

    for rdn in cert.subject().iter() {
        for attr in rdn.iter() {
            if *attr.attr_type() == OID_X509_COMMON_NAME {
                let cn = attr.as_str().context("Common Name is not a valid string")?;
                return Ok(cn.trim().to_string());
            }
        }
    }

    anyhow::bail!("Common Name (CN) not found in certificate")
}

/// 証明書ファイルから有効期限 (Not After) を抽出し、RFC3339形式で返す
fn extract_cert_expiration(cert_path: &std::path::Path) -> Result<String> {
    let cert_pem = std::fs::read_to_string(cert_path).context("Failed to read certificate file")?;
    let (_, pem) = pem::parse_x509_pem(cert_pem.as_bytes()).context("Failed to parse PEM")?;
    let cert = pem.parse_x509().context("Failed to parse X.509 certificate")?;

    let not_after = cert.validity().not_after;
    let dt = not_after.to_datetime();
    let rfc3339_fmt = time::format_description::well_known::Rfc3339;
    let formatted = dt.format(&rfc3339_fmt).map_err(|e| anyhow::anyhow!("Failed to format date: {}", e))?;

    Ok(formatted)
}
