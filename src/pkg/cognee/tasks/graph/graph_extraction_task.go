// Package graph は、LLMを使用してテキストから知識グラフを抽出するタスクを提供します。
// このタスクは、チャンクからエンティティと関係を抽出し、グラフデータを生成します。
package graph

import (
	"context"
	"encoding/json"
	"fmt"
	"strings"
	"sync"

	"mycute/pkg/cognee/pipeline"
	"mycute/pkg/cognee/prompts"
	"mycute/pkg/cognee/storage"

	"github.com/tmc/langchaingo/llms"
	"golang.org/x/sync/errgroup"
)

// GraphExtractionTask は、グラフ抽出タスクを表します。
// LLMを使用してテキストからエンティティ（ノード）と関係（エッジ）を抽出します。
type GraphExtractionTask struct {
	LLM llms.Model // テキスト生成LLM
}

// NewGraphExtractionTask は、新しいGraphExtractionTaskを作成します。
func NewGraphExtractionTask(llm llms.Model) *GraphExtractionTask {
	return &GraphExtractionTask{LLM: llm}
}

var _ pipeline.Task = (*GraphExtractionTask)(nil)

// Run は、グラフ抽出タスクを実行します。
// 各チャンクに対して並行してLLMを呼び出し、グラフデータを抽出します。
func (t *GraphExtractionTask) Run(ctx context.Context, input any) (any, error) {
	chunks, ok := input.([]*storage.Chunk)
	if !ok {
		return nil, fmt.Errorf("expected []*storage.Chunk input, got %T", input)
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

	for _, chunk := range chunks {
		chunk := chunk // ループ変数をキャプチャ
		g.Go(func() error {
			// ========================================
			// 1. プロンプトを生成
			// ========================================
			// JSON出力を保証するスキーマ指示
			schemaInstructions := `
Return the result as a JSON object with "nodes" and "edges" arrays.
Each node should have "id", "type", and "properties".
Each edge should have "source_id", "target_id", "type", and "properties".
`
			prompt := fmt.Sprintf("%s\n\n%s\n\nText:\n%s", prompts.GenerateGraphPrompt, schemaInstructions, chunk.Text)

			// ========================================
			// 2. LLMを呼び出し
			// ========================================
			resp, err := t.LLM.GenerateContent(ctx, []llms.MessageContent{
				llms.TextParts(llms.ChatMessageTypeHuman, prompt),
			})
			if err != nil {
				return fmt.Errorf("LLM call failed: %w", err)
			}

			if len(resp.Choices) == 0 {
				return fmt.Errorf("no response from LLM")
			}
			content := resp.Choices[0].Content

			// ========================================
			// 3. JSONをパース
			// ========================================
			// LLMがマークダウンコードブロックで返す場合があるのでクリーンアップ
			content = cleanJSON(content)

			var graphData storage.GraphData
			if err := json.Unmarshal([]byte(content), &graphData); err != nil {
				// パースエラーの場合は失敗
				return fmt.Errorf("failed to parse JSON: %w\nContent: %s", err, content)
			}

			// ========================================
			// 4. 結果を集約
			// ========================================
			mu.Lock()
			allNodes = append(allNodes, graphData.Nodes...)
			allEdges = append(allEdges, graphData.Edges...)
			mu.Unlock()

			return nil
		})
	}

	// 全てのgoroutineの完了を待つ
	if err := g.Wait(); err != nil {
		return nil, err
	}

	// CognifyOutputを返す（チャンクとグラフデータを含む）
	return &storage.CognifyOutput{
		Chunks: chunks,
		GraphData: &storage.GraphData{
			Nodes: allNodes,
			Edges: allEdges,
		},
	}, nil
}

// cleanJSON は、LLMの出力からマークダウンコードブロックを削除します。
// LLMが ```json ... ``` のような形式で返すことがあるため、これをクリーンアップします。
func cleanJSON(content string) string {
	content = strings.TrimSpace(content)
	if strings.HasPrefix(content, "```json") {
		content = strings.TrimPrefix(content, "```json")
		content = strings.TrimSuffix(content, "```")
	} else if strings.HasPrefix(content, "```") {
		content = strings.TrimPrefix(content, "```")
		content = strings.TrimSuffix(content, "```")
	}
	return strings.TrimSpace(content)
}
