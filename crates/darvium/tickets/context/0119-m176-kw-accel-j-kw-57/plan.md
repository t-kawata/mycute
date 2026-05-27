# M1.76-KW-ACCEL: J_kw 社会加速度定義完全一致 — 5因子再定義＋7指標追加

## RFC 既存実装状態検証

### RFC §15.9.2 vs 現行コード vs 新スペック

**RFC §15.9.2 の 5 因子定義**（現行コードはこれと完全一致）:
- S_viab = (J_pop + J_lifecycle + J_child_survival + J_freshness) / 4
- S_capa = (J_cov + J_diffusion + J_reuse) / 3
- S_coop = (J_benevolence + J_reciprocity + J_help + J_trust) / 4
- S_effi = (J_cost + J_execution) / 2
- S_fair = 1.0 - J_penalty

**新スペック（チケット119）の 5 因子定義**:
- s_growth = (j_pop_growth + j_lifecycle + j_child_survival + j_freshness) / 4
- s_density = (j_cov + j_diffusion + j_reuse + j_nest_depth + j_node_density) / 5
- s_topology = (j_benevolence + j_reciprocity + j_help + j_trust + j_clustering + j_local_density) / 6
- s_search = (j_cost + j_execution + j_search_radius_inv + j_reasoning_steps_inv) / 4
- s_fairness = 1.0 - j_penalty

### compute_kind_world_objective（kind_world.rs:421-617）

| 構成要素 | RFC §15.9.2 | 現行コード | 新スペック | 状態 |
|---------|-------------|-----------|-----------|------|
| j_pop → j_pop_growth | ✅ J_pop | ✅ j_pop | j_pop_growth rename | ❌ リネーム |
| j_lifecycle | ✅ | ✅ | S_growth移動なし | ✅ 一致 |
| j_child_survival | ✅ | ✅ | S_growth移動なし | ✅ 一致 |
| j_freshness | ✅ | ✅ | S_growth移動なし | ✅ 一致 |
| j_cov | ✅ | ✅ | S_density移動なし | ✅ 一致 |
| j_diffusion | ✅ | ✅ | S_density移動なし | ✅ 一致 |
| j_reuse | ✅ | ✅ | S_density移動なし | ✅ 一致 |
| j_nest_depth | (未定義) | (不在) | 新規 S_density | ❌ 欠落 |
| j_node_density | (未定義) | (不在) | 新規 S_density | ❌ 欠落 |
| j_benevolence | ✅ | ✅ | S_topology移動なし | ✅ 一致 |
| j_reciprocity | ✅ | ✅ | S_topology移動なし | ✅ 一致 |
| j_help | ✅ | ✅ | S_topology移動なし | ✅ 一致 |
| j_trust | ✅ | ✅ | S_topology移動なし | ✅ 一致 |
| j_clustering | (未定義) | (不在) | 新規 S_topology | ❌ 欠落 |
| j_local_density | (未定義) | (不在) | 新規 S_topology | ❌ 欠落 |
| j_cost | ✅ | ✅ | S_search移動なし | ✅ 一致 |
| j_execution | ✅ | ✅ | S_search移動なし | ✅ 一致 |
| j_search_radius_inv | (未定義) | (不在) | 新規 S_search | ❌ 欠落 |
| j_reasoning_steps_inv | (未定義) | (不在) | 新規 S_search | ❌ 欠落 |
| j_penalty | ✅ | ✅ | S_fairness移動なし | ✅ 一致 |
| S_viability → s_growth | ✅ S_viab = 4/4 | ✅ s_viability | 4因子→rename | ❌ リネーム |
| S_capability → s_density | ✅ S_capa = 3/3 | ✅ s_capability | 5因子(+2新規) | ❌ 構成変更 |
| S_cooperation → s_topology | ✅ S_coop = 4/4 | ✅ s_cooperation | 6因子(+2新規) | ❌ 構成変更 |
| S_efficiency → s_search | ✅ S_effi = 2/2 | ✅ s_efficiency | 4因子(+2新規) | ❌ 構成変更 |
| s_fairness | ✅ | ✅ | 変更なし | ✅ 一致 |

**評価サマリ**: 14/20 下位成分は既存コードで充足。7 成分の新規追加 + 1 成分のリネーム + 4 因子のリネーム + 3 因子の構成変更が必要。

### KindWorldMetricsInput（kind_world.rs:39-109）

17 フィールド（既存）→ 7 フィールド追加 → 24 フィールド。既存フィールドのデータ型・範囲は変更不要。

### KindWorldAssessment（kind_world.rs:183-247）

14 下位成分 + 5 因子 + legacy_flags → 7 下位成分追加 → 21 下位成分 + 5 因子（リネーム）+ legacy_flags。

### 既存データソースの可用性検証

| 新指標 | データソース | 所在 | 利用可否 |
|--------|------------|------|---------|
| mean_nest_depth | memoized_graph 各WFのSubWorkflow再帰トラバース | types.rs:53, types.rs:91 | ✅ 利用可 |
| mean_node_density | 全WFノード数合計÷グラフ数 | types.rs:91 | ✅ 利用可 |
| cluster_coefficient | positionsからk近傍(k=5) | simulation.rs:268, spaceposition.rs | ✅ 利用可（要l2_distance） |
| local_density | 閾値半径0.3内の近傍数 / 最大近傍数 | positions + l2_distance | ✅ 利用可 |
| search_radius_inverse | help_sessionsのL2距離平均の逆数 | simulation.rs:274, help.rs | ✅ 利用可 |
| reasoning_steps_inverse | compile_to_steps出力長 | compiler.rs:33 | ✅ 利用可（error 要ハンドリング） |
| collect_final_metrics_from_result | 旧経路、SimulationContext不在 | kind_world.rs:2676 | ⚠️ デフォルト値0.5のみ |

### Investigation 更新

spec 作成時刻と同一セッションのため、Investigation セクションの情報は最新。追加情報：

- compile_to_steps は `Result<Vec<NodeId>, DarviumError>` を返す（compiler.rs:33） → reasoning_steps_inverse 計算時に error 時は 0.5 フォールバックが必要
- positions の型は `HashMap<NodeId, SpacePositionEmbedding>` で SpacePositionEmbedding = `[f32; 3]` → L2距離計算のための f32→f64 変換が必要
- zero() コンストラクタ（kind_world.rs:113-157）は現状17フィールド → 24フィールドに拡張

## 要件の再確認

J_kw の 5 因子乗算結合モデルを社会加速度の定義に完全一致させる：

1. 4 因子名の変更（viability→growth, capability→density, cooperation→topology, efficiency→search）
2. j_pop→j_pop_growth へのリネーム
3. 7 新規下位成分の追加（nest_depth, node_density, clustering, local_density, search_radius_inv, reasoning_steps_inv）
4. 3 因子の構成変更（密度 3→5, トポロジー 4→6, 探索 2→4）
5. collect_final_metrics での新指標計算実装
6. KW4 TC6 出力フォーマット更新

**後方互換性**: KindWorldAssessment 構造体のフィールドは新規追加のみで破壊的変更なし。因子名フィールド（s_viability 等）は RFC §15.9.2 の定義名そのものであり、これを新定義名に変更することは RFC 追従であり破壊的変更ではない。呼び出し元（simulation.rs テストコード）の該当フィールド参照は更新が必要。

## 変更ファイル一覧

| ファイル | 種別 | 内容 |
|---------|------|------|
| src/constants.rs | 変更 | KW_ACCEL_K_NEAREST (=5), KW_ACCEL_DENSITY_RADIUS (=0.3) の 2 定数追加 |
| src/kind_world.rs | 変更 | 主要変更ファイル。KindWorldMetricsInput +7 フィールド追加、KindWorldAssessment +7 下位成分 +4 因子リネーム、compute_kind_world_objective 再構成、5 つの新規ヘルパー関数、collect_final_metrics 統合、collect_final_metrics_from_result デフォルト値、TC6 出力フォーマット更新、全テスト更新 |
| src/simulation.rs | 変更（軽微） | collect_final_metrics_from_result または compute_kind_world_objective 呼び出し元で KindWorldAssessment の因子名フィールドを参照している箇所の更新（あれば） |

## 計装・観測の実装計画

### 実装するテストコード

全テストは `kind_world.rs` 内 `mod tests` に実装：

| ID | テスト関数名 | 内容 |
|----|-------------|------|
| TC1 | `tc1_metrics_input_24_fields` | KindWorldMetricsInput 全24フィールド構築確認 |
| TC2 | `tc2_new_5_factor_computation` | s_growth/density/topology/search/fairness 計算確認 |
| TC3 | `tc3_multiplicative_structure` | 1因子=0.0→J_kw=0.0 |
| TC4 | `tc4_s_growth_composition` | 4指標算術平均確認 |
| TC5 | `tc5_s_density_composition` | 5指標算術平均確認 |
| TC6 | `tc6_s_topology_composition` | 6指標算術平均確認 |
| TC7 | `tc7_s_search_composition` | 4指標算術平均確認 |
| TC8 | `tc8_all_components_finite` | 全20下位成分[0,1]かつNaN/Infフリー |
| TC9 | `tc9_collect_final_metrics_new_fields` | SimulationContext から新7指標抽出（PositionIndex を使ったモック） |
| TC10 | `tc10_old_path_default_values` | collect_final_metrics_from_result で新7指標=0.5 |
| TC11 | `tc11_kw4_tc6_output_format` | KW4 TC6 出力が新因子名を含むことの確認 |
| TC12 | — | 既存テスト全PASSで代用 |

### 観測テスト（--nocapture）

`tc9_collect_final_metrics_new_fields` 内で以下の構造化出力を行う：
- JSON: 全20下位成分 + 新5因子値 + J_kw
- 新旧モデル比較（旧5因子値 vs 新5因子値の差分率）

### 観測すべき統計量とサンプルサイズ

| 統計量 | サンプルサイズ | 出力方法 |
|--------|-------------|---------|
| クラスター係数分布 | 1観測（全positions） | println! |
| 局所密度分布 | 1観測（同上） | println! |
| 探索半径逆数 | HELPセッション数依存 | println! |
| 推論深度逆数 | WF数依存 | println! |

### 較正対象の定数と目的関数 J(θ)

本チケットでは J_kw の式確定のみ。較正は KW4 で実施。
- 追加する定数（実装定数、Calibration Candidate ではない）:
  - `KW_ACCEL_K_NEAREST: usize = 5` — クラスター係数計算の k 近傍数
  - `KW_ACCEL_DENSITY_RADIUS: f64 = 0.3` — 局所密度計算の閾値半径

### 較正ループの停止条件

全テスト PASS + `cargo clippy -- -D warnings` 警告ゼロ

## Boy Scout 改善（スコープ外の翻訳可能性修正）

### 1. zero() コンストラクタのフォーマット異常
kind_world.rs:147-154 に、他のフィールドと異なるインデントで追記された 8 フィールドがある。この 7+1（新規追加含む）フィールドのインデントを統一する。

### 2. compute_kind_world_objective 内の 14 成分 clamp の関数抽出
現在 compute_kind_world_objective (kind_world.rs:481-527) で各成分の clamp がインラインで書かれている。各成分の clamp（`metrics.xxx.clamp(0.0, 1.0)`）は「正規化」という単一の責務を持つため、ヘルパー関数 `fn clamp_metric(value: f64) -> f64` に抽出する。これにより 14 行のパターンが意図（「成分を [0,1] に正規化する」）として明示される。

※ collect_final_metrics 内の新 7 指標計算は spec の「各 1 関数に分割」方針に従い、それぞれ独立した関数として抽出済み（Boy Scout ではなく設計方針）。

## 実装手順

### Step 1: constants.rs に 2 定数追加

`KW_ACCEL_K_NEAREST: usize = 5` と `KW_ACCEL_DENSITY_RADIUS: f64 = 0.3` を KW 関連定数セクションの末尾に追加。

### Step 2: KindWorldMetricsInput に 7 フィールド追加

kind_world.rs:39-109 の構造体に以下を追加：
- `mean_nest_depth: f64` — 平均サブWFネスト深度
- `mean_node_density: f64` — 1ルートWFあたり平均ノード数
- `cluster_coefficient: f64` — 平均Watts-Strogatzクラスター係数 [0,1]
- `local_density: f64` — 埋め込み空間局所密度 [0,1]
- `search_radius_inverse: f64` — 探索半径の減少関数 [0,1]
- `reasoning_steps_inverse: f64` — 推論深度の減少関数 [0,1]
- 注: population_growth_rate は既存フィールドを流用（j_pop → j_pop_growth）

合わせて zero() コンストラクタにも 7 フィールドを追加（全て 0.0）。

### Step 3: KindWorldAssessment に 7 下位成分追加 + 4 因子リネーム

kind_world.rs:183-247 の構造体を更新：
- `s_viability` → `s_growth` にリネーム（フィールド名変更）
- `s_capability` → `s_density` にリネーム
- `s_cooperation` → `s_topology` にリネーム
- `s_efficiency` → `s_search` にリネーム
- `s_fairness` → 変更なし
- `j_pop` → `j_pop_growth` にリネーム
- `j_nest_depth: f64` 追加
- `j_node_density: f64` 追加
- `j_clustering: f64` 追加
- `j_local_density: f64` 追加
- `j_search_radius_inv: f64` 追加
- `j_reasoning_steps_inv: f64` 追加

### Step 4: compute_kind_world_objective 再構成

kind_world.rs:421-617 の関数内部を以下の順で書き換え：

1. `j_pop` → `j_pop_growth` に変数名変更（値計算は同一）
2. 新規 7 成分の clamp 計算を追加
3. 5 因子の算術平均を新定義に変更
4. 乗算結合の変数名を新因子名に変更
5. min_factor の変数名を新因子名に変更
6. KindWorldAssessment コンストラクタのフィールド指定を新構造体に合わせる

### Step 5: 5 つの新規ヘルパー関数追加

kind_world.rs の collect_final_metrics 直前に以下 6 関数を追加：

```rust
fn compute_mean_nest_depth(graph: &crate::types::WorkflowGraph) -> f64
fn compute_mean_node_density(graph: &crate::types::WorkflowGraph) -> f64
fn compute_cluster_coefficient(positions: &HashMap<...>) -> f64
fn compute_local_density(positions: &HashMap<...>) -> f64
fn compute_search_radius_inverse(help_sessions: &[HelpSession], positions: &HashMap<...>) -> f64
fn compute_reasoning_steps_inverse(graph: &crate::types::WorkflowGraph) -> f64
```

各関数の詳細設計：
- `compute_mean_nest_depth`: WorkflowGraph の全ノードを再帰的にトラバースし、SubWorkflow 参照を辿った最大深度の平均。グラフが空なら 0.0。
- `compute_mean_node_density`: petgraph の `.node_count()` を使用。グラフ数が 0 なら 0.0。
- `compute_cluster_coefficient`: 各ノードの positions から L2 距離で k 近傍（k=5）を計算。近傍間のエッジ数 ÷ (k*(k-1)/2)。近傍数 < 2 のノードは 0.0。全ノードの平均。
- `compute_local_density`: 閾値半径 0.3 内の近傍数を計算。最大近傍数（全ノード中の max）で正規化した値の平均。全近傍数 0 なら 0.0。
- `compute_search_radius_inverse`: help_sessions から from/to 各ペアの L2 距離を計算し平均。1.0/(1.0+mean)。空なら 0.5。
- `compute_reasoning_steps_inverse`: `compile_to_steps` の出力長平均を反転。error 時は 0.5、空WFリストなら 0.5。

### Step 6: collect_final_metrics（SimulationContext版）統合

collect_final_metrics (kind_world.rs:2776) の返却値に新 7 フィールドを設定。Step 5 のヘルパー関数を呼び出す。

### Step 7: collect_final_metrics_from_result（旧経路）デフォルト値

collect_final_metrics_from_result (kind_world.rs:2676) の返却値に新 7 フィールドを全て 0.5 で追加。

### Step 8: KW4 TC6 出力フォーマット更新

tc6_kw4_optimize_run (kind_world.rs:7248) の `--- Kind World Check ---` セクションを以下のように変更：
- `"X/8 flags"` → 新 5 因子の各値と min_factor を表示
- legacy_flags ベースの欠落表示を因子値ベースに変更
- 因子名を新名（growth/density/topology/search/fairness）で表示

### Step 9: テストコード更新

- TC2〜TC8 の既存テストを新フィールド・新因子名に対応させる
- TC1-TC12 の新規テストを追加
- simulation.rs 側で KindWorldAssessment の因子名フィールド（s_viability 等）を参照している箇所を新しいフィールド名に更新

### Step 10: テスト・リント実行

```bash
cargo test
cargo clippy -- -D warnings
cargo fmt
```

## 物理的レビュー方法

### 1. run-quality-checks.js

```bash
_R=$(cat DARVIUM_PLUGIN_ROOT.md)
node "$_R/scripts/tickets/review/run-quality-checks.js" src/kind_world.rs src/constants.rs src/simulation.rs | node "$_R/scripts/tickets/review/generate-report.js"
```

### 2. 翻訳可能性 grep

```bash
# 新規追加された関数定義を確認（動詞句であること）
grep -n "^fn " src/kind_world.rs | grep -v "^fn test_\|^fn tc"

# 1文字変数・汎用名の新規追加がないか
grep -n "let [a-z] =" src/kind_world.rs | grep -v "for \|let i =\|let j =\|let k =\|\.iter()\|\.map\|\.filter"

# マジックナンバーが直接書かれていないか（0.0, 0.5, 1.0 以外の数値リテラル）
grep -n "[0-9]\.[0-9]" src/kind_world.rs | grep -v "0\.0\|0\.5\|1\.0\|0\.3\|0\.8\|0\.6\|5\.0\|4\.0\|2\.0\|3\.0\|6\.0\|0\.01\|0\.05\|0\.30\|0\.1\|0\.35\|0\.08\|0\.85\|0\.0001\|0\.00005\|0\.70\|0\.50\|0\.90\|0\.95"

# デバッグ出力が残っていないか
grep -n "eprintln!\|dbg!" src/kind_world.rs src/constants.rs

# コメントは「なぜ」のみか（「何を」を説明していないか）
# → 新規追加箇所のコメントを目視確認
```

### 3. 構造整合性チェック

```bash
_R=$(cat DARVIUM_PLUGIN_ROOT.md)
node "$_R/scripts/tickets/validate-structure.js"
```

### 4. テスト全通過確認

```bash
cargo test
```

## リスク

| リスク | 影響 | 緩和策 |
|--------|------|--------|
| cluster_coefficient O(n²) 計算量 | positions 数が大規模の場合のパフォーマンス低下 | 現状のシミュレーション規模では問題にならない。将来の最適化は KW4 以降に回す |
| compile_to_steps の error ハンドリング | 空グラフや循環依存で Result::Err が返る | 0.5 フォールバックで常に安全側 |
| 旧経路（collect_final_metrics_from_result）のデフォルト値 | 新指標が常に 0.5 で J_kw 値が不正確になる | 本チケットではやむを得ない（旧経路は P4 以降の移行で廃止予定）。KW4 も SimulationContext 版を使用する |
| simulation.rs でのフィールド名参照 | 因子名リネームによりコンパイルエラー | 事前に全参照箇所を grep して確認してから変更する |
