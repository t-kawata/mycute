# 実装サマリ: M1.75-9 small perturbation 実験スイート

## 変更ファイル一覧

| ファイル | 種別 | 内容 |
|---------|------|------|
| `src/constants.rs` | 編集 | M1.75-9 Small Perturbation 定数 5 種を追加（PERTURB_EMBEDDING_NOISE_SIGMA_DEFAULT, PERTURB_TRUST_DELTA_DEFAULT, PERTURB_USAGE_INCREMENT_DEFAULT, PERTURB_CHURN_MAX_P95_INCREASE, PERTURB_JSD_MAX_P95_INCREASE） |
| `src/replay.rs` | 編集 | StabilityRegressionSummary 型定義 + 5 種の perturbation generator（apply_embedding_noise, apply_trust_delta, apply_single_edge_patch, apply_usage_increment, apply_helper_quarantine）+ compare_perturbed_metrics 比較器 + P-1〜P-8 不変条件テスト + O-P1/O-P2 観測テスト |
| `src/lib.rs` | 編集 | StabilityRegressionSummary + 6 perturbation 関数の re-export 追加 |
| `src/search/pipeline.rs` | 編集（Boy Scout） | tie_break_sort の `&mut Vec` → `&mut [T]` 修正（clippy warning） |

## 実装内容の概要

- StabilityRegressionSummary: baseline/perturbed 間の churn P95 / helper JSD P95 / survival rate 変動量を保持する公開型
- 5 種の perturbation generator: 各摂動を既存 replay scenario に適用して perturbed scenario を生成
- compare_perturbed_metrics: 両 trace の metrics 比較による回帰サマリ出力
- 10 件の新規テスト: P-1〜P-8（不変条件）+ O-P1（σ sweep）+ O-P2（quarantine duration sweep）
- PolicyBundle: Default derive 化（Boy Scout）

## 検証結果

- cargo test: 873 lib tests + 17 integration tests 全 PASS
- cargo clippy -- -D warnings: PASS
- 観測テスト CSV 出力: 確認済み
