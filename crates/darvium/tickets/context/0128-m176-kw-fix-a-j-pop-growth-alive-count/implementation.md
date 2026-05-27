# 変更したファイル一覧と実装内容の概要

## 変更ファイル

### 1. src/simulation.rs

| 変更 | 行 | 内容 |
|------|----|------|
| バグ修正 | check_convergence 関数内 | alive_count 計算を `config.population_size.saturating_sub(dead.len())` → `ctx.population_count().saturating_sub(dead.len())` に修正 |
| Boy Scout 改善 | check_convergence 関数 (新規) | 収束判定ブロック（旧 35行）を独立関数 `check_convergence()` として抽出。責務を明確化 |
| 呼び出し置換 | inline ブロック | 旧インライン収束判定ブロックを `check_convergence()` 呼び出し1行に置換 |
| テスト追加 | mod tests 末尾 | FIX-A1〜A5 テストケース追加 |

**バグ修正の詳細**:
- 旧: `let alive_count = config.population_size.saturating_sub(dead.len());`
- 新: `let alive_count = ctx.population_count().saturating_sub(dead.len());`
- `config.population_size` は初期人口固定値 → 出生ノード非考慮
- `ctx.population_count()` は全グラフノード数（出生 + 初期）を返す
- `dead.len()` は全 tick で累積されるため初期人口を超え得る
- 旧式では常に `alive_count ≤ initial_population` → j_pop_growth = 0.0 固定

**追加テスト**:
| ID | 内容 | 検証 |
|----|------|------|
| FIX-A1 | 死亡なし・出生あり → j_pop_growth=0.5 | assert! |
| FIX-A2 | 死亡 > 出生 → j_pop_growth=0.0 | assert! |
| FIX-A3 | 死亡 = 出生 → j_pop_growth=0.0 | assert! |
| FIX-A4 | dead > total_nodes → saturating_sub=0 | assert_eq! |
| FIX-A5 | 観測テスト: ctx.population_count() 使用確認 | println! + --nocapture |

### 2. src/kind_world.rs

| 変更 | 行 | 内容 |
|------|----|------|
| テスト隔離 | tc6_kw4_optimize_run | `#[ignore]` 追加（長時間テスト） |
| テスト隔離 | tc7_kw4_different_ranges_different_results | `#[ignore]` 追加（長時間テスト） |
| テスト隔離 | tc6e_kw4_report_fields_present | `#[ignore]` 追加（長時間テスト） |
| テスト隔離 | tc7e_kw4_best_j_kw_social_positive | `#[ignore]` 追加（長時間テスト） |

`tc6_kw4_optimize_run`（200 iter Nelder-Mead）を始めとする較正ループテストを `#[ignore]` で隔離。
`cargo test` では走らず、`cargo test -- --ignored` でのみ実行される。

## 既知の問題（本チケットスコープ外）

- `d7_collect_final_metrics_capability_knowledge_valid` — reuse_ratio=0（MTR-D 領域の既存不具合）
- `e6_mtre_collect_final_metrics_non_default` — benevolent_ratio=1（MTR-D 領域の既存不具合）
