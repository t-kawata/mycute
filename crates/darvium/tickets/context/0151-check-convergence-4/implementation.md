# 変更したファイル一覧と実装内容の概要

## src/simulation.rs のみ

**Step 1**: 生存者限定版 Map 関数3つを SimulationContext に追加
- `gc_states_map_for_ids(ids: &[PersonId])` 
- `last_update_ticks_map_for_ids(ids: &[PersonId])`
- `positions_map_for_ids(ids: &[PersonId])`

**Step 2**: `check_convergence` 内の3つの Map 呼び出しを生存者限定版に変更
- `gc_states_map()` → `gc_states_map_for_ids(alive_ids)`
- `last_update_ticks_map()` → `last_update_ticks_map_for_ids(alive_ids)`
- `positions_map()` → `positions_map_for_ids(alive_ids)`

**Step 3**: Phase 3.7 首長性スコア計算ループを `alive_ids` 使用に変更
- `for person in ctx.population.iter_mut().filter(|p| p.alive)` → `for &pid in &alive_ids`
