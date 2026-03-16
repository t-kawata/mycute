use crate::config::settings::{DbInfo, Env};
use crate::constants::{DB_NAME, DOMAIN_LOCALHOST, IP_LOCALHOST, SQLITE_DEFAULT_FILENAME};
use crate::mycute_settings::DbDriver;
use crate::utils::init::LogLevel;
use anyhow::Context;
use chrono::NaiveDateTime;
use futures::future::join_all;
use sea_orm::{
    ActiveValue::{self, Set},
    ConnectOptions, ConnectionTrait, Database, DatabaseBackend, DatabaseConnection, Statement,
};
#[cfg(unix)]
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

// ============================================================
// データベース接続プール設定
// ============================================================

/// 最大接続数：プール内で保持できるコネクションの最大数
const MAX_CONNECTIONS: u32 = 100;
/// 最小接続数：プール内で常に維持される最小のコネクション数（予熱される接続数）
const MIN_CONNECTIONS: u32 = 10;
/// 接続タイムアウト：データベースへの新規接続試行のタイムアウト時間
const CONNECT_TIMEOUT_SECS: u64 = 5;
/// 取得タイムアウト：プールからコネクションを取得する際の待ち時間の最大値
const ACQUIRE_TIMEOUT_SECS: u64 = 5;
/// アイドルタイムアウト：未使用のコネクションが破棄されるまでの時間（30分）
const IDLE_TIMEOUT_SECS: u64 = 1800;
/// 最大生存時間：接続が確立されてから強制的に破棄・再接続されるまでの時間（6時間）
const MAX_LIFETIME_SECS: u64 = 21600;

pub struct DbPools {
    pub rw: DatabaseConnection,
    pub ro: Vec<DatabaseConnection>,
    pub ro_index: AtomicUsize,
}

impl DbPools {
    /// Read-Writeコネクションを取得する
    pub fn get_rw(&self) -> anyhow::Result<&DatabaseConnection> {
        Ok(&self.rw)
    }

    /// Read-Onlyコネクションをラウンドロビンで取得する
    pub fn get_ro(&self) -> anyhow::Result<&DatabaseConnection> {
        if self.ro.is_empty() {
            log::warn!("No Read-Only connections available, falling back to Read-Write.");
            return Ok(&self.rw);
        }
        let index = self.ro_index.fetch_add(1, Ordering::Relaxed);
        Ok(&self.ro[index % self.ro.len()])
    }
}

impl Clone for DbPools {
    fn clone(&self) -> Self {
        Self {
            rw: self.rw.clone(),
            ro: self.ro.clone(),
            ro_index: AtomicUsize::new(0),
        }
    }
}

pub async fn get_db(env: &Env, log_level: &LogLevel) -> anyhow::Result<DbPools> {
    // 1. RW接続（必須）
    let rw_url = format_url(&env.rw_db, &env.db_dir);
    let rw = connect(rw_url, log_level)
        .await
        .context("Failed to connect to RW database")?;

    // 2. RO接続（個別失敗を許容）
    let mut ro_futures = Vec::new();
    for ro_info in &env.ro_dbs {
        ro_futures.push(connect(format_url(ro_info, &env.db_dir), log_level));
    }

    let mut ro = Vec::new();
    if !ro_futures.is_empty() {
        let results = join_all(ro_futures).await;
        for (i, res) in results.into_iter().enumerate() {
            match res {
                Ok(conn) => ro.push(conn),
                Err(e) => {
                    let host = &env.ro_dbs[i].host;
                    log::error!("Failed to connect to RO database ({}): {}", host, e);
                }
            }
        }
    }

    // 3. ROが全滅した場合の代替
    if ro.is_empty() {
        if !env.ro_dbs.is_empty() {
            log::warn!(
                "All configured RO databases failed. Falling back to RW for read operations."
            );
        }
        ro.push(rw.clone());
    }

    Ok(DbPools {
        rw,
        ro,
        ro_index: AtomicUsize::new(0),
    })
}

fn format_url(info: &DbInfo, db_dir: &str) -> String {
    match info.driver {
        DbDriver::Postgres => format!(
            "postgres://{}:{}@{}:{}/{}",
            info.username, info.password, info.host, info.port, DB_NAME
        ),
        DbDriver::Sqlite => {
            // SQLiteの場合、hostをファイル名として扱う。
            // 以前のMySQL設定（127.0.0.1等）が残っている場合や空の場合は、デフォルトのファイル名を使用する。
            let filename = if info.host.is_empty()
                || info.host == IP_LOCALHOST
                || info.host == DOMAIN_LOCALHOST
            {
                SQLITE_DEFAULT_FILENAME
            } else {
                &info.host
            };
            let path = Path::new(db_dir).join(filename);
            format!("sqlite://{}?mode=rwc", path.to_string_lossy())
        }
        DbDriver::Mysql => format!(
            "mysql://{}:{}@{}:{}/{}?timezone=UTC",
            info.username, info.password, info.host, info.port, DB_NAME
        ),
    }
}

#[cfg(unix)]
fn adjust_perm(path: &Path) {
    if path.exists() {
        if let Ok(metadata) = path.metadata() {
            let mut perms = metadata.permissions();
            if perms.mode() & 0o666 != 0o666 {
                perms.set_mode(0o666); // -rw-rw-rw-
                if let Err(e) = fs::set_permissions(path, perms) {
                    log::warn!("Failed to set permissions for {}: {}", path.display(), e);
                } else {
                    log::debug!("Adjusted permissions for {}", path.display());
                }
            }
        }
    }
}

async fn connect(url: String, log_level: &LogLevel) -> Result<DatabaseConnection, sea_orm::DbErr> {
    let mut opt = ConnectOptions::new(&url);
    opt.max_connections(MAX_CONNECTIONS)
        .min_connections(MIN_CONNECTIONS)
        .connect_timeout(Duration::from_secs(CONNECT_TIMEOUT_SECS))
        .acquire_timeout(Duration::from_secs(ACQUIRE_TIMEOUT_SECS))
        .idle_timeout(Duration::from_secs(IDLE_TIMEOUT_SECS))
        .max_lifetime(Duration::from_secs(MAX_LIFETIME_SECS))
        .test_before_acquire(true) // 取得前に PING で生存確認を行う（論理的な Keep-Alive）
        .sqlx_logging(false);

    if log_level != &LogLevel::Debug && log_level != &LogLevel::Trace {
        // Debug 未満の場合は、念のため sqlx のログレベルを Off にしておきます
        opt.sqlx_logging_level(log::LevelFilter::Off);
    }

    let db = Database::connect(opt).await?;

    // SQLiteの場合、root権限で作成されたファイルに一般ユーザーがアクセスできるよう、パーミッションを調整
    #[cfg(unix)]
    {
        if url.starts_with("sqlite://") {
            let path_str = url
                .trim_start_matches("sqlite://")
                .split('?')
                .next()
                .unwrap_or("");
            if !path_str.is_empty() {
                let db_path = Path::new(path_str);
                // メインのDBファイル
                adjust_perm(db_path);
                // WALモード等の関連ファイルも存在する可能性があるため、パターンで調整
                let shm = format!("{}-shm", path_str);
                adjust_perm(Path::new(&shm));
                let wal = format!("{}-wal", path_str);
                adjust_perm(Path::new(&wal));
            }
        }
    }

    // [初期化 SQL]
    let backend = db.get_database_backend();
    match backend {
        DatabaseBackend::MySql => {
            db.execute(Statement::from_string(backend, "SET time_zone = '+00:00';"))
                .await
                .ok();
        }
        DatabaseBackend::Postgres => {
            db.execute(Statement::from_string(backend, "SET TIME ZONE 'UTC';"))
                .await
                .ok();
        }
        DatabaseBackend::Sqlite => {
            // WALモード有効化 (パフォーマンスと並行性向上)
            db.execute(Statement::from_string(
                backend,
                "PRAGMA journal_mode = WAL;",
            ))
            .await
            .ok();
            // 外部キー制約有効化
            db.execute(Statement::from_string(backend, "PRAGMA foreign_keys = ON;"))
                .await
                .ok();
        }
    }

    Ok(db)
}

/// YYYY-MM-DDThh:mm:ss 形式の文字列（UTC想定）を
/// ActiveValue<NaiveDateTime> に変換する
pub fn str_to_datetime(date_str: &str) -> anyhow::Result<ActiveValue<NaiveDateTime>> {
    let format = "%Y-%m-%dT%H:%M:%S";
    let naive = NaiveDateTime::parse_from_str(date_str, format)
        .map_err(|e| anyhow::anyhow!("Failed to parse date string: {}", e))?;
    Ok(Set(naive))
}

/// NaiveDateTime を YYYY-MM-DDThh:mm:ss 形式の文字列に変換する
pub fn datetime_to_str(dt: NaiveDateTime) -> String {
    dt.format("%Y-%m-%dT%H:%M:%S").to_string()
}
