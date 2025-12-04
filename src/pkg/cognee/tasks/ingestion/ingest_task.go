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
	"github.com/t-kawata/mycute/pkg/cognee/pipeline"
	"github.com/t-kawata/mycute/pkg/cognee/storage"
)

type IngestTask struct {
	vectorStorage storage.VectorStorage
}

func NewIngestTask(vectorStorage storage.VectorStorage) *IngestTask {
	return &IngestTask{vectorStorage: vectorStorage}
}

// Ensure interface implementation
var _ pipeline.Task = (*IngestTask)(nil)

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
		if t.vectorStorage.Exists(ctx, hash) {
			fmt.Printf("Skipping duplicate file: %s (hash: %s)\n", path, hash)
			// Optionally fetch existing data to return it?
			// For now, we just skip processing.
			// But if the pipeline expects data, we might need to return it.
			// Let's try to fetch it.
			// Since Exists doesn't return ID, we can't easily fetch it without ID or Hash query.
			// DuckDBStorage.Exists uses content_hash query.
			// We can add GetDataByHash if needed, but for now let's just skip adding to list
			// or maybe we should return it so subsequent tasks can process it?
			// If we skip, subsequent tasks won't have it.
			// If the goal is "Add", maybe we don't need to re-process.
			// But if we run "Cognify" later, we need the data.
			// Usually IngestTask returns NEW data.
			continue
		}

		_, err = os.Stat(path)
		if err != nil {
			return nil, fmt.Errorf("failed to stat file %s: %w", path, err)
		}

		// 3. Create Data Object
		// Use uuid5 or random? Python uses identify(classified_data, user) which uses content hash.
		// Let's use uuid5 with namespace and hash for deterministic ID.
		// Or just random for now as per docs example `uuid.New()`.
		// Docs say: `ID: uuid.New(), // またはハッシュから生成`
		// Let's use uuid.New() for simplicity, but deterministic is better.
		// Let's use uuid.New() as per docs example.

		data := &storage.Data{
			ID:              uuid.New().String(),
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
