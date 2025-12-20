# Phase-13: LangChainGo Removal & Eino Migration (Definitive Guide)

## 0. はじめに (Introduction)

本ドキュメントは、**Phase-13: LangChainGo Removal & Eino Migration** を「迷いなく」「確実に」「バグなく」遂行するための**完全詳細設計書**です。
本フェーズの完了条件は以下の通りです。

1.  **LangChainGoの完全除去**: `src/pkg/cuber` 配下から `github.com/tmc/langchaingo` の痕跡が1行たりとも残っていないこと。
2.  **Einoへの完全移行**: すべてのLLM呼び出し（Chat, Embedding）が CloudWeGo Eino フレームワークを通じて行われること。
3.  **トークン計算の正確性**: 外部ライブラリ（tiktoken）による推定ではなく、プロバイダーから返却される実際のUsage情報をCallback経由で取得していること。
4.  **マルチプロバイダー基盤**: OpenAI, Gemini, Claude, DeepSeek 等、将来的な拡張に耐えうるFactory構成となっていること。

> [!CRITICAL]
> **待機指示**
> 本ドキュメントの内容は、ユーザーの明示的な「実装開始」の指示があるまで、コードへの反映を行わないでください。

---

## 1. アーキテクチャと技術選定 (Architecture & Rationale)

### 1.1 Why Eino? (技術的妥当性)

*   **コンポーネント指向**: EinoはLLMアプリケーションを「Graph」や「Chain」として定義します。これは、将来的に「Cube」自体を配布可能な実行可能オブジェクト（バイナリ+設定）としてカプセル化する際の粒度として最適です。
*   **強力なCallbackシステム**: LangChainGoでは隠蔽されがちなAPIレスポンスの詳細（Raw Response, Usage, Latency）に対し、Einoは統一されたCallbackインターフェースを提供します。これにより、トークン課金管理の精度が飛躍的に向上します。
*   **CloudWeGoエコシステム**: 高性能RPCフレームワーク `Kitex` やHTTPフレームワーク `Hertz` との親和性が高く、将来的なマイクロサービス化や高トラフィック対応において有利です。

### 1.2 リファクタリング戦略

各コンポーネントを以下の戦略で移行します。

1.  **Provider Factory**: `providers` パッケージを新設し、プロバイダーごとの差異（OpenAI vs Gemini vs Others）を吸収する工場を集約します。
2.  **Shared Callback**: 各タスクでトークン取得コードが重複しないよう、`utils` に共通のCallback生成関数を配置します。
3.  **Task Refactoring**: 各タスク（`GraphExtraction`, `Summarization` etc.）の内部フィールドを `llms.Model` から `model.ChatModel` に変更し、メソッド呼び出しを書き換えます。

---

## 2. 依存関係の更新 (Dependencies)

実装開始直後に、以下の手順で依存関係を整理します。

```bash
# 1. Eino と主要拡張の導入
go get github.com/cloudwego/eino@latest
go get github.com/cloudwego/eino-ext/components/model/openai@latest
go get github.com/cloudwego/eino-ext/components/model/gemini@latest

# 2. その他、ツールチェーン互換性のため
go mod tidy
```

※ `github.com/tmc/langchaingo` の削除は、コード修正完了後の `go mod tidy` で自動的に行われます。手動で `go get ...@none` をする必要はありませんが、確認は必要です。

---

## 3. 実装詳細 (Implementation Details)

以下は、作成・修正する全ファイルの完全なソースコードです。

### 3.1 [NEW] `src/pkg/cuber/providers/factory.go`

プロバイダーの生成ロジックを集約します。

```go
package providers

import (
	"context"
	"fmt"
	"strings"

	"github.com/cloudwego/eino/components/embedding"
	"github.com/cloudwego/eino/components/model"
	"github.com/cloudwego/eino-ext/components/model/gemini"
	"github.com/cloudwego/eino-ext/components/model/openai"
)

// ProviderType はサポートするLLMプロバイダーの識別子です。
type ProviderType string

const (
	ProviderOpenAI    ProviderType = "openai"
	ProviderGemini    ProviderType = "gemini"
	ProviderGoogle    ProviderType = "google" // alias for gemini
	ProviderAzure     ProviderType = "azure"
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
func NewChatModel(ctx context.Context, cfg ProviderConfig) (model.ChatModel, error) {
	pType := ProviderType(strings.ToLower(string(cfg.Type)))

	switch pType {
	case ProviderOpenAI, ProviderAzure, ProviderAnthropic, ProviderMeta, ProviderMistral,
		ProviderDeepSeek, ProviderGroq, ProviderOllama, ProviderLocal:
		
		// OpenAI 互換クライアントを使用して初期化
		// BaseURL が空の場合は openai.NewChatModel 側でデフォルト(https://api.openai.com/v1)が使われます
		config := &openai.ChatModelConfig{
			APIKey:  cfg.APIKey,
			BaseURL: cfg.BaseURL,
			Model:   cfg.ModelName,
		}
		
		// 補足: Azureの場合、BaseURL等の形式が異なる場合があるため、将来的に分岐が必要になる可能性がありますが、
		// 現状の eino-ext/openai は Azure OpenAI にも(BaseURLとAPIKeyの調整で)対応可能です。
		
		chatModel, err := openai.NewChatModel(ctx, config)
		if err != nil {
			return nil, fmt.Errorf("failed to create openai-compatible chat model for %s: %w", pType, err)
		}
		return chatModel, nil

	case ProviderGemini, ProviderGoogle:
		chatModel, err := gemini.NewChatModel(ctx, &gemini.ChatModelConfig{
			APIKey: cfg.APIKey,
			Model:  cfg.ModelName,
		})
		if err != nil {
			return nil, fmt.Errorf("failed to create gemini chat model: %w", err)
		}
		return chatModel, nil

	default:
		return nil, fmt.Errorf("unsupported provider type: %s", cfg.Type)
	}
}

// NewEmbedder は指定された設定に基づいて Eino Embedder を生成します。
func NewEmbedder(ctx context.Context, cfg ProviderConfig) (embedding.Embedder, error) {
	pType := ProviderType(strings.ToLower(string(cfg.Type)))

	switch pType {
	case ProviderOpenAI, ProviderAzure, ProviderAnthropic, ProviderMeta, ProviderMistral,
		ProviderDeepSeek, ProviderGroq, ProviderOllama, ProviderLocal:
		
		emb, err := openai.NewEmbedder(ctx, &openai.EmbeddingConfig{
			APIKey:  cfg.APIKey,
			BaseURL: cfg.BaseURL,
			Model:   cfg.ModelName,
		})
		if err != nil {
			return nil, fmt.Errorf("failed to create openai-compatible embedder for %s: %w", pType, err)
		}
		return emb, nil

	case ProviderGemini, ProviderGoogle:
		// 注意: eino-ext/gemini が Embedder を提供していない場合のエラーハンドリング
		// 現時点では未実装と見なしてエラーを返します。
		return nil, fmt.Errorf("gemini embedding is not currently supported by the factory")

	default:
		return nil, fmt.Errorf("unsupported provider type for embedding: %s", cfg.Type)
	}
}
```

### 3.2 [NEW] `src/pkg/cuber/utils/callbacks.go`

全タスクで共通して使用する Callback 生成ロジックです。これを使わないと、各タスクに同じコードが散乱します。

```go
package utils

import (
	"context"
	"sync"

	"github.com/cloudwego/eino/callbacks"
	"github.com/t-kawata/mycute/pkg/cuber/types"
)

// TokenUsageAggregator は、複数のEino呼び出しにまたがってトークン使用量を集計するためのヘルパーです。
// スレッドセーフに実装されています。
type TokenUsageAggregator struct {
	TotalUsage types.TokenUsage
	mu         sync.Mutex
	ModelName  string // 集計時にモデル名をDetailsに記録する場合に使用
}

// NewTokenUsageAggregator は新しい集計器を作成します。
func NewTokenUsageAggregator(modelName string) *TokenUsageAggregator {
	return &TokenUsageAggregator{
		ModelName: modelName,
	}
}

// Handler は Eino の Callback ハンドラを生成して返します。
// このハンドラを callbacks.InitCallbacks(ctx, handler) で注入してください。
func (agg *TokenUsageAggregator) Handler() *callbacks.Handler {
	return &callbacks.Handler{
		OnEnd: func(ctx context.Context, info *callbacks.RunInfo, output callbacks.CallbackOutput) context.Context {
			agg.mu.Lock()
			defer agg.mu.Unlock()

			if output.TokenUsage != nil {
				// 今回の実行分のUsage
				currentInput := int64(output.TokenUsage.PromptTokens)
				currentOutput := int64(output.TokenUsage.CompletionTokens)

				// 合算
				agg.TotalUsage.InputTokens += currentInput
				agg.TotalUsage.OutputTokens += currentOutput

				// Details への記録
				if agg.TotalUsage.Details == nil {
					agg.TotalUsage.Details = make(map[string]types.TokenUsage)
				}
				
				// 既存のモデル詳細があれば加算、なければ新規作成
				modelKey := agg.ModelName
				if modelKey == "" {
					modelKey = "unknown_model"
				}
				
				detail := agg.TotalUsage.Details[modelKey]
				detail.InputTokens += currentInput
				detail.OutputTokens += currentOutput
				agg.TotalUsage.Details[modelKey] = detail
			}
			return ctx
		},
		OnError: func(ctx context.Context, info *callbacks.RunInfo, err error) context.Context {
			// 将来的にエラーログを集約する場合はここに追加
			return ctx
		},
	}
}
```

### 3.3 [MODIFY] `src/pkg/cuber/tools/query/eino_adapter.go` (置換)

`openai_adapter.go` を削除し、以下のアダプターを作成します。Storage インターフェースとのブリッジです。

```go
package query

import (
	"context"
	"fmt"

	"github.com/cloudwego/eino/callbacks"
	"github.com/cloudwego/eino/components/embedding"
	"github.com/t-kawata/mycute/pkg/cuber/types"
	"github.com/t-kawata/mycute/pkg/cuber/utils" // 上記で作成したパッケージ
)

// EinoEmbedderAdapter は Eino の Embedder を Cuber の Embedder インターフェースに適合させます。
type EinoEmbedderAdapter struct {
	embedder  embedding.Embedder
	modelName string
}

func NewEinoEmbedderAdapter(emb embedding.Embedder, modelName string) *EinoEmbedderAdapter {
	return &EinoEmbedderAdapter{
		embedder:  emb,
		modelName: modelName,
	}
}

func (a *EinoEmbedderAdapter) EmbedQuery(ctx context.Context, text string) ([]float32, types.TokenUsage, error) {
	// トークン集計器の作成
	agg := utils.NewTokenUsageAggregator(a.modelName)
	
	// Callback の注入
	ctx = callbacks.InitCallbacks(ctx, agg.Handler())

	// Embeddings の実行
	vectors, err := a.embedder.EmbedStrings(ctx, []string{text})
	if err != nil {
		return nil, agg.TotalUsage, fmt.Errorf("eino embed error: %w", err)
	}

	if len(vectors) == 0 {
		return nil, agg.TotalUsage, fmt.Errorf("no embeddings returned")
	}

	// 型変換 ([]float64 -> []float32)
	// Einoの仕様上、通常はfloat64で返却されます
	resultVector := make([]float32, len(vectors[0]))
	for i, v := range vectors[0] {
		resultVector[i] = float32(v)
	}

	return resultVector, agg.TotalUsage, nil
}
```

### 3.4 [MODIFY] `src/pkg/cuber/cuber.go`

`langchaingo` を排除し、Factoryを使用するように変更します。

```go
// ... imports
import (
	"github.com/t-kawata/mycute/pkg/cuber/providers"
	"github.com/t-kawata/mycute/pkg/cuber/tools/query"
	"github.com/cloudwego/eino/components/model"
    // langchaingo は削除
)

// 変更: LLMの型定義
type CuberService struct {
	// ...
	LLM        model.ChatModel // llms.Model から変更
	// ...
}

func NewCuberService(config CuberConfig) (*CuberService, error) {
    // ... 設定適用 ...

	// ========================================
	// 3. Embeddings LLM の初期化
	// ========================================
    // TODO: ProviderTypeはConfigから取得できるようにすべきだが、今回は"openai"固定とする
	ctx := context.Background()
	embConfig := providers.ProviderConfig{
		Type:      providers.ProviderOpenAI,
		APIKey:    config.EmbeddingsAPIKey,
		BaseURL:   config.EmbeddingsBaseURL,
		ModelName: config.EmbeddingsModel,
	}

	einoRawEmb, err := providers.NewEmbedder(ctx, embConfig)
	if err != nil {
		cleanupFunc()
		return nil, fmt.Errorf("failed to init eino embedder: %w", err)
	}
	embedder := query.NewEinoEmbedderAdapter(einoRawEmb, config.EmbeddingsModel)

	// ========================================
	// 4. Completion LLM の初期化
	// ========================================
	chatConfig := providers.ProviderConfig{
		Type:      providers.ProviderType("openai"), // 拡張性を考慮してConfig化推奨
		APIKey:    config.CompletionAPIKey,
		BaseURL:   config.CompletionBaseURL,
		ModelName: config.CompletionModel,
	}

	chatModel, err := providers.NewChatModel(ctx, chatConfig)
	if err != nil {
		cleanupFunc()
		return nil, fmt.Errorf("failed to init eino chat model: %w", err)
	}

    // ... (S3Client, Service作成) ...
    service := &CuberService{
        // ...
        LLM: chatModel, // 型一致を確認
        // ...
    }
    // ...
}
```

### 3.5 [MODIFY] 全タスクファイルの修正一覧

`langchaingo` を使用している全ファイルを修正対象とします。以下に修正パターンを示します。共通して、`llms.Model` から `model.ChatModel` への変更と、`GenerateContent` から `Generate` への変更、そして `utils.TokenUsageAggregator` の使用が必要です。

#### 対象ファイル:
1.  `src/pkg/cuber/tasks/graph/graph_extraction_task.go`
2.  `src/pkg/cuber/tasks/summarization/summarization_task.go`
3.  `src/pkg/cuber/tasks/metacognition/ignorance_manager.go`
4.  `src/pkg/cuber/tasks/metacognition/graph_refinement_task.go`
5.  `src/pkg/cuber/tasks/metacognition/self_reflection_task.go`
6.  `src/pkg/cuber/tasks/metacognition/crystallization_task.go`
7.  `src/pkg/cuber/tasks/memify/rule_extraction_task.go`
8.  `src/pkg/cuber/tools/query/graph_completion.go`

#### 修正パターン (例: GraphExtractionTask):

```go
package graph

import (
	"context"
	"fmt"

	"github.com/cloudwego/eino/callbacks"
	"github.com/cloudwego/eino/components/model" // chat model
	"github.com/cloudwego/eino/schema"           // message types
	"github.com/t-kawata/mycute/pkg/cuber/types"
	"github.com/t-kawata/mycute/pkg/cuber/utils"
    // langchaingo 削除
)

type GraphExtractionTask struct {
	LLM       model.ChatModel // 変更
	ModelName string
	MemoryGroup string
}

func NewGraphExtractionTask(llm model.ChatModel, modelName string, memoryGroup string) *GraphExtractionTask {
	return &GraphExtractionTask{LLM: llm, ModelName: modelName, MemoryGroup: memoryGroup}
}

func (t *GraphExtractionTask) Run(ctx context.Context, chunks []*storage.Chunk) (*storage.GraphData, types.TokenUsage, error) {
	agg := utils.NewTokenUsageAggregator(t.ModelName) // 集計器作成

	// ... ループ処理 ...
	for _, chunk := range chunks {
		// Callback 注入
		callCtx := callbacks.InitCallbacks(ctx, agg.Handler())

		// メッセージ構築
		msgs := []*schema.Message{
			schema.SystemMessage("You are..."),
			schema.UserMessage(chunk.Text),
		}

		// 生成実行 (Eino)
		result, err := t.LLM.Generate(callCtx, msgs)
		if err != nil {
			return nil, agg.TotalUsage, err
		}
		
		content := result.Content
		// ... パース処理 ...
	}

	return graphData, agg.TotalUsage, nil
}
```

### 3.6 [REMOVE] 不要ファイルの削除

以下のファイルは `NewCuberService` での初期化に統合されたため、不要となります。削除してください。

*   `src/pkg/cuber/llm.go`
*   `src/pkg/cuber/embedding.go`
*   `src/pkg/cuber/tools/query/openai_adapter.go` (eino_adapter.go に置換)

また、`src/pkg/cuber/types/usage.go` は `langchaingo` 依存を含んでいなければ（`ExtractTokenUsage`関数など）、その関数部分は削除または修正し、構造体定義のみ残します。`ExtractTokenUsage` は Eino の Callback に置き換わるため不要になります。

---

## 4. テスト・検証計画 (Verification Plan)

### 4.1 動作検証ツール (`src/cmd/verify_eino/main.go`)

このツールは実装の最初期に作成し、Adapter単体の動作を確認するために使用します。

```go
package main

import (
	"context"
	"fmt"
	"log"
	"os"

	"github.com/joho/godotenv"
	"github.com/t-kawata/mycute/pkg/cuber/providers"
	"github.com/t-kawata/mycute/pkg/cuber/tools/query"
)

func main() {
	_ = godotenv.Load()
	apiKey := os.Getenv("OPENAI_API_KEY")
	if apiKey == "" {
		log.Fatal("OPENAI_API_KEY is not set")
	}

	ctx := context.Background()

	// 1. Embedder Test
	fmt.Println("--- Testing Eino Embedder ---")
	cfg := providers.ProviderConfig{
		Type:      providers.ProviderOpenAI,
		APIKey:    apiKey,
		ModelName: "text-embedding-3-small",
	}
	
	rawEmb, err := providers.NewEmbedder(ctx, cfg)
	if err != nil {
		log.Fatalf("Embedder Init Error: %v", err)
	}
	
	adapter := query.NewEinoEmbedderAdapter(rawEmb, cfg.ModelName)
	vec, usage, err := adapter.EmbedQuery(ctx, "Eino integration test.")
	if err != nil {
		log.Fatalf("EmbedQuery Error: %v", err)
	}
	fmt.Printf("Vector Dim: %d\n", len(vec))
	fmt.Printf("Usage: In=%d, Out=%d\n", usage.InputTokens, usage.OutputTokens)
	if usage.InputTokens == 0 {
		log.Fatal("FAIL: InputTokens is 0. Callback might be broken.")
	}

    // 2. ChatModel Test (Optional but recommended)
    // ...
    
	fmt.Println("SUCCESS: All Eino components verified.")
}
```

### 4.2 結合テスト (Integration Test)

サーバーを起動し、実際のワークフローを通します。

**手順:**

1.  **環境リセット & 起動**:
    ```bash
    make run ARGS="am"
    make swag
    make run ARGS="rt"
    ```

2.  **API シーケンス実行**:
    *   **Create**: `POST /v1/cubes/create` -> `{"name": "EinoTest"}`
    *   **Absorb**: `POST /v1/cubes/absorb` -> 日本語テキストを投入。ログを確認し、`GraphExtractionTask` が実行され、トークン消費がログに出力されていることを確認。
    *   **Query**: `POST /v1/cubes/query` -> 質問を投げる。`GraphCompletionTool` が動作し、回答が返ってくること。
    *   **Search**: `POST /v1/cubes/search` -> `{"name": "EinoTest"}` で検索し、レスポンスの `stats` フィールドに消費されたトークン量が正しく計上されていることを確認（0でないこと）。

---
**END OF DIRECTIVES**