use crate::{
    mode::rt::{rtbl::lmgws_bl::BifrostClient, rtres::errs_res::ApiError},
    mycute_settings::ConfigManager,
    utils::jwt::{JwtRole, JwtUsr},
};
use axum::{
    body::Body,
    extract::Path,
    http::{HeaderMap, Method},
    response::IntoResponse,
    Extension,
};
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
| `GET /v1/providers` | `GET /v1/lmgw/v1/providers` |
| `POST /v1/providers` | `POST /v1/lmgw/v1/providers` |
| `GET /v1/providers/{name}` | `GET /v1/lmgw/v1/providers/{name}` |
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

    let client = BifrostClient::new(hc, config_manager);
    let response = client
        .proxy_lmgw_request(method, &proxy_path, headers, body)
        .await?;
    Ok(response)
}
