package metacognition

import (
	"context"
	"encoding/json"
	"fmt"

	"github.com/cloudwego/eino/components/model"

	"github.com/t-kawata/mycute/lib/common"
	"github.com/t-kawata/mycute/pkg/cuber/prompts"
	"github.com/t-kawata/mycute/pkg/cuber/storage"
	"github.com/t-kawata/mycute/pkg/cuber/types"
	"github.com/t-kawata/mycute/pkg/cuber/utils"
	"go.uber.org/zap"
)

// MetabolismConfig は、グラフ代謝のパラメータを保持します。
type MetabolismConfig struct {
	Alpha          float64 // 強化学習率: 支持された時のConfidence上昇幅
	Delta          float64 // 減衰ペナルティ: 矛盾した時のConfidence減少率
	PruneThreshold float64 // 淘汰閾値: S = W * C がこれを下回ると削除
}

// EdgeEvaluation は、エッジの評価結果を表します。
// EdgeIndex はLLMとのやり取りで使用するインデックス（evaluateEdges内で実際のエッジにマッピング）
type EdgeEvaluation struct {
	EdgeIndex int     `json:"edge_index"` // エッジのインデックス（0始まり）
	Action    string  `json:"action"`     // "strengthen", "weaken", "delete", "keep"
	NewWeight float64 `json:"new_weight"` // 新しい重み（0.0〜1.0）
	Reason    string  `json:"reason"`
	// 以下は内部使用（LLMからは返されず、evaluateEdges内で設定）
	SourceID string `json:"-"` // JSON出力から除外
	TargetID string `json:"-"` // JSON出力から除外
}

// EdgeEvaluationSet は、LLMから返されるエッジ評価のリストです。
type EdgeEvaluationSet struct {
	Evaluations []EdgeEvaluation `json:"evaluations"`
}

// GraphRefinementTask は、グラフのエッジを再評価・更新するタスクです。
type GraphRefinementTask struct {
	GraphStorage storage.GraphStorage
	LLM          model.ToolCallingChatModel // Eino ChatModel
	MemoryGroup  string
	Config       MetabolismConfig
	ModelName    string
	Logger       *zap.Logger
}

// NewGraphRefinementTask は、新しいGraphRefinementTaskを作成します。
func NewGraphRefinementTask(
	graphStorage storage.GraphStorage,
	llm model.ToolCallingChatModel,
	memoryGroup string,
	alpha, delta, pruneThreshold float64,
	modelName string,
	l *zap.Logger,
) *GraphRefinementTask {
	if modelName == "" {
		modelName = "gpt-4"
	}
	return &GraphRefinementTask{
		GraphStorage: graphStorage,
		LLM:          llm,
		MemoryGroup:  memoryGroup,
		Config: MetabolismConfig{
			Alpha:          alpha,
			Delta:          delta,
			PruneThreshold: pruneThreshold,
		},
		ModelName: modelName,
		Logger:    l,
	}
}

// RefineEdges は、新しいルールに基づいてエッジを再評価します。
// 1. 新しいルールを受け取る
// 2. 関連するエッジを取得
// 3. LLMでエッジの妥当性を評価
// 4. 評価結果に基づいてエッジを更新/削除（代謝モデル適用）
func (t *GraphRefinementTask) RefineEdges(ctx context.Context, newRules []string, targetNodeIDs []string) (types.TokenUsage, error) {
	var totalUsage types.TokenUsage
	if len(newRules) == 0 {
		return totalUsage, nil
	}

	// ターゲットノードが指定されていない場合は何もしない
	if len(targetNodeIDs) == 0 {
		utils.LogDebug(t.Logger, "GraphRefinementTask: No target nodes specified, skipping")
		return totalUsage, nil
	}

	// 各ターゲットノードのエッジを処理
	for _, nodeID := range targetNodeIDs {
		edges, err := t.GraphStorage.GetEdgesByNode(ctx, nodeID, t.MemoryGroup)
		if err != nil {
			utils.LogWarn(t.Logger, "GraphRefinementTask: Failed to get edges for node", zap.String("node_id", nodeID), zap.Error(err))
			continue
		}

		if len(edges) == 0 {
			continue
		}

		// LLMでエッジを評価
		evaluations, usage, err := t.evaluateEdges(ctx, edges, newRules)
		totalUsage.Add(usage)
		if err != nil {
			utils.LogWarn(t.Logger, "GraphRefinementTask: Failed to evaluate edges", zap.Error(err))
			continue
		}

		// 評価結果に基づいてエッジを更新（代謝モデル適用）
		for _, eval := range evaluations {
			if err := t.applyMetabolism(ctx, eval); err != nil {
				utils.LogWarn(t.Logger, "GraphRefinementTask: Failed to apply metabolism", zap.Error(err))
			}
		}
	}

	return totalUsage, nil
}

// applyMetabolism は、評価結果に基づいて代謝モデルを適用します。
// S = W × C が PruneThreshold を下回ると、エッジを削除します。
func (t *GraphRefinementTask) applyMetabolism(ctx context.Context, eval EdgeEvaluation) error {
	// 現在のエッジを取得して現在値を確認
	edges, err := t.GraphStorage.GetEdgesByNode(ctx, eval.SourceID, t.MemoryGroup)
	if err != nil {
		return err
	}

	var currentEdge *storage.Edge
	for _, e := range edges {
		if e.SourceID == eval.SourceID && e.TargetID == eval.TargetID {
			currentEdge = e
			break
		}
	}

	if currentEdge == nil {
		return fmt.Errorf("edge not found: %s -> %s", eval.SourceID, eval.TargetID)
	}

	var newWeight, newConfidence float64
	newWeight = currentEdge.Weight
	newConfidence = currentEdge.Confidence

	switch eval.Action {
	case "strengthen":
		// Confidence を Alpha 分だけ増加（最大1.0）
		newConfidence = min(1.0, currentEdge.Confidence+t.Config.Alpha)
		// Weight も若干増加
		newWeight = min(1.0, currentEdge.Weight+t.Config.Alpha*0.5)
		utils.LogDebug(t.Logger, "GraphRefinementTask: Strengthening edge", zap.String("from", eval.SourceID), zap.String("to", eval.TargetID), zap.Float64("old_conf", currentEdge.Confidence), zap.Float64("new_conf", newConfidence))

	case "weaken":
		// Confidence を Delta 分だけ減少（最小0.0）
		newConfidence = max(0.0, currentEdge.Confidence-t.Config.Delta)
		utils.LogDebug(t.Logger, "GraphRefinementTask: Weakening edge", zap.String("from", eval.SourceID), zap.String("to", eval.TargetID), zap.Float64("old_conf", currentEdge.Confidence), zap.Float64("new_conf", newConfidence))

	case "delete":
		// 直接削除
		if err := t.GraphStorage.DeleteEdge(ctx, eval.SourceID, currentEdge.Type, eval.TargetID, t.MemoryGroup); err != nil {
			return err
		}
		utils.LogDebug(t.Logger, "GraphRefinementTask: Deleted edge", zap.String("from", eval.SourceID), zap.String("to", eval.TargetID), zap.String("reason", eval.Reason))
		return nil

	case "keep":
		// 何もしない
		return nil

	default:
		return nil
	}

	// 生存スコアを計算: S = W × C
	survivalScore := newWeight * newConfidence

	// 淘汰閾値を下回った場合は削除
	if survivalScore < t.Config.PruneThreshold {
		if err := t.GraphStorage.DeleteEdge(ctx, eval.SourceID, currentEdge.Type, eval.TargetID, t.MemoryGroup); err != nil {
			return err
		}
		utils.LogDebug(t.Logger, "GraphRefinementTask: Pruned edge", zap.String("from", eval.SourceID), zap.String("to", eval.TargetID), zap.Float64("score", survivalScore), zap.Float64("threshold", t.Config.PruneThreshold))
		return nil
	}

	// エッジのメトリクスを更新 (現在時刻でタイムスタンプを更新)
	nowUnix := *common.GetNowUnixMilli()
	return t.GraphStorage.UpdateEdgeMetrics(ctx, eval.SourceID, eval.TargetID, t.MemoryGroup, newWeight, newConfidence, nowUnix)

}

// evaluateEdges は、LLMを使用してエッジの妥当性を評価します。
// LLMにはエッジのインデックス（0始まり）を使用し、連結済みIDをLLMに送らないことで
// IDの往復による破損を防ぎます。
func (t *GraphRefinementTask) evaluateEdges(ctx context.Context, edges []*storage.Edge, rules []string) ([]EdgeEvaluation, types.TokenUsage, error) {
	var usage types.TokenUsage

	// エッジ情報をテキスト化（インデックスを使用、IDは送らない）
	edgeTexts := ""
	for i, e := range edges {
		// GetNameStrByGraphNodeID で連結前のIDを取得して表示用に使用
		sourceDisplay := utils.GetNameStrByGraphNodeID(e.SourceID)
		targetDisplay := utils.GetNameStrByGraphNodeID(e.TargetID)
		edgeTexts += fmt.Sprintf("- [%d] %s -> %s (type: %s, weight: %.2f, confidence: %.2f)\n",
			i, sourceDisplay, targetDisplay, e.Type, e.Weight, e.Confidence)
	}

	// ルールを結合
	rulesText := ""
	for _, r := range rules {
		rulesText += fmt.Sprintf("- %s\n", r)
	}

	prompt := fmt.Sprintf(`Based on the following new rules/insights:
%s

Evaluate these existing edges and decide if they should be strengthened, weakened, deleted, or kept as-is:
%s

Respond with JSON in this format:
{
  "evaluations": [
    {"edge_index": 0, "action": "strengthen|weaken|delete|keep", "new_weight": 0.0-1.0, "reason": "..."}
  ]
}

IMPORTANT: Use the edge_index number (0, 1, 2...) shown in brackets to identify each edge.`, rulesText, edgeTexts)

	content, u, err := utils.GenerateWithUsage(ctx, t.LLM, t.ModelName, prompts.EdgeEvaluationSystemPrompt, prompt)
	usage.Add(u)
	if err != nil {
		return nil, usage, err
	}

	if content == "" {
		return nil, usage, fmt.Errorf("GraphRefinementTask: No response from LLM")
	}

	var result EdgeEvaluationSet
	if err := json.Unmarshal([]byte(extractJSON(content)), &result); err != nil {
		return nil, usage, err
	}

	// LLMから返されたEdgeIndexを実際のSourceID/TargetIDにマッピング
	for i := range result.Evaluations {
		idx := result.Evaluations[i].EdgeIndex
		if idx >= 0 && idx < len(edges) {
			result.Evaluations[i].SourceID = edges[idx].SourceID
			result.Evaluations[i].TargetID = edges[idx].TargetID
		} else {
			// 無効なインデックスの場合は警告ログを出力してスキップ可能にする
			utils.LogWarn(t.Logger, "GraphRefinementTask: Invalid edge_index from LLM",
				zap.Int("edge_index", idx),
				zap.Int("max_index", len(edges)-1))
		}
	}

	return result.Evaluations, usage, nil
}
