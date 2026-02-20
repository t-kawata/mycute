use crate::{
    constants::CLEANER_TASK_INTERVAL_SEC, mode::rt::rtbl::blacklists_bl, utils::db::DbPools,
};
use std::sync::Arc;
use tokio::time::{sleep, Duration};

/// 定期的なクリーナータスクを開始する
/// バックグラウンドで実行され、定期的（デフォルト1時間）に期限切れデータの削除などを行う。
pub fn start_cleaner_task(db: Arc<DbPools>) {
    tokio::spawn(async move {
        log::info!(
            "<Cleaner> Background task started. Interval: {}s",
            CLEANER_TASK_INTERVAL_SEC
        );

        loop {
            // 指定間隔待機
            sleep(Duration::from_secs(CLEANER_TASK_INTERVAL_SEC)).await;

            log::info!("<Cleaner> Running scheduled tasks...");

            // Task 1: 期限切れブラックリストの削除
            if let Err(e) = blacklists_bl::delete_expired_blacklists(&db).await {
                log::error!("<Cleaner> Failed to clean expired blacklists: {}", e);
            }

            // Future Task: ここに追加
            // ...
        }
    });
}
