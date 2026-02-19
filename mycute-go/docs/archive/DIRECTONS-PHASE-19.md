# Phase-19 Event Handling Observability

本フェーズでは、`CuberService` の主要なメソッド (`Absorb`, `Query`, `Memify`) に `EventBus` を導入し、詳細なイベントを `Emit` することで、処理の進行状況を細かく追跡可能な可観測性を実装します。

## 1. Event Definitions

イベント定義と実装は `src/pkg/cuber/event` パッケージに集約します。
`src/pkg/cuber/types/event_types.go` は作成しません。

### 1.1. Base Structure (`src/pkg/cuber/event/base.go`)

全てのイベントペイロードに必要な共通フィールドを定義します。

**【重要】ペイロード設計方針**:
ペイロードをイベント受信側でどのように利用するかは実装時点では考慮せず、将来的な柔軟性を確保するため、**その時点で利用可能な情報は可能な限り全て（冗長であっても）ペイロードに含める** という方針で定義してください。

```go
package event

import (
	"time"
	"github.com/gin-gonic/gin"
	"github.com/t-kawata/mycute/lib/eventbus"
)

// EventName definition
type EventName string

// EventSet definition
type EventSet[T any] struct {
	Name    EventName
	Handler eventbus.Handler[T]
}

// BasePayload contains common fields for all events
type BasePayload struct {
	CubeID      string
	MemoryGroup string
	Timestamp   int64 // Unix Milliseconds
}

func NewBasePayload(cubeID, memoryGroup string) BasePayload {
	return BasePayload{
		CubeID:      cubeID,
		MemoryGroup: memoryGroup,
		Timestamp:   time.Now().UnixMilli(),
	}
}
```

### 1.2. Absorb Events (`src/pkg/cuber/event/absorb.go`)

Absorb処理の各工程を詳細なイベントに分割して定義します。

```go
package event

import (
	"github.com/t-kawata/mycute/lib/eventbus"
	"github.com/t-kawata/mycute/pkg/cuber/types"
	"github.com/t-kawata/mycute/pkg/cuber/utils"
	"go.uber.org/zap"
)

const (
	EVENT_ABSORB_START                EventName = "ABSORB_START"
	EVENT_ABSORB_ADD_FILE_START       EventName = "ABSORB_ADD_FILE_START"
	EVENT_ABSORB_ADD_FILE_END         EventName = "ABSORB_ADD_FILE_END"
	EVENT_ABSORB_CHUNKING_READ_START  EventName = "ABSORB_CHUNKING_READ_START"
	EVENT_ABSORB_CHUNKING_READ_END    EventName = "ABSORB_CHUNKING_READ_END"
	EVENT_ABSORB_CHUNKING_SAVE_START  EventName = "ABSORB_CHUNKING_SAVE_START"
	EVENT_ABSORB_CHUNKING_SAVE_END    EventName = "ABSORB_CHUNKING_SAVE_END"
	EVENT_ABSORB_CHUNKING_PROCESS_START EventName = "ABSORB_CHUNKING_PROCESS_START"
	EVENT_ABSORB_CHUNKING_PROCESS_END   EventName = "ABSORB_CHUNKING_PROCESS_END"
	EVENT_ABSORB_GRAPH_REQUEST_START  EventName = "ABSORB_GRAPH_REQUEST_START"
	EVENT_ABSORB_GRAPH_REQUEST_END    EventName = "ABSORB_GRAPH_REQUEST_END"
	EVENT_ABSORB_GRAPH_PARSE_START    EventName = "ABSORB_GRAPH_PARSE_START"
	EVENT_ABSORB_GRAPH_PARSE_END      EventName = "ABSORB_GRAPH_PARSE_END"
	EVENT_ABSORB_STORAGE_CHUNK_START  EventName = "ABSORB_STORAGE_CHUNK_START"
	EVENT_ABSORB_STORAGE_CHUNK_END    EventName = "ABSORB_STORAGE_CHUNK_END"
	EVENT_ABSORB_STORAGE_NODE_START   EventName = "ABSORB_STORAGE_NODE_START"
	EVENT_ABSORB_STORAGE_NODE_END     EventName = "ABSORB_STORAGE_NODE_END"
	EVENT_ABSORB_STORAGE_EDGE_START   EventName = "ABSORB_STORAGE_EDGE_START"
	EVENT_ABSORB_STORAGE_EDGE_END     EventName = "ABSORB_STORAGE_EDGE_END"
	EVENT_ABSORB_SUMMARIZATION_START       EventName = "ABSORB_SUMMARIZATION_START"
	EVENT_ABSORB_SUMMARIZATION_CHUNK_START EventName = "ABSORB_SUMMARIZATION_CHUNK_START"
	EVENT_ABSORB_SUMMARIZATION_REQ_START   EventName = "ABSORB_SUMMARIZATION_REQ_START"
	EVENT_ABSORB_SUMMARIZATION_REQ_END     EventName = "ABSORB_SUMMARIZATION_REQ_END"
	EVENT_ABSORB_SUMMARIZATION_SAVE_START  EventName = "ABSORB_SUMMARIZATION_SAVE_START"
	EVENT_ABSORB_SUMMARIZATION_SAVE_END    EventName = "ABSORB_SUMMARIZATION_SAVE_END"
	EVENT_ABSORB_SUMMARIZATION_CHUNK_END   EventName = "ABSORB_SUMMARIZATION_CHUNK_END"
	EVENT_ABSORB_SUMMARIZATION_END         EventName = "ABSORB_SUMMARIZATION_END"
	EVENT_ABSORB_END                       EventName = "ABSORB_END"
	EVENT_ABSORB_ERROR                     EventName = "ABSORB_ERROR"
)

type AbsorbStartPayload struct {
	BasePayload
	FileCount int
}

type AbsorbAddFileStartPayload struct {
	BasePayload
	FileName string
}

type AbsorbAddFileEndPayload struct {
	BasePayload
	FileName string
	Size     int64
}

// Chunking Events Granularity
type AbsorbChunkingReadStartPayload struct {
	BasePayload
	FileName string
}

type AbsorbChunkingReadEndPayload struct {
	BasePayload
	FileName string
}

type AbsorbChunkingSaveStartPayload struct {
	BasePayload
}

type AbsorbChunkingSaveEndPayload struct {
	BasePayload
}

type AbsorbChunkingProcessStartPayload struct {
	BasePayload
}

type AbsorbChunkingProcessEndPayload struct {
	BasePayload
	ChunksCount int
}

// Graph Events Granularity (Per Chunk or Batch)
type AbsorbGraphRequestStartPayload struct {
	BasePayload
	ChunkID string
}

type AbsorbGraphRequestEndPayload struct {
	BasePayload
	ChunkID string
}

type AbsorbGraphParseStartPayload struct {
	BasePayload
	ChunkID string
}

type AbsorbGraphParseEndPayload struct {
	BasePayload
	ChunkID        string
	NodesExtracted int
	EdgesExtracted int
}

// Storage Events Granularity
type AbsorbStorageChunkStartPayload struct {
	BasePayload
	ChunkID string
}

type AbsorbStorageChunkEndPayload struct {
	BasePayload
	ChunkID string
}

type AbsorbStorageNodeStartPayload struct {
	BasePayload
	NodeCount int
}

type AbsorbStorageNodeEndPayload struct {
	BasePayload
	NodeCount int
}

type AbsorbStorageEdgeStartPayload struct {
	BasePayload
	EdgeCount int
}

type AbsorbStorageEdgeEndPayload struct {
	BasePayload
	EdgeCount int
}

type AbsorbSummarizationStartPayload struct {
	BasePayload
}

type AbsorbSummarizationEndPayload struct {
	BasePayload
	SummariesCreated int
}

type AbsorbEndPayload struct {
	BasePayload
	TotalTokens types.TokenUsage
}

type AbsorbErrorPayload struct {
	BasePayload
	Error error
}

func RegisterAbsorbEvents(eb *eventbus.EventBus, l *zap.Logger) {
	// ... Implement Subscribe calls for all above events using utils.Log* ...
	// Example:
	eventbus.Subscribe(eb, string(EVENT_ABSORB_START), func(p AbsorbStartPayload) error {
		utils.LogInfo(l, "Event: Absorb Started", zap.String("cube", p.CubeID), zap.Int("files", p.FileCount))
		return nil
	})
	// Implement others similarly...
}
```

### 1.3. Query Events (`src/pkg/cuber/event/query.go`)

```go
package event

import (
	"github.com/t-kawata/mycute/lib/eventbus"
	"github.com/t-kawata/mycute/pkg/cuber/types"
	"github.com/t-kawata/mycute/pkg/cuber/utils"
	"go.uber.org/zap"
)

const (
	EVENT_QUERY_START             EventName = "QUERY_START"
	EVENT_QUERY_EMBEDDING_START   EventName = "QUERY_EMBEDDING_START"
	EVENT_QUERY_EMBEDDING_END     EventName = "QUERY_EMBEDDING_END"
	
	EVENT_QUERY_SEARCH_VECTOR_START EventName = "QUERY_SEARCH_VECTOR_START"
	EVENT_QUERY_SEARCH_VECTOR_END   EventName = "QUERY_SEARCH_VECTOR_END"
	
	EVENT_QUERY_SEARCH_GRAPH_START  EventName = "QUERY_SEARCH_GRAPH_START"
	EVENT_QUERY_SEARCH_GRAPH_END    EventName = "QUERY_SEARCH_GRAPH_END"
	
	EVENT_QUERY_CONTEXT_START     EventName = "QUERY_CONTEXT_START"
	EVENT_QUERY_CONTEXT_END       EventName = "QUERY_CONTEXT_END"
	
	EVENT_QUERY_GENERATION_START  EventName = "QUERY_GENERATION_START"
	EVENT_QUERY_GENERATION_END    EventName = "QUERY_GENERATION_END"
	
	EVENT_QUERY_END               EventName = "QUERY_END"
	EVENT_QUERY_ERROR             EventName = "QUERY_ERROR"
)

type QueryStartPayload struct {
	BasePayload
	Text string
}

type QueryEmbeddingStartPayload struct {
	BasePayload
}

type QueryEmbeddingEndPayload struct {
	BasePayload
}

type QuerySearchVectorStartPayload struct {
	BasePayload
	Table string // "Chunk", "Summary", "Entity"
}

type QuerySearchVectorEndPayload struct {
	BasePayload
	Table    string
	HitCount int
}

type QuerySearchGraphStartPayload struct {
	BasePayload
}

type QuerySearchGraphEndPayload struct {
	BasePayload
	TriplesFound int
}

type QueryContextStartPayload struct {
	BasePayload
}

type QueryContextEndPayload struct {
	BasePayload
	ContextLength int
}

type QueryGenerationStartPayload struct {
	BasePayload
}

type QueryGenerationEndPayload struct {
	BasePayload
	AnswerPreview string
}

type QueryEndPayload struct {
	BasePayload
	TotalTokens types.TokenUsage
}

type QueryErrorPayload struct {
	BasePayload
	Error error
}

func RegisterQueryEvents(eb *eventbus.EventBus, l *zap.Logger) {
	// ... Implement Subscribe calls ...
}
```

### 1.4. Memify Events (`src/pkg/cuber/event/memify.go`)

```go
package event

import (
	"github.com/t-kawata/mycute/lib/eventbus"
	"github.com/t-kawata/mycute/pkg/cuber/types"
	"github.com/t-kawata/mycute/pkg/cuber/utils"
	"go.uber.org/zap"
)

const (
	EVENT_MEMIFY_START                    EventName = "MEMIFY_START"
	
	// Unknown Resolution Phase
	EVENT_MEMIFY_UNKNOWN_SEARCH_START     EventName = "MEMIFY_UNKNOWN_SEARCH_START"
	EVENT_MEMIFY_UNKNOWN_SEARCH_END       EventName = "MEMIFY_UNKNOWN_SEARCH_END"
	
	EVENT_MEMIFY_UNKNOWN_ITEM_START       EventName = "MEMIFY_UNKNOWN_ITEM_START"
	EVENT_MEMIFY_UNKNOWN_ITEM_SEARCH_START EventName = "MEMIFY_UNKNOWN_ITEM_SEARCH_START"
	EVENT_MEMIFY_UNKNOWN_ITEM_SEARCH_END   EventName = "MEMIFY_UNKNOWN_ITEM_SEARCH_END"
	EVENT_MEMIFY_UNKNOWN_ITEM_SOLVE_START  EventName = "MEMIFY_UNKNOWN_ITEM_SOLVE_START"
	EVENT_MEMIFY_UNKNOWN_ITEM_SOLVE_END    EventName = "MEMIFY_UNKNOWN_ITEM_SOLVE_END"
	EVENT_MEMIFY_UNKNOWN_ITEM_END          EventName = "MEMIFY_UNKNOWN_ITEM_END"
	
	// Graph Expansion Phase
	EVENT_MEMIFY_EXPANSION_LOOP_START     EventName = "MEMIFY_EXPANSION_LOOP_START"
	EVENT_MEMIFY_EXPANSION_LOOP_END       EventName = "MEMIFY_EXPANSION_LOOP_END"
	
	EVENT_MEMIFY_EXPANSION_BATCH_START    EventName = "MEMIFY_EXPANSION_BATCH_START"
	EVENT_MEMIFY_EXPANSION_BATCH_PROCESS_START EventName = "MEMIFY_EXPANSION_BATCH_PROCESS_START"
	EVENT_MEMIFY_EXPANSION_BATCH_PROCESS_END   EventName = "MEMIFY_EXPANSION_BATCH_PROCESS_END"
	EVENT_MEMIFY_EXPANSION_BATCH_END      EventName = "MEMIFY_EXPANSION_BATCH_END"
	
	EVENT_MEMIFY_END                      EventName = "MEMIFY_END"
	EVENT_MEMIFY_ERROR                    EventName = "MEMIFY_ERROR"
)

type MemifyStartPayload struct {
	BasePayload
}

type MemifyUnknownSearchStartPayload struct {
	BasePayload
}

type MemifyUnknownSearchEndPayload struct {
	BasePayload
	UnknownCount int
}

type MemifyUnknownItemStartPayload struct {
	BasePayload
	UnknownID string
}

type MemifyUnknownItemSearchStartPayload struct {
	BasePayload
	UnknownID string
}

type MemifyUnknownItemSearchEndPayload struct {
	BasePayload
	UnknownID string
	ResultCount int
}

type MemifyUnknownItemSolveStartPayload struct {
	BasePayload
	UnknownID string
}

type MemifyUnknownItemSolveEndPayload struct {
	BasePayload
	UnknownID string
	Insight   string
}

type MemifyUnknownItemEndPayload struct {
	BasePayload
	UnknownID string
}

type MemifyExpansionLoopStartPayload struct {
	BasePayload
	Level int
}

type MemifyExpansionLoopEndPayload struct {
	BasePayload
	Level int
}

type MemifyExpansionBatchStartPayload struct {
	BasePayload
	BatchIndex int
}

type MemifyExpansionBatchProcessStartPayload struct {
	BasePayload
}

type MemifyExpansionBatchProcessEndPayload struct {
	BasePayload
}

type MemifyExpansionBatchEndPayload struct {
	BasePayload
	BatchIndex int
}

type MemifyEndPayload struct {
	BasePayload
	TotalTokens types.TokenUsage
}

type MemifyErrorPayload struct {
	BasePayload
	Error error
}

func RegisterMemifyEvents(eb *eventbus.EventBus, l *zap.Logger) {
	// ... Implement Subscribe calls ...
}
```

---

## 2. CuberService Modifications (`src/pkg/cuber/cuber.go`)

### 2.1. Signature Updates

全ての外部公開メソッドおよび関連する内部メソッドに、`c *gin.Context` と `eb *eventbus.EventBus` を追加します。

```go
func (s *CuberService) Absorb(
	ctx context.Context,
	c *gin.Context, // Added
	eb *eventbus.EventBus, // Added
	// ...
) ...

func (s *CuberService) Query(
	ctx context.Context,
	c *gin.Context, // Added
	eb *eventbus.EventBus, // Added
	// ...
) ...

func (s *CuberService) Memify(
	ctx context.Context,
	c *gin.Context, // Added
	eb *eventbus.EventBus, // Added
	// ...
) ...

// Internal methods
func (s *CuberService) add(ctx context.Context, eb *eventbus.EventBus, ...) ...
func (s *CuberService) cognify(ctx context.Context, eb *eventbus.EventBus, ...) ...
```

### 2.2. Implementation Details

各メソッドで、上記のイベント定義に従って `Emit` を実装します。
`BasePayload` の生成には `event.NewBasePayload(cubeID, memoryGroup)` を使用して、タイムスタンプを自動付与してください。
`c *gin.Context` は今回は引数として受け取るのみとし、将来のSSE実装に備えます。

**Absorb Example:**
```go
func (s *CuberService) Absorb(...) ... {
    event.RegisterAbsorbEvents(eb, s.Logger)
    eventbus.Emit(eb, string(event.EVENT_ABSORB_START), event.AbsorbStartPayload{
        BasePayload: event.NewBasePayload(cubeID, memoryGroup),
        FileCount: len(filePaths),
    })

    // ... Add (Ingest) ...
    //   -> Iterate files
    //     -> Emit ABSORB_ADD_FILE_START / END
    
    // ... Cognify ...
    //   -> Emit ABSORB_CHUNKING_READ_START / END
    //   -> Emit ABSORB_CHUNKING_SAVE_START / END
    //   -> Emit ABSORB_CHUNKING_PROCESS_START / END
    //   -> Emit ABSORB_GRAPH_REQUEST_START / END (per chunk)
    //   -> Emit ABSORB_STORAGE_CHUNK_START / END
    //   -> Emit ABSORB_SUMMARIZATION_START
    //     -> Loop chunks
    //        -> Emit ABSORB_SUMMARIZATION_CHUNK_START
    //          -> Emit ABSORB_SUMMARIZATION_REQ_START / END (LLM)
    //          -> Emit ABSORB_SUMMARIZATION_SAVE_START / END
    //        -> Emit ABSORB_SUMMARIZATION_CHUNK_END
    //   -> Emit ABSORB_SUMMARIZATION_END
    //   These should be emitted within s.cognify or before/after calling tasks.

    eventbus.Emit(eb, string(event.EVENT_ABSORB_END), event.AbsorbEndPayload{
        BasePayload: event.NewBasePayload(cubeID, memoryGroup),
        TotalTokens: totalUsage,
    })
}
```

**Query Example:**
```go
func (s *CuberService) Query(...) ... {
    event.RegisterQueryEvents(eb, s.Logger)
    eventbus.Emit(eb, string(event.EVENT_QUERY_START), event.QueryStartPayload{...})

    // Embedding creation
    eventbus.Emit(eb, string(event.EVENT_QUERY_EMBEDDING_START), ...)
    // ...
    eventbus.Emit(eb, string(event.EVENT_QUERY_EMBEDDING_END), ...)

    // Search
    eventbus.Emit(eb, string(event.EVENT_QUERY_SEARCH_VECTOR_START), ...)
    // ... (Chunk/Summary/Entity search)
    eventbus.Emit(eb, string(event.EVENT_QUERY_SEARCH_VECTOR_END), ...)
    
    eventbus.Emit(eb, string(event.EVENT_QUERY_SEARCH_GRAPH_START), ...)
    // ... (Graph Traversal)
    eventbus.Emit(eb, string(event.EVENT_QUERY_SEARCH_GRAPH_END), ...)

    // Context Construction
    eventbus.Emit(eb, string(event.EVENT_QUERY_CONTEXT_START), ...)
    // ...
    eventbus.Emit(eb, string(event.EVENT_QUERY_CONTEXT_END), ...)

    // Generation
    eventbus.Emit(eb, string(event.EVENT_QUERY_GENERATION_START), ...)
    // ...
    eventbus.Emit(eb, string(event.EVENT_QUERY_GENERATION_END), ...)

    eventbus.Emit(eb, string(event.EVENT_QUERY_END), event.QueryEndPayload{...})
}
```

---

## 3. Caller Modifications (`src/mode/rt/rtbl/cubes_bl.go`)

`src/mode/rt/rtbl/cubes_bl.go` の呼び出し箇所を修正します。
`c *gin.Context` (第1引数で既に利用可能な変数 `c`) と `u.EventBus` を渡します。

*   `AbsorbCube`: `u.CuberService.Absorb(c.Request.Context(), c, u.EventBus, ...)`
*   `(QueryCube)`: `u.CuberService.Query(c.Request.Context(), c, u.EventBus, ...)`
*   `(MemifyCube)`: `u.CuberService.Memify(c.Request.Context(), c, u.EventBus, ...)`

---

## 4. Build & Verify

*   `make build` が通ることを確認。
*   イベントの定義ファイルが `src/pkg/cuber/event` に集約されていることを確認。
