use crate::{
    mode::rt::{
        rtbl::ca_bl,
        rtreq::ca_req::RegisterCaTokenReq,
        rtres::{ca_apps_res::CaStatusCaRes, ca_res::RegisterCaTokenRes, errs_res::ApiError},
    },
    utils::jwt::{JwtRole, JwtUsr},
    TAG_MACRO_P2P_OPTIONAL,
};
use axum::{Extension, Json};

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
    Json(req): Json<crate::mode::rt::rtreq::ca_req::RegisterCaTokenReq>,
) -> Result<Json<crate::mode::rt::rtres::ca_res::RegisterCaTokenRes>, ApiError> {
    ju.allow_roles(&[JwtRole::USR])?;
    log::info!("<CA> Register CA Cert requested by user_id={}", ju.usr_id);
    ca_bl::register_ca_token(config_manager, req)
        .await
        .map(Json)
}
