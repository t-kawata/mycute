# Phase-09 実装計画: 効率化と最適化 (Optimization & Performance)

Phase-08では「生きた知識グラフ」の基盤を構築しました。Phase-09では、これらの機能をより効率的に動作させるための最適化を行います。

---

## 1. Embedding キャッシュの活用 (Embedding Cache Optimization)

### 1.1 現状の課題

`CrystallizationTask.clusterBySimilarity()` において、以下の非効率な処理が存在します：

```go
// 現在の実装（非効率）
vector, err := t.Embedder.EmbedQuery(ctx, text)
```

**問題点:**
- 毎回 LLM API を呼び出して Embedding を再計算している
- ノードが既に `VectorStorage` に保存されている場合、その Embedding は既にキャッシュされているはずである
- API コストと処理時間の無駄が発生

### 1.2 改善方針

#### A. VectorStorage に Embedding 取得 API を追加

`VectorStorage` インターフェースに以下のメソッドを追加：

```go
// GetEmbeddingByID は、指定されたIDのEmbeddingを取得します。
// collectionName: コレクション名（例: "Rule_text"）
// id: ノードID
// groupID: グループID
GetEmbeddingByID(ctx context.Context, collectionName, id, groupID string) ([]float32, error)

// GetEmbeddingsByIDs は、複数IDのEmbeddingを一括取得します（バッチ処理用）。
GetEmbeddingsByIDs(ctx context.Context, collectionName string, ids []string, groupID string) (map[string][]float32, error)
```

#### B. DuckDBStorage への実装

```sql
-- 単一取得
SELECT id, vector FROM {collection}_vectors 
WHERE id = $id AND group_id = $group_id

-- バッチ取得
SELECT id, vector FROM {collection}_vectors 
WHERE id IN ($ids) AND group_id = $group_id
```

#### C. CrystallizationTask の改修

```go
// 改善後の実装
func (t *CrystallizationTask) clusterBySimilarity(ctx context.Context, nodes []*storage.Node, threshold float64) [][]*storage.Node {
    // Step 1: バッチで既存のEmbeddingを取得
    nodeIDs := make([]string, len(nodes))
    for i, n := range nodes {
        nodeIDs[i] = n.ID
    }
    
    cachedEmbeddings, err := t.VectorStorage.GetEmbeddingsByIDs(ctx, "Rule_text", nodeIDs, t.GroupID)
    if err != nil {
        // フォールバック: 個別にEmbedderを使用
    }
    
    // Step 2: キャッシュにないものだけEmbedderで計算
    for _, node := range nodes {
        if _, exists := cachedEmbeddings[node.ID]; !exists {
            text := node.Properties["text"].(string)
            vector, _ := t.Embedder.EmbedQuery(ctx, text)
            cachedEmbeddings[node.ID] = vector
        }
    }
    
    // Step 3: クラスタリング処理...
}
```

### 1.3 期待される効果

| 指標 | 改善前 | 改善後 |
|------|--------|--------|
| API呼び出し回数 | N回（ノード数） | 0〜数回（キャッシュミスのみ） |
| 処理時間 | 数秒〜数十秒 | ミリ秒〜数秒 |
| APIコスト | 高 | 大幅削減 |

---

## 2. その他の最適化候補

### 2.1 バッチ処理の強化

- `AddNodes`, `AddEdges` のバッチサイズを動的に調整
- トランザクション内での複数操作のグループ化

### 2.2 インデックス最適化

- CozoDB の `group_id` に対するインデックス確認と最適化
- DuckDB の HNSW インデックスパラメータのチューニング

### 2.3 メモリ使用量の削減

- 大規模グラフでのストリーミング処理の拡充
- ノードプロパティの遅延読み込み

---

## 3. PruningTask の効率化 (Efficient Orphan Detection)

### 3.1 現状の課題

`PruningTask.PruneOrphans()` において、以下の非効率な処理が存在します：

```go
// 現在の実装（非効率）
targetTypes := []string{"DocumentChunk", "Rule", "Entity", "Summary"}
for _, nodeType := range targetTypes {
    nodes, _ := t.GraphStorage.GetNodesByType(ctx, nodeType, t.GroupID)
    for _, node := range nodes {
        edges, _ := t.GraphStorage.GetEdgesByNode(ctx, node.ID, t.GroupID)
        if len(edges) == 0 {
            // 削除処理
        }
    }
}
```

**問題点:**
- 全ノードを一度メモリにロードしてから、各ノードについてエッジを個別クエリで取得
- ノード数 N に対して N+1 回のクエリが発生（N+1問題）
- 大規模グラフでは非現実的な処理時間とメモリ使用量

### 3.2 改善方針

#### A. CozoDB で孤立ノードを直接検出するクエリ

CozoDB の Datalog クエリで、エッジを持たないノードを一発で取得：

```datalog
# 孤立ノード検出クエリ
?[id, type, props, created_at] := 
    *nodes[id, group_id, type, props],
    group_id = $group_id,
    created_at = get(props, "created_at", ""),
    not *edges[id, _, group_id, _, _],          # Outbound エッジなし
    not *edges[_, id, group_id, _, _]           # Inbound エッジなし
```

#### B. GraphStorage インターフェースへの追加

```go
// GetOrphanNodes は、エッジを持たない孤立ノードを取得します。
// gracePeriod: この時間より新しいノードは除外（誤削除防止）
GetOrphanNodes(ctx context.Context, groupID string, gracePeriod time.Duration) ([]*Node, error)
```

#### C. CozoStorage への実装

```go
func (s *CozoStorage) GetOrphanNodes(ctx context.Context, groupID string, gracePeriod time.Duration) ([]*Node, error) {
    cutoffTime := time.Now().Add(-gracePeriod).Format(time.RFC3339)
    
    query := `
        ?[id, type, props] := 
            *nodes[id, group_id, type, props],
            group_id = $group_id,
            created_at = get(props, "created_at", ""),
            created_at < $cutoff_time,
            not *edges[id, _, $group_id, _, _],
            not *edges[_, id, $group_id, _, _]
    `
    params := map[string]any{
        "group_id":    groupID,
        "cutoff_time": cutoffTime,
    }
    
    // クエリ実行...
}
```

#### D. PruningTask の簡素化

```go
func (t *PruningTask) PruneOrphans(ctx context.Context) error {
    // 1クエリで全孤立ノードを取得
    orphans, err := t.GraphStorage.GetOrphanNodes(ctx, t.GroupID, t.GracePeriod)
    if err != nil {
        return err
    }
    
    // 一括削除
    for _, node := range orphans {
        t.GraphStorage.DeleteNode(ctx, node.ID, t.GroupID)
    }
    
    return nil
}
```

### 3.3 期待される効果

| 指標 | 改善前 | 改善後 |
|------|--------|--------|
| クエリ回数 | N+1回 | 1回 |
| メモリ使用量 | 全ノード + 全エッジ情報 | 孤立ノードのみ |
| 処理時間 | O(N × M) ※M=平均エッジ数 | O(1) クエリ + O(K) 削除 ※K=孤立ノード数 |

---

## Phase-09 実装チェックリスト

### Storage Layer (VectorStorage)
- [ ] `VectorStorage.GetEmbeddingByID` の追加
- [ ] `VectorStorage.GetEmbeddingsByIDs` の追加（バッチ版）
- [ ] `DuckDBStorage` への実装

### Storage Layer (GraphStorage)
- [ ] `GraphStorage.GetOrphanNodes` の追加
- [ ] `CozoStorage` への孤立ノード検出クエリの実装

### Metacognition Layer
- [ ] `CrystallizationTask` の Embedding キャッシュ活用への改修
- [ ] `GraphRefinementTask` の Embedding キャッシュ活用への改修
- [ ] `PruningTask` の `GetOrphanNodes` 活用への改修

### Performance Testing
- [ ] 大規模データ（1000+ ノード）でのベンチマーク
- [ ] API コスト削減効果の測定
- [ ] Pruning 処理時間の計測
