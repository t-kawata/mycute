// Package cognee は、LLMクライアントの初期化関数を提供します。
// この関数は、Bifrostプロキシ経由でOpenAI APIにアクセスするLLMクライアントを作成します。
// 注意: この関数は現在使用されていません。NewCogneeServiceで直接初期化されます。
package cognee

import (
	"context"
	"os"

	"github.com/tmc/langchaingo/llms"
	"github.com/tmc/langchaingo/llms/openai"
)

// NewLLMClient は、Bifrostプロキシを経由してOpenAI APIにアクセスする新しいLLMクライアントを初期化します。
//
// 注意: この関数は現在使用されていません。
// NewCogneeServiceで直接OpenAI LLMを初期化しています。
//
// 引数:
//   - ctx: コンテキスト
//
// 返り値:
//   - llms.Model: LLMクライアント
//   - error: エラーが発生した場合
func NewLLMClient(ctx context.Context) (llms.Model, error) {
	// 環境変数からBifrost URLを取得
	baseURL := os.Getenv("OPENAI_BASE_URL")
	if baseURL == "" {
		// デフォルト値またはエラー処理
		// 実際のシナリオでは、厳密に必要な場合はエラーを返すべきです
		baseURL = "https://bifrost.example.com/v1"
	}

	// langchaingoのOpenAIプロバイダーを初期化
	// 認証には環境変数OPENAI_API_KEYを使用します
	llm, err := openai.New(
		openai.WithBaseURL(baseURL),                   // BifrostプロキシのURL
		openai.WithToken(os.Getenv("OPENAI_API_KEY")), // APIキー
		openai.WithModel("gpt-4o"),                    // デフォルトモデル（設定可能にすることも可能）
	)
	if err != nil {
		return nil, err
	}

	return llm, nil
}
