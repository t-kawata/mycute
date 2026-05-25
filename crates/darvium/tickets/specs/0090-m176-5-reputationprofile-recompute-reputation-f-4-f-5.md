---
ticket_id: 90
title: M1.76-5: ReputationProfile 再計算 recompute_reputation (F-4, F-5)
slug: m176-5-reputationprofile-recompute-reputation-f-4-f-5
status: reviewed
created_at: 2026-05-26
updated_at: 2026-05-26
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0090-m176-5-reputationprofile-recompute-reputation-f-4-f-5/plan.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0090-m176-5-reputationprofile-recompute-reputation-f-4-f-5/observation-20260526-080114.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0090-m176-5-reputationprofile-recompute-reputation-f-4-f-5/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0090-m176-5-reputationprofile-recompute-reputation-f-4-f-5/review.md
---

# M1.76-5: ReputationProfile 再計算 recompute_reputation (F-4, F-5)

## Summary

RFC §15.10.3 式 F-4 および F-5 で定義される評判スコア再計算 `recompute_reputation` の純粋関数を実装する。式 F-4 は直接互恵性スコア、間接互恵性スコア、経験値正規化、継承スコアの4成分を係数結合し `[0, 1]` にクランプする。式 F-5 は経験値カウントを飽和正規化する（古参固定化防止）。本チケットは RFC §41C.3 の **M0.x（pure function validation）** フェーズに対応する。

## Background

M1.76-3 で `compute_direct_reciprocity` (F-1)、M1.76-4 で `compute_indirect_reciprocity` (F-2) および `compute_benevolence_score` (F-3) が実装され、互恵性スコア計算の基盤が整った。次のステップとして、これらのスコア成分と経験値・継承スコアを統合し、最終的な評判スコア `ReputationProfile.final_score` を計算する F-4 / F-5 が必要である。

`recompute_reputation` は以下の後続チケットの入力となる:
- M1.76-6: GC hazard with benevolence (F-7, F-8, F-9) — `final_score` を hazard 計算に使用
- M1.76-7: Child protection integration (F-10) — `final_score` を child protection に使用
- M1.76-8: Helper quality score with benevolence (F-11)
- M1.76-9: Benevolence-aware remote exploration (F-13)

### 参照観察レポート

- `tickets/context/0088-m176-3-compute-direct-reciprocity-f-1/observation-20260525-184956.md` — 直接互恵性スコア実装完了確認。sigmoid、重み割り当て、時間減衰の基盤関数確立。
- `tickets/context/0089-m176-4-compute-indirect-reciprocity-f-2-benevolence-f-3/observation-20260526-075128.md` — 間接互恵性スコア・BenevolenceScore 実装完了。応答曲面・β 感度 sweep の観測パターン確立。`logistic_sigmoid` を `pub(crate)` に変更済みで再利用可能。

## Scope

1. **`ReputationInputs` 構造体の定義** — F-4 入力4成分を保持する軽量構造体
2. **`recompute_reputation` 関数の実装** — 式 F-4 / F-5 の純粋関数
3. **F-4 / F-5 定数の追加** — `src/constants.rs` に θ_dir, θ_ind, θ_exp, θ_inh, κ_E を新規定義
4. **`ReciprocityLifecyclePolicy` デフォルト値の修正** — theta_dir が誤って RECIPROCITY_ALPHA_HELP (1.0) を参照している問題を是正
5. **7件のテスト**（Test Plan 参照）

## Non-scope

- GC hazard with benevolence (F-7, F-8, F-9) — チケット M1.76-6 で実装
- Child protection (F-10) — チケット M1.76-7 で実装
- Helper quality score (F-11) — チケット M1.76-8 で実装
- `ReciprocityEvent` のインジェスション・パイプライン — チケット M1.76-11 で実装
- EventBus や MetadataStore との結合 — 本チケットでは純粋関数として隔離
- ReputationProfile の永続化・バージョニング — 本関数は値を返すのみ
- 係数和 θ_dir + θ_ind + θ_exp + θ_inh = 1 の強制アサーション — runtime チェックは組み込まず、spec 推奨として文書化

## Investigation

### 物理的証拠

**証拠1: RFC §15.10.3 L4323-4348 — F-4 / F-5 完全数式**

```
F-4: Rep_i = clip_{[0,1]}( θ_dir·R_i^dir + θ_ind·R_i^ind + θ_exp·E_i^norm + θ_inh·I_i )
F-5: E_i^norm = 1 - exp(-κ_E · experience_count(i))
```

Required constraints:
- direct_score と indirect_score の寄与は 0 であってはならない (MUST NOT) unless environment policy が village-help を無効化している場合
- final_score は direct / indirect reciprocity が増加したとき、他条件一定なら非減少でなければならない (MUST)

**証拠2: 既存 ReputationProfile 構造体（`src/event.rs` L402-464）**

全16フィールドの ReputationProfile が定義済み。F-4 の4成分に対応するフィールド:
- `direct_score: f32` (L406) — 入力 R_i^dir
- `indirect_score: f32` (L408) — 入力 R_i^ind
- `experience_score: f32` (L410) — 出力 E_i^norm（F-5 計算結果を格納）
- `inherited_score: f32` (L412) — 入力 I_i
- `final_score: f32` (L414) — 出力 Rep_i（F-4 計算結果）

**証拠3: 既存 ReciprocityLifecyclePolicy（`src/event.rs` L481-537）**

F-4 の係数フィールドは既に定義済みだが、デフォルト値に問題がある:
```rust
theta_dir: crate::constants::RECIPROCITY_ALPHA_HELP,   // 1.0 — 誤り
theta_ind: crate::constants::REPUTATION_WEIGHT_INDIRECT,  // 0.35
theta_exp: 0.3,    // 生値ハードコード
theta_inherit: 0.2,  // 生値ハードコード
```

問題点:
1. `theta_dir` が `RECIPROCITY_ALPHA_HELP` (=1.0) を参照している。この定数は F-1 の α_h であり、F-4 の θ_dir とは意味が異なる。
2. `theta_exp` と `theta_inherit` が生値ハードコード。
3. デフォルト値の和が 1.0 + 0.35 + 0.3 + 0.2 = 1.85 となり、推奨の「和 = 1」から乖離。

**証拠4: 既存 constants.rs（L622-734）**

F-4 / F-5 に必要な定数は未定義。既存定数は全て F-1 / F-2 / F-3 用。

**証拠5: M1.76-3 / M1.76-4 テストパターン（`src/reciprocity.rs` L163-625）**

確立されたテストパターン: ゼロ入力、単調性 sweep、値域 [0,1] 拘束（n >= 10,000）、係数 sweep、`println!` + CSV 計装。

### 関数シグネチャ設計

```rust
/// ReputationProfile 再計算の入力構造体 (F-4, F-5)
pub struct ReputationInputs {
    /// 直接互恵性スコア R_i^dir (F-4)
    pub direct_score: f32,
    /// 間接互恵性スコア R_i^ind (F-4)
    pub indirect_score: f32,
    /// 経験値カウント experience_count(i) (F-5)
    pub experience_count: u32,
    /// 継承スコア I_i (F-4)
    pub inherited_score: f32,
}

/// 評判スコア Rep_i を再計算する (F-4, F-5)。
///
/// 式 F-4:
///   Rep_i = clip_{[0,1]}( θ_dir·R_i^dir + θ_ind·R_i^ind + θ_exp·E_i^norm + θ_inh·I_i )
///
/// 式 F-5:
///   E_i^norm = 1 - exp(-κ_E · experience_count(i))
pub fn recompute_reputation(
    inputs: ReputationInputs,
    policy: &ReciprocityLifecyclePolicy,
) -> ReputationProfile
```

**設計判断**:
1. `ReputationInputs` を軽量構造体として分離: ReputationProfile の全16フィールドを渡さず必要最小限に。
2. `experience_count` は u32（整数カウント）。F-5 の exp 計算時に f32 にキャスト。
3. `policy` から θ 係数と κ_E を読み取り、較正ハーネスからのパラメータ調整を可能に。
4. 戻り値は新規 ReputationProfile: 計算結果を反映した新しいインスタンス。
5. 係数和が 1 でない場合でもエラーにしない（推奨であり強制ではない）。

## Test Plan

### テストケース一覧

| # | 名称 | 種別 | 内容 |
|---|------|------|------|
| TC-1 | 全成分ゼロ | 正常系 | 全入力 0 → `experience_count=0 → E_norm=0`、`final_score=0` |
| TC-2 | direct_score sweep 単調増加 | 正常系 | `R_i^dir` sweep で `final_score` が単調非減少 |
| TC-3 | indirect_score sweep 単調増加 | 正常系 | `R_i^ind` sweep で `final_score` が単調非減少 |
| TC-4 | experience_count → E_norm 漸近 | 正常系 | `count=0 → E_norm=0`、`count → ∞ → E_norm → 1` |
| TC-5 | 全成分 1 で final_score = 1 | 正常系 | 全入力 1 → `final_score = 1`（clamp 上端） |
| TC-6 | inherited_score sweep | 正常系 | `I_i` sweep で単調非減少 |
| TC-7 (計装) | 経験値飽和曲線 + 応答曲面 | 計装 | κ_E sweep + 係数ベクトル確率単体ラテン方格サンプリング |

### モック依存

- テスト内で `ReputationInputs` を直接構築（外部依存なし、純粋関数のため）
- `ReciprocityLifecyclePolicy` は `default()` またはカスタム構築

## 計装方法・観測対象

### 計装方法

- **固定シード PRNG**: `StdRng::seed_from_u64(12345)` で確率的サンプリングを再現
- **κ_E sweep**: `[0.001, 0.005, 0.01, 0.05, 0.1]` の5値で sweep し、`experience_count=10` における `E_norm` の分布を観測
- **係数ベクトル確率単体サンプリング**: `(θ_dir, θ_ind, θ_exp, θ_inh)` を確率単体上でラテン方格サンプリングし、最終スコア `Rep_i` の超平面応答を計測
- **部分微分 ∂Rep_i/∂θ_k**: 各成分の感度分析により支配的成分を同定
- **出力形式**: `println!` で CSV 形式を `--nocapture` 経由で標準出力

### 観測対象

| 観測対象 | サンプルサイズ | 期待される性質 |
|---------|--------------|--------------|
| 値域 [0,1] 拘束 | n >= 10,000 | 全 final_score が閉区間内、NaN/Inf 不在 |
| direct_score sweep 単調性 | n = 100 | R_i^dir 増加で final_score 非減少 |
| indirect_score sweep 単調性 | n = 100 | R_i^ind 増加で final_score 非減少 |
| inherited_score sweep 単調性 | n = 100 | I_i 増加で final_score 非減少 |
| 経験値飽和曲線 | κ_E × count = 5×1000 | 式 F-5 の指数飽和曲線が理論値と一致 |
| 係数感度 ∂Rep_i/∂θ_k | 確率単体 100 点 | 各 θ_k の部分微分が正 |

### 較正計画

本チケットでは較正ループは実施しない（純粋関数実装の検証フェーズ）。初期値:

- `θ_dir = 0.35`（直接互恵性、中程度の寄与）
- `θ_ind = 0.35`（間接互恵性、中程度の寄与）
- `θ_exp = 0.20`（経験値、やや低め — 古参固定化防止）
- `θ_inh = 0.10`（継承、低め — 親の影響限定）
- `κ_E = 0.01`（経験値飽和率 — count=100 で約 0.63）

和: 0.35 + 0.35 + 0.20 + 0.10 = **1.00**（推奨一致）。

## 追加定数定義

以下の定数を `src/constants.rs` に追加する:

```rust
/// F-4 reputation 再計算 直接互恵性重み θ_dir (Calibration Candidate)
/// Default: 0.35, 感度分析推奨範囲: 0.20-0.50
pub const REPUTATION_THETA_DIR: f32 = 0.35;

/// F-4 reputation 再計算 間接互恵性重み θ_ind (Calibration Candidate)
/// Default: 0.35, 感度分析推奨範囲: 0.20-0.50
pub const REPUTATION_THETA_IND: f32 = 0.35;

/// F-4 reputation 再計算 経験値重み θ_exp (Calibration Candidate)
/// Default: 0.20, 感度分析推奨範囲: 0.10-0.35
pub const REPUTATION_THETA_EXP: f32 = 0.20;

/// F-4 reputation 再計算 継承重み θ_inh (Calibration Candidate)
/// Default: 0.10, 感度分析推奨範囲: 0.05-0.25
pub const REPUTATION_THETA_INHERIT: f32 = 0.10;

/// F-5 経験値正規化飽和率 κ_E (Calibration Candidate)
/// E_i^norm = 1 - exp(-κ_E * experience_count(i))
/// Default: 0.01, 感度分析推奨範囲: 0.001-0.10
pub const REPUTATION_KAPPA_E: f32 = 0.01;
```

### ReciprocityLifecyclePolicy デフォルト値修正

```rust
// 修正前:
theta_dir: crate::constants::RECIPROCITY_ALPHA_HELP,  // 誤: 1.0
theta_ind: crate::constants::REPUTATION_WEIGHT_INDIRECT,  // 0.35
theta_exp: 0.3,
theta_inherit: 0.2,

// 修正後:
theta_dir: crate::constants::REPUTATION_THETA_DIR,  // 0.35
theta_ind: crate::constants::REPUTATION_THETA_IND,  // 0.35
theta_exp: crate::constants::REPUTATION_THETA_EXP,  // 0.20
theta_inherit: crate::constants::REPUTATION_THETA_INHERIT,  // 0.10
```

## Boy Scout Rule — 翻訳可能性計画

### 改善対象

1. **`ReputationInputs` 構造体追加**: F-4 計算に必要な4成分を明確に命名されたフィールドで保持。タプルではなく構造体として型安全性を確保。
2. **`recompute_reputation` 関数**: 動詞句の関数名。内部は散文的な記述順: `compute_experience_norm` → 線形結合 → `clamp` → `ReputationProfile` 構築。
3. **`compute_experience_norm` 補助関数**: F-5 を名前付き関数として分離し、テスト容易性を向上。
4. **ハードコード禁止**: θ 係数、κ_E は全て `constants.rs` の名前付き定数から取得。
5. **既存コード修正**: `ReciprocityLifecyclePolicy` の `theta_dir` デフォルト誤り（RECIPROCITY_ALPHA_HELP）を是正。生値ハードコードされた `theta_exp`, `theta_inherit` を定数化。

### 影響範囲外

`constants.rs` への定数追加と `event.rs` のポリシーデフォルト修正は行うが、既存の M1.76-3 / M1.76-4 テストや関数のシグネチャは変更しない。

## Acceptance Criteria

- [ ] `recompute_reputation` が全入力 0 に対して `final_score = 0` を返す
- [ ] `direct_score` sweep で `final_score` が単調非減少
- [ ] `indirect_score` sweep で `final_score` が単調非減少
- [ ] `inherited_score` sweep で `final_score` が単調非減少
- [ ] `experience_count = 0 → E_norm = 0`、`experience_count → ∞ → E_norm → 1` の漸近
- [ ] 全入力 1 で `final_score = 1`（clamp 上端）
- [ ] 経験値飽和曲線 κ_E sweep が観測可能
- [ ] 確率単体ラテン方格サンプリングによる応答観測が可能
- [ ] `ReciprocityLifecyclePolicy` デフォルト値が正しい定数を参照
- [ ] 既存テスト（F-1, F-2, F-3）が通過している
- [ ] RFC §15.10.3 との無矛盾確認済み

## Notes

- `recompute_reputation` は純粋関数（副作用ゼロ）。永続化は本チケットの責務外。
- `ReputationInputs` は軽量構造体とし、`ReputationProfile` の全16フィールドを渡さない。
- 係数和が 1 であることは推奨であって強制ではない。
- F-5 の `exp` 計算は `f32` 精度で十分。`experience_count` は u32 から f32 にキャスト。
- `logistic_sigmoid` は F-4 では不使用（clamp による線形結合の直接クリップ）。
- M1.76-4 で変更された `logistic_sigmoid` の `pub(crate)` 可視性は維持。

### 成果物

- 計画: context/0090-m176-5-reputationprofile-recompute-reputation-f-4-f-5/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0090-m176-5-reputationprofile-recompute-reputation-f-4-f-5/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0090-m176-5-reputationprofile-recompute-reputation-f-4-f-5/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0090-m176-5-reputationprofile-recompute-reputation-f-4-f-5/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成）
