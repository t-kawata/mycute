package metacognition

import (
	"context"
	"fmt"
	"time"

	"github.com/cloudwego/eino/components/model"
	"github.com/google/uuid"

	"github.com/t-kawata/mycute/pkg/cuber/storage"
	"github.com/t-kawata/mycute/pkg/cuber/types"
	"github.com/t-kawata/mycute/pkg/cuber/utils"
	"go.uber.org/zap"
)

// Unknown は、現在答えられない問い・不足情報を表します。
type Unknown struct {
	ID                    string    `json:"id"`
	Text                  string    `json:"text"`
	ResolutionRequirement string    `json:"resolution_requirement"` // 追加: 解決に必要な情報・条件
	Source                string    `json:"source"`
	CreatedAt             time.Time `json:"created_at"`
	MemoryGroup           string    `json:"memory_group"`
}

// Capability は、獲得した能力・知識を表します。
// 複合的な要因はエッジとして表現されるため、構造体には最小限のメタデータのみ保持します。
type Capability struct {
	ID          string    `json:"id"`
	Text        string    `json:"text"`
	AcquiredAt  time.Time `json:"acquired_at"`
	MemoryGroup string    `json:"memory_group"`
}

// IgnoranceManager は、無知と成長のリストを管理します。
type IgnoranceManager struct {
	VectorStorage       storage.VectorStorage
	GraphStorage        storage.GraphStorage
	LLM                 model.ToolCallingChatModel // Eino ChatModel
	Embedder            storage.Embedder
	MemoryGroup         string
	SimilarityThreshold float64 // Unknown解決の類似度閾値
	// ...
	SearchLimit int    // Unknown解決時の検索数
	ModelName   string // モデル名
	Logger      *zap.Logger
}

// NewIgnoranceManager は、新しいIgnoranceManagerを作成します。
func NewIgnoranceManager(
	vectorStorage storage.VectorStorage,
	graphStorage storage.GraphStorage,
	llm model.ToolCallingChatModel,
	embedder storage.Embedder,
	memoryGroup string,
	similarityThreshold float64,
	searchLimit int,
	modelName string,
	l *zap.Logger,
) *IgnoranceManager {
	return &IgnoranceManager{
		VectorStorage:       vectorStorage,
		GraphStorage:        graphStorage,
		LLM:                 llm,
		Embedder:            embedder,
		MemoryGroup:         memoryGroup,
		SimilarityThreshold: similarityThreshold,
		SearchLimit:         searchLimit,
		ModelName:           modelName,
		Logger:              l,
	}
}

// RegisterUnknown は、新しい Unknown をグラフに登録します。
func (m *IgnoranceManager) RegisterUnknown(ctx context.Context, text string, requirement string, source string) (types.TokenUsage, error) {
	var usage types.TokenUsage
	// テキストをVector用に正規化
	normText := utils.NormalizeForVector(text)
	unknownID := uuid.NewSHA1(uuid.NameSpaceOID, []byte("Unknown:"+normText)).String()

	node := &storage.Node{
		ID:          unknownID,
		MemoryGroup: m.MemoryGroup,
		Type:        "Unknown",
		Properties: map[string]any{
			"text":                   normText,
			"resolution_requirement": requirement, // 追加
			"source":                 source,
			"created_at":             time.Now().Format(time.RFC3339),
		},
	}

	if err := m.GraphStorage.AddNodes(ctx, []*storage.Node{node}); err != nil {
		return usage, fmt.Errorf("IgnoranceManager: Failed to register Unknown: %w", err)
	}

	// ベクトル埋め込みを保存
	embedding, u, err := m.Embedder.EmbedQuery(ctx, normText)
	usage.Add(u)
	if err != nil {
		return usage, fmt.Errorf("IgnoranceManager: Failed to embed Unknown: %w", err)
	}

	if err := m.VectorStorage.SaveEmbedding(ctx, types.TABLE_NAME_UNKNOWN, unknownID, normText, embedding, m.MemoryGroup); err != nil {
		return usage, fmt.Errorf("IgnoranceManager: Failed to save Unknown embedding: %w", err)
	}

	utils.LogDebug(m.Logger, "IgnoranceManager: Registered Unknown", zap.String("text", normText), zap.String("requirement", requirement))
	return usage, nil
}

// RegisterCapability は、新しい Capability をグラフに登録します。
// 複数のソース、トリガー、解決済みUnknownをサポートします。
func (m *IgnoranceManager) RegisterCapability(
	ctx context.Context,
	text string,
	triggerTypes []string, // 複数可
	learnedFromUserIDs []string, // 複数可
	learnedFromSources []string, // 複数可
	resolvedUnknownIDs []string, // 複数可
) (types.TokenUsage, error) {
	var usage types.TokenUsage
	// テキストをVector用に正規化
	normText := utils.NormalizeForVector(text)
	capabilityID := uuid.NewSHA1(uuid.NameSpaceOID, []byte("Capability:"+normText)).String()

	node := &storage.Node{
		ID:          capabilityID,
		MemoryGroup: m.MemoryGroup,
		Type:        "Capability",
		Properties: map[string]any{
			"text":        normText,
			"acquired_at": time.Now().Format(time.RFC3339),
		},
	}

	if err := m.GraphStorage.AddNodes(ctx, []*storage.Node{node}); err != nil {
		return usage, fmt.Errorf("IgnoranceManager: failed to register Capability: %w", err)
	}

	// ========================================
	// エッジを作成（多対多の関係を表現）
	// ========================================
	edges := make([]*storage.Edge, 0)

	// 1. learned_from (User)
	for _, userID := range learnedFromUserIDs {
		if userID == "" {
			continue
		}
		edges = append(edges, &storage.Edge{
			SourceID:    capabilityID,
			TargetID:    userID,
			MemoryGroup: m.MemoryGroup,
			Type:        "learned_from",
			Properties:  map[string]any{"source_type": "user"},
		})
	}

	// 2. learned_from (Source/File)
	// ソースファイルもノードとして存在すると仮定、なければプロパティとして保持する設計も検討
	// ここでは簡易的にプロパティではなくエッジとして扱う（ソースノードIDが必要）
	// ※実装時はソースIDの解決ロジックが必要

	// 3. resolves (Unknown)
	for _, unknownID := range resolvedUnknownIDs {
		if unknownID == "" {
			continue
		}
		edges = append(edges, &storage.Edge{
			SourceID:    capabilityID,
			TargetID:    unknownID,
			MemoryGroup: m.MemoryGroup,
			Type:        "resolves",
			Properties:  map[string]any{},
		})
	}

	// 4. triggered_by (TriggerType)
	// トリガータイプをノードプロパティに追加（再設定）
	node.Properties["trigger_types"] = triggerTypes
	node.Properties["learned_from_sources"] = learnedFromSources // ソースも配列で保持

	// ノードを更新（プロパティ追加のため）
	// AddNodesはUpsert動作なので、再度呼び出しても問題ないが、
	// 最初からプロパティに入れておく方が効率的。
	// ここではコードの構造上、後から追加しているが、AddNodesを呼ぶ前に設定済みであれば不要。
	// 上記コードでは AddNodes を既に呼んでいるため、再度呼ぶ必要がある。
	if err := m.GraphStorage.AddNodes(ctx, []*storage.Node{node}); err != nil {
		return usage, fmt.Errorf("IgnoranceManager: failed to update Capability properties: %w", err)
	}

	if len(edges) > 0 {
		if err := m.GraphStorage.AddEdges(ctx, edges); err != nil {
			return usage, fmt.Errorf("IgnoranceManager: failed to add edges: %w", err)
		}
	}

	// ベクトル埋め込みを保存
	embedding, u, err := m.Embedder.EmbedQuery(ctx, text)
	usage.Add(u)
	if err != nil {
		return usage, fmt.Errorf("IgnoranceManager: failed to embed Capability: %w", err)
	}

	if err := m.VectorStorage.SaveEmbedding(ctx, types.TABLE_NAME_CAPABILITY, capabilityID, text, embedding, m.MemoryGroup); err != nil {
		return usage, fmt.Errorf("IgnoranceManager: Failed to save Capability embedding: %w", err)
	}

	utils.LogDebug(m.Logger, "IgnoranceManager: Registered Capability", zap.String("text", text))
	return usage, nil
}

// CheckAndResolveUnknowns は、新しい知識が既存の Unknown を解決するかチェックします。
// Cognify 時に呼び出されます。
func (m *IgnoranceManager) CheckAndResolveUnknowns(
	ctx context.Context,
	newKnowledgeTexts []string,
	userID string,
	source string,
) (types.TokenUsage, error) {
	var totalUsage types.TokenUsage
	// 新しい知識をベクトル化して Unknown との類似度を計算
	for _, knowledgeText := range newKnowledgeTexts {
		// 照合用テキストをVector正規化
		normText := utils.NormalizeForVector(knowledgeText)
		embedding, usage, err := m.Embedder.EmbedQuery(ctx, normText)
		totalUsage.Add(usage)
		if err != nil {
			continue
		}

		// Unknown テーブルから類似度検索
		results, err := m.VectorStorage.Query(ctx, types.TABLE_NAME_UNKNOWN, embedding, m.SearchLimit, m.MemoryGroup)
		if err != nil {
			continue
		}

		// 類似度が高い Unknown を解決済みとしてマーク
		for _, result := range results {
			if result.Distance < m.SimilarityThreshold {
				// Capability として登録
				capabilityText := fmt.Sprintf("「%s」について理解した", result.Text)
				u, err := m.RegisterCapability(
					ctx,
					capabilityText,
					[]string{"cognify"},
					[]string{userID},
					[]string{source},
					[]string{result.ID}, // resolvedUnknownID
				)
				totalUsage.Add(u)
				if err != nil {
					utils.LogWarn(m.Logger, "IgnoranceManager: Failed to register resolved capability", zap.Error(err))
				}
			}
		}
	}

	return totalUsage, nil
}

// GetUnresolvedUnknowns は、まだ解決されていない（Capabilityによって解決済みとマークされていない）Unknownを取得します。
func (m *IgnoranceManager) GetUnresolvedUnknowns(ctx context.Context) ([]*Unknown, error) {
	// GraphStorageは直接クエリ実行メソッドを公開していないため、
	// ここではCozoStorageにキャストして実行するか、GraphStorageに汎用クエリメソッドを追加する必要があります。
	// しかし、GraphStorageインターフェースを変更するのは影響範囲が大きいため、
	// ここではGetNodesByTypeで全Unknownを取得し、メモリ上でフィルタリングする方法をとります。
	// 将来的にはGraphStorageにクエリメソッドを追加することを検討してください。

	// 1. 全Unknownを取得
	nodes, err := m.GraphStorage.GetNodesByType(ctx, "Unknown", m.MemoryGroup)
	if err != nil {
		return nil, fmt.Errorf("IgnoranceManager: Failed to get Unknown nodes: %w", err)
	}

	var unknowns []*Unknown

	// 2. 各Unknownについて、解決済みかどうかチェック
	for _, node := range nodes {
		// 入ってくる "resolves" エッジがあるか確認
		edges, err := m.GraphStorage.GetEdgesByNode(ctx, node.ID, m.MemoryGroup)
		if err != nil {
			continue // エラー時はスキップ
		}

		isResolved := false
		for _, edge := range edges {
			if edge.TargetID == node.ID && edge.Type == "resolves" {
				isResolved = true
				break
			}
		}

		if !isResolved {
			createdAtStr, _ := node.Properties["created_at"].(string)
			createdAt, _ := time.Parse(time.RFC3339, createdAtStr)

			unknowns = append(unknowns, &Unknown{
				ID:                    node.ID,
				Text:                  node.Properties["text"].(string),
				ResolutionRequirement: node.Properties["resolution_requirement"].(string),
				Source:                node.Properties["source"].(string),
				CreatedAt:             createdAt,
				MemoryGroup:           m.MemoryGroup,
			})
		}
	}

	return unknowns, nil
}
