# 実装サマリ: M1.76-11 ReciprocityEvent インジェスション + reputation/hazard recompute パイプライン

## 変更ファイル一覧

| ファイル | 変更内容 |
|---------|---------|
| src/constants.rs | RECIPROCITY_EVENT_PROJECTION_NAME, RECIPROCITY_STORE_INITIAL_CAPACITY を追加 |
| src/event.rs | GraphMetrics 構造体（7フィールド + Default impl）を追加 |
| src/reciprocity.rs | ReciprocityEventStore, ingest_reciprocity_event, recompute_all_profiles, recompute_all_gc_hazards, ReciprocityReplaySnapshot, ReciprocityDiffReport, compute_replay_comparison, テスト R11-T1〜R11-T9 を追加 |

## テスト実行結果

- cargo test: 全件 PASS
- cargo clippy: 全警告クリア
- quality checks: 517 issues（すべて既存コード由来の pre-existing 問題、新規コードにはなし）

## 成果物パス

- Spec: tickets/specs/0096-m176-11-reciprocity-event-ingestion-reputation-hazard-recompute-pipeline.md
- Observation: tickets/context/0096-m176-11-reciprocity-event-ingestion-reputation-hazard-recompute-pipeline/observation-20260526-101011.md
