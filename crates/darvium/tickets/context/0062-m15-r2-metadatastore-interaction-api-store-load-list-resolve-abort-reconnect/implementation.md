# Implementation Summary: M1.5-R2 MetadataStore 汎用 Interaction API 拡張

## 変更したファイル一覧

### src/types.rs
- 追加: `InteractionFilter` 構造体（Debug, Clone, Default derive）
  - フィールド: `status: Option<InteractionStatus>`, `channel_id: Option<String>`, `created_after: Option<u64>`, `created_before: Option<u64>`, `limit: Option<usize>`

### src/store/metadata_store.rs
- `MetadataStore` トレイトに6つの汎用メソッドを追加:
  - `store_interaction` — StoredInteraction を保存
  - `load_interaction` — ID指定で読み込み（Ok(None) for not found）
  - `list_interactions` — InteractionFilter によるフィルタリング
  - `resolve_interaction` — Resolved 状態へ遷移
  - `abort_interaction` — Aborted 状態へ遷移
  - `reconnect_interaction` — updated_at 更新（暫定）
- 既存4つの HITL 特化メソッドを `#[inline]` デフォルト実装（ラッパー）に変更
- `InMemoryMetadataStore` に6メソッドの実装を追加
- テストモジュールに T1-T8 + OTS-1 を追加

### src/store/json_metadata_store.rs
- `JsonMetadataStore` に6メソッドの実装を追加（変更操作後に flush）
- 既存4つの HITL 特化メソッドの明示的実装を削除（デフォルト委譲）

### src/store/coordinator.rs
- `FailingMetadataStore` に6メソッドのスタブ実装を追加
  - 書き込み操作（store/resolve/abort/reconnect）: budget消費 + 委譲
  - 読み取り操作（load/list）: 常に委譲（budget消費なし）
