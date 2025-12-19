package event

import (
	"github.com/t-kawata/mycute/lib/eventbus"
	"github.com/t-kawata/mycute/pkg/cuber/types"
)

const (
	EVENT_ABSORB_START                        EventName = "ABSORB_START"                        // Absorb処理全体が開始された時に発火する
	EVENT_ABSORB_ADD_FILE_START               EventName = "ABSORB_ADD_FILE_START"               // 個別のファイルの取り込み（Add）処理が開始された時に発火する
	EVENT_ABSORB_ADD_FILE_END                 EventName = "ABSORB_ADD_FILE_END"                 // 個別のファイルの取り込み（Add）処理が完了した時に発火する
	EVENT_ABSORB_CHUNKING_READ_START          EventName = "ABSORB_CHUNKING_READ_START"          // ファイル読み込み処理が開始された時に発火する
	EVENT_ABSORB_CHUNKING_READ_END            EventName = "ABSORB_CHUNKING_READ_END"            // ファイル読み込み処理が完了した時に発火する
	EVENT_ABSORB_CHUNKING_SAVE_START          EventName = "ABSORB_CHUNKING_SAVE_START"          // チャンクデータの保存処理が開始された時に発火する
	EVENT_ABSORB_CHUNKING_SAVE_END            EventName = "ABSORB_CHUNKING_SAVE_END"            // チャンクデータの保存処理が完了した時に発火する
	EVENT_ABSORB_CHUNKING_PROCESS_START       EventName = "ABSORB_CHUNKING_PROCESS_START"       // チャンク分割処理（テキストスプリッターの実行）が開始された時に発火する
	EVENT_ABSORB_CHUNKING_PROCESS_END         EventName = "ABSORB_CHUNKING_PROCESS_END"         // チャンク分割処理が完了した時に発火する
	EVENT_ABSORB_GRAPH_REQUEST_START          EventName = "ABSORB_GRAPH_REQUEST_START"          // 知識グラフ抽出のためのLLMリクエストが開始された時に発火する
	EVENT_ABSORB_GRAPH_REQUEST_END            EventName = "ABSORB_GRAPH_REQUEST_END"            // 知識グラフ抽出のためのLLMリクエストが完了した時に発火する
	EVENT_ABSORB_GRAPH_PARSE_START            EventName = "ABSORB_GRAPH_PARSE_START"            // LLMの応答からグラフ要素（ノード・エッジ）をパースする処理が開始された時に発火する
	EVENT_ABSORB_GRAPH_PARSE_END              EventName = "ABSORB_GRAPH_PARSE_END"              // グラフ要素のパース処理が完了した時に発火する
	EVENT_ABSORB_GRAPH_INTERPRETED            EventName = "ABSORB_GRAPH_INTERPRETED"            // グラフの解析と解釈が完了した時に発火する
	EVENT_ABSORB_STORAGE_CHUNK_START          EventName = "ABSORB_STORAGE_CHUNK_START"          // チャンクのベクトルストアへの保存処理が開始された時に発火する
	EVENT_ABSORB_STORAGE_CHUNK_END            EventName = "ABSORB_STORAGE_CHUNK_END"            // チャンクのベクトルストアへの保存処理が完了した時に発火する
	EVENT_ABSORB_STORAGE_NODE_START           EventName = "ABSORB_STORAGE_NODE_START"           // ノードのベクトルストアへの保存処理が開始された時に発火する
	EVENT_ABSORB_STORAGE_NODE_END             EventName = "ABSORB_STORAGE_NODE_END"             // ノードのベクトルストアへの保存処理が完了した時に発火する
	EVENT_ABSORB_STORAGE_EDGE_START           EventName = "ABSORB_STORAGE_EDGE_START"           // エッジのベクトルストアへの保存処理が開始された時に発火する
	EVENT_ABSORB_STORAGE_EDGE_END             EventName = "ABSORB_STORAGE_EDGE_END"             // エッジのベクトルストアへの保存処理が完了した時に発火する
	EVENT_ABSORB_STORAGE_NODE_INDEX_START     EventName = "ABSORB_STORAGE_NODE_INDEX_START"     // ノードのベクトルストアへのインデックス保管用保存処理が開始された時に発火する
	EVENT_ABSORB_STORAGE_NODE_EMBEDDING_START EventName = "ABSORB_STORAGE_NODE_EMBEDDING_START" // ノードをエンティティとしてベクトルストアへ保存する処理が開始された時に発火する
	EVENT_ABSORB_STORAGE_NODE_EMBEDDING_END   EventName = "ABSORB_STORAGE_NODE_EMBEDDING_END"   // ノードをエンティティとしてベクトルストアへ保存する処理が完了した時に発火する
	EVENT_ABSORB_STORAGE_NODE_INDEX_END       EventName = "ABSORB_STORAGE_NODE_INDEX_END"       // ノードのベクトルストアへのインデックス保管用保存処理が完了した時に発火する
	EVENT_ABSORB_SUMMARIZATION_START          EventName = "ABSORB_SUMMARIZATION_START"          // 要約生成フェーズ全体が開始された時に発火する
	EVENT_ABSORB_SUMMARIZATION_REQ_START      EventName = "ABSORB_SUMMARIZATION_REQ_START"      // 要約生成のためのLLMリクエストが開始された時に発火する
	EVENT_ABSORB_SUMMARIZATION_REQ_END        EventName = "ABSORB_SUMMARIZATION_REQ_END"        // 要約生成のためのLLMリクエストが完了した時に発火する
	EVENT_ABSORB_SUMMARIZATION_SAVE_START     EventName = "ABSORB_SUMMARIZATION_SAVE_START"     // 生成された要約の保存処理が開始された時に発火する
	EVENT_ABSORB_SUMMARIZATION_SAVE_END       EventName = "ABSORB_SUMMARIZATION_SAVE_END"       // 生成された要約の保存処理が完了した時に発火する
	EVENT_ABSORB_SUMMARIZATION_END            EventName = "ABSORB_SUMMARIZATION_END"            // 要約生成フェーズ全体が完了した時に発火する
	EVENT_ABSORB_END                          EventName = "ABSORB_END"                          // Absorb処理全体が正常に完了した時に発火する
	EVENT_ABSORB_ERROR                        EventName = "ABSORB_ERROR"                        // Absorb処理中にエラーが発生した時に発火する
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
	ChunkID  string
	ChunkNum int
}

type AbsorbGraphRequestEndPayload struct {
	BasePayload
	ChunkID  string
	ChunkNum int
}

type AbsorbGraphParseStartPayload struct {
	BasePayload
	ChunkID  string
	ChunkNum int
}

type AbsorbGraphParseEndPayload struct {
	BasePayload
	ChunkID        string
	ChunkNum       int
	NodesExtracted int
	EdgesExtracted int
}

type AbsorbGraphInterpretedPayload struct {
	BasePayload
	InterpretedContent string
}

// Storage Events Granularity
type AbsorbStorageChunkStartPayload struct {
	BasePayload
	ChunkID  string
	ChunkNum int
}

type AbsorbStorageChunkEndPayload struct {
	BasePayload
	ChunkID  string
	ChunkNum int
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

type AbsorbStorageNodeIndexStartPayload struct {
	BasePayload
	NodeCount int
	EdgeCount int
}

type AbsorbStorageNodeIndexEndPayload struct {
	BasePayload
	NodeCount int
	EdgeCount int
}

type AbsorbStorageNodeEmbeddingStartPayload struct {
	BasePayload
	EntityName string
}

type AbsorbStorageNodeEmbeddingEndPayload struct {
	BasePayload
	EntityName string
}

type AbsorbSummarizationStartPayload struct {
	BasePayload
}

type AbsorbSummarizationReqStartPayload struct {
	BasePayload
	ChunkID  string
	ChunkNum int
}

type AbsorbSummarizationReqEndPayload struct {
	BasePayload
	ChunkID     string
	ChunkNum    int
	SummaryText string
}

type AbsorbSummarizationSaveStartPayload struct {
	BasePayload
	ChunkID  string
	ChunkNum int
}

type AbsorbSummarizationSaveEndPayload struct {
	BasePayload
	ChunkID  string
	ChunkNum int
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

// RegisterAbsorbStreamer subscribes to all absorb events and forwards them to the provided channel.
func RegisterAbsorbStreamer(eb *eventbus.EventBus, ch chan<- StreamEvent) {
	send := func(name EventName, p any) {
		ch <- StreamEvent{EventName: name, Payload: p}
	}
	eventbus.Subscribe(eb, string(EVENT_ABSORB_START), func(p AbsorbStartPayload) error { send(EVENT_ABSORB_START, p); return nil })
	eventbus.Subscribe(eb, string(EVENT_ABSORB_ADD_FILE_START), func(p AbsorbAddFileStartPayload) error { send(EVENT_ABSORB_ADD_FILE_START, p); return nil })
	eventbus.Subscribe(eb, string(EVENT_ABSORB_ADD_FILE_END), func(p AbsorbAddFileEndPayload) error { send(EVENT_ABSORB_ADD_FILE_END, p); return nil })
	eventbus.Subscribe(eb, string(EVENT_ABSORB_CHUNKING_READ_START), func(p AbsorbChunkingReadStartPayload) error { send(EVENT_ABSORB_CHUNKING_READ_START, p); return nil })
	eventbus.Subscribe(eb, string(EVENT_ABSORB_CHUNKING_READ_END), func(p AbsorbChunkingReadEndPayload) error { send(EVENT_ABSORB_CHUNKING_READ_END, p); return nil })
	eventbus.Subscribe(eb, string(EVENT_ABSORB_CHUNKING_SAVE_START), func(p AbsorbChunkingSaveStartPayload) error { send(EVENT_ABSORB_CHUNKING_SAVE_START, p); return nil })
	eventbus.Subscribe(eb, string(EVENT_ABSORB_CHUNKING_SAVE_END), func(p AbsorbChunkingSaveEndPayload) error { send(EVENT_ABSORB_CHUNKING_SAVE_END, p); return nil })
	eventbus.Subscribe(eb, string(EVENT_ABSORB_CHUNKING_PROCESS_START), func(p AbsorbChunkingProcessStartPayload) error {
		send(EVENT_ABSORB_CHUNKING_PROCESS_START, p)
		return nil
	})
	eventbus.Subscribe(eb, string(EVENT_ABSORB_CHUNKING_PROCESS_END), func(p AbsorbChunkingProcessEndPayload) error { send(EVENT_ABSORB_CHUNKING_PROCESS_END, p); return nil })
	eventbus.Subscribe(eb, string(EVENT_ABSORB_GRAPH_REQUEST_START), func(p AbsorbGraphRequestStartPayload) error { send(EVENT_ABSORB_GRAPH_REQUEST_START, p); return nil })
	eventbus.Subscribe(eb, string(EVENT_ABSORB_GRAPH_REQUEST_END), func(p AbsorbGraphRequestEndPayload) error { send(EVENT_ABSORB_GRAPH_REQUEST_END, p); return nil })
	eventbus.Subscribe(eb, string(EVENT_ABSORB_GRAPH_PARSE_START), func(p AbsorbGraphParseStartPayload) error { send(EVENT_ABSORB_GRAPH_PARSE_START, p); return nil })
	eventbus.Subscribe(eb, string(EVENT_ABSORB_GRAPH_PARSE_END), func(p AbsorbGraphParseEndPayload) error { send(EVENT_ABSORB_GRAPH_PARSE_END, p); return nil })
	eventbus.Subscribe(eb, string(EVENT_ABSORB_GRAPH_INTERPRETED), func(p AbsorbGraphInterpretedPayload) error { send(EVENT_ABSORB_GRAPH_INTERPRETED, p); return nil })
	eventbus.Subscribe(eb, string(EVENT_ABSORB_STORAGE_CHUNK_START), func(p AbsorbStorageChunkStartPayload) error { send(EVENT_ABSORB_STORAGE_CHUNK_START, p); return nil })
	eventbus.Subscribe(eb, string(EVENT_ABSORB_STORAGE_CHUNK_END), func(p AbsorbStorageChunkEndPayload) error { send(EVENT_ABSORB_STORAGE_CHUNK_END, p); return nil })
	eventbus.Subscribe(eb, string(EVENT_ABSORB_STORAGE_NODE_START), func(p AbsorbStorageNodeStartPayload) error { send(EVENT_ABSORB_STORAGE_NODE_START, p); return nil })
	eventbus.Subscribe(eb, string(EVENT_ABSORB_STORAGE_NODE_END), func(p AbsorbStorageNodeEndPayload) error { send(EVENT_ABSORB_STORAGE_NODE_END, p); return nil })
	eventbus.Subscribe(eb, string(EVENT_ABSORB_STORAGE_EDGE_START), func(p AbsorbStorageEdgeStartPayload) error { send(EVENT_ABSORB_STORAGE_EDGE_START, p); return nil })
	eventbus.Subscribe(eb, string(EVENT_ABSORB_STORAGE_EDGE_END), func(p AbsorbStorageEdgeEndPayload) error { send(EVENT_ABSORB_STORAGE_EDGE_END, p); return nil })
	eventbus.Subscribe(eb, string(EVENT_ABSORB_STORAGE_NODE_INDEX_START), func(p AbsorbStorageNodeIndexStartPayload) error {
		send(EVENT_ABSORB_STORAGE_NODE_INDEX_START, p)
		return nil
	})
	eventbus.Subscribe(eb, string(EVENT_ABSORB_STORAGE_NODE_EMBEDDING_START), func(p AbsorbStorageNodeEmbeddingStartPayload) error {
		send(EVENT_ABSORB_STORAGE_NODE_EMBEDDING_START, p)
		return nil
	})
	eventbus.Subscribe(eb, string(EVENT_ABSORB_STORAGE_NODE_EMBEDDING_END), func(p AbsorbStorageNodeEmbeddingEndPayload) error {
		send(EVENT_ABSORB_STORAGE_NODE_EMBEDDING_END, p)
		return nil
	})
	eventbus.Subscribe(eb, string(EVENT_ABSORB_STORAGE_NODE_INDEX_END), func(p AbsorbStorageNodeIndexEndPayload) error {
		send(EVENT_ABSORB_STORAGE_NODE_INDEX_END, p)
		return nil
	})
	eventbus.Subscribe(eb, string(EVENT_ABSORB_SUMMARIZATION_START), func(p AbsorbSummarizationStartPayload) error { send(EVENT_ABSORB_SUMMARIZATION_START, p); return nil })
	eventbus.Subscribe(eb, string(EVENT_ABSORB_SUMMARIZATION_REQ_START), func(p AbsorbSummarizationReqStartPayload) error {
		send(EVENT_ABSORB_SUMMARIZATION_REQ_START, p)
		return nil
	})
	eventbus.Subscribe(eb, string(EVENT_ABSORB_SUMMARIZATION_REQ_END), func(p AbsorbSummarizationReqEndPayload) error {
		send(EVENT_ABSORB_SUMMARIZATION_REQ_END, p)
		return nil
	})
	eventbus.Subscribe(eb, string(EVENT_ABSORB_SUMMARIZATION_SAVE_START), func(p AbsorbSummarizationSaveStartPayload) error {
		send(EVENT_ABSORB_SUMMARIZATION_SAVE_START, p)
		return nil
	})
	eventbus.Subscribe(eb, string(EVENT_ABSORB_SUMMARIZATION_SAVE_END), func(p AbsorbSummarizationSaveEndPayload) error {
		send(EVENT_ABSORB_SUMMARIZATION_SAVE_END, p)
		return nil
	})
	eventbus.Subscribe(eb, string(EVENT_ABSORB_SUMMARIZATION_END), func(p AbsorbSummarizationEndPayload) error { send(EVENT_ABSORB_SUMMARIZATION_END, p); return nil })
	eventbus.Subscribe(eb, string(EVENT_ABSORB_END), func(p AbsorbEndPayload) error { send(EVENT_ABSORB_END, p); return nil })
	eventbus.Subscribe(eb, string(EVENT_ABSORB_ERROR), func(p AbsorbErrorPayload) error { send(EVENT_ABSORB_ERROR, p); return nil })
}
