# M1.76-KW-MTR-C: Execution & Cost Metrics Backfill — 実装成果

## 変更ファイル一覧

### src/simulation.rs
- **SimulationContext** に 3 フィールド追加:
  - `total_gc_collections: u64` — GC で収集 (死亡) された累計ノード数
  - `total_help_attempts: u64` — HELP 試行総数
  - `total_help_successes: u64` — HELP 成功総数 (Succeeded 到達数)
- **SimulationContext::new** で全フィールドを 0 初期化
- **run_evaluation_simulation** 内:
  - phase3_help_protocol 後に `ctx.total_help_attempts += proposals as u64; ctx.total_help_successes += successes_count as u64;`
  - phase4_gc_survival 後に `ctx.total_gc_collections += gc_events as u64;`
- **run_kw_real_simulation** にも同一の counter accumulation を適用 (replace_all)

### src/kind_world.rs
- 新規関数 `compute_execution_success_rate(total_attempts: u64, total_successes: u64) -> f64`
- 新規関数 `compute_cost_efficiency_ratio(total_gc_collections: u64, total_help_attempts: u64, total_help_successes: u64) -> f64`
- **collect_final_metrics** 更新:
  - `execution_success_rate: 0.0` → `compute_execution_success_rate(ctx.total_help_attempts, ctx.total_help_successes)`
  - `cost_efficiency: 0.5` → `compute_cost_efficiency_ratio(ctx.total_gc_collections, ctx.total_help_attempts, ctx.total_help_successes)`
- テスト C1–C7 追加

## 修正した問題

- **既存関数との名前衝突**: `compute_cost_efficiency` が既存の pub fn (SimHelpSession 版, line 782) と衝突したため、新規関数名を `compute_cost_efficiency_ratio` に変更
- **collect_final_metrics の重複フィールド**: 編集途中で旧 `cost_efficiency: 0.5` が残存していた不具合を修正

## 検証結果

- `cargo test`: 1259 passed, 0 failed
- `cargo clippy -- -D warnings`: clean
- 品質チェック: 178 issues 全て既存、新規 issue なし
- 観測テスト: execution_success_rate=0.526316 (>0.0), cost_efficiency=0.114943 (≠0.5)
