use std::sync::Arc;

use crate::constants::ST_INTERNAL_SERVER_ERROR;
use crate::mode::rt::{rterr::rterr, rtres::errs_res::ApiError};
use crate::utils::db::DbPools;
use sea_orm::DatabaseConnection;

pub trait DbPoolsExt {
    fn get_rw_for_rt(&self) -> Result<&DatabaseConnection, ApiError>;
    fn get_ro_for_rt(&self) -> Result<&DatabaseConnection, ApiError>;
}

impl DbPoolsExt for DbPools {
    // Read & Write 用のマスターDBの接続を取得する
    fn get_rw_for_rt(&self) -> Result<&DatabaseConnection, ApiError> {
        self.get_rw().map_err(|e| {
            ApiError::new_system(
                ST_INTERNAL_SERVER_ERROR,
                rterr::ERR_DATABASE,
                format!("Failed to get RW connection: {}", e),
            )
        })
    }
    // Read 用のリードレプリカDBの接続を取得する
    fn get_ro_for_rt(&self) -> Result<&DatabaseConnection, ApiError> {
        self.get_ro().map_err(|e| {
            ApiError::new_system(
                ST_INTERNAL_SERVER_ERROR,
                rterr::ERR_DATABASE,
                format!("Failed to get RO connection: {}", e),
            )
        })
    }
}

impl DbPoolsExt for Arc<DbPools> {
    // Read & Write 用のマスターDBの接続を取得する
    fn get_rw_for_rt(&self) -> Result<&DatabaseConnection, ApiError> {
        self.as_ref().get_rw_for_rt()
    }
    // Read 用のリードレプリカDBの接続を取得する
    fn get_ro_for_rt(&self) -> Result<&DatabaseConnection, ApiError> {
        self.as_ref().get_ro_for_rt()
    }
}
