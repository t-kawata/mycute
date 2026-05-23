# 変更したファイル一覧と実装内容の概要

## 変更ファイル

| ファイル | 種別 | 内容 |
|---|---|---|
| src/constants.rs | 更新 | 8つのパッチ定数追加 (PATCH_CONFIDENCE_THRESHOLD 等) |
| src/types.rs | 更新 | use crate::patch::GraphPatch 追加、WorkflowNode に PartialEq 追加、GraphPatch スタブ削除、テストコード値更新 |
| src/patch.rs | 新規作成 | PatchOperation(7変種)、PatchConfidence(幾何平均+動的重み切替)、GraphPatch、PatchError(6変種)、apply_operation(7操作)、apply_patch_atomic(4フェーズ)、validate_patch_result(3段階)、validate_var_scope(V-03)、validate_subworkflow_refs、validate_dag(topo sort) |
| src/lib.rs | 更新 | pub mod patch 追加、公開API再エクスポート追加 |
| tests/m0_5.rs | 新規作成 | OTS-C1 (n=10,000 サイクル検出完全性) + OTS-C2 (n=1,000 ノイズ注入安全性) |

## 実装の概要

- RFC §12.1 準拠の PatchOperation enum (7変種) と GraphPatch struct
- RFC §12.3 準拠の PatchConfidence 幾何平均計算 + 動的重み切替 (cs<0.50 時 ws=0.20/wv=0.50)
- RFC §12.4 準拠の apply_patch_atomic 4フェーズ (clone→apply→validate→swap)
- RFC §12.6 準拠の PatchError enum (6変種)
- RFC §14.2 準拠の validate_patch_result 3段階検証 (DAG→変数スコープ→SubWorkflow参照)
- 固定シード PRNG (StdRng::seed_from_u64(12345)) による再現可能な観測テスト
