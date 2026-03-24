use crate::constants::ERR_OWNER_MODE_REQUIRED;
use crate::constants::ST_FORBIDDEN;
use crate::mode::rt::client::secure_client::SecureClient;
use crate::mode::rt::rtreq::owner_req::{ActivateOwnerReq, AssignCaReq};
use crate::mode::rt::rtres::owner_res::OwnerStatusRes;
use crate::mode::rt::{rtbl::owner_bl, rtres::errs_res::ApiError};
use crate::mycute_settings::ConfigManager;
use crate::types::{EventKind, InternalEvent};
use crate::utils::time;
use axum::{Extension, Json};
use garde::Validate;
use std::sync::Arc;
use tokio::sync::broadcast;

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
    Extension(config_manager): Extension<Arc<ConfigManager>>,
    Extension(client): Extension<Arc<SecureClient>>,
    Json(req): Json<AssignCaReq>,
) -> Result<Json<String>, ApiError> {
    // 動的な Owner Mode Check
    let is_owner = config_manager.is_owner_active();
    if !is_owner {
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

// ============================================================
// Activate Owner Mode
// ============================================================
const ACTIVATE_OWNER_DESC: &str = r#"
### ⚫︎ 概要
- クライアントから提供されたパスフレーズを用いて動的にオーナーモードをアクティベートする。
### ⚫︎ 権限
- 特になし（ただし正しいパスフレーズが必要）

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `passphrase` | string | required | オーナーパスフレーズ |
"#;
#[utoipa::path(
    tag = TAG,
    post,
    path = "/owner/activate",
    summary = "オーナーモードを有効化する。",
    description = ACTIVATE_OWNER_DESC,
    request_body = ActivateOwnerReq,
    responses(
        (status = 200, description = "Success", body = String),
        (status = 401, description = "Unauthorized (Invalid Passphrase)", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn activate_owner(
    Extension(config_manager): Extension<Arc<ConfigManager>>,
    Extension(event_tx): Extension<Arc<broadcast::Sender<InternalEvent>>>,
    Json(req): Json<ActivateOwnerReq>,
) -> Result<Json<()>, ApiError> {
    req.validate().map_err(|e| ApiError::from_garde(e))?;
    owner_bl::activate_owner(config_manager, &req.passphrase).await?;

    // アクティベーション成功を全クライアントにブロードキャスト（WebSocket経由でフロントに中継される）
    let seq = time::now_ts_ms();
    let event = InternalEvent {
        seq,
        kind: EventKind::OwnerStatusChanged(true),
    };
    let _ = event_tx.send(event); // リスナーがいなくてもエラーは無視
    log::debug!("<Owner> Owner Mode activated. Event broadcasted.");

    Ok(Json(()))
}

// ============================================================
// Deactivate Owner Mode
// ============================================================
const DEACTIVATE_OWNER_DESC: &str = r#"
### ⚫︎ 概要
- オーナーモードを解除し、メモリ上の秘密鍵を破棄する。
### ⚫︎ 権限
- オーナーモード中のみ実行可能。
"#;
#[utoipa::path(
    tag = TAG,
    post,
    path = "/owner/deactivate",
    summary = "オーナーモードを無効化する。",
    description = DEACTIVATE_OWNER_DESC,
    responses(
        (status = 200, description = "Success"),
        (status = 403, description = "Forbidden (Not Owner Mode)", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn deactivate_owner(
    Extension(config_manager): Extension<Arc<ConfigManager>>,
    Extension(event_tx): Extension<Arc<broadcast::Sender<InternalEvent>>>,
) -> Result<Json<()>, ApiError> {
    // 自身がオーナーモードでなければ解除できない
    if !config_manager.is_owner_active() {
        return Err(ApiError::new_system(
            ST_FORBIDDEN,
            ERR_OWNER_MODE_REQUIRED,
            "Owner mode is not active.",
        ));
    }

    owner_bl::deactivate_owner(config_manager).await?;

    // 解除成功を全クライアントにブロードキャスト
    let seq = time::now_ts_ms();
    let event = InternalEvent {
        seq,
        kind: EventKind::OwnerStatusChanged(false),
    };
    let _ = event_tx.send(event);
    log::debug!("<Owner> Owner Mode deactivated. Event broadcasted.");

    Ok(Json(()))
}

// ============================================================
// Get Owner Status
// ============================================================
const GET_OWNER_STATUS_DESC: &str = r#"
### ⚫︎ 概要
- 現在のバックエンドがオーナーモードとして動作しているかどうかのステータスを取得する。
### ⚫︎ 権限
- 特になし

### ⚫︎ Response
| KEY | TYPE | DESCRIPTION |
| --- | --- | --- |
| `is_active` | boolean | オーナーモードがアクティブかどうか |
"#;
#[utoipa::path(
    tag = TAG,
    get,
    path = "/owner/status",
    summary = "オーナーモードのステータスを取得する。",
    description = GET_OWNER_STATUS_DESC,
    responses(
        (status = 200, description = "Success", body = OwnerStatusRes),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn get_owner_status(
    Extension(config_manager): Extension<Arc<ConfigManager>>,
) -> Result<Json<OwnerStatusRes>, ApiError> {
    let is_active = config_manager.is_owner_active();
    Ok(Json(OwnerStatusRes { is_active }))
}
