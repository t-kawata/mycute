---
ticket_id: 120
title: M1.76-KW4-CAL: Kind World 較正継続（外側ループ 2-24）
slug: m176-kw4-cal-kind-world-2-24
status: done
created_at: 2026-05-27
updated_at: 2026-05-27
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0120-m176-kw4-cal-kind-world-2-24/plan.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0120-m176-kw4-cal-kind-world-2-24/observation-20260527-095403.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0120-m176-kw4-cal-kind-world-2-24/implementation.md
---

# M1.76-KW4-CAL: Kind World 較正継続（外側ループ 2-24）

## Summary

Kind World 較正ループの外側サイクルを継続する（最大 24 サイクル、現在 1/24 完了）。
既存の `evaluate_single`（ReciprocitySimulator 上の J_kw 評価）と `NelderMeadOptimizer` を基盤として、探索範囲の調整・結果解釈・定数変更を繰り返し、J_kw > 0.8 かつ全 5 因子 > 0.6（5 因子最小値ゲート）を達成する。

## Background

- **チケット #112**（M1.76-KW4: Kind World 較正ループ実行, `done`）: NelderMeadOptimizer、evaluate_single、OptimizationReport、kw4_optimize テスト（TC1-TC8）を実装。1 回の外側サイクルを実行し J_kw=0.2107 を記録したが、目標の 0.8 には遠く及ばなかった。
- **チケット #113-#118**（M1.76-KW-REAL P1-P6, `done`）: SimulationContext を用いた 6 フェーズシミュレーション基盤を構築。しかし #112 の evaluate_single は古い ReciprocitySimulator を使用しており、KW-REAL の SimulationContext を活用できていない。
- **チケット #119**（KW-ACCEL, `reviewed`）: J_kw を 5 因子乗算結合（20 下位成分）に再定義。6 つの新しいヘルパー関数（j_nest_depth, j_node_density, j_clustering, j_local_density, j_search_radius_inv, j_reasoning_steps_inv）を追加。しかし evaluate_single 経路ではこれらの指標が 0.0 fallback となり、実効的な J_kw が過小評価されている。

### 参照観察レポート

- `tickets/context/0112-m176-kw4-kind-world/observation-20260526-171302.md` — #112 初回較正結果（J_kw=0.2107, 4/8 フラグ）
- `tickets/context/0112-m176-kw4-kind-world/experiments.md` — 実験ログ（1 experiment 完了）
- `tickets/context/0119-m176-kw-accel-j-kw-57/observation-20260527-091630.md` — KW-ACCEL 較正観測結果

## Investigation

### 現行コードの状態（物理的証拠）

#### evaluate_single の経路（kind_world.rs:2389-2397）

```rust
fn evaluate_single(params: &MagnificentSevenParams, seed: u64) -> f64 {
    let config = params.to_sim_config(50, seed);
    let result = crate::simulation::run_simulation(&config);
    let metrics = collect_final_metrics_from_result(&result, config.population_size);
    compute_kind_world_objective(&metrics).j_kw
}
```

- `params.to_sim_config(50, seed)`（kind_world.rs:1849）: MagnificentSevenParams を ReciprocitySimulatorConfig に変換。固定人口 50。
- `run_simulation`（simulation.rs:1101）: ReciprocitySimulator 駆動。KW-REAL の SimulationContext（6 フェーズ、GMR、WorkflowGraph）は使用しない。
- `collect_final_metrics_from_result`（kind_world.rs:2068-2135）: 14/20 指標のみ ReciprocitySimulationResult から抽出可能。残り 6 指標は 0.0 fallback:

| 指標 | 値 | 理由 |
|------|-----|------|
| j_nest_depth | 0.0 | ReciprocitySimulator に HELP セッション履歴なし |
| j_node_density | 0.0 | WorkflowGraph ノードカウントなし |
| j_clustering | 0.5 fallback | 位置情報なし |
| j_local_density | 0.0 | 位置情報なし |
| j_search_radius_inv | 0.5 fallback | 探索半径記録なし |
| j_reasoning_steps_inv | 0.5 fallback | 推論ステップ記録なし |

#### collect_final_metrics（SimulationContext 版, kind_world.rs L2147+）

`#[allow(dead_code)]` が付与されており、現行コードでは未使用。こちらは 20 指標すべてを正しく計算する実装だが、`SimulationContext` を引数に取るため evaluate_single から呼び出せない。

#### 初回実験結果

- 実験 ID: kw4-1779783441-001
- J_kw = 0.2107（目標 0.8 の約 1/4）
- 収束 iteration: 3（異常に高速 — 局所最適もしくは 6 指標 fallback による見かけ上の収束の可能性）
- 4/8 レガシーフラグ

### 課題分析

1. **6/20 指標が 0.0 fallback**: evaluate_single が ReciprocitySimulator を使用しているため、KW-ACCEL で追加された 6 指標が実効的に 0.0 になる。乗算結合モデルでは 0.0 fallback が density/topology 因子の平均を押し下げ、J_kw 全体を過小評価する。
2. **SimulationContext への移行必要性**: KW-REAL（P1-P6）で SimulationContext が利用可能。evaluate_single を SimulationContext 版に書き換えれば 20 指標すべてが正しく計算される。
3. **外側ループの方針**: 上記の制約がある状態で外側ループを回しても真の J_kw が測定できない。したがって本チケットでは、(a) evaluate_single の SimulationContext 移行を先に行い、(b) その上で外側ループを継続する。

## Scope

### 実装スコープ（コード）

1. **evaluate_single の SimulationContext 移行**（kind_world.rs）:
   - `evaluate_single` を ReciprocitySimulator → SimulationContext（KW-REAL 6 フェーズ）に書き換え
   - `collect_final_metrics_from_result` → `collect_final_metrics`（SimulationContext 版）に切り替え
   - これにより 20 指標すべてが正しく計算される
   - 固定シード（StdRng::seed_from_u64(12345)）での決定論的再現性を維持
   - `NelderMeadOptimizer` の `evaluate_single` 呼び出しも自動的に更新される

2. **探索範囲の再調整**（constants.rs）:
   - 初回実験の結果を踏まえ探索範囲を見直し
   - 特に SimulationContext 移行後は J_kw の絶対値が変わるため、初期値も再設定

3. **外側ループ実行**（最大 23 サイクル）:
   - 各サイクル: 定数調整 → `cargo test`（内側ループ）→ 結果記録
   - 8 サイクルごと: 中間報告（平易な日本語）
   - 24 サイクル到達: 最終報告
   - J_kw > 0.8 かつ全 5 因子 > 0.6: Kind World 達成（早期終了）

4. **実験記録**（experiments.md）:
   - 各サイクルの結果を experiments.md に Markdown 形式で追記
   - 実験カウント管理

## Non-scope

- `compute_kind_world_objective` の計算ロジック修正
- SimulationContext 自体の修正（KW-REAL で実装済み）
- 既存テスト（TC1-TC8）の削除
- ベイズ最適化や勾配法等、Nelder-Mead 以外の最適化アルゴリズムの導入

## Test Plan

### ユニットテスト（既存 TC1-TC8 に追加）

| ID | テスト | アサーション |
|----|--------|-------------|
| TC9 | evaluate_single SimulationContext 移行後も決定論的 | 同一パラメータ + 同一シードで同一 J_kw |
| TC10 | evaluate_single SimulationContext 版で 6 指標が非ゼロ | j_nest_depth, j_node_density 等が 0.0 ではない |

### 観測テスト

| ID | テスト | 観測対象 |
|----|--------|---------|
| OBS3 | kw4_optimize（SimulationContext 版）収束曲線 | J_kw 時系列、5 因子内訳、20 下位成分 |
| OBS4 | 探索範囲変更による収束点変化 | 範囲変更が最適値に与える影響 |

## 計装方法・観測対象

`tc6_kw4_optimize_run`（既存 #112 実装）の出力形式を踏襲しつつ、5 因子内訳を追加：

```
=== kw4_optimize [experiment_id=kw4-CAL-{seq}] ===

--- Nelder-Mead iteration history ---
iter,J_kw,s_growth,s_density,s_topology,s_search,s_fairness,gamma_benevolence,...
0,0.4213,0.55,0.38,0.62,0.31,0.95,0.1500,...
...

--- Final Report (JSON) ---
{
  "experiment_id": "kw4-CAL-001",
  "best_j_kw": 0.7246,
  "iterations": 128,
  "converged": true,
  "assessment": {
    "is_kind_world": false,
    "j_kw": 0.7246,
    "s_growth": 0.85, "s_density": 0.72,
    "s_topology": 0.81, "s_search": 0.51, "s_fairness": 0.95,
    "min_factor": 0.51
  }
}
```

**観測対象**: 収束曲線、5 因子内訳と最小値ゲート、20 下位成分値。

### 較正計画

- **調整対象定数**: KW4_*_RANGE 系列の 7 定数
- **目的関数**: J_kw（5 因子乗算結合）
- **成功条件**: J_kw > 0.8 かつ全 5 因子 > 0.6
- **打ち切り条件**: 外側サイクル 24 回、または成功条件達成

## Boy Scout Rule — 翻訳可能性計画

**スコープ内**:
- `evaluate_single` の SimulationContext 移行に伴い、関数シグネチャを確認・更新
- `#[allow(dead_code)]` が付与された `collect_final_metrics`（SimulationContext 版）の属性を除去

## Acceptance Criteria

1. **evaluate_single 移行**: evaluate_single が SimulationContext（KW-REAL 6 フェーズ）を使用する
2. **20 指標完全計算**: 全 20 指標が 0.0 fallback なしで計算される
3. **決定論的再現性**: 同一パラメータ + 同一シードで同一 J_kw
4. **後方互換性**: 既存 TC1-TC8 が全 PASS
5. **外側ループ記録**: 各 cargo test の結果が experiments.md に追記される
6. **中間報告**: 8 サイクルごとに平易な日本語で中間報告
7. **Kind World 達成または打ち切り**: J_kw > 0.8 + min(s_i) > 0.6、または 24 サイクル到達

## Notes

- plan_path: /plan-ticket が plan.md 作成後に frontmatter に更新
- implementation_path: /start-ticket が implementation.md 作成後に frontmatter に更新
- review_report_path: /review-ticket が review.md 作成後に frontmatter に更新
- observation_report_path: /start-ticket が observation-YYYYMMDD-HHmmss.md 作成後に frontmatter に最新パスを更新

### 成果物

- 計画: context/0120-m176-kw4-cal-kind-world-2-24/plan.md（未作成）
- 実装サマリ: context/0120-m176-kw4-cal-kind-world-2-24/implementation.md（未作成）
- レビュー報告書: context/0120-m176-kw4-cal-kind-world-2-24/review.md（未作成）
- 観察レポート: context/0120-m176-kw4-cal-kind-world-2-24/observation-YYYYMMDD-HHmmss.md（未作成）
