use crate::{
    mode::rt::{
        rtbl::{ca_bl, license_bl},
        rtreq::ca_req::{GenLicenseReq, RegisterCaTokenReq},
        rtres::{
            ca_apps_res::CaStatusCaRes,
            ca_res::{GenLicenseRes, RegisterCaTokenRes, UnregisterCaTokenRes, CaStatusRes},
            errs_res::ApiError,
        },
    },
    utils::{jwt::{JwtRole, JwtUsr}, time},
    TAG_MACRO_P2P_OPTIONAL,
    types::{EventKind, InternalEvent},
};
use axum::{Extension, Json};
use garde::Validate;
use tokio::sync::broadcast;
use std::sync::Arc;

macro_rules! TAG_NAME {
    () => {
        "v1 CA"
    };
}
const TAG_P2P_OPTIONAL: &str = concat!(TAG_NAME!(), " ", TAG_MACRO_P2P_OPTIONAL!());

const CA_STATUS_DESC: &str = r#"
### CA 運用ステータス
認証局 (CA) のポリシーと現在の運用ステータスを透明化して提供します。
- **信頼ロジック**: L3 (15票), L2 (1票), 二次関数的加算 (Simple Quadratic Summing)。
- **ポリシー**: 分散型信頼市場 (Decentralized Trust Market)。

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| なし | - | - | - |

### ⚫︎ Response
| KEY | TYPE | DESCRIPTION |
| --- | --- | --- |
| `status` | string | CA の稼働ステータス (例: "Active") |
| `policies` | string | CA が採用している信頼ロジックやポリシーの概要 |
"#;

#[utoipa::path(
    tag = TAG_P2P_OPTIONAL,
    get,
    path = "/ca/status",
    summary = "CAステータスとポリシーを取得",
    description = CA_STATUS_DESC,
    responses(
        (status = 200, description = "Success", body = CaStatusCaRes),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn get_ca_status(ju: JwtUsr) -> Result<Json<CaStatusCaRes>, ApiError> {
    ju.allow_roles(&[JwtRole::BD, JwtRole::APX, JwtRole::VDR, JwtRole::USR])?;

    Ok(Json(CaStatusCaRes {
        status: "Active".to_string(),
        policies: "TrustLogic: QV (L3=15, L2=1). 1 Node = 1 Citizen.".to_string(),
    }))
}

// -------------------------------------------------------------------------
// Get Local CA Status
// -------------------------------------------------------------------------
const GET_CA_LOCAL_STATUS_DESC: &str = r#"
### ローカル CA ステータス取得
現在のノードが有効な CA（認証局）として動作しているかどうかのステータスを取得します。
任命証（`my_cat`）が登録されており、かつオーナーによる署名が有効で期限内である場合に `is_active: true` となります。
"#;

#[utoipa::path(
    tag = TAG_P2P_OPTIONAL,
    get,
    path = "/ca/status/local",
    summary = "ローカルのCAステータスを取得",
    description = GET_CA_LOCAL_STATUS_DESC,
    responses(
        (status = 200, description = "Success", body = CaStatusRes),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn get_ca_local_status(
    Extension(config_manager): Extension<std::sync::Arc<crate::mycute_settings::ConfigManager>>,
) -> Result<Json<CaStatusRes>, ApiError> {
    let ca_token = ca_bl::get_ca_status(config_manager).await;
    Ok(Json(CaStatusRes { ca_token }))
}

// -------------------------------------------------------------------------
// Register CA Cert
// -------------------------------------------------------------------------
const REGISTER_CA_CERT_DESC: &str = r#"
### CA任命証登録
オーナーによって発行された CA任命証を登録します。
この任命証により、当該ノードは正規の **中央認証局 (Central Authority / Trust Anchor)** として機能します。
- USR のみ使用可能です。正当な任命証を保持している場合のみ登録が成功します。

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `ca_token` | string | required | オーナー署名済みのCA任命証 |

### ⚫︎ Response
| KEY | TYPE | DESCRIPTION |
| --- | --- | --- |
| `success` | boolean | 登録が成功したかどうか |
| `message` | string | 処理結果のメッセージ |
| `ca_token` | string | 登録された任命証 |
"#;

#[utoipa::path(
    tag = TAG_P2P_OPTIONAL,
    post,
    security(("api_jwt_token" = [])),
    path = "/ca/token/register",
    summary = "CA任命証を登録",
    description = REGISTER_CA_CERT_DESC,
    request_body = RegisterCaTokenReq,
    responses(
        (status = 200, description = "Success", body = RegisterCaTokenRes),
        (status = 400, description = "Bad Request", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn register_ca_token_ca(
    ju: JwtUsr,
    Extension(config_manager): Extension<std::sync::Arc<crate::mycute_settings::ConfigManager>>,
    Extension(event_tx): Extension<Arc<broadcast::Sender<InternalEvent>>>,
    Json(req): Json<crate::mode::rt::rtreq::ca_req::RegisterCaTokenReq>,
) -> Result<Json<crate::mode::rt::rtres::ca_res::RegisterCaTokenRes>, ApiError> {
    ju.allow_roles(&[JwtRole::USR])?;
    log::info!("<CA> Register CA Cert requested by user_id={}", ju.usr_id);
    let res = ca_bl::register_ca_token(config_manager, req).await?;

    // 登録成功を通知 (caToken の同期用)
    let seq = time::now_ts_ms();
    let event = InternalEvent {
        seq,
        kind: EventKind::CaStatusChanged(res.ca_token.clone()),
    };
    let _ = event_tx.send(event);
    log::debug!("<CA> CA Cert registered. Event broadcasted.");

    Ok(Json(res))
}

// -------------------------------------------------------------------------
// Unregister CA Cert
// -------------------------------------------------------------------------
const UNREGISTER_CA_CERT_DESC: &str = r#"
### CA任命証削除 (登録解除)
登録されている CA任命証を削除し、認証局としての機能を停止します。
- 安全のため、**オーナーモードが有効な場合のみ** 実行可能です。
"#;

#[utoipa::path(
    tag = TAG_P2P_OPTIONAL,
    post,
    security(("api_jwt_token" = [])),
    path = "/ca/token/unregister",
    summary = "CA任命証を削除",
    description = UNREGISTER_CA_CERT_DESC,
    responses(
        (status = 200, description = "Success", body = UnregisterCaTokenRes),
        (status = 403, description = "Forbidden", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn unregister_ca_token_ca(
    ju: JwtUsr,
    Extension(config_manager): Extension<std::sync::Arc<crate::mycute_settings::ConfigManager>>,
    Extension(event_tx): Extension<Arc<broadcast::Sender<InternalEvent>>>,
) -> Result<Json<crate::mode::rt::rtres::ca_res::UnregisterCaTokenRes>, ApiError> {
    ju.allow_roles(&[JwtRole::USR])?;
    log::info!("<CA> Unregister CA Cert requested by user_id={}", ju.usr_id);
    
    let res = ca_bl::unregister_ca_token(config_manager).await?;

    // 削除成功を通知
    let seq = time::now_ts_ms();
    let event = InternalEvent {
        seq,
        kind: EventKind::CaStatusChanged(None),
    };
    let _ = event_tx.send(event);
    log::debug!("<CA> CA Cert unregistered. Event broadcasted.");

    Ok(Json(res))
}

// -------------------------------------------------------------------------
// Generate License (CA -> User)
// -------------------------------------------------------------------------
const GEN_LICENSE_DESC: &str = r#"
### ライセンス発行 (CA → ユーザー)
このノードが CA（認証局）として動作している場合に、指定した公開鍵のユーザーにライセンスを発行します。

- **USR のみ** 使用可能です。
- 自身が有効な CA 任命証を持っていない場合は失敗します。
- 発行するライセンスの有効期限は、**CA 任命証の有効期限を超えることはできません**。
- 権限内容 (`permissions`) を省略した場合、デフォルト `{"all": true}` が使用される。

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `pubkey_hex` | string | required, len=114 | ライセンス付与先ユーザー host の Ed448 公開鍵 (Hex) |
| `expire_hours` | number | required, 1..87600 | ライセンスの有効期限（時間） |
| `permissions` | object | optional | 権限内容 (JSON オブジェクト) |

### ⚫︎ アクセス権限
- **USR**: 使用可能。有効な CA 任命証が登録されている場合のみ成功します。

### ⚫︎ Response
| KEY | TYPE | DESCRIPTION |
| --- | --- | --- |
| `license` | string | 発行されたライセンス文字列 (base64(payload).sig_hex) |
"#;

#[utoipa::path(
    tag = TAG_P2P_OPTIONAL,
    post,
    security(("api_jwt_token" = [])),
    path = "/ca/genlicense",
    summary = "CA がユーザーにライセンスを発行する。",
    description = GEN_LICENSE_DESC,
    request_body = GenLicenseReq,
    responses(
        (status = 200, description = "Success", body = GenLicenseRes),
        (status = 400, description = "Bad Request", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn generate_license_ca(
    ju: JwtUsr,
    Extension(config_manager): Extension<std::sync::Arc<crate::mycute_settings::ConfigManager>>,
    Json(req): Json<GenLicenseReq>,
) -> Result<Json<GenLicenseRes>, ApiError> {
    ju.allow_roles(&[JwtRole::USR])?;
    req.validate().map_err(|e| ApiError::from_garde(e))?;

    log::debug!(
        "<CA> Generate license requested by user_id={}, target_pubkey={}..., expire_hours={}",
        ju.usr_id,
        &req.pubkey_hex[..8.min(req.pubkey_hex.len())],
        req.expire_hours
    );

    let (license, _expire_at_ms) = license_bl::generate_license(
        config_manager,
        req.pubkey_hex,
        req.expire_hours,
        req.permissions,
    )
    .await?;

    log::debug!("<CA> License generated successfully.");

    Ok(Json(GenLicenseRes {
        license,
    }))
}
