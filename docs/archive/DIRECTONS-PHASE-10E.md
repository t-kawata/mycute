# Cognee Go Implementation: Phase-10E Detailed Development Directives
# GraphStorage Implementation (グラフストレージ実装)

## 0. はじめに (Introduction)

本ドキュメントは、**Phase-10E: GraphStorage Implementation** の実装を「迷いなく」「確実に」遂行するための詳細な開発指示書です。

> [!IMPORTANT]
> **Phase-10Eのゴール**
> `KuzuDBStorage` に `GraphStorage` インターフェースの全メソッドを完全実装する。
> CozoStorageと同等の機能をKuzuDBで再現する。

> [!CAUTION]
> **前提条件**
> Phase-10A〜Phase-10Dが完了していること（VectorStorageが動作すること）

---

## 1. 実装対象メソッド一覧

| メソッド | CozoDBでの実装 | KuzuDBでの実装 | 優先度 |
|---------|---------------|-------------|--------|
| AddNodes | :put nodes | MERGE (n:GraphNode {...}) | 高 |
| AddEdges | :put edges | MATCH + MERGE -[r:GraphEdge]-> | 高 |
| GetTriplets | ?[...] := *nodes, *edges | MATCH (a)-[r]->(b) | 高 |
| StreamDocumentChunks | ?[...] := *nodes WHERE type | MATCH (n:GraphNode) WHERE type + channel | 高 |
| GetDocumentChunkCount | count() | MATCH ... RETURN count() | 中 |
| GetNodesByType | ?[...] := *nodes WHERE type | MATCH (n:GraphNode) WHERE type | 高 |
| GetNodesByEdge | ?[...] := *edges WHERE target | MATCH (a)-[r]->(b) WHERE b.id | 中 |
| UpdateEdgeWeight | :put edges SET | MATCH ... SET r.properties | 高 |
| UpdateEdgeMetrics | :put edges SET | MATCH ... SET r.properties | 高 |
| DeleteEdge | :rm edges | MATCH ... DELETE r | 高 |
| DeleteNode | :rm nodes | MATCH ... DELETE n | 高 |
| GetEdgesByNode | ?[...] := *edges | MATCH (a)-[r]-(b) | 中 |
| GetOrphanNodes | not *edges | NOT EXISTS pattern | 高 |

---

## Step 1: AddNodes実装

### 1.1 CozoDB参照実装

```go
// cozo_storage.go (84-111行目)
func (s *CozoStorage) AddNodes(ctx context.Context, nodes []*storage.Node) error {
    if len(nodes) == 0 {
        return nil
    }

    // 各ノードを [id, group_id, type, properties] の配列に変換
    rows := make([][]any, len(nodes))
    for i, n := range nodes {
        rows[i] = []any{n.ID, n.GroupID, n.Type, n.Properties}
    }

    // Datalogクエリ
    // :put nodes {...}: nodesリレーションにデータを挿入（既存データは上書き）
    query := "?[id, group_id, type, properties] <- $data :put nodes {id, group_id, type, properties}"
    params := map[string]any{"data": rows}

    if _, err := s.db.Run(query, params); err != nil {
        return fmt.Errorf("failed to add nodes: %w", err)
    }
    return nil
}
```

**根拠説明**: CozoDBでは `:put` コマンドでUPSERT（存在すれば更新、なければ挿入）を行っている。KuzuDBでは `MERGE` 句を使用して同等のアトミックなUPSERT操作を実現する。エラーメッセージによる判定は脆弱なため使用しない。

### 1.2 KuzuDB実装

```go
// AddNodes は、複数のノードをグラフに追加します。
//
// CozoDB参照実装:
//   rows := make([][]any, len(nodes))
//   for i, n := range nodes {
//       rows[i] = []any{n.ID, n.GroupID, n.Type, n.Properties}
//   }
//   query := "?[id, group_id, type, properties] <- $data :put nodes {id, group_id, type, properties}"
//   s.db.Run(query, map[string]any{"data": rows})
//
// KuzuDB実装:
//   各ノードに対して MERGE (n:GraphNode {id: ..., group_id: ...}) ON CREATE/ON MATCH SET ...
//
// MERGEを使用する理由:
//   - CozoDBの :put はUPSERTであり、MERGEは同等の動作
//   - エラーメッセージによる判定は脆弱（バージョン変更で壊れる）
//   - アトミックな操作で競合状態を回避
func (s *KuzuDBStorage) AddNodes(ctx context.Context, nodes []*storage.Node) error {
    // CozoDB同様: 空の場合は何もしない
    if len(nodes) == 0 {
        return nil
    }

    for _, node := range nodes {
        // CozoDB: n.Properties をそのまま渡す
        // KuzuDB: JSON文字列に変換して渡す
        propsJSON := "{}"
        if node.Properties != nil {
            if b, err := json.Marshal(node.Properties); err == nil {
                propsJSON = string(b)
            }
        }

        // CozoDB: :put nodes (UPSERT)
        // KuzuDB: MERGE ... ON CREATE SET / ON MATCH SET (UPSERT)
        query := fmt.Sprintf(`
            MERGE (n:GraphNode {id: '%s', group_id: '%s'})
            ON CREATE SET 
                n.type = '%s',
                n.properties = '%s'
            ON MATCH SET 
                n.type = '%s',
                n.properties = '%s'
        `, 
            escapeString(node.ID), 
            escapeString(node.GroupID), 
            escapeString(node.Type), 
            escapeString(propsJSON),
            escapeString(node.Type), 
            escapeString(propsJSON),
        )

        result, err := s.conn.Execute(query)
        if err != nil {
            return fmt.Errorf("failed to add node %s: %w", node.ID, err)
        }
        result.Close()
    }

    return nil
}
```

---

## Step 2: AddEdges実装

### 2.1 CozoDB参照実装

```go
// cozo_storage.go (122-156行目)
func (s *CozoStorage) AddEdges(ctx context.Context, edges []*storage.Edge) error {
    if len(edges) == 0 {
        return nil
    }

    rows := make([][]any, len(edges))
    for i, e := range edges {
        if e.Properties == nil {
            e.Properties = make(map[string]any)
        }
        // WeightとConfidenceをPropertiesにマッピング
        e.Properties["weight"] = e.Weight
        e.Properties["confidence"] = e.Confidence

        rows[i] = []any{e.SourceID, e.TargetID, e.GroupID, e.Type, e.Properties}
    }

    query := "?[source_id, target_id, group_id, type, properties] <- $data :put edges {source_id, target_id, group_id, type, properties}"
    params := map[string]any{"data": rows}

    if _, err := s.db.Run(query, params); err != nil {
        return fmt.Errorf("failed to add edges: %w", err)
    }
    return nil
}
```

**根拠説明**: CozoDBでは `Weight` と `Confidence` を `Properties` に格納している。KuzuDBでも同様にJSON文字列内に格納する。また、KuzuDBではエッジ作成時に両端のノードが存在する必要がある。エッジのアップサートは、既存エッジを削除してから再作成するパターンで実現する。

> [!NOTE]
> **エッジのMERGEについて**
> KuzuDBのMERGE句はノードに対しては完全に機能するが、エッジの場合は両端ノードのMATCHが必要。
> そのため、エッジのアップサートは MATCH + DELETE + CREATE パターンで実現する。

### 2.2 KuzuDB実装

```go
// AddEdges は、複数のエッジをグラフに追加します。
//
// CozoDB参照実装:
//   e.Properties["weight"] = e.Weight
//   e.Properties["confidence"] = e.Confidence
//   rows[i] = []any{e.SourceID, e.TargetID, e.GroupID, e.Type, e.Properties}
//   query := "?[...] <- $data :put edges {...}"
//
// KuzuDB実装:
//   MATCH (a:GraphNode {id: $source}), (b:GraphNode {id: $target})
//   // 既存エッジの削除 + 新規作成でUPSERTを実現
//
// 注意:
//   - エッジにMERGEを直接使うと、両端ノードが存在しない場合に問題が起こる
//   - そのため MATCH でノードを先に取得し、その後エッジ操作を行う
func (s *KuzuDBStorage) AddEdges(ctx context.Context, edges []*storage.Edge) error {
    // CozoDB同様: 空の場合は何もしない
    if len(edges) == 0 {
        return nil
    }

    for _, edge := range edges {
        // CozoDB同様: WeightとConfidenceをPropertiesに格納
        props := edge.Properties
        if props == nil {
            props = make(map[string]any)
        }
        props["weight"] = edge.Weight
        props["confidence"] = edge.Confidence

        propsJSON, _ := json.Marshal(props)

        // KuzuDB: まず既存エッジを削除（存在する場合）
        // CozoDBの :put はUPSERTなので、同じ動作を実現
        deleteQuery := fmt.Sprintf(`
            MATCH (a:GraphNode {id: '%s'})-[r:GraphEdge]->(b:GraphNode {id: '%s'})
            WHERE r.group_id = '%s' AND r.type = '%s'
            DELETE r
        `,
            escapeString(edge.SourceID),
            escapeString(edge.TargetID),
            escapeString(edge.GroupID),
            escapeString(edge.Type),
        )
        // 削除クエリのエラーは無視（エッジが存在しない場合）
        if result, err := s.conn.Execute(deleteQuery); err == nil && result != nil {
            result.Close()
        }

        // KuzuDB: 両端のノードをMATCHしてからエッジを作成
        createQuery := fmt.Sprintf(`
            MATCH (a:GraphNode {id: '%s'}), (b:GraphNode {id: '%s'})
            CREATE (a)-[r:GraphEdge {
                group_id: '%s',
                type: '%s',
                properties: '%s'
            }]->(b)
        `,
            escapeString(edge.SourceID),
            escapeString(edge.TargetID),
            escapeString(edge.GroupID),
            escapeString(edge.Type),
            escapeString(string(propsJSON)),
        )

        result, err := s.conn.Execute(createQuery)
        if err != nil {
            // ノードが見つからない場合はスキップ（CozoDBと同様の寛容な動作）
            if strings.Contains(err.Error(), "not found") || strings.Contains(err.Error(), "no match") {
                continue
            }
            return fmt.Errorf("failed to add edge %s->%s: %w", edge.SourceID, edge.TargetID, err)
        }
        if result != nil {
            result.Close()
        }
    }

    return nil
}
```

---

## Step 3: GetTriplets実装

### 3.1 CozoDB参照実装

```go
// cozo_storage.go (160-315行目)
func (s *CozoStorage) GetTriplets(ctx context.Context, nodeIDs []string, groupID string) ([]*storage.Triplet, error) {
    if len(nodeIDs) == 0 {
        return nil, nil
    }

    // 1. エッジを取得
    quotedIDs := make([]string, len(nodeIDs))
    for i, id := range nodeIDs {
        quotedIDs[i] = fmt.Sprintf("'%s'", strings.ReplaceAll(id, "'", "\\'"))
    }
    idsList := fmt.Sprintf("[%s]", strings.Join(quotedIDs, ", "))
    quotedGroupID := fmt.Sprintf("'%s'", strings.ReplaceAll(groupID, "'", "\\'"))

    query := fmt.Sprintf(`
        ?[source_id, target_id, group_id, type, properties] := 
            *edges[source_id, target_id, group_id, type, properties],
            (source_id in %s or target_id in %s),
            group_id = %s
    `, idsList, idsList, quotedGroupID)

    res, err := s.db.Run(query, nil)
    // ... エッジからノードIDを収集 ...
    // ... 関連ノードを取得 ...
    // ... トリプレットを構築 ...
}
```

**根拠説明**: CozoDBでは2段階クエリ（1. エッジ取得 → 2. 関連ノード取得）を行っている。KuzuDBでは `MATCH (a)-[r]->(b)` で1回のクエリで取得可能。

### 3.2 KuzuDB実装

```go
// GetTriplets は、指定されたノードIDに関連するトリプレット（ノード-エッジ-ノード）を取得します。
//
// CozoDB参照実装:
//   // 1. エッジを取得
//   query := "?[...] := *edges[...], (source_id in IDs or target_id in IDs), group_id = $gid"
//   edgeRows := s.db.Run(query, nil)
//   // 2. 関連ノードを取得
//   nodeQuery := "?[...] := *nodes[...], id in allIDs"
//   // 3. トリプレットを構築
//
// KuzuDB実装:
//   MATCH (a:GraphNode)-[r:GraphEdge]->(b:GraphNode)
//   WHERE (a.id IN [...] OR b.id IN [...]) AND a.group_id = $gid
//   RETURN a, r, b
//   ※ 1回のクエリで全情報を取得可能
func (s *KuzuDBStorage) GetTriplets(ctx context.Context, nodeIDs []string, groupID string) ([]*storage.Triplet, error) {
    // CozoDB同様: 空の場合はnilを返す
    if len(nodeIDs) == 0 {
        return nil, nil
    }

    // CozoDB同様: IDリストを作成
    quotedIDs := make([]string, len(nodeIDs))
    for i, id := range nodeIDs {
        quotedIDs[i] = fmt.Sprintf("'%s'", escapeString(id))
    }
    idList := "[" + strings.Join(quotedIDs, ", ") + "]"

    // KuzuDBの利点: 1回のクエリでノードとエッジを同時取得
    query := fmt.Sprintf(`
        MATCH (a:GraphNode)-[r:GraphEdge]->(b:GraphNode)
        WHERE (a.id IN %s OR b.id IN %s) AND a.group_id = '%s'
        RETURN 
            a.id, a.group_id, a.type, a.properties,
            r.type, r.properties,
            b.id, b.group_id, b.type, b.properties
    `, idList, idList, escapeString(groupID))

    result, err := s.conn.Execute(query)
    if err != nil {
        return nil, fmt.Errorf("failed to get triplets: %w", err)
    }
    defer result.Close()

    var triplets []*storage.Triplet

    for result.Next() {
        // Source node
        sourceID, _ := result.GetValue(0)
        sourceGroupID, _ := result.GetValue(1)
        sourceType, _ := result.GetValue(2)
        sourcePropsStr, _ := result.GetValue(3)

        // Edge
        edgeType, _ := result.GetValue(4)
        edgePropsStr, _ := result.GetValue(5)

        // Target node
        targetID, _ := result.GetValue(6)
        targetGroupID, _ := result.GetValue(7)
        targetType, _ := result.GetValue(8)
        targetPropsStr, _ := result.GetValue(9)

        // CozoDB同様: プロパティをパース
        sourceProps := parseJSONProperties(getString(sourcePropsStr))
        edgeProps := parseJSONProperties(getString(edgePropsStr))
        targetProps := parseJSONProperties(getString(targetPropsStr))

        // CozoDB同様: Weight と Confidence を抽出
        var weight, confidence float64
        if w, ok := edgeProps["weight"].(float64); ok {
            weight = w
        }
        if c, ok := edgeProps["confidence"].(float64); ok {
            confidence = c
        }

        triplets = append(triplets, &storage.Triplet{
            Source: &storage.Node{
                ID:         getString(sourceID),
                GroupID:    getString(sourceGroupID),
                Type:       getString(sourceType),
                Properties: sourceProps,
            },
            Edge: &storage.Edge{
                SourceID:   getString(sourceID),
                TargetID:   getString(targetID),
                GroupID:    groupID,
                Type:       getString(edgeType),
                Properties: edgeProps,
                Weight:     weight,
                Confidence: confidence,
            },
            Target: &storage.Node{
                ID:         getString(targetID),
                GroupID:    getString(targetGroupID),
                Type:       getString(targetType),
                Properties: targetProps,
            },
        })
    }

    return triplets, nil
}

// parseJSONProperties は、JSON文字列をmap[string]anyにパースします
func parseJSONProperties(s string) map[string]any {
    if s == "" {
        return make(map[string]any)
    }
    var props map[string]any
    if err := json.Unmarshal([]byte(s), &props); err != nil {
        return make(map[string]any)
    }
    return props
}
```

---

## Step 4: StreamDocumentChunks実装

### 4.1 CozoDB参照実装

```go
// cozo_storage.go (332-423行目)
func (s *CozoStorage) StreamDocumentChunks(ctx context.Context, groupID string) (<-chan *storage.ChunkData, <-chan error) {
    chunkChan := make(chan *storage.ChunkData)
    errChan := make(chan error, 1)

    go func() {
        defer close(chunkChan)
        defer close(errChan)

        offset := 0
        for {
            // LIMIT/OFFSET でページネーション
            query := `
                ?[id, text, document_id] := 
                    *nodes[id, group_id, type, properties],
                    group_id = $group_id,
                    type = "DocumentChunk",
                    text = get(properties, "text", ""),
                    document_id = get(properties, "document_id", "")
                :limit $limit
                :offset $offset
            `
            params := map[string]any{
                "group_id": groupID,
                "limit":    chunkFetchBatchSize,
                "offset":   offset,
            }

            result, err := s.db.Run(query, params)
            // ... チャネルに送信 ...
        }
    }()

    return chunkChan, errChan
}
```

**根拠説明**: CozoDBでは goroutine + channel でストリーミングを実現している。KuzuDBでも同様のパターンを使用するが、KuzuDBにはネイティブのページネーションがないため、全件取得してチャネルに送信する。

### 4.2 KuzuDB実装

```go
// StreamDocumentChunks は、DocumentChunkタイプのノードをストリーミングで取得します。
//
// CozoDB参照実装:
//   chunkChan := make(chan *storage.ChunkData)
//   errChan := make(chan error, 1)
//   go func() {
//       query := "?[id, text, document_id] := *nodes[...], type = \"DocumentChunk\" :limit :offset"
//       // ... ページネーションでチャネルに送信 ...
//   }()
//   return chunkChan, errChan
//
// KuzuDB実装:
//   MATCH (n:GraphNode) WHERE n.type = 'DocumentChunk' AND n.group_id = $gid RETURN ...
//   ※ goroutine + channel で非同期に返す
func (s *KuzuDBStorage) StreamDocumentChunks(ctx context.Context, groupID string) (<-chan *storage.ChunkData, <-chan error) {
    // CozoDB同様: バッファなしチャネルを作成
    dataCh := make(chan *storage.ChunkData)
    errCh := make(chan error, 1) // バッファ1: エラーを1回だけ送信

    go func() {
        defer close(dataCh)
        defer close(errCh)

        query := fmt.Sprintf(`
            MATCH (n:GraphNode)
            WHERE n.type = 'DocumentChunk' AND n.group_id = '%s'
            RETURN n.id, n.properties
        `, escapeString(groupID))

        result, err := s.conn.Execute(query)
        if err != nil {
            errCh <- fmt.Errorf("failed to stream document chunks: %w", err)
            return
        }
        defer result.Close()

        for result.Next() {
            // CozoDB同様: コンテキストのキャンセルをチェック
            select {
            case <-ctx.Done():
                errCh <- ctx.Err()
                return
            default:
            }

            id, _ := result.GetValue(0)
            propsStr, _ := result.GetValue(1)

            props := parseJSONProperties(getString(propsStr))

            // CozoDB同様: text と document_id をプロパティから抽出
            text := ""
            documentID := ""
            if t, ok := props["text"].(string); ok {
                text = t
            }
            if d, ok := props["document_id"].(string); ok {
                documentID = d
            }

            // CozoDB同様: 空のテキストはスキップ
            if text == "" {
                continue
            }

            dataCh <- &storage.ChunkData{
                ID:         getString(id),
                Text:       text,
                GroupID:    groupID,
                DocumentID: documentID,
            }
        }
    }()

    return dataCh, errCh
}
```

---

## Step 5: GetDocumentChunkCount実装

### 5.1 CozoDB参照実装

```go
// cozo_storage.go (427-451行目)
func (s *CozoStorage) GetDocumentChunkCount(ctx context.Context, groupID string) (int, error) {
    query := `
        ?[count(id)] := 
            *nodes[id, group_id, type, _],
            group_id = $group_id,
            type = "DocumentChunk"
    `
    params := map[string]any{"group_id": groupID}

    result, err := s.db.Run(query, params)
    if err != nil {
        return 0, fmt.Errorf("CozoDB GetDocumentChunkCount query failed: %w", err)
    }

    if len(result.Rows) > 0 && len(result.Rows[0]) > 0 {
        if count, ok := result.Rows[0][0].(float64); ok {
            return int(count), nil
        }
    }
    return 0, nil
}
```

**根拠説明**: CozoDBでは `count(id)` 集約関数でノード数をカウントしている。KuzuDBでも同様に `RETURN count(n)` を使用する。

### 5.2 KuzuDB実装

```go
// GetDocumentChunkCount は、DocumentChunkの数を取得します。
//
// CozoDB参照実装:
//   query := "?[count(id)] := *nodes[id, group_id, type, _], group_id = $group_id, type = \"DocumentChunk\""
//   result.Rows[0][0].(float64) → int
//
// KuzuDB実装:
//   MATCH (n:GraphNode) WHERE n.type = 'DocumentChunk' AND n.group_id = $gid RETURN count(n)
func (s *KuzuDBStorage) GetDocumentChunkCount(ctx context.Context, groupID string) (int, error) {
    query := fmt.Sprintf(`
        MATCH (n:GraphNode)
        WHERE n.type = 'DocumentChunk' AND n.group_id = '%s'
        RETURN count(n) as cnt
    `, escapeString(groupID))

    result, err := s.conn.Execute(query)
    if err != nil {
        return 0, fmt.Errorf("failed to count document chunks: %w", err)
    }
    defer result.Close()

    // CozoDB同様: 最初の行から取得
    if result.Next() {
        cnt, _ := result.GetValue(0)
        if count, ok := cnt.(int64); ok {
            return int(count), nil
        }
    }

    return 0, nil
}
```

---

## Step 6: GetNodesByType実装

### 6.1 CozoDB参照実装

```go
// cozo_storage.go (453-487行目)
func (s *CozoStorage) GetNodesByType(ctx context.Context, nodeType string, groupID string) ([]*storage.Node, error) {
    query := `
        ?[id, group_id, type, properties] := 
            *nodes[id, group_id, type, properties],
            group_id = $group_id,
            type = $type
    `
    params := map[string]any{"group_id": groupID, "type": nodeType}

    res, err := s.db.Run(query, params)
    // ... ノードを構築 ...
}
```

### 6.2 KuzuDB実装

```go
// GetNodesByType は、指定されたタイプのノードを取得します。
//
// CozoDB参照実装:
//   query := "?[id, group_id, type, properties] := *nodes[...], group_id = $group_id, type = $type"
//
// KuzuDB実装:
//   MATCH (n:GraphNode) WHERE n.type = $type AND n.group_id = $gid RETURN ...
func (s *KuzuDBStorage) GetNodesByType(ctx context.Context, nodeType string, groupID string) ([]*storage.Node, error) {
    query := fmt.Sprintf(`
        MATCH (n:GraphNode)
        WHERE n.type = '%s' AND n.group_id = '%s'
        RETURN n.id, n.group_id, n.type, n.properties
    `, escapeString(nodeType), escapeString(groupID))

    result, err := s.conn.Execute(query)
    if err != nil {
        return nil, fmt.Errorf("failed to get nodes by type: %w", err)
    }
    defer result.Close()

    var nodes []*storage.Node

    // CozoDB同様: 各行をノードに変換
    for result.Next() {
        id, _ := result.GetValue(0)
        gid, _ := result.GetValue(1)
        typ, _ := result.GetValue(2)
        propsStr, _ := result.GetValue(3)

        nodes = append(nodes, &storage.Node{
            ID:         getString(id),
            GroupID:    getString(gid),
            Type:       getString(typ),
            Properties: parseJSONProperties(getString(propsStr)),
        })
    }

    return nodes, nil
}
```

---

## Step 7: GetNodesByEdge実装

### 7.1 CozoDB参照実装

```go
// cozo_storage.go (489-530行目)
func (s *CozoStorage) GetNodesByEdge(ctx context.Context, targetID string, edgeType string, groupID string) ([]*storage.Node, error) {
    query := `
        ?[id, group_id, type, properties] := 
            *edges[source_id, target_id, group_id, edge_type, _],
            target_id = $target_id,
            edge_type = $edge_type,
            group_id = $group_id,
            *nodes[id, group_id, type, properties],
            id = source_id
    `
    params := map[string]any{
        "target_id": targetID,
        "edge_type": edgeType,
        "group_id":  groupID,
    }
    // ...
}
```

### 7.2 KuzuDB実装

```go
// GetNodesByEdge は、指定されたターゲットノードに接続されたノードを取得します。
//
// CozoDB参照実装:
//   query := "?[...] := *edges[source_id, target_id, ...], target_id = $target_id, edge_type = $edge_type, *nodes[id, ...], id = source_id"
//
// KuzuDB実装:
//   MATCH (a:GraphNode)-[r:GraphEdge]->(b:GraphNode)
//   WHERE b.id = $target_id AND r.type = $edge_type
//   RETURN a.id, a.group_id, a.type, a.properties
func (s *KuzuDBStorage) GetNodesByEdge(ctx context.Context, targetID string, edgeType string, groupID string) ([]*storage.Node, error) {
    query := fmt.Sprintf(`
        MATCH (a:GraphNode)-[r:GraphEdge]->(b:GraphNode)
        WHERE b.id = '%s' AND r.type = '%s' AND a.group_id = '%s'
        RETURN a.id, a.group_id, a.type, a.properties
    `, escapeString(targetID), escapeString(edgeType), escapeString(groupID))

    result, err := s.conn.Execute(query)
    if err != nil {
        return nil, fmt.Errorf("failed to get nodes by edge: %w", err)
    }
    defer result.Close()

    var nodes []*storage.Node

    for result.Next() {
        id, _ := result.GetValue(0)
        gid, _ := result.GetValue(1)
        typ, _ := result.GetValue(2)
        propsStr, _ := result.GetValue(3)

        nodes = append(nodes, &storage.Node{
            ID:         getString(id),
            GroupID:    getString(gid),
            Type:       getString(typ),
            Properties: parseJSONProperties(getString(propsStr)),
        })
    }

    return nodes, nil
}
```

---

## Step 8: UpdateEdgeMetrics実装

### 8.1 CozoDB参照実装

```go
// cozo_storage.go (538-603行目)
func (s *CozoStorage) UpdateEdgeMetrics(ctx context.Context, sourceID, targetID, groupID string, weight, confidence float64) error {
    // Step 1: 既存のエッジを取得
    getQuery := `
        ?[source_id, target_id, group_id, type, properties] := 
            *edges[source_id, target_id, group_id, type, properties],
            source_id = $source_id,
            target_id = $target_id,
            group_id = $group_id
    `
    // Step 2: プロパティを更新
    props["weight"] = weight
    props["confidence"] = confidence
    // Step 3: 更新されたエッジを書き戻す
    putQuery := `
        ?[source_id, target_id, group_id, type, properties] <- [[$source_id, $target_id, $group_id, $type, $properties]]
        :put edges {source_id, target_id, group_id, type, properties}
    `
}
```

**根拠説明**: CozoDBには `UPDATE` がないため、GET → 更新 → PUT の3段階で更新している。KuzuDBでは `SET` 句で直接プロパティを更新できるが、同様のパターンを使用する。

### 8.2 KuzuDB実装

```go
// UpdateEdgeMetrics は、エッジの重みと信頼度を更新します。
//
// CozoDB参照実装:
//   // Step 1: 既存のエッジを取得
//   getQuery := "?[...] := *edges[...], source_id = $source_id, target_id = $target_id"
//   // Step 2: プロパティを更新
//   props["weight"] = weight
//   props["confidence"] = confidence
//   // Step 3: 書き戻す
//   putQuery := "?[...] <- [[...]] :put edges {...}"
//
// KuzuDB実装:
//   MATCH (a)-[r:GraphEdge]->(b) WHERE a.id = ... SET r.properties = ...
func (s *KuzuDBStorage) UpdateEdgeMetrics(ctx context.Context, sourceID, targetID, groupID string, weight, confidence float64) error {
    // CozoDB同様: Step 1 - 既存のプロパティを取得
    query := fmt.Sprintf(`
        MATCH (a:GraphNode)-[r:GraphEdge]->(b:GraphNode)
        WHERE a.id = '%s' AND b.id = '%s' AND r.group_id = '%s'
        RETURN r.properties
    `, escapeString(sourceID), escapeString(targetID), escapeString(groupID))

    result, err := s.conn.Execute(query)
    if err != nil {
        return fmt.Errorf("failed to get edge for update: %w", err)
    }

    var propsStr string
    if result.Next() {
        p, _ := result.GetValue(0)
        propsStr = getString(p)
    } else {
        result.Close()
        return fmt.Errorf("edge not found: %s -> %s", sourceID, targetID)
    }
    result.Close()

    // CozoDB同様: Step 2 - プロパティを更新
    props := parseJSONProperties(propsStr)
    props["weight"] = weight
    props["confidence"] = confidence
    newPropsJSON, _ := json.Marshal(props)

    // CozoDB同様: Step 3 - 更新クエリ
    updateQuery := fmt.Sprintf(`
        MATCH (a:GraphNode)-[r:GraphEdge]->(b:GraphNode)
        WHERE a.id = '%s' AND b.id = '%s' AND r.group_id = '%s'
        SET r.properties = '%s'
    `, escapeString(sourceID), escapeString(targetID), escapeString(groupID), escapeString(string(newPropsJSON)))

    result, err = s.conn.Execute(updateQuery)
    if err != nil {
        return fmt.Errorf("failed to update edge metrics: %w", err)
    }
    result.Close()

    return nil
}
```

---

## Step 9: DeleteEdge実装

### 9.1 CozoDB参照実装

```go
// cozo_storage.go (605-638行目)
func (s *CozoStorage) DeleteEdge(ctx context.Context, sourceID, targetID, groupID string) error {
    // 削除対象のエッジタイプを取得
    queryFind := `
        ?[type] := *edges[source_id, target_id, group_id, type, _],
        source_id = $source_id,
        target_id = $target_id,
        group_id = $group_id
    `
    // :rm で削除
    rmQuery := "?[source_id, target_id, group_id, type] <- $data :rm edges {source_id, target_id, group_id, type}"
}
```

### 9.2 KuzuDB実装

```go
// DeleteEdge は、指定されたエッジを削除します。
//
// CozoDB参照実装:
//   rmQuery := "?[source_id, target_id, group_id, type] <- $data :rm edges {...}"
//
// KuzuDB実装:
//   MATCH (a)-[r:GraphEdge]->(b) WHERE a.id = ... DELETE r
func (s *KuzuDBStorage) DeleteEdge(ctx context.Context, sourceID, targetID, groupID string) error {
    query := fmt.Sprintf(`
        MATCH (a:GraphNode)-[r:GraphEdge]->(b:GraphNode)
        WHERE a.id = '%s' AND b.id = '%s' AND r.group_id = '%s'
        DELETE r
    `, escapeString(sourceID), escapeString(targetID), escapeString(groupID))

    result, err := s.conn.Execute(query)
    if err != nil {
        return fmt.Errorf("failed to delete edge: %w", err)
    }
    result.Close()

    return nil
}
```

---

## Step 10: DeleteNode実装

### 10.1 CozoDB参照実装

```go
// cozo_storage.go (640-661行目)
func (s *CozoStorage) DeleteNode(ctx context.Context, nodeID, groupID string) error {
    // CozoDBの:rmはリレーションの全カラムを指定する必要がある
    query := `
        ?[id, group_id, type, properties] := 
            *nodes[id, group_id, type, properties],
            id = $id,
            group_id = $group_id
        :rm nodes {id, group_id, type, properties}
    `
}
```

### 10.2 KuzuDB実装

```go
// DeleteNode は、指定されたノードを削除します。
//
// CozoDB参照実装:
//   query := "?[id, group_id, type, properties] := *nodes[...], id = $id :rm nodes {...}"
//
// KuzuDB実装:
//   まず関連エッジを削除、次にノードを削除
func (s *KuzuDBStorage) DeleteNode(ctx context.Context, nodeID, groupID string) error {
    // KuzuDB: まず関連するエッジを削除（CozoDBでは暗黙的に処理される可能性あり）
    edgeQuery := fmt.Sprintf(`
        MATCH (n:GraphNode)-[r:GraphEdge]-()
        WHERE n.id = '%s' AND n.group_id = '%s'
        DELETE r
    `, escapeString(nodeID), escapeString(groupID))

    result, err := s.conn.Execute(edgeQuery)
    if err != nil {
        // エッジがない場合のエラーは無視
        if !strings.Contains(err.Error(), "not found") {
            return fmt.Errorf("failed to delete related edges: %w", err)
        }
    }
    if result != nil {
        result.Close()
    }

    // ノードを削除
    nodeQuery := fmt.Sprintf(`
        MATCH (n:GraphNode)
        WHERE n.id = '%s' AND n.group_id = '%s'
        DELETE n
    `, escapeString(nodeID), escapeString(groupID))

    result, err = s.conn.Execute(nodeQuery)
    if err != nil {
        return fmt.Errorf("failed to delete node: %w", err)
    }
    result.Close()

    return nil
}
```

---

## Step 11: GetEdgesByNode実装

### 11.1 CozoDB参照実装

```go
// cozo_storage.go (663-713行目)
func (s *CozoStorage) GetEdgesByNode(ctx context.Context, nodeID string, groupID string) ([]*storage.Edge, error) {
    query := fmt.Sprintf(`
        ?[source_id, target_id, group_id, type, properties] := 
            *edges[source_id, target_id, group_id, type, properties],
            (source_id = %s or target_id = %s),
            group_id = %s
    `, quotedID, quotedID, quotedGroupID)
}
```

### 11.2 KuzuDB実装

```go
// GetEdgesByNode は、指定されたノードに接続されたエッジを取得します。
//
// CozoDB参照実装:
//   query := "?[...] := *edges[...], (source_id = $id or target_id = $id), group_id = $gid"
//
// KuzuDB実装:
//   MATCH (a:GraphNode)-[r:GraphEdge]-(b:GraphNode) WHERE (a.id = $id OR b.id = $id)
func (s *KuzuDBStorage) GetEdgesByNode(ctx context.Context, nodeID string, groupID string) ([]*storage.Edge, error) {
    // CozoDB同様: 出力エッジと入力エッジの両方を取得
    query := fmt.Sprintf(`
        MATCH (a:GraphNode)-[r:GraphEdge]->(b:GraphNode)
        WHERE (a.id = '%s' OR b.id = '%s') AND r.group_id = '%s'
        RETURN a.id, b.id, r.group_id, r.type, r.properties
    `, escapeString(nodeID), escapeString(nodeID), escapeString(groupID))

    result, err := s.conn.Execute(query)
    if err != nil {
        return nil, fmt.Errorf("failed to get edges: %w", err)
    }
    defer result.Close()

    var edges []*storage.Edge

    for result.Next() {
        sourceID, _ := result.GetValue(0)
        targetID, _ := result.GetValue(1)
        gid, _ := result.GetValue(2)
        edgeType, _ := result.GetValue(3)
        propsStr, _ := result.GetValue(4)

        props := parseJSONProperties(getString(propsStr))

        // CozoDB同様: weight と confidence を抽出
        var weight, confidence float64
        if w, ok := props["weight"].(float64); ok {
            weight = w
        }
        if c, ok := props["confidence"].(float64); ok {
            confidence = c
        }

        edges = append(edges, &storage.Edge{
            SourceID:   getString(sourceID),
            TargetID:   getString(targetID),
            GroupID:    getString(gid),
            Type:       getString(edgeType),
            Properties: props,
            Weight:     weight,
            Confidence: confidence,
        })
    }

    return edges, nil
}
```

---

## Step 12: GetOrphanNodes実装

### 12.1 CozoDB参照実装

```go
// cozo_storage.go (745-795行目)
func (s *CozoStorage) GetOrphanNodes(ctx context.Context, groupID string, gracePeriod time.Duration) ([]*storage.Node, error) {
    cutoffTime := time.Now().Add(-gracePeriod).Format(time.RFC3339)

    // 孤立ノード検出クエリ
    // CozoDBの否定(not)演算子を使用
    query := fmt.Sprintf(`
        ?[id, type, props] := 
            *nodes[id, group_id, type, props],
            group_id = %s,
            created_at = get(props, "created_at", ""),
            created_at != "",
            created_at < %s,
            not *edges[id, _, %s, _, _],
            not *edges[_, id, %s, _, _]
    `, quotedGroupID, quotedCutoffTime, quotedGroupID, quotedGroupID)
}
```

**根拠説明**: CozoDBでは `not *edges[...]` で否定パターンマッチングを使用している。KuzuDBでは `NOT EXISTS { MATCH ... }` を使用する。

### 12.2 KuzuDB実装

```go
// GetOrphanNodes は、エッジを持たない孤立ノードを取得します。
//
// CozoDB参照実装:
//   query := "?[id, type, props] := *nodes[...], group_id = $gid, not *edges[id, _, ...], not *edges[_, id, ...]"
//   ※ Datalogの否定(not)演算子を使用
//
// KuzuDB実装:
//   MATCH (n:GraphNode)
//   WHERE n.group_id = $gid AND NOT EXISTS { MATCH (n)-[:GraphEdge]-() }
//   ※ Cypherの NOT EXISTS を使用
func (s *KuzuDBStorage) GetOrphanNodes(ctx context.Context, groupID string, gracePeriod time.Duration) ([]*storage.Node, error) {
    // CozoDB同様: 猶予期間のカットオフ時刻を計算
    cutoffTime := time.Now().Add(-gracePeriod).Format(time.RFC3339)

    query := fmt.Sprintf(`
        MATCH (n:GraphNode)
        WHERE n.group_id = '%s'
        AND NOT EXISTS { MATCH (n)-[:GraphEdge]-() }
        RETURN n.id, n.group_id, n.type, n.properties
    `, escapeString(groupID))

    result, err := s.conn.Execute(query)
    if err != nil {
        return nil, fmt.Errorf("failed to get orphan nodes: %w", err)
    }
    defer result.Close()

    var orphans []*storage.Node

    for result.Next() {
        id, _ := result.GetValue(0)
        gid, _ := result.GetValue(1)
        typ, _ := result.GetValue(2)
        propsStr, _ := result.GetValue(3)

        props := parseJSONProperties(getString(propsStr))

        // CozoDB同様: GracePeriodのチェック
        if createdAtStr, ok := props["created_at"].(string); ok {
            if createdAt, err := time.Parse(time.RFC3339, createdAtStr); err == nil {
                if createdAt.Format(time.RFC3339) >= cutoffTime {
                    continue // 猶予期間内なのでスキップ
                }
            }
        }

        orphans = append(orphans, &storage.Node{
            ID:         getString(id),
            GroupID:    getString(gid),
            Type:       getString(typ),
            Properties: props,
        })
    }

    return orphans, nil
}
```

---

## 13. 成功条件チェックリスト

### Phase-10E 完了条件

- [ ] AddNodes が正常に動作（CozoDB :put nodes と同等）
- [ ] AddEdges が正常に動作（CozoDB :put edges と同等）
- [ ] GetTriplets が正常に動作（CozoDB *edges + *nodes と同等）
- [ ] StreamDocumentChunks が正常に動作（CozoDB ページネーション + channel と同等）
- [ ] GetDocumentChunkCount が正常に動作（CozoDB count() と同等）
- [ ] GetNodesByType が正常に動作（CozoDB *nodes WHERE type と同等）
- [ ] GetNodesByEdge が正常に動作（CozoDB *edges JOIN *nodes と同等）
- [ ] UpdateEdgeMetrics が正常に動作（CozoDB GET + PUT と同等）
- [ ] DeleteEdge が正常に動作（CozoDB :rm edges と同等）
- [ ] DeleteNode が正常に動作（CozoDB :rm nodes と同等）
- [ ] GetEdgesByNode が正常に動作（CozoDB *edges WHERE と同等）
- [ ] GetOrphanNodes が正常に動作（CozoDB not *edges と同等）
- [ ] `make build` がエラーなしで成功
- [ ] `test-kuzudb-graph` テストがPASSED
- [ ] 既存のDuckDB+CozoDBテストが引き続き動作

---

## 14. 次のフェーズへの準備

Phase-10Eが完了したら、GraphStorageの全機能がKuzuDBで動作する状態となる。
Phase-10Fでは、CogneeServiceのモード切替統合と全体テストを行う。
