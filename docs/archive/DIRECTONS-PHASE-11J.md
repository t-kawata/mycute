# Phase-11J: GenKey Cube API Implementation

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
`POST /v1/cubes/genkey` エンドポイントを実装。
エクスポートされた `.cube` ファイル（Zip）と、権限情報（Permissions, ExpireAt）を受け取り、それらを操作するための「鍵（Key）」を発行します。
このプロセスでは、Export レコードに保存された秘密鍵を使用して、ファイルを復号するためのAES鍵を取り出し、それを再パッケージして署名を行います。

## 2. 実装要件
*   **Authentication**: USR (Owner of the Export or Cube).
*   **Limit**: `GenKeyLimit` (Source Cube's permissions).
*   **Inputs**:
    *   `.cube` file (multipart/form-data)
    *   `permissions` (JSON)
    *   `expire_at` (Timestamp/Date)
*   **Security Sequence**:
    1.  **Parse .cube**: アップロードされたファイルを解凍し、以下を取得。
        *   `encrypted_aes_key.bin`
        *   `signature.bin`
        *   `public_key.pem`
        *   `export_id.txt`
        *   `encrypted_data.bin` (署名検証用)
    2.  **Retrieve Private Key**: `export_id.txt` から ID を読み取り、DBの `exports` テーブルから対応するレコード（および `PrivateKey`）を取得。
    3.  **Verify Signature**:
        *   `public_key.pem` を使用して `signature.bin` を検証（対象: `encrypted_data.bin` のハッシュ）。
        *   改ざん検知時はエラー。
    4.  **Decrypt AES Key**:
        *   DBから取得した `PrivateKey` を使用して `encrypted_aes_key.bin` を復号 -> `aesKey` (32 bytes) を取得。
    5.  **Create Key Payload**:
        *   以下の情報を JSON 構造体にする。
            ```json
            {
               "aes_key": "base64_encoded_aes_key",
               "permissions": { ... },
               "expire_at": "...",
               "export_id": 123
            }
            ```
    6.  **Sign Key Payload**:
        *   上記 JSON 文字列のハッシュに対し、`PrivateKey` で署名を作成。
    7.  **Format Output Key**:
        *   JSON と 署名 をまとめた構造体（または連結文字列）を作成し、Base64エンコードする。これがユーザーに渡される最終的な「鍵」文字列となる。
        *   Format例: `Base64( JSON + "." + Base64(Signature) )` など、クライアント（Import/ReKey側でパースしやすい形式）。

## 3. 詳細実装＆解説 (Detailed Implementation & Reasoning)

### Step 1: Input Handling & Parsing
**【解説】**
Multipart リクエストからファイルとメタデータを読み取ります。Zipを展開して各コンポーネントをメモリにロードします。

**【実装コードスニペット】**
```go
// Bind Multipart form
file, _ := c.FormFile("file")
// ... Open zip ...
// Read "export_id.txt" -> exportID
// Read "encrypted_aes_key.bin"
// Read "signature.bin"
// Read "public_key.pem"
// Read "encrypted_data.bin" (for verification)
```

### Step 2: DB Lookup & Verification
**【解説】**
Export IDを使って秘密鍵を取得し、ファイルの正当性を検証します。これで「このファイルが確かにこのシステムからエクスポートされたものであり、改ざんされていない」ことを保証します。

**【実装コードスニペット】**
```go
// Fetch Export Record
var exportRecord model.Export
db.First(&exportRecord, exportID)

// Parse Keys
block, _ := pem.Decode([]byte(exportRecord.PrivateKey))
privateKey, _ := x509.ParsePKCS1PrivateKey(block.Bytes)

blockPub, _ := pem.Decode(publicKeyBytesFromZip)
publicKey, _ := x509.ParsePKCS1PublicKey(blockPub.Bytes)

// Verify Signature
hash := sha256.Sum256(encryptedData)
err := rsa.VerifyPSS(publicKey, crypto.SHA256, hash[:], signature, nil)
if err != nil {
    return Error("Tampered file")
}
```

### Step 3: Decrypt AES Key
**【解説】**
秘密鍵を使ってAES鍵を取り出します。このAES鍵はこの後、JSONに埋め込まれて「鍵」の一部として再配布されます。

**【実装コードスニペット】**
```go
// Decrypt AES Key
aesKey, _ := rsa.DecryptOAEP(sha256.New(), rand.Reader, privateKey, encryptedAESKey, nil)
```

### Step 4: Generate Output Key
**【解説】**
AES鍵と権限情報をまとめて署名し、可搬性のある文字列（Key）として出力します。
この鍵があれば、Import時に `.cube` を復号し、かつ指定された権限で利用することができます。

**【実装コードスニペット】**
```go
// Construct Payload
payload := KeyPayload{
    AESKey: base64.StdEncoding.EncodeToString(aesKey),
    Permissions: req.Permissions,
    ExpireAt: req.ExpireAt,
    ExportID: exportID,
}
payloadBytes, _ := json.Marshal(payload)

// Sign Payload
hashPayload := sha256.Sum256(payloadBytes)
sigPayload, _ := rsa.SignPSS(rand.Reader, privateKey, crypto.SHA256, hashPayload[:], nil)

// Final Key String
// Format: Base64(Payload) + "." + Base64(Signature)
keyString := base64.StdEncoding.EncodeToString(payloadBytes) + "." + base64.StdEncoding.EncodeToString(sigPayload)

return keyString
```
※ `KeyPayload` 構造体の定義が必要。

## 注意事項
*   ユーザー入力の `permissions` が、発行元の Cube の権限を超えていないかチェックするロジック（Inheritance Check）は必須です。
*   `GenKeyLimit` の消費も忘れずに行うこと。
