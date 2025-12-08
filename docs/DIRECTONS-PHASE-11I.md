# Phase-11I: Stats Cube API Implementation

## 1. 概要 (Overview)
`GET /v1/cubes/stats` エンドポイントを実装。
`AllowStats` 権限チェックを行い、詳細な統計と系譜情報を返します。

## 2. 実装要件
*   **Permission**: `AllowStats` == true.
*   **Contents**: Usage (Total), Contributors (List), Lineage (List w/ Timestamp).

## 3. 詳細実装＆解説

### Step 1: 権限チェック

**【解説】**
`Permissions.AllowStats` が `false` の場合は 403 エラーを返します。
これは「統計情報を見せない」というビジネスロジック（例えば、中身のプロンプトエンジニアリング詳細を隠蔽したい場合など）に対応するためです。

### Step 2: データ取得と整形

**【解説】**
各テーブルからデータを取得し、レスポンス構造体にマッピングします。
Lineage の `ExportedAt` (int64 ms) も忘れずに含めます。

**【実装コードスニペット】**
```go
// Response Struct
type CubeStatsRes struct {
    Stats        []ModelStatRes
    Contributors []ContributorRes
    Lineage      []LineageRes
}

type ModelStatRes struct {
    ModelName    string
    ActionType   string
    InputTokens  int64
    OutputTokens int64
}

type ContributorRes struct {
    ContributorName string
    ModelName       string
    InputTokens     int64
    OutputTokens    int64
}

type LineageRes struct {
    UUID       string
    Owner      string
    ExportedAt int64 // timestamp ms
    Generation int
}

// BL
func GetStats(...) (*CubeStatsRes, error) {
    // Perm Check
    if !perm.AllowStats { return nil, 403 }

    // Fetch ModelStats
    var modelStats []model.CubeModelStat
    db.Where("cube_id = ? AND apx_id = ? AND vdr_id = ?", id, apxID, vdrID).Find(&modelStats)

    // Fetch Contributors
    var contribs []model.CubeContributor
    db.Where("cube_id = ? AND apx_id = ? AND vdr_id = ?", id, apxID, vdrID).Find(&contribs)

    // Fetch Lineage
    var lineage []model.CubeLineage
    db.Where("cube_id = ? AND apx_id = ? AND vdr_id = ?", id, apxID, vdrID).Order("generation asc").Find(&lineage)

    // Map to Res
    res := &CubeStatsRes{
        Stats:        make([]ModelStatRes, len(modelStats)),
        Contributors: make([]ContributorRes, len(contribs)),
        Lineage:      make([]LineageRes, len(lineage)),
    }
    
    for i, s := range modelStats {
        res.Stats[i] = ModelStatRes{
            ModelName: s.ModelName, ActionType: s.ActionType,
            InputTokens: s.InputTokens, OutputTokens: s.OutputTokens,
        }
    }
    for i, c := range contribs {
        res.Contributors[i] = ContributorRes{
            ContributorName: c.ContributorName, ModelName: c.ModelName,
            InputTokens: c.InputTokens, OutputTokens: c.OutputTokens,
        }
    }
    for i, l := range lineage {
        res.Lineage[i] = LineageRes{
            UUID: l.AncestorUUID, Owner: l.AncestorOwner,
            ExportedAt: l.ExportedAt, Generation: l.Generation,
        }
    }
    return res, nil
}
```
