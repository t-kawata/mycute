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
	groupID       string                // グループID（パーティション識別子）
}

// NewGraphCompletionTool は、新しいGraphCompletionToolを作成します。
// 引数:
//   - vectorStorage: ベクトルストレージ
//   - graphStorage: グラフストレージ
//   - llm: テキスト生成LLM
//   - embedder: Embedder
//   - groupID: グループID
//
// 返り値:
//   - *GraphCompletionTool: 新しいGraphCompletionToolインスタンス
func NewGraphCompletionTool(vectorStorage storage.VectorStorage, graphStorage storage.GraphStorage, llm llms.Model, embedder storage.Embedder, groupID string) *GraphCompletionTool {
	return &GraphCompletionTool{
		VectorStorage: vectorStorage,
		GraphStorage:  graphStorage,
		LLM:           llm,
		Embedder:      embedder,
		groupID:       groupID,
	}
}

// Search は、指定された検索タイプに応じて検索を実行します。
// 引数:
//   - ctx: コンテキスト
//   - query: 検索クエリ
//   - searchType: 検索タイプ
//
// 返り値:
//   - string: 検索結果（回答）
//   - error: エラーが発生した場合
func (t *GraphCompletionTool) Search(ctx context.Context, query string, searchType SearchType) (string, error) {
	switch searchType {
	case SearchTypeSummaries:
		return t.searchSummaries(ctx, query)
	case SearchTypeGraphSummaryCompletion:
		return t.searchGraphSummaryCompletion(ctx, query)
	case SearchTypeGraphCompletion:
		fallthrough
	default:
		return t.searchGraphCompletion(ctx, query)
	}
}

// searchSummaries は、要約のみを検索します。
// この関数は以下の処理を行います：
//  1. クエリをベクトル化
//  2. "TextSummary_text"コレクションから類似する要約を検索
//  3. 要約のリストを返す
//
// 引数:
//   - ctx: コンテキスト
//   - query: 検索クエリ
//
// 返り値:
//   - string: 要約のリスト
//   - error: エラーが発生した場合
func (t *GraphCompletionTool) searchSummaries(ctx context.Context, query string) (string, error) {
	// クエリをベクトル化
	queryVector, err := t.Embedder.EmbedQuery(ctx, query)
	if err != nil {
		return "", fmt.Errorf("failed to embed query: %w", err)
	}

	// "TextSummary_text"コレクションを検索
	results, err := t.VectorStorage.Search(ctx, "TextSummary_text", queryVector, 5, t.groupID)
	if err != nil {
		return "", fmt.Errorf("summary search failed: %w", err)
	}

	// 結果が見つからない場合
	if len(results) == 0 {
		return "No relevant summaries found.", nil
	}

	// 要約のリストを構築
	var sb strings.Builder
	for _, res := range results {
		sb.WriteString("- " + res.Text + "\n")
	}
	return sb.String(), nil
}

// searchGraphSummaryCompletion は、グラフを検索して要約を生成します。
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
func (t *GraphCompletionTool) searchGraphSummaryCompletion(ctx context.Context, query string) (string, error) {
	// ========================================
	// 1. ノードを検索
	// ========================================
	queryVector, err := t.Embedder.EmbedQuery(ctx, query)
	if err != nil {
		return "", fmt.Errorf("failed to embed query: %w", err)
	}

	// "Entity_name"コレクションからノードを検索
	nodeResults, err := t.VectorStorage.Search(ctx, "Entity_name", queryVector, 5, t.groupID)
	if err != nil {
		return "", fmt.Errorf("node search failed: %w", err)
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
	triplets, err := t.GraphStorage.GetTriplets(ctx, nodeIDs, t.groupID)
	if err != nil {
		return "", fmt.Errorf("graph traversal failed: %w", err)
	}

	// トリプレットが見つからない場合
	if len(triplets) == 0 {
		return "No relevant graph connections found to summarize.", nil
	}

	// ========================================
	// 3. トリプレットをテキストに変換
	// ========================================
	var graphText strings.Builder
	for _, triplet := range triplets {
		sourceName := getName(triplet.Source)
		targetName := getName(triplet.Target)
		graphText.WriteString(fmt.Sprintf("- %s --[%s]--> %s\n", sourceName, triplet.Edge.Type, targetName))
	}

	// ========================================
	// 4. グラフの要約を生成
	// ========================================
	summarizePrompt := fmt.Sprintf(prompts.SummarizeSearchResultsPrompt, query, graphText.String())

	summaryResp, err := t.LLM.GenerateContent(ctx, []llms.MessageContent{
		llms.TextParts(llms.ChatMessageTypeHuman, summarizePrompt),
	})
	if err != nil {
		return "", fmt.Errorf("failed to generate summary of graph: %w", err)
	}
	if len(summaryResp.Choices) == 0 {
		return "", fmt.Errorf("empty summary response from LLM")
	}
	summaryContext := summaryResp.Choices[0].Content

	// ========================================
	// 5. 要約をコンテキストとして最終的な回答を生成
	// ========================================
	// GraphCompletionと同じプロンプトを再利用
	finalUserPrompt := fmt.Sprintf(prompts.GraphContextForQuestionPrompt, query, summaryContext)

	finalResp, err := t.LLM.GenerateContent(ctx, []llms.MessageContent{
		llms.TextParts(llms.ChatMessageTypeSystem, prompts.AnswerSimpleQuestionPrompt),
		llms.TextParts(llms.ChatMessageTypeHuman, finalUserPrompt),
	})
	if err != nil {
		return "", fmt.Errorf("failed to generate final answer: %w", err)
	}

	if len(finalResp.Choices) == 0 {
		return "", fmt.Errorf("no final answer generated")
	}

	return finalResp.Choices[0].Content, nil
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
func (t *GraphCompletionTool) searchGraphCompletion(ctx context.Context, query string) (string, error) {
	// クエリをベクトル化
	queryVector, err := t.Embedder.EmbedQuery(ctx, query)
	if err != nil {
		return "", fmt.Errorf("failed to embed query: %w", err)
	}

	// ========================================
	// 1. ベクトル検索（並列実行が推奨されますが、ここでは順次実行）
	// ========================================

	// A. チャンクを検索
	chunkResults, err := t.VectorStorage.Search(ctx, "DocumentChunk_text", queryVector, 5, t.groupID)
	if err != nil {
		return "", fmt.Errorf("chunk search failed: %w", err)
	}

	// B. ノードを検索
	nodeResults, err := t.VectorStorage.Search(ctx, "Entity_name", queryVector, 5, t.groupID)
	if err != nil {
		return "", fmt.Errorf("node search failed: %w", err)
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
	triplets, err := t.GraphStorage.GetTriplets(ctx, nodeIDs, t.groupID)
	if err != nil {
		return "", fmt.Errorf("graph traversal failed: %w", err)
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
		sourceName := getName(triplet.Source)
		targetName := getName(triplet.Target)

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
		return "", fmt.Errorf("failed to generate answer: %w", err)
	}

	if len(resp.Choices) == 0 {
		return "", fmt.Errorf("no answer generated")
	}

	return resp.Choices[0].Content, nil
}

// getName は、ノードのプロパティから名前を安全に取得します。
// "name"プロパティが存在しない場合は、ノードIDをフォールバックとして使用します。
//
// 引数:
//   - node: ノード
//
// 返り値:
//   - string: ノードの名前またはID
func getName(node *storage.Node) string {
	// ノードまたはプロパティがnilの場合
	if node == nil || node.Properties == nil {
		return "Unknown"
	}
	// "name"プロパティを取得
	if name, ok := node.Properties["name"].(string); ok {
		return name
	}
	// フォールバック: ノードIDを使用
	return node.ID
}
