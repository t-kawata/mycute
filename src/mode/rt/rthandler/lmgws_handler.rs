use crate::{
    mode::rt::{
        rtbl::lmgws_bl,
        rterr::rterr,
        rtreq::lmgws_req::SaveLmgwProvidersReq,
        rtres::{
            errs_res::ApiError,
            lmgws_res::{GetLmgwProvidersRes, SaveLmgwProvidersRes},
        },
        rtutils::db_for_rt::DbPoolsExt,
    },
    mycute_settings::ConfigManager,
    utils::{
        db::DbPools,
        jwt::{JwtIDs, JwtRole, JwtUsr},
    },
};
use axum::{
    body::Body,
    extract::Path,
    http::{HeaderMap, Method},
    response::IntoResponse,
    Extension, Json,
};
use garde::Validate;
use std::sync::Arc;

const TAG: &str = "v1 LMGW";

const PROXY_LMGW_DESC: &str = r#"
### Bifrost 透過プロキシエンドポイント

本エンドポイントは、Bifrost が提供する**全ての** API（推論・管理・設定など）を透過的に中継します。

---

### 📖 API 仕様の参照先

APIの詳細な仕様（エンドポイントパス、HTTPメソッド、リクエストボディ、レスポンス形式）については、
**Bifrost 公式ドキュメント (https://docs.getbifrost.ai/) が 100% 正解（Single Source of Truth）** となります。

---

### 🔗 パス（URL）の変換ルール

Bifrost 公式ドキュメントに記載されているエンドポイントのパスの先頭に、
MYCUTE 上のベースパスである `/v1/lmgw` を付与して呼び出してください。

**変換式**: `Bifrost ドキュメントのパス` → `/v1/lmgw` + `Bifrost ドキュメントのパス`

#### 変換例

| Bifrost 公式ドキュメントのパス | MYCUTE 上で呼び出すパス |
| --- | --- |
| `POST /v1/chat/completions` | `POST /v1/lmgw/v1/chat/completions` |
| `POST /v1/completions` | `POST /v1/lmgw/v1/completions` |
| `GET /api/providers` | `GET /v1/lmgw/api/providers` |
| `POST /api/providers` | `POST /v1/lmgw/api/providers` |
| `GET /api/providers/{provider}` | `GET /v1/lmgw/api/providers/{provider}` |
| `GET /v1/models` | `GET /v1/lmgw/v1/models` |
| `GET /api/config` | `GET /v1/lmgw/api/config` |
| `POST /openai/v1/chat/completions` | `POST /v1/lmgw/openai/v1/chat/completions` |
| `POST /anthropic/v1/messages` | `POST /v1/lmgw/anthropic/v1/messages` |
| `POST /v1/embeddings` | `POST /v1/lmgw/v1/embeddings` |
| `POST /v1/audio/speech` | `POST /v1/lmgw/v1/audio/speech` |
| `POST /v1/images/generations` | `POST /v1/lmgw/v1/images/generations` |
| `POST /v1/batches` | `POST /v1/lmgw/v1/batches` |
| `POST /v1/files` | `POST /v1/lmgw/v1/files` |

---

### 🔐 アクセス権限

| ロール | 権限 |
| --- | --- |
| BD | ❌ 不可 |
| APX | ❌ 不可 |
| VDR | ❌ 不可 |
| USR | ✅ 利用可能 |

---

### ⚠️ 注意事項

- エラーレスポンスのフォーマットは Bifrost の仕様（JSON）に準拠します（MYCUTE 標準の ApiError 形式ではありません）。
- リクエストボディのバリデーションは行いません（Bifrost 側が処理します）。
- `stream: true` を指定した場合、SSE（Server-Sent Events）のストリーミングレスポンスがそのまま返されます。

---

### 🛠 プロバイダー及びAPI KEYの設定リクエスト例

Bifrost (v1.4.24) では、**プロバイダーの登録時に API KEY を配列として同時に設定する**必要があります。単独での「キー追加」エンドポイントは存在しないため、キーを追加・変更する場合はプロバイダーを再登録してください。

#### 1. プロバイダーの管理

- **プロバイダーの登録（API KEY を含む）**
  `openai`, `anthropic`, `google` などのキーワードを指定し、同時に `keys` フィールドにキーの配列を渡します。
  ```bash
  curl -X POST http://localhost:3910/v1/lmgw/api/providers \
    -H "Authorization: Bearer <TOKEN>" \
    -H "Content-Type: application/json" \
    -d '{
      "provider": "openai",
      "keys": [
        {
          "name": "openai-1",
          "value": "sk-proj-...",
          "models": [],
          "weight": 1.0
        }
      ]
    }'
  ```
- **カスタムプロバイダーを登録する場合**
  任意の名前を付ける場合は、`base_url` の指定が必須となります。
  ```bash
  curl -X POST http://localhost:3910/v1/lmgw/api/providers \
    -H "Authorization: Bearer <TOKEN>" \
    -H "Content-Type: application/json" \
    -d '{
      "provider": "my-custom-proxy",
      "base_url": "https://api.yourproxy.com/v1",
      "keys": [
        {
          "name": "proxy-key",
          "value": "your-secret",
          "models": [],
          "weight": 1.0
        }
      ]
    }'
  ```
- **プロバイダーの一覧取得**
  ```bash
  curl -H "Authorization: Bearer <TOKEN>" http://localhost:3910/v1/lmgw/api/providers
  ```
- **プロバイダーの削除**
  キーの追加・変更を行いたい場合は、一度プロバイダーを削除してから、新しいキー情報を含めて `POST` し直してください。
  ```bash
  curl -X DELETE http://localhost:3910/v1/lmgw/api/providers/openai \
    -H "Authorization: Bearer <TOKEN>"
  ```

#### 2. API KEY の動作と管理

Bifrost は設定された複数のキーを `weight` に基づいて自動的に負荷分散します。

- **API KEY の動作について**
  - `models: []` (空配列) を指定することで、そのプロバイダーがサポートする全てのモデルでこのキーが利用可能になります。
  - 複数のキーが同じモデルをカバーしている場合、リクエストはそれぞれの `weight` に応じた確率でランダムに各キーへ振り分けられます。
- **API KEY の編集・削除**
  独立したキー操作エンドポイントが 405 を返す場合は、前述の通り「プロバイダーの削除」と「キー情報を含めた再登録」を行うのが最も確実な方法です。
  ```bash
  # 1. 既存のプロバイダーを削除
  curl -X DELETE http://localhost:3910/v1/lmgw/api/providers/openai -H "Authorization: Bearer <TOKEN>"
  
  # 2. 新しいキーを含めて再登録
  curl -X POST http://localhost:3910/v1/lmgw/api/providers ... (略)
  ```
"#;

/// `/lmgw/*proxy_path` へのリクエストを Bifrost に透過転送するハンドラー。
///
/// # 責務
/// 1. JWT の USR ロール認証チェック（これのみ MYCUTE 側が担う）
/// 2. ヘッダーの hop-by-hop フィールド除去と BIFROST_AUTH_SECRET の注入
/// 3. リクエスト・レスポンスのストリーム透過転送
///
/// # ルーティング
/// axum の `Router::route("/lmgw/*proxy_path", any(...))` によって登録され、
/// `/lmgw/` 以降の全てのパス（メソッドも GET/POST/PUT/DELETE 等すべて）を受け付ける。
/// 具体的なパス定義（/lmgw/config 等）が存在した場合は axum が自動的に優先するが、
/// 現在は管理 API も含めて全て Bifrost に透過転送するため競合するパスは存在しない。
#[utoipa::path(
    tag = TAG,
    post,
    security(("api_jwt_token" = [])),
    path = "/v1/lmgw/{proxy_path}", // 実機に合わせて /v1 を追加
    summary = "Bifrost 透過プロキシ（全エンドポイント対応）",
    description = PROXY_LMGW_DESC,
    responses(
        (status = 200, description = "Bifrost からのレスポンスをそのまま返します（形式は Bifrost 公式ドキュメントを参照）"),
        (status = 401, description = "Unauthorized - MYCUTE JWT 認証失敗", body = ApiError),
        (status = 502, description = "Bad Gateway - Bifrost への接続失敗", body = ApiError),
    )
)]
pub async fn proxy_lmgw(
    ju: JwtUsr,
    method: Method,
    Path(proxy_path): Path<String>,
    headers: HeaderMap,
    Extension(hc): Extension<Arc<reqwest::Client>>,
    Extension(config_manager): Extension<Arc<ConfigManager>>,
    body: Body,
) -> Result<impl IntoResponse, ApiError> {
    // USR ロール以外のアクセスを最上部で遮断する（セキュリティの最優先事項）
    ju.allow_roles(&[JwtRole::USR])?;
    log::debug!(
        "<LMGW> proxy request received. method: {}, path: {}",
        method,
        proxy_path
    );

    let client = lmgws_bl::BifrostClient::new(hc, config_manager);
    let response = client
        .proxy_lmgw_request(method, &proxy_path, headers, body)
        .await?;
    Ok(response)
}

const GET_PROVIDERS_DESC: &str = r#"
### LMGWプロバイダー設定の取得
MYCUTE DBに保存されているプロバイダー設定（APIキー含む）を取得します。
APIキーは暗号化された状態で返却されます。
"#;

#[utoipa::path(
    tag = TAG,
    get,
    security(("api_jwt_token" = [])),
    path = "/lmgw/manage/providers",
    summary = "LMGWプロバイダー設定の取得",
    description = GET_PROVIDERS_DESC,
    responses(
        (status = 200, description = "Success", body = GetLmgwProvidersRes),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError),
    )
)]
pub async fn get_lmgw_providers(
    ju: JwtUsr,
    ids: JwtIDs,
    Extension(db): Extension<Arc<DbPools>>,
) -> Result<Json<GetLmgwProvidersRes>, ApiError> {
    ju.allow_roles(&[JwtRole::USR])?;
    let conn = db.get_ro_for_rt()?;
    let providers = lmgws_bl::get_lmgw_providers(conn, ids.apx_id, ids.vdr_id).await?;
    Ok(Json(providers))
}

const SAVE_PROVIDERS_DESC: &str = r#"
### LMGWプロバイダー設定の保存とBifrostへの同期
MYCUTE DBへ設定を保存し、同時にBifrostへの反映を行います。
新規のAPIキーは平文で送信し、バックエンドで暗号化して保存します。
既存のAPIキー（暗号化済み）はそのまま送信してください。
"#;

#[utoipa::path(
    tag = TAG,
    post,
    security(("api_jwt_token" = [])),
    path = "/lmgw/manage/providers",
    summary = "LMGWプロバイダー設定の保存と同期",
    description = SAVE_PROVIDERS_DESC,
    request_body = SaveLmgwProvidersReq,
    responses(
        (status = 200, description = "Success", body = SaveLmgwProvidersRes),
        (status = 400, description = "Bad Request", body = ApiError),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError),
        (status = 502, description = "Bad Gateway (Bifrost Sync Failed)", body = ApiError),
    )
)]
pub async fn save_lmgw_providers(
    ju: JwtUsr,
    ids: JwtIDs,
    Extension(db): Extension<Arc<DbPools>>,
    Extension(hc): Extension<Arc<reqwest::Client>>,
    Extension(config_manager): Extension<Arc<ConfigManager>>,
    Json(req): Json<SaveLmgwProvidersReq>,
) -> Result<Json<SaveLmgwProvidersRes>, ApiError> {
    ju.allow_roles(&[JwtRole::USR])?;
    req.validate().map_err(|e| {
        ApiError::new_system(axum::http::StatusCode::BAD_REQUEST, rterr::ERR_VALIDATION, e.to_string())
    })?;

    let conn = db.get_rw_for_rt()?;
    lmgws_bl::save_lmgw_providers(conn, ids.apx_id, ids.vdr_id, req, hc, config_manager).await?;
    Ok(Json(SaveLmgwProvidersRes { success: true }))
}
