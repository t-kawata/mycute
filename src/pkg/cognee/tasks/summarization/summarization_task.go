// Package summarization は、チャンクの要約を生成するタスクを提供します。
// このタスクは、LLMを使用してチャンクの要約を生成し、embeddingと共にKuzuDBに保存します。
package summarization

import (
	"context"
	"fmt"

	"mycute/pkg/cognee/pipeline"
	"mycute/pkg/cognee/prompts"
	"mycute/pkg/cognee/storage"

	"github.com/google/uuid"

	"github.com/tmc/langchaingo/llms"
)

// SummarizationTask は、要約生成タスクを表します。
// 各チャンクに対してLLMで要約を生成し、embeddingと共に保存します。
type SummarizationTask struct {
	VectorStorage storage.VectorStorage // ベクトルストレージ
	LLM           llms.Model            // テキスト生成LLM
	Embedder      storage.Embedder      // Embedder
	groupID       string                // グループID
}

// NewSummarizationTask は、新しいSummarizationTaskを作成します。
func NewSummarizationTask(vectorStorage storage.VectorStorage, llm llms.Model, embedder storage.Embedder, groupID string) *SummarizationTask {
	return &SummarizationTask{
		VectorStorage: vectorStorage,
		LLM:           llm,
		Embedder:      embedder,
		groupID:       groupID,
	}
}

var _ pipeline.Task = (*SummarizationTask)(nil)

// Run は、要約生成タスクを実行します。
// この関数は以下の処理を行います：
//  1. 各チャンクに対してLLMで要約を生成
//  2. 要約のembeddingを生成
//  3. KuzuDBに保存（コレクション: "TextSummary_text"）
func (t *SummarizationTask) Run(ctx context.Context, input any) (any, error) {
	output, ok := input.(*storage.CognifyOutput)
	if !ok {
		return nil, fmt.Errorf("expected *storage.CognifyOutput input, got %T", input)
	}

	fmt.Printf("Summarizing %d chunks...\n", len(output.Chunks))

	for _, chunk := range output.Chunks {
		// ========================================
		// 1. 要約プロンプトを生成
		// ========================================
		prompt := fmt.Sprintf(prompts.SummarizeContentPrompt, chunk.Text)

		// ========================================
		// 2. LLMを呼び出して要約を生成
		// ========================================
		resp, err := t.LLM.GenerateContent(ctx, []llms.MessageContent{
			llms.TextParts(llms.ChatMessageTypeHuman, prompt),
		})
		if err != nil {
			// エラーが発生しても他のチャンクの処理を続行
			fmt.Printf("Warning: failed to summarize chunk %s: %v\n", chunk.ID, err)
			continue
		}
		if len(resp.Choices) == 0 {
			continue
		}
		summaryText := resp.Choices[0].Content

		// ========================================
		// 3. 要約のembeddingを生成
		// ========================================
		embedding, err := t.Embedder.EmbedQuery(ctx, summaryText)
		if err != nil {
			fmt.Printf("Warning: failed to embed summary for chunk %s: %v\n", chunk.ID, err)
			continue
		}

		// ========================================
		// 4. 要約をKuzuDBに保存
		// ========================================
		// 決定論的なIDを生成（チャンクIDベース）
		namespace := uuid.MustParse("00000000-0000-0000-0000-000000000000")
		summaryID := uuid.NewSHA1(namespace, []byte(chunk.ID+"TextSummary")).String()

		// コレクション: "TextSummary_text"
		if err := t.VectorStorage.SaveEmbedding(ctx, "TextSummary_text", summaryID, summaryText, embedding, t.groupID); err != nil {
			return nil, fmt.Errorf("failed to save summary embedding: %w", err)
		}
	}

	return output, nil // 次のタスクのためにそのまま渡す
}
