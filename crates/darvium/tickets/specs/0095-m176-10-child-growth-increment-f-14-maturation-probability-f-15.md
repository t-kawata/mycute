---
ticket_id: 95
title: M1.76-10: Child growth increment (F-14) + Maturation probability (F-15)
slug: m176-10-child-growth-increment-f-14-maturation-probability-f-15
status: reviewed
created_at: 2026-05-26
updated_at: 2026-05-26
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0095-m176-10-child-growth-increment-f-14-maturation-probability-f-15/observation-20260526-091721.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0095-m176-10-child-growth-increment-f-14-maturation-probability-f-15/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0095-m176-10-child-growth-increment-f-14-maturation-probability-f-15/review.md
---

# M1.76-10: Child growth increment (F-14) + Maturation probability (F-15)

## Summary

2つの純粋関数を実装する：

1. **F-14 `compute_child_growth_increment`**: child workflow c の成長増分 ΔG_c を計算する。mission success、help success、helper benevolence 平均が正の寄与、failure burden が負の寄与を持つ線形結合。
2. **F-15 `compute_maturation_probability`**: child が adult に成熟する確率 P_mature(c) をロジスティックシグモイドで計算する。経験値正規化スコア・信頼・評判・helper benevolence 平均が正の寄与を持つ。

「優しい大人に囲まれた child は成長しやすく、成熟しやすい」という世界観を数理的に実現する。

## Background

- RFC §41B.20.4 式 F-14、§41B.20.5 式 F-15 で定義。
- **F-14**: ΔG_c = μ₁·MissionSuccess_c + μ₂·Σ_h HelpSuccess(h→c) + μ₃·B̄_helpers(c) - μ₄·FailureBurden_c
  - MissionSuccess_c ∈ {0, 1}（child 自身の mission 完了有無）
  - Σ_h HelpSuccess(h→c): child が受けた help 成功の総和
  - B̄_helpers(c): child の helper となった adult の BenevolenceScore 平均
  - FailureBurden_c: child の失敗負担量
  - 出力 ΔG_c は experience_count や maturation score に反映可能
- **F-15**: P_mature(c) = σ(ν₀ + ν₁·E_c^norm + ν₂·T_c + ν₃·Rep_c + ν₄·B̄_helpers(c))
  - E_c^norm: 経験値正規化スコア [0, 1]（既存 `compute_experience_norm` を使用）
  - T_c: 信頼複合スコア [0, 1]
  - Rep_c: 評判スコア [0, 1]
  - B̄_helpers(c): helper benevolence 平均 [0, 1]
  - σ: ロジスティックシグモイド（既存 `logistic_sigmoid` を使用）
  - 出力: (0, 1) の確率

### 既存コードとの関連

- `compute_experience_norm`（`src/reciprocity.rs:187`）: F-5 の E_i^norm 計算関数。F-15 で E_c^norm として再利用可能。
- `logistic_sigmoid`（`src/reciprocity.rs:34`）: F-1/F-2 で使用中の既存関数。F-15 でそのまま利用。
- `classify_maturity`（`src/village.rs:80`）: 現在は経験値・信頼・レピュテーションの3軸で Child/Adult を決定論的に判定。F-15 の確率的成熟判定は、この関数の外部から追加的な mature トリガーとして使用する前提。`classify_maturity` のシグネチャは変更しない。
- F-10 `compute_child_protection` の引数 `growth_improvement` に F-14 の結果を入力可能。
- `CHILD_GROWTH_WEIGHT_HELP_SUCCESS`（constants.rs:793）と `CHILD_GROWTH_WEIGHT_BENEVOLENT_HELPERS`（constants.rs:798）は既存定数。

## Scope

### 1. 定数追加（`src/constants.rs`）

**F-14 係数（新規）:**

| 定数名 | 記号 | Default | 範囲 | 説明 |
|--------|------|---------|------|------|
| `CHILD_GROWTH_MU_MISSION_SUCCESS` | μ₁ | 0.60 | 0.30-1.00 | mission success 1回の成長寄与 |
| `CHILD_GROWTH_MU_HELP_SUCCESS` | μ₂ | 0.40 | 0.20-0.60 | help success 1件あたりの成長寄与（既存 `CHILD_GROWTH_WEIGHT_HELP_SUCCESS`） |
| `CHILD_GROWTH_MU_HELPER_BENEVOLENCE` | μ₃ | 0.30 | 0.15-0.50 | helper benevolence 平均の成長寄与 |
| `CHILD_GROWTH_MU_FAILURE_BURDEN` | μ₄ | 0.20 | 0.10-0.40 | failure burden の成長減衰量 |

注: 既存 `CHILD_GROWTH_WEIGHT_HELP_SUCCESS`（μ₂）は本チケットで `CHILD_GROWTH_MU_HELP_SUCCESS` として再定義する。後方互換性のため旧定数名は削除せず新しい定数名で統一する。同様に `CHILD_GROWTH_WEIGHT_BENEVOLENT_HELPERS`（ν₄）も `MATURATION_NU_HELPER_BENEVOLENCE` に統一する。

**F-15 係数（新規）:**

| 定数名 | 記号 | Default | 範囲 | 説明 |
|--------|------|---------|------|------|
| `MATURATION_NU_BIAS` | ν₀ | -2.0 | -4.0-0.0 | シグモイドバイアス。負値によりデフォルトで成熟確率を低めに設定 |
| `MATURATION_NU_EXPERIENCE` | ν₁ | 1.0 | 0.5-2.0 | 経験値正規化スコアの重み |
| `MATURATION_NU_TRUST` | ν₂ | 1.0 | 0.5-2.0 | 信頼スコアの重み |
| `MATURATION_NU_REPUTATION` | ν₃ | 1.0 | 0.5-2.0 | 評判スコアの重み |
| `MATURATION_NU_HELPER_BENEVOLENCE` | ν₄ | 0.30 | 0.15-0.50 | helper benevolence 平均の重み |

### 2. `ReciprocityLifecyclePolicy` 拡張（`src/event.rs`）

**追加フィールド:**

```rust
pub child_growth_mu_mission_success: f32,
pub child_growth_mu_help_success: f32,
pub child_growth_mu_helper_benevolence: f32,
pub child_growth_mu_failure_burden: f32,
pub maturation_nu_bias: f32,
pub maturation_nu_experience: f32,
pub maturation_nu_trust: f32,
pub maturation_nu_reputation: f32,
pub maturation_nu_helper_benevolence: f32,
```

`Default` 実装は対応する定数から初期化する。

### 3. 純粋関数の実装（`src/reciprocity.rs`）

**F-14:**

```rust
pub fn compute_child_growth_increment(
    mission_success: bool,
    help_successes: &[f32],
    helper_benevolence_mean: f32,
    failure_burden: f32,
    policy: &ReciprocityLifecyclePolicy,
) -> f32
```

- 実装: ΔG_c = μ₁·M + μ₂·Σ_h HS + μ₃·B̄ - μ₄·F
  - M = if mission_success { 1.0 } else { 0.0 }
  - Σ_h HS = help_successes.iter().sum::<f32>()
  - B̄ = helper_benevolence_mean
  - F = failure_burden
- 出力値域: 非負（負の場合は 0.0 に clamp）

**F-15:**

```rust
pub fn compute_maturation_probability(
    experience_norm: f32,
    trust: f32,
    reputation: f32,
    helper_benevolence_mean: f32,
    policy: &ReciprocityLifecyclePolicy,
) -> f64
```

- 実装: P_mature = logistic_sigmoid(ν₀ + ν₁·E + ν₂·T + ν₃·Rep + ν₄·B̄) as f64
  - E = experience_norm ∈ [0, 1]
  - T = trust ∈ [0, 1]
  - Rep = reputation ∈ [0, 1]
  - B̄ = helper_benevolence_mean ∈ [0, 1]
- 出力値域: (0, 1)（シグモイドの値域）

### 4. テスト

- 不変条件テスト（境界値・単調性・退化ケース・boundedness）
- 観測テスト（μ 比率 sweep による成長増分感度・ν 係数 sweep による成熟確率応答曲線）

## Non-scope

- `classify_maturity` への F-15 直接埋め込み（本チケットでは純粋関数として切り出し、呼び出し元での統合は M1.76-11 以降のパイプラインで扱う）
- F-14 の成長増分を `experience_count` や F-10 の `growth_improvement` に配管する実際のフック
- 較正ループ（J(θ) は M1.76-16 で扱う）

## Investigation

### 参照観察レポート

- `context/0094-m176-9-benevolence-aware-remote-exploration-f-13/observation-20260526-090120.md` — F-13 の ε_remote は child need を使用。F-14/F-15 の growth increment / maturation と need の関係を整合させる必要がある。B_local_avg の応答曲面観測結果は F-14 μ₃ の較正に利用可能。
- `context/0093-m176-8-helper-quality-score-with-benevolence-f-11-softmax-selection-f-12/observation-20260526-084432.md` — helper quality score の benevolence 項 w_b が成立済み。F-14/F-15 で helper_benevolence_mean として参照する B_score の値域が [0, 1] であることを確認。

### 関連ソースコード箇所

| ファイル | 該当箇所 | 備考 |
|----------|----------|------|
| `src/constants.rs:790-798` | `CHILD_GROWTH_WEIGHT_HELP_SUCCESS` / `CHILD_GROWTH_WEIGHT_BENEVOLENT_HELPERS` | 既存の F-14/F-15 部分定数。μ₂, ν₄ に対応。名称統一が必要 |
| `src/event.rs:481-564` | `ReciprocityLifecyclePolicy` struct | 拡張対象。現在 18 フィールド + policy_version |
| `src/reciprocity.rs:34-36` | `logistic_sigmoid` | F-15 で再利用する既存関数（f32→f32） |
| `src/reciprocity.rs:187-193` | `compute_experience_norm` | F-15 の E_c^norm 入力として使用可能 |
| `src/village.rs:80-96` | `classify_maturity` | 現状は決定論的判定。F-15 は外部から確率的成熟判定として利用。シグネチャ変更なし |
| `src/reciprocity.rs:355-369` | `compute_child_protection` | 引数 `growth_improvement` に F-14 結果を入力可能 |

### 欠落している構成要素

1. `ReciprocityLifecyclePolicy` に F-14/F-15 の各係数フィールドが未定義。9 フィールド新規追加が必要。
2. `constants.rs` に μ₁, μ₃, μ₄, ν₀, ν₁, ν₂, ν₃ の 7 定数が未定義。
3. 既存の `CHILD_GROWTH_WEIGHT_HELP_SUCCESS` と `CHILD_GROWTH_WEIGHT_BENEVOLENT_HELPERS` は F-14/F-15 の定数だが、命名が RFC の記法（μ/ν）と一致しない。本チケットで統一する。

### 実装方針

F-14 と F-15 はどちらも純粋関数として `src/reciprocity.rs` に実装する。
- F-14 の出力は非負 f32（負の場合は 0.0 clamp）
- F-15 の出力は f64（既存 `compute_gc_probability` / `compute_survival_probability` と統一）
- 既存 `classify_maturity` は変更しない。F-15 の出力は呼び出し元（TrainingOrchestrator, replay 等）で確率的成熟判定に使用する想定
- F-14 の growth_increment は呼び出し元で `experience_count` の更新または `compute_child_protection` の `growth_improvement` に反映する想定

```rust
// F-14 呼び出し例（呼び出し元）:
let growth = compute_child_growth_increment(
    mission_success,
    &help_success_weights,
    helper_benevolence_mean,
    failure_burden,
    &reciprocity_policy,
);

// F-15 呼び出し例（呼び出し元、確率的成熟判定）:
let p_mature = compute_maturation_probability(
    compute_experience_norm(experience_count, policy.kappa_e),
    trust,
    reputation,
    helper_benevolence_mean,
    &reciprocity_policy,
);
if rng.gen::<f64>() < p_mature {
    // Child → Adult 遷移
}
```

## Test Plan

### ユニットテスト（`src/reciprocity.rs` の `mod tests` に追加）

**F-14: compute_child_growth_increment**

| ID | 種別 | 条件 | 期待結果 |
|-----|------|------|---------|
| F14-T1 | 境界値 | `mission_success=false, help_successes=[], helper_benevolence_mean=0, failure_burden=0` | ΔG_c = 0.0 |
| F14-T2 | 境界値 | `mission_success=true`, 他ゼロ | ΔG_c が正（μ₁ と一致） |
| F14-T3 | 境界値 | `failure_burden` が全寄与を上回る（例: μ₁=1, failure_burden=10） | ΔG_c が 0.0 に clamp（負を許容しない） |
| F14-T4 | 単調性 | `helper_benevolence_mean` 増加（0→1 step 0.01） | ΔG_c が単調非減少 |
| F14-T5 | 単調性 | `failure_burden` 増加（0→1 step 0.01） | ΔG_c が単調非増加 |
| F14-T6 | 単調性 | `help_successes` の要素追加（空→[0.5]→[0.5,0.3]） | ΔG_c が単調非減少 |
| F14-T7 | 劣化ケース | μ₁=0, μ₂=0, μ₃=0, μ₄=0 | ΔG_c = 0（全係数ゼロ） |
| F14-T8 | 観測 | ランダム入力 n=10⁴ | NaN/Inf 不在、値域非負 |

**F-15: compute_maturation_probability**

| ID | 種別 | 条件 | 期待結果 |
|-----|------|------|---------|
| F15-T1 | 境界値 | `experience_norm=0, trust=0, reputation=0, helper_benevolence_mean=0` | σ(ν₀) と一致。ν₀=-2.0 なら約 0.119 |
| F15-T2 | 境界値 | `experience_norm=1, trust=1, reputation=1, helper_benevolence_mean=1` | σ(ν₀ + ν₁+ν₂+ν₃+ν₄)。デフォルトで約 0.731 |
| F15-T3 | 有界性 | P_mature が常に 1.0 未満 | どんな入力でも P_mature < 1.0 |
| F15-T4 | 有界性 | 全入力範囲で random sweep n=10⁴ | P_mature が常に (0, 1) に bounded |
| F15-T5 | 単調性 | `helper_benevolence_mean` 増加（0→1 step 0.01） | P_mature が単調非減少 |
| F15-T6 | 単調性 | `experience_norm` 増加（0→1 step 0.01） | P_mature が単調非減少 |
| F15-T7 | 単調性 | `trust` 増加（0→1 step 0.01） | P_mature が単調非減少 |
| F15-T8 | 単調性 | `reputation` 増加（0→1 step 0.01） | P_mature が単調非減少 |
| F15-T9 | 退化 | ν₄=0（helper benevolence 無効） | P_mature が既存3変数（E, T, Rep）のみで決定。ν₄ の値が結果に影響しない |
| F15-T10 | 観測 | ランダム入力 n=10⁴ | NaN/Inf 不在、値域 (0, 1)、数値安定性 |

### 観測テスト

**F-14 応答曲面（μ 比率 sweep）:**
- `(μ₁, μ₂, μ₃, μ₄)` の比率を [1:1:1:1, 2:1:1:1, 1:2:1:1, 1:1:2:1, 1:1:1:2] で sweep
- 標準入力: mission_success=true, help_successes=[0.5, 0.3], helper_benevolence_mean=0.6, failure_burden=0.2
- 各比率における ΔG_c の値を観測・比較
- μ₃（helper benevolence）を [0.0, 1.0] step 0.1 で sweep → ΔG_c 成長曲線
- μ₄（failure burden）を [0.0, 1.0] step 0.1 で sweep → ΔG_c 減少曲線

**F-15 応答曲面（ν 係数 sweep）:**
- `(ν₀, ν₁, ν₂, ν₃, ν₄)` の各係数を [0.5×, 1.0×, 2.0×] で個別 sweep（その他の係数はデフォルト値固定）
- `helper_benevolence_mean` を [0.0, 1.0] step 0.1 で sweep → P_mature シグモイド応答曲線
- `experience_norm` を [0.0, 1.0] step 0.1 で sweep → 経験値に対する成熟確率の感受性曲線
- 固定シード PRNG（`StdRng::seed_from_u64(12345)`）を使用

## 計装方法・観測対象

### 計装方法

- 純粋関数 `compute_child_growth_increment` の入出力を `println!` + `--nocapture` で標準出力に書き出す
- 純粋関数 `compute_maturation_probability` の入出力を同様に出力
- F-14: μ 比率 sweep × (mission_success, help_successes, B̄, failure_burden) の合成応答
- F-15: ν sweep × (E_norm, T, Rep, B̄) のシグモイド曲線 sweep

### 観測対象

- F-14: 各 μ 比率における成長増分 ΔG_c の値
- F-14: helper benevolence（μ₃）が成長増分に与える影響の効果量
- F-14: failure burden（μ₄）が成長をどの程度抑制するかの減衰曲線
- F-15: 各 ν パラメータが成熟確率曲線の形状（傾き・中点）に与える影響
- F-15: `helper_benevolence_mean` → P_mature のシグモイド応答（ν₄ の効果）
- F-15: ν₄=0（benevolence 無効）時の成熟確率との乖離

### 較正計画

- 調整する定数: μ₁〜μ₄, ν₀〜ν₄
- 目的関数: M1.76-16 で定義（本チケットでは純粋関数実装と観測テストまで）
- 停止条件: N/A（較正は後続チケット）

## Boy Scout Rule — 翻訳可能性計画

- **新規関数名**:
  - `compute_child_growth_increment` — 「child の成長増分を計算する」
  - `compute_maturation_probability` — 「成熟確率を計算する」
- **変数名**: `mission_success` / `help_successes` / `helper_benevolence_mean` / `failure_burden` — ドメイン概念を直接表現
- **一関数一責務**: F-14 と F-15 は別関数として実装。片方の変更が他方に影響しない
- **ハードコード値禁止**: μ₁〜μ₄, ν₀〜ν₄ はすべて `constants.rs` の名前付き定数 + policy 経由で注入
- **既存関数の積極的再利用**: `logistic_sigmoid` と `compute_experience_norm` を F-15 で再利用
- **既存コード無変更**: `classify_maturity` のシグネチャは変更せず、F-15 は外部から利用するパターンを維持
- **定数名の統一**: 既存の曖昧な定数名（`CHILD_GROWTH_WEIGHT_*`）を RFC の記法（MU/NU）に合わせた名前に整理

## Acceptance Criteria

- [ ] F-14 `compute_child_growth_increment` 純粋関数が実装されている
- [ ] F-15 `compute_maturation_probability` 純粋関数が実装されている
- [ ] 対応する定数 μ₁〜μ₄, ν₀〜ν₄ が `constants.rs` に追加されている
- [ ] `ReciprocityLifecyclePolicy` に9つの新規フィールドが追加され、Default 実装が更新されている
- [ ] 以下の F-14 不変条件テストがすべて PASS:
  - F14-T1: 全入力ゼロ → ΔG_c = 0
  - F14-T2: mission_success=true → 正の増分
  - F14-T3: failure_burden 超過 → ΔG_c = 0（負 clamp）
  - F14-T4: helper_benevolence_mean 増加 → ΔG_c 単調非減少
  - F14-T5: failure_burden 増加 → ΔG_c 単調非増加
  - F14-T6: help_successes 追加 → ΔG_c 単調非減少
  - F14-T7: 全係数ゼロの劣化ケース → ΔG_c = 0
  - F14-T8: ランダム n=10⁴ で NaN/Inf 不在・値域非負
- [ ] 以下の F-15 不変条件テストがすべて PASS:
  - F15-T1: 全入力 0 → σ(ν₀) と一致
  - F15-T2: 全入力 1 → 理論値と一致
  - F15-T3: P_mature < 1.0
  - F15-T4: ランダム n=10⁴ で P_mature ∈ (0, 1)
  - F15-T5〜T8: 各入力軸の単調非減少
  - F15-T9: ν₄=0 → benevolence 無効（下位互換性）
  - F15-T10: NaN/Inf 不在・数値安定性
- [ ] 観測テストが μ 比率 sweep と ν 係数 sweep の応答を出力する
- [ ] `cargo test` が既存テスト含めて全件 PASS
- [ ] 翻訳可能性の検証が通っている

## Notes

- plan_path: context/0095-m176-10-child-growth-increment-f-14-maturation-probability-f-15/plan.md（未作成、/plan-ticket 承認後に作成）
- implementation_path: context/0095-m176-10-child-growth-increment-f-14-maturation-probability-f-15/implementation.md（未作成、/start-ticket 実装完了後に作成）
- review_report_path: context/0095-m176-10-child-growth-increment-f-14-maturation-probability-f-15/review.md（未作成、/review-ticket 全チェック通過後に作成）
- observation_report_path: context/0095-m176-10-child-growth-increment-f-14-maturation-probability-f-15/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成）

### 成果物

- 計画: context/0095-m176-10-child-growth-increment-f-14-maturation-probability-f-15/plan.md（未作成）
- 実装サマリ: context/0095-m176-10-child-growth-increment-f-14-maturation-probability-f-15/implementation.md（未作成）
- レビュー報告書: context/0095-m176-10-child-growth-increment-f-14-maturation-probability-f-15/review.md（未作成）
- 観察レポート: context/0095-m176-10-child-growth-increment-f-14-maturation-probability-f-15/observation-YYYYMMDD-HHmmss.md（未作成）
