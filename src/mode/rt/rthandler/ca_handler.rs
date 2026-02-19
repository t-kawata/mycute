use crate::{
    TAG_MACRO_P2P_OPTIONAL,
    mode::rt::{
        rtres::{errs_res::ApiError, ca_apps_res::CaStatusCaRes, ca_res::RegisterCaTokenRes},
        rtreq::ca_req::RegisterCaTokenReq,
        rtbl::ca_bl
    },
    utils::jwt::{JwtUsr, JwtRole},
};
use axum::{Extension, Json};

macro_rules! TAG_NAME { () => { "v1 CA" }; }
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
| TYPE | DESCRIPTION |
| --- | --- |
| Object (`CaStatusCaRes`) | CA の運用ステータスとポリシー |
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
pub async fn get_ca_status(
    ju: JwtUsr,
) -> Result<Json<CaStatusCaRes>, ApiError> {
    ju.allow_roles(&[JwtRole::BD, JwtRole::APX, JwtRole::VDR, JwtRole::USR])?;
    
    Ok(Json(CaStatusCaRes {
        status: "Active".to_string(),
        policies: "TrustLogic: QV (L3=15, L2=1). 1 Node = 1 Citizen.".to_string(),
    }))
}

// -------------------------------------------------------------------------
// Register CA Token
// -------------------------------------------------------------------------
const REGISTER_CA_TOKEN_DESC: &str = r#"
### CAトークン登録 (オーナー任命)
オーナーによって発行された CA トークン（任命証）を登録します。
このトークンにより、CA の自己検証レベルが L3 (Official Citizen) に昇格します。
- APX のみが使用可能です。

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `token` | string | required | オーナー署名済み CA トークン (形式: `signature_hex.expire_ts`) |

### ⚫︎ Response
| TYPE | DESCRIPTION |
| --- | --- |
| `RegisterCaTokenRes` | 登録成功メッセージ |
"#;

#[utoipa::path(
    tag = TAG_P2P_OPTIONAL,
    post,
    security(("api_jwt_token" = [])),
    path = "/ca/token/register",
    summary = "CAトークン(任命証)を登録",
    description = REGISTER_CA_TOKEN_DESC,
    request_body = RegisterCaTokenReq,
    responses(
        (status = 200, description = "Success", body = RegisterCaTokenRes),
        (status = 400, description = "Bad Request", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn register_ca_token_ca(
    ju: JwtUsr,
    Extension(config_manager): Extension<std::sync::Arc<crate::stt_config::ConfigManager>>,
    Json(req): Json<crate::mode::rt::rtreq::ca_req::RegisterCaTokenReq>,
) -> Result<Json<crate::mode::rt::rtres::ca_res::RegisterCaTokenRes>, ApiError> {
    ju.allow_roles(&[JwtRole::APX])?;
    log::info!("<CA> Register CA Token requested by user_id={}", ju.usr_id);
    ca_bl::register_ca_token(config_manager, req).await.map(Json)
}
