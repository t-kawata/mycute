package search

import (
	"context"
	"fmt"

	"github.com/tmc/langchaingo/llms/openai"
)

type OpenAIEmbedderAdapter struct {
	LLM *openai.LLM
}

func NewOpenAIEmbedderAdapter(llm *openai.LLM) *OpenAIEmbedderAdapter {
	return &OpenAIEmbedderAdapter{LLM: llm}
}

func (a *OpenAIEmbedderAdapter) EmbedQuery(ctx context.Context, text string) ([]float32, error) {
	embeddings, err := a.LLM.CreateEmbedding(ctx, []string{text})
	if err != nil {
		return nil, fmt.Errorf("failed to create embedding: %w", err)
	}
	if len(embeddings) == 0 {
		return nil, fmt.Errorf("no embeddings returned")
	}
	return embeddings[0], nil
}
