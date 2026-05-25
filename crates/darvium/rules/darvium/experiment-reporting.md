# 実験レポート形式（Experiment Reporting）

本ドキュメントは Darvium の観測ベース検証（Observational Testing）における実験レポートの形式・スケルトン・lineage 管理規則を定義する。

## 1. Lineage ID 命名規則

実験系列の識別子は以下の形式に従う：

```
exp-YYYYMMDD-NNN
```

- `YYYYMMDD`: 実験開始日（UTC）
- `NNN`: 当日の連番（001 から開始、999 まで）

例: `exp-20260525-001`, `exp-20260525-002`

親実験 ID は文字列として子実験の `parent_ids` フィールドに記録される。

## 2. レポート必須セクション

全ての実験レポート（Markdown 形式）は以下の 8 セクションを含まなければならない：

| # | セクション | 内容 | 必須 |
|---|-----------|------|------|
| 1 | `# Experiment Report: <id>` | レポートタイトル（実験 ID を含む） | ✅ |
| 2 | `## Lineage` | 実験系列情報（親 ID、説明、タグ、タイムスタンプ） | ✅ |
| 3 | `## Replay Metrics Summary` | replay trace の要約統計量（churn, JSD, survival rate 等） | ✅ |
| 4 | `## Perturbation Results` | perturbation 実験の結果一覧 | 任意 |
| 5 | `## Calibration Results` | 較正実験の結果（目的関数値、パラメータ設定） | 任意 |
| 6 | `## Failing Seeds` | property-based fuzzing で検出された違反 seed 一覧 | 任意 |
| 7 | `## Best-Known Parameters` | 現時点で最適と判断されるパラメータ設定 | 任意 |
| 8 | `## Open Anomalies` | 未解決の異常観測リスト | 任意 |

セクション 4〜8 はデータが空の場合でもセクションヘッダ自体は出力するが、
「(No data)」と表記して空であることを明示する。

## 3. Markdown レポートテンプレート

```markdown
# Experiment Report: exp-YYYYMMDD-NNN

## Lineage

- **Experiment ID**: exp-YYYYMMDD-NNN
- **Parent IDs**: [exp-YYYYMMDD-MMM]
- **Description**: <実験の目的・仮説>
- **Tags**: [tag1, tag2]
- **Timestamp**: YYYY-MM-DDTHH:MM:SSZ

## Replay Metrics Summary

| Metric | P50 | P95 |
|--------|-----|-----|
| Village Churn | <value> | <value> |
| Helper JSD | <value> | <value> |
| Child Survival Rate | <value> | — |
| Child Maturation Rate | <value> | — |
| Helper Count Mean | <value> | — |
| Total Help Sessions | <value> | — |

## Perturbation Results

<summary table or "No data">

## Calibration Results

- **Sweep Mode**: <OFAT | Grid | LHS>
- **Optimal J(θ)**: <value>
- **Optimal Parameters**: <param=value, ...>

## Failing Seeds

<seed list or "No data">

## Best-Known Parameters

<param=value, ... or "Not yet established">

## Open Anomalies

<anomaly list or "None">
```

## 4. JSON レポートスキーマ

JSON レポートは以下の構造を持つ：

```json
{
  "experiment_id": "exp-YYYYMMDD-NNN",
  "lineage": {
    "experiment_id": "exp-YYYYMMDD-NNN",
    "parent_ids": ["exp-YYYYMMDD-MMM"],
    "description": "...",
    "tags": ["tag1"],
    "created_at": "YYYY-MM-DDTHH:MM:SSZ"
  },
  "summary_metrics": {
    "village_churn_p50": 0.0,
    "village_churn_p95": 0.0,
    "helper_jsd_p50": 0.0,
    "helper_jsd_p95": 0.0,
    "child_survival_rate": 0.0,
    "child_maturation_rate": 0.0,
    "helper_count_mean": 0.0,
    "total_help_sessions": 0
  },
  "perturbation_results": [],
  "calibration_report": null,
  "failing_seeds": [],
  "best_known_params": null,
  "open_anomalies": [],
  "timestamp": "YYYY-MM-DDTHH:MM:SSZ"
}
```

全フィールドは存在しなければならない（null や空配列でもキーは出力する）。

## 5. Lineage 追跡規則

- 初回実験（親なし）: `parent_ids` は空配列
- 派生実験（親あり）: 親の `experiment_id` を `parent_ids` に含める
- 循環参照: 自身の ID を親として指定した場合、検証関数が `Err(CircularLineage)` を返さなければならない
- 深さ: 親なし = 深さ 0、親あり = 親の深さ + 1

## 6. ファイル保存規則

- レポートは `experiments/` ディレクトリ配下に保存する
- ファイル名: `<experiment_id>.md`（Markdown）および `<experiment_id>.json`（JSON）
- FsLineageStore の lineage データは `experiments/lineages.json` に保存する
