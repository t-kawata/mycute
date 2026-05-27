---
ticket_id: 127
title: "M1.76-KW4: Kind World 較正ループ実行（J_kw_social = J_kw × s_speed）"
slug: m176-kw4-kind-world-j-kw-social-j-kw-s-speed
status: reviewed
created_at: 2026-05-27
updated_at: 2026-05-27
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0127-m176-kw4-kind-world-j-kw-social-j-kw-s-speed/plan.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0127-m176-kw4-kind-world-j-kw-social-j-kw-s-speed/observation-20260527-152120.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0127-m176-kw4-kind-world-j-kw-social-j-kw-s-speed/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0127-m176-kw4-kind-world-j-kw-social-j-kw-s-speed/review.md
---

# M1.76-KW4: Kind World 較正ループ実行（J_kw_social = J_kw × s_speed）

## Summary

KW-REAL (P1〜P6) で構築した SimulationContext ベースの 6 フェーズシミュレーション上で、Nelder-Mead 最適化による自動較正を実行する。目的関数は 6 因子乗算結合 $J_{kw}^{social} = J_{kw} \times s_{speed}$（RFC §15.9.2）。$J_{kw}$ は状態の質（5 因子: 成長・密度・トポロジ・探索・公平性）、$s_{speed}$ は速度因子（収束 tick 数の逆数）である。内外二重ループ（内側: Nelder-Mead 自動最適化、外側: 実験者による解釈・定数調整）で Kind World 条件 $J_{kw}^{social} > 0.64 \land \min(s_i) > 0.6$ を目指す。

## Background

KW-REAL P1〜P6 で以下の基盤が完成した：

- SimulationContext + 6 フェーズ tick ループ（simulation.rs）
- MTR-A〜E で全 23 指標の実測値化完了
- collect_final_metrics による KindWorldMetricsInput 出力
- compute_kind_world_objective による J_kw（5 因子乗算結合）評価

しかし現在の evaluate_single（kind_world.rs:2794）は $J_{kw}$（状態の質）のみを返し、速度因子 $s_{speed}$ を考慮していない。速度を無視した最適化は「ゆっくり良い状態に達する」パラメータと「速く良い状態に達する」パラメータを区別できない。本チケットは $J_{kw}^{social}$ を導入し、質と速度を同時に最適化する。

## Investigation

**updated at: 2026-05-27**

### evaluate_single が J_kw のみを返す（kind_world.rs:2794-2798）

```rust
fn evaluate_single(params: &MagnificentSevenParams, seed: u64) -> f64 {
    let config = params.to_sim_config(50, seed);
    let metrics = crate::simulation::run_evaluation_simulation(&config);
    compute_kind_world_objective(&metrics).j_kw
}
```

戻り値は `compute_kind_world_objective` の `j_kw` のみ。`j_kw` は 5 因子乗算結合（s_growth × s_density × s_topology × s_search × s_fairness）。$s_{speed}$ は含まれていない。この関数を $J_{kw}^{social}$ を返すように変更する必要がある。

### OptimizationReport に s_speed 関連フィールドがない（kind_world.rs:2664-2685）

```rust
pub struct OptimizationReport {
    pub best_params: MagnificentSevenParams,
    pub best_j_kw: f64,             // ← J_kw のみ。J_kw_social ではない
    pub assessment: KindWorldAssessment,
    pub iterations: u32,
    pub history: Vec<(MagnificentSevenParams, f64)>,
    pub converged: bool,
    pub experiment_id: String,
}
```

`tick_to_convergence` と `s_speed` のフィールドがない。`history` の `f64` 値も現在は J_kw であり、J_kw_social に変更される。

### NelderMeadOptimizer::run が J_kw でレポート作成（kind_world.rs:3150-3173）

```rust
let best_j_kw = self.values[0];  // 評価値 = evaluate_single の戻り値
// ...
OptimizationReport {
    best_params,
    best_j_kw,
    assessment,
    // ...
}
```

`self.values` は evaluate_single の戻り値を格納している。evaluate_single が J_kw_social を返すように変更されれば、自動的に best 値も J_kw_social となる。ただしレポートに tick_to_convergence と s_speed を明示的に格納するフィールドは別途追加する。

### run_evaluation_simulation が tick_to_convergence を計算していない（simulation.rs:1413-1554）

- 現在は `config.max_ticks` 回の tick ループを実行し、最終 tick で collect_final_metrics を呼ぶ
- `metric_series`（Vec<SimulationTickSnapshot>）は収集されているが未使用
- SimulationTickSnapshot は慈悲・互恵性・評判のパーセンタイルと生存率のみ。s_growth や j_cov を含まない
- Phase 6（phase6_measure_jkw, simulation.rs:1974）は最終 tick のみ実行される診断用

**tick_to_convergence の計算には、tick ループ中に s_growth と j_cov をサンプリングする機構が必要。**

### 既存の内側ループ出力形式に s_speed が含まれていない

tc6_kw4_optimize_run（kind_world.rs:6510）の CSV 出力ヘッダーは `iter,J_kw,gamma_benevolence,...` であり J_kw_social ではない。最終 JSON レポートも `report.best_j_kw` を参照している。

### compute_kind_world_objective はそのまま利用可能

`compute_kind_world_objective`（kind_world.rs:369）は `&KindWorldMetricsInput` → `KindWorldAssessment` の変換を行い、5 因子 j_kw を計算する。この関数自体は変更不要。$s_{speed}$ は evaluate_single 側で tick_to_convergence から計算し、j_kw と乗算する。

### constants.rs に KW4_OBSERVATION_INTERVAL がない

constants.rs（1010-1064）には KW4_SIMULATION_TICKS（100）はあるが、観測間隔や収束閾値の定数が定義されていない。

### is_kind_world 判定に s_speed が含まれていない（kind_world.rs:453）

```rust
let is_kind_world = j_kw > 0.8 && min_factor > 0.6;
```

較正ループの Kind World 判定は RFC に従い $J_{kw}^{social} > 0.64 \land \min(s_i) > 0.6$ に更新する必要がある。

## Scope

1. **constants.rs: 定数追加（2 件）**
   - `KW4_OBSERVATION_INTERVAL: u64 = 10` — サンプリング間隔（10 tick ごと）
   - `KW4_CONVERGENCE_THRESHOLD: f64 = 0.8` — s_growth × j_cov の収束閾値

2. **simulation.rs: tick_to_convergence 計算機構の追加**
   - `run_evaluation_simulation` の戻り値を `KindWorldMetricsInput` から `(KindWorldMetricsInput, u64)` に変更
   - または新たな構造体 `EvaluationOutput { metrics: KindWorldMetricsInput, tick_to_convergence: u64 }` を定義
   - tick ループ内で `KW4_OBSERVATION_INTERVAL` tick ごとに mid-simulation サンプリング:
     - j_pop_growth: 生存人口 / 初期人口 - 1.0
     - j_lifecycle: ctx.node_gc_states から生存ノードの LifecycleScore 平均
     - j_child_survival: 子ノードの生存割合
     - j_freshness: ctx.node_last_update_tick からの平均新鮮度
     - j_cov: ctx.positions から compute_capability_coverage
   - 各サンプルから `s_growth = (4 成分平均)`, `j_cov` を計算
   - `s_growth × j_cov > KW4_CONVERGENCE_THRESHOLD` を満たした最初の tick を tick_to_convergence として記録
   - 閾値未到達の場合は `tick_to_convergence = KW4_SIMULATION_TICKS`
   - 既存の collect_final_metrics 呼び出しはそのまま維持

3. **kind_world.rs: evaluate_single を J_kw_social 対応に変更**
   - `run_evaluation_simulation` から tick_to_convergence を受け取る
   - `s_speed = 1.0 - (tick_to_convergence as f64 / KW4_SIMULATION_TICKS as f64)`
   - `j_kw_social = j_kw * s_speed`（j_kw は既存の compute_kind_world_objective 結果）
   - 戻り値の型は `f64` のまま（値の意味が J_kw → J_kw_social に変わる）

4. **kind_world.rs: OptimizationReport にフィールド追加**
   - `best_j_kw_social: f64` — 最良 J_kw_social 値
   - `tick_to_convergence: u64` — 収束 tick 数
   - `s_speed: f64` — 速度因子
   - `best_j_kw` は互換性のため残す（非推奨注釈を追加）
   - `history` の f64 値も J_kw → J_kw_social に変更

5. **kind_world.rs: NelderMeadOptimizer::run のレポート更新**
   - 最終レポート生成時に評価関数呼び出しを追加（tick_to_convergence 取得のため）
   - OptimizationReport 構築時に tick_to_convergence, s_speed, best_j_kw_social を格納

6. **kind_world.rs: is_kind_world 閾値の更新**
   - `j_kw > 0.8` の条件を `j_kw_social > 0.64` に変更
   - 内部的には compute_kind_world_objective の後で別途 s_speed を乗算した値を判定に使用

7. **テスト更新（kind_world.rs tests モジュール）:**

   | ID | テスト | 種別 | 内容 |
   |----|--------|------|------|
   | TC1e | tick_to_convergence 範囲 | 不変条件 | 0 ≤ ttc ≤ KW4_SIMULATION_TICKS |
   | TC2e | s_speed 範囲 | 不変条件 | 0.0 ≤ s_speed ≤ 1.0 |
   | TC3e | evaluate_single が J_kw_social | 不変条件 | 戻り値が [0, 1] 範囲 |
   | TC4e | 決定論性（evaluate_single） | 不変条件 | 同一 params + seed で同一 J_kw_social |
   | TC5e | 決定論性（tick_to_convergence） | 不変条件 | 同一 params + seed で同一 ttc |
   | TC6e | tc6 CSV/JSON 更新 | 観測 | CSV に J_kw_social, s_speed, ttc |
   | TC7e | best_j_kw_social が正 | 観測 | 最適化後 best_j_kw_social > 0 |
   | TC8e | J_kw_social vs J_kw 比較 | 観測 | 両者の差（s_speed 影響）を出力 |
   | TC9e | 既存テスト全 PASS | 回帰 | 全既存テスト通過 |

## Non-scope

- compute_kind_world_objective の変更（既存の 5 因子計算はそのまま）
- run_evaluation_simulation の全体的なリファクタリング
- phase6_measure_jkw の変更（診断用のまま、ハードコード 0.5 はそのまま）
- 外側ループの自動化（本チケットは内側ループの基盤整備。外側ループは手動）
- 旧 spec ファイル（0112, 0120）の削除

## 計装方法・観測対象

### 計装方法

1. **mid-simulation サンプリング**: run_evaluation_simulation 内で KW4_OBSERVATION_INTERVAL（10）tick ごとに、SimulationContext から mid-simulation メトリクスをサンプリング。固定シード（12345）で全テストの再現性を保証。
2. **CSV 出力拡張**: tc6_kw4_optimize_run の CSV ヘッダー列を `J_kw` → `J_kw_social` に変更し、`s_speed`, `tick_to_convergence` 列を追加。
3. **JSON レポート拡張**: OptimizationReport に `best_j_kw_social`, `tick_to_convergence`, `s_speed` フィールドを追加。

### 観測対象

- $J_{kw}^{social}$ の値と内訳（J_kw 5 因子 + s_speed）
- tick_to_convergence の分布（何 tick で収束したか）
- s_speed による最適化結果の変化（s_speed なしの J_kw との比較）
- s_growth × j_cov の tick 経過推移（収束過程の可視化）

### 較正計画

- 調整定数: 7 パラメータ（MagnificentSevenParams）— 既存の探索範囲を流用
- 目的関数: $J_{kw}^{social}(\theta) = J_{kw}(\theta) \times s_{speed}(\theta)$
- 内側ループ停止条件: シンプレックス頂点間分散 < 1e-6 または最大 200 iteration
- Kind World 条件: $J_{kw}^{social} > 0.64 \land \min(s_{growth}, s_{density}, s_{topology}, s_{search}, s_{fairness}) > 0.6$

## Boy Scout Rule — 翻訳可能性計画

1. **OptimizationReport のベスト値フィールド**: `best_j_kw` を残しつつ `best_j_kw_social` を追加。ダブルメンテナンスを避けるため `best_j_kw` に `#[deprecated]` 注釈を付与。
2. **evaluate_single 関数**: 関数名は動詞句であり翻訳可能。戻り値の意味が J_kw → J_kw_social に変わることを doc comment に明記。
3. **run_evaluation_simulation**: 戻り値型が変わるため、呼び出し元（evaluate_single, NelderMeadOptimizer::run）の更新を漏れなく行う。

## Acceptance Criteria

1. [ ] evaluate_single が J_kw_social（5 因子 × s_speed）を返す
2. [ ] tick_to_convergence が [0, KW4_SIMULATION_TICKS] の範囲で計算される
3. [ ] OptimizationReport に best_j_kw_social, tick_to_convergence, s_speed が含まれる
4. [ ] 内側ループの CSV 出力に J_kw_social, s_speed, ttc が含まれる
5. [ ] Kind World 判定が $J_{kw}^{social} > 0.64 \land \min(s_i) > 0.6$ に更新されている
6. [ ] 同一パラメータ + 同一 seed で決定論的（J_kw_social も tick_to_convergence も再現する）
7. [ ] 既存テスト全 PASS

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

- 計画: context/0127-m176-kw4-kind-world-j-kw-social-j-kw-s-speed/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0127-m176-kw4-kind-world-j-kw-social-j-kw-s-speed/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0127-m176-kw4-kind-world-j-kw-social-j-kw-s-speed/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0127-m176-kw4-kind-world-j-kw-social-j-kw-s-speed/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
