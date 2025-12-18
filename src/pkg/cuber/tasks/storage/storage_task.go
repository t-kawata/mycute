// Package storage は、チャンクとグラフデータをデータベースに保存するタスクを提供します。
// このタスクは、KuzuDBにデータを保存します。
package storage

import (
	"context"
	"fmt"

	"go.uber.org/zap"

	"github.com/t-kawata/mycute/lib/eventbus"
	"github.com/t-kawata/mycute/pkg/cuber/event"
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
	Logger        *zap.Logger
	EventBus      *eventbus.EventBus
}

// NewStorageTask は、新しいStorageTaskを作成します。
func NewStorageTask(vectorStorage storage.VectorStorage, graphStorage storage.GraphStorage, embedder storage.Embedder, memoryGroup string, l *zap.Logger, eb *eventbus.EventBus) *StorageTask {
	return &StorageTask{
		VectorStorage: vectorStorage,
		GraphStorage:  graphStorage,
		Embedder:      embedder,
		memoryGroup:   memoryGroup,
		Logger:        l,
		EventBus:      eb,
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
	for i, chunk := range output.Chunks {
		// embeddingが空の場合は再生成
		// 堅牢性のため、保存タスクで再生成を試みます
		if len(chunk.Embedding) == 0 {
			utils.LogWarn(t.Logger, "StorageTask: Embedding missing for chunk, regenerating", zap.String("id", chunk.ID))
			embedding, u, err := t.Embedder.EmbedQuery(ctx, chunk.Text)
			totalUsage.Add(u)
			if err != nil {
				return nil, totalUsage, fmt.Errorf("Storage: Failed to regenerate embedding for chunk %s: %w", chunk.ID, err)
			}
			chunk.Embedding = embedding
		}
		// Emit Chunk Save Start
		eventbus.Emit(t.EventBus, string(event.EVENT_ABSORB_STORAGE_CHUNK_START), event.AbsorbStorageChunkStartPayload{
			BasePayload: event.NewBasePayload(t.memoryGroup),
			ChunkID:     chunk.ID,
			ChunkNum:    i + 1,
		})

		if err := t.VectorStorage.SaveChunk(ctx, chunk); err != nil {
			return nil, totalUsage, fmt.Errorf("Storage: Failed to save chunk %s: %w", chunk.ID, err)
		}

		// Emit Chunk Save End
		eventbus.Emit(t.EventBus, string(event.EVENT_ABSORB_STORAGE_CHUNK_END), event.AbsorbStorageChunkEndPayload{
			BasePayload: event.NewBasePayload(t.memoryGroup),
			ChunkID:     chunk.ID,
			ChunkNum:    i + 1,
		})
		utils.LogDebug(t.Logger, "StorageTask: Saved chunk", zap.String("id", chunk.ID))
	}
	utils.LogInfo(t.Logger, "StorageTask: Saved chunks", zap.Int("count", len(output.Chunks)))
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
		// Emit Node Start
		eventbus.Emit(t.EventBus, string(event.EVENT_ABSORB_STORAGE_NODE_START), event.AbsorbStorageNodeStartPayload{
			BasePayload: event.NewBasePayload(t.memoryGroup),
			NodeCount:   len(output.GraphData.Nodes),
		})

		// ノードを保存
		if err := t.GraphStorage.AddNodes(ctx, output.GraphData.Nodes); err != nil {
			return nil, totalUsage, fmt.Errorf("Storage: Failed to add nodes: %w", err)
		}

		// Emit Node End
		eventbus.Emit(t.EventBus, string(event.EVENT_ABSORB_STORAGE_NODE_END), event.AbsorbStorageNodeEndPayload{
			BasePayload: event.NewBasePayload(t.memoryGroup),
			NodeCount:   len(output.GraphData.Nodes),
		})
		utils.LogDebug(t.Logger, "StorageTask: Added nodes batch", zap.Int("count", len(output.GraphData.Nodes)))

		// Emit Edge Start
		eventbus.Emit(t.EventBus, string(event.EVENT_ABSORB_STORAGE_EDGE_START), event.AbsorbStorageEdgeStartPayload{
			BasePayload: event.NewBasePayload(t.memoryGroup),
			EdgeCount:   len(output.GraphData.Edges),
		})

		// エッジを保存
		if err := t.GraphStorage.AddEdges(ctx, output.GraphData.Edges); err != nil {
			return nil, totalUsage, fmt.Errorf("Storage: Failed to add edges: %w", err)
		}

		// Emit Edge End
		eventbus.Emit(t.EventBus, string(event.EVENT_ABSORB_STORAGE_EDGE_END), event.AbsorbStorageEdgeEndPayload{
			BasePayload: event.NewBasePayload(t.memoryGroup),
			EdgeCount:   len(output.GraphData.Edges),
		})

		utils.LogDebug(t.Logger, "StorageTask: Added edges batch", zap.Int("count", len(output.GraphData.Edges)))
		utils.LogInfo(t.Logger, "StorageTask: Saved graph data", zap.Int("nodes", len(output.GraphData.Nodes)), zap.Int("edges", len(output.GraphData.Edges)))
		// ========================================
		// 3. ノードのインデックス化（エンティティ名のembedding）
		// ========================================

		// Emit Node Index Start
		eventbus.Emit(t.EventBus, string(event.EVENT_ABSORB_STORAGE_NODE_INDEX_START), event.AbsorbStorageNodeIndexStartPayload{
			BasePayload: event.NewBasePayload(t.memoryGroup),
			NodeCount:   len(output.GraphData.Nodes),
			EdgeCount:   len(output.GraphData.Edges),
		})

		utils.LogDebug(t.Logger, "StorageTask: Indexing nodes (entity embeddings)", zap.Int("nodes", len(output.GraphData.Nodes)))
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
			// エンティティ名のembeddingを生成
			embedding, u, err := t.Embedder.EmbedQuery(ctx, name)
			totalUsage.Add(u)
			if err != nil {
				utils.LogWarn(t.Logger, "StorageTask: Failed to embed node", zap.String("name", name), zap.Error(err))
				continue
			}
			// KuzuDBに保存
			if err := t.VectorStorage.SaveEmbedding(ctx, types.TABLE_NAME_ENTITY, node.ID, name, embedding, t.memoryGroup); err != nil {
				return nil, totalUsage, fmt.Errorf("Storage: Failed to save node embedding: %w", err)
			}
			utils.LogDebug(t.Logger, "StorageTask: Saved embedding for node", zap.String("name", name), zap.String("id", node.ID))
		}

		// Emit Node Index End
		eventbus.Emit(t.EventBus, string(event.EVENT_ABSORB_STORAGE_NODE_INDEX_END), event.AbsorbStorageNodeIndexEndPayload{
			BasePayload: event.NewBasePayload(t.memoryGroup),
			NodeCount:   len(output.GraphData.Nodes),
			EdgeCount:   len(output.GraphData.Edges),
		})
		utils.LogInfo(t.Logger, "StorageTask: Indexed nodes (entity embeddings)", zap.Int("nodes", len(output.GraphData.Nodes)))
	}
	return output, totalUsage, nil
}
