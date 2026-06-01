---
ticket_id: 147
title: 首長性スコア導入 — 洗練スコア・首長選出・フロントエンド可視化
slug: untitled-3
status: reviewed
created_at: 2026-05-29
updated_at: 2026-05-29
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0147-untitled-3/plan.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0147-untitled-3/implementation.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0147-untitled-3/observation-20260529-140858.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0147-untitled-3/review.md
---
# 首長性スコア導入 — 洗練スコア・首長選出・フロントエンド可視化

## Summary

各個人の社会的影響力を測定する「首長性スコア」を導入する。既存の評判スコア (`ReputationProfile.final_score`) と新設する「洗練スコア」（抽象化割合 + 抽象化深度の加重和）を統合した複合スコアを計算し、村内で最もスコアの高い個体を首長として選出、ブラウザ可視化で首長を黒色で描画する。

## Background

前チケット#146 (出生意味論) の完了により、人口爆発が観測可能なシミュレーションが動作している。次の進化として、個人間の社会的影響力の差を定量化する必要が生じた。既存の評判スコア（互恵性に基づく）だけでは、ワークフローグラフの構造的複雑さ（抽象化能力）が反映されていない。そこで、グラフ構造から導出される「洗練スコア」と評判スコアを統合した「首長性スコア」を導入し、村ごとに首長を選出することで、社会階層の創発を観測可能にする。

## Scope

- `ReputationProfile` に `chiefdom_score: f32` フィールドを追加
- `src/graph_query.rs` に以下の4つの独立関数を追加:
  - `compute_abstraction_ratio` — 抽象化割合（total/surface）→ [0,1] 正規化
  - `compute_abstraction_depth` — 最大ネスト深度 → [0,1] 正規化
  - `compute_sophistication_score` — 洗練スコア（上記2つの加重和）
  - `compute_chiefdom_score` — 首長性スコア（評判 + 洗練の加重和）
- `simulation.rs` に Phase 3.7（首長性スコア計算）と Phase 3.8（首長選出）を追加
- `build_clock_advanced_event` の payload に `village_chiefs` マップを追加
- フロントエンド: 首長性中央値の表示、首長の黒色描画
- `compute_all_node_count` に依存する再帰深さ計算関数 `calculate_max_nest_depth` の実装
- `constants.rs` に `CHIEFDOM_DEPTH_SCALE` 定数追加

## Non-scope

- 首長の行動変更（首長が特別な HELP プロトコルを持つ等）は実装しない
- 首長性スコアの動的な重み調整（現時点では固定 50/50）
- 首長の交代・クーデター・任期などの政治的メカニズム
- 首長性スコアの継承（親から子への継承は既存の InheritedScore で対応済み）
- RFC への定義追加（将来のフェーズで実施）

## Investigation

既存コードの調査結果（計画策定時の分析）:

### 既存構造体

- `ReputationProfile` (`src/event.rs:436-471`): `final_score: f32`（評判スコア）は既存。`chiefdom_score` の格納先として自然。
- `MemoizedGraph` (`src/trust.rs:32-68`): `graph: WorkflowGraph`, `reputation: ReputationProfile`, `village_assignment: Option<VillageId>`, `alive: bool` を保持。首長性スコア計算に必要な全データを持つ。
- `WorkflowGraph` (`src/types.rs:95`): `DiGraph<WorkflowNode, EdgeMeta>` の型エイリアス。`SubWorkflow` バリアント（`WorkflowNode::SubWorkflow { workflow_id, .. }`）でネストを表現。

### 既存ユーティリティ関数

- `compute_all_node_count` (`src/graph_query.rs:26-50`): SubWorkflow を再帰的に辿り全ノード数を計算。visited set で循環参照保護。**本実装で最重要の既存資産**。
- `compute_mean_nest_depth` (`src/kind_world.rs:2294-2311`): スタブ実装（常に depth=1）。本実装では使用せず、新規に `calculate_max_nest_depth` を実装。

### シミュレーションループ

- `run_evaluation_simulation_with_channel` (`src/simulation.rs:1849`): サーバーモードのメインループ。Phase 3.5（line 1939）で評判再計算、Phase 3.6（line 1941）で自己抽象化。
- `graph_store` (`src/simulation.rs:1860`): `InMemoryGraphStore` のローカル変数が既に存在し、`compute_all_node_count` で使用中。
- `build_clock_advanced_event` (`src/simulation.rs:2082`): `store: &dyn GraphStore` を受け取り、per-node JSON を構築。

### ブラウザ可視化

- `script.js`: `onTickMetrics` (line 361) で per-node payload 受信。`updateStatsPanel` (line 581) で統計表示。`ageColor` (line 119) で色決定。
- `index.html`: 既存の統計パネル（benevolenceP50Val 等）に表示行を追加可能。

### 参照観測レポート

- `tickets/context/0146-graphregistrationfix-helpgmr/observation-20260529-121622.md` — 出生意味論の確認、人口爆発観測
- `tickets/context/0145-untitled-2/observation-20260529-123330.md` — ワークフロー複雑化メカニズム活性化、GMR発火率

## Test Plan

### compute_abstraction_ratio のテスト

| ID | ケース | 入力 | 期待値 |
|---|---|---|---|
| T1 | 空グラフ | node_count=0 | 0.0 |
| T2 | フラットグラフ（SubWorkflow なし） | surface=total | 0.0 |
| T3 | SubWorkflow あり | total > surface | 0.0 < x < 1.0 |
| T4 | 大きな比率 | total >> surface | 1.0 に近い値 |

### compute_abstraction_depth のテスト

| ID | ケース | 入力 | 期待値 |
|---|---|---|---|
| T5 | SubWorkflow なし | depth=0 | 0.0 |
| T6 | 単一 SubWorkflow | depth=1 | 1/(1+3)=0.25 |
| T7 | 2段ネスト chain | depth=2 | 2/(2+3)=0.4 |
| T8 | 循環参照（A→B→A） | visited set 保護 | Ok(有限値) |

### compute_sophistication_score のテスト

| ID | ケース | 期待値 |
|---|---|---|
| T9 | 空グラフ | 0.0 |
| T10 | フラットグラフ | 0.0 |
| T11 | SubWorkflow あり | (ratio_score + depth_score) / 2 |

### compute_chiefdom_score のテスト

| ID | ケース | final_score | sophistication | 期待値 |
|---|---|---|---|---|
| T12 | 最小値 | 0.0 | 0.0 | 0.0 |
| T13 | 最大値 | 1.0 | 1.0 | 1.0 |
| T14 | 混合 | 0.5 | 0.3 | 0.4 |
| T15 | 片方のみ | 0.8 | 0.0 | 0.4 |

### elect_village_chiefs のテスト

| ID | ケース | 期待値 |
|---|---|---|
| T16 | 単一村、複数個体 | chiefdom_score 最大の個体が選出される |
| T17 | 複数村 | 各村で最大個体が選出される |
| T18 | 村未割り当ての個体 | 首長にならない |
| T19 | 死亡個体 | 選出対象外 |
| T20 | 空人口 | 空の HashMap |

### エッジケース

- T8: 循環参照は visited set で保護され、スタックオーバーフローしない
- T17: 各村独立に選出される（村間の首長競合は発生しない）
- T19: 死亡により前 tick の首長がいなくなってもエラーにならない（次 tick で再選出）

## 計装方法・観測対象

### 計装方法

1. `graph_query.rs` のテストモジュール (`#[cfg(test)]`) に T1-T15 を実装（`InMemoryGraphStore` 使用、固定シード PRNG は不要な純粋関数テスト）
2. `simulation.rs` に首長性スコアの println! 観測出力を追加
3. ブラウザ可視化により統計パネルで首長性中央値を表示

### 観測対象

- 首長性スコアの分布（中央値・最大値・最小値）
- 首長数（= 村数）
- 首長の chiefdom_score と非首長の chiefdom_score の差
- 抽象化割合と抽象化深度の分布（洗練スコアの内訳）
- サンプルサイズ: シミュレーション実行時の各 tick の全生存個体

### 較正計画

- 調整定数: `CHIEFDOM_DEPTH_SCALE` (default: 3.0)
- 目的関数 J(θ): 首長選出の安定性（連続して同じ個体が首長に留まる tick 数）
- 停止条件: 首長が 10 tick 以上連続で維持されるパラメータ領域が見つかったら較正完了

## Boy Scout Rule — 翻訳可能性計画

- **4関数の独立定義**: `compute_abstraction_ratio` / `compute_abstraction_depth` / `compute_sophistication_score` / `compute_chiefdom_score` はそれぞれ独立した自由関数として `graph_query.rs` に定義。呼び出し列が日本語に逐語訳可能。
- **`calculate_max_nest_depth`**: `count_recursive` と同一パターンの再帰ヘルパー。visited set による循環参照保護は既存実装からコピーせず、同じパターンで新規実装。
- **`elect_village_chiefs`**: 副作用のない純粋関数。`population: &[MemoizedGraph]` を受け取り `HashMap<VillageId, PersonId>` を返す。引数で状態を入力し戻り値で結果を返す。
- **ハードコード値の禁止**: 深度正規化スケールは `constants.rs` の `CHIEFDOM_DEPTH_SCALE` 定数を使用。重み 0.5 はリテラル直書きだが、50/50 はプロビジョナルであり将来定数化を検討。

## Acceptance Criteria

- [ ] `ReputationProfile` に `chiefdom_score: f32` が追加されている
- [ ] `compute_abstraction_ratio` / `compute_abstraction_depth` / `compute_sophistication_score` / `compute_chiefdom_score` が独立関数として `graph_query.rs` に実装されている
- [ ] `calculate_max_nest_depth` が循環参照を安全に処理する（visited set 保護）
- [ ] Phase 3.7（首長性スコア計算）と Phase 3.8（首長選出）が `run_evaluation_simulation_with_channel` に追加されている
- [ ] `elect_village_chiefs` が各村の chiefdom_score 最大個体を正しく選出する
- [ ] `build_clock_advanced_event` の payload に `village_chiefs` マップが含まれる
- [ ] フロントエンドに首長性中央値が表示される
- [ ] 首長の円が黒色 (`0x000000`) で描画される
- [ ] `CHIEFDOM_DEPTH_SCALE` 定数が `constants.rs` に追加されている
- [ ] T1-T20 のテストが全て PASS する
- [ ] `cargo test` の既存テストに回帰がない
- [ ] `cargo check --features server` が通る

## Notes

- plan_path: 未作成
- implementation_path: 未作成
- review_report_path: 未作成
- observation_report_path: 未作成

### 成果物

- 計画: context/0147-untitled-3/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0147-untitled-3/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0147-untitled-3/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0147-untitled-3/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
