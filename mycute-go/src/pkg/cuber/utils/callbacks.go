package utils

import (
	"context"
	"fmt"
	"sync"

	"github.com/cloudwego/eino/callbacks"
	"github.com/cloudwego/eino/components/embedding"
	"github.com/cloudwego/eino/components/model"
	"github.com/cloudwego/eino/schema"
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
// このハンドラを callbacks.InitCallbacks(ctx, info, handler) で注入してください。
func (agg *TokenUsageAggregator) Handler() callbacks.Handler {
	return callbacks.NewHandlerBuilder().
		OnEndFn(func(ctx context.Context, info *callbacks.RunInfo, output callbacks.CallbackOutput) context.Context {
			agg.mu.Lock()
			defer agg.mu.Unlock()

			// コールバック出力からトークン使用量を取得
			var currentInput, currentOutput int64

			if info.Type == string(types.MODEL_TYPE_CHAT_COMPLETION) {
				// Chat Model の場合
				modelOutput := model.ConvCallbackOutput(output)
				if modelOutput != nil && modelOutput.TokenUsage != nil {
					currentInput = int64(modelOutput.TokenUsage.PromptTokens)
					currentOutput = int64(modelOutput.TokenUsage.CompletionTokens)
				}
			} else if info.Type == string(types.MODEL_TYPE_EMBEDDING) {
				// Embedding Model の場合
				embOutput := embedding.ConvCallbackOutput(output)
				if embOutput != nil && embOutput.TokenUsage != nil {
					currentInput = int64(embOutput.TokenUsage.PromptTokens)
					currentOutput = int64(embOutput.TokenUsage.CompletionTokens)
				}
			}

			if currentInput > 0 || currentOutput > 0 {
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
		}).
		OnErrorFn(func(ctx context.Context, info *callbacks.RunInfo, err error) context.Context {
			// 将来的にエラーログを集約する場合はここに追加
			return ctx
		}).
		Build()
}

// GenerateWithUsage は、Eino ChatModel を呼び出し、トークン使用量を返します。
// この関数は、Callbackを注入し、トークン使用量を集計します。
// 引数:
//   - ctx: コンテキスト
//   - llm: Eino ChatModel
//   - modelName: モデル名（トークン集計用）
//   - systemPrompt: システムプロンプト
//   - userPrompt: ユーザープロンプト
//
// 返り値:
//   - string: 生成されたテキスト
//   - types.TokenUsage: トークン使用量
//   - error: エラーが発生した場合
func GenerateWithUsage(ctx context.Context, llm model.ToolCallingChatModel, modelName string, systemPrompt string, userPrompt string) (string, types.TokenUsage, error) {
	agg := NewTokenUsageAggregator(modelName)

	// RunInfo を作成
	runInfo := &callbacks.RunInfo{
		Name: "ChatModel",
		Type: string(types.MODEL_TYPE_CHAT_COMPLETION),
	}

	// Callback の注入
	ctx = callbacks.InitCallbacks(ctx, runInfo, agg.Handler())

	msgs := []*schema.Message{
		schema.SystemMessage(systemPrompt),
		schema.UserMessage(userPrompt),
	}

	result, err := llm.Generate(ctx, msgs)
	if err != nil {
		return "", agg.TotalUsage, fmt.Errorf("eino generate error: %w", err)
	}

	content := ""
	if result != nil {
		content = result.Content
	}

	return content, agg.TotalUsage, nil
}
