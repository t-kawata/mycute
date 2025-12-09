# Phase-11H: Delete Cube API Implementation

## 1. 概要 (Overview)
`DELETE /v1/cubes/delete` エンドポイントを実装。
`allow_delete` 権限は廃止されたため、所有者は常に削除可能です。
物理ファイルとDBレコードの両方を削除します（完全削除）。

## 2. 実装要件
*   **権限**: `usrtype.USR`
*   **削除対象**:
    *   KuzuDB ディレクトリ (物理)。
    *   `Cube` レコード。
    *   `CubeStat`, `CubeContributor`, `CubeLineage`。
    *   ※ `Export`, `BurnedKey` は整合性と履歴のため残すべきか削除すべきか判断が必要だが、今回は「Cubeに関連する従属データ」である Stat/Attrib/Lineage は消し、グローバル履歴である BurnedKey/Export は（外部キー制約がなければ）残す、あるいは `SET NULL` が望ましい。
    *   しかし、指示書上は「関連データ削除」となっているため、CASCADE 相当の削除を行うコードを書く。

> [!NOTE]
> **MemoryGroup 関連**
> 
> `delete` エンドポイントでは `memory_group` パラメータは不要です。
> Delete は Cube 全体（全 MemoryGroup データを含む）を物理削除します。

## 3. 詳細実装＆解説

### Step 1: 依存データの削除 (Transaction)

**【解説】**
外部キー制約エラーを避けるため、子テーブルから順に削除します。

**【実装コードスニペット】**
```go
func DeleteCube(ctx context.Context, cubeID uint, apxID, vdrID uint) error {
    return db.Transaction(func(tx *gorm.DB) error {
        // 1. 従属データ削除
        tx.Where("cube_id = ? AND apx_id = ? AND vdr_id = ?", cubeID, apxID, vdrID).Delete(&model.CubeModelStat{})
        tx.Where("cube_id = ? AND apx_id = ? AND vdr_id = ?", cubeID, apxID, vdrID).Delete(&model.CubeContributor{})
        tx.Where("cube_id = ? AND apx_id = ? AND vdr_id = ?", cubeID, apxID, vdrID).Delete(&model.CubeLineage{})
        
        // Export/BurnedKey は履歴的価値があるため残すか、親が消えるならゴミとなるか。
        // ここでは「Cubeを完全に消し去る」意図を汲み、削除対象とする（もしくはFK制約に任せる）。
        // 安全のため明示的に削除しない（GORMのAssociation設定次第だが、手動ではStat関係のみ消す）。

        // 2. 本体削除
        // WHERE apx/vdr を明示
        if err := tx.Where("id = ? AND apx_id = ? AND vdr_id = ?", cubeID, apxID, vdrID).Delete(&model.Cube{}).Error; err != nil {
            return err
        }
        return nil
    })
}
```

### Step 2: 物理削除

**【解説】**
DB削除がコミットされたら、物理ファイルを削除します。
順番は逆（ファイル消してからDB）でも良いですが、ファイル削除失敗時にDBが消えていると不整合になるため、DB削除成功 -> ファイル削除（失敗しても最悪ゴミファイルが残るだけでシステム動作には影響小）の順が安全です。
あるいはトランザクション内でファイルを消すが、ファイル削除はロールバックできないため、やはり最後が良い。

**【実装コードスニペット】**
```go
    // ... transaction success ...
    os.RemoveAll(cubePath)
```
