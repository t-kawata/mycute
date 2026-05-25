# 実装サマリ: M-0.5-7-P: WorkflowCache + RepositoryPair + CacheError/PersistenceError 型定義基盤

## 変更ファイル一覧

| ファイル | 種別 | 内容 |
|---|---|---|
| `src/store/workflow_cache.rs` | 新規 | WorkflowCache 構造体、CachePolicy enum、CacheError/PersistenceError enum、AnnHotIndex 型エイリアス、RepositoryPair 型エイリアス、get_or_load / update_graph_cas メソッド、in_memory / in_memory_pair コンストラクタ、T1-T17 テスト |
| `src/store/mod.rs` | 修正 | `mod workflow_cache;` 追加 + 全新規型の `pub use` エクスポート |
| `src/trust.rs` | 修正 | MemoizedGraph に `graph: WorkflowGraph` と `version: u64` フィールドを追加（CAS 用） |
| `src/lib.rs` | 修正 | 6型（WorkflowCache, RepositoryPair, CachePolicy, CacheError, PersistenceError, AnnHotIndex）を crate 公開 API としてエクスポート |

## 主要設計判断

- **RwLock**: `tokio::sync::RwLock` ではなく `std::sync::RwLock` を使用。tokio が依存関係にないため、同期的なロックで十分。
- **RepositoryPair**: RFC §8 の `{ sqlite, ladybug }` 構造体は実装せず、`DualStoreCoordinator` の型エイリアスとして定義。
- **AnnHotIndex**: `AnnIndex` が未定義のため、`MockHnswIndex` をエイリアス。
- **MemoizedGraph 拡張**: `graph` と `version` フィールドを追加以外は縮約実装のまま。

## 検証結果

- 新規テスト 17/17 PASS
- 既存テスト 834/834 PASS（回帰なし）
- clippy: 警告ゼロ
- fmt: 適合
- 品質チェック: 全 issue は既存コード由来（accept）
