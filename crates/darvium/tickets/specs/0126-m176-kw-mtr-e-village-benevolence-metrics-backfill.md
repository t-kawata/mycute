---
ticket_id: 126
title: "M1.76-KW-MTR-E: Village Churn & Benevolence Ratio Backfill"
slug: m176-kw-mtr-e-village-benevolence-metrics-backfill
status: reviewed
created_at: 2026-05-27
updated_at: 2026-05-27
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0126-m176-kw-mtr-e-village-benevolence-metrics-backfill/plan.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0126-m176-kw-mtr-e-village-benevolence-metrics-backfill/observation-20260527-142022.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0126-m176-kw-mtr-e-village-benevolence-metrics-backfill/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0126-m176-kw-mtr-e-village-benevolence-metrics-backfill/review.md
---

# M1.76-KW-MTR-E: Village Churn & Benevolence Ratio Backfill

## Summary

`collect_final_metrics`（SimulationContext 版, kind_world.rs:2386）において、現在ハードコードされている `village_churn_rate: 0.0` と `benevolent_vs_non_benevolent_coverage_ratio: 1.0` の 2 指標を、既存のシミュレーションデータから実測値で置き換える。これにより MTR-A〜D 完了後に残る最後の 2 プレースホルダが解消され、全 23 フィールドが実測値化される。

## Background

MTR-A〜D で 19 指標が実測値化され、残る未実装は次の 2 つ：

| フィールド | 現在値 | 因子影響 |
|-----------|--------|---------|
| `village_churn_rate` | 0.0 (hardcoded) | `s_growth` に間接影響（旧パスでは `village_flow_balance` から算出されていた） |
| `benevolent_vs_non_benevolent_coverage_ratio` | 1.0 (constant) | `s_fairness` のペナルティ項（$j_{penalty}$ = 1 - ratio）。1.0 ではペナルティ 0 |

両指標とも既存の SimulationContext フィールドから算出可能だが、`village_churn_rate` は経時比較のための新規フィールドが必要となる。

### Investigation

**updated at plan time: 2026-05-27**

#### village_churn_rate（現在 0.0, kind_world.rs:2461）

**現行コード**:
```rust
// kind_world.rs:2461
village_churn_rate: 0.0,
```

**RFC §15.9.3 定義**: 村離脱率。住民が村を離れる割合。

**旧パス実装**（kind_world.rs:1432-1469）:
```rust
pub fn compute_village_flow_balance(
    current_assignments: &[Option<usize>],
    previous_assignments: &[Option<usize>],
) -> f64 {
    // 両 assignment を先頭から比較し、村が変わったノードの割合を返す
    // None（未所属）同士または Some→None/None→Some はカウントしない
}
```

旧パスは `VillageInteractionObserver`（kind_world.rs:1495）が tick ごとに `previous_assignments` を保存し、`observe` メソッド内で差分を計算していた。

**新パスで利用可能なデータ**:
- `ctx.village_assignments: HashMap<NodeId, VillageAssignment>`（simulation.rs:269, `VillageAssignment = Option<usize>`）
- Phase2（`phase2_village_clustering`, simulation.rs:1628）で毎 tick 更新される

**問題**: collect_final_metrics はシミュレーション終了時に 1 回だけ呼ばれる。その時点の `village_assignments` のみでは churn（変化率）を計算できない。前 tick との比較が必要。

**解決**: SimulationContext に経時比較用のカウンタを追加する（新規フィールド最小限）。

方案 A（推奨）: 累積カウンタ方式
```rust
pub village_assignment_total_comparisons: u64,  // 比較したノード延べ数
pub village_assignment_changes: u64,             // 村が変わったノード延べ数
```
- `phase2_village_clustering` 内で、子ノード割り当て時に現在の所属と新しい所属を比較し、変わっていたら両カウンタをインクリメント
- 成人未割り当て処理（`or_insert(None)`）では比較不要（初回割り当ては churn とみなさない）
- 関数: `compute_village_churn_rate(changes: u64, comparisons: u64) -> f64`
- 計算式: `if comparisons == 0 { 0.0 } else { (changes as f64 / comparisons as f64).clamp(0.0, 1.0) }`

方案 B: スナップショット方式
```rust
pub previous_village_assignments: Option<HashMap<NodeId, VillageAssignment>>,
```
- 各 tick 終了時に現在の assignments を保存
- collect_final_metrics で最後の 2 tick のみ比較
- **問題**: 最終 tick のみの比較ではシミュレーション全体の churn を代表できない

方案 A を推奨。理由: (1) シミュレーション全体の平均 churn 率を取得できる、(2) カウンタ 2 つで済みメモリ効率が良い、(3) 旧パスの tick-by-tick 比較と同等の情報を得られる。

**phase2_village_clustering（simulation.rs:1628-1682）の該当箇所**:
```rust
// simulation.rs:1667-1669
if let Some(&anchor) = nearest {
    ctx.village_assignments.insert(child_id, Some(anchor));
}
```
ここで子ノードの村割り当てが行われる。この地点で「以前の割り当て（もしあれば）」と比較し、変化があればカウンタを更新する。

```rust
// simulation.rs:1673-1675
for &adult_id in &alive_adults {
    ctx.village_assignments.entry(adult_id).or_insert(None);
}
```
成人の初回 `or_insert(None)` は比較不要（初期化であり変化ではない）。

#### benevolent_vs_non_benevolent_coverage_ratio（現在 1.0, kind_world.rs:2469）

**現行コード**:
```rust
// kind_world.rs:2469
benevolent_vs_non_benevolent_coverage_ratio: 1.0,
```

**RFC §15.9.3 定義**: 慈悲的集団の能力カバー率 / 非慈悲的集団の能力カバー率。$j_{penalty}$ の入力。

**旧パス実装**（kind_world.rs:818-871）:
```rust
pub fn compute_benevolent_vs_non_benevolent_coverage_ratio(
    population: &[SimWorkflowState],
) -> f64 {
    // survived フィルタ → initial_benevolence 降順ソート
    // 上位 20% (BENEVOLENT_TOP_FRACTION) vs 下位 20% (BENEVOLENT_BOTTOM_FRACTION)
    // 各グループの Shannon 多様性指数の比を返す
}
```

**新パスで利用可能なデータ**:
- `ctx.trust_profiles: HashMap<NodeId, TrustProfile>`（simulation.rs:267）
- `ctx.positions: HashMap<NodeId, SpacePositionEmbedding>`（simulation.rs:271）

**TrustProfile**（types.rs:4807）:
```rust
pub struct TrustProfile {
    pub operational: f64,   // [0.0, 1.0]
    pub semantic: f64,      // [0.0, 1.0]
    pub temporal: f64,      // [0.0, 1.0]
    pub human: HumanTrustLogistic,
}
```

**プロキシ方案**: TrustProfile の 3 成分平均 `(operational + semantic + temporal) / 3.0` を「慈悲スコア」のプロキシとして使用。この値は既に `compute_mean_benevolence`（kind_world.rs:2236）で平均値計算に使われている。各ノードの慈悲スコアで降順ソートし、上位 20% と下位 20% の能力カバー率（Shannon 多様性）の比を計算する。

**能力カバー率の計算**: 各グループ内のノードの `positions` から、`compute_capability_coverage`（MTR-D）と同一の Shannon 多様性指数を計算する。

関数シグネチャ案:
```rust
fn compute_benevolent_vs_non_benevolent_coverage_from_trust(
    trust_profiles: &HashMap<NodeId, TrustProfile>,
    positions: &HashMap<NodeId, SpacePositionEmbedding>,
) -> f64
```
- trust_profiles と positions の両方に存在するノードのみを対象とする
- 慈悲スコア = (operational + semantic + temporal) / 3.0 で降順ソート
- 上位 20%（最低 1 ノード）を benevolent、下位 20%（最低 1 ノード）を non-benevolent と定義
- 上位と下位が重なる（population 過小）場合は 1.0 を返す
- 非慈悲的集団の多様性が 0 の場合: 慈悲的集団の多様性が 0 なら 1.0、正なら 2.0（慈悲的優位）
- 通常時: `top_h / bottom_h`（上限なし）
- 空マップの場合は 1.0 を返す

**新規 SimulationContext フィールドが不要な理由**: trust_profiles は MTR-B の時点で既に利用可能。positions は MTR-D で利用済み。両方とも既存フィールドで十分。

### 参照観察レポート

- tickets/context/0125-m176-kw-mtr-d-capability-knowledge-metrics-backfill-2/observation-20260527-135743.md — MTR-D 完了。capability_coverage=0.802, reuse_ratio=0.056, knowledge_diffusion_rate=0.947。s_density が 0.5 に改善。残るプレースホルダは village_churn_rate と benevolent_vs_non_benevolent_coverage_ratio の 2 つ。

### SimulationContext の既存フィールド（simulation.rs:263-297）

```rust
pub memoized_graph: &'a mut MemoizedGraph,
pub trust_profiles: HashMap<NodeId, TrustProfile>,
pub village_assignments: HashMap<NodeId, VillageAssignment>,
pub positions: HashMap<NodeId, SpacePositionEmbedding>,
pub tick: u64,
pub rng: StdRng,
pub help_sessions: Vec<HelpSession>,
pub use_gmr: bool,
pub node_gc_states: HashMap<NodeId, GcEvent>,              // MTR-A
pub node_last_update_tick: HashMap<NodeId, u64>,             // MTR-A
pub total_births: u64,                                       // MTR-A
pub total_inheritance_fidelity: f64,                         // MTR-B
pub inheritance_event_count: u64,                            // MTR-B
pub reciprocity_pair_counts: HashMap<(NodeId, NodeId), u64>, // MTR-B
pub total_gc_collections: u64,                               // MTR-C
pub total_help_attempts: u64,                                // MTR-C
pub total_help_successes: u64,                               // MTR-C
```

### RFC §15.9.3 定義 vs 実装（updated at plan time: 2026-05-27）

RFC §15.9.3 は以下を定義する:
- `village_churn_rate`: 村離脱率 [0, 1]
- `benevolent_vs_non_benevolent_coverage_ratio`: 慈悲的/非慈悲的集団の能力カバー率比

新パスでは:
- `village_churn_rate`: 累積カウンタ方式で tick 間の村割り当て変更率を計算 → RFC 定義と一致。旧パスの `compute_village_flow_balance` と同一ロジック
- `benevolent_vs_non_benevolent_coverage_ratio`: TrustProfile の合成スコアを慈悲スコアのプロキシとして使用し、旧パスと同一の上位 20% / 下位 20% 分割 + Shannon 多様性比のロジックを適用 → RFC §15.9.3 定義と実質一致（initial_benevolence の代わりに TrustProfile 合成スコアを使用する点がプロキシ）

## Scope

1. **SimulationContext へのフィールド追加**（simulation.rs）:
   - `village_assignment_total_comparisons: u64` — 比較したノード延べ数
   - `village_assignment_changes: u64` — 村所属が変わったノード延べ数
   - 初期化を `SimulationContext::new` に追加（全 0）

2. **シミュレーションループでのカウンター更新**（simulation.rs: phase2_village_clustering）:
   - 子ノード割り当て時（`ctx.village_assignments.insert(child_id, Some(anchor))` の直前）に、既存の割り当てと比較
   - 変化があれば両カウンタをインクリメント、変化がなければ comparisons のみインクリメント

3. **kind_world.rs に関数追加**:
   - `compute_village_churn_rate(changes: u64, comparisons: u64) -> f64` — 村 churn 率
   - `compute_benevolent_vs_non_benevolent_coverage_from_trust(...)` — 慈悲/非慈悲カバー率比

4. **collect_final_metrics の更新**（kind_world.rs:2461, 2469）:
   - `village_churn_rate: 0.0` → `compute_village_churn_rate(ctx.village_assignment_changes, ctx.village_assignment_total_comparisons)`
   - `benevolent_vs_non_benevolent_coverage_ratio: 1.0` → `compute_benevolent_vs_non_benevolent_coverage_from_trust(&ctx.trust_profiles, &ctx.positions)`

5. **テスト追加**:
   - E1: compute_village_churn_rate 空（comparisons=0 で 0.0）
   - E2: compute_village_churn_rate 全変化（changes=comparisons で 1.0）
   - E3: compute_village_churn_rate 部分変化（3/10 で 0.3）
   - E4: compute_benevolent_ratio 空 trust_profiles で 1.0
   - E5: compute_benevolent_ratio 全同一慈悲スコアで 1.0（上位下位の多様性が等しい）
   - E6: collect_final_metrics 経由での non-default 確認（2 指標改善）
   - E7: 既存テスト全 PASS（回帰）

## Non-scope

- 他の 20 指標は対象外（MTR-A〜D で完了済み）
- VillageInteractionObserver の変更（旧パスとの互換性維持）
- `compute_village_flow_balance` 旧パス関数の削除（将来のクリーンアップ対象）
- 村形成ロジック（`phase2_village_clustering`）の変更 — カウンタ更新のみ追加
- BENEVOLENT_TOP_FRACTION / BENEVOLENT_BOTTOM_FRACTION 定数の変更

## Test Plan

| ID | テスト | アサーション |
|----|--------|-------------|
| E1 | compute_village_churn_rate 空 | comparisons=0 で 0.0 |
| E2 | compute_village_churn_rate 全変化 | changes=comparisons で 1.0 |
| E3 | compute_village_churn_rate 部分変化 | 3/10 = 0.3 |
| E4 | compute_benevolent_ratio 空 | trust_profiles 空で 1.0 |
| E5 | compute_benevolent_ratio 全同一慈悲 | 全ノード同一慈悲スコア + 同一位置で 0.0（または低値） |
| E6 | collect_final_metrics 2 指標改善 | village_churn_rate が 0.0 以外、benevolent_ratio が 1.0 以外 |
| E7 | 既存テスト全 PASS | regression |

## 計装方法・観測対象

collect_final_metrics の出力に village dynamics 関連の 2 指標（village_churn_rate, benevolent_vs_non_benevolent_coverage_ratio）を含め、シミュレーション実行後の値を観測。village_churn_rate の分布（累積カウンタ方式の妥当性）と benevolent_ratio の TrustProfile プロキシ値の意味論的妥当性を観測テストで記録。

## Boy Scout Rule — 翻訳可能性計画

- `compute_village_churn_rate` は旧パス `compute_village_flow_balance` とは別名・別引数（カウンタ方式）として実装（混同防止）
- `compute_benevolent_vs_non_benevolent_coverage_from_trust` も旧パスとは別名（`_from_trust` サフィックス）
- プロキシ値の限界（TrustProfile 合成スコアを慈悲スコアの近似として使用）をドキュメントコメントに明記
- phase2_village_clustering へのカウンタ更新コードは、元のロジックを変更せず追加のみ行う（外科的修正）

## Acceptance Criteria

1. village_churn_rate が村割り当て変化率から正しく計算される（0.0 ではなくなる）
2. benevolent_vs_non_benevolent_coverage_ratio が TrustProfile 合成スコアから正しく計算される（1.0 ではなくなる）
3. SimulationContext に 2 フィールド（village_assignment_total_comparisons, village_assignment_changes）が追加され、初期化される
4. 既存テスト全 PASS
5. 全 23 フィールドが実測値化される（MTR 系列完了）

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

- 計画: context/0126-m176-kw-mtr-e-village-benevolence-metrics-backfill/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0126-m176-kw-mtr-e-village-benevolence-metrics-backfill/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0126-m176-kw-mtr-e-village-benevolence-metrics-backfill/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0126-m176-kw-mtr-e-village-benevolence-metrics-backfill/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
