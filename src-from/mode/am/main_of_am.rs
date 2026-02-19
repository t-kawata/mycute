use crate::utils::db::get_db;
use crate::utils::init::{CommonFlgs, HasCommonFlgs, init};
use clap::Parser;
use serde::Serialize;
use std::iter::{Chain, Cloned, Once};
use std::slice::Iter;
use crate::migration::{Migrator, MigratorTrait};

#[derive(Debug, Parser, Serialize)]
#[command(override_usage = "mycute am [OPTIONS]")]
pub struct AMFlgs {
    #[command(flatten)]
    pub common: CommonFlgs,

    /// Refresh migrations (drop all tables and re-run)
    #[arg(long, default_value_t = false)]
    pub refresh: bool,
}

impl HasCommonFlgs for AMFlgs {
    fn common_flgs(&self) -> &CommonFlgs {
        &self.common
    }
}

pub async fn main_of_am(args: Chain<Once<String>, Cloned<Iter<'_, String>>>) {
    // ==============================
    // 初期化
    // ==============================
    let (flgs, env) = init::<AMFlgs>(args).expect("Failed to init am mode.");

    // ==============================
    // フラグの出力
    // ==============================
    let flgs_json = serde_json::to_string(&flgs).expect("Failed to serialize flgs to json.");
    log::debug!("AM-FLAGS: {}", flgs_json);

    // ==============================
    // DB接続
    // ==============================
    let db_result = get_db(&env, &flgs.common.log_level).await;
    let db = match db_result {
        Ok(db) => { log::debug!("DB created successfully."); db }
        Err(e) => { eprintln!("Failed to create DB: {}", e); std::process::exit(1); }
    };

    // ==============================
    // AutoMigration の実行
    // ==============================
    let rw_conn = db.get_rw().expect("Failed to get RW connection for migration.");
    if flgs.refresh {
        log::info!("Running AutoMigration (Refresh: DESTRUCTIVE)...");
        Migrator::refresh(rw_conn).await.expect("Failed to refresh migrations.");
        log::info!("AutoMigration (Refresh) completed successfully.");
    } else {
        log::info!("Running AutoMigration (Up: Safe)...");
        Migrator::up(rw_conn, None).await.expect("Failed to run migrations.");
        log::info!("AutoMigration (Up) completed successfully.");
    }
}
