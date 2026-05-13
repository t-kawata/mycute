use crate::mycute_settings::ConfigManager;
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

/// キーローテーションが必要かどうかを判定する（純粋関数）
///
/// # 引数
/// - `last_rotated_at`: 前回のローテーション日時（None = 未ローテーション）
/// - `rotation_days`: ローテーション間隔（日数）
///
/// # 戻り値
/// ローテーションが必要な場合は `true`
pub fn should_rotate_keys(last_rotated_at: &Option<String>, rotation_days: u64) -> bool {
    if let Some(s) = last_rotated_at.as_deref() {
        // Try parsing as RFC3339 first (standard), then Naive (legacy/fallback)
        if let Ok(last) = DateTime::parse_from_rfc3339(s) {
            let now = time::now_utc();
            let diff = now.signed_duration_since(last.with_timezone(&Utc));
            diff.num_days() >= rotation_days as i64
        } else if let Ok(last_naive) = NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S") {
            let now_naive = time::now();
            let diff = now_naive - last_naive;
            diff.num_days() >= rotation_days as i64
        } else {
            // Parse error, rotate to be safe
            true
        }
    } else {
        // Never rotated
        true
    }
}

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
    let should_rotate = should_rotate_keys(&last_rotated_at, rotation_days);

    log::info!(
        "[DIAG] rotation_bl: should_rotate={}, current_key={}, last_rotated_at={:?}, rotation_days={}",
        should_rotate,
        &current_key[..current_key.len().min(16)],
        last_rotated_at,
        rotation_days,
    );

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

    // 7. 設定ファイルの保存 (既存のトランザクションを使用)
    config_manager
        .save_db_with_conn(&txn)
        .await
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::time;

    /// `last_rotated_at = None` の場合はローテーションが必要と判定される
    #[test]
    fn test_should_rotate_when_never_rotated() {
        assert!(should_rotate_keys(&None, 90));
    }

    /// `last_rotated_at` がパース不能な形式の場合は安全側に倒してローテーション
    #[test]
    fn test_should_rotate_when_parse_error() {
        let invalid = Some("not-a-date".to_string());
        assert!(should_rotate_keys(&invalid, 90));
    }

    /// Naive 形式の日付がローテーション期間内ならローテーション不要
    #[test]
    fn test_should_not_rotate_naive_within_period() {
        let recent = time::naive_to_str(&time::now());
        assert!(!should_rotate_keys(&Some(recent), 90));
    }

    /// Naive 形式の日付がローテーション期間を超えていればローテーション必要
    #[test]
    fn test_should_rotate_naive_past_period() {
        let old = "2025-01-01T00:00:00".to_string();
        assert!(should_rotate_keys(&Some(old), 90));
    }

    /// RFC3339 形式の日付がローテーション期間内ならローテーション不要
    #[test]
    fn test_should_not_rotate_rfc3339_within_period() {
        let now = chrono::Utc::now();
        let recent = now.to_rfc3339();
        assert!(!should_rotate_keys(&Some(recent), 90));
    }

    /// RFC3339 形式の日付がローテーション期間を超えていればローテーション必要
    #[test]
    fn test_should_rotate_rfc3339_past_period() {
        let old = "2025-01-01T00:00:00+00:00".to_string();
        assert!(should_rotate_keys(&Some(old), 90));
    }

    /// ensure_unique_secret_keys で設定される Naive 形式が正しく認識される
    #[test]
    fn test_naive_format_from_ensure_unique_secret_keys_is_parsable() {
        let naive_str = time::naive_to_str(&time::now());
        assert!(NaiveDateTime::parse_from_str(&naive_str, "%Y-%m-%dT%H:%M:%S").is_ok());
    }
}
