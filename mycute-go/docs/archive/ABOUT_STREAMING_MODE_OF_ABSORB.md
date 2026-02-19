この文書は、AbsorbCubeエンドポイントにSSEストリーミングを追加する改修について、要件と実装方針を専門家に共有するための「要件及び実装説明書」です。

***

## 1. 背景・目的

`AbsorbCube` エンドポイントは、コンテンツを解析し、ベクトルDBおよびグラフDBに吸収するための処理を実行する。  
従来は、処理完了後に最終的なトークン使用量・Limit情報等を JSON で返す同期型APIとして動作していた。

本改修の目的は、**リクエストごとに「ストリーミングモード」と「非ストリーミングモード」を切り替えられるようにし、ストリーミングモード時には Absorb 処理中の中間情報を SSE でクライアントへ逐次送信すること**である。  
同時に、**トークン使用量や DB 更新などのビジネスロジックは、モードに依存せず完全に同一の結果になること**が求められる。

***

## 2. 機能要件

### 2.1 モード切り替え

- `req.Stream` が `false` の場合  
  - 現行実装どおり：
    - Absorb 処理完了後に `AbsorbCubeResData` を JSON で返却。
    - 中間イベントはサーバ側ログ (`utils.LogInfo`) のみ。

- `req.Stream` が `true` の場合  
  - HTTP レスポンスは **SSE (Server-Sent Events)** 形式でストリーミングを行う。
  - Absorb 処理中に発行されるイベントを加工し、OpenAI の ChatCompletion ストリームにできるだけ近いフォーマットでクライアントに送る。
  - 処理完了時には、`AbsorbCubeResData` 相当の情報を**自然言語に変換したテキストとしてストリームで送信**し、最後に終了チャンクを送信してストリームを閉じる。
  - JSON の最終レスポンスは返さない（ストリームが「最終レスポンス」を含む）。

### 2.2 ビジネスロジックの一貫性

- `req.Stream == true` / `false` のいずれの場合も、以下は**必ず共通のフローで実行され、結果が一致しなければならない**：
  - Absorb 処理本体 (`u.CuberService.Absorb`) の実行
  - トークン使用量 (`usage`) の検証
  - `AbsorbLimit` の更新
  - `CubeModelStat` と `CubeContributor` の更新
  - それらを含む DB トランザクションの整合性

- ストリーミングモードであっても、**エラーが発生した場合は DB トランザクション全体をロールバックする**  
  → 「途中までのイベントがクライアントに表示されていても、DB には反映されない」ことが仕様。

### 2.3 エラーハンドリング

- Absorb 処理または DB トランザクション中にエラーが発生した場合：
  - ストリーミングモード:
    - エラーメッセージをテキストとしてストリームに送信（トークン化してチャンク送信）。
    - その後、終了チャンクを送りストリームを閉じる。
    - HTTP レスポンスとしては 5xx 系を返した扱いとなる（Gin のレスポンス制御との兼ね合いは実装側で調整）。
  - 非ストリーミングモード:
    - 現行どおり `InternalServerErrorCustomMsg` などで JSON エラーを返す。

- コンテキストキャンセル (`c.Request.Context().Done()`) が発生した場合：
  - Absorb ゴルーチンに渡すコンテキストをキャンセルし、処理を中断する。
  - ストリーミングモードの場合もストリームをクローズし、DB トランザクションは実行されないかロールバックされる。

***

## 3. SSE ストリーミングの設計

### 3.1 全体フロー

1. `req.Stream` を確認し、`true` の場合はレスポンスヘッダを SSE 用に設定する。
   - `Content-Type: text/event-stream`
   - `Cache-Control: no-cache`
   - `Connection: keep-alive`
   - `X-Accel-Buffering: no` (Nginx 等のバッファ抑止)

2. ストリーミング専用の `StreamWriter` を生成し、クライアントへ出力する専用ゴルーチンを起動する。

3. `u.CuberService.Absorb` を別ゴルーチンで起動し、  
   `dataCh` に `event.StreamEvent` を、`resultCh` に最終結果（`TokenUsage` と `error`）を送らせる。

4. メインゴルーチンで `select` により `dataCh` / `resultCh` / `ctx.Done()` を監視しつつ：
   - `dataCh` からイベントを受信したら `event.FormatEvent` で整形し、その文字列を **トークン分割 → チャンク送信** する（ストリーミングモード時）。
   - 非ストリーミングモード時は従来通りログ出力のみ。
   - `resultCh` を受信したらループを終了し、以降はエラー判定と DB トランザクションへ進む。

5. エラーがなければ DB トランザクションを実行し、その後：
   - ストリーミングモード時は、`AbsorbCubeResData` 相当の情報を自然言語に変換したテキストをトークン化してストリーム送信し、終了チャンクを送ってクローズ。
   - 非ストリーミングモード時は従来通り `OK(c, &data, res)` を返す。

### 3.2 StreamWriter の仕様

`StreamWriter` は「トークン単位の文字列を受け取って SSE のチャンクとして順次送信する」ためのヘルパーである。

- フィールド例：
  - `ch chan string`  
    送信待ちトークンを格納するチャネル（小さめのバッファ付き）。
  - `ctx context.Context`  
    リクエストコンテキスト。キャンセル時に送信ループも終了。
  - `minDelay time.Duration`  
    「次のチャンクを読むまで最低でも何 ms 待つか」の制御用。

- 主なメソッド：
  - `Write(token string)`  
    - `ch <- token` でトークンを送信（`ctx.Done()` による中断にも対応）。
  - `Close()`  
    - `close(ch)` により送信ゴルーチンに終了を通知。

- 送信側ゴルーチンの動作：
  - `for { select { ... } }` で `ch` と `ctx.Done()` を監視。
  - `ch` からトークンを受信したら、OpenAI 互換の JSON チャンクにラップし、  
    `data: {json}\n\n` の形式で `c.Writer` に書き込み `Flush()`。
  - 各トークン送信の後に `time.Ticker` 等で `minDelay` を待機し、連続送信しすぎないようにする。
  - `ch` がクローズされたら終了チャンク（`[DONE]`）を送ってストリームを終了。

### 3.3 OpenAI 互換フォーマット

- 各チャンクは、OpenAI ChatCompletion のストリームに準拠した以下のような JSON を `data:` 行で送信する想定：

```jsonc
{
  "id": "chatcmpl-<uuid>",
  "object": "chat.completion.chunk",
  "created": <unix_timestamp>,
  "model": "cuber-absorb",
  "choices": [
    {
      "index": 0,
      "delta": {
        "content": "<token>"
      },
      "finish_reason": null
    }
  ]
}
```

- 実際には文字列として `data: {...}\n\n` を送る。
- ストリーム終了時には `data: [DONE]\n\n` を送信する。

***

## 4. トークン分割ロジック

### 4.1 tokenize 関数

- シグネチャ：  
  `func tokenize(text string) []string`

- 実装方針：
  - 内部で tiktoken を用い、指定のエンコーディング（例: `"cl100k_base"`）で `text` をトークン化する。
  - `Encode` で得たトークンID列を `Decode` して、**文字列単位のスライス**として返す。
  - もし tiktoken の利用に失敗した場合は、フォールバックとして rune 単位での分割を行う。

- 利用箇所：
  1. 各 `event.StreamEvent` を `event.FormatEvent` した `msg` に対して適用し、  
     そのトークン列を `StreamWriter.Write` で順次送信。
  2. 最終的なサマリー（`AbsorbCubeResData` 相当の自然言語テキスト）に対しても同様に適用。

***

## 5. コンカレンシーとチャネル設計

### 5.1 dataCh / resultCh

- `dataCh chan event.StreamEvent`
  - バッファ無し（アンバッファ）で定義し、Absorb 側とイベントループを同期させる。
  - イベント数に上限を設けない代わりに、メインループで確実に受信・処理する。
- `resultCh chan AbsorbResult`
  - バッファ 1。
  - Absorb 処理完了時に `TokenUsage` と `error` を送信。

### 5.2 イベントループ

- `select` で以下を監視：
  - `case evt := <-dataCh`  
    - `event.FormatEvent` で `msg` を生成。
    - `req.Stream == true` → `tokenize(msg)` → `StreamWriter.Write` で送信。
    - `req.Stream == false` → 従来通りログ出力。
  - `case res := <-resultCh`  
    - `usage` と `err` を受け取り、ループを抜ける。
  - `case <-ctx.Done()`  
    - ストリーミングモードなら `StreamWriter.Close()` を呼び、エラー応答を返す。

### 5.3 コンテキストとキャンセル

- `ctx, cancel := context.WithCancel(c.Request.Context())` を Absorb に渡す。
- HTTP クライアントが接続を切った場合でも、`ctx.Done()` によって Absorb 処理とストリーム送信が終了。

***

## 6. DB トランザクションとロールバックポリシー

- Absorb 完了後、エラーがなければ **従来と同一のトランザクション処理**を実行：
  - `AbsorbLimit` の更新
  - `CubeModelStat` の更新
  - `CubeContributor` の更新
- トランザクション内でエラーが発生した場合：
  - トランザクションはロールバック。
  - ストリーミングモード:
    - DB 更新失敗のメッセージをストリームで送信してから終了チャンクを送る。
  - 非ストリーミングモード:
    - 従来のエラーレスポンス。

- このポリシーにより、「ストリーミングの有無によって DB の状態やトークン使用量集計が変わる」ことはない。

***

## 7. 最終レスポンス仕様（ストリーミングモード）

- ストリーミングモード (`req.Stream == true`) の場合、最終的な JSON レスポンスは返さない。
- 代わりに、以下の内容を自然言語で生成してストリームに流す：
  - 処理成功メッセージ
  - `InputTokens`, `OutputTokens`, `AbsorbLimit` の値
- 例（英語の一例）：
  - `"Absorb completed successfully. Input tokens: 1234, Output tokens: 567. Remaining absorb limit: 10."`
- このテキストも `tokenize` によってトークン分割され、OpenAI 互換チャンクとして送信される。

***

## 8. 構造体とヘルパー関数
```go
// Tokenize splits text into tokens using tiktoken (cl100k_base).
// Returns a slice of token strings.
// グローバル変数としてエンコーダーを保持
var tiktokenEncoding *tiktoken.Tiktoken

func init() {
	// 初期化時にロード (失敗時はログ出力等を行うか、使用時にチェック)
	enc, err := tiktoken.GetEncoding("cl100k_base")
	if err == nil {
		tiktokenEncoding = enc
	}
}

// SSE用のストリームライター
type StreamWriter struct {
    ch       chan string
    ctx      context.Context
    minDelay time.Duration
}

func NewStreamWriter(ctx context.Context, minDelay time.Duration) *StreamWriter {
    return &StreamWriter{
        ch:       make(chan string, 10), // 小さなバッファ
        ctx:      ctx,
        minDelay: minDelay,
    }
}

func (sw *StreamWriter) Write(token string) {
    select {
    case sw.ch <- token:
    case <-sw.ctx.Done():
        return
    }
}

func (sw *StreamWriter) Close() {
    close(sw.ch)
}

// トークン分割関数
func tokenize(text string) []string {
    // tiktoken を使った実装
    encoding, err := tiktoken.GetEncoding("cl100k_base") // GPT-4/3.5用
    if err != nil {
        // フォールバック: 文字単位分割
        runes := []rune(text)
        tokens := make([]string, len(runes))
        for i, r := range runes {
            tokens[i] = string(r)
        }
        return tokens
    }
    
    tokenIDs := encoding.Encode(text, nil, nil)
    tokens := make([]string, len(tokenIDs))
    for i, id := range tokenIDs {
        tokens[i] = encoding.Decode([]uint{id})
    }
    return tokens
}

// OpenAI互換のSSEチャンク生成
func createSSEChunk(content string, finish bool) string {
    if finish {
        return "data: [DONE]\n\n"
    }
    
    // OpenAI ChatCompletion chunk format
    chunk := map[string]interface{}{
        "id":      fmt.Sprintf("chatcmpl-%s", uuid.New().String()),
        "object":  "chat.completion.chunk",
        "created": time.Now().Unix(),
        "model":   "cuber-absorb",
        "choices": []map[string]interface{}{
            {
                "index": 0,
                "delta": map[string]string{
                    "content": content,
                },
                "finish_reason": nil,
            },
        },
    }
    
    jsonBytes, _ := json.Marshal(chunk)
    return fmt.Sprintf("data: %s\n\n", string(jsonBytes))
}
```

## 9. メインの関数の改修
```go
func AbsorbCube(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, req *rtreq.AbsorbCubeReq, res *rtres.AbsorbCubeRes) bool {
    ids := ju.IDs(!(ju.IsApx() || ju.IsFromKey()))
    
    // 1. Cubeの取得と権限チェック（既存コード）
    cube, err := getCube(u, req.CubeID, *ids.ApxID, *ids.VdrID)
    if err != nil {
        return NotFoundCustomMsg(c, res, "Cube not found.")
    }
    
    perm, err := common.ParseDatatypesJson[model.CubePermissions](&cube.Permissions)
    if err != nil {
        return InternalServerErrorCustomMsg(c, res, "Failed to parse permissions.")
    }
    
    // 2. Limit チェック（既存コード）
    if perm.AbsorbLimit < 0 {
        return BadRequestCustomMsg(c, res, "Absorb limit exceeded.")
    }
    
    nextLimit := perm.AbsorbLimit
    shouldUpdateLimit := false
    if perm.AbsorbLimit > 0 {
        nextLimit = perm.AbsorbLimit - 1
        if nextLimit == 0 {
            nextLimit = -1
        }
        shouldUpdateLimit = true
    }
    
    // 3. 一時ファイル作成（既存コード）
    tempFile := filepath.Join(os.TempDir(), fmt.Sprintf("%s.txt", *common.GenUUID()))
    if err := os.WriteFile(tempFile, []byte(req.Content), 0644); err != nil {
        return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Failed to write temp file: %s", err.Error()))
    }
    defer os.Remove(tempFile)
    
    // 4. 準備（既存コード）
    if u.CuberService == nil {
        return InternalServerErrorCustomMsg(c, res, "CuberService is not available.")
    }
    
    cubeDbFilePath, err := u.GetCubeDBFilePath(&cube.UUID, ids.ApxID, ids.VdrID, ids.UsrID)
    if err != nil {
        return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Failed to get cube path: %s", err.Error()))
    }
    
    decryptedEmbeddingApiKey, err := mycrypto.Decrypt(cube.EmbeddingApiKey, u.CuberCryptoSkey)
    if err != nil {
        return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Failed to decrypt embedding API key: %s", err.Error()))
    }
    
    chatConf, err := fetchChatModelConfig(u, req.ChatModelID, *ids.ApxID, *ids.VdrID)
    if err != nil {
        return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Failed to fetch chat model: %s", err.Error()))
    }
    
    // 5. ストリーミング設定
    var streamWriter *StreamWriter
    if req.Stream {
        c.Header("Content-Type", "text/event-stream")
        c.Header("Cache-Control", "no-cache")
        c.Header("Connection", "keep-alive")
        c.Header("X-Accel-Buffering", "no") // Nginx対策
        
        streamWriter = NewStreamWriter(c.Request.Context(), 50*time.Millisecond) // 最低50ms間隔
        
        // ストリーム送信ゴルーチン
        go func() {
            ticker := time.NewTicker(streamWriter.minDelay)
            defer ticker.Stop()
            
            for {
                select {
                case token, ok := <-streamWriter.ch:
                    if !ok {
                        // チャンネルクローズ = 終了
                        c.SSEvent("", "[DONE]")
                        c.Writer.Flush()
                        return
                    }
                    
                    // OpenAI形式のチャンク送信
                    chunk := createSSEChunk(token, false)
                    fmt.Fprint(c.Writer, chunk)
                    c.Writer.Flush()
                    
                    // 最低遅延を保証
                    <-ticker.C
                    
                case <-c.Request.Context().Done():
                    return
                }
            }
        }()
    }
    
    // 6. Absorb実行
    type AbsorbResult struct {
        Usage types.TokenUsage
        Err   error
    }
    
    dataCh := make(chan event.StreamEvent) // アンバッファに変更
    resultCh := make(chan AbsorbResult, 1)
    isEn := false
    
    ctx, cancel := context.WithCancel(c.Request.Context())
    defer cancel()
    
    go func() {
        u, e := u.CuberService.Absorb(ctx, u.EventBus, cubeDbFilePath, req.MemoryGroup, []string{tempFile},
            types.CognifyConfig{
                ChunkSize:    req.ChunkSize,
                ChunkOverlap: req.ChunkOverlap,
            },
            types.EmbeddingModelConfig{
                Provider:  cube.EmbeddingProvider,
                Model:     cube.EmbeddingModel,
                Dimension: cube.EmbeddingDimension,
                BaseURL:   cube.EmbeddingBaseURL,
                ApiKey:    decryptedEmbeddingApiKey,
            },
            chatConf,
            dataCh,
            isEn,
        )
        resultCh <- AbsorbResult{Usage: u, Err: e}
    }()
    
    var usage types.TokenUsage
AbsorbLoop:
    for {
        select {
        case evt := <-dataCh:
            msg, err := event.FormatEvent(evt, isEn)
            if err == nil {
                if req.Stream {
                    // トークン化してストリームに流す
                    tokens := tokenize(msg)
                    for _, token := range tokens {
                        streamWriter.Write(token)
                    }
                } else {
                    // 既存のログ出力
                    utils.LogInfo(u.Logger, "=================================")
                    utils.LogInfo(u.Logger, fmt.Sprintf("%s: %s", evt.EventName, msg))
                    utils.LogInfo(u.Logger, "=================================")
                }
            }
            
        case res := <-resultCh:
            usage = res.Usage
            err = res.Err
            break AbsorbLoop
            
        case <-ctx.Done():
            if req.Stream && streamWriter != nil {
                streamWriter.Close()
            }
            return InternalServerErrorCustomMsg(c, res, "Request cancelled")
        }
    }
    
    // 7. エラーチェック（ストリーミング含む）
    if err != nil {
        if req.Stream && streamWriter != nil {
            // エラーメッセージをストリームで送信
            errorMsg := fmt.Sprintf("Error: Absorb failed - %s", err.Error())
            tokens := tokenize(errorMsg)
            for _, token := range tokens {
                streamWriter.Write(token)
            }
            streamWriter.Close()
        }
        // ストリームモードでもエラーはロールバック（DB更新しない）
        return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Absorb failed: %s", err.Error()))
    }
    
    if usage.InputTokens < 0 || usage.OutputTokens < 0 {
        if req.Stream && streamWriter != nil {
            streamWriter.Close()
        }
        return InternalServerErrorCustomMsg(c, res, "Invalid token usage reported.")
    }
    
    // 8. DBトランザクション（既存コード - ストリームモードでも同じ）
    err = u.DB.Transaction(func(tx *gorm.DB) error {
        if shouldUpdateLimit {
            perm.AbsorbLimit = nextLimit
            newJSONStr, err := common.ToJson(perm)
            if err != nil {
                return err
            }
            cube.Permissions = datatypes.JSON(newJSONStr)
            if err := tx.Save(cube).Error; err != nil {
                return err
            }
        }
        
        for modelName, detail := range usage.Details {
            var ms model.CubeModelStat
            if err := tx.Where("cube_id = ? AND memory_group = ? AND model_name = ? AND action_type = ? AND apx_id = ? AND vdr_id = ?",
                cube.ID, req.MemoryGroup, modelName, types.ACTION_TYPE_ABSORB, *ids.ApxID, *ids.VdrID).
                FirstOrCreate(&ms, model.CubeModelStat{
                    CubeID: cube.ID, MemoryGroup: req.MemoryGroup, ModelName: modelName, ActionType: string(types.ACTION_TYPE_ABSORB),
                    ApxID: *ids.ApxID, VdrID: *ids.VdrID,
                }).Error; err != nil {
                return err
            }
            ms.InputTokens += detail.InputTokens
            ms.OutputTokens += detail.OutputTokens
            if err := tx.Save(&ms).Error; err != nil {
                return err
            }
            
            contributorName, err := getJwtUsrName(u, ids.ApxID, ids.VdrID, ids.UsrID)
            if err != nil {
                return fmt.Errorf("Failed to get contributor name: %s", err.Error())
            }
            
            var cc model.CubeContributor
            if err := tx.Where("cube_id = ? AND memory_group = ? AND contributor_name = ? AND model_name = ? AND apx_id = ? AND vdr_id = ?",
                cube.ID, req.MemoryGroup, contributorName, modelName, *ids.ApxID, *ids.VdrID).
                FirstOrCreate(&cc, model.CubeContributor{
                    CubeID: cube.ID, MemoryGroup: req.MemoryGroup, ContributorName: contributorName, ModelName: modelName,
                    ApxID: *ids.ApxID, VdrID: *ids.VdrID,
                }).Error; err != nil {
                return err
            }
            cc.InputTokens += detail.InputTokens
            cc.OutputTokens += detail.OutputTokens
            if err := tx.Save(&cc).Error; err != nil {
                return err
            }
        }
        return nil
    })
    
    if err != nil {
        if req.Stream && streamWriter != nil {
            streamWriter.Close()
        }
        return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("DB update failed: %s", err.Error()))
    }
    
    // 9. レスポンス作成
    data := rtres.AbsorbCubeResData{
        InputTokens:  usage.InputTokens,
        OutputTokens: usage.OutputTokens,
        AbsorbLimit:  perm.AbsorbLimit,
    }
    if shouldUpdateLimit {
        data.AbsorbLimit = nextLimit
    }
    
    if req.Stream {
        // 最終結果を自然言語で送信
        summary := fmt.Sprintf("\n\nAbsorb completed successfully. Input tokens: %d, Output tokens: %d. Remaining absorb limit: %d.",
            data.InputTokens, data.OutputTokens, data.AbsorbLimit)
        
        tokens := tokenize(summary)
        for _, token := range tokens {
            streamWriter.Write(token)
        }
        
        streamWriter.Close()
        return true // ストリームは既に送信完了
    }
    
    return OK(c, &data, res)
}
```

## 8. 想定されるレビュー観点

- SSE 実装と Gin のレスポンスライフサイクルの整合性：
  - 既にヘッダ送信後に HTTP ステータスコードを変更できない点の扱い。
- コンテキストキャンセル時のリソースリークが起きないか：
  - Absorb 側ゴルーチン、送信ゴルーチン、チャネルクローズの整合性。
- tiktoken 利用時のパフォーマンスとエラー耐性：
  - エンコーディング指定、キャッシュ戦略、フォールバック挙動。
- OpenAI 互換 JSON の構造：
  - 実際のクライアント（フロントエンド / SDK）が期待するフィールドとの整合性。
