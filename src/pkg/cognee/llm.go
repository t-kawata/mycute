package cognee

import (
	"context"
	"os"

	"github.com/tmc/langchaingo/llms"
	"github.com/tmc/langchaingo/llms/openai"
)

// NewLLMClient initializes a new LLM client pointing to Bifrost.
func NewLLMClient(ctx context.Context) (llms.Model, error) {
	// Get Bifrost URL from environment variable
	baseURL := os.Getenv("OPENAI_BASE_URL")
	if baseURL == "" {
		// Default or error handling. For now, we assume it's set or use a default if needed.
		// In a real scenario, we might want to return an error if strictly required.
		baseURL = "https://bifrost.example.com/v1"
	}

	// Initialize Langchaingo OpenAI provider
	// We use the environment variable OPENAI_API_KEY for authentication.
	llm, err := openai.New(
		openai.WithBaseURL(baseURL),
		openai.WithToken(os.Getenv("OPENAI_API_KEY")),
		openai.WithModel("gpt-4o"), // Default model, can be made configurable
	)
	if err != nil {
		return nil, err
	}

	return llm, nil
}
