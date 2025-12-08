# Phase-11G: Search API Implementation

## 1. 概要 (Overview)
`GET /v1/cubes/search` エンドポイントを実装。
検索行動もトークンコストが発生するため、**厳格な課金・集計** 対象です。
Search は利用 (Usage) であり、貢献 (Contribution) ではないため、`CubeContributor` への加算は行いませんが、`CubeStat` への加算は必須です。

## 2. 実装要件
*   **Limit**: `SearchLimit` (0=Unlim, >0=Decr, <0=Forbid).
*   **Type**: `SearchTypeLimit` に含まれる `type` のみ許可。
*   **Token**: `Input`/`Output` を厳密に集計。失敗時はロールバック（検索結果を返さない）。

## 3. 詳細実装＆解説 (Detailed Implementation & Reasoning)

### Step 1: 権限チェック (BL層)

**【解説】**
Limit チェックに加え、`SearchType` のホワイトリストチェックを行います。
Limit の消費は、検索実行前に確定させるか、実行後に確定させるかですが、トークン集計が失敗したらロールバックすべきなので、トランザクションは最後に回します。ただし、消費枠の予約（メモリチェック）は最初に行います。

**【実装コードスニペット】**
```go
    // Type Check
    if len(perm.SearchTypeLimit) > 0 {
        allowed := false
        for _, t := range perm.SearchTypeLimit {
            if t == query.SearchType { allowed = true; break }
        }
        if !allowed { return fmt.Errorf("search type not allowed") }
    }

    // Limit Check
    if perm.SearchLimit < 0 { return fmt.Errorf("limit exceeded") }
    // ... logic for >0 ...
```

### Step 2: Search 実行 (pkg/cuber)

**【解説】**
`cuber.Search` は `(Result, TokenUsage, error)` を返します。
RAG プロセスでは、Embedding のトークン、Retrieval 結果を Prompt に埋め込んだトークン、LLM の生成トークンなどが発生します。これらを全て合算した Usage を返す必要があります。

**【実装コードスニペット】**
```go
    res, usage, err := cuber.Search(cube.Path, query.Q, query.SearchType)
    if err != nil { return nil, err }
    
    // Strict Check
    if usage.TotalTokens == 0 {
        return nil, fmt.Errorf("token accounting failed")
    }
```

### Step 3: DB更新

**【解説】**
検索結果をユーザーに返す前に、必ずトークン使用量を DB にコミットします。
トランザクション内で Limit 更新と Stats 更新を行います。
Contributor は更新しません。

**【実装コードスニペット】**
```go
    err := db.Transaction(func(tx *gorm.DB) error {
        // Limit Update
        if shouldUpdateLimit { ... }

        // Stats Update (Strict Model Breakdown, Search Only)
        // usage.Details を回して ActionType="search" で記録
        for modelName, detail := range usage.Details {
             var ms model.CubeModelStat
             tx.Where("cube_id = ? AND model_name = ? AND action_type = ? AND apx_id = ? AND vdr_id = ?", cube.ID, modelName, "search", cube.ApxID, cube.VdrID).
                FirstOrCreate(&ms, model.CubeModelStat{
                    CubeID: cube.ID, ModelName: modelName, ActionType: "search",
                    ApxID: cube.ApxID, VdrID: cube.VdrID,
                })
             ms.InputTokens += detail.InputTokens
             ms.OutputTokens += detail.OutputTokens
             tx.Save(&ms)
        }
        
        return nil
    })
    
    if err != nil { return nil, err }
    return res, nil
```
