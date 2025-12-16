package providers

import (
	"context"
	"fmt"
	"strings"

	openaiemb "github.com/cloudwego/eino-ext/components/embedding/openai"
	openaimodel "github.com/cloudwego/eino-ext/components/model/openai"
	"github.com/cloudwego/eino/components/embedding"
	"github.com/cloudwego/eino/components/model"
)

// ProviderType はサポートするLLMプロバイダーの識別子です。
type ProviderType string

const (
	ProviderOpenAI ProviderType = "openai"
	ProviderGemini ProviderType = "gemini"
	ProviderGoogle ProviderType = "google" // alias for gemini
	ProviderAzure  ProviderType = "azure"
	// 以下のプロバイダーは OpenAI 互換 API として扱います
	ProviderAnthropic ProviderType = "anthropic"
	ProviderMeta      ProviderType = "meta"
	ProviderMistral   ProviderType = "mistral"
	ProviderDeepSeek  ProviderType = "deepseek"
	ProviderGroq      ProviderType = "groq"
	ProviderOllama    ProviderType = "ollama"
	ProviderLocal     ProviderType = "local"
)

// ProviderConfig はプロバイダー接続に必要な設定情報です。
type ProviderConfig struct {
	Type      ProviderType
	APIKey    string
	BaseURL   string // OpenAI互換プロバイダーの場合は必須
	ModelName string
}

// NewChatModel は指定された設定に基づいて Eino ChatModel を生成します。
func NewChatModel(ctx context.Context, cfg ProviderConfig) (model.ToolCallingChatModel, error) {
	providerType := ProviderType(strings.ToLower(string(cfg.Type)))
	switch providerType {
	case ProviderOpenAI, ProviderAzure, ProviderAnthropic, ProviderMeta, ProviderMistral,
		ProviderDeepSeek, ProviderGroq, ProviderOllama, ProviderLocal:
		// OpenAI 互換クライアントを使用して初期化
		// BaseURL が空の場合は openaimodel.NewChatModel 側でデフォルト(https://api.openai.com/v1)が使われる
		config := &openaimodel.ChatModelConfig{
			APIKey:  cfg.APIKey,
			BaseURL: cfg.BaseURL,
			Model:   cfg.ModelName,
		}
		chatModel, err := openaimodel.NewChatModel(ctx, config)
		if err != nil {
			return nil, fmt.Errorf("Failed to create openai-compatible chat model for %s: %w", providerType, err)
		}
		return chatModel, nil
	case ProviderGemini, ProviderGoogle:
		// Gemini は現時点で eino-ext に含まれていない可能性があるため、エラーを返す
		return nil, fmt.Errorf("Gemini chat model is not currently supported by the factory")
	default:
		return nil, fmt.Errorf("Unsupported provider type: %s", cfg.Type)
	}
}

// NewEmbedder は指定された設定に基づいて Eino Embedder を生成。
func NewEmbedder(ctx context.Context, cfg ProviderConfig) (embedding.Embedder, error) {
	providerType := ProviderType(strings.ToLower(string(cfg.Type)))
	switch providerType {
	case ProviderOpenAI, ProviderAzure, ProviderAnthropic, ProviderMeta, ProviderMistral,
		ProviderDeepSeek, ProviderGroq, ProviderOllama, ProviderLocal:
		emb, err := openaiemb.NewEmbedder(ctx, &openaiemb.EmbeddingConfig{
			APIKey:  cfg.APIKey,
			BaseURL: cfg.BaseURL,
			Model:   cfg.ModelName,
		})
		if err != nil {
			return nil, fmt.Errorf("Failed to create openai-compatible embedder for %s: %w", providerType, err)
		}
		return emb, nil
	case ProviderGemini, ProviderGoogle:
		return nil, fmt.Errorf("Gemini embedding is not currently supported by the factory")
	default:
		return nil, fmt.Errorf("Unsupported provider type for embedding: %s", cfg.Type)
	}
}
