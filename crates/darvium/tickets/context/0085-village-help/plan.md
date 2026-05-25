# 実装計画: M1.75-12 village-help 実験レポート生成と系列管理の統合

## 要件

既存4実験系（M1.75-8 replay, M1.75-9 perturbation, M1.75-10 fuzzing, M1.75-11 calibration）の出力を統合する `VillageExperimentReport` 構造体の実装。Markdown/JSON レポート生成と系列管理（ExperimentLineage）を提供する。

## 変更ファイル一覧

| ファイル | 種別 | 内容 |
|---|---|---|
| `src/report.rs` | 新規 | `VillageExperimentReport`, `ExperimentLineage`, `LineageStore`/`FsLineageStore`, `ReportError`, `BestKnownParams`, Markdown/JSON 出力 |
| `src/replay.rs` | 変更 | `FailingSeedEntry` を公開構造体に昇格、`StabilityRegressionSummary`/`SummaryMetrics` に PartialEq + serde derives 追加、モジュールレベルに serde インポート追加 |
| `src/calibration.rs` | 変更 | `ParameterRange` の `&'static str`→`String` 変更、全構造体に PartialEq + serde derives 追加 |
| `src/village.rs` | 変更 | `VillageMetricsSnapshot` に serde derives 追加 |
| `src/lib.rs` | 変更 | `pub mod report;` 追加、`pub use replay::FailingSeedEntry;` + report 型の re-export 追加 |
| `rules/darvium/experiment-reporting.md` | 新規 | 実験レポート形式・系列管理ルールのドキュメント |

## 計装・観測の実装計画

- 全17テスト: R-1〜R-4（レポート生成検証）、W-1〜W-5（ファイル出力）、L-1〜L-4（系列管理）、I-1〜I-3（統合）、B-1（改行パディング）
- 全て `#[test]` 通常テスト（観測テストは含まず、不変条件検証のみ）
- 観測出力は `println!` + `--nocapture` で取得

## Boy Scout 改善（スコープ外の翻訳可能性修正）

- `calibration.rs` の `ParameterRange.name: &'static str` → `String`（serde Deserialize 対応のため）

## 実装手順

1. `src/report.rs` 新規作成（全型定義 + 17テスト）
2. `src/replay.rs` 変更（serde import、FailingSeedEntry 昇格、derives 追加）
3. `src/calibration.rs` 変更（&str→String、derives 追加）
4. `src/village.rs` 変更（VillageMetricsSnapshot derives 追加）
5. `src/lib.rs` 変更（module + re-export 追加）
6. `rules/darvium/experiment-reporting.md` 新規作成
7. `cargo test` で全テスト合格確認

## リスク

- serde の `Deserialize` は `&'static str` と互換性がない → `String` への変更が必要
- `proptest` は dev-dependency → `FailingSeedEntry` の cfg(test) 解除時に mod tests の cfg(test) 維持が必要
