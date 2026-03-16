use crate::{
    constants::PERIODICAL_STORE_INTERVAL_SEC, mode::rt::rtbl::identities_bl,
    mode::rt::rtutils::db_for_rt::DbPoolsExt, mycute_settings::ConfigManager, utils::db::DbPools,
};
use std::sync::Arc;
use tokio::time::{sleep, Duration};

/// 定期的な情報保持タスクを開始する
/// 1時間ごとに実行され、CA選定結果のキャッシュ更新などを行う。
pub fn start_periodical_store_task(db: Arc<DbPools>, config_manager: Arc<ConfigManager>) {
    tokio::spawn(async move {
        log::info!(
            "<PeriodicStore> Background task started. Interval: {}s",
            PERIODICAL_STORE_INTERVAL_SEC
        );

        loop {
            log::info!("<PeriodicStore> Updating cached information...");

            // Task 1: 信頼できる CA URL の事前選定とキャッシュ
            let conn = match db.get_ro_for_rt() {
                Ok(c) => c,
                Err(e) => {
                    log::error!("<PeriodicStore> Failed to get DB connection: {}", e);
                    sleep(Duration::from_secs(10)).await;
                    continue;
                }
            };

            // 信頼できる CA の全リストを DB から取得してキャッシュする
            let reliable_ca_list =
                identities_bl::select_reliable_ca_url_from_db(conn, &config_manager).await;
            let list_len = reliable_ca_list.as_ref().map(|l| l.len()).unwrap_or(0);

            {
                let mut guard = config_manager.reliable_ca_cache.write();
                *guard = reliable_ca_list;
            }

            log::info!(
                "<PeriodicStore> Updated reliable CA cache with {} URLs.",
                list_len
            );

            // 次のインターバルまで待機
            sleep(Duration::from_secs(PERIODICAL_STORE_INTERVAL_SEC)).await;
        }
    });
}
