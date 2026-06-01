# 計画: 洗練スコアの GC 保護組み込み

## 変更ファイル
| ファイル | 内容 |
|---------|------|
| src/trust.rs | MemoizedGraph.sophistication_score: f32 追加 |
| src/constants.rs | GC_HAZARD_GAMMA_SOPHISTICATION = 0.5 追加 |
| src/event.rs | ReciprocityLifecyclePolicy.gamma_sophistication 追加 |
| src/reciprocity.rs | compute_gc_hazard 第4引数追加 + 全テスト呼出に 0.0 + TC1-TC4 |
| src/lifecycle.rs | compute_and_update_gc_state でキャッシュ値参照 |
| src/simulation.rs | Phase 3.7 キャッシュ書き込み + run_lifecycle_gc 更新 |

## 実装手順
1. MemoizedGraph にフィールド追加（trust.rs）
2. 定数追加（constants.rs）+ ポリシーフィールド（event.rs）
3. compute_gc_hazard シグネチャ変更（reciprocity.rs）
4. compute_and_update_gc_state 更新（lifecycle.rs）
5. Phase 3.7 キャッシュ書き込み + 全呼出更新（simulation.rs）
6. 全テスト呼出に 0.0 追加 + TC1-TC4（reciprocity.rs）
7. cargo check → cargo test

## リスク
- compute_gc_hazard の呼出が約30箇所あり更新漏れリスク → replace_all + cargo check で網羅確認
