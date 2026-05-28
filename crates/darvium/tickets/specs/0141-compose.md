---
ticket_id: 141
title: ワークフロー生成パイプラインの完全実装: COMPOSE/変異/自己抽象化のシミュレーション統合
slug: compose
status: reviewed
created_at: 2026-05-28
updated_at: 2026-05-28
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0141-compose/implementation.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0141-compose/observation-20260528-162907.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0141-compose/review.md
---

# ワークフロー生成パイプラインの完全実装: COMPOSE/変異/自己抽象化のシミュレーション統合

## Summary

Phase A-F で実装したワークフロー生成パイプライン（`workflow_generation` / `SearchWorkflow` / `self_refinement` / `workflow_registry`）をシミュレーションと完全に統合する。

現状5つのギャップがある：

1. **COMPOSE がスタブ**: `try_compose()` が `CompositionPlan` を生成するだけで `compose_workflows()` を呼び出していない。シミュレーションでは `ComposeExisting` 結果がフォールバックに捨てられている。
2. **Self-Refinement が未配線**: `run_self_refinement_round()` がシミュレーションループから一度も呼ばれていない。
3. **初期人口が空グラフ**: シミュレーション開始時の成人全員が `WorkflowGraph::new()`（空）を持ち、可視化で半径 0 で描画される。
4. **Differential Mutation が未使用**: `generate_differential_mutation()` が `SearchWorkflow` からもシミュレーションからも呼ばれていない。
5. **PatchExisting がパッチ未適用**: `generate_workflow_for_child()` の `PatchExisting` 分岐が単なるクローンでパッチを適用していない。

## Background

チケット #137（Phase A-F: ワークフロー生成機構の実装）の検証段階で、5つのギャップが発見された。いずれも「動作はするが完全ではない」状態であり、RFC §4A.3 の人口増加メカニズム（Mechanism 16 NEW / Mechanism 17 Differential Inference / COMPOSE / Self-Refinement）を完全に満たすために修正が必要。

### 証拠

**Gap 1: COMPOSE スタブ**
- `search_workflow.rs:150-168` — `try_compose()` は `CompositionPlan::new()` の呼び出しのみで、`compose_workflows()`（`composition.rs:151`）を呼ばない
- `simulation.rs:2188` — `generate_workflow_for_child()` の `_` アームで `ComposeExisting` が `generate_new_workflow()` にフォールバックされる
- `composition.rs:151` — `compose_workflows(a: &WorkflowGraph, b: &WorkflowGraph) -> WorkflowGraph` は完成

**Gap 2: Self-Refinement 未配線**
- テスト (`self_refinement.rs:417,449`) と `lib.rs` の `pub use` のみ。シミュレーションループからの呼び出しなし
- 一方 `GED_GRAPH_SIZE_LIMIT = 50`, `MIN_ABSTRACTION_GROUP_SIZE = 3` は `constants.rs` に定義済み

**Gap 3: 初期人口空グラフ**
- `simulation.rs:1484-1501` (`run_kw_real_simulation`) — `add_person()` で生成した個人の `graph` は `WorkflowGraph::new()`（空）
- 同様に `run_evaluation_simulation`（line 1680-1697）、サーバー variant（line 1820-1835）も空
- `graph_query.rs` の `compute_all_node_count()` は `m.graph.node_count()` を合計するが、初期成人は 0

**Gap 4: Differential Mutation 未使用**
- `workflow_generation.rs:79-105` — `generate_differential_mutation()` は完成
- `SearchWorkflow` は `propose_new()` でのみ生成し、変異パスを持たない
- RFC §4A.3 Mechanism 17 に対応する動作がない

**Gap 5: PatchExisting 未適用**
- `simulation.rs:2182-2187` — `PatchExisting` の match arm が `registry.resolve(graph_id).map(|m| m.graph.clone())` で元グラフをクローンするだけ
- `patch.rs` に `apply_patch_atomic()` が存在するが呼ばれていない

## Scope

以下の5項目を実装する：

1. **COMPOSE 実装**: `SearchWorkflow::try_compose()` で `compose_workflows()` を呼び出し、合成結果を `SearchOutcome::ComposeExisting` に含める。シミュレーションの `generate_workflow_for_child()` で合成グラフを抽出する。
2. **Self-Refinement 配線**: シミュレーションのメインループ（`run_kw_real_simulation` / `run_evaluation_simulation` / サーバー variant）の各 tick 終了時に `run_self_refinement_round()` を全生存グラフに対して実行する。
3. **初期人口グラフ生成**: シミュレーション開始時の初期成人にも `SearchWorkflow` 経由で実ワークフローグラフを割り当てる。
4. **Differential Mutation 統合**: `SearchWorkflow` に変異パスを追加。`Retrieve` で類似候補が見つかり評価が閾値未満の場合、`generate_new_workflow` の代わりに `generate_differential_mutation` を試行する。
5. **PatchExisting 修正**: `generate_workflow_for_child()` の `PatchExisting` 分岐で `apply_patch_atomic()` を呼び出し、パッチを適用したグラフを返す。

## Non-scope

- RFC 文書の修正
- constants.rs のパラメータ変更（既存定数値はそのまま）
- 既存の SearchWorkflow FSM の再設計
- シミュレーション以外の呼び出し元（MYCUTE direct use）の修正
- COMPOSE 以外の新しい RFC メカニズムの追加

## Investigation

### 参照観察レポート

- `tickets/context/0137-untitled/observation-20260528-154420.md` — 事前 Phase 完了時の観測。ワークフローグラフが空であることが可視化で確認済み。

### ソースコード証拠

```
search_workflow.rs:150     try_compose → CompositionPlan::new() のみ、compose_workflows() 未呼び出し
search_workflow.rs:180     propose_new → StdRng::seed_from_u64(seed) （#49 でミッションハッシュ化済み）
search_workflow.rs:79-105  generate_differential_mutation() → SearchWorkflow から未使用
simulation.rs:2180         generate_workflow_for_child → _ アームで ComposeExisting を fallback
simulation.rs:1484-1501    初期人口ループ → add_person() のみ、graph は空
simulation.rs:1712,1860    2つの variant も同様
composition.rs:151         compose_workflows(a, b) → WorkflowGraph — 完成・未使用
patch.rs:106               apply_patch_atomic(graph, patch) → Result — 完成・未使用
self_refinement.rs:213     run_self_refinement_round → シミュレーションから未呼び出し
```

## Test Plan

### 全ギャップ共通（既存回帰テスト）

- `cargo test --package darvium` 全テスト通過
- `cargo clippy --package darvium -- -D warnings`

### Gap 1: COMPOSE 実装

| # | テスト | 種別 | 内容 |
|---|--------|------|------|
| C1 | 2つのワークフローを COMPOSE | 正常系 | `try_compose()` が `compose_workflows()` を呼び出し、合成グラフが両方のノードを含む |
| C2 | 空グラフとの COMPOSE | 境界値 | 片方が空グラフの場合、合成結果が非空グラフと等しい |
| C3 | 同一グラフの COMPOSE | 異常系 | 重複ノードが除去される（compose_workflows の重複排除） |

### Gap 2: Self-Refinement 配線

| # | テスト | 種別 | 内容 |
|---|--------|------|------|
| S1 | シミュレーション tick 後に抽象化実行 | 正常系 | グラフが `GED_GRAPH_SIZE_LIMIT` を超えた場合、ノード数が減少 |
| S2 | 抽象化不要時のスルー | 正常系 | グラフが閾値未満の場合、何も起こらない |
| S3 | Registry に SubWorkflow が登録される | 正常系 | Self-Refinement 後、レジストリに SubWorkflow が追加 |

### Gap 3: 初期人口グラフ生成

| # | テスト | 種別 | 内容 |
|---|--------|------|------|
| P1 | 初期人口の全員が空でないグラフを持つ | 正常系 | 初期化後、全 `MemoizedGraph.graph.node_count() > 0` |
| P2 | 初期人口のグラフが DAG | 不変条件 | 全グラフが `petgraph::algo::toposort` を通過 |

### Gap 4: Differential Mutation 統合

| # | テスト | 種別 | 内容 |
|---|--------|------|------|
| M1 | 変異が元グラフと異なる | 正常系 | 変異適用後、元グラフと構造が異なる |
| M2 | 変異後も DAG 性維持 | 不変条件 | 変異グラフが `toposort` を通過 |
| M3 | 空グラフの変異は空のまま | 境界値 | 変異元が空なら結果も空 |

### Gap 5: PatchExisting 修正

| # | テスト | 種別 | 内容 |
|---|--------|------|------|
| PA1 | パッチ適用で構造変化 | 正常系 | `PatchExisting` から `apply_patch_atomic()` 経由でパッチ適用 |
| PA2 | 無効パッチはエラー | 異常系 | 不正パッチで fallback 発動 |

## 計装方法・観測対象

### Gap 3 観測

```rust
let counts: Vec<usize> = ctx.population.iter().map(|p| p.graph.node_count()).collect();
println!("Initial population node_count distribution: min={} max={} avg={}",
    counts.iter().min().unwrap(),
    counts.iter().max().unwrap(),
    counts.iter().sum::<usize>() as f64 / counts.len() as f64);
```

### 使用シード

全観測テストで `StdRng::seed_from_u64(12345)` を使用し完全再現を保証する。

## Boy Scout Rule — 翻訳可能性計画

- `generate_workflow_for_child()` 内の巨大な match 式をアウトカム種別ごとのヘルパー関数に分割する
- `try_compose()` が `CompositionPlan` だけでなく実際の合成グラフも返すようにシグネチャ変更する
- シミュレーションの初期化部分（3箇所の類似ループ）の共通化を検討
- `phase1_population_growth()` 内にインラインで書かれたミッション生成と SearchWorkflow 呼び出しを、責務ごとに関数に抽出する

## Acceptance Criteria

- [ ] Gap 1 (COMPOSE): `try_compose()` が `compose_workflows()` を呼び出し、シミュレーションで合成グラフが利用される
- [ ] Gap 2 (Self-Refinement): シミュレーションループ内で `run_self_refinement_round()` が定期実行される
- [ ] Gap 3 (初期人口): シミュレーション開始時の全成人が `node_count > 0` のグラフを持つ
- [ ] Gap 4 (Differential Mutation): `SearchWorkflow` に変異パスが追加され使用される
- [ ] Gap 5 (PatchExisting): `generate_workflow_for_child()` が `PatchExisting` で `apply_patch_atomic()` を呼ぶ
- [ ] 全既存テスト: `cargo test --package darvium` が全パス（既存 1350 + 新規テスト）
- [ ] clippy: `cargo clippy --package darvium -- -D warnings` がクリーン

## Notes

### 成果物

- 計画: `context/0141-compose/plan.md`（未作成、`/plan-ticket` 承認後に作成）
- 実装サマリ: `context/0141-compose/implementation.md`（未作成、`/start-ticket` 実装完了後に作成）
- レビュー報告書: `context/0141-compose/review.md`（未作成、`/review-ticket` 全チェック通過後に作成）
- 観察レポート: `context/0141-compose/observation-YYYYMMDD-HHmmss.md`（未作成、`/start-ticket` 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
