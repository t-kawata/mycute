// Package cuber は、Embedderの初期化関数を提供します。
// この関数は、OpenAI APIを使用してEmbedderを作成します。
// 注意: この関数は現在使用されていません。NewCuberServiceで直接初期化されます。
package cuber

import (
	"context"
	"os"

	"github.com/tmc/langchaingo/embeddings"
	"github.com/tmc/langchaingo/llms/openai"
)

// NewEmbedder は、OpenAI APIを使用して新しいEmbedderクライアントを初期化します。
// Bifrostプロキシ経由でOpenAI APIにアクセスします。
//
// 注意: この関数は現在使用されていません。
// NewCuberServiceで直接OpenAI LLMを初期化し、OpenAIEmbedderAdapterでラップしています。
//
// 引数:
//   - ctx: コンテキスト
//
// 返り値:
//   - embeddings.Embedder: Embedderインスタンス
//   - error: エラーが発生した場合
func NewEmbedder(ctx context.Context) (embeddings.Embedder, error) {
	// OpenAI LLMを初期化
	llm, err := openai.New(
		openai.WithBaseURL(os.Getenv("OPENAI_BASE_URL")), // BifrostプロキシのURL
		openai.WithToken(os.Getenv("OPENAI_API_KEY")),    // APIキー
		openai.WithModel("text-embedding-3-small"),       // Embeddingモデル
	)
	if err != nil {
		return nil, err
	}

	// langchaingoのEmbedderを作成
	return embeddings.NewEmbedder(llm)
}
