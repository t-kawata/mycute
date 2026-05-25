---
ticket_id: 85
title: village-help 実験レポート生成と系列管理の統合
slug: village-help
status: reviewed
created_at: 2026-05-25
updated_at: 2026-05-25
ticket_ref: M1.75-12
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0085-village-help/plan.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0085-village-help/implementation.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0085-village-help/observation-20260525-172042.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0085-village-help/review.md
---

# M1.75-12: village-help 実験レポート生成と系列管理の統合

## Summary

M1.75-8（replay）、M1.75-9（perturbation）、M1.75-10（property-based fuzzing）、M1.75-11（calibration harness）の 4 つの実験系統の出力を単一の `VillageExperimentReport` に統合し、Markdown および JSON 形式で出力する report writer を実装する。あわせて実験系列（lineage）管理の枠組みを導入し、各実験がどの親実験から派生したかを追跡可能にする。

## Background

village-help 実装（M1.75）の完了条件は「village-help が導入されたこと」ではなく「village-help が観測と較正の対象として運用可能になったこと」である。M1.75-8〜11 までのテストは各実験系統を確立したが、それらの出力はばらばらに存在している：

- replay trace は `--nocapture` の println! 出力としてしか残らない
- perturbation の `StabilityRegressionSummary` はテスト内で生成されるが永続化されない
- fuzzing の `FailingSeedEntry` は fixture として保存されるが、他の実験結果との関連づけがない
- calibration の `CalibrationReport` はメモリ上の構造体で完結し、ファイル出力されない

これらの出力を統一的に扱うレポート基盤が欠落しているため、以下のリスクが生じる：

1. **再現性の喪失**: 実験結果が人間の記憶にのみ依存する
2. **系列追跡不能**: どのパラメータ変更がどの観測結果を生んだかトレースできない
3. **回帰検出不能**: 後日の実験と比較する baseline が存在しない
4. **説明可能性の欠如**: 第三者（または未来の自分）が実験の意図と結果を理解できない

本チケットはこれらのギャップを埋める。

## Scope

1. **`VillageExperimentReport` 構造体の定義**（src/report.rs 新規ファイル）
   - replay trace 要約（SummaryMetrics）
   - perturbation 結果一覧（Vec\<StabilityRegressionSummary\>）
   - calibration 結果（CalibrationReport を内包）
   - failing seed 一覧（Vec\<FailingSeedEntry\> → 公開型へ昇格）
   - best-known parameter bundle（現在の最適パラメータ設定とその J 値）
   - open anomalies リスト（未解決の異常観測）
   - lineage 情報（experiment_id, parent_experiment_ids, description, timestamp）

2. **`rules/darvium/experiment-reporting.md` の新規作成**
   - report skeleton（必須セクション一覧）の定義
   - lineage ID 命名規則（"exp-YYYYMMDD-NNN"）
   - Markdown report のテンプレート
   - JSON report のスキーマ

3. **`VillageExperimentReportWriter` の実装**
   - Markdown 形式へのシリアライズ（`write_markdown(&self, path: &Path) -> Result<()>`）
   - JSON 形式へのシリアライズ（`write_json(&self, path: &Path) -> Result<()>`）
   - 空 metrics / failure-only ケースでも必須フィールドを維持すること

4. **Lineage 管理の枠組み**
   - `ExperimentLineage` 構造体: `experiment_id`, `parent_ids: Vec<String>`, `description`, `tags: Vec<String>`, `created_at`
   - `LineageStore` トレイト + `FsLineageStore`（ファイルシステムベース実装）
   - 実験系列の stateless 検証（循環参照検出、親 ID の存在確認）

5. **既存データ型の公開 API 昇格**
   - `FailingSeedEntry` を `#[cfg(test)]` の内部構造体から公開型へ昇格
   - 既存の `VillageCalibrationResult` / `CalibrationReport` を report 統合に対応

6. **統合テスト**
   - replay・perturbation・fuzz・calibration の各実験結果が単一レポートへ欠落なく統合されること
   - lineage の親子関係が正しく追跡できること
   - empty metrics / failure-only ケースでも壊れたレポートを出さないこと
   - failing seed と golden trace 参照がレポート中で相互整合していること

## Non-scope

- レポートの可視化（グラフ描画、ダッシュボード）は含まない。Markdown/JSON 出力まで。
- 旧実験レポートのマイグレーションは含まない。本チケット以降の新しい実験が対象。
- M1.76 系の互恵性指標は含まない。M1.76-20 で別チケット化済み。
- 自動実験実行スケジューラは含まない。人間駆動のワークフローを前提とする。

## Investigation

### 参照観察レポート

- `tickets/context/0084-m175-11-village-calibration-loop-harness-j-village/observation-20260525-161942.md` — 較正ハーネス完成。`CalibrationReport` 構造体（experiment_id, parent_experiment_id, config, mode, results, timestamp）が定義済みだがファイル出力未実装。
- `tickets/context/0083-m175-10-property-based-village-invariant-fuzzing-failing-seed-replay-fixture/observation-20260525-155110.md` — F-6/F-7 で `FailingSeedEntry` の JSON fixture 保存基盤確立。ただし `FailingSeedEntry` は `#[cfg(test)]` 内部構造体であり公開 API ではない。
- `tickets/context/0082-m175-9-small-perturbation-ranking-stability-village-stability/observation-20260525-151745.md` — `StabilityRegressionSummary` と `compare_perturbed_metrics` 実装済み。CSV 出力形式は calibration harness と互換性あり。
- `tickets/context/0081-m175-8-deterministic-replay-village-help/observation-20260525-150011.md` — `SummaryMetrics` と 18 条件グリッド掃引完了。trace の bit-level 再現性確認済み。

### ソースコード調査結果

**既存の実験データ型（統合対象）:**

| 型 | ファイル | 行 | 状態 |
|---|---|---|---|
| `SummaryMetrics` | src/replay.rs | 232 | 公開構造体、replay の要約統計量 |
| `StabilityRegressionSummary` | src/replay.rs | 202 | 公開構造体、perturbation 比較結果 |
| `CalibrationReport` | src/calibration.rs | 139 | 公開構造体、experiment_id/parent_experiment_id を既に保持 |
| `VillageCalibrationResult` | src/calibration.rs | 118 | 公開構造体、J 値と成分を保持 |
| `ReplayTrace` | src/replay.rs | 92 | 公開構造体、全 tick の位置・村・重み・HELP セッション・成長イベント |
| `VillageMetricsSnapshot` | src/village.rs | 353 | 公開構造体、village 状態の統計量 |
| `FailingSeedEntry` | src/replay.rs | 2111 | `#[cfg(test)]` 内部構造体、公開 API へ昇格が必要 |
| `CalibrationReport::parent_experiment_id` | src/calibration.rs | 143 | Option\<String\>、系列管理は単一フィールドのみで体系化されていない |

**既存の定数（再利用可能）:**

| 定数 | 値 | 場所 |
|---|---|---|
| `VILLAGE_FIXTURE_DIR` | "tests/fixtures/village_invariant_failures" | src/constants.rs:583 |
| `OBJECTIVE_WEIGHT_*` | [0.35, 0.25, 0.25, 0.10, 0.05] | src/constants.rs:591-607 |
| `SWEEP_*` | steps/divisions/samples | src/constants.rs:609-619 |

**不足しているインフラ:**

| 項目 | 現状 |
|---|---|
| `VillageExperimentReport` 構造体 | 未定義 |
| Markdown report writer | 未実装 |
| JSON report writer | 未実装（FailingSeedEntry は個別ファイル保存のみ） |
| `rules/darvium/experiment-reporting.md` | ファイル存在せず |
| 系列管理フレームワーク | CalibrationReport の parent_experiment_id のみ、体系なし |
| `FailingSeedEntry` の公開型昇格 | `#[cfg(test)]` スコープ内 |

## Test Plan

### 1. `VillageExperimentReport` 構造体テスト（R-1〜R-4）

**R-1: 全フィールド正常構築**
- 意味: 全実験結果（replay / perturbation / fuzz / calibration）を含むレポートが欠損なく構築できる。
- 入力: ダミーの `SummaryMetrics`、`Vec<StabilityRegressionSummary>`、`Vec<FailingSeedEntry>`、`CalibrationReport` 各1件ずつ
- 期待: `VillageExperimentReport` の各フィールドに値が設定され、`is_empty()` が false を返す

**R-2: empty metrics 耐性**
- 意味: どの実験系統も未実行（空ベクター）の場合でもエラーなく構築できる。
- 入力: 全リストが空、lineage のみ設定
- 期待: `is_empty()` が true を返す。全フィールドにアクセス可能（panic しない）。

**R-3: failure-only ケース耐性**
- 意味: replay trace が空（0 tick）でもレポートが生成できる。
- 入力: `SummaryMetrics` の全値が 0.0、他は正常データ
- 期待: レポートが panic せず構築され、metrics がゼロ埋めされている。

**R-4: best-known parameter bundle の初期値設定**
- 意味: 初回実験時に best-known bundle が未設定（None）であることを確認。
- 期待: `best_known_params` フィールドが `None`。

### 2. Markdown/JSON report writer テスト（W-1〜W-5）

**W-1: Markdown レポートの正常生成**
- 意味: `VillageExperimentReport` から Markdown 文字列が生成され、必須セクションが全て含まれる。
- 期待出力セクション:
  - `# Experiment Report: <id>`
  - `## Lineage`
  - `## Replay Metrics Summary`
  - `## Perturbation Results`
  - `## Calibration Results`
  - `## Failing Seeds`
  - `## Best-Known Parameters`
  - `## Open Anomalies`
- 検証: 各セクションヘッダが Markdown 文字列に含まれることをアサート。

**W-2: JSON レポートのラウンドトリップ**
- 意味: 全フィールドを設定したレポートが JSON にシリアライズ後、デシリアライズで復元できる。
- 検証: `VillageExperimentReport → JSON → VillageExperimentReport` のラウンドトリップで全フィールド一致。

**W-3: 空レポートの JSON 出力**
- 意味: 空（empty）レポートでも有効な JSON が出力され、必須フィールド（experiment_id, lineage, timestamp）が含まれる。
- 検証: JSON パース成功、必須フィールドが null でない。

**W-4: ファイル書き込みの atomicity**
- 意味: `write_markdown` / `write_json` のファイル出力が正常に行われる。
- 検証: 一時ファイルへの書き込み後、内容を読み込んで検証。

**W-5: 不正パス時のエラー伝播**
- 意味: 存在しないディレクトリへの書き込みが適切なエラーを返す。
- 期待: `io::Error` が伝播される。

### 3. Lineage 管理テスト（L-1〜L-4）

**L-1: `ExperimentLineage` の正常構築**
- 意味: 子を持たない lineage が正常に構築できる。
- 入力: experiment_id="exp-20260525-001", parent_ids=[], description="initial"
- 期待: lineage の実験系列深さが 0。

**L-2: 親 lineage の正しい継承**
- 意味: 親実験 ID を指定した子 lineage が正しい深さを持つ。
- 入力: parent_ids=["exp-20260525-001"] の子 lineage
- 期待: lineage の実験系列深さが 1。

**L-3: 循環参照の検出**
- 意味: 自身を親として指定した場合に循環参照エラーが発生する。
- 入力: experiment_id="exp-A", parent_ids=["exp-A"]
- 期待: 検証関数が `Err(CircularLineage)` を返す。

**L-4: `FsLineageStore` のファイル永続化**
- 意味: `FsLineageStore` に lineage を保存 → 読み込みで同一内容が復元される。
- 検証: ラウンドトリップ一致。

### 4. 統合テスト（I-1〜I-3）

**I-1: 4 系統統合レポート生成**
- 意味: M1.75-8/9/10/11 の各出力を模したデータを `VillageExperimentReport` に統合し、Markdown と JSON の両方で出力できる。
- 入力: 各系統 1 件ずつ + lineage 設定
- 検証: Markdown に全セクションが含まれ、JSON ラウンドトリップが一致する。

**I-2: 実験系列の親子関係追跡**
- 意味: 親実験 → 子実験 → 孫実験の 3 世代 lineage が正しく追跡できる。
- 検証: 各世代の lineage から祖先一覧が正しく導出される。

**I-3: failing seed と golden trace の相互参照整合性**
- 意味: FailingSeedEntry の seed が SummaryMetrics の元となった ReplayTrace の seed と矛盾しない。
- 検証: レポート内の全ての seed が同一 seed 空間に属する（同一 source PRNG 系列から生成される）。

### 5. Boy Scout テスト（B-1）

**B-1: `FailingSeedEntry` の公開型昇格後も既存テストが通過**
- 意味: `#[cfg(test)]` 内部構造体から公開型への昇格が既存の fuzzing テスト（F-6, F-7）を破壊しない。
- 検証: 既存テスト全件 PASS。

## 計装方法・観測対象

### 計装方法

- **テストコード**: `src/report.rs` の `mod tests` に全テスト（R-1〜R-4, W-1〜W-5, L-1〜L-4, I-1〜I-3, B-1）を実装
- **固定シード PRNG**: 不要（レポート生成は純粋関数、乱数使用なし）
- **println! 出力**: W-1 で Markdown 出力の内容を --nocapture で観測可能にする
- **ファイル入出力**: 一時ディレクトリ（`std::env::temp_dir()`）を使用し、テスト終了後に削除

### 観測対象

- **レポート完全性**: 各セクションの欠落率（目標: 0%）
- **系列追跡精度**: 世代数に対する正しい祖先導出割合（目標: 100%）
- **Markdown レンダリング結果**: 人間にとって読みやすい整形が行われているか
- **JSON 互換性**: ラウンドトリップ一致率（目標: 100%）

### 較正計画

本チケットは較正ループを実施しない（較正対象の定数なし）。レポート形式自体の設計が主目的であり、パラメータチューニングは M1.75-11 で完了している。

## Boy Scout Rule — 翻訳可能性計画

1. **関数名の動詞句化**: report writer の関数は `generate_report` ではなく `write_markdown_report` / `write_json_report` と命名し、何を・どの形式で出力するかが名前に現れるようにする。
2. **FailingSeedEntry の公開型昇格に伴うリネーム**: 内部構造体のまま放置せず、`pub struct FailingSeedEntry` として公開 API に昇格する。フィールド名も散文として読めるか確認する。
3. **責務分割**: 1 ファイルにレポート構造体定義・writer・lineage 管理を詰め込まず、`src/report.rs` に構造体定義と writer を、lineage 管理は同一ファイル内の独立した型として配置する。Fat module 化を避ける。
4. **マジック文字列禁止**: Markdown セクションヘッダや JSON フィールド名はテスト内でもリテラル直書きせず、定数または生成関数経由で参照する。

## Acceptance Criteria

- [ ] `VillageExperimentReport` 構造体が定義され、4 系統の実験結果を統合できる
- [ ] Markdown report writer が全必須セクションを含むレポートを生成する
- [ ] JSON report writer がラウンドトリップ可能な出力を生成する
- [ ] empty metrics / failure-only ケースでも壊れたレポートを出さない
- [ ] lineage 管理（`ExperimentLineage` + `FsLineageStore`）が正しく動作する
- [ ] 循環参照検出が正しく機能する
- [ ] `FailingSeedEntry` が `#[cfg(test)]` から公開型に昇格されている
- [ ] 全テスト通過（特に既存の replay/perturbation/fuzz/calibration テストに回帰がないこと）
- [ ] `rules/darvium/experiment-reporting.md` が作成されている
- [ ] 翻訳可能性の検証が通っている（関数名・変数名・責務分割が適切）
