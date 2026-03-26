use crate::constants::MYCUTE_VERSION;
use crate::mode::rt::rtres::mycute_res::{MyCuteHomeDirRes, MyCuteVersionRes};
use std::path::Path;

pub async fn get_version() -> MyCuteVersionRes {
    MyCuteVersionRes {
        version: MYCUTE_VERSION.to_string(),
    }
}

pub async fn get_home_dir(home: &Path) -> MyCuteHomeDirRes {
    MyCuteHomeDirRes {
        home_dir: home.to_string_lossy().to_string(),
    }
}
