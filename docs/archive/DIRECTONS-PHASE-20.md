# Phase 20: Event Streaming Implementation

本フェーズでは、`CuberService` のコアメソッド (`Absorb`, `Query`, `Memify`) を非同期イベントストリーミングに対応させます。`gin.Context` への依存を排除し、Go Channel を通じたイベント伝播メカニズムを確立します。
最終的なSSEレスポンスの実装は次フェーズで行いますが、本フェーズでは `rtbl` レイヤでチャンネル経由のイベント受信とペイロード変換が正常に動作することを検証できる状態までを実装します。

## 1. Streaming Type Definition

ストリーミングイベントの統一型と、ペイロード変換ユーティリティを定義します。

### 1.1. `src/pkg/cuber/event/stream.go` (新規作成)

```go
package event

import (
	"fmt"
)

// StreamEvent represents a unified event structure for streaming
type StreamEvent struct {
	EventName EventName
	Payload   any
}

// ConvertEventPayload ensures the Payload is converted to the correct struct type based on EventName.
// Since the Payload in StreamEvent is 'any', this function acts as a type assertion/validation layer.
// In a real scenario (e.g., unmarshalling from JSON), this would handle parsing.
// Here validity is checked by type switch.
func ConvertEventPayload(e StreamEvent) (any, error) {
	switch e.EventName {
	// --- Absorb Events ---
	case EVENT_ABSORB_START:
		if p, ok := e.Payload.(AbsorbStartPayload); ok { return p, nil }
	case EVENT_ABSORB_ADD_FILE_START:
		if p, ok := e.Payload.(AbsorbAddFileStartPayload); ok { return p, nil }
	case EVENT_ABSORB_ADD_FILE_END:
		if p, ok := e.Payload.(AbsorbAddFileEndPayload); ok { return p, nil }
	case EVENT_ABSORB_CHUNKING_READ_START:
		if p, ok := e.Payload.(AbsorbChunkingReadStartPayload); ok { return p, nil }
	case EVENT_ABSORB_CHUNKING_READ_END:
		if p, ok := e.Payload.(AbsorbChunkingReadEndPayload); ok { return p, nil }
	case EVENT_ABSORB_CHUNKING_SAVE_START:
		if p, ok := e.Payload.(AbsorbChunkingSaveStartPayload); ok { return p, nil }
	case EVENT_ABSORB_CHUNKING_SAVE_END:
		if p, ok := e.Payload.(AbsorbChunkingSaveEndPayload); ok { return p, nil }
	case EVENT_ABSORB_CHUNKING_PROCESS_START:
		if p, ok := e.Payload.(AbsorbChunkingProcessStartPayload); ok { return p, nil }
	case EVENT_ABSORB_CHUNKING_PROCESS_END:
		if p, ok := e.Payload.(AbsorbChunkingProcessEndPayload); ok { return p, nil }
	case EVENT_ABSORB_GRAPH_REQUEST_START:
		if p, ok := e.Payload.(AbsorbGraphRequestStartPayload); ok { return p, nil }
	case EVENT_ABSORB_GRAPH_REQUEST_END:
		if p, ok := e.Payload.(AbsorbGraphRequestEndPayload); ok { return p, nil }
	case EVENT_ABSORB_GRAPH_PARSE_START:
		if p, ok := e.Payload.(AbsorbGraphParseStartPayload); ok { return p, nil }
	case EVENT_ABSORB_GRAPH_PARSE_END:
		if p, ok := e.Payload.(AbsorbGraphParseEndPayload); ok { return p, nil }
	case EVENT_ABSORB_STORAGE_CHUNK_START:
		if p, ok := e.Payload.(AbsorbStorageChunkStartPayload); ok { return p, nil }
	case EVENT_ABSORB_STORAGE_CHUNK_END:
		if p, ok := e.Payload.(AbsorbStorageChunkEndPayload); ok { return p, nil }
	case EVENT_ABSORB_STORAGE_NODE_START:
		if p, ok := e.Payload.(AbsorbStorageNodeStartPayload); ok { return p, nil }
	case EVENT_ABSORB_STORAGE_NODE_END:
		if p, ok := e.Payload.(AbsorbStorageNodeEndPayload); ok { return p, nil }
	case EVENT_ABSORB_STORAGE_EDGE_START:
		if p, ok := e.Payload.(AbsorbStorageEdgeStartPayload); ok { return p, nil }
	case EVENT_ABSORB_STORAGE_EDGE_END:
		if p, ok := e.Payload.(AbsorbStorageEdgeEndPayload); ok { return p, nil }
	case EVENT_ABSORB_SUMMARIZATION_START:
		if p, ok := e.Payload.(AbsorbSummarizationStartPayload); ok { return p, nil }
	case EVENT_ABSORB_SUMMARIZATION_CHUNK_START:
		if p, ok := e.Payload.(AbsorbSummarizationChunkStartPayload); ok { return p, nil }
	case EVENT_ABSORB_SUMMARIZATION_REQ_START:
		if p, ok := e.Payload.(AbsorbSummarizationReqStartPayload); ok { return p, nil }
	case EVENT_ABSORB_SUMMARIZATION_REQ_END:
		if p, ok := e.Payload.(AbsorbSummarizationReqEndPayload); ok { return p, nil }
	case EVENT_ABSORB_SUMMARIZATION_SAVE_START:
		if p, ok := e.Payload.(AbsorbSummarizationSaveStartPayload); ok { return p, nil }
	case EVENT_ABSORB_SUMMARIZATION_SAVE_END:
		if p, ok := e.Payload.(AbsorbSummarizationSaveEndPayload); ok { return p, nil }
	case EVENT_ABSORB_SUMMARIZATION_CHUNK_END:
		if p, ok := e.Payload.(AbsorbSummarizationChunkEndPayload); ok { return p, nil }
	case EVENT_ABSORB_SUMMARIZATION_END:
		if p, ok := e.Payload.(AbsorbSummarizationEndPayload); ok { return p, nil }
	case EVENT_ABSORB_END:
		if p, ok := e.Payload.(AbsorbEndPayload); ok { return p, nil }
	case EVENT_ABSORB_ERROR:
		if p, ok := e.Payload.(AbsorbErrorPayload); ok { return p, nil }

	// --- Query Events ---
	case EVENT_QUERY_START:
		if p, ok := e.Payload.(QueryStartPayload); ok { return p, nil }
	case EVENT_QUERY_EMBEDDING_START:
		if p, ok := e.Payload.(QueryEmbeddingStartPayload); ok { return p, nil }
	case EVENT_QUERY_EMBEDDING_END:
		if p, ok := e.Payload.(QueryEmbeddingEndPayload); ok { return p, nil }
	case EVENT_QUERY_SEARCH_VECTOR_START:
		if p, ok := e.Payload.(QuerySearchVectorStartPayload); ok { return p, nil }
	case EVENT_QUERY_SEARCH_VECTOR_END:
		if p, ok := e.Payload.(QuerySearchVectorEndPayload); ok { return p, nil }
	case EVENT_QUERY_SEARCH_GRAPH_START:
		if p, ok := e.Payload.(QuerySearchGraphStartPayload); ok { return p, nil }
	case EVENT_QUERY_SEARCH_GRAPH_END:
		if p, ok := e.Payload.(QuerySearchGraphEndPayload); ok { return p, nil }
	case EVENT_QUERY_CONTEXT_START:
		if p, ok := e.Payload.(QueryContextStartPayload); ok { return p, nil }
	case EVENT_QUERY_CONTEXT_END:
		if p, ok := e.Payload.(QueryContextEndPayload); ok { return p, nil }
	case EVENT_QUERY_GENERATION_START:
		if p, ok := e.Payload.(QueryGenerationStartPayload); ok { return p, nil }
	case EVENT_QUERY_GENERATION_END:
		if p, ok := e.Payload.(QueryGenerationEndPayload); ok { return p, nil }
	case EVENT_QUERY_END:
		if p, ok := e.Payload.(QueryEndPayload); ok { return p, nil }
	case EVENT_QUERY_ERROR:
		if p, ok := e.Payload.(QueryErrorPayload); ok { return p, nil }

	// --- Memify Events ---
	case EVENT_MEMIFY_START:
		if p, ok := e.Payload.(MemifyStartPayload); ok { return p, nil }
	case EVENT_MEMIFY_UNKNOWN_SEARCH_START:
		if p, ok := e.Payload.(MemifyUnknownSearchStartPayload); ok { return p, nil }
	case EVENT_MEMIFY_UNKNOWN_SEARCH_END:
		if p, ok := e.Payload.(MemifyUnknownSearchEndPayload); ok { return p, nil }
	case EVENT_MEMIFY_UNKNOWN_ITEM_START:
		if p, ok := e.Payload.(MemifyUnknownItemStartPayload); ok { return p, nil }
	case EVENT_MEMIFY_UNKNOWN_ITEM_SEARCH_START:
		if p, ok := e.Payload.(MemifyUnknownItemSearchStartPayload); ok { return p, nil }
	case EVENT_MEMIFY_UNKNOWN_ITEM_SEARCH_END:
		if p, ok := e.Payload.(MemifyUnknownItemSearchEndPayload); ok { return p, nil }
	case EVENT_MEMIFY_UNKNOWN_ITEM_SOLVE_START:
		if p, ok := e.Payload.(MemifyUnknownItemSolveStartPayload); ok { return p, nil }
	case EVENT_MEMIFY_UNKNOWN_ITEM_SOLVE_END:
		if p, ok := e.Payload.(MemifyUnknownItemSolveEndPayload); ok { return p, nil }
	case EVENT_MEMIFY_UNKNOWN_ITEM_END:
		if p, ok := e.Payload.(MemifyUnknownItemEndPayload); ok { return p, nil }
	case EVENT_MEMIFY_EXPANSION_LOOP_START:
		if p, ok := e.Payload.(MemifyExpansionLoopStartPayload); ok { return p, nil }
	case EVENT_MEMIFY_EXPANSION_LOOP_END:
		if p, ok := e.Payload.(MemifyExpansionLoopEndPayload); ok { return p, nil }
	case EVENT_MEMIFY_EXPANSION_BATCH_START:
		if p, ok := e.Payload.(MemifyExpansionBatchStartPayload); ok { return p, nil }
	case EVENT_MEMIFY_EXPANSION_BATCH_PROCESS_START:
		if p, ok := e.Payload.(MemifyExpansionBatchProcessStartPayload); ok { return p, nil }
	case EVENT_MEMIFY_EXPANSION_BATCH_PROCESS_END:
		if p, ok := e.Payload.(MemifyExpansionBatchProcessEndPayload); ok { return p, nil }
	case EVENT_MEMIFY_EXPANSION_BATCH_END:
		if p, ok := e.Payload.(MemifyExpansionBatchEndPayload); ok { return p, nil }
	case EVENT_MEMIFY_END:
		if p, ok := e.Payload.(MemifyEndPayload); ok { return p, nil }
	case EVENT_MEMIFY_ERROR:
		if p, ok := e.Payload.(MemifyErrorPayload); ok { return p, nil }
	}
	return nil, fmt.Errorf("unknown event name or mismatch payload: %s", e.EventName)
}

// FormatEvent converts the event payload to a string format "<EventName> = <Key>: <Value>, ..."
// This creates a CSV-like string for logging purposes.
func FormatEvent(e StreamEvent) (string, error) {
	payload, err := ConvertEventPayload(e)
	if err != nil {
		return "", err
	}

	switch p := payload.(type) {
	// --- Absorb Events ---
	case AbsorbStartPayload:
		return fmt.Sprintf("%s = FileCount: %d", e.EventName, p.FileCount), nil
	case AbsorbAddFileStartPayload:
		return fmt.Sprintf("%s = FileName: %s", e.EventName, p.FileName), nil
	case AbsorbAddFileEndPayload:
		return fmt.Sprintf("%s = FileName: %s, Size: %d", e.EventName, p.FileName, p.Size), nil
	case AbsorbChunkingReadStartPayload:
		return fmt.Sprintf("%s = FileName: %s", e.EventName, p.FileName), nil
	case AbsorbChunkingReadEndPayload:
		return fmt.Sprintf("%s = FileName: %s", e.EventName, p.FileName), nil
	case AbsorbChunkingProcessEndPayload:
		return fmt.Sprintf("%s = ChunksCount: %d", e.EventName, p.ChunksCount), nil
	case AbsorbGraphRequestStartPayload:
		return fmt.Sprintf("%s = ChunkID: %s", e.EventName, p.ChunkID), nil
	// ... (Implement for ALL Absorb events) ...
	case AbsorbEndPayload:
		return fmt.Sprintf("%s = TotalTokens: %+v", e.EventName, p.TotalTokens), nil
	case AbsorbErrorPayload:
		return fmt.Sprintf("%s = Error: %s", e.EventName, p.Error.Error()), nil

	// --- Query Events ---
	case QueryStartPayload:
		return fmt.Sprintf("%s = QueryType: %s", e.EventName, "Start"), nil // Placeholder
	// ... (Implement for ALL Query events) ...
	case QueryEndPayload:
		return fmt.Sprintf("%s = TokenUsage: %+v", e.EventName, p.TokenUsage), nil
	case QueryErrorPayload:
		return fmt.Sprintf("%s = Error: %s", e.EventName, p.Error.Error()), nil

	// --- Memify Events ---
	case MemifyStartPayload:
		return fmt.Sprintf("%s = MemoryGroup: %s", e.EventName, p.MemoryGroup), nil
	case MemifyUnknownSearchStartPayload:
		return fmt.Sprintf("%s = Unknown Search Start", e.EventName), nil
	case MemifyUnknownSearchEndPayload:
		return fmt.Sprintf("%s = UnknownCount: %d", e.EventName, p.UnknownCount), nil
	// ... (Implement for ALL Memify events) ...
	case MemifyEndPayload:
		return fmt.Sprintf("%s = TotalTokens: %+v", e.EventName, p.TotalTokens), nil
	case MemifyErrorPayload:
		return fmt.Sprintf("%s = Error: %s", e.EventName, p.Error.Error()), nil
	
	default:
		// Fallback for events with no specific fields other than BasePayload or if missed
		return fmt.Sprintf("%s = (No specific payload details)", e.EventName), nil
	}
}
```

## 2. Implement Stream Register Functions

各イベントファイルに、チャンネルへのブリッジとなる登録関数を追加します。

### 2.1. `src/pkg/cuber/event/absorb_event.go`

```go
func RegisterAbsorbStreamer(eb *eventbus.EventBus, ch chan<- StreamEvent) {
	// Helper to send to channel without blocking
	send := func(name EventName, p any) {
		select {
		case ch <- StreamEvent{EventName: name, Payload: p}:
		default:
			// Buffer full, drop or log (should utilize buffered chan)
		}
	}

	eventbus.Subscribe(eb, string(EVENT_ABSORB_START), func(p AbsorbStartPayload) error { send(EVENT_ABSORB_START, p); return nil })
	eventbus.Subscribe(eb, string(EVENT_ABSORB_ADD_FILE_START), func(p AbsorbAddFileStartPayload) error { send(EVENT_ABSORB_ADD_FILE_START, p); return nil })
	eventbus.Subscribe(eb, string(EVENT_ABSORB_ADD_FILE_END), func(p AbsorbAddFileEndPayload) error { send(EVENT_ABSORB_ADD_FILE_END, p); return nil })
	// ... (Implement for ALL Absorb events) ...
	eventbus.Subscribe(eb, string(EVENT_ABSORB_END), func(p AbsorbEndPayload) error { send(EVENT_ABSORB_END, p); return nil })
	eventbus.Subscribe(eb, string(EVENT_ABSORB_ERROR), func(p AbsorbErrorPayload) error { send(EVENT_ABSORB_ERROR, p); return nil })
}
```

### 2.2. `src/pkg/cuber/event/query_event.go`

```go
func RegisterQueryStreamer(eb *eventbus.EventBus, ch chan<- StreamEvent) {
	send := func(name EventName, p any) {
		select {
		case ch <- StreamEvent{EventName: name, Payload: p}:
		default:
		}
	}
	// ... (Subscribe for ALL Query events) ...
}
```

### 2.3. `src/pkg/cuber/event/memify_event.go`

```go
func RegisterMemifyStreamer(eb *eventbus.EventBus, ch chan<- StreamEvent) {
	send := func(name EventName, p any) {
		select {
		case ch <- StreamEvent{EventName: name, Payload: p}:
		default:
		}
	}
	// ... (Subscribe for ALL Memify events) ...
}
```

## 3. CuberService Refactoring (`src/pkg/cuber/cuber.go`)

`Absorb`, `Query`, `Memify` のシグネチャを変更し、ストリーミング用のチャンネル引数を追加します。
`doneCh` は `resultCh` の導入により不要となったため削除しました。

```go
func (s *CuberService) Absorb(
	// ... existing args ...
	dataCh chan<- event.StreamEvent, // NEW
) (types.TokenUsage, error) {
	// Register Streamer
	if dataCh != nil {
		event.RegisterAbsorbStreamer(eb, dataCh)
	}
	
	// ... (existing logic) ...
}

func (s *CuberService) Query(
	// ... existing args ...
	dataCh chan<- event.StreamEvent, // NEW
) (...) {
	if dataCh != nil { event.RegisterQueryStreamer(eb, dataCh) }
	// ...
}

func (s *CuberService) Memify(
	// ... existing args ...
	dataCh chan<- event.StreamEvent, // NEW
) (...) {
	if dataCh != nil { event.RegisterMemifyStreamer(eb, dataCh) }
	// ...
}
```

## 4. BL Layer Implementation (`src/mode/rt/rtbl/cubes_bl.go`)

### 4.0. Background & Requirements (Important)
**本フェーズの役割と将来のSSE対応について:**
本フェーズでは `CuberService` のメソッドを非同期実行 (`goroutine`) し、イベントをチャンネル経由で受け取る仕組みを導入しますが、**ユーザーに対するレスポンスの挙動は変更しません**。
将来的なフェーズでは、リクエストパラメータ（例: `stream=true`）に応じて、SSE (Server-Sent Events) でイベントを逐次返すか、従来通り処理完了後に JSON レスポンスを返すかを切り替えられるようにします。
そのため、本フェーズ (`Phase 20`) の実装完了条件は、**「`goroutine` 外で `CuberService` の戻り値（`TokenUsage`, `error` 等）を完全に取得し、従来通りの同期的なレスポンスを返せる状態にすること」** です。
戻り値を破棄する実装は、非ストリームモード（同期モード）での動作を破壊するため、絶対に行ってはいけません。

### 4.1. Implementation Pattern

`rtbl` 内の `AbsorbCube`, `QueryCube`, `MemifyCube` を修正します。
`ResultChannel` パターンを使用して、非同期実行された関数の戻り値をメインゴルーチンで確実に回収します。

#### Example: `AbsorbCube`

```go
// 戻り値を保持するための構造体を定義（関数内で定義しても良いが、可読性のため明示）
type absorbResult struct {
	usage types.TokenUsage
	err   error
}

func AbsorbCube(c *gin.Context) {
	// ... (Parameter Binding & Setup) ...

	// 1. Prepare Channels
	// dataCh: イベント受信用。バッファを持たせて送信側のブロックを防ぐ。
	dataCh := make(chan event.StreamEvent, 100)
	// resultCh: サービスの戻り値（Usage, Error）回収用。
	resultCh := make(chan absorbResult, 1)

	// 2. Execute Async within Goroutine
	go func() {
		// 戻り値を capture する
		usage, err := u.CuberService.Absorb(c.Request.Context(), u.EventBus, cubeDbFilePath, req.MemoryGroup, []string{tempFile},
			cognifyConfig, embeddingModelConfig, chatModelConfig,
			dataCh) // Pass dataCh only
		
		// 戻り値をチャンネル送信
		resultCh <- absorbResult{usage: usage, err: err}
		close(resultCh) // クローズして受信側を解放（必須ではないがマナー）
	}()

	// 3. Event Loop
	// 将来的にここで c.Stream() を使ってSSE配信を行う。
	// 現在はイベントを受信してログに出力/検証するのみ。
	for {
		select {
		case evt := <-dataCh:
			// Debug Logging
			msg, err := event.FormatEvent(evt)
			if err != nil {
				utils.LogWarn(u.Logger, "Format event failed", zap.Error(err))
			} else {
				utils.LogDebug(u.Logger, msg)
			}
			
		case res := <-resultCh:
			// 4. Handle Result & Resume Flow
			usage = res.usage
			err = res.err
			goto AfterLoop
		}
	}

AfterLoop:	
	// Default Logic Resumes Here...
	if err != nil {
		return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Absorb failed: %s", err.Error()))
	}
	// ... (Token Usage Check, DB Transaction, Stats Update, etc.) ...
}
```

**重要な注意:**
`Loop` ラベルは `select` 文ではなく `for` 文を抜けるために必要になる場合があります（`break Loop`）。
Go言語では `select` 内の `break` は `select` を抜けるだけで `for` ループは継続するため、明示的にループを脱出する手段（ラベル付きbreakやフラグ管理）が必要です。
上記例では `break` ではなく、`usage, err` を更新してループを脱出するロジック（`goto` や `break LoopLabel`）を適切に実装してください。

#### Example: `QueryCube`

```go
type queryResult struct {
	answer    *string
	chunks    *string
	summaries *string
	graph     *[]*storage.Triple
	usage     types.TokenUsage
	err       error
}

func QueryCube(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, req *rtreq.QueryCubeReq, res *rtres.QueryCubeRes) bool {
	// ... (Setup args until CuberService call) ...
	// (Note: CuberService call setup remains identical to current implementation)

	// 1. Prepare Channels
	dataCh := make(chan event.StreamEvent, 100)
	resultCh := make(chan queryResult, 1)

	// 2. Execute Async
	go func() {
		// Note: u.CuberService.Query returns (answer, chunks, summaries, graph, embedding, usage, err)
		// We ignore embedding (vectors) as it is not used in response/logic currently.
		answer, chunks, summaries, graph, _, usage, err := u.CuberService.Query(c.Request.Context(), u.EventBus, cubeDBFilePath, req.MemoryGroup, req.Text,
			types.QueryConfig{
				QueryType:   types.QueryType(queryType),
				SummaryTopk: req.SummaryTopk,
				ChunkTopk:   req.ChunkTopk,
				EntityTopk:  req.EntityTopk,
			},
			types.EmbeddingModelConfig{
				Provider:  cube.EmbeddingProvider,
				Model:     cube.EmbeddingModel,
				Dimension: cube.EmbeddingDimension,
				BaseURL:   cube.EmbeddingBaseURL,
				ApiKey:    decryptedEmbeddingApiKey,
			},
			chatConf,
			dataCh, // NEW: Pass dataCh
		)
		resultCh <- queryResult{
			answer:    answer,
			chunks:    chunks,
			summaries: summaries,
			graph:     graph,
			usage:     usage,
			err:       err,
		}
		close(resultCh)
	}()

	// 3. Event Loop
	var answer *string
	var chunks *string
	var summaries *string
	var graph *[]*storage.Triple
	var usage types.TokenUsage
	var qErr error

	for {
		select {
		case evt := <-dataCh:
			msg, err := event.FormatEvent(evt)
			if err != nil {
				utils.LogWarn(u.Logger, "Format event failed", zap.Error(err))
			} else {
				utils.LogDebug(u.Logger, msg)
			}
		case res := <-resultCh:
			answer = res.answer
			chunks = res.chunks
			summaries = res.summaries
			graph = res.graph
			usage = res.usage
			qErr = res.err
			goto AfterLoop
		}
	}

AfterLoop:
	// ... (Existing Logic) ...
	if qErr != nil {
		return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Query failed: %s", qErr.Error()))
	}
	// ...
}
```

#### Example: `MemifyCube`

```go
type memifyResult struct {
	usage types.TokenUsage
	err   error
}

func MemifyCube(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, req *rtreq.MemifyCubeReq, res *rtres.MemifyCubeRes) bool {
	// ... (Setup args until CuberService call) ...

	// 1. Prepare Channels
	dataCh := make(chan event.StreamEvent, 100)
	resultCh := make(chan memifyResult, 1)

	// 2. Execute Async
	go func() {
		usage, err := u.CuberService.Memify(c.Request.Context(), u.EventBus, cubeDBFilePath, req.MemoryGroup,
			&types.MemifyConfig{
				RecursiveDepth:     epochs - 1, // epochs=1 means depth=0
				PrioritizeUnknowns: req.PrioritizeUnknowns,
			},
			types.EmbeddingModelConfig{
				Provider:  cube.EmbeddingProvider,
				Model:     cube.EmbeddingModel,
				Dimension: cube.EmbeddingDimension,
				BaseURL:   cube.EmbeddingBaseURL,
				ApiKey:    decryptedEmbeddingApiKey,
			},
			chatConf,
			dataCh, // NEW
		)
		resultCh <- memifyResult{usage: usage, err: err}
		close(resultCh)
	}()

	// 3. Event Loop
	var usage types.TokenUsage
	var mErr error

	for {
		select {
		case evt := <-dataCh:
			msg, err := event.FormatEvent(evt)
			if err != nil {
				utils.LogWarn(u.Logger, "Format event failed", zap.Error(err))
			} else {
				utils.LogDebug(u.Logger, msg)
			}
		case res := <-resultCh:
			usage = res.usage
			mErr = res.err
			goto AfterLoop
		}
	}

AfterLoop:
	// ... (Existing Logic) ...
	if mErr != nil {
		return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Memify failed: %s", mErr.Error()))
	}
	// ...
}
```

### 4.2. Helper Struct Definitions (Summary)

各関数内で利用する Result 構造体の定義は、関数内で定義しても、パッケージレベルで定義しても構いませんが、明示的な型定義を行うことで可読性を高めてください。

- `absorbResult`: `usage`, `err`
- `queryResult`: `answer`, `chunks`, `summaries`, `graph`, `usage`, `err`
- `memifyResult`: `usage`, `err`
