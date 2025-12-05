# Cognee Go Implementation: Phase-05 Development Directives

## 0. はじめに (Context & Prerequisites)

本ドキュメントは、**Phase-05** における開発指示書の決定版です。
これまでのPhase-04までの実装は「シングルユーザー・全データ共有」であり、マルチテナント環境において致命的なデータ混在リスクがありました。
Phase-05の目的は、このアーキテクチャを根本から刷新し、**「物理的かつ論理的に完全に分離された保守性の高いデータ基盤」** を構築することです。

実装時の迷いをゼロにするため、本ドキュメントのコードスニペットと指示に一言一句従ってください。

---

## 1. 開発の目的 (Objectives)

1.  **物理的パーティション (Physical Partitioning)**:
    *   `CogneeService` インスタンスは、生成時に指定された特定の **DBファイルパス以外へのアクセスを物理的に遮断** します。
    *   これにより、インスタンスAが他人のDBファイルを読むミスをコードレベルで防ぎます。

2.  **論理的パーティション (Logical Partitioning)**:
    *   単一のDBファイル内においても、`group_id` (例: `"12-yokohama-AB"`) カラムによるフィルタリングを全クエリで強制します。

3.  **決定論的ID生成 (Deterministic ID Generation)**:
    *   データのハッシュ値とパーティションIDを組み合わせたUUID v5を生成し、データの重複登録を完璧に防ぎます。

---

## 2. 開発ロードマップ (Implementation Steps)

以下の順序で実装を進めます。

1.  **Step 1**: スキーマと型定義の更新 (`schema.sql`, `models.go`)
2.  **Step 2**: Storage層の改修 (`duckdb_storage.go`, `cozo_storage.go`)
3.  **Step 3**: 決定論的IDロジックの実装 (`ingest_task.go`)
4.  **Step 4**: `CogneeService` の構造化とAPI改修 (`cognee.go`)
5.  **Step 5**: 検証スクリプトによる厳密なテスト

---

## 3. 実装詳細 (Detailed Implementation)

### Step 1: スキーマと型定義の更新

まず、全てのデータモデルに `group_id` を追加し、主キーを変更します。

#### 1-1. DuckDB Schema (`src/pkg/cognee/db/duckdb/schema.sql`)

既存のファイルを以下の内容で **完全に置換** してください。
ポイント：全てのテーブルで `group_id` を主キーの第一要素にします。

```sql
-- src/pkg/cognee/db/duckdb/schema.sql

-- Files metadata
CREATE TABLE IF NOT EXISTS data (
    id VARCHAR,          -- Changed from UUID to VARCHAR for deterministic ID
    group_id VARCHAR,    -- [NEW] Partition Key
    name VARCHAR,
    raw_data_location VARCHAR,
    extension VARCHAR,
    mime_type VARCHAR,
    content_hash VARCHAR,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (group_id, id) -- Partition Key First
);

-- Extracted text content
CREATE TABLE IF NOT EXISTS documents (
    id UUID,
    group_id VARCHAR,    -- [NEW]
    data_id VARCHAR REFERENCES data(group_id, id), -- Composite FK
    text VARCHAR,
    metadata JSON,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (group_id, id),
    FOREIGN KEY (group_id, data_id) REFERENCES data(group_id, id)
);

-- Text chunks
CREATE TABLE IF NOT EXISTS chunks (
    id UUID,             -- Chunk ID needs UUID? Or deterministic? Keep UUID for now.
    group_id VARCHAR,    -- [NEW]
    document_id UUID,
    text VARCHAR,
    chunk_index INTEGER,
    token_count INTEGER,
    metadata JSON,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (group_id, id),
    FOREIGN KEY (group_id, document_id) REFERENCES documents(group_id, id)
);

-- Vector storage
CREATE TABLE IF NOT EXISTS vectors (
    id VARCHAR,
    group_id VARCHAR,    -- [NEW]
    collection_name VARCHAR,
    text VARCHAR,
    embedding FLOAT[1536],
    PRIMARY KEY (group_id, collection_name, id)
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_vectors_collection ON vectors(group_id, collection_name);
```

#### 1-2. データモデルの更新 (`src/pkg/cognee/storage/types.go`)

`Data` 構造体などに `GroupID` フィールドを追加します。

```go
// src/pkg/cognee/storage/types.go

type Data struct {
	ID              string    `json:"id"`
	GroupID         string    `json:"group_id"` // [NEW]
	Name            string    `json:"name"`
	RawDataLocation string    `json:"raw_data_location"`
	Extension       string    `json:"extension"`
	MimeType        string    `json:"mime_type"`
	ContentHash     string    `json:"content_hash"`
	CreatedAt       time.Time `json:"created_at"`
}

type Document struct {
	ID        string    `json:"id"` // Keep simple string or uuid
	GroupID   string    `json:"group_id"` // [NEW]
    // ... (other fields remain similar but ensure GroupID is propagated)
}

// Chunk, Node, Edge にも同様に GroupID を追加してください。
// NodeとEdgeはCozoDB用ですが、構造体定義はここで共有している場合は追加が必要です。
```

---

### Step 2: Storage層の改修

全てのCRUD操作で `group_id` を受け取り、フィルタリングするように書き換えます。

#### 2-1. DuckDB Storage (`src/pkg/cognee/db/duckdb/duckdb_storage.go`)

`VectorStorage` インターフェースのメソッドシグネチャ変更に伴う修正です。
**インターフェース定義 (`storage/interfaces.go`) も忘れずに修正してください。**

```go
// src/pkg/cognee/db/duckdb/duckdb_storage.go

func (s *DuckDBStorage) SaveData(ctx context.Context, data *storage.Data) error {
	query := `
		INSERT INTO data (id, group_id, name, raw_data_location, extension, content_hash, created_at)
		VALUES (?, ?, ?, ?, ?, ?, ?)
		ON CONFLICT (group_id, id) DO UPDATE SET
			name = excluded.name,
            -- ...
	`
	_, err := s.db.ExecContext(ctx, query, data.ID, data.GroupID, data.Name, ...)
	return err
}

// Search メソッドに groupID 引数を追加
func (s *DuckDBStorage) Search(ctx context.Context, collectionName string, vector []float32, k int, groupID string) ([]*storage.SearchResult, error) {
	query := `
		SELECT id, text, array_cosine_similarity(embedding, ?::FLOAT[1536]) as score
		FROM vectors
		WHERE collection_name = ? AND group_id = ?
		ORDER BY score DESC
		LIMIT ?
	`
	// ...
}
```

#### 2-2. CozoDB Storage (`src/pkg/cognee/db/cozodb/cozo_storage.go`)

CozoDBのスキーマ更新とクエリ修正です。

```go
// EnsureSchema で group_id を追加
queries := []string{
    ":create nodes { id: String, group_id: String, type: String, properties: Json }",
    ":create edges { source_id: String, target_id: String, group_id: String, type: String, properties: Json }",
}

// GetTriplets で group_id フィルタリング
// graph_completion.go から呼び出される際、nodeIDs は既に検索結果（＝GroupIDフィルタ済）のはずですが、
// 念のためEdge検索時にもGroupIDを条件に加えるのが安全です。
// ただしCozoDBは主キー検索が高速なので、IDリストがあるならIDで引くだけでも十分かもしれません。
// ここでは堅牢性のため、構造体定義に合わせて group_id カラムへの保存を実装します。
```

---

### Step 3: 決定論的ID生成の実装

`add` タスクにおいて、Python版と互換性のあるID生成ロジックを実装します。

#### 3-1. Ingestion Task (`src/pkg/cognee/tasks/ingestion/ingest_task.go`)

```go
// src/pkg/cognee/tasks/ingestion/ingest_task.go

type IngestTask struct {
	vectorStorage storage.VectorStorage
	groupID       string // [NEW] Task instance holds the group context
}

func NewIngestTask(vectorStorage storage.VectorStorage, groupID string) *IngestTask {
	return &IngestTask{
		vectorStorage: vectorStorage,
		groupID:       groupID,
	}
}

// Helper: Deterministic ID Generation
func generateDeterministicID(contentHash string, groupID string) string {
	// Namespace similar to Python's implementation
	namespace := uuid.MustParse("6ba7b810-9dad-11d1-80b4-00c04fd430c8") // UUID OID Namespace or similar
	return uuid.NewSHA1(namespace, []byte(contentHash+groupID)).String()
}

func (t *IngestTask) Run(ctx context.Context, input any) (any, error) {
    // ...
    hash, _ := calculateFileHash(path)
    
    // Generate Deterministic ID
    dataID := generateDeterministicID(hash, t.groupID)
    
    data := &storage.Data{
        ID:          dataID,
        GroupID:     t.groupID, // Set Partition ID
        ContentHash: hash,
        // ...
    }
    // ...
}
```

---

### Step 4: CogneeServiceの構造化

物理パーティションを実現するため、`CogneeService` のコンストラクタでDBパスを受け取り、インスタンス生成時に接続を確立するように変更します。

#### 4-1. CogneeService (`src/pkg/cognee/cognee.go`)

```go
// src/pkg/cognee/cognee.go

type CogneeService struct {
	VectorStorage storage.VectorStorage
	GraphStorage  storage.GraphStorage
	Embedder      storage.Embedder
    // インスタンスごとに固有の設定（本来はここでDBコネクション自体を管理するのが望ましいが、リファクタリングの影響範囲を考慮しStorageを受け取る形を維持しつつ、main.goでの生成ロジックを変える）
}

// NewCogneeService のシグネチャは変えず、呼び出し側（main.go）で制御する方針とする。
// ただし、Add/Cognify/Search メソッドのシグネチャ変更は必須。

func (s *CogneeService) Add(ctx context.Context, filePaths []string, dataset string, user string) error {
    // dataset + user から groupID を生成（今は単純結合でOK）
    groupID := user + "-" + dataset
    
    ingestTask := ingestion.NewIngestTask(s.VectorStorage, groupID)
    // ...
}

func (s *CogneeService) Search(..., groupID string) (...) {
    // ...
    searchTool := search.NewGraphCompletionTool(..., groupID) // ToolにもGroupIDを渡す
    // ...
}
```

#### 4-2. CLI Entry Point (`src/main.go`)

`main.go` で、コマンドライン引数や環境変数から「ユーザーごとのDBパス」を決定し、そのパスで `CogneeService` を初期化するロジックを組みます。

```go
// src/main.go (イメージ)

func main() {
    // 1. Determine DB Path based on User/Env
    // For single-user local usage, can be static for now.
    // BUT to prove physical partitioning, we should allow configuring it.
    duckDBPath := "data/cognee.duckdb" // Default
    cozoDBPath := "data/graph.cozodb"  // Default
    
    // 2. Initialize DBs
    // 3. Initialize Service
    service := cognee.NewCogneeService(...)
    
    // 4. Exec Command with GroupID
    // CLI引数から user, dataset を取得し groupID を構成
    groupID := "default-user-main-dataset"
    service.Add(ctx, files, "main-dataset", "default-user")
}
```

---

### Step 5: 注意点 (Critical Implementation Details)

1.  **インターフェース変更の影響**: `storage.VectorStorage` メソッドに `groupID` を追加すると、実装している全てのファイル（`DuckDBStorage`）だけでなく、モックやテストコードも修正が必要になります。
2.  **既存データの破棄**: スキーマが主キーを含めて完全に変わるため、`make run` 等で実行する前に必ず既存の `*.duckdb`, `*.cozodb` ファイルを削除してください。
3.  **ダウンタイムなしの移行は考慮しない**: 今回は開発段階のため、データ移行スクリプトは作成しません。

---

### Step 6: 完了の定義 (Done Definition)

*   `src/pkg/cognee/db/duckdb/schema.sql` が更新されている。
*   `src/pkg/cognee/storage/types.go` に `GroupID` がある。
*   `ingest_task.go` が決定論的ID (`uuid.NewSHA1`) を使用している。
*   `duckdb_storage.go` の全てのクエリに `WHERE group_id = ?` がある。
*   `search` コマンドを実行した際、異なる `group_id` のデータが混ざらないことが（ログレベルでも）確認できる。
