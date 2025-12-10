// Package benchmark は、Cuberシステムのベンチマークツールを提供します。
// Q&Aデータセットを使用して、検索精度を評価します。
package benchmark

import (
	"context"
	"encoding/json"
	"fmt"
	"math"
	"math/rand"
	"os"
	"strings"
	"time"

	"github.com/t-kawata/mycute/pkg/cuber"
	"github.com/t-kawata/mycute/pkg/cuber/storage"
	"github.com/t-kawata/mycute/pkg/cuber/tools/query"
)

// QAEntry は、質問と回答のペアを表します。
// ベンチマークデータセットの各エントリです。
type QAEntry struct {
	Question     string `json:"question"`      // 質問
	Answer       string `json:"answer"`        // 期待される回答
	IsCorrect    bool   `json:"is_correct"`    // 回答が事実として正しいかどうか
	IsAnswerable bool   `json:"is_answerable"` // 回答可能かどうか（文脈的に）
}

// BenchmarkResult はベンチマークの結果要約です。
type BenchmarkResult struct {
	TotalQuestions int     `json:"total_questions"`
	CorrectCount   int     `json:"correct_count"`
	Accuracy       float64 `json:"accuracy"`
	AverageScore   float64 `json:"average_score"`
}

// ResultEntry は1件のQ&A結果を記録する構造体です。
type ResultEntry struct {
	Question       string  `json:"question"`
	ExpectedAnswer string  `json:"expected_answer"`
	ActualResult   string  `json:"actual_result"`
	Similarity     float64 `json:"similarity"`
	IsCorrect      bool    `json:"is_correct"`
}

// RunBenchmark は、ベンチマークを実行します。
// logDir が指定された場合、各結果を JSON ファイルに記録します。
//
// 引数:
//   - ctx: コンテキスト
//   - qaFile: QA JSON ファイルパス
//   - n: 実行する質問数 (0=全件)
//   - service: CuberService インスタンス
//   - logDir: ログ出力先ディレクトリ (空文字列の場合はログ出力なし)
//   - phaseName: テストフェーズ名 (ログファイル名に使用)
//   - runNumber: 実行番号 (ログファイル名に使用)
//
// 返り値:
//   - *BenchmarkResult: 結果サマリー
//   - error: エラー
func RunBenchmark(ctx context.Context, qaFile string, n int, service *cuber.CuberService, logDir string, phaseName string, runNumber int) (*BenchmarkResult, error) {
	// ========================================
	// 1. Q&Aデータを読み込み
	// ========================================
	content, err := os.ReadFile(qaFile)
	if err != nil {
		return nil, fmt.Errorf("Benchmark: Failed to read QA file: %w", err)
	}
	var allQA []QAEntry
	if err := json.Unmarshal(content, &allQA); err != nil {
		return nil, fmt.Errorf("Benchmark: Failed to parse QA file: %w", err)
	}

	// 2. フィルタリング (is_correct=true && is_answerable=true)
	var qaList []QAEntry
	for _, qa := range allQA {
		if qa.IsCorrect && qa.IsAnswerable {
			qaList = append(qaList, qa)
		}
	}

	fmt.Printf("Benchmark: Loaded %d entries. Filtered to %d valid (correct & answerable) entries.\n", len(allQA), len(qaList))

	// 3. ランダムシャッフル
	r := rand.New(rand.NewSource(time.Now().UnixNano()))
	r.Shuffle(len(qaList), func(i, j int) {
		qaList[i], qaList[j] = qaList[j], qaList[i]
	})

	// 4. n件サンプリング
	if n > 0 && n < len(qaList) {
		qaList = qaList[:n]
	}

	fmt.Printf("Running benchmark on %d random valid questions...\n", len(qaList))

	// ========================================
	// 5. 各質問を処理
	// ========================================
	correctCount := 0
	totalScore := 0.0
	var results []ResultEntry

	for i, qa := range qaList {
		fmt.Printf("[%d/%d] Q: %s\n", i+1, len(qaList), qa.Question)

		entry := ResultEntry{
			Question:       qa.Question,
			ExpectedAnswer: qa.Answer,
		}

		// 検索を実行
		// Note: Benchmark assumes a specific cube. We might need to configure this.
		// For now using "benchmark_cube.db" as cubeDbFilePath
		actualAnswer, _, err := service.Query(ctx, "benchmark_cube.db", "benchmark_group", query.QUERY_TYPE_GRAPH_COMPLETION, qa.Question)
		if err != nil {
			fmt.Printf("  Error: %v\n", err)
			entry.ActualResult = fmt.Sprintf("ERROR: %v", err)
			entry.Similarity = 0
			entry.IsCorrect = false
			results = append(results, entry)
			continue
		}
		entry.ActualResult = actualAnswer

		// 回答不可能かチェック
		if isUnanswerable(actualAnswer) {
			fmt.Println("  Result: Incorrect (Got unanswerable for answerable question)")
			entry.Similarity = 0
			entry.IsCorrect = false
			results = append(results, entry)
			continue
		}

		// 類似度を計算
		score, err := calculateSimilarity(ctx, service.Embedder, qa.Answer, actualAnswer)
		if err != nil {
			fmt.Printf("  Failed to calculate similarity: %v\n", err)
			entry.Similarity = 0
			entry.IsCorrect = false
			results = append(results, entry)
			continue
		}
		entry.Similarity = score
		totalScore += score
		fmt.Printf("  Similarity: %.4f\n", score)

		// 類似度が0.85以上なら正解
		if score >= 0.85 {
			fmt.Println("  Result: Correct")
			entry.IsCorrect = true
			correctCount++
		} else {
			fmt.Println("  Result: Incorrect")
			entry.IsCorrect = false
		}
		results = append(results, entry)
	}

	// ========================================
	// 6. ログファイル出力
	// ========================================
	if logDir != "" {
		if err := os.MkdirAll(logDir, 0755); err != nil {
			fmt.Printf("Warning: Failed to create log directory: %v\n", err)
		} else {
			logFileName := fmt.Sprintf("%s/%s_run%d_%s.json", logDir, phaseName, runNumber, time.Now().Format("20060102_150405"))
			logData, _ := json.MarshalIndent(results, "", "  ")
			if err := os.WriteFile(logFileName, logData, 0644); err != nil {
				fmt.Printf("Warning: Failed to write log file: %v\n", err)
			} else {
				fmt.Printf("Results logged to: %s\n", logFileName)
			}
		}
	}

	// ========================================
	// 7. 精度を計算して表示
	// ========================================
	// ========================================
	if len(qaList) == 0 {
		return &BenchmarkResult{}, nil
	}

	accuracy := float64(correctCount) / float64(len(qaList)) * 100
	avgScore := totalScore / float64(len(qaList))

	fmt.Printf("\nBenchmark Complete.\n Accuracy: %.2f%%\n Average Similarity: %.4f\n", accuracy, avgScore)

	return &BenchmarkResult{
		TotalQuestions: len(qaList),
		CorrectCount:   correctCount,
		Accuracy:       accuracy,
		AverageScore:   avgScore,
	}, nil
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
	emb1, _, err := embedder.EmbedQuery(ctx, text1)
	if err != nil {
		return 0, err
	}
	emb2, _, err := embedder.EmbedQuery(ctx, text2)
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
