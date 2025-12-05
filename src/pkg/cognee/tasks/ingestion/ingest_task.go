package ingestion

import (
	"context"
	"crypto/sha256"
	"encoding/hex"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"time"

	"github.com/google/uuid"
	"mycute/pkg/cognee/pipeline"
	"mycute/pkg/cognee/storage"
)

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

// Ensure interface implementation
var _ pipeline.Task = (*IngestTask)(nil)

// Helper: Deterministic ID Generation
func generateDeterministicID(contentHash string, groupID string) string {
	// Namespace for Cognee Ingestion
	namespace := uuid.MustParse("6ba7b810-9dad-11d1-80b4-00c04fd430c8") // UUID OID Namespace or similar
	return uuid.NewSHA1(namespace, []byte(contentHash+groupID)).String()
}

func (t *IngestTask) Run(ctx context.Context, input any) (any, error) {
	filePaths, ok := input.([]string)
	if !ok {
		return nil, fmt.Errorf("expected []string input, got %T", input)
	}

	var dataList []*storage.Data

	for _, path := range filePaths {
		// 1. Calculate Hash & Metadata
		hash, err := calculateFileHash(path)
		if err != nil {
			return nil, fmt.Errorf("failed to calculate hash for %s: %w", path, err)
		}

		// 2. Check Duplication (DuckDB)
		// Note: With deterministic IDs, ON CONFLICT handles deduplication.
		// However, checking existence might still be useful to skip reprocessing.
		// DuckDBStorage.Exists checks by content_hash, but now we should probably trust the ID.
		if t.vectorStorage.Exists(ctx, hash) {
			fmt.Printf("Skipping duplicate file: %s (hash: %s)\n", path, hash)
			// Retrieve existing data ID if possible, or regenerate it deterministically to return it?
			id := generateDeterministicID(hash, t.groupID)
			data := &storage.Data{ID: id, GroupID: t.groupID, ContentHash: hash, Name: filepath.Base(path)} // Minimal for return
			dataList = append(dataList, data)
			continue
		}

		_, err = os.Stat(path)
		if err != nil {
			return nil, fmt.Errorf("failed to stat file %s: %w", path, err)
		}

		// 3. Create Data Object
		// Generate Deterministic ID
		dataID := generateDeterministicID(hash, t.groupID)

		data := &storage.Data{
			ID:              dataID,
			GroupID:         t.groupID, // Set Partition ID
			Name:            filepath.Base(path),
			Extension:       filepath.Ext(path),
			ContentHash:     hash,
			RawDataLocation: path,
			CreatedAt:       time.Now(),
		}

		// 4. Save to DuckDB
		if err := t.vectorStorage.SaveData(ctx, data); err != nil {
			return nil, fmt.Errorf("failed to save data %s: %w", data.Name, err)
		}

		dataList = append(dataList, data)
		fmt.Printf("Ingested file: %s (id: %s)\n", data.Name, data.ID)
	}

	return dataList, nil
}

func calculateFileHash(filePath string) (string, error) {
	file, err := os.Open(filePath)
	if err != nil {
		return "", err
	}
	defer file.Close()

	hash := sha256.New()
	if _, err := io.Copy(hash, file); err != nil {
		return "", err
	}

	return hex.EncodeToString(hash.Sum(nil)), nil
}
