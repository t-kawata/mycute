# Phase-11C: Export Cube API Implementation

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
`GET /v1/cubes/export` エンドポイントを実装。
指定された Cube を暗号化し、`.cube` ファイル（Zip形式）としてエクスポートします。
セキュリティ強化のため、ファイル自体はAES共通鍵で暗号化され、その共通鍵はRSA公開鍵で暗号化されて同梱されます。
また、改ざん防止のために署名も付与されます。

## 2. 実装要件
*   **Limit**: `ExportLimit` (0=Unlim, >0=Decr, <0=Forbid).
*   **Security Sequence**:
    1.  **RSA Key Pair**: エクスポートごとに新しいRSAキーペア（2048bit以上）を生成。
    2.  **Export Record**: 秘密鍵（PrivateKey）は `exports` テーブルに保存（PEM形式推奨）。公開鍵（PublicKey）はファイルに同梱。
    3.  **AES Key**: 32バイト（256bit）のランダムなAES共通鍵を生成。
    4.  **Encryption**:
        *   CubeのDBファイル（Zip圧縮前の生データ or Zip圧縮後のデータ？） -> **Zip圧縮後のデータ** (`zipBuffer`) をAES-GCM等で暗号化。
        *   AES共通鍵をRSA公開鍵で暗号化 (OAEP)。
    5.  **Signature**:
        *   暗号化されたデータ（Encrypted File）のハッシュ値に対し、RSA秘密鍵で署名を作成 (PSS/PKCS1v15)。
    6.  **Package**: 以下のファイルをまとめた新しい Zip ファイルを作成（拡張子 `.cube`）。
        *   `encrypted_data.bin`: 暗号化されたCubeデータ
        *   `signature.bin`: 署名データ
        *   `public_key.pem`: RSA公開鍵
        *   `encrypted_aes_key.bin`: 暗号化されたAES共通鍵
        *   `export_id.txt`: `exports` テーブルのレコード ID (文字列形式)

## 3. 詳細実装＆解説 (Detailed Implementation & Reasoning)

### Step 1: RSAキーペア生成とレコード作成
**【解説】**
エクスポートのたびに独自の鍵ペアを作ることで、万が一鍵が漏洩しても他のエクスポートデータへの影響を最小限にします。
秘密鍵はDBに保存し、GenKeyフェーズでのみ使用します。

**【実装コードスニペット】**
```go
// 1. RSA Key Pair Generation
privateKey, err := rsa.GenerateKey(rand.Reader, 2048)
// Public Key extraction
publicKey := &privateKey.PublicKey

// 2. Save Private Key to DB (PEM Encode)
privBytes := x509.MarshalPKCS1PrivateKey(privateKey)
privPEM := pem.EncodeToMemory(&pem.Block{Type: "RSA PRIVATE KEY", Bytes: privBytes})

// Create Export Record
exportRecord := model.Export{
    CubeID: cube.ID,
    // ... stored private key ...
    PrivateKey: string(privPEM), 
    // ...
}
db.Create(&exportRecord)
```
※ `model.Export` に `PrivateKey` カラムを追加する必要があります（TEXT型）。

### Step 2: AES鍵生成とデータ暗号化
**【解説】**
巨大なファイルをRSAで直接暗号化するのは効率が悪いため、ハイブリッド暗号方式を採用します。
データ本体は高速なAESで暗号化し、そのAES鍵をRSAで暗号化します。

**【実装コードスニペット】**
```go
// 3. Generate AES Key (32 bytes)
aesKey := make([]byte, 32)
rand.Read(aesKey)

// 4. Encrypt zipBuffer (Cube Data) with AES-GCM
block, _ := aes.NewCipher(aesKey)
gcm, _ := cipher.NewGCM(block)
nonce := make([]byte, gcm.NonceSize())
rand.Read(nonce)
encryptedData := gcm.Seal(nonce, nonce, zipBuffer.Bytes(), nil)

// Encrypt AES Key with RSA Public Key
encryptedAESKey, _ := rsa.EncryptOAEP(sha256.New(), rand.Reader, publicKey, aesKey, nil)
```

### Step 3: 署名作成 (Signature)
**【解説】**
データの改ざんを検知するため、暗号化データのハッシュに対して署名を行います。

**【実装コードスニペット】**
```go
// 5. Sign (SHA256 hash of encryptedData)
hash := sha256.Sum256(encryptedData)
signature, _ := rsa.SignPSS(rand.Reader, privateKey, crypto.SHA256, hash[:], nil)
```

### Step 4: パッケージング
**【解説】**
必要なコンポーネントをZipにまとめます。これが最終的にユーザーにダウンロードされる `.cube` ファイルです。

**【実装コードスニペット】**
```go
// 6. Package
finalZip := new(bytes.Buffer)
w := zip.NewWriter(finalZip)

// Helper to write file to zip
addToZip(w, "encrypted_data.bin", encryptedData)
addToZip(w, "signature.bin", signature)
addToZip(w, "public_key.pem", pem.EncodeToMemory(&pem.Block{Type: "RSA PUBLIC KEY", Bytes: x509.MarshalPKCS1PublicKey(publicKey)}))
addToZip(w, "encrypted_aes_key.bin", encryptedAESKey)
addToZip(w, "export_id.txt", []byte(fmt.Sprintf("%d", exportRecord.ID)))

w.Close()

// Return finalZip to user
```

## 注意事項
*   既存の `uuid` ベースの `Hash` カラムの扱いはどうするか？ -> `encrypted_data.bin` のハッシュなどを入れておくと良い。
*   `Export` モデルへの `PrivateKey` カラムの追加マイグレーションが必要。
