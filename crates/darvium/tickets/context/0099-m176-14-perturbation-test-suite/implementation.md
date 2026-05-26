# M1.76-14: 摂動テストスイート（SHOULD perturbation）— 実装サマリ

## 変更したファイル

### src/reciprocity.rs

**新規追加した型・関数（非テストコード、行 ~1326〜1540）:**

1. `PerturbationKind` enum — 5 種の摂動種別（HelpSuccessAddition, TrustDelta, LocalityDistanceDelta, AcceptedOfferToOneRejected, SingleHelperReputationDelta）
2. `StabilityRegressionSummary` struct — flip_rate, churn_delta, hazard_drift, survival_drift, oscillation_detected, village_churn_delta
3. `OscillationDetector` struct — 2 時点間の ranking 逆転数から振動を検出（flip_rate > 0.5 でフラグ）
4. `apply_perturbation()` — ReciprocityReplayScenario に摂動を適用して新しいシナリオを生成
5. `compute_stability_summary()` — 2 つの ReciprocityReplayTrace 間の安定性メトリクスを計算
6. `run_perturbation_suite()` — 全 PerturbationKind に対する摂動テストを一括実行

**新規追加したテスト関数（行 ~4600〜4870）:**

| テスト | 内容 | 閾値 |
|--------|------|------|
| T1 | HelpSuccessAddition(1) → flip_rate <= 0.40 | 0.40 |
| T2 | TrustDelta(±0.01) → village_churn_delta <= 0.15 | 0.15 |
| T3 | Accepted→Rejected → survival_drift <= 0.10 | 0.10 |
| T4 | ReputationDelta(±0.01) → churn_delta < 1.0 | 1.0 |
| T5 | 全摂動種の oscillation 観測 | 観測のみ |
| T6 | σ sweep [0.001, 0.1] → 応答曲線 | 単調非減少（観測） |
| T7 | n=100 統計量（平均・標準偏差） | flip_rate <= 0.40 |

**品質改善:**
- compute_stability_summary 内の `.unwrap()` を `.enumerate()` を使った安全なコードに修正

## 既存テストへの影響

全 1022 テスト通過（0 失敗、0 回帰）
