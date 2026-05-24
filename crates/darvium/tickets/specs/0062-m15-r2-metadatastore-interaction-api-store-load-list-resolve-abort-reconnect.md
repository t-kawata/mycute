---
ticket_id: 62
title: M1.5-R2: MetadataStore 汎用 Interaction API 拡張（store / load / list / resolve / abort / reconnect）
slug: m15-r2-metadatastore-interaction-api-store-load-list-resolve-abort-reconnect
status: reviewed
created_at: 2026-05-24
updated_at: 2026-05-24
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0062-m15-r2-metadatastore-interaction-api-store-load-list-resolve-abort-reconnect/plan.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0062-m15-r2-metadatastore-interaction-api-store-load-list-resolve-abort-reconnect/observation-20260524-111627.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0062-m15-r2-metadatastore-interaction-api-store-load-list-resolve-abort-reconnect/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0062-m15-r2-metadatastore-interaction-api-store-load-list-resolve-abort-reconnect/review.md
---

# M1.5-R2: MetadataStore 汎用 Interaction API 拡張（store / load / list / resolve / abort / reconnect）

## Summary

`MetadataStore` トレイトに6つの汎用 Interaction メソッド（`store_interaction` / `load_interaction` / `list_interactions` / `resolve_interaction` / `abort_interaction` / `reconnect_interaction`）を追加する。既存の HITL 特化メソッド（`store_human_interaction` 等）はこれら汎用メソッドを呼び出すラッパーとして再実装する。`InteractionFilter` 構造体を新規定義する。

## Background

M1.5-R1 で `InteractionRecord<TPayload>` ジェネリック型と `InteractionStatus` 7状態が定義された。しかし現状の `MetadataStore` トレイトは HITL 特化のメソッドのみを持ち、汎用的な Interaction 操作インターフェースが存在しない。今後の Event Architecture（M1.5-R3〜R11）では HITL 以外のドメイン（Training、Fusion 等）も InteractionRecord を用いるため、ドメイン非依存の操作インターフェースが必要となる。

また、crash recovery（M1.5-3）では `Aborted` 状態やチャネル再接続（`reconnect`）が重要な役割を果たすが、現状のインターフェースはこれらの操作を表現できない。

## Scope

- `src/types.rs`: `InteractionFilter` 構造体の定義
- `src/store/metadata_store.rs`:
  - `MetadataStore` トレイトに6つの汎用メソッドを追加
  - 既存4つの HITL 特化メソッドを汎用メソッド呼び出しに委譲するデフォルト実装に変更
  - `InMemoryMetadataStore` に6つの汎用メソッド実装を追加
  - 既存 HITL メソッドの明示的実装を削除（デフォルト実装に委譲）
- `src/store/json_metadata_store.rs`:
  - `JsonMetadataStore` に6つの汎用メソッド実装を追加
  - 既存 HITL メソッドの明示的実装を削除（デフォルト実装に委譲）
- `InteractionFilter`: `status` / `channel_id` / `created_after` / `created_before` / `limit` フィールド

## Non-scope

- `InteractionRecord<TPayload>` の定義変更（M1.5-R1 完了済み）
- `StoredInteraction` 型エイリアスの再定義（M1.5-R3）
- MetadataStore 以外のストア（GraphStore）の変更
- 実際の EventBus 実装（M1.5-R5）
- crash recovery プロトコルの変更（不変）

## Investigation

### 参照観察レポート

- `tickets/context/0061-m15-r1-interactionrecordtpayload-interactionstatus-7/observation-20260524-110006.md` — M1.5-R1 完了確認。7状態遷移行列とJSONラウンドトリップ n=1000 で 100% 成功。次チケットへの示唆として「AwaitingExternal → Resolved 経由の遷移プロトコル実装」が挙げられている。

### ソースコード調査結果

**現状の MetadataStore トレイト** (`src/store/metadata_store.rs:20-72`):
- `store_human_interaction(&self, record: &StoredInteraction) -> Result<(), DarviumError>` — HITL 特化
- `load_human_interaction(&self, interaction_id: &str) -> Result<StoredInteraction, DarviumError>` — NotFound 時に Err を返す
- `list_pending_human_interactions(&self) -> Result<Vec<StoredInteraction>, DarviumError>` — Pending 固定フィルタ
- `resolve_human_interaction(&self, interaction_id: &str, outcome: &HumanOutcome) -> Result<(), DarviumError>` — Resolved のみ

**不足している操作**:
- `abort_interaction`: 現状、レコードを Aborted 状態にする手段がない（`resolve_human_interaction` では Resolved にしかできない）
- `reconnect_interaction`: チャネル再接続時の情報更新手段がない
- `load_interaction` が Result（Err で NotFound）を返す設計 → 汎用 API では `Result<Option<...>>` が適切
- `list_interactions`: Pending フィルタ固定ではなく、任意の InteractionFilter でフィルタ可能にすべき

**型情報**:
- `StoredInteraction` は `InteractionRecord<HitlPayload>` の型エイリアス (`src/types.rs:5088`)
- `HitlPayload` は `{ request: HumanRequest }` + `Outcome = HumanOutcome` (`src/types.rs:5077-5084`)
- `InteractionStatus` は 7 状態: `Pending`, `AwaitingExternal`, `Resolved`, `TimedOut`, `Unreachable`, `ChannelClosed`, `Aborted`

**トレイトのオブジェクト安全性**:
- `MetadataStore` トレイトは `Box<dyn MetadataStore>` として使用されている（`src/store/metadata_store.rs:255-258`）。
- そのため、追加するメソッドはジェネリックメソッド（型パラメータを持つメソッド）にできない。
- 代替策: メソッドは `StoredInteraction` 具象型で定義し、「汎用性」は呼び出し側が `InteractionRecord<TPayload>` を `StoredInteraction` にキャストして使うことで実現する。

**InMemoryMetadataStore 内部構造** (`src/store/metadata_store.rs:83-104`):
- `human_interactions: RefCell<HashMap<String, StoredInteraction>>` — interaction_id をキーとする HashMap
- この構造は汎用 Interaction API でもそのまま使用可能

**JsonMetadataStore 内部構造** (`src/store/json_metadata_store.rs:44-52`):
- `human_interactions: RefCell<HashMap<String, StoredInteraction>>` — 同上
- 変更操作ごとに `self.flush()` でファイルへ原子書き込み

### 設計上の判断

1. **オブジェクト安全性の維持**: 6つの新メソッドは `StoredInteraction` 具象型（型パラメータなし）で定義する。これにより `Box<dyn MetadataStore>` が引き続き使用可能。真のジェネリック性は M1.5-R3 以降の型エイリアス再定義で担保する。

2. **`load_interaction` の戻り値**: 汎用 API では `Result<Option<StoredInteraction>>` とする（存在しない場合は `Ok(None)`）。既存の `load_human_interaction` はデフォルト実装で `Ok(None)` を `Err(NotFound)` に変換する。

3. **`abort_interaction` と `reconnect_interaction`**:
   - `abort_interaction`: 指定 ID のレコードの status を `Aborted` に設定する。outcome は変更しない。
   - `reconnect_interaction`: 指定 ID のレコードの `updated_at` を更新する。HITL ペイロードに channel_id フィールドがないため暫定的に時刻のみ更新（後続 M1.5-R7 で拡張）。

4. **既存メソッドのデフォルト実装**: トレイトにデフォルト実装を追加し、既存の明示的実装を削除する。これにより新ストア実装（SqliteMetadataStore 等）は6つの汎用メソッドのみ実装すればよい。

## Test Plan

テストは `src/store/metadata_store.rs` の既存テストモジュールに追加する。全テストは `InMemoryMetadataStore` と `JsonMetadataStore` の両方で実行可能なように設計する。

### T1: `store_interaction` + `load_interaction` write-after-read 一貫性（n=100 ラウンドトリップ）
- 100 個の `StoredInteraction`（各種 InteractionStatus を含む）を `store_interaction` で保存
- 各レコードを `load_interaction` で読み出し、内容が完全一致することを確認
- 存在しない ID の `load_interaction` は `Ok(None)` を返すことを確認

### T2: `list_interactions` + `InteractionFilter` フィルタ精度
- 複数 status・複数時刻のレコードを保存
- `InteractionFilter` の各フィールド（status, created_after, created_before, limit）でフィルタが正確に動作することを確認
- フィルタ未指定（全フィールド None）の場合、全件返ることを確認

### T3: `resolve_interaction` 状態遷移
- Pending レコードを保存し、`resolve_interaction` で解決
- `load_interaction` で status が `Resolved`、outcome が設定されていることを確認
- 存在しない ID の resolve → `Err(NotFound)`

### T4: `abort_interaction` 状態遷移
- Pending / AwaitingExternal のレコードを保存し、`abort_interaction` で中断
- status が `Aborted` になっていることを確認
- 存在しない ID → `Err(NotFound)`

### T5: `reconnect_interaction` 再接続
- レコードを保存し、`reconnect_interaction` で再接続
- `updated_at` が更新されていることを確認
- 存在しない ID → `Err(NotFound)`

### T6: 既存 HITL 特化メソッドのラッパー検証
- 各メソッドの呼び出し結果が汎用メソッドの呼び出し結果と一致することを確認
- `load_human_interaction` が存在しない ID に対して `Err(NotFound)` を返すことを確認（ラッパー側の変換）

### T7: 後方互換性
- 既存の T10 テスト群（store_and_load, list_pending, resolve, overwrite, export_to_store, crash_recovery）が変更なしで全件パスすることを確認

### T8: JsonMetadataStore 永続化検証
- 汎用 API で保存したレコードがファイルに正しく永続化され、再読込できることを確認
- `abort_interaction` 後の状態がファイルに正しく反映されることを確認

### OTS-1: スループット計測
- 6メソッド × 1000 呼び出しのスループットを計測
- 期待: 全操作が O(1) で線形パフォーマンス

## 計装方法・観測対象

### 計装方法

- OTS-1: 6メソッド × 1000 回ずつ呼び出し、合計実行時間と平均レイテンシを `println!` + `--nocapture` で出力
- 計装コードは `src/store/metadata_store.rs` の `mod tests` に追加

### 観測対象

- 各メソッドの呼び出し成功率（100% であること）
- スループット（6メソッド × 1000呼び出し / 総実行時間）
- 呼び出しごとのレイテンシ分布（最小・最大・平均）

### 較正計画

本チケットはインターフェース追加のみであり、較正対象の定数はない。

## Boy Scout Rule — 翻訳可能性計画

### 改善対象

1. **`resolve_human_interaction` の outcome 書き換え**: 現状 `record.outcome = Some(outcome.clone())` は既存の outcome を無条件に上書きする。新しい汎用 `resolve_interaction` ではこの挙動を維持するが、冪等性保証は後続チケットの課題としてコメントに記録する。

2. **ラッパーメソッドに `#[inline]` 属性を付与**: デフォルト実装がインライン展開されるよう最適化ヒントを追加し、オーバーヘッドをゼロにする。

3. **`InteractionFilter` のフィールド命名**: `created_after` / `created_before` は時刻範囲の意味が自明である。`u64` (VirtualClock 値) であることをドキュメントコメントに明記する。

4. **`reconnect_interaction` の暫定実装**: HumanRequest に channel_id がないため暫定的に `updated_at` のみ更新。M1.5-R7 で拡張予定である旨をコメントに記述する。

## Acceptance Criteria

- [ ] 6つの汎用 Interaction メソッドが `MetadataStore` トレイトに追加されている
- [ ] `InteractionFilter` 構造体が定義されている（status / channel_id / created_after / created_before / limit）
- [ ] 既存4つの HITL 特化メソッドが汎用メソッドのラッパーとして再実装されている
- [ ] `InMemoryMetadataStore` に全6メソッドの実装が追加されている
- [ ] `JsonMetadataStore` に全6メソッドの実装が追加されている（flush 付き）
- [ ] 既存テストが全件パスする（後方互換性）
- [ ] オブジェクト安全性（`Box<dyn MetadataStore>`）が維持されている
- [ ] 全テスト正常系・異常系・境界値がカバーされている
- [ ] OTS-1: スループット計測で全操作が O(1) であることを確認

## Notes

- `load_interaction` は `Result<Option<StoredInteraction>>` を返す（既存の `load_human_interaction` は `Err(NotFound)`）
- `abort_interaction` と `reconnect_interaction` は完全に新規の操作
- `reconnect_interaction` は HumanRequest に channel_id がないため暫定的に `updated_at` のみ更新
- `InteractionFilter` の `channel_id` フィールドは現状 unused（後続 M1.5-R7 で活用予定）

### 成果物

- 計画: context/0062-m15-r2-metadatastore-interaction-api-store-load-list-resolve-abort-reconnect/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0062-m15-r2-metadatastore-interaction-api-store-load-list-resolve-abort-reconnect/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0062-m15-r2-metadatastore-interaction-api-store-load-list-resolve-abort-reconnect/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0062-m15-r2-metadatastore-interaction-api-store-load-list-resolve-abort-reconnect/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
