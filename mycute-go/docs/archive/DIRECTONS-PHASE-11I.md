# Phase-11I: Stats Cube API Implementation

## 1. 概要 (Overview)
`GET /v1/cubes/stats` エンドポイントを実装。
`AllowStats` 権限チェックを行い、詳細な統計と系譜情報を返します。

> [!IMPORTANT]
> **統計情報の目的**
> 
> Stats API は、ユーザーが「この Cube はどの分野にどれだけ詳しいか」を判断できる情報を提供します。
> 例: 「legal_expert に 10人が500万トークン投入 → 法律に詳しい」「medical_expert に 1人が5万トークン → 医療は弱い」

## 2. 実装要件
*   **Permission**: `AllowStats` == true.
*   **Contents**: MemoryGroup別のUsage, Contributors, Lineage を返す。

> [!NOTE]
> **MemoryGroup 関連**
> 
> `stats` エンドポイントでは `memory_group` パラメータはオプションです。
> - 指定なし: Cube 全体の統計をMemoryGroup別にグループ化して返す
> - 指定あり: 特定の MemoryGroup に絞った統計を返す

## 3. 詳細実装＆解説

### Step 1: 権限チェック

**【解説】**
`Permissions.AllowStats` が `false` の場合は 403 エラーを返します。
これは「統計情報を見せない」というビジネスロジック（例えば、中身のプロンプトエンジニアリング詳細を隠蔽したい場合など）に対応するためです。

### Step 2: データ取得と整形

**【解説】**
各テーブルからデータを取得し、**MemoryGroup を最上位の粒度として**レスポンス構造体にマッピングします。
Lineage の `ExportedAt` (int64 ms) も忘れずに含めます。

**【実装コードスニペット】**
```go
// Response Struct (MemoryGroup を最上位の粒度とした階層構造)
type CubeStatsRes struct {
    MemoryGroups []MemoryGroupStatsRes `json:"memory_groups"`
    Lineage      []LineageRes          `json:"lineage"`
}

// MemoryGroup別の統計 (新設)
type MemoryGroupStatsRes struct {
    MemoryGroup  string            `json:"memory_group"`  // e.g. "legal_expert"
    Stats        []ModelStatRes    `json:"stats"`         // モデル別使用量
    Contributors []ContributorRes  `json:"contributors"`  // 貢献者リスト
}

type ModelStatRes struct {
    ModelName    string `json:"model_name"`
    ActionType   string `json:"action_type"` // "training" or "search"
    InputTokens  int64  `json:"input_tokens"`
    OutputTokens int64  `json:"output_tokens"`
}

type ContributorRes struct {
    ContributorName string `json:"contributor_name"`
    ModelName       string `json:"model_name"`
    InputTokens     int64  `json:"input_tokens"`
    OutputTokens    int64  `json:"output_tokens"`
}

type LineageRes struct {
    UUID       string `json:"uuid"`
    Owner      string `json:"owner"`
    ExportedAt int64  `json:"exported_at"` // timestamp ms
    Generation int    `json:"generation"`
}
```

### Step 3: ビジネスロジック

**【解説】**
MemoryGroup でグループ化して返すことで、ユーザーは「どの専門分野が充実しているか」を一目で把握できます。

**【実装コードスニペット】**
```go
// BL
func GetStats(ctx context.Context, cubeID uint, apxID, vdrID uint) (*CubeStatsRes, error) {
    // Perm Check
    cube, err := getCube(cubeID, apxID, vdrID)
    perm := parsePerm(cube.Permissions)
    if !perm.AllowStats { return nil, errors.New("forbidden") }

    // Fetch ModelStats (MemoryGroup を含む)
    var modelStats []model.CubeModelStat
    db.Where("cube_id = ? AND apx_id = ? AND vdr_id = ?", cubeID, apxID, vdrID).Find(&modelStats)

    // Fetch Contributors (MemoryGroup を含む)
    var contribs []model.CubeContributor
    db.Where("cube_id = ? AND apx_id = ? AND vdr_id = ?", cubeID, apxID, vdrID).Find(&contribs)

    // Fetch Lineage
    var lineage []model.CubeLineage
    db.Where("cube_id = ? AND apx_id = ? AND vdr_id = ?", cubeID, apxID, vdrID).Order("generation asc").Find(&lineage)

    // MemoryGroup でグループ化
    mgMap := make(map[string]*MemoryGroupStatsRes)
    
    for _, s := range modelStats {
        if _, ok := mgMap[s.MemoryGroup]; !ok {
            mgMap[s.MemoryGroup] = &MemoryGroupStatsRes{MemoryGroup: s.MemoryGroup}
        }
        mgMap[s.MemoryGroup].Stats = append(mgMap[s.MemoryGroup].Stats, ModelStatRes{
            ModelName: s.ModelName, ActionType: s.ActionType,
            InputTokens: s.InputTokens, OutputTokens: s.OutputTokens,
        })
    }
    
    for _, c := range contribs {
        if _, ok := mgMap[c.MemoryGroup]; !ok {
            mgMap[c.MemoryGroup] = &MemoryGroupStatsRes{MemoryGroup: c.MemoryGroup}
        }
        mgMap[c.MemoryGroup].Contributors = append(mgMap[c.MemoryGroup].Contributors, ContributorRes{
            ContributorName: c.ContributorName, ModelName: c.ModelName,
            InputTokens: c.InputTokens, OutputTokens: c.OutputTokens,
        })
    }

    // Map to slice
    var mgList []MemoryGroupStatsRes
    for _, mg := range mgMap {
        mgList = append(mgList, *mg)
    }

    // Lineage mapping
    var lineageRes []LineageRes
    for _, l := range lineage {
        lineageRes = append(lineageRes, LineageRes{
            UUID: l.AncestorUUID, Owner: l.AncestorOwner,
            ExportedAt: l.ExportedAt, Generation: l.Generation,
        })
    }

    return &CubeStatsRes{MemoryGroups: mgList, Lineage: lineageRes}, nil
}
```

---

**注意点**:
- レスポンスは `memory_groups` 配列として返し、各要素に `stats` と `contributors` を含める
- これにより、フロントエンドで「どの分野が充実しているか」を視覚的に表示しやすくなる
