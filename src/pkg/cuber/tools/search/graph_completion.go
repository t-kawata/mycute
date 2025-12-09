// Package search は、グラフベースの検索ツールを提供します。
// GraphCompletionToolは、ベクトル検索とグラフトラバーサルを組み合わせて
// 質問に対する回答を生成します。
package search

import (
	"context"
	"fmt"
	"strings"

	"github.com/t-kawata/mycute/pkg/cuber/prompts"
	"github.com/t-kawata/mycute/pkg/cuber/storage"
	"github.com/t-kawata/mycute/pkg/cuber/types"

	"github.com/tmc/langchaingo/llms"
)

// GraphCompletionTool は、グラフベースの検索ツールです。
// このツールは、以下の検索タイプをサポートします：
//   - SUMMARIES: 要約のみを検索
//   - GRAPH_SUMMARY_COMPLETION: グラフを検索して要約を生成
//   - GRAPH_COMPLETION: グラフとチャンクを組み合わせて回答を生成（デフォルト）
type GraphCompletionTool struct {
	VectorStorage storage.VectorStorage // ベクトルストレージ（KuzuDB）
	GraphStorage  storage.GraphStorage  // グラフストレージ（KuzuDB）
	LLM           llms.Model            // テキスト生成LLM
	Embedder      storage.Embedder      // Embedder
	memoryGroup   string                // メモリーグループ（パーティション識別子）
	ModelName     string                // 使用するモデル名（トークン集計用）
}

// NewGraphCompletionTool は、新しいGraphCompletionToolを作成します。
// 引数:
//   - vectorStorage: ベクトルストレージ
//   - graphStorage: グラフストレージ
//   - llm: テキスト生成LLM
//   - embedder: Embedder
//   - memoryGroup: メモリーグループ
//   - modelName: 使用するモデル名
//
// 返り値:
//   - *GraphCompletionTool: 新しいGraphCompletionToolインスタンス
func NewGraphCompletionTool(vectorStorage storage.VectorStorage, graphStorage storage.GraphStorage, llm llms.Model, embedder storage.Embedder, memoryGroup string, modelName string) *GraphCompletionTool {
	return &GraphCompletionTool{
		VectorStorage: vectorStorage,
		GraphStorage:  graphStorage,
		LLM:           llm,
		Embedder:      embedder,
		memoryGroup:   memoryGroup,
		ModelName:     modelName,
	}
}

// Search は、指定されたタイプで検索を実行し、回答を生成します。
// 引数:
//   - ctx: コンテキスト
//   - query: 検索クエリ
//   - searchType: 検索タイプ
//
// 返り値:
//   - string: 検索結果（回答）
func (t *GraphCompletionTool) Search(ctx context.Context, query string, searchType SearchType) (string, types.TokenUsage, error) {
	var usage types.TokenUsage
	switch searchType {
	case SearchTypeSummaries:
		return t.searchSummaries(ctx, query)
	case SearchTypeGraphSummaryCompletion:
		return t.searchGraphSummaryCompletion(ctx, query)
	case SearchTypeGraphCompletion:
		return t.searchGraphCompletion(ctx, query)
	default:
		return "", usage, fmt.Errorf("GraphCompletionTool: Unknown search type: %s", searchType)
	}
}

// searchSummaries は、要約のみを検索して返します。
// この関数は以下の処理を行います：
//  1. クエリをベクトル化
//  2. "Summary_text"コレクションから類似する要約を検索
//  3. 要約のリストを返す
//
// 引数:
//   - ctx: コンテキスト
//   - query: 検索クエリ
//
// 返り値:
//   - string: 要約のリスト
//   - error: エラーが発生した場合
func (t *GraphCompletionTool) searchSummaries(ctx context.Context, query string) (string, types.TokenUsage, error) {
	var usage types.TokenUsage
	// クエリをベクトル化
	embedding, u, err := t.Embedder.EmbedQuery(ctx, query)
	usage.Add(u)
	if err != nil {
		return "", usage, fmt.Errorf("GraphCompletionTool: Failed to embed query: %w", err)
	}

	// "Summary_text"コレクションを検索
	results, err := t.VectorStorage.Search(ctx, "Summary_text", embedding, 5, t.memoryGroup)
	if err != nil {
		return "", usage, fmt.Errorf("GraphCompletionTool: Failed to search summaries: %w", err)
	}

	// 結果が見つからない場合
	if len(results) == 0 {
		return "No relevant summaries found.", usage, nil
	}

	// 要約のリストを構築
	var sb strings.Builder
	for _, res := range results {
		sb.WriteString("- " + res.Text + "\n")
	}

	return sb.String(), usage, nil
}

// searchGraphSummaryCompletion は、グラフ（要約）を検索して回答を生成します。
// この関数は以下の処理を行います：
//  1. ノードを検索
//  2. グラフトラバーサルでトリプレットを取得
//  3. トリプレットをテキストに変換
//  4. LLMでグラフの要約を生成
//  5. 要約をコンテキストとして最終的な回答を生成
//
// 引数:
//   - ctx: コンテキスト
//   - query: 検索クエリ
//
// 返り値:
//   - string: 回答
//   - error: エラーが発生した場合
func (t *GraphCompletionTool) searchGraphSummaryCompletion(ctx context.Context, query string) (string, types.TokenUsage, error) {
	var usage types.TokenUsage
	// 1. 関連するSummaryを検索
	summaries, u, err := t.searchSummaries(ctx, query)
	usage.Add(u)
	if err != nil {
		return "", usage, fmt.Errorf("GraphCompletionTool: Failed to search summaries: %w", err)
	}

	// ========================================
	// 1. ノードを検索
	// ========================================
	queryVector, u, err := t.Embedder.EmbedQuery(ctx, query)
	usage.Add(u)
	if err != nil {
		return "", usage, fmt.Errorf("GraphCompletionTool: Failed to embed query: %w", err)
	}

	// "Entity_name"コレクションからノードを検索
	nodeResults, err := t.VectorStorage.Search(ctx, "Entity_name", queryVector, 5, t.memoryGroup)
	if err != nil {
		return "", usage, fmt.Errorf("GraphCompletionTool: Node search failed: %w", err)
	}

	// ノードIDを収集（重複を除く）
	var nodeIDs []string
	uniqueNodes := make(map[string]bool)

	for _, res := range nodeResults {
		if !uniqueNodes[res.ID] {
			nodeIDs = append(nodeIDs, res.ID)
			uniqueNodes[res.ID] = true
		}
	}

	// ========================================
	// 2. グラフトラバーサル
	// ========================================
	triplets, err := t.GraphStorage.GetTriplets(ctx, nodeIDs, t.memoryGroup)
	if err != nil {
		return "", usage, fmt.Errorf("GraphCompletionTool: Graph traversal failed: %w", err)
	}

	// トリプレットが見つからない場合
	if len(triplets) == 0 && summaries == "No relevant summaries found." {
		return "No relevant graph connections or summaries found to answer the question.", usage, nil
	}

	// ========================================
	// 3. トリプレットをテキストに変換
	// ========================================
	var graphText strings.Builder
	for _, triplet := range triplets {
		// エッジの方向と関係を記述
		edgeText := fmt.Sprintf("- %s (%s) %s -> %s (%s)",
			getName(triplet.Source.Properties), triplet.Source.ID,
			triplet.Edge.Type,
			getName(triplet.Target.Properties), triplet.Target.ID)
		graphText.WriteString(edgeText + "\n")
	}

	// ========================================
	// 4. グラフの要約を生成
	// ========================================
	summarizePrompt := fmt.Sprintf(prompts.SummarizeSearchResultsPrompt, query, graphText.String())

	summaryResp, err := t.LLM.GenerateContent(ctx, []llms.MessageContent{
		llms.TextParts(llms.ChatMessageTypeHuman, summarizePrompt),
	})
	if err != nil {
		return "", usage, fmt.Errorf("GraphCompletionTool: Failed to generate summary of graph: %w", err)
	}
	if len(summaryResp.Choices) == 0 {
		return "", usage, fmt.Errorf("GraphCompletionTool: Empty summary response from LLM")
	}
	summaryContext := summaryResp.Choices[0].Content
	if len(summaryResp.Choices) > 0 {
		if u, err := extractTokenUsage(summaryResp.Choices[0].GenerationInfo, t.ModelName); err != nil {
			return "", usage, fmt.Errorf("GraphCompletionTool: Failed to extract token usage from summary: %w", err)
		} else {
			usage.Add(u)
		}
	}

	// ========================================
	// 5. 要約をコンテキストとして最終的な回答を生成
	// ========================================
	// GraphCompletionと同じプロンプトを再利用
	finalUserPrompt := fmt.Sprintf(prompts.GraphContextForQuestionPrompt, query, summaryContext+"\n\n"+summaries)

	finalResp, err := t.LLM.GenerateContent(ctx, []llms.MessageContent{
		llms.TextParts(llms.ChatMessageTypeSystem, prompts.AnswerSimpleQuestionPrompt),
		llms.TextParts(llms.ChatMessageTypeHuman, finalUserPrompt),
	})
	if err != nil {
		return "", usage, fmt.Errorf("GraphCompletionTool: Failed to generate final answer: %w", err)
	}

	if len(finalResp.Choices) == 0 {
		return "", usage, fmt.Errorf("GraphCompletionTool: No final answer generated")
	}
	if len(finalResp.Choices) > 0 {
		if u, err := extractTokenUsage(finalResp.Choices[0].GenerationInfo, t.ModelName); err != nil {
			return "", usage, fmt.Errorf("GraphCompletionTool: Failed to extract token usage from final response: %w", err)
		} else {
			usage.Add(u)
		}
	}

	return finalResp.Choices[0].Content, usage, nil
}

// searchGraphCompletion は、グラフとチャンクを組み合わせて回答を生成します（デフォルト）。
// この関数は以下の処理を行います：
//  1. ベクトル検索（チャンクとノード）
//  2. グラフトラバーサル
//  3. コンテキストを構築（チャンク + グラフ）
//  4. LLMで回答を生成
//
// 引数:
//   - ctx: コンテキスト
//   - query: 検索クエリ
//
// 返り値:
//   - string: 回答
//   - error: エラーが発生した場合
func (t *GraphCompletionTool) searchGraphCompletion(ctx context.Context, query string) (string, types.TokenUsage, error) {
	var usage types.TokenUsage
	// クエリをベクトル化
	queryVector, u, err := t.Embedder.EmbedQuery(ctx, query)
	usage.Add(u)
	if err != nil {
		return "", usage, fmt.Errorf("GraphCompletionTool: Failed to embed query: %w", err)
	}

	// ========================================
	// 1. ベクトル検索（並列実行が推奨されますが、ここでは順次実行）
	// ========================================

	// A. チャンクを検索
	chunkResults, err := t.VectorStorage.Search(ctx, "DocumentChunk_text", queryVector, 5, t.memoryGroup)
	if err != nil {
		return "", usage, fmt.Errorf("GraphCompletionTool: Chunk search failed: %w", err)
	}

	// B. ノードを検索
	nodeResults, err := t.VectorStorage.Search(ctx, "Entity_name", queryVector, 5, t.memoryGroup)
	if err != nil {
		return "", usage, fmt.Errorf("GraphCompletionTool: Node search failed: %w", err)
	}

	// ========================================
	// 2. グラフトラバーサル
	// ========================================
	// ノードIDを収集（重複を除く）
	var nodeIDs []string
	uniqueNodes := make(map[string]bool)

	for _, res := range nodeResults {
		if !uniqueNodes[res.ID] {
			nodeIDs = append(nodeIDs, res.ID)
			uniqueNodes[res.ID] = true
		}
	}

	// グラフストレージからトリプレットを取得
	triplets, err := t.GraphStorage.GetTriplets(ctx, nodeIDs, t.memoryGroup)
	if err != nil {
		return "", usage, fmt.Errorf("GraphCompletionTool: Graph traversal failed: %w", err)
	}

	// ========================================
	// 3. コンテキストを構築
	// ========================================
	var contextBuilder strings.Builder

	// チャンクのコンテキスト
	contextBuilder.WriteString("### Relevant Text Chunks:\n")
	if len(chunkResults) == 0 {
		contextBuilder.WriteString("No relevant text chunks found.\n")
	}
	for _, res := range chunkResults {
		contextBuilder.WriteString("- " + res.Text + "\n")
	}

	// グラフのコンテキスト
	contextBuilder.WriteString("\n### Knowledge Graph Connections:\n")
	if len(triplets) == 0 {
		contextBuilder.WriteString("No relevant graph connections found.\n")
	}
	for _, triplet := range triplets {
		// フォーマット: Source --[Type]--> Target
		sourceName := getName(triplet.Source.Properties)
		targetName := getName(triplet.Target.Properties)

		contextBuilder.WriteString(fmt.Sprintf("- %s --[%s]--> %s\n",
			sourceName,
			triplet.Edge.Type,
			targetName))
	}

	// ========================================
	// 4. 回答を生成
	// ========================================
	// Python実装と同じように、SystemメッセージとUserメッセージを使用

	// コンテキストを含むUserプロンプトを作成
	userPrompt := fmt.Sprintf(prompts.GraphContextForQuestionPrompt, query, contextBuilder.String())

	// System（動作指示）とUser（クエリ+コンテキスト）メッセージを使用してコンテンツを生成
	resp, err := t.LLM.GenerateContent(ctx, []llms.MessageContent{
		llms.TextParts(llms.ChatMessageTypeSystem, prompts.AnswerSimpleQuestionPrompt),
		llms.TextParts(llms.ChatMessageTypeHuman, userPrompt),
	})
	if err != nil {
		return "", usage, fmt.Errorf("GraphCompletionTool: Failed to generate completion: %w", err)
	}

	// Extract Usage
	if len(resp.Choices) > 0 {
		if u, err := extractTokenUsage(resp.Choices[0].GenerationInfo, t.ModelName); err != nil {
			return "", usage, fmt.Errorf("GraphCompletionTool: Failed to extract token usage: %w", err)
		} else {
			usage.Add(u)
		}
	}

	if len(resp.Choices) == 0 {
		return "", usage, fmt.Errorf("GraphCompletionTool: No response from LLM")
	}

	return resp.Choices[0].Content, usage, nil
}

// getName は、ノードのプロパティから名前を安全に取得します。
// "name"プロパティが存在しない場合は、ノードIDをフォールバックとして使用します。
//
// 引数:
//   - props: ノードのプロパティ
//
// 返り値:
//   - string: ノードの名前またはID
func getName(props map[string]any) string {
	if name, ok := props["name"].(string); ok {
		return name
	}
	return "unknown"
}

func extractTokenUsage(info map[string]any, modelName string) (types.TokenUsage, error) {
	var u types.TokenUsage
	if info == nil {
		return u, fmt.Errorf("extractTokenUsage: GenerationInfo is nil")
	}
	getInt := func(k string) int64 {
		if v, ok := info[k]; ok {
			if f, ok := v.(float64); ok {
				return int64(f)
			}
			if i, ok := v.(int); ok {
				return int64(i)
			}
			if i, ok := v.(int64); ok {
				return i
			}
		}
		return 0
	}
	u.InputTokens = getInt("prompt_tokens")
	u.OutputTokens = getInt("completion_tokens")
	if u.InputTokens == 0 && u.OutputTokens == 0 {
		return u, fmt.Errorf("extractTokenUsage: Token counts are zero (input=%d, output=%d)", u.InputTokens, u.OutputTokens)
	}
	if modelName != "" {
		u.Details = map[string]types.TokenUsage{
			modelName: {
				InputTokens:  u.InputTokens,
				OutputTokens: u.OutputTokens,
			},
		}
	}
	return u, nil
}
