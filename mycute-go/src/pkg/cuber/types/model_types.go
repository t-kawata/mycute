package types

type ModelType string

const (
	MODEL_TYPE_CHAT_COMPLETION ModelType = "chat_completion"
	MODEL_TYPE_EMBEDDING       ModelType = "embedding"
)

// ChatModelConfig holds configuration for a chat model.
type ChatModelConfig struct {
	Provider    string   `json:"provider"`
	Model       string   `json:"model"`
	BaseURL     string   `json:"base_url,omitempty"`
	ApiKey      string   `json:"-"`
	MaxTokens   int      `json:"max_tokens"`
	Temperature *float64 `json:"temperature"` // pointer to distinguish 0 from nil
}
