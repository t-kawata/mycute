// Package storage は、チャンクとグラフデータをデータベースに保存するタスクを提供します。
// このタスクは、KuzuDBにデータを保存します。
package storage

import (
	"context"
	"fmt"

	"github.com/t-kawata/mycute/pkg/cuber/pipeline"
	"github.com/t-kawata/mycute/pkg/cuber/storage"
	"github.com/t-kawata/mycute/pkg/cuber/types"
	"github.com/t-kawata/mycute/pkg/cuber/utils"
)

// StorageTask は、ストレージタスクを表します。
// チャンク、グラフ、エンティティのインデックスをデータベースに保存します。
type StorageTask struct {
	VectorStorage storage.VectorStorage // ベクトルストレージ（KuzuDB）
	GraphStorage  storage.GraphStorage  // グラフストレージ（KuzuDB）
	Embedder      storage.Embedder      // Embedder
	memoryGroup   string                // メモリーグループ
}

// NewStorageTask は、新しいStorageTaskを作成します。
func NewStorageTask(vectorStorage storage.VectorStorage, graphStorage storage.GraphStorage, embedder storage.Embedder, memoryGroup string) *StorageTask {
	return &StorageTask{
		VectorStorage: vectorStorage,
		GraphStorage:  graphStorage,
		Embedder:      embedder,
		memoryGroup:   memoryGroup,
	}
}

var _ pipeline.Task = (*StorageTask)(nil)

// Run は、ストレージタスクを実行します。
// この関数は以下の処理を行います：
//  1. チャンクをKuzuDBに保存
//  2. ノードとエッジをKuzuDBに保存
//  3. エンティティ名のembeddingを生成してKuzuDBに保存
func (t *StorageTask) Run(ctx context.Context, input any) (any, types.TokenUsage, error) {
	var totalUsage types.TokenUsage
	output, ok := input.(*storage.CognifyOutput)
	if !ok {
		return nil, totalUsage, fmt.Errorf("Storage: Expected *storage.CognifyOutput input, got %T", input)
	}
	// ========================================
	// 1. チャンク（ベクトル）を保存
	// ========================================
	for _, chunk := range output.Chunks {
		// embeddingが空の場合は再生成
		// 堅牢性のため、保存タスクで再生成を試みます
		if len(chunk.Embedding) == 0 {
			fmt.Printf("Warning: embedding missing for chunk %s. Regenerating...\n", chunk.ID)
			embedding, u, err := t.Embedder.EmbedQuery(ctx, chunk.Text)
			totalUsage.Add(u)
			if err != nil {
				return nil, totalUsage, fmt.Errorf("Storage: Failed to regenerate embedding for chunk %s: %w", chunk.ID, err)
			}
			chunk.Embedding = embedding
		}
		if err := t.VectorStorage.SaveChunk(ctx, chunk); err != nil {
			return nil, totalUsage, fmt.Errorf("Storage: Failed to save chunk %s: %w", chunk.ID, err)
		}
	}
	fmt.Printf("Storage: Saved %d chunks to KuzuDB\n", len(output.Chunks))
	// ========================================
	// 2. グラフ（ノード/エッジ）を保存
	// ========================================
	if output.GraphData == nil {
		output.GraphData = &storage.GraphData{}
	}
	// SPECIAL_NODE_TYPE_DOCUMENT_CHUNK ノードをグラフに追加
	// これにより、Memify 等のタスクでチャンクを参照できるようになります
	var chunkNodes []*storage.Node
	for _, chunk := range output.Chunks {
		chunkNode := &storage.Node{
			ID:          chunk.ID,
			MemoryGroup: t.memoryGroup,
			Type:        string(types.SPECIAL_NODE_TYPE_DOCUMENT_CHUNK),
			Properties: map[string]any{
				"text":        chunk.Text,
				"document_id": chunk.DocumentID,
				"chunk_index": chunk.ChunkIndex,
			},
		}
		chunkNodes = append(chunkNodes, chunkNode)
	}
	output.GraphData.Nodes = append(output.GraphData.Nodes, chunkNodes...)
	if output.GraphData != nil {
		// ノードを保存
		if err := t.GraphStorage.AddNodes(ctx, output.GraphData.Nodes); err != nil {
			return nil, totalUsage, fmt.Errorf("Storage: Failed to add nodes: %w", err)
		}
		// エッジを保存
		if err := t.GraphStorage.AddEdges(ctx, output.GraphData.Edges); err != nil {
			return nil, totalUsage, fmt.Errorf("Storage: Failed to add edges: %w", err)
		}
		fmt.Printf("Saved %d nodes and %d edges to KuzuDB\n", len(output.GraphData.Nodes), len(output.GraphData.Edges))
		// ========================================
		// 3. ノードのインデックス化（エンティティ名のembedding）
		// ========================================
		fmt.Printf("Indexing %d nodes...\n", len(output.GraphData.Nodes))
		for _, node := range output.GraphData.Nodes {
			// SPECIAL_NODE_TYPE_DOCUMENT_CHUNK は、エンティティではないのでスキップ
			if node.Type == string(types.SPECIAL_NODE_TYPE_DOCUMENT_CHUNK) {
				continue
			}
			// ノードの"name"プロパティを取得
			var name string
			if nameInterface, ok := node.Properties["name"]; ok {
				name, _ = nameInterface.(string)
			}
			if name == "" {
				name = node.ID // フォールバック: IDを使用
			}
			// IDのメモリーグループを除去
			name = utils.GetNameStrByGraphNodeID(name)
			if name == "" {
				continue
			}
			fmt.Printf("Getting embedding for node %s...\n", name)
			// エンティティ名のembeddingを生成
			embedding, u, err := t.Embedder.EmbedQuery(ctx, name)
			totalUsage.Add(u)
			if err != nil {
				fmt.Printf("Storage: Warning: failed to embed node %s: %v\n", name, err)
				continue
			}
			fmt.Printf("Saving embedding for node %s...\n", name)
			// KuzuDBに保存
			if err := t.VectorStorage.SaveEmbedding(ctx, types.TABLE_NAME_ENTITY, node.ID, name, embedding, t.memoryGroup); err != nil {
				return nil, totalUsage, fmt.Errorf("Storage: Failed to save node embedding: %w", err)
			}
		}
	}
	return output, totalUsage, nil
}
