# フェーズ 29: MemoryGroup CRUD API の実装

このディレクティブは、LadybugDB に存在する `MemoryGroup` に対する完全な CRUD (Search, Get, Create, Update, Delete) 操作を提供する REST API の実装指針を詳細に記述します。

---

## 1. 概要と基本方針

### 1.1 背景
現在、`MemoryGroup` は Absorb タスク実行時に動的に作成されていますが、これを管理（一覧取得、設定変更、一括削除など）するための API が不足しています。

### 1.2 基本方針
1.  **LadybugDB 完結型**: `MemoryGroup` の情報は LadybugDB の `MemoryGroup` ノードテーブルにのみ保存します。MySQL（GORM）側には定義しません。
2.  **物理削除の徹底**: `Delete` 操作時は、そのメモリーグループに紐付く全てのデータ（Data, Document, Chunk, Node, Edge, Embedding 等）を LadybugDB から完全に物理削除します。
3.  **REST API ルールの遵守**: `docs/REST_API_STRUCTURE_AND_RULES.md` に定める 5 層構造（param, req, res, bl, handler）を厳守します。
4.  **トランザクション**: データの整合性を保つため、特に削除処理において適切なトランザクション制御を行います（KuzuDB/LadybugDB の仕様範囲内）。

---

## 2. ストレージ層の拡張

まず、LadybugDB 操作を担当する `GraphStorage` インターフェースを拡張します。

### 2.1 `GraphStorage` インターフェースの更新
*   **対象ファイル**: [interfaces.go](file:///Users/kawata/shyme/mycute/src/pkg/cuber/storage/interfaces.go)

```go
type GraphStorage interface {
    // ... 既存メソッド ...

    // SearchMemoryGroups は、IDパターンに一致するメモリーグループを検索します。
    SearchMemoryGroups(ctx context.Context, idPattern string) ([]*MemoryGroupConfig, error)

    // DeleteMemoryGroup は、メモリーグループとそれに紐付く全てのデータを物理削除します。
    DeleteMemoryGroup(ctx context.Context, memoryGroup string) error
}
```

### 2.2 `LadybugDBStorage` の実装
*   **対象ファイル**: [ladybugdb_storage.go](file:///Users/kawata/shyme/mycute/src/pkg/cuber/db/ladybugdb/ladybugdb_storage.go)

#### `SearchMemoryGroups` の実装
```go
func (s *LadybugDBStorage) SearchMemoryGroups(ctx context.Context, idPattern string) ([]*storage.MemoryGroupConfig, error) {
    query := `MATCH (mg:MemoryGroup) `
    if idPattern != "" {
        query += fmt.Sprintf("WHERE mg.id CONTAINS '%s' ", escapeString(idPattern))
    }
    query += "RETURN mg.id, mg.half_life_days, mg.prune_threshold, mg.min_survival_protection_hours, mg.mdl_k_neighbors"

    result, err := s.conn.Query(query)
    if err != nil {
        return nil, err
    }
    defer result.Close()

    var configs []*storage.MemoryGroupConfig
    for result.HasNext() {
        row, _ := result.Next()
        conf := &storage.MemoryGroupConfig{}
        if v, _ := row.GetValue(0); v != nil { conf.ID = getString(v) }
        if v, _ := row.GetValue(1); v != nil { conf.HalfLifeDays = getFloat(v) }
        if v, _ := row.GetValue(2); v != nil { conf.PruneThreshold = getFloat(v) }
        if v, _ := row.GetValue(3); v != nil { conf.MinSurvivalProtectionHours = getFloat(v) }
        if v, _ := row.GetValue(4); v != nil { conf.MdlKNeighbors = int(getInt(v)) }
        configs = append(configs, conf)
        row.Close()
    }
    return configs, nil
}
```

#### `DeleteMemoryGroup` の実装（物理一括削除）
> [!IMPORTANT]
> この処理は LadybugDB 内の全ての関連テーブルに対してフィルタを実行します。
```go
func (s *LadybugDBStorage) DeleteMemoryGroup(ctx context.Context, memoryGroup string) error {
    escapedMG := escapeString(memoryGroup)
    
    // トランザクション・ヘルパーを使用して一括削除
    return s.Transaction(ctx, func(txCtx context.Context) error {
        queries := []string{
            // 1. グラフ要素 (Node & Edge)
            fmt.Sprintf("MATCH (n:GraphNode {memory_group: '%s'}) DETACH DELETE n", escapedMG),
            
            // 2. 抽出・ベクトル関連
            fmt.Sprintf("MATCH (n:Data {memory_group: '%s'}) DETACH DELETE n", escapedMG),
            fmt.Sprintf("MATCH (n:Document {memory_group: '%s'}) DETACH DELETE n", escapedMG),
            fmt.Sprintf("MATCH (n:Chunk {memory_group: '%s'}) DETACH DELETE n", escapedMG),
            fmt.Sprintf("MATCH (n:Entity {memory_group: '%s'}) DETACH DELETE n", escapedMG),
            fmt.Sprintf("MATCH (n:Summary {memory_group: '%s'}) DETACH DELETE n", escapedMG),
            fmt.Sprintf("MATCH (n:Rule {memory_group: '%s'}) DETACH DELETE n", escapedMG),
            fmt.Sprintf("MATCH (n:Unknown {memory_group: '%s'}) DETACH DELETE n", escapedMG),
            fmt.Sprintf("MATCH (n:Capability {memory_group: '%s'}) DETACH DELETE n", escapedMG),
            
            // 3. 設定自体
            fmt.Sprintf("MATCH (mg:MemoryGroup {id: '%s'}) DELETE mg", escapedMG),
        }

        for _, q := range queries {
            // トランザクション用の接続を使用して実行
            if res, err := s.getConn(txCtx).Query(q); err != nil {
                return fmt.Errorf("DeleteMemoryGroup failed at query [%s]: %w", q, err)
            } else {
                res.Close()
            }
        }
        return nil
    })
}
```

---

## 3. REST API レイヤーの実装

### 3.1 レスポンス定義 (rtres)
*   **対象ファイル**: `src/mode/rt/rtres/memory_groups_res.go` [NEW]

```go
// Search
type SearchMemoryGroupsResData struct {
    ID                         string  `json:"id" swaggertype:"string" example:"legal-dept"`
    HalfLifeDays               float64 `json:"half_life_days" swaggertype:"number" example:"30"`
    PruneThreshold             float64 `json:"prune_threshold" swaggertype:"number" example:"0.1"`
    MinSurvivalProtectionHours float64 `json:"min_survival_protection_hours" swaggertype:"number" example:"72"`
    MdlKNeighbors              int     `json:"mdl_k_neighbors" swaggertype:"integer" example:"5"`
}
func (d *SearchMemoryGroupsResData) Of(ms []*storage.MemoryGroupConfig) *[]SearchMemoryGroupsResData {
    res := make([]SearchMemoryGroupsResData, 0, len(ms))
    for _, m := range ms {
        res = append(res, SearchMemoryGroupsResData{
            ID:                         m.ID,
            HalfLifeDays:               m.HalfLifeDays,
            PruneThreshold:             m.PruneThreshold,
            MinSurvivalProtectionHours: m.MinSurvivalProtectionHours,
            MdlKNeighbors:              m.MdlKNeighbors,
        })
    }
    return &res
}
type SearchMemoryGroupsRes struct {
    Data   []SearchMemoryGroupsResData `json:"data"`
    Errors []Err                       `json:"errors"`
}

// Get
type GetMemoryGroupResData struct {
    ID                         string  `json:"id" swaggertype:"string" example:"legal-dept"`
    HalfLifeDays               float64 `json:"half_life_days" swaggertype:"number" example:"30"`
    PruneThreshold             float64 `json:"prune_threshold" swaggertype:"number" example:"0.1"`
    MinSurvivalProtectionHours float64 `json:"min_survival_protection_hours" swaggertype:"number" example:"72"`
    MdlKNeighbors              int     `json:"mdl_k_neighbors" swaggertype:"integer" example:"5"`
}
func (d *GetMemoryGroupResData) Of(m *storage.MemoryGroupConfig) *GetMemoryGroupResData {
    return &GetMemoryGroupResData{
        ID:                         m.ID,
        HalfLifeDays:               m.HalfLifeDays,
        PruneThreshold:             m.PruneThreshold,
        MinSurvivalProtectionHours: m.MinSurvivalProtectionHours,
        MdlKNeighbors:              m.MdlKNeighbors,
    }
}
type GetMemoryGroupRes struct {
    Data   GetMemoryGroupResData `json:"data"`
    Errors []Err                 `json:"errors"`
}

// Create
type CreateMemoryGroupResData struct {
    ID                         string  `json:"id" swaggertype:"string" example:"new-group"`
    HalfLifeDays               float64 `json:"half_life_days" swaggertype:"number" example:"30"`
    PruneThreshold             float64 `json:"prune_threshold" swaggertype:"number" example:"0.1"`
    MinSurvivalProtectionHours float64 `json:"min_survival_protection_hours" swaggertype:"number" example:"72"`
    MdlKNeighbors              int     `json:"mdl_k_neighbors" swaggertype:"integer" example:"5"`
}
func (d *CreateMemoryGroupResData) Of(m *storage.MemoryGroupConfig) *CreateMemoryGroupResData {
    return &CreateMemoryGroupResData{
        ID:                         m.ID,
        HalfLifeDays:               m.HalfLifeDays,
        PruneThreshold:             m.PruneThreshold,
        MinSurvivalProtectionHours: m.MinSurvivalProtectionHours,
        MdlKNeighbors:              m.MdlKNeighbors,
    }
}
type CreateMemoryGroupRes struct {
    Data   CreateMemoryGroupResData `json:"data"`
    Errors []Err                    `json:"errors"`
}

// Update
type UpdateMemoryGroupResData struct{} // 空
type UpdateMemoryGroupRes struct {
    Data   UpdateMemoryGroupResData `json:"data"`
    Errors []Err                    `json:"errors"`
}

// Delete
type DeleteMemoryGroupResData struct{} // 空
type DeleteMemoryGroupRes struct {
    Data   DeleteMemoryGroupResData `json:"data"`
    Errors []Err                    `json:"errors"`
}
```

### 3.2 パラメータ定義 (rtparam)
*   **対象ファイル**: `src/mode/rt/rtparam/memory_groups_param.go` [NEW]

```go
type SearchMemoryGroupsParam struct {
    IDPattern string `json:"id_pattern" swaggertype:"string" example:"legal"`
}

type CreateMemoryGroupParam struct {
    ID                         string  `json:"id" swaggertype:"string" example:"new-group"`
    HalfLifeDays               float64 `json:"half_life_days" swaggertype:"number" example:"30"`
    PruneThreshold             float64 `json:"prune_threshold" swaggertype:"number" example:"0.1"`
    MinSurvivalProtectionHours float64 `json:"min_survival_protection_hours" swaggertype:"number" example:"72"`
    MdlKNeighbors              int     `json:"mdl_k_neighbors" swaggertype:"integer" example:"5"`
}

type UpdateMemoryGroupParam struct {
    HalfLifeDays               *float64 `json:"half_life_days,omitempty" swaggertype:"number"`
    PruneThreshold             *float64 `json:"prune_threshold,omitempty" swaggertype:"number"`
    MinSurvivalProtectionHours *float64 `json:"min_survival_protection_hours,omitempty" swaggertype:"number"`
    MdlKNeighbors              *int     `json:"mdl_k_neighbors,omitempty" swaggertype:"integer"`
}
```

### 3.3 リクエスト定義とバインド (rtreq)
*   **対象ファイル**: `src/mode/rt/rtreq/memory_groups_req.go` [NEW]

```go
// Search
type SearchMemoryGroupsReq struct {
    IDPattern string `json:"id_pattern"`
}
func SearchMemoryGroupsReqBind(c *gin.Context, u *rtutil.RtUtil) (SearchMemoryGroupsReq, rtres.SearchMemoryGroupsRes, bool) {
    var req SearchMemoryGroupsReq
    var res rtres.SearchMemoryGroupsRes
    if err := c.ShouldBindJSON(&req); err != nil {
        res.Errors = u.GetValidationErrs(err)
        return req, res, false
    }
    return req, res, true
}

// Get
type GetMemoryGroupReq struct {
    ID string `json:"id"`
}
func GetMemoryGroupReqBind(c *gin.Context, u *rtutil.RtUtil) (GetMemoryGroupReq, rtres.GetMemoryGroupRes, bool) {
    req := GetMemoryGroupReq{ID: c.Param("memory_group_id")}
    return req, rtres.GetMemoryGroupRes{}, true
}

// Create
type CreateMemoryGroupReq struct {
    ID                         string  `json:"id" binding:"required"`
    HalfLifeDays               float64 `json:"half_life_days" binding:"required"`
    PruneThreshold             float64 `json:"prune_threshold" binding:"required"`
    MinSurvivalProtectionHours float64 `json:"min_survival_protection_hours" binding:"required"`
    MdlKNeighbors              int     `json:"mdl_k_neighbors" binding:"required"`
}
func CreateMemoryGroupReqBind(c *gin.Context, u *rtutil.RtUtil) (CreateMemoryGroupReq, rtres.CreateMemoryGroupRes, bool) {
    var req CreateMemoryGroupReq
    var res rtres.CreateMemoryGroupRes
    if err := c.ShouldBindJSON(&req); err != nil {
        res.Errors = u.GetValidationErrs(err)
        return req, res, false
    }
    return req, res, true
}

// Update
type UpdateMemoryGroupReq struct {
    ID                         string   `json:"id"`
    HalfLifeDays               *float64 `json:"half_life_days"`
    PruneThreshold             *float64 `json:"prune_threshold"`
    MinSurvivalProtectionHours *float64 `json:"min_survival_protection_hours"`
    MdlKNeighbors              *int     `json:"mdl_k_neighbors"`
}
func UpdateMemoryGroupReqBind(c *gin.Context, u *rtutil.RtUtil) (UpdateMemoryGroupReq, rtres.UpdateMemoryGroupRes, bool) {
    req := UpdateMemoryGroupReq{ID: c.Param("memory_group_id")}
    var res rtres.UpdateMemoryGroupRes
    if err := c.ShouldBindJSON(&req); err != nil {
        res.Errors = u.GetValidationErrs(err)
        return req, res, false
    }
    return req, res, true
}

// Delete
type DeleteMemoryGroupReq struct {
    ID string `json:"id"`
}
func DeleteMemoryGroupReqBind(c *gin.Context, u *rtutil.RtUtil) (DeleteMemoryGroupReq, rtres.DeleteMemoryGroupRes, bool) {
    req := DeleteMemoryGroupReq{ID: c.Param("memory_group_id")}
    return req, rtres.DeleteMemoryGroupRes{}, true
}
```

---

### 3.4 ビジネスロジック (rtbl)
*   **対象ファイル**: `src/mode/rt/rtbl/memory_groups_bl.go` [NEW]

```go
func SearchMemoryGroups(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, req *rtreq.SearchMemoryGroupsReq, res *rtres.SearchMemoryGroupsRes) bool {
    configs, err := u.Cuber.GraphStorage.SearchMemoryGroups(c, req.IDPattern)
    if err != nil {
        return InternalServerError(c, res)
    }
    var d rtres.SearchMemoryGroupsResData
    return OK(c, d.Of(configs), res)
}

func GetMemoryGroup(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, req *rtreq.GetMemoryGroupReq, res *rtres.GetMemoryGroupRes) bool {
    config, err := u.Cuber.GraphStorage.GetMemoryGroupConfig(c, req.ID)
    if err != nil {
        return InternalServerError(c, res)
    }
    if config == nil {
        return NotFound(c, res)
    }
    var d rtres.GetMemoryGroupResData
    return OK(c, d.Of(config), res)
}

func CreateMemoryGroup(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, req *rtreq.CreateMemoryGroupReq, res *rtres.CreateMemoryGroupRes) bool {
    // 重複チェック
    existing, _ := u.Cuber.GraphStorage.GetMemoryGroupConfig(c, req.ID)
    if existing != nil {
        res.Errors = append(res.Errors, rtres.Err{Field: "id", Message: "MemoryGroup already exists"})
        return BadRequest(c, res)
    }

    config := &storage.MemoryGroupConfig{
        ID:                         req.ID,
        HalfLifeDays:               req.HalfLifeDays,
        PruneThreshold:             req.PruneThreshold,
        MinSurvivalProtectionHours: req.MinSurvivalProtectionHours,
        MdlKNeighbors:              req.MdlKNeighbors,
    }
    if err := u.Cuber.GraphStorage.UpsertMemoryGroup(c, config); err != nil {
        return InternalServerError(c, res)
    }
    var d rtres.CreateMemoryGroupResData
    return OK(c, d.Of(config), res)
}

func UpdateMemoryGroup(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, req *rtreq.UpdateMemoryGroupReq, res *rtres.UpdateMemoryGroupRes) bool {
    config, err := u.Cuber.GraphStorage.GetMemoryGroupConfig(c, req.ID)
    if err != nil || config == nil {
        return NotFound(c, res)
    }

    if req.HalfLifeDays != nil { config.HalfLifeDays = *req.HalfLifeDays }
    if req.PruneThreshold != nil { config.PruneThreshold = *req.PruneThreshold }
    if req.MinSurvivalProtectionHours != nil { config.MinSurvivalProtectionHours = *req.MinSurvivalProtectionHours }
    if req.MdlKNeighbors != nil { config.MdlKNeighbors = *req.MdlKNeighbors }

    if err := u.Cuber.GraphStorage.UpsertMemoryGroup(c, config); err != nil {
        return InternalServerError(c, res)
    }
    return OK(c, &rtres.UpdateMemoryGroupResData{}, res)
}

func DeleteMemoryGroup(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, req *rtreq.DeleteMemoryGroupReq, res *rtres.DeleteMemoryGroupRes) bool {
    if err := u.Cuber.GraphStorage.DeleteMemoryGroup(c, req.ID); err != nil {
        return InternalServerError(c, res)
    }
    return OK(c, &rtres.DeleteMemoryGroupResData{}, res)
}
```

---

### 3.5 ハンドラー (rthandler)
*   **対象ファイル**: `src/mode/rt/rthandler/hv1/memory_groups_handler.go` [NEW]

> [!NOTE]
> 実装パターンは `cubes_handler.go` と同様の `rtbl.RejectUsr` -> `rtreq.ReqBind` -> `rtbl.Action` の順序で行います。

```go
// @Tags v1 MemoryGroup
// @Router /v1/memory_groups/search [post]
func SearchMemoryGroups(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr) {
    if rtbl.RejectUsr(c, u, ju, []usrtype.UsrType{usrtype.KEY, usrtype.APX, usrtype.VDR}) { return }
    if req, res, ok := rtreq.SearchMemoryGroupsReqBind(c, u); ok {
        rtbl.SearchMemoryGroups(c, u, ju, &req, &res)
    } else { rtbl.BadRequest(c, &res) }
}

// ... Get, Create, Update, Delete も同様に実装 (省略禁止ルールに従い実際の実装では全て記述する) ...
```

---

## 4. ルーティングの設定

*   **対象ファイル**: [request_mapper.go](file:///Users/kawata/shyme/mycute/src/mode/rt/request_mapper.go)

```go
// MemoryGroup
mgs := v1.Group("/memory_groups")
mgs.POST("/search", func(c *gin.Context) {
    if u, ju, ok := GetUtil(c); ok { hv1.SearchMemoryGroups(c, u, ju) }
})
mgs.GET("/:memory_group_id", func(c *gin.Context) {
    if u, ju, ok := GetUtil(c); ok { hv1.GetMemoryGroup(c, u, ju) }
})
mgs.POST("/", func(c *gin.Context) {
    if u, ju, ok := GetUtil(c); ok { hv1.CreateMemoryGroup(c, u, ju) }
})
mgs.PATCH("/:memory_group_id", func(c *gin.Context) {
    if u, ju, ok := GetUtil(c); ok { hv1.UpdateMemoryGroup(c, u, ju) }
})
mgs.DELETE("/:memory_group_id", func(c *gin.Context) {
    if u, ju, ok := GetUtil(c); ok { hv1.DeleteMemoryGroup(c, u, ju) }
})
```

---

## 5. 検証手順

1.  **Swag 生成とビルド**: `make swag`, `make build`, `make build-linux-amd64` が通ることを確認。
2.  **グループ作成**: `POST /v1/memory_groups/` で新規グループを作成。
3.  **データ投入**: 作成した `memory_group` を指定して `Absorb` を実行し、LadybugDB にデータが作成されることを確認。
4.  **物理削除テスト**: `DELETE /v1/memory_groups/{memory_group_id}` を実行。
5.  **データ抹消の確認**: LadybugDB の各テーブルをクエリし、該当する `memory_group` のデータが 1 件も残っていないことを確認。

---

このフェーズ 29 の実装により、メモリーグループのライフサイクルを完全に制御できるようになり、不要なデータの安全な一括削除が保証されます。
