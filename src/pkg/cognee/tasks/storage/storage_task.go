package storage

import (
	"context"
	"fmt"

	"github.com/t-kawata/mycute/pkg/cognee/pipeline"
	"github.com/t-kawata/mycute/pkg/cognee/storage"
)

type StorageTask struct {
	VectorStorage storage.VectorStorage
	GraphStorage  storage.GraphStorage
	Embedder      storage.Embedder
}

func NewStorageTask(vectorStorage storage.VectorStorage, graphStorage storage.GraphStorage, embedder storage.Embedder) *StorageTask {
	return &StorageTask{
		VectorStorage: vectorStorage,
		GraphStorage:  graphStorage,
		Embedder:      embedder,
	}
}

// Ensure interface implementation
var _ pipeline.Task = (*StorageTask)(nil)

func (t *StorageTask) Run(ctx context.Context, input any) (any, error) {
	output, ok := input.(*storage.CognifyOutput)
	if !ok {
		return nil, fmt.Errorf("expected *storage.CognifyOutput input, got %T", input)
	}

	// 1. Save Chunks (Vectors)
	for _, chunk := range output.Chunks {
		// TODO: Generate embedding for chunk if not already present?
		// Currently ChunkingTask doesn't generate embeddings.
		// If we want embeddings, we should have an EmbeddingTask or do it in ChunkingTask.
		// For now, we save chunks without embeddings or with empty embeddings.
		// The VectorStorage.SaveChunk handles empty embeddings gracefully (just skips vector table insert).
		if err := t.VectorStorage.SaveChunk(ctx, chunk); err != nil {
			return nil, fmt.Errorf("failed to save chunk %s: %w", chunk.ID, err)
		}
	}
	fmt.Printf("Saved %d chunks to DuckDB\n", len(output.Chunks))

	// 2. Save Graph (Nodes/Edges)
	if output.GraphData != nil {
		if err := t.GraphStorage.AddNodes(ctx, output.GraphData.Nodes); err != nil {
			return nil, fmt.Errorf("failed to add nodes: %w", err)
		}
		if err := t.GraphStorage.AddEdges(ctx, output.GraphData.Edges); err != nil {
			return nil, fmt.Errorf("failed to add edges: %w", err)
		}
		fmt.Printf("Saved %d nodes and %d edges to CozoDB\n", len(output.GraphData.Nodes), len(output.GraphData.Edges))

		// 3. Index Nodes (Embeddings) [Phase 3]
		fmt.Printf("Indexing %d nodes...\n", len(output.GraphData.Nodes))
		for _, node := range output.GraphData.Nodes {
			// Check "name" property
			nameInterface, ok := node.Properties["name"]
			if !ok {
				continue
			}
			name, ok := nameInterface.(string)
			if !ok || name == "" {
				continue
			}

			// Generate Embedding for Node Name
			embedding, err := t.Embedder.EmbedQuery(ctx, name)
			if err != nil {
				fmt.Printf("Warning: failed to embed node %s: %v\n", name, err)
				continue
			}

			// Save to DuckDB (Collection: "Entity_name")
			if err := t.VectorStorage.SaveEmbedding(ctx, "Entity_name", node.ID, name, embedding); err != nil {
				return nil, fmt.Errorf("failed to save node embedding: %w", err)
			}
		}
	}

	return output, nil
}
