# Phase-11-PRE: Cuber Refactoring for Exact Token Counting

## 1. 概要 (Overview)
`src/pkg/cuber` パッケージを改修し、OpenAI形式のトークン使用量 (`usage`) を**完全に正確に**取得できる基盤を整えます。
Cube の市場価値算定においてトークン量は金銭と同等の価値を持つため、集計漏れや不正確さは許容されません。
LLM (Chat Completion) だけでなく、**Embeddings (Embedding API)** のトークン使用量も漏らさず集計します。

## 2. 改修要件 (Requirements)
*   **Struct Definition**: `TokenUsage` 構造体を定義し、`InputTokens`, `OutputTokens` (int64) を持つ。
*   **Mandatory Return**: `Absorb`, `Memify`, `Search` などの LLM/Embedding を使用する全関数は、戻り値として `TokenUsage` を返さなければならない。
*   **Error on Missing Usage**: LLM/Embedding プロバイダからのレスポンスに `usage` フィールドが含まれていない場合、またはパースに失敗した場合は、**処理全体をエラー**とし、不完全な状態での続行を認めない。
*   **Aggregation**: 内部で複数回APIを呼ぶ場合（Memifyループや RAG の Embedding + Generation 等）、全ての Usage を正確に加算して返す。

## 3. 具体的な改修指示 (Detailed Instructions)

### Step 1: `TokenUsage` 定義

`src/pkg/cuber/cuber.go` (または `types.go`) に定義します。
OpenAI の API 仕様において、通常 `Embeddings` API には `completion_tokens` (Output) は存在しませんが（入力のみ）、`total_tokens` が返ります。
我々の `TokenUsage` 構造体では、Embedding の場合は `InputTokens` (prompt_tokens) に加算します。`OutputTokens` は 0 になります。

```go
package cuber

// TokenUsage は LLM/Embedding のトークン使用量を記録します。
type TokenUsage struct {
	InputTokens  int64 `json:"prompt_tokens"`
	OutputTokens int64 `json:"completion_tokens"`
    // Details はモデルごとの使用量内訳を保持します。Keyはモデル名です。
    Details      map[string]TokenUsage `json:"details,omitempty"`
}

// Add は他の Usage を加算します。
func (t *TokenUsage) Add(other TokenUsage) {
	t.InputTokens += other.InputTokens
	t.OutputTokens += other.OutputTokens
    
    if t.Details == nil {
        t.Details = make(map[string]TokenUsage)
    }
    // Details のマージ (自分自身にはDetailsがない場合もあるため再帰的に扱うか、単にモデル名で足す)
    // ここでは単純にモデル名合算を想定
    for model, usage := range other.Details {
        if existing, ok := t.Details[model]; ok {
            existing.InputTokens += usage.InputTokens
            existing.OutputTokens += usage.OutputTokens
            t.Details[model] = existing
        } else {
            t.Details[model] = usage
        }
    }
    // other自体が単体のUsage（Detailsなし）で、もしモデル名が判明しているならここで足すべきだが、
    // 基本的に末端の callLLM で Details をセットアップして返す設計とする。
}
```

### Step 2: OpenAI Client Wrapper 改修

内部関数（`callChatCompletion`, `callEmbedding` 等）を改修します。

**【Chat Completion の場合】**
```go
func callLLM(ctx context.Context, req ChatReq) (string, TokenUsage, error) {
    resp, err := client.CreateChatCompletion(ctx, req)
    if err != nil {
        return "", TokenUsage{}, err
    }
    
    if resp.Usage.TotalTokens == 0 {
        return "", TokenUsage{}, fmt.Errorf("Critical: LLM response missing token usage data")
    }

    usage := TokenUsage{
        InputTokens:  int64(resp.Usage.PromptTokens),
        OutputTokens: int64(resp.Usage.CompletionTokens),
        Details:      make(map[string]TokenUsage),
    }
    // モデル名ごと (resp.Model または req.Model を使用)
    modelName := resp.Model 
    if modelName == "" { modelName = req.Model } // Fallback
    
    usage.Details[modelName] = TokenUsage{
        InputTokens:  int64(resp.Usage.PromptTokens),
        OutputTokens: int64(resp.Usage.CompletionTokens),
    }

    return resp.Choices[0].Message.Content, usage, nil
}
```

**【Embeddings の場合 (New)】**
```go
func callEmbedding(ctx context.Context, req EmbeddingReq) ([]float32, TokenUsage, error) {
    resp, err := client.CreateEmbeddings(ctx, req)
    if err != nil {
        return nil, TokenUsage{}, err
    }
    
    if resp.Usage.TotalTokens == 0 {
        return nil, TokenUsage{}, fmt.Errorf("Critical: Embedding response missing token usage data")
    }

    usage := TokenUsage{
        InputTokens:  int64(resp.Usage.PromptTokens),
        OutputTokens: 0,
        Details:      make(map[string]TokenUsage),
    }
    
    modelName := req.Model.String() // or resp.Model if available
    usage.Details[modelName] = TokenUsage{
        InputTokens: int64(resp.Usage.PromptTokens),
        OutputTokens: 0,
    }

    return resp.Data[0].Embedding, usage, nil
}
```

### Step 3: Public Method 改修

`Absorb` (Embedding + Summary), `Search` (Embedding + Retrieval + LLM Generation) など、複合的な処理を行うメソッドは、全てのステップの Usage を合算します。

```go
func Absorb(ctx context.Context, path string, content string) (TokenUsage, error) {
    var totalUsage TokenUsage
    
    // 1. Embedding (Vector化)
    // vec, embedUsage, err := callEmbedding(...)
    // if err != nil { return TokenUsage{}, err }
    // totalUsage.Add(embedUsage)  <-- 加算
    
    // 2. LLM Summarization (要約生成)
    // _, llmUsage, err := callLLM(...)
    // if err != nil { return TokenUsage{}, err }
    // totalUsage.Add(llmUsage)    <-- 加算
    
    return totalUsage, nil
}
```

`Search` の場合 (RAG):

```go
func Search(ctx context.Context, path string, query string) (Result, TokenUsage, error) {
    var totalUsage TokenUsage

    // 1. Query Embedding
    // vec, u1, err := callEmbedding(..., query)
    // totalUsage.Add(u1)

    // 2. Retrieval (KuzuDB) - No Token Usage (DB操作のみ)
    
    // 3. Generation (Answer)
    // answer, u2, err := callLLM(..., context + query)
    // totalUsage.Add(u2)

    return result, totalUsage, nil
}
```

## 4. 検証 (Verification)
*   単体テスト: Mock Client を使い、Embedding API のレスポンスに usage が含まれない場合にエラーになることを確認。
*   結合動作: `Absorb` 実行後、返却される `TokenUsage.InputTokens` が Embedding 分 + LLM Input 分の合計になっていることを確認。
