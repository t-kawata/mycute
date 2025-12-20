package metacognition

import (
	"context"
	"fmt"

	"github.com/cloudwego/eino/components/model"
	"github.com/google/uuid"

	"github.com/t-kawata/mycute/pkg/cuber/prompts"
	"github.com/t-kawata/mycute/pkg/cuber/storage"
	"github.com/t-kawata/mycute/pkg/cuber/types"
	"github.com/t-kawata/mycute/pkg/cuber/utils"
	"go.uber.org/zap"
)

// CrystallizationTask は、類似ノードを統合するタスクです。
type CrystallizationTask struct {
	VectorStorage       storage.VectorStorage
	GraphStorage        storage.GraphStorage
	LLM                 model.ToolCallingChatModel // Eino ChatModel
	Embedder            storage.Embedder
	MemoryGroup         string
	SimilarityThreshold float64 // クラスタリング類似度閾値
	MinClusterSize      int     // 最小クラスタサイズ
	ModelName           string
	Logger              *zap.Logger
}

// NewCrystallizationTask は、新しいCrystallizationTaskを作成します。
func NewCrystallizationTask(
	vectorStorage storage.VectorStorage,
	graphStorage storage.GraphStorage,
	llm model.ToolCallingChatModel,
	embedder storage.Embedder,
	memoryGroup string,
	similarityThreshold float64,
	minClusterSize int,
	modelName string,
	l *zap.Logger,
) *CrystallizationTask {
	if modelName == "" {
		modelName = "gpt-4"
	}
	return &CrystallizationTask{
		VectorStorage:       vectorStorage,
		GraphStorage:        graphStorage,
		LLM:                 llm,
		Embedder:            embedder,
		MemoryGroup:         memoryGroup,
		SimilarityThreshold: similarityThreshold,
		MinClusterSize:      minClusterSize,
		ModelName:           modelName,
		Logger:              l,
	}
}

// CrystallizeRules は、類似したルールを統合します。
// 1. 全ルールを取得
// 2. ベクトル検索に基づいてクラスタリング（Top-K検索による近傍グラフ構築）
// 3. 各クラスタを1つの統合ルールにまとめる
// 4. エッジの付け替え（Re-wiring）
// 5. 元のルールの削除
func (t *CrystallizationTask) CrystallizeRules(ctx context.Context) (types.TokenUsage, error) {
	var totalUsage types.TokenUsage
	// ルールノードを取得
	ruleNodes, err := t.GraphStorage.GetNodesByType(ctx, "Rule", t.MemoryGroup)
	if err != nil {
		return totalUsage, fmt.Errorf("CrystallizationTask: failed to get rules: %w", err)
	}

	if len(ruleNodes) < t.MinClusterSize {
		utils.LogDebug(t.Logger, "CrystallizationTask: Not enough rules to crystallize", zap.Int("count", len(ruleNodes)), zap.Int("required", t.MinClusterSize))
		return totalUsage, nil
	}

	// 類似度クラスタリング（ベクトル検索ベース）
	clusters, usage1 := t.clusterBySimilarity(ctx, ruleNodes, t.SimilarityThreshold)
	totalUsage.Add(usage1)

	if len(clusters) > 0 {
		utils.LogInfo(t.Logger, "CrystallizationTask: Clusters found", zap.Int("clusters", len(clusters)), zap.Int("total_rules", len(ruleNodes)))
	}

	for _, cluster := range clusters {
		if len(cluster) < t.MinClusterSize {
			continue // 単一ノードのクラスタはスキップ
		}

		// クラスタ内のテキストを統合
		texts := make([]string, 0)
		ids := make([]string, 0)
		for _, node := range cluster {
			if text, ok := node.Properties["text"].(string); ok {
				texts = append(texts, text)
				ids = append(ids, node.ID)
			}
		}

		// LLMで統合テキストを生成
		crystallized, usage2, err := t.mergTexts(ctx, texts)
		totalUsage.Add(usage2)
		if err != nil {
			utils.LogWarn(t.Logger, "CrystallizationTask: Merge failed", zap.Error(err))
			continue
		}

		// 新しい統合ノードを作成
		crystallizedID := uuid.NewSHA1(uuid.NameSpaceOID, []byte("Crystallized:"+crystallized)).String()
		crystallizedNode := &storage.Node{
			ID:          crystallizedID,
			MemoryGroup: t.MemoryGroup,
			Type:        "Rule", // 統合後もRuleとして扱う
			Properties: map[string]any{
				"text":            crystallized,
				"source_node_ids": ids,
				"is_crystallized": true,
			},
		}

		// 1. 新しいノードを追加
		if err := t.GraphStorage.AddNodes(ctx, []*storage.Node{crystallizedNode}); err != nil {
			utils.LogWarn(t.Logger, "CrystallizationTask: Failed to add crystallized node", zap.Error(err))
			continue
		}

		// 2. エッジの付け替え (Re-wiring)
		for _, oldNodeID := range ids {
			// Inbound Edges (Others -> Old) => (Others -> New)
			inEdges, err := t.GraphStorage.GetEdgesByNode(ctx, oldNodeID, t.MemoryGroup)
			if err == nil {
				for _, edge := range inEdges {
					if edge.TargetID == oldNodeID {
						// 自分自身へのループエッジは除外
						if edge.SourceID == oldNodeID {
							continue
						}
						// クラスタ内の他のノードからのエッジも除外（内部リンクは解消）
						isInternal := false
						for _, internalID := range ids {
							if edge.SourceID == internalID {
								isInternal = true
								break
							}
						}
						if isInternal {
							continue
						}

						// 新しいエッジを作成
						newEdge := &storage.Edge{
							SourceID:    edge.SourceID,
							TargetID:    crystallizedID,
							MemoryGroup: t.MemoryGroup,
							Type:        edge.Type,
							Properties:  edge.Properties,
							Weight:      edge.Weight,
							Confidence:  edge.Confidence,
						}
						t.GraphStorage.AddEdges(ctx, []*storage.Edge{newEdge})
					}
				}
			}

			// Outbound Edges (Old -> Others) => (New -> Others)
			// GetEdgesByNodeは双方向のエッジを返すため、SourceIDチェックでフィルタリング
			if err == nil {
				for _, edge := range inEdges {
					if edge.SourceID == oldNodeID {
						// 自分自身へのループエッジは除外
						if edge.TargetID == oldNodeID {
							continue
						}
						// クラスタ内の他のノードへのエッジも除外
						isInternal := false
						for _, internalID := range ids {
							if edge.TargetID == internalID {
								isInternal = true
								break
							}
						}
						if isInternal {
							continue
						}

						// 新しいエッジを作成
						newEdge := &storage.Edge{
							SourceID:    crystallizedID,
							TargetID:    edge.TargetID,
							MemoryGroup: t.MemoryGroup,
							Type:        edge.Type,
							Properties:  edge.Properties,
							Weight:      edge.Weight,
							Confidence:  edge.Confidence,
						}
						t.GraphStorage.AddEdges(ctx, []*storage.Edge{newEdge})
					}
				}
			}
		}

		// 3. 元のノードを削除
		for _, oldNodeID := range ids {
			if err := t.GraphStorage.DeleteNode(ctx, oldNodeID, t.MemoryGroup); err != nil {
				utils.LogWarn(t.Logger, "CrystallizationTask: Failed to delete old node", zap.String("node_id", oldNodeID), zap.Error(err))
			}
		}

		utils.LogDebug(t.Logger, "CrystallizationTask: Crystallized rules", zap.Int("rule_count", len(cluster)), zap.String("new_id", crystallizedID))
	}

	return totalUsage, nil
}

// clusterBySimilarity は、ノードを類似度でクラスタリングします。
// ベクトル検索を使用して近傍グラフを構築し、連結成分分解を行います。
//
// Phase-09最適化:
//   - VectorStorageからEmbeddingをバッチ取得（キャッシュ活用）
//   - キャッシュミスの場合のみEmbedderを使用
//   - API呼び出し回数を大幅に削減
func (t *CrystallizationTask) clusterBySimilarity(ctx context.Context, nodes []*storage.Node, threshold float64) ([][]*storage.Node, types.TokenUsage) {
	var usage types.TokenUsage
	if len(nodes) == 0 {
		return nil, usage
	}

	// ========================================
	// Step 1: ノードIDからインデックスへのマップ作成
	// ========================================
	nodeIndex := make(map[string]int)
	nodeIDs := make([]string, len(nodes))
	for i, n := range nodes {
		nodeIndex[n.ID] = i
		nodeIDs[i] = n.ID
	}

	// ========================================
	// Step 2: Embeddingをバッチ取得（キャッシュ活用）
	// ========================================
	// VectorStorageから既存のEmbeddingを一括取得
	// テーブル名は "Rule" を使用（Rule ノードの text フィールドに対応）
	cachedEmbeddings, err := t.VectorStorage.GetEmbeddingsByIDs(ctx, types.TABLE_NAME_RULE, nodeIDs, t.MemoryGroup)
	if err != nil {
		// エラーの場合は空のマップで続行（フォールバックでEmbedderを使用）
		utils.LogWarn(t.Logger, "CrystallizationTask: Failed to fetch cached embeddings", zap.Error(err))
		cachedEmbeddings = make(map[string][]float32)
	}

	// キャッシュヒット率をログ出力
	cacheHitCount := len(cachedEmbeddings)
	utils.LogDebug(t.Logger, "CrystallizationTask: Embedding cache stats", zap.Int("hits", cacheHitCount), zap.Int("total", len(nodes)), zap.Float64("rate", float64(cacheHitCount)/float64(len(nodes))*100))

	// ========================================
	// Step 3: キャッシュミスのノードのみEmbedderで計算
	// ========================================
	embeddings := make(map[string][]float32)
	cacheMissCount := 0

	for _, node := range nodes {
		// キャッシュにあればそれを使用
		if vec, exists := cachedEmbeddings[node.ID]; exists {
			embeddings[node.ID] = vec
			continue
		}

		// キャッシュにない場合はEmbedderで計算
		text, ok := node.Properties["text"].(string)
		if !ok {
			continue
		}

		vec, u, err := t.Embedder.EmbedQuery(ctx, text)
		usage.Add(u)
		if err != nil {
			utils.LogWarn(t.Logger, "CrystallizationTask: Failed to embed text", zap.String("node_id", node.ID), zap.Error(err))
			continue
		}
		embeddings[node.ID] = vec
		cacheMissCount++
	}

	if cacheMissCount > 0 {
		utils.LogDebug(t.Logger, "CrystallizationTask: Cache miss - computed embeddings", zap.Int("count", cacheMissCount))
	}

	// ========================================
	// Step 4: 隣接リストの構築
	// ========================================
	adj := make([][]int, len(nodes))

	for i, node := range nodes {
		vec, exists := embeddings[node.ID]
		if !exists {
			continue
		}

		// VectorStorageで類似検索
		results, err := t.VectorStorage.Query(ctx, types.TABLE_NAME_RULE, vec, 10, t.MemoryGroup)
		if err != nil {
			continue
		}

		for _, res := range results {
			// 類似度が閾値以上かチェック
			// LadybugDBのarray_cosine_similarityは類似度を返す（大きいほど類似）
			// res.Distance >= threshold なら類似
			if res.Distance < threshold {
				continue
			}

			// 検索結果のIDが現在の処理対象ノードリストに含まれているか確認
			if idx, exists := nodeIndex[res.ID]; exists {
				if idx != i { // 自分自身は除外
					adj[i] = append(adj[i], idx)
					adj[idx] = append(adj[idx], i) // 無向グラフとして扱う
				}
			}
		}
	}

	// ========================================
	// Step 5: 連結成分分解（BFS）
	// ========================================
	visited := make([]bool, len(nodes))
	var clusters [][]*storage.Node

	for i := 0; i < len(nodes); i++ {
		if visited[i] {
			continue
		}

		var cluster []*storage.Node
		queue := []int{i}
		visited[i] = true

		for len(queue) > 0 {
			curr := queue[0]
			queue = queue[1:]
			cluster = append(cluster, nodes[curr])

			for _, neighbor := range adj[curr] {
				if !visited[neighbor] {
					visited[neighbor] = true
					queue = append(queue, neighbor)
				}
			}
		}

		if len(cluster) > 0 {
			clusters = append(clusters, cluster)
		}
	}

	return clusters, usage
}

// mergTexts は、複数のテキストを1つに統合します。
func (t *CrystallizationTask) mergTexts(ctx context.Context, texts []string) (string, types.TokenUsage, error) {
	var usage types.TokenUsage
	prompt := fmt.Sprintf("以下の複数の知識を1つの包括的な記述に統合してください:\n\n%s",
		joinWithNumbers(texts))

	content, u, err := utils.GenerateWithUsage(ctx, t.LLM, t.ModelName, prompts.KnowledgeCrystallizationSystemPrompt, prompt)
	usage.Add(u)
	if err != nil {
		return "", usage, err
	}

	if content == "" {
		return "", usage, fmt.Errorf("CrystallizationTask: No response from LLM")
	}

	return content, usage, nil
}

func joinWithNumbers(texts []string) string {
	var result string
	for i, t := range texts {
		result += fmt.Sprintf("%d. %s\n", i+1, t)
	}
	return result
}
