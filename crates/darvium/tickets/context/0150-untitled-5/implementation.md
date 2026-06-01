# 変更したファイル一覧と実装内容の概要

## src/simulation.rs のみ (再実装)

8つの関数に `alive_ids: &[PersonId]` 引数を追加し、内部の `ctx.alive_ids()` 呼び出しを置換:

| 関数 | 変更内容 |
|------|---------|
| `phase1_population_growth` | 生存成人フィルタを引数リストから派生 |
| `phase2_village_clustering` | `ctx.alive_ids()` → 引数 |
| `compute_village_centrality` | 同上 |
| `phase3_help_protocol` | 同上（成人/子分割も引数から派生） |
| `phase4_gc_survival` | 同上 |
| `run_self_refinement_for_population` | 同上 |
| `observe_kw_real_tick` | 同上 |
| `check_convergence` | 同上 |

3つのシミュレーションループで同一パターンを適用:
- tick 先頭で `ctx.alive_ids()` を1回のみ呼出
- Phase 1 出生後に `extend(&births_ids)` で生存者リストを更新
- 更新済みリストを全フェーズに引数として渡す

テスト内の呼び出し元も全箇所修正。
