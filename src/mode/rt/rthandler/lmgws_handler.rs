use std::sync::Arc;
use axum::{Extension, Json, extract::Path, response::IntoResponse};
use garde::Validate;
use crate::{
    mode::rt::{
        rtreq::lmgws_req::{
            UpdateLmgwConfigReq,
            UpdateLmgwProxyConfigReq,
            SearchLmgwProvidersReq,
            CreateLmgwProviderReq,
            UpdateLmgwProviderReq,
            SearchLmgwKeysReq,
            SearchLmgwModelsReq,
            SearchLmgwModelParametersReq,
            SearchLmgwBaseModelsReq,
        },
        rtres::{
            errs_res::ApiError,
            lmgws_res::{
                GetLmgwConfigRes,
                UpdateLmgwConfigRes,
                LmgwProxyConfigRes,
                UpdateLmgwProxyConfigRes,
                SearchLmgwProvidersRes,
                GetLmgwProviderRes,
                CreateLmgwProviderRes,
                UpdateLmgwProviderRes,
                DeleteLmgwProviderRes,
                SearchLmgwKeysRes,
                SearchLmgwModelsRes,
                SearchLmgwModelParametersRes,
                SearchLmgwBaseModelsRes,
            },
        },
        rtbl::lmgws_bl::BifrostClient,
    },
    utils::jwt::{JwtUsr, JwtRole},
    mycute_settings::ConfigManager,
};

const TAG: &str = "v1 LMGW";

// ============================================================
// Config (構成設定)
// ============================================================

const GET_CONFIG_DESC: &str = r#"
### ⚫︎ 概要
- Bifrost の現在の構成設定を取得します。

### ⚫︎ Response
| KEY | TYPE | DESCRIPTION |
| --- | --- | --- |
| `client` | object | クライアント設定 |
| `client.dropExcessRequests` | boolean | 過剰リクエストをドロップするか否か |
| `client.enableLogging` | boolean | ロギングを有効にするか否か |
| `client.allowedOrigins` | string[] | 許可するオリジンのリスト |

### ⚫︎ アクセス権限
| ロール | 権限 |
| --- | --- |
| BD | ❌ 不可 |
| APX | ❌ 不可 |
| VDR | ❌ 不可 |
| USR | ✅ 利用可能 |
"#;
#[utoipa::path(
    tag = TAG,
    get,
    security(("api_jwt_token" = [])),
    path = "/lmgw/config",
    summary = "Bifrost の構成設定を取得する。",
    description = GET_CONFIG_DESC,
    responses(
        (status = 200, description = "Success", body = GetLmgwConfigRes),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn get_lmgw_config(
    ju: JwtUsr,
    Extension(hc): Extension<Arc<reqwest::Client>>,
    Extension(config_manager): Extension<Arc<ConfigManager>>,
) -> Result<impl IntoResponse, ApiError> {
    ju.allow_roles(&[JwtRole::USR])?;
    log::debug!("<LMGW> get_lmgw_config called.");
    let client = BifrostClient::new(hc, config_manager);
    let res = client.get_config().await?;
    Ok(Json(res))
}

const UPDATE_CONFIG_DESC: &str = r#"
### ⚫︎ 概要
- Bifrost の構成設定を更新します。
- 一部の設定はホットリロード対応ですが、設定によっては再起動が必要です。

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `client` | object | optional | クライアント設定（詳細は GET 参照） |

### ⚫︎ Response
| KEY | TYPE | DESCRIPTION |
| --- | --- | --- |
| `client` | object | 更新後のクライアント設定 |

### ⚫︎ アクセス権限
| ロール | 権限 |
| --- | --- |
| BD | ❌ 不可 |
| APX | ❌ 不可 |
| VDR | ❌ 不可 |
| USR | ✅ 利用可能 |
"#;
#[utoipa::path(
    tag = TAG,
    put,
    security(("api_jwt_token" = [])),
    path = "/lmgw/config",
    summary = "Bifrost の構成設定を更新する。",
    description = UPDATE_CONFIG_DESC,
    request_body = UpdateLmgwConfigReq,
    responses(
        (status = 200, description = "Success", body = UpdateLmgwConfigRes),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 422, description = "Validation Error", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn update_lmgw_config(
    ju: JwtUsr,
    Extension(hc): Extension<Arc<reqwest::Client>>,
    Extension(config_manager): Extension<Arc<ConfigManager>>,
    Json(req): Json<UpdateLmgwConfigReq>,
) -> Result<impl IntoResponse, ApiError> {
    ju.allow_roles(&[JwtRole::USR])?;
    req.validate().map_err(|e| ApiError::from_garde(e))?;
    log::debug!("<LMGW> update_lmgw_config called.");
    let client = BifrostClient::new(hc, config_manager);
    let res = client.update_config(req).await?;
    Ok(Json(res))
}

// ============================================================
// ProxyConfig (プロキシ設定)
// ============================================================

const GET_PROXY_CONFIG_DESC: &str = r#"
### ⚫︎ 概要
- Bifrost のグローバルプロキシ設定を取得します。

### ⚫︎ Response
| KEY | TYPE | DESCRIPTION |
| --- | --- | --- |
| `url` | string | プロキシサーバーURL |
| `username` | string | ユーザー名 |
| `password` | string | パスワード |

### ⚫︎ アクセス権限
| ロール | 権限 |
| --- | --- |
| BD | ❌ 不可 |
| APX | ❌ 不可 |
| VDR | ❌ 不可 |
| USR | ✅ 利用可能 |
"#;
#[utoipa::path(
    tag = TAG,
    get,
    security(("api_jwt_token" = [])),
    path = "/lmgw/proxy-config",
    summary = "Bifrost のプロキシ設定を取得する。",
    description = GET_PROXY_CONFIG_DESC,
    responses(
        (status = 200, description = "Success", body = LmgwProxyConfigRes),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn get_lmgw_proxy_config(
    ju: JwtUsr,
    Extension(hc): Extension<Arc<reqwest::Client>>,
    Extension(config_manager): Extension<Arc<ConfigManager>>,
) -> Result<impl IntoResponse, ApiError> {
    ju.allow_roles(&[JwtRole::USR])?;
    log::debug!("<LMGW> get_lmgw_proxy_config called.");
    let client = BifrostClient::new(hc, config_manager);
    let res = client.get_proxy_config().await?;
    Ok(Json(res))
}

const UPDATE_PROXY_CONFIG_DESC: &str = r#"
### ⚫︎ 概要
- Bifrost のグローバルプロキシ設定を更新します。

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `url` | string | required | プロキシサーバーURL |
| `username` | string | optional | ユーザー名 |
| `password` | string | optional | パスワード |

### ⚫︎ Response
| KEY | TYPE | DESCRIPTION |
| --- | --- | --- |
| `url` | string | 更新後のプロキシサーバーURL |
| `username` | string | 更新後のユーザー名 |
| `password` | string | 更新後のパスワード |

### ⚫︎ アクセス権限
| ロール | 権限 |
| --- | --- |
| BD | ❌ 不可 |
| APX | ❌ 不可 |
| VDR | ❌ 不可 |
| USR | ✅ 利用可能 |
"#;
#[utoipa::path(
    tag = TAG,
    put,
    security(("api_jwt_token" = [])),
    path = "/lmgw/proxy-config",
    summary = "Bifrost のプロキシ設定を更新する。",
    description = UPDATE_PROXY_CONFIG_DESC,
    request_body = UpdateLmgwProxyConfigReq,
    responses(
        (status = 200, description = "Success", body = UpdateLmgwProxyConfigRes),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 422, description = "Validation Error", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn update_lmgw_proxy_config(
    ju: JwtUsr,
    Extension(hc): Extension<Arc<reqwest::Client>>,
    Extension(config_manager): Extension<Arc<ConfigManager>>,
    Json(req): Json<UpdateLmgwProxyConfigReq>,
) -> Result<impl IntoResponse, ApiError> {
    ju.allow_roles(&[JwtRole::USR])?;
    req.validate().map_err(|e| ApiError::from_garde(e))?;
    log::debug!("<LMGW> update_lmgw_proxy_config called.");
    let client = BifrostClient::new(hc, config_manager);
    let res = client.update_proxy_config(req).await?;
    Ok(Json(res))
}

// ============================================================
// Providers (プロバイダー管理)
// ============================================================

const SEARCH_PROVIDERS_DESC: &str = r#"
### ⚫︎ 概要
- 設定されている全てのプロバイダーのリストを取得します。

### ⚫︎ Response
| KEY | TYPE | DESCRIPTION |
| --- | --- | --- |
| `providers` | array | プロバイダーオブジェクトの配列 |
| `providers[].name` | string | プロバイダー識別名 |
| `providers[].provider` | string | 種別（openai, anthropic 等） |
| `providers[].description` | string | 説明 |

### ⚫︎ アクセス権限
| ロール | 権限 |
| --- | --- |
| BD | ❌ 不可 |
| APX | ❌ 不可 |
| VDR | ❌ 不可 |
| USR | ✅ 利用可能 |
"#;
#[utoipa::path(
    tag = TAG,
    post,
    security(("api_jwt_token" = [])),
    path = "/lmgw/providers/search",
    summary = "プロバイダーを検索・一覧取得する。",
    description = SEARCH_PROVIDERS_DESC,
    request_body = SearchLmgwProvidersReq,
    responses(
        (status = 200, description = "Success", body = SearchLmgwProvidersRes),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn search_lmgw_providers(
    ju: JwtUsr,
    Extension(hc): Extension<Arc<reqwest::Client>>,
    Extension(config_manager): Extension<Arc<ConfigManager>>,
    Json(_req): Json<SearchLmgwProvidersReq>,
) -> Result<impl IntoResponse, ApiError> {
    ju.allow_roles(&[JwtRole::USR])?;
    log::debug!("<LMGW> search_lmgw_providers called.");
    let client = BifrostClient::new(hc, config_manager);
    let res = client.search_providers().await?;
    Ok(Json(res))
}

const GET_PROVIDER_DESC: &str = r#"
### ⚫︎ 概要
- 指定されたプロバイダーの設定を取得します。

### ⚫︎ Response
| KEY | TYPE | DESCRIPTION |
| --- | --- | --- |
| `name` | string | プロバイダー識別名 |
| `provider` | string | 種別（openai, anthropic 等） |
| `description` | string | 説明 |

### ⚫︎ アクセス権限
| ロール | 権限 |
| --- | --- |
| BD | ❌ 不可 |
| APX | ❌ 不可 |
| VDR | ❌ 不可 |
| USR | ✅ 利用可能 |
"#;
#[utoipa::path(
    tag = TAG,
    get,
    security(("api_jwt_token" = [])),
    path = "/lmgw/providers/{provider_name}",
    summary = "特定のプロバイダー設定を取得する。",
    description = GET_PROVIDER_DESC,
    params(
        ("provider_name" = String, Path, description = "プロバイダー識別名"),
    ),
    responses(
        (status = 200, description = "Success", body = GetLmgwProviderRes),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 404, description = "Not Found", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn get_lmgw_provider(
    ju: JwtUsr,
    Path(provider_name): Path<String>,
    Extension(hc): Extension<Arc<reqwest::Client>>,
    Extension(config_manager): Extension<Arc<ConfigManager>>,
) -> Result<impl IntoResponse, ApiError> {
    ju.allow_roles(&[JwtRole::USR])?;
    log::debug!("<LMGW> get_lmgw_provider called. provider: {}", provider_name);
    let client = BifrostClient::new(hc, config_manager);
    let res = client.get_provider(&provider_name).await?;
    Ok(Json(res))
}

const CREATE_PROVIDER_DESC: &str = r#"
### ⚫︎ 概要
- 新しいプロバイダーを追加します。

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `name` | string | required | プロバイダー識別名 |
| `provider` | string | required | 種別（openai 等） |
| `description` | string | optional | 説明 |

### ⚫︎ Response
| KEY | TYPE | DESCRIPTION |
| --- | --- | --- |
| `name` | string | 追加されたプロバイダー識別名 |

### ⚫︎ アクセス権限
| ロール | 権限 |
| --- | --- |
| BD | ❌ 不可 |
| APX | ❌ 不可 |
| VDR | ❌ 不可 |
| USR | ✅ 利用可能 |
"#;
#[utoipa::path(
    tag = TAG,
    post,
    security(("api_jwt_token" = [])),
    path = "/lmgw/providers",
    summary = "新しいプロバイダーを追加する。",
    description = CREATE_PROVIDER_DESC,
    request_body = CreateLmgwProviderReq,
    responses(
        (status = 200, description = "Success", body = CreateLmgwProviderRes),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 422, description = "Validation Error", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn create_lmgw_provider(
    ju: JwtUsr,
    Extension(hc): Extension<Arc<reqwest::Client>>,
    Extension(config_manager): Extension<Arc<ConfigManager>>,
    Json(req): Json<CreateLmgwProviderReq>,
) -> Result<impl IntoResponse, ApiError> {
    ju.allow_roles(&[JwtRole::USR])?;
    req.validate().map_err(|e| ApiError::from_garde(e))?;
    log::debug!("<LMGW> create_lmgw_provider called. name: {}", req.name);
    let client = BifrostClient::new(hc, config_manager);
    let res = client.create_provider(req).await?;
    Ok(Json(res))
}

const UPDATE_PROVIDER_DESC: &str = r#"
### ⚫︎ 概要
- 既存のプロバイダー設定を更新します。
- **この操作は全フィールドを上書きします。** 変更しないフィールドも必ず含めて送信してください。

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `provider` | string | required | 種別 |
| `description` | string | optional | 説明 |

### ⚫︎ Response
| KEY | TYPE | DESCRIPTION |
| --- | --- | --- |
| `name` | string | 更新されたプロバイダー識別名 |

### ⚫︎ アクセス権限
| ロール | 権限 |
| --- | --- |
| BD | ❌ 不可 |
| APX | ❌ 不可 |
| VDR | ❌ 不可 |
| USR | ✅ 利用可能 |
"#;
#[utoipa::path(
    tag = TAG,
    put,
    security(("api_jwt_token" = [])),
    path = "/lmgw/providers/{provider_name}",
    summary = "プロバイダー設定を更新する。",
    description = UPDATE_PROVIDER_DESC,
    params(
        ("provider_name" = String, Path, description = "プロバイダー識別名"),
    ),
    request_body = UpdateLmgwProviderReq,
    responses(
        (status = 200, description = "Success", body = UpdateLmgwProviderRes),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 422, description = "Validation Error", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn update_lmgw_provider(
    ju: JwtUsr,
    Path(provider_name): Path<String>,
    Extension(hc): Extension<Arc<reqwest::Client>>,
    Extension(config_manager): Extension<Arc<ConfigManager>>,
    Json(req): Json<UpdateLmgwProviderReq>,
) -> Result<impl IntoResponse, ApiError> {
    ju.allow_roles(&[JwtRole::USR])?;
    req.validate().map_err(|e| ApiError::from_garde(e))?;
    log::debug!("<LMGW> update_lmgw_provider called. provider: {}", provider_name);
    let client = BifrostClient::new(hc, config_manager);
    let res = client.update_provider(&provider_name, req).await?;
    Ok(Json(res))
}

const DELETE_PROVIDER_DESC: &str = r#"
### ⚫︎ 概要
- 指定されたプロバイダーを削除します。

### ⚫︎ Response
| KEY | TYPE | DESCRIPTION |
| --- | --- | --- |
| `name` | string | 削除されたプロバイダー識別名 |

### ⚫︎ アクセス権限
| ロール | 権限 |
| --- | --- |
| BD | ❌ 不可 |
| APX | ❌ 不可 |
| VDR | ❌ 不可 |
| USR | ✅ 利用可能 |
"#;
#[utoipa::path(
    tag = TAG,
    delete,
    security(("api_jwt_token" = [])),
    path = "/lmgw/providers/{provider_name}",
    summary = "プロバイダーを削除する。",
    description = DELETE_PROVIDER_DESC,
    params(
        ("provider_name" = String, Path, description = "プロバイダー識別名"),
    ),
    responses(
        (status = 200, description = "Success", body = DeleteLmgwProviderRes),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn delete_lmgw_provider(
    ju: JwtUsr,
    Path(provider_name): Path<String>,
    Extension(hc): Extension<Arc<reqwest::Client>>,
    Extension(config_manager): Extension<Arc<ConfigManager>>,
) -> Result<impl IntoResponse, ApiError> {
    ju.allow_roles(&[JwtRole::USR])?;
    log::debug!("<LMGW> delete_lmgw_provider called. provider: {}", provider_name);
    let client = BifrostClient::new(hc, config_manager);
    let res = client.delete_provider(&provider_name).await?;
    Ok(Json(res))
}

// ============================================================
// Keys (API キー管理)
// ============================================================

const SEARCH_KEYS_DESC: &str = r#"
### ⚫︎ 概要
- 全プロバイダーにわたる設定済み API キーのリストを取得します。

### ⚫︎ Response
| KEY | TYPE | DESCRIPTION |
| --- | --- | --- |
| `keys` | array | キー情報の配列 |
| `keys[].provider` | string | プロバイダー名 |
| `keys[].key` | string | マスクされた API キー |

### ⚫︎ アクセス権限
| ロール | 権限 |
| --- | --- |
| BD | ❌ 不可 |
| APX | ❌ 不可 |
| VDR | ❌ 不可 |
| USR | ✅ 利用可能 |
"#;
#[utoipa::path(
    tag = TAG,
    post,
    security(("api_jwt_token" = [])),
    path = "/lmgw/keys/search",
    summary = "API キーを検索・一覧取得する。",
    description = SEARCH_KEYS_DESC,
    request_body = SearchLmgwKeysReq,
    responses(
        (status = 200, description = "Success", body = SearchLmgwKeysRes),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn search_lmgw_keys(
    ju: JwtUsr,
    Extension(hc): Extension<Arc<reqwest::Client>>,
    Extension(config_manager): Extension<Arc<ConfigManager>>,
    Json(_req): Json<SearchLmgwKeysReq>,
) -> Result<impl IntoResponse, ApiError> {
    ju.allow_roles(&[JwtRole::USR])?;
    log::debug!("<LMGW> search_lmgw_keys called.");
    let client = BifrostClient::new(hc, config_manager);
    let res = client.search_keys().await?;
    Ok(Json(res))
}

// ============================================================
// Models (モデル情報)
// ============================================================

const SEARCH_MODELS_DESC: &str = r#"
### ⚫︎ 概要
- 全てのプロバイダーを通じて利用可能なモデルの一覧を取得します。

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `query` | string | optional | モデル名の部分一致フィルター |
| `provider` | string | optional | プロバイダー名フィルター |
| `limit` | integer | optional | 最大返却件数 |

### ⚫︎ Response
| KEY | TYPE | DESCRIPTION |
| --- | --- | --- |
| `models` | array | モデルオブジェクトの配列 |

### ⚫︎ アクセス権限
| ロール | 権限 |
| --- | --- |
| BD | ❌ 不可 |
| APX | ❌ 不可 |
| VDR | ❌ 不可 |
| USR | ✅ 利用可能 |
"#;
#[utoipa::path(
    tag = TAG,
    post,
    security(("api_jwt_token" = [])),
    path = "/lmgw/models/search",
    summary = "利用可能なモデルを検索・一覧取得する。",
    description = SEARCH_MODELS_DESC,
    request_body = SearchLmgwModelsReq,
    responses(
        (status = 200, description = "Success", body = SearchLmgwModelsRes),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn search_lmgw_models(
    ju: JwtUsr,
    Extension(hc): Extension<Arc<reqwest::Client>>,
    Extension(config_manager): Extension<Arc<ConfigManager>>,
    Json(req): Json<SearchLmgwModelsReq>,
) -> Result<impl IntoResponse, ApiError> {
    ju.allow_roles(&[JwtRole::USR])?;
    log::debug!("<LMGW> search_lmgw_models called.");
    let client = BifrostClient::new(hc, config_manager);
    let res = client.search_models(req).await?;
    Ok(Json(res))
}

const SEARCH_MODEL_PARAMETERS_DESC: &str = r#"
### ⚫︎ 概要
- モデルで利用可能なパラメーター定義の一覧を取得します。

### ⚫︎ Response
| KEY | TYPE | DESCRIPTION |
| --- | --- | --- |
| `parameters` | object | プロバイダーごとのパラメーター定義 |

### ⚫︎ アクセス権限
| ロール | 権限 |
| --- | --- |
| BD | ❌ 不可 |
| APX | ❌ 不可 |
| VDR | ❌ 不可 |
| USR | ✅ 利用可能 |
"#;
#[utoipa::path(
    tag = TAG,
    post,
    security(("api_jwt_token" = [])),
    path = "/lmgw/models/parameters/search",
    summary = "モデルパラメーター定義を取得する。",
    description = SEARCH_MODEL_PARAMETERS_DESC,
    request_body = SearchLmgwModelParametersReq,
    responses(
        (status = 200, description = "Success", body = SearchLmgwModelParametersRes),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn search_lmgw_model_parameters(
    ju: JwtUsr,
    Extension(hc): Extension<Arc<reqwest::Client>>,
    Extension(config_manager): Extension<Arc<ConfigManager>>,
    Json(_req): Json<SearchLmgwModelParametersReq>,
) -> Result<impl IntoResponse, ApiError> {
    ju.allow_roles(&[JwtRole::USR])?;
    log::debug!("<LMGW> search_lmgw_model_parameters called.");
    let client = BifrostClient::new(hc, config_manager);
    let res = client.search_model_parameters().await?;
    Ok(Json(res))
}

const SEARCH_BASE_MODELS_DESC: &str = r#"
### ⚫︎ 概要
- モデルカタログからベースモデルの一覧を取得します。

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `query` | string | optional | モデル名フィルター |
| `provider` | string | optional | プロバイダーフィルター |
| `limit` | integer | optional | 最大返却件数 |

### ⚫︎ Response
| KEY | TYPE | DESCRIPTION |
| --- | --- | --- |
| `models` | array | ベースモデル情報の配列 |

### ⚫︎ アクセス権限
| ロール | 権限 |
| --- | --- |
| BD | ❌ 不可 |
| APX | ❌ 不可 |
| VDR | ❌ 不可 |
| USR | ✅ 利用可能 |
"#;
#[utoipa::path(
    tag = TAG,
    post,
    security(("api_jwt_token" = [])),
    path = "/lmgw/models/base/search",
    summary = "ベースモデル一覧を取得する。",
    description = SEARCH_BASE_MODELS_DESC,
    request_body = SearchLmgwBaseModelsReq,
    responses(
        (status = 200, description = "Success", body = SearchLmgwBaseModelsRes),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn search_lmgw_base_models(
    ju: JwtUsr,
    Extension(hc): Extension<Arc<reqwest::Client>>,
    Extension(config_manager): Extension<Arc<ConfigManager>>,
    Json(req): Json<SearchLmgwBaseModelsReq>,
) -> Result<impl IntoResponse, ApiError> {
    ju.allow_roles(&[JwtRole::USR])?;
    log::debug!("<LMGW> search_lmgw_base_models called.");
    let client = BifrostClient::new(hc, config_manager);
    let res = client.search_base_models(req).await?;
    Ok(Json(res))
}
