---
ticket_id: 2
title: M-2-1.5: Dual-Store 抽象トレイト階層の定義 (GraphStore / MetadataStore)
slug: m-2-15-dual-store-graphstore-metadatastore
status: reviewed
created_at: 2026-05-21
updated_at: 2026-05-21
plan_path: /Users/shyme01/shyme/mycute/crates/darvium/tickets/context/0002-m-2-15-dual-store-graphstore-metadatastore/plan.md
implementation_path: /Users/shyme01/shyme/mycute/crates/darvium/tickets/context/0002-m-2-15-dual-store-graphstore-metadatastore/implementation.md
review_report_path: /Users/shyme01/shyme/mycute/crates/darvium/tickets/context/0002-m-2-15-dual-store-graphstore-metadatastore/review.md
---

# M-2-1.5: Dual-Store 抽象トレイト階層の定義 (GraphStore / MetadataStore)

## Summary

全13フェーズはメモリ内完結だが、後段の実DB接続フェーズで SQLite / LadybugDB への差し替えを可能にするため、ストレージ操作を抽象化する2系統のトレイトを**全ストレージ利用コードに先立って**定義する。これにより各チケットの実装はトレイトに対するプログラミングとなり、テストは Mock 実装で完結、実DB接続フェーズでは具象実装（`SqliteMetadataStore` / `LadybugGraphStore`）を書くだけで置き換えが完了する。

## Background

Darvium RFC-0001 v2.0-final では、以下の2系統のデータストアが規定されている：

1. **LadybugDB** — ワークフローグラフ・埋め込みベクトル・知識オブジェクトの格納（グラフ構造 + ベクトル検索）
2. **SQLite** — メタデータ・信頼スコア・系列監査ログの永続化（リレーショナルデータ）

全13フェーズ（M-2 〜 M4）は**SQLite も LadybugDB も一切使用せず**、Rust の `Vec` / `HashMap` 等のメモリ内データ構造でエミュレーションされる。しかし、各チケットの実装が直接 `Vec` / `HashMap` を操作してしまうと、後から実DBに差し替える際に全コードの修正が発生する。

本チケットはこの問題を解決するため、以下の方針で実装する：

- **全ストレージ利用コードに先立って** トレイトを定義する（Fake-First の前提条件）
- トレイトに対するプログラミングにより、各チケットは具象ストレージの詳細から完全に独立する
- メモリ内実装（`InMemoryGraphStore` / `InMemoryMetadataStore`）をトレイトと同時に提供し、全13フェーズのテスト基盤とする
- 実DB接続フェーズでは、トレイトを実装する具象構造体（`SqliteMetadataStore` / `LadybugGraphStore`）を追加するだけで置き換えが完了する

### 関連RFCセクション

- §12.2 Stage 2a, 2b Dual Retrieval — 双ルート検索におけるストア責務
- §25 データベース構成 — LadybugDB / SQLite の責務分離
- §18.2 クロスストア書き込み規約 — 2相コミットの論理的一貫性
- §37 Birth Commit Discipline — 融合時のアトミック永続化

### 他チケットとの依存関係

- 本チケットは **M-2-1（RetrievalPrimitive）の完了を前提とする**（既存の `DarviumError` 型を使用するため）
- M-2-3（Mock クライアント）以降の全ストレージ利用チケットは本トレイトを利用する
- M-0.5-1（メモリ内デュアルストア候補抽出）は本トレイトの `InMemoryGraphStore` / `InMemoryMetadataStore` を使用する

## Scope

1. **`GraphStore` トレイトの定義**（LadybugDB 責務）:
   - `fn store_workflow_graph(&self, graph: &WorkflowGraph) -> Result<GraphId, DarviumError>`
   - `fn load_workflow_graph(&self, graph_id: &GraphId) -> Result<WorkflowGraph, DarviumError>`
   - `fn store_embedding(&self, key: &str, vector: &[f32]) -> Result<(), DarviumError>`
   - `fn semantic_search(&self, query: &[f32], top_k: u32) -> Result<Vec<(String, f64)>, DarviumError>`
   - `fn store_knowledge_object(&self, obj: &KnowledgeObject) -> Result<(), DarviumError>`
   - `fn load_knowledge_object(&self, object_id: &str) -> Result<KnowledgeObject, DarviumError>`
   - `fn store_relation(&self, relation: &RelationRecord) -> Result<(), DarviumError>`
   - `fn load_relations(&self, object_id: &str) -> Result<Vec<RelationRecord>, DarviumError>`
   - `fn record_origin_trace(&self, trace: &OriginTrace) -> Result<(), DarviumError>`
   - `fn load_origin_traces(&self, object_id: &str) -> Result<Vec<OriginTrace>, DarviumError>`

2. **`MetadataStore` トレイトの定義**（SQLite 責務）:
   - `fn store_search_trace(&self, trace: &SearchTrace) -> Result<(), DarviumError>`
   - `fn load_search_traces(&self, mission_id: &str) -> Result<Vec<SearchTrace>, DarviumError>`
   - `fn store_trust_audit_log(&self, log: &TrustAuditLog) -> Result<(), DarviumError>`
   - `fn load_trust_audit_logs(&self, target_id: &str) -> Result<Vec<TrustAuditLog>, DarviumError>`
   - `fn store_patch_history(&self, history: &PatchHistory) -> Result<(), DarviumError>`
   - `fn load_patch_histories(&self, graph_id: &str) -> Result<Vec<PatchHistory>, DarviumError>`
   - `fn store_training_metadata(&self, metadata: &TrainingMetadata) -> Result<(), DarviumError>`
   - `fn load_training_metadata(&self, mission_id: &str) -> Result<TrainingMetadata, DarviumError>`
   - `fn store_fusion_metadata(&self, metadata: &FusionMetadata) -> Result<(), DarviumError>`
   - `fn load_fusion_metadata(&self, pair_id: &str) -> Result<FusionMetadata, DarviumError>`

3. **`InMemoryGraphStore` の実装**:
   - `HashMap<GraphId, WorkflowGraph>` によるグラフ格納
   - `HashMap<String, Vec<f32>>` による埋め込みベクトル格納
   - `semantic_search`: 全ベクトルとのコサイン類似度計算による線形探索（O(n)）
   - `HashMap<String, KnowledgeObject>` による知識オブジェクト格納
   - `Vec<RelationRecord>` によるリレーション格納（object_id でフィルタ）
   - `Vec<OriginTrace>` による origin trace 格納（object_id でフィルタ）

4. **`InMemoryMetadataStore` の実装**:
   - `HashMap<String, Vec<SearchTrace>>` による search trace 格納
   - `HashMap<String, Vec<TrustAuditLog>>` による trust audit log 格納
   - `HashMap<String, Vec<PatchHistory>>` による patch history 格納
   - `HashMap<String, TrainingMetadata>` による training metadata 格納
   - `HashMap<String, FusionMetadata>` による fusion metadata 格納

5. **エラー型の拡充**:
   - `DarviumError::Storage(String)` — ストレージ操作の一般エラー
   - `DarviumError::NotFound(String)` — キー未存在エラー
   - `DarviumError::EmbeddingDimensionMismatch { expected: usize, actual: usize }` — ベクトル次元不一致

6. **補助データ型の定義**:
   - `GraphId` 型（既存の型エイリアスを利用または拡張）
   - `KnowledgeObject` / `RelationRecord` / `OriginTrace` の構造体定義（必要最小限）
   - `TrainingMetadata` / `FusionMetadata` の構造体定義（必要最小限）

## Non-scope

- **実データベース接続**（`SqliteMetadataStore` / `LadybugGraphStore`）— 全13フェーズ完了後の別フェーズ
- **トランザクション処理** — メモリ内実装では単一操作のアトミック性のみ保証。複数操作のトランザクションは実DBフェーズで導入
- **HNSW 近似近傍探索** — `semantic_search` は線形探索（O(n)）で実装。HNSW アルゴリズムは実 LadybugDB 結合フェーズで導入
- **`serde` シリアライゼーション** — メモリ内実装では不要。実DB接続時に必要に応じて追加
- **`KnowledgeObject` / `RelationRecord` の完全なフィールド定義** — 本チケットでは必要最小限のフィールドのみ定義。詳細は後続チケットで具体化
- **並行アクセス制御**（`Mutex` / `RwLock`）— メモリ内実装では単純な内部可変性のみ。本格的な並行制御は後続チケットで導入

## Investigation

### 現状確認（2026-05-21）

**ソースコード調査結果:**

1. **`src/types.rs`**:
   - `NodeId = usize`, `GraphVersion = u64`, `PairId = String`, `ActorId = String` — ID 型が既に定義済み
   - `WorkflowGraph = DiGraph<WorkflowNode, EdgeMeta>` — グラフ型が既に定義済み
   - `SearchTrace` / `TrustAuditLog` / `PatchHistory` — 空構造体として既に存在（410-416行目）
   - `TrainingMission` / `TrainingFeedback` — 空構造体として既に存在（424-427行目）
   - `RepositoryPair` / `FusionPlan` — 空構造体として既に存在（430-434行目）
   - **`KnowledgeObject` / `RelationRecord` / `OriginTrace` は未定義**
   - **`GraphStore` / `MetadataStore` トレイトは未定義**

2. **`src/error.rs`**:
   - `DarviumError::DualStoreCommit(String)` — 既に存在（66-67行目）
   - `DarviumError::DualStoreInconsistency(String)` — 既に存在（69-70行目）
   - **`DarviumError::Storage` / `DarviumError::NotFound` は未定義**

3. **`src/lib.rs`**:
   - `pub mod constants;`, `pub mod error;`, `pub mod types;` — 3モジュールのみ
   - **ストアモジュール（store 等）は未作成**

4. **Darvium-Tickets.md の記述（87-90行目）**:
   - エラー型: 既存の `DarviumError` を拡充（`Storage` / `NotFound` 等のバリアント追加）
   - `InMemoryGraphStore`: `HashMap` / `Vec` による全操作のメモリ内実装（高速・決定論的）
   - `InMemoryMetadataStore`: `HashMap` / `Vec` による全操作のメモリ内実装

### 設計判断

1. **トレイトの分割粒度**: `GraphStore` と `MetadataStore` の2系統に分け、LadybugDB 責務と SQLite 責務を明確に分離する。これにより実DB差し替え時に各ストアの実装を独立して開発できる。

2. **エラー型**: `DarviumError::Storage(String)` を追加してストレージ操作の一般エラーを表現。`DarviumError::NotFound(String)` で存在しないキーへのアクセスを表現。`EmbeddingDimensionMismatch` も同時に追加する。

3. **オブジェクト安全性**: 両トレイトは `Box<dyn GraphStore>` / `Box<dyn MetadataStore>` として使用可能な設計とする（メソッドにジェネリクスを含めない）。

4. **semantic_search の実装**: `InMemoryGraphStore` では全ベクトルとのコサイン類似度を計算する線形探索（O(n)）を採用。HNSW 等の近似アルゴリズムは実 LadybugDB 結合時に導入する。

## Test Plan

### コンパイル時検証

| # | テスト種別 | テスト内容 | 期待結果 |
|---|-----------|-----------|---------|
| 1 | 型コンパイル | `InMemoryGraphStore` が `GraphStore` トレイトを充足することのコンパイル時検証 | コンパイル成功 |
| 2 | 型コンパイル | `InMemoryMetadataStore` が `MetadataStore` トレイトを充足することのコンパイル時検証 | コンパイル成功 |
| 3 | トレイト境界 | `Box<dyn GraphStore>` のオブジェクト安全性確認 | コンパイル成功 |
| 4 | トレイト境界 | `Box<dyn MetadataStore>` のオブジェクト安全性確認 | コンパイル成功 |

### 正常系テスト

| # | テスト種別 | テスト内容 | 期待結果 |
|---|-----------|-----------|---------|
| 5 | CRUD | グラフの登録 → 読取 → 内容一致確認 | 登録したグラフが正確に読み出せる |
| 6 | CRUD | 埋め込みベクトルの登録 → semantic_search で同一ベクトルが最高類似度で見つかる | 同一ベクトルが類似度 1.0 で最上位に返る |
| 7 | CRUD | 知識オブジェクトの登録 → 読取 → 内容一致確認 | 登録したオブジェクトが正確に読み出せる |
| 8 | CRUD | リレーションの登録 → object_id 検索 → 該当リレーション取得 | 登録したリレーションが正確に検索できる |
| 9 | CRUD | origin trace の記録 → 読取 → 内容一致確認 | 記録したトレースが正確に読み出せる |
| 10 | CRUD | search trace の保存 → mission_id 検索 → 該当トレース取得 | 保存したトレースが正確に検索できる |
| 11 | CRUD | trust audit log の保存 → target_id 検索 → 該当ログ取得 | 保存した監査ログが正確に検索できる |
| 12 | CRUD | patch history の保存 → graph_id 検索 → 該当履歴取得 | 保存した履歴が正確に検索できる |
| 13 | CRUD | training metadata の保存 → 読取 → 内容一致確認 | 保存したメタデータが正確に読み出せる |
| 14 | CRUD | fusion metadata の保存 → 読取 → 内容一致確認 | 保存したメタデータが正確に読み出せる |
| 15 | 複数 | 3件のベクトル登録後、クエリに最も近いベクトルが top-1 で正しく返る | 類似度順位が正しい |
| 16 | 複数 | 同一 object_id に複数の origin trace を追加 → 全件取得できる | 保存件数と取得件数が一致 |

### 異常系テスト

| # | テスト種別 | テスト内容 | 期待結果 |
|---|-----------|-----------|---------|
| 17 | 異常系 | 存在しない graph_id の load → NotFound エラー | `Err(DarviumError::NotFound(...))` |
| 18 | 異常系 | 存在しない knowledge object の load → NotFound エラー | `Err(DarviumError::NotFound(...))` |
| 19 | 異常系 | 異なる次元数のベクトルで semantic_search → DimensionMismatch エラー | `Err(DarviumError::EmbeddingDimensionMismatch{...})` |
| 20 | 異常系 | 空のベクトルで semantic_search → 適切なエラー | `Err(DarviumError::Storage(...))` |

### テスト実装場所

- `GraphStore` トレイト定義 + `InMemoryGraphStore` → `src/store/graph_store.rs` 内の `#[cfg(test)] mod tests`
- `MetadataStore` トレイト定義 + `InMemoryMetadataStore` → `src/store/metadata_store.rs` 内の `#[cfg(test)] mod tests`
- 補助データ型 → `src/types.rs` に追加
- エラー型 → `src/error.rs` に追加

## Boy Scout Rule — 翻訳可能性計画

### 改善対象（既存コード）

| 箇所 | 問題 | 改善計画 |
|------|------|---------|
| `src/types.rs` L398-402 | `TrustProfile` / `Provenance` が空構造体で翻訳不可能 | 本チケットでは触らない（後続チケットで具体化）。ただし新規追加する `KnowledgeObject` / `RelationRecord` / `OriginTrace` は最小限のフィールドを持たせて翻訳可能にする |
| `src/types.rs` L405-416 | `WorkflowPatch` / `SearchTrace` / `TrustAuditLog` / `PatchHistory` が空構造体 | 本チケットでは触らない（後続チケットで具体化）。新規 `TrainingMetadata` / `FusionMetadata` は必要最小限のフィールドを持たせる |

### 遵守事項

- 関数名は動詞句（`store_workflow_graph`, `semantic_search` など）とする
- 変数名はドメイン概念を表す（`map` ではなく `graph_store_map` のような明確な命名）
- 一関数一責務を徹底する
- ハードコード値は全て `src/constants.rs` の名前付き定数を参照する
- エラーの握りつぶし（`unwrap()` / `expect()` の無断使用）を禁止する
- コメントは「なぜ」を説明し、「何を」はコード自身に語らせる

## Acceptance Criteria

- [ ] `GraphStore` トレイトが定義され、全必須メソッドが `Result` を返す
- [ ] `MetadataStore` トレイトが定義され、全必須メソッドが `Result` を返す
- [ ] `InMemoryGraphStore` が `GraphStore` を実装し、全CRUD操作が正常動作する
- [ ] `InMemoryMetadataStore` が `MetadataStore` を実装し、全CRUD操作が正常動作する
- [ ] `DarviumError::Storage`, `NotFound`, `EmbeddingDimensionMismatch` が追加されている
- [ ] 存在しないキーへのアクセスが `NotFound` エラーを返す
- [ ] 次元不一致のベクトル検索が `EmbeddingDimensionMismatch` エラーを返す
- [ ] 両トレイトがオブジェクト安全である（`Box<dyn ...>` で使用可能）
- [ ] 既存テスト（M-2-1 の全テスト）が通過している
- [ ] 翻訳可能性の検証が通っている（新規コードが命名規則・責務分離に従っている）

## Notes

<!--
注: このコメントは人間向けの説明である。AI は以下の手順に従うこと。

- plan_path: /plan-ticket が plan.md を作成後に frontmatter に更新する
- implementation_path: /start-ticket が implementation.md を作成後に frontmatter に更新する
- review_report_path: /review-ticket が review.md を作成後に frontmatter に更新する

各コマンドのワークフロー手順が frontmatter 更新の正しい手順である。
-->

### 成果物

- 計画: context/0002-m-2-15-dual-store-graphstore-metadatastore/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0002-m-2-15-dual-store-graphstore-metadatastore/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0002-m-2-15-dual-store-graphstore-metadatastore/review.md（未作成、/review-ticket 全チェック通過後に作成）
