package ladybugdb

import (
	"context"
	"encoding/json"
	"fmt"
	"strings"
	"sync"
	"time"

	"github.com/ikawaha/kagome/v2/tokenizer"
	"github.com/t-kawata/mycute/lib/common"
	"github.com/t-kawata/mycute/pkg/cuber/storage"
	"github.com/t-kawata/mycute/pkg/cuber/types"
	"github.com/t-kawata/mycute/pkg/cuber/utils"

	ladybug "github.com/t-kawata/mycute/pkg/cuber/lib/go-ladybug"
	"go.uber.org/zap"
)

type contextKey string

const TX_CONN_KEY contextKey = "TX_CONN"

// LadybugDBStorage は、LadybugDBを使用した統合ストレージ実装です。
// VectorStorage と GraphStorage の両インターフェースを実装します。
type LadybugDBStorage struct {
	db     *ladybug.Database
	conn   *ladybug.Connection  // デフォルト接続（非トランザクション用）
	kagome *tokenizer.Tokenizer // 日本語形態素解析器（Kagome）- FTS用
	Logger *zap.Logger
	mu     sync.Mutex // トランザクションのシリアライズ用
}

// コンパイル時チェック: インターフェースを満たしているか確認
var _ storage.VectorStorage = (*LadybugDBStorage)(nil)
var _ storage.GraphStorage = (*LadybugDBStorage)(nil)

// NewLadybugDBStorage は新しい LadybugDBStorage インスタンスを作成します。
// kagome: 日本語形態素解析器（CuberServiceのシングルトンを共有）
func NewLadybugDBStorage(dbPath string, kagome *tokenizer.Tokenizer, l *zap.Logger) (*LadybugDBStorage, error) {
	var db *ladybug.Database
	var err error
	// データベースを開く
	if dbPath == ":memory:" {
		utils.LogInfo(l, "LadybugDB: Opening in-memory database")
		db, err = ladybug.OpenInMemoryDatabase(ladybug.DefaultSystemConfig())
	} else {
		db, err = ladybug.OpenDatabase(dbPath, ladybug.DefaultSystemConfig())
	}
	if err != nil {
		return nil, fmt.Errorf("Failed to open LadybugDB database: %w", err)
	}
	// 接続を開く
	conn, err := ladybug.OpenConnection(db)
	if err != nil {
		db.Close()
		return nil, fmt.Errorf("Failed to open LadybugDB connection: %w", err)
	}

	// FTS拡張のインストールとロード
	// これにより create_fts_index, query_fts_index 等の関数が利用可能になります
	// LadybugDB/KuzuDBでは大文字小文字両方で試行
	ftsCommands := []string{
		"INSTALL fts",
		"LOAD EXTENSION fts",
	}
	for _, cmd := range ftsCommands {
		if result, err := conn.Query(cmd); err != nil {
			utils.LogWarn(l, fmt.Sprintf("LadybugDB: FTS command failed: %s", cmd), zap.Error(err))
			// 拡張が見つからない場合や既にロード済みの場合があるので、エラーは無視
		} else {
			result.Close()
			utils.LogDebug(l, fmt.Sprintf("LadybugDB: FTS command succeeded: %s", cmd))
		}
	}

	return &LadybugDBStorage{
		db:     db,
		conn:   conn,
		kagome: kagome,
		Logger: l,
	}, nil
}

// Close はリソースを解放します。
func (s *LadybugDBStorage) Close() error {
	if s.conn != nil {
		// 明示的にチェックポイントを実行してWALをマージする
		// これにより、プロセス終了後に他ツール（lbug CLIなど）から安全にアクセスできるようになります。
		s.Checkpoint()
		s.conn.Close()
		s.conn = nil
	}
	if s.db != nil {
		s.db.Close()
		s.db = nil
	}
	return nil
}

// Checkpoint は、WAL（Write-Ahead Log）をメインのデータベースファイルにマージします。
// getConn はコンテキストからトランザクション用接続を取得します。
// 存在しない場合は、ストレージのデフォルト接続を返します。
func (s *LadybugDBStorage) getConn(ctx context.Context) *ladybug.Connection {
	if conn, ok := ctx.Value(TX_CONN_KEY).(*ladybug.Connection); ok {
		return conn
	}
	return s.conn
}

// checkContext はコンテキストがキャンセルされているか確認します。
func (s *LadybugDBStorage) checkContext(ctx context.Context) error {
	select {
	case <-ctx.Done():
		return ctx.Err()
	default:
		return nil
	}
}

// Transaction は、新しい接続をオープンしてトランザクションを実行します。
// Mutex により、書き込みトランザクションの競合を防止します。
func (s *LadybugDBStorage) Transaction(ctx context.Context, fn func(txCtx context.Context) error) error {
	s.mu.Lock()
	defer s.mu.Unlock()

	// 1. 新しい接続のオープン（独立性を確保）
	conn, err := ladybug.OpenConnection(s.db)
	if err != nil {
		return fmt.Errorf("LadybugDB: Failed to open transaction connection: %w", err)
	}
	defer conn.Close()

	// 2. トランザクション開始
	if result, err := conn.Query("BEGIN TRANSACTION"); err != nil {
		return fmt.Errorf("LadybugDB: Failed to begin transaction: %w", err)
	} else {
		result.Close()
	}

	// 3. 接続を Context に埋め込んで実行
	txCtx := context.WithValue(ctx, TX_CONN_KEY, conn)

	// パニック復旧のための無名関数
	err = func() (err error) {
		defer func() {
			if r := recover(); r != nil {
				err = fmt.Errorf("LadybugDB: Panic in transaction: %v", r)
			}
		}()
		return fn(txCtx)
	}()

	if err != nil {
		// ロールバック
		if res, rerr := conn.Query("ROLLBACK"); rerr == nil {
			res.Close()
		}
		return err
	}

	// 4. コミット
	if result, err := conn.Query("COMMIT"); err != nil {
		return fmt.Errorf("LadybugDB: Commit failed: %w", err)
	} else {
		result.Close()
	}
	return nil
}

func (s *LadybugDBStorage) Checkpoint() error {
	if s.conn != nil {
		s.mu.Lock()
		defer s.mu.Unlock()
		if result, err := s.conn.Query("CHECKPOINT"); err == nil {
			result.Close()
		} else {
			utils.LogWarn(s.Logger, "LadybugDB: Failed to execute CHECKPOINT", zap.Error(err))
			return err
		}
	}
	return nil
}

// IsOpen は、ストレージ接続が開いているかどうかを返します。
func (s *LadybugDBStorage) IsOpen() bool {
	return s.db != nil && s.conn != nil
}

// EnsureSchema は必要なテーブルスキーマを作成します。
// config.Dimension を使用して、ベクトルカラムの次元数を動的に設定します。
func (s *LadybugDBStorage) EnsureSchema(ctx context.Context, config types.EmbeddingModelConfig) error {
	utils.LogDebug(s.Logger, "LadybugDB: Starting schema creation.")
	// ベクトル型の定義文字列を生成 (例: "FLOAT[1536]")
	vectorType := fmt.Sprintf("FLOAT[%d]", config.Dimension)
	// 1. Node Tables
	// ---------------------------------------------------------
	// LadybugDBではDDLをトランザクション内で実行可能かはバージョンによるが、
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
		fmt.Sprintf(`CREATE NODE TABLE Chunk (
			id STRING,
			memory_group STRING,
			document_id STRING,
			text STRING,
			keywords STRING,
			nouns STRING,
			nouns_verbs STRING,
			token_count INT64,
			chunk_index INT64,
			embedding %s,
			PRIMARY KEY (id)
		)`, vectorType),
		// GraphNode: 知識グラフのノード
		`CREATE NODE TABLE GraphNode (
			id STRING,
			memory_group STRING,
			type STRING,
			properties STRING,
			PRIMARY KEY (id)
		)`,
		// Entity: エンティティ（人名、組織名、場所名など）
		fmt.Sprintf(`CREATE NODE TABLE Entity (
			id STRING,
			memory_group STRING,
			text STRING,
			embedding %s,
			PRIMARY KEY (id)
		)`, vectorType),
		// Summary: 要約
		fmt.Sprintf(`CREATE NODE TABLE Summary (
			id STRING,
			memory_group STRING,
			text STRING,
			embedding %s,
			PRIMARY KEY (id)
		)`, vectorType),
		// Rule: ルール
		fmt.Sprintf(`CREATE NODE TABLE Rule (
			id STRING,
			memory_group STRING,
			text STRING,
			embedding %s,
			PRIMARY KEY (id)
		)`, vectorType),
		// Unknown: 知らないことできないこと
		fmt.Sprintf(`CREATE NODE TABLE Unknown (
			id STRING,
			memory_group STRING,
			text STRING,
			embedding %s,
			PRIMARY KEY (id)
		)`, vectorType),
		// Capability: できるようになったこと
		fmt.Sprintf(`CREATE NODE TABLE Capability (
			id STRING,
			memory_group STRING,
			text STRING,
			embedding %s,
			PRIMARY KEY (id)
		)`, vectorType),
		// MemoryGroup: メモリーグループごとの代謝パラメータ
		`CREATE NODE TABLE MemoryGroup (
			id STRING,
			half_life_days DOUBLE,
			prune_threshold DOUBLE,
			min_survival_protection_hours DOUBLE,
			mdl_k_neighbors INT64,
			created_at TIMESTAMP,
			updated_at TIMESTAMP,
			PRIMARY KEY (id)
		)`,
	}

	for _, query := range nodeTables {
		if err := s.createTable(ctx, query); err != nil {
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
			confidence DOUBLE,
			unix INT64
		)`,
	}
	for _, query := range relTables {
		if err := s.createTable(ctx, query); err != nil {
			return err
		}
	}
	// 3. FTS Indexes (Full-Text Search)
	// ---------------------------------------------------------
	// LadybugDB v0.11.3+ では FTS 拡張がプリインストールされています。
	// 各レイヤーのキーワードカラムに対して BM25 ベースの FTS インデックスを作成します。
	ftsIndexes := []string{
		// Layer 0: 名詞のみ (nouns)
		// 構文: create_fts_index('テーブル名', 'インデックス名', ['カラム名'])
		`CALL create_fts_index('Chunk', 'chunk_nouns_idx', ['nouns'])`,
		// Layer 1: 名詞 + 動詞 (nouns_verbs)
		`CALL create_fts_index('Chunk', 'chunk_nouns_verbs_idx', ['nouns_verbs'])`,
		// Layer 2: 全内容語 (keywords)
		`CALL create_fts_index('Chunk', 'chunk_keywords_idx', ['keywords'])`,
	}
	for _, query := range ftsIndexes {
		if err := s.createFtsIndex(ctx, query); err != nil {
			utils.LogWarn(s.Logger, fmt.Sprintf("FTS index creation skipped or failed: %v", err))
			// FTS インデックス作成失敗はスキーマ全体の失敗にはしない
		}
	}
	utils.LogDebug(s.Logger, "LadybugDB: Schema creation completed")
	return nil
}

// createFtsIndex は FTS インデックス作成を実行し、既存の場合はスキップするヘルパー関数です。
func (s *LadybugDBStorage) createFtsIndex(ctx context.Context, query string) error {
	conn := s.getConn(ctx)
	if conn == s.conn {
		s.mu.Lock()
		defer s.mu.Unlock()
	}
	result, err := conn.Query(query)
	if err != nil {
		errMsg := strings.ToLower(err.Error())
		// 既存のインデックスはスキップ
		if strings.Contains(errMsg, "exists") || strings.Contains(errMsg, "already") {
			return nil
		}
		return fmt.Errorf("failed to create FTS index: %w", err)
	}
	result.Close()
	return nil
}

// createTable はテーブル作成を実行し、"already exists" エラーを無視するヘルパー関数です。
func (s *LadybugDBStorage) createTable(ctx context.Context, query string) error {
	conn := s.getConn(ctx)
	if conn == s.conn {
		s.mu.Lock()
		defer s.mu.Unlock()
	}
	result, err := conn.Query(query)
	if err != nil {
		// エラーメッセージに "already exists" が含まれている場合は成功とみなす
		// LadybugDBのエラーメッセージはバージョンによって異なる可能性があるため、
		// 複数のパターンをチェックするか、単純にログを出して続行する戦略を取る。
		errMsg := strings.ToLower(err.Error())
		if strings.Contains(errMsg, "exists") {
			// log.Printf("[info] Table already exists or similar error: %v", err)
			return nil
		}
		return fmt.Errorf("failed to create table with query: %s, error: %w", query, err)
	}
	result.Close()
	return nil
}

// =================================================================================
// VectorStorage Interface Implementation
// =================================================================================

func (s *LadybugDBStorage) SaveData(ctx context.Context, data *storage.Data) error {
	if err := s.checkContext(ctx); err != nil {
		return err
	}
	// LadybugDBで日時を扱う場合はISO8601形式の文字列を使用
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
	conn := s.getConn(ctx)
	if conn == s.conn {
		s.mu.Lock()
		defer s.mu.Unlock()
	}

	result, err := conn.Query(query)
	if err != nil {
		return fmt.Errorf("Failed to save data: %w", err)
	}
	result.Close()
	return nil
}

func (s *LadybugDBStorage) Exists(ctx context.Context, contentHash string, memoryGroup string) bool {
	if err := s.checkContext(ctx); err != nil {
		return false
	}
	query := fmt.Sprintf(`
		MATCH (d:Data)
		WHERE d.content_hash = '%s' AND d.memory_group = '%s'
		RETURN count(d)
	`, escapeString(contentHash), escapeString(memoryGroup))
	result, err := s.getConn(ctx).Query(query)
	if err != nil {
		utils.LogWarn(s.Logger, "LadybugDB: Exists query failed", zap.Error(err))
		return false
	}
	defer result.Close()
	if result.HasNext() {
		row, err := result.Next()
		if err != nil {
			return false
		}
		defer row.Close()
		cntV, _ := row.GetValue(0)
		cnt := getInt64(cntV)
		return cnt > 0
	}
	return false
}

func (s *LadybugDBStorage) GetDataByID(ctx context.Context, id string, memoryGroup string) (*storage.Data, error) {
	query := fmt.Sprintf(`
		MATCH (d:Data)
		WHERE d.id = '%s' AND d.memory_group = '%s'
		RETURN d.id, d.memory_group, d.name, d.raw_data_location, d.original_data_location, d.extension, d.mime_type, d.content_hash, d.owner_id, d.created_at
	`, escapeString(id), escapeString(memoryGroup))
	result, err := s.getConn(ctx).Query(query)
	if err != nil {
		return nil, fmt.Errorf("Failed to get data by id: %w", err)
	}
	defer result.Close()
	if result.HasNext() {
		row, err := result.Next()
		if err != nil {
			return nil, fmt.Errorf("Failed to get next row: %w", err)
		}
		defer row.Close()
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

func (s *LadybugDBStorage) GetDataList(ctx context.Context, memoryGroup string) ([]*storage.Data, error) {
	query := fmt.Sprintf(`
		MATCH (d:Data)
		WHERE d.memory_group = '%s'
		RETURN d.id, d.memory_group, d.name, d.raw_data_location, d.original_data_location, d.extension, d.mime_type, d.content_hash, d.owner_id, d.created_at
	`, escapeString(memoryGroup))
	result, err := s.getConn(ctx).Query(query)
	if err != nil {
		return nil, fmt.Errorf("Failed to get data list: %w", err)
	}
	defer result.Close()
	var dataList []*storage.Data
	for result.HasNext() {
		row, err := result.Next()
		if err != nil {
			return nil, fmt.Errorf("Failed to get next row: %w", err)
		}

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
		row.Close() // 手動でClose
	}
	return dataList, nil
}

func (s *LadybugDBStorage) GetDocumentByID(ctx context.Context, id string, memoryGroup string) (*storage.Document, error) {
	query := fmt.Sprintf(`
		MATCH (d:%s)
		WHERE d.id = '%s' AND d.memory_group = '%s'
		RETURN d.id, d.memory_group, d.data_id, d.text, d.metadata
	`, types.TABLE_NAME_DOCUMENT, escapeString(id), escapeString(memoryGroup))
	result, err := s.getConn(ctx).Query(query)
	if err != nil {
		return nil, fmt.Errorf("Failed to get document by id: %w", err)
	}
	defer result.Close()
	if result.HasNext() {
		row, err := result.Next()
		if err != nil {
			return nil, fmt.Errorf("Failed to get next row: %w", err)
		}
		defer row.Close()
		doc := &storage.Document{}
		if v, _ := row.GetValue(0); v != nil {
			doc.ID = getString(v)
		}
		if v, _ := row.GetValue(1); v != nil {
			doc.MemoryGroup = getString(v)
		}
		if v, _ := row.GetValue(2); v != nil {
			doc.DataID = getString(v)
		}
		if v, _ := row.GetValue(3); v != nil {
			doc.Text = getString(v)
		}
		if v, _ := row.GetValue(4); v != nil {
			metaStr := getString(v)
			json.Unmarshal([]byte(metaStr), &doc.MetaData)
		}
		return doc, nil
	}
	return nil, fmt.Errorf("document not found")
}

func (s *LadybugDBStorage) SaveDocument(ctx context.Context, document *storage.Document) error {
	if err := s.checkContext(ctx); err != nil {
		return err
	}
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
	conn := s.getConn(ctx)
	if conn == s.conn {
		s.mu.Lock()
		defer s.mu.Unlock()
	}

	if result, err := conn.Query(queryDoc); err != nil {
		return fmt.Errorf("Failed to save document node: %w", err)
	} else {
		result.Close()
	}
	// 2. Data -> Document リレーションシップ (HAS_DOCUMENT)
	// リレーションのMERGEはLadybugDBのバージョンによってはサポート外の場合があるが、
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
	if result, err := conn.Query(queryRel); err != nil {
		// Dataノードが見つからない場合など
		return fmt.Errorf("Failed to create HAS_DOCUMENT relation: %w", err)
	} else {
		result.Close()
	}
	return nil
}

func (s *LadybugDBStorage) SaveChunk(ctx context.Context, chunk *storage.Chunk) error {
	if err := s.checkContext(ctx); err != nil {
		return err
	}
	// 1. Chunkノード作成 (MERGE)
	// Embeddingがあれば保存 (FLOAT[1536])
	embeddingStr := "NULL"
	if len(chunk.Embedding) > 0 {
		embeddingStr = formatVectorForLadybugDB(chunk.Embedding)
	}
	queryChunk := fmt.Sprintf(`
		MERGE (c:%s {id: '%s', memory_group: '%s'})
		ON CREATE SET
			c.document_id = '%s',
			c.text = '%s',
			c.keywords = '%s',
			c.nouns = '%s',
			c.nouns_verbs = '%s',
			c.token_count = %d,
			c.chunk_index = %d,
			c.embedding = %s
		ON MATCH SET
			c.document_id = '%s',
			c.text = '%s',
			c.keywords = '%s',
			c.nouns = '%s',
			c.nouns_verbs = '%s',
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
		escapeString(chunk.Keywords),
		escapeString(chunk.Nouns),
		escapeString(chunk.NounsVerbs),
		chunk.TokenCount,
		chunk.ChunkIndex,
		embeddingStr,
		// ON MATCH
		escapeString(chunk.DocumentID),
		escapeString(chunk.Text),
		escapeString(chunk.Keywords),
		escapeString(chunk.Nouns),
		escapeString(chunk.NounsVerbs),
		chunk.TokenCount,
		chunk.ChunkIndex,
		embeddingStr,
	)
	conn := s.getConn(ctx)
	if conn == s.conn {
		s.mu.Lock()
		defer s.mu.Unlock()
	}

	if result, err := conn.Query(queryChunk); err != nil {
		return fmt.Errorf("Failed to save chunk node: %w", err)
	} else {
		result.Close()
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
	if result, err := conn.Query(queryRel); err != nil {
		return fmt.Errorf("Failed to create HAS_CHUNK relation: %w", err)
	} else {
		result.Close()
	}
	return nil
}

func (s *LadybugDBStorage) SaveEmbedding(ctx context.Context, tableName types.TableName, id string, text string, vector []float32, memoryGroup string) error {
	if len(vector) == 0 {
		return nil
	}
	vecStr := formatVectorForLadybugDB(vector)
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
	conn := s.getConn(ctx)
	if conn == s.conn {
		s.mu.Lock()
		defer s.mu.Unlock()
	}

	if result, err := conn.Query(query); err != nil {
		return fmt.Errorf("Failed to save embedding: %w", err)
	} else {
		result.Close()
	}
	return nil
}

func (s *LadybugDBStorage) Query(ctx context.Context, tableName types.TableName, vector []float32, topk int, memoryGroup string) ([]*storage.QueryResult, error) {
	if len(vector) == 0 {
		return nil, fmt.Errorf("Query vector is empty.")
	}
	vecStr := formatVectorForLadybugDB(vector)
	// array_cosine_similarity 関数を使って類似度計算
	// LadybugDBのバージョンによっては cosine_similarity または array_cosine_similarity
	query := fmt.Sprintf(`
		MATCH (c:%s)
		WHERE c.memory_group = '%s' AND c.embedding IS NOT NULL
		RETURN c.id, c.text, array_cosine_similarity(c.embedding, %s) AS score
		ORDER BY score DESC
		LIMIT %d
	`, tableName, escapeString(memoryGroup), vecStr, topk)
	result, err := s.getConn(ctx).Query(query)
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
		row.Close()
	}
	return results, nil
}

// FullTextSearch は、BM25 ベースの全文検索を実行します。
// 検索クエリを形態素解析し、指定されたレイヤーの FTS インデックスを使用して検索します。
// LadybugDB の FTS 拡張 (query_fts_index) を使用し、スコア順に結果を返します。
func (s *LadybugDBStorage) FullTextSearch(ctx context.Context, tableName types.TableName, queryText string, topk int, memoryGroup string, isEn bool, layer types.FtsLayer) ([]*storage.QueryResult, error) {
	// 1. 検索クエリ自体を形態素解析して、検索語を正規化・抽出
	kwRes := utils.ExtractKeywords(s.kagome, queryText, isEn)

	var searchQuery string
	var indexName string
	switch layer {
	case types.FTS_LAYER_NOUNS:
		searchQuery = kwRes.Nouns
		indexName = "chunk_nouns_idx"
	case types.FTS_LAYER_NOUNS_VERBS:
		searchQuery = kwRes.NounsVerbs
		indexName = "chunk_nouns_verbs_idx"
	default:
		searchQuery = kwRes.AllContentWords
		indexName = "chunk_keywords_idx"
	}

	if searchQuery == "" {
		return nil, nil // 検索語がない場合は空
	}

	// 2. query_fts_index を使用して BM25 ベースの全文検索を実行
	// 構文: CALL query_fts_index('テーブル名', 'インデックス名', '検索クエリ') RETURN node, score
	// memory_group による厳密な絞り込みをクエリに含める
	ftsQuery := fmt.Sprintf(`
		CALL query_fts_index('%s', '%s', '%s')
		WITH node, score
		WHERE node.memory_group = '%s'
		RETURN node.id, node.nouns, node.nouns_verbs, score
		ORDER BY score DESC
		LIMIT %d
	`, tableName, indexName, escapeString(searchQuery), escapeString(memoryGroup), topk)

	result, err := s.getConn(ctx).Query(ftsQuery)
	if err != nil {
		return nil, fmt.Errorf("FTS query failed: %w", err)
	}
	defer result.Close()

	// 3. 結果の収集
	var results []*storage.QueryResult
	for result.HasNext() {
		row, err := result.Next()
		if err != nil {
			return nil, fmt.Errorf("FTS query next failed: %w", err)
		}

		idVal, _ := row.GetValue(0)
		nounsVal, _ := row.GetValue(1)
		nounsVerbsVal, _ := row.GetValue(2)
		scoreVal, _ := row.GetValue(3)

		if idVal == nil || nounsVal == nil || nounsVerbsVal == nil || scoreVal == nil {
			row.Close()
			continue
		}

		res := &storage.QueryResult{}
		res.ID = getString(idVal)
		res.Nouns = getString(nounsVal)
		res.NounsVerbs = getString(nounsVerbsVal)
		// BM25 スコアを Distance として使用（高いほど関連性が高い）
		res.Distance = getFloat64(scoreVal)
		results = append(results, res)
		row.Close()
	}
	return results, nil
}

func (s *LadybugDBStorage) GetEmbeddingByID(ctx context.Context, tableName types.TableName, id string, memoryGroup string) ([]float32, error) {
	// Chunkテーブルから取得
	query := fmt.Sprintf(`
		MATCH (c:%s)
		WHERE c.id = '%s' AND c.memory_group = '%s'
		RETURN c.embedding
	`, tableName, escapeString(id), escapeString(memoryGroup))
	result, err := s.getConn(ctx).Query(query)
	if err != nil {
		return nil, fmt.Errorf("failed to get embedding: %w", err)
	}
	defer result.Close()
	if result.HasNext() {
		row, err := result.Next()
		if err != nil {
			return nil, err
		}
		defer row.Close()
		if v, _ := row.GetValue(0); v != nil {
			return parseEmbedding(v), nil
		}
	}
	return nil, nil // Not found
}

func (s *LadybugDBStorage) GetEmbeddingsByIDs(ctx context.Context, tableName types.TableName, ids []string, memoryGroup string) (map[string][]float32, error) {
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
	// LadybugDBのIN句またはlist_containsを使用
	// MATCH (c:Chunk) WHERE c.id IN [...]
	query := fmt.Sprintf(`
		MATCH (c:%s)
		WHERE c.memory_group = '%s' AND c.id IN %s
		RETURN c.id, c.embedding
	`, tableName, escapeString(memoryGroup), idListStr.String())
	result, err := s.getConn(ctx).Query(query)
	if err != nil {
		return nil, fmt.Errorf("Failed to get embeddings list: %w", err)
	}
	defer result.Close()
	embeddings := make(map[string][]float32)
	for result.HasNext() {
		row, err := result.Next()
		if err != nil {
			return nil, err
		}

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
		row.Close()
	}
	return embeddings, nil
}

// =================================================================================
// GraphStorage Interface Implementation
// =================================================================================

func (s *LadybugDBStorage) AddNodes(ctx context.Context, nodes []*storage.Node) error {
	if len(nodes) == 0 {
		return nil
	}
	conn := s.getConn(ctx)
	if conn == s.conn {
		s.mu.Lock()
		defer s.mu.Unlock()
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
		if result, err := conn.Query(query); err != nil {
			return fmt.Errorf("Failed to add node %s: %w", node.ID, err)
		} else {
			result.Close()
		}
	}
	return nil
}

func (s *LadybugDBStorage) AddEdges(ctx context.Context, edges []*storage.Edge) error {
	if len(edges) == 0 {
		return nil
	}
	conn := s.getConn(ctx)
	if conn == s.conn {
		s.mu.Lock()
		defer s.mu.Unlock()
	}

	for _, edge := range edges {
		propsJSON, _ := json.Marshal(edge.Properties)
		propsStr := string(propsJSON)
		// 両端のノードが存在する必要があります
		// MERGEを使ってエッジをUPSERTします
		// エッジの識別は (Source, Target, Type) で行うのが一般的ですが、
		// ここでは GraphEdge という単一のリレーションタイプを使用し、typeプロパティで区別します。
		query := fmt.Sprintf(`
			MATCH (a:%s {id: '%s', memory_group: '%s'}), (b:%s {id: '%s', memory_group: '%s'})
			MERGE (a)-[r:%s {memory_group: '%s', type: '%s'}]->(b)
			ON CREATE SET 
				r.properties = '%s',
				r.weight = %f,
				r.confidence = %f,
				r.unix = %d
			ON MATCH SET 
				r.properties = '%s',
				r.weight = %f,
				r.confidence = %f,
				r.unix = %d
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
			edge.Unix,
			// ON MATCH
			escapeString(propsStr),
			edge.Weight,
			edge.Confidence,
			edge.Unix,
		)
		if result, err := conn.Query(query); err != nil {
			return fmt.Errorf("Failed to add edge %s->%s: %w", edge.SourceID, edge.TargetID, err)
		} else {
			result.Close()
		}
	}
	return nil
}

func (s *LadybugDBStorage) GetTriples(ctx context.Context, nodeIDs []string, memoryGroup string) ([]*storage.Triple, error) {
	if len(nodeIDs) == 0 {
		return nil, nil
	}
	// IDリスト作成 (reconstruct full IDs with memory group suffix)
	var idListStr strings.Builder
	idListStr.WriteString("[")
	for i, id := range nodeIDs {
		if i > 0 {
			idListStr.WriteString(", ")
		}
		fullID := utils.EnsureFullGraphNodeID(id, memoryGroup)
		idListStr.WriteString(fmt.Sprintf("'%s'", escapeString(fullID)))
	}
	idListStr.WriteString("]")
	// 指定されたノードID群に関連する(SourceまたはTargetとなる)エッジとその両端ノードを取得
	query := fmt.Sprintf(`
		MATCH (a:%s {memory_group: '%s'})-[r:%s {memory_group: '%s'}]->(b:%s {memory_group: '%s'})
		WHERE a.id IN %s OR b.id IN %s
		RETURN 
			a.id, a.type, a.properties, 
			r.type, r.properties, r.weight, r.confidence, r.unix,
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
	result, err := s.getConn(ctx).Query(query)
	if err != nil {
		return nil, fmt.Errorf("failed to get triples: %w", err)
	}
	defer result.Close()
	var triples []*storage.Triple
	for result.HasNext() {
		row, err := result.Next()
		if err != nil {
			return nil, err
		}

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
		edge.SourceID = nodeA.ID
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
		if v, _ := row.GetValue(7); v != nil {
			edge.Unix = getInt64(v)
		}
		// Parse Node B
		nodeB := &storage.Node{MemoryGroup: memoryGroup}
		if v, _ := row.GetValue(8); v != nil {
			tmpID := getString(v)
			nodeB.ID = utils.GetNameStrByGraphNodeID(tmpID)
		}
		if v, _ := row.GetValue(9); v != nil {
			nodeB.Type = getString(v)
		}
		if v, _ := row.GetValue(10); v != nil {
			nodeB.Properties = parseJSONProperties(getString(v))
		}
		edge.TargetID = nodeB.ID
		triples = append(triples, &storage.Triple{
			Source: nodeA,
			Edge:   edge,
			Target: nodeB,
		})
		row.Close()
	}
	return triples, nil
}

// GetSourceNodeIDs は、エッジの発信元となるノードIDを重複なしでページング取得します。
// Cypherの SKIP 句を使用してオフセットベースのページングを実現します。
func (s *LadybugDBStorage) GetSourceNodeIDs(ctx context.Context, memoryGroup string, offset, limit int) ([]string, error) {
	query := fmt.Sprintf(`
		MATCH (a:%s {memory_group: '%s'})-[r:%s {memory_group: '%s'}]->()
		RETURN DISTINCT a.id
		ORDER BY a.id
		SKIP %d
		LIMIT %d
	`,
		types.TABLE_NAME_GRAPH_NODE,
		escapeString(memoryGroup),
		types.TABLE_NAME_GRAPH_EDGE,
		escapeString(memoryGroup),
		offset,
		limit,
	)
	result, err := s.getConn(ctx).Query(query)
	if err != nil {
		return nil, fmt.Errorf("failed to get source node IDs: %w", err)
	}
	defer result.Close()

	var nodeIDs []string
	for result.HasNext() {
		row, err := result.Next()
		if err != nil {
			return nil, err
		}
		if v, _ := row.GetValue(0); v != nil {
			tmpID := getString(v)
			nodeIDs = append(nodeIDs, utils.GetNameStrByGraphNodeID(tmpID))
		}
		row.Close()
	}
	return nodeIDs, nil
}

// GetTriplesBySourceIDs は、指定されたSourceノードIDに関連するトリプルを取得します。
func (s *LadybugDBStorage) GetTriplesBySourceIDs(ctx context.Context, sourceIDs []string, memoryGroup string) ([]*storage.Triple, error) {
	if len(sourceIDs) == 0 {
		return nil, nil
	}
	// IDリスト作成 (reconstruct full IDs with memory group suffix)
	var idListStr strings.Builder
	idListStr.WriteString("[")
	for i, id := range sourceIDs {
		if i > 0 {
			idListStr.WriteString(", ")
		}
		fullID := utils.EnsureFullGraphNodeID(id, memoryGroup)
		idListStr.WriteString(fmt.Sprintf("'%s'", escapeString(fullID)))
	}
	idListStr.WriteString("]")
	// Source IDのみでフィルタリング
	query := fmt.Sprintf(`
		MATCH (a:%s {memory_group: '%s'})-[r:%s {memory_group: '%s'}]->(b:%s {memory_group: '%s'})
		WHERE a.id IN %s
		RETURN 
			a.id, a.type, a.properties, 
			r.type, r.properties, r.weight, r.confidence, r.unix,
			b.id, b.type, b.properties
	`,
		types.TABLE_NAME_GRAPH_NODE,
		escapeString(memoryGroup),
		types.TABLE_NAME_GRAPH_EDGE,
		escapeString(memoryGroup),
		types.TABLE_NAME_GRAPH_NODE,
		escapeString(memoryGroup),
		idListStr.String(),
	)
	result, err := s.getConn(ctx).Query(query)
	if err != nil {
		return nil, fmt.Errorf("failed to get triples by source IDs: %w", err)
	}
	defer result.Close()
	var triples []*storage.Triple
	for result.HasNext() {
		row, err := result.Next()
		if err != nil {
			return nil, err
		}

		// Parse Node A
		nodeA := &storage.Node{MemoryGroup: memoryGroup}
		if v, _ := row.GetValue(0); v != nil {
			tmpID := getString(v)
			nodeA.ID = utils.GetNameStrByGraphNodeID(tmpID)
		}
		if v, _ := row.GetValue(1); v != nil {
			nodeA.Type = getString(v)
		}
		if v, _ := row.GetValue(2); v != nil {
			nodeA.Properties = parseJSONProperties(getString(v))
		}
		// Parse Edge
		edge := &storage.Edge{MemoryGroup: memoryGroup}
		edge.SourceID = nodeA.ID
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
		if v, _ := row.GetValue(7); v != nil {
			edge.Unix = getInt64(v)
		}
		// Parse Node B
		nodeB := &storage.Node{MemoryGroup: memoryGroup}
		if v, _ := row.GetValue(8); v != nil {
			tmpID := getString(v)
			nodeB.ID = utils.GetNameStrByGraphNodeID(tmpID)
		}
		if v, _ := row.GetValue(9); v != nil {
			nodeB.Type = getString(v)
		}
		if v, _ := row.GetValue(10); v != nil {
			nodeB.Properties = parseJSONProperties(getString(v))
		}
		edge.TargetID = nodeB.ID
		triples = append(triples, &storage.Triple{
			Source: nodeA,
			Edge:   edge,
			Target: nodeB,
		})
		row.Close()
	}
	return triples, nil
}

// GetOrphanNodes は、エッジを持たない孤立ノードを取得します。
func (s *LadybugDBStorage) GetOrphanNodes(ctx context.Context, memoryGroup string, gracePeriod time.Duration) ([]*storage.Node, error) {
	// 現在時刻から猶予期間を引き、それ以前に作成されたノードのみを対象とする
	cutoffTime := time.Now().Add(-gracePeriod).Format(time.RFC3339)

	// InDegree=0 かつ OutDegree=0 のノードを検索
	// 注意: LadybugDBのクエリ文法に合わせて調整
	query := fmt.Sprintf(`
		MATCH (n:GraphNode)
		WHERE n.memory_group = '%s' 
		  AND n.properties CONTAINS '"created_at":"'
		  AND timestamp(parse_json_get(n.properties, 'created_at')) < timestamp('%s')
		  AND NOT EXISTS { MATCH (n)-[]->() }
		  AND NOT EXISTS { MATCH ()-[]->(n) }
		RETURN n.id, n.type, n.properties
	`, escapeString(memoryGroup), cutoffTime)

	result, err := s.getConn(ctx).Query(query)
	if err != nil {
		return nil, fmt.Errorf("GetOrphanNodes query failed: %w", err)
	}
	defer result.Close()

	var nodes []*storage.Node
	for result.HasNext() {
		row, err := result.Next()
		if err != nil {
			return nil, err
		}
		n := &storage.Node{MemoryGroup: memoryGroup}
		if v, _ := row.GetValue(0); v != nil {
			n.ID = getString(v)
		}
		if v, _ := row.GetValue(1); v != nil {
			n.Type = getString(v)
		}
		if v, _ := row.GetValue(2); v != nil {
			n.Properties = parseJSONProperties(getString(v))
		}
		nodes = append(nodes, n)
		row.Close()
	}
	return nodes, nil
}

// GetWeaklyConnectedNodes は、全エッジが弱い（Thickness ≤ threshold）ノードを取得します。
// MDL Principle に基づくノード削除の候補を抽出するために使用されます。
func (s *LadybugDBStorage) GetWeaklyConnectedNodes(
	ctx context.Context,
	memoryGroup string,
	thicknessThreshold float64,
	gracePeriod time.Duration,
) ([]*storage.Node, error) {
	// 猶予期間を計算
	cutoffTime := time.Now().Add(-gracePeriod).Format(time.RFC3339)

	// Cypher クエリ：
	// 1. 指定 memoryGroup のノードを全て取得
	// 2. そのノードに接続する全エッジの Thickness (weight × confidence) の最大値を計算
	// 3. 全エッジの Thickness が閾値以下であり、かつエッジが1本以上存在するノードのみを返す
	// 4. 猶予期間内に作成されたノードは除外
	query := fmt.Sprintf(`
		MATCH (n:%s {memory_group: '%s'})
		WHERE n.properties CONTAINS '"created_at":"'
		  AND timestamp(parse_json_get(n.properties, 'created_at')) < timestamp('%s')
		WITH n
		OPTIONAL MATCH (n)-[r:%s {memory_group: '%s'}]-()
		WITH n,
		     count(r) AS edgeCount,
		     max(CASE WHEN r IS NOT NULL THEN r.weight * r.confidence ELSE 0.0 END) AS maxThickness
		WHERE edgeCount > 0 AND maxThickness <= %f
		RETURN n.id, n.type, n.properties
	`,
		types.TABLE_NAME_GRAPH_NODE,
		escapeString(memoryGroup),
		cutoffTime,
		types.TABLE_NAME_GRAPH_EDGE,
		escapeString(memoryGroup),
		thicknessThreshold,
	)

	result, err := s.getConn(ctx).Query(query)
	if err != nil {
		return nil, fmt.Errorf("GetWeaklyConnectedNodes query failed: %w", err)
	}
	defer result.Close()

	var nodes []*storage.Node
	for result.HasNext() {
		row, err := result.Next()
		if err != nil {
			return nil, err
		}

		node := &storage.Node{MemoryGroup: memoryGroup}
		if v, _ := row.GetValue(0); v != nil {
			// GetOrphanNodes と同様、連結済みIDをそのまま返す（DeleteNode が期待するフォーマット）
			node.ID = getString(v)
		}
		if v, _ := row.GetValue(1); v != nil {
			node.Type = getString(v)
		}
		if v, _ := row.GetValue(2); v != nil {
			node.Properties = parseJSONProperties(getString(v))
		}
		nodes = append(nodes, node)
		row.Close()
	}

	return nodes, nil
}

func (s *LadybugDBStorage) StreamDocumentChunks(ctx context.Context, memoryGroup string) (<-chan *storage.ChunkData, <-chan error) {
	outCh := make(chan *storage.ChunkData)
	errCh := make(chan error, 1)

	go func() {
		defer close(outCh)
		defer close(errCh)

		query := fmt.Sprintf(`
			MATCH (doc:%s {memory_group: '%s'})-[:HAS_CHUNK]->(c:%s)
			RETURN doc.id, doc.text, doc.metadata, c.id, c.text, c.token_count, c.chunk_index
		`, types.TABLE_NAME_DOCUMENT, escapeString(memoryGroup), types.TABLE_NAME_CHUNK)

		result, err := s.getConn(ctx).Query(query)
		if err != nil {
			errCh <- fmt.Errorf("StreamDocumentChunks query failed: %w", err)
			return
		}
		defer result.Close()

		for result.HasNext() {
			row, err := result.Next()
			if err != nil {
				errCh <- fmt.Errorf("StreamDocumentChunks next failed: %w", err)
				return
			}

			chunkData := &storage.ChunkData{
				MemoryGroup: memoryGroup,
			}
			// Doc
			if v, _ := row.GetValue(0); v != nil {
				chunkData.DocumentID = getString(v)
			} // doc.id -> DocumentID? No, wait.
			// Chunk
			if v, _ := row.GetValue(3); v != nil {
				chunkData.ID = getString(v)
			}
			if v, _ := row.GetValue(4); v != nil {
				chunkData.Text = getString(v)
			}
			// If we need the document text/metadata, we might need to adjust ChunkData struct or handle separately.
			// Looking at storage/interfaces.go:L186-191:
			// type ChunkData struct {
			// 	ID          string // チャンクID
			// 	Text        string // チャンクのテキスト内容
			// 	MemoryGroup string // メモリーグループ
			// 	DocumentID  string // 親ドキュメントID
			// }

			if v, _ := row.GetValue(0); v != nil {
				chunkData.DocumentID = getString(v)
			}

			select {
			case <-ctx.Done():
				row.Close()
				return
			case outCh <- chunkData:
			}
			row.Close()
		}
	}()

	return outCh, errCh
}

func (s *LadybugDBStorage) GetDocumentChunkCount(ctx context.Context, memoryGroup string) (int, error) {
	query := fmt.Sprintf(`
		MATCH (c:%s {memory_group: '%s'})
		RETURN count(c)
	`, types.TABLE_NAME_CHUNK, escapeString(memoryGroup))
	result, err := s.getConn(ctx).Query(query)
	if err != nil {
		return 0, fmt.Errorf("GetDocumentChunkCount query failed: %w", err)
	}
	defer result.Close()
	if result.HasNext() {
		row, err := result.Next()
		if err != nil {
			return 0, err
		}
		defer row.Close()
		v, _ := row.GetValue(0)
		return int(getInt64(v)), nil
	}
	return 0, nil
}

func (s *LadybugDBStorage) GetNodesByType(ctx context.Context, nodeType string, memoryGroup string) ([]*storage.Node, error) {
	query := fmt.Sprintf(`
		MATCH (n:%s {memory_group: '%s', type: '%s'})
		RETURN n.id, n.type, n.properties
	`, types.TABLE_NAME_GRAPH_NODE, escapeString(memoryGroup), escapeString(nodeType))
	result, err := s.getConn(ctx).Query(query)
	if err != nil {
		return nil, fmt.Errorf("GetNodesByType query failed: %w", err)
	}
	defer result.Close()
	var nodes []*storage.Node
	for result.HasNext() {
		row, err := result.Next()
		if err != nil {
			return nil, err
		}
		n := &storage.Node{MemoryGroup: memoryGroup}
		if v, _ := row.GetValue(0); v != nil {
			n.ID = getString(v)
		}
		if v, _ := row.GetValue(1); v != nil {
			n.Type = getString(v)
		}
		if v, _ := row.GetValue(2); v != nil {
			n.Properties = parseJSONProperties(getString(v))
		}
		nodes = append(nodes, n)
		row.Close()
	}
	return nodes, nil
}

func (s *LadybugDBStorage) GetNodesByEdge(ctx context.Context, targetID string, edgeType string, memoryGroup string) ([]*storage.Node, error) {
	query := fmt.Sprintf(`
		MATCH (a:%s {memory_group: '%s'})-[:%s {memory_group: '%s', type: '%s'}]->(b:%s {id: '%s', memory_group: '%s'})
		RETURN a.id, a.type, a.properties
	`, types.TABLE_NAME_GRAPH_NODE, escapeString(memoryGroup),
		types.TABLE_NAME_GRAPH_EDGE, escapeString(memoryGroup), escapeString(edgeType),
		types.TABLE_NAME_GRAPH_NODE, escapeString(targetID), escapeString(memoryGroup))
	result, err := s.getConn(ctx).Query(query)
	if err != nil {
		return nil, fmt.Errorf("GetNodesByEdge query failed: %w", err)
	}
	defer result.Close()
	var nodes []*storage.Node
	for result.HasNext() {
		row, err := result.Next()
		if err != nil {
			return nil, err
		}
		n := &storage.Node{MemoryGroup: memoryGroup}
		if v, _ := row.GetValue(0); v != nil {
			n.ID = getString(v)
		}
		if v, _ := row.GetValue(1); v != nil {
			n.Type = getString(v)
		}
		if v, _ := row.GetValue(2); v != nil {
			n.Properties = parseJSONProperties(getString(v))
		}
		nodes = append(nodes, n)
		row.Close()
	}
	return nodes, nil
}

func (s *LadybugDBStorage) UpdateEdgeWeight(ctx context.Context, sourceID, targetID, memoryGroup string, weight float64) error {
	// Reconstruct full IDs with memory group suffix
	fullSourceID := utils.EnsureFullGraphNodeID(sourceID, memoryGroup)
	fullTargetID := utils.EnsureFullGraphNodeID(targetID, memoryGroup)
	query := fmt.Sprintf(`
		MATCH (a:%s {id: '%s', memory_group: '%s'})-[r:%s {memory_group: '%s'}]->(b:%s {id: '%s', memory_group: '%s'})
		SET r.weight = %f
	`, types.TABLE_NAME_GRAPH_NODE, escapeString(fullSourceID), escapeString(memoryGroup),
		types.TABLE_NAME_GRAPH_EDGE, escapeString(memoryGroup),
		types.TABLE_NAME_GRAPH_NODE, escapeString(fullTargetID), escapeString(memoryGroup),
		weight)
	conn := s.getConn(ctx)
	if conn == s.conn {
		s.mu.Lock()
		defer s.mu.Unlock()
	}

	if result, err := conn.Query(query); err != nil {
		return fmt.Errorf("UpdateEdgeWeight failed: %w", err)
	} else {
		result.Close()
	}
	return nil
}

func (s *LadybugDBStorage) UpdateEdgeMetrics(ctx context.Context, sourceID, targetID, memoryGroup string, weight, confidence float64, unix int64) error {
	// Reconstruct full IDs with memory group suffix
	fullSourceID := utils.EnsureFullGraphNodeID(sourceID, memoryGroup)
	fullTargetID := utils.EnsureFullGraphNodeID(targetID, memoryGroup)
	query := fmt.Sprintf(`
		MATCH (a:%s {id: '%s', memory_group: '%s'})-[r:%s {memory_group: '%s'}]->(b:%s {id: '%s', memory_group: '%s'})
		SET r.weight = %f, r.confidence = %f, r.unix = %d
	`, types.TABLE_NAME_GRAPH_NODE, escapeString(fullSourceID), escapeString(memoryGroup),
		types.TABLE_NAME_GRAPH_EDGE, escapeString(memoryGroup),
		types.TABLE_NAME_GRAPH_NODE, escapeString(fullTargetID), escapeString(memoryGroup),
		weight, confidence, unix)
	conn := s.getConn(ctx)
	if conn == s.conn {
		s.mu.Lock()
		defer s.mu.Unlock()
	}

	if result, err := conn.Query(query); err != nil {
		return fmt.Errorf("UpdateEdgeMetrics failed: %w", err)
	} else {
		result.Close()
	}
	return nil
}

func (s *LadybugDBStorage) DeleteEdge(ctx context.Context, sourceID, edgeType, targetID, memoryGroup string) error {
	// Reconstruct full IDs with memory group suffix
	fullSourceID := utils.EnsureFullGraphNodeID(sourceID, memoryGroup)
	fullTargetID := utils.EnsureFullGraphNodeID(targetID, memoryGroup)
	query := fmt.Sprintf(`
		MATCH (a:%s {id: '%s', memory_group: '%s'})-[r:%s {type: '%s', memory_group: '%s'}]->(b:%s {id: '%s', memory_group: '%s'})
		DELETE r
	`, types.TABLE_NAME_GRAPH_NODE, escapeString(fullSourceID), escapeString(memoryGroup),
		types.TABLE_NAME_GRAPH_EDGE, escapeString(edgeType), escapeString(memoryGroup),
		types.TABLE_NAME_GRAPH_NODE, escapeString(fullTargetID), escapeString(memoryGroup))
	conn := s.getConn(ctx)
	if conn == s.conn {
		s.mu.Lock()
		defer s.mu.Unlock()
	}

	if result, err := conn.Query(query); err != nil {
		return fmt.Errorf("DeleteEdge failed: %w", err)
	} else {
		result.Close()
	}
	return nil
}

func (s *LadybugDBStorage) DeleteNode(ctx context.Context, nodeID, memoryGroup string) error {
	// Reconstruct full ID with memory group suffix
	fullNodeID := utils.EnsureFullGraphNodeID(nodeID, memoryGroup)
	query := fmt.Sprintf(`
		MATCH (n:%s {id: '%s', memory_group: '%s'})
		DETACH DELETE n
	`, types.TABLE_NAME_GRAPH_NODE, escapeString(fullNodeID), escapeString(memoryGroup))
	conn := s.getConn(ctx)
	if conn == s.conn {
		s.mu.Lock()
		defer s.mu.Unlock()
	}

	if result, err := conn.Query(query); err != nil {
		return fmt.Errorf("DeleteNode failed: %w", err)
	} else {
		result.Close()
	}
	return nil
}

func (s *LadybugDBStorage) GetEdgesByNode(ctx context.Context, nodeID string, memoryGroup string) ([]*storage.Edge, error) {
	// Reconstruct full ID with memory group suffix
	fullNodeID := utils.EnsureFullGraphNodeID(nodeID, memoryGroup)
	query := fmt.Sprintf(`
		MATCH (a:%s {id: '%s', memory_group: '%s'})-[r:%s {memory_group: '%s'}]->(b:%s {memory_group: '%s'})
		RETURN r.type, r.properties, r.weight, r.confidence, r.unix, b.id
	`, types.TABLE_NAME_GRAPH_NODE, escapeString(fullNodeID), escapeString(memoryGroup),
		types.TABLE_NAME_GRAPH_EDGE, escapeString(memoryGroup),
		types.TABLE_NAME_GRAPH_NODE, escapeString(memoryGroup))
	result, err := s.getConn(ctx).Query(query)
	if err != nil {
		return nil, fmt.Errorf("GetEdgesByNode query failed: %w", err)
	}
	defer result.Close()
	var edges []*storage.Edge
	for result.HasNext() {
		row, err := result.Next()
		if err != nil {
			return nil, err
		}
		e := &storage.Edge{SourceID: nodeID, MemoryGroup: memoryGroup}
		if v, _ := row.GetValue(0); v != nil {
			e.Type = getString(v)
		}
		if v, _ := row.GetValue(1); v != nil {
			e.Properties = parseJSONProperties(getString(v))
		}
		if v, _ := row.GetValue(2); v != nil {
			e.Weight = getFloat64(v)
		}
		if v, _ := row.GetValue(3); v != nil {
			e.Confidence = getFloat64(v)
		}
		if v, _ := row.GetValue(4); v != nil {
			e.Unix = getInt64(v)
		}
		if v, _ := row.GetValue(5); v != nil {
			e.TargetID = getString(v)
		}
		edges = append(edges, e)
		row.Close()
	}
	return edges, nil
}

// GetMaxUnix は、指定されたメモリーグループ内のエッジの最大Unixタイムスタンプを取得します。
func (s *LadybugDBStorage) GetMaxUnix(ctx context.Context, memoryGroup string) (int64, error) {
	query := fmt.Sprintf(`
		MATCH ()-[r:%s {memory_group: '%s'}]->()
		RETURN max(r.unix) AS max_unix
	`, types.TABLE_NAME_GRAPH_EDGE, escapeString(memoryGroup))
	result, err := s.getConn(ctx).Query(query)
	if err != nil {
		return 0, fmt.Errorf("GetMaxUnix query failed: %w", err)
	}
	defer result.Close()
	if result.HasNext() {
		row, err := result.Next()
		if err != nil {
			return 0, err
		}
		defer row.Close()
		if v, _ := row.GetValue(0); v != nil {
			return getInt64(v), nil
		}
	}
	return 0, nil // エッジがない場合は0を返す
}

// GetMemoryGroupConfig は、指定されたメモリーグループの設定を取得します。
func (s *LadybugDBStorage) GetMemoryGroupConfig(ctx context.Context, memoryGroup string) (*storage.MemoryGroupConfig, error) {
	query := fmt.Sprintf(`
		MATCH (mg:MemoryGroup {id: '%s'})
		RETURN mg.id, mg.half_life_days, mg.prune_threshold, mg.min_survival_protection_hours, mg.mdl_k_neighbors
	`, escapeString(memoryGroup))
	result, err := s.getConn(ctx).Query(query)
	if err != nil {
		return nil, fmt.Errorf("GetMemoryGroupConfig query failed: %w", err)
	}
	defer result.Close()
	if result.HasNext() {
		row, err := result.Next()
		if err != nil {
			return nil, err
		}
		defer row.Close()
		config := &storage.MemoryGroupConfig{}
		if v, _ := row.GetValue(0); v != nil {
			config.ID = getString(v)
		}
		if v, _ := row.GetValue(1); v != nil {
			config.HalfLifeDays = getFloat64(v)
		}
		if v, _ := row.GetValue(2); v != nil {
			config.PruneThreshold = getFloat64(v)
		}
		if v, _ := row.GetValue(3); v != nil {
			config.MinSurvivalProtectionHours = getFloat64(v)
		}
		if v, _ := row.GetValue(4); v != nil {
			config.MdlKNeighbors = int(getInt64(v))
		}
		return config, nil
	}
	return nil, nil // 存在しない場合はnilを返す
}

// UpsertMemoryGroup は、メモリーグループの設定を作成または更新します。
func (s *LadybugDBStorage) UpsertMemoryGroup(ctx context.Context, config *storage.MemoryGroupConfig) error {
	now := common.GetNow().Format(time.RFC3339)
	query := fmt.Sprintf(`
		MERGE (mg:MemoryGroup {id: '%s'})
		ON CREATE SET
			mg.half_life_days = %f,
			mg.prune_threshold = %f,
			mg.min_survival_protection_hours = %f,
			mg.mdl_k_neighbors = %d,
			mg.created_at = timestamp('%s'),
			mg.updated_at = timestamp('%s')
		ON MATCH SET
			mg.half_life_days = %f,
			mg.prune_threshold = %f,
			mg.min_survival_protection_hours = %f,
			mg.mdl_k_neighbors = %d,
			mg.updated_at = timestamp('%s')
	`,
		escapeString(config.ID),
		// ON CREATE
		config.HalfLifeDays,
		config.PruneThreshold,
		config.MinSurvivalProtectionHours,
		config.MdlKNeighbors,
		now,
		now,
		// ON MATCH
		config.HalfLifeDays,
		config.PruneThreshold,
		config.MinSurvivalProtectionHours,
		config.MdlKNeighbors,
		now,
	)
	conn := s.getConn(ctx)
	if conn == s.conn {
		s.mu.Lock()
		defer s.mu.Unlock()
	}

	if result, err := conn.Query(query); err != nil {
		return fmt.Errorf("UpsertMemoryGroup failed: %w", err)
	} else {
		result.Close()
	}
	return nil
}

// ---------------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------------

func escapeString(s string) string {
	return strings.ReplaceAll(s, "'", "''")
}

func formatVectorForLadybugDB(vector []float32) string {
	var sb strings.Builder
	sb.WriteString("[")
	for i, v := range vector {
		if i > 0 {
			sb.WriteString(",")
		}
		sb.WriteString(fmt.Sprintf("%f", v))
	}
	sb.WriteString("]")
	return sb.String()
}

func getString(v interface{}) string {
	if s, ok := v.(string); ok {
		return s
	}
	return fmt.Sprint(v)
}

func getInt64(v interface{}) int64 {
	switch val := v.(type) {
	case int64:
		return val
	case int:
		return int64(val)
	case float64:
		return int64(val)
	}
	return 0
}

func getFloat64(v interface{}) float64 {
	switch val := v.(type) {
	case float64:
		return val
	case float32:
		return float64(val)
	case int64:
		return float64(val)
	}
	return 0
}

func parseTimestamp(v interface{}) time.Time {
	if t, ok := v.(time.Time); ok {
		return t
	}
	if s, ok := v.(string); ok {
		t, _ := time.Parse(time.RFC3339, s)
		return t
	}
	return time.Time{}
}

func parseEmbedding(v interface{}) []float32 {
	if vec, ok := v.([]float32); ok {
		return vec
	}
	// go-ladybug returns vector as slice of interface or similar?
	// Handle appropriately based on actual type.
	return nil
}

func parseJSONProperties(s string) map[string]interface{} {
	var m map[string]interface{}
	json.Unmarshal([]byte(s), &m)
	return m
}
