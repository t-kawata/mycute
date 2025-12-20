# Phase-02A: DB Infrastructure Layer

## 0. 前提条件 (Prerequisites)

### 開発環境
- [ ] `make` コマンドが実行可能であること。
- [ ] CGOビルド環境（DuckDB, CozoDBのライブラリ）が整っていること。

## 1. 目的
Phase-02の最初のステップとして、**DuckDB (Vector/Metadata)** と **CozoDB (Graph)** の永続化層を確立します。
本ドキュメントでは、単なるスキーマ定義だけでなく、**「なぜその設計なのか」**をオリジナルのPython実装の解析に基づいて解説します。

## 2. 実装詳細とPython解析

### 2.1. DuckDB (Metadata & Vector Storage)

**ファイル**: `src/pkg/cognee/db/duckdb/schema.sql`

#### Python実装の解析 (`cognee/tasks/storage/index_data_points.py`)
Cogneeは単にチャンクを保存するだけでなく、`DataPoint`（ノード）の特定のプロパティ（例: `Entity.name`）ごとにベクトルインデックスを作成しています。

```python
# cognee/tasks/storage/index_data_points.py
for field_name in data_point.metadata["index_fields"]:
    index_name = f"{data_point_type.__name__}_{field_name}"
    await vector_engine.create_vector_index(data_point_type.__name__, field_name)
    # ... batch indexing ...
```

**Why (設計根拠)**:
これにより、ユーザーのクエリが「エンティティ名」や「要約テキスト」にヒットするようになり、単なる全文検索よりも高精度なグラフエントリポイントの特定が可能になります。
Go実装では、これを `vectors` テーブルの `collection_name` カラムで表現し、単一テーブルで効率的に管理します。

#### Go実装方針 (Schema Definition)
DuckDBのVSS拡張を使用し、ベクトル検索を可能にします。

```sql
-- ベクトル拡張のインストールとロードは main.go で行われるため、ここには記述しません。
-- INSTALL vss;
-- LOAD vss;

-- Files metadata
CREATE TABLE IF NOT EXISTS data (
    id UUID PRIMARY KEY,
    name VARCHAR,
    raw_data_location VARCHAR,
    extension VARCHAR,
    mime_type VARCHAR,
    content_hash VARCHAR,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Extracted text content
CREATE TABLE IF NOT EXISTS documents (
    id UUID PRIMARY KEY,
    data_id UUID REFERENCES data(id),
    text VARCHAR,
    metadata JSON
);

-- Text chunks
CREATE TABLE IF NOT EXISTS chunks (
    id UUID PRIMARY KEY,
    document_id UUID REFERENCES documents(id),
    text VARCHAR,
    chunk_index INTEGER,
    chunk_size INTEGER,
    metadata JSON
);

-- Vector storage (Universal)
-- collection_name = Python版の index_name (例: "Entity_name", "DocumentChunk_text")
CREATE TABLE IF NOT EXISTS vectors (
    id UUID, -- References chunks(id) or nodes(id)
    collection_name VARCHAR, 
    text VARCHAR, -- For keyword search / debug
    embedding FLOAT[1536], -- Adjust dimension based on model
    PRIMARY KEY (id, collection_name)
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_vectors_collection ON vectors(collection_name);
-- HNSW Index for vector similarity (Cosine)
CREATE INDEX IF NOT EXISTS idx_vectors_embedding ON vectors USING HNSW (embedding) WITH (metric = 'cosine');
```

### 2.2. CozoDB (Graph Storage)

**ファイル**: `src/pkg/cognee/db/cozodb/schema.go` (または初期化ロジック内)

#### Python実装の解析 (`cognee/tasks/storage/add_data_points.py`)
グラフへの保存時に最も重要なのは**重複排除 (Deduplication)** です。

```python
# cognee/tasks/storage/add_data_points.py
nodes, edges = deduplicate_nodes_and_edges(nodes, edges)
await graph_engine.add_nodes(nodes)
await graph_engine.add_edges(edges)
```

**Why (設計根拠)**:
LLMは同じエンティティ（例: "Apple"）を何度も抽出しますが、グラフ上ではこれらを単一のノードに統合しなければなりません。これを行わないと、グラフが断片化し、接続性が失われます。
Go実装では、CozoDBの `:put` (Upsert) オペレーションを使用することで、DBレベルでこの重複排除を保証します。

#### Go実装方針 (Datalog Schema & Logic)

**Schema (Stored Relations)**:
```datalog
::create nodes {
    id: String,
    type: String,
    properties: Json
}

::create edges {
    source_id: String,
    target_id: String,
    type: String,
    properties: Json
}
```

**Upsert Logic (Nodes)**:
`:put` を使用して、IDベースでノードを保存・更新します。これはPython版の `deduplicate_nodes_and_edges` と同等の効果を持ちます。
```datalog
:put nodes {id, type, properties}
```

**Get Triplets Logic (Search)**:
指定したノードIDリストに関連するエッジ（1-hop）を取得します。これは検索時のコンテキスト構築に使用されます。
```datalog
?[source_id, target_id, type, properties] := 
  *edges[source_id, target_id, type, properties],
  or(source_id in $node_ids, target_id in $node_ids)
```

### 2.3. Storage Interfaces (Go Interfaces)

**ファイル**: `src/pkg/cognee/storage/interfaces.go`

Phase-02B以降で使用されるStorage層の抽象化インターフェースを定義します。

#### VectorStorage Interface
```go
package storage

import (
    "context"
    "time"
)

type Data struct {
    ID                string
    Name              string
    Extension         string
    ContentHash       string
    RawDataLocation   string
    CreatedAt         time.Time
}

type Chunk struct {
    ID          string
    DocumentID  string
    Text        string
    ChunkIndex  int
    Embedding   []float32
}

type SearchResult struct {
    ID       string
    Text     string
    Distance float64
}

type VectorStorage interface {
    // Metadata operations
    SaveData(ctx context.Context, data *Data) error
    Exists(ctx context.Context, contentHash string) bool
    GetDataByID(ctx context.Context, id string) (*Data, error)
    
    // Vector operations
    SaveChunk(ctx context.Context, chunk *Chunk) error
    Search(ctx context.Context, collectionName string, vector []float32, k int) ([]*SearchResult, error)
}
```

#### GraphStorage Interface
```go
type Node struct {
    ID         string
    Type       string
    Properties map[string]any
}

type Edge struct {
    SourceID   string
    TargetID   string
    Type       string
    Properties map[string]any
}

type Triplet struct {
    Source *Node
    Edge   *Edge
    Target *Node
}

type GraphStorage interface {
    AddNodes(ctx context.Context, nodes []*Node) error
    AddEdges(ctx context.Context, edges []*Edge) error
    GetTriplets(ctx context.Context, nodeIDs []string) ([]*Triplet, error)
}
```

## 3. 開発ステップ & Checkpoints

### Checkpoint 1: DB接続と初期化
- [ ] `make build` が成功すること。
- [ ] アプリケーション起動時に DuckDB と CozoDB のインスタンスが正常に初期化されること。
- [ ] `schema.sql` が DuckDB に適用され、テーブルが作成されていること。
- [ ] CozoDB に `nodes`, `edges` リレーションが作成されていること。

### Checkpoint 1の詳細な検証手順

以下のSQLクエリを実行して、スキーマが正しく適用されているか確認してください。

#### DuckDBの検証
```go
// src/main.go または一時的なテストコード
rows, err := duckDB.Query("SHOW TABLES")
// Expected: data, documents, chunks, vectors

rows, err := duckDB.Query("SELECT COUNT(*) FROM pragma_database_list()")
// VSS拡張がロードされているか確認
```

#### CozoDBの検証
```go
// CozoDBのクエリ実行
result, err := cozoDB.Run("::relations")
// Expected: nodes, edges が表示されること
```

### Checkpoint 2: インターフェースとCRUDの実装
- [ ] `src/pkg/cognee/storage/interfaces.go` が作成され、上記インターフェースが定義されていること。
- [ ] `src/pkg/cognee/db/duckdb/duckdb_storage.go` が `VectorStorage` インターフェースを実装していること。
- [ ] `src/pkg/cognee/db/cozodb/cozo_storage.go` が `GraphStorage` インターフェースを実装していること。
- [ ] 単体テスト（または `src/main.go` の一時的なコード）で、データの保存と取得ができることを確認する。

## 4. 実行コマンド
開発中は以下のコマンドでビルドと実行を行ってください。

```bash
# ビルド
make build

# 実行 (引数は適宜変更)
make run ARGS="version"
```
