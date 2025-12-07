package metacognition

import (
	"context"
	"fmt"
	"math" // Added

	// Added, assuming "strings"ithub.com/google/uuid" was a typo and meant to add "strings"
	"github.com/google/uuid"
	"github.com/tmc/langchaingo/llms"

	"mycute/pkg/cognee/prompts"
	"mycute/pkg/cognee/storage"
)

// CrystallizationTask は、類似ノードを統合するタスクです。
type CrystallizationTask struct {
	VectorStorage       storage.VectorStorage
	GraphStorage        storage.GraphStorage
	LLM                 llms.Model
	Embedder            storage.Embedder
	GroupID             string
	SimilarityThreshold float64 // クラスタリング類似度閾値
	MinClusterSize      int     // 最小クラスタサイズ
}

// NewCrystallizationTask は、新しいCrystallizationTaskを作成します。
func NewCrystallizationTask(
	vectorStorage storage.VectorStorage,
	graphStorage storage.GraphStorage,
	llm llms.Model,
	embedder storage.Embedder,
	groupID string,
	similarityThreshold float64,
	minClusterSize int,
) *CrystallizationTask {
	return &CrystallizationTask{
		VectorStorage:       vectorStorage,
		GraphStorage:        graphStorage,
		LLM:                 llm,
		Embedder:            embedder,
		GroupID:             groupID,
		SimilarityThreshold: similarityThreshold,
		MinClusterSize:      minClusterSize,
	}
}

// CrystallizeRules は、類似したルールを統合します。
// 1. 全ルールを取得
// 2. 類似度に基づいてクラスタリング
// 3. 各クラスタを1つの統合ルールにまとめる
// 4. 元のルールを削除し、統合ルールを追加
func (t *CrystallizationTask) CrystallizeRules(ctx context.Context) error {
	// ルールノードを取得
	ruleNodes, err := t.GraphStorage.GetNodesByType(ctx, "Rule", t.GroupID)
	if err != nil {
		return fmt.Errorf("CrystallizationTask: failed to get rules: %w", err)
	}

	if len(ruleNodes) < t.MinClusterSize {
		fmt.Println("CrystallizationTask: Not enough rules to crystallize")
		return nil
	}

	// 類似度クラスタリング
	clusters := t.clusterBySimilarity(ctx, ruleNodes, t.SimilarityThreshold)

	if len(clusters) > 0 {
		fmt.Printf("CrystallizationTask: Found %d clusters from %d rules\n", len(clusters), len(ruleNodes))
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
		crystallized, err := t.mergTexts(ctx, texts)
		if err != nil {
			fmt.Printf("CrystallizationTask: Warning - merge failed: %v\n", err)
			continue
		}

		// 新しい統合ノードを作成
		crystallizedID := uuid.NewSHA1(uuid.NameSpaceOID, []byte("Crystallized:"+crystallized)).String()
		crystallizedNode := &storage.Node{
			ID:      crystallizedID,
			GroupID: t.GroupID,
			Type:    "CrystallizedRule",
			Properties: map[string]any{
				"text":            crystallized,
				"source_node_ids": ids,
			},
		}

		if err := t.GraphStorage.AddNodes(ctx, []*storage.Node{crystallizedNode}); err != nil {
			fmt.Printf("CrystallizationTask: Warning - failed to add crystallized node: %v\n", err)
			continue
		}

		// 元のノードを「統合済み」としてマーク（削除はしない）
		// TODO: GraphStorage.UpdateNodeProperties の実装が必要
		// ここではとりあえずログ出力のみ
		// fmt.Printf("CrystallizationTask: Crystallized %d rules into 1\n", len(cluster))
	}

	return nil
}

// clusterBySimilarity は、ノードを類似度でクラスタリングします。
func (t *CrystallizationTask) clusterBySimilarity(ctx context.Context, nodes []*storage.Node, threshold float64) [][]*storage.Node {
	// 簡易実装: 全探索で類似度計算（O(N^2)）
	// ノード数が多くなると遅くなるため、本番環境ではベクトル検索を活用すべき

	// 1. 各ノードのembeddingを取得（キャッシュされていない場合は再計算）
	// ここではVectorStorageから取得するのが理想だが、APIがないためEmbedderで再計算
	embeddings := make([][]float32, len(nodes))
	for i, node := range nodes {
		text, ok := node.Properties["text"].(string)
		if !ok {
			continue
		}
		emb, err := t.Embedder.EmbedQuery(ctx, text)
		if err != nil {
			continue
		}
		embeddings[i] = emb
	}

	// 2. 隣接行列を作成（類似度が閾値以上ならエッジあり）
	adj := make([][]int, len(nodes))
	for i := 0; i < len(nodes); i++ {
		for j := i + 1; j < len(nodes); j++ {
			if len(embeddings[i]) == 0 || len(embeddings[j]) == 0 {
				continue
			}
			sim := cosineSimilarity(embeddings[i], embeddings[j])
			if sim >= float32(threshold) {
				adj[i] = append(adj[i], j)
				adj[j] = append(adj[j], i)
			}
		}
	}

	// 3. 連結成分分解（BFS/DFS）
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

	return clusters
}

func cosineSimilarity(a, b []float32) float32 {
	if len(a) != len(b) {
		return 0
	}
	var dot, normA, normB float32
	for i := 0; i < len(a); i++ {
		dot += a[i] * b[i]
		normA += a[i] * a[i]
		normB += b[i] * b[i]
	}
	if normA == 0 || normB == 0 {
		return 0
	}
	// sqrt calculation is needed for exact cosine similarity
	// Since we don't want to import math just for Sqrt if possible, or we can use a simple approximation/import math
	// Go's math package is standard.
	return dot / (sqrt(normA) * sqrt(normB))
}

// sqrt helper using math.Sqrt
func sqrt(x float32) float32 {
	return float32(math.Sqrt(float64(x)))
}

// mergTexts は、複数のテキストを1つに統合します。
func (t *CrystallizationTask) mergTexts(ctx context.Context, texts []string) (string, error) {
	prompt := fmt.Sprintf("以下の複数の知識を1つの包括的な記述に統合してください:\n\n%s",
		joinWithNumbers(texts))

	response, err := t.LLM.GenerateContent(ctx, []llms.MessageContent{
		llms.TextParts(llms.ChatMessageTypeSystem, prompts.KnowledgeCrystallizationSystemPrompt),
		llms.TextParts(llms.ChatMessageTypeHuman, prompt),
	})
	if err != nil {
		return "", err
	}

	if len(response.Choices) == 0 {
		return "", fmt.Errorf("no response from LLM")
	}

	return response.Choices[0].Content, nil
}

func joinWithNumbers(texts []string) string {
	var result string
	for i, t := range texts {
		result += fmt.Sprintf("%d. %s\n", i+1, t)
	}
	return result
}
