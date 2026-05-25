---
ticket_id: 93
title: M1.76-8: Helper quality score with benevolence (F-11) + softmax selection (F-12)
slug: m176-8-helper-quality-score-with-benevolence-f-11-softmax-selection-f-12
status: reviewed
created_at: 2026-05-26
updated_at: 2026-05-26
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0093-m176-8-helper-quality-score-with-benevolence-f-11-softmax-selection-f-12/plan.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0093-m176-8-helper-quality-score-with-benevolence-f-11-softmax-selection-f-12/observation-20260526-084432.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0093-m176-8-helper-quality-score-with-benevolence-f-11-softmax-selection-f-12/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0093-m176-8-helper-quality-score-with-benevolence-f-11-softmax-selection-f-12/review.md
---

# M1.76-8: Helper quality score with benevolence (F-11) + softmax selection (F-12)

## Summary

RFC §41B.20.1 式 F-11 (Helper quality score) および §41B.20.2 式 F-12 (Softmax helper selection) の純粋関数を実装する。式 F-11 は既存の helper quality score (41B-8) に benevolence 項 `w_b·B(h)` を additive に追加し、式 F-12 は温度パラメータ τ_Q 付き softmax により helper 選択確率を計算する。本チケットは RFC §41C.3 の **M0.x（pure function validation）** フェーズに対応する。

既存の helper weighting (41B-18 / `compute_helper_weights`) のベース式は変更せず、F-11/F-12 を新たな純粋関数として実装する。

## Background

M1.76-7 までで、直接互恵性 (F-1)、間接互恵性 (F-2)、BenevolenceScore (F-3)、ReputationProfile 再計算 (F-4/F-5)、GC hazard with benevolence (F-7/F-8/F-9)、Child protection (F-10) が実装され、reciprocity-aware survival の基盤が整った。

次のステップとして、HELP プロトコルにおける helper 選定に benevolence を反映する。既存の helper quality score (41B-8) は mission 適合性 S、信頼 T、評判 Rep、child need N、距離 d の 5 要素で構成されていたが、F-11 はこれに benevolence 項 `w_b·B(h)` を additive に追加する。F-12 は温度パラメータ τ_Q を持つ softmax 関数により候補 helper の選択確率を計算する。

### 参照観察レポート

- `tickets/context/0088-m176-3-compute-direct-reciprocity-f-1/observation-20260525-184956.md` — 直接互恵性スコア実装完了確認。
- `tickets/context/0089-m176-4-compute-indirect-reciprocity-f-2-benevolence-f-3/observation-20260526-075128.md` — 間接互恵性スコア・BenevolenceScore 実装完了確認。
- `tickets/context/0090-m176-5-reputationprofile-recompute-reputation-f-4-f-5/observation-20260526-080114.md` — ReputationProfile 再計算実装完了確認。
- `tickets/context/0091-m176-6-gc-hazard-with-benevolence-f-7-f-8-f-9/observation-20260526-081149.md` — GC hazard 実装完了確認。
- `tickets/context/0092-m176-7-child-protection-integration-f-10/observation-20260526-082844.md` — Child protection (F-10) 実装完了確認。

## Scope

### 実装スコープ

1. **定数追加** (`src/constants.rs`): F-11 の重み定数 (Calibration Candidates) を定義
   - `HELP_QUALITY_SUITABILITY_WEIGHT`: w_s — mission 適合性重み
   - `HELP_QUALITY_TRUST_WEIGHT`: w_t — 信頼重み
   - `HELP_QUALITY_REPUTATION_WEIGHT`: w_r — 評判重み
   - `HELP_QUALITY_CHILD_NEED_WEIGHT`: w_n — child need 重み
   - `HELP_QUALITY_DISTANCE_PENALTY`: w_d — 距離ペナルティ重み
   - 注: w_b (benevolence 重み) は `HELP_WEIGHT_BENEVOLENCE` (0.20) として既存

2. **`ReciprocityLifecyclePolicy` 拡張** (`src/event.rs`):
   - 以下のフィールドを追加し、Default 実装で constants 参照:
     - `helper_quality_w_s: f32` → `HELP_QUALITY_SUITABILITY_WEIGHT`
     - `helper_quality_w_t: f32` → `HELP_QUALITY_TRUST_WEIGHT`
     - `helper_quality_w_r: f32` → `HELP_QUALITY_REPUTATION_WEIGHT`
     - `helper_quality_w_b: f32` → `HELP_WEIGHT_BENEVOLENCE`
     - `helper_quality_w_n: f32` → `HELP_QUALITY_CHILD_NEED_WEIGHT`
     - `helper_quality_w_d: f32` → `HELP_QUALITY_DISTANCE_PENALTY`

3. **データ型定義** (`src/event.rs`):
   - `QualityScoreBreakdown` 構造体 — F-11 の各成分の内訳
   - `SoftmaxWeight` 構造体 — F-12 の出力

4. **純粋関数実装** (`src/reciprocity.rs`):
   - `compute_helper_quality_score(...)` — 式 F-11 線形結合
   - `softmax_helper_selection(scores: &[f64], tau: f32) -> Vec<f64>` — 式 F-12 log-sum-exp trick
   - 空の入力には空の `Vec` を返す

5. **テストコード** (`src/reciprocity.rs mod tests`): 以下「Test Plan」の全 TC を実装

### 既に存在するもの（実装不要）

以下の要素は既に実装済みであり、本チケットでは触れない:

- `HELP_WEIGHT_BENEVOLENCE` (0.20) — constants.rs に既存の w_b 定数
- `HELP_SOFTMAX_TAU` (1.0) — constants.rs に既存の τ_Q 定数
- `ReciprocityLifecyclePolicy::tau_helper_softmax` — event.rs に既存のフィールド
- `compute_helper_weights` — childsupport.rs の既存 helper weighting (41B-18)
- `mix_with_remote_exploration` — childsupport.rs の既存遠隔探索
- `select_helpers` — childsupport.rs の既存 helper 選抜

## Non-scope

- `compute_helper_weights` (41B-18) の変更は行わない（F-11 は独立した新規関数）
- `mix_with_remote_exploration` (41B-19) の変更は行わない
- `select_helpers` の変更は行わない
- `HelperSelectionPolicy` の変更は行わない
- `HelperWeight` 構造体の変更は行わない（`SoftmaxWeight` は別途定義）
- 実際の HELPerOffer ポリシーへの F-11/F-12 統合は M1.76-9 以降の結合スコープ
- F-13 (Benevolence-aware remote exploration) は M1.76-9 で実装

## Investigation

### ソースコード調査結果

**証拠1: RFC §41B.20.1 L7893-7909 — F-11 完全数式**

```
F-11: Q(h,c,M) = w_s·S(h,c,M) + w_t·T(h) + w_r·Rep(h) + w_b·B(h) + w_n·N(c) - w_d·d(h,c)
```

Normative constraints:
- ∂Q/∂B ≥ 0: benevolence が高いほど quality score は非減少 (MUST)
- w_b > 0: benevolence 項は正の重みを持つ (MUST)

**証拠2: RFC §41B.20.2 L7911-7921 — F-12 完全数式**

```
F-12: π(h|c,M) = exp(τ_Q·Q(h,c,M)) / Σ_{g∈N_t(c)} exp(τ_Q·Q(g,c,M))
```

Normative constraints:
- τ_Q > 0: 温度は正 (MUST)
- Σ_h π(h|c,M) = 1.0 ± 浮動小数点誤差 (MUST)
- τ_Q → ∞ で argmax に収束 (SHOULD)
- τ_Q → 0 で一様分布に収束 (SHOULD)

**証拠3: 既存定数 (`src/constants.rs` L735-743)**

F-11/F-12 関連の一部定数は既に定義済み:
```rust
pub const HELP_WEIGHT_BENEVOLENCE: f32 = 0.20;  // w_b (OK)
pub const HELP_SOFTMAX_TAU: f32 = 1.0;          // τ_Q (OK)
```

未定義の重み定数（追加が必要）:
- `HELP_QUALITY_SUITABILITY_WEIGHT`: w_s (未定義)
- `HELP_QUALITY_TRUST_WEIGHT`: w_t (未定義)
- `HELP_QUALITY_REPUTATION_WEIGHT`: w_r (未定義)
- `HELP_QUALITY_CHILD_NEED_WEIGHT`: w_n (未定義)
- `HELP_QUALITY_DISTANCE_PENALTY`: w_d (未定義)

**証拠4: `ReciprocityLifecyclePolicy` (`src/event.rs` L481-516)**

既存のポリシーに F-11 の個別重みフィールドは未定義。`tau_helper_softmax` のみ存在:
```rust
pub tau_helper_softmax: f32,  // F-12 (既存)
```

**証拠5: `compute_helper_weights` (`src/childsupport.rs` L169-204)**

既存 helper weighting (41B-18) は乗法的スコア (exp(-β·d) · T^μ · R^ν / Σ) であり、F-11 の線形結合とは根本的に異なる。F-11/F-12 は独立した新規関数として実装する。

**証拠6: M1.76-5 テストパターン (`src/reciprocity.rs` mod tests)**

確立されたテストパターン: ゼロ入力、単調性 sweep、値域拘束、係数 sweep、`println!` + CSV 計装。

### 関数シグネチャ設計

```rust
/// Helper quality score Q(h,c,M) を計算する (F-11)。
///
/// 式 F-11:
///   Q(h,c,M) = w_s·S + w_t·T + w_r·Rep + w_b·B + w_n·N - w_d·d
///
/// 既存の helper quality score (41B-8) に benevolence 項 w_b·B(h) を
/// additive に追加した拡張。各成分は呼び出し元で正規化済みであることを前提とする。
pub fn compute_helper_quality_score(
    mission_suitability: f32,
    trust: f32,
    reputation: f32,
    benevolence: f32,
    child_need: f32,
    distance: f32,
    policy: &ReciprocityLifecyclePolicy,
) -> f32

/// Softmax helper selection 確率分布を計算する (F-12)。
///
/// 式 F-12:
///   π(h|c,M) = exp(τ_Q·Q(h,c,M)) / Σ_g exp(τ_Q·Q(g,c,M))
///
/// 数値的安定性のため log-sum-exp trick を使用。
pub fn softmax_helper_selection(scores: &[f64], tau: f32) -> Vec<f64>
```

**設計判断**:
1. `compute_helper_quality_score` は f32 精度（他のスコア計算と一貫性）
2. `softmax_helper_selection` は f64 精度で計算（log-sum-exp の数値安定性）
3. 各成分の正規化は呼び出し元の責務
4. log-sum-exp trick: `max_logit = max(τ·scores)` → `π_i = exp(τ·score_i - max_logit) / Σ_j exp(τ·score_j - max_logit)`
5. 温度パラメータは関数の直接引数として分離（同一スコアに異なる温度を適用可能）
6. 空の candidates に対して空の Vec を返す

**updated at plan time: 2026-05-26**

**RFC 既存実装状態検証:**

| 対象 | RFC の定義 | 現行コード | 状態 |
|------|-----------|-----------|------|
| w_s (mission suitability weight) | F-11 の係数 | `HELP_QUALITY_SUITABILITY_WEIGHT` 未定義 | ❌ 定数欠落 |
| w_t (trust weight) | F-11 の係数 | `HELP_QUALITY_TRUST_WEIGHT` 未定義 | ❌ 定数欠落 |
| w_r (reputation weight) | F-11 の係数 | `HELP_QUALITY_REPUTATION_WEIGHT` 未定義 | ❌ 定数欠落 |
| w_b (benevolence weight) | F-11 の係数 | `HELP_WEIGHT_BENEVOLENCE = 0.20` | ✅ 一致 |
| w_n (child need weight) | F-11 の係数 | `HELP_QUALITY_CHILD_NEED_WEIGHT` 未定義 | ❌ 定数欠落 |
| w_d (distance penalty) | F-11 の係数 | `HELP_QUALITY_DISTANCE_PENALTY` 未定義 | ❌ 定数欠落 |
| τ_Q (softmax temperature) | F-12 のパラメータ | `HELP_SOFTMAX_TAU = 1.0` | ✅ 一致 |
| `helper_quality_w_s`..`_w_d` | F-11 ポリシーフィールド | `ReciprocityLifecyclePolicy` に未定義 | ❌ フィールド欠落 |
| `QualityScoreBreakdown` | F-11 データ型 | 未定義 | ❌ 型未定義 |
| `SoftmaxWeight` | F-12 データ型 | 未定義 | ❌ 型未定義 |
| `compute_helper_quality_score` | F-11 純粋関数 | 未実装 | ❌ 関数未実装 |
| `softmax_helper_selection` | F-12 純粋関数 | 未実装 | ❌ 関数未実装 |

**評価サマリ**: F-11/F-12 の全実装要素が未実装。本チケットで新規実装が必要。

### 不完全性

**w_b の値**: `HELP_WEIGHT_BENEVOLENCE` = 0.20 は低めに設定。既存 (41B-8) からの逸脱を最小限にする意図。較正フェーズ (M1.76-16/19) で調整。

**quality score の値域**: F-11 の出力は正規化されていない任意の実数。softmax が任意の実数値を確率に写像するため、値域制約は不要。

## Test Plan

### テストケース一覧

全テストは `src/reciprocity.rs` の `mod tests` 内に実装する。

| # | 名称 | 種別 | 内容 |
|---|------|------|------|
| TC-1 | w_b=0 下位互換性 | 正常系 | w_b=0 で benevolence が quality score に影響しない |
| TC-2 | benevolence 単調性 | 正常系 | 同一条件で benevolence が高い candidate が常に高い Q を得る |
| TC-3 | 全成分ゼロ | 正常系 | 全入力 0 で Q = 0 |
| TC-4 | quality score NaN/Inf | 正常系 | ランダム入力で NaN/Inf 不在 |
| TC-5 | softmax 総和 = 1 | 正常系 | 任意スコアで総和が 1.0 ± 1e-6 |
| TC-6 | τ_high で argmax | 正常系 | τ=100.0 で最大スコア要素が確率 ≈ 1.0 |
| TC-7 | τ_low で一様 | 正常系 | τ=0.001 ですべて ≈ 1/N |
| TC-8 | 空リスト | 正常系 | 空の scores に空の Vec |
| TC-9 (計装) | softmax 数値安定性 | 計装 | n=10^5 ランダム [-100, 100] で NaN/Inf 不在 |
| TC-10 (計装) | τ_Q 感度 sweep | 計装 | τ_Q を [0.01, 0.1, 0.5, 1.0, 2.0, 5.0, 10.0] で sweep しエントロピー応答曲線を観測 |

### モック依存

- すべての関数は純粋関数（外部依存なし）
- `ReciprocityLifecyclePolicy` は `default()` またはカスタム構築
- `compute_helper_quality_score` の各成分値はテスト時に明示的に渡す

## 計装方法・観測対象

### 計装方法

- **固定シード PRNG**: `StdRng::seed_from_u64(12345)` で再現性保証
- **softmax 数値安定性**: n = 10^5 一様ランダム入力 [-100, 100] で NaN/Inf/負値を検査
- **τ_Q 応答曲線**: 固定スコア分布に対し τ_Q を 7 水準 sweep、エントロピー H(π) = -Σ π_i log π_i を観測
- **benevolence 単一要素感度**: w_b を [0.0, 0.1, 0.2, 0.3, 0.5] で sweep、確率変化を定量化
- **出力形式**: `println!` で CSV 形式を `--nocapture` 経由で標準出力

### 観測対象

| 観測対象 | サンプルサイズ | 期待される性質 |
|---------|--------------|--------------|
| softmax 総和 = 1 | n >= 10^5 | 全出力が 1.0 ± 1e-6 |
| softmax NaN/Inf | n >= 10^5 | 全出力が有限値 |
| τ_Q → ∞ 極限 | τ=100 | argmax 確率 > 0.999 |
| τ_Q → 0 極限 | τ=0.001 | 全確率が 1/N ± 0.01 |
| benevolence 単調性 | n=101 | B 増加で Q 非減少 |
| エントロピー応答曲線 | 7 水準 | τ_Q 増加でエントロピー単調減少 |
| w_b 感度 | 5 水準 | 線形的な確率変化が観測可能 |

### 較正計画

本チケットでは較正ループは実施しない（純粋関数実装の検証フェーズ）。初期値:

| パラメータ | 値 | 意味 |
|-----------|-----|------|
| w_s | 1.0 | mission 適合性重み |
| w_t | 1.0 | 信頼重み |
| w_r | 1.0 | 評判重み |
| w_b | 0.20 | benevolence 重み（既存） |
| w_n | 1.0 | child need 重み |
| w_d | 1.0 | 距離ペナルティ重み |
| τ_Q | 1.0 | softmax 温度（既存） |

各 w = 1.0 は「全成分が等スケール」の中立状態。較正フェーズ (M1.76-16/19) で調整される。

## 追加定数定義

以下の定数を `src/constants.rs` に追加する:

```rust
/// F-11 Helper quality mission 適合性重み w_s (Calibration Candidate)
/// Q = w_s·S + w_t·T + w_r·Rep + w_b·B + w_n·N - w_d·d
/// Default: 1.0, 感度分析推奨範囲: 0.5-2.0
pub const HELP_QUALITY_SUITABILITY_WEIGHT: f32 = 1.0;

/// F-11 Helper quality 信頼重み w_t (Calibration Candidate)
/// Default: 1.0, 感度分析推奨範囲: 0.5-2.0
pub const HELP_QUALITY_TRUST_WEIGHT: f32 = 1.0;

/// F-11 Helper quality 評判重み w_r (Calibration Candidate)
/// Default: 1.0, 感度分析推奨範囲: 0.5-2.0
pub const HELP_QUALITY_REPUTATION_WEIGHT: f32 = 1.0;

/// F-11 Helper quality child need 重み w_n (Calibration Candidate)
/// Default: 1.0, 感度分析推奨範囲: 0.5-2.0
pub const HELP_QUALITY_CHILD_NEED_WEIGHT: f32 = 1.0;

/// F-11 Helper quality 距離ペナルティ重み w_d (Calibration Candidate)
/// Q では -w_d·d として効くため w_d > 0 で距離ペナルティ。
/// Default: 1.0, 感度分析推奨範囲: 0.5-2.0
pub const HELP_QUALITY_DISTANCE_PENALTY: f32 = 1.0;
```

### ReciprocityLifecyclePolicy 拡張

```rust
// 追加フィールド:
pub helper_quality_w_s: f32,
pub helper_quality_w_t: f32,
pub helper_quality_w_r: f32,
pub helper_quality_w_b: f32,
pub helper_quality_w_n: f32,
pub helper_quality_w_d: f32,

// Default 実装:
helper_quality_w_s: crate::constants::HELP_QUALITY_SUITABILITY_WEIGHT,
helper_quality_w_t: crate::constants::HELP_QUALITY_TRUST_WEIGHT,
helper_quality_w_r: crate::constants::HELP_QUALITY_REPUTATION_WEIGHT,
helper_quality_w_b: crate::constants::HELP_WEIGHT_BENEVOLENCE,
helper_quality_w_n: crate::constants::HELP_QUALITY_CHILD_NEED_WEIGHT,
helper_quality_w_d: crate::constants::HELP_QUALITY_DISTANCE_PENALTY,
```

## Boy Scout Rule — 翻訳可能性計画

1. **関数抽出**: F-11 (`compute_helper_quality_score`) と F-12 (`softmax_helper_selection`) は各々単一責務の純粋関数として実装
2. **内部関数化**: log-sum-exp trick は内部ロジックとして隠蔽
3. **変数名**: 引数名は F-11 式の各項との対応が一目でわかる名前
4. **定数名**: `HELP_QUALITY_*` プレフィックスで F-11 の重量であることを明示
5. **一関数一責務**: 品質スコア計算と softmax 選択を分離。後者は任意のスコアベクトルに再利用可能

## Acceptance Criteria

- [ ] 5 つの HELP_QUALITY_* 定数が constants.rs に Calibration Candidates として定義されている
- [ ] `ReciprocityLifecyclePolicy` に `helper_quality_w_s` 他 5 フィールドが追加され、Default 実装が正しい定数を参照している
- [ ] `compute_helper_quality_score` 純粋関数が実装されている（F-11）
- [ ] `softmax_helper_selection` 純粋関数が実装されている（F-12、log-sum-exp trick 使用）
- [ ] TC-1〜TC-8 の全テストが PASS すること
- [ ] TC-9 (計装) / TC-10 (計装) が構造化出力を生成すること
- [ ] 既存テストが全て通過していること
- [ ] RFC §41B.20.1 / §41B.20.2 との無矛盾確認済み

## Notes

- 2関数とも純粋関数（副作用ゼロ）。永続化は本チケットの責務外。
- `compute_helper_quality_score` の戻り値は f32（他のスコア計算と一貫）。
- `softmax_helper_selection` の戻り値は f64（softmax 裾野の精度と log-sum-exp の安定性のため）。
- log-sum-exp trick: `logits_i = τ·score_i` → `max_logit = max(logits)` → `π_i = exp(logits_i - max_logit) / Σ_j exp(logits_j - max_logit)`
- 既存 `HELP_WEIGHT_BENEVOLENCE` (0.20) が w_b として機能する。
- F-11/F-12 の実際の HELPerOffer ポリシーへの統合は M1.76-9 (F-13) 以降。
- 既存の定数ダンプテスト (`test_calibration_candidates_constants_non_nan`) に新しい HELP_QUALITY_* 定数の非 NaN アサーションを追加する必要がある。

### 成果物

- 計画: context/0093-m176-8-helper-quality-score-with-benevolence-f-11-softmax-selection-f-12/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0093-m176-8-helper-quality-score-with-benevolence-f-11-softmax-selection-f-12/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0093-m176-8-helper-quality-score-with-benevolence-f-11-softmax-selection-f-12/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0093-m176-8-helper-quality-score-with-benevolence-f-11-softmax-selection-f-12/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
