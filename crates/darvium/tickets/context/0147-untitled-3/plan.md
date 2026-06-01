# 首長性スコア (Chiefdom Score) 実装計画

## Context

各個人の社会的影響力を測定する「首長性スコア」を導入する。これは既存の評判スコア (`ReputationProfile.final_score`) と新設する「洗練スコア」(抽象化割合 + 抽象化深度) の加重和として定義される。首長性スコアが最も高い個体が村の首長として選出され、ブラウザ上で視覚的に識別可能になる。

## 要件

### 首長性スコア計算
- `chiefdom_score = 0.5 × final_score + 0.5 × sophistication_score`, 範囲 [0, 1]
- `sophistication_score = 0.5 × 正規化抽象化割合 + 0.5 × 正規化抽象化深度`
  - 抽象化割合 = total_nodes / surface_nodes（内部ノード隠蔽率）
  - 抽象化深度 = SubWorkflow 最大再帰ネスト深度
- 重み: 50/50（プロビジョナル）

### 首長選出
- Phase 3.8: 各村で `chiefdom_score` 最大の個体を首長に選出
- 村に割り当てられていない個体は首長にならない
- 1村につき1人の首長

### フロントエンド通知
- 既存の `ClockAdvanced` イベントに村ごとの首長IDマップを追加
- フロントエンドは首長を黒色の円で描画（年齢色を上書き）

## 関数設計（4層分離—翻訳可能性保証）

各関数は独立にテスト可能で、呼び出しの並びが日本語に逐語訳できる:

| 関数 | ファイル | 逐語訳 |
|---|---|---|
| `compute_abstraction_ratio(graph, store)` | `graph_query.rs` | 抽象化割合を計算する |
| `compute_abstraction_depth(graph, store)` | `graph_query.rs` | 抽象化深度を計算する |
| `compute_sophistication_score(graph, store)` | `graph_query.rs` | 洗練スコアを計算する |
| `compute_chiefdom_score(reputation, sophistication)` | `graph_query.rs` | 首長性スコアを計算する |
| `elect_village_chiefs(population)` | `simulation.rs` | 村の首長を選出する |

呼び出し列の逐語訳:
「抽象化割合を計算し、抽象化深度を計算し、洗練スコアを計算し、評判スコアと統合して首長性スコアを計算し、各村の首長を選出する」

## 変更ファイル一覧

| ファイル | 種別 | 内容 |
|---|---|---|
| `src/constants.rs` | 追加 | CHIEFDOM_DEPTH_SCALE 定数 |
| `src/event.rs` | 追加 | ReputationProfile に chiefdom_score フィールド追加 |
| `src/graph_query.rs` | 追加 | 4関数 + 再帰ヘルパー + テスト |
| `src/simulation.rs` | 追加 | Phase 3.7/3.8、elect_village_chiefs、payload 拡張 |
| `web/cube/observation/index.html` | 追加 | 首長性中央値表示行、首長数の表示行 |
| `web/cube/observation/script.js` | 追加 | chiefdom_score 受信・集計・首長の黒色描画 |

## 実装手順

### Step 1: `src/constants.rs` — 定数追加

`KW_ACCEL_NODE_DENSITY_MAX` (line 1479) 直後:

```rust
/// 首長性スコア: 抽象化深度の正規化スケール定数
///
/// depth / (depth + SCALE) による正規化に使用。
/// depth=0 → 0.0, depth=SCALE → 0.5, depth→∞ → 1.0
/// Calibration Candidate, 推奨範囲: 1.0-10.0
pub const CHIEFDOM_DEPTH_SCALE: f64 = 3.0;
```

### Step 2: `src/event.rs` — ReputationProfile に chiefdom_score 追加

`ReputationProfile` (line 436-471) の `benevolence_score` 直後:
```rust
/// 首長性スコア (0.5 * final_score + 0.5 * sophistication_score, [0, 1])。
/// Phase 3.7 で各 tick 再計算。
pub chiefdom_score: f32,
```

`cold_start()` (line 477-496) の最終フィールド:
```rust
chiefdom_score: 0.5,
```

### Step 3: `src/graph_query.rs` — 4関数追加

`count_recursive` (line 34) 直後に以下を追加。既存の `compute_all_node_count` を内部で再利用する。

#### 関数1: `compute_abstraction_ratio`

```rust
/// 抽象化割合を計算し [0, 1] に正規化する。
///
/// ratio = total_nodes / surface_nodes
/// 正規化: (ratio - 1.0) / ratio
/// ratio=1.0（subgraph なし）→ 0.0, ratio→∞ → 1.0
pub fn compute_abstraction_ratio(
    graph: &WorkflowGraph,
    store: &dyn GraphStore,
) -> Result<f32, DarviumError> {
    let total = compute_all_node_count(graph, store)?;
    let surface = graph.node_count();
    if surface == 0 || total == surface {
        return Ok(0.0_f32);
    }
    let ratio = total as f64 / surface as f64;
    Ok(((ratio - 1.0) / ratio) as f32)
}
```

#### 関数2: `compute_abstraction_depth`

```rust
/// 最大 SubWorkflow ネスト深度を計算し [0, 1] に正規化する。
///
/// 正規化: depth / (depth + CHIEFDOM_DEPTH_SCALE)
/// depth=0 → 0.0, depth=SCALE → 0.5, depth→∞ → 1.0
pub fn compute_abstraction_depth(
    graph: &WorkflowGraph,
    store: &dyn GraphStore,
) -> Result<f32, DarviumError> {
    let max_depth = calculate_max_nest_depth(graph, store)?;
    if max_depth == 0 {
        return Ok(0.0_f32);
    }
    let depth = max_depth as f64;
    Ok((depth / (depth + CHIEFDOM_DEPTH_SCALE)) as f32)
}
```

#### 再帰ヘルパー: `calculate_max_nest_depth`

```rust
/// SubWorkflow 参照を再帰的に辿り、最大ネスト深度を計算する。
/// 循環参照は visited set で保護。
fn calculate_max_nest_depth(
    graph: &WorkflowGraph,
    store: &dyn GraphStore,
    visited: &mut HashSet<GraphId>,
) -> Result<usize, DarviumError> {
    let mut has_sub = false;
    let mut max_child = 0usize;
    for node_idx in graph.node_indices() {
        if let WorkflowNode::SubWorkflow { workflow_id, .. } = &graph[node_idx] {
            has_sub = true;
            if visited.insert(workflow_id.clone()) {
                let sub = store.load_workflow_graph(workflow_id)?;
                let child = calculate_max_nest_depth(&sub, store, visited)?;
                max_child = max_child.max(child);
            }
        }
    }
    Ok(if has_sub { max_child + 1 } else { 0 })
}
```

#### 関数3: `compute_sophistication_score`

```rust
/// 洗練スコアを計算する。
/// sophistication = 0.5 * abstraction_ratio + 0.5 * abstraction_depth
pub fn compute_sophistication_score(
    graph: &WorkflowGraph,
    store: &dyn GraphStore,
) -> Result<f32, DarviumError> {
    let ratio = compute_abstraction_ratio(graph, store)?;
    let depth = compute_abstraction_depth(graph, store)?;
    Ok(0.5_f32 * ratio + 0.5_f32 * depth)
}
```

#### 関数4: `compute_chiefdom_score`

```rust
/// 首長性スコアを計算する。
/// chiefdom = 0.5 * final_score + 0.5 * sophistication
pub fn compute_chiefdom_score(
    reputation: &ReputationProfile,
    sophistication: f32,
) -> f32 {
    0.5_f32 * reputation.final_score + 0.5_f32 * sophistication
}
```

#### graph_query.rs テスト追加

既存の `mod tests` (line 52) に以下を追加:
- `compute_abstraction_ratio`: 空グラフ→0.0、フラット→0.0、SubWorkflow有→(0,1)
- `compute_abstraction_depth`: SubWorkflowなし→0.0、単一→0.25(1/(1+3))、2段→0.4(2/(2+3))
- `compute_sophistication_score`: 上記の合成値
- `compute_chiefdom_score`: 既知入力→正しい加重和

### Step 4: `src/simulation.rs` — Phase 3.7/3.8 追加 + payload 拡張

**4a: import 追加** (line 39 付近):
```rust
use crate::graph_query::compute_sophistication_score;
use crate::graph_query::compute_chiefdom_score;
```

**4b: `elect_village_chiefs` 関数追加** (任意の場所、recompute_reputation_for_population 直後など):

```rust
/// Phase 3.8: 各村の首長を選出する。
///
/// 各村内で chiefdom_score が最大の生存個体を首長とする。
/// 村に属さない個体は首長にならない。
/// 戻り値: HashMap<VillageId, PersonId>
fn elect_village_chiefs(population: &[MemoizedGraph]) -> HashMap<VillageId, PersonId> {
    let mut best_score: HashMap<VillageId, (PersonId, f32)> = HashMap::new();
    for (pid, person) in population.iter().enumerate() {
        if !person.alive {
            continue;
        }
        if let Some(v_id) = person.village_assignment {
            let entry = best_score.entry(v_id).or_insert((pid, -1.0_f32));
            if person.reputation.chiefdom_score > entry.1 {
                *entry = (pid, person.reputation.chiefdom_score);
            }
        }
    }
    best_score.into_iter().map(|(v, (pid, _))| (v, pid)).collect()
}
```

**4c: `run_evaluation_simulation_with_channel` の Phase ループ** (line 1900-2006):

Phase 3.5 (line 1939) 直後、Phase 3.6 (line 1941) の前に挿入:
```rust
// Phase 3.5: 評判再計算（既存）
recompute_reputation_for_population(&mut ctx.population, &kw_sessions, tick, &config.policy);

// Phase 3.7: 首長性スコア計算（評判スコア + 洗練スコアの加重和）
for person in ctx.population.iter_mut().filter(|p| p.alive) {
    let sophistication = compute_sophistication_score(&person.graph, &graph_store)
        .unwrap_or_else(|e| {
            eprintln!("[simulation] compute_sophistication_score failed for {}: {:?}", person.id, e);
            0.0_f32
        });
    person.reputation.chiefdom_score = compute_chiefdom_score(&person.reputation, sophistication);
}

// Phase 3.8: 各村の首長選出
let village_chiefs = elect_village_chiefs(&ctx.population);

// Phase 3.6: 自己抽象化（既存）
let abstraction_count = run_self_refinement_for_population(&mut ctx);
```

**4d: `build_clock_advanced_event` の payload** (line 2177-2221):

トップレベルフィールドに首長マップを追加 (line 2194 `village_densities` 直後):
```rust
"village_densities": densities,
"village_chiefs": serde_json::to_value(&village_chiefs)
    .unwrap_or(serde_json::Value::Null),
```

`village_chiefs` 変数は呼び出し元から渡す必要があるため、関数シグネチャに追加:
```rust
fn build_clock_advanced_event(
    tick: u64,
    snapshot: &SimulationTickSnapshot,
    ctx: &SimulationContext,
    births: usize,
    deaths: usize,
    village_count: usize,
    village_chiefs: &HashMap<VillageId, PersonId>,
    store: &dyn GraphStore,
) -> DarviumEvent {
```

呼び出し元 (line 1984-1992) も合わせて更新。

### Step 5: `web/cube/observation/index.html` — 統計パネル

`benevolenceP50Val` 行 (line 94) 直後:
```html
<tr>
    <td>首長性中央値</td>
    <td id="chiefdomP50Val" class="value">0.000</td>
</tr>
<tr>
    <td>首長数</td>
    <td id="chiefCountVal" class="value">0</td>
</tr>
```

### Step 6: `web/cube/observation/script.js` — ブラウザ側

**6a: `onTickMetrics` 内で per-node chiefdom_score を保存** (line 405 付近):
```javascript
if (serverNode.chiefdom_score != null) {
    node.chiefdom_score = serverNode.chiefdom_score;
}
```

**6b: 首長マップの処理** — `onTickMetrics` 内:
```javascript
// 首長情報をリセットしてから設定
STATE.chiefs = {};
if (payload.village_chiefs) {
    for (const [villageId, chiefPid] of Object.entries(payload.village_chiefs)) {
        STATE.chiefs[chiefPid] = true;
    }
}
```

**6c: 首長性中央値の集計** — `onTickMetrics` 内:
```javascript
const chiefdomScores = [];
for (const [, node] of STATE.nodes) {
    if (node.chiefdom_score != null) {
        chiefdomScores.push(node.chiefdom_score);
    }
}
if (chiefdomScores.length > 0) {
    chiefdomScores.sort((a, b) => a - b);
    const mid = Math.floor(chiefdomScores.length / 2);
    STATE.chiefdomP50 = chiefdomScores.length % 2 === 0
        ? (chiefdomScores[mid - 1] + chiefdomScores[mid]) / 2
        : chiefdomScores[mid];
} else {
    STATE.chiefdomP50 = 0;
}
STATE.chiefCount = Object.keys(payload.village_chiefs || {}).length || 0;
```

**6d: 描画色の決定** — ノード作成時 (line 318-323) と再描画時 (line 431-437):
```javascript
// 首長は黒色、それ以外は年齢色
const fillColor = STATE.chiefs[pid]
    ? 0x000000  // 首長は黒
    : ageColor(node.age);
```

**6e: `updateStatsPanel`** (line 581 付近):
```javascript
document.getElementById("chiefdomP50Val").textContent = (
    STATE.chiefdomP50 || 0
).toFixed(3);
document.getElementById("chiefCountVal").textContent = (
    STATE.chiefCount || 0
);
```

**6f: `clearVisualization`** — STATE リセット:
```javascript
STATE.chiefdomP50 = 0;
STATE.chiefCount = 0;
STATE.chiefs = {};
```

## Phase 実行順序（該当箇所のみ抜粋）

```text
Phase 2:     村クラスタリング         → village_assignment 決定
Phase 2.5:   村中心性算出            → village_centrality
Phase 3:     HELP プロトコル          → 相互作用記録
Phase 3.5:   評判再計算              → reputation.final_score
──────────────────────────────────────────────────
Phase 3.7:   首長性スコア計算         → reputation.chiefdom_score
Phase 3.8:   各村の首長選出           → HashMap<VillageId, PersonId>
──────────────────────────────────────────────────
Phase 3.6:   自己抽象化              → SubWorkflow 登録（次 tick から反映）
Phase 4:     GC 生存                 → 死亡判定
Phase 5:     能力拡散                → グラフ複製
```

Phase 3.7/3.8 を Phase 3.6（自己抽象化）の前に配置する理由: 自己抽象化で SubWorkflow が増えると次 tick から抽象化割合と深度が変わるため、**現在の tick の状態**で首長性スコアを計算するには自己抽象化より前が適切。

## エッジケース

| ケース | 挙動 |
|---|---|
| 空グラフ (surface=0) | ratio=0.0, depth=0.0 → sophistication=0.0 |
| SubWorkflow なし | ratio=0.0, depth=0.0 → sophistication=0.0（評判のみ） |
| 循環参照 | visited set で保護、有限値を返す |
| GraphStore 未設定 | compute_sophistication_score → Err → fallback 0.0 + eprintln |
| 死亡個体 | Phase 3.7: filter でスキップ。Phase 3.8: filter でスキップ |
| 村に属さない個体 | Phase 3.8: 首長にならない（village_assignment is None） |
| 村の人口が0 | elect_village_chiefs → その村は結果に含まれない |
| 首長が死亡 | 次 tick の Phase 3.8 で再選出される |
| surface=0 かつ total=0 | surface==0 → ratio_score=0.0（早期 return） |

## 検証方法

```bash
# コンパイル確認
cargo check --features server

# テスト実行（graph_query.rs の新規テストを含む）
cargo test

# ブラウザ可視化確認
cargo run --features server
# → 首長性中央値と首長数が統計パネルに表示される
# → 各村の首長が黒色の円で描画される
```
