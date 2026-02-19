# Phase-11B: Import Cube API Implementation

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
`POST /v1/cubes/import` エンドポイントを実装。
`.cube` ファイルと `Key` を受け取り、Cubeをシステムに復元（Import）します。
「鍵」の検証、ファイルの復号、データのデータベースへの格納を行います。

## 2. 実装要件
*   **Authentication**: USR.
*   **Permissions**: `Import` 自体の制限はないが、Key 内の権限 (`Permissions`) に従う。
*   **Inputs**:
    *   `.cube` file (multipart/form-data)
    *   `key` (string)
    *   `name`, `description` (optional override)
*   **Security Sequence**:
    1.  **Parse Inputs**:
        *   `.cube` (Zip) -> `public_key.pem`, `encrypted_data.bin`, `export_id.txt` 等を取得。
        *   `Key` -> Split by `.` -> `Payload(Base64)` & `Signature(Base64)`.
        *   `Payload` -> Decode -> JSON (`AESKey`, `Permissions`, `ExpireAt`, `ExportID`).
    2.  **Verify Key Signature**:
        *   `.cube` 内の `public_key.pem` を使用して、Key の `Payload` に対する `Signature` を検証。
        *   これにパスすれば、「この鍵はこのCubeの正当な所有者（秘密鍵保持者）が発行したもの」と証明される。
    3.  **Verify Integrity**:
        *   JSON 内の `export_id` と、`.cube` 内の `export_id.txt` が一致することを確認。
        *   `ExpireAt` が現在時刻より未来であることを確認。
    4.  **Decrypt Data**:
        *   JSON から `aesKey` (Base64 decode) を取得。
        *   `.cube` 内の `encrypted_data.bin` を `aesKey` で復号 -> `zipBuffer` (Plain Cube Data)。
    5.  **Restore**:
        *   `zipBuffer` を展開し、KuzuDB ファイル群を所定のディレクトリに配置。
        *   `cubes` テーブルにレコードを作成。
            *   UUID: 新規生成
            *   Permissions: Key 内の `Permissions` を設定。
            *   ExpireAt: Key 内の `ExpireAt` を設定。
            *   Import元の情報として Lineage を更新（metadata.jsonを使用）。

## 3. 詳細実装＆解説 (Detailed Implementation & Reasoning)

### Step 1: Parse & Validate Key
**【解説】**
ユーザーから渡された「鍵」が正当か検証します。鍵自体にAES復号キーが含まれているため、署名検証に成功すれば復号が可能になります。

**【実装コードスニペット】**
```go
// 1. Separate Key
parts := strings.Split(keyStr, ".")
payloadBytes, _ := base64.StdEncoding.DecodeString(parts[0])
sigBytes, _ := base64.StdEncoding.DecodeString(parts[1])

// 2. Unmarshal Payload
var payload KeyPayload
json.Unmarshal(payloadBytes, &payload)

// 3. Verify Signature with Public Key (from .cube)
blockPub, _ := pem.Decode(publicKeyBytesFromZip)
publicKey, _ := x509.ParsePKCS1PublicKey(blockPub.Bytes)

hash := sha256.Sum256(payloadBytes)
err := rsa.VerifyPSS(publicKey, crypto.SHA256, hash[:], sigBytes, nil)
if err != nil {
    return Error("Invalid key signature")
}

// 4. Integrity Checks
if payload.ExportID != exportIDFromZip {
    return Error("Key mismatch (Export ID)")
}
if payload.ExpireAt != nil && payload.ExpireAt.Before(time.Now()) {
    return Error("Key expired")
}
```

### Step 2: Decrypt & Restore
**【解説】**
鍵に含まれていたAESキーでデータを復号し、システムに取り込みます。

**【実装コードスニペット】**
```go
// 5. Decrypt Cube Data
aesKey, _ := base64.StdEncoding.DecodeString(payload.AESKey)
block, _ := aes.NewCipher(aesKey)
gcm, _ := cipher.NewGCM(block)
nonce := encryptedData[:gcm.NonceSize()]
ciphertext := encryptedData[gcm.NonceSize():]
plainData, err := gcm.Open(nil, nonce, ciphertext, nil)

// 6. Restore to DB (FileSystem)
// Unzip plainData to target directory...
```

### Step 3: Register Cube
**【解説】**
データベースにCube情報を登録します。ここでの権限は「鍵」で指定されたものになります。

**【実装コードスニペット】**
```go
newCube := model.Cube{
    UUID: common.GenUUID(),
    Permissions: payload.Permissions, // From Key
    ExpireAt: payload.ExpireAt,       // From Key
    // ...
}
db.Create(&newCube)

// Import履歴（Action Log）として、BurnedKey なども記録すると良い
```

## 注意事項
*   Import された Cube は新たな UUID を持ちますが、Lineage (metadata.json) は保持または継承されるべきです。
*   `Key` に含まれる `Permissions` がそのまま適用されるため、GenKey での継承ロジックの正確性が重要になります。
