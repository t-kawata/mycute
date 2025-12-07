package metacognition

import (
	"context"
	"encoding/json"
	"fmt"

	"github.com/tmc/langchaingo/llms"

	"mycute/pkg/cognee/prompts"
	"mycute/pkg/cognee/storage"
)

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
}

// NewGraphRefinementTask は、新しいGraphRefinementTaskを作成します。
func NewGraphRefinementTask(
	graphStorage storage.GraphStorage,
	llm llms.Model,
	groupID string,
) *GraphRefinementTask {
	return &GraphRefinementTask{
		GraphStorage: graphStorage,
		LLM:          llm,
		GroupID:      groupID,
	}
}

// RefineEdges は、新しいルールに基づいてエッジを再評価します。
// 1. 新しいルールを受け取る
// 2. 関連するエッジを取得
// 3. LLMでエッジの妥当性を評価
// 4. 評価結果に基づいてエッジを更新/削除
func (t *GraphRefinementTask) RefineEdges(ctx context.Context, newRules []string) error {
	if len(newRules) == 0 {
		return nil
	}

	// 新しいルール関連のノードIDを取得
	// TODO: 実装（ルールテキストから関連ノードを推定）
	// 現時点では、ルールに関連するノードを特定するのが難しいため、
	// 全エッジをスキャンするか、重要なノード（PageRank等）周辺のエッジを評価するなどの戦略が必要。
	// ここではプレースホルダーとして何もしない。

	// 関連するエッジを取得
	// edges, err := t.GraphStorage.GetEdgesByNode(ctx, nodeID, t.GroupID)

	// LLMでエッジを評価
	// evaluations, err := t.evaluateEdges(ctx, edges, newRules)

	// 評価結果に基づいてエッジを更新
	// for _, eval := range evaluations {
	//     switch eval.Action {
	//     case "strengthen":
	//         t.GraphStorage.UpdateEdgeWeight(ctx, eval.SourceID, eval.TargetID, t.GroupID, eval.NewWeight)
	//     case "weaken":
	//         t.GraphStorage.UpdateEdgeWeight(ctx, eval.SourceID, eval.TargetID, t.GroupID, eval.NewWeight)
	//     case "delete":
	//         t.GraphStorage.DeleteEdge(ctx, eval.SourceID, eval.TargetID, t.GroupID)
	//     }
	// }

	return nil
}

// evaluateEdges は、LLMを使用してエッジの妥当性を評価します。
func (t *GraphRefinementTask) evaluateEdges(ctx context.Context, edges []*storage.Edge, rules []string) ([]EdgeEvaluation, error) {
	// エッジ情報をテキスト化
	edgeTexts := ""
	for _, e := range edges {
		edgeTexts += fmt.Sprintf("- %s -> %s (type: %s)\n", e.SourceID, e.TargetID, e.Type)
	}

	// ルールを結合
	rulesText := ""
	for _, r := range rules {
		rulesText += fmt.Sprintf("- %s\n", r)
	}

	prompt := fmt.Sprintf(`Based on the following new rules/insights:
%s

Evaluate these existing edges and decide if they should be strengthened, weakened, deleted, or kept as-is:
%s`, rulesText, edgeTexts)

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
