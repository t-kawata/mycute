# Phase-14: Multi-Provider Support Expansion

## 0. はじめに (Introduction)

本ドキュメントは、**Phase-14: Multi-Provider Support Expansion** の詳細設計書です。
Phase-13 で構築した `src/pkg/cuber/providers/factory.go` を拡張し、下記のプロバイダーリストを完全にサポートすることを目的とします。

本フェーズでは、「**Eino公式の専用実装がある場合はそれを優先利用し、ない場合はOpenAI互換実装を利用する**」という方針を徹底します。

## 1. 対応対象プロバイダーと実装方針

各プロバイダーの実装方針は、Einoの対応状況に基づき以下のように決定されました。

| プロバイダー | Chat Model 実装 | Embedding 実装 | 備考 |
| :--- | :--- | :--- | :--- |
| **OpenAI** | `model/openai` | `embedding/openai` | |
| **Anthropic** | `model/claude` | (未対応) | EmbeddingAPIなし |
| **Gemini** | `model/gemini` | `embedding/openai` (互換) | Chatは専用、EmbはOpenAI互換EP |
| **Ollama** | `model/ollama` | `embedding/ollama` | |
| **DeepSeek** | `model/deepseek` | `embedding/openai` (互換) | Chatは専用、Embは互換EP |
| **OpenRouter** | `model/openrouter` | `embedding/openai` (互換) | Chatは専用、Embは互換EP |
| **Qwen** | `model/qwen` | `embedding/openai` (互換) | Chatは専用、Embは互換EP |
| **Mistral** | `model/openai` (互換) | `embedding/openai` (互換) | |
| **Meta** | `model/openai` (互換) | `embedding/openai` (互換) | |
| **xAI** | `model/openai` (互換) | `embedding/openai` (互換) | |
| **llama.cpp** | `model/openai` (互換) | `embedding/openai` (互換) | |

---

## 2. 依存関係の更新 (Dependencies)

以下のコマンドでEinoの追加コンポーネントを導入します（実施済み）。

```bash
# Anthropic (Claude)
go get github.com/cloudwego/eino-ext/components/model/claude@latest

# Ollama
go get github.com/cloudwego/eino-ext/components/model/ollama@latest
go get github.com/cloudwego/eino-ext/components/embedding/ollama@latest

# DeepSeek
go get github.com/cloudwego/eino-ext/components/model/deepseek@latest

# OpenRouter
go get github.com/cloudwego/eino-ext/components/model/openrouter@latest

# Qwen
go get github.com/cloudwego/eino-ext/components/model/qwen@latest

# 既存依存（念のため）
go get github.com/cloudwego/eino-ext/components/model/openai@latest
go get github.com/cloudwego/eino-ext/components/model/gemini@latest
go get github.com/cloudwego/eino-ext/components/embedding/openai@latest
```

---

## 3. 実装詳細 (Implementation Details)

### 3.1 [MODIFY] `src/pkg/cuber/providers/factory.go`

`factory.go` を全面的に改修し、追加された専用プロバイダーも網羅します。

```go
package providers

import (
	"context"
	"fmt"
	"strings"

	// Eino Core
	"github.com/cloudwego/eino/components/embedding"
	"github.com/cloudwego/eino/components/model"

	// Eino Extensions - Chat Models
	openaimodel "github.com/cloudwego/eino-ext/components/model/openai"
	geminimodel "github.com/cloudwego/eino-ext/components/model/gemini"
	claudemodel "github.com/cloudwego/eino-ext/components/model/claude"
	ollamamodel "github.com/cloudwego/eino-ext/components/model/ollama"
	deepseekmodel "github.com/cloudwego/eino-ext/components/model/deepseek"
	openroutermodel "github.com/cloudwego/eino-ext/components/model/openrouter"
	qwenmodel "github.com/cloudwego/eino-ext/components/model/qwen"

	// Eino Extensions - Embeddings
	openaiemb "github.com/cloudwego/eino-ext/components/embedding/openai"
	ollamaemb "github.com/cloudwego/eino-ext/components/embedding/ollama"
)

// ProviderType はサポートするLLMプロバイダーの識別子です。
type ProviderType string

const (
	// Dedicated Implementations
	ProviderOpenAI    ProviderType = "openai"
	ProviderGemini    ProviderType = "gemini"
	ProviderAnthropic ProviderType = "anthropic"
	ProviderOllama    ProviderType = "ollama"
	ProviderDeepSeek  ProviderType = "deepseek"
	ProviderOpenRouter ProviderType = "openrouter"
	ProviderQwen      ProviderType = "qwen"

	// OpenAI Compatible Implementations
	ProviderMistral    ProviderType = "mistral"
	ProviderMeta       ProviderType = "meta"
	ProviderXAI        ProviderType = "xai"
	ProviderLlamaCpp   ProviderType = "llamacpp"
)

// ProviderConfig はプロバイダー接続に必要な設定情報です。
type ProviderConfig struct {
	Type      ProviderType
	APIKey    string
	BaseURL   string // OpenAI互換プロバイダー、または特定の専用プロバイダーで必要な場合
	ModelName string
	MaxTokens int    // 生成する最大トークン数 (0の場合はデフォルト値またはプロバイダーのデフォルトが使用される)
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
	case ProviderOpenAI, ProviderMistral, ProviderMeta, ProviderXAI, ProviderLlamaCpp,
		ProviderGemini, ProviderAnthropic, ProviderOllama, ProviderDeepSeek,
		ProviderOpenRouter, ProviderQwen:
		return true
	default:
		return false
	}
}
```

---

## 4. 検証手順

各プロバイダー専用実装が導入されたため、特に新規追加された `DeepSeek`, `OpenRouter`, `Qwen` についても、API Keyが準備できれば接続確認を行いますが、当面はビルドが通ること、および既存のOpenAI/Ollama等が引き続き動作することを確認します。

**以上**
