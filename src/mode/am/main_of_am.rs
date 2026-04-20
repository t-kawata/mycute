use crate::config::settings::Env;
use crate::constants::LOCK_FILE_SERVER;
use crate::migration::{Migrator, MigratorTrait};
use crate::mycute_settings::ConfigManager;
use crate::utils::db::get_db;
use crate::utils::init::{CommonFlgs, HasCommonFlgs};
use crate::utils::singleton;
use anyhow::Context;
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
    // 1. [Bootstrap] まずは設定の読み込みのみを行う (DB情報取得のため)
    let temp_config_mgr = ConfigManager::new_bootstrap(flgs.common.home.clone())?;
    let env = Env::from_settings(&temp_config_mgr.settings.read().storage);

    // ==============================
    // フラグの出力
    // ==============================
    let flgs_json = serde_json::to_string(&flgs).context("Failed to serialize flgs to json.")?;
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
        if let Err(e) = singleton::acquire_lock(LOCK_FILE_SERVER) {
            log::error!("Singleton lock failed: {}", e);
            anyhow::bail!("{}", e);
        }

        // ==============================
        // DB接続
        // ==============================
        let db_pools = get_db(&env, &flgs.common.log_level)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to create DB: {}", e))?;
        log::debug!("DB created successfully.");

        let rw_conn = db_pools.get_rw().map_err(|e| anyhow::anyhow!("{}", e))?.clone();
        
        // ==============================
        // [Live] ConfigManager を DB 付きで実初期化
        // ==============================
        // temp_config_mgr から settings を引き継いで「完成版」を作成
        let config_mgr = ConfigManager::new_live(db_pools.clone(), flgs.common.home.clone())?;
        config_mgr.ensure_initialized_with_db().await?;
        log::info!("ConfigManager initialized with DB (Live).");

        // ==============================
        // AutoMigration の実行
        // ==============================
        if flgs.fresh {
            log::info!("Running AutoMigration (Fresh: NUCLEAR)...");
            Migrator::fresh(&rw_conn)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to fresh migrations: {}", e))?;
            log::info!("AutoMigration (Fresh) completed successfully.");
        } else if flgs.refresh {
            log::info!("Running AutoMigration (Refresh: DESTRUCTIVE)...");
            Migrator::refresh(&rw_conn)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to refresh migrations: {}", e))?;
            log::info!("AutoMigration (Refresh) completed successfully.");
        } else {
            log::info!("Running AutoMigration (Up: Safe)...");
            Migrator::up(&rw_conn, None)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to run migrations: {}", e))?;
            log::info!("AutoMigration (Up) completed successfully.");
        }
        Ok::<(), anyhow::Error>(())
    })?;

    Ok(())
}
