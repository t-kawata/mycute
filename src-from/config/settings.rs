pub const VERSION: &str = concat!("v", env!("CARGO_PKG_VERSION"));
pub const DB_NAME: &str = "mycute";
pub const DEFAULT_SKEY: &str = "6JsfNZwZgc4VvDZyvhebvjVz/+J3IkKpvkb++HYc39Y/=";
pub const DEFAULT_CRYPTO_KEY: &str = "kS9yzX2!vB5*mN8@qW0&eP3_rY6*tU9!";

#[derive(Debug, Clone)]
pub struct DbInfo {
    pub host: String,
    pub port: String,
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone)]
pub struct Env {
    pub name: String,
    pub empty: bool,
    pub rw_db: DbInfo,
    pub ro_dbs: Vec<DbInfo>,
}

impl Default for Env {
    fn default() -> Self {
        Self {
            name: String::new(),
            empty: true,
            rw_db: DbInfo {
                host: String::new(),
                port: String::new(),
                username: String::new(),
                password: String::new(),
            },
            ro_dbs: Vec::new(),
        }
    }
}

fn create_local_env() -> Env {
    let db = DbInfo {
        host: "127.0.0.1".to_string(),
        port: "3306".to_string(),
        username: "asterisk".to_string(),
        password: "yu51043chie3".to_string(),
    };
    Env {
        name: "local".to_string(),
        empty: false,
        rw_db: db.clone(),
        ro_dbs: vec![db],
    }
}

fn create_dev_env() -> Env {
    let db = DbInfo {
        host: "127.0.0.1".to_string(),
        port: "3306".to_string(),
        username: "asterisk".to_string(),
        password: "yu51043chie3".to_string(),
    };
    Env {
        name: "dev".to_string(),
        empty: false,
        rw_db: db.clone(),
        ro_dbs: vec![db],
    }
}

fn create_stg_env() -> Env {
    let db = DbInfo {
        host: "127.0.0.1".to_string(),
        port: "3306".to_string(),
        username: "asterisk".to_string(),
        password: "yu51043chie3".to_string(),
    };
    Env {
        name: "stg".to_string(),
        empty: false,
        rw_db: db.clone(),
        ro_dbs: vec![db],
    }
}

fn create_prod_env() -> Env {
    let db = DbInfo {
        host: "127.0.0.1".to_string(),
        port: "3306".to_string(),
        username: "asterisk".to_string(),
        password: "yu51043chie3".to_string(),
    };
    Env {
        name: "prod".to_string(),
        empty: false,
        rw_db: db.clone(),
        ro_dbs: vec![db],
    }
}

pub fn get_env(e: &str) -> Env {
    match e {
        "local" => create_local_env(),
        "dev" => create_dev_env(),
        "stg" => create_stg_env(),
        "prod" => create_prod_env(),
        _ => Env::default(),
    }
}
