---
ticket_id: 86
title: ReciprocityEvent / ReciprocityEventKind データ型定義
slug: reciprocityevent-reciprocityeventkind
status: reviewed
created_at: 2026-05-25
updated_at: 2026-05-25
ticket_ref: M1.76-1
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0086-reciprocityevent-reciprocityeventkind/plan.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0086-reciprocityevent-reciprocityeventkind/implementation.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0086-reciprocityevent-reciprocityeventkind/observation-20260525-182113.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0086-reciprocityevent-reciprocityeventkind/review.md
---

# M1.76-1: ReciprocityEvent / ReciprocityEventKind データ型定義

## Summary

RFC §15.10.6 に定義される ReciprocityEvent 構造体と ReciprocityEventKind 列挙型を実装する。既存の `ReciprocityEvent` 列挙型（event.rs:305、現在 DarviumEventKind::Reciprocity の variant 型として使用中）を `ReciprocityEventKind` にリネームし、新たに `ReciprocityEvent` 構造体を定義する。`DarviumEvent`（`DarviumEventKind::Reciprocity`）からの `TryFrom` 変換を実装し、`DarviumError` に `ReciprocityError` を追加する。

## Background

M1.76 系列（Reciprocity-Aware Survival and Benevolence Integration）の基盤となるデータ型定義。現在のコードでは `ReciprocityEvent` という名前が HELP プロトコルの状態遷移に対応するイベント種別（Kind）に使用されており、これは RFC §15.10.6 の `ReciprocityEventKind` に相当する。M1.76-3 以降で計算される直接互恵性スコア・間接互恵性スコアの入力となる `ReciprocityEvent` 構造体（event_id, mission_id, source_graph_id, target_graph_id, weight 等のフィールドを持つ）は未定義である。

本チケットでは以下の命名整理を行う：
- **既存 `ReciprocityEvent` 列挙型** → `ReciprocityEventKind` にリネーム
- **新規 `ReciprocityEvent` 構造体** → RFC §15.10.6 準拠の全フィールドを持つ

### 参照観測レポート

過去の観察レポートは見つかりませんでした（tickets/context/ に observation-*.md なし）。M1.76 系列の初めての実験系列として開始する。

### 既存コードの証拠

- **`event.rs:305-322`**: `ReciprocityEvent` 列挙型が存在。8 バリアント（HelpOffered, HelpAccepted, HelpRejected, HelpExecuted, HelpSucceeded, HelpAbandoned, HarmfulMismatch, ReturnedFavor）。コメントには「RFC §15.10.6 ReciprocityEventKind の variant を DarviumEventKind 用に流用」とあり、命名の誤りが認識されている。
- **`event.rs:389`**: `DarviumEventKind::Reciprocity(ReciprocityEvent)` — リネーム後は `Reciprocity(ReciprocityEventKind)` になる。
- **`event.rs:1165-1180`**: `DomainProjection::reciprocity_event()` が全 8 variant をフィルタ条件として使用。
- **`help.rs:546-554`**: `transition_to_event()` 関数が状態遷移 → `ReciprocityEvent`（将来的には `ReciprocityEventKind`）を返却。
- **`help.rs:850-943`**: 各種 HELP プロトコル関数が `DarviumEventKind::Reciprocity(ReciprocityEvent::*)` を生成。
- **`event_channel.rs:636`**: StdinoutEventChannel の互換性判別に使用。
- **`lib.rs:94`**: 公開 API として `ReciprocityEvent` を re-export。
- **`error.rs`**: `DarviumError` に `ReciprocityError` バリアントは未定義。
- **`event.rs テストコード内`**: ランダムイベント生成（`generate_random_event_kind`）・プロパティテスト戦略（`reciprocity_event_strategy`）・Projection テスト（R10 TC-3）で広範囲に参照。

## Scope

1. **`ReciprocityEventKind` 列挙型（リネーム）**: 既存 `ReciprocityEvent` の全 8 バリアントを維持し、名前のみ変更。
   - `HelpOffered`, `HelpAccepted`, `HelpRejected`, `HelpExecuted`, `HelpSucceeded`, `HelpAbandoned`, `HarmfulMismatch`, `ReturnedFavor`
   - `#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]` を維持

2. **`ReciprocityEvent` 構造体（新規）**: RFC §15.10.6 準拠の全フィールドを持つ。`WorkflowGraphId` は `src/types.rs:19` に `pub type WorkflowGraphId = String;` として定義済み。
   - `event_id: String` — UUIDv4 イベント識別子
   - `mission_id: String` — 関連ミッション識別子
   - `source_graph_id: WorkflowGraphId` — 送信元グラフ ID
   - `target_graph_id: WorkflowGraphId` — 送信先グラフ ID
   - `event_kind: ReciprocityEventKind` — イベント種別
   - `weight: f32` — イベント重み（互恵性スコア計算の入力）
   - `created_at: SystemTime` — イベント発生日時
   - `virtual_clock: u64` — EventBus clock 値
   - `trace_ref: Option<String>` — トレース識別子（任意）
   - `#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]`

3. **`TryFrom<DarviumEvent> for ReciprocityEvent`**:
   - `DarviumEventKind::Reciprocity(ReciprocityEventKind)` の場合のみ変換成功
   - マッピング元：DarviumEvent の envelope フィールドから抽出
     - `event_id` → `DarviumEvent.event_id`
     - `mission_id` → `DarviumEvent.causality.mission_id`（なければ空文字列）
     - `source_graph_id` → `DarviumEvent.payload["source_graph_id"]`
     - `target_graph_id` → `DarviumEvent.payload["target_graph_id"]`
     - `event_kind` → DarviumEventKind の内包する ReciprocityEventKind
     - `weight` → `DarviumEvent.payload["weight"]`
     - `created_at` → `DarviumEvent.metadata.timestamp`
     - `virtual_clock` → `DarviumEvent.metadata.clock`
     - `trace_ref` → `DarviumEvent.causality.trace_ref`
   - DarviumEventKind が Reciprocity 以外の場合は `DarviumError::ReciprocityError` を返す

4. **`DarviumError::ReciprocityError(String)` 追加**:
   - `error.rs` に `#[error("Reciprocity error: {0}")] ReciprocityError(String)` を追加

5. **既存コードの全参照箇所をリネーム対応**:
   - `event.rs`: 列挙型定義・DarviumEventKind variant・DomainProjection・テストコード
   - `help.rs`: 全 `ReciprocityEvent::*` → `ReciprocityEventKind::*`。`transition_to_event()` の戻り値型も `ReciprocityEventKind` に変更
   - `event_channel.rs`: 同様
   - `lib.rs`: `ReciprocityEvent`(struct) + `ReciprocityEventKind`(enum) 両方を export。`transition_to_event` の戻り値型変更に伴い re-export の型参照を更新

## Non-scope

- 互恵性スコアの計算ロジック（M1.76-3, M1.76-4）
- ReputationProfile 拡張（M1.76-2）
- ReciprocityEvent の永続化 / MetadataStore 操作
- GC hazard / Child protection / Helper quality score 等の後続チケット

## Investigation

### 調査結果: 既存 ReciprocityEvent の全参照箇所一覧

リネーム対応が必要なファイル：

| ファイル | 参照箇所数 | 主な使用法 |
|---------|-----------|-----------|
| `src/event.rs` | 30+ | 定義（305行目）、DarviumEventKind variant、DomainProjection フィルタ、テスト戦略 |
| `src/help.rs` | 12 | `transition_to_event()` 戻り値、DarviumEvent 生成時の variant 指定 |
| `src/event_channel.rs` | 2 | StdinoutEventChannel 互換性判別（636行目） |
| `src/lib.rs` | 1 | pub use re-export（94行目） |

### 命名整理の方針

```
現状: ReciprocityEvent (enum) → DarviumEventKind::Reciprocity(ReciprocityEvent)
変更後:
  - ReciprocityEventKind (enum) → DarviumEventKind::Reciprocity(ReciprocityEventKind)
  - ReciprocityEvent (struct) → 新規定義、TryFrom<DarviumEvent> で materialize
```

### 確認済み型定義

- **`WorkflowGraphId`**: `src/types.rs:19` に `pub type WorkflowGraphId = String;` として定義済み。追加定義不要。

### RFC §12C.2 との命名一貫性

RFC §12C.2 line 3094 は `DarviumEventKind::Reciprocity(ReciprocityEvent)` と記載しているが、これは §15.10.6 で `ReciprocityEvent` が9フィールドの構造体として定義される前に書かれたものであり、RFC 内部に軽微な命名不一致が存在する。M1.76-1 では以下の解釈でこの不一致を解決する：

- `DarviumEventKind::Reciprocity` の variant 型は他の全 variant（`Search(SearchEvent)`、`Fusion(FusionEvent)` 等）と同様に軽量な**種別判別用列挙型**とする → **`ReciprocityEventKind`** を使用
- `ReciprocityEvent` 構造体は §15.10.6 の完全な定義に従い、TryFrom&lt;DarviumEvent&gt; による materialize で生成される別体型とする
- RFC 補足注釈（line 4454）「ReciprocityEvent は §12E の EventProjection として再構成される」とも整合する

## Test Plan

### ユニットテスト計画

1. **TC-1: `ReciprocityEventKind` 全 8 バリアントのトレイト実装確認**
   - 全 variant の Debug + Clone + PartialEq + Serialize + Deserialize 確認
   - serde_json ラウンドトリップ

2. **TC-2: `ReciprocityEvent` 構造体の全フィールド設定・アクセス**
   - 全 9 フィールドを設定したインスタンス生成と読み取り確認
   - serde_json ラウンドトリップ

3. **TC-3: `DarviumEvent` → `ReciprocityEvent` TryFrom 変換（成功系）**
   - `DarviumEventKind::Reciprocity(kind)` からの変換成功
   - 各フィールドの正しいマッピング確認

4. **TC-4: `DarviumEvent` → `ReciprocityEvent` TryFrom 変換（失敗系）**
   - 非 Reciprocity kind からの変換 → `ReciprocityError`

5. **TC-5: `ReciprocityEventKind` パターンマッチ網羅性**
   - 全 8 variant を `_ =>` なしで網羅する match のコンパイル確認

6. **TC-6: 計装 — 往復変換完全性（n = 1000）**
   - 固定シード PRNG で 1000 件生成 → TryFrom 変換 → 全フィールド一致確認

7. **TC-7: コンパイル時検証 — 既存参照箇所のリネーム追従**
   - 全既存テストが通過すること

## 計装方法・観測対象

### 計装方法

- TC-6: 固定シード `StdRng::seed_from_u64(12345)` + `println!` + `--nocapture`
- 型構造の JSON 形式出力

### 観測対象

- 往復変換の成功率（期待値: n = 1000 で 100%）
- 失敗 variant からの変換が 100% ブロックされること

### 較正計画

本チケットはデータ型定義のみ。較正は M1.76-3 以降で実施。

## Boy Scout Rule — 翻訳可能性計画

1. **`event.rs`**: 既存コメント「ReciprocityEvent の variant を DarviumEventKind 用に流用」→「ReciprocityEventKind の variant」に修正。
2. **`help.rs`**: `transition_to_event()` → `transition_to_event_kind()` に改名。関数名が「イベント種別に変換する」ことを明確に表現する。
3. **`DarviumError`**: 適切な分類位置に `ReciprocityError` を追加。
4. **全リネーム作業**: 機械的置換ではなく、各参照箇所のコメントも併せて修正し型名と内容の一致を確保。

## Acceptance Criteria

- [ ] `ReciprocityEventKind` にリネームされ、全 8 variant が Debug + Clone + PartialEq + Serialize + Deserialize を実装
- [ ] `ReciprocityEvent` 構造体が 9 フィールドで定義され、TryFrom&lt;DarviumEvent&gt; が実装されている
- [ ] DarviumError に `ReciprocityError(String)` が追加されている
- [ ] 既存の全参照箇所（event.rs / help.rs / event_channel.rs / lib.rs）がリネーム追従し、コンパイルが通る
- [ ] 全テストが通過する（既存テスト + 新規 7 TC）
- [ ] `ReciprocityEventKind` は `_ =>` なしの網羅的パターンマッチが可能
- [ ] n = 1000 の往復変換完全性テストが 100% 成功

## Notes

### 成果物

- 計画: context/0086-reciprocityevent-reciprocityeventkind/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0086-reciprocityevent-reciprocityeventkind/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0086-reciprocityevent-reciprocityeventkind/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0086-reciprocityevent-reciprocityeventkind/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成）
