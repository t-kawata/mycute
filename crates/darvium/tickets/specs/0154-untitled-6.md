---
ticket_id: 154
title: 評判再計算の間隔設定可能化
slug: reputation-recompute-interval
status: done
created_at: 2026-06-01
updated_at: 2026-06-01
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0154-untitled-6/plan.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0154-untitled-6/implementation.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0154-untitled-6/observation-20260601-150339.md
---

# 評判再計算の間隔設定可能化

## Summary

`recompute_reputation_for_population` が毎 tick O(n × m)（生存者数 × 全 HELP セッション数）の二重ループで評判値を再計算し、シミュレーション全時間の 15.2% を占めている。k-means（#152）と同様のパターンで再計算間隔を設定可能にし、間引き tick では評判値を据え置く。

## Background

`simulation.rs:1046` の `recompute_reputation_for_population` は以下を行う：
1. 生存者 `n` 人それぞれに対して `update_individual_reputation` を実行
2. その中で `build_events_for_workflow` が全 HELP セッション `m` をスキャン
3. 直接互恵性・間接互恵性・慈悲スコア・評判スコアを再計算

評判値は HELP イベントの蓄積に基づいて徐々に変化するものであり、毎 tick フル再計算する必要はない。GC の hazard 計算は tick 単位の精度を要求しない。

プロファイル結果（#152 観測より）：
```
Phase 3.5 (評判再計算): 1,363,804 µs / 8,975,919 µs = 15.2%
```

## Scope

1. **`ReciprocitySimulatorConfig.reputation_recompute_interval` 追加**: 評判再計算間隔（tick数）、デフォルト 1
2. **`run_kw_real_simulation` ループ分岐**: 間引き tick では再計算をスキップ
3. **フロントエンド連携**: スライダー + server.rs config parsing

## Non-scope

- 評判計算アルゴリズム自体の変更（F-4 / F-5 の数式変更は別チケット）
- インクリメンタル更新方式の導入（本チケットは単純間引き）
- `recompute_trust_reputation`（旧式）の改造（`run_simulation` 用）

## Investigation

### 物理的証拠

**証拠1**: `recompute_reputation_for_population`（`simulation.rs:1046`）の O(n × m) 構造
- 外側ループ: 全生存者 `n`（人口増加に比例して増加）
- 内側ループ: `update_individual_reputation` → `build_events_for_workflow`（`simulation.rs:972`）が全セッション `m` をスキャン
- セッション数 `m` は tick 経過とともに累積的に増加
- → `n` と `m` が両方増加するため二次関数的

**証拠2**: `update_individual_reputation`（`simulation.rs:963`）の処理
- `compute_direct_reciprocity(&events, tick, policy)` — イベントから直接互恵性
- `compute_indirect_reciprocity(...)` — 村中心性・参加率から間接互恵性
- `compute_benevolence_score(...)` — 慈悲スコア再計算
- `recompute_reputation(...)` — 評判スコア再計算（F-4 + F-5）

**証拠3**: 評判値は数 tick で大きく変化しない
- HELP イベントの発生は確率的であり、1 tick で全人口の評判値が劇的に変化することはない
- GC の hazard 計算（Phase 4）にも入力されるが、tick 単位の精度は不要
- k-means（#152）と同様の間引きが有効

**証拠4**: プロファイルで Phase 3.5 が 15.2% を占める（#152 観測より）
- 人口 1430 時点で Phase 3.5 累積時間 1.36秒
- Phase 2（k-means）の約 14 倍

### 参照観察レポート

- `tickets/context/0152-k-means/observation-20260601-142517.md` — Phase 1 が 81.4%、Phase 3.5 が 15.2% であることのプロファイル証拠を含む

### 参照チケット
- #152: k-means 実行間隔の設定可能化（本チケットのパターン元）
- #153: generate_workflow_for_child の検索スキップ最適化（同時発行の姉妹チケット）

## Test Plan

### T1: `reputation_recompute_interval=1` で後方互換性
- デフォルト（`1`）でシミュレーションを実行
- `interval=1` なし（従来コード）と同一の評判値履歴になることを検証
- **正常系**: 同一 seed → 同一の評判値分布

### T2: `reputation_recompute_interval=N` で N tick ごとに再計算
- `reputation_recompute_interval=5` でシミュレーションを実行
- **正常系**: 間引き tick では評判値が前回の値から変化しない（据え置き）
- **正常系**: 再計算 tick では評判値が正しく更新される

### T3: GC 動作への影響確認（観測テスト）
- `reputation_recompute_interval` を大きくしても GC が破綻しないことを確認
- **観測対象**: GC survival rate、人口推移
- **期待**: 間引きによる GC の誤動作が発生しない

### T4: 実行時間の削減確認（観測テスト）
- `reputation_recompute_interval=1` vs `=10` で同一設定のシミュレーションを実行
- Phase 3.5 の累積時間を比較
- **期待**: `interval=10` で Phase 3.5 の時間が約 1/10 に削減

## 計装方法・観測対象

### 計装方法
- `println!` + `--nocapture` で Phase 3.5 累積時間を出力
- 固定シード `StdRng::seed_from_u64(12345)` で決定論的実行
- `SimulationContext` に `phase35_cumulative_time: Duration` を追加して計測

### 観測対象
- **統計量**: `reputation_recompute_interval=1` と `=N` の Phase 3.5 実行時間比
- **サンプルサイズ**: 同一設定で 3 回実行
- **期待される現象**: `reputation_recompute_interval=10` で Phase 3.5 が約 1/10 に短縮

### 較正計画
- **調整する定数**: `reputation_recompute_interval`（Config フィールド、k-means と同様）
- **目的関数 J(θ)**: `J = -time + w * accuracy_penalty`
  - `time`: Phase 3.5 累積時間（対数スケール）
  - `accuracy_penalty`: 間引きによる評判値誤差（最終 tick の値と毎 tick 計算した場合の差分）
  - `w`: 重み係数（デフォルト 0.1）

## Implementation Plan

### Step 1: `ReciprocitySimulatorConfig.reputation_recompute_interval` 追加
- 型: `u64`、デフォルト: `1`（毎 tick = 従来動作）
- `Default` impl に `reputation_recompute_interval: 1` を追加

### Step 2: `run_kw_real_simulation` で条件分岐
```rust
// Phase 3.5: 評判再計算（reputation_recompute_interval に基づき分岐）
if tick % config.reputation_recompute_interval.max(1) == 0 || tick == 0 {
    recompute_reputation_for_population(
        &mut ctx.population,
        &kw_sessions,
        tick,
        &config.policy,
    );
}
// 間引き tick では全く再計算しない（評判値は前回の値が維持される）
```

### Step 3: サーバー設定パース追加 (`server.rs`)
```rust
if let Some(val) = obj.get("reputation_recompute_interval").and_then(|v| v.as_u64()) {
    cfg.reputation_recompute_interval = val.max(1);
}
```

### Step 4: フロントエンド UI 追加
- index.html: スライダー `id="reputationRecomputeInterval"`, min=1, max=50, value=1
- script.js: start コマンドに値を含める

## Boy Scout Rule — 翻訳可能性計画

- `recompute_reputation_for_population` → `update_individual_reputation` の呼び出し階層を明確にし、関数名だけで意図が伝わることを確認する
- `build_events_for_workflow` 内に `sessions.iter().filter(|s| ...)` パターンが複数出現している。責務が混在していれば分割を検討

## Acceptance Criteria

- [ ] 評判再計算間隔をフロントエンドのスライダーで設定できる
- [ ] 間引き tick では評判値が据え置かれる
- [ ] GC が正常動作する（間引きによる破綻なし）
- [ ] 既存テストがすべて通過する
- [ ] Phase 3.5 の実行時間が間引き率に比例して削減される

## Notes

### 成果物

- 計画: context/0154-untitled-6/plan.md
- 実装サマリ: context/0154-untitled-6/implementation.md
- レビュー報告書: context/0154-untitled-6/review.md
- 観察レポート: context/0154-untitled-6/observation-YYYYMMDD-HHmmss.md
