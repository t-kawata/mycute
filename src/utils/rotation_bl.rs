use crate::stt_config::ConfigManager;
use crate::utils::crypto::{decrypt, encrypt};
use crate::utils::crypto_registry::{get_registry, CryptoTarget};
use crate::utils::time;
use chrono::{DateTime, NaiveDateTime, Utc};
use rand::{rng, Rng};
use sea_orm::{
    sea_query::{Alias, Expr, Query},
    ConnectionTrait, DatabaseConnection, TransactionTrait,
};
use std::sync::Arc;

/// キーローテーションが必要かどうかをチェックし、必要であれば実行する。
/// 実行はアトミックに行われ、失敗時はロールバックされる。
pub async fn check_and_rotate_keys(
    config_manager: Arc<ConfigManager>,
    db: &DatabaseConnection,
) -> anyhow::Result<()> {
    // 1. Get current settings
    let (current_key, rotation_days, last_rotated_at) = {
        let settings = config_manager.settings.read();
        (
            settings.server.rt_crypto_key.clone(),
            settings.server.rt_crypto_key_rotation_days,
            settings.server.last_rotated_at.clone(),
        )
    };

    // 2. Check if rotation is needed
    let should_rotate = if let Some(s) = last_rotated_at.as_deref() {
        // Try parsing as RFC3339 first (standard), then Naive (legacy/fallback)
        if let Ok(last) = DateTime::parse_from_rfc3339(s) {
            let now = time::now_utc();
            let diff = now.signed_duration_since(last.with_timezone(&Utc));
            if diff.num_days() < rotation_days as i64 {
                log::info!(
                    "Key rotation not needed. (Last: {}, Limit: {} days)",
                    last,
                    rotation_days
                );
                false
            } else {
                true
            }
        } else if let Ok(last_naive) = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
            let now_naive = time::now();
            let diff = now_naive - last_naive;
            if diff.num_days() < rotation_days as i64 {
                log::info!(
                    "Key rotation not needed. (Last: {}, Limit: {} days)",
                    last_naive,
                    rotation_days
                );
                false
            } else {
                true
            }
        } else {
            // Parse error, rotate to be safe
            true
        }
    } else {
        // Never rotated
        true
    };

    if !should_rotate {
        return Ok(());
    }

    log::info!("Starting key rotation...");

    // 3. 新しいキーを生成
    let new_key = generate_new_crypto_key();

    // 4. トランザクション開始 (SeaORM のトランザクション機能を利用)
    let txn = db.begin().await?;

    // 5. 各ターゲットの再暗号化
    let targets = get_registry();
    for target in targets {
        match target {
            CryptoTarget::Db {
                table_name,
                col_name,
                pk_col,
            } => {
                log::info!("Rotating DB table: {}", table_name);
                // 5-1. 全レコード取得
                let backend = db.get_database_backend();
                let query = Query::select()
                    .column(Alias::new(pk_col))
                    .column(Alias::new(col_name))
                    .from(Alias::new(table_name))
                    .to_owned();

                let builder = backend.build(&query);
                let rows = txn.query_all(builder).await?;

                for row in rows {
                    let id: i64 = row.try_get("", pk_col)?;
                    let old_enc_val: String = row.try_get("", col_name)?;

                    // 復号 -> 再暗号化
                    let decrypted = decrypt(&old_enc_val, &current_key)?;
                    let new_enc_val = encrypt(&decrypted, &new_key)?;

                    // 更新
                    let update = Query::update()
                        .table(Alias::new(table_name))
                        .values(vec![(Alias::new(col_name), new_enc_val.into())])
                        .and_where(Expr::col(Alias::new(pk_col)).eq(id))
                        .to_owned();

                    let builder = backend.build(&update);
                    txn.execute(builder).await?;
                }
            }
            CryptoTarget::Config { path, getter: _ } => {
                log::info!("Computing new value for Config: {}", path);
                // Config update handled later
            }
        }
    }

    // 6. 設定ファイルの再暗号化とキー更新 (メモリ上)
    {
        let mut settings = config_manager.settings.write();
        let targets = get_registry();
        for target in targets {
            if let CryptoTarget::Config { path: _, getter } = target {
                let field = getter(&mut *settings);
                if let Some(enc_val) = field {
                    if !enc_val.is_empty() {
                        let decrypted = decrypt(enc_val, &current_key)?;
                        let new_enc = encrypt(&decrypted, &new_key)?;
                        *field = Some(new_enc);
                    }
                }
            }
        }
        settings.server.rt_crypto_key = new_key;
        settings.server.last_rotated_at = Some(time::naive_to_str(&time::now()));
    }

    // 7. 設定ファイルの保存
    config_manager
        .save()
        .map_err(|e| anyhow::anyhow!("Failed to save config: {}", e))?;

    // 8. DBコミット
    txn.commit().await?;

    log::info!("Key rotation completed successfully.");
    Ok(())
}

// Generate 32-char generic key
fn generate_new_crypto_key() -> String {
    const CHARSET: &[u8] =
        b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789!@#$%^&*()-_=+";
    let mut rng = rng();
    let key: String = (0..32)
        .map(|_| {
            let idx = rng.random_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect();
    key
}
