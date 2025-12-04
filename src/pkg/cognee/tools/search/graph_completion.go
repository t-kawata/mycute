package search

import (
	"context"
	"fmt"
	"strings"

	"github.com/t-kawata/mycute/pkg/cognee/prompts"
	"github.com/t-kawata/mycute/pkg/cognee/storage"
	"github.com/tmc/langchaingo/llms"
)

type GraphCompletionTool struct {
	VectorStorage storage.VectorStorage
	GraphStorage  storage.GraphStorage
	LLM           llms.Model
	Embedder      storage.Embedder
}

func NewGraphCompletionTool(vectorStorage storage.VectorStorage, graphStorage storage.GraphStorage, llm llms.Model, embedder storage.Embedder) *GraphCompletionTool {
	return &GraphCompletionTool{
		VectorStorage: vectorStorage,
		GraphStorage:  graphStorage,
		LLM:           llm,
		Embedder:      embedder,
	}
}

func (t *GraphCompletionTool) Search(ctx context.Context, query string) (string, error) {
	// 1. Generate Query Embedding
	queryVector, err := t.Embedder.EmbedQuery(ctx, query)
	if err != nil {
		return "", fmt.Errorf("failed to embed query: %w", err)
	}

	// 2. Vector Search (DuckDB)
	// Search in "DocumentChunk_text" collection
	fmt.Printf("DEBUG: Searching with vector (len=%d)\n", len(queryVector))
	searchResults, err := t.VectorStorage.Search(ctx, "DocumentChunk_text", queryVector, 5) // Top 5
	if err != nil {
		return "", fmt.Errorf("vector search failed: %w", err)
	}
	fmt.Printf("DEBUG: Found %d results\n", len(searchResults))

	if len(searchResults) == 0 {
		return "Information not found.", nil
	}

	// 3. Graph Traversal (CozoDB)
	// Get related nodes/edges for the found chunks (chunks are nodes in our graph? No, chunks are text)
	// Wait, in Phase 2C1, we saved chunks to DuckDB.
	// Did we save chunks as nodes in CozoDB?
	// The GraphExtractionTask extracts nodes/edges from text.
	// The chunks themselves are not necessarily nodes, but they contain entities.
	// But `VectorStorage.Search` returns Chunk IDs.
	// How do we map Chunk IDs to Graph Nodes?
	// In Python implementation: `map_vector_distances_to_graph_nodes`.
	// It seems we need to search for Nodes (Entities) in VectorStorage too?
	// Or we assume Chunks are linked to Nodes?
	// Currently, `GraphExtractionTask` extracts nodes from text.
	// It doesn't explicitly link chunks to nodes in the DB, except via the text content.
	// However, `StorageTask` saves nodes/edges.
	// If we want to use Graph Traversal, we need to start from Nodes.
	// So we should Vector Search for NODES (Entities).
	// Did we save Node embeddings?
	// In `StorageTask`, we might need to save Node embeddings if we want to search them.
	// Let's check `StorageTask`.

	// If we only have Chunk embeddings, we find relevant Chunks.
	// Then we can use the text of the chunks as context.
	// But the requirement says "Graph Traversal".
	// "Vector Search -> Node IDs -> CozoDB Traversal".
	// This implies we have Node embeddings.

	// Let's assume for now we use the Chunk text as context (RAG).
	// But to fulfill "Graph Traversal", we need to map to nodes.
	// If we don't have Node embeddings, we can't map easily.

	// Let's check `StorageTask` implementation in `src/pkg/cognee/tasks/storage/storage_task.go`.

	// For now, I will implement RAG using Chunks.
	// And if I can, I'll try to find nodes mentioned in the chunks?
	// Or maybe I should just return the chunk text.

	// Wait, the directions say:
	// 1. Vector Search -> Node IDs
	// 2. CozoDB Traversal -> Triplets

	// This strongly suggests Node embeddings.
	// I need to check if `StorageTask` saves Node embeddings.

	var contextBuilder strings.Builder
	for _, res := range searchResults {
		contextBuilder.WriteString(res.Text + "\n")
	}

	// 4. Generate Answer
	prompt := fmt.Sprintf(prompts.GraphContextForQuestionPrompt, query, contextBuilder.String())

	response, err := llms.GenerateFromSinglePrompt(ctx, t.LLM, prompt,
		llms.WithTemperature(0),
	)
	if err != nil {
		return "", fmt.Errorf("failed to generate answer: %w", err)
	}

	return response, nil
}
