use chrono::{DateTime, NaiveDateTime, Utc};
use std::time::{SystemTime, UNIX_EPOCH};

/// A. [UTC] DB保存、ファイル名、UI表示、ローカルログ用
/// 現在時刻を UTC の NaiveDateTime として取得します。
/// システム全体でこの関数を標準の時刻取得として使用します。
pub fn now() -> NaiveDateTime {
    Utc::now().naive_utc()
}

/// B. [UTC] JWT、SSL、外部プロトコル用
/// 現在時刻を UTC の DateTime として取得します。
pub fn now_utc() -> DateTime<Utc> {
    Utc::now()
}

/// C. [TS] システムイベント、Unixタイムスタンプ用 (秒)
pub fn now_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

/// C. [TS] システムイベント、Unixタイムスタンプ用 (ミリ秒)
pub fn now_ts_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

// For DB/UI (NaiveDateTime), we output as is.
pub fn datetime_to_str(dt: &NaiveDateTime) -> String {
    dt.format("%Y-%m-%dT%H:%M:%S").to_string()
}

pub fn naive_to_str(dt: &NaiveDateTime) -> String {
    dt.format("%Y-%m-%dT%H:%M:%S").to_string()
}

/// [Naive -> UTC TS]
/// NaiveDateTime (UTC) を Unix Timestamp (u64) に変換します。
pub fn to_ts(dt: NaiveDateTime) -> u64 {
    dt.and_utc().timestamp() as u64
}

/// [Naive -> UTC TS (ms)]
/// NaiveDateTime (UTC) を Unix Timestamp (ミリ秒, u64) に変換します。
/// CA トークンの有効期限など、ミリ秒単位で統一されたタイムスタンプ比較に使用します。
pub fn to_ts_ms(dt: NaiveDateTime) -> u64 {
    dt.and_utc().timestamp_millis() as u64
}

/// [UTC TS -> Naive]
/// Unix Timestamp (u64) を NaiveDateTime (UTC) に変換します。
pub fn from_ts(ts: u64) -> NaiveDateTime {
    use chrono::{TimeZone, LocalResult};
    match Utc.timestamp_opt(ts as i64, 0) {
        LocalResult::Single(dt) => dt.naive_utc(),
        _ => {
            log::warn!("Invalid timestamp received: {}. Falling back to EPOCH.", ts);
            // 0,0 は常に有効なため、safe
            chrono::DateTime::from_timestamp(0, 0).map(|dt| dt.naive_utc()).unwrap_or_else(|| {
                // 万が一失敗した場合は真の最小値を返す (panicを避ける)
                NaiveDateTime::MIN
            })
        }
    }
}

/// [UTC TS (ms) -> Naive]
/// Unix Timestamp (ms) を NaiveDateTime (UTC) に変換します。
pub fn from_ts_ms(ts: i64) -> NaiveDateTime {
    use chrono::{TimeZone, LocalResult};
    match Utc.timestamp_millis_opt(ts) {
        LocalResult::Single(dt) => dt.naive_utc(),
        _ => {
            log::warn!("Invalid millis timestamp received: {}. Falling back to EPOCH.", ts);
            chrono::DateTime::from_timestamp(0, 0).map(|dt| dt.naive_utc()).unwrap_or_else(|| {
                NaiveDateTime::MIN
            })
        }
    }
}
/// [Sleep] 指定したミリ秒数だけ現在のスレッドをスリープさせます。
pub fn sleep_ms(ms: u64) {
    std::thread::sleep(std::time::Duration::from_millis(ms));
}
