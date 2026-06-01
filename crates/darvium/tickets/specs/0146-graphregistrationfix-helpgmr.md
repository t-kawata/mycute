---
ticket_id: 146
title: GraphStore一元化 + HELP/GMR/自己抽象化の個体登録完全化
slug: graphstore-unification-and-population-registration
status: reviewed
created_at: 2026-05-29
updated_at: 2026-05-29
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0146-graphregistrationfix-helpgmr/plan.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0146-graphregistrationfix-helpgmr/implementation.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0146-graphregistrationfix-helpgmr/observation-20260529-121622.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0146-graphregistrationfix-helpgmr/review.md
---

# GraphStore一元化 + HELP/GMR/自己抽象化の個体登録完全化

## Summary

Darvium シミュレーションには2つの根本問題がある。

**問題A — アーキテクチャ違反**: GraphStore トレイトがグラフデータベースの抽象化として設計されているにもかかわらず、WorkflowRegistry が独自の `HashMap` で全グラフを保持し、GraphStore を経由していない。このため2つの独立ストアが存在し、`compute_all_node_count` が SubWorkflow を解決できずノード数が 0 と計上される。

**問題B — 個体登録欠落**: RFC §12.4 の「パッチ結果を新規独立エントリとして登録」が「グラフの保存」としてのみ解釈され、「新個体として population に追加する（＝進化・出生）」が実装されていない。HELP/GMR/自己抽象化の各経路で、以下の誤った動作が起きている：

| 機構 | 現状の動作 | 正しい動作 |
|------|-----------|-----------|
| HELP | `helpee.graph = new_graph`（上書き） | 元の helpee は維持 + 新個体として追加 |
| GMR | `helpee.graph = new_graph`（上書き） | 元のメンバーは維持 + 新個体として追加 |
| 自己抽象化 | SubWorkflow を registry に登録するのみ | SubWorkflow を新個体として population に追加 |

本チケットでは **問題A（基盤）と問題B（個体登録）の両方を修正** し、ノード数過小計上と個体数過小評価を同時に解決する。

## Background

### RFC §12.4 patch_and_register の正しい解釈

RFC `patch_and_register`（Darvium-RFC-0001-Unified-v2.3-final.md:4365-4384）は以下を定義する：

```rust
async fn patch_and_register(
    cache: &WorkflowCache,
    pair: &RepositoryPair,
    gold_id: WorkflowGraphId,
    patch: &GraphPatch,
    parent_trust: &TrustProfile,
    patch_conf: f32,
) -> Result<WorkflowGraphId, CacheError> {
    let new_graph = apply_patch_atomic(&gold_graph, patch)?;
    let new_id = WorkflowGraphId::new_v4();
    // ↑ 新規 ID で登録 = 独立したエンティティとして扱う
}
```

この `new_id` は「パッチ結果が元のグラフとは独立した新規エンティティであること」を意味する。シミュレーションにおける正しい解釈は：

1. **グラフレベルの独立**: WorkflowRegistry + GraphStore に新規 ID で保存する
2. **個体レベルの独立**: `ctx.add_person()` で新個体として population に追加する

現状は1も2も実装されていない。

### 2つの独立した問題の関係

```
問題A: アーキテクチャ違反
       WorkflowRegistry が GraphStore を経由しない
       → compute_all_node_count が SubWorkflow 解決に失敗
       → .unwrap_or(0) → ノード数 0

問題B: 個体登録欠落
       HELP/GMR → 上書き（add_person なし）
       自己抽象化 → population 追加なし
       → 個体数が過小評価される
       → 進化（出生）がシミュレーションに反映されない

両者は独立した問題であり、両方の修正が必要。
問題Aの修正だけでは問題Bは解決しない。
```

### 現状のアーキテクチャ違反（問題A）

```
WorkflowRegistry (独自の HashMap を持つ「第二のストア」)
  │
  ├── register_graph_only() → self.graphs.insert(id, memoized)
  └── resolve() → self.graphs.get(id)

InMemoryGraphStore (GraphStore 実装＝本来の単一正典)
  │
  ├── store_workflow_graph_with_id() → self.graphs.insert(id, graph)
  └── load_workflow_graph() → self.graphs.get(id)

  ✗ 両者は完全に独立した HashMap（同期機構なし）
  ✗ WorkflowRegistry の書き込みが GraphStore に届かない
  ✗ compute_all_node_count は GraphStore 経由で SubWorkflow 解決を試みるが空
```

### あるべきアーキテクチャ（問題A 修正後）

```
WorkflowRegistry (論理レイヤー: 検索・管理・キャッシュ)
  │
  ├── register_graph_only()
  │   ├── GraphStore に書き込む（正典）
  │   └── キャッシュに追加（高速 lookup）
  │
  └── resolve()
      ├── キャッシュヒット → 即時返却
      └── キャッシュミス → GraphStore から復元

GraphStore トレイト (単一正典層)
  │
  ├── InMemoryGraphStore (シミュレーション用)
  └── LadybugGraphStore (本物実装用)
```

### 現在の個体登録の欠落（問題B）

```rust
// simulation.rs:3019-3021 — HELP Phase 5（誤り）
match apply_patch_atomic(&ctx.population[helpee_id].graph, &patch) {
    Ok(new_graph) => {
        ctx.population[helpee_id].graph = new_graph;
        // ↑ 上書き。元の helpee のグラフが失われる
        // ↑ 新個体の追加がない
    }
}

// simulation.rs:3237-3239 — GMR（誤り）
match apply_patch_atomic(helpee_graph, &patch) {
    Ok(new_graph) => {
        ctx.population[helpee_id].graph = new_graph;
        // ↑ 上書き。個体登録なし
    }
}

// self_refinement.rs:141, 213-249 — 自己抽象化（誤り）
// SubWorkflow を registry に登録するが population への追加がない
// → サブグラフが「専門個体」として生きていない
```

RFC §12.4 のエンティティ独立性は「個体レベルの新規登録」まで含意する。HELP を受け入れた helpee は「編集を受ける」のではなく、「マージされた新たな個体が誕生する」。元の helpee はそのまま残り、新しい個体が実務を担う。これは進化であり出生である。

個体数爆発の制御は GC メカニズムの責務であり、人工的な抑制を入れてはならない。

## Scope

### 対応範囲

#### 問題A: GraphStore 一元化（基盤アーキテクチャ修正）

1. **GraphStore トレイトの拡張**: `store_memoized_graph` / `load_memoized_graph` を追加。`Send + Sync` 境界を追加。

2. **InMemoryGraphStore の Sync 化**: `RefCell` → `Mutex`、`Cell` → `AtomicU64`。新規メソッドを実装。

3. **WorkflowRegistry の再実装**: `Arc<dyn GraphStore>` を保持し、全書き込み操作を GraphStore に委譲。内部 HashMap はキャッシュに再位置づけ。

4. **SimulationContext への統合**: `ctx.registry` が GraphStore を内包。`build_node_created_event` / `build_clock_advanced_event` の store 引数を `ctx.registry.store_ref()` から取得。

#### 問題B: HELP/GMR/自己抽象化の個体登録

5. **HELP `propose_subgraph_and_accept` の修正**:
   - `apply_patch_atomic` 成功後、`ctx.registry.register_graph_only()` でグラフを登録
   - `ctx.add_person()` で新個体を population に追加
   - 新個体の `graph` にパッチ結果をセット
   - 元の helpee のグラフは変更しない
   - 信頼・評判・位置などの属性を helpee から継承する設計とする

6. **GMR `try_gmr_diffusion` の修正**:
   - 5 と同様に `register_graph_only` + `add_person`
   - 元の個体は変更しない

7. **自己抽象化 `register_abstracted_subworkflow` + `run_self_refinement_round` の修正**:
   - 抽出された SubWorkflow グラフを `ctx.add_person()` で新個体として population に追加
   - SubWorkflow ノードによる置換（`replace_with_subworkflow`）は親グラフに対して行う（これは抽象化として正しい）
   - 新個体（SubWorkflow）は独立した専門個体として振る舞う

### 非対応範囲

- LadybugGraphStore の実装（本チケットは InMemoryGraphStore のみ）
- フロントエンド（script.js）の変更
- RFC 文書自体の改訂
- 個体数爆発の人工的抑制（GC メカニズムに委ねる）
- `compute_all_node_count` のアルゴリズム変更
- Phase 5 以外の HELP/GMR フロー

## Investigation

### 発見A1: GraphStore が Sync ではない（store/graph_store.rs:84-92）

**ファイル**: `src/store/graph_store.rs` 行 84-92

```rust
pub struct InMemoryGraphStore {
    graphs: RefCell<HashMap<GraphId, WorkflowGraph>>,
    graph_id_counter: Cell<u64>,
    embeddings: RefCell<HashMap<String, Vec<f32>>>,
    // ...
}
```

- `RefCell` → `!Sync` → `Arc<InMemoryGraphStore>` 不可能
- Box 経由でもスレッド共有不可

### 発見A2: GraphStore に MemoizedGraph 操作がない（store/graph_store.rs:17-72）

```rust
pub trait GraphStore {
    fn store_workflow_graph(&self, graph: &WorkflowGraph) -> Result<GraphId, DarviumError>;
    fn load_workflow_graph(&self, graph_id: &GraphId) -> Result<WorkflowGraph, DarviumError>;
    // ... ❌ store_memoized_graph / load_memoized_graph が存在しない
}
```

- WorkflowRegistry が store に委譲しようとしても、`MemoizedGraph` 全体を扱う手段がない
- そのため registry が独自 HashMap を持つインセンティブになっている

### 発見A3: WorkflowRegistry のコメントと実装の矛盾（workflow_registry.rs:7-8）

```rust
// 永続化は GraphStore (LadybugDB / InMemoryGraphStore) が責務を持つ。
// WorkflowRegistry はドメイン層の registry であり、高速 lookup と全件走査を目的とする。
```

コメントは「永続化は GraphStore の責務」と書いているが、実装は独自 HashMap で全データを保持し GraphStore を一切使わない。Everything as Code ポリシー違反。

### 発見A4: WorkflowRegistry が store を持たない（workflow_registry.rs:24-29）

```rust
pub struct WorkflowRegistry {
    graphs: HashMap<WorkflowGraphId, MemoizedGraph>,
    id_counter: u64,
    // ❌ store フィールドなし
}
```

- `register_graph_only`, `register`, `register_with_id` の全メソッドが HashMap にのみ書き込む
- GraphStore に一切届かない

### 発見A5: サーバーパスの InMemoryGraphStore が空（simulation.rs:1858）

```rust
let graph_store = InMemoryGraphStore::new(); // 空のまま使い続けられる
```

- シミュレーション中、誰も store にグラフを書き込まない
- 発見A4 の結果、全 SubWorkflow 参照が NotFound

### 発見B1: HELP — 上書きによる個体消滅（simulation.rs:3019-3021）

```rust
match apply_patch_atomic(&ctx.population[helpee_id].graph, &patch) {
    Ok(new_graph) => {
        ctx.population[helpee_id].graph = new_graph;
        // ↑ 上書き: 元の helpee のグラフが失われる
        // ↑ add_person なし: 新個体の出生がない
        // ↑ register_graph_only なし: グラフの検索性がない
    }
}
```

RFC §12.4 が要求する「新規独立エントリ」が満たされていない。

### 発見B2: GMR — 上書きによる個体消滅（simulation.rs:3237-3239）

```rust
match apply_patch_atomic(helpee_graph, &patch) {
    Ok(new_graph) => {
        ctx.population[helpee_id].graph = new_graph;
        // ↑ B1 と同様の上書き。個体登録なし。
    }
}
```

### 発見B3: 自己抽象化 — SubWorkflow の population 未登録（self_refinement.rs:126-249）

```rust
pub fn register_abstracted_subworkflow(
    subgraph: WorkflowGraph,
    registry: &mut WorkflowRegistry,
) -> Result<WorkflowGraphId, DarviumError> {
    let id = registry.register_graph_only(subgraph, &mission);
    // ↑ registry に登録するだけ。population には追加しない
    Ok(id)
}
```

抽出されたサブグラフは独立した専門個体として振る舞うべきだが、`ctx.population` に追加されない。

### 発見B4: compute_all_node_count — Err の握りつぶし（simulation.rs:2027,2096）

```rust
let node_count = compute_all_node_count(&person.graph, store).unwrap_or(0);
```

発見A5 + B3 により SubWorkflow 参照が NotFound → Err → `unwrap_or(0)` → 黙殺。エラーログすら出力されない。

### 問題箇所マップ

| # | ファイル | 行 | 問題 | 問題種別 |
|---|----------|-----|------|---------|
| A1 | store/graph_store.rs | 84-92 | InMemoryGraphStore が RefCell で !Sync | A 基盤 |
| A2 | store/graph_store.rs | 17-72 | GraphStore に MemoizedGraph 操作なし | A 基盤 |
| A3 | workflow_registry.rs | 7-8 | コメントと実装の矛盾 | A 基盤 |
| A4 | workflow_registry.rs | 24-29 | store フィールド欠落 | A 基盤 |
| A5 | simulation.rs | 1858 | InMemoryGraphStore が空 | A 基盤 |
| B1 | simulation.rs | 3019-3021 | HELP: 上書き + add_person なし | B 個体登録 |
| B2 | simulation.rs | 3237-3239 | GMR: 上書き + add_person なし | B 個体登録 |
| B3 | self_refinement.rs | 126-249 | 自己抽象化: population 未登録 | B 個体登録 |
| B4 | simulation.rs | 2027,2096 | .unwrap_or(0) で黙殺 | A+B 両方 |

## Test Plan

### 不変条件テスト（`assert!` / `assert_eq!`）

#### 問題A: GraphStore 一元化

##### T-A1: GraphStore が MemoizedGraph 全体を store/load できる
- `InMemoryGraphStore` に全フィールド設定済み `MemoizedGraph` を `store_memoized_graph` で格納
- `load_memoized_graph` で全フィールドが一致することを確認

##### T-A2: WorkflowRegistry の register が GraphStore に書き込む
- `Arc<InMemoryGraphStore>` を保持する `WorkflowRegistry` を生成
- `register_graph_only(graph, mission)` を呼ぶ
- `registry.store_ref().load_workflow_graph(&id)` が `Ok` を返す
- `registry.store_ref().load_embedding(&id)` が `Ok` を返す

##### T-A3: resolve がキャッシュから即時応答する
- T-A2 の登録後、`resolve(&id)` が `Ok(&MemoizedGraph)` を返す
- キャッシュがヒットしていること（store への問い合わせが発生しないこと）

##### T-A4: 既存 WorkflowRegistry テストが回帰しない
- `cargo test workflow_registry` が全て PASS

#### 問題B: 個体登録

##### T-B1: HELP 成功後に新個体が population に追加される
- `propose_subgraph_and_accept` を成功させる
- `ctx.population_count()` が 1 増加している
- 元の helpee のグラフが変更されていない
- 新個体のグラフが `compute_all_node_count` で正の値を返す
- 新個体の ID が WorkflowRegistry に登録されている

##### T-B2: GMR 成功後に新個体が population に追加される
- `try_gmr_diffusion` 成功後、`ctx.population_count()` が増加
- 元の個体は変更されていない

##### T-B3: 自己抽象化後に SubWorkflow が新個体として population に追加される
- `run_self_refinement_round` 成功後、`ctx.population_count()` が増加
- 新個体のグラフが SubWorkflow として抽出された部分グラフと一致する
- 親のグラフに SubWorkflow 参照が含まれている
- `compute_all_node_count(parent_graph, store)` が正の値を返す

##### T-B4: HELP → 自己抽象化 → GMR の複合サイクルで個体数が正確
- 3サイクル実行後、各サイクルで population が増加している
- 全個体の `compute_all_node_count` が 0 でない
- 個体数の推移が HELP/GMR/自己抽象化イベント数と整合している

##### T-B5: 元の個体が変更されない
- T-B1〜T-B3 の各操作後、操作対象となった個体（helpee, helper, 親）のグラフが操作前と同一であることを確認

##### T-B6: 新個体の属性が元の個体から適切に継承される
- HELP 後の新個体の `position` が helpee の `position` と一致する
- GMR 後の新個体の `village_assignment` が元の個体と一致する
- 自己抽象化後の新個体の `birth_tick` が現在 tick である

### 観測テスト（`println!` + `--nocapture`）

##### O1: 人口推移の観測
- 固定シード（`StdRng::seed_from_u64(12345)`）、population_size=20、max_ticks=100
- CSV 出力: `tick,population_count,alive_count,help_count,gmr_count,abstraction_count,registry_size`
- 人口推移を時系列で観測可能にする

##### O2: 個体別ノード数分布
- O1 と同一設定で各 tick の個体別ノード数ヒストグラム
- CSV 出力: `tick,person_id,node_count,is_new_birth,parent_id`
- 新個体のノード数が正しく計上されていることを確認

##### O3: 統合シミュレーション（フルスタック）
- `run_evaluation_simulation_with_channel` 実行
- 全 Event の payload.node_count が 0 でないことを確認
- フロントエンド描画に十分なデータが出力されることを確認

### 較正対象

本チケットでは新たな較正定数は導入しない。

### 目的関数 J(θ)

- **ノード数正確性**: `compute_all_node_count` が 0 を返す割合 = 0%
- **個体登録完全性**: HELP/GMR/自己抽象化イベント数と population 増加数が一致
- **元個体保全性**: 操作対象となった全個体のグラフ変更前後一致率 = 100%
- **SubWorkflow 解決率**: store 経由の SubWorkflow 解決成功率 = 100%

## 計装方法・観測対象

### 計装方法

- テストを `src/workflow_registry.rs` のテストモジュールおよび `src/simulation.rs` のテストモジュールに追加
- 固定シード `StdRng::seed_from_u64(12345)` を使用
- `println!` + `--nocapture` で構造化 CSV 出力
- HELP/GMR/自己抽象化の各経路に出生イベントの計装プローブを追加

### 観測対象

| 統計量 | 計測方法 | 期待値 |
|--------|---------|--------|
| 個体登録率 | add_person 増加数 / 処理成功回数 | 100% |
| 元個体保全率 | 操作前後のグラフ同一性 | 100% |
| SubWorkflow 解決率 | compute_all_node_count 成功/総数 | 100% |
| ノード数最小値 | 全イベントの node_count 最小値 | > 0 |
| 人口増加トレンド | tick 経過に伴う人口増加 | 単調非減少 |

## 実装手順（建築順序）

### Step 1: GraphStore トレイト拡張（問題A 基盤）

**ファイル**: `src/store/graph_store.rs`

- `Send + Sync` 境界を追加:
  ```rust
  pub trait GraphStore: Send + Sync {
  ```
- `store_memoized_graph(&self, memoized: &MemoizedGraph) -> Result<GraphId, DarviumError>` 追加
- `load_memoized_graph(&self, graph_id: &GraphId) -> Result<MemoizedGraph, DarviumError>` 追加

### Step 2: InMemoryGraphStore の Sync 化（問題A 基盤）

**ファイル**: `src/store/graph_store.rs`

- `RefCell<HashMap<K, V>>` → `Mutex<HashMap<K, V>>`
- `Cell<u64>` → `AtomicU64`
- 全アクセスメソッドで `.lock().unwrap()` を使用
- `store_memoized_graph` / `load_memoized_graph` を実装

### Step 3: WorkflowRegistry の再実装（問題A 核心）

**ファイル**: `src/workflow_registry.rs`

- `store: Option<Arc<dyn GraphStore>>` フィールドを追加
- `WorkflowRegistry::with_store(store: Arc<dyn GraphStore>) -> Self` 追加
- `store_ref(&self) -> Option<&dyn GraphStore>` 追加
- `register_graph_only`: GraphStore に書き込んでからキャッシュに追加
- `register`, `register_with_id`: 同様に GraphStore 委譲
- 嘘のコメント（`workflow_registry.rs:7-8`）を削除し、正しい説明に更新

### Step 4: サーバーパスの統合（問題A 完了）

**ファイル**: `src/simulation.rs`

- `run_evaluation_simulation_with_channel`:
  ```rust
  let graph_store = Arc::new(InMemoryGraphStore::new());
  let mut ctx = SimulationContext::new(rng);
  ctx.use_gmr = config.use_gmr;
  ctx.registry = WorkflowRegistry::with_store(graph_store.clone());
  ```
- イベントビルダー関数の store 引数を `ctx.registry.store_ref().unwrap()` から取得（直接参照またはクローン）

### Step 5: HELP 個体登録（問題B）

**ファイル**: `src/simulation.rs` 行 3019-3021

`propose_subgraph_and_accept` の成功パスを以下のように変更：

```rust
Ok(new_graph) => {
    // 1. WorkflowRegistry + GraphStore に登録（Step 3 により自動的に GraphStore へ）
    let _new_id = ctx.registry.register_graph_only(
        new_graph.clone(),
        &format!("help-merge-{}-{}", helper_id, helpee_id),
    );
    // 2. 新個体として population に追加（進化＝出生）
    let new_pid = ctx.add_person();
    let new_person = &mut ctx.population[new_pid];
    new_person.graph = new_graph;
    // 3. helpee から属性を継承
    new_person.position = ctx.population[helpee_id].position.clone();
    new_person.village_assignment = ctx.population[helpee_id].village_assignment;
    // 4. 元の helpee は変更しない
    let added_count = nodes.len();
    // ...
}
```

### Step 6: GMR 個体登録（問題B）

**ファイル**: `src/simulation.rs` 行 3237-3239

`try_gmr_diffusion` の成功パスを Step 5 と同様に変更。元の個体は変更しない。

### Step 7: 自己抽象化の個体登録（問題B）

**ファイル**: `src/self_refinement.rs` + `src/simulation.rs`

- `register_abstracted_subworkflow` に `ctx: &mut SimulationContext` または `add_to_population: impl FnOnce(WorkflowGraph)` を追加
- または、`run_self_refinement_round` の呼び出し元である `run_self_refinement_for_population` で新個体追加を行う
- SubWorkflow のグラフを新個体として `ctx.add_person()` に渡す
- 親グラフの SubWorkflow 置換（`replace_with_subworkflow`）は従来通り行う（抽象化として正しい）

### Step 8: .unwrap_or(0) の改善（問題A+B）

**ファイル**: `src/simulation.rs` 行 2027, 2096

- エラー時に `warn!` または `eprintln!` で NotFound の ID を出力する
- 本チケットの修正完了後は NotFound が発生しないことが理想だが、防衛的措置として残す

## Boy Scout Rule — 翻訳可能性計画

1. **`workflow_registry.rs:7-8`**: 嘘のコメントを削除し、新しい責務（GraphStore 委譲＋キャッシュ）に合わせて正確なコメントに更新する。

2. **`workflow_registry.rs:81-97`**: `register_graph_only` の関数コメントを「GraphStore に書き込み、キャッシュにも保持する」と更新。

3. **`simulation.rs:3019-3021`**: `propose_subgraph_and_accept` 内の上書き代入を「出生（birth）」の処理ブロックとして抽出し、関数名で意図を語らせる。

4. **`simulation.rs:3237-3239`**: `try_gmr_diffusion` 内も同様に抽出。

5. **`self_refinement.rs:126`**: `register_abstracted_subworkflow` の責務を「レジストリ登録 + 個体登録」に拡張し、関数名を `register_abstracted_subworkflow_as_individual` 等に変更する。

## Acceptance Criteria

### 問題A: GraphStore 一元化
- [ ] GraphStore トレイトが `Send + Sync` である
- [ ] InMemoryGraphStore が `Sync` である
- [ ] GraphStore に `store_memoized_graph` / `load_memoized_graph` が追加されている
- [ ] WorkflowRegistry が `Arc<dyn GraphStore>` を保持する
- [ ] WorkflowRegistry の `register` / `register_graph_only` / `register_with_id` が GraphStore に書き込む
- [ ] WorkflowRegistry の内部 HashMap はキャッシュとして機能する

### 問題B: 個体登録
- [ ] HELP `propose_subgraph_and_accept` 成功時に新個体が `add_person()` で追加される
- [ ] HELP 時に元の helpee のグラフは変更されない
- [ ] GMR `try_gmr_diffusion` 成功時に新個体が `add_person()` で追加される
- [ ] GMR 時に元の個体のグラフは変更されない
- [ ] 自己抽象化時に抽出された SubWorkflow が新個体として `add_person()` で追加される
- [ ] 全イベントの `node_count` が正しく計上される（0 にならない）
- [ ] 個体数爆発の人工的抑制を入れていない

### テスト
- [ ] T-A1〜T-A4 の全不変条件テストが通過する
- [ ] T-B1〜T-B6 の全不変条件テストが通過する
- [ ] O1〜O3 の全観測テストが構造化 CSV を出力する
- [ ] 既存の全テストが回帰しない
- [ ] Boy Scout Rule の改善項目が適用されている

## Notes

- plan_path: context/0146-graphstore-unification-and-population-registration/plan.md（未作成）
- implementation_path: context/0146-graphstore-unification-and-population-registration/implementation.md（未作成）
- review_report_path: context/0146-graphstore-unification-and-population-registration/review.md（未作成）
- observation_report_path: context/0146-graphstore-unification-and-population-registration/observation-YYYYMMDD-HHmmss.md（未作成）

### 成果物

- 計画: context/0146-.../plan.md（未作成）
- 実装サマリ: context/0146-.../implementation.md（未作成）
- レビュー報告書: context/0146-.../review.md（未作成）
- 観察レポート: context/0146-.../observation-YYYYMMDD-HHmmss.md（未作成）
