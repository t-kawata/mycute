---
ticket_id: 125
title: M1.76-KW-MTR-D: Capability & Knowledge Metrics Backfill
slug: m176-kw-mtr-d-capability-knowledge-metrics-backfill-2
status: reviewed
created_at: 2026-05-27
updated_at: 2026-05-27
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0125-m176-kw-mtr-d-capability-knowledge-metrics-backfill-2/plan.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0125-m176-kw-mtr-d-capability-knowledge-metrics-backfill-2/observation-20260527-135743.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0125-m176-kw-mtr-d-capability-knowledge-metrics-backfill-2/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0125-m176-kw-mtr-d-capability-knowledge-metrics-backfill-2/review.md
---
# M1.76-KW-MTR-D: Capability & Knowledge Metrics Backfill

## Summary

`collect_final_metrics`（SimulationContext 版, kind_world.rs:2295）において、現在 0.0 でハードコードされている `capability_coverage`, `knowledge_diffusion_rate`, `reuse_ratio` の 3 指標を、既存のシミュレーションデータ（positions, reciprocity_pair_counts）からプロキシ値で置き換える。これにより s_density 因子が上昇する。

## Background

s_density = (j_cov + j_diffusion + j_reuse + j_nest_depth + j_node_density) / 5 のうち、j_cov, j_diffusion, j_reuse の 3 指標が 0.0（s_density を 60% 押し下げ）。これらの指標は能力分布・知識伝播・再利用という概念を扱うが、本チケットでは既存データから妥当なプロキシ値を算出する。**新規の SimulationContext フィールドは不要** — MTR-B で追加済みの `reciprocity_pair_counts` と既存の `positions` から 3 指標すべてを導出できる。

## Scope

1. **kind_world.rs に関数追加**:
   - `compute_capability_coverage(positions: &HashMap<NodeId, SpacePositionEmbedding>) -> f64` — 位置の Shannon 多様性
   - `compute_reuse_ratio(pair_counts: &HashMap<(NodeId, NodeId), u64>) -> f64` — ペア再利用比率
   - `compute_knowledge_diffusion_rate(pair_counts: &HashMap<(NodeId, NodeId), u64>) -> f64` — 相互作用多様性

2. **collect_final_metrics の更新**（kind_world.rs:2364, 2366, 2374）:
   - `capability_coverage: 0.0` → `compute_capability_coverage(&ctx.positions)`
   - `reuse_ratio: 0.0` → `compute_reuse_ratio(&ctx.reciprocity_pair_counts)`
   - `knowledge_diffusion_rate: 0.0` → `compute_knowledge_diffusion_rate(&ctx.reciprocity_pair_counts)`

3. **テスト追加**:
   - D1–D6: ユニットテスト（境界値 + 典型値）
   - D7: collect_final_metrics 経由での non-default 確認（観測テスト）
   - D8: 既存テスト全 PASS（回帰）

## Non-scope

- 他の 9 指標（lifecycle, child_survival, freshness, benevolence, reciprocity, trust, execution, cost, help_success_rate）は対象外
- SimulationContext への新規フィールド追加（既存の fields で十分）
- 旧パス `compute_capability_coverage_shannon` / `compute_reuse_ratio` / `compute_knowledge_diffusion_rate` の削除
- positions / reciprocity_pair_counts の収集ロジックの変更
- VillageInteractionObserver の変更

## Investigation

### capability_coverage（現在 0.0, kind_world.rs:2364）

**現行コード**:
```rust
// kind_world.rs:2363-2364
population_growth_rate: population_growth_rate.clamp(0.0, 1.0),
capability_coverage: 0.0,  // ハードコード
```

**旧パス実装**（kind_world.rs:623-676）:
```rust
pub fn compute_capability_coverage_shannon(
    population: &[crate::simulation::SimWorkflowState],
) -> f64 {
    let survived: Vec<&SimWorkflowState> = population.iter().filter(|w| w.survived).collect();
    // N×N グリッドに量子化 → Shannon 多様性指数 H = -Σ p_i log p_i
    // H_max = log(N²) で正規化
}
```

旧パスは `SimWorkflowState.position[0], position[1]`（f32 配列）と `survived` フラグを使用。新パスでは `ctx.positions: HashMap<NodeId, SpacePositionEmbedding>` を使用する。

**SpacePositionEmbedding**（spaceposition.rs:23）:
```rust
pub struct SpacePositionEmbedding(Option<[f32; 3]>);
// inner() → &Option<[f32; 3]>
// 所有ノードは alive ⇒ positions の全エントリが survived 相当
```

**ECOSYSTEM_GRID_DIVISIONS = 10**（constants.rs:1073）

関数シグネチャ案:
```rust
fn compute_capability_coverage(
    positions: &std::collections::HashMap<crate::types::NodeId, crate::spaceposition::SpacePositionEmbedding>,
) -> f64
```
- positions に含まれる全エントリが alive ノード
- 各エントリの `inner()` から `Some([x, y, _])` を取得、x/y を [0,0.999] に clamp してグリッド量子化
- Shannon 多様性指数計算（旧パスと同一ロジック）

### reuse_ratio（現在 0.0, kind_world.rs:2366）

**現行コード**:
```rust
// kind_world.rs:2365-2366
capability_coverage: 0.0,
reuse_ratio: 0.0,
```

**旧パス実装**（kind_world.rs:702-764）:
```rust
pub fn compute_reuse_ratio(
    events: &[ReciprocityEvent],
    sessions: &[SimHelpSession],
) -> f64 {
    // 全インタラクションの ID 頻度をカウント
    // 2 回以上出現する workflow を「再利用」と判定
}
```

旧パスは `ReciprocityEvent.source_graph_id / target_graph_id` と `SimHelpSession.helper_id / requester_id` の出現頻度を使用。

**新パスで利用可能なデータ**: `ctx.reciprocity_pair_counts: HashMap<(NodeId, NodeId), u64>`（MTR-B, simulation.rs:291）。各 HELP セッション作成時に `(helper_id, child_id)` ペアの頻度がインクリメントされている（simulation.rs:1724）:
```rust
*ctx.reciprocity_pair_counts.entry((helper_id, child_id)).or_insert(0) += 1;
```

関数シグネチャ案:
```rust
fn compute_reuse_ratio(pair_counts: &std::collections::HashMap<(crate::types::NodeId, crate::types::NodeId), u64>) -> f64
```
- total_pairs = pair_counts.len()
- 頻度 >= 2 のペア数 / total_pairs（空なら 0.0）
- これにより「再利用されているペア」の割合を算出

### knowledge_diffusion_rate（現在 0.0, kind_world.rs:2374）

**現行コード**:
```rust
// kind_world.rs:2373-2374
cross_village_interaction_rate,
knowledge_diffusion_rate: 0.0,
```

**旧パス実装**（kind_world.rs:1339-1409）:
```rust
pub fn compute_knowledge_diffusion_rate(
    population: &[SimWorkflowState],
    current_assignments: &[Option<usize>],
    previous_assignments: &[Option<usize>],
) -> f64 {
    // 各村の経験値(experience) 平均の標本標準偏差の減少速度
    // (σ_prev - σ_cur) / σ_prev, [0,1] に clamp
}
```

旧パスは `SimWorkflowState.experience` と village assignments の 2 時点比較が必要。新パスではこのデータ構造が利用できない（experience フィールドがない）。

**新パスでのプロキシ方案**: `reciprocity_pair_counts` の多様性を diffusion のプロキシとする:
- ユニークペア数 / 全インタラクション数 = pair_counts.len() / pair_counts.values().sum()
- 多様なペアが多いほど知識が広く拡散 → 高値
- 同じペアばかり → 低値
- 空 → 0.0

```rust
fn compute_knowledge_diffusion_rate(
    pair_counts: &std::collections::HashMap<(crate::types::NodeId, crate::types::NodeId), u64>,
) -> f64
```
- total_unique = pair_counts.len()
- total_interactions = pair_counts.values().sum::<u64>()
- if total_interactions == 0 { 0.0 } else { total_unique as f64 / total_interactions as f64 }

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

### 主要シミュレーションパス
`run_evaluation_simulation`（simulation.rs:1394）→ `collect_final_metrics(&ctx, config.population_size, child_count)`（simulation.rs:1531）。MTR-A/B/C と同一パス。

**物理的証拠の確認: reciprocity_pair_counts はシミュレーションループ内で適切に充填されている**（simulation.rs:1723-1724）:
```rust
ctx.help_sessions.push(session);
*ctx.reciprocity_pair_counts.entry((helper_id, child_id)).or_insert(0) += 1;
```

### RFC §15.9.3 定義 vs 実装（updated at plan time: 2026-05-27）

RFC §15.9.3 は以下を定義する:
- `capability_coverage`: 能力空間を 10×10 グリッドに量子化し Shannon 多様性指数を計算
- `reuse_ratio`: 同一 workflow が複数回ヘルプ提供または依頼を受けている割合 = 再利用回数 / 全インタラクション数
- `knowledge_diffusion_rate`: 村間の知識 (experience) 分散の時間変化率

新パスでは:
- `capability_coverage`: positions の位置分布から Shannon 多様性指数を計算 → RFC §15.9.3 定義と実質一致（能力空間の代わりに物理位置を使用。現状のシミュレーションでは positions が唯一の位置情報であるため、能力空間のプロキシとして妥当）
- `reuse_ratio`: reciprocity_pair_counts から頻出ペアの割合を計算 → RFC §15.9.3 定義と一致（全インタラクションの代わりに全ペアを使用。1 ペア 1 インタラクションが基本のため同等）
- `knowledge_diffusion_rate`: ペア多様性（ユニークペア数 / 全インタラクション数）をプロキシとして使用 → RFC §15.9.3 とは異なる定義だが合理的なプロキシ。experience データがない現状での最善近似

### 参照観察レポート

- tickets/context/0123-m176-kw-mtr-c-execution-cost-metrics-backfill/observation-20260527-134254.md — cost_efficiency の低値 (0.115) が GC interval に依存。MTR-D 完了後、全 20 指標が揃う
- tickets/context/0121-m176-kw-mtr-a-lifecycle-freshness-metrics-backfill/observation-20260527-120529.md — MTR-A 完了。lifecycle/freshness の実測値化
- tickets/context/0122-m176-kw-mtr-b-trust-reciprocity-metrics-backfill/observation-20260527-132953.md — MTR-B 完了。reciprocity_pair_counts が利用可能になった

### 新規 SimulationContext フィールドが不要な理由

Darvium-Tickets-v2.3.md（line 2162）は `knowledge_interaction_unique_pairs: HashSet<(NodeId, NodeId)>` の追加を挙げていたが、MTR-B で追加済みの `reciprocity_pair_counts: HashMap<(NodeId, NodeId), u64>` が既にペア頻度を保持している。HashMap のキーがユニークペア集合を兼ねるため、HashSet の追加は不要。同マップから reuse（頻度 >= 2 の比率）と diffusion（ユニークペア / 全インタラクション）の両方が計算可能。

## Test Plan

| ID | テスト | アサーション |
|----|--------|-------------|
| D1 | compute_capability_coverage 空 | 空 positions で 0.0 |
| D2 | compute_capability_coverage 全同一位置 | 全ノード同一グリッドセルで低値 |
| D3 | compute_reuse_ratio 空 | pair_counts 空で 0.0 |
| D4 | compute_reuse_ratio 全再利用 | 全ペア頻度 >= 2 で 1.0 |
| D5 | compute_knowledge_diffusion_rate 空 | pair_counts 空で 0.0 |
| D6 | compute_knowledge_diffusion_rate 全異種ペア | 全ペアが異なるユニークペアで 1.0 |
| D7 | collect_final_metrics 3 指標非ゼロ | シミュレーション実行後に all > 0.0 |
| D8 | 既存テスト全 PASS | regression |

## 計装方法・観測対象

collect_final_metrics の出力に capability/knowledge 関連の 3 指標（capability_coverage, reuse_ratio, knowledge_diffusion_rate）を含め、位置分散度・ペア多様性の観測値を確認。各プロキシ値の意味論的妥当性（distribution が直感と合致するか）を観測テストで記録。

## Boy Scout Rule — 翻訳可能性計画

- `compute_capability_coverage` / `compute_reuse_ratio` / `compute_knowledge_diffusion_rate` は旧パス関数とは別名・別引数として実装（混同防止）
- 旧パスと新パスの関数名を明確に区別（SimulationContext 版は名前の重複を避ける）
- プロキシ値の限界をドキュメントコメントに明記

## Acceptance Criteria

1. capability_coverage が positions の位置分布から正しく計算される（0.0 ではなくなる）
2. reuse_ratio が reciprocity_pair_counts から正しく計算される（0.0 ではなくなる）
3. knowledge_diffusion_rate が reciprocity_pair_counts から正しく計算される（0.0 ではなくなる）
4. SimulationContext への新規フィールド追加なし（既存フィールドのみで実現）
5. 既存テスト全 PASS
6. s_density が 0.30 より大きくなる（j_cov, j_diffusion, j_reuse が 0.0 でなくなる）

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

- 計画: context/0125-m176-kw-mtr-d-capability-knowledge-metrics-backfill-2/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0125-m176-kw-mtr-d-capability-knowledge-metrics-backfill-2/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0125-m176-kw-mtr-d-capability-knowledge-metrics-backfill-2/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0125-m176-kw-mtr-d-capability-knowledge-metrics-backfill-2/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
