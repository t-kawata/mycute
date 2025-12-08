# Phase-11E: Absorb API Implementation

## 1. 概要 (Overview)
`PUT /v1/cubes/absorb` エンドポイントを実装し、外部データを Cube に取り込みます。
最大の要件は **正確なトークン集計** です。
LLM (Prompt/Completion) の使用量を **OpenAI形式** で厳密に計測し、失敗時はロールバックします。

## 2. 実装要件 (Requirements)
*   **Limit Logic**: `AbsorbLimit`
    *   `0`: 無制限。
    *   `> 0`: 残回数。`1 -> -1` ロジックで減算。
    *   `< 0`: 禁止。
*   **Strict Token Counting**:
    *   `pkg/cuber` から `InputTokens`, `OutputTokens` を取得。
    *   失敗・欠落時はエラー（不正確なデータ許容ゼロ）。

## 3. 詳細実装＆解説 (Detailed Implementation & Reasoning)

### Step 1: 権限チェックと消費準備 (BL層)

**【解説】**
`AbsorbLimit` をチェックします。
消費が必要な場合 (`>0`)、メモリ上で減算値を計算しておきますが、**DB保存は Absorb 成功・トークン集計成功の後** に、トランザクション内で行うのがベストです（失敗したら消費させない）。

**【実装コードスニペット】**
```go
// BL層
func Absorb(ctx context.Context, cubeID uint, content string) error {
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
`pkg/cuber` の `Absorb` 関数は、以下のようなシグネチャデあるべきです。
`func Absorb(...) (TokenUsage, error)`
`TokenUsage` 構造体には `InputTokens`, `OutputTokens` (int64) が含まれます。
内部で OpenAI API (または互換API) のレスポンス `usage` フィールドをパースし、合算して返します。APIエラーや、usage が取れなかった場合はエラーを返します。

**【実装コードスニペット】**
```go
    // pkg/cuber 呼び出し
    usage, err := cuber.Absorb(cube.Path, content)
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

        // 2. Stats & Contributor 更新 (Strict Model Breakdown)
        // usage.Details は map[string]TokenUsage (key=model_name) または list を想定
        for modelName, detail := range usage.Details {
             // CubeModelStat (Training)
             var ms model.CubeModelStat
             tx.Where("cube_id = ? AND model_name = ? AND action_type = ? AND apx_id = ? AND vdr_id = ?", cube.ID, modelName, "training", cube.ApxID, cube.VdrID).
                FirstOrCreate(&ms, model.CubeModelStat{
                    CubeID: cube.ID, ModelName: modelName, ActionType: "training",
                    ApxID: cube.ApxID, VdrID: cube.VdrID,
                })
             ms.InputTokens += detail.InputTokens
             ms.OutputTokens += detail.OutputTokens
             tx.Save(&ms)

             // CubeContributor (Training)
             var cc model.CubeContributor
             tx.Where("cube_id = ? AND contributor_name = ? AND model_name = ? AND apx_id = ? AND vdr_id = ?", cube.ID, usrName, modelName, cube.ApxID, cube.VdrID).
                FirstOrCreate(&cc, model.CubeContributor{
                    CubeID: cube.ID, ContributorName: usrName, ModelName: modelName,
                    ApxID: cube.ApxID, VdrID: cube.VdrID,
                })
             cc.InputTokens += detail.InputTokens
             cc.OutputTokens += detail.OutputTokens
             tx.Save(&cc)
        }

        return nil
    })
```
