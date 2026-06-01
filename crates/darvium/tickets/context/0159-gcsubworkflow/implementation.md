# 実装サマリ: 本番 GC パスへの SubWorkflow 親生存ガード

## 変更ファイル（3ファイル）

### src/lifecycle.rs
- compute_and_update_gc_state に `parent_is_alive: Option<bool>` 引数を追加
- transition_gc_state の結果が SoftDeleted 以上で親生存中の場合、gc_state の進行を抑制

### src/lib.rs
- Darvium::run_lifecycle_gc で全グラフの gc_state から親生存マップを事前構築
- compute_and_update_gc_state に親生存状態を渡す
- parent_id > 0 で親が Active/Protected → 子の gc_state 進行をブロック

### src/simulation.rs
- phase4_gc_survival の compute_and_update_gc_state 呼出に None を追加

## ガード論理
```
親生存(Some(true)) + 子の gc_state が SoftDeleted 以上に進行しようとしている
  → gc_state の変更をブロック（Active に留める）
親死亡(Some(false)) または parent_id=0(None)
  → 通常通り gc_state 進行
```

## 検証結果
- cargo test --features server → 1394 passed, 0 failed（回帰なし）
