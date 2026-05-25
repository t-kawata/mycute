# 実装サマリー: M1.75-12 village-help 実験レポート生成と系列管理の統合

## 変更したファイル一覧と実装内容の概要

### 新規: `src/report.rs`
- `ReportError` enum: Io, Serialization, CircularLineage — Display + From 実装
- `ExperimentLineage` 構造体: experiment_id, parent_ids, depth(), validate() (循環検出)
- `LineageStore` trait + `FsLineageStore`: ファイルベースの系列永続化（experiments/lineages.json）
- `BestKnownParams`: params HashMap + j_value による最良パラメータ管理
- `VillageExperimentReport`: 4実験系の出力を統合する報告構造体
- `to_markdown()`: 8セクションの Markdown レポート生成（Title/Lineage/Summary/Perturbation/Calibration/Fuzzing/Params/Anomalies）
- `write_markdown_report()` / `write_json_report()`: ファイル出力ユーティリティ
- 全17テスト（R-1〜R-4, W-1〜W-5, L-1〜L-4, I-1〜I-3, B-1）: 全 18 tests (1 naming + calibration_report_format) passing

### 変更: `src/replay.rs`
- モジュールレベルに `use serde::{Deserialize, Serialize};` 追加
- `FailingSeedEntry` を `#[cfg(test)]` 内部から公開構造体に昇格（lib.rs からの re-export 対応）
- `StabilityRegressionSummary` / `SummaryMetrics` に PartialEq + serde derives 追加
- `mod tests` に `#[cfg(test)]` 維持（proptest は dev-dependency のため）

### 変更: `src/calibration.rs`
- `ParameterRange.name: &'static str` → `String`（Deserialize 互換性）
- `ParameterRange`, `SweepMode`, `VillageCalibrationConfig`, `VillageCalibrationResult`, `CalibrationReport` に PartialEq + serde derives 追加

### 変更: `src/village.rs`
- `VillageMetricsSnapshot` に serde derives 追加

### 変更: `src/lib.rs`
- `pub mod report;` 追加
- `pub use replay::FailingSeedEntry;` 追加
- report モジュールの全公開型を re-export

### 新規: `rules/darvium/experiment-reporting.md`
- 実験レポート形式（8セクション必須）
- Markdown テンプレートと JSON スキーマ
- Lineage ID 命名規則（exp-YYYYMMDD-NNN）
- 系列管理ルール

## テスト結果

- 全 926 tests passed（元の 892 から 34 増加）
- 既存テストに影響なし
- proptest! マクロ正常動作確認
