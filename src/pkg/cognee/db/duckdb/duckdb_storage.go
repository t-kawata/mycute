package duckdb

import (
	"context"
	"database/sql"
	"encoding/json"
	"fmt" // For array support if needed, or manual parsing

	"mycute/pkg/cognee/storage"
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
		INSERT INTO data (id, group_id, name, raw_data_location, extension, content_hash, created_at)
		VALUES (?, ?, ?, ?, ?, ?, ?)
		ON CONFLICT (group_id, id) DO UPDATE SET
			name = excluded.name,
			raw_data_location = excluded.raw_data_location,
			extension = excluded.extension,
			content_hash = excluded.content_hash,
			created_at = excluded.created_at
	`
	_, err := s.db.ExecContext(ctx, query, data.ID, data.GroupID, data.Name, data.RawDataLocation, data.Extension, data.ContentHash, data.CreatedAt)
	return err
}

// Exists checks if data with the given content hash exists within the specified group.
// group_id filtering is enforced for strict partitioning consistency,
// even though content_hash alone might be unique across the entire database.
func (s *DuckDBStorage) Exists(ctx context.Context, contentHash string, groupID string) bool {
	var count int
	query := `SELECT COUNT(*) FROM data WHERE content_hash = ? AND group_id = ?`
	err := s.db.QueryRowContext(ctx, query, contentHash, groupID).Scan(&count)
	if err != nil {
		return false
	}
	return count > 0
}

// GetDataByID retrieves data by ID, strictly filtered by group_id for partition consistency.
// Even though ID might be globally unique, we enforce group_id filtering
// to maintain strict partitioning across all database operations.
func (s *DuckDBStorage) GetDataByID(ctx context.Context, id string, groupID string) (*storage.Data, error) {
	query := `SELECT id::VARCHAR, group_id, name, raw_data_location, extension, content_hash, created_at FROM data WHERE id = ? AND group_id = ?`
	row := s.db.QueryRowContext(ctx, query, id, groupID)

	var data storage.Data
	if err := row.Scan(&data.ID, &data.GroupID, &data.Name, &data.RawDataLocation, &data.Extension, &data.ContentHash, &data.CreatedAt); err != nil {
		return nil, err
	}
	return &data, nil
}

func (s *DuckDBStorage) GetDataList(ctx context.Context, groupID string) ([]*storage.Data, error) {
	query := `SELECT id::VARCHAR, group_id, name, raw_data_location, extension, content_hash, created_at FROM data WHERE group_id = ?`
	rows, err := s.db.QueryContext(ctx, query, groupID)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var dataList []*storage.Data
	for rows.Next() {
		var data storage.Data
		if err := rows.Scan(&data.ID, &data.GroupID, &data.Name, &data.RawDataLocation, &data.Extension, &data.ContentHash, &data.CreatedAt); err != nil {
			return nil, err
		}
		dataList = append(dataList, &data)
	}
	return dataList, nil
}

func (s *DuckDBStorage) SaveDocument(ctx context.Context, doc *storage.Document) error {
	query := `
	INSERT INTO documents (id, group_id, data_id, text, metadata, created_at)
	VALUES (?, ?, ?, ?, ?, current_timestamp)
	ON CONFLICT (group_id, id) DO UPDATE SET
		text = excluded.text,
		metadata = excluded.metadata
	`
	// Metadata to JSON string
	metaJSON, err := json.Marshal(doc.MetaData)
	if err != nil {
		return fmt.Errorf("failed to marshal metadata: %w", err)
	}

	if _, err := s.db.ExecContext(ctx, query, doc.ID, doc.GroupID, doc.DataID, doc.Text, string(metaJSON)); err != nil {
		return fmt.Errorf("failed to save document: %w", err)
	}
	return nil
}

func (s *DuckDBStorage) SaveChunk(ctx context.Context, chunk *storage.Chunk) error {
	// 1. Save Chunk
	chunkQuery := `
		INSERT INTO chunks (id, group_id, document_id, text, chunk_index, token_count, created_at)
		VALUES (?, ?, ?, ?, ?, ?, current_timestamp)
		ON CONFLICT (group_id, id) DO UPDATE SET
			text = excluded.text,
			chunk_index = excluded.chunk_index,
			token_count = excluded.token_count
	`
	_, err := s.db.ExecContext(ctx, chunkQuery, chunk.ID, chunk.GroupID, chunk.DocumentID, chunk.Text, chunk.ChunkIndex, chunk.TokenCount)
	if err != nil {
		return fmt.Errorf("failed to save chunk: %w", err)
	}

	// 2. Save Vector (if embedding exists)
	if len(chunk.Embedding) > 0 {
		vectorQuery := `
			INSERT INTO vectors (id, group_id, collection_name, text, embedding)
			VALUES (?, ?, ?, ?, ?)
			ON CONFLICT (group_id, collection_name, id) DO UPDATE SET
				text = excluded.text,
				embedding = excluded.embedding
		`
		collectionName := "DocumentChunk_text"
		_, err = s.db.ExecContext(ctx, vectorQuery, chunk.ID, chunk.GroupID, collectionName, chunk.Text, chunk.Embedding)
		if err != nil {
			return fmt.Errorf("failed to save vector: %w", err)
		}
	}

	return nil
}

func (s *DuckDBStorage) SaveEmbedding(ctx context.Context, collectionName, id, text string, vector []float32, groupID string) error {
	query := `
		INSERT INTO vectors (id, group_id, collection_name, text, embedding)
		VALUES (?, ?, ?, ?, ?)
		ON CONFLICT (group_id, collection_name, id) DO UPDATE SET
			text = excluded.text,
			embedding = excluded.embedding
	`
	_, err := s.db.ExecContext(ctx, query, id, groupID, collectionName, text, vector)
	if err != nil {
		return fmt.Errorf("failed to save embedding: %w", err)
	}
	return nil
}

func (s *DuckDBStorage) Search(ctx context.Context, collectionName string, vector []float32, k int, groupID string) ([]*storage.SearchResult, error) {
	query := `
		SELECT id, text, array_cosine_similarity(embedding, ?::FLOAT[1536]) as score
		FROM vectors
		WHERE collection_name = ? AND group_id = ?
		ORDER BY score DESC
		LIMIT ?
	`

	rows, err := s.db.QueryContext(ctx, query, vector, collectionName, groupID, k)
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

func (s *DuckDBStorage) Close() error {
	return s.db.Close()
}
