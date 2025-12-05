package summarization

import (
	"context"
	"fmt"

	"github.com/google/uuid"
	"mycute/pkg/cognee/pipeline"
	"mycute/pkg/cognee/prompts"
	"mycute/pkg/cognee/storage"
	"github.com/tmc/langchaingo/llms"
)

type SummarizationTask struct {
	VectorStorage storage.VectorStorage
	LLM           llms.Model
	Embedder      storage.Embedder
	groupID       string // [NEW] Partition Context
}

func NewSummarizationTask(vectorStorage storage.VectorStorage, llm llms.Model, embedder storage.Embedder, groupID string) *SummarizationTask {
	return &SummarizationTask{
		VectorStorage: vectorStorage,
		LLM:           llm,
		Embedder:      embedder,
		groupID:       groupID,
	}
}

// Ensure interface implementation
var _ pipeline.Task = (*SummarizationTask)(nil)

func (t *SummarizationTask) Run(ctx context.Context, input any) (any, error) {
	output, ok := input.(*storage.CognifyOutput)
	if !ok {
		return nil, fmt.Errorf("expected *storage.CognifyOutput input, got %T", input)
	}

	fmt.Printf("Summarizing %d chunks...\n", len(output.Chunks))

	for _, chunk := range output.Chunks {
		// 1. Generate Summary Prompt
		prompt := fmt.Sprintf(prompts.SummarizeContentPrompt, chunk.Text)

		// 2. Call LLM (Generate Summary)
		resp, err := t.LLM.GenerateContent(ctx, []llms.MessageContent{
			llms.TextParts(llms.ChatMessageTypeHuman, prompt),
		})
		if err != nil {
			// Continue on error? Or Fail? For now, log and continue to robustly process others.
			fmt.Printf("Warning: failed to summarize chunk %s: %v\n", chunk.ID, err)
			continue
		}
		if len(resp.Choices) == 0 {
			continue
		}
		summaryText := resp.Choices[0].Content

		// 3. Generate Embedding for Summary
		embedding, err := t.Embedder.EmbedQuery(ctx, summaryText)
		if err != nil {
			fmt.Printf("Warning: failed to embed summary for chunk %s: %v\n", chunk.ID, err)
			continue
		}

		// 4. Save Summary to DuckDB
		// ID: Consistent ID based on Chunk ID (Simulate UUID5 behavior roughly by hashing)
		// Python uses uuid5(chunk.id, "TextSummary").
		namespace := uuid.MustParse("00000000-0000-0000-0000-000000000000") // Or specific namespace
		summaryID := uuid.NewSHA1(namespace, []byte(chunk.ID+"TextSummary")).String()

		// Collection: "TextSummary_text"
		if err := t.VectorStorage.SaveEmbedding(ctx, "TextSummary_text", summaryID, summaryText, embedding, t.groupID); err != nil {
			return nil, fmt.Errorf("failed to save summary embedding: %w", err)
		}
	}

	return output, nil // Pass through for any subsequent tasks
}
