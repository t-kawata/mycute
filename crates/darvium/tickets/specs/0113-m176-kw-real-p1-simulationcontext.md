---
ticket_id: 113
title: M1.76-KW-REAL-P1: SimulationContext 基盤
slug: m176-kw-real-p1-simulationcontext
status: reviewed
created_at: 2026-05-26
updated_at: 2026-05-26
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0113-m176-kw-real-p1-simulationcontext/plan.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0113-m176-kw-real-p1-simulationcontext/observation-20260526-191047.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0113-m176-kw-real-p1-simulationcontext/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0113-m176-kw-real-p1-simulationcontext/review.md
---

# M1.76-KW-REAL-P1: SimulationContext 基盤

## Summary

現行の `SimWorkflowState`（simulation.rs:70-95, flat struct, 13 フィールド, String ID, おもちゃモデル）を、**実際の Darvium 部品** — `MemoizedGraph`（trust.rs:23-36）をラップする `SimulationContext` — で置き換える。これにより KW-REAL シリーズ全チケットの基盤構造を提供し、「シミュレーション用おもちゃデータ」から「本物の Darvium データ構造で駆動する実験装置」への移行の第一歩とする。

## Background

### なぜこのチケットが必要か

KW1〜KW4 の実験は全て `SimWorkflowState`（おもちゃのモデル）上で行われ、J_kw ≈ 0.21 に留まっている（observation-20260526-171302.md）。根本原因は、シミュレーションが Darvium の実際の内部表現（階層 DAG トポロジー）をまったく使用していないことにある。実際の Darvium では個人は `WorkflowGraph = DiGraph<WorkflowNode, EdgeMeta>`（types.rs:91）という複雑な DAG として表現され、MemoizedGraph がそのコンテナとなる。

Darvium の社会加速度理論（Kind World）の検証に必要なのは、おもちゃのモデルではなく、**実際のメカニズムが実データ上で示す創発的パターン**の観測である。本チケットはこの観測を可能にするための基盤構造を提供する。

### 「シミュレーションはツールであって目的ではない」

本シミュレーション基盤は社会加速度理論の数学的検証のための実験装置であり、それ自体が目的ではない。57 機構の監査結果（🟢REAL 37 / 🟡PARTIAL 5 / 🔴MISSING 13）に基づき、以下の 3 原則を厳守する：

1. **存在するものは本物の部品をそのまま使う**: 既存実装を直接呼び出す。コピーもラップも独自再実装も禁止。
2. **存在しないものは abstract 実装とし、将来の置換を保証する**: trait で抽象化し、後日本物の実装ができ次第差し替え可能にする。
3. **理論検証に必要な範囲に限定する**: J_kw の 5 因子乗算結合モデル（RFC §15.9.2: S_viab × S_capa × S_coop × S_effi × S_fair）の 14 下位成分の算出に直接関係するものだけを実装範囲とする。

### 過去観察レポートからの知見

- tickets/context/0112-m176-kw4-kind-world/observation-20260526-171302.md — J_kw ≈ 0.21 に停滞。根本原因として SimWorkflowState の限界が示唆されている。
- tickets/context/0111-m176-kw3/observation-20260526-162259.md — 村間相互作用・知識拡散の計装は完了したが、これも SimWorkflowState 上での観測であり、実際の Darvium DAG トポロジーを考慮していない。

## Scope

### 1. `SimulationContext` 構造体の導入

現在の `SimWorkflowState`（simulation.rs:70-95）を以下の `SimulationContext` で置き換える。`SimWorkflowState` の全 13 フィールドは削除され、代わりに実際の Darvium 型を参照する。

```rust
pub struct SimulationContext<'a> {
    pub memoized_graph: &'a mut MemoizedGraph,
    pub trust_profiles: HashMap<NodeId, TrustProfile>,
    pub village_assignments: HashMap<NodeId, VillageAssignment>,
    pub positions: HashMap<NodeId, SpacePositionEmbedding>,
    pub tick: u64,
    pub rng: StdRng,
    pub help_sessions: Vec<HelpSession>,
}
```

フィールドの設計根拠:
- `memoized_graph`: &mut MemoizedGraph への参照。実体は呼び出し元が所有（シミュレーション内で生存させるため）。
- `trust_profiles: HashMap<NodeId, TrustProfile>`: 各 NodeId に対する TrustProfile。MemoizedGraph 自身も TrustProfile を持つが（trust.rs:29）、個人ごとに独立した信頼プロファイルが必要。
- `village_assignments: HashMap<NodeId, VillageAssignment>`: 各 NodeId の村所属。既存の `build_local_village_radius` 等で設定。
- `positions: HashMap<NodeId, SpacePositionEmbedding>`: 各 NodeId の空間位置。SpacePositionEmbedding は既存（spaceposition.rs:23）。
- `tick`: 現在のシミュレーション tick 数。
- `rng: StdRng`: 固定シード PRNG（seed 12345）。
- `help_sessions: Vec<HelpSession>`: アクティブな HELP セッション一覧。本物の `HelpSession`（help.rs:247-256）をそのまま使用。`SimHelpSession`（simulation.rs:129-146, String ID 使用）は削除し、本物の型に統一する。

### 2. `MemoizedGraph` 操作インターフェース

- **ノード追加（出生）**: `MemoizedGraph.graph`（WorkflowGraph = `DiGraph<WorkflowNode, EdgeMeta>`）に `WorkflowNode::SubWorkflow` を追加。NodeId は usize で、petgraph のノードインデックス（`petgraph::graph::NodeIndex`）と相互変換可能にする。
- **ノード削除（死亡）**: 削除インターフェースのみ定義し、実際の GC 制御（状態遷移を含む）は P5 で実装する。本チケットでは `remove_node(node_id: NodeId) -> Result<(), DarviumError>` を提供。
- **人口取得**: `memoized_graph.graph.node_count()` で人口を取得。

### 3. 位置分解 `decompose_position`

`spaceposition.rs` に未実装の `decompose_position`（RFC §41B-2）を完成させる。位置ベクトル `[f32; 3]` を受け取り、各次元成分（空間的・時間的・認知的）に分解する。既存の `update_space_position`（spaceposition.rs:108）および `l2_distance`（spaceposition.rs:144）はそのまま流用。

```rust
/// 3 次元位置ベクトルを各成分に分解する (RFC §41B-2)。
pub fn decompose_position(pos: [f32; 3]) -> (f32, f32, f32) {
    (pos[0], pos[1], pos[2])
}
```

### 4. ノード ID 生成

出生時に一意の `NodeId`（= usize, types.rs:13）を生成する関数。`SimulationContext` に `fn generate_node_id(&mut self) -> NodeId` として実装（現在の最大ノードインデックス + 1 を返す）。

## Non-scope

- **GC 5 状態機械の完全実装**: P5 で行う。本チケットでは削除インターフェースのみ定義。
- **信頼継承・評判継承**: P5 で行う。
- **二軸時間管理・BlendedFreshness**: P5 で行う。
- **LifecycleScore 構造体**: P5 で行う。
- **6 フェーズ tick ループ**: P4 で行う。
- **GMR 抽象化（AG-01〜AG-07, DeterminismScore 等）**: P2 で行う。
- **ワークフロー実行抽象化（compile_to_steps, ErrorMode 等）**: P3 で行う。
- **J_kw 計装インターフェース更新**: P6 で行う。
- **既存 `SimWorkflowState` を使用する全 kind_world.rs 関数の書き換え**: P6 または KW4 で段階的に行う。
- **`ReciprocitySimulatorConfig` / `ReciprocitySimulationResult` の削除**: シミュレーションランナー全体の置き換えは P4 以降で行う。

## Investigation

### 物理的証拠（ソースコード調査 2026-05-26）

#### 現行のおもちゃモデル: `SimWorkflowState`

- **定義**: simulation.rs:70-95 — 13 フィールドの flat struct
  - `id: String` — String ID（本物の Darvium は NodeId = usize）
  - `position: [f32; 3]` — raw array（本物は SpacePositionEmbedding newtype）
  - `experience: u64` / `trust: f32` / `benevolence: f32` / `direct_reciprocity: f32` / `indirect_reciprocity: f32` / `hazard: f32` / `survived: bool` / `is_child: bool` / `initial_benevolence: f32`
  - `reputation: ReputationProfile` — このフィールドのみ本物の型を使用
- **使用箇所**: simulation.rs で約 30 箇所、kind_world.rs で約 30 箇所（関数 signature の `&[SimWorkflowState]` として）→ **合計 60+ 箇所**

#### 本物の Darvium 個人表現: `MemoizedGraph`

- **定義**: trust.rs:23-36 — WorkflowGraph をラップする struct
  - `id: String` — グラフ識別子
  - `graph: WorkflowGraph` — DiGraph<WorkflowNode, EdgeMeta>（types.rs:91）
  - `trust: TrustProfile` — 信頼プロファイル
  - `version: u64` — CAS 用楽観的バージョンカウンタ
  - `cache_invalidated: bool` — キャッシュ無効化フラグ
  - `task_embedding: Vec<f32>` — タスク埋め込みベクトル
- **WorkflowGraph** = `DiGraph<WorkflowNode, EdgeMeta>`（types.rs:91）
- **WorkflowNode** は 3 variant（types.rs:53-68）:
  - `AgentStep { agent, prompt_template, inputs, output_var }`
  - `SubWorkflow { workflow_id, input_mapping, output_var }`
  - `Placeholder`

#### 位置関連

- `SpacePositionEmbedding`: spaceposition.rs:23 — `Option<[f32; 3]>` の newtype
- `SpacePositionEmbedding::unknown()`: 局所性不明を表現
- `update_space_position`: spaceposition.rs:108 — 指数平滑化による位置更新
- `l2_distance`: spaceposition.rs:144 — ユークリッド距離計算
- `is_position_equal`: spaceposition.rs:152 — 閾値ベース位置一致判定
- **`decompose_position`: 未実装** — spaceposition.rs に関数定義が存在しない。RFC §41B-2 に記載された位置分解関数は未実装である。

#### HELP セッション

- **本物**: `HelpSession`（help.rs:247-256） — `help_id: String`, `from_workflow: WorkflowGraphId`, `to_workflow: WorkflowGraphId`, `current_state: HelpState`
- **おもちゃ**: `SimHelpSession`（simulation.rs:129-146） — String ID 使用、本物と非互換
- **本チケットでは**: `SimHelpSession` を削除し、`SimulationContext` 内で本物の `HelpSession` を `Vec<HelpSession>` として保持

#### ノード ID

- `NodeId = usize`（types.rs:13） — petgraph のノードインデックスと互換（petgraph graph のノード追加時に自動採番）

### 参照観察レポート

- tickets/context/0112-m176-kw4-kind-world/observation-20260526-171302.md — KW4 較正実験、J_kw ≈ 0.21 で停滞。SimWorkflowState の限界を示唆。
- tickets/context/0111-m176-kw3/observation-20260526-162259.md — 村間相互作用計装完了。しかしこれも SimWorkflowState 上での観測。

### 設計上の注意点

`SimulationContext` は `&'a mut MemoizedGraph` を参照として保持する。これはシミュレーションの各 tick 間で MemoizedGraph の生存を保証しつつ、借用規則に従うための設計である。`SimulationContext` のライフタイムはシミュレーション実行関数のスコープに制限される。

`HashMap<NodeId, TrustProfile>` を使用する理由：`MemoizedGraph` の `trust: TrustProfile` はグラフ全体の信頼値であり、個人ごとの独立した信頼プロファイルは格納されていない。各個人（WorkflowGraph 内の各ノード）には個別の TrustProfile が必要であるため、SimulationContext 側で管理する。

## Test Plan

以下は `mod tests` ブロックに追加するテストコード。全テストは固定シード `StdRng::seed_from_u64(12345)` を使用する。

### TC1: SimulationContext 生成と初期ノード数確認
- `SimulationContext::new(memoized_graph, config, rng)` で正しく生成され、`memoized_graph.graph.node_count()` が初期設定値と一致すること
- 初期全ノードの positions が空でないこと
- help_sessions が空であること

### TC2: ノード追加（出生）
- `add_person()` を呼び出すと `MemoizedGraph.graph` に新しい `WorkflowNode::SubWorkflow` が追加されること
- 追加前後で `node_count()` が 1 増加すること
- 追加後に `positions` に新しいエントリが存在すること
- 追加後に `trust_profiles` に新しいエントリが存在すること

### TC3: ノード削除インターフェース（GC は P5 範囲）
- `remove_node(node_id)` を呼び出すと `MemoizedGraph.graph` から指定ノードが削除されること
- 削除後に `positions` から該当エントリが削除されること
- 削除後に `trust_profiles` から該当エントリが削除されること
- 存在しない `NodeId` に対してエラーを返すこと

### TC4: `decompose_position` の実装確認
- 位置ベクトル `[1.0, 2.0, 3.0]` が `(1.0, 2.0, 3.0)` に分解されること（各次元正しく抽出）
- 全成分が 0 のベクトルも正しく分解されること（境界値）
- 負値成分も含むベクトルが正しく分解されること

### TC5: `NodeId` 生成の一意性
- `generate_node_id()` を連続 100 回呼び出し、全 ID が一意であること
- 既存ノードの NodeId と重複しないこと

### TC6: 後方互換性 — 既存テストの PASS 維持
- （期待動作）既存の `KindWorldMetricsInput` / `EcosystemGrowthObserver` / `VillageInteractionObserver` のテストが本変更後も全 PASS すること
- （注意）現行の kind_world.rs のテスト群は `&[SimWorkflowState]` を引数に取る関数をテストしている。本チケットではこれらの関数のシグネチャを変更しないため、既存テストはそのまま PASS するはず。

### TC7: `HelpSession` の型統一確認
- `SimulationContext.help_sessions` に本物の `HelpSession`（help.rs:247）が格納可能であること
- 少なくとも 1 件の `HelpSession` を追加し、フィールドアクセス可能であること

### TC8: 空の SimulationContext 境界値テスト
- ノード数 0 の `SimulationContext` で `node_count()` が 0 を返すこと
- 空のコンテキストで `add_person()` が正常動作すること（1 人目追加）

## 計装方法・観測対象

### 計装方法

- `test_simulation_context_instrumentation` 観測テストを `#[test]` + `println!` で実装
- 出力形式: CSV（SimulationContext 生成時の初期ノード数・初期位置分布）+ JSON（位置分解の各次元値）
- 固定シード: `StdRng::seed_from_u64(12345)`
- 実行コマンド: `cargo test -- --nocapture`

### 観測対象

- **SimulationContext 生成統計**: 初期ノード数、各ノードの初期位置（3 次元）、TrustProfile 初期値
- **ノード追加・削除操作ログ**: 操作種別（add/remove）, NodeId, tick, 操作前後の人口
- **位置分解の各次元値**: 3 次元それぞれの平均・分散（tick ごとに JSON 出力）
- **置き換え漏れカバレッジ**: `SimWorkflowState` を使用する全箇所を grep で洗い出し、本チケットで置き換えた箇所と後続チケット（P4/P6/KW4）で置き換える箇所を分別

### 較正計画

本チケット（P1）は較正対象の定数を持たない。SimulationContext の動作確認が目的であり、パラメータ調整は行わない。

## Boy Scout Rule — 翻訳可能性計画

### `simulation.rs` — `SimWorkflowState` の削除と責務整理

- `SimWorkflowState`（simulation.rs:70-95）を削除し、`SimulationContext` で置き換え
- `SimHelpSession`（simulation.rs:129-146）を削除し、本物の `HelpSession`（help.rs:247-256）に統一
- `SimMission`（simulation.rs:98-106）はこの段階では削除しない（P4 で整理）
- `run_simulation`（simulation.rs:240-）は P4 で書き換えるため、本チケットでは触れない

### 改善点

1. **関数名を動詞句に**: 新規関数は全て動詞始まり（`add_person`, `remove_node`, `generate_node_id`）
2. **ハードコード値排除**: 定数化（`DEFAULT_SEED: u64 = 12345` 等を constants.rs に追加要検討）
3. **エラー握りつぶし排除**: 新規関数は全て `Result` を返し、エラーを伝播
4. **責務分離**: `SimulationContext` は状態保持のみ。操作ロジックはメソッドとして Context に実装。複雑な演算（GC, life cycle）は行わない — P5 の責務範囲に明確に分離

### 関数名ベースの翻訳可能性チェック

実装前に以下を確認する:
- 新規関数がすべて動詞句で始まっていること（`add_*`, `remove_*`, `generate_*`, `get_*`）
- 引数名がドメイン概念を表していること（`person_count` ではなく `initial_population_size` 等）
- 一時変数に `x`, `tmp`, `data` 等の汎用名を使用していないこと

## Acceptance Criteria

- [ ] `SimulationContext` が正しく生成され、初期ノード数が指定通りであること（TC1）
- [ ] ノード追加（出生）が `MemoizedGraph` に新しい `WorkflowNode::SubWorkflow` を追加すること（TC2）
- [ ] ノード削除が `MemoizedGraph` から指定ノードを削除すること（TC3）
- [ ] `decompose_position`（新規実装）が正しく各次元に分解されること（TC4）
- [ ] `NodeId` 生成が毎回一意の ID を返すこと（TC5）
- [ ] 既存の KindWorldMetricsInput / EcosystemGrowthObserver / VillageInteractionObserver のテストが本変更後も全 PASS すること（TC6）
- [ ] 本物の `HelpSession` が `SimulationContext.help_sessions` に格納可能であること（TC7）
- [ ] `cargo test` が全 PASS すること
- [ ] 翻訳可能性チェック（関数名・変数名・ドメイン概念）を通過していること

## Notes

- plan_path: /plan-ticket が plan.md 作成後に frontmatter に更新する
- implementation_path: /start-ticket が implementation.md 作成後に frontmatter に更新する
- review_report_path: /review-ticket が review.md 作成後に frontmatter に更新する
- observation_report_path: /start-ticket が observation-YYYYMMDD-HHmmss.md 作成後に frontmatter に最新パスを更新する

### 依存関係

- **必須**: なし（本チケットが KW-REAL シリーズの最初の実装単位）
- **ブロック**: P5（ライフサイクル・成熟機構）、P4（6 フェーズループ）、P2（GMR 抽象化）、P3（実行抽象化）、P6（計装更新）

### 成果物

- 計画: context/0113-m176-kw-real-p1-simulationcontext/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0113-m176-kw-real-p1-simulationcontext/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0113-m176-kw-real-p1-simulationcontext/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0113-m176-kw-real-p1-simulationcontext/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
