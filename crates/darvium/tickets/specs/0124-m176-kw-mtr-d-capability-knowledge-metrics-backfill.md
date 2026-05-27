---
ticket_id: 124
title: M1.76-KW-MTR-D: Capability & Knowledge Metrics Backfill
slug: m176-kw-mtr-d-capability-knowledge-metrics-backfill
status: draft
created_at: 2026-05-27
updated_at: 2026-05-27
---

# M1.76-KW-MTR-D: Capability & Knowledge Metrics Backfill

## Summary

`collect_final_metrics`（SimulationContext 版）において、現在 0.0 でハードコードされている 3 指標（`capability_coverage`, `knowledge_diffusion_rate`, `reuse_ratio`）を、RFC §15.9.3 で定義された計算式に基づいて実際に計算する。これにより s_density 因子が現在の 0.30 から上昇する。各指標は RFC 定義に従い、真のプロキシではない実測値として計算される。

## Background

s_density = (j_cov + j_diffusion + j_reuse + j_nest_depth + j_node_density) / 5 のうち、j_cov, j_diffusion, j_reuse の 3 指標が 0.0 のままである。3 指標は RFC §15.9.3 で定義済みだが、SimulationContext 型（`HashMap<NodeId, SpacePositionEmbedding>`、`Vec<HelpSession>` 等）に対するアダプテーションが未実装である。

RFC §15.9.3（参照）:
- `capability_coverage`: ワークフローの能力空間 (position) を N×N グリッドに量子化し Shannon 多様性指数を計算。
- `knowledge_diffusion_rate`: 村間の知識 (experience) 分散の時間変化率。
- `reuse_ratio`: 同一 workflow が複数回ヘルプ提供または依頼を受けている割合。

KW2/KW3 に同等関数が既に存在する（`compute_capability_coverage_shannon`、`compute_reuse_ratio`、`compute_knowledge_diffusion_rate`）が、これらは旧 `SimWorkflowState` / `SimHelpSession` 型を使用している。本チケットでは SimulationContext の型に適合した RFC 準拠版を新規実装する。

### Investigation

**capability_coverage**（RFC §15.9.3、現在 0.0）:
- RFC 定義: 「ワークフローの能力空間 (position/experience) を 10×10 グリッドに量子化し Shannon 多様性指数を計算」
- SimulationContext は `positions: HashMap<NodeId, SpacePositionEmbedding>` を持つ（各ノードの 3 次元位置、`[f32; 3]`）。
- 既存の KW2 `compute_capability_coverage_shannon` は `SimWorkflowState` の position[0]/position[1] をグリッドに量子化し Shannon 指数を計算。
- SimulationContext 版では `ctx.positions` の XY 座標を使用し、同様の N×N グリッド量子化 + Shannon 指数で計算する。
- 位置自体が能力空間内のニッチを表現しているため（RFC §41B.2）、position-only のグリッドでも RFC 定義を充足する。
- 関数シグネチャ: `compute_capability_coverage(positions: &HashMap<NodeId, SpacePositionEmbedding>) -> f64`

**knowledge_diffusion_rate**（RFC §15.9.3、現在 0.0）:
- RFC 定義: 「村間の知識 (experience) 分散の時間変化率。各村の平均 experience の標準偏差が時間とともに減少する速度」
- 現在のシミュレーションは `node_experiences: HashMap<NodeId, u64>` をローカル変数として保持している。
- 本チケットでは `node_experiences` を SimulationContext に昇格させ、過去の村割り当ても追跡する。
- 既存の KW3 `compute_knowledge_diffusion_rate` のロジックを SimulationContext 型に適合させる。
- 関数シグネチャ: `compute_knowledge_diffusion_rate(node_experiences: &HashMap<NodeId, u64>, current_assignments: &HashMap<NodeId, VillageAssignment>, previous_assignments: &HashMap<NodeId, VillageAssignment>) -> f64`

**reuse_ratio**（RFC §15.9.3、現在 0.0）:
- RFC 定義: 「同一 workflow が複数回ヘルプ提供または依頼を受けている割合 = 再利用回数 / 全インタラクション数」
- SimulationContext は `help_sessions: Vec<HelpSession>` を持ち、各セッションは `from_workflow: NodeId` と `to_workflow: NodeId` を持つ。
- 各 NodeId の全セッションにおける出現頻度をカウントし、2 回以上出現するノードによるインタラクションを「再利用」とみなす。
- 既存の KW2 `compute_reuse_ratio` のロジック（workflow ID 頻度カウント）を SimulationContext 型に適合させる。
- 関数シグネチャ: `compute_reuse_ratio(help_sessions: &[HelpSession]) -> f64`

## Scope

1. **SimulationContext へのフィールド追加**（simulation.rs）:
   - `node_experiences: HashMap<NodeId, u64>` — 各ノードの累積経験値（現在ローカル変数）
   - `previous_village_assignments: HashMap<NodeId, VillageAssignment>` — 前 tick の村割り当て（knowledge_diffusion_rate 計算用）

2. **シミュレーションフェーズでの更新**（simulation.rs）:
   - シミュレーションループ内で `ctx.node_experiences` を既存のローカル `node_experiences` と同期
   - Phase 2（村クラスタリング）後に `ctx.previous_village_assignments` を更新

3. **kind_world.rs に関数追加**（RFC §15.9.3 準拠）:
   - `compute_capability_coverage(positions: &HashMap<NodeId, SpacePositionEmbedding>) -> f64` — 位置のグリッド量子化 + Shannon 多様性指数
   - `compute_knowledge_diffusion_rate(node_experiences: &HashMap<NodeId, u64>, current_assignments: &HashMap<NodeId, VillageAssignment>, previous_assignments: &HashMap<NodeId, VillageAssignment>) -> f64` — 村間 experience 分散の時間変化率
   - `compute_reuse_ratio(help_sessions: &[HelpSession]) -> f64` — 同一 workflow の複数回出現割合

4. **collect_final_metrics の更新**（kind_world.rs:2147）:
   - `capability_coverage: 0.0` → `compute_capability_coverage(&ctx.positions)`
   - `knowledge_diffusion_rate: 0.0` → `compute_knowledge_diffusion_rate(&ctx.node_experiences, &ctx.village_assignments, &ctx.previous_village_assignments)`
   - `reuse_ratio: 0.0` → `compute_reuse_ratio(&ctx.help_sessions)`

## Non-scope

- 各ノードに明示的な能力ベクトル（スキルセット）を持たせる真の CapabilityFamily ベース能力管理（将来の RFC 拡張が必要）
- HELP 以外の経路（GMR 再利用など）を含めた総合再利用トラッキング
- 他の 8 指標（lifecycle, child_survival, freshness, benevolence, reciprocity, trust, execution, cost）は対象外
- 既存 KW2/KW3 の `compute_capability_coverage_shannon` / `compute_reuse_ratio` / `compute_knowledge_diffusion_rate` の変更（旧型は引き続き使用）

## Test Plan

| ID | テスト | アサーション |
|----|--------|-------------|
| D1 | compute_capability_coverage 同一位置 | 全ノード同一位置で Shannon 指数 0.0（多様性なし） |
| D2 | compute_capability_coverage 分散位置 | 全ノード均一分散で Shannon 指数 1.0 に近い |
| D3 | compute_knowledge_diffusion_rate 単一村 | 村が 1 つしかない場合 0.0 |
| D4 | compute_knowledge_diffusion_rate 拡散進行 | 村間 experience 分散減少時に正の拡散率 |
| D5 | compute_reuse_ratio 0 session | 空セッションで 0.0 |
| D6 | compute_reuse_ratio 全同一ノード | 全セッション同一ノードが関与で 1.0 |
| D7 | collect_final_metrics 3 指標非ゼロ | シミュレーション実行後に all > 0.0 |
| D8 | 既存テスト全 PASS | regression |

## 計装方法・観測対象

collect_final_metrics の出力に capability/knowledge 関連の 3 指標を含める。各指標値を JSON 出力し、Shannon 多様性指数・村間 experience 分散・HELP 再利用割合の時間変化を観測。RFC §15.9.3 定義値との整合性を確認。

## Boy Scout Rule — 翻訳可能性計画

- 各関数は RFC §15.9.3 の定義式をそのままコードに落とし込み、コメントには RFC セクション番号を明記
- `compute_capability_coverage` はグリッド量子化 + Shannon 指数の内部関数に分割
- `compute_knowledge_diffusion_rate` は「村 experience 平均」と「標準偏差変化率」を分離
- `compute_reuse_ratio` は「頻度カウント」と「再利用判定」を分離

## Acceptance Criteria

1. capability_coverage が Shannon 多様性指数（RFC §15.9.3）として計算される（0.0 ではなくなる）
2. knowledge_diffusion_rate が村間 experience 分散変化率（RFC §15.9.3）として計算される（0.0 ではなくなる）
3. reuse_ratio が同一 workflow 出現頻度（RFC §15.9.3）として計算される（0.0 ではなくなる）
4. SimulationContext に node_experiences / previous_village_assignments が追加される
5. 各関数に参照する RFC セクション番号が doc comment に記載される
6. 既存テスト全 PASS
7. s_density が 0.30 より大きくなる
