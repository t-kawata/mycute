package memify

import (
	"context"
	"encoding/json"
	"fmt"
	"strings"

	"github.com/google/uuid"
	"github.com/tmc/langchaingo/llms"

	"github.com/t-kawata/mycute/pkg/cuber/prompts"
	"github.com/t-kawata/mycute/pkg/cuber/storage"
	"github.com/t-kawata/mycute/pkg/cuber/types"
)

// Rule は、LLMによって抽出されたコーディングルールを表します。
// Python版の Rule DataPoint に対応します。
type Rule struct {
	Text string `json:"text"` // ルールのテキスト
}

// RuleSet は、LLMから返されるルールのリストです。
// Python版の RuleSet DataPoint に対応します。
type RuleSet struct {
	Rules []Rule `json:"rules"` // 抽出されたルールのリスト
}

// RuleExtractionTask は、テキストバッチからコーディングルールを抽出して
// グラフとベクトルストレージに保存するタスクです。
//
// このタスクは以下の処理を行います：
//  1. バッチテキストをLLMに送信してルールを抽出
//  2. ルールノードをKuzuDBに保存
//  3. ルールとNodeSetの関係エッジを保存
//  4. ルールのベクトルインデックスをKuzuDBに保存
//
// 各バッチの結果はその場で保存されるため、メモリ蓄積を防ぎます。
type RuleExtractionTask struct {
	// VectorStorage はベクトルストレージ（KuzuDB）です
	VectorStorage storage.VectorStorage

	// GraphStorage はグラフストレージ（KuzuDB）です
	GraphStorage storage.GraphStorage

	// LLM はテキスト生成用のLLMクライアントです
	LLM llms.Model

	// Embedder はテキストのベクトル化を行うEmbedderです
	Embedder storage.Embedder

	// MemoryGroup はパーティション分離用のメモリーグループです
	MemoryGroup string

	// RulesNodeSetName はルールセットの名前です（例: "coding_agent_rules"）
	RulesNodeSetName string

	// extractedRulesCount は抽出されたルールの累計数（進捗追跡用）
	extractedRulesCount int

	// ModelName は使用するLLMのモデル名です
	ModelName string
}

// NewRuleExtractionTask は、新しいタスクインスタンスを作成します。
func NewRuleExtractionTask(
	vectorStorage storage.VectorStorage,
	graphStorage storage.GraphStorage,
	llm llms.Model,
	embedder storage.Embedder,
	memoryGroup string,
	rulesNodeSetName string,
	modelName string,
) *RuleExtractionTask {
	if modelName == "" {
		modelName = "gpt-4"
	}
	return &RuleExtractionTask{
		VectorStorage:       vectorStorage,
		GraphStorage:        graphStorage,
		LLM:                 llm,
		Embedder:            embedder,
		MemoryGroup:         memoryGroup,
		RulesNodeSetName:    rulesNodeSetName,
		extractedRulesCount: 0,
		ModelName:           modelName,
	}
}

// ProcessBatch は、テキストバッチからルールを抽出してストレージに保存します。
//
// 処理フロー：
//  1. テキストを結合してLLMに送信
//  2. JSONレスポンスをパース
//  3. ルールノードとエッジを作成
//  4. グラフに保存
//  5. ベクトルインデックスを作成
func (t *RuleExtractionTask) ProcessBatch(ctx context.Context, texts []string) (types.TokenUsage, error) {
	var totalUsage types.TokenUsage
	if len(texts) == 0 {
		return totalUsage, nil
	}

	// ========================================
	// 1. テキストを結合
	// ========================================
	combinedText := strings.Join(texts, "\n\n")

	// ========================================
	// 2. 既存ルールを取得（将来拡張用、現在は空）
	// ========================================
	// TODO: GraphStorageから既存のルールを取得する機能を追加
	existingRules := ""

	// ========================================
	// 3. LLMでルールを抽出
	// ========================================
	userPrompt := fmt.Sprintf(prompts.RuleExtractionUserPromptTemplate, combinedText, existingRules)

	response, err := t.LLM.GenerateContent(ctx, []llms.MessageContent{
		llms.TextParts(llms.ChatMessageTypeSystem, prompts.RuleExtractionSystemPrompt),
		llms.TextParts(llms.ChatMessageTypeHuman, userPrompt),
	})
	if err != nil {
		return totalUsage, fmt.Errorf("RuleExtractionTask: LLM call failed: %w", err)
	}

	// Extract Usage
	if len(response.Choices) > 0 {
		info := response.Choices[0].GenerationInfo
		usage, err := types.ExtractTokenUsage(info, t.ModelName, "RuleExtractionTask", true)
		if err != nil {
			fmt.Printf("RuleExtractionTask: Warning: Token extraction failed: %v\n", err)
		} else {
			totalUsage.Add(usage)
		}
	}

	if len(response.Choices) == 0 {
		fmt.Println("RuleExtractionTask: No response from LLM")
		return totalUsage, nil
	}

	// ========================================
	// 4. JSONをパース
	// ========================================
	responseText := response.Choices[0].Content
	jsonStr := extractJSON(responseText)

	var ruleSet RuleSet
	if err := json.Unmarshal([]byte(jsonStr), &ruleSet); err != nil {
		fmt.Printf("RuleExtractionTask: Warning - failed to parse JSON: %v\n", err)
		fmt.Printf("RuleExtractionTask: Raw response: %s\n", responseText)
		return totalUsage, nil // パースエラーは警告として続行
	}

	if len(ruleSet.Rules) == 0 {
		fmt.Println("RuleExtractionTask: No new rules extracted from this batch")
		return totalUsage, nil
	}

	fmt.Printf("RuleExtractionTask: Extracted %d new rules from batch\n", len(ruleSet.Rules))

	// ========================================
	// 5. NodeSetノードを作成（冪等）
	// ========================================
	// 決定論的IDを生成（同じ名前なら同じID）
	ruleSetNodeID := uuid.NewSHA1(uuid.NameSpaceOID, []byte(t.RulesNodeSetName)).String()
	ruleSetNode := &storage.Node{
		ID:          ruleSetNodeID,
		MemoryGroup: t.MemoryGroup,
		Type:        "NodeSet",
		Properties: map[string]any{
			"name": t.RulesNodeSetName,
		},
	}

	// ========================================
	// 6. ルールノードとエッジを作成
	// ========================================
	nodes := []*storage.Node{ruleSetNode}
	edges := make([]*storage.Edge, 0)

	for _, rule := range ruleSet.Rules {
		// ルールIDを生成（ルールテキストから決定論的に）
		ruleID := uuid.NewSHA1(uuid.NameSpaceOID, []byte(rule.Text)).String()

		ruleNode := &storage.Node{
			ID:          ruleID,
			MemoryGroup: t.MemoryGroup,
			Type:        "Rule",
			Properties: map[string]any{
				"text": rule.Text,
			},
		}
		nodes = append(nodes, ruleNode)

		// ルール -> NodeSet のエッジ
		edge := &storage.Edge{
			SourceID:    ruleID,
			TargetID:    ruleSetNodeID,
			MemoryGroup: t.MemoryGroup,
			Type:        "belongs_to",
			Properties: map[string]any{
				"relationship_name": "belongs_to",
			},
		}
		edges = append(edges, edge)
	}

	// ========================================
	// 7. グラフに保存（その場で保存してメモリ解放）
	// ========================================
	if err := t.GraphStorage.AddNodes(ctx, nodes); err != nil {
		return totalUsage, fmt.Errorf("RuleExtractionTask: failed to add nodes: %w", err)
	}

	if err := t.GraphStorage.AddEdges(ctx, edges); err != nil {
		return totalUsage, fmt.Errorf("RuleExtractionTask: failed to add edges: %w", err)
	}

	// ========================================
	// 8. ベクトルインデックスを作成（その場で保存してメモリ解放）
	// ========================================
	for _, rule := range ruleSet.Rules {
		embedding, u, err := t.Embedder.EmbedQuery(ctx, rule.Text)
		totalUsage.Add(u)
		if err != nil {
			fmt.Printf("RuleExtractionTask: Warning - failed to embed rule: %v\n", err)
			continue
		}

		ruleID := uuid.NewSHA1(uuid.NameSpaceOID, []byte(rule.Text)).String()
		if err := t.VectorStorage.SaveEmbedding(ctx, "Rule_text", ruleID, rule.Text, embedding, t.MemoryGroup); err != nil {
			fmt.Printf("RuleExtractionTask: Warning - failed to save embedding: %v\n", err)
		}
	}

	t.extractedRulesCount += len(ruleSet.Rules)
	fmt.Printf("RuleExtractionTask: Total rules extracted so far: %d\n", t.extractedRulesCount)

	return totalUsage, nil
}

// GetExtractedRulesCount は、抽出されたルールの累計数を返します。
func (t *RuleExtractionTask) GetExtractedRulesCount() int {
	return t.extractedRulesCount
}

// extractJSON は、LLMレスポンスからJSONを抽出します。
// マークダウンのコードブロックやテキストに埋め込まれたJSONを処理します。
func extractJSON(s string) string {
	// マークダウンコードブロックを除去
	s = strings.ReplaceAll(s, "```json", "")
	s = strings.ReplaceAll(s, "```", "")
	s = strings.TrimSpace(s)

	// JSONオブジェクトの開始と終了を検出
	start := strings.Index(s, "{")
	end := strings.LastIndex(s, "}")

	if start == -1 || end == -1 || start > end {
		return `{"rules":[]}`
	}

	return s[start : end+1]
}
