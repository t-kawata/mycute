// Package query は、グラフベースの検索ツールを提供します。
// GraphCompletionToolは、ベクトル検索とグラフトラバーサルを組み合わせて
// 質問に対する回答を生成します。
package query

import (
	"context"
	"errors"
	"fmt"
	"slices"
	"strings"

	"github.com/cloudwego/eino/components/model"
	"github.com/t-kawata/mycute/pkg/cuber/prompts"
	"github.com/t-kawata/mycute/pkg/cuber/storage"
	"github.com/t-kawata/mycute/pkg/cuber/types"
	"github.com/t-kawata/mycute/pkg/cuber/utils"
)

// GraphCompletionTool は、グラフベースの検索ツールです。
// このツールは、以下の検索タイプをサポートします：
//   - SUMMARIES: 要約のみを検索
//   - GRAPH_SUMMARY_COMPLETION: グラフを検索して要約を生成
//   - GRAPH_COMPLETION: グラフとチャンクを組み合わせて回答を生成（デフォルト）
type GraphCompletionTool struct {
	VectorStorage storage.VectorStorage      // ベクトルストレージ（KuzuDB）
	GraphStorage  storage.GraphStorage       // グラフストレージ（KuzuDB）
	LLM           model.ToolCallingChatModel // テキスト生成LLM (Eino)
	Embedder      storage.Embedder           // Embedder
	memoryGroup   string                     // メモリーグループ（パーティション識別子）
	ModelName     string                     // 使用するモデル名（トークン集計用）
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
func NewGraphCompletionTool(vectorStorage storage.VectorStorage, graphStorage storage.GraphStorage, llm model.ToolCallingChatModel, embedder storage.Embedder, memoryGroup string, modelName string) *GraphCompletionTool {
	return &GraphCompletionTool{
		VectorStorage: vectorStorage,
		GraphStorage:  graphStorage,
		LLM:           llm,
		Embedder:      embedder,
		memoryGroup:   memoryGroup,
		ModelName:     modelName,
	}
}

// Query は、指定されたタイプで検索を実行し、回答を生成します。
// 引数:
//   - ctx: コンテキスト
//   - query: 検索クエリ
//   - queryType: 検索タイプ
//
// 返り値:
//   - string: 検索結果（回答）
func (t *GraphCompletionTool) Query(ctx context.Context, query string, config types.QueryConfig) (answer *string, chunks *string, summaries *string, graph *[]*storage.Triple, embedding *[]float32, usage types.TokenUsage, err error) {
	if !types.IsValidQueryType(uint8(config.QueryType)) {
		err = fmt.Errorf("GraphCompletionTool: Unknown query type: %d", config.QueryType)
		return
	}
	switch config.QueryType {
	case types.QUERY_TYPE_GET_GRAPH:
		if config.EntityTopk == 0 {
			err = fmt.Errorf("GraphCompletionTool: EntityTopk must be greater than 0")
			return
		}
		embedding, graph, usage, err = t.getGraph(ctx, config.EntityTopk, query, nil)
		return
	case types.QUERY_TYPE_GET_CHUNKS:
		if config.ChunkTopk == 0 {
			err = fmt.Errorf("GraphCompletionTool: ChunkTopk must be greater than 0")
			return
		}
		embedding, chunks, usage, err = t.getChunks(ctx, config.ChunkTopk, query, nil)
		return
	case types.QUERY_TYPE_GET_PRE_MADE_SUMMARIES:
		if config.SummaryTopk == 0 {
			err = fmt.Errorf("GraphCompletionTool: SummaryTopk must be greater than 0")
			return
		}
		embedding, summaries, usage, err = t.getSummaries(ctx, config.SummaryTopk, query, nil)
		return
	case types.QUERY_TYPE_GET_GRAPH_AND_CHUNKS:
		if config.EntityTopk == 0 || config.ChunkTopk == 0 {
			err = fmt.Errorf("GraphCompletionTool: EntityTopk and ChunkTopk must be greater than 0")
			return
		}
		embedding, graph, chunks, usage, err = t.getGraphAndChunks(ctx, config.EntityTopk, config.ChunkTopk, query, nil)
		return
	case types.QUERY_TYPE_GET_GRAPH_AND_PRE_MADE_SUMMARIES:
		if config.EntityTopk == 0 || config.SummaryTopk == 0 {
			err = fmt.Errorf("GraphCompletionTool: EntityTopk and SummaryTopk must be greater than 0")
			return
		}
		embedding, graph, summaries, usage, err = t.getGraphAndSummaries(ctx, config.EntityTopk, config.SummaryTopk, query, nil)
		return
	case types.QUERY_TYPE_GET_GRAPH_AND_CHUNKS_AND_PRE_MADE_SUMMARIES:
		if config.EntityTopk == 0 || config.ChunkTopk == 0 || config.SummaryTopk == 0 {
			err = fmt.Errorf("GraphCompletionTool: EntityTopk, ChunkTopk and SummaryTopk must be greater than 0")
			return
		}
		embedding, graph, chunks, summaries, usage, err = t.getGraphAndChunksAndSummaries(ctx, config.EntityTopk, config.ChunkTopk, config.SummaryTopk, query, nil)
		return
	case types.QUERY_TYPE_GET_GRAPH_EXPLANATION_EN:
		if config.EntityTopk == 0 {
			err = fmt.Errorf("GraphCompletionTool: EntityTopk must be greater than 0")
			return
		}
		embedding, answer, usage, err = t.getEnglishGraphExplanation(ctx, config.EntityTopk, query, nil)
		return
	case types.QUERY_TYPE_GET_GRAPH_EXPLANATION_JA:
		if config.EntityTopk == 0 {
			err = fmt.Errorf("GraphCompletionTool: EntityTopk must be greater than 0")
			return
		}
		embedding, answer, usage, err = t.getJapaneseGraphExplanation(ctx, config.EntityTopk, query, nil)
		return
	case types.QUERY_TYPE_GET_GRAPH_SUMMARY_EN:
		if config.EntityTopk == 0 {
			err = fmt.Errorf("GraphCompletionTool: EntityTopk must be greater than 0")
			return
		}
		embedding, answer, usage, err = t.getEnglishGraphSummary(ctx, config.EntityTopk, query, nil)
		return
	case types.QUERY_TYPE_GET_GRAPH_SUMMARY_JA:
		if config.EntityTopk == 0 {
			err = fmt.Errorf("GraphCompletionTool: EntityTopk must be greater than 0")
			return
		}
		embedding, answer, usage, err = t.getJapaneseGraphSummary(ctx, config.EntityTopk, query, nil)
		return
	case types.QUERY_TYPE_GET_GRAPH_SUMMARY_TO_ANSWER_EN:
		if config.EntityTopk == 0 {
			err = fmt.Errorf("GraphCompletionTool: EntityTopk must be greater than 0")
			return
		}
		embedding, answer, usage, err = t.getEnglishGraphSummaryToAnswer(ctx, config.EntityTopk, query, nil)
		return
	case types.QUERY_TYPE_GET_GRAPH_SUMMARY_TO_ANSWER_JA:
		if config.EntityTopk == 0 {
			err = fmt.Errorf("GraphCompletionTool: EntityTopk must be greater than 0")
			return
		}
		embedding, answer, usage, err = t.getJapaneseGraphSummaryToAnswer(ctx, config.EntityTopk, query, nil)
		return
	case types.QUERY_TYPE_ANSWER_BY_PRE_MADE_SUMMARIES_AND_GRAPH_SUMMARY_EN:
		if config.SummaryTopk == 0 || config.EntityTopk == 0 {
			err = fmt.Errorf("GraphCompletionTool: SummaryTopk and EntityTopk must be greater than 0")
			return
		}
		embedding, answer, usage, err = t.getGraphSummaryCompletionEN(ctx, config.SummaryTopk, config.EntityTopk, query, nil)
		return
	case types.QUERY_TYPE_ANSWER_BY_PRE_MADE_SUMMARIES_AND_GRAPH_SUMMARY_JA:
		if config.SummaryTopk == 0 || config.EntityTopk == 0 {
			err = fmt.Errorf("GraphCompletionTool: SummaryTopk and EntityTopk must be greater than 0")
			return
		}
		embedding, answer, usage, err = t.getGraphSummaryCompletionJA(ctx, config.SummaryTopk, config.EntityTopk, query, nil)
		return
	case types.QUERY_TYPE_ANSWER_BY_CHUNKS_AND_GRAPH_SUMMARY_EN:
		if config.ChunkTopk == 0 || config.EntityTopk == 0 {
			err = fmt.Errorf("GraphCompletionTool: ChunkTopk and EntityTopk must be greater than 0")
			return
		}
		embedding, answer, usage, err = t.getGraphCompletionEN(ctx, config.ChunkTopk, config.EntityTopk, query, nil)
		return
	case types.QUERY_TYPE_ANSWER_BY_CHUNKS_AND_GRAPH_SUMMARY_JA:
		if config.ChunkTopk == 0 || config.EntityTopk == 0 {
			err = fmt.Errorf("GraphCompletionTool: ChunkTopk and EntityTopk must be greater than 0")
			return
		}
		embedding, answer, usage, err = t.getGraphCompletionJA(ctx, config.ChunkTopk, config.EntityTopk, query, nil)
		return
	default:
		err = fmt.Errorf("GraphCompletionTool: Unknown query type: %d", config.QueryType)
		return
	}
}

func (t *GraphCompletionTool) getGraph(ctx context.Context, entityTopk int, query string, embeddingVecs *[]float32) (embedding *[]float32, graph *[]*storage.Triple, usage types.TokenUsage, err error) {
	// ========================================
	// 1. ノードを検索
	// ========================================
	var embeddingVectors []float32
	if embeddingVecs != nil && len(*embeddingVecs) > 0 {
		embeddingVectors = *embeddingVecs
	} else {
		tmpEmbeddingVectors, u, errr := t.Embedder.EmbedQuery(ctx, query)
		usage.Add(u)
		if errr != nil {
			err = fmt.Errorf("GraphCompletionTool: Failed to embed query: %w", errr)
			return
		}
		embeddingVectors = tmpEmbeddingVectors
	}
	// Entityテーブルを検索
	entityResults, err := t.VectorStorage.Query(ctx, types.TABLE_NAME_ENTITY, embeddingVectors, entityTopk, t.memoryGroup)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Node query failed: %w", err)
		return
	}
	// GraghNodeIDを収集（重複を除く）
	var graphNodeIDs []string
	uniqueGraphNodeIDs := make(map[string]bool)
	for _, res := range entityResults {
		if !uniqueGraphNodeIDs[res.ID] {
			graphNodeIDs = append(graphNodeIDs, res.ID)
			uniqueGraphNodeIDs[res.ID] = true
		}
	}
	// ========================================
	// 2. グラフトラバーサル
	// ========================================
	triples, err := t.GraphStorage.GetTriples(ctx, graphNodeIDs, t.memoryGroup)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Graph traversal failed: %w", err)
		return
	}
	graph = &triples
	embedding = &embeddingVectors
	return
}

// getChunks は、Chunkのみを検索して返します。
// この関数は以下の処理を行います：
//  1. クエリをベクトル化
//  2. "Chunk"テーブルから類似するChunkを検索
//  3. Chunkのリストを返す
//
// 引数:
//   - ctx: コンテキスト
//   - chunkTopk: 返す結果の最大数
//   - query: 検索クエリ
//
// 返り値:
//   - string: Chunkのリスト
//   - error: エラーが発生した場合
func (t *GraphCompletionTool) getChunks(ctx context.Context, chunkTopk int, query string, embeddingVecs *[]float32) (embedding *[]float32, chunks *string, usage types.TokenUsage, err error) {
	// クエリをベクトル化
	var embeddingVectors []float32
	if embeddingVecs != nil && len(*embeddingVecs) > 0 {
		embeddingVectors = *embeddingVecs
	} else {
		tmpEmbeddingVectors, u, errr := t.Embedder.EmbedQuery(ctx, query)
		usage.Add(u)
		if errr != nil {
			err = fmt.Errorf("GraphCompletionTool: Failed to embed query: %w", errr)
			return
		}
		embeddingVectors = tmpEmbeddingVectors
	}
	// Chunkテーブルを検索
	results, err := t.VectorStorage.Query(ctx, types.TABLE_NAME_CHUNK, embeddingVectors, chunkTopk, t.memoryGroup)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to query chunks: %w", err)
		return
	}
	// 結果が見つからない場合
	if len(results) == 0 {
		tmp := ""
		chunks = &tmp
		return
	}
	// 要約のリストを構築
	var sb strings.Builder
	for _, result := range results {
		sb.WriteString("- " + result.Text + "\n\n")
	}
	tmp := strings.TrimSpace(sb.String())
	chunks = &tmp
	embedding = &embeddingVectors
	return
}

// getSummaries は、要約のみを検索して返します。
// この関数は以下の処理を行います：
//  1. クエリをベクトル化
//  2. "Summary"テーブルから類似する要約を検索
//  3. 要約のリストを返す
//
// 引数:
//   - ctx: コンテキスト
//   - summaryTopk: 返す結果の最大数
//   - query: 検索クエリ
//
// 返り値:
//   - string: 要約のリスト
//   - error: エラーが発生した場合
func (t *GraphCompletionTool) getSummaries(ctx context.Context, summaryTopk int, query string, embeddingVecs *[]float32) (embedding *[]float32, summaries *string, usage types.TokenUsage, err error) {
	// クエリをベクトル化
	var embeddingVectors []float32
	if embeddingVecs != nil && len(*embeddingVecs) > 0 {
		embeddingVectors = *embeddingVecs
	} else {
		tmpEmbeddingVectors, u, errr := t.Embedder.EmbedQuery(ctx, query)
		usage.Add(u)
		if errr != nil {
			err = fmt.Errorf("GraphCompletionTool: Failed to embed query: %w", errr)
			return
		}
		embeddingVectors = tmpEmbeddingVectors
	}
	// Summaryテーブルを検索
	results, err := t.VectorStorage.Query(ctx, types.TABLE_NAME_SUMMARY, embeddingVectors, summaryTopk, t.memoryGroup)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to query summaries: %w", err)
		return
	}
	// 結果が見つからない場合
	if len(results) == 0 {
		tmp := ""
		summaries = &tmp
		return
	}
	// 要約のリストを構築
	var sb strings.Builder
	for _, result := range results {
		sb.WriteString("- " + result.Text + "\n\n")
	}
	tmp := strings.TrimSpace(sb.String())
	summaries = &tmp
	embedding = &embeddingVectors
	return
}

func (t *GraphCompletionTool) getGraphAndChunks(ctx context.Context, entityTopk int, chunkTopk int, query string, embeddingVecs *[]float32) (embedding *[]float32, graph *[]*storage.Triple, chunks *string, usage types.TokenUsage, err error) {
	embeddingVectors, triples, u, err := t.getGraph(ctx, entityTopk, query, embeddingVecs)
	usage.Add(u)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to get graph: %w", err)
		return
	}
	_, tmpChunks, u, err := t.getChunks(ctx, chunkTopk, query, embeddingVectors)
	usage.Add(u)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to get chunks: %w", err)
		return
	}
	graph = triples
	chunks = tmpChunks
	embedding = embeddingVectors
	return
}

func (t *GraphCompletionTool) getGraphAndSummaries(ctx context.Context, entityTopk int, summaryTopk int, query string, embeddingVecs *[]float32) (embedding *[]float32, graph *[]*storage.Triple, summaries *string, usage types.TokenUsage, err error) {
	embeddingVectors, triples, u, err := t.getGraph(ctx, entityTopk, query, embeddingVecs)
	usage.Add(u)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to get graph: %w", err)
		return
	}
	_, tmpSummaries, u, err := t.getSummaries(ctx, summaryTopk, query, embeddingVectors)
	usage.Add(u)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to get summaries: %w", err)
		return
	}
	graph = triples
	summaries = tmpSummaries
	embedding = embeddingVectors
	return
}

func (t *GraphCompletionTool) getGraphAndChunksAndSummaries(ctx context.Context, entityTopk int, chunkTopk int, summaryTopk int, query string, embeddingVecs *[]float32) (embedding *[]float32, graph *[]*storage.Triple, chunks *string, summaries *string, usage types.TokenUsage, err error) {
	embeddingVectors, triples, u, err := t.getGraph(ctx, entityTopk, query, embeddingVecs)
	usage.Add(u)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to get graph: %w", err)
		return
	}
	_, tmpChunks, u, err := t.getChunks(ctx, chunkTopk, query, embeddingVectors)
	usage.Add(u)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to get chunks: %w", err)
		return
	}
	_, tmpSummaries, u, err := t.getSummaries(ctx, summaryTopk, query, embeddingVectors)
	usage.Add(u)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to get summaries: %w", err)
		return
	}
	graph = triples
	chunks = tmpChunks
	summaries = tmpSummaries
	embedding = embeddingVectors
	return
}

func (t *GraphCompletionTool) getEnglishGraphExplanation(ctx context.Context, entityTopk int, query string, embeddingVecs *[]float32) (embedding *[]float32, graphExplanation *string, usage types.TokenUsage, err error) {
	// 1. 関連するグラフを検索
	embeddingVectors, triples, u, err := t.getGraph(ctx, entityTopk, query, embeddingVecs)
	usage.Add(u)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to get graph: %w", err)
		return
	}
	// 2. トリプルをテキスト説明文に変換
	graphText := &strings.Builder{}
	graphText = generateNaturalEnglishGraphExplanationByTiples(triples, graphText)
	tmp := strings.TrimSpace(graphText.String())
	graphExplanation = &tmp
	embedding = embeddingVectors
	return
}

func (t *GraphCompletionTool) getJapaneseGraphExplanation(ctx context.Context, entityTopk int, query string, embeddingVecs *[]float32) (embedding *[]float32, graphExplanation *string, usage types.TokenUsage, err error) {
	// 1. 関連するグラフを検索
	embeddingVectors, triples, u, err := t.getGraph(ctx, entityTopk, query, embeddingVecs)
	usage.Add(u)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to get graph: %w", err)
		return
	}
	// 2. トリプルをテキスト説明文に変換
	graphText := &strings.Builder{}
	graphText = generateNaturalJapaneseGraphExplanationByTriples(triples, graphText)
	tmp := strings.TrimSpace(graphText.String())
	graphExplanation = &tmp
	embedding = embeddingVectors
	return
}

func (t *GraphCompletionTool) getEnglishGraphSummary(ctx context.Context, entityTopk int, query string, embeddingVecs *[]float32) (embedding *[]float32, graphSummary *string, usage types.TokenUsage, err error) {
	embeddingVectors, graphExplanation, u, err := t.getEnglishGraphExplanation(ctx, entityTopk, query, embeddingVecs)
	usage.Add(u)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to get graph explanation: %w", err)
		return
	}
	summarizePrompt := fmt.Sprintf("USER QUERY: %s\n\nKNOWLEDGE GRAPH INFORMATION:\n%s", query, *graphExplanation)
	summaryContent, u, err := utils.GenerateWithUsage(ctx, t.LLM, t.ModelName, prompts.SUMMARIZE_GRAPH_ITSELF_EN_PROMPT, summarizePrompt)
	usage.Add(u)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to generate summary of graph: %w", err)
		return
	}
	if summaryContent == "" {
		err = fmt.Errorf("GraphCompletionTool: Empty summary response from LLM.")
		return
	}
	graphSummary = &summaryContent
	embedding = embeddingVectors
	return
}

func (t *GraphCompletionTool) getJapaneseGraphSummary(ctx context.Context, entityTopk int, query string, embeddingVecs *[]float32) (embedding *[]float32, graphSummary *string, usage types.TokenUsage, err error) {
	embeddingVectors, graphExplanation, u, err := t.getEnglishGraphExplanation(ctx, entityTopk, query, embeddingVecs) // 要約時点で日本語にするので、ここは英語で良い
	usage.Add(u)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to get graph explanation: %w", err)
		return
	}
	summarizePrompt := fmt.Sprintf("USER QUERY: %s\n\nKNOWLEDGE GRAPH INFORMATION:\n%s", query, *graphExplanation)
	summaryContent, u, err := utils.GenerateWithUsage(ctx, t.LLM, t.ModelName, prompts.SUMMARIZE_GRAPH_ITSELF_JA_PROMPT, summarizePrompt)
	usage.Add(u)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to generate summary of graph: %w", err)
		return
	}
	if summaryContent == "" {
		err = fmt.Errorf("GraphCompletionTool: Empty summary response from LLM.")
		return
	}
	graphSummary = &summaryContent
	embedding = embeddingVectors
	return
}

func (t *GraphCompletionTool) getEnglishGraphSummaryToAnswer(ctx context.Context, entityTopk int, query string, embeddingVecs *[]float32) (embedding *[]float32, graphSummary *string, usage types.TokenUsage, err error) {
	embeddingVectors, graphExplanation, u, err := t.getEnglishGraphExplanation(ctx, entityTopk, query, embeddingVecs)
	usage.Add(u)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to get graph explanation: %w", err)
		return
	}
	summarizePrompt := fmt.Sprintf("USER QUERY: %s\n\nKNOWLEDGE GRAPH INFORMATION:\n%s", query, *graphExplanation)
	summaryContent, u, err := utils.GenerateWithUsage(ctx, t.LLM, t.ModelName, prompts.SUMMARIZE_GRAPH_EXPLANATION_TO_ANSWER_EN_PROMPT, summarizePrompt)
	usage.Add(u)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to generate summary of graph: %w", err)
		return
	}
	if summaryContent == "" {
		err = fmt.Errorf("GraphCompletionTool: Empty summary response from LLM.")
		return
	}
	graphSummary = &summaryContent
	embedding = embeddingVectors
	return
}

func (t *GraphCompletionTool) getJapaneseGraphSummaryToAnswer(ctx context.Context, entityTopk int, query string, embeddingVecs *[]float32) (embedding *[]float32, graphSummary *string, usage types.TokenUsage, err error) {
	embeddingVectors, graphExplanation, u, err := t.getEnglishGraphExplanation(ctx, entityTopk, query, embeddingVecs) // 要約時点で日本語にするので、ここは英語で良い
	usage.Add(u)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to get graph explanation: %w", err)
		return
	}
	summarizePrompt := fmt.Sprintf("USER QUERY: %s\n\nKNOWLEDGE GRAPH INFORMATION:\n%s", query, *graphExplanation)
	summaryContent, u, err := utils.GenerateWithUsage(ctx, t.LLM, t.ModelName, prompts.SUMMARIZE_GRAPH_EXPLANATION_TO_ANSWER_JA_PROMPT, summarizePrompt)
	usage.Add(u)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to generate summary of graph: %w", err)
		return
	}
	if summaryContent == "" {
		err = fmt.Errorf("GraphCompletionTool: Empty summary response from LLM")
		return
	}
	graphSummary = &summaryContent
	embedding = embeddingVectors
	return
}

// getGraphSummaryCompletion は、グラフ（要約）を検索して回答を生成します。（英語で回答）
// この関数は以下の処理を行います：
//  1. ノードを検索
//  2. グラフトラバーサルでトリプルを取得
//  3. トリプルをテキストに変換
//  4. LLMでグラフの要約を生成
//  5. 要約をコンテキストとして最終的な回答を生成
//
// 引数:
//   - ctx: コンテキスト
//   - summaryTopk: Summaryテーブルを検索する際のtopk
//   - entityTopk: Entityテーブルを検索する際のtopk
//   - query: 検索クエリ
//
// 返り値:
//   - string: 回答
//   - error: エラーが発生した場合
func (t *GraphCompletionTool) getGraphSummaryCompletionEN(ctx context.Context, summaryTopk int, entityTopk int, query string, embeddingVecs *[]float32) (embedding *[]float32, answer *string, usage types.TokenUsage, err error) {
	// 1. 関連するSummaryを検索
	embeddingVectors, summaries, u, err := t.getSummaries(ctx, summaryTopk, query, embeddingVecs)
	usage.Add(u)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to query summaries: %w", err)
		return
	}
	if *summaries == "" {
		answer = summaries
		return
	}
	// 2. グラフの「クエリ回答用要約」を生成
	_, graphSummaryText, u, err := t.getEnglishGraphSummary(ctx, entityTopk, query, embeddingVectors)
	usage.Add(u)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to get graph summary: %w", err)
		return
	}
	// 3. 取得した事前要約群とグラフ要約文をコンテキストとして最終的な回答を生成
	tmpAnswer, u, err := t.answerQueryByVectorAndGraphResultEN(ctx, summaries, graphSummaryText, query)
	usage.Add(u)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to answer query by vector and graph result: %w", err)
		return
	}
	answer = tmpAnswer
	embedding = embeddingVectors
	return
}

// getGraphSummaryCompletion は、グラフ（要約）を検索して回答を生成します。（日本語で回答）
// この関数は以下の処理を行います：
//  1. ノードを検索
//  2. グラフトラバーサルでトリプルを取得
//  3. トリプルをテキストに変換
//  4. LLMでグラフの要約を生成
//  5. 要約をコンテキストとして最終的な回答を生成
//
// 引数:
//   - ctx: コンテキスト
//   - summaryTopk: Summaryテーブルを検索する際のtopk
//   - entityTopk: Entityテーブルを検索する際のtopk
//   - query: 検索クエリ
//
// 返り値:
//   - string: 回答
//   - error: エラーが発生した場合
func (t *GraphCompletionTool) getGraphSummaryCompletionJA(ctx context.Context, summaryTopk int, entityTopk int, query string, embeddingVecs *[]float32) (embedding *[]float32, answer *string, usage types.TokenUsage, err error) {
	// 1. 関連するSummaryを検索
	embeddingVectors, summaries, u, err := t.getSummaries(ctx, summaryTopk, query, embeddingVecs)
	usage.Add(u)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to query summaries: %w", err)
		return
	}
	if *summaries == "" {
		answer = summaries
		return
	}
	// 2. グラフの「クエリ回答用要約」を生成
	_, graphSummaryText, u, err := t.getEnglishGraphSummary(ctx, entityTopk, query, embeddingVectors) // 最後の回答で日本語にするので、ここは英語で良い
	usage.Add(u)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to get graph summary: %w", err)
		return
	}
	// 3. 取得した事前要約群とグラフ要約文をコンテキストとして最終的な回答を生成
	tmpAnswer, u, err := t.answerQueryByVectorAndGraphResultJA(ctx, summaries, graphSummaryText, query)
	usage.Add(u)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to answer query by vector and graph result: %w", err)
		return
	}
	answer = tmpAnswer
	embedding = embeddingVectors
	return
}

// getGraphCompletionEN は、グラフとチャンクを組み合わせて回答を生成します（英語）。
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
func (t *GraphCompletionTool) getGraphCompletionEN(ctx context.Context, chunkTopk int, entityTopk int, query string, embeddingVecs *[]float32) (embedding *[]float32, answer *string, usage types.TokenUsage, err error) {
	// 1. 関連するSummaryを検索
	embeddingVectors, chunks, u, err := t.getChunks(ctx, chunkTopk, query, embeddingVecs)
	usage.Add(u)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to query chunks: %w", err)
		return
	}
	if *chunks == "" {
		answer = chunks
		return
	}
	// 2. グラフの「クエリ回答用要約」を生成
	_, graphSummaryText, u, err := t.getEnglishGraphSummary(ctx, entityTopk, query, embeddingVectors)
	usage.Add(u)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to get graph summary: %w", err)
		return
	}
	// 3. 取得した事前要約群とグラフ要約文をコンテキストとして最終的な回答を生成
	tmpAnswer, u, err := t.answerQueryByVectorAndGraphResultEN(ctx, chunks, graphSummaryText, query)
	usage.Add(u)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to answer query by vector and graph result: %w", err)
		return
	}
	answer = tmpAnswer
	embedding = embeddingVectors
	return
}

// getGraphCompletionJA は、グラフとチャンクを組み合わせて回答を生成します（日本語）。
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
func (t *GraphCompletionTool) getGraphCompletionJA(ctx context.Context, chunkTopk int, entityTopk int, query string, embeddingVecs *[]float32) (embedding *[]float32, answer *string, usage types.TokenUsage, err error) {
	// 1. 関連するSummaryを検索
	embeddingVectors, chunks, u, err := t.getChunks(ctx, chunkTopk, query, embeddingVecs)
	usage.Add(u)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to query chunks: %w", err)
		return
	}
	if *chunks == "" {
		answer = chunks
		return
	}
	// 2. グラフの「クエリ回答用要約」を生成
	_, graphSummaryText, u, err := t.getEnglishGraphSummary(ctx, entityTopk, query, embeddingVectors) // 最後の回答で日本語にするので、ここは英語で良い
	usage.Add(u)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to get graph summary: %w", err)
		return
	}
	// 3. 取得した事前要約群とグラフ要約文をコンテキストとして最終的な回答を生成
	tmpAnswer, u, err := t.answerQueryByVectorAndGraphResultJA(ctx, chunks, graphSummaryText, query)
	usage.Add(u)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to answer query by vector and graph result: %w", err)
		return
	}
	answer = tmpAnswer
	embedding = embeddingVectors
	return
}

// ベクトル検索結果とグラフ検索結果をコンテキストとして回答を生成する（英語で回答）
func (t *GraphCompletionTool) answerQueryByVectorAndGraphResultEN(ctx context.Context, vectorResult *string, graphResult *string, query string) (answer *string, usage types.TokenUsage, err error) {
	finalUserPrompt := fmt.Sprintf("User Question: %s\n\nVector Search Results:\n%s\n\nKnowledge Graph Summary:\n%s", query, *vectorResult, *graphResult)
	answerContent, u, err := utils.GenerateWithUsage(ctx, t.LLM, t.ModelName, prompts.ANSWER_QUERY_WITH_HYBRID_RAG_EN_PROMPT, finalUserPrompt)
	usage.Add(u)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to generate final answer: %w", err)
		return
	}
	if answerContent == "" {
		err = errors.New("GraphCompletionTool: No final answer generated.")
		return
	}
	answer = &answerContent
	return
}

// ベクトル検索結果とグラフ検索結果をコンテキストとして回答を生成する（日本語で回答）
func (t *GraphCompletionTool) answerQueryByVectorAndGraphResultJA(ctx context.Context, vectorResult *string, graphResult *string, query string) (answer *string, usage types.TokenUsage, err error) {
	finalUserPrompt := fmt.Sprintf("User Question: %s\n\nVector Search Results:\n%s\n\nKnowledge Graph Summary:\n%s", query, *vectorResult, *graphResult)
	answerContent, u, err := utils.GenerateWithUsage(ctx, t.LLM, t.ModelName, prompts.ANSWER_QUERY_WITH_HYBRID_RAG_JA_PROMPT, finalUserPrompt)
	usage.Add(u)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to generate final answer: %w", err)
		return
	}
	if answerContent == "" {
		err = errors.New("GraphCompletionTool: No final answer generated.")
		return
	}
	answer = &answerContent
	return
}

/**
 * 与えられた知識グラフトリプルから、自然な英語の説明文を構成する（英語）
 */
func generateNaturalEnglishGraphExplanationByTiples(triples *[]*storage.Triple, graphText *strings.Builder) *strings.Builder {
	// =================================
	// Information about word entities
	// =================================
	graphText.WriteString("# Information about word entities\n")
	doneWords := []string{}
	for _, triple := range *triples {
		// 1. Source
		if !slices.Contains(doneWords, triple.Source.ID) {
			doneWords = append(doneWords, triple.Source.ID)
			fmt.Fprintf(graphText, "- '%s' is a type of '%s'.", triple.Source.ID, triple.Source.Type)
			if len(triple.Source.Properties) > 0 {
				fmt.Fprintf(graphText, " Additional information about '%s' is as follows:\n", triple.Source.ID)
				for k, prop := range triple.Source.Properties {
					fmt.Fprintf(graphText, "    * %s: %v\n", k, prop)
				}
			} else {
				graphText.WriteString("\n")
			}
		}
		// 2. Target
		if !slices.Contains(doneWords, triple.Target.ID) {
			doneWords = append(doneWords, triple.Target.ID)
			fmt.Fprintf(graphText, "- '%s' is a type of '%s'.", triple.Target.ID, triple.Target.Type)
			if len(triple.Target.Properties) > 0 {
				fmt.Fprintf(graphText, " Additional information about '%s' is as follows:\n", triple.Target.ID)
				for k, prop := range triple.Target.Properties {
					fmt.Fprintf(graphText, "    * %s: %v\n", k, prop)
				}
			} else {
				graphText.WriteString("\n")
			}
		}
	}
	// =================================
	// Relations between word entities
	// =================================
	graphText.WriteString("\n# Relations between word entities\n")
	for _, triple := range *triples {
		fmt.Fprintf(
			graphText,
			"- '%s' and '%s' are connected by the relation '%s', where '%s' is the source (from) and '%s' is the target (to).",
			triple.Edge.SourceID,
			triple.Edge.TargetID,
			triple.Edge.Type,
			triple.Edge.SourceID,
			triple.Edge.TargetID,
		)
		if len(triple.Edge.Properties) > 0 {
			graphText.WriteString(" Additional information about their relationship is as follows:\n")
			for k, prop := range triple.Edge.Properties {
				fmt.Fprintf(graphText, "    * %s: %v\n", k, prop)
			}
		} else {
			graphText.WriteString("\n")
		}
	}
	return graphText
}

/**
 * 与えられた知識グラフトリプルから、自然な日本語の説明文を構成する（日本語）
 */
func generateNaturalJapaneseGraphExplanationByTriples(triples *[]*storage.Triple, graphText *strings.Builder) *strings.Builder {
	// =================================
	// 単語エンティティの情報
	// =================================
	graphText.WriteString("# 単語エンティティの情報\n")
	doneWords := []string{}
	for _, triple := range *triples {
		// 1. Source
		if !slices.Contains(doneWords, triple.Source.ID) {
			doneWords = append(doneWords, triple.Source.ID)
			fmt.Fprintf(graphText, "- 「%s」は「%s」型のエンティティです。", triple.Source.ID, triple.Source.Type)
			if len(triple.Source.Properties) > 0 {
				fmt.Fprintf(graphText, "「%s」の追加情報は以下の通りです:\n", triple.Source.ID)
				for k, prop := range triple.Source.Properties {
					fmt.Fprintf(graphText, "    * %s: %v\n", k, prop)
				}
			} else {
				graphText.WriteString("\n")
			}
		}
		// 2. Target
		if !slices.Contains(doneWords, triple.Target.ID) {
			doneWords = append(doneWords, triple.Target.ID)
			fmt.Fprintf(graphText, "- 「%s」は「%s」型のエンティティです。", triple.Target.ID, triple.Target.Type)
			if len(triple.Target.Properties) > 0 {
				fmt.Fprintf(graphText, "「%s」の追加情報は以下の通りです:\n", triple.Target.ID)
				for k, prop := range triple.Target.Properties {
					fmt.Fprintf(graphText, "    * %s: %v\n", k, prop)
				}
			} else {
				graphText.WriteString("\n")
			}
		}
	}
	// =================================
	// 単語エンティティ間の関係
	// =================================
	graphText.WriteString("\n# 単語エンティティ間の関係\n")
	for _, triple := range *triples {
		fmt.Fprintf(
			graphText,
			"- 「%s」と「%s」は「%s」という関係で結ばれています。「%s」が始点、「%s」が終点です。",
			triple.Edge.SourceID,
			triple.Edge.TargetID,
			triple.Edge.Type,
			triple.Edge.SourceID,
			triple.Edge.TargetID,
		)
		if len(triple.Edge.Properties) > 0 {
			graphText.WriteString("この関係の追加情報は以下の通りです:\n")
			for k, prop := range triple.Edge.Properties {
				fmt.Fprintf(graphText, "    * %s: %v\n", k, prop)
			}
		} else {
			graphText.WriteString("\n")
		}
	}
	return graphText
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
