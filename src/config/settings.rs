use crate::mycute_settings::DbDriver;

pub const VERSION: &str = crate::constants::MYCUTE_VERSION;
pub const DB_NAME: &str = crate::constants::DB_NAME;
pub const DEFAULT_SKEY: &str = crate::constants::DEFAULT_SKEY;
pub const DEFAULT_CRYPTO_KEY: &str = crate::constants::DEFAULT_CRYPTO_KEY;

#[derive(Debug, Clone)]
pub struct DbInfo {
    pub driver: DbDriver,
    pub host: String, // SQLiteの場合はファイル名
    pub port: String,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone)]
pub struct Env {
    pub name: String,
    pub empty: bool,
    pub db_dir: String, // SQLite用ディレクトリ
    pub rw_db: DbInfo,
    pub ro_dbs: Vec<DbInfo>,
}

impl Default for Env {
    fn default() -> Self {
        Self {
            name: String::new(),
            empty: true,
            db_dir: String::new(),
            rw_db: DbInfo {
                driver: DbDriver::Sqlite,
                host: crate::constants::SQLITE_DEFAULT_FILENAME.to_string(), // Default file name
                port: String::new(),
                username: String::new(),
                password: String::new(),
            },
            ro_dbs: Vec::new(),
        }
    }
}

impl Env {
    pub fn from_settings(settings: &crate::mycute_settings::StorageSettings) -> Self {
        let rw_db = DbInfo {
            driver: settings.rw_db.driver.clone(),
            host: settings.rw_db.host.clone(),
            port: settings.rw_db.port.clone(),
            username: settings.rw_db.user.clone(),
            password: settings.rw_db.pass.clone(),
        };
        let ro_dbs = settings
            .ro_dbs
            .iter()
            .map(|ro| DbInfo {
                driver: ro.driver.clone(),
                host: ro.host.clone(),
                port: ro.port.clone(),
                username: ro.user.clone(),
                password: ro.pass.clone(),
            })
            .collect();

        Env {
            name: "unified".to_string(),
            empty: false,
            db_dir: settings.db_dir_path.clone(),
            rw_db,
            ro_dbs,
        }
    }
}
