// Package benchmark は、Cogneeシステムのベンチマークツールを提供します。
// Q&Aデータセットを使用して、検索精度を評価します。
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

// QAEntry は、質問と回答のペアを表します。
// ベンチマークデータセットの各エントリです。
type QAEntry struct {
	Question string `json:"question"` // 質問
	Answer   string `json:"answer"`   // 期待される回答
}

// RunBenchmark は、ベンチマークを実行します。
// この関数は以下の処理を行います：
//  1. Q&AデータセットをJSONファイルから読み込み
//  2. 各質問に対してCogneeで検索を実行
//  3. 実際の回答と期待される回答の類似度を計算
//  4. 精度を計算して表示
//
// 引数:
//   - ctx: コンテキスト
//   - qaFile: Q&AデータセットのJSONファイルパス
//   - n: 実行する質問数（0の場合は全て）
//   - service: CogneeServiceインスタンス
//
// 返り値:
//   - error: エラーが発生した場合
func RunBenchmark(ctx context.Context, qaFile string, n int, service *cognee.CogneeService) error {
	// ========================================
	// 1. Q&Aデータを読み込み
	// ========================================
	content, err := os.ReadFile(qaFile)
	if err != nil {
		return fmt.Errorf("failed to read QA file: %w", err)
	}
	var qaList []QAEntry
	if err := json.Unmarshal(content, &qaList); err != nil {
		return fmt.Errorf("failed to parse QA file: %w", err)
	}

	// n件のみ実行する場合
	if n > 0 && n < len(qaList) {
		qaList = qaList[:n]
	}

	fmt.Printf("Running benchmark on %d questions...\n", len(qaList))

	// ========================================
	// 2. 各質問を処理
	// ========================================
	correctCount := 0
	for i, qa := range qaList {
		fmt.Printf("[%d/%d] Q: %s\n", i+1, len(qaList), qa.Question)

		// 検索を実行
		actualAnswer, err := service.Search(ctx, qa.Question, search.SearchTypeGraphCompletion, "benchmark_dataset", "benchmark_user")
		if err != nil {
			fmt.Printf("  Error: %v\n", err)
			continue
		}
		fmt.Printf("  A: %s\n", actualAnswer)
		fmt.Printf("  Expected: %s\n", qa.Answer)

		// ========================================
		// 3. 評価
		// ========================================
		// 回答不可能かチェック
		if isUnanswerable(actualAnswer) {
			if isUnanswerable(qa.Answer) {
				fmt.Println("  Result: Correct (Both unanswerable)")
				correctCount++
			} else {
				fmt.Println("  Result: Incorrect (Expected answer, got unanswerable)")
			}
			continue
		}

		// 類似度を計算
		score, err := calculateSimilarity(ctx, service.Embedder, qa.Answer, actualAnswer)
		if err != nil {
			fmt.Printf("  Failed to calculate similarity: %v\n", err)
			continue
		}
		fmt.Printf("  Similarity: %.4f\n", score)

		// 類似度が0.85以上なら正解
		if score >= 0.85 {
			fmt.Println("  Result: Correct")
			correctCount++
		} else {
			fmt.Println("  Result: Incorrect")
		}
	}

	// ========================================
	// 4. 精度を計算して表示
	// ========================================
	accuracy := float64(correctCount) / float64(len(qaList)) * 100
	fmt.Printf("\nBenchmark Complete.\nAccuracy: %.2f%%\n", accuracy)

	return nil
}

// isUnanswerable は、テキストが「回答不可能」を示すかをチェックします。
// 英語と日本語の両方のフレーズをチェックします。
func isUnanswerable(text string) bool {
	lower := strings.ToLower(text)
	return strings.Contains(lower, "i don't know") ||
		strings.Contains(lower, "information not found") ||
		strings.Contains(lower, "not mentioned") ||
		strings.Contains(text, "わかりません") ||
		strings.Contains(text, "情報が見つかりません") ||
		strings.Contains(text, "記載されていません")
}

// calculateSimilarity は、2つのテキストの類似度を計算します。
// 各テキストのembeddingを生成し、コサイン類似度を計算します。
//
// 引数:
//   - ctx: コンテキスト
//   - embedder: Embedder
//   - text1: 1つ目のテキスト
//   - text2: 2つ目のテキスト
//
// 返り値:
//   - float64: コサイン類似度（0〜1）
//   - error: エラーが発生した場合
func calculateSimilarity(ctx context.Context, embedder storage.Embedder, text1, text2 string) (float64, error) {
	// 各テキストのembeddingを生成
	emb1, err := embedder.EmbedQuery(ctx, text1)
	if err != nil {
		return 0, err
	}
	emb2, err := embedder.EmbedQuery(ctx, text2)
	if err != nil {
		return 0, err
	}

	// コサイン類似度を計算
	return cosineSimilarity(emb1, emb2), nil
}

// cosineSimilarity は、2つのベクトルのコサイン類似度を計算します。
// コサイン類似度 = (a・b) / (||a|| * ||b||)
//
// 引数:
//   - a: 1つ目のベクトル
//   - b: 2つ目のベクトル
//
// 返り値:
//   - float64: コサイン類似度（-1〜1、通常は0〜1）
func cosineSimilarity(a, b []float32) float64 {
	// ベクトルの次元数が異なる場合は0を返す
	if len(a) != len(b) {
		return 0
	}

	var dot, magA, magB float64
	// 内積とベクトルの大きさを計算
	for i := 0; i < len(a); i++ {
		dot += float64(a[i] * b[i])
		magA += float64(a[i] * a[i])
		magB += float64(b[i] * b[i])
	}

	// ゼロベクトルの場合は0を返す
	if magA == 0 || magB == 0 {
		return 0
	}

	// コサイン類似度を計算
	return dot / (math.Sqrt(magA) * math.Sqrt(magB))
}
