# 計画: M-2-1.5 Dual-Store 抽象トレイト階層の定義

## 要件の再確認

DARVIUM のストレージ操作を GraphStore / MetadataStore の2系統トレイトで抽象化し、InMemoryGraphStore / InMemoryMetadataStore のメモリ内実装を提供する。

## 変更ファイル一覧

| ファイル | 種別 | 内容 |
|---------|------|------|
| src/error.rs | 修正 | Storage, NotFound, EmbeddingDimensionMismatch 追加 |
| src/types.rs | 修正 | GraphId, KnowledgeObject, RelationRecord, OriginTrace, TrainingMetadata, FusionMetadata 追加 |
| src/store/mod.rs | 新規 | ストアモジュール宣言 |
| src/store/graph_store.rs | 新規 | GraphStore トレイト + InMemoryGraphStore + テスト |
| src/store/metadata_store.rs | 新規 | MetadataStore トレイト + InMemoryMetadataStore + テスト |
| src/lib.rs | 修正 | pub mod store; 追加 |

## 実装手順

1. error.rs にエラーバリアント追加
2. types.rs に補助データ型追加
3. store/mod.rs 作成
4. store/graph_store.rs 作成 (トレイト + 実装 + テスト)
5. store/metadata_store.rs 作成 (トレイト + 実装 + テスト)
6. lib.rs に pub mod store; 追加
7. ビルド・テスト検証

## 物理的レビュー方法

1. cargo build でコンパイルエラーなし
2. cargo test で全テスト + 既存テスト通過
3. cargo clippy -- -D warnings
4. 翻訳可能性 grep (関数名が動詞句であること、定数参照確認)
5. run-quality-checks.js 実行

## リスク

- RefCell による実行時ボローチェック (シングルスレッド想定、後日 Mutex 化)
- WorkflowGraph の Clone 要件 (既に WorkflowNode/EdgeMeta に Clone あり)
- SearchTrace 等の空構造体 (後続チケットで具体化、トレイトシグネチャ不変)
