# Cognee Go Implementation: Phase-03 Development Directives

## 0. はじめに (Context & Prerequisites)

本ドキュメントは、**Phase-03** における開発指示書です。
Phase-02の振り返りにより、現在の実装は「グラフデータベースを使用しているが、検索ロジックは単純なRAG（チャンク検索）である」ことが判明しました。
Phase-03の絶対的な目的は、**「真のGraphRAG」** を実装し、オリジナルCogneeの検索ロジックを完全に再現することです。

### 前提条件
*   Phase-02が完了し、`make run ARGS="cognify"` が正常に動作していること。
*   `src/pkg/cognee/db/cozodb/cozo_storage.go` に `GetTriplets` が既に実装されていること（Phase-02Aで実装済み）。

---

## 1. 開発の目的 (Objectives)

Phase-03のゴールは、以下の「GraphRAGのミッシングリンク」を埋めることです。

1.  **Node Embedding (Cognify)**: 抽出されたエンティティ（ノード）の名前をベクトル化し、DuckDBにインデックスを作成する。
2.  **Graph Traversal Search (Search)**: 検索時に「チャンク」だけでなく「ノード」をベクトル検索し、そこからグラフ（CozoDB）を辿って関連するトリプレットを取得する。

これにより、単なるキーワードマッチングやテキスト類似度を超えた、**「意味的な関係性に基づく検索」** を実現します。

---

## 2. 実装詳細とPython解析

### Phase-03A: Node Embedding & Indexing

**ファイル**: `src/pkg/cognee/tasks/storage/storage_task.go` (修正)

#### Python実装の解析 (`cognee/tasks/storage/index_data_points.py`)
Cogneeは、`DataPoint`（ノード）の特定のプロパティ（主に `name`）に対してベクトルインデックスを作成します。

```python
# cognee/tasks/storage/index_data_points.py
async def index_data_points(data_points, ...):
    for data_point in data_points:
        # "Entity" タイプのノードの "name" プロパティなどをインデックス対象とする
        for field_name in data_point.metadata["index_fields"]:
            # 例: collection_name = "Entity_name"
            await vector_engine.create_vector_index(data_point_type.__name__, field_name)
            await vector_engine.index_data_points([data_point])
```

#### Go実装方針
`StorageTask` を拡張し、CozoDBにノードを保存する際、同時にDuckDBにもノードのベクトルを保存します。

1.  **構造体の変更**: `StorageTask` に `Embedder` フィールドを追加します。
2.  **コンストラクタの変更**: `NewStorageTask` で `Embedder` を受け取るようにします。
3.  **Runメソッドの変更**: `GraphData.Nodes` をループし、`name` プロパティを持つノードの埋め込みを生成して保存します。

```go
// src/pkg/cognee/tasks/storage/storage_task.go

type StorageTask struct {
    VectorStorage storage.VectorStorage
    GraphStorage  storage.GraphStorage
    Embedder      storage.Embedder // [NEW]
}

func NewStorageTask(vectorStorage storage.VectorStorage, graphStorage storage.GraphStorage, embedder storage.Embedder) *StorageTask {
    return &StorageTask{
        VectorStorage: vectorStorage,
        GraphStorage:  graphStorage,
        Embedder:      embedder, // [NEW]
    }
}

func (t *StorageTask) Run(ctx context.Context, input any) (any, error) {
    output, ok := input.(*storage.CognifyOutput)
    if !ok {
        return nil, fmt.Errorf("expected *storage.CognifyOutput input, got %T", input)
    }

    // 1. Save Chunks (Existing)
    // ...

    // 2. Save Graph (Nodes/Edges)
    if output.GraphData != nil {
        // ... (Existing CozoDB saving logic) ...

        // [NEW] 3. Index Nodes (Embeddings)
        fmt.Printf("Indexing %d nodes...\n", len(output.GraphData.Nodes))
        for _, node := range output.GraphData.Nodes {
            // プロパティ "name" をチェック
            nameInterface, ok := node.Properties["name"]
            if !ok {
                continue
            }
            name, ok := nameInterface.(string)
            if !ok || name == "" {
                continue
            }

            // Generate Embedding for Node Name
            // Note: バッチ処理が理想的ですが、まずはループで実装します。
            embedding, err := t.Embedder.EmbedQuery(ctx, name)
            if err != nil {
                fmt.Printf("Warning: failed to embed node %s: %v\n", name, err)
                continue
            }
            
            // Save to DuckDB
            // Collection: "Entity_name"
            // Chunk構造体を再利用して保存します（DuckDBのvectorsテーブルはChunk構造に対応しているため）
            nodeVector := &storage.Chunk{
                ID:         node.ID,
                Text:       name, // デバッグ用に名前を保存
                Embedding:  embedding,
                // DocumentIDなどは空で良い、またはダミーを入れる
            }
            
            // SaveVectorメソッドがない場合はSaveChunkを使うか、VectorStorageにメソッドを追加する
            // DuckDBStorageの実装を見ると、SaveChunkは chunks テーブルと vectors テーブルの両方に書き込む可能性があります。
            // vectorsテーブルだけに書き込むメソッド `SaveVector` を追加するのが最もクリーンですが、
            // ここでは既存の `SaveChunk` を流用し、collection_name を指定できるように `VectorStorage` を拡張することを推奨します。
            // しかし、インターフェース変更を避けるため、`SaveChunk` が `vectors` テーブルへの書き込みも行っていることを利用します。
            // ただし、`SaveChunk` は `chunks` テーブルへの外部キー制約(`document_id`)があるかもしれません。
            
            // ★重要★: DuckDBのスキーマ設計 (`docs/DIRECTONS-PHASE-02A.md`) では、
            // vectorsテーブルは `id` と `collection_name` を持ちます。
            // `VectorStorage` インターフェースには `SaveChunk` しかありません。
            // Phase-03では `SaveEmbedding(ctx, collectionName, id, text, vector)` メソッドを `VectorStorage` に追加することを強く推奨します。
            
            if err := t.VectorStorage.SaveEmbedding(ctx, "Entity_name", node.ID, name, embedding); err != nil {
                 return nil, fmt.Errorf("failed to save node embedding: %w", err)
            }
        }
    }
    return output, nil
}
```

**※補足**: `VectorStorage` インターフェースに `SaveEmbedding` メソッドを追加し、`DuckDBStorage` で実装してください。
```go
// src/pkg/cognee/storage/interfaces.go
type VectorStorage interface {
    // ...
    SaveEmbedding(ctx context.Context, collectionName, id, text string, vector []float32) error
}
```

---

### Phase-03B: Graph Traversal Search

**ファイル**: `src/pkg/cognee/tools/search/graph_completion.go` (修正)

#### Python実装の解析 (`cognee/modules/retrieval/utils/brute_force_triplet_search.py`)
検索処理は、ベクトル検索で見つけたノードを起点に、グラフを探索してコンテキストを拡張します。

#### Go実装方針
`GraphCompletionTool.Search` を大幅に改修し、マルチコレクション検索とグラフトラバーサルを実装します。

```go
// src/pkg/cognee/tools/search/graph_completion.go

func (t *GraphCompletionTool) Search(ctx context.Context, query string) (string, error) {
    queryVector, err := t.Embedder.EmbedQuery(ctx, query)
    if err != nil {
        return "", fmt.Errorf("failed to embed query: %w", err)
    }

    // 1. Vector Search (Parallel execution using errgroup is recommended)
    
    // A. Search Chunks (Existing)
    chunkResults, err := t.VectorStorage.Search(ctx, "DocumentChunk_text", queryVector, 5)
    if err != nil {
        return "", fmt.Errorf("chunk search failed: %w", err)
    }
    
    // B. Search Nodes [NEW]
    nodeResults, err := t.VectorStorage.Search(ctx, "Entity_name", queryVector, 5)
    if err != nil {
        return "", fmt.Errorf("node search failed: %w", err)
    }

    // 2. Graph Traversal [NEW]
    var nodeIDs []string
    uniqueNodes := make(map[string]bool)
    
    for _, res := range nodeResults {
        if !uniqueNodes[res.ID] {
            nodeIDs = append(nodeIDs, res.ID)
            uniqueNodes[res.ID] = true
        }
    }
    
    // Get Triplets from GraphStorage
    // CozoStorage.GetTriplets is already implemented
    triplets, err := t.GraphStorage.GetTriplets(ctx, nodeIDs)
    if err != nil {
        return "", fmt.Errorf("graph traversal failed: %w", err)
    }

    // 3. Construct Context
    var contextBuilder strings.Builder
    
    contextBuilder.WriteString("### Relevant Text Chunks:\n")
    if len(chunkResults) == 0 {
        contextBuilder.WriteString("No relevant text chunks found.\n")
    }
    for _, res := range chunkResults {
        contextBuilder.WriteString("- " + res.Text + "\n")
    }

    contextBuilder.WriteString("\n### Knowledge Graph Connections:\n")
    if len(triplets) == 0 {
        contextBuilder.WriteString("No relevant graph connections found.\n")
    }
    for _, triplet := range triplets {
        // Format: Source --[Type]--> Target
        // Ensure properties exist and are strings
        sourceName := getName(triplet.Source)
        targetName := getName(triplet.Target)
        
        contextBuilder.WriteString(fmt.Sprintf("- %s --[%s]--> %s\n", 
            sourceName, 
            triplet.Edge.Type, 
            targetName))
    }

    // 4. Generate Answer
    prompt := fmt.Sprintf(prompts.GraphContextForQuestionPrompt, query, contextBuilder.String())
    
    response, err := llms.GenerateFromSinglePrompt(ctx, t.LLM, prompt,
        llms.WithTemperature(0),
    )
    if err != nil {
        return "", fmt.Errorf("failed to generate answer: %w", err)
    }

    return response, nil
}

// Helper to safely get name from node properties
func getName(node *storage.Node) string {
    if node == nil || node.Properties == nil {
        return "Unknown"
    }
    if name, ok := node.Properties["name"].(string); ok {
        return name
    }
    return node.ID // Fallback to ID
}
```

---

## 3. 開発ステップ

### Step 1: VectorStorageインターフェースの拡張
*   `src/pkg/cognee/storage/interfaces.go`: `SaveEmbedding` メソッドを追加。
*   `src/pkg/cognee/db/duckdb/duckdb_storage.go`: `SaveEmbedding` を実装（`vectors` テーブルへの INSERT）。

### Step 2: StorageTaskの改修 (Node Embedding)
*   `src/pkg/cognee/tasks/storage/storage_task.go`: `Embedder` フィールド追加、`Run` メソッドでノード埋め込み保存ロジック追加。
*   `src/pkg/cognee/cognee.go`: `NewStorageTask` 呼び出し時に `s.Embedder` を渡すように修正。

### Step 3: GraphCompletionToolの改修 (Graph Search)
*   `src/pkg/cognee/tools/search/graph_completion.go`: マルチコレクション検索とグラフトラバーサルロジックを実装。

### Step 4: 検証
1.  **DBリセット**: `rm -rf src/data/cognee_v2.db src/data/graph.cozodb`
2.  **データ投入**: `make run ARGS="add"`
3.  **グラフ構築**: `make run ARGS="cognify"`
    *   ログに "Indexing X nodes..." が表示されることを確認。
4.  **検索**: `make run ARGS="search -q 'Cogneeとは？'"`
    *   ログに "Searching with vector..." に加え、"Found X triplets" が表示されることを確認。
5.  **ベンチマーク**: `make run ARGS="benchmark -j test_data/QA.json"`
    *   正解率が維持、または向上していることを確認。特に「関連情報が見つからない」ケースでの挙動を確認。

---

## 4. エラーハンドリング & 注意点

*   **Embedderのコスト**: ノード数が多い場合、埋め込みAPIの呼び出し回数が増えます。今回はテストデータが少量なのでループ処理で問題ありませんが、将来的な大量データ対応のために `TODO: Implement batch embedding` コメントを残してください。
*   **DuckDBのロック**: `SaveEmbedding` 実装時、並行書き込みによるロックエラーに注意してください。DuckDBは単一ライター制約があるため、必要に応じてMutexで保護するか、コネクションプール設定を見直してください（現状の `main.go` の設定で十分か確認）。
*   **CozoDBクエリ**: `GetTriplets` は既に実装済みですが、もしエラーが出る場合はクエリ内の引用符のエスケープ処理などを再確認してください。
