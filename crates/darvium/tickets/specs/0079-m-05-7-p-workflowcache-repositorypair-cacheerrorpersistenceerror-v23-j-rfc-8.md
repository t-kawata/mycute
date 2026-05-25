---
ticket_id: 79
title: M-0.5-7-P: WorkflowCache + RepositoryPair + CacheError/PersistenceError 型定義基盤（v2.3-j RFC §8 追従）
slug: m-05-7-p-workflowcache-repositorypair-cacheerrorpersistenceerror-v23-j-rfc-8
status: reviewed
created_at: 2026-05-25
updated_at: 2026-05-25
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0079-m-05-7-p-workflowcache-repositorypair-cacheerrorpersistenceerror-v23-j-rfc-8/observation-20260525-140351.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0079-m-05-7-p-workflowcache-repositorypair-cacheerrorpersistenceerror-v23-j-rfc-8/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0079-m-05-7-p-workflowcache-repositorypair-cacheerrorpersistenceerror-v23-j-rfc-8/review.md
---

# M-0.5-7-P: WorkflowCache + RepositoryPair + CacheError/PersistenceError 型定義基盤（v2.3-j RFC §8 追従）

## Summary

v2.3-j の用語是正により `WorkflowRepository` は `WorkflowCache`（runtime cache）と `RepositoryPair`（永続化ペア）へ分割された。本チケットはこの新しい型体系（WorkflowCache / CachePolicy / AnnHotIndex / RepositoryPair / CacheError / PersistenceError）をコード上に定義し、crate 公開 API として整備する。

## Background

v2.3-i までは `WorkflowRepository` が runtime cache と永続化の責務を兼ねていたが、v2.3-j RFC §8 で以下の明確な責務分離が定義された：

- **WorkflowCache**: Repository Pair 上の MemoizedGraph 群の **runtime cache / in-memory working set**。source-of-truth ではなく、検索高速化・局所再利用・compile-time / retrieval-time 参照を提供する。
- **RepositoryPair**: SQLite + LadybugDB から構成される **永続化ペア**（source-of-truth）。canonical persistence, consistency, repair, quarantine, availability を担保する。
- **CacheError**: WorkflowCache 層（インメモリ操作・CAS 競合）のエラー型
- **PersistenceError**: RepositoryPair 層（デュアルストア一貫性）のエラー型

既存の `DualStoreCoordinator`（`src/store/coordinator.rs`）は `RepositoryPair` の具象実装として位置付け直される。

## Scope

1. **`WorkflowCache` 構造体の定義**（新規 `src/store/workflow_cache.rs`）:
   - `working_set: Arc<RwLock<Vec<MemoizedGraph>>>`
   - `ann_hint: Arc<RwLock<AnnHotIndex>>`
   - `policy: CachePolicy`

2. **`CachePolicy` 列挙型の定義**: `Default`, `Pinned { workflow_ids }`, `Preload { workflow_ids }`

3. **`AnnHotIndex` 型エイリアス**: `pub type AnnHotIndex = AnnIndex;`

4. **`RepositoryPair` 型エイリアス**: `pub type RepositoryPair = DualStoreCoordinator;`

5. **`CacheError` 列挙型の定義**（RFC §8.4 準拠）:
   - `CasConflict { expected: u64, actual: u64 }`
   - `NotFound(WorkflowGraphId)`
   - `LoadFailed(String)`

6. **`PersistenceError` 列挙型の定義**（RFC §8.4 準拠）:
   - `CrossStoreInconsistency(String)`
   - `SqliteError(String)`
   - `LadybugError(String)`
   - `PairNotFound(String)`

7. **`WorkflowCache::get_or_load`**: cache hit → 即時返却、cache miss → RepositoryPair から lazy load

8. **`WorkflowCache::update_graph_cas`**: 楽観的更新、バージョン不一致で `CacheError::CasConflict`

9. **`WorkflowCache::in_memory()`**: テスト用コンストラクタ

10. **`RepositoryPair::in_memory()`**: テスト用コンストラクタ（`DualStoreCoordinator` に委譲）

11. **`CacheError` / `PersistenceError` → `DarviumError` の `From` impl**

12. **crate 公開 API への追加**: `pub use store::{WorkflowCache, RepositoryPair, CachePolicy, AnnHotIndex, CacheError, PersistenceError};`

## Non-scope

- `RepositoryPair` の RFC §8 定義通りの `{ sqlite, ladybug }` 構造体は実装しない（`DualStoreCoordinator` が役割を担う）
- `RepositoryPair::load` の完全実装（M-0.5-7-R 以降）
- `RepositoryPair::commit_dual_store_update` の完全実装（既存実績あり）
- キャッシュ逐次ポリシー（LRU 等）
- preset registry 統合（別チケット）

## Investigation

### 参照観察レポート

該当する過去の観察レポートはなし。本チケットは純粋な型定義基盤であり観測テストを伴わない。

### ソースコード調査結果

1. **`DualStoreCoordinator`**（`src/store/coordinator.rs:41-47`）:
   - `graph_store: Box<dyn GraphStore + Send>`, `metadata_store: Box<dyn MetadataStore + Send>` を保持
   - `commit_dual_store_update` は実装済み、T13-T50 でテスト済み
   - `pub use store::DualStoreCoordinator` で既に公開 API エクスポート済み

2. **`WorkflowCache` 関連の型は未存在**: コード上に未定義。RFC §8 でのみ定義。

3. **`DarviumError`**（`src/error.rs:9-145`）:
   - `Storage(String)`, `NotFound(String)`, `DualStoreCommit(String)`, `DualStoreInconsistency(String)` 既存
   - キャッシュ層専用エラー variant は未追加
   - `CacheError` / `PersistenceError` は RFC §8.4 で別 enum 定義済み

4. **`MemoizedGraph` の `version: u64`**: 既に実装済み（v2.3-i）、CAS の前提条件は整っている

5. **`src/store/mod.rs:17`**: `pub use coordinator::{DualStoreCoordinator, RepairScanSummary};` — ここに新規型を追加

## Test Plan

| # | テスト名 | 種別 | 内容 |
|---|---------|------|------|
| T1 | `workflow_cache_get_or_load_hit` | 正常系 | キャッシュヒット時の即時返却 |
| T2 | `workflow_cache_get_or_load_miss` | 正常系 | キャッシュミス → RepositoryPair から lazy load → 昇格 |
| T3 | `workflow_cache_update_graph_cas_ok` | 正常系 | 期待バージョン一致時の CAS 更新成功 |
| T4 | `workflow_cache_update_graph_cas_conflict` | 異常系 | バージョン不一致 → `CacheError::CasConflict` |
| T5 | `workflow_cache_get_or_load_not_found` | 異常系 | RepositoryPair にも不在 → エラー伝播 |
| T6 | `cache_policy_default` | 正常系 | `CachePolicy::Default` の生成 |
| T7 | `cache_policy_pinned` | 正常系 | `CachePolicy::Pinned` の生成 |
| T8 | `cache_policy_preload` | 正常系 | `CachePolicy::Preload` の生成 |
| T9 | `cache_error_display` | 正常系 | 全 `CacheError` variant の `Display` 確認 |
| T10 | `persistence_error_display` | 正常系 | 全 `PersistenceError` variant の `Display` 確認 |
| T11 | `cache_error_into_darvium_error` | 正常系 | `CacheError` → `DarviumError` 変換 |
| T12 | `persistence_error_into_darvium_error` | 正常系 | `PersistenceError` → `DarviumError` 変換 |
| T13 | `repository_pair_in_memory` | 正常系 | `RepositoryPair::in_memory()` の生成 |
| T14 | `workflow_cache_in_memory` | 正常系 | `WorkflowCache::in_memory()` の生成 |
| T15 | `workflow_cache_clone_graph` | 正常系 | cache 取得時に MemoizedGraph が clone されること |
| T16 | `workflow_cache_get_or_load_miss_twice` | 正常系 | 同一 id で2回目が cache hit すること |
| T17 | `workflow_cache_multiple_graphs` | 正常系 | 複数 Graph の管理と個別アクセス |

## 計装方法・観測対象

本チケットは純粋な型定義基盤であり観測テストを伴わない。全テストは不変条件ベースの `assert_eq!` / `assert!` / `assert!(matches!())` で二値判定する。固定シード PRNG 不使用、`println!` 計装なし。

## Boy Scout Rule — 翻訳可能性計画

### `src/store/workflow_cache.rs` 新規作成時

- 関数名は `get_or_load`, `update_graph_cas`, `in_memory` とし動詞句として読めること
- `working_set`, `ann_hint` はドメイン概念を表現する命名
- `CachePolicy` の variant 名 `Pinned`, `Preload` は用途がコード断片だけで理解できること

### 既存ファイル修正時

- `src/store/mod.rs`: 既存の `pub use coordinator::...` と命名規則統一
- `src/lib.rs`: `pub use store::{...}` のアルファベット順を維持
- error 変換 impl にマッピング根拠のコメントを付与

## Acceptance Criteria

- [ ] `WorkflowCache` 構造体が RFC §8 定義通りに実装されている
- [ ] `CachePolicy` / `AnnHotIndex` の型定義が RFC §8 定義通り
- [ ] `CacheError` / `PersistenceError` が RFC §8.4 定義通り（`Display + Debug + Clone + PartialEq`）
- [ ] `RepositoryPair = DualStoreCoordinator` 型エイリアス定義
- [ ] `WorkflowCache::get_or_load` / `update_graph_cas` が RFC §8.4 疑似コード準拠
- [ ] `WorkflowCache::in_memory()` / `RepositoryPair::in_memory()` 利用可能
- [ ] `CacheError` / `PersistenceError` → `DarviumError` の `From` impl
- [ ] 全新規型が crate 公開 API としてエクスポート済み
- [ ] T1-T17 全テスト通過
- [ ] 既存テスト（協調フィルタリング T13-T50 等）に悪影響なし

## Notes

- 本チケットの完了は `⚠️ M-0.5-7-R` の前提条件 (MUST)
- RFC §8 の `RepositoryPair { sqlite, ladybug }` 構造体は設計定義。runtime では既存 `DualStoreCoordinator` を型エイリアスとして再利用する
- エラー型の `Clone + PartialEq` は既存の `DarviumError` に合わせて付与

### 成果物

- 計画: context/0079/plan.md（未作成）
- 実装サマリ: context/0079/implementation.md（未作成）
- レビュー報告書: context/0079/review.md（未作成）
- 観察レポート: context/0079/observation-YYYYMMDD-HHmmss.md（未作成）
