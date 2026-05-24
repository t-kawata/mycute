# 実装サマリー: M1.5-R1 InteractionRecord<TPayload> + InteractionStatus 7状態

## 変更ファイル

| ファイル | 種別 | 内容 |
|---------|------|------|
| src/types.rs | コア変更 | `InteractionPayload` トレイト、`InteractionRecord<TPayload>` ジェネリック構造体、`HitlPayload` 構造体、`InteractionStatus` 7状態 enum、`StoredInteraction` 型エイリアス、後方互換アクセサを定義 |
| src/human_channel.rs | 呼出元修正 | StoredInteraction 構築箇所を `payload: HitlPayload { request: ... }` 形式に更新、HitlPayload インポート追加 |
| src/recovery.rs | 呼出元修正 | StoredInteraction 構築箇所を全件更新、`record.request` フィールドアクセスを `record.request()` メソッドに変更 |
| src/store/metadata_store.rs | 呼出元修正 | StoredInteraction 構築箇所更新 + `pending_record.request` → `pending_record.request()` |
| src/store/json_metadata_store.rs | 呼出元修正 | StoredInteraction 構築箇所更新 + `loaded.request.subject` → `loaded.request().subject` |

## 実装内容

1. `InteractionPayload` トレイト: ペイロード型ごとに Outcome 型を関連付ける
2. `InteractionRecord<TPayload>`: ドメイン非依存の汎用インタラクションレコード（interaction_id, payload, outcome, status, created_at, updated_at）
3. `HitlPayload`: HITL ドメイン固有ペイロード（request: HumanRequest）
4. `InteractionStatus`: Pending, AwaitingExternal, Resolved, TimedOut, Unreachable, ChannelClosed, Aborted の7状態
5. `type StoredInteraction = InteractionRecord<HitlPayload>` 型エイリアス
6. 後方互換アクセサ `StoredInteraction::request()` と `StoredInteraction::outcome()`
7. serde バウンドは `#[serde(bound(deserialize = "TPayload: DeserializeOwned, TPayload::Outcome: DeserializeOwned"))]` で処理

## 検証結果

- cargo check: 成功（warnings 0）
- cargo test: 611 tests all passing
