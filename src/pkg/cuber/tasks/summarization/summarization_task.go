// Package summarization は、チャンクの要約を生成するタスクを提供します。
// このタスクは、LLMを使用してチャンクの要約を生成し、embeddingと共にLadybugDBに保存します。
package summarization

import (
	"context"
	"fmt"
	"strings"

	"go.uber.org/zap"

	"github.com/cloudwego/eino/components/model"
	"github.com/t-kawata/mycute/pkg/cuber/pipeline"
	"github.com/t-kawata/mycute/pkg/cuber/prompts"
	"github.com/t-kawata/mycute/pkg/cuber/storage"
	"github.com/t-kawata/mycute/pkg/cuber/types"
	"github.com/t-kawata/mycute/pkg/cuber/utils"

	"github.com/google/uuid"
	"github.com/t-kawata/mycute/lib/eventbus"
	"github.com/t-kawata/mycute/pkg/cuber/event"
)

// SummarizationTask は、要約生成タスクを表します。
// 各チャンクに対してLLMで要約を生成し、embeddingと共に保存します。
type SummarizationTask struct {
	VectorStorage storage.VectorStorage      // ベクトルストレージ
	LLM           model.ToolCallingChatModel // テキスト生成LLM (Eino)
	Embedder      storage.Embedder           // Embedder
	memoryGroup   string                     // メモリーグループ
	ModelName     string                     // モデル名
	Logger        *zap.Logger
	EventBus      *eventbus.EventBus
	IsEn          bool // true=English output, false=Japanese output
}

// NewSummarizationTask は、新しいSummarizationTaskを作成します。
func NewSummarizationTask(vectorStorage storage.VectorStorage, llm model.ToolCallingChatModel, embedder storage.Embedder, memoryGroup string, modelName string, l *zap.Logger, eb *eventbus.EventBus, isEn bool) *SummarizationTask {
	if modelName == "" {
		modelName = "gpt-4"
	}
	return &SummarizationTask{
		VectorStorage: vectorStorage,
		LLM:           llm,
		Embedder:      embedder,
		memoryGroup:   memoryGroup,
		ModelName:     modelName,
		Logger:        l,
		EventBus:      eb,
		IsEn:          isEn,
	}
}

var _ pipeline.Task = (*SummarizationTask)(nil)

// Run は、要約生成タスクを実行します。
// この関数は以下の処理を行います：
//  1. 各チャンクに対してLLMで要約を生成
//  2. 要約のembeddingを生成
//  3. LadybugDBに保存
func (t *SummarizationTask) Run(ctx context.Context, input any) (any, types.TokenUsage, error) {
	var totalUsage types.TokenUsage
	output, ok := input.(*storage.CognifyOutput)
	if !ok {
		return nil, totalUsage, fmt.Errorf("Summarization: Expected *storage.CognifyOutput input, got %T", input)
	}
	utils.LogInfo(t.Logger, "SummarizationTask: Starting", zap.Int("chunks", len(output.Chunks)))

	summariesCreated := 0

	for i, chunk := range output.Chunks {
		// ========================================
		// 1. プロンプトを作成しLLMを呼び出して要約を生成 (Eino)
		// ========================================
		prompt := fmt.Sprintf("Summarize the following text:\n\n%s", chunk.Text)
		utils.LogDebug(t.Logger, "SummarizationTask: Summarizing chunk", zap.String("chunk_id", chunk.ID), zap.Int("text_len", len(chunk.Text)))

		// Emit Summarization Request Start
		eventbus.Emit(t.EventBus, string(event.EVENT_ABSORB_SUMMARIZATION_REQ_START), event.AbsorbSummarizationReqStartPayload{
			BasePayload: event.NewBasePayload(t.memoryGroup),
			ChunkID:     chunk.ID,
			ChunkNum:    i + 1,
		})

		// Select prompt based on language mode
		var promptTemplate string
		if t.IsEn {
			promptTemplate = prompts.SUMMARIZE_CONTENT_EN_PROMPT
		} else {
			promptTemplate = prompts.SUMMARIZE_CONTENT_JA_PROMPT
		}
		summaryText, chunkUsage, err := utils.GenerateWithUsage(ctx, t.LLM, t.ModelName, promptTemplate, prompt)

		// Emit Summarization Request End
		eventbus.Emit(t.EventBus, string(event.EVENT_ABSORB_SUMMARIZATION_REQ_END), event.AbsorbSummarizationReqEndPayload{
			BasePayload: event.NewBasePayload(t.memoryGroup),
			ChunkID:     chunk.ID,
			ChunkNum:    i + 1,
			SummaryText: strings.TrimSpace(summaryText),
		})

		totalUsage.Add(chunkUsage)
		if err != nil {
			eventbus.Emit(t.EventBus, string(event.EVENT_ABSORB_ERROR), event.AbsorbErrorPayload{
				BasePayload: event.NewBasePayload(t.memoryGroup),
				Error:       fmt.Errorf("Summarization failed to query chunks: %w", err),
			})
			// エラーが発生しても他のチャンクの処理を続行
			utils.LogWarn(t.Logger, "SummarizationTask: Failed to summarize chunk", zap.String("chunk_id", chunk.ID), zap.Error(err))
			continue
		}
		if summaryText == "" {
			continue
		}
		summaryText = strings.TrimSpace(summaryText)
		// 要約テキストをVector用に正規化
		summaryText = utils.NormalizeForVector(summaryText)
		utils.LogDebug(t.Logger, "SummarizationTask: Generated summary", zap.String("chunk_id", chunk.ID), zap.String("summary_preview", truncate(summaryText, 50)))
		// ========================================
		// 2. 要約のembeddingを生成
		// ========================================
		embedding, u, err := t.Embedder.EmbedQuery(ctx, summaryText)
		totalUsage.Add(u)
		if err != nil {
			utils.LogWarn(t.Logger, "SummarizationTask: Failed to embed summary", zap.String("chunk_id", chunk.ID), zap.Error(err))
			continue
		}
		// ========================================
		// 4. 要約をLadybugDBに保存
		// ========================================
		// 決定論的なIDを生成（チャンクIDベース）
		namespace := uuid.MustParse("00000000-0000-0000-0000-000000000000")
		summaryID := uuid.NewSHA1(namespace, []byte(chunk.ID+"TextSummary")).String()
		// ========================================
		// 5. embeddingを保存
		// ========================================
		if err := t.VectorStorage.SaveEmbedding(ctx, types.TABLE_NAME_SUMMARY, summaryID, summaryText, embedding, t.memoryGroup); err != nil {
			return nil, totalUsage, fmt.Errorf("Summarization: Failed to save summary embedding: %w", err)
		}

		// Emit Summarization Save Start (Included embedding generation implicitly in "chunk processing" but Save is here)
		eventbus.Emit(t.EventBus, string(event.EVENT_ABSORB_SUMMARIZATION_SAVE_START), event.AbsorbSummarizationSaveStartPayload{
			BasePayload: event.NewBasePayload(t.memoryGroup),
			ChunkID:     chunk.ID,
			ChunkNum:    i + 1,
		})

		// ... actually saved above. Let's readjust logic or emit start/end around save.
		// Since SaveEmbedding is the save op.

		// Emit Summarization Save End
		eventbus.Emit(t.EventBus, string(event.EVENT_ABSORB_SUMMARIZATION_SAVE_END), event.AbsorbSummarizationSaveEndPayload{
			BasePayload: event.NewBasePayload(t.memoryGroup),
			ChunkID:     chunk.ID,
			ChunkNum:    i + 1,
		})

		utils.LogDebug(t.Logger, "SummarizationTask: Saved summary", zap.String("id", summaryID))
		summariesCreated++

	}

	// Emit Summarization End
	eventbus.Emit(t.EventBus, string(event.EVENT_ABSORB_SUMMARIZATION_END), event.AbsorbSummarizationEndPayload{
		BasePayload:      event.NewBasePayload(t.memoryGroup),
		SummariesCreated: summariesCreated,
	})

	return output, totalUsage, nil // 次のタスクのためにそのまま渡す
}

func truncate(s string, maxLen int) string {
	if len(s) <= maxLen {
		return s
	}
	return s[:maxLen] + "..."
}
