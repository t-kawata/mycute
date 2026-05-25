# 変更したファイル一覧と実装内容の概要

## 変更ファイル

| ファイル | 種別 | 内容 |
|----------|------|------|
| src/constants.rs | 定数追加 | Reciprocity-Aware Survival 用16較正定数を追記 (Calibration Candidates) |
| src/event.rs | 構造体定義 | ReputationProfile (16フィールド) + ReciprocityLifecyclePolicy (16フィールド) 新規定義 |
| src/event.rs | テスト追加 | TC-1〜TC-5 (5テストケース) を mod tests に追加 |
| src/lib.rs | re-export | ReciprocityLifecyclePolicy / ReputationProfile の公開API追加 |

## ReputationProfile 構造体 (src/event.rs)

- v2.3-e 既存8フィールド維持: direct_score, indirect_score, experience_score, inherited_score, final_score, alpha_positive, beta_negative, last_recomputed_at
- v2.3-f 追加8フィールド: direct_help_count, direct_success_count, direct_reject_count, harm_event_count, accepted_offer_rate, help_success_rate, village_centrality, benevolence_score
- cold_start() メソッド: scores=0.5, counts=0, last_recomputed_at=UNIX_EPOCH
- derive: Debug, Clone, PartialEq, Serialize, Deserialize

## ReciprocityLifecyclePolicy 構造体 (src/event.rs)

- 16フィールド: theta_dir, theta_ind, theta_exp, theta_inherit, lambda_gc_base, gamma_lifecycle, gamma_benevolence, gamma_child_protect, rho_direct_decay, tau_helper_softmax, epsilon_remote_base, epsilon_remote_max, adult_experience_threshold(u32), adult_trust_threshold(f32), adult_reputation_threshold(f32), policy_version(String)
- Default: 各フィールドは constants.rs 参照または固定値、policy_version は空文字列
- derive: Debug, Clone, PartialEq, Serialize, Deserialize

## 較正定数 (src/constants.rs)

16種全て f32 型、Calibration Candidates 分類、RFC §15.10 数式参照付き:
RECIPROCITY_ALPHA_HELP(1.0), RECIPROCITY_ALPHA_SUCCESS(2.0), RECIPROCITY_ALPHA_REJECT(1.0), RECIPROCITY_ALPHA_HARM(2.0), RECIPROCITY_DIRECT_DECAY_RHO(0.01), REPUTATION_WEIGHT_DIRECT(0.35), REPUTATION_WEIGHT_INDIRECT(0.35), LIFECYCLE_WEIGHT_BENEVOLENCE(0.15), GC_HAZARD_GAMMA_BENEVOLENCE(0.10), GC_HAZARD_GAMMA_CHILD_PROTECT(0.20), HELP_WEIGHT_BENEVOLENCE(0.20), HELP_SOFTMAX_TAU(1.0), REMOTE_EXPLORATION_BASE(0.05), REMOTE_EXPLORATION_MAX(0.20), CHILD_GROWTH_WEIGHT_HELP_SUCCESS(0.40), CHILD_GROWTH_WEIGHT_BENEVOLENT_HELPERS(0.30)

## テスト結果

- cargo test: 938 tests PASS (0 failures)
- cargo clippy -- -D warnings: PASS
- 品質チェック: 328 findings (全て既存、新規issueなし)
