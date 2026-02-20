use crate::constants::ERR_OWNER_MODE_REQUIRED;
use crate::constants::ST_FORBIDDEN;
use crate::mode::rt::client::secure_client::SecureClient;
use crate::mode::rt::rtreq::owner_req::AssignCaReq;
use crate::mode::rt::{req_map::IsOwner, rtbl::owner_bl, rtres::errs_res::ApiError};
use crate::stt_config::ConfigManager;
use axum::{Extension, Json};
use garde::Validate;
use std::sync::Arc;

const TAG: &str = "v1 Owner";

// ============================================================
// Assign CA (Owner -> CA Candidate)
// ============================================================
const ASSIGN_CA_DESC: &str = r#"
### ⚫︎ 概要
- オーナーが指定された CA 候補（一般ノード）に対して「CA 任命証（CA Token）」を発行する。
- 1. 指定されたターゲット(`target_url`)の `/v1/identities/pubkey` を叩いて公開鍵を取得する。
- 2. 取得した公開鍵と有効期限に対して、オーナー秘密鍵で署名を行う。
- 3. 生成された「CA Token」を含む任命証をレスポンスとして返す。
- **注意**: このレスポンスは「手渡し」で CA 候補者に渡されることを前提としている。

### ⚫︎ 権限
- **Owner Only**: オーナーモードで起動しているノードのみ実行可能。
"#;
#[utoipa::path(
    tag = TAG,
    post,
    path = "/owner/assignca",
    summary = "CA候補を任命し、CAトークンを発行する。",
    description = ASSIGN_CA_DESC,
    request_body = AssignCaReq,
    responses(
        (status = 200, description = "Success", body = String),
        (status = 403, description = "Forbidden (Not Owner Mode)", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn assign_ca(
    Extension(is_owner): Extension<IsOwner>,
    Extension(config_manager): Extension<Arc<ConfigManager>>,
    Extension(client): Extension<Arc<SecureClient>>,
    Json(req): Json<AssignCaReq>,
) -> Result<Json<String>, ApiError> {
    // Owner Mode Check
    if !is_owner.0 {
        return Err(ApiError::new_system(
            ST_FORBIDDEN,
            ERR_OWNER_MODE_REQUIRED,
            "This endpoint requires Owner Mode activation.",
        ));
    }
    req.validate().map_err(|e| ApiError::from_garde(e))?;
    let ca_token_info =
        owner_bl::assign_ca(config_manager, &client, req.target_url, req.expire_hours).await?;
    Ok(Json(ca_token_info))
}
