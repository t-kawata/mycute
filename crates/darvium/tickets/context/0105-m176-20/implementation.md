# 変更したファイル一覧と実装内容の概要

## 変更ファイル

| ファイル | 種別 | 変更内容 |
|---|---|---|
| `src/report.rs` | 変更 | ReciprocityExperimentReport 構造体追加（315〜368行） |
| `src/report.rs` | 変更 | `reciprocity_report_to_markdown()` 関数追加（515〜680行） |
| `src/report.rs` | 変更 | `write_reciprocity_markdown_report()` / `write_reciprocity_json_report()` 追加（683〜708行） |
| `src/report.rs` | 変更 | テスト 13 ケース追加（RRecip-1〜5, W-RRecip-1〜4, L-Recip-1〜2, I-Recip-1） |
| `src/report.rs` | 変更 | calibration.rs からの import 拡張（CalibrationPhase, PhaseStatus, ReciprocityCalibrationReport, ReciprocityOperationalMetrics） |
| `src/lib.rs` | 変更 | 公開 API に ReciprocityExperimentReport 他を追加 |
| `src/calibration.rs` | 変更 | ReciprocityOperationalMetrics / CalibrationPhase / PhaseStatus に Serialize/Deserialize derive を追加 |

## 実装の概要

### ReciprocityExperimentReport 構造体
既存の VillageExperimentReport と同様のパターンで、以下のフィールドを持つ：
- experiment_id / lineage / summary_metrics / calibration_report / perturbation_results
- failing_seeds / best_known_params / phase_status / open_anomalies / timestamp
- 既存型（ExperimentLineage / FailingSeedEntry / BestKnownParams / StabilityRegressionSummary / ReciprocityOperationalMetrics / ReciprocityCalibrationReport）を再利用

### Markdown 出力（9 セクション構成）
1. Title, 2. Lineage, 3. Replay Metrics Summary, 4. Perturbation Results
5. Calibration Results, 6. Phase Status（新規）, 7. Failing Seeds
8. Best-Known Parameters, 9. Open Anomalies

### テスト（13 テスト）
- 構造体構築・空・異常耐性テスト（RRecip-1〜5）
- Markdown/JSON/ファイル出力テスト（W-RRecip-1〜4）
- Lineage 統合・ID 一意性テスト（L-Recip-1〜2）
- 統合レポート出力テスト（I-Recip-1）

## 依存関係
- 既存 VillageExperimentReport は一切変更なし
- calibration.rs への変更は Serialize/Deserialize derive 追加のみ（後方互換）
