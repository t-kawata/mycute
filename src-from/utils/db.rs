use crate::config::settings::{Env, DbInfo, DB_NAME};
use crate::utils::init::LogLevel;
use sea_orm::{Database, DatabaseConnection, ConnectOptions, ActiveValue::{self, Set}};
use chrono::NaiveDateTime;
use std::time::Duration;
use std::sync::atomic::{AtomicUsize, Ordering};
use futures::future::join_all;
use anyhow::Context;

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
    ro_index: AtomicUsize,
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

pub async fn get_db(env: &Env, log_level: &LogLevel) -> anyhow::Result<DbPools> {
    // 1. RW接続（必須）
    let rw_url = format_url(&env.rw_db);
    let rw = connect(rw_url, log_level).await.context("Failed to connect to RW database")?;

    // 2. RO接続（個別失敗を許容）
    let mut ro_futures = Vec::new();
    for ro_info in &env.ro_dbs {
        ro_futures.push(connect(format_url(ro_info), log_level));
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
            log::warn!("All configured RO databases failed. Falling back to RW for read operations.");
        }
        ro.push(rw.clone());
    }

    Ok(DbPools {
        rw,
        ro,
        ro_index: AtomicUsize::new(0),
    })
}

fn format_url(info: &DbInfo) -> String {
    format!(
        "mysql://{}:{}@{}:{}/{}?timezone=%2B09:00",
        info.username, info.password, info.host, info.port, DB_NAME
    )
}

async fn connect(url: String, log_level: &LogLevel) -> Result<DatabaseConnection, sea_orm::DbErr> {
    let mut opt = ConnectOptions::new(url);
    opt.max_connections(MAX_CONNECTIONS)
        .min_connections(MIN_CONNECTIONS)
        .connect_timeout(Duration::from_secs(CONNECT_TIMEOUT_SECS))
        .acquire_timeout(Duration::from_secs(ACQUIRE_TIMEOUT_SECS))
        .idle_timeout(Duration::from_secs(IDLE_TIMEOUT_SECS))
        .max_lifetime(Duration::from_secs(MAX_LIFETIME_SECS))
        .test_before_acquire(true) // 取得前に PING で生存確認を行う（論理的な Keep-Alive）
        .sqlx_logging(false);

    // sqlx_logging(false) を設定することで、SQLx 側のプレースホルダー (?) 付きログを無効化します。
    // SeaORM の debug-print feature による生SQL出力は、ロガーのレベルが Debug 以上の時に自動で行われます。
    opt.sqlx_logging(false);

    if log_level != &LogLevel::Debug && log_level != &LogLevel::Trace {
        // Debug 未満の場合は、念のため sqlx のログレベルを Off にしておきます
        opt.sqlx_logging_level(log::LevelFilter::Off);
    }

    Database::connect(opt).await
}

/// YYYY-MM-DDThh:mm:ss 形式の文字列（JST想定）を 
/// ActiveValue<NaiveDateTime> に変換する
pub fn str_to_datetime(date_str: &str) -> anyhow::Result<ActiveValue<NaiveDateTime>> {
    let format = "%Y-%m-%dT%H:%M:%S";
    let naive = NaiveDateTime::parse_from_str(date_str, format).map_err(|e| anyhow::anyhow!("Failed to parse date string: {}", e))?;
    Ok(Set(naive))
}

/// NaiveDateTime を YYYY-MM-DDThh:mm:ss 形式の文字列に変換する
pub fn datetime_to_str(dt: NaiveDateTime) -> String {
    dt.format("%Y-%m-%dT%H:%M:%S").to_string()
}
