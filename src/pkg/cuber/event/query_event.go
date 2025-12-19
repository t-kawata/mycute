package event

import (
	"github.com/t-kawata/mycute/lib/eventbus"
	"github.com/t-kawata/mycute/pkg/cuber/types"
)

const (
	EVENT_QUERY_START               EventName = "QUERY_START"               // クエリ処理全体が開始された時に発火する
	EVENT_QUERY_EMBEDDING_START     EventName = "QUERY_EMBEDDING_START"     // クエリテキストの埋め込みベクトル生成処理が開始された時に発火する
	EVENT_QUERY_EMBEDDING_END       EventName = "QUERY_EMBEDDING_END"       // クエリテキストの埋め込みベクトル生成処理が完了した時に発火する
	EVENT_QUERY_SEARCH_VECTOR_START EventName = "QUERY_SEARCH_VECTOR_START" // ベクトル検索（チャンク、サマリー、エンティティ検索）が開始された時に発火する
	EVENT_QUERY_SEARCH_VECTOR_END   EventName = "QUERY_SEARCH_VECTOR_END"   // ベクトル検索が完了し、ヒットした件数が確定した時に発火する
	EVENT_QUERY_SEARCH_GRAPH_START  EventName = "QUERY_SEARCH_GRAPH_START"  // 知識グラフの探索処理が開始された時に発火する
	EVENT_QUERY_SEARCH_GRAPH_END    EventName = "QUERY_SEARCH_GRAPH_END"    // 知識グラフの探索処理が完了し、関連するトリプルが見つかった時に発火する
	EVENT_QUERY_CONTEXT_START       EventName = "QUERY_CONTEXT_START"       // LLMに渡すコンテキスト（検索結果の統合）の構築が開始された時に発火する
	EVENT_QUERY_CONTEXT_END         EventName = "QUERY_CONTEXT_END"         // LLMに渡すコンテキストの構築が完了した時に発火する
	EVENT_QUERY_GENERATION_START    EventName = "QUERY_GENERATION_START"    // 最終的な回答生成のためのLLMリクエストが開始された時に発火する
	EVENT_QUERY_GENERATION_END      EventName = "QUERY_GENERATION_END"      // 最終的な回答生成が完了した時に発火する
	EVENT_QUERY_END                 EventName = "QUERY_END"                 // クエリ処理全体が正常に完了した時に発火する
	EVENT_QUERY_ERROR               EventName = "QUERY_ERROR"               // クエリ処理中にエラーが発生した時に発火する
)

type QueryStartPayload struct {
	BasePayload
	QueryType string
	QueryText string
}

type QueryEmbeddingStartPayload struct {
	BasePayload
	Text string
}

type QueryEmbeddingEndPayload struct {
	BasePayload
	Dimension int
}

type QuerySearchVectorStartPayload struct {
	BasePayload
	TargetTable string // "Chunk", "Summary", "Entity"
}

type QuerySearchVectorEndPayload struct {
	BasePayload
	TargetTable string
	ResultCount int
}

type QuerySearchGraphStartPayload struct {
	BasePayload
	StartNodeCount int
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
	PromptName string
}

type QueryGenerationEndPayload struct {
	BasePayload
	TokenUsage types.TokenUsage
	Response   string
}

type QueryEndPayload struct {
	BasePayload
	QueryType   string
	Response    string
	TotalTokens types.TokenUsage
}

type QueryErrorPayload struct {
	BasePayload
	QueryType    string
	ErrorMessage string
}

// RegisterQueryStreamer subscribes to all query events and forwards them to the provided channel.
func RegisterQueryStreamer(eb *eventbus.EventBus, ch chan<- StreamEvent) {
	send := func(name EventName, p any) {
		ch <- StreamEvent{EventName: name, Payload: p}
	}
	eventbus.Subscribe(eb, string(EVENT_QUERY_START), func(p QueryStartPayload) error { send(EVENT_QUERY_START, p); return nil })
	eventbus.Subscribe(eb, string(EVENT_QUERY_EMBEDDING_START), func(p QueryEmbeddingStartPayload) error { send(EVENT_QUERY_EMBEDDING_START, p); return nil })
	eventbus.Subscribe(eb, string(EVENT_QUERY_EMBEDDING_END), func(p QueryEmbeddingEndPayload) error { send(EVENT_QUERY_EMBEDDING_END, p); return nil })
	eventbus.Subscribe(eb, string(EVENT_QUERY_SEARCH_VECTOR_START), func(p QuerySearchVectorStartPayload) error { send(EVENT_QUERY_SEARCH_VECTOR_START, p); return nil })
	eventbus.Subscribe(eb, string(EVENT_QUERY_SEARCH_VECTOR_END), func(p QuerySearchVectorEndPayload) error { send(EVENT_QUERY_SEARCH_VECTOR_END, p); return nil })
	eventbus.Subscribe(eb, string(EVENT_QUERY_SEARCH_GRAPH_START), func(p QuerySearchGraphStartPayload) error { send(EVENT_QUERY_SEARCH_GRAPH_START, p); return nil })
	eventbus.Subscribe(eb, string(EVENT_QUERY_SEARCH_GRAPH_END), func(p QuerySearchGraphEndPayload) error { send(EVENT_QUERY_SEARCH_GRAPH_END, p); return nil })
	eventbus.Subscribe(eb, string(EVENT_QUERY_CONTEXT_START), func(p QueryContextStartPayload) error { send(EVENT_QUERY_CONTEXT_START, p); return nil })
	eventbus.Subscribe(eb, string(EVENT_QUERY_CONTEXT_END), func(p QueryContextEndPayload) error { send(EVENT_QUERY_CONTEXT_END, p); return nil })
	eventbus.Subscribe(eb, string(EVENT_QUERY_GENERATION_START), func(p QueryGenerationStartPayload) error { send(EVENT_QUERY_GENERATION_START, p); return nil })
	eventbus.Subscribe(eb, string(EVENT_QUERY_GENERATION_END), func(p QueryGenerationEndPayload) error { send(EVENT_QUERY_GENERATION_END, p); return nil })
	eventbus.Subscribe(eb, string(EVENT_QUERY_END), func(p QueryEndPayload) error { send(EVENT_QUERY_END, p); return nil })
	eventbus.Subscribe(eb, string(EVENT_QUERY_ERROR), func(p QueryErrorPayload) error { send(EVENT_QUERY_ERROR, p); return nil })
}
