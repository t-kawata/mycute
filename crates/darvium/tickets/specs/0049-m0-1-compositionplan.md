---
ticket_id: 49
title: M0-1: CompositionPlan データ整合性及び変数スコープ静的バリデータの実装
slug: m0-1-compositionplan
status: done
created_at: 2026-05-23
updated_at: 2026-05-23
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0049-m0-1-compositionplan/observation-20260523-132423.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0049-m0-1-compositionplan/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0049-m0-1-compositionplan/review.md
---
# M0-1: CompositionPlan データ整合性及び変数スコープ静的バリデータの実装

## Summary

`CompositionPlan` 構造体を RFC §13.3 に準拠した完全なフィールドを持つ型に拡張し、組成エッジの変数スコープ整合性（RFC §6.4 V-03 / V-04）を検証する静的バリデータを実装する。さらに v2.3 の frontier-based parallel execution を前提として、並列 frontier 上での未解決変数と scope leakage 検出も可能にする。

## Background

### 対象不変条件 / 規範
- **RFC §13.3**: `CompositionPlan` 構造定義（component_graph_ids, composition_edges, expected_inputs, expected_output, confidence）
- **RFC §6.4 V-03**: `DataFlow.from_var` は送信ノードの `output_var` と一致すること
- **RFC §6.4 V-04**: `DataFlow.to_var` は受信ノードの `inputs` に含まれること

現在の `CompositionPlan` は空のユニット構造体であり、上記の不変条件を一切検証できていない。組成プランが実際の探索で利用される前に、不正な変数参照・スコープ漏洩を静的に検出する必要がある。

### 過去の観察レポートからの知見

- `tickets/context/0048-m1-4-hitl-pending/observation-20260523-130402.md` — HITL 回復ループ実装。変数スコープとは無関係だが、計装パターン（println! + --nocapture 構造化出力）の参考になる。
- `tickets/context/0047-m-05-4-notifier/observation-20260523-123238.md` — HumanChannel トレイト定義。型定義 + テストパターンの参考になる（Fake 駆動、Serde ラウンドトリップ）。
- `tickets/context/0046-ag-06-ag-07/observation-20260523-103051.md` — バリデーションハードゲートの観測テスト。ゲート通過確率の統計的検証手法の参考になる。

## Scope

### 実装スコープ

1. **`VarDecl` 構造体の定義** (`src/types.rs`)
   - RFC §6.1 に従い `name: String, required: bool` を定義
   - `#[derive(Debug, Clone, PartialEq)]`

2. **`EdgeMeta` の拡張** (`src/types.rs`)
   - 現在のユニット構造体から RFC §6.2 準拠の enum に拡張（少なくとも `DependsOn` / `DataFlow { from_var: String, to_var: String }` を定義）
   - 組成プランの検証に必要なバリアントのみ実装し、他は `// TODO` または部分定義とする

3. **`CompositionPlan` 構造体の拡張** (`src/types.rs`)
   - RFC §13.3 に準拠：`component_graph_ids: Vec<WorkflowGraphId>`, `composition_edges: Vec<(NodeIndex, NodeIndex, EdgeMeta)>`, `expected_inputs: Vec<VarDecl>`, `expected_output: VarDecl`, `confidence: f32`
   - `NodeId` は `NodeIndex`（petgraph）を使用

4. **静的バリデータ関数** (`src/composition.rs` または `src/validate.rs` の新設、あるいは `src/types.rs` 内)
   - `validate_composition_plan(plan: &CompositionPlan, graphs: &HashMap<WorkflowGraphId, WorkflowGraph>) -> Result<(), DarviumError>`
   - 各 `composition_edges` の送信元グラフのノードから `output_var` を取得し、`DataFlow.from_var` と一致するか検証 (V-03)
   - 各 `composition_edges` の送信先グラフのノードから `inputs` を取得し、`DataFlow.to_var` が含まれるか検証 (V-04)
   - 並列 frontier（互いに独立なノード）が同名変数を扱う scope leakage の検出
   - 不正時は `DarviumError::VariableScopeViolation(String)` を返却

### 非スコープ
- 実グラフストア（GraphStore/MetadataStore）との結合は含まない。検証はメモリ上のグラフマップに対して行う。
- Layer 1 コンパイラ（`compile_to_steps`）の変更は含まない。
- 探索状態機械（SearchState / SearchStep）の変更は含まない。
- SubWorkflow 展開時の名前空間隔離（V-05 / V-06）は含まない。

## Investigation

### ソースコード調査結果（2026-05-23）

#### 証拠①：`CompositionPlan` は空のユニット構造体
- **ファイル**: `src/types.rs:4242-4245`
- **現状**: `#[derive(Debug, Clone, PartialEq)] pub struct CompositionPlan;`
- **RFC §13.3 要求**: `component_graph_ids: Vec<WorkflowGraphId>`, `composition_edges: Vec<(NodeId, NodeId, EdgeMeta)>`, `expected_inputs: Vec<VarDecl>`, `expected_output: VarDecl`, `confidence: f32`
- **乖離**: 全フィールドが未定義

#### 証拠②：`EdgeMeta` は空のユニット構造体
- **ファイル**: `src/types.rs:26-28`
- **現状**: `#[derive(Debug, Clone)] pub struct EdgeMeta;`
- **RFC §6.2 要求**: enum バリアント（`DependsOn`, `DataFlow { from_var, to_var }`, `Conditional`, `FanOut`, `Collect`）
- **乖離**: 構造体が enum になっておらず、`DataFlow.from_var/to_var` が存在しない（V-03/V-04 検証に必須）

#### 証拠③：`VarDecl` が未定義
- **ファイル**: `src/types.rs` 全体（grep 結果: 該当なし）
- **RFC §6.1 要求**: `struct VarDecl { name: String, required: bool }`
- **現状**: 存在しない。ワークフローノードの `inputs` / `output_var` が文字列として表現できない。

#### 証拠④：`DarviumError::VariableScopeViolation(String)` は既存
- **ファイル**: `src/error.rs:18-19`
- **現状**: エラー型は既に定義済み。実装側で利用可能。

#### 証拠⑤：ランダム有向グラフ生成のアンサンブル検証は未実装
- **現状**: 既存テストで Erdős–Rényi ランダムグラフを生成する仕組みはない。
- **必要**: `StdRng` を用いたランダム有向グラフ生成 + 未解決変数注入 + バリデータ投入パイプライン。

#### 証拠⑥：`WorkflowNode` のノード種別が未実装
- **ファイル**: `src/types.rs:23-24`
- **現状**: `pub struct WorkflowNode;`（空のユニット）
- **RFC §6.1 要求**: enum に `AgentStep { agent, prompt_template, inputs: Vec<VarDecl>, output_var, ... }` 等が必要
- **影響**: 組成エッジの送信元ノードの `output_var` や送信先ノードの `inputs` を取得するには、WorkflowNode の拡張が必要。

## Test Plan

### 単体テスト（T1〜T12）

#### 基本構造テスト

| ID | 内容 | 種別 |
|----|------|------|
| T1 | `VarDecl` 構造体の正常構築 | 正常系 |
| T2 | `VarDecl` の `name` 空文字列ケース（許容判断） | 境界値 |
| T3 | `EdgeMeta::DataFlow` の正常構築とフィールドアクセス | 正常系 |
| T4 | `CompositionPlan` 構造体の全フィールド正常構築 | 正常系 |

#### V-03 / V-04 検証テスト

| ID | 内容 | 種別 |
|----|------|------|
| T5 | 送信元ノードの `output_var` と `DataFlow.from_var` が一致: 正常系（V-03 通過） | 正常系 |
| T6 | 送信元ノードの `output_var` と `DataFlow.from_var` が不一致: `VariableScopeViolation` 検出（V-03 遮断） | 異常系 |
| T7 | 送信先ノードの `inputs` に `DataFlow.to_var` が含まれる: 正常系（V-04 通過） | 正常系 |
| T8 | 送信先ノードの `inputs` に `DataFlow.to_var` が非含: `VariableScopeViolation` 検出（V-04 遮断） | 異常系 |
| T9 | 複数エッジを持つ組成プランで全ての V-03/V-04 が通過 | 正常系 |
| T10 | 一部エッジのみ不正な組成プランで不正エッジのみエラー報告 | 異常系 |

#### 並列 Frontier 検証テスト

| ID | 内容 | 種別 |
|----|------|------|
| T11 | 互いに独立な parallel-ready ノードが同名変数を扱うケースで scope leakage 検出 | 異常系 |
| T12 | 互いに独立な parallel-ready ノードが別名変数を扱うケースで正常通過 | 正常系 |

### 観測テスト（OTS-1〜OTS-3）

| ID | 内容 | サンプルサイズ |
|----|------|----------------|
| OTS-1 | ランダム有向グラフ（Erdős–Rényi）アンサンブル n=10,000 で未解決変数注入時の捕捉率が 100% であること | n=10,000 |
| OTS-2 | ノード数 n ∈ [10, 512] での検証ステップ数スケーリング C_validate ~ n^γ の実測 | n=10..512 各100アンサンブル |
| OTS-3 | 最大ノード数 (512) における最悪計算ステップ数の有界性実証 | n=512, アンサンブル数=1,000 |

## 計装方法・観測対象

### 計装方法

1. **観測テスト OTS-1**:
   - `StdRng::seed_from_u64(12345)` で固定シード
   - Erdős–Rényi モデル: ノード数 n ∈ [10, MAX_GRAPH_NODES]、エッジ生成確率 p ∈ [0.0, 1.0]
   - 各グラフに確率的に未解決変数を埋め込み（一部エッジの `from_var`/`to_var` をランダム改変）
   - バリデータが検出した不正数をカウントし、注入した不正数と比較 → 捕捉率を計測
   - 出力: `println!("OTS-1: captured={}/{} injected, capture_rate={:.4}", ...)`

2. **観測テスト OTS-2 / OTS-3**:
   - 検証の際の内部命令数（エッジ走査 + ルックアップ回数）をカウント
   - n の関数としてプロット
   - 出力: 構造化JSON `println!("{{\"n\":{}, \"steps\":{}, \"gamma\":{:.4}}}", ...)`

### 観測対象

| 統計量 | 説明 | 期待値 |
|--------|------|--------|
| 捕捉率 | 注入した未解決変数をバリデータが検出した割合 | = 1.0 (assert) |
| スケーリング指数 γ | C_validate ~ n^γ の指数 | γ ≈ 1.0 (線形) |
| 最大計算ステップ数 | n=512 における最悪値 | < 10,000 (有界) |

### 較正計画

該当なし — 本チケットはパラメータ較正を伴わない純粋な静的検証ロジックの実装である。定数変更が発生する場合は constants.rs に Safety Invariant として追記する。

## Boy Scout Rule — 翻訳可能性計画

### 新規コード

- 関数名は動詞句: `validate_composition_plan`, `check_var_scope_v03`, `check_var_scope_v04`, `detect_frontier_leakage`
- エラー握りつぶし禁止: バリデーションエラーは必ず `Result` で伝播
- 一関数一責務: バリデータ関数は V-03、V-04、frontier leakage で分割
- ハードコード値は名前付き定数

### 既存コードの改善（Boy Scout Rule）

- `src/types.rs:4245` `pub struct CompositionPlan;` → RFC §13.3 準拠の完全構造体に変更（破壊的変更にはならない：現状は誰もフィールドアクセスしていない）
- `src/types.rs:28` `pub struct EdgeMeta;` → RFC §6.2 の enum に拡張（同上、破壊的影響なし）
- `src/types.rs:24` `pub struct WorkflowNode;` → RFC §6.1 の enum に拡張（同上）

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

- 計画: context/0049-m0-1-compositionplan/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0049-m0-1-compositionplan/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0049-m0-1-compositionplan/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0049-m0-1-compositionplan/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
