# 計画: チケット#140 — 評判ベースGCのプロダクション実装とMYCUTE結合設計

## 要件の再確認

1. DarviumConfig に gc_interval/gc_enabled/lifecycle_success_stub を追加
2. Darvium::run_lifecycle_gc() — GC パイプラインのプロダクション駆動
3. GraphStore トレイトに delete_workflow_graph 追加
4. DualStoreCoordinator::delete_graph() 追加
5. MYCUTE 結合設計の提示
6. テスト T1-T5 + cargo test 回帰

## 変更ファイル一覧

| ファイル | 種別 | 内容 |
|----------|------|------|
| src/lib.rs | 修正 | DarviumConfig 拡張、run_lifecycle_gc() 追加、T1-T5 テスト追加 |
| src/store/graph_store.rs | 修正 | GraphStore トレイト + InMemoryGraphStore に delete_workflow_graph 追加 |
| src/store/coordinator.rs | 修正 | DualStoreCoordinator::delete_graph() 追加、FailingGraphStore 修正 |
| tickets/specs/0140-gcmycute.md | 修正 | Investigation 行番号更新 |

## 計装・観測の実装計画

- テスト: lib.rs #[cfg(test)] に T1-T5 追加
- --nocapture で削除候補数・LifecycleScore 分布出力
- spec の「計装方法・観測対象」通り通常の cargo test

## 実装手順

1. GraphStore トレイトに delete_workflow_graph 追加 + InMemoryGraphStore 実装
2. FailingGraphStore + DualStoreCoordinator::delete_graph 追加
3. DarviumConfig 拡張 + Darvium::run_lifecycle_gc() 実装
4. Investigation 更新
5. テスト実装 + cargo test
6. 観察レポート保存

## 物理的レビュー方法

run-quality-checks.js src/lib.rs src/store/graph_store.rs src/store/coordinator.rs | generate-report.js

## リスク

- 低: delete_workflow_graph のトレイト追加によるコンパイル要求
- 低: DarviumConfig 拡張による破壊（MYCUTE 未使用のため実質影響なし）
- 低: gc_enabled: false デフォルトで安全
