package metacognition

import (
	"context"
	"encoding/json"
	"fmt"
	"strings"

	"github.com/cloudwego/eino/components/model"

	"github.com/t-kawata/mycute/pkg/cuber/prompts"
	"github.com/t-kawata/mycute/pkg/cuber/storage"
	"github.com/t-kawata/mycute/pkg/cuber/types"
	"github.com/t-kawata/mycute/pkg/cuber/utils"
)

// Question は、自問自答で生成された問いを表します。
type Question struct {
	Text string `json:"text"`
}

// QuestionSet は、LLMから返される問いのリストです。
type QuestionSet struct {
	Questions []Question `json:"questions"`
}

// SelfReflectionTask は、自問自答ループを実行するタスクです。
type SelfReflectionTask struct {
	VectorStorage       storage.VectorStorage
	GraphStorage        storage.GraphStorage
	LLM                 model.ToolCallingChatModel // Eino ChatModel
	Embedder            storage.Embedder
	MemoryGroup         string
	IgnoranceManager    *IgnoranceManager
	SimilarityThreshold float64 // 関連情報の類似度閾値
	SearchLimitChunk    int     // チャンク検索数
	SearchLimitRule     int     // ルール検索数
	ModelName           string
}

// NewSelfReflectionTask は、新しいSelfReflectionTaskを作成します。
func NewSelfReflectionTask(
	vectorStorage storage.VectorStorage,
	graphStorage storage.GraphStorage,
	llm model.ToolCallingChatModel,
	embedder storage.Embedder,
	memoryGroup string,
	similarityThreshold float64,
	searchLimitChunk int,
	searchLimitRule int,
	ignoranceSimilarityThreshold float64,
	ignoranceSearchLimit int,
	modelName string,
) *SelfReflectionTask {
	if modelName == "" {
		modelName = "gpt-4"
	}
	return &SelfReflectionTask{
		VectorStorage:       vectorStorage,
		GraphStorage:        graphStorage,
		LLM:                 llm,
		Embedder:            embedder,
		MemoryGroup:         memoryGroup,
		SimilarityThreshold: similarityThreshold,
		SearchLimitChunk:    searchLimitChunk,
		SearchLimitRule:     searchLimitRule,
		IgnoranceManager: NewIgnoranceManager(
			vectorStorage, graphStorage, llm, embedder, memoryGroup,
			ignoranceSimilarityThreshold, ignoranceSearchLimit, modelName,
		),
		ModelName: modelName,
	}
}

// Run は、自問自答ループを1回実行します。
// 1. 既存のルールから問いを生成
// 2. 各問いに対して検索を試行
// 3. 回答できた場合は Capability を登録
// 4. 回答できない場合は Unknown を登録
func (t *SelfReflectionTask) Run(ctx context.Context, rules []string) (types.TokenUsage, error) {
	var totalUsage types.TokenUsage
	if len(rules) == 0 {
		return totalUsage, nil
	}

	// ========================================
	// 1. ルールから問いを生成
	// ========================================
	questions, usage1, err := t.generateQuestions(ctx, rules)
	totalUsage.Add(usage1)
	if err != nil {
		return totalUsage, fmt.Errorf("SelfReflectionTask: Failed to generate questions: %w", err)
	}

	fmt.Printf("SelfReflectionTask: Generated %d questions\n", len(questions))

	// ========================================
	// 2. 各問いに対して検索を試行
	// ========================================
	for _, q := range questions {
		answered, insight, usage2, err := t.TryAnswer(ctx, q.Text)
		totalUsage.Add(usage2)
		if err != nil {
			fmt.Printf("SelfReflectionTask: Warning - TryAnswer failed: %v\n", err)
			continue
		}

		if answered {
			// 回答できた: Capability として登録
			u, err := t.IgnoranceManager.RegisterCapability(
				ctx,
				insight,
				[]string{"self_reflection"},
				[]string{""}, // 自己発見なのでユーザーIDなし
				[]string{"self_reflection"},
				[]string{""},
			)
			totalUsage.Add(u)
			if err != nil {
				fmt.Printf("SelfReflectionTask: Warning - RegisterCapability failed: %v\n", err)
			}
		} else {
			// 回答できなかった: Unknown として登録
			u, err := t.IgnoranceManager.RegisterUnknown(ctx, q.Text, "self_reflection", "self_reflection")
			totalUsage.Add(u)
			if err != nil {
				fmt.Printf("SelfReflectionTask: Warning - RegisterUnknown failed: %v\n", err)
			}
		}
	}

	return totalUsage, nil
}

// generateQuestions は、ルールから問いを生成します。
func (t *SelfReflectionTask) generateQuestions(ctx context.Context, rules []string) ([]Question, types.TokenUsage, error) {
	var usage types.TokenUsage
	combinedRules := strings.Join(rules, "\n")

	content, u, err := utils.GenerateWithUsage(ctx, t.LLM, t.ModelName, prompts.QuestionGenerationSystemPrompt, combinedRules)
	usage.Add(u)
	if err != nil {
		return nil, usage, err
	}

	if content == "" {
		return nil, usage, fmt.Errorf("SelfReflectionTask: No response from LLM")
	}

	var qs QuestionSet
	if err := json.Unmarshal([]byte(extractJSON(content)), &qs); err != nil {
		return nil, usage, err
	}

	return qs.Questions, usage, nil
}

// TryAnswer は、問いに対して検索を試行し、回答できるかを判定します。
func (t *SelfReflectionTask) TryAnswer(ctx context.Context, question string) (bool, string, types.TokenUsage, error) {
	var usage types.TokenUsage
	// 検索を実行
	embedding, u, err := t.Embedder.EmbedQuery(ctx, question)
	usage.Add(u)
	if err != nil {
		return false, "", usage, err
	}

	// チャンク検索
	chunkResults, err := t.VectorStorage.Query(ctx, types.TABLE_NAME_CHUNK, embedding, t.SearchLimitChunk, t.MemoryGroup)
	if err != nil {
		return false, "", usage, err
	}

	// ルール検索
	ruleResults, err := t.VectorStorage.Query(ctx, types.TABLE_NAME_RULE, embedding, t.SearchLimitRule, t.MemoryGroup)
	if err != nil {
		// Rule_text がない場合は無視
		ruleResults = nil
	}

	// 十分な情報があるか判定（距離が近い結果が存在するか）
	hasRelevantInfo := false
	var ctxStr strings.Builder

	for _, r := range chunkResults {
		if r.Distance < t.SimilarityThreshold {
			hasRelevantInfo = true
			ctxStr.WriteString(r.Text)
			ctxStr.WriteString("\n")
		}
	}
	for _, r := range ruleResults {
		if r.Distance < t.SimilarityThreshold {
			hasRelevantInfo = true
			ctxStr.WriteString(r.Text)
			ctxStr.WriteString("\n")
		}
	}

	if !hasRelevantInfo {
		return false, "", usage, nil
	}

	// LLMで回答を生成
	prompt := fmt.Sprintf("Question: %s\n\nContext:\n%s", question, ctxStr.String())
	answer, u, err := utils.GenerateWithUsage(ctx, t.LLM, t.ModelName, prompts.AnswerSimpleQuestionPrompt, prompt)
	usage.Add(u)
	if err != nil {
		return false, "", usage, err
	}

	if answer == "" {
		return false, "", usage, fmt.Errorf("SelfReflectionTask: No response from LLM")
	}

	// 「わからない」等の回答でないかチェック
	if containsUncertainty(answer) {
		return false, "", usage, nil
	}

	// 回答できた: 洞察を生成
	insight := fmt.Sprintf("「%s」という問いに対して、以下のように回答できる: %s",
		truncate(question, 30), truncate(answer, 100))

	return true, insight, usage, nil
}

func containsUncertainty(s string) bool {
	uncertainPhrases := []string{
		"わかりません", "不明です", "情報がありません",
		"知りません", "分かりません", "答えられません",
	}
	for _, phrase := range uncertainPhrases {
		if strings.Contains(s, phrase) {
			return true
		}
	}
	return false
}

func truncate(s string, maxLen int) string {
	runes := []rune(s)
	if len(runes) <= maxLen {
		return s
	}
	return string(runes[:maxLen]) + "..."
}
