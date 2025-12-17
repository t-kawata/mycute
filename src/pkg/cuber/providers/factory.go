package providers

import (
	"context"
	"fmt"
	"strings"

	// Eino Core
	"github.com/cloudwego/eino/components/embedding"
	"github.com/cloudwego/eino/components/model"

	// Eino Extensions - Chat Models
	claudemodel "github.com/cloudwego/eino-ext/components/model/claude"
	deepseekmodel "github.com/cloudwego/eino-ext/components/model/deepseek"
	geminimodel "github.com/cloudwego/eino-ext/components/model/gemini"
	ollamamodel "github.com/cloudwego/eino-ext/components/model/ollama"
	openaimodel "github.com/cloudwego/eino-ext/components/model/openai"
	openroutermodel "github.com/cloudwego/eino-ext/components/model/openrouter"
	qwenmodel "github.com/cloudwego/eino-ext/components/model/qwen"

	// Eino Extensions - Embeddings
	ollamaemb "github.com/cloudwego/eino-ext/components/embedding/ollama"
	openaiemb "github.com/cloudwego/eino-ext/components/embedding/openai"

	"google.golang.org/genai"
)

// ProviderType はサポートするLLMプロバイダーの識別子です。
type ProviderType string

const (
	// Dedicated Implementations
	ProviderOpenAI     ProviderType = "openai"
	ProviderGemini     ProviderType = "gemini"
	ProviderAnthropic  ProviderType = "anthropic"
	ProviderOllama     ProviderType = "ollama"
	ProviderDeepSeek   ProviderType = "deepseek"
	ProviderOpenRouter ProviderType = "openrouter"
	ProviderQwen       ProviderType = "qwen"

	// OpenAI Compatible Implementations
	ProviderMistral  ProviderType = "mistral"
	ProviderMeta     ProviderType = "meta"
	ProviderXAI      ProviderType = "xai"
	ProviderLlamaCpp ProviderType = "llamacpp"
)

// ProviderConfig はプロバイダー接続に必要な設定情報です。
type ProviderConfig struct {
	Type        ProviderType
	APIKey      string
	BaseURL     string // OpenAI互換プロバイダー、または特定の専用プロバイダーで必要な場合
	ModelName   string
	MaxTokens   int      // 生成する最大トークン数 (0の場合はデフォルト値またはプロバイダーのデフォルトが使用される)
	Temperature *float64 // Temperature (nilの場合はデフォルト値)
}

// NewChatModel は指定された設定に基づいて Eino ChatModel を生成します。
func NewChatModel(ctx context.Context, cfg ProviderConfig) (model.ToolCallingChatModel, error) {
	providerType := ProviderType(strings.ToLower(string(cfg.Type)))
	switch providerType {
	// ========================================================================
	// 1. OpenAI (Dedicated) & Compatible Providers
	// ========================================================================
	case ProviderOpenAI, ProviderMistral, ProviderMeta, ProviderXAI, ProviderLlamaCpp:
		config := &openaimodel.ChatModelConfig{
			APIKey:  cfg.APIKey,
			BaseURL: cfg.BaseURL,
			Model:   cfg.ModelName,
		}
		if cfg.MaxTokens > 0 {
			config.MaxCompletionTokens = &cfg.MaxTokens
		}
		if cfg.Temperature != nil {
			t := float32(*cfg.Temperature)
			config.Temperature = &t
		} else {
			tmp := float32(0.2)
			config.Temperature = &tmp
		}
		chatModel, err := openaimodel.NewChatModel(ctx, config)
		if err != nil {
			return nil, fmt.Errorf("Failed to create openai-compatible chat model for %s: %w", providerType, err)
		}
		return chatModel, nil
	// ========================================================================
	// 2. Dedicated Implementations
	// ========================================================================
	case ProviderGemini:
		client, err := genai.NewClient(ctx, &genai.ClientConfig{
			APIKey: cfg.APIKey,
		})
		if err != nil {
			return nil, fmt.Errorf("Failed to create genai client for gemini: %w", err)
		}
		config := &geminimodel.Config{
			Client: client,
			Model:  cfg.ModelName,
		}
		if cfg.MaxTokens > 0 {
			config.MaxTokens = &cfg.MaxTokens
		}
		if cfg.Temperature != nil {
			t := float32(*cfg.Temperature)
			config.Temperature = &t
		} else {
			tmp := float32(0.2)
			config.Temperature = &tmp
		}
		chatModel, err := geminimodel.NewChatModel(ctx, config)
		if err != nil {
			return nil, fmt.Errorf("Failed to create gemini chat model: %w", err)
		}
		return chatModel, nil
	case ProviderAnthropic:
		var baseURL *string
		if cfg.BaseURL != "" {
			s := cfg.BaseURL
			baseURL = &s
		}
		maxTokens := cfg.MaxTokens
		if maxTokens == 0 {
			maxTokens = 4096 // Default for Claude if not specified
		}
		config := &claudemodel.Config{
			APIKey:    cfg.APIKey,
			BaseURL:   baseURL,
			Model:     cfg.ModelName,
			MaxTokens: maxTokens,
		}
		if cfg.Temperature != nil {
			t := float32(*cfg.Temperature)
			config.Temperature = &t
		} else {
			tmp := float32(0.2)
			config.Temperature = &tmp
		}
		chatModel, err := claudemodel.NewChatModel(ctx, config)
		if err != nil {
			return nil, fmt.Errorf("Failed to create anthropic chat model: %w", err)
		}
		return chatModel, nil
	case ProviderOllama:
		config := &ollamamodel.ChatModelConfig{
			BaseURL: cfg.BaseURL,
			Model:   cfg.ModelName,
		}
		if cfg.MaxTokens > 0 {
			config.Options = &ollamamodel.Options{
				NumPredict: cfg.MaxTokens,
			}
		}
		if cfg.Temperature != nil {
			if config.Options == nil {
				config.Options = &ollamamodel.Options{}
			}
			t := float32(*cfg.Temperature)
			config.Options.Temperature = t
		} else {
			tmp := float32(0.2)
			config.Options.Temperature = tmp
		}
		chatModel, err := ollamamodel.NewChatModel(ctx, config)
		if err != nil {
			return nil, fmt.Errorf("Failed to create ollama chat model: %w", err)
		}
		return chatModel, nil
	case ProviderDeepSeek:
		config := &deepseekmodel.ChatModelConfig{
			APIKey:  cfg.APIKey,
			BaseURL: cfg.BaseURL,
			Model:   cfg.ModelName,
		}
		if cfg.MaxTokens > 0 {
			config.MaxTokens = cfg.MaxTokens
		}
		if cfg.Temperature != nil {
			t := float32(*cfg.Temperature)
			config.Temperature = t
		} else {
			tmp := float32(0.2)
			config.Temperature = tmp
		}
		chatModel, err := deepseekmodel.NewChatModel(ctx, config)
		if err != nil {
			return nil, fmt.Errorf("Failed to create deepseek chat model: %w", err)
		}
		return chatModel, nil
	case ProviderOpenRouter:
		config := &openroutermodel.Config{
			APIKey:  cfg.APIKey,
			Model:   cfg.ModelName,
			BaseURL: cfg.BaseURL,
		}
		if cfg.MaxTokens > 0 {
			config.MaxTokens = &cfg.MaxTokens
		}
		if cfg.Temperature != nil {
			t := float32(*cfg.Temperature)
			config.Temperature = &t
		} else {
			tmp := float32(0.2)
			config.Temperature = &tmp
		}
		chatModel, err := openroutermodel.NewChatModel(ctx, config)
		if err != nil {
			return nil, fmt.Errorf("Failed to create openrouter chat model: %w", err)
		}
		return chatModel, nil
	case ProviderQwen:
		config := &qwenmodel.ChatModelConfig{
			APIKey: cfg.APIKey,
			Model:  cfg.ModelName,
		}
		if cfg.MaxTokens > 0 {
			config.MaxTokens = &cfg.MaxTokens
		}
		if cfg.Temperature != nil {
			t := float32(*cfg.Temperature)
			config.Temperature = &t
		} else {
			tmp := float32(0.2)
			config.Temperature = &tmp
		}
		chatModel, err := qwenmodel.NewChatModel(ctx, config)
		if err != nil {
			return nil, fmt.Errorf("Failed to create qwen chat model: %w", err)
		}
		return chatModel, nil
	default:
		return nil, fmt.Errorf("Unsupported chat provider type: %s", cfg.Type)
	}
}

// NewEmbedder は指定された設定に基づいて Eino Embedder を生成します。
func NewEmbedder(ctx context.Context, cfg ProviderConfig) (embedding.Embedder, error) {
	providerType := ProviderType(strings.ToLower(string(cfg.Type)))
	switch providerType {
	// ========================================================================
	// 1. OpenAI (Dedicated) & Compatible Providers (including those with no emb pkg)
	// ========================================================================
	case ProviderOpenAI, ProviderMistral, ProviderMeta, ProviderXAI, ProviderLlamaCpp, ProviderGemini, ProviderDeepSeek, ProviderOpenRouter, ProviderQwen:
		config := &openaiemb.EmbeddingConfig{
			APIKey:  cfg.APIKey,
			BaseURL: cfg.BaseURL,
			Model:   cfg.ModelName,
		}
		emb, err := openaiemb.NewEmbedder(ctx, config)
		if err != nil {
			return nil, fmt.Errorf("Failed to create openai-compatible embedder for %s: %w", providerType, err)
		}
		return emb, nil
	// ========================================================================
	// 2. Ollama
	// ========================================================================
	case ProviderOllama:
		config := &ollamaemb.EmbeddingConfig{
			BaseURL: cfg.BaseURL,
			Model:   cfg.ModelName,
		}
		emb, err := ollamaemb.NewEmbedder(ctx, config)
		if err != nil {
			return nil, fmt.Errorf("Failed to create ollama embedder: %w", err)
		}
		return emb, nil
	// ========================================================================
	// 3. Unsupported
	// ========================================================================
	case ProviderAnthropic:
		return nil, fmt.Errorf("Anthropic does not support embeddings via this factory")
	default:
		return nil, fmt.Errorf("Unsupported embedding provider type: %s", cfg.Type)
	}
}

// IsValidProviderType checks if the given provider type is supported.
func IsValidProviderType(pType ProviderType) bool {
	switch pType {
	case ProviderOpenAI, ProviderMistral, ProviderMeta, ProviderXAI, ProviderLlamaCpp, ProviderGemini, ProviderAnthropic, ProviderOllama, ProviderDeepSeek, ProviderOpenRouter, ProviderQwen:
		return true
	default:
		return false
	}
}
