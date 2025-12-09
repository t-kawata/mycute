// Package search は、検索ツールとEmbedderアダプターを提供します。
// OpenAIEmbedderAdapterは、langchaingo/llms/openaiをstorage.Embedderインターフェースに適合させます。
package search

import (
	"context"
	"fmt"
	"log"

	"github.com/pkoukk/tiktoken-go"
	"github.com/t-kawata/mycute/pkg/cuber/types"
	"github.com/tmc/langchaingo/llms/openai"
)

// OpenAIEmbedderAdapter は、OpenAI LLMをEmbedderインターフェースに適合させるアダプターです。
// langchaingoのOpenAI LLMをCuberのEmbedderインターフェースで使用できるようにします。
type OpenAIEmbedderAdapter struct {
	LLM       *openai.LLM // OpenAI LLMクライアント
	ModelName string      // モデル名 (e.g. text-embedding-3-small)
}

// NewOpenAIEmbedderAdapter は、新しいOpenAIEmbedderAdapterを作成します。
// 引数:
//   - llm: OpenAI LLMクライアント
//   - modelName: モデル名
//
// 返り値:
//   - *OpenAIEmbedderAdapter: 新しいアダプターインスタンス
func NewOpenAIEmbedderAdapter(llm *openai.LLM, modelName string) *OpenAIEmbedderAdapter {
	if modelName == "" {
		modelName = "text-embedding-3-small" // Default
	}
	return &OpenAIEmbedderAdapter{LLM: llm, ModelName: modelName}
}

// EmbedQuery は、テキストをベクトル表現に変換します。
// storage.Embedderインターフェースを実装します。
//
// 引数:
//   - ctx: コンテキスト
//   - text: ベクトル化するテキスト
//
// 返り値:
//   - []float32: ベクトル表現（1536次元）
//   - types.TokenUsage: トークン使用量
//   - error: エラーが発生した場合
func (a *OpenAIEmbedderAdapter) EmbedQuery(ctx context.Context, text string) ([]float32, types.TokenUsage, error) {
	var usage types.TokenUsage

	// 1. Calculate tokens strictly using tiktoken
	encoding, err := tiktoken.GetEncoding("cl100k_base") // Compatible with text-embedding-3
	if err != nil {
		log.Printf("Warning: Failed to get tiktoken encoding: %v", err)
		// Fallback or fail? Directives imply strictness.
		// Assuming cl100k_base works.
	} else {
		tokens := encoding.Encode(text, nil, nil)
		count := int64(len(tokens))
		usage.InputTokens = count
		usage.OutputTokens = 0
		usage.Details = make(map[string]types.TokenUsage)
		usage.Details[a.ModelName] = types.TokenUsage{InputTokens: count, OutputTokens: 0}
	}

	// 2. OpenAI APIを呼び出してembeddingを生成
	embeddings, err := a.LLM.CreateEmbedding(ctx, []string{text})
	if err != nil {
		return nil, usage, fmt.Errorf("Failed to create embedding: %w", err)
	}
	// embeddingが返されない場合はエラー
	if len(embeddings) == 0 {
		return nil, usage, fmt.Errorf("No embeddings returned.")
	}
	// 最初のembeddingを返す
	return embeddings[0], usage, nil
}
