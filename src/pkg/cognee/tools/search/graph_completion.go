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
	queryVector, err := t.Embedder.EmbedQuery(ctx, query)
	if err != nil {
		return "", fmt.Errorf("failed to embed query: %w", err)
	}

	// 1. Vector Search (Parallel execution using errgroup is recommended)

	// A. Search Chunks (Existing)
	chunkResults, err := t.VectorStorage.Search(ctx, "DocumentChunk_text", queryVector, 5)
	if err != nil {
		return "", fmt.Errorf("chunk search failed: %w", err)
	}

	// B. Search Nodes [NEW]
	nodeResults, err := t.VectorStorage.Search(ctx, "Entity_name", queryVector, 5)
	if err != nil {
		return "", fmt.Errorf("node search failed: %w", err)
	}

	// 2. Graph Traversal [NEW]
	var nodeIDs []string
	uniqueNodes := make(map[string]bool)

	for _, res := range nodeResults {
		if !uniqueNodes[res.ID] {
			nodeIDs = append(nodeIDs, res.ID)
			uniqueNodes[res.ID] = true
		}
	}

	// Get Triplets from GraphStorage
	// CozoStorage.GetTriplets is already implemented
	triplets, err := t.GraphStorage.GetTriplets(ctx, nodeIDs)
	if err != nil {
		return "", fmt.Errorf("graph traversal failed: %w", err)
	}

	// 3. Construct Context
	var contextBuilder strings.Builder

	contextBuilder.WriteString("### Relevant Text Chunks:\n")
	if len(chunkResults) == 0 {
		contextBuilder.WriteString("No relevant text chunks found.\n")
	}
	for _, res := range chunkResults {
		contextBuilder.WriteString("- " + res.Text + "\n")
	}

	contextBuilder.WriteString("\n### Knowledge Graph Connections:\n")
	if len(triplets) == 0 {
		contextBuilder.WriteString("No relevant graph connections found.\n")
	}
	for _, triplet := range triplets {
		// Format: Source --[Type]--> Target
		// Ensure properties exist and are strings
		sourceName := getName(triplet.Source)
		targetName := getName(triplet.Target)

		contextBuilder.WriteString(fmt.Sprintf("- %s --[%s]--> %s\n",
			sourceName,
			triplet.Edge.Type,
			targetName))
	}

	// 4. Generate Answer
	// Use explicit System and User messages to align with Python implementation

	// Create the User prompt using the context-aware template
	userPrompt := fmt.Sprintf(prompts.GraphContextForQuestionPrompt, query, contextBuilder.String())

	// Generate content using System (behavior instruction) and User (query+context) messages
	resp, err := t.LLM.GenerateContent(ctx, []llms.MessageContent{
		llms.TextParts(llms.ChatMessageTypeSystem, prompts.AnswerSimpleQuestionPrompt),
		llms.TextParts(llms.ChatMessageTypeHuman, userPrompt),
	})
	if err != nil {
		return "", fmt.Errorf("failed to generate answer: %w", err)
	}

	if len(resp.Choices) == 0 {
		return "", fmt.Errorf("no answer generated")
	}

	return resp.Choices[0].Content, nil
}

// Helper to safely get name from node properties
func getName(node *storage.Node) string {
	if node == nil || node.Properties == nil {
		return "Unknown"
	}
	if name, ok := node.Properties["name"].(string); ok {
		return name
	}
	return node.ID // Fallback to ID
}
