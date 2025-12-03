package cognee

import (
	"context"
	"os"

	"github.com/tmc/langchaingo/embeddings"
	"github.com/tmc/langchaingo/llms/openai"
)

// NewEmbedder initializes a new Embedder client using OpenAI API (via Bifrost).
func NewEmbedder(ctx context.Context) (embeddings.Embedder, error) {
	llm, err := openai.New(
		openai.WithBaseURL(os.Getenv("OPENAI_BASE_URL")),
		openai.WithToken(os.Getenv("OPENAI_API_KEY")),
		openai.WithModel("text-embedding-3-small"),
	)
	if err != nil {
		return nil, err
	}

	return embeddings.NewEmbedder(llm)
}
