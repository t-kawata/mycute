use crate::config::settings::{DEFAULT_SKEY, DEFAULT_CRYPTO_KEY};
use crate::utils::db::get_db;
use crate::utils::env::get_env_or;
use crate::utils::init::{CommonFlgs, HasCommonFlgs, init};
use crate::utils::s3client;
use crate::mode::rt::req_map;

use clap::Parser;
use serde::Serialize;
use std::iter::{Chain, Cloned, Once};
use std::slice::Iter;
use std::sync::Arc;

#[derive(Debug, Parser, Serialize)]
#[command(override_usage = "mycute rt [OPTIONS]")]
pub struct RTFlgs {
    #[command(flatten)]
    pub common: CommonFlgs,

    #[arg(short = 'd', long = "dotenv", default_value_t = String::from(".env"), help = "Path to .env file")]
    pub dotenv: String,
}

impl HasCommonFlgs for RTFlgs {
    fn common_flgs(&self) -> &CommonFlgs {
        &self.common
    }
}

pub async fn main_of_rt(args: Chain<Once<String>, Cloned<Iter<'_, String>>>) {
    // ==============================
    // 初期化
    // ==============================
    let (flgs, env) = init::<RTFlgs>(args).expect("Failed to init rt mode.");

    // ==============================
    // .envファイルの読み込み
    // ==============================
    dotenvy::from_path(&flgs.dotenv).expect(&format!("Failed to load .env from {}", flgs.dotenv));
    log::debug!("Loaded .env from: {}", flgs.dotenv);

    // ==============================
    // フラグの出力
    // ==============================
    let flgs_json = serde_json::to_string(&flgs).expect("Failed to serialize flgs to json.");
    log::debug!("RT-FLAGS: {}", flgs_json);

    // ==============================
    // 環境変数収集
    // ==============================
    let rt_port = get_env_or("RT_PORT", 8888);
    let cors_on_rt = get_env_or("CORS_ON_RT", false);
    let rt_skey = get_env_or("RT_SKEY", DEFAULT_SKEY.to_string());
    let rt_crypto_key = get_env_or("RT_CRYPTO_KEY", DEFAULT_CRYPTO_KEY.to_string());
    let s3_use_local = get_env_or("S3_USE_LOCAL", false);
    let s3_local_dir = get_env_or("S3_LOCAL_DIR", "dummy".to_string());
    let s3_down_dir = get_env_or("S3_DOWN_DIR", "dummy".to_string());
    let s3_access_key = get_env_or("S3_ACCESS_KEY", "dummy".to_string());
    let s3_secret_access_key = get_env_or("S3_SECRET_ACCESS_KEY", "dummy".to_string());
    let s3_region = get_env_or("S3_REGION", "dummy".to_string());
    let s3_bucket = get_env_or("S3_BUCKET", "dummy".to_string());
    let s3_min_free_disk = get_env_or("S3_MIN_FREE_DISK", 0);
    log::debug!("RT_PORT: {}", rt_port);
    log::debug!("CORS_ON_RT: {}", cors_on_rt);
    log::debug!("RT_SKEY: {}", rt_skey);
    log::debug!("RT_CRYPTO_KEY: {}", rt_crypto_key);
    log::debug!("S3_USE_LOCAL: {}", s3_use_local);
    log::debug!("S3_LOCAL_DIR: {}", s3_local_dir);
    log::debug!("S3_DOWN_DIR: {}", s3_down_dir);
    log::debug!("S3_ACCESS_KEY: {}", s3_access_key);
    log::debug!("S3_SECRET_ACCESS_KEY: {}", s3_secret_access_key);
    log::debug!("S3_REGION: {}", s3_region);
    log::debug!("S3_BUCKET: {}", s3_bucket);
    log::debug!("S3_MIN_FREE_DISK: {}", s3_min_free_disk);

    // ==============================
    // s3clientの初期化
    // ==============================
    let s3c = s3client::S3Client::new(&s3_access_key, &s3_secret_access_key, &s3_region, &s3_bucket, &s3_local_dir, &s3_down_dir, s3_use_local).await;
    let s3_client = match s3c {
        Ok(s) => { log::debug!("S3Client created successfully."); Arc::new(s) }
        Err(e) => { eprintln!("Failed to create s3client: {}", e); std::process::exit(1); }
    };

    // ==============================
    // DB接続
    // ==============================
    let db_result = get_db(&env, &flgs.common.log_level).await;
    let db = match db_result {
        Ok(db) => { log::debug!("DB created successfully."); db }
        Err(e) => { eprintln!("Failed to create DB: {}", e); std::process::exit(1); }
    };

    // ==============================
    // CuberServiceの初期化 (Phase 4)
    // ==============================
    let cuber_config = crate::cuber::config::CuberConfig::from_env();
    let cuber_service = match crate::cuber::service::CuberService::new(cuber_config, Arc::clone(&s3_client)).await {
        Ok(s) => Arc::new(s),
        Err(e) => { eprintln!("Failed to initialize CuberService: {:?}", e); std::process::exit(1); }
    };

    // ==============================
    // Axum リクエストマッピングと起動
    // ==============================
    let router = req_map::map_request(cors_on_rt, db, &rt_skey, &rt_crypto_key, cuber_service);
    log::debug!("Starting RT server on port {}...", rt_port);
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{rt_port}")).await.expect("Failed to bind listener.");
    axum::serve(listener, router).await.expect("Failed to serve.");
}
