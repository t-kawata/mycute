# Phase-11E: Absorb API Implementation

## 1. 概要 (Overview)
`PUT /v1/cubes/absorb` エンドポイントを実装し、外部データを Cube に取り込みます。
最大の要件は **正確なトークン集計** です。
LLM (Prompt/Completion) の使用量を **OpenAI形式** で厳密に計測し、失敗時はロールバックします。

> [!IMPORTANT]
> **MemoryGroup 設計変更**
> 
> 本エンドポイントでは、`memory_group` パラメータが**必須**です。
> これにより、同一 Cube 内で複数の知識領域（例: 法律、医療、一般）を分離して Absorb できます。
> 詳細は `docs/DIRECTONS-PHASE-11.md` セクション 2.4 を参照してください。

## 2. 実装要件 (Requirements)
*   **Limit Logic**: `AbsorbLimit`
    *   `0`: 無制限。
    *   `> 0`: 残回数。`1 -> -1` ロジックで減算。
    *   `< 0`: 禁止。
*   **MemoryGroup**: 必須パラメータ。KuzuDB 内の `group_id` としてそのまま使用。
*   **Strict Token Counting**:
    *   `pkg/cuber` から `InputTokens`, `OutputTokens` を取得。
    *   失敗・欠落時はエラー（不正確なデータ許容ゼロ）。

## 3. 詳細実装＆解説 (Detailed Implementation & Reasoning)

### Step 0: リクエストパラメータ定義

**【解説】**
`memory_group` フィールドが必須として追加されます。
このフィールドは、ユーザーが Cube 内のどの「知識領域」にデータを投入するかを指定します。

**【実装コードスニペット】**
```go
// rtparam/cubes_param.go
type AbsorbBody struct {
    CubeID      uint   `json:"cube_id" binding:"required" example:"123"`    // ← CubeIDで指定
    MemoryGroup string `json:"memory_group" binding:"required" example:"legal_expert"` // ← 必須
    Content     string `json:"content" binding:"required"`
}
```

### Step 1: 権限チェックと消費準備 (BL層)

**【解説】**
`AbsorbLimit` をチェックします。
消費が必要な場合 (`>0`)、メモリ上で減算値を計算しておきますが、**DB保存は Absorb 成功・トークン集計成功の後** に、トランザクション内で行うのがベストです（失敗したら消費させない）。

**【実装コードスニペット】**
```go
// BL層
func Absorb(ctx context.Context, cubeID uint, memoryGroup string, content string) error {
    // Cube 取得 & Perm Parse
    // ...
    perm := ParsePerm(cube.Permissions)

    if perm.AbsorbLimit < 0 { return fmt.Errorf("limit exceeded") }
    
    // 消費後値の計算
    nextLimit := perm.AbsorbLimit
    shouldUpdateLimit := false
    if perm.AbsorbLimit > 0 {
        nextLimit = perm.AbsorbLimit - 1
        if nextLimit == 0 { nextLimit = -1 }
        shouldUpdateLimit = true
    }
```

### Step 2: Absorb 実行 (pkg/cuber)

**【解説】**
`pkg/cuber` の `Absorb` 関数は、以下のようなシグネチャです。
`memoryGroup` はそのまま `group_id` として使用されます。

**【実装コードスニペット】**
```go
    // cuber.Absorb の新シグネチャ:
    // func (s *CuberService) Absorb(ctx context.Context, cubeDbFilePath string, memoryGroup string, filePaths []string) (types.TokenUsage, error)
    
    // pkg/cuber 呼び出し
    // memoryGroup をそのまま渡す (例: "legal_expert")
    usage, err := u.CuberService.Absorb(ctx, cubeDbFilePath, memoryGroup, []string{tempFile})
    if err != nil {
        return fmt.Errorf("absorb failed: %w", err)
    }
    // usage チェック (念の為)
    if usage.InputTokens < 0 || usage.OutputTokens < 0 {
        return fmt.Errorf("invalid token usage reported")
    }
```

### Step 3: DB更新 (Transaction)

**【解説】**
全て成功したら、トランザクションを開始して「Limit消費」と「Stats加算」を同時に行います。
**Stats には必ず `MemoryGroup` を含める**ことで、後から「どの分野に誰がどれだけ貢献したか」を集計できます。

**【実装コードスニペット】**
```go
    return db.Transaction(func(tx *gorm.DB) error {
        // 1. Limit 更新
        if shouldUpdateLimit {
            perm.AbsorbLimit = nextLimit
            newJSON, _ := json.Marshal(perm)
            cube.Permissions = datatypes.JSON(newJSON)
            tx.Save(&cube)
        }

        // 2. Stats & Contributor 更新 (MemoryGroup を含む階層構造)
        // usage.Details は map[string]TokenUsage (key=model_name) を想定
        for modelName, detail := range usage.Details {
             // CubeModelStat (Training) - MemoryGroup を必ず含める
             var ms model.CubeModelStat
             tx.Where("cube_id = ? AND memory_group = ? AND model_name = ? AND action_type = ? AND apx_id = ? AND vdr_id = ?",
                 cube.ID, req.MemoryGroup, modelName, "training", cube.ApxID, cube.VdrID).
                FirstOrCreate(&ms, model.CubeModelStat{
                    CubeID: cube.ID, MemoryGroup: req.MemoryGroup, ModelName: modelName, ActionType: "training",
                    ApxID: cube.ApxID, VdrID: cube.VdrID,
                })
             ms.InputTokens += detail.InputTokens
             ms.OutputTokens += detail.OutputTokens
             tx.Save(&ms)

             // CubeContributor (Training) - MemoryGroup を必ず含める
             var cc model.CubeContributor
             tx.Where("cube_id = ? AND memory_group = ? AND contributor_name = ? AND model_name = ? AND apx_id = ? AND vdr_id = ?",
                 cube.ID, req.MemoryGroup, usrName, modelName, cube.ApxID, cube.VdrID).
                FirstOrCreate(&cc, model.CubeContributor{
                    CubeID: cube.ID, MemoryGroup: req.MemoryGroup, ContributorName: usrName, ModelName: modelName,
                    ApxID: cube.ApxID, VdrID: cube.VdrID,
                })
             cc.InputTokens += detail.InputTokens
             cc.OutputTokens += detail.OutputTokens
             tx.Save(&cc)
        }

        return nil
    })
```
