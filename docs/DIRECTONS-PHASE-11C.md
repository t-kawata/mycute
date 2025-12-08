# Phase-11C: Export Cube API Implementation

## 1. 概要 (Overview)
`GET /v1/cubes/export` エンドポイントを実装し、Cube を `.cube` ファイルとして書き出します。
ここでは **ExportLimit の消費** と **Export Record の作成** が重要タスクです。鍵の発行は行いません。

## 2. 実装要件 (Requirements)
*   **エンドポイント**: `GET /v1/cubes/export`
*   **権限**: `usrtype.USR`
*   **Lineage**: `ExportedAt` (ms) を記録。
*   **Limit Logic**: `ExportLimit`
    *   `0`: 無制限。
    *   `> 0`: 残回数。消費ロジック `val - 1` (もし `0` になるなら `-1` にセット)。
    *   `< 0`: 禁止。
*   **Export Record**: `Export` テーブルにレコード作成。

## 3. 詳細実装＆解説 (Detailed Implementation & Reasoning)

### Step 1: 権限チェックと消費 (BL層)

**【解説】**
まず `Permissions` JSON をパースして `CubePermission` 構造体に復元します。
`ExportLimit` をチェックします。
消費が発生する場合（Limit > 0）、値を減算して DB に保存します。
**重要**: `1` から `1` を引くと `0` になりますが、システム上 `0` は「無制限」を意味するため、ここでは `-1`（終了）を明示的にセットする必要があります。

**【実装コードスニペット】**
```go
// BL層
func ExportCube(ctx context.Context, cubeID uint, apxID, vdrID uint) (*bytes.Buffer, error) {
    // 1. Cube 取得
    var cube model.Cube
    if err := db.First(&cube, "id = ? AND apx_id = ? AND vdr_id = ?", cubeID, apxID, vdrID).Error; err != nil {
        return nil, err
    }

    // 2. Permission Check & Consume
    var perm model.CubePermission
    json.Unmarshal(cube.Permissions, &perm)

    limit := perm.ExportLimit
    if limit < 0 {
        return nil, fmt.Errorf("export limit exceeded (forbidden)")
    }
    
    // Limit > 0 の場合のみ消費
    if limit > 0 {
        newLimit := limit - 1
        if newLimit == 0 {
            newLimit = -1 // 0=Unlimited回避のため -1(Finished) にする
        }
        perm.ExportLimit = newLimit
        
        // 更新保存 (Atomicにやるべきだが、ここでは簡易記述。実際はTransaction推奨)
        newJSON, _ := json.Marshal(perm)
        cube.Permissions = datatypes.JSON(newJSON)
        db.Save(&cube)
    }
    // Limit == 0 (Unlimited) の場合は何もしない
```

### Step 2: データ準備と Lineage 更新

**【解説】**
エクスポート用に新しい UUID を発行します。
Lineage リストを取得し、現在の Cube と所有者情報を末尾に追加します。この際、`ExportedAt` に現在時刻 (UnixMilli) を記録します。
これを `metadata.json` に書き込みます。

**【実装コードスニペット】**
```go
    // 3. New UUID & Lineage
    newUUID := uuid.NewString()

    // 現在の Lineage 取得
    var ancestors []model.CubeLineage
    db.Where("cube_id = ? AND apx_id = ? AND vdr_id = ?", cube.ID, apxID, vdrID).Order("generation asc").Find(&ancestors)

    // 自分自身の情報を Lineage に追加 (Metadata用)
    myLineage := model.CubeLineage{
        AncestorUUID: cube.UUID,
        AncestorOwner: GetUsrName(cube.UsrID), // ユーザー名取得
        ExportedAt: time.Now().UnixMilli(),
        Generation: len(ancestors) + 1,
    }
    // エクスポートデータに含めるリスト
    exportLineageList := append(ancestors, myLineage) 
```

### Step 3: ファイル作成とハッシュ計算

**【解説】**
KuzuDB ディレクトリ全体をコピーし、メタデータを同梱して Zip 圧縮します。
作成された Zip ファイルの SHA256 ハッシュを計算します。これは `Export` テーブルに記録するためです。

### Step 4: Record 保存

**【解説】**
エクスポートの事実を `Export` テーブルに記録します。
これにより、後でこの `NewUUID` に対して鍵を発行 (`genkey`) することが許可されます。

**【実装コードスニペット】**
```go
    // ... Zip作成 (zipBuffer) ...
    // Stats Export (JSON)
    var modelStats []model.CubeModelStat
    db.Where("cube_id = ? AND apx_id = ? AND vdr_id = ?", cube.ID, apxID, vdrID).Find(&modelStats)
    statsUsageJSON, _ := json.Marshal(modelStats)
    cuber.AddToZip(zipBuffer, "stats_usage.json", statsUsageJSON)

    var contributors []model.CubeContributor
    db.Where("cube_id = ? AND apx_id = ? AND vdr_id = ?", cube.ID, apxID, vdrID).Find(&contributors)
    statsContribJSON, _ := json.Marshal(contributors)
    cuber.AddToZip(zipBuffer, "stats_contributors.json", statsContribJSON)

    hash := CalculateSHA256(zipBuffer.Bytes())

    // 4. Export Record
    record := model.Export{
        CubeID: cube.ID, // Source
        NewUUID: newUUID,
        Hash: hash,
        ApxID: apxID, VdrID: vdrID,
    }
    db.Create(&record)

    return zipBuffer, nil
}
```

---
**注意点**:
*   `ExportLimit` の消費ロジック (`1 -> -1`) は非常に重要です。テストケースで必ず確認してください。
*   `GET` リクエストですが、副作用（Limit消費、Record作成）があるため、厳密には冪等ではありません。
