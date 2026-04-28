use crate::{
    mode::rt::{
        rtbl::lmgws_bl,
        rterr::rterr,
        rtreq::lmgws_req::SaveLmgwProvidersReq,
        rtres::{
            errs_res::ApiError,
            lmgws_res::{GetLmgwProvidersRes, SaveLmgwProvidersRes, DeleteLmgwProviderRes},
        },
        rtutils::db_for_rt::DbPoolsExt,
    },
    mycute_settings::ConfigManager,
    types::{EventKind, InternalEvent},
    utils::{
        db::DbPools,
        jwt::{JwtIDs, JwtRole, JwtUsr},
        time,
    },
};
use axum::{
    body::Body,
    extract::Path,
    http::{HeaderMap, Method, StatusCode},
    response::IntoResponse,
    Extension, Json,
};
use garde::Validate;
use std::sync::Arc;

const TAG: &str = "v1 LMGW";

const PROXY_LMGW_DESC: &str = r#"
### Bifrost 透過プロキシエンドポイント

本エンドポイントは、Bifrost が提供する API（推論、参照など）を透過的に中継します。
MYCUTE 側で認証・認可を行った後、リクエストを Bifrost へ転送し、レスポンス（SSE ストリーム含む）をそのままクライアントへ返却します。

---

### ⚠️ 設定変更に関する重要な制限

**DB 整合性およびリアルタイム同期の維持のため、本プロキシ経由での設定変更（POST/DELETE）は制限されています。**

- **拒否される操作**: `POST /v1/lmgw/api/providers` など（403 Forbidden）
- **理由**: プロキシ経由で直接 Bifrost を書き換えると、MYCUTE のデータベースとの間に不整合が生じ、GUI への自動反映や設定の永続化が正常に機能しなくなるためです。

---

### ✅ 推奨される設定変更方法（専用管理 API）

プロバイダーや API キーの追加・変更には、必ず以下の **MYCUTE 専用管理 API** を使用してください。この API を使用すると、DB 更新と同時に全クライアントへのリアルタイム同期（イベント発火）が自動的に行われます。

- **エンドポイント**: `POST /v1/lmgw/manage/providers`
- **認証**: `Authorization: Bearer [JWT]`

#### リクエストボディの構造
MYCUTE の管理 API は、Bifrost ネイティブの形式ではなく、以下の構造を期待します。
特に、API キー（平文）を登録する際は `"is_new": true` を含めることが必須です。

```json
{
  "providers": [
    {
      "provider_name": "openai",
      "config_json": "{\"keys\":[{\"name\":\"my-key\",\"value\":\"sk-...\",\"weight\":1.0,\"is_new\":true}]}"
    }
  ]
}
```

#### 具体的な curl 実行例
```bash
curl -i -X POST http://localhost:3910/v1/lmgw/manage/providers \
  -H "Authorization: Bearer [JWT_TOKEN]" \
  -H "Content-Type: application/json" \
  -d '{
    "providers": [
      {
        "provider_name": "openai",
        "config_json": "{\"keys\":[{\"name\":\"openai-1\",\"value\":\"sk-proj-...\",\"weight\":1.0,\"is_new\":true}]}"
      }
    ]
  }'
```

#### プロバイダーの削除
プロバイダーを削除する場合は、以下の専用エンドポイントを使用します。
```bash
curl -i -X DELETE http://localhost:3910/v1/lmgw/manage/providers/openai \
  -H "Authorization: Bearer [JWT_TOKEN]"
```

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
| `POST /api/providers` | `POST /v1/lmgw/api/providers` (※利用不可・403 Forbidden) |
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

    let normalized_path = proxy_path.trim_matches('/');
    if method == Method::POST && normalized_path == "api/providers" {
        log::warn!(
            "<LMGW> Blocked direct provider registration to maintain DB consistency: {} {}",
            method,
            proxy_path
        );
        return Err(ApiError::new_system(
            StatusCode::FORBIDDEN,
            rterr::ERR_UNEXPECTED,
            "Direct provider management via transparent proxy is restricted to maintain database consistency. Please use MYCUTE dedicated management API instead.".to_string()
        ));
    }

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
    Extension(event_tx): Extension<
        Arc<tokio::sync::broadcast::Sender<InternalEvent>>,
    >,
    Json(req): Json<SaveLmgwProvidersReq>,
) -> Result<Json<SaveLmgwProvidersRes>, ApiError> {
    ju.allow_roles(&[JwtRole::USR])?;
    req.validate().map_err(|e| {
        ApiError::new_system(
            StatusCode::BAD_REQUEST,
            rterr::ERR_VALIDATION,
            e.to_string(),
        )
    })?;

    let conn = db.get_rw_for_rt()?;
    lmgws_bl::save_lmgw_providers(conn, ids.apx_id, ids.vdr_id, req, hc, config_manager).await?;

    // 保存完了後の最新の状態を再取得してイベントで飛ばす
    let current_providers = lmgws_bl::get_lmgw_providers(conn, ids.apx_id, ids.vdr_id).await?;

    let _ = event_tx.send(InternalEvent {
        seq: time::now_ts_ms(),
        kind: EventKind::LmgwProvidersChanged(current_providers.providers),
    });

    Ok(Json(SaveLmgwProvidersRes { success: true }))
}

const DELETE_PROVIDERS_DESC: &str = r#"
### LMGW プロバイダーの削除

指定したプロバイダーを MYCUTE データベースおよび Bifrost から完全に削除します。
削除成功時には全クライアントへリアルタイム同期イベント（`LmgwProvidersChanged`）が発火します。

- プロキシ経由の `DELETE /api/providers/{provider_name}` は 403 でブロックされるため、必ず本 API を利用してください。
"#;

#[utoipa::path(
    tag = TAG,
    delete,
    security(("api_jwt_token" = [])),
    path = "/lmgw/manage/providers/{provider_name}",
    summary = "LMGW プロバイダーの削除（DB/Bifrost同期）",
    description = DELETE_PROVIDERS_DESC,
    params(
        ("provider_name" = String, Path, description = "削除対象のプロバイダー名（例: openai）"),
    ),
    responses(
        (status = 200, description = "Success", body = DeleteLmgwProviderRes),
        (status = 401, description = "Unauthorized", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError),
    )
)]
pub async fn delete_lmgw_provider(
    ju: JwtUsr,
    ids: JwtIDs,
    Path(provider_name): Path<String>,
    Extension(db): Extension<Arc<DbPools>>,
    Extension(hc): Extension<Arc<reqwest::Client>>,
    Extension(config_manager): Extension<Arc<ConfigManager>>,
    Extension(event_tx): Extension<
        Arc<tokio::sync::broadcast::Sender<InternalEvent>>,
    >,
) -> Result<Json<DeleteLmgwProviderRes>, ApiError> {
    ju.allow_roles(&[JwtRole::USR])?;

    let conn = db.get_rw_for_rt()?;
    lmgws_bl::delete_lmgw_provider(conn, ids.apx_id, ids.vdr_id, &provider_name, hc, config_manager).await?;

    // 削除完了後の最新の状態を再取得してイベントで飛ばす
    let current_providers = lmgws_bl::get_lmgw_providers(conn, ids.apx_id, ids.vdr_id).await?;

    let _ = event_tx.send(InternalEvent {
        seq: time::now_ts_ms(),
        kind: EventKind::LmgwProvidersChanged(current_providers.providers),
    });

    Ok(Json(DeleteLmgwProviderRes { success: true }))
}
