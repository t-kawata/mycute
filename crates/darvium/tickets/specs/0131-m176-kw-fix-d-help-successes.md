---
ticket_id: 131
title: M1.76-KW-FIX-D: help_successes 二重処理バグ修正 — 経験値重複加算の解消
slug: m176-kw-fix-d-help-successes
status: reviewed
created_at: 2026-05-27
updated_at: 2026-05-27
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0131-m176-kw-fix-d-help-successes/plan.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0131-m176-kw-fix-d-help-successes/observation-20260527-185845.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0131-m176-kw-fix-d-help-successes/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0131-m176-kw-fix-d-help-successes/review.md
---
# M1.76-KW-FIX-D: help_successes 二重処理バグ修正 — 経験値重複加算の解消

## Summary

`run_kw_real_simulation` と `run_evaluation_simulation` の双方で、`help_successes` が各 tick で `extend` されるがクリアされないため、`phase5_capability_diffusion` が全過去 success を毎 tick 再処理している。結果、HELP に関与したノードの経験値(experience)が tick ごとに重複加算され、人為的に膨張する。本チケットは当該バグを修正し、**各 HELP 成功につき経験値が正確に 1 回のみ加算される** ことを保証する。

## Background

### 問題の経緯

FIX-C（#130）で HELP プロトコルが修正され、任意ペア間の HELP が発生するようになった。これにより HELP 成功数が増加し、重複加算の影響が従来より顕著になっている。FIX-B（#129）で子供の lifecycle_score = 0 問題を修正したが、経験値の重複加算が lifecycle_score を不当に引き上げている副作用が残っている。

**⚠️ 実行順序制約: 本チケットは FIX-B（#129）完了後に対応すること。** 既に FIX-B は完了している。

## Scope

1. `simulation.rs:1295` の `help_successes.extend(new_successes)` の後、`phase5_capability_diffusion` に `new_successes`（当 tick 分のみ）を渡すよう修正
2. 同じ修正を `simulation.rs:1538`（`run_evaluation_simulation` 内）にも適用
3. 変更後、経験値の加算が各 HELP 成功につき 1 回のみになることを確認

### 修正方針

2 つの選択肢がある:

**Option A — 変数分割**: `help_successes` の累積をやめ、`phase5_capability_diffusion` に `new_successes` のみを渡す。`help_successes` 変数自体を削除する。

**Option B — clear 追加**: `phase5_capability_diffusion` 呼び出し後に `help_successes.clear()` を追加。

Darvium-Tickets-v2.3.md では両方の選択肢が示唆されている。Option A のほうが意図が明確（累積変数そのものをなくす）であり、翻訳可能性の観点からも優れている。

## Non-scope

1. 他のシミュレーション関数（`run_simulation` 等）の help_successes 処理は変更しない
2. GMR 拡散（`try_gmr_diffusion`）の二重処理は対象外
3. RFC 改訂は含まない

## Investigation

### 物理的証拠

#### 証拠 1: `run_kw_real_simulation` の help_successes 二重処理（simulation.rs:1266-1324）

```rust
// line 1266: 関数スコープで宣言、一度だけ初期化
let mut help_successes: Vec<(NodeId, NodeId)> = Vec::new();

for tick in 0..config.max_ticks {
    // ...
    // line 1285: phase3 は new_successes（当 tick 分のみ）を返す
    let (proposals, new_successes) = phase3_help_protocol(...);
    let successes_count = new_successes.len();
    // line 1295: new_successes を help_successes に追加（クリアなし）
    help_successes.extend(new_successes);
    // ...
    // line 1315-1324: help_successes（累積全件）を phase5 に渡す
    let diffusions = if !help_successes.is_empty() {
        phase5_capability_diffusion(
            &mut ctx,
            &help_successes,     // ← 全 tick の累積を毎回処理！
            &mut node_reputations,
            &mut node_experiences,
        )
    };
}
```

- **確認日**: 2026-05-27
- **該当行**: simulation.rs:1266, 1295, 1315-1324
- **状態**: 現行コードで確認。変更なし。

#### 証拠 2: `run_evaluation_simulation` の同一バグ（simulation.rs:1505-1561）

```rust
// line 1505: 同一パターン
let mut help_successes: Vec<(NodeId, NodeId)> = Vec::new();

for tick in 0..config.max_ticks {
    // ...
    // line 1538
    help_successes.extend(new_successes);
    // ...
    // line 1558-1567
    let diffusions = if !help_successes.is_empty() {
        phase5_capability_diffusion(
            &mut ctx,
            &help_successes,     // ← 同一バグ
            ...
        )
    };
}
```

- **確認日**: 2026-05-27
- **該当行**: simulation.rs:1505, 1538, 1558-1567
- **状態**: 現行コードで確認。

#### 証拠 3: phase5_capability_diffusion の経験値加算（simulation.rs:2014-2016）

```rust
if let Some(exp) = node_experiences.get_mut(&helpee_id) {
    *exp = exp.saturating_add(1);  // 毎回 +1
}
```

- **確認日**: 2026-05-27
- **該当行**: simulation.rs:2014-2016
- **影響**: 重複処理されるたびに経験値が加算される

#### 証拠 4: 重複加算の定量的影響

tick=50 のシミュレーションで、tick=0 に HELP 成功したノードは 50 回経験値が加算される（本来 1 回）。tick=49 の成功は 2 回加算される。つまり、早期に HELP されたノードほど経験値が大きく歪む。

## Test Plan

### FIX-D1〜D4 テスト一覧

| ID | テスト名 | ファイル | 種別 | 内容 |
|----|---------|---------|------|------|
| D1 | `test_fixd_single_help_single_exp` | simulation.rs | 不変条件 | 1 回の HELP 成功後に経験値が正確に 1 だけ増加すること |
| D2 | `test_fixd_two_ticks_separate_exp` | simulation.rs | 不変条件 | 2 tick 連続で HELP が発生した場合、各 tick の成功がそれぞれ 1 回ずつ加算されること |
| D3 | `test_fixd_avg_exp_decrease` | simulation.rs | 観測 | 修正前後で平均経験値が減少することを確認（println! 出力） |
| D4 | `test_fixd_existing_tests_pass` | simulation.rs | 不変条件 | 既存テスト全 PASS |

### テスト詳細

- **D1**: 単一ノードペアで 1 回 HELP 成功させる。そのノードの experience が 1 であることを確認。
- **D2**: 2 tick 連続で別々の HELP を発生させる。両ノードの experience がそれぞれ 1 であることを確認。
- **D3**: 修正後の平均経験値を出力。修正前（現在のコード）では経験値が人為的に膨張しているため、修正後の平均経験値が減少することを観測。
- **D4**: `cargo test` 全テストが PASS することを確認。

## 計装方法・観測対象

### 計装方法

- simulation.rs の `test_fixd_*` テストで実装
- 観測テスト（D3）は `println!` + `--nocapture` で構造化出力
- 固定シード `StdRng::seed_from_u64(12345)`

### 観測対象

- D1: 単一 HELP 成功後の experience 値（== 1 を assert）
- D2: 2 tick 連続 HELP 後の各 experience 値（両方 == 1 を assert）
- D3: 修正後の平均経験値（全ノードの experience 平均）
- D4: 既存テスト全 PASS

### 較正計画

本チケットは純粋なバグ修正であり、較正を要する定数は存在しない。

## Boy Scout Rule — 翻訳可能性計画

1. **help_successes 変数の削除**: Option A（変数分割）を採用し、`help_successes` 累積変数そのものを削除する。代わりに `phase5_capability_diffusion` には `new_successes` を直接渡す。
2. **2 箇所の同一修正**: `run_kw_real_simulation` と `run_evaluation_simulation` の両方を同一パターンで修正する（DRY）。

## Acceptance Criteria

- [ ] FIX-D1: 1 回の HELP 成功後に経験値が正確に 1 だけ増加する
- [ ] FIX-D2: 2 tick 連続の HELP で各 tick の成功がそれぞれ 1 回ずつ加算される
- [ ] FIX-D3: 修正後の平均経験値が出力される（観測）
- [ ] FIX-D4: 既存テスト全 PASS

## Notes

### 実行順序制約

- FIX-B（#129）は完了済み。本チケットの前提条件成立。
- FIX-C（#130）は完了済み。HELP プロトコル修正後なので HELP 成功数は増加しているが、本バグの本質（重複加算）は不変。

### 成果物

- 計画: context/0131-m176-kw-fix-d-help-successes/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0131-m176-kw-fix-d-help-successes/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0131-m176-kw-fix-d-help-successes/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0131-m176-kw-fix-d-help-successes/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成）
