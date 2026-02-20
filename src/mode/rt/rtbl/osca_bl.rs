use crate::constants::{PATH_OSCA_CERT_DOWNLOAD, ST_INTERNAL_SERVER_ERROR};
use crate::mode::rt::{
    rterr::rterr,
    rtres::{errs_res::ApiError, osca_res::GetOscaUrlRes},
};
use local_ip_address::local_ip;

pub async fn get_osca_url(sw_port: u16) -> Result<GetOscaUrlRes, ApiError> {
    // ローカルIPの取得
    let my_local_ip = local_ip().map_err(|e| {
        log::error!("Failed to get local ip: {}", e);
        ApiError::new_system(
            ST_INTERNAL_SERVER_ERROR,
            rterr::ERR_UNEXPECTED,
            "Failed to get local ip",
        )
    })?;

    // URLの構築
    // IPv6の場合はブラケットで囲む必要があるが、local_ip()はIpAddrを返す。
    // ip.to_string() で適切にフォーマットされるか確認が必要だが、通常はIPv4が優先される環境が多い。
    // IPv6対応を厳密にするなら my_local_ip.is_ipv6() チェックを入れる。
    let ip_str = if my_local_ip.is_ipv6() {
        format!("[{}]", my_local_ip)
    } else {
        my_local_ip.to_string()
    };

    let osca_url = format!("http://{}:{}{}", ip_str, sw_port, PATH_OSCA_CERT_DOWNLOAD);

    Ok(GetOscaUrlRes { osca_url })
}
