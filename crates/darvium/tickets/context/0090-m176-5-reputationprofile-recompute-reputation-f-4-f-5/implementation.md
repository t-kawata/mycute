# 変更したファイル一覧と実装内容の概要

## 変更ファイル

### 1. src/constants.rs
- 追加: 5個の定数 (M1.76-5: ReputationProfile recompute (F-4, F-5))
  - `REPUTATION_THETA_DIR` = 0.35 (θ_dir, F-4 直接互恵性重み)
  - `REPUTATION_THETA_IND` = 0.35 (θ_ind, F-4 間接互恵性重み)
  - `REPUTATION_THETA_EXP` = 0.20 (θ_exp, F-4 経験値重み)
  - `REPUTATION_THETA_INHERIT` = 0.10 (θ_inh, F-4 継承重み)
  - `REPUTATION_KAPPA_E` = 0.01 (κ_E, F-5 経験値正規化飽和率)

### 2. src/event.rs
- **構造体変更**: `ReciprocityLifecyclePolicy` に `kappa_e: f32` フィールドを追加
- **Default 値修正（Boy Scout Rule）**:
  - `theta_dir`: `RECIPROCITY_ALPHA_HELP` (1.0, 誤) → `REPUTATION_THETA_DIR` (0.35, 正)
  - `theta_ind`: `REPUTATION_WEIGHT_INDIRECT` (0.35) → `REPUTATION_THETA_IND` (0.35, 意味的修正)
  - `theta_exp`: 生値 0.3 → `REPUTATION_THETA_EXP` (0.20)
  - `theta_inherit`: 生値 0.2 → `REPUTATION_THETA_INHERIT` (0.10)
  - `kappa_e`: 新規追加 → `REPUTATION_KAPPA_E` (0.01)
- **JSON roundtrip テスト修正**: `ReciprocityLifecyclePolicy` 構築に `kappa_e` フィールドを追加

### 3. src/reciprocity.rs
- **構造体追加**: `ReputationInputs` (direct_score, indirect_score, experience_count, inherited_score)
- **関数追加**:
  - `fn compute_experience_norm(count: u32, kappa_e: f32) -> f32` — F-5 の経験値飽和正規化
  - `pub fn recompute_reputation(inputs: ReputationInputs, policy: &ReciprocityLifecyclePolicy) -> ReputationProfile` — F-4 の評判スコア再計算
- **テスト追加 (7件)**:
  - TC-1: 全成分ゼロ → final_score = 0
  - TC-2: direct_score sweep 単調非減少
  - TC-3: indirect_score sweep 単調非減少
  - TC-4: experience_count → E_norm 漸近 (理論値との一致確認)
  - TC-5: 全成分 1 → final_score = 1
  - TC-6: inherited_score sweep 単調非減少
  - TC-7 (計装): 値域 [0,1] 拘束 (n=10,000) + κ_E sweep + 確率単体サンプリング

## 検証結果
- cargo test: 941 テスト全件通過 (library) + 統合テスト全件通過
- 既存テスト（M1.76-3, M1.76-4）の回帰なし
- 観測テスト: κ_E sweep 5 値 × 10 カウント + 確率単体 100 サンプル出力確認
