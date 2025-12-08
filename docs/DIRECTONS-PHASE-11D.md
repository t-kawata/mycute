# Phase-11D: Rekey Cube API Implementation

## 1. 概要 (Overview)
`PUT /v1/cubes/rekey` エンドポイントを実装し、既存の Cube に新しい鍵を適用します。
ポイントは `RekeyLimit` のチェックロジックと、`BurnedKey` へのアトミックな記録です。

## 2. 実装要件 (Requirements)
*   **エンドポイント**: `PUT /v1/cubes/rekey`
*   **権限**: `usrtype.USR`
*   **Limit Logic**: `RekeyLimit`
    *   `0`: 無制限。
    *   `> 0`: 許可 (消費はしない)。
        *   ※ Rekey が成功すると `Permissions` 全体が新しい鍵の内容で上書きされるため、現在の Limit を減算して保存する意味がありません。現在の権利が残っているかチェックするだけで十分です。
    *   `< 0`: 禁止。
*   **Burn Check**: `KeyID` 未使用確認。

## 3. 詳細実装＆解説 (Detailed Implementation & Reasoning)

### Step 1: 権限チェック (BL層)

**【解説】**
現在の鍵で Rekey する権利があるか確認します。
`RekeyLimit < 0` ならエラーです。それ以外 (`0` または `>0`) なら OK です。

**【実装コードスニペット】**
```go
// BL層
func RekeyCube(ctx context.Context, cubeID uint, keyString string, apxID, vdrID uint) error {
    // Cube 取得
    var cube model.Cube
    db.Where("id = ? AND apx_id = ? AND vdr_id = ?", cubeID, apxID, vdrID).First(&cube)

    // Permission Check
    var perm model.CubePermission
    json.Unmarshal(cube.Permissions, &perm)

    if perm.RekeyLimit < 0 {
        return fmt.Errorf("rekey forbidden")
    }
    // ここで減算は不要（上書きされるため）
    
    // ... 次へ
}
```

### Step 2: 鍵検証と Burn Check

**【解説】**
Import 時と同様に、鍵の署名や Expire を検証し、`BurnedKey` テーブルを確認します。
さらに、`TargetUUID` がこの Cube の UUID と一致することも重要です。

**【実装コードスニペット】**
```go
    // 鍵解析
    payload, err := mycrypto.ParseAndVerifyKey(keyString)

    // UUID一致
    if payload.TargetUUID != cube.UUID { return fmt.Errorf("uuid mismatch") }

    // Burn Check
    var count int64
    db.Model(&model.BurnedKey{}).
        Where("key_id = ? AND apx_id = ? AND vdr_id = ?", payload.KeyID, apxID, vdrID).
        Count(&count)
    if count > 0 { return fmt.Errorf("key already used") }
```

### Step 3: DB更新 (Atomic)

**【解説】**
`Cube` の権限更新と `BurnedKey` の追加をトランザクションで行います。
これにより、鍵だけ消費されて権限が変わらない事態を防ぎます。
`UpdatedAt` も更新します。

**【実装コードスニペット】**
```go
    return db.Transaction(func(tx *gorm.DB) error {
        // Burn
        burned := model.BurnedKey{
            KeyID: payload.KeyID,
            ActionType: "rekey",
            UsedForCubeUUID: cube.UUID,
            // ...
        }
        if err := tx.Create(&burned).Error; err != nil { return err }

        // Update Cube
        newPermJSON, _ := json.Marshal(payload.Permissions)
        cube.Permissions = datatypes.JSON(newPermJSON)
        cube.ExpireAt = payload.ExpireAt
        
        if err := tx.Save(&cube).Error; err != nil { return err }
        
        return nil
    })
```
