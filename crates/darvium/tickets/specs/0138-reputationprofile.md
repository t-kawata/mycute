---
ticket_id: 138
title: ReputationProfileの永続化と再読込
slug: reputationprofile
status: reviewed
created_at: 2026-05-28
updated_at: 2026-05-28
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0138-reputationprofile/plan.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0138-reputationprofile/observation-20260528-162534.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0138-reputationprofile/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0138-reputationprofile/review.md
---

# ReputationProfileの永続化と再読込

## Summary

`store_memoized_graph` が `MemoizedGraph.reputation` を保存せず、`load_memoized_graph` が常に `ReputationProfile::cold_start()` で上書きする問題を修正する。評判値を GraphStore に永続化し、再読込時に復元できるようにする。

## Background

チケット#137 で `Darvium::recompute_reputations()` による評判再計算パイプラインは実装された。しかし、計算された評判値を保存する機構が存在しないため、以下の問題がある：

1. **評判値が永続化されない**: `store_memoized_graph()`（coordinator.rs:223）は `graph` と `task_embedding` のみを persist し、`reputation` を含む他の全フィールドを黙って捨てている
2. **再読込時に cold_start で上書き**: `load_memoized_graph()`（coordinator.rs:240）は reputation を常に `ReputationProfile::cold_start()` で初期化する。せっかく計算した評判値が読み込み時に消失する
3. **MemoizedGraph がシリアライズ不可**: `MemoizedGraph` は `#[derive(Debug, Clone)]` のみで、`Serialize` / `Deserialize` を実装していない。一方 `ReputationProfile` は `Serialize + Deserialize` 済み

結果として、プロダクションで `recompute_reputations()` を呼び出して評判値を計算しても、次回のロード時に無価値になる。シミュレーション内ではメモリ上に保持されるため問題にならないが、プロダクション利用には永続化が必須である。

## Scope

### A. MemoizedGraph のシリアライズ対応
- `MemoizedGraph`（trust.rs:31）に `Serialize, Deserialize` の derive を追加
- 全フィールドが serde 対応であることを確認（`WorkflowGraph`, `TrustProfile`, `SpacePositionEmbedding`, `ReputationProfile` 等）

### B. GraphStore トレイトに reputation 保存メソッドを追加
- `GraphStore` トレイトに `store_reputation` / `load_reputation` メソッドを追加
- LadybugDB / InMemoryGraphStore の両実装を追加
- 保存形式: `serde_json::to_vec(&profile)` のバイト列

### C. store_memoized_graph の拡張
- `DualStoreCoordinator::store_memoized_graph()`（coordinator.rs:223）で `store_reputation` を呼び出す
- `store_workflow_graph_with_id` や `store_embedding` と同列のエラーハンドリング

### D. load_memoized_graph の拡張
- `DualStoreCoordinator::load_memoized_graph()`（coordinator.rs:240）で `load_reputation` の結果を使用
- 保存データがない場合のみ `cold_start()` にフォールバック
- 保存/読込失敗は `warn!` ログ + `cold_start()` フォールバック（非 fatal）

### E. テスト
- 永続化→再読込のラウンドトリップテスト
- 保存データがない場合のフォールバックテスト

## Non-scope

- MYCUTE タイマーループからの定期的な `store_memoized_graph` 呼び出し設計（別チケット#140）
- 評判値の DB インデックスやクエリ最適化
- GraphStore 以外のストレージバックエンド追加
- `MemoizedGraph` の他フィールド（trust, alive, gc_state 等）の永続化（スコープ拡大防止）

## Investigation

### [E1] store_memoized_graph は graph と embedding のみ保存

coordinator.rs:223-231:
```rust
pub fn store_memoized_graph(&self, memoized: &MemoizedGraph) -> Result<(), DarviumError> {
    self.graph_store
        .store_workflow_graph_with_id(&memoized.id, &memoized.graph)?;
    if !memoized.task_embedding.is_empty() {
        self.graph_store
            .store_embedding(&memoized.id, &memoized.task_embedding)?;
    }
    Ok(())
}
```
`reputation` を含む全フィールドが無視されている。

### [E2] load_memoized_graph は reputation を cold_start() で上書き

coordinator.rs:240-273:
```rust
pub fn load_memoized_graph(&self, graph_id: &str) -> Result<MemoizedGraph, DarviumError> {
    let graph = self.graph_store.load_workflow_graph(&graph_id.to_string())?;
    let task_embedding = self.graph_store.load_embedding(graph_id).unwrap_or_default();
    Ok(MemoizedGraph {
        id: graph_id.to_string(),
        graph,
        trust: crate::types::TrustProfile { /* 固定値 */ },
        version: 0,
        cache_invalidated: false,
        task_embedding,
        birth_tick: None,
        alive: true,
        position: crate::spaceposition::SpacePositionEmbedding::unknown(),
        village_assignment: None,
        gc_state: crate::event::GcEvent::Active,
        last_update_tick: 0,
        experience_count: 0,
        reputation: crate::event::ReputationProfile::cold_start(),  // ← 常に上書き
        last_virtual_seen: 0,
    })
}
```

### [E3] MemoizedGraph に Serialize/Deserialize なし

trust.rs:30:
```rust
#[derive(Debug, Clone)]
pub struct MemoizedGraph { ... }
```
`ReputationProfile`（event.rs:434）と `TrustProfile`（types.rs）は `Serialize + Deserialize` 済み。

### [E4] GraphStore トレイトに reputation 保存メソッドなし

GraphStore トレイトには `store_workflow_graph`、`store_embedding`、`load_workflow_graph`、`load_embedding` はあるが、reputation 保存/読込メソッドは存在しない。

### 参照観察レポート

- `tickets/context/0137-untitled/observation-20260528-154420.md` — チケット#137 で評判再計算パイプラインが実装されたが、永続化は未着手

## Test Plan

### T1: ラウンドトリップ（正常系）
- `MemoizedGraph` を作成し、reputation に有意な値を設定
- `store_memoized_graph` で保存
- `load_memoized_graph` で再読込
- reputation の全16フィールドが元の値と一致することを確認（誤差 1e-6）

### T2: 保存データなし → cold_start フォールバック
- 永続化されていない graph_id に対して `load_memoized_graph` を呼び出し
- reputation が `cold_start()` と一致することを確認

### T3: 非 fatal エラーハンドリング
- 保存失敗時も全体の処理が中断しないことを確認
- 読込失敗時も `cold_start()` にフォールバックすることを確認

### T4: 既存テスト回帰なし
- `cargo test` 全テスト通過

## 計装方法・観測対象

### 計装方法
- T1-T3: 通常の `#[test]` + `println!` + `--nocapture` で観測
- T4: `cargo test` で全テスト実行

### 観測対象
- T1: ラウンドトリップ前後の reputation フィールド比較（誤差 < 1e-6）
- T4: 全テストの PASS/FAIL カウント

### 較正計画
- 本チケットでは較正は行わない

## Boy Scout Rule — 翻訳可能性計画

- `load_memoized_graph` 内のハードコードされた初期値（`version: 0`, `alive: true`, `experience_count: 0` 等）が coordinator.rs:260-271 に散在している。これらを `MemoizedGraph::default()` または `Default` 実装に抽出することを検討（ただし #137 の E9 で既知）。
- GraphStore トレイトに追加する `store_reputation` / `load_reputation` の命名が動詞句であることを確認する。

## Acceptance Criteria

- [ ] GraphStore トレイトに `store_reputation` / `load_reputation` が追加されている
- [ ] `store_memoized_graph` が reputation を保存する
- [ ] `load_memoized_graph` が保存済み reputation を復元する
- [ ] 保存データがない場合、`cold_start()` にフォールバックする
- [ ] 保存/読込の失敗が non-fatal である
- [ ] 既存全テストが通過する

## 成果物

- 計画: context/0138-reputationprofile/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0138-reputationprofile/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0138-reputationprofile/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0138-reputationprofile/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
