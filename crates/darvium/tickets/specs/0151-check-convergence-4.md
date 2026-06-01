---
ticket_id: 151
title: check_convergence 内の4重全人口スキャンを削減
slug: check-convergence-4
status: reviewed
created_at: 2026-06-01
updated_at: 2026-06-01
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0151-check-convergence-4/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0151-check-convergence-4/review.md
---
# check_convergence 内の4重全人口スキャンを削減

## Summary

`check_convergence` 関数が1回の呼び出しで4回の全人口スキャンを行っている問題を修正する。#150 で `alive_ids` は引数化済みだが、残る3つのスキャン（`gc_states_map` `last_update_ticks_map` `positions_map` → HashMap）が未対応。加えて、Phase 3.7 の首長性スコア計算でも同様の全人口スキャンが残っている。

## Background

#149/#150 の調査・修正過程で発見された残課題。`check_convergence` は `KW4_OBSERVATION_INTERVAL` ごとに呼ばれ、以下の4つの全人口スキャンを行う:

| # | 呼び出し | 行 | コスト |
|---|---|---|---|
| 1 | `alive_ids` | 引数で #150 対応済み | ✅ |
| 2 | `gc_states_map()` | `simulation.rs:501-507` | 全人口 Vec → HashMap |
| 3 | `last_update_ticks_map()` | `simulation.rs:510-516` | 全人口 Vec → HashMap |
| 4 | `positions_map()` | `simulation.rs:466-472` | 全人口 Vec → HashMap |

各 Map 関数は `compute_mean_lifecycle_score` / `compute_mean_freshness` / `compute_capability_coverage`（kind_world.rs）に渡すために全人口から HashMap を新規構築している。しかしこれらの関数は実際には生存者のデータのみを必要とする。

## Scope

1. **`gc_states_map` / `last_update_ticks_map` / `positions_map` の生存者限定版を追加**: 引数 `alive_ids` を受け取り、生存者のみから HashMap を構築する
2. **Phase 3.7 首長性スコア計算**（`simulation.rs:2268-2280`）: `ctx.population.iter_mut().filter(|p| p.alive)` → `alive_ids.iter()` に変更（#150 の流用）

## Non-scope

- 空間グリッドインデックスの導入（別チケット）
- `check_convergence` の収束判定ロジックそのものの変更

## Investigation

### 物理的証拠

**証拠1**: `check_convergence`（`simulation.rs:1993`）:
```rust
fn check_convergence(ctx, config, tick, tick_to_convergence, convergence_reached, alive_ids) {
    ...
    let j_lifecycle = compute_mean_lifecycle_score(&ctx.gc_states_map());      // 全人口スキャン①
    ...
    let j_freshness = compute_mean_freshness(&ctx.last_update_ticks_map(), tick); // 全人口スキャン②
    ...
    let j_cov = compute_capability_coverage(&ctx.positions_map());               // 全人口スキャン③
}
```

**証拠2**: 各 Map 関数の実装（`simulation.rs:466-516`）:
```rust
pub fn positions_map(&self) -> HashMap<PersonId, SpacePositionEmbedding> {
    self.population.iter().enumerate().map(|(i, p)| (i, p.position)).collect()  // 全人口
}
pub fn gc_states_map(&self) -> HashMap<PersonId, GcEvent> {
    self.population.iter().enumerate().map(|(i, p)| (i, p.gc_state)).collect()  // 全人口
}
pub fn last_update_ticks_map(&self) -> HashMap<PersonId, u64> {
    self.population.iter().enumerate().map(|(i, p)| (i, p.last_update_tick)).collect()  // 全人口
}
```

**証拠3**: 下流関数（`kind_world.rs`）は HashMap イテレートするのみで、生存者フィルタは行わない:
- `compute_mean_lifecycle_score`（kind_world.rs:2633）: HashMap の values() を平均
- `compute_mean_freshness`（kind_world.rs:2662）: HashMap の values() を平均
- `compute_capability_coverage`（kind_world.rs:2775）: HashMap の values() でグリッド量子化

**証拠4**: Phase 3.7 首長性スコア計算（`simulation.rs:2268-2280`）:
```rust
for person in ctx.population.iter_mut().filter(|p| p.alive) {
    // alive_ids が利用可能なのに毎回スキャン
}
```
#150 で `alive_ids` が利用可能になったため、`alive_ids.iter()` に変更可能。

### 参照チケット

- tickets/context/0149-node-count/implementation.md — #149 実装サマリ
- tickets/context/0150-untitled-5/implementation.md — #150 実装サマリ（alive_ids 引数化）

## Test Plan

- **T1**: 生存者限定版 `gc_states_map` の結果が元の `gc_states_map` の生存者サブセットと一致する
- **T2**: Phase 3.7 の首長性スコアが従来と同じ値を返す（alive_ids 版 vs filter版）
- **T3**: `check_convergence` の収束判定結果が変更前と一致する

## 計装方法・観測対象

<!--
Darvium は観測ベース検証（Observational Testing First）を基本とする。
このセクションでは計装と観測対象を定義する。

### 計装方法
- どのテストコードで計装を実装するか
- どのような計測プローブを仕掛けるか（println! + --nocapture 等）
- 固定シード PRNG（StdRng::seed_from_u64(12345)）を使用するか

### 観測対象
- 観測する統計量（平均・分散・エントロピー・分布形状等）
- サンプルサイズの要件（分布同定 n >= 10,000、ドリフト検出 n >= 1,000）
- 期待される現象（不変条件として assert すべき性質と、観測として記録すべき傾向）

### 較正計画
- 調整する定数（constants.rs の該当定数）
- 目的関数 J(θ) の設計（収束速度・定常誤差・オーバーシュート等の合成評価）
- 較正ループの停止条件
-->

## Boy Scout Rule — 翻訳可能性計画

- `gc_states_map` / `positions_map` / `last_update_ticks_map` は責務が混在: 「全人口から Map 構築」と「生存者から Map 構築」を関数名で区別する（例: `gc_states_map_for_ids`）
- Phase 3.7 のスコア計算ループは `alive_ids` を使うことで「どの個人に対して計算しているか」がシグネチャから明らかになる

## Acceptance Criteria

- [ ] `check_convergence` 内の Map 構築が生存者のみを対象とする
- [ ] Phase 3.7 首長性スコア計算が `alive_ids` を使用する
- [ ] 収束判定の結果が変更前と同一である

## Notes

<!--
注: このコメントは人間向けの説明である。AI は以下の手順に従うこと。

- plan_path: /plan-ticket が plan.md を作成後に frontmatter に更新する
- implementation_path: /start-ticket が implementation.md を作成後に frontmatter に更新する
- review_report_path: /review-ticket が review.md を作成後に frontmatter に更新する
- observation_report_path: /start-ticket が observation-YYYYMMDD-HHmmss.md を作成後に frontmatter に最新パスを更新する

各コマンドのワークフロー手順が frontmatter 更新の正しい手順である。
-->

### 成果物

- 計画: context/0151-check-convergence-4/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0151-check-convergence-4/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0151-check-convergence-4/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0151-check-convergence-4/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
