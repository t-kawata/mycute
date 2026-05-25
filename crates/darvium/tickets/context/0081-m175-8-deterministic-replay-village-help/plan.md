# 計画: M1.75-8 village-help 決定論的リプレイ

## 要件

RFC §41B.16 に従い、以下の8出力の決定論的リプレイカバレッジを実装：
1. 空間位置更新、2. ローカルビレッジメンバーシップ、3. ヘルプ提案集合
4. アダルトオファー決定、5. チャイルド受入/拒否決定、6. 実現ヘルパー集合
7. ヘルプ成功結果、8. チャイルド成長指標

## 変更ファイル一覧

| ファイル | 種別 | 内容 |
|----------|------|------|
| src/replay.rs | 新規 | 全リプレイ種別・関数・テスト |
| src/constants.rs | 編集 | REPLAY_POSITION_DELTA_SIGMA 追加 |
| src/lib.rs | 編集 | mod + re-export + Darvium メソッド追加 |

## 新規型

- VillageReplayScenario, WorkflowConfig, MissionSpec, ClockSchedule, PolicyBundle
- ReplayTrace, TickPositions, TickVillages, TickHelperWeights, HelpSessionTrace, GrowthEvent, SummaryMetrics

## 関数

- run_replay_scenario, trace_eq, trace_diff_fields, trace_summary_metrics

## テスト

T-1〜T-9 (不変条件) + T-O1, T-O2 (観測)

## レビュー方法

1. run-quality-checks.js
2. cargo test (全PASS)
