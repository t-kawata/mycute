# 実装サマリ: M1.76-10 Child growth increment (F-14) + Maturation probability (F-15)

## 変更ファイル一覧

### 1. `src/constants.rs`
- 既存の2定数（CHILD_GROWTH_WEIGHT_HELP_SUCCESS, CHILD_GROWTH_WEIGHT_BENEVOLENT_HELPERS）を9つの名前付き定数に拡張
- 追加した定数:
  - CHILD_GROWTH_MU_MISSION_SUCCESS (0.60) — μ₁
  - CHILD_GROWTH_MU_HELP_SUCCESS (0.40) — μ₂
  - CHILD_GROWTH_MU_HELPER_BENEVOLENCE (0.30) — μ₃
  - CHILD_GROWTH_MU_FAILURE_BURDEN (0.20) — μ₄
  - MATURATION_NU_BIAS (-2.0) — ν₀
  - MATURATION_NU_EXPERIENCE (1.0) — ν₁
  - MATURATION_NU_TRUST (1.0) — ν₂
  - MATURATION_NU_REPUTATION (1.0) — ν₃
  - MATURATION_NU_HELPER_BENEVOLENCE (0.30) — ν₄
- 旧名の backward-compat aliases を維持

### 2. `src/event.rs`
- ReciprocityLifecyclePolicy 構造体に9フィールド追加（child_growth_mu_* 4件, maturation_nu_* 5件）
- Default impl に対応する初期化を追加
- JSON ラウンドトリップテストの struct literal に9フィールド追加

### 3. `src/reciprocity.rs`
- `compute_child_growth_increment()` (F-14) — 4つの μ 係数による線形結合、非負 clip
- `compute_maturation_probability()` (F-15) — 5つの ν 係数による logistic sigmoid
- 不変条件テスト: F-14 8件 (T1-T8), F-15 10件 (T1-T10)
- 観測テスト: F-14 応答曲面 (mission_success×failure_burden 11×11 grid)
- 観測テスト: F-15 応答曲面 (experience_norm×helper_benevolence 11×11 grid)

## テスト結果
- cargo test: 994 passed, 0 failed ✅
- F-14/F-15 全18不変条件テスト PASS ✅
- 両観測テストで応答曲面出力確認済み ✅
