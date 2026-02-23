use crate::config::settings::Env;
use crate::constants::LOCK_FILE_SERVER;
use crate::migration::{Migrator, MigratorTrait};
use crate::stt_config::ConfigManager;
use crate::utils::db::get_db;
use crate::utils::init::{CommonFlgs, HasCommonFlgs};
use crate::utils::singleton;
use clap::Parser;
use serde::Serialize;

#[derive(Debug, Parser, Serialize)]
#[command(override_usage = "mycute am [OPTIONS]")]
pub struct AMFlgs {
    #[command(flatten)]
    pub common: CommonFlgs,

    /// Refresh migrations (drop all tables and re-run)
    #[arg(long, default_value_t = false)]
    pub refresh: bool,

    /// Fresh migrations (drop all tables WITHOUT rollback and re-run)
    #[arg(long, default_value_t = false)]
    pub fresh: bool,
}

impl HasCommonFlgs for AMFlgs {
    fn common_flgs(&self) -> &CommonFlgs {
        &self.common
    }
}

pub fn main_of_am(flgs: AMFlgs) -> anyhow::Result<()> {
    let config_mgr = ConfigManager::new();

    let env = Env::from_settings(&config_mgr.settings.read().storage);

    // ==============================
    // フラグの出力
    // ==============================
    let flgs_json = serde_json::to_string(&flgs).expect("Failed to serialize flgs to json.");
    log::debug!("AM-FLAGS: {}", flgs_json);

    // ==============================
    // 非同期ランタイムの作成と実行
    // ==============================
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| anyhow::anyhow!("Failed to create tokio runtime: {}", e))?;

    rt.block_on(async {
        // ==============================
        // 多重起動防止 (Singleton Lock)
        // ==============================
        // AM モードでもDBを操作するため、サーバーやAMの同時実行を防止する。
        if let Err(e) = singleton::acquire_lock(LOCK_FILE_SERVER) {
            log::error!("Singleton lock failed: {}", e);
            anyhow::bail!("{}", e);
        }

        // ==============================
        // DB接続
        // ==============================
        let db = get_db(&env, &flgs.common.log_level)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create DB: {}", e))?;
        log::debug!("DB created successfully.");

        // ==============================
        // AutoMigration の実行
        // ==============================
        let rw_conn = db
            .get_rw()
            .map_err(|e| anyhow::anyhow!("Failed to get RW connection for migration: {}", e))?;
        if flgs.fresh {
            log::info!("Running AutoMigration (Fresh: NUCLEAR)...");
            Migrator::fresh(rw_conn)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to fresh migrations: {}", e))?;
            log::info!("AutoMigration (Fresh) completed successfully.");
        } else if flgs.refresh {
            log::info!("Running AutoMigration (Refresh: DESTRUCTIVE)...");
            Migrator::refresh(rw_conn)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to refresh migrations: {}", e))?;
            log::info!("AutoMigration (Refresh) completed successfully.");
        } else {
            log::info!("Running AutoMigration (Up: Safe)...");
            log::info!("Note: Startup migration is also enabled in RT/CL modes.");
            Migrator::up(rw_conn, None)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to run migrations: {}", e))?;
            log::info!("AutoMigration (Up) completed successfully.");
        }
        Ok::<(), anyhow::Error>(())
    })?;

    Ok(())
}
