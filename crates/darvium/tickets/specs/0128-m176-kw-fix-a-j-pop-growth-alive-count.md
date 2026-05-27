---
ticket_id: 128
title: M1.76-KW-FIX-A: j_pop_growth 計算バグ修正 — alive_count の計算式を出生ノード対応に修正
slug: m176-kw-fix-a-j-pop-growth-alive-count
status: reviewed
created_at: 2026-05-27
updated_at: 2026-05-27
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0128-m176-kw-fix-a-j-pop-growth-alive-count/observation-20260527-174849.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0128-m176-kw-fix-a-j-pop-growth-alive-count/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0128-m176-kw-fix-a-j-pop-growth-alive-count/review.md
---

# M1.76-KW-FIX-A: j_pop_growth 計算バグ修正 — alive_count の計算式を出生ノード対応に修正

## Summary

`simulation.rs` の収束判定ブロック（`tick_to_convergence`）内で `j_pop_growth` が常に 0.0 に固定されるバグを修正する。原因は `alive_count` 計算が初期人口固定の `config.population_size` をベースにしており、出生ノードが計上されていないため。`ctx.population_count()` に変更することで出生ノードを正しく反映する。

**重要**: 本バグは収束判定の `j_pop_growth` にのみ影響する。最終 metrics（`collect_final_metrics` → `phase6_measure_jkw`）の `population_growth_rate` 経由の `j_pop_growth` は正しい計算式で算出されている。

## Background

Kind World 較正ループ（#127）の構造的問題分析により、収束判定ブロックの `j_pop_growth` が常に 0.0 になっていることが判明。

**Buggy code**（`simulation.rs:1531-1535`）:
```rust
let alive_count = config.population_size.saturating_sub(dead.len());
let j_pop_growth =
    ((alive_count as f64 / config.population_size as f64) - 1.0).clamp(0.0, 1.0);
```

**問題点**:
1. `dead.len()` は出生ノードの死亡も含むため初期 `config.population_size` を超え得る
2. `saturating_sub` により `alive_count` は常に ≤ `config.population_size`
3. `(alive / initial - 1.0) ≤ 0.0` → `clamp(0.0, 1.0)` 後 **0.0 固定**
4. 出生ノードそのものが `alive_count` に計上されていない（`config.population_size` は初期値固定）

`s_growth` の 25%（4 成分中の `j_pop_growth`）が永久欠損し、`tick_to_convergence` が正しく機能しない。

**対照: 正しい実装パターン**（`simulation.rs:2027-2029`, `phase6_measure_jkw`）:
```rust
let alive_count = (0..ctx.population_count())
    .filter(|id| !dead.contains(id))
    .count();
```
こちらは `ctx.population_count()` を使用して出生ノードを正しく計上している。

## Scope

1. **`simulation.rs:1531` の修正**: `config.population_size.saturating_sub(dead.len())` を `ctx.population_count().saturating_sub(dead.len())` に変更
2. **テストコードの追加**: 修正前後の動作を検証するテストケース FIX-A1〜FIX-A6

## Non-scope

- **収束閾値 `KW4_CONVERGENCE_THRESHOLD` の調整は行わない**（FIX-B/C/D 完了後に KW4 較正ループで実施）
- **`s_speed` 計算式の変更は行わない**
- **`collect_final_metrics` の `population_growth_rate`** は既に正しいため修正不要
- 他の FIX チケット（B/C/D）の範囲には介入しない

## Investigation

### 物理的証拠 1: バグのある実装箇所

`simulation.rs:1531-1535`:
```rust
let alive_count = config.population_size.saturating_sub(dead.len());
let j_pop_growth =
    ((alive_count as f64 / config.population_size as f64) - 1.0).clamp(0.0, 1.0);
```

`config.population_size` は初期人口固定値。`dead` は `HashSet<NodeId>` として `simulation.rs:1430` で初期化され、全 tick で累積される。出生ノードが死亡しても `dead` に追加されるため、`dead.len()` は初期人口を超え得る。

### 物理的証拠 2: 正しい実装パターン（同一ファイル内）

`simulation.rs:2027-2029`（`phase6_measure_jkw`）:
```rust
let alive_count = (0..ctx.population_count())
    .filter(|id| !dead.contains(id))
    .count();
let pop_growth = if initial_population > 0 {
    alive_count as f64 / initial_population as f64 - 1.0
};
```

こちらは `ctx.population_count()` を使用し、出生ノードも正しく計上している。この値が最終的な `KindWorldMetricsInput.population_growth_rate` として `j_pop_growth` に反映される。

### 物理的証拠 3: population_count() の定義

`simulation.rs:364-366`:
```rust
pub fn population_count(&self) -> usize {
    self.memoized_graph.graph.node_count()
}
```

グラフ上の全ノード数（初期ノード + `add_person` で追加された出生ノード）を返す。死亡ノードはグラフから削除されず `dead` セットで管理されるため、`population_count()` は生存＋死亡の総ノード数を返す。

### 物理的証拠 4: dead は全 tick で累積

`simulation.rs:1430`:
```rust
let mut dead: HashSet<NodeId> = HashSet::new();
```

tick ループの外で初期化され、GC フェーズ（Phase 4）で死亡ノードが追加される。リセットされることはない。

### 物理的証拠 5: 観測結果からの間接的証拠

`observation-20260527-163702.md`（チケット #127 KW4 較正）:
```
Cycle 3: J_kw_social = 0.061, ttc=10, converged=false（200 iter 収束せず）
```

`ttc=10`（最初のサンプリングポイントで収束検出）は、`j_pop_growth=0.0` でも `s_growth * j_cov > 0.1` が成立することを示す。

### 参照観察レポート

- `tickets/context/0127-m176-kw4-kind-world-j-kw-social-j-kw-s-speed/observation-20260527-163702.md` — Cycle 3 で j_reciprocity=0 と早期収束 (ttc=10) を確認

## Test Plan

| ID | カテゴリ | 内容 | 検証方法 |
|----|---------|------|---------|
| FIX-A1 | 正常系 | 死亡なし・出生あり → j_pop_growth > 0.0 | 単体テスト |
| FIX-A2 | 正常系 | 死亡 > 出生 → j_pop_growth == 0.0（人口減少） | 単体テスト |
| FIX-A3 | 正常系 | 死亡 = 出生 → j_pop_growth == 0.0（横ばい） | 単体テスト |
| FIX-A4 | 境界値 | dead.len() > total_nodes → saturating_sub → alive_count = 0 | 単体テスト |
| FIX-A5 | 観測 | 修正前後の ttc 変化を比較 | 観測テスト（--nocapture） |
| FIX-A6 | 回帰 | 既存テスト全 PASS | cargo test |

### FIX-A1: 死亡なし・出生あり

初期人口 N=10, 出生=5, 死亡=0 → `total_nodes=15, alive=15, j_pop_growth=(15/10-1)=0.5`

### FIX-A2: 死亡 > 出生（減少）

初期人口 N=10, 出生=2, 死亡=5 → `total_nodes=12, alive=7, j_pop_growth=clamp(7/10-1,0,1)=0.0`

### FIX-A3: 死亡 = 出生（横ばい）

初期人口 N=10, 出生=3, 死亡=3 → `total_nodes=13, alive=10, j_pop_growth=clamp(10/10-1,0,1)=0.0`

### FIX-A4: 過剰死亡

初期人口 N=10, 死亡=15（超過）→ `saturating_sub` により `alive=0`, パニックしない

### FIX-A5: 観測テスト

`run_evaluation_simulation` を実行し修正前後の `tick_to_convergence` を比較出力:
- `--nocapture` で "FIX-A5: ttc_before={}, ttc_after={}" を出力

### FIX-A6: 回帰

```bash
cargo test
```
全 PASS 確認

## 計装方法・観測対象

### 計装方法

1. 修正前後の収束判定値を `println!` で `--nocapture` 出力
2. 固定シード `StdRng::seed_from_u64(12345)` で決定論的実行

### 観測対象

- 修正前の `j_pop_growth = 0.0` 固定が解消されていること
- `tick_to_convergence` の変化（修正前より大きくなる=正確な収束検出）
- `s_growth` 成分ごとの内訳（j_pop_growth, j_lifecycle, j_child_survival, j_freshness）

### 較正計画

本チケットに較正ループは不要。単一のバグフィックスでありパラメータチューニングではない。

## Boy Scout Rule — 翻訳可能性計画

1. 該当ブロックのローカル変数名がアルゴリズムを説明しているか確認
2. 収束条件 `s_growth * j_cov > KW4_CONVERGENCE_THRESHOLD` にマジックナンバーがないことを確認
3. convergence チェックブロック（`simulation.rs:1529-1564`、約 35 行）が 1 責務に閉じているか確認。必要なら `check_convergence()` 関数として抽出

## Acceptance Criteria

- [x] コード調査完了: Investigation に物理的証拠を記録済み
- [ ] `simulation.rs:1531` の `alive_count` 計算式を修正
- [ ] テスト FIX-A1〜FIX-A4 が PASS
- [ ] 観測テスト FIX-A5 で ttc 変化が確認できる
- [ ] `cargo test` 全 PASS（FIX-A6）
- [ ] 翻訳可能性レビュー通過

## Notes

- **依存関係**: なし（独立して完了可能）
- **後続への影響**: FIX-A 完了後、KW4 較正ループで `s_speed` が正しく算出されるようになる
- **発見したバグの再現**: `config.population_size`（初期値固定）を `ctx.population_count()`（出生ノード含む）に変更するのみ

### 成果物

- 計画: `tickets/context/0128-m176-kw-fix-a-j-pop-growth-alive-count/plan.md`
- 実装サマリ: `tickets/context/0128-m176-kw-fix-a-j-pop-growth-alive-count/implementation.md`
- レビュー報告書: `tickets/context/0128-m176-kw-fix-a-j-pop-growth-alive-count/review.md`
- 観察レポート: `tickets/context/0128-m176-kw-fix-a-j-pop-growth-alive-count/observation-*.md`
