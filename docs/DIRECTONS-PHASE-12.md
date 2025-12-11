# Phase-12: Cube Get & Search Implementation

## 0. はじめに (Introduction)

本ドキュメントは、**Phase-12: Cube Get & Search Implementation** の実装を「迷いなく」「確実に」遂行するための詳細な開発指示書です。
Phase-11 で実装された Cube の基本操作に加え、Cube の情報を参照・検索するための `Get` および `Search` エンドポイントを実装します。

> [!IMPORTANT]
> **方針変更: Stats → Get**
> 当初計画されていた `Stats` エンドポイントは廃止し、より汎用的な `Get` エンドポイントとして再定義・実装します。
> これにより、Cube の基本情報と統計情報を一つのレスポンスで取得可能にします。

> [!CRITICAL]
> **待機指示**
> 本ドキュメントの内容は、**ユーザーの明示的な「実装開始」の指示があるまで、コードへの反映を行わないでください**。
> まずはドキュメントの承認を得ることを優先します。

---

## 1. 全体スケジュールとサブフェーズ (Schedule & Sub-phases)

| Sub-phase | 対応エンドポイント | HTTP Method | 主なタスク |
|-----------|-------------------|-------------|-----------|
| **Phase-12A** | `get` | `GET` | 旧Stats計画の改修。CubeID指定で詳細情報(Cube+Stats+Lineage)を取得する `GET /v1/cubes/get` の実装。 |
| **Phase-12B** | `search` | `GET` | 検索条件を指定して複数のCube詳細情報を取得する `GET /v1/cubes/search` の実装。 |

---

## 2. 共通レスポンス構造 (Common Response Structure)

`Get` および `Search` エンドポイントでは、Cube に関する包括的な情報を返す共通のデータ構造を使用します。

### 2.1 データ構造定義
レスポンスは以下の3つの要素を含む複合オブジェクトとなります。

1.  **Cube**: `model.Cube` レコードそのもの（ID, Name, Description, Permissions等）
2.  **Lineage**: Cube の系譜情報（先祖の履歴）
3.  **MemoryGroups**: 分野（MemoryGroup）ごとの統計情報

```go
// GetCubeResData (Getレスポンス / Searchレスポンスの要素)
type GetCubeResData struct {
    Cube         model.Cube            `json:"cube"`
    Lineage      []LineageRes          `json:"lineage"`
    MemoryGroups []MemoryGroupStatsRes `json:"memory_groups"`
}

// MemoryGroupStatsRes (Phase-11I 計画からの継承)
type MemoryGroupStatsRes struct {
    MemoryGroup  string            `json:"memory_group"`
    Stats        []ModelStatRes    `json:"stats"`
    Contributors []ContributorRes  `json:"contributors"`
}
```

---

## 3. Phase-12A: Get Endpoint Implementation

**概要**:
指定された `CubeID` に基づき、Cube の基本情報、統計情報、および系譜情報を一括して取得します。

**変更点 (vs 旧Stats計画)**:
*   エンドポイント名を `stats` から `get` に変更。
*   リクエストパラメータから `memory_group` を**廃止**（Cube全体を対象とするため）。
*   レスポンスに `cube` (model.Cube) を追加。

### 3.1 仕様
*   **Method**: `GET`
*   **Path**: `/v1/cubes/get`
*   **Query Params**:
    *   `cube_id` (Required, uint): 取得対象のCube ID
*   **Permission Checks**:
    *   `ApxID`, `VdrID` によるパーティションチェック（必須）。
    *   `Permissions.AllowStats` チェック: `false` の場合、統計情報（`MemoryGroups`, `Lineage`）は空にするか、あるいはエラーとするか（**要件定義**: ここでは「統計情報は空リストで返す（隠蔽する）」仕様とします）。

### 3.2 実装詳細 (rtbl)

```go
func GetCube(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, req *rtreq.GetCubeReq, res *rtres.GetCubeRes) bool {
    // 1. Cube取得 (getCube helper使用)
    cube, err := getCube(u, req.CubeID, *ids.ApxID, *ids.VdrID)
    if err != nil { return NotFoundCustomMsg(...) }

    // 2. 統計情報の取得準備
    var lineageRes []LineageRes
    var memoryGroupsRes []MemoryGroupStatsRes

    // 3. AllowStats チェック
    perm, _ := common.ParseDatatypesJson[model.CubePermission](&cube.Permissions)
    if perm.AllowStats {
        // 4. 統計・系譜情報の取得 (Stats取得ロジック)
        lineageRes = fetchLineage(...)
        memoryGroupsRes = fetchMemoryGroupStats(...)
    }

    // 5. レスポンス構築
    data := rtres.GetCubeResData{
        Cube:         *cube,
        Lineage:      lineageRes,
        MemoryGroups: memoryGroupsRes,
    }
    return OK(c, &data, res)
}
```

---

## 4. Phase-12B: Search Endpoint Implementation

**概要**:
検索条件に基づいて Cube を検索し、ヒットした各 Cube について詳細情報（Cube+Stats+Lineage）を返します。

**実装方針**:
`src/mode/rt/rtbl/usrs_bl.go` の `SearchUsrs` の実装パターンを踏襲し、`restsql` パッケージを利用して検索を行います。

### 4.1 仕様
*   **Method**: `POST`
*   **Path**: `/v1/cubes/search`
*   **Query Params** (`SearchCubesParam` -> `SearchCubesReq`):
    *   `id`, `name`, `description` 等の検索条件 (restsql準拠)
    *   `limit`, `offset` 等のページネーション
*   **Response**: `[]GetCubeResData` (Getのレスポンスデータの配列)

### 4.2 実装詳細 (rtbl)

`SearchUsrs` のパターンと同様に、まずは対象レコード(`model.Cube`)を検索し、その後、各レコードに対して統計情報を付加します。

```go
func SearchCubes(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, req *rtreq.SearchCubesReq, res *rtres.SearchCubesRes) bool {
    // 1. Cube レコードの検索 (restsql パターン)
    cubes := []model.Cube{}
    // restsql.SearchCubes 相当の処理 (汎用的なSearch関数 または 新規実装)
    // 検索対象カラム: "name", "description" 等
    r := restsql.SearchCubes(u.DB, &cubes, ju.IDs(false), "c1", req, &[]string{"name", "description"}, nil, ...)
    if r.Error != nil { return InternalServerError(...) }

    // 2. 詳細情報の付加 (N+1 問題に注意しつつ実装、またはループ処理)
    results := []rtres.GetCubeResData{}

    for _, cube := range cubes {
        var lineageRes []LineageRes = []LineageRes{}
        var memoryGroupsRes []MemoryGroupStatsRes = []MemoryGroupStatsRes{}

        // AllowStats チェック & データ取得
        perm, _ := common.ParseDatatypesJson[model.CubePermission](&cube.Permissions)
        if perm.AllowStats {
            lineageRes = fetchLineage(u.DB, cube.ID, ...)
            memoryGroupsRes = fetchMemoryGroupStats(u.DB, cube.ID, ...)
        }

        results = append(results, rtres.GetCubeResData{
            Cube:         cube,
            Lineage:      lineageRes,
            MemoryGroups: memoryGroupsRes,
        })
    }

    // 3. レスポンス返却 (Of メソッドは使わず直接構築)
    return OK(c, &results, res)
}
```

> [!TIP]
> **Performance Consideration**
> `Search` で大量の Cube がヒットした場合、ループ内で個別に統計情報をSELECTするとパフォーマンスが悪化する可能性があります（N+1問題）。
> 実装初期段階ではループ処理で可としますが、将来的には ID リストを使って一括取得し、メモリ上でマッピングする最適化を検討してください。

---

## 5. 作業手順 (Workflow)

以下の順序で実装を行ってください。**必ず1ステップごとにビルド(`make build`)を確認すること。**

1.  **Phase-12A (Get)**:
    *   `rtparam`, `rtreq`, `rtres` の定義 (`GetCubeParam` 等)
    *   `GetCube` (Handler, BL) の実装
    *   `11I` で計画していた `fetchLines`, `fetchMemoryGroups` 等のヘルパー関数実装
2.  **Phase-12B (Search)**:
    *   `restsql` への `SearchCubes` 追加 (または既存利用)
    *   `rtparam`, `rtreq`, `rtres` の定義 (`SearchCubesParam` 等)
    *   `SearchCubes` (Handler, BL) の実装
3.  **Route Mapping**:
    *   `request_mapper.go` へのルート追加

---
**END OF DIRECTIVES**
