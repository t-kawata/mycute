use crate::constants::MYCUTE_VERSION;
use crate::mode::rt::rtres::mycute_res::{MyCuteHomeDirRes, MyCuteVersionRes};
use crate::utils::my_path::get_mycute_home;

pub async fn get_version() -> MyCuteVersionRes {
    MyCuteVersionRes {
        version: MYCUTE_VERSION.to_string(),
    }
}

pub async fn get_home_dir() -> MyCuteHomeDirRes {
    let home = get_mycute_home();
    MyCuteHomeDirRes {
        home_dir: home.to_string_lossy().to_string(),
    }
}
