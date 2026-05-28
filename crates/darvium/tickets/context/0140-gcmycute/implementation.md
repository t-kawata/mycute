# 変更したファイル一覧と実装内容の概要

## src/lib.rs
- DarviumConfig 拡張: gc_interval (default 1000), gc_enabled (default false), lifecycle_success_stub (default 0.5) を追加
- `#[allow(dead_code)]` 削除（config フィールドが run_lifecycle_gc で使用されるため）
- `Darvium::run_lifecycle_gc()` 追加: WorkflowRegistry 内の全グラフに対して LifecycleScore → compute_gc_hazard → transition_gc_state パイプラインを実行
- テスト T1-T5 追加:
  - T1: 3グラフ登録で GC 2回実行 → HardDeleteCandidate 確認
  - T2: 空レジストリ → 空リスト
  - T3: gc_enabled=false → 空リスト
  - T4: transition_gc_state 全7ルール検証
  - T5: DualStoreCoordinator::delete_graph() 削除確認

## src/store/graph_store.rs
- GraphStore トレイトに `delete_workflow_graph(&self, graph_id: &GraphId)` 追加
- InMemoryGraphStore 実装: graphs/embeddings/reputations の RefCell マップから削除

## src/store/coordinator.rs
- FailingGraphStore: delete_workflow_graph デリゲート追加（maybe_fail → inner）
- DualStoreCoordinator::delete_graph() 追加
- GraphId インポート追加
