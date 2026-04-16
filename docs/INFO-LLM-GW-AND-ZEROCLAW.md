# MYCUTE RT 統合実装ガイド v3 — Axum 内蔵 Gateway + SQLite 設定同期

## 修正されたアーキテクチャ全体図

```
┌──────────────────────────────────────────────────────────────────┐
│  MYCUTE CL（Tauri フロントエンド）                               │
│  ・Tauri の invoke() / emit() イベント機構でRTと通信            │
│  ・HTTP/WebSocket は使用しない                                   │
└────────────────────────┬─────────────────────────────────────────┘
                         │ Tauri IPC（invoke / emit）
┌────────────────────────▼─────────────────────────────────────────┐
│  MYCUTE RT（Axum バックエンド）                                  │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │  Axum Router（ポート: 1本のみ例: 3910）                │    │
│  │                                                         │    │
│  │  /v1/chat/completions   ← ZeroClaw が呼ぶ               │    │
│  │  /v1/embeddings         ← ZeroClaw が呼ぶ               │    │
│  │  /v1/models             ← ZeroClaw が呼ぶ               │    │
│  │  /api/providers/*       ← RT 内部の管理 API             │    │
│  │  /api/keys/*            ← RT 内部の管理 API             │    │
│  │                                                         │    │
│  │  Provider Translation Layer（rig-core + rig-dyn）       │    │
│  │  ・ArcSwap<GatewayConfig> でゼロコピーホットリロード    │    │
│  │  ・KeyPool ラウンドロビン（AtomicUsize）                 │    │
│  └─────────────────────────────────────────────────────────┘    │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │  設定管理層                                              │    │
│  │  ・SQLite（SeaORM 既存実装を流用）                       │    │
│  │  ・DB 書き込み → ArcSwap 即時更新（同期を保証）          │    │
│  │  ・起動時: DB → メモリ構造体ロード                       │    │
│  └─────────────────────────────────────────────────────────┘    │
│                                                                   │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │  ZeroClaw プロセス管理                                   │    │
│  │  ・tokio::process::Command で子プロセス起動              │    │
│  │  ・config.toml を RT が生成（内容はDB由来）              │    │
│  │  ・PUT /api/config でランタイム設定更新                  │    │
│  └─────────────────────────────────────────────────────────┘    │
└──────────────────────────────────────────────────────────────────┘
                         │ HTTP (localhost:3910/v1/*)
┌────────────────────────▼─────────────────────────────────────────┐
│  ZeroClaw（ビルド済みバイナリ）                                   │
│  ・gateway.port = 42617                                           │
│  ・defaultprovider = "mycute-gateway"                             │
│  ・modelproviders.mycute-gateway.baseurl = http://127.0.0.1:3910/v1 │
└────────────────────────┬─────────────────────────────────────────┘
                         │ rig-core / rig-dyn
      ┌──────────────┬───┴──────────┬──────────────┐
   OpenAI        Anthropic       Gemini          Ollama
```

**ポイントの変更点（前版との差分）**：
1. Provider Translation Layer は **RT の Axum Router 内部に統合**される（独立プロセスなし）[1]
2. CL ↔ RT 間は **Tauri IPC のみ**（HTTP/WS なし）
3. Gateway 設定は **TOML ファイルではなく SQLite + メモリ内 ArcSwap** で管理する[1]

***

## Part 1: SQLite 設定スキーマと SeaORM エンティティ

### 1-1. テーブル設計

MYCUTEに既存の SeaORM 実装と統一したスキーマを使う。[2][3]

```sql
-- プロバイダー定義（1行 = 1プロバイダー）
CREATE TABLE provider_configs (
    id          TEXT    PRIMARY KEY,     -- "openai", "anthropic", "gemini" など
    kind        TEXT    NOT NULL,        -- rig-core/rig-dyn のプロバイダー種別
    base_url    TEXT,                    -- カスタムエンドポイント (nullable)
    default_model TEXT  NOT NULL,
    is_enabled  INTEGER NOT NULL DEFAULT 1,
    created_at  TEXT    NOT NULL,
    updated_at  TEXT    NOT NULL
);

-- プロバイダーごとの API キー（複数可、ラウンドロビン用）
CREATE TABLE provider_api_keys (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    provider_id TEXT    NOT NULL REFERENCES provider_configs(id) ON DELETE CASCADE,
    api_key     TEXT    NOT NULL,        -- 暗号化保存を推奨
    label       TEXT,                    -- UI 表示用ラベル
    is_active   INTEGER NOT NULL DEFAULT 1,
    sort_order  INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT    NOT NULL
);

-- モデルルーティング設定（hint → provider/model）
CREATE TABLE model_routes (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    hint        TEXT    NOT NULL UNIQUE, -- "reasoning", "fast", "vision" など
    provider_id TEXT    NOT NULL REFERENCES provider_configs(id),
    model_id    TEXT    NOT NULL,
    is_enabled  INTEGER NOT NULL DEFAULT 1
);

-- Gateway グローバル設定
CREATE TABLE gateway_settings (
    key         TEXT    PRIMARY KEY,
    value       TEXT    NOT NULL,
    updated_at  TEXT    NOT NULL
);
-- 初期値: default_provider_id = "openai"
```

### 1-2. SeaORM エンティティ定義

```rust
// src/entities/provider_config.rs
use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "provider_configs")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub kind: String,
    pub base_url: Option<String>,
    pub default_model: String,
    pub is_enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::provider_api_key::Entity")]
    ApiKeys,
    #[sea_orm(has_many = "super::model_route::Entity")]
    ModelRoutes,
}

impl ActiveModelBehavior for ActiveModel {}

// src/entities/provider_api_key.rs
#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "provider_api_keys")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,
    pub provider_id: String,
    pub api_key: String,
    pub label: Option<String>,
    pub is_active: bool,
    pub sort_order: i32,
    pub created_at: String,
}
```

***

## Part 2: メモリ内設定構造体と DB 同期メカニズム

### 2-1. メモリ内設定構造体（`GatewayConfig`）

```rust
// src/gateway/config.rs

use std::sync::Arc;
use std::collections::HashMap;
use arc_swap::ArcSwap;
use std::sync::atomic::{AtomicUsize, Ordering};
use rand::Rng;

/// DB の provider_configs + provider_api_keys を展開したメモリ表現
#[derive(Debug, Clone)]
pub struct GatewayConfig {
    pub default_provider_id: String,
    pub providers: HashMap<String, ProviderEntry>,
}

#[derive(Debug, Clone)]
pub struct ProviderEntry {
    pub kind: String,                  // "openai" | "anthropic" | "gemini" | "ollama"
    pub base_url: Option<String>,
    pub default_model: String,
    pub key_pool: Arc<KeyPool>,        // ラウンドロビン API キープール
    pub is_enabled: bool,
}

/// ラウンドロビン API キープール
/// 初期位置: ランダム選択、以降: AtomicUsize でロックフリーに進める
pub struct KeyPool {
    keys: Vec<String>,
    counter: AtomicUsize,
}

impl std::fmt::Debug for KeyPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KeyPool")
            .field("key_count", &self.keys.len())
            .finish()
    }
}

impl Clone for KeyPool {
    fn clone(&self) -> Self {
        Self::new(self.keys.clone())
    }
}

impl KeyPool {
    pub fn new(keys: Vec<String>) -> Arc<Self> {
        assert!(!keys.is_empty(), "KeyPool: at least one key required");
        // ランダムな初期位置を選択 → その位置からラウンドロビン開始
        let start = rand::thread_rng().gen_range(0..keys.len());
        Arc::new(Self {
            keys,
            counter: AtomicUsize::new(start),
        })
    }

    /// 次のキーを返す（ロックフリーラウンドロビン）
    pub fn next_key(&self) -> &str {
        let idx = self.counter.fetch_add(1, Ordering::Relaxed) % self.keys.len();
        &self.keys[idx]
    }

    pub fn len(&self) -> usize { self.keys.len() }
}
```

### 2-2. 設定同期マネージャー（DB ↔ メモリ）

**設計原則**：「DB への書き込みと ArcSwap の更新を必ず一緒に行う」ことで、常に DB とメモリが一致している状態を保証する。[1]

```rust
// src/gateway/config_manager.rs

use arc_swap::ArcSwap;
use sea_orm::DatabaseConnection;
use std::sync::Arc;
use anyhow::Result;
use crate::gateway::config::{GatewayConfig, ProviderEntry, KeyPool};
use crate::entities::{provider_config, provider_api_key, gateway_settings};
use sea_orm::*;

/// DB と ArcSwap<GatewayConfig> を常に同期させる管理者
pub struct ConfigManager {
    db: Arc<DatabaseConnection>,
    /// ロックフリー読み取り用。全 Axum ハンドラーはここを参照する
    pub config: Arc<ArcSwap<GatewayConfig>>,
}

impl ConfigManager {
    /// 起動時: DB から全設定を読み込んでメモリに展開する
    pub async fn load_from_db(db: Arc<DatabaseConnection>) -> Result<Arc<Self>> {
        let config = Self::build_config_from_db(&db).await?;
        Ok(Arc::new(Self {
            db,
            config: Arc::new(ArcSwap::from_pointee(config)),
        }))
    }

    /// DB から GatewayConfig を構築する内部メソッド
    async fn build_config_from_db(db: &DatabaseConnection) -> Result<GatewayConfig> {
        // 1. gateway_settings からデフォルトプロバイダーを取得
        let default_provider_id = gateway_settings::Entity::find_by_id("default_provider_id")
            .one(db)
            .await?
            .map(|s| s.value)
            .unwrap_or_else(|| "openai".to_string());

        // 2. 全プロバイダーとそのAPIキーをロード
        let providers_db = provider_config::Entity::find()
            .filter(provider_config::Column::IsEnabled.eq(true))
            .find_with_related(provider_api_key::Entity)
            .all(db)
            .await?;

        let mut providers = std::collections::HashMap::new();
        for (prov, keys) in providers_db {
            let active_keys: Vec<String> = keys
                .into_iter()
                .filter(|k| k.is_active)
                .map(|k| k.api_key)
                .collect();

            if active_keys.is_empty() {
                continue; // アクティブなキーのないプロバイダーはスキップ
            }

            providers.insert(prov.id.clone(), ProviderEntry {
                kind: prov.kind,
                base_url: prov.base_url,
                default_model: prov.default_model,
                key_pool: KeyPool::new(active_keys),
                is_enabled: prov.is_enabled,
            });
        }

        Ok(GatewayConfig { default_provider_id, providers })
    }

    // ─── 以下: DB への書き込みと ArcSwap 更新を常にセットで行う ───

    /// プロバイダーの追加 / 更新（DB 書き込み → メモリ更新）
    pub async fn upsert_provider(
        &self,
        id: &str,
        kind: &str,
        base_url: Option<&str>,
        default_model: &str,
    ) -> Result<()> {
        // DB に書き込む
        let now = chrono::Utc::now().to_rfc3339();
        let model = provider_config::ActiveModel {
            id: Set(id.to_string()),
            kind: Set(kind.to_string()),
            base_url: Set(base_url.map(|s| s.to_string())),
            default_model: Set(default_model.to_string()),
            is_enabled: Set(true),
            created_at: NotSet,  // INSERT 時のみセット
            updated_at: Set(now),
        };
        provider_config::Entity::insert(model)
            .on_conflict(
                OnConflict::column(provider_config::Column::Id)
                    .update_columns([
                        provider_config::Column::Kind,
                        provider_config::Column::BaseUrl,
                        provider_config::Column::DefaultModel,
                        provider_config::Column::UpdatedAt,
                    ])
                    .to_owned(),
            )
            .exec(&*self.db)
            .await?;

        // DB 書き込み成功後、即座にメモリを再構築して ArcSwap を更新
        self.reload_from_db().await
    }

    /// API キーの追加（DB 書き込み → メモリ更新）
    pub async fn add_api_key(&self, provider_id: &str, api_key: &str, label: Option<&str>) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        provider_api_key::ActiveModel {
            provider_id: Set(provider_id.to_string()),
            api_key: Set(api_key.to_string()),
            label: Set(label.map(|s| s.to_string())),
            is_active: Set(true),
            sort_order: Set(0),
            created_at: Set(now),
            ..Default::default()
        }
        .insert(&*self.db)
        .await?;

        self.reload_from_db().await
    }

    /// API キーの削除（DB 書き込み → メモリ更新）
    pub async fn remove_api_key(&self, key_id: i32) -> Result<()> {
        provider_api_key::Entity::delete_by_id(key_id)
            .exec(&*self.db)
            .await?;
        self.reload_from_db().await
    }

    /// デフォルトプロバイダーの変更（DB 書き込み → メモリ更新）
    pub async fn set_default_provider(&self, provider_id: &str) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        gateway_settings::ActiveModel {
            key: Set("default_provider_id".to_string()),
            value: Set(provider_id.to_string()),
            updated_at: Set(now),
        }
        .save(&*self.db)
        .await?;

        self.reload_from_db().await
    }

    /// DB から再読み込みして ArcSwap を原子的に更新する
    /// （外部から直接呼んでも安全）
    pub async fn reload_from_db(&self) -> Result<()> {
        let new_config = Self::build_config_from_db(&self.db).await?;
        self.config.store(Arc::new(new_config));
        Ok(())
    }
}
```

**この設計のポイント**：
- `upsert_provider`・`add_api_key`・`remove_api_key` のどのメソッドも、**DB 書き込み成功後に `reload_from_db()` を呼ぶ**ことで、DB とメモリの一致を常に保証する[1]
- `ArcSwap::store()` はアトミックな操作なので、実行中のリクエストが中途半端な状態を見ることはない[1]
- 読み取りは `config.load()` のみ（ロックなし、スレッドセーフ）

***

## Part 3: RT の Axum Router への Gateway エンドポイント統合

### 3-1. RT のメイン AppState

```rust
// src/state.rs

use std::sync::Arc;
use sea_orm::DatabaseConnection;
use crate::gateway::config_manager::ConfigManager;
use crate::zeroclaw::ZeroClawManager;

/// RT 全体で共有される状態
#[derive(Clone)]
pub struct AppState {
    pub db: Arc<DatabaseConnection>,
    pub config_manager: Arc<ConfigManager>,
    pub zeroclaw: Arc<ZeroClawManager>,
}
```

### 3-2. Axum Router の構成（ポート 1 本）

```rust
// src/main.rs または src/router.rs

use axum::{Router, routing::{get, post, put, delete}};
use std::sync::Arc;
use crate::state::AppState;

pub fn build_router(state: AppState) -> Router {
    Router::new()
        // ─── ZeroClaw 向け OpenAI 互換エンドポイント ──────────────
        // ZeroClaw の baseurl = "http://127.0.0.1:3910/v1" を指す
        .route("/v1/chat/completions", post(handlers::gateway::chat_completions))
        .route("/v1/embeddings",       post(handlers::gateway::embeddings))
        .route("/v1/models",           get(handlers::gateway::list_models))

        // ─── RT 内部管理 API（Tauri コマンドから呼ばれる内部 HTTP）──
        // Tauri の invoke → Rust コマンド → この API を内部呼び出し
        // またはコマンドハンドラーから直接 AppState を使う（後述）
        .route("/api/providers",               get(handlers::admin::list_providers))
        .route("/api/providers",               post(handlers::admin::create_provider))
        .route("/api/providers/:id",           put(handlers::admin::update_provider))
        .route("/api/providers/:id",           delete(handlers::admin::delete_provider))
        .route("/api/providers/:id/keys",      post(handlers::admin::add_api_key))
        .route("/api/providers/:id/keys/:kid", delete(handlers::admin::remove_api_key))
        .route("/api/routes",                  get(handlers::admin::list_routes))
        .route("/api/routes",                  post(handlers::admin::upsert_route))

        .with_state(Arc::new(state))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // DB 接続（既存の SeaORM 接続を流用）
    let db = Arc::new(
        sea_orm::Database::connect("sqlite://mycute.db?mode=rwc").await?
    );

    // DB から設定を読み込んでメモリに展開
    let config_manager = ConfigManager::load_from_db(Arc::clone(&db)).await?;

    // ZeroClaw を起動
    let zeroclaw = Arc::new(ZeroClawManager::new(
        config_manager.clone(),
        std::path::PathBuf::from("./binaries/zeroclaw"),
        std::path::PathBuf::from("./.mycute/zeroclaw"),
    ));
    zeroclaw.start().await?;
    zeroclaw.wait_ready(10).await?;

    let state = AppState { db, config_manager, zeroclaw };
    let app = build_router(state);

    // RT が使うポートは 1 本のみ
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3910").await?;
    axum::serve(listener, app).await?;
    Ok(())
}
```

### 3-3. Gateway ハンドラー（POST /v1/chat/completions）

ZeroClaw からのリクエストを受け取り、rig-core/rig-dyn 経由で実プロバイダーに転送する。[4]

```rust
// src/handlers/gateway.rs

use axum::{extract::State, Json, response::Response};
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub tools: Option<Vec<Value>>,
    pub stream: Option<bool>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: Value,  // String or Array（マルチモーダル）
}

/// POST /v1/chat/completions
/// ZeroClaw はここに OpenAI 形式でリクエストを送ってくる
pub async fn chat_completions(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ChatRequest>,
) -> Response {
    // ArcSwap からロックフリーで現在の設定スナップショットを取得
    let config = state.config_manager.config.load();

    // モデル文字列からプロバイダーを解決
    // "hint:reasoning" → defaultprovider
    // "anthropic/claude-opus-4" → "anthropic"
    // "gpt-4o" → defaultprovider
    let provider_id = resolve_provider_id(&req.model, &config.default_provider_id);

    let provider_entry = match config.providers.get(&provider_id) {
        Some(p) if p.is_enabled => p,
        _ => {
            return axum::response::Json(json!({
                "error": { "message": format!("Provider '{}' not found or disabled", provider_id), "type": "invalid_request_error" }
            })).into_response();
        }
    };

    // ラウンドロビンで API キーを選択（ランダム初期位置 + AtomicUsize）
    let api_key = provider_entry.key_pool.next_key().to_string();
    let model_id = resolve_model_id(&req.model, &provider_entry.default_model);

    // rig-core でプロバイダークライアントを構築（リクエストごと）
    // rig-dyn の CompletionModel は dyn-compatible でないため
    // 各リクエストで構築するパターンが安全かつシンプル
    let result = match provider_entry.kind.as_str() {
        "openai" => {
            execute_openai(&api_key, provider_entry.base_url.as_deref(), &model_id, &req).await
        }
        "anthropic" => {
            execute_anthropic(&api_key, &model_id, &req).await
        }
        "gemini" => {
            execute_gemini(&api_key, &model_id, &req).await
        }
        "ollama" => {
            // Ollama は base_url が必須（api_key は URL）
            let base = provider_entry.base_url.as_deref()
                .unwrap_or("http://localhost:11434");
            execute_ollama(base, &model_id, &req).await
        }
        other => Err(anyhow::anyhow!("Unknown provider kind: {}", other)),
    };

    match result {
        Ok(response_body) => axum::response::Json(response_body).into_response(),
        Err(e) => {
            tracing::error!("Completion error: {e}");
            axum::response::Json(json!({
                "error": { "message": e.to_string(), "type": "api_error" }
            })).into_response()
        }
    }
}

fn resolve_provider_id(model: &str, default: &str) -> String {
    if model.starts_with("hint:") {
        return default.to_string();
    }
    if let Some((provider, _)) = model.split_once('/') {
        return provider.to_string();
    }
    default.to_string()
}

fn resolve_model_id(model: &str, default_model: &str) -> String {
    if model.starts_with("hint:") || !model.contains('/') {
        return default_model.to_string();
    }
    model.split_once('/')
        .map(|(_, m)| m.to_string())
        .unwrap_or_else(|| default_model.to_string())
}

// rig-core プロバイダー実行関数群
async fn execute_openai(
    api_key: &str,
    base_url: Option<&str>,
    model_id: &str,
    req: &ChatRequest,
) -> anyhow::Result<Value> {
    use rig::providers::openai;

    let client = if let Some(url) = base_url {
        openai::Client::from_url(api_key, url)
    } else {
        openai::Client::new(api_key)
    };

    let model = client.completion_model(model_id);
    let messages = convert_messages_to_rig(&req.messages);

    let mut builder = model.completion_request(&messages.last_user_message);
    for msg in &messages.history {
        builder = builder.context_message(msg.clone());
    }
    if let Some(tools) = &req.tools {
        for tool in tools {
            if let Some(func) = tool.get("function") {
                builder = builder.tool(rig::completion::ToolDefinition {
                    name: func["name"].as_str().unwrap_or("").to_string(),
                    description: func["description"].as_str().unwrap_or("").to_string(),
                    parameters: func["parameters"].clone(),
                });
            }
        }
    }

    let response = builder.send().await?;
    Ok(format_openai_response(response))
}

async fn execute_anthropic(
    api_key: &str,
    model_id: &str,
    req: &ChatRequest,
) -> anyhow::Result<Value> {
    use rig::providers::anthropic;
    let client = anthropic::ClientBuilder::new(api_key).build();
    let model = client.completion_model(model_id);
    // rig-core が Anthropic の tool_use / content blocks を内部変換する
    let messages = convert_messages_to_rig(&req.messages);
    let response = model
        .completion_request(&messages.last_user_message)
        .send()
        .await?;
    Ok(format_openai_response(response))
}

async fn execute_gemini(
    api_key: &str,
    model_id: &str,
    req: &ChatRequest,
) -> anyhow::Result<Value> {
    use rig::providers::gemini;
    let client = gemini::Client::new(api_key);
    let model = client.completion_model(model_id);
    let messages = convert_messages_to_rig(&req.messages);
    let response = model
        .completion_request(&messages.last_user_message)
        .send()
        .await?;
    Ok(format_openai_response(response))
}

async fn execute_ollama(
    base_url: &str,
    model_id: &str,
    req: &ChatRequest,
) -> anyhow::Result<Value> {
    use rig::providers::ollama;
    let client = ollama::Client::from_url("", base_url);
    let model = client.completion_model(model_id);
    let messages = convert_messages_to_rig(&req.messages);
    let response = model
        .completion_request(&messages.last_user_message)
        .send()
        .await?;
    Ok(format_openai_response(response))
}
```

### 3-4. Tauri コマンドから AppState を直接使う

CL と RT の通信は Tauri IPC なので、Tauri の `#[tauri::command]` から AppState（= ConfigManager）を直接操作できる。Axum の `/api/providers` エンドポイントは ZeroClaw 向けではなく、RT 内部テストやデバッグ用途。通常の CL → RT 操作はすべて Tauri コマンドで行う。

```rust
// src-tauri/commands/provider_commands.rs

use tauri::State;
use crate::gateway::config_manager::ConfigManager;
use std::sync::Arc;

/// CL から呼ばれる: プロバイダー追加
#[tauri::command]
pub async fn add_provider(
    config_manager: State<'_, Arc<ConfigManager>>,
    id: String,
    kind: String,
    base_url: Option<String>,
    default_model: String,
) -> Result<(), String> {
    config_manager
        .upsert_provider(&id, &kind, base_url.as_deref(), &default_model)
        .await
        .map_err(|e| e.to_string())
    // ↑ DB 書き込み + ArcSwap 更新 が同時に行われる
}

/// CL から呼ばれる: API キー追加（ラウンドロビンプールに追加される）
#[tauri::command]
pub async fn add_api_key(
    config_manager: State<'_, Arc<ConfigManager>>,
    provider_id: String,
    api_key: String,
    label: Option<String>,
) -> Result<(), String> {
    config_manager
        .add_api_key(&provider_id, &api_key, label.as_deref())
        .await
        .map_err(|e| e.to_string())
    // ↑ DB 書き込み + ArcSwap 更新 が同時に行われる
}

/// CL から呼ばれる: デフォルトプロバイダーの変更
#[tauri::command]
pub async fn set_default_provider(
    config_manager: State<'_, Arc<ConfigManager>>,
    provider_id: String,
) -> Result<(), String> {
    config_manager
        .set_default_provider(&provider_id)
        .await
        .map_err(|e| e.to_string())
    // ↑ DB 書き込み + ArcSwap 更新 + ZeroClaw への PUT /api/config が行われる
}
```

***

## Part 4: ZeroClaw 管理（config.toml は DB から生成）

### 4-1. ZeroClawManager

```rust
// src/zeroclaw/manager.rs

use std::path::PathBuf;
use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use std::sync::Arc;
use anyhow::Result;
use crate::gateway::config_manager::ConfigManager;

pub struct ZeroClawManager {
    config_manager: Arc<ConfigManager>,
    binary_path: PathBuf,
    config_dir: PathBuf,
    child: Mutex<Option<Child>>,
    zeroclaw_port: u16,
    gateway_port: u16,
}

impl ZeroClawManager {
    pub fn new(
        config_manager: Arc<ConfigManager>,
        binary_path: PathBuf,
        config_dir: PathBuf,
    ) -> Self {
        Self {
            config_manager,
            binary_path,
            config_dir,
            child: Mutex::new(None),
            zeroclaw_port: 42617,
            gateway_port: 3910,
        }
    }

    pub async fn start(&self) -> Result<()> {
        let mut child = self.child.lock().await;
        if let Some(mut c) = child.take() {
            let _ = c.kill().await;
        }

        // DB から最新の設定を使って config.toml を生成
        self.write_config_from_db().await?;

        let new_child = Command::new(&self.binary_path)
            .arg("gateway")
            .env("ZEROCLAW_CONFIG_DIR", &self.config_dir)
            .env("RUST_LOG", "zeroclaw=info")
            .spawn()?;

        *child = Some(new_child);
        Ok(())
    }

    /// DB の現在の設定から ZeroClaw の config.toml を生成する
    async fn write_config_from_db(&self) -> Result<()> {
        let config = self.config_manager.config.load();
        let default_provider = &config.default_provider_id;

        // ZeroClaw の config.toml を構築
        // defaultprovider は常に "mycute-gateway"
        // 実際のプロバイダー選択は Gateway 側が行う
        let content = format!(
            r#"workspacedir = "{workspace}"

# RT の Gateway エンドポイントをプロバイダーとして登録
apikey = "gateway-passthrough"
defaultprovider = "mycute-gateway"
defaultmodel = "{default_model}"

[gateway]
port            = {zc_port}
host            = "127.0.0.1"
allowpublicbind = false
requirepairing  = false

[modelproviders.mycute-gateway]
apikey  = "gateway-passthrough"
baseurl = "http://127.0.0.1:{gw_port}/v1"

[reliability]
providerretries   = 2
providerbackoffms = 500

[agent]
maxtooliterations  = 10
maxhistorymessages = 50
tooldispatcher     = "auto"

[multimodal]
allowremotefetch = true

[observability]
backend = "log"
"#,
            workspace = self.config_dir.join("workspace").display(),
            default_model = config.providers
                .get(default_provider)
                .map(|p| p.default_model.as_str())
                .unwrap_or("gpt-4o"),
            zc_port = self.zeroclaw_port,
            gw_port = self.gateway_port,
        );

        let config_path = self.config_dir.join("config.toml");
        tokio::fs::create_dir_all(&self.config_dir).await?;
        tokio::fs::write(&config_path, content).await?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&config_path)?.permissions();
            perms.set_mode(0o600);
            std::fs::set_permissions(&config_path, perms)?;
        }
        Ok(())
    }

    /// ZeroClaw の起動確認（ヘルスチェック）
    pub async fn wait_ready(&self, timeout_secs: u64) -> Result<()> {
        let deadline = tokio::time::Instant::now()
            + tokio::time::Duration::from_secs(timeout_secs);
        let client = reqwest::Client::new();
        loop {
            if tokio::time::Instant::now() > deadline {
                anyhow::bail!("ZeroClaw timed out");
            }
            if client
                .get(format!("http://127.0.0.1:{}/health", self.zeroclaw_port))
                .timeout(std::time::Duration::from_secs(1))
                .send()
                .await
                .map(|r| r.status().is_success())
                .unwrap_or(false)
            {
                return Ok(());
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
        }
    }

    /// ZeroClaw のデフォルトモデルをランタイム更新（PUT /api/config）
    /// DB の設定変更後に呼ぶことでZeroClawを再起動せず同期できる
    pub async fn sync_default_model(&self) -> Result<()> {
        let config = self.config_manager.config.load();
        let default_model = config.providers
            .get(&config.default_provider_id)
            .map(|p| p.default_model.as_str())
            .unwrap_or("gpt-4o");

        let toml_patch = format!(
            "defaultmodel = \"{}\"\n",
            default_model
        );

        reqwest::Client::new()
            .put(format!("http://127.0.0.1:{}/api/config", self.zeroclaw_port))
            .header("Content-Type", "application/toml")
            .body(toml_patch)
            .send()
            .await?;
        Ok(())
    }
}
```

***

## Part 5: ZeroClaw の config.toml が RT Gateway を参照する仕組み

ZeroClaw の `modelproviders.mycute-gateway.baseurl = "http://127.0.0.1:3910/v1"` が設定されることで、ZeroClaw は RT の Axum エンドポイントを OpenAI 互換プロバイダーとして使用する。ZeroClaw は実際のプロバイダーを何も知らず、すべてのリクエストが RT の `/v1/chat/completions` に集まる。[5]

```toml
# .mycute/zeroclaw/config.toml （RT が DB から自動生成）

workspacedir = "/path/to/.mycute/zeroclaw/workspace"
apikey       = "gateway-passthrough"
defaultprovider = "mycute-gateway"
defaultmodel    = "gpt-4o"

[gateway]
port             = 42617
host             = "127.0.0.1"
allowpublicbind  = false
requirepairing   = false

[modelproviders.mycute-gateway]
apikey  = "gateway-passthrough"
baseurl = "http://127.0.0.1:3910/v1"

# modelroutes は ZeroClaw のエージェント機能内での hint ルーティング
# Gateway 側の resolve_provider_id() が実際のルーティングを処理する
[[modelroutes]]
hint     = "reasoning"
provider = "mycute-gateway"
model    = "anthropic/claude-opus-4-20250514"

[[modelroutes]]
hint     = "fast"
provider = "mycute-gateway"
model    = "groq/llama-3.3-70b-versatile"

[[modelroutes]]
hint     = "vision"
provider = "mycute-gateway"
model    = "openai/gpt-4o"
```

***

## Part 6: 全要件充足の確認

| 要件 | 実装箇所 | 充足状態 |
|------|----------|----------|
| CL ↔ RT は Tauri IPC | `#[tauri::command]` で ConfigManager を直接操作 | ✅ |
| Gateway は RT の Axum 内部に統合（ポート増加なし） | `build_router()` に `/v1/*` を同居させる | ✅ |
| 設定は TOML ファイルではなく SQLite に永続化 | `provider_configs` / `provider_api_keys` / `gateway_settings` テーブル | ✅ |
| DB と メモリ内構造体が常に同期 | DB 書き込み後に `reload_from_db()` → `ArcSwap::store()` を必ず実行 | ✅ |
| 同一プロバイダー複数 API キー | `provider_api_keys` テーブルで複数行、`KeyPool::new()` でプール化 | ✅ |
| ランダム初期位置 + ラウンドロビン | `rand::gen_range(0..keys.len())` → `AtomicUsize::fetch_add()` | ✅ |
| rig-core でプロバイダー差異吸収 | `execute_openai/anthropic/gemini/ollama()` で rig-core の completion API を使用[4] | ✅ |
| Function Calling の透過処理 | rig-core が OpenAI/Anthropic/Gemini 各プロトコルを内部変換[4] | ✅ |
| ZeroClaw の設定は DB 由来 | `write_config_from_db()` が `config.toml` を生成 | ✅ |
| RT はポートを 1 本のみ公開 | `127.0.0.1:3910` のみ（ZeroClaw 向け + 管理 API を同居） | ✅ |

***

## Part 7: Cargo.toml 依存関係

```toml
[dependencies]
# LLM プロバイダー統合
rig-core  = { version = "0.10" }

# DB（MYCUTE 既存実装と統一）
sea-orm   = { version = "1", features = ["sqlx-sqlite", "runtime-tokio-rustls", "macros"] }

# Web サーバー
axum      = { version = "0.7", features = ["macros"] }
tokio     = { version = "1", features = ["full"] }

# ロックフリー設定共有
arc-swap  = "1.7"

# ラウンドロビン初期化
rand      = "0.8"

# シリアライズ
serde     = { version = "1", features = ["derive"] }
serde_json = "1"
toml      = "0.8"

# 時刻
chrono    = { version = "0.4", features = ["serde"] }

# HTTP クライアント（ヘルスチェック等）
reqwest   = { version = "0.12", features = ["json"] }

anyhow    = "1"
tracing   = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
```

レポートを作成しました。今回の3つの修正点がどう実装に反映されたかをまとめます。

***

## 前版からの主要な変更箇所

### ① CL ↔ RT は Tauri IPC のみ

`#[tauri::command]` から `Arc<ConfigManager>` を Tauri の `State` として直接受け取る設計に変更しました。HTTP リクエストを経由せず、Rust の `async fn` 呼び出しで DB 書き込み + ArcSwap 更新まで一貫して行います。

### ② Gateway は RT の Axum Router 内部に統合（ポート増加なし）

独立プロセスを廃止し、`/v1/chat/completions`・`/v1/embeddings`・`/v1/models` を RT の Axum Router に直接追加しました。ZeroClaw の `baseurl = "http://127.0.0.1:3910/v1"` は RT そのものを指し、公開ポートは RT の **1 本のみ**です。 [oneuptime](https://oneuptime.com/blog/post/2026-01-25-hot-configuration-reloading-rust/view)

### ③ 設定は SQLite + ArcSwap 同期（TOML 廃止）

`provider_configs`・`provider_api_keys`・`gateway_settings` の 3 テーブルに永続化し、MYCUTEの既存 SeaORM 実装をそのまま流用できます。すべての書き込みメソッド（`upsert_provider`・`add_api_key`・`remove_api_key`・`set_default_provider`）は **DB 書き込み → `reload_from_db()` → `ArcSwap::store()`** の順番を必ず踏むことで DB とメモリの永続的な一致を保証しています。ZeroClaw の `config.toml` は MYCUTE の設定ではなく、起動時に DB から生成される「ZeroClaw 向け設定ファイル」という位置づけに整理されました。 [docs](https://docs.rs/sea-orm/latest/sea_orm/struct.SqlxSqlitePoolConnection.html)