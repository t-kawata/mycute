package query

import (
	"context"
	"fmt"

	"github.com/cloudwego/eino/callbacks"
	"github.com/cloudwego/eino/components/embedding"
	"github.com/t-kawata/mycute/pkg/cuber/types"
	"github.com/t-kawata/mycute/pkg/cuber/utils"
)

// EinoEmbedderAdapter は Eino の Embedder を Cuber の Embedder インターフェースに適合させます。
type EinoEmbedderAdapter struct {
	embedder  embedding.Embedder
	modelName string
}

// NewEinoEmbedderAdapter は新しい Eino ベースの Embedder アダプターを作成します。
func NewEinoEmbedderAdapter(emb embedding.Embedder, modelName string) *EinoEmbedderAdapter {
	return &EinoEmbedderAdapter{
		embedder:  emb,
		modelName: modelName,
	}
}

// EmbedQuery はテキストをベクトル化し、トークン使用量を返します。
// storage.Embedder インターフェースを満たします。
func (a *EinoEmbedderAdapter) EmbedQuery(ctx context.Context, text string) ([]float32, types.TokenUsage, error) {
	// トークン集計器の作成
	agg := utils.NewTokenUsageAggregator(a.modelName)

	// Callback の注入
	runInfo := &callbacks.RunInfo{
		Name: "Embedder",
		Type: string(types.MODEL_TYPE_EMBEDDING),
	}
	ctx = callbacks.InitCallbacks(ctx, runInfo, agg.Handler())

	// Embeddings の実行
	vectors, err := a.embedder.EmbedStrings(ctx, []string{text})
	if err != nil {
		return nil, agg.TotalUsage, fmt.Errorf("eino embed error: %w", err)
	}

	if len(vectors) == 0 {
		return nil, agg.TotalUsage, fmt.Errorf("no embeddings returned")
	}

	// 型変換 ([]float64 -> []float32)
	// Einoの仕様上、通常はfloat64で返却されます
	resultVector := make([]float32, len(vectors[0]))
	for i, v := range vectors[0] {
		resultVector[i] = float32(v)
	}

	return resultVector, agg.TotalUsage, nil
}
