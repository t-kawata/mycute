// Package summarization は、チャンクの要約を生成するタスクを提供します。
// このタスクは、LLMを使用してチャンクの要約を生成し、embeddingと共にKuzuDBに保存します。
package summarization

import (
	"context"
	"fmt"
	"strings"

	"github.com/cloudwego/eino/components/model"
	"github.com/t-kawata/mycute/pkg/cuber/pipeline"
	"github.com/t-kawata/mycute/pkg/cuber/prompts"
	"github.com/t-kawata/mycute/pkg/cuber/storage"
	"github.com/t-kawata/mycute/pkg/cuber/types"
	"github.com/t-kawata/mycute/pkg/cuber/utils"

	"github.com/google/uuid"
)

// SummarizationTask は、要約生成タスクを表します。
// 各チャンクに対してLLMで要約を生成し、embeddingと共に保存します。
type SummarizationTask struct {
	VectorStorage storage.VectorStorage      // ベクトルストレージ
	LLM           model.ToolCallingChatModel // テキスト生成LLM (Eino)
	Embedder      storage.Embedder           // Embedder
	memoryGroup   string                     // メモリーグループ
	ModelName     string                     // モデル名
}

// NewSummarizationTask は、新しいSummarizationTaskを作成します。
func NewSummarizationTask(vectorStorage storage.VectorStorage, llm model.ToolCallingChatModel, embedder storage.Embedder, memoryGroup string, modelName string) *SummarizationTask {
	if modelName == "" {
		modelName = "gpt-4"
	}
	return &SummarizationTask{
		VectorStorage: vectorStorage,
		LLM:           llm,
		Embedder:      embedder,
		memoryGroup:   memoryGroup,
		ModelName:     modelName,
	}
}

var _ pipeline.Task = (*SummarizationTask)(nil)

// Run は、要約生成タスクを実行します。
// この関数は以下の処理を行います：
//  1. 各チャンクに対してLLMで要約を生成
//  2. 要約のembeddingを生成
//  3. KuzuDBに保存
func (t *SummarizationTask) Run(ctx context.Context, input any) (any, types.TokenUsage, error) {
	var totalUsage types.TokenUsage
	output, ok := input.(*storage.CognifyOutput)
	if !ok {
		return nil, totalUsage, fmt.Errorf("Summarization: Expected *storage.CognifyOutput input, got %T", input)
	}
	fmt.Printf("Summarization: Summarizing %d chunks...\n", len(output.Chunks))
	for _, chunk := range output.Chunks {
		// ========================================
		// 1. プロンプトを作成しLLMを呼び出して要約を生成 (Eino)
		// ========================================
		prompt := fmt.Sprintf("Summarize the following text:\n\n%s", chunk.Text)
		summaryText, chunkUsage, err := utils.GenerateWithUsage(ctx, t.LLM, t.ModelName, prompts.SUMMARIZE_CONTENT_PROMPT, prompt)
		totalUsage.Add(chunkUsage)
		if err != nil {
			// エラーが発生しても他のチャンクの処理を続行
			fmt.Printf("Summarization: Warning: Failed to summarize chunk %s: %v\n", chunk.ID, err)
			continue
		}
		if summaryText == "" {
			continue
		}
		summaryText = strings.TrimSpace(summaryText)
		// ========================================
		// 2. 要約のembeddingを生成
		// ========================================
		embedding, u, err := t.Embedder.EmbedQuery(ctx, summaryText)
		totalUsage.Add(u)
		if err != nil {
			fmt.Printf("Summarization: Warning: Failed to embed summary for chunk %s: %v\n", chunk.ID, err)
			continue
		}
		// ========================================
		// 4. 要約をKuzuDBに保存
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
	}
	return output, totalUsage, nil // 次のタスクのためにそのまま渡す
}
