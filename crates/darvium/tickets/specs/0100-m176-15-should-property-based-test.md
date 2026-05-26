---
ticket_id: 100
title: M1.76-15: プロパティベース不変条件ファジング（SHOULD property-based test）
slug: m176-15-should-property-based-test
status: reviewed
created_at: 2026-05-26
updated_at: 2026-05-26
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0100-m176-15-should-property-based-test/plan.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0100-m176-15-should-property-based-test/observation-20260526-113421.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0100-m176-15-should-property-based-test/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0100-m176-15-should-property-based-test/review.md
---
# M1.76-15: プロパティベース不変条件ファジング（SHOULD property-based test）

## Summary

`proptest` クレートを用いて Reciprocity モジュールの6種類の不変条件（benevolence monotonicity、hazard non-negativity、probability boundedness、no negative reputation、no silent overflow/NaN、grace period child protection）をランダム生成した入力全域で検証する。違反を検出した場合は failing seed を export し、replay fixture に昇格する機構を実装する。

## Background

M1.76-14（摂動テストスイート）までは、特定の摂動種別に対して事前定義されたシナリオで安定性を観測してきた。しかし、**事前に設計可能な摂動の範囲は常に限定的**であり、開発者が想定しなかったエッジケースでの不変条件違反は発見できない。

本チケットでは `proptest` が自動生成する広範なランダム入力に対して6種の不変条件を自動検証することで、以下のリスクをカバーする：

1. パラメータ空間の未探索領域での non-monotonicity（単調性違反）
2. 極端な population 構成（child 過多 / adult 過少 / 距離行列の特異値）における確率境界逸脱
3. NaN/Inf の暗黙的伝播 — f32 演算の意外な経路での発散
4. Grace Period child の誤 GC — `is_child=true` かつ low reputation で hazard 暴走

物理的証拠として、既存の `compute_direct_reciprocity` 等の関数群は全て不変条件コメントを持つが、それらが全域で成立することを確認するテストは存在しない（M1.76-12 の単調性テストは特定条件の sweep のみで、proptest を使ったランダム生成ではない）。

## Scope

- `proptest` 戦略群の実装:
  - `workflow_population_strategy()` — グラフ population サイズ（0〜20）、ID 文字列生成
  - `child_adult_ratio_strategy()` — child/adult 比率（0.0〜1.0）
  - `distance_matrix_strategy()` — 距離行列（成分 0.0〜1.0、対称性は任意）
  - `help_event_stream_strategy()` — ReciprocityEvent 列（種別・時刻・重みのランダム生成）
  - `harm_reject_noise_strategy()` — 有害/拒否ノイズ注入量（0.0〜1.0）
  - `policy_coefficient_strategy()` — ReciprocityLifecyclePolicy の各係数（0.0〜上限）
- `ReciprocityInvariantSuite` — 全不変条件の定義と自動検証器
- 検証 invariant 群（6種）:
  1. `benevolence_monotonicity`: 直接互恵性/間接互恵性の増加で benevolence が非減少
  2. `hazard_non_negativity`: GC hazard（F-7 softplus）が常に非負
  3. `probability_boundedness`: 全確率出力（GC確率・生存確率）が `[0, 1]`
  4. `no_negative_reputation`: ReputationScore 全成分（final_score, direct_score, indirect_score 等）が非負
  5. `no_silent_overflow_nan`: NaN/Inf がいかなる入力でも発生しない
  6. `grace_period_child_protection`: Grace Period 中の child は一時的低 reputation でも GC hazard が有限に留まる
- failing seed export → replay fixture 昇格機構（`tickets/context/0100-m176-15-should-property-based-test/fixtures/` 配下に `.seed` ファイル保存）
- `#[cfg(test)] mod tests` 内の `proptest!` マクロによるテスト関数

## Non-scope

- Reciprocity モジュール以外のドメイン（Village, Event Architecture, Training 等）の invariant は対象外
- 性能測定・ベンチマーク
- 実 LLM 結合テスト
- 較正パラメータの変更（M1.76-16 で実施）
- 摂動テストスイート（M1.76-14 完了済み）の再実装

## Investigation

### ソースコード調査結果

#### 1. `proptest` 依存関係（Cargo.toml:22）

既に `proptest = "1"` が `[dev-dependencies]`（ビルド依存）に追加済み。

#### 2. 既存不変条件コメントの所在（src/reciprocity.rs）

各関数に不変条件のコメントが記述されているが、proptest による全称検証はない：

| 関数 | 行 | 宣言されている不変条件 |
|------|---|----------------------|
| `compute_direct_reciprocity` | 74 | 協力行為で非減少、裏切りで非増加 (MUST) |
| `compute_indirect_reciprocity` | 124 | 中心性/参加度等の増加で非減少、負評価で非増加 (MUST) |
| `compute_benevolence_score` | 160 | 出力は [0, 1] にクランプ (MUST) |
| `compute_gc_hazard` | 286 | softplus により常に非負 (MUST) |
| `compute_gc_probability` | 310 | 出力 [0, 1) (MUST)、hazard=0 で 0 |
| `compute_survival_probability` | 329 | 出力 (0, 1] (MUST)、hazard=0 で 1 |
| `compute_child_protection` | 362 | is_child=true で η₁ 以上 (MUST) |
| `compute_helper_quality_score` | 406 | 全重み非負 (MUST)、スコア増加で非減少 |
| `softmax_helper_selection` | 450 | 総和=1.0、全要素非負、空入力→空出力 (MUST) |
| `compute_child_growth_increment` | 545 | 出力非負 (MUST) |
| `compute_maturation_probability` | 588 | 出力 (0, 1) (MUST, sigmoid) |

#### 3. 既存の proptest 利用パターン（src/replay.rs）

`replay.rs` では M1.75-10 用に以下の proptest 戦略が実装済み：
- `maturity_strategy`, `adult_candidate_strategy`, `adult_candidate_list_strategy`
- テスト：F-1〜F-7（全7種）
- `ProptestConfig { cases: PROPTEST_DEFAULT_CASES }` パターン

#### 4. 既存の proptest 利用パターン（src/event.rs）

`event.rs` では M1.5-R11 用に以下の proptest 戦略が実装済み：
- `event_kind_strategy`, `interaction_mode_strategy`, `darvium_event_strategy`
- テスト：P-1〜P-9（全9種）

#### 5. 既存の不変条件テスト（src/reciprocity.rs ~1200-1319）

M1.76-12 単調性テストスイートでは特定条件での sweep 検証を行っているが、使用している PRNG は `StdRng::seed_from_u64(12345)` の固定シードであり、パラメータ空間の網羅的探索は行っていない。`proptest` が自動縮小（shrinking）機能で違反事例を最小化できる利点は活用されていない。

#### 6. Grace Period 定義（constants.rs:321）

```
MIN_SURVIVAL_EXPERIENCE = 5  // experience_count < 5 → Child
E_ADULT_THRESHOLD = 20       // E(G) >= 20 → Adult 条件の一つ
```

`grace_period_child_protection` invariant の検証では、`experience_count < MIN_SURVIVAL_EXPERIENCE` の child に対して `reputation = 0.0, benevolence = 0.0` でも GC hazard が有限に留まることを確認する。

#### 7. 既存の不変条件 violation 率の計測（M1.76-12）

M1.76-12 の `run_monotonicity_suite` は `random_sweep_violation_rates` を出力する枠組みを持つ。本チケットではこれを proptest の枠組みで拡張する。

### 参照観察レポート

- `tickets/context/0099-m176-14-perturbation-test-suite/observation-20260526-120000.md` — 摂動テスト結果。HelpSuccessAddition が最も影響力大、TrustDelta/ReputationDelta は flip_rate=0.00 で安定。摂動強度 σ に対して hazard_drift が線形増加。本チケットの policy_coefficient_strategy の設計に活用する。
- `tickets/context/0098-m176-13-must-replay-test/observation-20260526-110021.md` — 決定論的リプレイ確認。全6テスト PASS、golden trace hash 一致。本チケットでも replay fixture 昇格機構で同様のハッシュ比較手法を採用する。
- `tickets/context/0097-m176-12-must-monotonicity-tests/observation-20260526-104115.md` — 単調性検証。全 MUST 条件成立、random_sweep_violation_rates=0.0。4条件のうち直接スコア増加と reputation 増加で 0% 違反を確認。既存の固定シード sweep を補完する形で proptest のランダム探索を設計する。

## Test Plan

### 戦略定義（`use proptest::{prelude::*, prop_compose}` を src/reciprocity.rs の test モジュールに追加）

#### 1. `workflow_population_strategy()` → `Vec<(WorkflowGraphId, f32)>`
- グラフ ID 文字列（"graph-{index}") と lifecycle_score [0, 1] のタプルの Vec
- サイズ 0〜20（空を含む）

#### 2. `child_adult_ratio_strategy(adult_count, child_count)` → `(f32, f32)`
- adult_ratio = adult_count / max(1, total)
- child_ratio = child_count / max(1, total)
- 極端ケース: adult 0, child 0, adult only, child only

#### 3. `distance_matrix_strategy(graph_count)` → `Vec<Vec<f32>>`
- graph_count × graph_count の行列（各成分 [0, 1]）
- 対角成分は 0.0

#### 4. `help_event_stream_strategy()` → `Vec<ReciprocityEvent>`
- サイズ 0〜30
- 各イベント:
  - event_kind: 全 ReciprocityEventKind から一様選択
  - weight: [0.1, 1.0]
  - virtual_clock: [0, 1000]
  - source_graph_id: "graph-{idx}"

#### 5. `harm_reject_noise_strategy()` → `f32`
- [0.0, 1.0] の一様分布

#### 6. `policy_coefficient_strategy()` → `ReciprocityLifecyclePolicy`
- 各係数: [0.0, 2.0]（実際のデフォルト値の 0〜2 倍の範囲）
- 極端ケース: 全係数 0、全係数 max

### テスト関数（6種の proptest）

#### T1: `benevolence_monotonicity`
- 生成: `(workflow_population, help_event_stream, policy_coefficient)`
- 検証: 任意のイベント列に対し、HelpSucceeded を 1 件追加しても benevolence_score が非減少
- アサーション: `prop_assert!(benevolence_after >= benevolence_before - epsilon)`

#### T2: `hazard_non_negativity`
- 生成: `(lifecycle_score, benevolence_score, child_protection, policy)`
- すべての入力値を [0.0, 1.0] の範囲（child_protection は [0.0, 5.0]）でランダム
- 検証: `compute_gc_hazard` の出力が常に非負
- アサーション: `prop_assert!(hazard >= 0.0)`

#### T3: `probability_boundedness`
- 生成: `(hazard [0.0, 100.0], delta_t [0, 10000])`
- 検証: `compute_gc_probability` + `compute_survival_probability` が [0, 1]
- アサーション: `prop_assert!((0.0..=1.0).contains(&prob))`

#### T4: `no_negative_reputation`
- 生成: `(direct_score [-0.1, 1.1], indirect_score [-0.1, 1.1], experience_count [0, 100], inherited_score [-0.1, 1.1], policy)`
- 入力範囲に負値を含める（異常入力に対する頑健性）
- 検証: `recompute_reputation` の `final_score` が常に [0, 1]
- アサーション: `prop_assert!(profile.final_score >= 0.0 && profile.final_score <= 1.0)`

#### T5: `no_silent_overflow_nan`
- 生成: 全公開関数に対してランダム入力を生成
- 検証: NaN/Inf が一切発生しない
- `prop_assert!(!(output.is_nan() || output.is_infinite()))` を各純粋関数の出力に適用

#### T6: `grace_period_child_protection`
- 生成: `(lifecycle_score, benevolence_score, child_protection_extra, policy)`
- child_protection に `CHILD_PROTECT_ETA1` を加算（child 保護項が有効な状態）
- 検証: GC hazard が `softplus(lambda_0 - gamma_child_protect * CHILD_PROTECT_ETA1)` を超えない
- さらに `child_protection = 0`（保護なし）の場合との比較で保護効果を確認

### 極端ケーステスト（3種の個別テスト）

#### E1: 全係数ゼロ
- `ReciprocityLifecyclePolicy` の全係数 = 0.0
- 全ての invariant が違反なく成立する（または graceful に動作する）こと

#### E2: 全係数最大
- 各係数を許容範囲の上限に設定
- NaN/Inf が発生しないこと

#### E3: Population 極値
- adult = 0, child = 0 の空 population
- adult = 20, child = 0 の adult only
- adult = 0, child = 20 の child only
- 全 invariant が違反なく成立すること

### Failing Seed Export

- 各 `proptest!` ブロックのデフォルト設定: `ProptestConfig { cases: PROPTEST_DEFAULT_CASES }`
- 違反検出時: シード値をファイル `tickets/context/0100-m176-15-should-property-based-test/fixtures/{test_name}.seed` に保存
- 保存したシードは `--seed` 引数で再実行可能（`cargo test -- --seed XXX`）

## 計装方法・観測対象

### 計装方法

- `src/reciprocity.rs` の `#[cfg(test)] mod tests` 内に実装
- `use proptest::{prelude::*, prop_compose, test_runner::TestRunner}` を追加
- `ProptestConfig { cases: PROPTEST_DEFAULT_CASES }`（10,000 ケース）を使用
- 固定シードではなく `ProptestConfig { failure_persistence: None }` とし、違反検出時のみ seed を保存

### 観測対象

- **Invariant violation 率**: fuzz ケース全体に対する各 invariant の違反率（期待値: 0）
- **Violation clustering**: パラメータ空間における違反集中領域の有無を観測（`eprintln!` で違反時の入力を出力）
- **Grace Period 保護効果の統計検証**:
  - Grace Period 中の child に対する GC hazard 分布と Grace Period 超過後の分布を比較
  - Welch の t 検定（$p < 0.05$）で保護効果の統計的有意差を検証
  - `hazard_with_protection < hazard_without_protection` が有意水準を満たすか
- **Failing seed 昇格数**: 発見されたエッジケースの蓄積をカウント
- **Proptest shrink 効率**: 違反検出時の shrink ステップ数を記録

### 観測テストからの出力形式（`--nocapture` 使用）

```
M1.76-15 T1: benevolence_monotonicity — cases=10000, violations=0
M1.76-15 T2: hazard_non_negativity — cases=10000, violations=0
M1.76-15 T3: probability_boundedness — cases=10000, violations=0
M1.76-15 T4: no_negative_reputation — cases=10000, violations=0
M1.76-15 T5: no_silent_overflow_nan — cases=10000, violations=0
M1.76-15 T6: grace_period_child_protection
  with_protection:  mean=XX.XXXX, std=XX.XXXX
  without_protection: mean=XX.XXXX, std=XX.XXXX
  welch_t_stat=XX.XXXX, p_value=XX.XXXX, significant=true/false
M1.76-15 E1: all_coefficients_zero — PASS
M1.76-15 E2: all_coefficients_max — no_nan=true
M1.76-15 E3: population_extremes — empty=ok, adult_only=ok, child_only=ok
M1.76-15 failing_seeds_exported: 0
M1.76-15 全ての不変条件を確認しました
```

### 較正計画

本チケットは直接の較正（constants.rs の変更）を伴わない。違反が検出された場合、その原因分析を元に M1.76-16 で較正する。

## Boy Scout Rule — 翻訳可能性計画

1. **関数名の統一**: 新規の proptest 戦略定義関数は `*_strategy()` で命名統一する。

2. **不変条件コメントの補強**: 既存関数の不変条件コメントに「proptest 検証済み」のマークを追加する。例：
   ```
   /// # 不変条件
   /// - 出力は常に非負 (MUST, softplus 保証)
   /// - **proptest 10000 cases で T2 検証済み**
   ```

3. **ハードコード値の定数化**: ProptestConfig のケース数は既に `PROPTEST_DEFAULT_CASES` が定数定義済み。新規戦略の範囲パラメータ（policy 係数の上限等）にも名前付き定数を導入する。

4. **既存コードの改善**: M1.76-12 単調性テストの `random_sweep_violation_rates` 用の sweep ループは、proptest の shrinking 機能に置き換え可能だが、既存テストとの互換性を維持するため本チケットでは変更しない。

## Acceptance Criteria

- [ ] 6種の proptest 戦略が実装され、コンパイルが通ること
- [ ] 6種の invariant テストが `cargo test` で PASS すること
- [ ] 3種の極端ケーステスト（全係数ゼロ/最大、population 極値）が PASS すること
- [ ] `--nocapture` 出力で観測統計量が表示されること
- [ ] 既存テストが全て通過していること（回帰なし）
- [ ] Grace Period 保護効果の統計的検定（Welch t-test）が実装され、出力されること
- [ ] failing seed の export → replay fixture 昇格機構が動作すること
- [ ] 翻訳可能性の検証が通っていること

## Notes

- plan_path: context/0100-m176-15-should-property-based-test/plan.md（未作成、/plan-ticket 承認後に作成）
- implementation_path: context/0100-m176-15-should-property-based-test/implementation.md（未作成、/start-ticket 実装完了後に作成）
- review_report_path: context/0100-m176-15-should-property-based-test/review.md（未作成、/review-ticket 全チェック通過後に作成）
- observation_report_path: context/0100-m176-15-should-property-based-test/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）

### 成果物

- 計画: context/0100-m176-15-should-property-based-test/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0100-m176-15-should-property-based-test/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0100-m176-15-should-property-based-test/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0100-m176-15-should-property-based-test/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
