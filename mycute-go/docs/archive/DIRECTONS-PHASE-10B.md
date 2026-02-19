# Cognee Go Implementation: Phase-10B Detailed Development Directives
# KuzuDBStorage Interface Design (インターフェース設計)

## 0. はじめに (Introduction)

本ドキュメントは、**Phase-10B: KuzuDBStorage Interface Design** の実装を「迷いなく」「確実に」遂行するための詳細な開発指示書です。

> [!IMPORTANT]
> **Phase-10Bのゴール**
> `KuzuDBStorage` 構造体の骨格を作成し、`VectorStorage` と `GraphStorage` の両インターフェースを実装する準備を整える。
> CogneeConfig に `DatabaseMode` を追加し、DB切替の基盤を整備する。

> [!CAUTION]
> **前提条件**
> Phase-10Aが完了していること（KuzuDBのビルドが成功していること）

---

## 1. 実装ステップ一覧 (Implementation Steps)

| Step | 内容 | 対象ファイル | 行数目安 |
|------|------|-------------|---------|
| 1 | kuzudb ディレクトリ作成 | `src/pkg/cognee/db/kuzudb/` | ディレクトリ |
| 2 | KuzuDBStorage構造体定義 | `kuzudb_storage.go` | +80行 |
| 3 | NewKuzuDBStorage関数実装 | `kuzudb_storage.go` | +40行 |
| 4 | EnsureSchema実装 | `kuzudb_storage.go` | +100行 |
| 5 | CogneeConfig拡張 | `cognee.go` | +15行 |
| 6 | インターフェース適合確認 | - | コンパイルのみ |
| 7 | ビルド確認 | - | - |

---

## Step 1: kuzudbディレクトリ作成

### 1.1 DuckDB + CozoDB での参照実装

現在のディレクトリ構造：

```
src/pkg/cognee/db/
├── cozodb/
│   ├── cozo_storage.go   # CozoStorage実装
│   └── lib/              # CozoDBネイティブライブラリ
└── duckdb/
    ├── duckdb_storage.go # DuckDBStorage実装
    ├── extensions/       # DuckDB VSS拡張
    └── schema.sql        # DuckDBスキーマ定義
```

**根拠説明**: DuckDBとCozoDBはそれぞれ独立したディレクトリに実装が配置されている。これにより、各データベースの実装が明確に分離され、保守性が向上している。KuzuDBも同様のパターンに従う。

### 1.2 KuzuDB実装

```
src/pkg/cognee/db/
├── cozodb/
├── duckdb/
└── kuzudb/               ← 新規作成
    └── kuzudb_storage.go ← 新規作成
```

作成コマンド:
```bash
mkdir -p src/pkg/cognee/db/kuzudb
```

---

## Step 2: KuzuDBStorage構造体定義

### 2.1 DuckDB + CozoDB での参照実装

```go
// duckdb_storage.go より
// Package duckdb は、DuckDBを使用したベクトルストレージの実装を提供します。
package duckdb

type DuckDBStorage struct {
    db *sql.DB // DuckDBへの接続
}

var _ storage.VectorStorage = (*DuckDBStorage)(nil)
```

```go
// cozo_storage.go より
// Package cozodb は、CozoDBを使用したグラフストレージの実装を提供します。
package cozodb

type CozoStorage struct {
    db *cozo.CozoDB // CozoDBへの接続
}

var _ storage.GraphStorage = (*CozoStorage)(nil)
```

**根拠説明**: 
- DuckDBStorageは `*sql.DB` を保持し、`VectorStorage` インターフェースのみを実装
- CozoStorageは `*cozo.CozoDB` を保持し、`GraphStorage` インターフェースのみを実装
- KuzuDBStorageはグラフ+ベクトルの統合DBなので、両方のインターフェースを実装する必要がある

### 2.2 KuzuDB実装

```go
// Package kuzudb は、KuzuDBを使用した統合ストレージの実装を提供します。
// このパッケージは、VectorStorageとGraphStorageの両方のインターフェースを
// 単一のKuzuDBインスタンスで実装します。
//
// KuzuDBの特徴:
//   - グラフデータベース機能（Cypher言語）
//   - ベクトルデータベース機能（FLOAT[]型、cosine_similarity関数）
//   - ACID トランザクション
//   - ディスク永続化（インメモリモードなし）
//
// 使用例:
//
//	storage, err := kuzudb.NewKuzuDBStorage("./db/kuzudb")
//	if err != nil {
//	    log.Fatal(err)
//	}
//	defer storage.Close()
//
//	// VectorStorage として使用
//	storage.SaveData(ctx, data)
//
//	// GraphStorage として使用
//	storage.AddNodes(ctx, nodes)
package kuzudb

import (
	"context"
	"encoding/json"
	"fmt"
	"strings"
	"time"

	"github.com/t-kawata/go-kuzu/pkg/kuzu"
	"mycute/pkg/cognee/storage"
)

// KuzuDBStorage は、KuzuDBを使用した統合ストレージ実装です。
// VectorStorage と GraphStorage の両インターフェースを実装します。
//
// 設計方針:
//   - 単一のKuzuDBデータベースインスタンスを使用
//   - DuckDBのテーブル構造とCozoDBのリレーション構造を統合
//   - group_id によるパーティション分離を維持
//
// DuckDBとの比較:
//   DuckDB: sql.DB経由でリレーショナルテーブルにアクセス
//   KuzuDB: kuzu.Connection経由でグラフノード/エッジにアクセス
//
// CozoDBとの比較:
//   CozoDB: Datalog言語でノード/エッジを操作
//   KuzuDB: Cypher言語でノード/エッジを操作
//
// スレッドセーフ性:
//   - KuzuDBは複数接続をサポート
//   - 必要に応じてコネクションプールを実装可能
type KuzuDBStorage struct {
	// db はKuzuDBデータベースインスタンス
	// Close()で解放される
	db *kuzu.Database

	// conn はデフォルトの接続
	// 単一接続で十分な場合に使用
	conn *kuzu.Connection

	// dbPath はデータベースのファイルパス
	// デバッグやログ出力に使用
	dbPath string
}

// コンパイル時にインターフェース実装を検証
// DuckDBStorage: var _ storage.VectorStorage = (*DuckDBStorage)(nil)
// CozoStorage:   var _ storage.GraphStorage = (*CozoStorage)(nil)
// KuzuDBStorage: 両方のインターフェースを実装
var _ storage.VectorStorage = (*KuzuDBStorage)(nil)
var _ storage.GraphStorage = (*KuzuDBStorage)(nil)
```

---

## Step 3: NewKuzuDBStorage関数実装

### 3.1 DuckDB + CozoDB での参照実装

```go
// duckdb_storage.go より
func NewDuckDBStorage(db *sql.DB) *DuckDBStorage {
    return &DuckDBStorage{db: db}
}

// cozo_storage.go より
func NewCozoStorage(db *cozo.CozoDB) *CozoStorage {
    return &CozoStorage{db: db}
}
```

**根拠説明**:
- DuckDBStorageとCozoStorageは、既に開かれたDB接続を受け取る設計
- しかし、KuzuDBは`kuzu.Database`と`kuzu.Connection`の2段階の初期化が必要
- そのため、KuzuDBStorageはパスのみを受け取り、内部でDB作成と接続確立を行う

### 3.2 KuzuDB実装

```go
// NewKuzuDBStorage は、新しいKuzuDBStorageインスタンスを作成します。
//
// この関数は以下の処理を行います:
//   1. 指定されたパスにKuzuDBデータベースを作成または開く
//   2. データベースへの接続を確立
//   3. KuzuDBStorageインスタンスを返す
//
// DuckDBとの比較:
//   DuckDB: sql.Open("duckdb", path) で直接接続
//   KuzuDB: kuzu.NewDatabase(path) + db.Connect() の2段階
//
// CozoDBとの比較:
//   CozoDB: cozo.NewCozoDB("rocksdb", path) で直接接続
//   KuzuDB: kuzu.NewDatabase(path) + db.Connect() の2段階
//
// 引数:
//   - dbPath: データベースディレクトリのパス（例: "./db/kuzudb"）
//
// 返り値:
//   - *KuzuDBStorage: 作成されたストレージインスタンス
//   - error: データベースの作成や接続に失敗した場合
//
// 注意:
//   - KuzuDBはディスク永続化のみサポート（インメモリモードなし）
//   - 指定されたディレクトリが存在しない場合は自動作成される
//   - Close()を呼び出してリソースを解放すること
func NewKuzuDBStorage(dbPath string) (*KuzuDBStorage, error) {
	// 1. データベースを作成/開く
	// DuckDB:  db, _ := sql.Open("duckdb", path)
	// CozoDB:  db, _ := cozo.NewCozoDB("rocksdb", path)
	// KuzuDB:  db, _ := kuzu.NewDatabase(path)
	db, err := kuzu.NewDatabase(dbPath)
	if err != nil {
		return nil, fmt.Errorf("failed to create/open KuzuDB database at %s: %w", dbPath, err)
	}

	// 2. 接続を確立
	// DuckDB/CozoDB: 接続は暗黙的に管理される
	// KuzuDB: 明示的にConnect()を呼び出す必要がある
	conn, err := db.Connect()
	if err != nil {
		db.Close() // データベースをクリーンアップ
		return nil, fmt.Errorf("failed to connect to KuzuDB database: %w", err)
	}

	return &KuzuDBStorage{
		db:     db,
		conn:   conn,
		dbPath: dbPath,
	}, nil
}

// Close は、KuzuDBStorageのリソースを解放します。
//
// DuckDBでの参照実装:
//   func (s *DuckDBStorage) Close() error {
//       return s.db.Close()
//   }
//
// CozoDBでの参照実装:
//   func (s *CozoStorage) Close() error {
//       s.db.Close()
//       return nil
//   }
//
// 返り値:
//   - error: クローズ中にエラーが発生した場合
func (s *KuzuDBStorage) Close() error {
	var errs []string

	// KuzuDBは接続とデータベースの両方を明示的にクローズする必要がある
	if s.conn != nil {
		s.conn.Close()
		s.conn = nil
	}

	if s.db != nil {
		s.db.Close()
		s.db = nil
	}

	if len(errs) > 0 {
		return fmt.Errorf("errors during close: %s", strings.Join(errs, "; "))
	}
	return nil
}
```

---

## Step 4: EnsureSchema実装

### 4.1 DuckDB + CozoDB での参照実装

```go
// duckdb: schema.sql (外部ファイル)
// CREATE TABLE IF NOT EXISTS data (
//     id UUID PRIMARY KEY,
//     group_id VARCHAR NOT NULL,
//     ...
// );
// CREATE TABLE IF NOT EXISTS vectors (
//     id VARCHAR,
//     group_id VARCHAR,
//     collection_name VARCHAR,
//     text VARCHAR,
//     embedding FLOAT[1536],
//     PRIMARY KEY (group_id, collection_name, id)
// );
```

```go
// cozo_storage.go より
func (s *CozoStorage) EnsureSchema(ctx context.Context) error {
    queries := []string{
        ":create nodes { id: String, group_id: String, type: String, properties: Json }",
        ":create edges { source_id: String, target_id: String, group_id: String, type: String, properties: Json }",
    }
    for _, q := range queries {
        if _, err := s.db.Run(q, nil); err != nil {
            // "already exists" エラーは無視
            if !strings.Contains(err.Error(), "already exists") {
                return err
            }
        }
    }
    return nil
}
```

**根拠説明**:
- DuckDBはSQLのCREATE TABLE文でスキーマを定義
- CozoDBはDatalogの:createコマンドでリレーションを定義
- KuzuDBはCypherのCREATE NODE TABLE / CREATE REL TABLE文でスキーマを定義
- いずれも冪等性（2回実行してもエラーにならない）を考慮

### 4.2 KuzuDB実装

```go
// EnsureSchema は、KuzuDBデータベースに必要なスキーマを作成します。
//
// DuckDBでの参照実装（schema.sql）:
//   CREATE TABLE IF NOT EXISTS data (id UUID, group_id VARCHAR, ...);
//   CREATE TABLE IF NOT EXISTS documents (id UUID, group_id VARCHAR, ...);
//   CREATE TABLE IF NOT EXISTS chunks (id UUID, group_id VARCHAR, ...);
//   CREATE TABLE IF NOT EXISTS vectors (id VARCHAR, group_id VARCHAR, embedding FLOAT[1536], ...);
//
// CozoDBでの参照実装（EnsureSchema）:
//   :create nodes { id: String, group_id: String, type: String, properties: Json }
//   :create edges { source_id: String, target_id: String, group_id: String, type: String, properties: Json }
//
// KuzuDBでは、DuckDBの4テーブル + CozoDBの2リレーションを統合したスキーマを作成する
func (s *KuzuDBStorage) EnsureSchema(ctx context.Context) error {
	// ========================================
	// 1. ノードテーブルの作成
	// ========================================
	// DuckDBのテーブル → KuzuDBのノードテーブルへのマッピング
	nodeTableQueries := []string{
		// Data テーブル（DuckDB data 相当）
		// DuckDB: CREATE TABLE data (id UUID, group_id VARCHAR, name VARCHAR, ...)
		// KuzuDB: CREATE NODE TABLE Data (id STRING PRIMARY KEY, group_id STRING, ...)
		`CREATE NODE TABLE IF NOT EXISTS Data (
			id STRING PRIMARY KEY,
			group_id STRING,
			name STRING,
			raw_data_location STRING,
			original_data_location STRING,
			extension STRING,
			mime_type STRING,
			content_hash STRING,
			owner_id STRING,
			created_at TIMESTAMP
		)`,

		// Document テーブル（DuckDB documents 相当）
		`CREATE NODE TABLE IF NOT EXISTS Document (
			id STRING PRIMARY KEY,
			group_id STRING,
			data_id STRING,
			text STRING,
			metadata STRING
		)`,

		// Chunk テーブル（DuckDB chunks 相当）
		`CREATE NODE TABLE IF NOT EXISTS Chunk (
			id STRING PRIMARY KEY,
			group_id STRING,
			document_id STRING,
			text STRING,
			token_count INT32,
			chunk_index INT32
		)`,

		// Vector テーブル（DuckDB vectors 相当）
		// DuckDB: embedding FLOAT[1536]
		// KuzuDB: embedding FLOAT[]
		`CREATE NODE TABLE IF NOT EXISTS Vector (
			id STRING PRIMARY KEY,
			group_id STRING,
			collection_name STRING,
			text STRING,
			embedding FLOAT[]
		)`,

		// GraphNode テーブル（CozoDB nodes 相当）
		// CozoDB: :create nodes { id: String, group_id: String, type: String, properties: Json }
		// KuzuDB: CREATE NODE TABLE GraphNode (id STRING PRIMARY KEY, ...)
		`CREATE NODE TABLE IF NOT EXISTS GraphNode (
			id STRING PRIMARY KEY,
			group_id STRING,
			type STRING,
			properties STRING
		)`,
	}

	// ========================================
	// 2. リレーションシップテーブルの作成
	// ========================================
	// CozoDBのedgesリレーション → KuzuDBのREL TABLEへのマッピング
	relTableQueries := []string{
		// GraphEdge テーブル（CozoDB edges 相当）
		// CozoDB: :create edges { source_id: String, target_id: String, ... }
		// KuzuDB: CREATE REL TABLE GraphEdge (FROM GraphNode TO GraphNode, ...)
		`CREATE REL TABLE IF NOT EXISTS GraphEdge (
			FROM GraphNode TO GraphNode,
			group_id STRING,
			type STRING,
			properties STRING
		)`,

		// has_document: Data → Document の関連
		`CREATE REL TABLE IF NOT EXISTS has_document (
			FROM Data TO Document
		)`,

		// has_chunk: Document → Chunk の関連
		`CREATE REL TABLE IF NOT EXISTS has_chunk (
			FROM Document TO Chunk
		)`,
	}

	// ========================================
	// 3. インデックスの作成
	// ========================================
	indexQueries := []string{
		// group_id によるフィルタリング高速化（DuckDBの暗黙的インデックスと同等）
		`CREATE INDEX IF NOT EXISTS idx_data_group ON Data (group_id)`,
		`CREATE INDEX IF NOT EXISTS idx_document_group ON Document (group_id)`,
		`CREATE INDEX IF NOT EXISTS idx_chunk_group ON Chunk (group_id)`,
		`CREATE INDEX IF NOT EXISTS idx_vector_group ON Vector (group_id)`,
		`CREATE INDEX IF NOT EXISTS idx_graphnode_group ON GraphNode (group_id)`,

		// Vector コレクション検索の高速化
		`CREATE INDEX IF NOT EXISTS idx_vector_collection ON Vector (group_id, collection_name)`,

		// GraphNode タイプ検索の高速化
		`CREATE INDEX IF NOT EXISTS idx_graphnode_type ON GraphNode (group_id, type)`,
	}

	// ========================================
	// 4. クエリの実行
	// ========================================

	// ノードテーブル作成
	for _, query := range nodeTableQueries {
		if _, err := s.conn.Execute(query); err != nil {
			// CozoDBと同様: "already exists" エラーは無視
			if !strings.Contains(err.Error(), "already exists") {
				return fmt.Errorf("failed to create node table: %w (query: %s)", err, query)
			}
		}
	}

	// リレーションシップテーブル作成
	for _, query := range relTableQueries {
		if _, err := s.conn.Execute(query); err != nil {
			if !strings.Contains(err.Error(), "already exists") {
				return fmt.Errorf("failed to create rel table: %w (query: %s)", err, query)
			}
		}
	}

	// インデックス作成
	for _, query := range indexQueries {
		if _, err := s.conn.Execute(query); err != nil {
			if !strings.Contains(err.Error(), "already exists") {
				// インデックス作成エラーは警告として扱う（致命的ではない）
				fmt.Printf("Warning: index creation failed: %v\n", err)
			}
		}
	}

	return nil
}
```

---

## Step 5: CogneeConfig拡張

### 5.1 DuckDB + CozoDB での参照実装

```go
// cognee.go より
type CogneeConfig struct {
    COGNEE_DB_DIR              string  // データベース格納ディレクトリ
    DATAPIPE_URL               string  // DatapipeサービスURL
    OPENAI_API_KEY             string  // OpenAI APIキー
    // ... 他のフィールド
}
```

**根拠説明**: 現在のCogneeConfigはDuckDB+CozoDB固定の設計になっている。KuzuDBモードを追加するために、データベースモードを選択できるフィールドを追加する必要がある。

### 5.2 KuzuDB実装

```go
type CogneeConfig struct {
    // 既存フィールド...
    
    // ========================================
    // Phase-10追加: データベースモード設定
    // ========================================

    // DatabaseMode はデータベースの動作モードを指定します。
    //   - "duckdb+cozodb" (デフォルト): DuckDBとCozoDBを併用
    //   - "kuzudb": KuzuDBのみを使用（VectorStorage + GraphStorage統合）
    //
    // 注意:
    //   - 起動時に決定され、実行中の切り替えは不可
    //   - モードによって使用されるデータベースファイルが異なる
    DatabaseMode string

    // KuzuDBDatabasePath はKuzuDBデータベースのパスを指定します。
    // DatabaseMode が "kuzudb" の場合に使用されます。
    // 空の場合は COGNEE_DB_DIR + "/kuzudb" がデフォルト値として使用されます。
    //
    // 例: "./db/kuzudb", "/var/lib/mycute/kuzudb"
    KuzuDBDatabasePath string
}
```

---

## Step 6: インターフェース適合スタブ

### 6.1 VectorStorageスタブ

```go
// ========================================
// VectorStorage インターフェース実装（スタブ）
// Phase-10D で完全実装予定
// ========================================

// DuckDB参照: duckdb_storage.go の各メソッドを参照

func (s *KuzuDBStorage) SaveData(ctx context.Context, data *storage.Data) error {
    // DuckDB: INSERT INTO data VALUES (?, ?, ...) ON CONFLICT DO UPDATE
    return fmt.Errorf("not implemented: SaveData")
}

func (s *KuzuDBStorage) Exists(ctx context.Context, contentHash string, groupID string) bool {
    // DuckDB: SELECT COUNT(*) FROM data WHERE content_hash = ? AND group_id = ?
    return false
}

func (s *KuzuDBStorage) GetDataByID(ctx context.Context, id string, groupID string) (*storage.Data, error) {
    // DuckDB: SELECT ... FROM data WHERE id = ? AND group_id = ?
    return nil, fmt.Errorf("not implemented: GetDataByID")
}

func (s *KuzuDBStorage) GetDataList(ctx context.Context, groupID string) ([]*storage.Data, error) {
    // DuckDB: SELECT ... FROM data WHERE group_id = ?
    return nil, fmt.Errorf("not implemented: GetDataList")
}

func (s *KuzuDBStorage) SaveDocument(ctx context.Context, document *storage.Document) error {
    // DuckDB: INSERT INTO documents VALUES (...) ON CONFLICT DO UPDATE
    return fmt.Errorf("not implemented: SaveDocument")
}

func (s *KuzuDBStorage) SaveChunk(ctx context.Context, chunk *storage.Chunk) error {
    // DuckDB: INSERT INTO chunks + INSERT INTO vectors
    return fmt.Errorf("not implemented: SaveChunk")
}

func (s *KuzuDBStorage) SaveEmbedding(ctx context.Context, collectionName, id, text string, vector []float32, groupID string) error {
    // DuckDB: INSERT INTO vectors VALUES (...) ON CONFLICT DO UPDATE
    return fmt.Errorf("not implemented: SaveEmbedding")
}

func (s *KuzuDBStorage) Search(ctx context.Context, collectionName string, vector []float32, k int, groupID string) ([]*storage.SearchResult, error) {
    // DuckDB: SELECT id, text, array_cosine_similarity(...) FROM vectors ORDER BY score DESC LIMIT k
    return nil, fmt.Errorf("not implemented: Search")
}

func (s *KuzuDBStorage) GetEmbeddingByID(ctx context.Context, collectionName, id, groupID string) ([]float32, error) {
    // DuckDB: SELECT embedding FROM vectors WHERE id = ? AND collection_name = ? AND group_id = ?
    return nil, fmt.Errorf("not implemented: GetEmbeddingByID")
}

func (s *KuzuDBStorage) GetEmbeddingsByIDs(ctx context.Context, collectionName string, ids []string, groupID string) (map[string][]float32, error) {
    // DuckDB: SELECT id, embedding FROM vectors WHERE id IN (...) AND collection_name = ? AND group_id = ?
    return nil, fmt.Errorf("not implemented: GetEmbeddingsByIDs")
}
```

### 6.2 GraphStorageスタブ

```go
// ========================================
// GraphStorage インターフェース実装（スタブ）
// Phase-10E で完全実装予定
// ========================================

// CozoDB参照: cozo_storage.go の各メソッドを参照

func (s *KuzuDBStorage) AddNodes(ctx context.Context, nodes []*storage.Node) error {
    // CozoDB: ?[id, group_id, type, properties] <- $data :put nodes {...}
    return fmt.Errorf("not implemented: AddNodes")
}

func (s *KuzuDBStorage) AddEdges(ctx context.Context, edges []*storage.Edge) error {
    // CozoDB: ?[source_id, target_id, ...] <- $data :put edges {...}
    return fmt.Errorf("not implemented: AddEdges")
}

func (s *KuzuDBStorage) GetTriplets(ctx context.Context, nodeIDs []string, groupID string) ([]*storage.Triplet, error) {
    // CozoDB: *edges[source_id, target_id, ...], (source_id in IDs or target_id in IDs)
    return nil, fmt.Errorf("not implemented: GetTriplets")
}

func (s *KuzuDBStorage) StreamDocumentChunks(ctx context.Context, groupID string) (<-chan *storage.ChunkData, <-chan error) {
    // CozoDB: *nodes[...], type = "DocumentChunk" :limit :offset
    dataCh := make(chan *storage.ChunkData)
    errCh := make(chan error, 1)
    errCh <- fmt.Errorf("not implemented: StreamDocumentChunks")
    close(dataCh)
    close(errCh)
    return dataCh, errCh
}

func (s *KuzuDBStorage) GetDocumentChunkCount(ctx context.Context, groupID string) (int, error) {
    // CozoDB: ?[count(id)] := *nodes[...], type = "DocumentChunk"
    return 0, fmt.Errorf("not implemented: GetDocumentChunkCount")
}

func (s *KuzuDBStorage) GetNodesByType(ctx context.Context, nodeType string, groupID string) ([]*storage.Node, error) {
    // CozoDB: *nodes[...], type = $type, group_id = $group_id
    return nil, fmt.Errorf("not implemented: GetNodesByType")
}

func (s *KuzuDBStorage) GetNodesByEdge(ctx context.Context, targetID string, edgeType string, groupID string) ([]*storage.Node, error) {
    // CozoDB: *edges[source_id, target_id, ...], target_id = $target_id, edge_type = $edge_type
    return nil, fmt.Errorf("not implemented: GetNodesByEdge")
}

func (s *KuzuDBStorage) UpdateEdgeWeight(ctx context.Context, sourceID, targetID, groupID string, weight float64) error {
    // CozoDB: GET edge -> UPDATE properties -> PUT edge
    return fmt.Errorf("not implemented: UpdateEdgeWeight")
}

func (s *KuzuDBStorage) UpdateEdgeMetrics(ctx context.Context, sourceID, targetID, groupID string, weight, confidence float64) error {
    // CozoDB: GET edge -> UPDATE properties -> PUT edge
    return fmt.Errorf("not implemented: UpdateEdgeMetrics")
}

func (s *KuzuDBStorage) DeleteEdge(ctx context.Context, sourceID, targetID, groupID string) error {
    // CozoDB: :rm edges {...}
    return fmt.Errorf("not implemented: DeleteEdge")
}

func (s *KuzuDBStorage) DeleteNode(ctx context.Context, nodeID, groupID string) error {
    // CozoDB: :rm nodes {...}
    return fmt.Errorf("not implemented: DeleteNode")
}

func (s *KuzuDBStorage) GetEdgesByNode(ctx context.Context, nodeID string, groupID string) ([]*storage.Edge, error) {
    // CozoDB: *edges[...], (source_id = $id or target_id = $id)
    return nil, fmt.Errorf("not implemented: GetEdgesByNode")
}

func (s *KuzuDBStorage) GetOrphanNodes(ctx context.Context, groupID string, gracePeriod time.Duration) ([]*storage.Node, error) {
    // CozoDB: *nodes[...], not *edges[id, _, ...], not *edges[_, id, ...]
    return nil, fmt.Errorf("not implemented: GetOrphanNodes")
}
```

---

## 6.3 ヘルパー関数（Phase-10D以降で使用）

```go
// ========================================
// ヘルパー関数
// これらの関数はPhase-10D/10Eの実装で使用される
// Phase-10Bの段階で定義しておくことで、後続フェーズの実装が容易になる
// ========================================

// escapeString は、Cypher文字列リテラル内の特殊文字をエスケープします。
//
// 注意:
//   - ユーザー入力を含む全ての文字列フィールドで使用すること
//   - バックスラッシュを最初にエスケープすること（他の文字に影響を与えるため）
//   - エスケープ忘れはCypher injection脆弱性につながる
func escapeString(s string) string {
	s = strings.ReplaceAll(s, "\\", "\\\\")  // バックスラッシュ（最初に処理）
	s = strings.ReplaceAll(s, "'", "\\'")    // シングルクォート
	s = strings.ReplaceAll(s, "\n", "\\n")   // 改行
	s = strings.ReplaceAll(s, "\r", "\\r")   // キャリッジリターン
	s = strings.ReplaceAll(s, "\t", "\\t")   // タブ
	return s
}

// getString は、KuzuDBのGetValue()結果を安全に文字列に変換します。
//
// DuckDBとの比較:
//   DuckDB: row.Scan(&stringVar) で直接スキャン可能
//   KuzuDB: GetValue()がany型を返すため、型アサーションが必要
func getString(v any) string {
	if v == nil {
		return ""
	}
	switch val := v.(type) {
	case string:
		return val
	case []byte:
		return string(val)
	default:
		return fmt.Sprintf("%v", v)
	}
}

// getInt64 は、KuzuDBのGetValue()結果を安全にint64に変換します。
//
// 注意:
//   - count()関数はfloat64を返す場合がある
//   - 型の不一致を避けるため、複数の型を処理する
func getInt64(v any) int64 {
	if v == nil {
		return 0
	}
	switch val := v.(type) {
	case int64:
		return val
	case float64:
		return int64(val)
	case int32:
		return int64(val)
	case int:
		return int64(val)
	default:
		return 0
	}
}

// getFloat64 は、KuzuDBのGetValue()結果を安全にfloat64に変換します。
func getFloat64(v any) float64 {
	if v == nil {
		return 0
	}
	switch val := v.(type) {
	case float64:
		return val
	case float32:
		return float64(val)
	case int64:
		return float64(val)
	default:
		return 0
	}
}

// formatVectorForKuzuDB は、[]float32をKuzuDBのFLOAT[]リテラルに変換します。
//
// 重要:
//   - fmt.Sprint(vec) は "[0.1 0.2 0.3]" を返す（スペース区切り）
//   - KuzuDBは "[0.1, 0.2, 0.3]" を要求する（カンマ区切り）
//
// DuckDBとの比較:
//   DuckDB: ?::FLOAT[1536] でパラメータ化可能
//   KuzuDB: 文字列リテラルで埋め込む必要あり
func formatVectorForKuzuDB(vec []float32) string {
	parts := make([]string, len(vec))
	for i, v := range vec {
		parts[i] = fmt.Sprintf("%f", v)
	}
	return "[" + strings.Join(parts, ", ") + "]"
}

// parseEmbedding は、KuzuDBから取得したembedding値を[]float32に変換します。
//
// 注意:
//   - KuzuDBは FLOAT[] を []any または 文字列として返す場合がある
//   - 両方のケースを処理する必要がある
func parseEmbedding(v any) []float32 {
	if v == nil {
		return nil
	}
	switch val := v.(type) {
	case []any:
		result := make([]float32, len(val))
		for i, elem := range val {
			result[i] = float32(getFloat64(elem))
		}
		return result
	case string:
		// 文字列形式の場合: "[0.1, 0.2, ...]"
		s := strings.Trim(val, "[]")
		if s == "" {
			return nil
		}
		parts := strings.Split(s, ",")
		result := make([]float32, len(parts))
		for i, p := range parts {
			f, _ := strconv.ParseFloat(strings.TrimSpace(p), 32)
			result[i] = float32(f)
		}
		return result
	default:
		return nil
	}
}

// parseJSONProperties は、JSON文字列をmap[string]anyに変換します。
//
// CozoDBとの比較:
//   CozoDB: properties: Json 型をそのまま使用
//   KuzuDB: properties: STRING 型にJSON文字列を保存
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

// formatTimestamp は、time.TimeをKuzuDBのTIMESTAMP形式に変換します。
//
// 重要:
//   - RFC3339形式を使用: "2024-01-01T12:00:00+09:00"
//   - datetime()関数で囲む: datetime('2024-01-01T12:00:00+09:00')
func formatTimestamp(t time.Time) string {
	return fmt.Sprintf("datetime('%s')", t.Format(time.RFC3339))
}

// parseTimestamp は、KuzuDBのTIMESTAMP値をtime.Timeに変換します。
func parseTimestamp(v any) time.Time {
	if v == nil {
		return time.Time{}
	}
	s := getString(v)
	if s == "" {
		return time.Time{}
	}
	t, err := time.Parse(time.RFC3339, s)
	if err != nil {
		// 他の形式も試行
		t, _ = time.Parse("2006-01-02T15:04:05", s)
	}
	return t
}
```

---

## 6.4 完全なインポートブロック

```go
// kuzudb_storage.go の完全なインポートブロック
// Phase-10D/10E実装時に必要な全てのインポートを含む
package kuzudb

import (
	"context"
	"encoding/json"
	"fmt"
	"strconv"
	"strings"
	"time"

	"github.com/t-kawata/go-kuzu/pkg/kuzu"
	"mycute/pkg/cognee/storage"
)
```

---

## 6.5 ファイル構成ガイド

Phase-10B完了後のファイル構成:

```
src/pkg/cognee/db/kuzudb/
└── kuzudb_storage.go          # 全実装（600-800行程度）

ファイル内の構成:
┌─────────────────────────────────────┐
│ 1. package宣言 + import             │
├─────────────────────────────────────┤
│ 2. 構造体定義 (KuzuDBStorage)        │
├─────────────────────────────────────┤
│ 3. コンストラクタ (NewKuzuDBStorage) │
│ 4. Close メソッド                   │
├─────────────────────────────────────┤
│ 5. EnsureSchema                     │
├─────────────────────────────────────┤
│ 6. ヘルパー関数                      │
│    - escapeString                   │
│    - getString / getInt64 / ...     │
│    - formatVectorForKuzuDB          │
│    - parseEmbedding                 │
│    - parseJSONProperties            │
├─────────────────────────────────────┤
│ 7. VectorStorage 実装（Phase-10D）   │
│    - SaveData                       │
│    - Exists                         │
│    - GetDataByID / GetDataList      │
│    - SaveDocument / SaveChunk       │
│    - SaveEmbedding / Search         │
│    - GetEmbeddingByID/ByIDs         │
├─────────────────────────────────────┤
│ 8. GraphStorage 実装（Phase-10E）    │
│    - AddNodes / AddEdges            │
│    - GetTriplets                    │
│    - StreamDocumentChunks           │
│    - その他全メソッド                 │
└─────────────────────────────────────┘
```

---

## 7. 成功条件チェックリスト

### Phase-10B 完了条件

- [ ] `src/pkg/cognee/db/kuzudb/` ディレクトリが作成されている
- [ ] `kuzudb_storage.go` が作成されている
- [ ] `KuzuDBStorage` 構造体が定義されている
- [ ] `NewKuzuDBStorage` 関数が実装されている
- [ ] `Close` メソッドが実装されている
- [ ] `EnsureSchema` メソッドが実装されている
- [ ] 全 `VectorStorage` メソッドのスタブが存在
- [ ] 全 `GraphStorage` メソッドのスタブが存在
- [ ] `var _ storage.VectorStorage = (*KuzuDBStorage)(nil)` がコンパイル通過
- [ ] `var _ storage.GraphStorage = (*KuzuDBStorage)(nil)` がコンパイル通過
- [ ] `CogneeConfig` に `DatabaseMode` が追加されている
- [ ] `CogneeConfig` に `KuzuDBDatabasePath` が追加されている
- [ ] `make build` がエラーなしで成功
- [ ] 既存のテストが引き続き動作

---

## 8. 次のフェーズへの準備

Phase-10Bが完了したら、以下が確立された状態となる：

1. **KuzuDBStorage構造体**: 骨格が完成
2. **スキーマ定義**: 全テーブルの定義が完了
3. **インターフェース適合**: コンパイル時検証が通過
4. **設定拡張**: モード切替の基盤が整備

Phase-10Cでは、スキーマ作成のテストと検証を行う。
Phase-10Dでは、VectorStorageの各メソッドを完全実装する。
