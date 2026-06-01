# 実装サマリ: SubWorkflow 親子関係に基づく GC 生存ガード

## 変更ファイル（3ファイル）

### src/trust.rs
- MemoizedGraph に parent_id: usize 追加
- new(), new_with_position() で 0 初期化

### src/store/coordinator.rs
- MemoizedGraph 構築箇所に parent_id: 0 追加

### src/simulation.rs
- run_self_refinement_for_population 内で子生成時に person.parent_id = pid を設定
- phase4_gc_survival に親生存ガード追加（alive=false 直前で親が生きている子はスキップ）
- TC1〜TC4 テスト追加

## ガード条件
```rust
if parent_id > 0
    && parent_id < ctx.population.len()
    && ctx.population[parent_id].alive
{
    continue; // 親生存 → 子を殺さない
}
```

## 検証結果
- cargo test --features server → 1394 passed, 0 failed（4 new tests）
- TC1〜TC4 全 PASS
