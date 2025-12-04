package duckdb

import (
	"context"
	"database/sql"
	"encoding/json"
	"fmt" // For array support if needed, or manual parsing

	"github.com/t-kawata/mycute/pkg/cognee/storage"
)

type DuckDBStorage struct {
	db *sql.DB
}

func NewDuckDBStorage(db *sql.DB) *DuckDBStorage {
	return &DuckDBStorage{db: db}
}

// Ensure interface implementation
var _ storage.VectorStorage = (*DuckDBStorage)(nil)

func (s *DuckDBStorage) SaveData(ctx context.Context, data *storage.Data) error {
	query := `
		INSERT INTO data (id, name, raw_data_location, extension, content_hash, created_at)
		VALUES (?, ?, ?, ?, ?, ?)
		ON CONFLICT (id) DO UPDATE SET
			name = excluded.name,
			raw_data_location = excluded.raw_data_location,
			extension = excluded.extension,
			content_hash = excluded.content_hash,
			created_at = excluded.created_at
	`
	_, err := s.db.ExecContext(ctx, query, data.ID, data.Name, data.RawDataLocation, data.Extension, data.ContentHash, data.CreatedAt)
	return err
}

func (s *DuckDBStorage) Exists(ctx context.Context, contentHash string) bool {
	var count int
	query := `SELECT COUNT(*) FROM data WHERE content_hash = ?`
	err := s.db.QueryRowContext(ctx, query, contentHash).Scan(&count)
	if err != nil {
		return false
	}
	return count > 0
}

func (s *DuckDBStorage) GetDataByID(ctx context.Context, id string) (*storage.Data, error) {
	query := `SELECT id::VARCHAR, name, raw_data_location, extension, content_hash, created_at FROM data WHERE id = ?`
	row := s.db.QueryRowContext(ctx, query, id)

	var data storage.Data
	if err := row.Scan(&data.ID, &data.Name, &data.RawDataLocation, &data.Extension, &data.ContentHash, &data.CreatedAt); err != nil {
		return nil, err
	}
	return &data, nil
}

func (s *DuckDBStorage) GetDataList(ctx context.Context) ([]*storage.Data, error) {
	query := `SELECT id::VARCHAR, name, raw_data_location, extension, content_hash, created_at FROM data`
	rows, err := s.db.QueryContext(ctx, query)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var dataList []*storage.Data
	for rows.Next() {
		var data storage.Data
		if err := rows.Scan(&data.ID, &data.Name, &data.RawDataLocation, &data.Extension, &data.ContentHash, &data.CreatedAt); err != nil {
			return nil, err
		}
		dataList = append(dataList, &data)
	}
	return dataList, nil
}

func (s *DuckDBStorage) SaveDocument(ctx context.Context, doc *storage.Document) error {
	query := `
	INSERT INTO documents (id, data_id, text, metadata, created_at)
	VALUES (?, ?, ?, ?, current_timestamp)
	`
	// Metadata to JSON string
	metaJSON, err := json.Marshal(doc.MetaData)
	if err != nil {
		return fmt.Errorf("failed to marshal metadata: %w", err)
	}

	if _, err := s.db.ExecContext(ctx, query, doc.ID, doc.DataID, doc.Text, string(metaJSON)); err != nil {
		return fmt.Errorf("failed to save document: %w", err)
	}
	return nil
}

func (s *DuckDBStorage) SaveChunk(ctx context.Context, chunk *storage.Chunk) error {
	// 1. Save Chunk
	chunkQuery := `
		INSERT INTO chunks (id, document_id, text, chunk_index, token_count, created_at)
		VALUES (?, ?, ?, ?, ?, current_timestamp)
		ON CONFLICT (id) DO UPDATE SET
			text = excluded.text,
			chunk_index = excluded.chunk_index,
			token_count = excluded.token_count
	`
	_, err := s.db.ExecContext(ctx, chunkQuery, chunk.ID, chunk.DocumentID, chunk.Text, chunk.ChunkIndex, chunk.TokenCount)
	if err != nil {
		return fmt.Errorf("failed to save chunk: %w", err)
	}

	// 2. Save Vector (if embedding exists)
	if len(chunk.Embedding) > 0 {
		// Convert embedding to array string or use driver support
		// DuckDB Go driver supports []float32 for ARRAY type
		vectorQuery := `
			INSERT INTO vectors (id, collection_name, text, embedding)
			VALUES (?, ?, ?, ?)
			ON CONFLICT (id, collection_name) DO UPDATE SET
				text = excluded.text,
				embedding = excluded.embedding
		`
		// Assuming "DocumentChunk_text" as default collection for chunks
		collectionName := "DocumentChunk_text"
		_, err = s.db.ExecContext(ctx, vectorQuery, chunk.ID, collectionName, chunk.Text, chunk.Embedding)
		if err != nil {
			return fmt.Errorf("failed to save vector: %w", err)
		}
	}

	return nil
}

func (s *DuckDBStorage) Search(ctx context.Context, collectionName string, vector []float32, k int) ([]*storage.SearchResult, error) {
	// Using VSS extension syntax: array_cosine_similarity or similar
	// DuckDB VSS uses specific syntax. Assuming HNSW index usage.
	// Query: SELECT id, text, array_cosine_similarity(embedding, ?) as score FROM vectors WHERE collection_name = ? ORDER BY score DESC LIMIT ?

	query := `
		SELECT id, text, array_cosine_similarity(embedding, ?::FLOAT[1536]) as score
		FROM vectors
		WHERE collection_name = ?
		ORDER BY score DESC
		LIMIT ?
	`

	rows, err := s.db.QueryContext(ctx, query, vector, collectionName, k)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var results []*storage.SearchResult
	for rows.Next() {
		var res storage.SearchResult
		if err := rows.Scan(&res.ID, &res.Text, &res.Distance); err != nil {
			return nil, err
		}
		results = append(results, &res)
	}
	return results, nil
}
