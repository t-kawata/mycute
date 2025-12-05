package benchmark

import (
	"context"
	"encoding/json"
	"fmt"
	"math"
	"os"
	"strings"

	"mycute/pkg/cognee"
	"mycute/pkg/cognee/storage"
	"mycute/pkg/cognee/tools/search"
)

type QAEntry struct {
	Question string `json:"question"`
	Answer   string `json:"answer"`
}

func RunBenchmark(ctx context.Context, qaFile string, n int, service *cognee.CogneeService) error {
	// 1. Load QA Data
	content, err := os.ReadFile(qaFile)
	if err != nil {
		return fmt.Errorf("failed to read QA file: %w", err)
	}
	var qaList []QAEntry
	if err := json.Unmarshal(content, &qaList); err != nil {
		return fmt.Errorf("failed to parse QA file: %w", err)
	}

	if n > 0 && n < len(qaList) {
		qaList = qaList[:n]
	}

	fmt.Printf("Running benchmark on %d questions...\n", len(qaList))

	correctCount := 0
	for i, qa := range qaList {
		fmt.Printf("[%d/%d] Q: %s\n", i+1, len(qaList), qa.Question)

		// 2. Search
		actualAnswer, err := service.Search(ctx, qa.Question, search.SearchTypeGraphCompletion, "benchmark_dataset", "benchmark_user")
		if err != nil {
			fmt.Printf("  Error: %v\n", err)
			continue
		}
		fmt.Printf("  A: %s\n", actualAnswer)
		fmt.Printf("  Expected: %s\n", qa.Answer)

		// 3. Evaluate
		// Check if unanswerable
		if isUnanswerable(actualAnswer) {
			if isUnanswerable(qa.Answer) {
				fmt.Println("  Result: Correct (Both unanswerable)")
				correctCount++
			} else {
				fmt.Println("  Result: Incorrect (Expected answer, got unanswerable)")
			}
			continue
		}

		// Calculate Similarity
		score, err := calculateSimilarity(ctx, service.Embedder, qa.Answer, actualAnswer)
		if err != nil {
			fmt.Printf("  Failed to calculate similarity: %v\n", err)
			continue
		}
		fmt.Printf("  Similarity: %.4f\n", score)

		if score >= 0.85 {
			fmt.Println("  Result: Correct")
			correctCount++
		} else {
			fmt.Println("  Result: Incorrect")
		}
	}

	accuracy := float64(correctCount) / float64(len(qaList)) * 100
	fmt.Printf("\nBenchmark Complete.\nAccuracy: %.2f%%\n", accuracy)

	return nil
}

func isUnanswerable(text string) bool {
	lower := strings.ToLower(text)
	return strings.Contains(lower, "i don't know") ||
		strings.Contains(lower, "information not found") ||
		strings.Contains(lower, "not mentioned") ||
		strings.Contains(text, "わかりません") ||
		strings.Contains(text, "情報が見つかりません") ||
		strings.Contains(text, "記載されていません")
}

func calculateSimilarity(ctx context.Context, embedder storage.Embedder, text1, text2 string) (float64, error) {
	emb1, err := embedder.EmbedQuery(ctx, text1)
	if err != nil {
		return 0, err
	}
	emb2, err := embedder.EmbedQuery(ctx, text2)
	if err != nil {
		return 0, err
	}

	return cosineSimilarity(emb1, emb2), nil
}

func cosineSimilarity(a, b []float32) float64 {
	if len(a) != len(b) {
		return 0
	}
	var dot, magA, magB float64
	for i := 0; i < len(a); i++ {
		dot += float64(a[i] * b[i])
		magA += float64(a[i] * a[i])
		magB += float64(b[i] * b[i])
	}
	if magA == 0 || magB == 0 {
		return 0
	}
	return dot / (math.Sqrt(magA) * math.Sqrt(magB))
}
