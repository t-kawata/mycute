# 実装サマリ: SearchWorkflow 複雑化

## 変更したファイル

### src/constants.rs
- DIFF_MUT_UPDATE_PROMPT_PROB: 0.05 (Calibration Candidate)
- DIFF_MUT_ADD_EDGE_PROB: 0.10 (Calibration Candidate)
- DIFF_MUT_ADD_NODE_PROB: 0.70 (Calibration Candidate) — 20%→70%に引上
- DIFF_MUT_REPLACE_NODE_PROB: 0.10 (Calibration Candidate)
- DIFF_MUT_REMOVE_EDGE_PROB: 0.05 (Calibration Candidate)
- COMPOSE_CANDIDATE_COUNT: 3 (従来2→3に増加)
- PATCH_EXISTING_THRESHOLD: 0.25 (新規)

### src/workflow_generation.rs
- generate_differential_mutation のマジックナンバー(0.30,0.55,0.75,0.90)を
  名前付き定数から累積閾値を計算するよう変更
- add_node確率 20%→70%、update_prompt 30%→5%、他も低減

### src/search_workflow.rs
- COMPOSE_CANDIDATE_COUNT と GENERATION_COMPLEXITY を削除 (constants.rsに移動)
- try_patch_existing メソッド追加: 単一AgentStepノードを追加するGraphPatchを生成
- execute() にPATCH分岐追加: best_score >= 0.25 で PatchExisting を返す
- FSMコメント更新: PATCHパスを明記
- テスト追加: T1 (PatchExisting到達性確認, 2件)

### src/simulation.rs
- generate_workflow_for_child のフォールバック複雑度をtick依存に変更
  - tick < 10: complexity=1 (sequential)
  - tick >= 10: complexity=2 (WORKFLOW_GENERATION_MAX_COMPLEXITY=DAG)
- 各outcome選択を [OBS] println! で計装
- _ ワイルドカードを明示的アーム (AbortSearch / NeedsHumanReview / Err) に変更

## テスト結果
- cargo test: 1349 passed, 0 failed, 62 ignored
- 新規テスト:
  - patch_existing_returns_valid_patch: ✅ (T1)
  - execute_returns_patch_existing_for_single_candidate: ✅ (T1)
  - differential_mutation_add_node_probability: ✅ (T3)
