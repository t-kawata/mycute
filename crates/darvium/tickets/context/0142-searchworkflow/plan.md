# SearchWorkflow 複雑化 — 実装計画

## 要件の再確認

出生時のワークフロー生成パス（PatchExisting / ComposeExisting / Differential Mutation / GenerateNew）を修正し、世代を経るにつれてノード数が増加するようにする。

## 変更ファイル一覧

| ファイル | 種別 | 内容 |
|---------|------|------|
| src/constants.rs | 追加 | DIFF_MUT_* 確率定数 (5件)、COMPOSE_CANDIDATE_COUNT、PATCH_EXISTING_THRESHOLD |
| src/workflow_generation.rs | 修正 | マジックナンバー→名前付き定数、add_node確率20%→70% |
| src/search_workflow.rs | 修正 | PatchExisting FSM分岐追加、COMPOSE_CANDIDATE_COUNT 3に増加 |
| src/simulation.rs | 修正 | フォールバック複雑度の動的化 (tick依存) |

## 計装・観測の実装計画

- generate_workflow_for_child 内で各outcomeの選択を [OBS] println! で計装
- StdRng::seed_from_u64(12345) 固定シードで再現性保証
- 観測対象: outcome種別 (PATCH/COMPOSE/NEW/REUSE/Fallback) と生成グラフノード数

## 実装手順

1. constants.rs にDIFF_MUT確率定数、COMPOSE_CANDIDATE_COUNT=3、PATCH_EXISTING_THRESHOLD=0.25 を追加
2. workflow_generation.rs で累積閾値を名前付き定数から計算するよう変更
3. search_workflow.rs に try_patch_existing 追加、execute() にPATCH分岐追加
4. simulation.rs でフォールバック複雑度をtick依存に変更
5. 不変条件テスト追加 (T1: PatchExisting到達性, T3: add_node確率)
6. cargo test 全通過確認

## Boy Scout 改善

- simulate_workflow_for_child の _ ワイルドカード→明示的アームに変更 (AbortSearch / NeedsHumanReview / Err)

## レビュー方法

- cargo test 全通過
- run-quality-checks + generate-report

## リスク

- FSM遷移違反による実行時エラー (対応済: Refine→ProposeNew 経由で遷移)
