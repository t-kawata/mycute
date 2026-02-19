# Cognee Go Implementation: Phase-08 Detailed Development Directives

## 0. はじめに (Introduction)

本ドキュメントは、**Phase-08: The Living Graph (生きた知識グラフ)** の実装を「迷いなく」「確実に」遂行するための詳細な開発指示書です。
`docs/AFTER-PHASE-07.md` で策定された設計に基づき、具体的なコード、ファイルパス、手順を網羅しています。

> [!IMPORTANT]
> **実装のゴール**
> 知識グラフを「静的なデータベース」から「動的に成長・代謝する脳」へと進化させること。
> 具体的には、厳格なデータ分離、スケーラブルな知識統合、そして情報の鮮度と確からしさに基づく自動淘汰（代謝）を実現します。

---

## 1. 実装ステップ一覧 (Implementation Steps)

以下の順序で実装を進めます。各ステップは依存関係に基づいています。

1.  **Strict Data Partitioning**: CozoDBクエリにおける `GroupID` の厳格な適用。
2.  **Graph Metabolism Foundation**: エッジの重み・信頼度を更新する基盤の実装。
3.  **Production-Ready Crystallization**: ベクトル検索を用いたスケーラブルな知識統合。
4.  **Graph Hygiene (Pruning)**: 孤立ノードの削除とガベージコレクション。
5.  **Graph Refinement Integration**: 代謝モデル（強化・減衰・淘汰）のパイプライン統合。
6.  **Configuration Management**: 全パラメータの環境変数化と管理。

---

## Step 1: Strict Data Partitioning

**目的**: ユーザーやデータセット間でのデータ漏洩をデータベースレベルで完全に防ぐため、全てのCozoDBクエリで `group_id` を強制します。

### 1.1 `src/pkg/cognee/db/cozodb/cozo_storage.go` の修正

全てのメソッドを監査し、クエリを修正します。

**実装ルール**:
*   クエリ内では必ず `group_id = $group_id` を結合条件に含める。
*   `DeleteEdge`, `DeleteNode` (新規実装) でも `group_id` を必須とする。

#### 1.1.1 `GetTriplets` の修正
```go
func (s *CozoStorage) GetTriplets(ctx context.Context, nodeIDs []string, groupID string) ([]*storage.Triplet, error) {
    // ...
    query := `
        ?[source_id, target_id, group_id, edge_type, edge_props, 
          source_type, source_props, target_type, target_props] := 
            *edges[source_id, target_id, group_id, edge_type, edge_props],
            source_id in $node_ids,
            group_id = $group_id,  // [CRITICAL] 必須
            *nodes[source_id, group_id, source_type, source_props],
            *nodes[target_id, group_id, target_type, target_props]
    `
    // ...
}
```

#### 1.1.2 `GetNodesByType` の修正
```go
func (s *CozoStorage) GetNodesByType(ctx context.Context, nodeType string, groupID string) ([]*storage.Node, error) {
    query := `
        ?[id, group_id, type, properties] := 
            *nodes[id, group_id, type, properties],
            group_id = $group_id,
            type = $type
    `
    params := map[string]any{"group_id": groupID, "type": nodeType}
    // ...
}
```

#### 1.1.3 `DeleteEdge` の実装
```go
func (s *CozoStorage) DeleteEdge(ctx context.Context, sourceID, targetID, groupID string) error {
    // typeが不明なため、source/target/group_idに一致する全エッジを削除
    // まず削除対象のエッジタイプを取得
    queryFind := `
        ?[type] := *edges[source_id, target_id, group_id, type, _],
        source_id = $source_id,
        target_id = $target_id,
        group_id = $group_id
    `
    // 取得した各typeに対して削除を実行
    // :rm edges {source_id, target_id, group_id, type}
}
```

#### 1.1.4 `DeleteNode` の実装 (新規)
```go
func (s *CozoStorage) DeleteNode(ctx context.Context, nodeID, groupID string) error {
    // :rm nodes {id, group_id}
    // group_idを含めることで、他人の同IDノードを削除するリスクを回避
    query := `:rm nodes {id: $id, group_id: $group_id}`
    // ...
}
```

---

## Step 2: Graph Metabolism Foundation

**目的**: グラフの「代謝」を実現するための、エッジ評価指標（重み・信頼度）の更新メソッドを実装します。

### 2.1 `src/pkg/cognee/db/cozodb/cozo_storage.go` の拡張

`UpdateEdgeMetrics` メソッドを実装し、WeightとConfidenceをアトミックに更新できるようにします。

```go
// [NEW] UpdateEdgeMetrics
// 指定されたエッジの重み(Weight)と信頼度(Confidence)を更新します。
func (s *CozoStorage) UpdateEdgeMetrics(ctx context.Context, sourceID, targetID, groupID string, weight, confidence float64) error {
    // プロパティJSON内の "weight", "confidence" を更新するクエリ
    // json_set関数を使用して既存のJSONを更新
    query := `
        ?[source_id, target_id, group_id, type, new_props] := 
            *edges[source_id, target_id, group_id, type, props],
            source_id = $source_id,
            target_id = $target_id,
            group_id = $group_id,
            new_props = json_set(props, 'weight', $weight, 'confidence', $confidence)
        :put edges {source_id, target_id, group_id, type, properties: new_props}
    `
    params := map[string]any{
        "source_id":  sourceID,
        "target_id":  targetID,
        "group_id":   groupID,
        "weight":     weight,
        "confidence": confidence,
    }
    // Run query...
}
```

---

## Step 3: Production-Ready Crystallization

**目的**: `O(N^2)` の全探索クラスタリングを廃止し、ベクトル検索を用いたスケーラブルな実装に置き換えます。また、統合時のエッジ付け替えを実装します。

### 3.1 `src/pkg/cognee/tasks/metacognition/crystallization_task.go` の改修

`clusterBySimilarity` をベクトル検索ベースに書き換えます。

**アルゴリズム詳細**:
1.  **入力**: `nodes` (ルールノード群)
2.  **処理**:
    *   各ノード `n` について:
        *   `text` プロパティを取得。
        *   `Embedder.EmbedQuery` でベクトル化。
        *   `VectorStorage.Search` で類似ノードを検索 (K=10, Threshold=0.8)。
        *   検索結果のIDが `nodes` リストに含まれている場合のみ、エッジとして記録。
    *   構築されたグラフ（Adjacency List）に対して、BFS/DFSで連結成分分解を行う。
3.  **出力**: `[][]*storage.Node` (クラスタのリスト)

**エッジ付け替えロジック (CrystallizeRules内)**:
統合ノード `C` を作成した後：

```go
// 1. 旧ノード群 (cluster) に接続しているエッジを取得
for _, oldNode := range cluster {
    // Outgoing Edges
    outEdges, _ := t.GraphStorage.GetEdgesByNode(ctx, oldNode.ID, t.GroupID)
    for _, edge := range outEdges {
        if edge.SourceID == oldNode.ID {
            // C -> Target のエッジを作成
            newEdge := &storage.Edge{
                SourceID: t.GroupID, // C.ID
                TargetID: edge.TargetID,
                // ... copy type & properties
            }
            t.GraphStorage.AddEdges(ctx, []*storage.Edge{newEdge})
        } else {
            // Source -> C のエッジを作成
            newEdge := &storage.Edge{
                SourceID: edge.SourceID,
                TargetID: t.GroupID, // C.ID
                // ...
            }
            t.GraphStorage.AddEdges(ctx, []*storage.Edge{newEdge})
        }
    }
    
    // 2. 旧ノードを論理削除 (または物理削除)
    // ここでは物理削除を選択 (PruningTaskに任せるなら論理削除だが、今回は即時削除でOK)
    t.GraphStorage.DeleteNode(ctx, oldNode.ID, t.GroupID)
}
```

---

## Step 4: Graph Hygiene (Pruning)

**目的**: 孤立したノードや、代謝によって不要と判断された情報を削除します。

### 4.1 `src/pkg/cognee/tasks/metacognition/pruning_task.go` の作成

**Datalogクエリ (孤立ノード検出)**:
「エッジテーブルに `source_id` としても `target_id` としても存在しないノード」を探します。

```go
func (t *PruningTask) PruneOrphans(ctx context.Context, groupID string) error {
    // 1. 孤立ノードを特定
    // 注意: CozoDBの否定(not)や集計を使って実装
    // 簡易実装: 全ノード取得 -> 全エッジ取得 -> メモリ上で判定 (小規模ならOK)
    // 本番実装: Datalogで完結させる
    
    query := `
        // 全ノードID
        ?[id] := *nodes[id, group_id, _, _], group_id = $group_id
        
        // エッジで使用されているID (Source)
        :create _used_ids {id}
        ?[source_id] -> _used_ids :put *edges[source_id, _, group_id, _, _], group_id = $group_id
        
        // エッジで使用されているID (Target)
        ?[target_id] -> _used_ids :put *edges[_, target_id, group_id, _, _], group_id = $group_id
        
        // 差分を取る (nodes - used_ids)
        ?[id] := *nodes[id, group_id, _, _], group_id = $group_id, not _used_ids[id]
    `
    
    // 2. 取得したIDに対して DeleteNode を実行
    // ただし、GracePeriod (作成日時) のチェックが必要。
    // nodesテーブルに created_at がない場合、メタデータ管理が必要になるが、
    // Phase-08では「作成直後のノードはエッジがないのが普通」なので、
    // PruningTaskの実行タイミングを制御する（Memifyの最後など）ことで回避可能。
}
```

---

## Step 5: Graph Refinement Integration (Metabolism)

**目的**: `GraphRefinementTask` に「代謝モデル」を組み込み、エッジの生存スコアに基づいて強化・減衰・淘汰を行います。

### 5.1 `src/pkg/cognee/tasks/metacognition/graph_refinement_task.go` の実装

数理モデル $S = W \times C$ を適用します。

**更新ロジック**:
*   **Strengthen**: $C_{new} = C_{old} + (1 - C_{old}) \times \alpha$
*   **Weaken**: $C_{new} = C_{old} \times (1 - \delta)$
*   **Prune**: $S < T_{prune} \implies$ Delete

```go
// [MODIFY] evaluateEdges 内でLLMの判定結果に基づき計算
func (t *GraphRefinementTask) applyMetabolism(ctx context.Context, edge *storage.Edge, action string) error {
    alpha := t.Config.MetabolismAlpha
    delta := t.Config.MetabolismDelta
    threshold := t.Config.PruneThreshold

    // 現在の値を取得 (デフォルト値考慮)
    w := edge.Weight
    if w == 0 { w = 0.5 }
    c := edge.Confidence
    if c == 0 { c = 0.9 } // 初期値高め

    var newW, newC float64

    switch action {
    case "strengthen":
        newC = c + (1.0 - c) * alpha
        newW = w + (1.0 - w) * 0.1 // Beta
    case "weaken":
        newC = c * (1.0 - delta)
        newW = w // 重みは維持
    case "delete":
        return t.GraphStorage.DeleteEdge(ctx, edge.SourceID, edge.TargetID, t.GroupID)
    default:
        return nil
    }

    survivalScore := newW * newC
    if survivalScore < threshold {
        return t.GraphStorage.DeleteEdge(ctx, edge.SourceID, edge.TargetID, t.GroupID)
    }
    
    return t.GraphStorage.UpdateEdgeMetrics(ctx, edge.SourceID, edge.TargetID, t.GroupID, newW, newC)
}
```

---

## Step 6: Configuration Management

**目的**: 全ての設定値を環境変数化し、ハードコーディングを排除します。

### 6.1 `src/.env` & `src/.env.example` の更新

以下の変数を追加し、詳細な日本語コメントを付与します。

```bash
# グラフ代謝設定
COGNEE_GRAPH_METABOLISM_ALPHA=0.2       # 強化学習率 (0.0-1.0)
COGNEE_GRAPH_METABOLISM_DELTA=0.3       # 減衰ペナルティ率 (0.0-1.0)
COGNEE_GRAPH_METABOLISM_PRUNE_THRESHOLD=0.1 # 淘汰閾値 (生存スコア < この値なら削除)
COGNEE_GRAPH_PRUNING_GRACE_PERIOD_MINUTES=60 # 孤立ノード削除の猶予期間
```

### 6.2 `src/pkg/cognee/cognee.go` の修正

`CogneeConfig` 構造体にフィールドを追加します。

```go
type CogneeConfig struct {
    // ...
    GraphMetabolismAlpha          float64
    GraphMetabolismDelta          float64
    GraphMetabolismPruneThreshold float64
    GraphPruningGracePeriodMinutes int
}
```

---

## 7. 検証計画 (Verification Plan)

実装の完了は、以下のテスト計画に基づき、全ての機能が期待通りに動作することを確認した時点と定義します。
そのために `main.go` を拡張し、検証用コマンドを追加します。

### 7.1 `main.go` の拡張

以下のサブコマンドを追加します。

1.  `test-metabolism`: グラフ代謝（強化・減衰・淘汰）のシミュレーションを実行。
2.  `test-pruning`: 孤立ノードの削除（GracePeriodの考慮）を検証。
3.  `test-crystallization`: ベクトル検索ベースの結晶化とエッジ付け替えを検証。

```go
// main.go 実装イメージ
case "test-metabolism":
    // 1. テスト用ノード・エッジを作成 (Weight=0.5, Confidence=0.5)
    // 2. "strengthen" を実行 -> 値の上昇を確認
    // 3. "weaken" を実行 -> 値の減少を確認
    // 4. "prune" 条件まで下げて削除されるか確認
case "test-pruning":
    // 1. 孤立ノードA (Old), 孤立ノードB (New), 接続ノードC を作成
    // 2. PruningTaskを実行
    // 3. Aのみが削除され、BとCが残ることを確認
```

### 7.2 テストシナリオと合格基準

#### シナリオA: 代謝サイクル (Metabolism Cycle)
1.  **準備**: エッジ E1 (W=0.5, C=0.5) を作成。
2.  **強化**: `RefineEdges` で「支持」判定をシミュレート。
    *   *合格基準*: C > 0.5, W > 0.5 に更新されていること。
3.  **減衰**: `RefineEdges` で「矛盾」判定をシミュレート。
    *   *合格基準*: C が減少していること。
4.  **淘汰**: Cを極端に下げて (例: 0.01) 代謝を実行。
    *   *合格基準*: エッジ E1 が物理削除されていること。

#### シナリオB: 知識結晶化 (Crystallization)
1.  **準備**: 類似したルールノード R1, R2, R3 を作成。
2.  **実行**: `CrystallizationTask` を実行。
3.  **検証**:
    *   *合格基準*: 新しい統合ノード C1 が作成されていること。
    *   *合格基準*: R1, R2, R3 が削除（または論理削除）されていること。
    *   *合格基準*: R1, R2 に接続していたエッジが C1 に付け替えられていること。

#### シナリオC: 衛生管理 (Hygiene)
1.  **準備**:
    *   Node A: 孤立, CreatedAt = 2時間前
    *   Node B: 孤立, CreatedAt = 10分前 (GracePeriod内)
    *   Node C: エッジあり
2.  **実行**: `PruningTask` を実行。
3.  **検証**:
    *   *合格基準*: Node A のみが削除されていること。

---

## 8. 最終確認 (Final Review)

全ての実装とテストが完了したら、以下のコマンドで最終ビルド確認を行います。

```bash
make build
./bin/cognee test-metabolism
./bin/cognee test-pruning
./bin/cognee test-crystallization
```
