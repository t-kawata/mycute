# Phase-11F: Memify API Implementation

## 1. 概要 (Overview)
`PUT /v1/cubes/memify` エンドポイントを実装します。
`MemifyLimit` のほか、`MemifyConfigLimit` による設定値制限を行い、厳密なトークン集計を実施します。

> [!IMPORTANT]
> **MemoryGroup 設計変更**
> 
> 本エンドポイントでは、`memory_group` パラメータが**必須**です。
> これにより、指定した知識領域のみを対象に Memify（自己強化）を行うことができます。
> 詳細は `docs/DIRECTONS-PHASE-11.md` セクション 2.4 を参照してください。

## 2. 実装要件
*   **Permissions**:
    *   `MemifyLimit`: `0`=Unlim, `>0`=Decr(`1->-1`), `<0`=Forbid.
    *   `MemifyConfigLimit`: params (`epochs` 等) の上限チェック。
*   **MemoryGroup**: 必須パラメータ。KuzuDB 内の `group_id` としてそのまま使用。
*   **Strict Token Counting**:
    *   自己強化ループ全体のトークン使用量を OpenAI 形式で正確に取得。
    *   失敗時はエラー。

## 3. 詳細実装＆解説 (Detailed Implementation & Reasoning)

### Step 0: リクエストパラメータ定義

**【解説】**
`memory_group` フィールドが必須として追加されます。

**【実装コードスニペット】**
```go
// rtparam/cubes_param.go
type MemifyBody struct {
    CubeID      uint   `json:"cube_id" binding:"required" example:"123"`    // ← CubeIDで指定
    MemoryGroup string `json:"memory_group" binding:"required" example:"legal_expert"` // ← 必須
    Epochs      int    `json:"epochs" example:"3"` // オプショナル、デフォルト適用
    // ... 他の config params
}
```

### Step 1: Config 検証 (BL層)

**【解説】**
Permissions に含まれる `MemifyConfigLimit` マップをチェックします。
例えば `max_epochs` キーがあれば、リクエストされた `epochs` がそれ以下であることを確認します。
これにより、計算資源の浪費を防ぎます。

**【実装コードスニペット】**
```go
    // Config Check
    configLimit := perm.MemifyConfigLimit
    if val, ok := configLimit["max_epochs"]; ok {
        maxEp := int(val.(float64)) // JSON decode は float64 になりがち
        if body.Epochs > maxEp {
            return fmt.Errorf("epochs exceeds limit %d", maxEp)
        }
    }
```

### Step 2: 実行リクエストと Token 集計

**【解説】**
Memify は長時間かかる可能性があるため、通常非同期が好ましいですが、ここでは同期的に結果（Usage）を待ち、Stats を更新する設計とします。
`cuber.Memify` は内部で複数回の推論を行うため、その合計 Usage を返さなければなりません。

**【実装コードスニペット】**
```go
    // cuber.Memify の新シグネチャ:
    // func (s *CuberService) Memify(ctx context.Context, cubeDbFilePath string, memoryGroup string, config *MemifyConfig) (types.TokenUsage, error)
    
    // memoryGroup をそのまま渡す (例: "legal_expert")
    usage, err := u.CuberService.Memify(ctx, cubeDbFilePath, memoryGroup, &cuber.MemifyConfig{
        RecursiveDepth: body.Epochs,
        // ...
    })
    if err != nil { return err }
    
    // Usage ゼロチェック（Memifyでゼロはあり得ない）
    if usage.InputTokens == 0 && usage.OutputTokens == 0 {
        return fmt.Errorf("memify usage zero: strict accounting required")
    }
```

### Step 3: DB更新

**【解説】**
Absorb と同様に、トランザクション内で Limit 消費と Stats 加算を行います。
**Stats には必ず `MemoryGroup` を含める**ことで、後から「どの分野に誰がどれだけ貢献したか」を集計できます。

**【実装コードスニペット】**
```go
    return db.Transaction(func(tx *gorm.DB) error {
        // Limit Update (1 -> -1)
        if perm.MemifyLimit > 0 {
             // ... logic ...
             tx.Save(&cube)
        }
        
        // Stats & Contributor Update (MemoryGroup を含む階層構造)
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
             // Self-improvement なので Contributor は実行者(User) とする
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
