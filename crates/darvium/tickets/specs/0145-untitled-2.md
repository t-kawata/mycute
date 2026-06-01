---
ticket_id: 145
title: ワークフロー複雑化メカニズム完全活性化
slug: workflow-complexity-full-activation
status: reviewed
created_at: 2026-05-29
updated_at: 2026-05-29
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0145-untitled-2/plan.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0145-untitled-2/implementation.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0145-untitled-2/observation-20260529-123330.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0145-untitled-2/review.md
---

# ワークフロー複雑化メカニズム完全活性化

## Summary

#142〜#144 で「複雑化の仕組み」は実装されたが、**それらが実際に機能していない**ことがブラウザ観測で判明した。本チケットでは以下の修正を一括して行い、ワークフロー複雑化の全メカニズムを活性化する：

1. **GMR 常時有効化**: 全 3 プロダクションシミュレーションパスで `use_gmr = false` にハードコードされている問題を修正し、`ReciprocitySimulatorConfig` からの制御可能かつデフォルト有効にする
2. **HELP 提案 + 確率的受入モデル**: `copy_graph_if_more_complex` を受動的全体コピーから能動的部分提案 + 確率的受入に置き換える
3. **出生時複雑化の強化**: `WORKFLOW_GENERATION_MAX_COMPLEXITY` を引き上げ、SearchWorkflow の `try_patch_existing` を複数ノード追加に対応させる
4. **GMR DeterminismScore のグラフ構造ベース化**: 乱数ではなく実グラフ構造から DeterminismScore を計算する
5. **戻り値の破棄修正**: `_diffusions` プレフィックスを削除し、戻り値を実際に使用する

これらにより、出生・HELP・GMR の全経路でワークフロー複雑化が常時動作する状態を達成する。

## Background

### 発見経緯

#142（SearchWorkflow 複雑化）・#143（HELP ワークフロー伝搬）・#144（GMR DifferentialInference）の実装完了後、ブラウザ経由のシミュレーションを観測したところ、**誰も 15 ノードを超えない**ことが確認された。ノード数の分布は初期生成時の 4〜7 ノードからほとんど増加せず、複雑化が起きていない。

調査の結果、複数の原因が重層的に作用していることが判明した：

### 原因 1: GMR が全プロダクションパスで無効（最重要）

`SimulationContext.use_gmr` フラグが全 3 つのシミュレーションエントリポイントでハードコード `false`：

| パス | ファイル | 行 | use_gmr |
|------|---------|----|---------|
| `run_kw_real_simulation` | simulation.rs | 1501 | `false` |
| `run_evaluation_simulation` | simulation.rs | 1707 | `false` |
| `run_evaluation_simulation_with_channel` | simulation.rs | 1852 | `false` ← **ブラウザ経由** |

`use_gmr = true` はテスト関数内（simulation.rs:4716, 7512）のみ。加えて `ReciprocitySimulatorConfig` に `use_gmr` フィールドが存在しないため、外部から有効化する手段が完全に欠落している。

**結果**: #144 で実装した BFS サブグラフ抽出・GraphPatch 構築・`apply_patch_atomic` のパイプラインには決して到達しない。

### 原因 2: DeterminismScore が乱数ベース

`try_gmr_diffusion`（simulation.rs:3103-3105）：
```rust
let det_values: Vec<f64> = (0..5)
    .map(|_| ctx.rng.random::<f64>() * 0.5 + 0.5)
    .collect();
let det_score = DeterminismScore::compute(&det_values, SOFT_MIN_BETA);
```

グラフ構造を一切参照せず、乱数で DeterminismScore を擬似的に生成している。TODO コメントはあるが放置。たとえ GMR が有効でも、差分推論の「差分」が実際のグラフ構造と無関係である。

### 原因 3: phase5_capability_diffusion の戻り値が破棄されている

`run_evaluation_simulation`（simulation.rs:1787）と `run_evaluation_simulation_with_channel`（simulation.rs:1949）で：
```rust
let _diffusions = if !new_successes.is_empty() {
```

アンダースコアプリフィックスにより戻り値が破棄されている。`run_kw_real_simulation`（simulation.rs:1580）は変数に束縛されているが後続で未使用。

### 原因 4: HELP が受動的すぎる

現在の `copy_graph_if_more_complex`（simulation.rs:2940-2953）は **helper のノード数 >= helpee のノード数** のときに全体グラフを上書きするだけ。条件不成立時は何もしない：

```
[PHASE5] helper_node_count=14 helpee_node_count=15 copied=false  ← 何も起きない
```

これにより helper が helpee よりわずかに単純なだけで、HELP 成功が複雑化に一切寄与しなくなる。RFC §4A.5 Mechanism 25-26 が期待する「支援による能力伝搬」には能動的な提案モデルが適切である。

### 原因 5: 出生時複雑度の上限が低すぎる

- `WORKFLOW_GENERATION_MAX_COMPLEXITY = 2`（constants.rs:129）— フォールバック生成が最大 2〜3 ノード
- `try_patch_existing`（search_workflow.rs:227）— GraphPatch が単一 AddNode のみ。1 世代で高々 1 ノードしか増加しない
- COMPOSE 経路は複数ノードの再利用が可能だが、registry に十分な候補が蓄積されないと発動しない

### 参照観察レポート

- tickets/context/0144-gmr-differentialinference-phase-5/observation-20260529-093257.md — GMR ユニットテストでは avg_added=2.62 と正常動作確認済みだが、プロダクションで無効
- tickets/context/0143-help-phase-5/observation-20260529-091631.md — Phase5 条件付きコピーの動作確認。helper=14/helpee=15 で copied=false を観測
- tickets/context/0142-searchworkflow/observation-20260529-090248.md — SearchWorkflow 複雑化の観測。フォールバック複雑度が 1〜2 に制限されている

## Scope

本チケットは以下の 6 項目を全て実装する：

### 1. GMR 常時有効化（設定経由）

- `ReciprocitySimulatorConfig` に `pub use_gmr: bool` フィールドを追加（デフォルト `true`）
- 全 3 シミュレーションパスで `ctx.use_gmr = config.use_gmr` に変更（ハードコード `false` を削除）
- `SimulationContext::new()` のデフォルトは `false` のまま（テストの明示性維持）
- **デフォルト `true` とすることで、GMR が全シミュレーションで常時動作する**

### 2. DeterminismScore のグラフ構造ベース化

- `try_gmr_diffusion` 内の乱数ベース DeterminismScore 計算を、実際のグラフ構造に基づく計算に置き換える
- 具体的には helper と helpee のグラフのノード種別分布・エッジ密度・DAG 深さ等から DeterminismScore を算出
- TODO コメントで本来のセマンティック差分に基づく計算への拡張ポイントを記載

### 3. phase5_capability_diffusion 戻り値の活用

- `run_evaluation_simulation`（simulation.rs:1787）と `run_evaluation_simulation_with_channel`（simulation.rs:1949）の `_diffusions` を `diffusions` に変更
- `run_kw_real_simulation`（simulation.rs:1580）で `diffusions` 変数を実際に使用する（println! 計装に統合する等）

### 4. HELP 提案 + 確率的受入モデル

`copy_graph_if_more_complex` 関数を以下のロジックで置き換える：

1. **helper がサブグラフを提案**: helper のグラフから `extract_connected_subgraph` と同等の BFS で 2〜4 ノードの接続 AgentStep サブグラフを抽出する
2. **helpee が確率的に受け入れ**: `accept_probability` を helper の `benevolence_score` と `trust`（operational/semantic/temporal の平均）から計算する。`accept_probability = (helper_benevolence + avg_trust) / 2.0` 程度の式を基本とし、定数 `HELP_PROPOSAL_ACCEPT_BASE` で調整可能にする
3. **受入時は部分マージ**: `apply_patch_atomic` で helpee の既存グラフを温存したまま helper のサブグラフを追加する。全体上書きではなく部分追加にする
4. **本物実装への拡張ポイント**: TODO コメントで「本物実装では LLM が提案内容とミッションの関連性を評価し、helpee が「本当にミッション達成に有益か」を判断する」と記載
5. 元の `copy_graph_if_more_complex`（全体コピー）は削除する（提案モデルで代替完了）

これにより helper が helpee よりノード数が少なくても「助けようと頑張る」動作が生まれる。

### 5. 出生時複雑化の強化

- `WORKFLOW_GENERATION_MAX_COMPLEXITY` を 2 → 4 に引き上げ（constants.rs:129）
- `try_patch_existing`（search_workflow.rs:227）を複数ノード（2〜3）の AddNode を生成するよう拡張。TODO コメントで本来の完全実装への拡張ポイントを維持

### 6. Boy Scout 改善

- `try_gmr_diffusion` 内の乱数 DeterminismScore の TODO コメントを具体的な拡張方法に更新
- `_diffusions` → `diffusions` のリネーム
- 翻訳可能性の点検（関数名が動詞句か、変数名がドメイン概念を表すか）

## Non-scope

- セマンティックマージ（複数ワークフローの知的な統合）— TODO で拡張ポイントを示すに留める
- ApplicabilityScore による Stage5 5 方向分岐（REUSE/PATCH/COMPOSE/NEW/ABORT）— TODO
- SearchWorkflow の根本的再設計
- RFC §4A.3 の完全実装（本物の GraphPatchGenerator + PatchConfidence）
- ブラウザ可視化の改善

## Investigation

### 証拠 1: use_gmr = false のハードコード箇所（全 3 パス）

```bash
$ grep -n "use_gmr\s*=\s*false" src/simulation.rs
1501:    ctx.use_gmr = false;   # run_kw_real_simulation
1707:    ctx.use_gmr = false;   # run_evaluation_simulation
1852:    ctx.use_gmr = false;   # run_evaluation_simulation_with_channel
7104:    ctx.use_gmr = false;   # テスト（initial_population_has_non_empty_graphs）
7156:    ctx.use_gmr = false;   # テスト（initial_population_graphs_are_dag）
```

use_gmr = true の箇所：
```bash
$ grep -n "use_gmr\s*=\s*true" src/simulation.rs
4716:    ctx.use_gmr = true;    # テスト（test_gmr_enabled_simulation）のみ
7512:    ctx.use_gmr = true;    # テスト（test_gmr_t6b_integration_diffusion_count）のみ
```

### 証拠 2: ReciprocitySimulatorConfig に use_gmr フィールドがない

simulation.rs:82-128 — 全フィールドを確認。`use_gmr` は存在しない。

### 証拠 3: DeterminismScore が乱数ベース

simulation.rs:3102-3107:
```rust
// DeterminismScore の計算（TODO: 本物のグラフ構造に基づく計算に置き換え）
let det_values: Vec<f64> = (0..5)
    .map(|_| ctx.rng.random::<f64>() * 0.5 + 0.5)
    .collect();
let det_score =
    crate::gmr::DeterminismScore::compute(&det_values, crate::constants::SOFT_MIN_BETA);
```

### 証拠 4: _diffusions 破棄

simulation.rs:1787:
```rust
let _diffusions = if !new_successes.is_empty() {
    phase5_capability_diffusion(
        &mut ctx,
        &new_successes,
    )
```

simulation.rs:1949: 同様。

### 証拠 5: copy_graph_if_more_complex の受動性

simulation.rs:2940-2953: helper.node_count >= helpee.node_count のときのみ全体クローン。条件不成立時は false を返すだけで何もしない。

### 証拠 6: WORKFLOW_GENERATION_MAX_COMPLEXITY = 2

constants.rs:129:
```rust
pub const WORKFLOW_GENERATION_MAX_COMPLEXITY: usize = 2;
```

### 証拠 7: try_patch_existing が単一 AddNode

search_workflow.rs:237-245:
```rust
let operations = vec![PatchOperation::AddNode { node: new_node }];
```
単一ノードのみ。複数ノード追加のロジックは存在しない。

## Test Plan

### 不変条件テスト

**T1: GMR 有効化 — 全 3 パスで use_gmr が config 値に従う**
- `ReciprocitySimulatorConfig { use_gmr: true, .. }` で各パスを実行後、`ctx.use_gmr == true` を確認
- `ReciprocitySimulatorConfig { use_gmr: false, .. }` で各パスを実行後、`ctx.use_gmr == false` を確認
- テスト対象: `run_kw_real_simulation` / `run_evaluation_simulation` / `run_evaluation_simulation_with_channel` の内部で `ctx.use_gmr` を読み取る

**T2: DeterminismScore がグラフ構造に基づく**
- 高複雑グラフ（ノード数 10+、複数分岐）と低複雑グラフ（ノード数 2、直列）で DeterminismScore が有意に異なることを確認
- 同一グラフに対しては同一スコアが計算される（決定論性）

**T3: phase5_capability_diffusion 戻り値が破棄されない**
- 戻り値が `_diffusions` ではなく `diffusions` として受取られていることを grep 確認
- `run_kw_real_simulation` 内で `diffusions` 変数が使用されていることを確認

**T4: HELP 提案 + 確率的受入が動作する**
- helper が helpee より単純（helper 3 ノード < helpee 5 ノード）でも、提案が行われ確率的に受入されることを確認
- helper が helpee より複雑（helper 8 ノード > helpee 3 ノード）でも提案+受入が行われ、全体コピーでないことを確認（helpee のノード数が helper のノード数未満）
- 受入確率が helper の benevolence/trust に応じて変化することを確認
- 高 benevolence で高確率、低 benevolence で低確率

**T5: DAG 性が維持される**
- 提案+受入後も helpee のグラフが DAG であることを `toposort` で確認

**T6: 空グラフの helper でもパニックしない**
- 空グラフ（0 ノード）の helper に対する提案でクラッシュせず 0 追加を返す

**T7: WORKFLOW_GENERATION_MAX_COMPLEXITY が 4 に増加**
- constants.rs:129 の値が 4 であることを確認
- `generate_new_workflow` が complexity=4 で動作可能であることを確認（エラーにならない）

**T8: try_patch_existing が複数ノード（2〜3）を追加**
- 十分なノード数（8+）のベースグラフに対して `try_patch_existing` が 2 以上の AddNode 操作を含むことを確認
- 操作が DAG 性を維持することを確認

**T9: 既存テスト回帰なし** — `cargo test` 全パス

### 観測テスト

- **観測 1**: GMR 有効時の全 HELP 成功に対する GMR 発火率と追加ノード数分布
- **観測 2**: HELP 提案+受入の受入率（benevolence/trust との相関）
- **観測 3**: 出生時ノード数分布（最小・最大・平均）。tick 経過に伴う上昇傾向
- **観測 4**: 長期シミュレーション（200 tick）でのノード数推移。従来の「15 ノード頭打ち」が解消されていること

## 計装方法・観測対象

### 計装方法

- `phase5_capability_diffusion` 内に `println!` 計装：
  - `[PHASE5] helper_node_count=N helpee_node_count=N accepted=true/false proposal_nodes=N`
  - `[PHASE5] accept_probability=0.XX (benevolence=0.XX trust=0.XX)`
- `try_gmr_diffusion` 内に追記：
  - `[GMR] det_score=0.XXXX (computed_from=graph_structure) added=N`
- `generate_workflow_for_child` 内：
  - `[BIRTH] tick=N outcome=XXX nodes=N`
- 固定シード `StdRng::seed_from_u64(12345)` 使用

### 観測対象

- GMR 発火率と 1 イベントあたり追加ノード数（最小・最大・平均）
- HELP 提案受入率と benevolence/trust との相関
- 出生時ノード数の tick 経過分布
- 長期シミュレーションのノード数最大値（目標: 15 超え）

### 較正計画

新たに導入する定数：
- `HELP_PROPOSAL_ACCEPT_BASE: f64 = 0.30` — 提案受入確率のベース値（Calibration Candidate）
- `HELP_PROPOSAL_MIN_NODES: usize = 2` — 提案の最小ノード数（Algorithm Constant）
- `HELP_PROPOSAL_MAX_NODES: usize = 4` — 提案の最大ノード数（Algorithm Constant）

変更する定数：
- `WORKFLOW_GENERATION_MAX_COMPLEXITY: 2 → 4`（Algorithm Constant）

目的関数 J(θ): `(mean_node_count_200tick - baseline) / (max_possible - baseline)` — 200 tick 時点の平均ノード数で評価。

## Boy Scout Rule — 翻訳可能性計画

- `_diffusions` → `diffusions` のリネーム（全 3 パス）
- `try_gmr_diffusion` 内の乱数 DeterminismScore TODO コメントを具体的な拡張方法に更新
- `copy_graph_if_more_complex` 関数の削除（代わりに提案モデル関数を新設）。関数名は `propose_subgraph_and_accept` のような動詞句とする
- `ReciprocitySimulatorConfig` のフィールド追加に伴う Default 実装修正
- 新規追加する定数には `(Calibration Candidate)` / `(Algorithm Constant)` の分類コメントを付与

## Acceptance Criteria

- [ ] GMR が全プロダクションシミュレーションパスでデフォルト有効（`use_gmr = true`）になった
- [ ] `ReciprocitySimulatorConfig.use_gmr` で GMR 有効/無効を制御可能
- [ ] DeterminismScore がグラフ構造に基づいて計算される（乱数ではない）
- [ ] `phase5_capability_diffusion` の戻り値が全パスで破棄されていない
- [ ] HELP 提案+確率的受入モデルが実装され、helper が helpee より単純でも提案が行われる
- [ ] 受入確率が helper の benevolence/trust に基づいて計算される
- [ ] LLM による判断への拡張 TODO が記載されている
- [ ] `WORKFLOW_GENERATION_MAX_COMPLEXITY` が 4 に引き上げられた
- [ ] `try_patch_existing` が複数ノード（2〜3）を追加するよう拡張された
- [ ] T1〜T9 の不変条件テストが全通過している
- [ ] 観測テストで、長期シミュレーションで 15 ノードを超える個体が出現することが確認できる

## Notes

### 成果物

- 計画: context/0145-untitled-2/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0145-untitled-2/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0145-untitled-2/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0145-untitled-2/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
