# Cognee Go Implementation: Phase-04 Development Directives

## 0. はじめに (Context & Prerequisites)

本ドキュメントは、**Phase-04** における開発指示書です。
Phase-03までに、基本的な "Add" (Ingestion)、"Cognify" (GraphRAG Construction)、"Search" (GraphRAG Search) の実装が完了し、`main.go` での動作検証も完了しています。

Phase-04の目的は、**「Summarization (要約)」機能の完全実装** です。
これまでスコープ外としてきた要約機能を実装し、Cognifyフェーズでの要約生成と、Searchフェーズでの要約を用いた検索を可能にします。

**※重要方針**:
*   入力は **テキストのみ** に限定します（Document Classificationはスコープ外）。
*   プロンプトエンジニアリングは **オリジナル完全準拠** を徹底します。
*   実装時の迷いをゼロにするため、本ドキュメントのコード例をそのまま使用してください。

---

## 1. 開発の目的 (Objectives)

Phase-04のゴールは以下の通りです。

1.  **Summarization (Cognify)**: 各テキストチャンクに対して要約を生成し、DuckDB (`TextSummary_text`) に保存する。
2.  **Summaries Search (Search)**: 要約テキスト自体を検索対象とする `SUMMARIES` 検索タイプを実装する。
3.  **Graph Summary Search (Search)**: グラフ探索結果を要約して回答する `GRAPH_SUMMARY_COMPLETION` 検索タイプを実装する。

---

## 2. 実装詳細と開発ステップ

### Step 1: プロンプトの同期

`src/pkg/cognee/prompts/prompts.go` に以下の定数を追加します。
**警告**: 内容はオリジナルファイル (`cognee/infrastructure/llm/prompts/...`) から一言一句変更しないでください。

```go
// src/pkg/cognee/prompts/prompts.go

// File: cognee/infrastructure/llm/prompts/summarize_content.txt
const SummarizeContentPrompt = `Summarize the following text while strictly keeping the details that are essential for the understanding of the text.
The answer should be as detailed as possible.

Text:
%s`

// File: cognee/infrastructure/llm/prompts/summarize_search_results.txt
const SummarizeSearchResultsPrompt = `Summarize the search results to answer the query: %s

Search Results:
%s`
```

---

### Step 2: SummarizationTask の実装 (Cognify)

`SummarizationTask` を新規作成し、`ChunkingTask` で生成された各チャンクの要約を生成し、DuckDBに保存します。

**ファイル作成**: `src/pkg/cognee/tasks/summarization/summarization_task.go`

```go
package summarization

import (
	"context"
	"fmt"

	"github.com/google/uuid"
	"github.com/t-kawata/mycute/pkg/cognee/pipeline"
	"github.com/t-kawata/mycute/pkg/cognee/prompts"
	"github.com/t-kawata/mycute/pkg/cognee/storage"
	"github.com/tmc/langchaingo/llms"
)

type SummarizationTask struct {
	VectorStorage storage.VectorStorage
	LLM           llms.Model
	Embedder      storage.Embedder
}

func NewSummarizationTask(vectorStorage storage.VectorStorage, llm llms.Model, embedder storage.Embedder) *SummarizationTask {
	return &SummarizationTask{
		VectorStorage: vectorStorage,
		LLM:           llm,
		Embedder:      embedder,
	}
}

// Ensure interface implementation
var _ pipeline.Task = (*SummarizationTask)(nil)

func (t *SummarizationTask) Run(ctx context.Context, input any) (any, error) {
	output, ok := input.(*storage.CognifyOutput)
	if !ok {
		return nil, fmt.Errorf("expected *storage.CognifyOutput input, got %T", input)
	}

	fmt.Printf("Summarizing %d chunks...\n", len(output.Chunks))

	for _, chunk := range output.Chunks {
		// 1. Generate Summary Prompt
		prompt := fmt.Sprintf(prompts.SummarizeContentPrompt, chunk.Text)

		// 2. Call LLM (Generate Summary)
		resp, err := t.LLM.GenerateContent(ctx, []llms.MessageContent{
			llms.TextParts(llms.ChatMessageTypeHuman, prompt),
		})
		if err != nil {
			// Continue on error? Or Fail? For now, log and continue to robustly process others.
			fmt.Printf("Warning: failed to summarize chunk %s: %v\n", chunk.ID, err)
			continue
		}
		if len(resp.Choices) == 0 {
			continue
		}
		summaryText := resp.Choices[0].Content

		// 3. Generate Embedding for Summary
		embedding, err := t.Embedder.EmbedQuery(ctx, summaryText)
		if err != nil {
			fmt.Printf("Warning: failed to embed summary for chunk %s: %v\n", chunk.ID, err)
			continue
		}

		// 4. Save Summary to DuckDB
		// ID: Consistent ID based on Chunk ID (Simulate UUID5 behavior roughly by hashing or just new random if acceptable. 
		// Python uses uuid5(chunk.id, "TextSummary"). For simplicity here, we use NewRandom but you can use NewSHA1 if needed.
		// Using NewSHA1 with a namespace for idempotency is better.)
		namespace := uuid.MustParse("00000000-0000-0000-0000-000000000000") // Or specific namespace
		summaryID := uuid.NewSHA1(namespace, []byte(chunk.ID+"TextSummary")).String()

		// Collection: "TextSummary_text"
		if err := t.VectorStorage.SaveEmbedding(ctx, "TextSummary_text", summaryID, summaryText, embedding); err != nil {
			return nil, fmt.Errorf("failed to save summary embedding: %w", err)
		}
	}

	return output, nil // Pass through for any subsequent tasks
}
```

**パイプラインへの組み込み**:
`src/pkg/cognee/cognee.go` の `Cognify` メソッドを修正し、`StorageTask` の後に `SummarizationTask` を追加します。

```go
// src/pkg/cognee/cognee.go

func (s *CogneeService) Cognify(ctx context.Context, dataList []*storage.Data) error {
    // ... existing tasks ...
    chunkingTask, _ := chunking.NewChunkingTask(s.VectorStorage)
    graphTask := graph.NewGraphExtractionTask(llm)
    storageTask := storageTaskPkg.NewStorageTask(s.VectorStorage, s.GraphStorage, s.Embedder)
    
    // [NEW]
    summarizationTask := summarization.NewSummarizationTask(s.VectorStorage, llm, s.Embedder)

    p := pipeline.NewPipeline([]pipeline.Task{
        chunkingTask,
        graphTask,
        storageTask,
        summarizationTask, // Added as the last step
    })
    
    // ...
}
```

---

### Step 3: Search機能の拡張 (Search)

`src/pkg/cognee/tools/search/graph_completion.go` を拡張（または `SearchTool` としてリファクタリング）し、異なる検索タイプをサポートします。実用上、既存の `GraphCompletionTool` にメソッドを追加するか、`Search` メソッド内で分岐するのが簡単です。

**修正方針**: `GraphCompletionTool` 内の `Search` メソッドで `searchType` を受け取り分岐します。

**ファイル修正**: `src/pkg/cognee/tools/search/graph_completion.go`

```go
// Update generic Search signature or implementation
func (t *GraphCompletionTool) Search(ctx context.Context, query string, searchType cognee.SearchType) (string, error) {
    switch searchType {
    case cognee.SearchTypeSummaries:
        return t.searchSummaries(ctx, query)
    case cognee.SearchTypeGraphSummaryCompletion:
        return t.searchGraphSummaryCompletion(ctx, query)
    case cognee.SearchTypeGraphCompletion:
        fallthrough
    default:
        // Existing Logic (Move to separate method if needed for cleanliness)
        return t.searchGraphCompletion(ctx, query)
    }
}

// [NEW] Summaries Search Implementation
func (t *GraphCompletionTool) searchSummaries(ctx context.Context, query string) (string, error) {
    queryVector, err := t.Embedder.EmbedQuery(ctx, query)
    if err != nil {
        return "", err
    }
    
    // Search "TextSummary_text" collection
    results, err := t.VectorStorage.Search(ctx, "TextSummary_text", queryVector, 5)
    if err != nil {
        return "", err
    }
    
    if len(results) == 0 {
        return "No relevant summaries found.", nil
    }
    
    var sb strings.Builder
    for _, res := range results {
        sb.WriteString("- " + res.Text + "\n")
    }
    return sb.String(), nil
}

// [NEW] Graph Summary Completion Implementation
func (t *GraphCompletionTool) searchGraphSummaryCompletion(ctx context.Context, query string) (string, error) {
    // 1. Run Standard Graph Completion Logic (Nodes -> Graph Traversal)
    // Refactor existing logic to reuse 'getTriplets' part or simply copy logic if small.
    // For specific requirement: "Summarize the retrieved edges first".
    
    // ... (Perform Vector Search on Nodes) ...
    // ... (Perform Graph Traversal to get triplets) ...
    // Assuming we have 'triplets' []*storage.Triplet
    
    if len(triplets) == 0 {
         return "No relevant graph connections found to summarize.", nil
    }

    // 2. Convert Triplets to Text
    var graphText strings.Builder
    for _, triplet := range triplets {
        sourceName := getName(triplet.Source)
        targetName := getName(triplet.Target)
        graphText.WriteString(fmt.Sprintf("- %s --[%s]--> %s\n", sourceName, triplet.Edge.Type, targetName))
    }
    
    // 3. Summarize the Graph Text
    summarizePrompt := fmt.Sprintf(prompts.SummarizeSearchResultsPrompt, query, graphText.String())
    
    summaryResp, err := t.LLM.GenerateContent(ctx, []llms.MessageContent{
         llms.TextParts(llms.ChatMessageTypeHuman, summarizePrompt),
    })
    if err != nil {
        return "", err
    }
    summaryContext := summaryResp.Choices[0].Content
    
    // Python実装では、ここでさらに "answer_simple_question" プロンプトを使って回答を作っているか？
    // graph_summary_completion_retriever.py を見ると:
    // resolve_edges_to_text() で要約を行い、その結果をコンテキストとして
    // 親クラスの completion() が走る構造。
    // つまり: 最終回答 = LLM(UserPrompt(query, summaryContext), SystemPrompt)

    // 4. Generate Final Answer using the Summary as Context
    // Reuse the exact same prompts as GraphCompletion
    finalUserPrompt := fmt.Sprintf(prompts.GraphContextForQuestionPrompt, query, summaryContext)
    
    finalResp, err := t.LLM.GenerateContent(ctx, []llms.MessageContent{
        llms.TextParts(llms.ChatMessageTypeSystem, prompts.AnswerSimpleQuestionPrompt),
        llms.TextParts(llms.ChatMessageTypeHuman, finalUserPrompt),
    })
     if err != nil {
        return "", err
    }
    return finalResp.Choices[0].Content, nil
}
```

**インターフェース修正**:
`src/pkg/cognee/cognee.go` の `Search` メソッドと `CogneeService` も `SearchType` を受け取るようにシグネチャを変更済みなはず（Phase 3ですでに変更したが、もしハードコーディングされていたら修正）です。
`main.go` からはCLI引数などでタイプ指定（`-t` フラグなど、もしくはデフォルトが `GRAPH_COMPLETION`）できるように対応が必要ですが、`main.go` の修正指示はすでに完了していると思われるため、ここではツールの実装に集中してください。

---

### Step 4: 検証手順

1.  **DBリセット & データ投入**:
    ```bash
    rm -rf src/data/cognee_v2.db src/data/graph.cozodb
    make run ARGS="add"
    ```

2.  **Cognify (Summarization実行)**:
    ```bash
    make run ARGS="cognify"
    ```
    *   ログ確認: `Summarizing X chunks...` と出力され、エラーがないこと。

3.  **Search Validation**:
    *   **SUMMARIES**:
        ```bash
        # コード内あるいはmain.goで指定が必要。動作確認用の一時的なハードコード変更も可。
        # 例: main.go で Default Search Type を SUMMARIES に変えて実行
        make run ARGS="search -q 'Cogneeとは？'" 
        ```
        *   期待値: 要約テキストのリストが返されること。

    *   **GRAPH_SUMMARY_COMPLETION**:
        ```bash
        # 例: main.go で Default Search Type を GRAPH_SUMMARY_COMPLETION に変えて実行
        make run ARGS="search -q 'Cogneeとは？'"
        ```
        *   期待値: グラフ情報に基づくが、より要約された洗練された回答が返されること。

---

## 3. 注意点とエラーハンドリング

*   **DuckDB Lock**: `SaveEmbedding` は並列実行されるとロック競合する可能性があります。Phase-03までの実装で `sync.Mutex` 等で保護されているか、あるいは直列実行になっているか確認してください。今回追加する `SummarizationTask` は直列ループで実装しているため安全です。
*   **Prompt**: 必ず `prompts.go` に定義し、コード内にハードコーディングしないでください。
*   **Import Cycle**: `src/pkg/cognee/tasks/summarization` から `src/pkg/cognee/cognee` を参照しないように注意してください（依存は `pkg/cognee/tasks` -> `pkg/cognee` ではなく `pkg/cognee` -> `pkg/cognee/tasks` の方向）。
