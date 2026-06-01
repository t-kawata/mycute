---
ticket_id: 149
title: シミュレーション性能最適化 — 生存者インデックス + node_countキャッシュ + 密度計算除去
slug: node-count
status: reviewed
created_at: 2026-06-01
updated_at: 2026-06-01
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0149-node-count/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0149-node-count/review.md
---
# シミュレーション性能最適化 — 生存者インデックス + node_countキャッシュ + 密度計算除去

## Summary

シミュレーション（`run_evaluation_simulation_with_channel`）が人口増加に伴い著しく低速化する問題を修正する。3つの対策を実施する：
1. **`node_count` キャッシュ**: 毎 tick 全生存者のグラフを再帰走査するのをやめ、キャッシュを参照する
2. **`alive_ids()` 呼び出し集約**: 各 tick 先頭で1回だけ生存者リストを構築し、全フェーズに渡す
3. **村密度計算の完全削除**: サーバー側（simulation.rs）とフロントエンド側（script.js, index.html）両方の O(m²) 距離計算を全て除去する

## Background

`make run-observation` において、人口 2488 の時点で1 tick あたりの処理時間が数秒に悪化することを確認した。3つのシミュレーションループすべてに共通するパフォーマンス劣化の原因:

- **`build_clock_advanced_event`**: 毎 tick、全生存者に対してグラフ再帰走査 + 村密度 O(m²) 計算を実行（simulation.rs:2422-2500）
- **`alive_ids()`**: 7箇所から呼ばれ、毎回死者を含む全人口 Vec をスキャンしてフィルタリング
- **ComputeAllNodeCount**: グラフ内容が不変でも毎 tick 再帰 SubWorkflow 解決を実行

## Scope

1. **`MemoizedGraph.cached_node_count` 追加**: graph_query.rs の代わりにキャッシュフィールドを読み取る。Phase 5 拡散時・出生時にのみ更新。
2. **`alive_ids()` の1回呼び出し**: 各 tick 先頭で生存者リストを作成し、引数として全フェーズに渡す。関数シグネチャの変更あり。
3. **村密度計算の完全削除**:
   - サーバー: `build_clock_advanced_event` 内の densities 構築ブロック（simulation.rs:2478-2495）を削除
   - フロントエンド: `script.js` の `computeVillageDensities` 関数を削除（polygonArea, convexHull, cross も使用箇所がなければ併せて削除）
   - フロントエンド: `index.html` の `<div id="densityList">` を削除
   - フロントエンド: `updateStatsPanel` 内の densityList 更新コードを削除
   - フロントエンド: `clearVisualization` 内の densityList クリア行を削除

## Non-scope

- 空間グリッドインデックスの導入（別チケット）
- セッションインデックス（Bottleneck #1: 既に修正済み）
- k-means 反復回数の調整（既に修正済み）
- `phase2_village_clustering` のアルゴリズム変更

## Investigation

### 現象の確認

`make run-observation` で人口 2488 に到達した時点で、Phase5: diffusions の1行出力間隔が数秒にまで悪化することを確認。初期 tick ではミリ秒単位で出力されるのに対し、時間経過とともに単調に悪化する。

### 原因分析

3つのシミュレーションループ（`run_kw_real_simulation` `run_evaluation_simulation` `run_evaluation_simulation_with_channel`）の全 trace から、以下のボトルネックを特定。

#### ボトルネック A: `build_clock_advanced_event` — O(n × nodecount) 毎tick

**ファイル**: `src/simulation.rs:2422-2500`
**呼び出し**: `run_evaluation_simulation_with_channel` の各 tick（`:2324`）

毎 tick 以下を実行する：
1. **全生存者に対して `compute_all_node_count` を呼ぶ**（`:2437-2459`）— 2488人が毎 tick 再帰グラフ走査
2. **全生存者をもう一度スキャン**して村グループを構築（`:2467-2474`）
3. **村ごとに全ペア平均距離を計算**（`:2478-2495`）— O(m²)

`compute_all_node_count`（`src/graph_query.rs:28-52`）は `count_recursive` を呼び出し、グラフ内の SubWorkflow ノードを再帰的に解決しながら総ノード数を計算する。これは各生存者のグラフ構造に依存するため軽くはない。

**問題点**: 生存者の `node_count` はグラフの内容が変わらない限り変化しない。GMR 拡散が発生しない tick では前回と同じ値になる。それにもかかわらず毎 tick 再計算している。

#### ボトルネック B: 全人口スキャンの累積 — O(total_pop) × 多数箇所

`ctx.population`（`Vec<MemoizedGraph>`）は出生により要素数が増加するが、死亡した個体は削除されない（`alive = false` のみ）。そのため `total_population` は単調増加し、毎 tick 以下の箇所で全スキャンが発生する：

| 呼び出し元 | ファイル行 | スキャン対象 | フィルタ |
|---|---|---|---|
| `alive_ids()` | `simulation.rs:458-464` | 全人口 → 生存者Vec | filter(alive) |
| `phase1_population_growth` | `simulation.rs:2633-2701` | 全人口 → 生存成人 | filter(alive + adult) |
| `phase2_village_clustering` | `simulation.rs:2719-2723` | `alive_ids()` | — |
| `compute_village_centrality` | `simulation.rs:2868-2910` | `alive_ids()` | — |
| `phase3_help_protocol` | `simulation.rs:2970-3002` | `alive_ids()` → 成人/子分割 | filter |
| `recompute_reputation_for_population` | `simulation.rs:1020-1036` | filter(alive) | — |
| `phase3_chief_movement`内 首長性計算 | `simulation.rs:2256-2267` | filter(alive) | — |
| `run_self_refinement_for_population` | `simulation.rs:2612-2616` | `alive_ids()` | — |
| `phase4_gc_survival` | `simulation.rs:3169-3171` | `alive_ids()` | — |
| `observe_kw_real_tick` | `simulation.rs:3595-3630` | `alive_ids()` | — |
| `check_convergence` | `simulation.rs:1945-1983` | `gc_states_map()` + `last_update_ticks_map()` + `positions_map()` | 各々全人口→HashMap |

`alive_ids()` は単一箇所で7回呼ばれ、各々が全人口 Vec を `iter().filter(alive).collect()` している。この `Vec<PersonId>` のアロケーションも各 tick で発生。

#### ボトルネック C: 村密度計算の全ペア距離 — O(m²)

`build_clock_advanced_event`（`simulation.rs:2478-2495`）内で、各村の生存者数 m に対して O(m²) の距離計算を行っている。これはフロントエンドの密度表示（`script.js` の `computeVillageDensities`）のみに使用される。フロントエンド側でも同様の計算をしていて二重。

#### ボトルネック D: 空間距離計算の非効率

`phase2_village_clustering`（k-means）、`phase3_chief_movement`（首長間距離）、`compute_village_centrality`（重心からの距離）がそれぞれ独立に距離計算を行っている。共通の空間インデックスがないため、同じ距離が複数回計算される。

### 証拠の物理的トレース

1. **`simulation.rs:458-464`**: `alive_ids()` が全人口スキャン — 2488生存でも Vec は死者含め4000+エントリ
2. **`simulation.rs:2422-2500`**: `build_clock_advanced_event` 内で `compute_all_node_count` × 生存者全員
3. **`simulation.rs:2478-2495`**: 村密度計算の O(m²) 距離計算 — フロントエンド表示のみ
4. **`simulation.rs:2467-2474`**: 村グループ構築の2回目の全生存者スキャン
5. **`graph_query.rs:28-52`**: `compute_all_node_count` の再帰グラフ走査 — 生存者のグラフが未変更なら毎 tick 同じ値

### 修正方針（案）

**Step A**: `MemoizedGraph` に `cached_node_count: usize` フィールドを追加し、グラフ変更時（Phase 5 拡散時・出生時）のみ更新する。`build_clock_advanced_event` での呼び出しをキャッシュ参照に置き換え。`build_node_created_event` も同様。

**Step B**: `SimulationContext` に `alive_indices: Vec<PersonId>` を追加し、生存者のリストをキャッシュする。生死変更時（Phase 1 出生、Phase 4 GC死亡）のみ差分更新。`alive_ids()` をこのキャッシュ返却に変更。または `alive_ids()` の呼び出しを集約し、各 tick 先頭で1回だけ `alive_ids()` を呼び、各フェーズに結果を渡す。

**Step C**: `build_clock_advanced_event` から村密度計算ブロック（`:2478-2495`）を削除する。同時にフロントエンド側の `computeVillageDensities` と密度表示UIも全て削除する（二重計算であり、密度表示は重要度が低い）。

**Step D**（将来）: 空間グリッド（固定解像度 3D ハッシュグリッド）の導入。

## Test Plan

- **T1**: `MemoizedGraph` の `cached_node_count` が初期化直後に `compute_all_node_count` と一致する
- **T2**: `cached_node_count` がグラフ変更後（`try_gmr_diffusion` 経由の出生後）に正しく更新される
- **T3**: `alive_indices` キャッシュが `alive_ids()` の結果と一致する（Phase 1 出生後・Phase 4 死亡後）
- **T4**: `build_clock_advanced_event` から密度計算ブロック削除後のイベント形式がフロントエンド互換である
- **T5**: フロントエンドで `computeVillageDensities` 削除後も統計パネル（Tick・人口等）が正常に表示される

## 計装方法・観測対象

<!--
Darvium は観測ベース検証（Observational Testing First）を基本とする。
このセクションでは計装と観測対象を定義する。

### 計装方法
- どのテストコードで計装を実装するか
- どのような計測プローブを仕掛けるか（println! + --nocapture 等）
- 固定シード PRNG（StdRng::seed_from_u64(12345)）を使用するか

### 観測対象
- 観測する統計量（平均・分散・エントロピー・分布形状等）
- サンプルサイズの要件（分布同定 n >= 10,000、ドリフト検出 n >= 1,000）
- 期待される現象（不変条件として assert すべき性質と、観測として記録すべき傾向）

### 較正計画
- 調整する定数（constants.rs の該当定数）
- 目的関数 J(θ) の設計（収束速度・定常誤差・オーバーシュート等の合成評価）
- 較正ループの停止条件
-->

## Boy Scout Rule — 翻訳可能性計画

- `build_clock_advanced_event` を分割: イベント構築と密度計算は責務が異なる
- `alive_ids()` の7回呼び出しを集約し、呼び出し元の関数シグネチャを明示的にする

## Acceptance Criteria

- [ ] `build_clock_advanced_event` が密度計算ブロックを含まない
- [ ] `script.js` に `computeVillageDensities` が存在しない
- [ ] `index.html` に `densityList` が存在しない
- [ ] 各 tick の先頭で1回だけ生存者リストを構築し、全フェーズに結果を共有する
- [ ] `node_count` がキャッシュから読み取られ、グラフ未変更 tick で再帰走査が発生しない
- [ ] 既存テストがすべて通過する（`cargo test`）
- [ ] `cargo check --features server` が警告ゼロ

## Notes

<!--
注: このコメントは人間向けの説明である。AI は以下の手順に従うこと。

- plan_path: /plan-ticket が plan.md を作成後に frontmatter に更新する
- implementation_path: /start-ticket が implementation.md を作成後に frontmatter に更新する
- review_report_path: /review-ticket が review.md を作成後に frontmatter に更新する
- observation_report_path: /start-ticket が observation-YYYYMMDD-HHmmss.md を作成後に frontmatter に最新パスを更新する

各コマンドのワークフロー手順が frontmatter 更新の正しい手順である。
-->

### 成果物

- 計画: context/0149-node-count/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0149-node-count/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0149-node-count/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0149-node-count/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
