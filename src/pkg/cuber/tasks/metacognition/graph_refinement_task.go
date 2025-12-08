package metacognition

import (
	"context"
	"encoding/json"
	"fmt"

	"github.com/tmc/langchaingo/llms"

	"mycute/pkg/cuber/prompts"
	"mycute/pkg/cuber/storage"
)

// MetabolismConfig は、グラフ代謝のパラメータを保持します。
type MetabolismConfig struct {
	Alpha          float64 // 強化学習率: 支持された時のConfidence上昇幅
	Delta          float64 // 減衰ペナルティ: 矛盾した時のConfidence減少率
	PruneThreshold float64 // 淘汰閾値: S = W * C がこれを下回ると削除
}

// EdgeEvaluation は、エッジの評価結果を表します。
type EdgeEvaluation struct {
	SourceID  string  `json:"source_id"`
	TargetID  string  `json:"target_id"`
	Action    string  `json:"action"`     // "strengthen", "weaken", "delete", "keep"
	NewWeight float64 `json:"new_weight"` // 新しい重み（0.0〜1.0）
	Reason    string  `json:"reason"`
}

// EdgeEvaluationSet は、LLMから返されるエッジ評価のリストです。
type EdgeEvaluationSet struct {
	Evaluations []EdgeEvaluation `json:"evaluations"`
}

// GraphRefinementTask は、グラフのエッジを再評価・更新するタスクです。
type GraphRefinementTask struct {
	GraphStorage storage.GraphStorage
	LLM          llms.Model
	GroupID      string
	Config       MetabolismConfig
}

// NewGraphRefinementTask は、新しいGraphRefinementTaskを作成します。
func NewGraphRefinementTask(
	graphStorage storage.GraphStorage,
	llm llms.Model,
	groupID string,
	alpha, delta, pruneThreshold float64,
) *GraphRefinementTask {
	return &GraphRefinementTask{
		GraphStorage: graphStorage,
		LLM:          llm,
		GroupID:      groupID,
		Config: MetabolismConfig{
			Alpha:          alpha,
			Delta:          delta,
			PruneThreshold: pruneThreshold,
		},
	}
}

// RefineEdges は、新しいルールに基づいてエッジを再評価します。
// 1. 新しいルールを受け取る
// 2. 関連するエッジを取得
// 3. LLMでエッジの妥当性を評価
// 4. 評価結果に基づいてエッジを更新/削除（代謝モデル適用）
func (t *GraphRefinementTask) RefineEdges(ctx context.Context, newRules []string, targetNodeIDs []string) error {
	if len(newRules) == 0 {
		return nil
	}

	// ターゲットノードが指定されていない場合は何もしない
	if len(targetNodeIDs) == 0 {
		fmt.Println("GraphRefinementTask: No target nodes specified, skipping")
		return nil
	}

	// 各ターゲットノードのエッジを処理
	for _, nodeID := range targetNodeIDs {
		edges, err := t.GraphStorage.GetEdgesByNode(ctx, nodeID, t.GroupID)
		if err != nil {
			fmt.Printf("GraphRefinementTask: Failed to get edges for node %s: %v\n", nodeID, err)
			continue
		}

		if len(edges) == 0 {
			continue
		}

		// LLMでエッジを評価
		evaluations, err := t.evaluateEdges(ctx, edges, newRules)
		if err != nil {
			fmt.Printf("GraphRefinementTask: Failed to evaluate edges: %v\n", err)
			continue
		}

		// 評価結果に基づいてエッジを更新（代謝モデル適用）
		for _, eval := range evaluations {
			if err := t.applyMetabolism(ctx, eval); err != nil {
				fmt.Printf("GraphRefinementTask: Failed to apply metabolism: %v\n", err)
			}
		}
	}

	return nil
}

// applyMetabolism は、評価結果に基づいて代謝モデルを適用します。
// S = W × C が PruneThreshold を下回ると、エッジを削除します。
func (t *GraphRefinementTask) applyMetabolism(ctx context.Context, eval EdgeEvaluation) error {
	// 現在のエッジを取得して現在値を確認
	edges, err := t.GraphStorage.GetEdgesByNode(ctx, eval.SourceID, t.GroupID)
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
		fmt.Printf("GraphRefinementTask: Strengthening edge %s -> %s (C: %.2f -> %.2f)\n",
			eval.SourceID, eval.TargetID, currentEdge.Confidence, newConfidence)

	case "weaken":
		// Confidence を Delta 分だけ減少（最小0.0）
		newConfidence = max(0.0, currentEdge.Confidence-t.Config.Delta)
		fmt.Printf("GraphRefinementTask: Weakening edge %s -> %s (C: %.2f -> %.2f)\n",
			eval.SourceID, eval.TargetID, currentEdge.Confidence, newConfidence)

	case "delete":
		// 直接削除
		if err := t.GraphStorage.DeleteEdge(ctx, eval.SourceID, eval.TargetID, t.GroupID); err != nil {
			return err
		}
		fmt.Printf("GraphRefinementTask: Deleted edge %s -> %s (reason: %s)\n",
			eval.SourceID, eval.TargetID, eval.Reason)
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
		if err := t.GraphStorage.DeleteEdge(ctx, eval.SourceID, eval.TargetID, t.GroupID); err != nil {
			return err
		}
		fmt.Printf("GraphRefinementTask: Pruned edge %s -> %s (S=%.3f < threshold=%.3f)\n",
			eval.SourceID, eval.TargetID, survivalScore, t.Config.PruneThreshold)
		return nil
	}

	// エッジのメトリクスを更新
	return t.GraphStorage.UpdateEdgeMetrics(ctx, eval.SourceID, eval.TargetID, t.GroupID, newWeight, newConfidence)
}

// evaluateEdges は、LLMを使用してエッジの妥当性を評価します。
func (t *GraphRefinementTask) evaluateEdges(ctx context.Context, edges []*storage.Edge, rules []string) ([]EdgeEvaluation, error) {
	// エッジ情報をテキスト化
	edgeTexts := ""
	for _, e := range edges {
		edgeTexts += fmt.Sprintf("- %s -> %s (type: %s, weight: %.2f, confidence: %.2f)\n",
			e.SourceID, e.TargetID, e.Type, e.Weight, e.Confidence)
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
    {"source_id": "...", "target_id": "...", "action": "strengthen|weaken|delete|keep", "new_weight": 0.0-1.0, "reason": "..."}
  ]
}`, rulesText, edgeTexts)

	response, err := t.LLM.GenerateContent(ctx, []llms.MessageContent{
		llms.TextParts(llms.ChatMessageTypeSystem, prompts.EdgeEvaluationSystemPrompt),
		llms.TextParts(llms.ChatMessageTypeHuman, prompt),
	})
	if err != nil {
		return nil, err
	}

	if len(response.Choices) == 0 {
		return nil, fmt.Errorf("no response from LLM")
	}

	var result EdgeEvaluationSet
	if err := json.Unmarshal([]byte(extractJSON(response.Choices[0].Content)), &result); err != nil {
		return nil, err
	}

	return result.Evaluations, nil
}
