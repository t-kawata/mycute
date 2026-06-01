---
ticket_id: 158
title: SubWorkflow 親子関係に基づく GC 生存ガード
slug: subworkflow-gc
status: reviewed
created_at: 2026-06-01
updated_at: 2026-06-01
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0158-subworkflow-gc/plan.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0158-subworkflow-gc/implementation.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0158-subworkflow-gc/observation-20260601-172718.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0158-subworkflow-gc/review.md
---
# SubWorkflow 親子関係に基づく GC 生存ガード

## Summary

抽象化（self-refinement）により別の個体の SubWorkflow として参照されているワークフローグラフが、親が生存中に GC で死亡すると、親のグラフ構造に dangling reference が発生する。本チケットでは各 `MemoizedGraph` に `parent_id: usize`（0 = 親なし）を追加し、`phase4_gc_survival` において「親が生存中は子を殺さない」ガード条件を実装する。

## Background

**問題**: 抽象化パイプライン（self-refinement）は `AbstractableSubgraph` を抽出し、元グラフ内の該当ノードを `WorkflowNode::SubWorkflow { workflow_id }` で置換する。抽出された部分グラフは独立した `MemoizedGraph` として登録される。しかし GC は各 `MemoizedGraph` を独立に評価するため、親が生存中でも子が死亡しうる。このとき親の WorkflowGraph 内の `SubWorkflow` ノードが参照先を失う（dangling reference）。

**ガード条件**: 子の GC 判定時、「親が生存中（`alive = true`）かつ親の ID が 0 以外」であれば、子の生存確率にかかわらず子を殺さない。親がすでに死亡している場合は通常の GC 判定で子の生死を決める（親が死んだ＝子の保護が解かれる）。

**注意**: 親が死んでも子が自動的に死ぬわけではない。親死亡後も子は自身の LifecycleScore・benevolence・sophistication で評価される。

## Scope

1. **`MemoizedGraph` に `parent_id: usize` フィールド追加**（`src/trust.rs`）
   - 0 = 親なし（ルートグラフ）、1+ = 親の population 内インデックス
2. **抽象化時に parent_id を設定**（`src/simulation.rs`）
3. **`phase4_gc_survival` に親生存ガードを追加**（`src/simulation.rs`）
4. **全 struct リテラル更新**（coordinator.rs 等）

## Non-scope

- 親→子の一覧（参照カウント方式）は実装しない
- 推移的な親子保護（親の親を辿る）は最初は行わない
- SubWorkflow の多重継承（複数の親が同一子を参照）は非対応

## Investigation

### 物理的証拠

**証拠1: phase4_gc_survival にガードなし**（`simulation.rs:3440-3473`）
→ 子が無条件に殺される。

**証拠2: MemoizedGraph に parent_id なし**（`trust.rs:32-70`）
→ 新規フィールド追加が必要。

**証拠3: AbstractableSubgraph に parent_id あり**（`types.rs:109`）
→ 抽象化抽出時に親の WorkflowGraphId は把握可能。

## Test Plan

| # | 内容 |
|---|------|
| TC1 | parent_id=0 の子 → 従来通り通常 GC 判定 |
| TC2 | parent_id>0, 親生存 → 絶対に死なない |
| TC3 | parent_id>0, 親死亡 → 通常 GC 判定 |
| TC4 | parent_id が範囲外 → panics しない |

## Acceptance Criteria

- [ ] MemoizedGraph.parent_id 追加、0 初期化
- [ ] 親生存中の子が絶対に死なない
- [ ] 親死亡後は通常 GC 判定
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

- 計画: context/0158-subworkflow-gc/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0158-subworkflow-gc/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0158-subworkflow-gc/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0158-subworkflow-gc/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
