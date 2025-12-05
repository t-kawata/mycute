// Package storage は、チャンクとグラフデータをデータベースに保存するタスクを提供します。
// このタスクは、DuckDB（ベクトル）とCozoDB（グラフ）の両方にデータを保存します。
package storage

import (
	"context"
	"fmt"

	"mycute/pkg/cognee/pipeline"
	"mycute/pkg/cognee/storage"
)

// StorageTask は、ストレージタスクを表します。
// チャンク、グラフ、エンティティのインデックスをデータベースに保存します。
type StorageTask struct {
	VectorStorage storage.VectorStorage // ベクトルストレージ（DuckDB）
	GraphStorage  storage.GraphStorage  // グラフストレージ（CozoDB）
	Embedder      storage.Embedder      // Embedder
	groupID       string                // グループID
}

// NewStorageTask は、新しいStorageTaskを作成します。
func NewStorageTask(vectorStorage storage.VectorStorage, graphStorage storage.GraphStorage, embedder storage.Embedder, groupID string) *StorageTask {
	return &StorageTask{
		VectorStorage: vectorStorage,
		GraphStorage:  graphStorage,
		Embedder:      embedder,
		groupID:       groupID,
	}
}

var _ pipeline.Task = (*StorageTask)(nil)

// Run は、ストレージタスクを実行します。
// この関数は以下の処理を行います：
//  1. チャンクをDuckDBに保存
//  2. ノードとエッジをCozoDBに保存
//  3. エンティティ名のembeddingを生成してDuckDBに保存
func (t *StorageTask) Run(ctx context.Context, input any) (any, error) {
	output, ok := input.(*storage.CognifyOutput)
	if !ok {
		return nil, fmt.Errorf("expected *storage.CognifyOutput input, got %T", input)
	}

	// ========================================
	// 1. チャンク（ベクトル）を保存
	// ========================================
	for _, chunk := range output.Chunks {
		// embeddingが空の場合は再生成
		// 堅牢性のため、保存タスクで再生成を試みます
		if len(chunk.Embedding) == 0 {
			fmt.Printf("Warning: embedding missing for chunk %s. Regenerating...\n", chunk.ID)
			embedding, err := t.Embedder.EmbedQuery(ctx, chunk.Text)
			if err != nil {
				return nil, fmt.Errorf("failed to regenerate embedding for chunk %s: %w", chunk.ID, err)
			}
			chunk.Embedding = embedding
		}
		if err := t.VectorStorage.SaveChunk(ctx, chunk); err != nil {
			return nil, fmt.Errorf("failed to save chunk %s: %w", chunk.ID, err)
		}
	}
	fmt.Printf("Saved %d chunks to DuckDB\n", len(output.Chunks))

	// ========================================
	// 2. グラフ（ノード/エッジ）を保存
	// ========================================
	if output.GraphData != nil {
		// ノードを保存
		if err := t.GraphStorage.AddNodes(ctx, output.GraphData.Nodes); err != nil {
			return nil, fmt.Errorf("failed to add nodes: %w", err)
		}
		// エッジを保存
		if err := t.GraphStorage.AddEdges(ctx, output.GraphData.Edges); err != nil {
			return nil, fmt.Errorf("failed to add edges: %w", err)
		}
		fmt.Printf("Saved %d nodes and %d edges to CozoDB\n", len(output.GraphData.Nodes), len(output.GraphData.Edges))

		// ========================================
		// 3. ノードのインデックス化（エンティティ名のembedding）
		// ========================================
		fmt.Printf("Indexing %d nodes...\n", len(output.GraphData.Nodes))
		for _, node := range output.GraphData.Nodes {
			// ノードの"name"プロパティを取得
			var name string
			if nameInterface, ok := node.Properties["name"]; ok {
				name, _ = nameInterface.(string)
			}
			if name == "" {
				name = node.ID // フォールバック: IDを使用
			}

			if name == "" {
				continue
			}

			// エンティティ名のembeddingを生成
			embedding, err := t.Embedder.EmbedQuery(ctx, name)
			if err != nil {
				fmt.Printf("Warning: failed to embed node %s: %v\n", name, err)
				continue
			}

			// DuckDBに保存（コレクション: "Entity_name"）
			if err := t.VectorStorage.SaveEmbedding(ctx, "Entity_name", node.ID, name, embedding, t.groupID); err != nil {
				return nil, fmt.Errorf("failed to save node embedding: %w", err)
			}
		}
	}

	return output, nil
}
