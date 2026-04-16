# MYCUTE RT 統合実装計画: 内蔵 LLM Gateway + ZeroClaw 設定同期

本ドキュメントは、[INFO-LLM-GW-AND-ZEROCLAW.md](file:///Users/kawata/shyme/mycute/docs/INFO-LLM-GW-AND-ZEROCLAW.md) のコンセプトを現在の MYCUTE の実装（Rust/Axum/SeaORM）に適用するための具体的かつ詳細な設計図です。

## 1. データベース・層 (SeaORM / Migration)
物理的なスキーマ変更と、それに基づくエンティティ生成を定義します。

### 1-1. LlmProviders マイグレーション作成
[NEW] `src/migration/m20260415_180000_create_llm_providers_tbl.rs`

```rust
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.create_table(Table::create()
            .table(LlmProviders::Table)
            .if_not_exists()
            .col(ColumnDef::new(LlmProviders::Id).string().not_null().primary_key())
            .col(ColumnDef::new(LlmProviders::Kind).string().not_null())
            .col(ColumnDef::new(LlmProviders::BaseUrl).string())
            .col(ColumnDef::new(LlmProviders::DefaultModel).string().not_null())
            .col(ColumnDef::new(LlmProviders::IsEnabled).boolean().not_null().default(true))
            .to_owned()).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(LlmProviders::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
enum LlmProviders { Table, Id, Kind, BaseUrl, DefaultModel, IsEnabled }
```

### 1-2. LlmApiKeys マイグレーション作成
[NEW] `src/migration/m20260415_180001_create_llm_api_keys_tbl.rs`

```rust
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.create_table(Table::create()
            .table(LlmApiKeys::Table)
            .if_not_exists()
            .col(ColumnDef::new(LlmApiKeys::Id).integer().not_null().auto_increment().primary_key())
            .col(ColumnDef::new(LlmApiKeys::ProviderId).string().not_null())
            .col(ColumnDef::new(LlmApiKeys::ApiKey).string().not_null())
            .col(ColumnDef::new(LlmApiKeys::Label).string())
            .col(ColumnDef::new(LlmApiKeys::IsActive).boolean().not_null().default(true))
            .foreign_key(ForeignKey::create()
                .name("fk_llm_api_keys_provider")
                .from(LlmApiKeys::Table, LlmApiKeys::ProviderId)
                .to(LlmProviders::Table, LlmProviders::Id)
                .on_delete(ForeignKeyAction::Cascade))
            .to_owned()).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(LlmApiKeys::Table).to_owned()).await
    }
}

#[derive(DeriveIden)]
enum LlmProviders { Table, Id }
#[derive(DeriveIden)]
enum LlmApiKeys { Table, Id, ProviderId, ApiKey, Label, IsActive }
```

### 1-3. エンティティ同期
- コマンド: `make gen-entities` (内部で `sea-orm-cli generate entity` を実行)
- 補足: 自動的に `impl_utc_timestamp_behavior!` が付与されることを確認。

---

## 2. LLM Gateway エンジン (RT Core)

### 2-1. メモリ内設定構造体とマネージャー
[NEW] `src/zeroclaw/config_manager.rs`
- `ArcSwap` を使用したアトミックな設定更新を実現。

```rust
use std::sync::Arc;
use std::collections::HashMap;
use arc_swap::ArcSwap;
use std::sync::atomic::{AtomicUsize, Ordering};
use sea_orm::DatabaseConnection;

pub struct LlmConfig {
    pub providers: HashMap<String, ProviderEntry>,
}

pub struct ProviderEntry {
    pub kind: String,
    pub base_url: Option<String>,
    pub default_model: String,
    pub key_pool: Arc<KeyPool>,
}

pub struct KeyPool {
    keys: Vec<String>,
    counter: AtomicUsize,
}

impl KeyPool {
    pub fn get_next(&self) -> &str {
        if self.keys.is_empty() { return ""; }
        let idx = self.counter.fetch_add(1, Ordering::Relaxed) % self.keys.len();
        &self.keys[idx]
    }
}

pub struct LlmGatewayManager {
    conf: ArcSwap<LlmConfig>,
    db: Arc<DatabaseConnection>,
}

impl LlmGatewayManager {
    pub async fn reload(&self) -> anyhow::Result<()> {
        // DB から全プロバイダーとアクティブなキーを取得して LlmConfig を再構築
        // ArcSwap::store() で原子的入れ替え
        Ok(())
    }
}
```

---

## 3. RT (Axum) 統合

### 3-1. ゲートウェイ・ハンドラー
[NEW] `src/mode/rt/rthandler/llmgw_handler.rs`
- ハイブリッド方式:
    - `path == "v1/chat/completions"` 等の特定エンドポイントは OpenAI 形式のリクエストをデシリアライズし、`rig-core` を用いて実プロバイダー（Anthropic, Gemini 等）へ翻訳・転送。
    - それ以外のパスや未知のパスについては「透過プロキシ」としてパススルーし、互換性を維持。

### 3-2. ルート登録
[MODIFY] `src/mode/rt/req_map.rs`
- `/llmgw/:provider_id/*path` ワイルドカードルートを使用して、上記ハンドラーをバインド。

---

## 4. ZeroClaw プロセス管理の拡張

### 4-1. 動的な設定ファイル管理
[MODIFY] `src/zeroclaw/executor.rs`
- `ZeroClawManager` に `LlmGatewayManager` または RT がリッスンしているポート (`rt_port`) を渡し、起動直前に `config.toml` を書き出す。
- `config.toml` 内の `baseurl` には固定のポートではなく、MYCUTE RT が実際にリッスンしている URL を指定する（例: `http://127.0.0.1:{rt_port}/llmgw/v1` またはクライアントの挙動に合わせて `/llmgw`）。

---

## 5. 段階的実装フェーズ (詳細 60ステップ)

### Phase 1: データベース・スキーマとエンティティ (Step 1-13)
1. [x] `src/migration/m20260415_180000_create_llm_providers_tbl.rs` (Providersテーブル用) を作成。
2. [x] `src/migration/m20260415_180001_create_llm_api_keys_tbl.rs` (ApiKeysテーブル用) を作成。
3. [x] `LlmProviders` テーブルの物理構成定義（Id, Kind, BaseUrl, DefaultModel, IsEnabled）。
4. [x] `LlmApiKeys` テーブルの物理構成定義（Id, ProviderId, ApiKey, Label, IsActive）。
5. [x] `LlmApiKeys` から `LlmProviders` への外部キー制約設定。
6. [x] `src/migration/mod.rs` への 2 つの新規マイグレーション登録。
7. [x] 既存のDBに対してマイグレーション実行 (`make check-be` 等）。
8. [x] `make gen-entities` を実行してエンティティファイルを一括生成。
9. [x] `src/entities/llm_providers.rs` の生成および自動パッチ確認。
10. [x] `src/entities/llm_api_keys.rs` の生成および自動パッチ確認。
11. [x] `src/entities/prelude.rs` へのエンティティ登録確認。
12. [x] ユニットテストまたは直接SQLによるテーブル構造の妥当性確認。
13. [x] DB接続プールからのメタデータ取得確認。

### Phase 2: LLM Gateway 内部コア (Step 14-26)
14. [x] `Cargo.toml` に `rig-core`, `arc-swap`, `rand` を追加（`cargo add`）。
15. [x] `src/mode/rt/llmgw/manager.rs` (計画時の `src/zeroclaw/config_manager.rs`) を新規作成。
16. [x] `KeyPool` 構造体と `get_next` (AtomicUsize) メソッドの実装。
17. [x] `ProviderEntry` 構造体の実装（kind, base_url, default_model, key_pool）。
18. [x] `LlmConfig` 構造体の実装（HashMap によるプロバイダー管理）。
19. [x] `LlmGatewayManager` 構造体の実装（ArcSwap による保持）。
20. [x] `LlmGatewayManager::new()` の実装（空の状態での初期化）。
21. [x] `LlmGatewayManager::reload()` の骨組み実装。
22. [x] DB から `LlmProviders` を全件ロードするロジックの実装。
23. [x] ロードした各プロバイダーに対して関連する `LlmApiKeys` を JOIN して取得。
24. [x] 取得したデータを `LlmConfig` 形式に変換。
25. [x] `ArcSwap::store()` による原子的な設定更新の実装。
26. [x] ユニットテストに準ずるビルド確認 (`make check-be`)。

### Phase 3: Axum ハンドラーとリクエストモデル (Step 27-39)
27. [ ] OpenAI 互換リクエストモデル (`ChatRequest`) の定義。
28. [ ] OpenAI 互換レスポンスモデル (`ChatResponse`) の定義。
29. [ ] `rig-core` と連携するためのプロバイダー別実行ロジックの実装。
30. [ ] `ChatRequest` から `rig` へのメッセージ変換ユーティリティ作成。
31. [ ] ストリーミングレスポンス (Server-Sent Events) への対応検討。
32. [x] `src/mode/rt/rthandler/llmgw_handler.rs` を作成.
33. [x] `TAG` 定数 (`"v1 LlmGateway"`) と `DESC` 定数の定義。
34. [/] `proxy_handler` を拡張し、パスによる「翻訳」と「透過」の分岐ロジックを実装。
35. [x] 内部でのプロバイダー解決ロジック（パスパラメータから解決）の実装。
36. [ ] `rig-core` を用いて、特定パスに対するプロバイダー種別に応じた翻訳転送の実装。
37. [x] 非対応パスに対する透過プロキシ（フォールバック）の維持。
38. [x] デバッグログ出力 (`log::debug!`) の実装。
39. [x] `/llmgw` プレフィックスによる例外処理の不要化（ワイルドカードルーティング）。

### Phase 4: ルーティングと起動フローの統合 (Step 40-52)
40. [x] `src/mode/rt/req_map.rs` への `llm_gw_handler` のインポート追加。
41. [x] `/llmgw/:provider_id/*path` ルーティングの定義。
42. [x] ゲートウェイプロキシルートの登録。
43. [x] 共通プロキシによる Embeddings 等のサポート完了。
44. [x] ワイルドカードルーティングによる全パスのキャッチ。
45. [x] `src/mode/rt/main_of_rt.rs` で `LlmGatewayManager` をインスタンス化。
46. [x] 起動時に一度 `reload()` を呼び出して DB 状態を同期。
47. [x] `Extension(LlmGatewayManager)` として Axum Router にレイヤーを追加。
48. [x] `src/zeroclaw/executor.rs` の更新（ZeroClaw 起動ロジック追加）。
49. [x] `executor.rs` に `generate_config()` メソッドを実装。
50. [x] 起動直前の `spawn_agent()` 呼び出しの実装。
51. [x] 初期起動テストおよび `toml` クレートの追加完了。
52. [x] ログ出力の追加。

### Phase 5: 管理 API と UI 連携 (Step 53-62)
53. [ ] `src/mode/rt/rtbl/llm_gw_bl.rs` を作成し、プロバイダー管理ロジックを集約。
54. [ ] `find_llm_providers_base` 等の共通クエリの定義。
55. [ ] プロバイダーの追加・更新・削除 API ハンドラーの実装。
56. [ ] API キーの追加・削除 API ハンドラーの実装。
57. [ ] 各操作の完了後に `LlmGatewayManager::reload()` を自動実行して設定を即時反映。
58. [ ] 権限チェックを `JwtUsr::allow_roles(&[JwtRole::USR])` に変更し、実利用者が設定を管理できるようにガード。
59. [ ] フロントエンド（Tauri IPC）向けに `src-tauri/commands/` 配下に設定変更用コマンドを作成、または既存の仕組みに連結。
60. [ ] `zeroclaw status` 相当の情報を MYCUTE UI に表示するためのエンドポイント/IPC 実装。
61. [ ] 最終的なドキュメント修正と内部マニュアル（管理者向け）の整備。
62. [ ] 全体の回帰テストとパフォーマンス確認（Gateway 経由の遅延測定）。

---

## 注意点とリスク管理
- **循環参照**: `zeroclaw` モジュールと `rt` モジュール間での依存関係が複雑にならないよう、インターフェースを整理します。
- **デッドロック**: `ArcSwap` を使用することで読み取りパフォーマンスは最大化されますが、大量の同時書き込みが発生した場合の DB ロックに注意します。
- **後方互換性**: ZeroClaw が要求する OpenAI API のバージョンが変更された場合でも、Gateway 側で吸収できるように抽象化を維持します。
