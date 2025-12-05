package search

import (
	"context"
	"fmt"
	"strings"

	"mycute/pkg/cognee/prompts"
	"mycute/pkg/cognee/storage"
	"github.com/tmc/langchaingo/llms"
)

type GraphCompletionTool struct {
	VectorStorage storage.VectorStorage
	GraphStorage  storage.GraphStorage
	LLM           llms.Model
	Embedder      storage.Embedder
	groupID       string // [NEW] Partition Context
}

func NewGraphCompletionTool(vectorStorage storage.VectorStorage, graphStorage storage.GraphStorage, llm llms.Model, embedder storage.Embedder, groupID string) *GraphCompletionTool {
	return &GraphCompletionTool{
		VectorStorage: vectorStorage,
		GraphStorage:  graphStorage,
		LLM:           llm,
		Embedder:      embedder,
		groupID:       groupID,
	}
}

func (t *GraphCompletionTool) Search(ctx context.Context, query string, searchType SearchType) (string, error) {
	switch searchType {
	case SearchTypeSummaries:
		return t.searchSummaries(ctx, query)
	case SearchTypeGraphSummaryCompletion:
		return t.searchGraphSummaryCompletion(ctx, query)
	case SearchTypeGraphCompletion:
		fallthrough
	default:
		return t.searchGraphCompletion(ctx, query)
	}
}

// [NEW] Summaries Search Implementation
func (t *GraphCompletionTool) searchSummaries(ctx context.Context, query string) (string, error) {
	queryVector, err := t.Embedder.EmbedQuery(ctx, query)
	if err != nil {
		return "", fmt.Errorf("failed to embed query: %w", err)
	}

	// Search "TextSummary_text" collection
	results, err := t.VectorStorage.Search(ctx, "TextSummary_text", queryVector, 5, t.groupID)
	if err != nil {
		return "", fmt.Errorf("summary search failed: %w", err)
	}

	if len(results) == 0 {
		return "No relevant summaries found.", nil
	}

	var sb strings.Builder
	for _, res := range results {
		sb.WriteString("- " + res.Text + "\n")
	}
	return sb.String(), nil
}

// [NEW] Graph Summary Completion Implementation
func (t *GraphCompletionTool) searchGraphSummaryCompletion(ctx context.Context, query string) (string, error) {
	// 1. Run Standard Graph Completion Logic (Nodes -> Graph Traversal)
	// We need to extract the logic that gets the triplets.
	// For reusing code, we can extract the triplet retrieval part into a helper,
	// but to follow the instruction strictly and avoid complex refactoring risks now,
	// I will duplicate the core logic of finding triplets here as described in directives.

	queryVector, err := t.Embedder.EmbedQuery(ctx, query)
	if err != nil {
		return "", fmt.Errorf("failed to embed query: %w", err)
	}

	// Search Nodes
	nodeResults, err := t.VectorStorage.Search(ctx, "Entity_name", queryVector, 5, t.groupID)
	if err != nil {
		return "", fmt.Errorf("node search failed: %w", err)
	}

	var nodeIDs []string
	uniqueNodes := make(map[string]bool)

	for _, res := range nodeResults {
		if !uniqueNodes[res.ID] {
			nodeIDs = append(nodeIDs, res.ID)
			uniqueNodes[res.ID] = true
		}
	}

	triplets, err := t.GraphStorage.GetTriplets(ctx, nodeIDs)
	if err != nil {
		return "", fmt.Errorf("graph traversal failed: %w", err)
	}

	if len(triplets) == 0 {
		return "No relevant graph connections found to summarize.", nil
	}

	// 2. Convert Triplets to Text
	var graphText strings.Builder
	for _, triplet := range triplets {
		sourceName := getName(triplet.Source)
		targetName := getName(triplet.Target)
		graphText.WriteString(fmt.Sprintf("- %s --[%s]--> %s\n", sourceName, triplet.Edge.Type, targetName))
	}

	// 3. Summarize the Graph Text
	summarizePrompt := fmt.Sprintf(prompts.SummarizeSearchResultsPrompt, query, graphText.String())

	summaryResp, err := t.LLM.GenerateContent(ctx, []llms.MessageContent{
		llms.TextParts(llms.ChatMessageTypeHuman, summarizePrompt),
	})
	if err != nil {
		return "", fmt.Errorf("failed to generate summary of graph: %w", err)
	}
	if len(summaryResp.Choices) == 0 {
		return "", fmt.Errorf("empty summary response from LLM")
	}
	summaryContext := summaryResp.Choices[0].Content

	// 4. Generate Final Answer using the Summary as Context
	// Reuse the exact same prompts as GraphCompletion
	finalUserPrompt := fmt.Sprintf(prompts.GraphContextForQuestionPrompt, query, summaryContext)

	finalResp, err := t.LLM.GenerateContent(ctx, []llms.MessageContent{
		llms.TextParts(llms.ChatMessageTypeSystem, prompts.AnswerSimpleQuestionPrompt),
		llms.TextParts(llms.ChatMessageTypeHuman, finalUserPrompt),
	})
	if err != nil {
		return "", fmt.Errorf("failed to generate final answer: %w", err)
	}

	if len(finalResp.Choices) == 0 {
		return "", fmt.Errorf("no final answer generated")
	}

	return finalResp.Choices[0].Content, nil
}

// Original Logic moved to method
func (t *GraphCompletionTool) searchGraphCompletion(ctx context.Context, query string) (string, error) {
	queryVector, err := t.Embedder.EmbedQuery(ctx, query)
	if err != nil {
		return "", fmt.Errorf("failed to embed query: %w", err)
	}

	// 1. Vector Search (Parallel execution using errgroup is recommended)

	// A. Search Chunks (Existing)
	chunkResults, err := t.VectorStorage.Search(ctx, "DocumentChunk_text", queryVector, 5, t.groupID)
	if err != nil {
		return "", fmt.Errorf("chunk search failed: %w", err)
	}

	// B. Search Nodes [NEW]
	nodeResults, err := t.VectorStorage.Search(ctx, "Entity_name", queryVector, 5, t.groupID)
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
