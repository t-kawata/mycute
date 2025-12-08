// Package search は、検索ツールとEmbedderアダプターを提供します。
// OpenAIEmbedderAdapterは、langchaingo/llms/openaiをstorage.Embedderインターフェースに適合させます。
package search

import (
	"context"
	"fmt"

	"github.com/tmc/langchaingo/llms/openai"
)

// OpenAIEmbedderAdapter は、OpenAI LLMをEmbedderインターフェースに適合させるアダプターです。
// langchaingoのOpenAI LLMをCuberのEmbedderインターフェースで使用できるようにします。
type OpenAIEmbedderAdapter struct {
	LLM *openai.LLM // OpenAI LLMクライアント
}

// NewOpenAIEmbedderAdapter は、新しいOpenAIEmbedderAdapterを作成します。
// 引数:
//   - llm: OpenAI LLMクライアント
//
// 返り値:
//   - *OpenAIEmbedderAdapter: 新しいアダプターインスタンス
func NewOpenAIEmbedderAdapter(llm *openai.LLM) *OpenAIEmbedderAdapter {
	return &OpenAIEmbedderAdapter{LLM: llm}
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
//   - error: エラーが発生した場合
func (a *OpenAIEmbedderAdapter) EmbedQuery(ctx context.Context, text string) ([]float32, error) {
	// OpenAI APIを呼び出してembeddingを生成
	embeddings, err := a.LLM.CreateEmbedding(ctx, []string{text})
	if err != nil {
		return nil, fmt.Errorf("failed to create embedding: %w", err)
	}
	// embeddingが返されない場合はエラー
	if len(embeddings) == 0 {
		return nil, fmt.Errorf("no embeddings returned")
	}
	// 最初のembeddingを返す
	return embeddings[0], nil
}
