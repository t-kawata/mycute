package rtstream

import (
	"context"
	"encoding/json"
	"fmt"
	"time"
)

// StreamWriter はSSEストリーミング用のライター。
// トークンをチャネル経由で受け取り、OpenAI互換のSSE形式で送信する。
type StreamWriter struct {
	ch       chan string
	done     chan struct{} // SSE送信ゴルーチンの完了を通知するチャネル
	ctx      context.Context
	minDelay time.Duration
}

// NewStreamWriter は新しい StreamWriter を作成する。
// ctx: リクエストコンテキスト（キャンセル検知用）
// minDelay: 各トークン送信間の最小遅延
func NewStreamWriter(ctx context.Context, minDelay time.Duration) *StreamWriter {
	return &StreamWriter{
		ch:       make(chan string, 1000),
		done:     make(chan struct{}),
		ctx:      ctx,
		minDelay: minDelay,
	}
}

// Write はトークンをチャネルに送信する。
// 順序を保証するため、コンテキストキャンセル時を除きブロックして送信する。
func (sw *StreamWriter) Write(token string) {
	select {
	case sw.ch <- token:
	case <-sw.ctx.Done():
		return
	}
}

// Close はストリームを終了する。
// チャネルをクローズすることでSSE送信ゴルーチンに終了を通知する。
func (sw *StreamWriter) Close() {
	close(sw.ch)
}

// Wait はSSE送信ゴルーチンが完了するまでブロックする。
// ハンドラから return する前に必ず呼び出すこと。
func (sw *StreamWriter) Wait() {
	<-sw.done
}

// Done はSSE送信ゴルーチンの完了を通知する。
// SSE送信ゴルーチン内で、終了時に必ず呼び出すこと。
func (sw *StreamWriter) Done() {
	close(sw.done)
}

// Ch はトークンを消費するためのチャネルを返す。
func (sw *StreamWriter) Ch() <-chan string {
	return sw.ch
}

// MinDelay は設定された最小遅延を返す。
func (sw *StreamWriter) MinDelay() time.Duration {
	return sw.minDelay
}

// CreateSSEChunk はOpenAI互換のSSEチャンクを生成する。
// requestId: リクエスト単位で共通のID（呼び出し元で生成すること）
// modelName: モデル名（例: "cuber-absorb"）
// content: 送信するトークン内容
// finish: trueの場合は終了シグナル [DONE] を返す
func CreateSSEChunk(requestId string, modelName string, content string, finish bool) string {
	if finish {
		return "data: [DONE]\n\n"
	}
	// OpenAI ChatCompletion chunk format
	chunk := map[string]any{
		"id":      fmt.Sprintf("chatcmpl-%s", requestId),
		"object":  "chat.completion.chunk",
		"created": time.Now().Unix(),
		"model":   modelName,
		"choices": []map[string]any{
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
