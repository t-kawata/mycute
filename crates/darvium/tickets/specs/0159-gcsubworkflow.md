---
ticket_id: 159
title: 本番GCパスへのSubWorkflow親生存ガード実装
slug: gcsubworkflow
status: reviewed
created_at: 2026-06-01
updated_at: 2026-06-01
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0159-gcsubworkflow/plan.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0159-gcsubworkflow/implementation.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0159-gcsubworkflow/observation-20260601-173808.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0159-gcsubworkflow/review.md
---
# 本番GCパスへのSubWorkflow親生存ガード実装

## Summary

#158 で実装した SubWorkflow 親生存ガードはシミュレーションパス（`phase4_gc_survival`）のみ対応。本チケットでは本番 GC パス（`Darvium::run_lifecycle_gc` in `src/lib.rs`）にも同様の親生存ガードを実装する。

## Background

**#158 の未完了タスク**: `MemoizedGraph.parent_id` は全グラフ型に追加済みだが、GC ガードは `simulation.rs` の `phase4_gc_survival` にしか実装されていない。

**本番パスの違い**:
- シミュレーション: `alive = false` をスキップ（即死防止）
- 本番: `gc_state` の進行を止める（`Active` に留めて SoftDeleted 以上に進ませない）

**共通基盤**: 本番パスも `MemoizedGraph` を操作する（`registry.resolve_mut` が `&mut MemoizedGraph` を返す）。`parent_id` は既に存在する。

## Scope

`compute_and_update_gc_state`（`src/lifecycle.rs:68`）に親生存ガードを追加：
- `transition_gc_state` の結果が SoftDeleted 以上の場合、親が生存中（`gc_state == Active`）なら子の gc_state を進行させない
- 親の `parent_id` は `MemoizedGraph` に存在。親の所在は `WorkflowGraphId` 経由（本番は String ID）

## Non-scope

- シミュレーションパスの変更（#158 で完了）
- 推移的な親子保護

## Investigation

### 物理的証拠

**証拠1: 本番 GC パス（`src/lib.rs:246-282`）**
→ parent_id の参照が一切ない。

**証拠2: compute_and_update_gc_state（`lifecycle.rs:90`）**
```rust
let new_gc_state = transition_gc_state(graph.gc_state, hazard);
if new_gc_state != graph.gc_state {
    graph.gc_state = new_gc_state;  // ← 親生存チェックなし
}
```

**証拠3**: 親の生存判定は `gc_state <= Active`（Protected または Active）で判断可能。

## Test Plan

| # | 内容 |
|---|------|
| TC1 | 親 Active → 子の gc_state が Active に留まる |
| TC2 | 親 Tombstoned → 子の gc_state が進行可能 |
| TC3 | parent_id=0 → 通常進行 |
| TC4 | 親がレジストリ不在 → 安全にスキップ |

## Acceptance Criteria

- [ ] 本番 GC パスで親生存中の子が SoftDeleted 以上に進行しない
- [ ] 親死亡後は子が通常通り GC 進行可能
- [ ] 既存テスト全通過

## Notes

<!--
注: このコメントは人間向けの説明である。AI は以下の手順に従うこと。

- plan_path: /plan-ticket が plan.md を作成後に frontmatter に更新する
- implementation_path: /start-ticket が implementation.md を作成後に frontmatter に更新する
- review_report_path: /review-ticket が review.md を作成後に frontmatter に更新する
- observation_report_path: /start-ticket が observation-YYYYMMDD-HHmmss.md を作成後に frontmatter に最新パスを更新する

各コマンドのワークフロー手順が frontmatter 更新の正しい手順である。
-->

### 成果物

- 計画: context/0159-gcsubworkflow/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0159-gcsubworkflow/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0159-gcsubworkflow/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0159-gcsubworkflow/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
