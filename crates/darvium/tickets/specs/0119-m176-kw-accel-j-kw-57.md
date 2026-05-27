---
ticket_id: 119
title: M1.76-KW-ACCEL: J_kw 社会加速度定義完全一致 — 5因子再定義＋7指標追加
slug: m176-kw-accel-j-kw-57
status: reviewed
created_at: 2026-05-27
updated_at: 2026-05-27
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0119-m176-kw-accel-j-kw-57/plan.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0119-m176-kw-accel-j-kw-57/observation-20260527-091630.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0119-m176-kw-accel-j-kw-57/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0119-m176-kw-accel-j-kw-57/review.md
---
# M1.76-KW-ACCEL: J_kw 社会加速度定義完全一致 — 5因子再定義＋7指標追加

## Summary

RFC §15.9.2 の5因子乗算結合モデルを社会加速度の定義に基づき再定式化する。因子名を定義の4側面に合わせて改名し、未計装だった埋め込み空間トポロジー指標と探索効率指標を新規追加する。既存14指標は全て維持し、因子間再配置＋7指標追加により、J_kwの式が「より少ないVirtualClock進行の間に人口がより増え、個々のワークフローの密度がより多層的に高くなり、Darvium空間の中で構造的クラスター係数および局所密度がより増大し、新規タスクの実行に必要な探索半径と最上階推論ステップ数がより減少している状態」を矛盾なく数学的に語る式となる。

## Background

### チケット系列の位置づけ

KW-REAL P6（チケット118）で5因子乗積モデルを導入したが、以下の乖離が残っていた：

1. **因子名の不一致**: S_viability / S_capability / S_cooperation / S_efficiency は「状態の高さ」を想起させる名前であり、「変化率・密度・構造・効率」を要求する社会加速度定義と一致していない
2. **指標構成の不足**: 定義③（空間クラスター係数・局所密度）に対応する指標がS_cooperationに存在せず、社会行動指標（benevolence/reciprocity）が代用されている。定義④（探索半径・推論ステップ数）に対応する指標がS_efficiencyに存在しない
3. **j_popの意味**: j_popは絶対人口ではなくpopulation_growth_rateを指していたが、名前が定義①「人口が増える」（変化率）と一致していない

本チケットはこれらの乖離を解消し、J_kwの式を定義に対して完全に整合させる。較正ループ（KW4）の直前でJ_kwの式を確定させる位置づけである。

## Scope

### 1. 因子名変更（4因子）

| 旧名 | 新名 | 定義対応 |
|------|------|---------|
| s_viability | s_growth | 定義①「人口増加速度」 |
| s_capability | s_density | 定義②「ワークフロー多層密度」 |
| s_cooperation | s_topology | 定義③「空間クラスター係数・局所密度」 |
| s_efficiency | s_search | 定義④「探索半径・推論ステップ減少」 |
| s_fairness | 変更なし | 正則化項 |

### 2. KindWorldMetricsInput に7新規フィールド追加

| フィールド | 型 | 範囲 | 意味 | 計算方法 |
|-----------|-----|------|------|---------|
| `mean_nest_depth` | f64 | [0,∞) | 平均サブWFネスト深度 | memoized_graph の各WorkflowGraphを再帰的トラバース、最大深度の平均 |
| `mean_node_density` | f64 | [0,∞) | 1ルートWFあたり平均ノード数 | 全WorkflowGraphノード数合計 ÷ グラフ数 |
| `cluster_coefficient` | f64 | [0,1] | 平均Watts-Strogatzクラスター係数 | 3次元positionsからk近傍グラフ構築（k=5）、各ノードのクラスター係数平均 |
| `local_density` | f64 | [0,1] | 埋め込み空間局所密度 | 閾値半径内の平均近傍数 ÷ 最大近傍数（閾値=0.3） |
| `search_radius_inverse` | f64 | [0,1] | 探索半径の減少関数 | 1.0 / (1.0 + mean_help_distance)。HELP参加者間の平均L2距離を反転。HELP不在時=0.5 |
| `reasoning_steps_inverse` | f64 | [0,1] | 推論深度の減少関数 | 1.0 / (1.0 + mean_topo_depth)。全WFにcompile_to_stepsを適用した出力長の平均を反転。空グラフ時=0.5 |
| — | — | — | population_growth_rate（既存）はj_pop→j_pop_growthに相当 | 新規追加不要（既存フィールドを流用） |

### 3. KindWorldAssessment に7新規下位成分追加

- `j_pop_growth: f64`（j_popからの名前変更）
- `j_nest_depth: f64`, `j_node_density: f64`（S_density用）
- `j_clustering: f64`, `j_local_density: f64`（S_topology用）
- `j_search_radius_inv: f64`, `j_reasoning_steps_inv: f64`（S_search用）
- 下位成分合計: 14 → 20

### 4. compute_kind_world_objective の因子計算式変更

```rust
let s_growth = (j_pop_growth + j_lifecycle + j_child_survival + j_freshness) / 4.0;
let s_density = (j_cov + j_diffusion + j_reuse + j_nest_depth + j_node_density) / 5.0;
let s_topology = (j_benevolence + j_reciprocity + j_help + j_trust + j_clustering + j_local_density) / 6.0;
let s_search = (j_cost + j_execution + j_search_radius_inv + j_reasoning_steps_inv) / 4.0;
let s_fairness = 1.0 - j_penalty;  // 変更なし

let j_kw = (s_growth * s_density * s_topology * s_search * s_fairness).clamp(0.0, 1.0);
let min_factor = s_growth.min(s_density).min(s_topology).min(s_search).min(s_fairness);
let is_kind_world = j_kw > 0.8 && min_factor > 0.6;
```

### 5. collect_final_metrics（SimulationContext版）で新7指標を計算

- **mean_nest_depth**: SimulationContext.memoized_graph の各 WorkflowGraph を再帰的にトラバースし、サブWF参照（WorkflowNode::SubWorkflow）を辿った最大深度の平均
- **mean_node_density**: 全 WorkflowGraph のノード数合計 ÷ グラフ数
- **cluster_coefficient**: positions から各ノードのk近傍（k=5）を計算し、Watts-Strogatz クラスター係数の平均。各ノードの近傍間のエッジ数 ÷ (k*(k-1)/2)。近傍数が2未満のノードは0.0
- **local_density**: positions から各ノードの閾値半径0.3内の近傍数を測定、最大近傍数（全ノード中の最大値）で正規化した値の平均。全ノードの近傍数が0の場合は0.0
- **search_radius_inverse**: help_sessions の各セッションで from_workflow と to_workflow の位置間L2距離を計算し、平均を 1.0 / (1.0 + mean_distance) で反転。help_sessionsが空の場合は0.5
- **reasoning_steps_inverse**: memoized_graph 内の全 WorkflowGraph に compile_to_steps を適用し、出力Vecの長さの平均を 1.0 / (1.0 + mean_length) で反転。グラフが空の場合は0.5

### 6. collect_final_metrics_from_result（旧経路）
SimulationContextにアクセスできないため、新7指標は全てデフォルト値0.5を設定する。

### 7. KW4 TC6 Kind World Check出力更新
legacy_flagsベースの判定表示を新5因子の各値＋min_factor表示に変更する。

### 8. KW4 specの該当箇所更新
- 旧因子名参照を新因子名に更新
- 「全8条件成立」→「5因子最小値ゲート通過（min_factor > 0.6）」に修正

## Non-scope

- compute_blended_freshness / compute_experience_normalization の修正（変更不要）
- village.rs / spaceposition.rs への関数追加（collect_final_metrics内部で完結）
- simulation.rs の6フェーズループ変更（不要）
- is_kind_world 閾値（0.8/0.6）の変更（較正はKW4で実施）
- 新規クレート・外部依存の追加（不要）
- Legacy 8二値フラグの削除（KindWorldAssessment.legacy_flagsはdiagnosticsとして維持）

## Investigation

### 調査日時: 2026-05-27

以下の物理的証拠に基づき、現状の5因子モデルと社会加速度定義の乖離を検証した。

#### 現状の compute_kind_world_objective（kind_world.rs:421-617）

```
S_viability = (j_pop + j_lifecycle + j_child_survival + j_freshness) / 4.0
S_capability = (j_cov + j_diffusion + j_reuse) / 3.0
S_cooperation = (j_benevolence + j_reciprocity + j_help + j_trust) / 4.0
S_efficiency = (j_cost + j_execution) / 2.0
S_fairness = 1.0 - j_penalty
```

#### 不足している指標のgrep確認

```bash
grep -rn "cluster_coefficient\|clustering_coefficient\|local_density\|embedding_density" src/ → 該当なし
grep -rn "search_radius\|reasoning_depth\|reasoning_steps\|exploration_radius" src/ → 該当なし
grep -rn "nest_depth\|subworkflow_depth\|nested_depth\|mean_depth" src/ → 該当なし
```

#### 既存データソースの確認

**cluster_coefficient / local_density の計算可能性:**
| データソース | ファイル | 行 | 備考 |
|-------------|---------|----|------|
| SimulationContext.positions | simulation.rs | 268 | HashMap<NodeId, SpacePositionEmbedding> = [f32; 3] |
| l2_distance | spaceposition.rs | 存在確認済 | L2距離計算関数 |

**search_radius_inverse の計算可能性:**
| データソース | ファイル | 行 | 備考 |
|-------------|---------|----|------|
| SimulationContext.help_sessions | simulation.rs | 274 | Vec<HelpSession> |
| HelpSession.from_workflow / to_workflow | help.rs | 存在確認済 | WorkflowGraphId (String) |
| positionsからのNodeId解決 | — | — | 村割り当てロジックから流用可能 |

**reasoning_steps_inverse の計算可能性:**
| データソース | ファイル | 行 | 備考 |
|-------------|---------|----|------|
| compile_to_steps | compiler.rs | 新規実装済 | トポロジカルソート長 = ステップ数 |
| memoized_graph | types.rs | 91 | DiGraph<WorkflowNode, EdgeMeta> → 全WFコンテナ |

**nest_depth / node_density の計算可能性:**
| データソース | ファイル | 行 | 備考 |
|-------------|---------|----|------|
| WorkflowNode::SubWorkflow | types.rs | 53 | サブWF参照検出可能 |
| WorkflowGraph = DiGraph<WorkflowNode, EdgeMeta> | types.rs | 91 | ノード数・エッジ数取得可能 |

#### 旧経路（collect_final_metrics_from_result）の制約

collect_final_metrics_from_result（kind_world.rs:2676）は ReciprocitySimulationResult を受け取る旧経路であり、SimulationContext にアクセスできない。新7指標の正確な計算に必要な positions / help_sessions / memoized_graph が利用不可であるため、デフォルト値0.5を設定する。

#### 既存14指標の因子間再配置マップ

| 指標 | 旧因子 | 新因子 | 扱い |
|------|--------|--------|------|
| j_pop (→j_pop_growth) | S_viability | S_growth | 名前変更のみ（値は同一） |
| j_lifecycle | S_viability | S_growth | 移動なし |
| j_child_survival | S_viability | S_growth | 移動なし |
| j_freshness | S_viability | S_growth | 移動なし |
| j_cov | S_capability | S_density | 移動なし |
| j_diffusion | S_capability | S_density | 移動なし |
| j_reuse | S_capability | S_density | 移動なし |
| j_nest_depth | (新規) | S_density | NEW |
| j_node_density | (新規) | S_density | NEW |
| j_benevolence | S_cooperation | S_topology | 移動なし |
| j_reciprocity | S_cooperation | S_topology | 移動なし |
| j_help | S_cooperation | S_topology | 移動なし |
| j_trust | S_cooperation | S_topology | 移動なし |
| j_clustering | (新規) | S_topology | NEW |
| j_local_density | (新規) | S_topology | NEW |
| j_cost | S_efficiency | S_search | 移動なし |
| j_execution | S_efficiency | S_search | 移動なし |
| j_search_radius_inv | (新規) | S_search | NEW |
| j_reasoning_steps_inv | (新規) | S_search | NEW |
| j_penalty | S_fairness | S_fairness | 移動なし |

#### 参照観察レポート

- tickets/context/0118-m176-kw-real-p6/observation-20260527-082055.md — P6完了。5因子乗算モデル導入完了。1237テストPASS。KW_ALPHA_* 6定数削除。次チケットへの示唆として「通常作業へ移行」を記録。
- tickets/context/0117-m176-kw-real-p3/observation-20260527-075011.md — P3完了。compile_to_steps実装。ワークフロー実行抽象化としてトポロジカルソートが利用可能に。
- tickets/context/0116-m176-kw-real-p2-gmr/observation-20260526-204244.md — P2完了。GMR抽象化層実装。AGチャネルスコア分布を観測。
- tickets/context/0114-m176-kw-real-p5/observation-20260526-195519.md — P5完了。ライフサイクル・成熟機構完了。LifecycleScore計算に mean_lifecycle_score が利用可能に。

## Test Plan

### ユニットテスト（kind_world.rs 内 mod tests）

| ID | テスト | アサーション |
|----|--------|-------------|
| TC1 | KindWorldMetricsInput 24フィールド構築 | 新7指標が全て正しく構築可能（zero()含む） |
| TC2 | compute_kind_world_objective 新5因子計算 | s_growth, s_density, s_topology, s_search, s_fairness が各々[0,1]範囲で正しく計算される |
| TC3 | 乗算構造維持 | 任意の1因子を0.0に設定 → J_kw = 0.0 |
| TC4 | s_growth 組成確認 | j_pop_growth + j_lifecycle + j_child_survival + j_freshness の4指標算術平均であること |
| TC5 | s_density 組成確認 | j_cov + j_diffusion + j_reuse + j_nest_depth + j_node_density の5指標算術平均であること |
| TC6 | s_topology 組成確認 | j_benevolence + j_reciprocity + j_help + j_trust + j_clustering + j_local_density の6指標算術平均であること |
| TC7 | s_search 組成確認 | j_cost + j_execution + j_search_radius_inv + j_reasoning_steps_inv の4指標算術平均であること |
| TC8 | 全指標NaN/Infフリー | 新7指標を含む全20下位成分が有限値かつ[0,1]範囲 |
| TC9 | collect_final_metrics 新指標実測 | SimulationContextから新7指標が正しく抽出される |
| TC10 | collect_final_metrics_from_result デフォルト値 | 旧経路で新7指標が全て0.5を返す |
| TC11 | KW4 TC6 出力形式 | Kind World Check出力が新因子名（growth/density/topology/search/fairness）を含む |
| TC12 | 後方互換性 | 既存テスト全PASS |

### 観測テスト

| ID | テスト | 観測対象 |
|----|--------|---------|
| OBS1 | 新旧モデル比較出力 | 旧5因子値 vs 新5因子値の差分率、新7指標の寄与率 |
| OBS2 | 新7指標の値分布 | cluster_coefficient/local_density が[0,1]範囲に収まることの確認 |
| OBS3 | デフォルト値フォールバック警告 | search_radius_inverse/reasoning_steps_inverseがデフォルト値にフォールバックした場合の警告出力 |

## 計装方法・観測対象

### 計装方法

- 専用テスト関数: kind_world.rs 内 `mod tests` に上記TC1〜TC12を実装
- 観測テスト: --nocapture で以下の構造化出力を行う
  - JSON: collect_final_metrics 出力（全20下位成分 + 新5因子値 + J_kw）
  - CSV: 新旧モデル比較（旧5因子 vs 新5因子、各因子の差分率、新7指標の寄与率）
  - 時系列: 各metrics成分の値をtick別に出力
- 固定シード: StdRng::seed_from_u64(12345) でシミュレーション決定論的再現
- 新7指標の計算に使用する定数:
  - クラスター係数のk近傍: k=5（KW_ACCEL_K_NEAREST）
  - 局所密度の閾値半径: 0.3（KW_ACCEL_DENSITY_RADIUS）
  - HELP距離計算の定数: なし（直接L2距離を使用）

### 観測対象

| 観測対象 | 内容 | サンプルサイズ |
|---------|------|--------------|
| クラスター係数分布 | 全WorkflowGraphの平均Watts-Strogatz係数 | 1観測（シミュレーション1回の全positions） |
| 局所密度分布 | 全ノードの閾値半径内近傍数 | 1観測（同上） |
| 探索半径逆数 | HELPセッション平均距離の逆数 | HELPセッション数に依存 |
| 推論深度逆数 | compile_to_steps出力長の平均 | WF数に依存 |

### 較正計画

本チケットではJ_kwの式そのものを確定させるのが目的であり、閾値の較正は行わない。
- 調整候補: なし（定数の追加はKW_ACCEL_K_NEAREST, KW_ACCEL_DENSITY_RADIUSのみ）
- 目的関数 J(θ): 該当なし（閾値較正はKW4で実施）
- 停止条件: 全テストPASS + cargo clippy 警告ゼロ

## Boy Scout Rule — 翻訳可能性計画

### 本チケットで新規作成/変更するコード

- **src/kind_world.rs**:
  - KindWorldMetricsInput: 7新規フィールドの変数名はドメイン概念を表す（mean_nest_depth, cluster_coefficient 等）
  - compute_kind_world_objective: 因子変数名を定義側面に合わせて改名（s_viability→s_growth等）
  - collect_final_metrics（SimulationContext版）: 新7指標の計算は各1関数に分割。例えば `compute_cluster_coefficient(positions)`、`compute_mean_nest_depth(memoized_graph)` のように関数抽出し、処理の流れが関数名の並びだけで追えるようにする
  - 既存のunwrap禁止、?演算子によるエラー伝播
  - コメントは「なぜ」のみ。「何を」は関数名と変数名が語る

### 修正する既存コード

- **src/constants.rs**: KW_ACCEL_K_NEAREST, KW_ACCEL_DENSITY_RADIUS の2定数追加
- **Darvium-Tickets-v2.3.md**: KW4の仕様に新因子名を反映（別途実施済み）

## Acceptance Criteria

- [ ] KindWorldMetricsInput の24フィールドが全て正しく構築可能である（TC1）
- [ ] compute_kind_world_objective が新5因子（s_growth, s_density, s_topology, s_search, s_fairness）を正しく計算する（TC2）
- [ ] 新5因子の乗算構造が維持されている（TC3）
- [ ] 各因子の組成が定義通りである（TC4〜TC7）
- [ ] 全指標が[0,1]範囲かつNaN/Infフリーである（TC8）
- [ ] collect_final_metrics がSimulationContextから新7指標を正しく抽出する（TC9）
- [ ] collect_final_metrics_from_result が新7指標にデフォルト値0.5を設定する（TC10）
- [ ] KW4 TC6 のKind World Check出力が新因子名でフォーマットされる（TC11）
- [ ] 既存テスト全PASS + cargo clippy 警告ゼロ（TC12）
- [ ] 翻訳可能性の検証が通っている（関数名・変数名・定数化・責務分割）

## Notes

### 依存関係

- **必須前提**: P3（compile_to_steps）完了 → reasoning_steps_inverseで使用
- **必須前提**: P6（5因子乗積モデル導入）完了 → 本チケットはその修正版
- **後続条件**: 本チケット完了がKW4（較正ループ）の必須前提 (MUST NOT)
- 新規クレート追加: 不要（petgraph + 標準ライブラリで完結）

### 設計判断: j_search_radius_inv

**決定**: HELPセッション参加者間の平均L2距離の逆数 = 1.0 / (1.0 + mean_help_distance)

- 根拠: SimulationContext.positions に全個人の3次元座標が格納済み。HELP発生時に助けを求めた個人と応答した個人の空間距離が探索半径のプロキシとなる
- v1として十分な理由: 埋め込み空間で「近い個人を探索する」という現象を直接測定しており、定義④の要求を近似として満たす
- HELP不在時のフォールバック: 0.5（中立値）

### 設計判断: j_reasoning_steps_inv

**決定**: compile_to_steps の出力長（トポロジカル順序のノード数）の平均の逆数 = 1.0 / (1.0 + mean_topo_depth)

- 根拠: compile_to_steps が既に実装済み（チケットP3）。各WorkflowGraphのトポロジカルソート結果のノード数がステップ数のプロキシとなる
- v1として十分な理由: 追加の計装ゼロで実現可能。WFの構造的複雑さを直接測定
- 空グラフ時のフォールバック: 0.5（中立値）

### 成果物

- 計画: context/0119-m176-kw-accel-j-kw-57/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0119-m176-kw-accel-j-kw-57/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0119-m176-kw-accel-j-kw-57/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0119-m176-kw-accel-j-kw-57/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
