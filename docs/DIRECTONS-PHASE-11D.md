# Phase-11D: ReKey Cube API Implementation

## Export / GenKey / Import / ReKey のシーケンスの関係性

### Exportシーケンス

1. zipBuffer まで作成
2. RSAキーペア（公開鍵・秘密鍵）を生成
   - 秘密鍵は Export テーブルレコードに保管
3. AES共有鍵（32バイト=256ビット）をランダム生成
4. zipBuffer をAES共有鍵で暗号化
5. 公開鍵でAES共有鍵を暗号化
6. 秘密鍵で暗号化済みファイルのハッシュ値に対して署名を作成
7. 暗号化済みファイル、署名、公開鍵、暗号化済みAES共有鍵、Export ID をzipにまとめる
8. 7のバイナリを application/octet-stream でダウンロードさせる（拡張子は .cube）

### GenKeyシーケンス

1. Export でダウンロードされたファイルそのものと、Permissions、ExpireAt をPOSTする
2. ファイルをzipとして解凍し、暗号化済みファイル、署名、公開鍵、暗号化済みAES共有鍵、Export テーブルレコードの ID uint を取り出す
3. Export テーブルレコードの ID uint を使って Export レコードを取得して秘密鍵を入手
4. 公開鍵で署名を検証し、暗号化済みファイルが改ざんされていないことを確認
5. 秘密鍵で暗号化済みAES共有鍵を復号してAES鍵を取得
6. AES鍵、Permissions、ExpireAt、Export ID をまとめてJSONにし、そのJSON全体のハッシュに対して秘密鍵で署名を作成
7. JSON（AES鍵含む）と署名をまとめた構造体をBase64エンコードした文字列を「鍵」として出力

### Importシーケンス

1. Exportファイル(.cube)と GenKey で生成された「鍵」を入力
2. .cubeファイルをzipとして解凍し、公開鍵、暗号化済みファイル、Export ID を取り出す
3. 「鍵」をBase64デコードし、JSON（AES鍵、Permissions、ExpireAt、Export ID含む）と署名を取り出す
4. 公開鍵で署名を検証し、鍵の正当性を確認
5. JSONのExport IDと.cubeファイルのExport IDが一致することを確認
6. ExpireAtが現在時刻より未来であることを確認（有効期限チェック）
7. Permissionsを確認し、実行しようとしている操作が許可されているかチェック
8. JSONから取り出したAES鍵で暗号化済みファイルを復号
9. 復号されたzipBufferを展開して元のファイル群を取得
10. 適切にファイルの保管とデータの登録を行う

### ReKeyシーケンス

1. CubeテーブルのレコードIDと、GenKeyで生成された新しい「鍵」をPOSTする
2. CubeレコードIDでCubeテーブルからレコードを取得し、紐づくExport IDを確認
3. 「鍵」をBase64デコードし、JSON（AES鍵、Permissions、ExpireAt、Export ID含む）と署名を取り出す
4. Export IDを使ってExportレコードから公開鍵を取得
5. 公開鍵で署名を検証し、鍵の正当性を確認
6. JSONのExport IDとCubeレコードに紐づくExport IDが一致することを確認
7. 新しいExpireAtが現在時刻より未来であることを確認（有効期限チェック）
8. Cubeテーブルレコードの Permissions と ExpireAt を、「鍵」に含まれる新しい値で更新

## 1. 概要 (Overview)
`POST /v1/cubes/rekey` エンドポイントを実装。
既存の Cube に対し、新しい `Key` を適用することで、権限（Permissions）や有効期限（ExpireAt）を更新します。
データの再インポートは行わず、メタデータのみを更新する軽量な処理です。

## 2. 実装要件
*   **Authentication**: USR (Owner of the Cube).
*   **Limit**: `RekeyLimit` (Current permissions).
*   **Inputs**:
    *   `cube_id` (or UUID) - 更新対象のCube
    *   `key` (string) - 新しい権限を含む署名付き鍵
*   **Security Sequence**:
    1.  **Identify Cube & Export ID**:
        *   `cube_id` から `cubes` テーブルを参照し、現在のレコードを取得。
        *   このCubeがどの `Export` (Source) から来たかを特定する必要がある (Lineage or ExportID stored in Cube?)。
        *   ※ ReKeyを行うには、そのCubeが「Importされたものである（Export ID情報を持っている）」か、あるいは「自分がExportしたもののコピー（Export IDと紐づく）」である必要があります。
        *   ここでは、Cubeモデルまたは関連テーブルに `SourceExportID` があると仮定、あるいは `cube_lineages` から直近の親 Export ID を特定して使用する。
    2.  **Parse & Verify Key**:
        *   `Key` -> `Payload` & `Signature`.
        *   `Payload` -> JSON (`ExportID`, `Permissions`, `ExpireAt`, ...).
        *   JSON 内の `ExportID` を使って、`exports` テーブルからレコード（および公開鍵？いや、Exportレコードには秘密鍵しかない可能性がある。Import時と異なり `.cube` ファイルがないため公開鍵が手元にない。）
        *   **重要**: ReKey を可能にするためには、`exports` テーブル自体に `PublicKey` も保存しておくか、PrivateKey から PublicKey を復元する必要があります。
        *   ここでは、Exportレコードから（秘密鍵を使って）公開鍵を取得し、署名を検証します。
    3.  **Integrity Checks**:
        *   Key 内の `ExportID` と、対象 Cube の `SourceExportID` が一致することを確認（「このCubeのための鍵」であるか）。
        *   Key 内の `ExpireAt` が現在時刻より未来であること。
    4.  **Update**:
        *   署名検証OKなら、`Payload.Permissions` と `Payload.ExpireAt` で `cubes` レコードを更新。
        *   `RekeyLimit` を消費。

## 3. 詳細実装＆解説 (Detailed Implementation & Reasoning)

### Step 1: Identify Key Source
**【解説】**
ReKeyは「元の所有者が発行した新しい許可証」を適用する行為です。したがって、検証には「元の所有者の公開鍵」が必要です。
Import時とは異なり `.cube` ファイル（公開鍵同梱）がないため、DBの `exports` テーブルから鍵を取得する必要があります。
つまり、ReKeyは「同じシステム内（または連携されたシステム間）」で、Export履歴にアクセスできる環境でのみ可能です。

**【実装コードスニペット】**
```go
// 1. Parse Key
// ... Payload & Signature ...

// 2. Fetch Export Record
var exportRecord model.Export
if err := db.First(&exportRecord, payload.ExportID).Error; err != nil {
    return Error("Export record not found (Cannot ReKey independent cube?)")
}

// 3. Derive Public Key from Private Key (stored in DB)
block, _ := pem.Decode([]byte(exportRecord.PrivateKey))
privateKey, _ := x509.ParsePKCS1PrivateKey(block.Bytes)
publicKey := &privateKey.PublicKey

// 4. Verify Signature
hash := sha256.Sum256(payloadBytes)
err := rsa.VerifyPSS(publicKey, crypto.SHA256, hash[:], sigBytes, nil)
if err != nil {
    return Error("Invalid key signature")
}
```

### Step 2: Update Cube
**【解説】**
正当性が確認できたら、Cubeの設定を上書き更新します。

**【実装コードスニペット】**
```go
// 5. Update Permissions & Expire
cube.Permissions = payload.Permissions
cube.ExpireAt = payload.ExpireAt
db.Save(&cube)

// Log the action...
```

## 注意事項
*   ReKey の前提として、対象の Cube がどの Export ID に由来するかを知る手段が必要です。
    *   案1: `cubes` テーブルに `SourceExportID` カラムを追加。
    *   案2: `cube_lineages` の直近のレコード（Generationが最大のもの）の `ExportedAt` や `AncestorUUID` から推測する（少し弱い）。
    *   推奨: Import時に `SourceExportID` を `cubes` テーブル（または `imported_cubes` 的な拡張テーブル）に保存する設計にする。

### 補足: DBスキーマへの影響
Phase-11D (ReKey) を実現するためには、CubeとExportの紐付けを強固にする必要があります。
`model.Cube` に `SourceExportID *uint` を追加することを推奨します。
