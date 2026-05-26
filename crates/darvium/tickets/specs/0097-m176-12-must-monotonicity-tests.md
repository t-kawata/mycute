---
ticket_id: 97
title: M1.76-12: 単調性テストスイート（MUST monotonicity tests）
slug: m176-12-must-monotonicity-tests
status: reviewed
created_at: 2026-05-26
updated_at: 2026-05-26
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0097-m176-12-must-monotonicity-tests/observation-20260526-104115.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0097-m176-12-must-monotonicity-tests/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0097-m176-12-must-monotonicity-tests/review.md
---

# M1.76-12: 単調性テストスイート（MUST monotonicity tests）

## Summary

Reciprocity-Aware Survival の全 MUST 単調性条件を一元的に定義・検証する `MonotonicityTestSuite` を実装する。既存の散在する単調性テストを参照しつつ、4種の MUST 条件を体系的に sweep + ランダムパラメータ検証する統合的テストスイートを追加する。

## Background

RFC §41B.20.8 Testing discipline は以下の MUST 単調性条件を規定する：

1. 他条件一定で `direct_score` 増加 → `survival_probability` 非減少
2. 他条件一定で `indirect_score` 増加 → `GC hazard` 非増加
3. 他条件一定で `Reputation` 増加 → `GC hazard` 非増加
4. 同能力 helper 間で benevolence が高い方が ranking で不利にならない

これらの条件は現在、個別の関数テストとして散在している（`test_benevolence_monotonic`, `test_help_succeeded_monotonic_increase` 等）。本チケットではこれらを統一的に扱う `MonotonicityTestSuite` を実装し、全条件の体系的検証と計装を提供する。

前チケット M1.76-11（ReciprocityEvent インジェスション + reputation/hazard recompute パイプライン）が完了し、`recompute_all_profiles` / `recompute_all_gc_hazards` / `compute_replay_comparison` が実装済み。本チケットはこのパイプラインの数学的正しさ（単調性）を全 MUST 条件に対して検証する。

### 参照観察レポート

- `tickets/context/0096-m176-11-reciprocity-event-ingestion-reputation-hazard-recompute-pipeline/observation-20260526-101011.md` — パイプラインは決定論的で、全 hazard 非負、応答曲面が観測済み。単調性検証の基盤として使用可能。
- `tickets/context/0095-m176-10-child-growth-increment-f-14-maturation-probability-f-15/observation-20260526-091721.md` — F-14/F-15 実装完了。

## Investigation

### 既存コード調査結果

**src/reciprocity.rs** に以下の単調性テストが既存（散在）：

| 行 | テスト関数 | 検証内容 |
|---|-----------|---------|
| L923 | `test_help_succeeded_monotonic_increase` | HelpSucceeded 追加で直接互恵性スコア単調増加 |
| L962 | `test_harmful_mismatch_monotonic_decrease` | HarmfulMismatch 追加で直接互恵性スコア単調減少 |
| L1171 | `test_centrality_monotonic_increase` | 中心性 sweep で間接互恵性スコア単調増加 |
| L1194 | `test_harm_score_monotonic_decrease` | 負評価 sweep で間接互恵性スコア単調減少 |
| L1396 | `test_recompute_direct_score_monotonic` | direct_score sweep で final_score 単調非減少 |
| L1424 | `test_recompute_indirect_score_monotonic` | indirect_score sweep で final_score 単調非減少 |
| L1526 | `test_recompute_inherited_score_monotonic` | inherited_score sweep で final_score 単調非減少 |
| L1466 | `test_experience_norm_asymptotic` | count 増加で E_norm 非減少 |
| L1680 | `test_benevolence_monotonic` | benevolence_score ↑ → hazard 単調非増加 |
| L1702 | `test_lifecycle_monotonic` | lifecycle_score ↑ → hazard 単調非増加 |
| L1737 | `test_hazard_positive_survival` | Δt 増加で P_survive 単調減少 |
| L1943 | `test_help_received_monotonic` | help_received ↑ → C_protect 単調非減少 |
| L1964 | `test_growth_improvement_monotonic` | growth ↑ → C_protect 単調非減少 |

**不足している検証（本チケットで追加）**:
- `direct_score → survival_probability` の直接結合: 現状は別々にテストされている
- `indirect_score → GC hazard` の直接結合
- `Reputation → GC hazard` の直接結合
- `benevolence → helper ranking` の単調性
- ランダムパラメータ sweep での単調性不変条件検証

**依存する公開関数**（既存、src/reciprocity.rs）:
- `compute_direct_reciprocity(events, now, policy) -> f32` (L72)
- `compute_indirect_reciprocity(centrality, village, accepted, success, harm) -> f32` (L122)
- `compute_benevolence_score(direct, indirect, reputation) -> f32` (L158)
- `recompute_reputation(inputs, policy) -> ReputationProfile` (L223)
- `compute_gc_hazard(lifecycle, benevolence, child_protection, policy) -> f32` (L288)
- `compute_survival_probability(hazard, delta_t) -> f64` (L331)
- `compute_helper_quality_score(..., policy) -> f32` (L408)
- `softmax_helper_selection(scores, policy) -> Vec<f64>` (L452)

**実装すべき新規構造体**（grep で未発現を確認）:
- `MonotonicityTestSuite` — 未存在
- `MonotonicityCondition` — 未存在
- `check_monotonicity` — 未存在
- `MonotonicityReport` — 未存在

## Scope

1. `MonotonicityTestSuite` 構造体を `src/reciprocity.rs` に追加（全 MUST 単調性条件の定義と自動検証器）
2. `MonotonicityCondition` 列挙型（4 variant）の定義
3. `check_monotonicity(suite: &MonotonicityTestSuite) -> MonotonicityReport` 関数の実装
4. `MonotonicityReport` 構造体の定義（conditions_passed + failure_details）
5. 以下の 4 条件を個別テスト関数として `mod tests` 内に実装:
   - `test_direct_score_survival_monotonicity`
   - `test_indirect_score_gc_hazard_monotonicity`
   - `test_reputation_gc_hazard_monotonicity`
   - `test_benevolence_helper_ranking_monotonicity`
6. ランダムパラメータ sweep n=1000 での不変条件検証（条件1〜4 全て）
7. 条件4 の benevolence 差 ΔB sweep [0.001, 0.5] で ranking reversal 臨界閾値検出
8. 計装: 全条件のパス/フェイルを println! で観測出力

## Non-scope

- 決定論的リプレイテスト（MUST）は M1.76-13 のスコープ
- 摂動テスト（SHOULD）は M1.76-14 のスコープ
- プロパティベースファジング（SHOULD）は M1.76-15 のスコープ
- 較正ループ J(θ) は M1.76-16 のスコープ
- 既存の散在する単調性テストは削除しない（本スイートは追加のみ）

## Test Plan

### 実装構造

```rust
/// 単調性条件の列挙型。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MonotonicityCondition {
    /// 条件 1: direct_score 増加 → survival_probability 非減少
    DirectScoreIncrease,
    /// 条件 2: indirect_score 増加 → GC hazard 非増加
    IndirectScoreIncrease,
    /// 条件 3: Reputation (final_score) 増加 → GC hazard 非増加
    ReputationIncrease,
    /// 条件 4: 同能力 helper 間で高 benevolence が ranking で不利にならない
    BenevolenceHelperRanking,
}

/// 単調性テスト結果レポート。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonotonicityReport {
    /// 条件ごとの PASS/FAIL
    pub conditions_passed: Vec<(MonotonicityCondition, bool)>,
    /// 違反の詳細説明（空 = 全 PASS）
    pub failure_details: Vec<String>,
    /// ランダム sweep の違反発生率（条件ごと）
    pub random_sweep_violation_rates: HashMap<MonotonicityCondition, f64>,
}

/// 単調性テストスイート — 全 MUST 条件の定義と自動検証。
#[derive(Debug, Clone)]
pub struct MonotonicityTestSuite {
    pub direct_score_points: Vec<f32>,
    pub indirect_score_points: Vec<f32>,
    pub reputation_points: Vec<f32>,
    pub benevolence_delta_points: Vec<f32>,
    pub fixed_params: MonotonicityFixedParams,
    pub random_sweep_samples: usize,
}

#[derive(Debug, Clone)]
pub struct MonotonicityFixedParams {
    pub lifecycle_score: f32,
    pub child_protection: f32,
    pub delta_t: u64,
    pub policy: ReciprocityLifecyclePolicy,
}

pub fn check_monotonicity(suite: &MonotonicityTestSuite) -> MonotonicityReport { ... }
```

### 条件 1: direct_score → survival_probability

```
direct_score points: [0.0, 0.25, 0.5, 0.75, 1.0]
各点で:
  benevolence = compute_benevolence_score(direct_score, 0, 0)
  hazard = compute_gc_hazard(lifecycle, benevolence, 0, policy)
  survival = compute_survival_probability(hazard, delta_t)
→ survival 系列が非減少であることを assert
ランダム sweep n=1000: lifecycle, delta_t をランダム化
```

### 条件 2: indirect_score → GC hazard

```
indirect_score points: [0.0, 0.25, 0.5, 0.75, 1.0]
各点で:
  benevolence = compute_benevolence_score(0, indirect_score, 0)
  hazard = compute_gc_hazard(lifecycle, benevolence, 0, policy)
→ hazard 系列が非増加であることを assert
ランダム sweep n=1000: lifecycle, delta_t, child_protection をランダム化
```

### 条件 3: Reputation → GC hazard

```
reputation points: [0.0, 0.25, 0.5, 0.75, 1.0]
各点で:
  benevolence = compute_benevolence_score(0.5, 0.5, reputation)
  hazard = compute_gc_hazard(lifecycle, benevolence, 0, policy)
→ hazard 系列が非増加であることを assert
ランダム sweep n=1000: lifecycle, delta_t, child_protection をランダム化
```

### 条件 4: benevolence → helper ranking

```
固定パラメータ: S=0.5, T=0.5, Rep=0.5, N=0.5, d=0.1
helper_A: benevolence=0.3
helper_B: benevolence=0.9

Q_A = compute_helper_quality_score(S, T, Rep, 0.3, N, d, policy)
Q_B = compute_helper_quality_score(S, T, Rep, 0.9, N, d, policy)
probs = softmax_helper_selection(&[Q_A, Q_B], policy)
→ probs[1] >= probs[0] (高 benevolence が不利にならない)

ΔB sweep [0.001, 0.5]:
  各 ΔB で helper_A(B=0.5-ΔB/2), helper_B(B=0.5+ΔB/2)
  → ranking reversal なし
```

## 計装方法・観測対象

### 計装方法

- 全テスト関数内で `println!` + `--nocapture` により観測出力
- 固定シード PRNG: `StdRng::seed_from_u64(12345)`
- ランダム sweep: n=1000（全条件共通）
- 4 条件すべての PASS/FAIL をブール値として `MonotonicityReport` に記録
- `MonotonicityReport.failure_details` に違反詳細を格納（空なら全 PASS）

### 観測対象

| 観測量 | 期待値 | 方法 |
|--------|--------|------|
| 条件1 違反発生率 | 0 | sweep n=1000 中の違反数 / n |
| 条件2 違反発生率 | 0 | sweep n=1000 中の違反数 / n |
| 条件3 違反発生率 | 0 | sweep n=1000 中の違反数 / n |
| 条件4 違反発生率 | 0 | sweep n=1000 中の違反数 / n |
| ranking reversal 発生 ΔB 閾値 | 無し | ΔB sweep [0.001, 0.5] 全域で 0 |
| survival_probability 曲線 | 非減少 | 5 点 sweep の出力系列確認 |
| GC hazard 曲線 | 非増加 | 5 点 sweep の出力系列確認 |

### 較正計画

本チケットはテストスイートの実装であり、constants.rs の定数は変更しない。
単調性違反が検出された場合のみ constants.rs の該当定数（α, β, γ 等）見直しを M1.76-16 較正フェーズに提案する。

## Boy Scout Rule — 翻訳可能性計画

- `MonotonicityCondition` variant 名はドメイン概念を直接表現（`DirectScoreIncrease` 等）
- `check_monotonicity` は「suite → report」の一責務
- 各テスト関数は動詞句で命名（条件の意味が自明）
- 条件 4 の helper 比較では `helper_a_prob`, `helper_b_prob` で可読性確保
- エラー握りつぶし禁止: `MonotonicityReport.failure_details` に必ず記録
- 既存の散在単調性テストとの重複を避け、本スイートは純粋に追加

## Acceptance Criteria

- [ ] `MonotonicityTestSuite` + `MonotonicityCondition` + `MonotonicityReport` + `check_monotonicity` が実装されている
- [ ] 条件1: direct_score ↑ → survival_probability 非減少が 5 点 sweep + n=1000 ランダムで検証されている
- [ ] 条件2: indirect_score ↑ → GC hazard 非増加が 5 点 sweep + n=1000 ランダムで検証されている
- [ ] 条件3: Reputation ↑ → GC hazard 非増加が 5 点 sweep + n=1000 ランダムで検証されている
- [ ] 条件4: 高 benevolence helper が ranking で不利にならないことが検証されている
- [ ] 条件4 ΔB sweep [0.001, 0.5] で ranking reversal が発生しないこと
- [ ] `MonotonicityReport` に failure_details が正しく記録される
- [ ] 既存の全テストが通過している
- [ ] `cargo test -- --nocapture` で観測出力が確認できる
- [ ] RFC §41B.20.8 の MUST monotonicity tests と無矛盾

## Notes

- plan_path: context/0097-m176-12-must-monotonicity-tests/plan.md（未作成、/plan-ticket 承認後に作成）
- implementation_path: context/0097-m176-12-must-monotonicity-tests/implementation.md（未作成、/start-ticket 実装完了後に作成）
- review_report_path: context/0097-m176-12-must-monotonicity-tests/review.md（未作成、/review-ticket 全チェック通過後に作成）
- observation_report_path: context/0097-m176-12-must-monotonicity-tests/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成）

### 成果物

- 計画: context/0097-m176-12-must-monotonicity-tests/plan.md（未作成）
- 実装サマリ: context/0097-m176-12-must-monotonicity-tests/implementation.md（未作成）
- レビュー報告書: context/0097-m176-12-must-monotonicity-tests/review.md（未作成）
- 観察レポート: context/0097-m176-12-must-monotonicity-tests/observation-YYYYMMDD-HHmmss.md（未作成）
