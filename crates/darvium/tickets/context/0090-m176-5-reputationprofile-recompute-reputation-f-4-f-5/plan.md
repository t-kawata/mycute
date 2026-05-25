# Plan: M1.76-5 ReputationProfile 再計算 recompute_reputation (F-4, F-5)

## 要件
RFC §15.10.3 式 F-4 / F-5 の純粋関数実装。評判スコア Rep_i を4成分（直接互恵性、間接互恵性、経験値正規化、継承スコア）の線形結合 + clamp で計算する。

## RFC 既存実装状態検証

### RFC §15.10.3 ReputationProfile — ✅ 全16フィールド一致
### RFC §15.10.7 ReciprocityLifecyclePolicy — ❌ 4件の不整合
- theta_dir: RECIPROCITY_ALPHA_HELP (α_h=1.0) を参照 — 修正必要
- theta_exp: 生値0.3ハードコード — 修正必要
- theta_inherit: 生値0.2ハードコード — 修正必要
- kappa_e: フィールド欠落 — 追加必要

## 変更ファイル一覧

| ファイル | 種別 | 内容 |
|----------|------|------|
| src/constants.rs | 追加 | 5定数追加 |
| src/event.rs | 修正 | kappa_e フィールド追加 + Default修正 |
| src/reciprocity.rs | 追加 | ReputationInputs + recompute_reputation + テスト7件 |

## 実装手順

1. constants.rs: REPUTATION_THETA_DIR/_IND/_EXP/_INHERIT + REPUTATION_KAPPA_E
2. event.rs: kappa_e フィールド追加、Default修正
3. reciprocity.rs: ReputationInputs + compute_experience_norm + recompute_reputation + テスト7件
4. cargo test 実行、観測出力確認

## テスト計画

- TC-1: 全成分ゼロ → final_score=0
- TC-2: direct_score sweep → 単調非減少
- TC-3: indirect_score sweep → 単調非減少
- TC-4: experience_count → E_norm 漸近
- TC-5: 全成分1 → final_score=1
- TC-6: inherited_score sweep → 単調非減少
- TC-7 (計装): κ_E sweep + 確率単体サンプリング + 値域検証 n=10,000

## 計装・観測

- 固定シード StdRng::seed_from_u64(12345)
- κ_E sweep: [0.001, 0.005, 0.01, 0.05, 0.1], experience_count=10
- 係数ベクトル確率単体ラテン方格サンプリング n=100
- 部分微分 ∂Rep_i/∂θ_k 中心差分推定
- println! CSV → --nocapture

## Boy Scout 改善
- ReciprocityLifecyclePolicy Default: theta_dir 誤参照修正、生値ハードコード定数化
- kappa_e フィールド新設

## リスク
- 低（純粋関数追加のみ、既存テスト共存確認必須）

## レビュー方法
1. run-quality-checks.js で全変更ファイルチェック
2. 翻訳可能性 grep（関数名動詞句確認、汎用変数名チェック）
3. 既存テスト全件通過確認
4. RFC §15.10.3 無矛盾確認
