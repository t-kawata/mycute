package graph

import (
	"context"
	"encoding/json"
	"fmt"
	"strings"
	"sync"

	"mycute/pkg/cognee/pipeline"
	"mycute/pkg/cognee/prompts"
	"mycute/pkg/cognee/storage"
	"github.com/tmc/langchaingo/llms"
	"golang.org/x/sync/errgroup"
)

type GraphExtractionTask struct {
	LLM llms.Model
}

func NewGraphExtractionTask(llm llms.Model) *GraphExtractionTask {
	return &GraphExtractionTask{LLM: llm}
}

// Ensure interface implementation
var _ pipeline.Task = (*GraphExtractionTask)(nil)

func (t *GraphExtractionTask) Run(ctx context.Context, input any) (any, error) {
	chunks, ok := input.([]*storage.Chunk)
	if !ok {
		return nil, fmt.Errorf("expected []*storage.Chunk input, got %T", input)
	}

	var (
		allNodes []*storage.Node
		allEdges []*storage.Edge
		mu       sync.Mutex
	)

	g, ctx := errgroup.WithContext(ctx)
	// Limit concurrency to avoid rate limits?
	g.SetLimit(5) // Example limit

	for _, chunk := range chunks {
		chunk := chunk // capture loop variable
		g.Go(func() error {
			// 1. Generate Prompt
			// Convert the original system prompt (which is just instructions) into a complete prompt
			// that ensures JSON output and includes the text to process.
			schemaInstructions := `
Return the result as a JSON object with "nodes" and "edges" arrays.
Each node should have "id", "type", and "properties".
Each edge should have "source_id", "target_id", "type", and "properties".
`
			prompt := fmt.Sprintf("%s\n\n%s\n\nText:\n%s", prompts.GenerateGraphPrompt, schemaInstructions, chunk.Text)

			// 2. Call LLM
			// Using GenerateContent or Call depending on interface.
			// llms.Model has GenerateContent.
			resp, err := t.LLM.GenerateContent(ctx, []llms.MessageContent{
				llms.TextParts(llms.ChatMessageTypeHuman, prompt),
			})
			if err != nil {
				return fmt.Errorf("LLM call failed: %w", err)
			}

			if len(resp.Choices) == 0 {
				return fmt.Errorf("no response from LLM")
			}
			content := resp.Choices[0].Content

			// 3. Parse JSON
			// LLM might return markdown code block ```json ... ```
			content = cleanJSON(content)

			var graphData storage.GraphData
			if err := json.Unmarshal([]byte(content), &graphData); err != nil {
				// Log error but maybe continue? Or fail?
				// For now, fail to be safe.
				return fmt.Errorf("failed to parse JSON: %w\nContent: %s", err, content)
			}

			mu.Lock()
			allNodes = append(allNodes, graphData.Nodes...)
			allEdges = append(allEdges, graphData.Edges...)
			mu.Unlock()

			return nil
		})
	}

	if err := g.Wait(); err != nil {
		return nil, err
	}

	return &storage.CognifyOutput{
		Chunks: chunks,
		GraphData: &storage.GraphData{
			Nodes: allNodes,
			Edges: allEdges,
		},
	}, nil
}

func cleanJSON(content string) string {
	content = strings.TrimSpace(content)
	if strings.HasPrefix(content, "```json") {
		content = strings.TrimPrefix(content, "```json")
		content = strings.TrimSuffix(content, "```")
	} else if strings.HasPrefix(content, "```") {
		content = strings.TrimPrefix(content, "```")
		content = strings.TrimSuffix(content, "```")
	}
	return strings.TrimSpace(content)
}
