# 変更したファイル一覧と実装内容の概要

## ファイル一覧

| ファイル | 種別 | 内容 |
|---|---|---|
| src/constants.rs | MODIFY | HITL_DEFAULT_TIMEOUT_SECS, HITL_RECONNECT_BACKOFF_SECS 追加 |
| src/store/json_metadata_store.rs | CREATE | JsonMetadataStore（ファイル永続化 MetadataStore、原子書き込み） |
| src/store/mod.rs | MODIFY | JsonMetadataStore を pub use でエクスポート |
| src/recovery.rs | CREATE | recover_pending_interactions() 回復ループ関数 + 全テスト（11不変条件 + 2観測） |
| src/lib.rs | MODIFY | recovery モジュール登録 + recover_pending_interactions 公開 API |

## 実装サマリ

### JsonMetadataStore
- JSON ファイルによる簡易永続化 MetadataStore 実装
- 書込ごとに一時ファイルへの原子書き込み + rename でクラッシュ耐性を確保
- 初回起動時（ファイル不在）は空状態で初期化、破損検出時は Err を返す
- 既存 MetadataStore トレイトを完全実装

### recover_pending_interactions()
- MetadataStore::list_pending_human_interactions() 全件走査
- 各レコード: channel.reconnect() → handle.wait(timeout) → resolve_human_interaction()
- 失敗時は HITL_RECONNECT_BACKOFF_SECS による指数バックオフ再試行
- RecoverySummary で成功率・タイムアウト率・到達不能率を集計

### テスト（全 13 テスト）
- 不変条件テスト 11件: 単一回復、N≥10一括、混合シナリオ、Stdinoutクロス、TimedOut、競合状態、異種チャネル差し替え、JM基本、JM原子書き込み、JMファイル不在、JM破損
- 観測テスト 2件: バッチ回復成功率 (N∈{1,10,100})、回復レイテンシ分布
