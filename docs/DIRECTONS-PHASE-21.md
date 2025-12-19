# Phase 21: SSE Streaming Mode Implementation

## 1. 概要 (Overview)

本フェーズでは、`Absorb`, `Query`, `Memify` の各エンドポイントにおいて、リクエストパラメータ `req.Stream` が `true` の場合に **SSE (Server-Sent Events) プロトコルによるストリーミング応答** を返す機能を実装します。

これは `docs/ABOUT_STREAMING_MODE_OF_ABSORB.md` で定義された Absorb のストリーミングモード仕様を、他の長時間実行タスク（Query, Memify）にも適用・拡張するものです。

### 目的
- 長時間実行される処理の進捗状況（イベント）をリアルタイムにクライアントへフィードバックする。
- OpenAI ChatCompletion API 互換（またはそれに準ずる）形式でレスポンスし、既存の AI クライアントライブラリ等との親和性を高める。
- ビジネスロジック（DB更新、トークン集計）はストリーミング有無に関わらず厳密に一致させる。

---

## 2. 共通基盤の実装 (Shared Infrastructure)

各ハンドラで共通して利用するストリーミング用ヘルパーを `src/mode/rt/rtstream` パッケージに実装します。

### 2.1 パッケージ作成と依存追加

1. `src/mode/rt/rtstream` ディレクトリを作成します。
2. `src` ディレクトリで以下のコマンドを実行し、tiktoken ライブラリを追加します。
   ```bash
   go get github.com/pkoukk/tiktoken-go
   ```

### 2.2 `Tokenize` ヘルパーの実装

**ファイル:** `src/mode/rt/rtstream/tokenizer.go`

`docs/ABOUT_STREAMING_MODE_OF_ABSORB.md` の「8. 構造体とヘルパー関数」に基づき実装します。

```go
package rtstream

import (
	"strings"

	"github.com/pkoukk/tiktoken-go"
)

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

// トークン分割関数
func Tokenize(text string) []string {
	// tiktoken を使った実装
	if tiktokenEncoding == nil {
		// リトライまたはフォールバック
		var err error
		tiktokenEncoding, err = tiktoken.GetEncoding("cl100k_base")
		if err != nil {
			// フォールバック: 文字単位分割
			runes := []rune(text)
			tokens := make([]string, len(runes))
			for i, r := range runes {
				tokens[i] = string(r)
			}
			return tokens
		}
	}
	
	tokenIDs := tiktokenEncoding.Encode(text, nil, nil)
	tokens := make([]string, len(tokenIDs))
	for i, id := range tokenIDs {
		tokens[i] = tiktokenEncoding.Decode([]int{id})
	}
	return tokens
}
```

### 2.3 `StreamWriter` の実装

**ファイル:** `src/mode/rt/rtstream/stream_writer.go`

`docs/ABOUT_STREAMING_MODE_OF_ABSORB.md` の仕様に準拠した `StreamWriter` を実装します。

```go
package rtstream

import (
	"context"
	"encoding/json"
	"fmt"
	"time"

	"github.com/google/uuid"
)

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

// Ch returns the channel for consuming tokens
func (sw *StreamWriter) Ch() <-chan string {
	return sw.ch
}

// MinDelay returns the configured minimum delay
func (sw *StreamWriter) MinDelay() time.Duration {
	return sw.minDelay
}

// OpenAI互換のSSEチャンク生成
func CreateSSEChunk(modelName string, content string, finish bool) string {
	if finish {
		return "data: [DONE]\n\n"
	}
	
	// OpenAI ChatCompletion chunk format
	chunk := map[string]interface{}{
		"id":      fmt.Sprintf("chatcmpl-%s", uuid.New().String()),
		"object":  "chat.completion.chunk",
		"created": time.Now().Unix(),
		"model":   modelName,
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

---

## 3. 各エンドポイントの実装指示

各エンドポイント (`AbsorbCube`, `QueryCube`, `MemifyCube`) を、以下の実装例に従って改修してください。

### 3.1 AbsorbCube の改修

**対象ファイル**: `src/mode/rt/rtbl/cubes_bl.go`

`docs/ABOUT_STREAMING_MODE_OF_ABSORB.md` の「9. メインの関数の改修」に記載されたコードで `AbsorbCube` 関数を完全に置換・実装してください。
※ 外部パッケージ呼び出し（`rtstream` 等）へのパスは適切に調整してください。

```go
func AbsorbCube(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, req *rtreq.AbsorbCubeReq, res *rtres.AbsorbCubeRes) bool {
    // ... (ABOUT_STREAMING_MODE_OF_ABSORB.md の実装をコピーし、パッケージ名を rtstream に適合させる)
    // streamWriter := rtstream.NewStreamWriter(...)
    // rtstream.CreateSSEChunk(...)
    // rtstream.Tokenize(...)
}
```
※ 具体的なコードは `docs/ABOUT_STREAMING_MODE_OF_ABSORB.md` を参照のこと。

### 3.2 QueryCube の改修

**対象ファイル**: `src/mode/rt/rtbl/cubes_bl.go`

`AbsorbCube` と同様の構造で実装します。

```go
func QueryCube(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, req *rtreq.QueryCubeReq, res *rtres.QueryCubeRes) bool {
    // ... [前処理: 権限チェック、Limitチェック] ...

    // ストリーミング設定
    var streamWriter *rtstream.StreamWriter
    if req.Stream {
        c.Header("Content-Type", "text/event-stream")
        c.Header("Cache-Control", "no-cache")
        c.Header("Connection", "keep-alive")
        c.Header("X-Accel-Buffering", "no")
        
        streamWriter = rtstream.NewStreamWriter(c.Request.Context(), 50*time.Millisecond)
        
        // ストリーム送信ゴルーチン
        go func() {
            ticker := time.NewTicker(streamWriter.MinDelay())
            defer ticker.Stop()
            for {
                select {
                case token, ok := <-streamWriter.Ch():
                    if !ok {
                        c.SSEvent("", "[DONE]")
                        c.Writer.Flush()
                        return
                    }
                    chunk := rtstream.CreateSSEChunk("cuber-query", token, false)
                    fmt.Fprint(c.Writer, chunk)
                    c.Writer.Flush()
                    <-ticker.C
                case <-c.Request.Context().Done():
                    return
                }
            }
        }()
    }

    // Query実行
    type QueryResult struct {
        Usage types.TokenUsage
        Err   error
    }
    dataCh := make(chan event.StreamEvent)
    resultCh := make(chan QueryResult, 1)
    
    ctx, cancel := context.WithCancel(c.Request.Context())
    defer cancel()

    // イベント購読
    event.RegisterQueryStreamer(u.EventBus, dataCh)

    go func() {
         // u.CuberService.Query(...) 呼び出し
         // resultCh <- QueryResult{...}
    }()

    // イベントループ
QueryLoop:
    for {
        select {
        case evt := <-dataCh:
            // QueryGenerationEnd から回答を取得してストリーミング
            if evt.EventName == event.EVENT_QUERY_GENERATION_END {
                if p, ok := evt.Payload.(event.QueryGenerationEndPayload); ok {
                     if req.Stream {
                        tokens := rtstream.Tokenize(p.Response)
                        for _, t := range tokens {
                            streamWriter.Write(t)
                        }
                     }
                }
            } else {
                 if msg, err := event.FormatEvent(evt, false); err == nil {
                    if !req.Stream {
                        utils.LogInfo(u.Logger, fmt.Sprintf("%s: %s", evt.EventName, msg))
                    }
                 }
            }
        case res := <-resultCh:
             // ...
             break QueryLoop
        case <-ctx.Done():
             // ...
        }
    }
    
    // ... [エラー処理、DBトランザクション、最終レスポンス] ...
    // Streamモード完了時は回答全文や要約をトークン化して流す
}
```

### 3.3 MemifyCube の改修

**対象ファイル**: `src/mode/rt/rtbl/cubes_bl.go`

`AbsorbCube`, `QueryCube` と同様のパターンの適用に加え、`RegisterMemifyStreamer` を使用します。
Memify のイベント (`EVENT_MEMIFY_START` 等) を `StreamWriter` でユーザに進捗としてフィードバックする実装を行ってください。



---

## 4. 検証計画 (Verification Plan)

### 4.1 動作確認
- `curl` コマンドを使用し、`"stream": true` のリクエストを送信して SSE ストリームが流れてくるか確認する。
    - `vdr_id`, `apx_id` 等のヘッダが必要な点に注意。
    - `Absorb`, `Query`, `Memify` 全てで確認。
- DB の状態（TokenUsage, Limit, Stats）が、ストリームモードと通常モードで同一の結果になることを確認する。
- 途中キャンセル時の挙動（DBロールバック）を確認する。

### 4.2 ビルド確認
- `make build` (macOS) および `make build-linux-amd64` (Linux) が通ることを確認。

---

## 5. 補足

- ストリーム出力時のフォーマットは、フロントエンドが扱いやすいように一貫性を持たせること。
- 既存の `AbsorbCube` 実装内にある `dataCh` 関連のコードは今回のアーキテクチャに合わせてリファクタリングする。
