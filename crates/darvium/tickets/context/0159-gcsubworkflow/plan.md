# 計画: 本番 GC パスへの SubWorkflow 親生存ガード

## 変更ファイル
| ファイル | 内容 |
|---------|------|
| src/lifecycle.rs | compute_and_update_gc_state に is_parent_alive クロージャ引数追加 |
| src/lib.rs | Darvium::run_lifecycle_gc で親生存チェック |
| src/simulation.rs | phase4_gc_survival の呼出に None 追加 |

## 実装手順
1. compute_and_update_gc_state に is_parent_alive: Option<&dyn Fn() -> bool> 追加
2. transition_gc_state 結果が SoftDeleted 以上で親生存中は遷移スキップ
3. Darvium::run_lifecycle_gc で親 gc_state を確認するクロージャを渡す
4. phase4_gc_survival の呼出に None 追加
5. テスト追加
