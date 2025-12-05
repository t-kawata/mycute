package storage

import (
	"context"
	"fmt"

	"mycute/pkg/cognee/pipeline"
	"mycute/pkg/cognee/storage"
)

type StorageTask struct {
	VectorStorage storage.VectorStorage
	GraphStorage  storage.GraphStorage
	Embedder      storage.Embedder
	groupID       string
}

func NewStorageTask(vectorStorage storage.VectorStorage, graphStorage storage.GraphStorage, embedder storage.Embedder, groupID string) *StorageTask {
	return &StorageTask{
		VectorStorage: vectorStorage,
		GraphStorage:  graphStorage,
		Embedder:      embedder,
		groupID:       groupID,
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
		// Check if embedding is present
		if len(chunk.Embedding) == 0 {
			// 埋め込みが空の場合のみ、ログ警告を出して再生成を試みる
			// 最も堅牢だが、「保存タスクが生成も行う」ことになり、責務が少し曖昧になる
			// でもひとまず堅牢性のために許容するものとする
			fmt.Printf("Warning: embedding missing for chunk %s. Regenerating...\n", chunk.ID)
			embedding, err := t.Embedder.EmbedQuery(ctx, chunk.Text)
			if err != nil {
				return nil, fmt.Errorf("failed to regenerate embedding for chunk %s: %w", chunk.ID, err)
			}
			chunk.Embedding = embedding
		}
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
			// Check "name" property or use ID
			var name string
			if nameInterface, ok := node.Properties["name"]; ok {
				name, _ = nameInterface.(string)
			}
			if name == "" {
				name = node.ID // Fallback to ID
			}

			if name == "" {
				continue
			}

			// Generate Embedding for Node Name
			embedding, err := t.Embedder.EmbedQuery(ctx, name)
			if err != nil {
				fmt.Printf("Warning: failed to embed node %s: %v\n", name, err)
				continue
			}

			// Save to DuckDB (Collection: "Entity_name")
			if err := t.VectorStorage.SaveEmbedding(ctx, "Entity_name", node.ID, name, embedding, t.groupID); err != nil {
				return nil, fmt.Errorf("failed to save node embedding: %w", err)
			}
		}
	}

	return output, nil
}
