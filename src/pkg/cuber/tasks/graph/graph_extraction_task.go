// Package graph は、LLMを使用してテキストから知識グラフを抽出するタスクを提供します。
// このタスクは、チャンクからエンティティと関係を抽出し、グラフデータを生成します。
package graph

import (
	"context"
	"encoding/json"
	"fmt"
	"strings"
	"sync"

	"go.uber.org/zap"

	"github.com/cloudwego/eino/components/model"
	"github.com/t-kawata/mycute/pkg/cuber/consts"
	"github.com/t-kawata/mycute/pkg/cuber/pipeline"
	"github.com/t-kawata/mycute/pkg/cuber/prompts"
	"github.com/t-kawata/mycute/pkg/cuber/storage"
	"github.com/t-kawata/mycute/pkg/cuber/tools/query"
	"github.com/t-kawata/mycute/pkg/cuber/types"
	"github.com/t-kawata/mycute/pkg/cuber/utils"

	"golang.org/x/sync/errgroup"

	"github.com/t-kawata/mycute/lib/eventbus"
	"github.com/t-kawata/mycute/pkg/cuber/event"
)

// GraphExtractionTask は、グラフ抽出タスクを表します。
// LLMを使用してテキストからエンティティ（ノード）と関係（エッジ）を抽出します。
type GraphExtractionTask struct {
	LLM         model.ToolCallingChatModel // テキスト生成LLM (Eino)
	ModelName   string                     // モデル名
	MemoryGroup string                     // メモリグループ
	Logger      *zap.Logger
	EventBus    *eventbus.EventBus
	IsEn        bool
}

// NewGraphExtractionTask は、新しいGraphExtractionTaskを作成します。
func NewGraphExtractionTask(llm model.ToolCallingChatModel, modelName string, memoryGroup string, l *zap.Logger, eb *eventbus.EventBus, isEn bool) *GraphExtractionTask {
	if modelName == "" {
		modelName = "gpt-4o-mini" // Default fallback
	}
	return &GraphExtractionTask{LLM: llm, ModelName: modelName, MemoryGroup: memoryGroup, Logger: l, EventBus: eb, IsEn: isEn}
}

var _ pipeline.Task = (*GraphExtractionTask)(nil)

// Run は、グラフ抽出タスクを実行します。
// 各チャンクに対して並行してLLMを呼び出し、グラフデータを抽出します。
func (t *GraphExtractionTask) Run(ctx context.Context, input any) (any, types.TokenUsage, error) {
	var totalUsage types.TokenUsage
	chunks, ok := input.([]*storage.Chunk)
	if !ok {
		return nil, totalUsage, fmt.Errorf("Graph Extraction: expected []*storage.Chunk input, got %T", input)
	}
	var (
		allNodes []*storage.Node
		allEdges []*storage.Edge
		mu       sync.Mutex // ノードとエッジのリストへの並行アクセスを保護
	)
	// errgroup: 並行処理とエラーハンドリング
	g, ctx := errgroup.WithContext(ctx)
	// 並行数を制限（レート制限を避けるため）
	g.SetLimit(5)
	utils.LogInfo(t.Logger, "GraphExtractionTask: Starting", zap.Int("chunks", len(chunks)), zap.String("model", t.ModelName))
	for i, chunk := range chunks {
		chunk := chunk // ループ変数をキャプチャ
		g.Go(func() error {
			// ========================================
			// 1. プロンプトを作ってLLMを呼び出し (Eino)
			// ========================================
			prompt := fmt.Sprintf("Extract a knowledge graph from the following Japanese text:\n\n%s", chunk.Text)
			utils.LogDebug(t.Logger, "GraphExtractionTask: Sending request to LLM", zap.String("chunk_id", chunk.ID), zap.Int("prompt_len", len(prompt)))

			// Emit Graph Request Start
			eventbus.Emit(t.EventBus, string(event.EVENT_ABSORB_GRAPH_REQUEST_START), event.AbsorbGraphRequestStartPayload{
				BasePayload: event.NewBasePayload(t.MemoryGroup),
				ChunkID:     chunk.ID,
				ChunkNum:    i + 1,
			})

			content, chunkUsage, err := utils.GenerateWithUsage(ctx, t.LLM, t.ModelName, prompts.GENERATE_GRAPH_PROMPT, prompt)

			// Emit Graph Request End
			eventbus.Emit(t.EventBus, string(event.EVENT_ABSORB_GRAPH_REQUEST_END), event.AbsorbGraphRequestEndPayload{
				BasePayload: event.NewBasePayload(t.MemoryGroup),
				ChunkID:     chunk.ID,
				ChunkNum:    i + 1,
			})

			if err != nil {
				utils.LogWarn(t.Logger, "GraphExtractionTask: LLM call failed", zap.Error(err))
				return fmt.Errorf("LLM call failed: %w", err)
			}
			if content == "" {
				return fmt.Errorf("no response from LLM")
			}
			utils.LogDebug(t.Logger, "GraphExtractionTask: Received response from LLM", zap.String("chunk_id", chunk.ID), zap.Int("response_len", len(content)))
			// ========================================
			// 2. JSONをパース
			// ========================================
			// Emit Graph Parse Start (implicit in logic but good to track)
			eventbus.Emit(t.EventBus, string(event.EVENT_ABSORB_GRAPH_PARSE_START), event.AbsorbGraphParseStartPayload{
				BasePayload: event.NewBasePayload(t.MemoryGroup),
				ChunkID:     chunk.ID,
				ChunkNum:    i + 1,
			})

			content = cleanJSON(content) // JSONオブジェクト部分だけ取り出す
			var graphData storage.GraphData
			if err := json.Unmarshal([]byte(content), &graphData); err != nil {
				// パースエラーの場合は失敗
				utils.LogWarn(t.Logger, "GraphExtractionTask: JSON parse failed", zap.String("content", content), zap.Error(err))
				return fmt.Errorf("Failed to parse Graph Data JSON: %w\nContent: %s", err, content)
			}
			// ========================================
			// 3. 結果を集約
			// ========================================
			mu.Lock()
			allNodes = append(allNodes, graphData.Nodes...)
			allEdges = append(allEdges, graphData.Edges...)
			totalUsage.Add(chunkUsage)
			mu.Unlock()
			utils.LogDebug(t.Logger, "GraphExtractionTask: Extracted graph from chunk", zap.Int("nodes", len(graphData.Nodes)), zap.Int("edges", len(graphData.Edges)))

			// Emit Graph Parse End
			eventbus.Emit(t.EventBus, string(event.EVENT_ABSORB_GRAPH_PARSE_END), event.AbsorbGraphParseEndPayload{
				BasePayload:    event.NewBasePayload(t.MemoryGroup),
				ChunkID:        chunk.ID,
				ChunkNum:    i + 1,
				NodesExtracted: len(graphData.Nodes),
				EdgesExtracted: len(graphData.Edges),
			})
			return nil
		})
	}
	// 全てのgoroutineの完了を待つ
	if err := g.Wait(); err != nil {
		return nil, totalUsage, err
	}
	// 知識グラフのノードとエッジを説明文に変換
	triples, err := storage.ConvertNodesAndEdgesToTriples(&allNodes, &allEdges)
	if err != nil {
		return nil, totalUsage, fmt.Errorf("Failed to convert nodes and edges to triples: %w", err)
	}
	graphText := &strings.Builder{}
	if t.IsEn {
		graphText = query.GenerateNaturalEnglishGraphExplanationByTriples(triples, graphText)
	} else {
		graphText = query.GenerateNaturalJapaneseGraphExplanationByTriples(triples, graphText)
	}
	// Emit Graph Interpreted
	eventbus.Emit(t.EventBus, string(event.EVENT_ABSORB_GRAPH_INTERPRETED), event.AbsorbGraphInterpretedPayload{
		BasePayload:    event.NewBasePayload(t.MemoryGroup),
		InterpretedContent: graphText.String(),
	})
	
	// CognifyOutputを返す（チャンクとグラフデータを含む）
	for i := range allNodes { // メモリーグループ単位でIDがユニークになるように連結する（KuzuDBでは複合ユニークキーが作れないため）
		allNodes[i].ID = fmt.Sprintf("%s%s%s", strings.TrimSpace(allNodes[i].ID), consts.ID_MEMORY_GROUP_SEPARATOR, t.MemoryGroup)
		allNodes[i].MemoryGroup = t.MemoryGroup
	}
	for i := range allEdges { // メモリーグループ単位でSourceID, TargetIDがユニークになるように連結する（KuzuDBでは複合ユニークキーが作れないため）
		allEdges[i].SourceID = fmt.Sprintf("%s%s%s", strings.TrimSpace(allEdges[i].SourceID), consts.ID_MEMORY_GROUP_SEPARATOR, t.MemoryGroup)
		allEdges[i].TargetID = fmt.Sprintf("%s%s%s", strings.TrimSpace(allEdges[i].TargetID), consts.ID_MEMORY_GROUP_SEPARATOR, t.MemoryGroup)
		allEdges[i].MemoryGroup = t.MemoryGroup
		allEdges[i].Weight = 1.0
		allEdges[i].Confidence = 1.0
	}
	return &storage.CognifyOutput{
		Chunks: chunks,
		GraphData: &storage.GraphData{
			Nodes: allNodes,
			Edges: allEdges,
		},
	}, totalUsage, nil
}

// cleanJSON は、LLMの出力から最初の{から最後の}までのJSON部分を抽出します。
// オブジェクト型のJSON部分だけを確実に取り出します。
func cleanJSON(content string) string {
	// 最初の { を探す
	firstBrace := strings.Index(content, "{")
	if firstBrace == -1 {
		return "{}" // { が見つからない場合は空オブジェクトを返す
	}
	// 最後の } を探す
	lastBrace := strings.LastIndex(content, "}")
	if lastBrace == -1 || lastBrace < firstBrace {
		return "{}" // } が見つからない、または位置関係が不正な場合は空オブジェクトを返す
	}
	// { から } までを切り取る（両端を含む）
	return content[firstBrace : lastBrace+1]
}
