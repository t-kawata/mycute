# Phase-11B: Import Cube API Implementation

## 1. 概要 (Overview)
`POST /v1/cubes/import` エンドポイントを実装し、外部の `.cube` ファイルを取り込みます。
最重要ポイントは **Burn-on-use (鍵は使い捨て)** の徹底です。KeyID の重複チェックと記録をアトミックに行う必要があります。

## 2. 実装要件 (Requirements)
*   **エンドポイント**: `POST /v1/cubes/import`
*   **権限**: `usrtype.USR`
*   **鍵検証**:
    *   署名検証。
    *   `Expire` チェック。
    *   **Burn Check**: `KeyID` が `BurnedKey` テーブルに存在しないこと。
*   **Import**:
    *   UUID の維持（`.cube` メタデータを使用）。
    *   ファイルの物理配置。
    *   DBレコード (`Cube`, `Lineage`, `Stat`, `Contributor`) の作成。
    *   Lineageには `ExportedAt` タイムスタンプが含まれる場合、それも保存。
    *   **Burn Record**: `BurnedKey` に使用記録を追加。

## 3. 詳細実装＆解説 (Detailed Implementation & Reasoning)

### Step 1: リクエスト受付とファイル保存 (Temporary)

**【解説】**
アップロードされたファイルはまず一時ディレクトリに保存・展開します。
いきなり本番パスに展開すると、検証失敗時にゴミが残るリスクがあるためです。
`os.MkdirTemp` を使用して安全な作業領域を確保します。

**【実装コードスニペット】**
```go
// BL層
func ImportCube(ctx context.Context, fileHeader *multipart.FileHeader, keyString string, apxID, vdrID uint, usrID string) error {
    // 1. 一時ディレクトリ作成
    tempDir, err := os.MkdirTemp("", "cube_import_")
    if err != nil { return err }
    defer os.RemoveAll(tempDir) // 関数終了時に確実に削除

    // 2. zip保存 & 展開 (pkg/cuber 利用想定)
    srcPath := filepath.Join(tempDir, "upload.cube")
    if err := c.SaveUploadedFile(fileHeader, srcPath); err != nil { return err }
    
    extractedPath := filepath.Join(tempDir, "extracted")
    if err := cuber.Unzip(srcPath, extractedPath); err != nil { return err }

    // 3. Metadata 読み込み
    meta, err := cuber.ReadMetadata(extractedPath) // UUID, Lineageなどを含む
    if err != nil { return err }
    
    // ... 次へ
}
```

### Step 2: 鍵検証と Burn Check

**【解説】**
鍵文字列（JWT等で署名されたトークン想定）を解析し、ペイロードを取り出します。
`BurnedKey` テーブルを検索し、もし既に `KeyID` が存在すれば、その鍵は使用済みであるため **即時エラー** とします。
また、`TargetUUID` がインポートしようとしている Cube の UUID と一致するかも確認します。

**【実装コードスニペット】**
```go
    // 4. キー解析 (pkg/crypto 利用想定)
    keyPayload, err := mycrypto.ParseAndVerifyKey(keyString)
    if err != nil { return err }

    // 5. UUID一致確認
    if keyPayload.TargetUUID != meta.UUID {
        return fmt.Errorf("key target UUID mismatch")
    }

    // 6. Burn Check (DB)
    var count int64
    db.Model(&model.BurnedKey{}).
      Where("key_id = ? AND apx_id = ? AND vdr_id = ?", keyPayload.KeyID, apxID, vdrID).
      Count(&count)
    if count > 0 {
        return fmt.Errorf("key already used (burned)")
    }
```

### Step 3: ディレクトリ移動 (Final Placement)

**【解説】**
検証が通ったら、KuzuDB ディレクトリを本番パスに移動します。
もし既に同名ディレクトリ（UUID重複）が存在する場合は、上書きするかエラーにするかですが、通常「別ユーザーが同じCubeを持つ」ことはあり得ますが、「同一ユーザーが同一UUIDを持つ」場合は上書き（再インポート）の可能性があります。
しかし、DBレコードとの整合性を考えると、**同一ユーザー・同一UUIDの重複は禁止**（既に存在するなら Rekey せよ）とするのが安全です。

**【実装コードスニペット】**
```go
    // 7. 本番パス決定
    finalPath := filepath.Join(os.Getenv("DB_DIR_PATH"), fmt.Sprintf("%d-%d-%d", apxID, vdrID, usrID), meta.UUID+".db")
    
    // 既存チェック
    if _, err := os.Stat(finalPath); err == nil {
        return fmt.Errorf("cube already exists. use rekey to update permissions")
    }

    // 移動
    if err := os.MkdirAll(filepath.Dir(finalPath), 0755); err != nil { return err }
    if err := os.Rename(filepath.Join(extractedPath, "db"), finalPath); err != nil { return err }
```

### Step 4: DB保存 (Atomic Update)

**【解説】**
`Cube` レコードの作成と `BurnedKey` の記録は **必ず単一のトランザクション** で行います。
これにより、「鍵だけ消費されて Cube が作られない」あるいはその逆の不整合を防ぎます。
Lineage や Stats のインポートもここで行います。
Permissions は鍵から取得した詳細設定を使用します。

**【実装コードスニペット】**
```go
    // 8. DB Transaction
    return db.Transaction(func(tx *gorm.DB) error {
        // Permissions JSON化
        permJSON, _ := json.Marshal(keyPayload.Permissions)

        // Cube 作成
        cube := model.Cube{
            UUID: meta.UUID,
            UsrID: usrID,
            // ... Name/Desc from meta ...
            ExpireAt: keyPayload.ExpireAt, // 鍵の期限
            Permissions: datatypes.JSON(permJSON), // 鍵の権限
            ApxID: apxID, VdrID: vdrID,
        }
        if err := tx.Create(&cube).Error; err != nil { return err }

        // Lineage Import (Loop)
        // meta.Lineage (loop) -> tx.Create(&model.CubeLineage{..., ExportedAt: l.ExportedAt, ...})

        // Stats Import (Restore from JSON)
        // stats_usage.json
        if bytes, err := os.ReadFile(filepath.Join(extractedPath, "stats_usage.json")); err == nil {
            var stats []model.CubeModelStat
            if err := json.Unmarshal(bytes, &stats); err == nil {
                for _, s := range stats {
                    s.ID = 0 // Reset ID
                    s.CubeID = cube.ID
                    s.ApxID = apxID; s.VdrID = vdrID
                    tx.Create(&s)
                }
            }
        }
        // stats_contributors.json
        if bytes, err := os.ReadFile(filepath.Join(extractedPath, "stats_contributors.json")); err == nil {
            var contribs []model.CubeContributor
            if err := json.Unmarshal(bytes, &contribs); err == nil {
                for _, c := range contribs {
                    c.ID = 0 // Reset ID
                    c.CubeID = cube.ID
                    c.ApxID = apxID; c.VdrID = vdrID
                    tx.Create(&c)
                }
            }
        }

        // Burn Key 記録 (重要)
        burned := model.BurnedKey{
            KeyID: keyPayload.KeyID,
            UsedByUsrID: usrID,
            UsedForCubeUUID: cube.UUID,
            ActionType: "import",
            ApxID: apxID, VdrID: vdrID,
        }
        if err := tx.Create(&burned).Error; err != nil { return err }

        return nil
    })
    // Transaction終了後、エラーがあれば defer で tempDir が消える。
    // 成功していれば finalPath にデータは移動済み。
```

---
**注意点**:
*   `ActionType: "import"` を間違えないこと。
*   トランザクションエラー時の `finalPath` のクリーンアップが必要かも検討（トランザクション内でエラーなら `os.RemoveAll(finalPath)` を呼ぶなど）。上記コードでは `Rename` 後にトランザクションをしているため、失敗時は手動クリーンアップが必要。
    *   **改善案**: トランザクション成功後に `Rename` する手もあるが、DBだけ出来てファイルがない状態も怖い。
    *   **推奨手順**: ファイル移動 -> トランザクション(DB作成) -> 失敗ならファイル削除。これが一番安全。
