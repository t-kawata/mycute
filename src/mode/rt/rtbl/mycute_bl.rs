use crate::constants::MYCUTE_VERSION;
use crate::mode::rt::rtbl::identities_bl;
use crate::mode::rt::rtres::mycute_res::{MyCuteHomeDirRes, MyCuteVersionRes, VerifyCaTokenRes};
use crate::utils::time;
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

pub async fn verify_ca_token(ca_token_hex: &str) -> VerifyCaTokenRes {
    let now = time::now_ts_ms() as u64;
    let res = identities_bl::verify_ca_token(ca_token_hex, now);

    if let Some(pubkey) = res {
        // CA任命証から有効期限と権限を再取得
        let (expire_at, permissions) = identities_bl::parse_ca_token_raw(ca_token_hex)
            .map(|(p, _)| (Some(p.expire_at), Some(p.permissions)))
            .unwrap_or((None, None));

        VerifyCaTokenRes {
            success: true,
            message: "CA Cert is valid and signed by Owner.".to_string(),
            ca_pubkey: Some(pubkey),
            expire_at,
            permissions,
        }
    } else {
        VerifyCaTokenRes {
            success: false,
            message: "Invalid CA Cert, expired, or signature verification failed.".to_string(),
            ca_pubkey: None,
            expire_at: None,
            permissions: None,
        }
    }
}
