# Phase-11J: GenKey Cube API Implementation

## 1. 概要 (Overview)
`POST /v1/cubes/genkey` エンドポイントを実装。
Export 済みの Cube (Record) または所有する Cube に対し、**権限継承ルールの範囲内で** 新しい鍵を発行します。

## 2. 実装要件
*   **Source**: 自所有の Cube または `Export` レコード。
*   **Limit**: `GenKeyLimit` (0=Unlim, >0=Decr, <0=Forbid).
*   **Inheritance**:
    *   発行する鍵の権限 (`newPerm`) は、ソースの権限 (`srcPerm`) のサブセットでなければならない。
    *   例: 親が `AbsorbLimit=-1` (禁止) なら、子は `AbsorbLimit>0` (許可) にできない。
    *   例: 親の `Expire` より後の `Expire` は設定できない。

> [!NOTE]
> **MemoryGroup 関連**
> 
> `genkey` エンドポイントでは `memory_group` パラメータは不要です。
> 鍵は Cube 全体へのアクセス権限を表し、MemoryGroup ごとの制御は行いません。

## 3. 詳細実装＆解説 (Detailed Implementation & Reasoning)

### Step 1: Source 特定 (BL層)

**【解説】**
`target_uuid` がどこにあるか探します。優先順位は 1. `Cube` (自分が持っている実体), 2. `Export` (自分が輸出した記録)。
どちらかが見つかれば、その時点での権限（`Export`の場合は実体の権限を参照するか、Export時の記録...いや、Exportレコードには権限が記録されていないため、Export元の `CubeID` を辿って現在の親Cubeの権限を参照するのが妥当）を取得します。

**【実装コードスニペット】**
```go
    // 1. Try Cube
    var srcCube model.Cube
    err := db.Where("uuid = ? AND usr_id = ? AND apx_id = ? AND vdr_id = ?", targetUUID, usrID, apxID, vdrID).First(&srcCube).Error
    if err != nil {
        // 2. Try Export
        var export model.Export
        if err2 := db.Where("new_uuid = ? AND apx_id = ? AND vdr_id = ?", targetUUID, apxID, vdrID).First(&export).Error; err2 == nil {
             // Exportが見つかった -> 親Cubeを取得
             // ここでも apx/vdr で絞る
             db.Where("id = ? AND apx_id = ? AND vdr_id = ?", export.CubeID, apxID, vdrID).First(&srcCube)
             // ※ ここで srcCube の UsrID == usrID チェックが必要 (自分のExportか？)
        } else {
             return fmt.Errorf("target not found")
        }
    }
```

### Step 2: 権限継承チェック (Inheritance)

**【解説】**
親が許可していないことを子が許可することはできません。
Limit 値については、親が `0` (無制限) なら子は何でもOK。
親が `>0` なら、子はそれ以下...というのは難しい（切り出された時点で独立するので）。
重要なのは「機能の有無」です。
親が `<0` (禁止) している機能は、子も `<0` でなければなりません。
Expire については、親の期限を超えることはできません。

**【実装コードスニペット】**
```go
func CheckInheritance(parent, child model.CubePermission, pExpire, cExpire *time.Time) error {
    // 禁止機能の復活チェック
    if parent.AbsorbLimit < 0 && child.AbsorbLimit >= 0 { return fmt.Errorf("cannot enable absorb") }
    if parent.ExportLimit < 0 && child.ExportLimit >= 0 { return fmt.Errorf("cannot enable export") }
    // ... 他のLimitも同様

    // Expire チェック
    if pExpire != nil {
        if cExpire == nil { return fmt.Errorf("cannot remove expiration") }
        if cExpire.After(*pExpire) { return fmt.Errorf("cannot extend expiration") }
    }
    return nil
}
```

### Step 3: 鍵生成と保存

**【解説】**
チェックが通ったら、`GenKeyLimit` を消費し、署名付き鍵を生成します。

**【実装コードスニペット】**
```go
    // Limit Check
    if srcPerm.GenKeyLimit < 0 { return error }
    // ... consume logic ...

    // Key Gen
    keyID := uuid.NewString()
    token, err := mycrypto.SignKey(keyID, targetUUID, childPerm, cExpire)

    // Save Limit
    db.Save(&srcCube)

    return token
```
