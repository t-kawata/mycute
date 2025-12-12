package kuzudb

import (
	"context"
	"encoding/json"
	"fmt"
	"log"
	"strings"
	"time"

	"github.com/t-kawata/mycute/pkg/cuber/storage"
	"github.com/t-kawata/mycute/pkg/cuber/types"
	"github.com/t-kawata/mycute/pkg/cuber/utils"

	kuzu "github.com/kuzudb/go-kuzu"
)

// KuzuDBStorage は、KuzuDBを使用した統合ストレージ実装です。
// VectorStorage と GraphStorage の両インターフェースを実装します。
type KuzuDBStorage struct {
	db   *kuzu.Database
	conn *kuzu.Connection
}

// コンパイル時チェック: インターフェースを満たしているか確認
var _ storage.VectorStorage = (*KuzuDBStorage)(nil)
var _ storage.GraphStorage = (*KuzuDBStorage)(nil)

// NewKuzuDBStorage は新しいKuzuDBStorageインスタンスを作成します。
func NewKuzuDBStorage(dbPath string) (*KuzuDBStorage, error) {
	var db *kuzu.Database
	var err error

	// データベースを開く
	if dbPath == ":memory:" {
		log.Println("[KuzuDB] Opening in-memory database...")
		db, err = kuzu.OpenInMemoryDatabase(kuzu.DefaultSystemConfig())
	} else {
		db, err = kuzu.OpenDatabase(dbPath, kuzu.DefaultSystemConfig())
	}

	if err != nil {
		return nil, fmt.Errorf("failed to open KuzuDB database: %w", err)
	}

	// 接続を開く
	conn, err := kuzu.OpenConnection(db)
	if err != nil {
		db.Close()
		return nil, fmt.Errorf("failed to open KuzuDB connection: %w", err)
	}

	return &KuzuDBStorage{
		db:   db,
		conn: conn,
	}, nil
}

// Close はリソースを解放します。
func (s *KuzuDBStorage) Close() error {
	if s.conn != nil {
		s.conn.Close()
		s.conn = nil
	}
	if s.db != nil {
		s.db.Close()
		s.db = nil
	}
	return nil
}

// IsOpen は、ストレージ接続が開いているかどうかを返します。
func (s *KuzuDBStorage) IsOpen() bool {
	return s.db != nil && s.conn != nil
}

// EnsureSchema は必要なテーブルスキーマを作成します。
// Phase-10Cの実装: データ階層(Data->Document->Chunk)とナレッジグラフ(GraphNode, GraphEdge)のテーブルを作成します。
func (s *KuzuDBStorage) EnsureSchema(ctx context.Context) error {
	log.Println("[KuzuDB] EnsureSchema: Starting schema creation...")

	// 1. Node Tables
	// ---------------------------------------------------------
	// KuzuDBではDDLをトランザクション内で実行可能かはバージョンによるが、
	// 基本的にDDLはAutoCommitモード推奨。
	// エラー(既に存在する等)は個別にチェックする。

	nodeTables := []string{
		// Data: ファイルメタデータ
		`CREATE NODE TABLE Data (
			id STRING,
			memory_group STRING,
			name STRING,
			raw_data_location STRING,
			original_data_location STRING,
			extension STRING,
			mime_type STRING,
			content_hash STRING,
			owner_id STRING,
			created_at TIMESTAMP,
			PRIMARY KEY (id)
		)`,
		// Document: ドキュメント
		`CREATE NODE TABLE Document (
			id STRING,
			memory_group STRING,
			data_id STRING,
			text STRING,
			metadata STRING,
			PRIMARY KEY (id)
		)`,
		// Chunk: チャンクとEmbedding
		`CREATE NODE TABLE Chunk (
			id STRING,
			memory_group STRING,
			document_id STRING,
			text STRING,
			token_count INT64,
			chunk_index INT64,
			embedding FLOAT[1536],
			PRIMARY KEY (id)
		)`,
		// GraphNode: 知識グラフのノード
		`CREATE NODE TABLE GraphNode (
			id STRING,
			memory_group STRING,
			type STRING,
			properties STRING,
			PRIMARY KEY (id)
		)`,
		// Entity: エンティティ（人名、組織名、場所名など）
		`CREATE NODE TABLE Entity (
			id STRING,
			memory_group STRING,
			text STRING,
			embedding FLOAT[1536],
			PRIMARY KEY (id)
		)`,
		// Summary: 要約
		`CREATE NODE TABLE Summary (
			id STRING,
			memory_group STRING,
			text STRING,
			embedding FLOAT[1536],
			PRIMARY KEY (id)
		)`,
		// Rule: ルール
		`CREATE NODE TABLE Rule (
			id STRING,
			memory_group STRING,
			text STRING,
			embedding FLOAT[1536],
			PRIMARY KEY (id)
		)`,
		// Unknown: 知らないことできないこと
		`CREATE NODE TABLE Unknown (
			id STRING,
			memory_group STRING,
			text STRING,
			embedding FLOAT[1536],
			PRIMARY KEY (id)
		)`,
		// Capability: できるようになったこと
		`CREATE NODE TABLE Capability (
			id STRING,
			memory_group STRING,
			text STRING,
			embedding FLOAT[1536],
			PRIMARY KEY (id)
		)`,
	}

	for _, query := range nodeTables {
		if err := s.createTable(query); err != nil {
			return err
		}
	}

	// 2. Rel Tables (Relationships)
	// ---------------------------------------------------------
	relTables := []string{
		// Data -> Document
		`CREATE REL TABLE HAS_DOCUMENT (
			FROM Data TO Document,
			memory_group STRING
		)`,
		// Document -> Chunk
		`CREATE REL TABLE HAS_CHUNK (
			FROM Document TO Chunk,
			memory_group STRING
		)`,
		// Chunk -> Chunk (Sequence)
		`CREATE REL TABLE NEXT_CHUNK (
			FROM Chunk TO Chunk,
			memory_group STRING
		)`,
		// GraphNode -> GraphNode (Knowledge Graph Edges)
		`CREATE REL TABLE GraphEdge (
			FROM GraphNode TO GraphNode,
			memory_group STRING,
			type STRING,
			properties STRING,
			weight DOUBLE,
			confidence DOUBLE
		)`,
	}

	for _, query := range relTables {
		if err := s.createTable(query); err != nil {
			return err
		}
	}

	log.Println("[KuzuDB] EnsureSchema: Schema creation completed.")
	return nil
}

// createTable はテーブル作成を実行し、"already exists" エラーを無視するヘルパー関数です。
func (s *KuzuDBStorage) createTable(query string) error {
	_, err := s.conn.Query(query)
	if err != nil {
		// エラーメッセージに "already exists" が含まれている場合は成功とみなす
		// KuzuDBのエラーメッセージはバージョンによって異なる可能性があるため、
		// 複数のパターンをチェックするか、単純にログを出して続行する戦略を取る。
		errMsg := strings.ToLower(err.Error())
		if strings.Contains(errMsg, "exists") {
			// log.Printf("[info] Table already exists or similar error: %v", err)
			return nil
		}
		return fmt.Errorf("failed to create table with query: %s, error: %w", query, err)
	}
	return nil
}

// =================================================================================
// VectorStorage Interface Implementation (Stub)
// =================================================================================

func (s *KuzuDBStorage) SaveData(ctx context.Context, data *storage.Data) error {
	// KuzuDBで日時を扱う場合はISO8601形式の文字列を使用
	createdAt := data.CreatedAt.Format(time.RFC3339)

	// MERGE を使用して UPSERT を実現
	query := fmt.Sprintf(`
		MERGE (d:Data {id: '%s', memory_group: '%s'})
		ON CREATE SET 
			d.name = '%s',
			d.raw_data_location = '%s',
			d.original_data_location = '%s',
			d.extension = '%s',
			d.mime_type = '%s',
			d.content_hash = '%s',
			d.owner_id = '%s',
			d.created_at = timestamp('%s')
		ON MATCH SET 
			d.name = '%s',
			d.raw_data_location = '%s',
			d.original_data_location = '%s',
			d.extension = '%s',
			d.mime_type = '%s',
			d.content_hash = '%s',
			d.owner_id = '%s',
			d.created_at = timestamp('%s')
	`,
		escapeString(data.ID),
		escapeString(data.MemoryGroup),
		// ON CREATE SET
		escapeString(data.Name),
		escapeString(data.RawDataLocation),
		escapeString(data.OriginalDataLocation),
		escapeString(data.Extension),
		escapeString(data.MimeType),
		escapeString(data.ContentHash),
		escapeString(data.OwnerID),
		createdAt,
		// ON MATCH SET
		escapeString(data.Name),
		escapeString(data.RawDataLocation),
		escapeString(data.OriginalDataLocation),
		escapeString(data.Extension),
		escapeString(data.MimeType),
		escapeString(data.ContentHash),
		escapeString(data.OwnerID),
		createdAt,
	)

	result, err := s.conn.Query(query)
	if err != nil {
		return fmt.Errorf("failed to save data: %w", err)
	}
	result.Close()
	return nil
}

func (s *KuzuDBStorage) Exists(ctx context.Context, contentHash string, memoryGroup string) bool {
	query := fmt.Sprintf(`
		MATCH (d:Data)
		WHERE d.content_hash = '%s' AND d.memory_group = '%s'
		RETURN count(d)
	`, escapeString(contentHash), escapeString(memoryGroup))
	result, err := s.conn.Query(query)
	if err != nil {
		log.Printf("[WARN] Exists query failed: %v", err)
		return false
	}
	defer result.Close()
	if result.HasNext() {
		row, _ := result.Next()
		cntV, _ := row.GetValue(0)
		cnt := getInt64(cntV)
		return cnt > 0
	}
	return false
}

func (s *KuzuDBStorage) GetDataByID(ctx context.Context, id string, memoryGroup string) (*storage.Data, error) {
	query := fmt.Sprintf(`
		MATCH (d:Data)
		WHERE d.id = '%s' AND d.memory_group = '%s'
		RETURN d.id, d.memory_group, d.name, d.raw_data_location, d.original_data_location, d.extension, d.mime_type, d.content_hash, d.owner_id, d.created_at
	`, escapeString(id), escapeString(memoryGroup))

	result, err := s.conn.Query(query)
	if err != nil {
		return nil, fmt.Errorf("failed to get data by id: %w", err)
	}
	defer result.Close()

	if result.HasNext() {
		row, _ := result.Next()
		// GetValueのインデックスはRETURN句の順序に対応
		data := &storage.Data{}
		if v, _ := row.GetValue(0); v != nil {
			data.ID = getString(v)
		}
		if v, _ := row.GetValue(1); v != nil {
			data.MemoryGroup = getString(v)
		}
		if v, _ := row.GetValue(2); v != nil {
			data.Name = getString(v)
		}
		if v, _ := row.GetValue(3); v != nil {
			data.RawDataLocation = getString(v)
		}
		if v, _ := row.GetValue(4); v != nil {
			data.OriginalDataLocation = getString(v)
		}
		if v, _ := row.GetValue(5); v != nil {
			data.Extension = getString(v)
		}
		if v, _ := row.GetValue(6); v != nil {
			data.MimeType = getString(v)
		}
		if v, _ := row.GetValue(7); v != nil {
			data.ContentHash = getString(v)
		}
		if v, _ := row.GetValue(8); v != nil {
			data.OwnerID = getString(v)
		}
		if v, _ := row.GetValue(9); v != nil {
			data.CreatedAt = parseTimestamp(v)
		}
		return data, nil
	}
	return nil, fmt.Errorf("data not found")
}

func (s *KuzuDBStorage) GetDataList(ctx context.Context, memoryGroup string) ([]*storage.Data, error) {
	query := fmt.Sprintf(`
		MATCH (d:Data)
		WHERE d.memory_group = '%s'
		RETURN d.id, d.memory_group, d.name, d.raw_data_location, d.original_data_location, d.extension, d.mime_type, d.content_hash, d.owner_id, d.created_at
	`, escapeString(memoryGroup))

	result, err := s.conn.Query(query)
	if err != nil {
		return nil, fmt.Errorf("failed to get data list: %w", err)
	}
	defer result.Close()

	var dataList []*storage.Data
	for result.HasNext() {
		row, _ := result.Next()
		data := &storage.Data{}
		if v, _ := row.GetValue(0); v != nil {
			data.ID = getString(v)
		}
		if v, _ := row.GetValue(1); v != nil {
			data.MemoryGroup = getString(v)
		}
		if v, _ := row.GetValue(2); v != nil {
			data.Name = getString(v)
		}
		if v, _ := row.GetValue(3); v != nil {
			data.RawDataLocation = getString(v)
		}
		if v, _ := row.GetValue(4); v != nil {
			data.OriginalDataLocation = getString(v)
		}
		if v, _ := row.GetValue(5); v != nil {
			data.Extension = getString(v)
		}
		if v, _ := row.GetValue(6); v != nil {
			data.MimeType = getString(v)
		}
		if v, _ := row.GetValue(7); v != nil {
			data.ContentHash = getString(v)
		}
		if v, _ := row.GetValue(8); v != nil {
			data.OwnerID = getString(v)
		}
		if v, _ := row.GetValue(9); v != nil {
			data.CreatedAt = parseTimestamp(v)
		}
		dataList = append(dataList, data)
	}
	return dataList, nil
}

func (s *KuzuDBStorage) SaveDocument(ctx context.Context, document *storage.Document) error {
	// MetadataはJSON文字列として保存
	metaJSON, _ := json.Marshal(document.MetaData)
	metaStr := string(metaJSON)

	// 1. Documentノード作成 (MERGE)
	queryDoc := fmt.Sprintf(`
		MERGE (doc:%s {id: '%s', memory_group: '%s'})
		ON CREATE SET
			doc.data_id = '%s',
			doc.text = '%s',
			doc.metadata = '%s'
		ON MATCH SET
			doc.data_id = '%s',
			doc.text = '%s',
			doc.metadata = '%s'
	`,
		types.TABLE_NAME_DOCUMENT,
		escapeString(document.ID),
		escapeString(document.MemoryGroup),
		// ON CREATE
		escapeString(document.DataID),
		escapeString(document.Text),
		escapeString(metaStr),
		// ON MATCH
		escapeString(document.DataID),
		escapeString(document.Text),
		escapeString(metaStr),
	)

	if _, err := s.conn.Query(queryDoc); err != nil {
		return fmt.Errorf("failed to save document node: %w", err)
	}

	// 2. Data -> Document リレーションシップ (HAS_DOCUMENT)
	// リレーションのMERGEはKuzuDBのバージョンによってはサポート外の場合があるが、
	// 基本的には MATCH ... MATCH ... MERGE (a)-[:REL]->(b) が使える。
	queryRel := fmt.Sprintf(`
		MATCH (d:%s {id: '%s', memory_group: '%s'}), (doc:%s {id: '%s', memory_group: '%s'})
		MERGE (d)-[r:HAS_DOCUMENT {memory_group: '%s'}]->(doc)
	`,
		types.TABLE_NAME_DATA,
		escapeString(document.DataID), escapeString(document.MemoryGroup),
		types.TABLE_NAME_DOCUMENT,
		escapeString(document.ID), escapeString(document.MemoryGroup),
		escapeString(document.MemoryGroup),
	)

	if _, err := s.conn.Query(queryRel); err != nil {
		// Dataノードが見つからない場合など
		return fmt.Errorf("failed to create HAS_DOCUMENT relation: %w", err)
	}

	return nil
}

func (s *KuzuDBStorage) SaveChunk(ctx context.Context, chunk *storage.Chunk) error {
	// 1. Chunkノード作成 (MERGE)
	// Embeddingがあれば保存 (FLOAT[1536])
	embeddingStr := "NULL"
	if len(chunk.Embedding) > 0 {
		embeddingStr = formatVectorForKuzuDB(chunk.Embedding)
	}
	queryChunk := fmt.Sprintf(`
		MERGE (c:%s {id: '%s', memory_group: '%s'})
		ON CREATE SET
			c.document_id = '%s',
			c.text = '%s',
			c.token_count = %d,
			c.chunk_index = %d,
			c.embedding = %s
		ON MATCH SET
			c.document_id = '%s',
			c.text = '%s',
			c.token_count = %d,
			c.chunk_index = %d,
			c.embedding = %s
	`,
		types.TABLE_NAME_CHUNK,
		escapeString(chunk.ID),
		escapeString(chunk.MemoryGroup),
		// ON CREATE
		escapeString(chunk.DocumentID),
		escapeString(chunk.Text),
		chunk.TokenCount,
		chunk.ChunkIndex,
		embeddingStr,
		// ON MATCH
		escapeString(chunk.DocumentID),
		escapeString(chunk.Text),
		chunk.TokenCount,
		chunk.ChunkIndex,
		embeddingStr,
	)
	if _, err := s.conn.Query(queryChunk); err != nil {
		return fmt.Errorf("Failed to save chunk node: %w", err)
	}
	// 2. Document -> Chunk リレーションシップ (HAS_CHUNK)
	queryRel := fmt.Sprintf(`
		MATCH (doc:%s {id: '%s', memory_group: '%s'}), (c:%s {id: '%s', memory_group: '%s'})
		MERGE (doc)-[r:HAS_CHUNK {memory_group: '%s'}]->(c)
	`,
		types.TABLE_NAME_DOCUMENT,
		escapeString(chunk.DocumentID), escapeString(chunk.MemoryGroup),
		types.TABLE_NAME_CHUNK,
		escapeString(chunk.ID), escapeString(chunk.MemoryGroup),
		escapeString(chunk.MemoryGroup),
	)
	if _, err := s.conn.Query(queryRel); err != nil {
		return fmt.Errorf("Failed to create HAS_CHUNK relation: %w", err)
	}
	return nil
}

func (s *KuzuDBStorage) SaveEmbedding(ctx context.Context, tableName types.TableName, id string, text string, vector []float32, memoryGroup string) error {
	if len(vector) == 0 {
		return nil
	}
	vecStr := formatVectorForKuzuDB(vector)
	// IDでChunkを検索し、embedding と text を更新
	query := fmt.Sprintf(`
		MERGE (c:%s {id: '%s'})
		ON CREATE SET
			c.memory_group = '%s',
			c.embedding = %s,
			c.text = '%s'
		ON MATCH SET
			c.memory_group = '%s',
			c.embedding = %s,
			c.text = '%s'
	`, tableName, escapeString(id), escapeString(memoryGroup), vecStr, escapeString(text), escapeString(memoryGroup), vecStr, escapeString(text))
	if _, err := s.conn.Query(query); err != nil {
		return fmt.Errorf("Failed to save embedding: %w", err)
	}
	return nil
}

func (s *KuzuDBStorage) Query(ctx context.Context, tableName types.TableName, vector []float32, topk int, memoryGroup string) ([]*storage.QueryResult, error) {
	if len(vector) == 0 {
		return nil, fmt.Errorf("Query vector is empty.")
	}
	vecStr := formatVectorForKuzuDB(vector)
	// array_cosine_similarity 関数を使って類似度計算
	// KuzuDBのバージョンによっては cosine_similarity または array_cosine_similarity
	query := fmt.Sprintf(`
		MATCH (c:%s)
		WHERE c.memory_group = '%s' AND c.embedding IS NOT NULL
		RETURN c.id, c.text, array_cosine_similarity(c.embedding, %s) AS score
		ORDER BY score DESC
		LIMIT %d
	`, tableName, escapeString(memoryGroup), vecStr, topk)
	result, err := s.conn.Query(query)
	if err != nil {
		return nil, fmt.Errorf("Query failed: %w", err)
	}
	defer result.Close()
	var results []*storage.QueryResult
	for result.HasNext() {
		row, err := result.Next()
		if err != nil {
			return nil, fmt.Errorf("Query next failed: %w", err)
		}
		res := &storage.QueryResult{}
		if v, _ := row.GetValue(0); v != nil {
			res.ID = getString(v)
		}
		if v, _ := row.GetValue(1); v != nil {
			res.Text = getString(v)
		}
		if v, _ := row.GetValue(2); v != nil {
			res.Distance = getFloat64(v)
		}
		results = append(results, res)
	}
	return results, nil
}

func (s *KuzuDBStorage) GetEmbeddingByID(ctx context.Context, tableName types.TableName, id string, memoryGroup string) ([]float32, error) {
	// Chunkテーブルから取得
	query := fmt.Sprintf(`
		MATCH (c:%s)
		WHERE c.id = '%s' AND c.memory_group = '%s'
		RETURN c.embedding
	`, tableName, escapeString(id), escapeString(memoryGroup))
	result, err := s.conn.Query(query)
	if err != nil {
		return nil, fmt.Errorf("failed to get embedding: %w", err)
	}
	defer result.Close()
	if result.HasNext() {
		row, _ := result.Next()
		if v, _ := row.GetValue(0); v != nil {
			return parseEmbedding(v), nil
		}
	}
	return nil, nil // Not found
}

func (s *KuzuDBStorage) GetEmbeddingsByIDs(ctx context.Context, tableName types.TableName, ids []string, memoryGroup string) (map[string][]float32, error) {
	if len(ids) == 0 {
		return make(map[string][]float32), nil
	}
	// IDリストを作成
	var idListStr strings.Builder
	idListStr.WriteString("[")
	for i, id := range ids {
		if i > 0 {
			idListStr.WriteString(", ")
		}
		idListStr.WriteString(fmt.Sprintf("'%s'", escapeString(id)))
	}
	idListStr.WriteString("]")
	// KuzuDBのIN句またはlist_containsを使用
	// MATCH (c:Chunk) WHERE c.id IN [...]
	query := fmt.Sprintf(`
		MATCH (c:%s)
		WHERE c.memory_group = '%s' AND c.id IN %s
		RETURN c.id, c.embedding
	`, tableName, escapeString(memoryGroup), idListStr.String())
	result, err := s.conn.Query(query)
	if err != nil {
		return nil, fmt.Errorf("Failed to get embeddings list: %w", err)
	}
	defer result.Close()
	embeddings := make(map[string][]float32)
	for result.HasNext() {
		row, _ := result.Next()
		var id string
		var vec []float32
		if v, _ := row.GetValue(0); v != nil {
			id = getString(v)
		}
		if v, _ := row.GetValue(1); v != nil {
			vec = parseEmbedding(v)
		}
		if id != "" && len(vec) > 0 {
			embeddings[id] = vec
		}
	}
	return embeddings, nil
}

// =================================================================================
// GraphStorage Interface Implementation (Stub)
// =================================================================================

func (s *KuzuDBStorage) AddNodes(ctx context.Context, nodes []*storage.Node) error {
	if len(nodes) == 0 {
		return nil
	}
	for _, node := range nodes {
		propsJSON, _ := json.Marshal(node.Properties)
		propsStr := string(propsJSON)
		query := fmt.Sprintf(`
			MERGE (n:%s {id: '%s', memory_group: '%s'})
			ON CREATE SET 
				n.type = '%s',
				n.properties = '%s'
			ON MATCH SET 
				n.type = '%s',
				n.properties = '%s'
		`,
			types.TABLE_NAME_GRAPH_NODE,
			escapeString(node.ID),
			escapeString(node.MemoryGroup),
			// ON CREATE
			escapeString(node.Type),
			escapeString(propsStr),
			// ON MATCH
			escapeString(node.Type),
			escapeString(propsStr),
		)
		if _, err := s.conn.Query(query); err != nil {
			return fmt.Errorf("Failed to add node %s: %w", node.ID, err)
		}
	}
	return nil
}

func (s *KuzuDBStorage) AddEdges(ctx context.Context, edges []*storage.Edge) error {
	if len(edges) == 0 {
		return nil
	}
	for _, edge := range edges {
		propsJSON, _ := json.Marshal(edge.Properties)
		propsStr := string(propsJSON)
		// 両端のノードが存在する必要があります
		// MERGEを使ってエッジをUPSERTします
		// エッジの識別は (Source, Target, Type) で行うのが一般的ですが、
		// ここでは GraphEdge という単一のリレーションタイプを使用し、typeプロパティで区別します。
		// しかし、MERGEでプロパティ指定すると、そのプロパティを持つエッジを探します。
		// typeが変わる可能性があるなら、MERGEのキーにtypeを含めるべきです。
		query := fmt.Sprintf(`
			MATCH (a:%s {id: '%s', memory_group: '%s'}), (b:%s {id: '%s', memory_group: '%s'})
			MERGE (a)-[r:%s {memory_group: '%s', type: '%s'}]->(b)
			ON CREATE SET 
				r.properties = '%s',
				r.weight = %f,
				r.confidence = %f
			ON MATCH SET 
				r.properties = '%s',
				r.weight = %f,
				r.confidence = %f
		`,
			types.TABLE_NAME_GRAPH_NODE,
			escapeString(edge.SourceID), escapeString(edge.MemoryGroup),
			types.TABLE_NAME_GRAPH_NODE,
			escapeString(edge.TargetID), escapeString(edge.MemoryGroup),
			// MERGE Keys
			types.TABLE_NAME_GRAPH_EDGE,
			escapeString(edge.MemoryGroup),
			escapeString(edge.Type),
			// ON CREATE
			escapeString(propsStr),
			edge.Weight,
			edge.Confidence,
			// ON MATCH
			escapeString(propsStr),
			edge.Weight,
			edge.Confidence,
		)
		if _, err := s.conn.Query(query); err != nil {
			return fmt.Errorf("Failed to add edge %s->%s: %w", edge.SourceID, edge.TargetID, err)
		}
	}
	return nil
}

func (s *KuzuDBStorage) GetTriples(ctx context.Context, nodeIDs []string, memoryGroup string) ([]*storage.Triple, error) {
	if len(nodeIDs) == 0 {
		return nil, nil
	}
	// IDリスト作成
	var idListStr strings.Builder
	idListStr.WriteString("[")
	for i, id := range nodeIDs {
		if i > 0 {
			idListStr.WriteString(", ")
		}
		idListStr.WriteString(fmt.Sprintf("'%s'", escapeString(id)))
	}
	idListStr.WriteString("]")
	// 指定されたノードID群に関連する(SourceまたはTargetとなる)エッジとその両端ノードを取得
	query := fmt.Sprintf(`
		MATCH (a:%s {memory_group: '%s'})-[r:%s {memory_group: '%s'}]->(b:%s {memory_group: '%s'})
		WHERE a.id IN %s OR b.id IN %s
		RETURN 
			a.id, a.type, a.properties, 
			r.type, r.properties, r.weight, r.confidence,
			b.id, b.type, b.properties
	`,
		types.TABLE_NAME_GRAPH_NODE,
		escapeString(memoryGroup),
		types.TABLE_NAME_GRAPH_EDGE,
		escapeString(memoryGroup),
		types.TABLE_NAME_GRAPH_NODE,
		escapeString(memoryGroup),
		idListStr.String(), idListStr.String(),
	)
	result, err := s.conn.Query(query)
	if err != nil {
		return nil, fmt.Errorf("failed to get triples: %w", err)
	}
	defer result.Close()
	var triples []*storage.Triple
	for result.HasNext() {
		row, _ := result.Next()
		// Parse Node A
		nodeA := &storage.Node{MemoryGroup: memoryGroup}
		if v, _ := row.GetValue(0); v != nil {
			tmpID := getString(v)
			nodeA.ID = utils.GetNameStrByGraphNodeID(tmpID) // IDのメモリーグループを除去
		}
		if v, _ := row.GetValue(1); v != nil {
			nodeA.Type = getString(v)
		}
		if v, _ := row.GetValue(2); v != nil {
			nodeA.Properties = parseJSONProperties(getString(v))
		}
		// Parse Edge
		edge := &storage.Edge{MemoryGroup: memoryGroup}
		edge.SourceID = nodeA.ID // 仮代入、後で調整不要
		// r.type
		if v, _ := row.GetValue(3); v != nil {
			edge.Type = getString(v)
		}
		if v, _ := row.GetValue(4); v != nil {
			edge.Properties = parseJSONProperties(getString(v))
		}
		if v, _ := row.GetValue(5); v != nil {
			edge.Weight = getFloat64(v)
		}
		if v, _ := row.GetValue(6); v != nil {
			edge.Confidence = getFloat64(v)
		}
		// Parse Node B
		nodeB := &storage.Node{MemoryGroup: memoryGroup}
		if v, _ := row.GetValue(7); v != nil {
			tmpID := getString(v)
			nodeB.ID = utils.GetNameStrByGraphNodeID(tmpID) // IDのメモリーグループを除去
		}
		edge.TargetID = nodeB.ID
		if v, _ := row.GetValue(8); v != nil {
			nodeB.Type = getString(v)
		}
		if v, _ := row.GetValue(9); v != nil {
			nodeB.Properties = parseJSONProperties(getString(v))
		}
		triples = append(triples, &storage.Triple{
			Source: nodeA,
			Target: nodeB,
			Edge:   edge,
		})
	}
	return triples, nil
}

func (s *KuzuDBStorage) StreamDocumentChunks(ctx context.Context, memoryGroup string) (<-chan *storage.ChunkData, <-chan error) {
	outCh := make(chan *storage.ChunkData)
	errCh := make(chan error, 1)

	go func() {
		defer close(outCh)
		defer close(errCh)

		// Chunkテーブルを全検索
		query := fmt.Sprintf(`
			MATCH (c:%s {memory_group: '%s'})
			RETURN c.id, c.text, c.memory_group, c.document_id
		`, types.TABLE_NAME_CHUNK, escapeString(memoryGroup))

		result, err := s.conn.Query(query)
		if err != nil {
			errCh <- fmt.Errorf("query failed: %w", err)
			return
		}
		defer result.Close()

		for result.HasNext() {
			select {
			case <-ctx.Done():
				return // Context canceled
			default:
				row, _ := result.Next()
				chunk := &storage.ChunkData{}
				if v, _ := row.GetValue(0); v != nil {
					chunk.ID = getString(v)
				}
				if v, _ := row.GetValue(1); v != nil {
					chunk.Text = getString(v)
				}
				if v, _ := row.GetValue(2); v != nil {
					chunk.MemoryGroup = getString(v)
				}
				if v, _ := row.GetValue(3); v != nil {
					chunk.DocumentID = getString(v)
				}

				outCh <- chunk
			}
		}
	}()

	return outCh, errCh
}

func (s *KuzuDBStorage) GetDocumentChunkCount(ctx context.Context, memoryGroup string) (int, error) {
	query := fmt.Sprintf(`
		MATCH (c:%s {memory_group: '%s'})
		RETURN count(c)
	`, types.TABLE_NAME_CHUNK, escapeString(memoryGroup))

	result, err := s.conn.Query(query)
	if err != nil {
		return 0, fmt.Errorf("failed to get chunk count: %w", err)
	}
	defer result.Close()

	if result.HasNext() {
		row, _ := result.Next()
		if v, _ := row.GetValue(0); v != nil {
			return int(getInt64(v)), nil
		}
	}
	return 0, nil
}

func (s *KuzuDBStorage) GetNodesByType(ctx context.Context, nodeType string, memoryGroup string) ([]*storage.Node, error) {
	query := fmt.Sprintf(`
		MATCH (n:%s {memory_group: '%s', type: '%s'})
		RETURN n.id, n.type, n.properties
	`, types.TABLE_NAME_GRAPH_NODE, escapeString(memoryGroup), escapeString(nodeType))

	result, err := s.conn.Query(query)
	if err != nil {
		return nil, fmt.Errorf("failed to get nodes by type: %w", err)
	}
	defer result.Close()

	var nodes []*storage.Node
	for result.HasNext() {
		row, _ := result.Next()
		node := &storage.Node{MemoryGroup: memoryGroup}
		if v, _ := row.GetValue(0); v != nil {
			node.ID = getString(v)
		}
		if v, _ := row.GetValue(1); v != nil {
			node.Type = getString(v)
		}
		if v, _ := row.GetValue(2); v != nil {
			node.Properties = parseJSONProperties(getString(v))
		}
		nodes = append(nodes, node)
	}
	return nodes, nil
}

func (s *KuzuDBStorage) GetNodesByEdge(ctx context.Context, targetID string, edgeType string, memoryGroup string) ([]*storage.Node, error) {
	// targetIDに向かうエッジを持つSourceノードを取得
	query := fmt.Sprintf(`
		MATCH (n:%s {memory_group: '%s'})-[r:%s {memory_group: '%s', type: '%s'}]->(t:%s {id: '%s', memory_group: '%s'})
		RETURN n.id, n.type, n.properties
	`,
		types.TABLE_NAME_GRAPH_NODE,
		escapeString(memoryGroup),
		types.TABLE_NAME_GRAPH_EDGE,
		escapeString(memoryGroup),
		escapeString(edgeType),
		types.TABLE_NAME_GRAPH_NODE,
		escapeString(targetID),
		escapeString(memoryGroup),
	)

	result, err := s.conn.Query(query)
	if err != nil {
		return nil, fmt.Errorf("failed to get nodes by edge: %w", err)
	}
	defer result.Close()

	var nodes []*storage.Node
	for result.HasNext() {
		row, _ := result.Next()
		node := &storage.Node{MemoryGroup: memoryGroup}
		if v, _ := row.GetValue(0); v != nil {
			node.ID = getString(v)
		}
		if v, _ := row.GetValue(1); v != nil {
			node.Type = getString(v)
		}
		if v, _ := row.GetValue(2); v != nil {
			node.Properties = parseJSONProperties(getString(v))
		}
		nodes = append(nodes, node)
	}
	return nodes, nil
}

func (s *KuzuDBStorage) UpdateEdgeWeight(ctx context.Context, sourceID, targetID, memoryGroup string, weight float64) error {
	query := fmt.Sprintf(`
		MATCH (a:%s {id: '%s', memory_group: '%s'})-[r:%s {memory_group: '%s'}]->(b:%s {id: '%s', memory_group: '%s'})
		SET r.weight = %f
	`,
		types.TABLE_NAME_GRAPH_NODE,
		escapeString(sourceID),
		escapeString(memoryGroup),
		types.TABLE_NAME_GRAPH_EDGE,
		escapeString(memoryGroup),
		types.TABLE_NAME_GRAPH_NODE,
		escapeString(targetID),
		escapeString(memoryGroup),
		weight,
	)

	if _, err := s.conn.Query(query); err != nil {
		return fmt.Errorf("failed to update edge weight: %w", err)
	}
	return nil
}

func (s *KuzuDBStorage) UpdateEdgeMetrics(ctx context.Context, sourceID, targetID, memoryGroup string, weight, confidence float64) error {
	query := fmt.Sprintf(`
		MATCH (a:%s {id: '%s', memory_group: '%s'})-[r:%s {memory_group: '%s'}]->(b:%s {id: '%s', memory_group: '%s'})
		SET r.weight = %f, r.confidence = %f
	`,
		types.TABLE_NAME_GRAPH_NODE,
		escapeString(sourceID),
		escapeString(memoryGroup),
		types.TABLE_NAME_GRAPH_EDGE,
		escapeString(memoryGroup),
		types.TABLE_NAME_GRAPH_NODE,
		escapeString(targetID),
		escapeString(memoryGroup),
		weight,
		confidence,
	)

	if _, err := s.conn.Query(query); err != nil {
		return fmt.Errorf("failed to update edge metrics: %w", err)
	}
	return nil
}

func (s *KuzuDBStorage) DeleteEdge(ctx context.Context, sourceID, targetID, memoryGroup string) error {
	query := fmt.Sprintf(`
		MATCH (a:%s {id: '%s', memory_group: '%s'})-[r:%s {memory_group: '%s'}]->(b:%s {id: '%s', memory_group: '%s'})
		DELETE r
	`,
		types.TABLE_NAME_GRAPH_NODE,
		escapeString(sourceID),
		escapeString(memoryGroup),
		types.TABLE_NAME_GRAPH_EDGE,
		escapeString(memoryGroup),
		types.TABLE_NAME_GRAPH_NODE,
		escapeString(targetID),
		escapeString(memoryGroup),
	)

	if _, err := s.conn.Query(query); err != nil {
		return fmt.Errorf("failed to delete edge: %w", err)
	}
	return nil
}

func (s *KuzuDBStorage) DeleteNode(ctx context.Context, nodeID, memoryGroup string) error {
	// DETACH DELETE n (リレーションも削除)
	query := fmt.Sprintf(`
		MATCH (n:%s {id: '%s', memory_group: '%s'})
		DETACH DELETE n
	`, types.TABLE_NAME_GRAPH_NODE, escapeString(nodeID), escapeString(memoryGroup))

	if _, err := s.conn.Query(query); err != nil {
		return fmt.Errorf("failed to delete node: %w", err)
	}
	return nil
}

func (s *KuzuDBStorage) GetEdgesByNode(ctx context.Context, nodeID string, memoryGroup string) ([]*storage.Edge, error) {
	// nodeIDがSourceまたはTargetとなっているエッジを取得
	query := fmt.Sprintf(`
		MATCH (a:%s {memory_group: '%s'})-[r:%s {memory_group: '%s'}]->(b:%s {memory_group: '%s'})
		WHERE a.id = '%s' OR b.id = '%s'
		RETURN a.id, b.id, r.type, r.properties, r.weight, r.confidence
	`,
		types.TABLE_NAME_GRAPH_NODE,
		escapeString(memoryGroup),
		types.TABLE_NAME_GRAPH_EDGE,
		escapeString(memoryGroup),
		types.TABLE_NAME_GRAPH_NODE,
		escapeString(memoryGroup),
		escapeString(nodeID),
		escapeString(nodeID),
	)

	result, err := s.conn.Query(query)
	if err != nil {
		return nil, fmt.Errorf("failed to get edges by node: %w", err)
	}
	defer result.Close()

	var edges []*storage.Edge
	for result.HasNext() {
		row, _ := result.Next()
		edge := &storage.Edge{MemoryGroup: memoryGroup}
		if v, _ := row.GetValue(0); v != nil {
			edge.SourceID = getString(v)
		}
		if v, _ := row.GetValue(1); v != nil {
			edge.TargetID = getString(v)
		}
		if v, _ := row.GetValue(2); v != nil {
			edge.Type = getString(v)
		}
		if v, _ := row.GetValue(3); v != nil {
			edge.Properties = parseJSONProperties(getString(v))
		}
		if v, _ := row.GetValue(4); v != nil {
			edge.Weight = getFloat64(v)
		}
		if v, _ := row.GetValue(5); v != nil {
			edge.Confidence = getFloat64(v)
		}
		edges = append(edges, edge)
	}
	return edges, nil
}

func (s *KuzuDBStorage) GetOrphanNodes(ctx context.Context, memoryGroup string, gracePeriod time.Duration) ([]*storage.Node, error) {
	// エッジを持たないノードを取得
	// OPTIONAL MATCH (n)-[r]-() WHERE r IS NULL
	// KuzuDB: MATCH (n) WHERE NOT (n)-[]-()

	// TODO: gracePeriod (created_at?) 対応はGraphNodeスキーマにcreated_atがないためスキップ
	// 必要ならGraphNodeにcreated_atを追加するか、プロパティ内を検索する。
	// ここでは単純に孤立ノードを返す。

	query := fmt.Sprintf(`
		MATCH (n:%s {memory_group: '%s'})
		WHERE NOT (n)-[]-()
		RETURN n.id, n.type, n.properties
	`, types.TABLE_NAME_GRAPH_NODE, escapeString(memoryGroup))

	result, err := s.conn.Query(query)
	if err != nil {
		return nil, fmt.Errorf("failed to get orphan nodes: %w", err)
	}
	defer result.Close()

	var nodes []*storage.Node
	for result.HasNext() {
		row, _ := result.Next()
		node := &storage.Node{MemoryGroup: memoryGroup}
		if v, _ := row.GetValue(0); v != nil {
			node.ID = getString(v)
		}
		if v, _ := row.GetValue(1); v != nil {
			node.Type = getString(v)
		}
		if v, _ := row.GetValue(2); v != nil {
			node.Properties = parseJSONProperties(getString(v))
		}
		nodes = append(nodes, node)
	}
	return nodes, nil
}

// =================================================================================
// Helper Functions (Phase-10B)
// =================================================================================

// escapeString は文字列をCypherクエリ用にエスケープします。
// バックスラッシュとシングルクォートをエスケープします。
func escapeString(s string) string {
	s = strings.ReplaceAll(s, "\\", "\\\\")
	s = strings.ReplaceAll(s, "'", "\\'")
	return s
}

// getString はkuzu.Valueから文字列を取得するヘルパー関数です。
// 型アサーションを行い、失敗した場合はログを出力して空文字を返します。
// より厳密なエラーハンドリングが必要な場合は、戻り地でerrorを返すように変更してください。
func getString(v any) string {
	if v == nil {
		return ""
	}
	s, ok := v.(string)
	if !ok {
		log.Printf("[WARN] Expected string but got %T: %v", v, v)
		return ""
	}
	return s
}

// getInt64 はkuzu.Valueからint64を取得するヘルパー関数です。
func getInt64(v any) int64 {
	if v == nil {
		return 0
	}
	// KuzuDBの数値型はGoの型にマッピングされる際に注意が必要
	switch val := v.(type) {
	case int64:
		return val
	case int:
		return int64(val)
	case float64:
		return int64(val)
	default:
		log.Printf("[WARN] Expected int64 but got %T: %v", v, v)
		return 0
	}
}

// getFloat64 はkuzu.Valueからfloat64を取得するヘルパー関数です。
func getFloat64(v any) float64 {
	if v == nil {
		return 0.0
	}
	switch val := v.(type) {
	case float64:
		return val
	case float32:
		return float64(val)
	case int64:
		return float64(val)
	default:
		log.Printf("[WARN] Expected float64 but got %T: %v", v, v)
		return 0.0
	}
}

// formatVectorForKuzuDB は []float32 を KuzuDB の FLOAT[] リテラル文字列に変換します。
// 例: [0.1, 0.2, 0.3] -> "[0.1,0.2,0.3]"
func formatVectorForKuzuDB(vec []float32) string {
	var sb strings.Builder
	sb.WriteString("[")
	for i, v := range vec {
		if i > 0 {
			sb.WriteString(",")
		}
		sb.WriteString(fmt.Sprintf("%f", v))
	}
	sb.WriteString("]")
	return sb.String()
}

// parseEmbedding は KuzuDB から取得した値を []float32 に変換します。
func parseEmbedding(v any) []float32 {
	if v == nil {
		return nil
	}

	switch val := v.(type) {
	case string:
		// 文字列として返ってくる場合 (例: "[0.1, 0.2]")
		s := strings.Trim(val, "[]")
		parts := strings.Split(s, ",")
		vec := make([]float32, 0, len(parts))
		for _, p := range parts {
			var f float32
			fmt.Sscanf(strings.TrimSpace(p), "%f", &f)
			vec = append(vec, f)
		}
		return vec

	case []any:
		// インターフェースのスライスとして返ってくる場合
		vec := make([]float32, len(val))
		for i, item := range val {
			vec[i] = float32(getFloat64(item))
		}
		return vec

	// go-kuzuが直接スライスを返す場合
	case []float32:
		return val
	case []float64:
		vec := make([]float32, len(val))
		for i, f := range val {
			vec[i] = float32(f)
		}
		return vec

	default:
		log.Printf("[WARN] parseEmbedding: Unknown type %T: %v", v, v)
		return nil
	}
}

// parseJSONProperties は JSON文字列を map[string]any に変換します。
func parseJSONProperties(s string) map[string]any {
	var m map[string]any
	if err := json.Unmarshal([]byte(s), &m); err != nil {
		log.Printf("[WARN] Failed to parse JSON properties: %v", err)
		return make(map[string]any)
	}
	return m
}

// formatTimestamp は time.Time を KuzuDB の datetime() 関数で使用できる形式に変換します。
// RFC3339形式を使用します。
func formatTimestamp(t time.Time) string {
	return t.Format(time.RFC3339)
}

// parseTimestamp は KuzuDB から取得した値を time.Time に変換します。
func parseTimestamp(v any) time.Time {
	if v == nil {
		return time.Time{}
	}
	switch val := v.(type) {
	case time.Time:
		return val
	case string:
		t, err := time.Parse(time.RFC3339, val)
		if err != nil {
			log.Printf("[WARN] Failed to parse timestamp string: %v", err)
			return time.Time{}
		}
		return t
	default:
		log.Printf("[WARN] Expected time.Time or string but got %T: %v", v, v)
		return time.Time{}
	}
}
