use crate::constants::ST_BAD_REQUEST;
use crate::{
    constants::{
        APP_BUILD_ZIP_DEFAULT_FILENAME, APP_BUILD_ZIP_PARAM, APP_INSTALL_MYCUTE_PARAM,
        ERR_EMPTY_FILE, ERR_MULTIPART, ERR_READ_FILE,
    },
    mode::rt::client::secure_client::SecureClient,
    mode::rt::{
        rtbl::node_apps_bl,
        rtreq::node_apps_req::{
            AdvertiseAppNodeReq, BuildAppNodeReq, DiscoverAppNodeReq, InstallAppFileNodeReq,
            VerifyAppNodeReq, VoteAppNodeReq,
        },
        rtres::{
            errs_res::ApiError,
            node_apps_res::{
                AdvertiseAppNodeRes, AppInfoNodeRes, DiscoverAppNodeRes, VerifyAppNodeRes,
                VoteAppNodeRes,
            },
        },
        rtutils::db_for_rt::DbPoolsExt,
    },
    utils::{
        db::DbPools,
        jwt::{JwtIDs, JwtRole, JwtUsr},
    },
};
use axum::{response::IntoResponse, Extension, Json};
use garde::Validate;
use std::sync::Arc;

const TAG: &str = "v1 Node Apps";

// ============================================================
// Build App
// ============================================================
const BUILD_DESC: &str = r#"
### ⚫︎ 概要
アプリケーションのソースコード (.zip) を入力として受け取り、配布可能な独自のパッケージ形式 **(.mycute)** を生成（ビルド）します。

このプロセスは単なるファイルの圧縮・アーカイブではなく、MYCUTE エコシステムにおける **「分散型信頼モデル」** の基点となる重要なステップです。ビルドを実行したノード自身が「このパッケージは私が責任を持って作成した」という証として、自身の秘密鍵を用いてパッケージ全体に **署名（封印）** を施します。

### ⚫︎ 開発・ビルドの手順
開発者は以下の手順でアプリケーションを配布可能な状態にします：

1.  **ソースコードの準備**:
    - アプリの実体（HTML/JS/Assets 等）を用意します。
    - ルートディレクトリに必ず `mycute.json` (マニフェストファイル) を配置してください。

2.  **マニフェストの記述**:
    - `global_app_id` (UUID v4) と `global_app_version` (00000.00.00 形式) を定義します。
    - `name` (アプリ名), `author` (開発者名), `description` (説明文) は必須項目です。

3.  **アーカイブの作成**:
    - マニフェストを含むルートディレクトリの内容を ZIP 形式で圧縮します。

4.  **ビルド API の呼び出し**:
    - 本エンドポイントに対し、ZIP ファイルを multipart/form-data で送信します。
    - システムは内部でマニフェストの正当性を検証し、制御文字の自動除去（サニタイズ）を行い、Zstd (Level 19) で再圧縮してパッケージ化します。

5.  **成果物の受け取り**:
    - 成功すると `{Name}.{Version}.mycute` というパッケージファイルがバイナリストリームとして返却されます。

### ⚫︎ バリデーションとサニタイズ規則
システムの健全性を保つため、ビルド時に以下の自動補正とチェックが適用されます：
- **制御文字の自動除去**: `name`, `author`, `description` 内の改行、タブ、および非表示文字は、ビルドプロセス中に自動的に削除（サニタイズ）されます。
- **必須チェック**: サニタイズ後にこれらのフィールドが空になった場合、または最大長を超えている場合はビルドが拒否されます。
- **依存関係**: `dependencies` に指定された ID が正しい UUID 形式であることを確認します。

### ⚫︎ 成果物 (.mycute) の利用
得られた `.mycute` パッケージは、以下の属性を持ちます：
- **不変性**: ビルド後のパッケージはバイナリ全体が暗号学的に署名されており、1ビットの改ざんも許しません。
- **検証可能性**: インストール先のノードは、パッケージに同封された検証レコードとアプリ署名を照合し、作成者の身元と整合性を即座に検証できます。

### ⚫︎ 権限
- **USR**: 本操作は「信頼の起点」となるため、開発者として認証された USR ロールのみが実行可能です。

### ⚫︎ Request (Multipart)
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `zip` | binary (zip) | required | ソースコード一式と mycute.json を含んだ ZIP ファイル |

### ⚫︎ Response
| TYPE | CONTENT-TYPE | FILENAME | DESCRIPTION |
| --- | --- | --- | --- |
| binary stream | application/octet-stream | {name}.{ver}.mycute | ビルド・署名済みのアプリケーションパッケージ |
"#;
#[utoipa::path(
    tag = TAG,
    post,
    security(("api_jwt_token" = [])),
    path = "/node/apps/build",
    summary = "アプリケーションをビルドする (Node)。",
    description = BUILD_DESC,
    request_body(content = BuildAppNodeReq, content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "Success", body = Vec<u8>),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn build_app_node(
    ju: JwtUsr,
    ids: JwtIDs,
    Extension(db): Extension<Arc<DbPools>>,
    Extension(config_manager): Extension<Arc<crate::stt_config::ConfigManager>>,
    mut multipart: axum::extract::Multipart,
) -> Result<impl IntoResponse, ApiError> {
    ju.allow_roles(&[JwtRole::USR])?;
    let conn = db.get_ro_for_rt()?;

    let mut zip_data = Vec::new();
    let mut filename = String::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::new_system(ST_BAD_REQUEST, ERR_MULTIPART, e.to_string()))?
    {
        let name = field.name().unwrap_or("").to_string();
        if name == APP_BUILD_ZIP_PARAM {
            filename = field
                .file_name()
                .unwrap_or(APP_BUILD_ZIP_DEFAULT_FILENAME)
                .to_string();
            zip_data = field
                .bytes()
                .await
                .map_err(|e| ApiError::new_system(ST_BAD_REQUEST, ERR_READ_FILE, e.to_string()))?
                .to_vec();
        }
    }

    if zip_data.is_empty() {
        return Err(ApiError::new_system(
            ST_BAD_REQUEST,
            ERR_EMPTY_FILE,
            "Zip file is required.",
        ));
    }

    let (output_filename, binary_data) =
        node_apps_bl::build_app_node(conn, &ju, &ids, zip_data, filename, config_manager).await?;

    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        "application/octet-stream".parse().unwrap(),
    );
    headers.insert(
        axum::http::header::CONTENT_DISPOSITION,
        format!("attachment; filename=\"{}\"", output_filename)
            .parse()
            .unwrap(),
    );

    Ok((headers, binary_data))
}

// ============================================================
// Install App
// ============================================================
const INSTALL_FILE_DESC: &str = r#"
### ⚫︎ 概要
- クライアントからアップロードされた .mycute パッケージをインストールする。
- 署名検証後にローカルディレクトリに展開し、DBに登録する。

### ⚫︎ 権限
- **USR**: のみ実行可能。

### ⚫︎ Request (Multipart)
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `mycute` | binary | required | .mycute パッケージファイル |

### ⚫︎ Response
| TYPE | DESCRIPTION |
| --- | --- |
| Object (`AppInfoNodeRes`) | インストールされたアプリの情報 |
"#;
#[utoipa::path(
    tag = TAG,
    post,
    security(("api_jwt_token" = [])),
    path = "/node/apps/install/file",
    summary = "ローカルファイルからアプリをインストールする (Node)。",
    description = INSTALL_FILE_DESC,
    request_body(content = InstallAppFileNodeReq, content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "Success", body = AppInfoNodeRes),
    )
)]
pub async fn install_app_file_node(
    ju: JwtUsr,
    ids: JwtIDs,
    Extension(db): Extension<Arc<DbPools>>,
    Extension(config_manager): Extension<Arc<crate::stt_config::ConfigManager>>,
    mut multipart: axum::extract::Multipart,
) -> Result<Json<AppInfoNodeRes>, ApiError> {
    ju.allow_roles(&[JwtRole::USR])?;
    let conn = db.get_rw_for_rt()?;
    let mut package_data = Vec::new();
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::new_system(ST_BAD_REQUEST, ERR_MULTIPART, e.to_string()))?
    {
        let name = field.name().unwrap_or("").to_string();
        if name == APP_INSTALL_MYCUTE_PARAM {
            package_data = field
                .bytes()
                .await
                .map_err(|e| ApiError::new_system(ST_BAD_REQUEST, ERR_READ_FILE, e.to_string()))?
                .to_vec();
        }
    }
    let res =
        node_apps_bl::install_app_file_node(conn, &ju, &ids, package_data, config_manager).await?;
    Ok(Json(res))
}

// ============================================================
// Verify App
// ============================================================
const VERIFY_DESC: &str = r#"
### ⚫︎ 概要
- クライアントからアップロードされた .mycute パッケージの署名と信頼チェーンを検証する。
- この操作は**非破壊的**であり、アプリケーションのインストールやデータベースへの登録は行いません。
- パッケージから抽出された「信用情報 (AppTrustInfo)」を返却します。

### ⚫︎ 権限
- **USR**: のみ実行可能。

### ⚫︎ Request (Multipart)
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `mycute` | binary | required | 検証対象の .mycute パッケージファイル |

### ⚫︎ Response
| TYPE | DESCRIPTION |
| --- | --- |
| Object (`VerifyAppNodeRes`) | 解析・検証されたアプリの信用情報 |
"#;
#[utoipa::path(
    tag = TAG,
    post,
    security(("api_jwt_token" = [])),
    path = "/node/apps/verify",
    summary = "アプリパッケージの正当性を検証する (Node)。",
    description = VERIFY_DESC,
    request_body(content = VerifyAppNodeReq, content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "Success", body = VerifyAppNodeRes),
    )
)]
pub async fn verify_app_node(
    ju: JwtUsr,
    ids: JwtIDs,
    Extension(db): Extension<Arc<DbPools>>,
    Extension(config_manager): Extension<Arc<crate::stt_config::ConfigManager>>,
    mut multipart: axum::extract::Multipart,
) -> Result<Json<crate::mode::rt::rtres::node_apps_res::VerifyAppNodeRes>, ApiError> {
    ju.allow_roles(&[JwtRole::USR])?;
    let conn = db.get_ro_for_rt()?; // 検証だけなら RO で十分
    let mut package_data = Vec::new();
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| ApiError::new_system(ST_BAD_REQUEST, ERR_MULTIPART, e.to_string()))?
    {
        let name = field.name().unwrap_or("").to_string();
        if name == APP_INSTALL_MYCUTE_PARAM {
            package_data = field
                .bytes()
                .await
                .map_err(|e| ApiError::new_system(ST_BAD_REQUEST, ERR_READ_FILE, e.to_string()))?
                .to_vec();
        }
    }
    if package_data.is_empty() {
        return Err(ApiError::new_system(
            ST_BAD_REQUEST,
            ERR_EMPTY_FILE,
            "Package file is required.",
        ));
    }
    let res = node_apps_bl::verify_app_node(conn, &ju, &ids, package_data, config_manager).await?;
    Ok(Json(res))
}

// ============================================================
// Discover App (Proxy)
// ============================================================
const DISCOVER_DESC: &str = r#"
### ⚫︎ 概要
- 指定したアプリを保持しているノードを CA に問い合わせる (Proxy)。
- 複数のアプリ ID 指定、または名前による曖昧検索をサポート。

### ⚫︎ 権限
- **USR / VDR**: 実行可能。

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `ca_base_url` | string (url) | required | 問い合わせ先 CA の URL |
| `app_ids` | array (string) | optional | 検索対象のアプリ ID リスト |
| `query` | string | optional | 名前による曖昧検索クエリ |

### ⚫︎ Response
| TYPE | DESCRIPTION |
| --- | --- |
| Object (`DiscoverAppNodeRes`) | 検索結果（アプリごとのノードリスト） |
"#;
#[utoipa::path(
    tag = TAG,
    post,
    security(("api_jwt_token" = [])),
    path = "/node/apps/discover",
    summary = "アプリを保持するノードを問い合わせる (Node Proxy)。",
    description = DISCOVER_DESC,
    request_body = DiscoverAppNodeReq,
    responses(
        (status = 200, description = "Success", body = DiscoverAppNodeRes),
    )
)]
pub async fn discover_app_node(
    ju: JwtUsr,
    Extension(client): Extension<Arc<SecureClient>>,
    Json(req): Json<DiscoverAppNodeReq>,
) -> Result<Json<DiscoverAppNodeRes>, ApiError> {
    ju.allow_roles(&[JwtRole::USR, JwtRole::VDR])?;
    if let Err(e) = req.validate() {
        return Err(ApiError::from_garde(e));
    }
    let res = node_apps_bl::discover_app_node(&client, req).await?;
    Ok(Json(res))
}

// ============================================================
// Advertise App (Proxy)
// ============================================================
const ADVERTISE_DESC: &str = r#"
### ⚫︎ 概要
- アプリを CA ネットワークを介して P2P 上に公開 (広告) する (Proxy)。

### ⚫︎ 権限
- **USR / VDR**: 実行可能。

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `ca_base_url` | string (url) | required | 広告先 CA の URL |
| `app_id` | string (uuid) | required | 広告対象のアプリ ID |

### ⚫︎ Response
| TYPE | DESCRIPTION |
| --- | --- |
| Object (`AdvertiseAppNodeRes`) | 広告の結果 |
"#;
#[utoipa::path(
    tag = TAG,
    post,
    security(("api_jwt_token" = [])),
    path = "/node/apps/advertise",
    summary = "アプリをCAネットワークに広告する (Node Proxy)。",
    description = ADVERTISE_DESC,
    request_body = AdvertiseAppNodeReq,
    responses(
        (status = 200, description = "Success", body = AdvertiseAppNodeRes),
    )
)]
pub async fn advertise_app_node(
    ju: JwtUsr,
    Extension(client): Extension<Arc<SecureClient>>,
    Json(req): Json<AdvertiseAppNodeReq>,
) -> Result<Json<AdvertiseAppNodeRes>, ApiError> {
    ju.allow_roles(&[JwtRole::USR, JwtRole::VDR])?;
    if let Err(e) = req.validate() {
        return Err(ApiError::from_garde(e));
    }
    let res = node_apps_bl::advertise_app_node(&client, req).await?;
    Ok(Json(res))
}

// ============================================================
// Vote App (Proxy)
// ============================================================
const VOTE_DESC: &str = r#"
### ⚫︎ 概要
- アプリケーションに対して投票を行う (Proxy)。

### ⚫︎ 権限
- **USR**: のみ実行可能。

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `ca_base_url` | string (url) | required | 投票先 CA の URL |
| `app_id` | string (uuid) | required | 対象アプリ ID |
| `vote` | number (0~15) | required | 投票値 (0は取り消し) |

### ⚫︎ Response
| TYPE | DESCRIPTION |
| --- | --- |
| Object (`VoteAppNodeRes`) | 更新後のスコアとレイヤー |
"#;
#[utoipa::path(
    tag = TAG,
    post,
    security(("api_jwt_token" = [])),
    path = "/node/apps/vote",
    summary = "アプリに投票する (Node Proxy)",
    description = VOTE_DESC,
    request_body = VoteAppNodeReq,
    responses(
        (status = 200, description = "Success", body = VoteAppNodeRes),
    )
)]
pub async fn vote_app_node(
    ju: JwtUsr,
    Extension(client): Extension<Arc<SecureClient>>,
    Extension(config_manager): Extension<Arc<crate::stt_config::ConfigManager>>,
    Extension(db): Extension<Arc<DbPools>>, // Need DB to read tickets
    Json(req): Json<VoteAppNodeReq>,
) -> Result<Json<VoteAppNodeRes>, ApiError> {
    ju.allow_roles(&[JwtRole::USR])?;
    if let Err(e) = req.validate() {
        return Err(ApiError::from_garde(e));
    }
    let res = node_apps_bl::vote_app_node(&db, &client, req, config_manager).await?;
    Ok(Json(res))
}
