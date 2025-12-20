# Phase-11G: Query API Implementation

## 1. 概要 (Overview)
`GET /v1/cubes/query` エンドポイントを実装。
クエリ（質問）行動もトークンコストが発生するため、**厳格な課金・集計** 対象です。
Query は利用 (Usage) であり、貢献 (Contribution) ではないため、`CubeContributor` への加算は行いませんが、`CubeStat` への加算は必須です。

> [!IMPORTANT]
> **MemoryGroup 設計変更**
> 
> 本エンドポイントでは、`memory_group` パラメータが**必須**です。
> これにより、同一 Cube 内の特定の知識領域（例: 法律専門家）のみを対象にクエリできます。
> 詳細は `docs/DIRECTONS-PHASE-11.md` セクション 2.4 を参照してください。

## 2. 実装要件
*   **Limit**: `QueryLimit` (0=Unlim, >0=Decr, <0=Forbid).
*   **Type**: `QueryTypeLimit` に含まれる `type` のみ許可。
*   **MemoryGroup**: 必須パラメータ。KuzuDB 内の `memory_group` としてそのまま使用。
*   **Token**: `Input`/`Output` を厳密に集計。失敗時はロールバック（クエリ結果を返さない）。

> [!CAUTION]
> **統計情報の正確性は最優先事項**
> 
> Phase-11I (Stats API) の完了により、統計情報がいつでも参照可能な状態になりました。
> このエンドポイントの実装およびテストにおいて、**統計情報 (`CubeModelStat`) が正しく記録されているか**を必ず確認してください。
> 
> - `MemoryGroup` が正しく記録されているか
> - `ActionType` が `"query"` として記録されているか（`"training"` ではない）
> - `InputTokens` / `OutputTokens` が正確に加算されているか
> - **注意**: Query は「利用」であり「貢献」ではないため、`CubeContributor` には加算しない
> 
> **統計情報に誤りがある場合は、機能実装よりも統計情報の修正を最優先してください。**
> これは mycute サービスの商品価値に直結する重要事項です。

## 3. 詳細実装＆解説 (Detailed Implementation & Reasoning)

### Step 0: リクエストパラメータ定義

**【解説】**
`memory_group` フィールドが必須として追加されます。

**【実装コードスニペット】**
```go
// rtparam/cubes_param.go
type QueryCubeParam struct {
    CubeID      uint   `form:"cube_id" binding:"required" example:"123"`    // ← CubeIDで指定
    MemoryGroup string `form:"memory_group" binding:"required" example:"legal_expert"` // ← 必須
    Text        string `form:"text" binding:"required" example:"契約違反の場合の対処法は？"`
    QueryType   string `form:"query_type" example:"graph_completion"`
}
```

### Step 1: 権限チェック (BL層)

**【解説】**
Limit チェックに加え、`QueryType` のホワイトリストチェックを行います。
Limit の消費は、クエリ実行前に確定させるか、実行後に確定させるかですが、トークン集計が失敗したらロールバックすべきなので、トランザクションは最後に回します。ただし、消費枠の予約（メモリチェック）は最初に行います。

**【実装コードスニペット】**
```go
    // Type Check
    if len(perm.QueryTypeLimit) > 0 {
        allowed := false
        for _, t := range perm.QueryTypeLimit {
            if t == req.QueryType { allowed = true; break }
        }
        if !allowed { return fmt.Errorf("query type not allowed") }
    }

    // Limit Check
    if perm.QueryLimit < 0 { return fmt.Errorf("limit exceeded") }
    // ... logic for >0 ...
```

### Step 2: Query 実行 (pkg/cuber)

**【解説】**
`cuber.Query` は `(Result, TokenUsage, error)` を返します。
`memoryGroup` はそのまま `memory_group` として使用されます。
RAG プロセスでは、Embedding のトークン、Retrieval 結果を Prompt に埋め込んだトークン、LLM の生成トークンなどが発生します。これらを全て合算した Usage を返す必要があります。

**【実装コードスニペット】**
```go
    // cuber.Query の新シグネチャ:
    // func (s *CuberService) Query(ctx context.Context, cubeDbFilePath string, memoryGroup string, queryType query.QueryType, text string) (string, types.TokenUsage, error)
    
    // memoryGroup をそのまま渡す (例: "legal_expert")
    res, usage, err := u.CuberService.Query(ctx, cubeDbFilePath, memoryGroup, query.QueryType(req.QueryType), req.Text)
    if err != nil { return nil, err }
    
    // Strict Check
    if usage.TotalTokens == 0 {
        return nil, fmt.Errorf("token accounting failed")
    }
```

### Step 3: DB更新

**【解説】**
クエリ結果をユーザーに返す前に、必ずトークン使用量を DB にコミットします。
トランザクション内で Limit 更新と Stats 更新を行います。
Contributor は更新しません（Query は「利用」であり「貢献」ではないため）。
**Stats には必ず `MemoryGroup` を含める**ことで、「どの分野のクエリでどれだけトークンが使われたか」を集計できます。

**【実装コードスニペット】**
```go
    err := db.Transaction(func(tx *gorm.DB) error {
        // Limit Update
        if shouldUpdateLimit { ... }

        // Stats Update (MemoryGroup を含む階層構造, Query Only)
        // usage.Details を回して ActionType="query" で記録
        for modelName, detail := range usage.Details {
             // CubeModelStat (Query) - MemoryGroup を必ず含める
             var ms model.CubeModelStat
             tx.Where("cube_id = ? AND memory_group = ? AND model_name = ? AND action_type = ? AND apx_id = ? AND vdr_id = ?",
                 cube.ID, req.MemoryGroup, modelName, "query", cube.ApxID, cube.VdrID).
                FirstOrCreate(&ms, model.CubeModelStat{
                    CubeID: cube.ID, MemoryGroup: req.MemoryGroup, ModelName: modelName, ActionType: "query",
                    ApxID: cube.ApxID, VdrID: cube.VdrID,
                })
             ms.InputTokens += detail.InputTokens
             ms.OutputTokens += detail.OutputTokens
             tx.Save(&ms)
             // CubeContributor は更新しない（Query は利用であり貢献ではない）
        }
        
        return nil
    })
    
    if err != nil { return nil, err }
    return res, nil
```
