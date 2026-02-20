use chrono::{Duration, Utc};
use mycute::config::settings::Env;
use mycute::stt_config::DbDriver;
use mycute::stt_config::{ConfigManager, Settings};
use mycute::utils::crypto::{decrypt, encrypt};
use mycute::utils::db::get_db;
use mycute::utils::init::LogLevel;
use mycute::utils::rotation_bl::check_and_rotate_keys;
use mycute::utils::time;
use std::fs;
use std::sync::Arc;
use tempfile::NamedTempFile;

#[tokio::test]
async fn test_key_rotation_full_cycle() -> anyhow::Result<()> {
    // 1. Setup Temporary Config & DB
    // Load real settings for DB credentials
    let real_settings_content = fs::read_to_string("settings.json")?;
    let real_settings: Settings = serde_json::from_str(&real_settings_content)?;

    // Create temporary settings for rotation test (copying DB info)
    let tmp_file = NamedTempFile::new()?;
    let tmp_path = tmp_file.path();

    let mut test_settings = Settings::new_default();
    test_settings.storage = real_settings.storage.clone();

    let initial_key = "initial-test-key-32-chars-long!!";
    test_settings.server.rt_crypto_key = initial_key.to_string();
    test_settings.server.rt_crypto_key_rotation_days = 1;
    // Force rotation by setting last_rotated_at to 2 days ago
    let two_days_ago = time::now_utc() - Duration::days(2);
    test_settings.server.last_rotated_at = Some(two_days_ago.to_rfc3339());

    // Seed some encrypted data in config
    let secret_val = "sensitive-node-identity-sec";
    test_settings.my_sec = Some(encrypt(secret_val, initial_key)?);

    let content = serde_json::to_string_pretty(&test_settings)?;
    fs::write(tmp_path, content)?;

    let config_manager = Arc::new(ConfigManager::new(
        None,
        Some(tmp_path.to_string_lossy().to_string()),
    ));

    // 44. Connect to DB (using the temporary SQLite DB)
    // We override the DB settings to use a temporary SQLite file
    let _sqlite_path = tmp_path.parent().unwrap().join("test_rotation.sqlite");
    test_settings.storage.rw_db.driver = DbDriver::Sqlite;
    test_settings.storage.rw_db.host = "test_rotation.sqlite".to_string(); // Filename
                                                                           // db_dir_path overrides to temp dir
    test_settings.storage.db_dir_path = tmp_path.parent().unwrap().to_string_lossy().to_string();

    let env = Env::from_settings(&test_settings.storage);
    let db_pools = get_db(&env, &LogLevel::Debug).await?;
    let db = &db_pools.rw;

    // Create table locally for SQLite (since we don't have migrations in this test env)
    let create_table_sql = r#"
    CREATE TABLE IF NOT EXISTS cryptos (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        apx_id INTEGER NOT NULL,
        vdr_id INTEGER NOT NULL,
        `key` TEXT NOT NULL,
        `value` TEXT NOT NULL,
        created_at DATETIME NOT NULL,
        updated_at DATETIME NOT NULL
    );
    "#;
    use sea_orm::{ConnectionTrait, DatabaseBackend, Statement};
    db.execute(Statement::from_string(
        DatabaseBackend::Sqlite,
        create_table_sql,
    ))
    .await?;

    // 2. Seed Encrypted Data in DB (cryptos table)
    let test_key_name = "test_rotation_marker";
    let db_secret = "db-secret-token-12345";
    let encrypted_db_secret = encrypt(db_secret, initial_key)?;

    // Ensure cleanup
    let delete_query = sea_orm::sea_query::Query::delete()
        .from_table(sea_orm::sea_query::Alias::new("cryptos"))
        .and_where(
            sea_orm::sea_query::Expr::col(sea_orm::sea_query::Alias::new("key")).eq(test_key_name),
        )
        .to_owned();
    let builder = db.get_database_backend();
    db.execute(builder.build(&delete_query)).await?;

    let insert_query = sea_orm::sea_query::Query::insert()
        .into_table(sea_orm::sea_query::Alias::new("cryptos"))
        .columns(vec![
            sea_orm::sea_query::Alias::new("apx_id"),
            sea_orm::sea_query::Alias::new("vdr_id"),
            sea_orm::sea_query::Alias::new("key"),
            sea_orm::sea_query::Alias::new("value"),
            sea_orm::sea_query::Alias::new("created_at"),
            sea_orm::sea_query::Alias::new("updated_at"),
        ])
        .values_panic(vec![
            0.into(),
            0.into(),
            test_key_name.into(),
            encrypted_db_secret.into(),
            Utc::now().naive_utc().into(),
            Utc::now().naive_utc().into(),
        ])
        .to_owned();
    db.execute(builder.build(&insert_query)).await?;

    // 3. RUN ROTATION
    check_and_rotate_keys(config_manager.clone(), db).await?;

    // 4. VERIFY
    let updated_settings = config_manager.settings.read();
    let new_key = &updated_settings.server.rt_crypto_key;

    assert_ne!(new_key, initial_key, "Key should have changed");
    assert!(
        updated_settings.server.last_rotated_at.is_some(),
        "Timestamp should be updated"
    );

    // Verify Config Re-encryption
    let rotated_my_sec = updated_settings
        .my_sec
        .as_ref()
        .expect("my_sec should exist");
    let decrypted_my_sec = decrypt(rotated_my_sec, new_key)?;
    assert_eq!(
        decrypted_my_sec, secret_val,
        "Config data should be correctly re-encrypted"
    );

    // Verify DB Re-encryption
    let select_query = sea_orm::sea_query::Query::select()
        .column(sea_orm::sea_query::Alias::new("value"))
        .from(sea_orm::sea_query::Alias::new("cryptos"))
        .and_where(
            sea_orm::sea_query::Expr::col(sea_orm::sea_query::Alias::new("key")).eq(test_key_name),
        )
        .to_owned();

    let row = db
        .query_one(builder.build(&select_query))
        .await?
        .expect("Test record should exist");

    let rotated_db_val: String = row.try_get("", "value")?;
    let decrypted_db_val = decrypt(&rotated_db_val, new_key)?;
    assert_eq!(
        decrypted_db_val, db_secret,
        "DB data should be correctly re-encrypted"
    );

    // Verify OLD key fails
    assert!(
        decrypt(&rotated_db_val, initial_key).is_err(),
        "Old key should no longer work for decryption"
    );

    // Cleanup
    db.execute(builder.build(&delete_query)).await?;

    println!("Key Rotation Test Passed Successfully.");
    Ok(())
}
