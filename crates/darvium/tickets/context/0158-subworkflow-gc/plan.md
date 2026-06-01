# 計画: SubWorkflow 親子関係に基づく GC 生存ガード

## 変更ファイル
| ファイル | 内容 |
|---------|------|
| src/trust.rs | MemoizedGraph.parent_id: usize 追加 |
| src/store/coordinator.rs | struct リテラルに parent_id: 0 追加 |
| src/simulation.rs | 抽象化時に parent_id 設定 + GC ガード + TC1-TC4 |

## 実装手順
1. MemoizedGraph.parent_id 追加 + 0 初期化
2. coordinator.rs 更新
3. run_self_refinement_for_population で person.parent_id = pid
4. phase4_gc_survival に親生存ガード追加
5. TC1-TC4 テスト追加
6. cargo check → cargo test

## リスク
- parent_id の範囲外アクセス → ガード条件で防止
