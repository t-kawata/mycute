use mycute::stt_config::ConfigManager;
use mycute::stt_config::Settings;
use mycute::utils::crypto::{self, Ed448KeyValuePair};
use std::fs;
use std::path::PathBuf;

// Helper to create a temp config path
fn temp_config_path(name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!("mycute_test_{}", name));
    path.push("settings.json");
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    path
}

// Helper to create a valid settings with ID
fn create_settings_with_id() -> Settings {
    let mut settings = Settings::new_default();

    // Generate valid ID
    let kp = Ed448KeyValuePair::generate().expect("Generate keypair");
    let sec_hex = hex::encode(kp.secret);
    let pub_hex = hex::encode(kp.public);

    // Encrypt ID (using default key for simplicity in tests, assuming stt_config uses it)
    // Note: ConfigManager uses settings.server.rt_crypto_key which has a default value
    let key = settings.server.rt_crypto_key.clone();

    settings.my_sec = Some(crypto::encrypt(&sec_hex, &key).expect("Encrypt sec"));
    settings.my_pub = Some(crypto::encrypt(&pub_hex, &key).expect("Encrypt pub"));

    settings
}

#[test]
fn test_config_init_valid_no_rem() {
    let path = temp_config_path("valid_no_rem");
    let settings = create_settings_with_id();

    // Save settings
    let content = serde_json::to_string_pretty(&settings).unwrap();
    fs::write(&path, content).unwrap();

    // Should NOT panic (just logs info about missing my_rem)
    let _ = ConfigManager::new(None, Some(path.to_string_lossy().to_string()));

    // Cleanup
    let _ = fs::remove_file(path);
}

#[test]
#[should_panic(expected = "CRITICAL: my_rem corrupted or tampered")]
fn test_config_init_corrupted_rem() {
    let path = temp_config_path("corrupted_rem");
    let mut settings = create_settings_with_id();

    // Set invalid my_rem (random garbage)
    let key = settings.server.rt_crypto_key.clone();
    settings.my_rem = Some(crypto::encrypt("INVALID_PAYLOAD:SIG", &key).unwrap());

    // Save settings
    let content = serde_json::to_string_pretty(&settings).unwrap();
    fs::write(&path, content).unwrap();

    // Should panic due to integrity check failure
    let _ = ConfigManager::new(None, Some(path.to_string_lossy().to_string()));
}

#[test]
#[should_panic(expected = "CRITICAL: my_rem corrupted or tampered")]
fn test_config_init_tampered_sig() {
    let path = temp_config_path("tampered_sig");
    let mut settings = create_settings_with_id();
    let key = settings.server.rt_crypto_key.clone();

    // Create a seemingly valid format but with invalid signature
    // Format: JSON:SIG_HEX
    let fake_payload = r#"{"ca_states":{}}"#;
    let fake_sig = "00".repeat(114); // 114 bytes for Ed448 signature
    let raw = format!("{}:{}", fake_payload, fake_sig);

    settings.my_rem = Some(crypto::encrypt(&raw, &key).unwrap());

    // Save settings
    let content = serde_json::to_string_pretty(&settings).unwrap();
    fs::write(&path, content).unwrap();

    // Should panic due to signature verification failure
    let _ = ConfigManager::new(None, Some(path.to_string_lossy().to_string()));
}
