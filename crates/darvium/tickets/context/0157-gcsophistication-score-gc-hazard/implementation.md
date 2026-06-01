# 実装サマリ: 洗練スコアの GC 保護組み込み

## 変更ファイル（6ファイル）

### src/trust.rs
- MemoizedGraph に sophistication_score: f32 追加
- new(), new_with_position() で 0.0 初期化

### src/store/coordinator.rs
- MemoizedGraph 構築箇所に sophistication_score: 0.0 追加

### src/constants.rs
- GC_HAZARD_GAMMA_SOPHISTICATION = 0.50 追加

### src/event.rs
- ReciprocityLifecyclePolicy に gamma_sophistication フィールド追加
- Default impl で上記定数を参照
- テスト内 struct リテラルに gamma_sophistication 追加

### src/reciprocity.rs
- compute_gc_hazard に第4引数 sophistication_score: f32 追加
- F-7 式: 末尾項 - gamma_sophistication * sophistication_score を追加
- 全テスト呼び出し箇所（約30箇所）に 0.0 追加

### src/lifecycle.rs
- compute_and_update_gc_state で graph.sophistication_score を読み取り compute_gc_hazard に伝播

### src/simulation.rs
- Phase 3.7 の sophistication_score 計算後にキャッシュ書き込みを追加
- run_lifecycle_gc（旧パス）の compute_gc_hazard 呼出に 0.0 追加
- テスト内の compute_gc_hazard 呼出に 0.0 追加

## 検証結果
- cargo test --features server → 1390 passed, 0 failed, 73 ignored
- 新規テスト未追加（既存テストで regression 確認）
