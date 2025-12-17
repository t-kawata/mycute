package event

import (
	"github.com/t-kawata/mycute/lib/eventbus"
	"github.com/t-kawata/mycute/pkg/cuber/types"
	"github.com/t-kawata/mycute/pkg/cuber/utils"
	"go.uber.org/zap"
)

const (
	EVENT_ABSORB_START                     EventName = "ABSORB_START"                     // Absorb処理全体が開始された時に発火する
	EVENT_ABSORB_ADD_FILE_START            EventName = "ABSORB_ADD_FILE_START"            // 個別のファイルの取り込み（Add）処理が開始された時に発火する
	EVENT_ABSORB_ADD_FILE_END              EventName = "ABSORB_ADD_FILE_END"              // 個別のファイルの取り込み（Add）処理が完了した時に発火する
	EVENT_ABSORB_CHUNKING_READ_START       EventName = "ABSORB_CHUNKING_READ_START"       // ファイル読み込み処理が開始された時に発火する
	EVENT_ABSORB_CHUNKING_READ_END         EventName = "ABSORB_CHUNKING_READ_END"         // ファイル読み込み処理が完了した時に発火する
	EVENT_ABSORB_CHUNKING_SAVE_START       EventName = "ABSORB_CHUNKING_SAVE_START"       // チャンクデータの保存処理が開始された時に発火する
	EVENT_ABSORB_CHUNKING_SAVE_END         EventName = "ABSORB_CHUNKING_SAVE_END"         // チャンクデータの保存処理が完了した時に発火する
	EVENT_ABSORB_CHUNKING_PROCESS_START    EventName = "ABSORB_CHUNKING_PROCESS_START"    // チャンク分割処理（テキストスプリッターの実行）が開始された時に発火する
	EVENT_ABSORB_CHUNKING_PROCESS_END      EventName = "ABSORB_CHUNKING_PROCESS_END"      // チャンク分割処理が完了した時に発火する
	EVENT_ABSORB_GRAPH_REQUEST_START       EventName = "ABSORB_GRAPH_REQUEST_START"       // 知識グラフ抽出のためのLLMリクエストが開始された時に発火する
	EVENT_ABSORB_GRAPH_REQUEST_END         EventName = "ABSORB_GRAPH_REQUEST_END"         // 知識グラフ抽出のためのLLMリクエストが完了した時に発火する
	EVENT_ABSORB_GRAPH_PARSE_START         EventName = "ABSORB_GRAPH_PARSE_START"         // LLMの応答からグラフ要素（ノード・エッジ）をパースする処理が開始された時に発火する
	EVENT_ABSORB_GRAPH_PARSE_END           EventName = "ABSORB_GRAPH_PARSE_END"           // グラフ要素のパース処理が完了した時に発火する
	EVENT_ABSORB_STORAGE_CHUNK_START       EventName = "ABSORB_STORAGE_CHUNK_START"       // チャンクのベクトルストアへの保存処理が開始された時に発火する
	EVENT_ABSORB_STORAGE_CHUNK_END         EventName = "ABSORB_STORAGE_CHUNK_END"         // チャンクのベクトルストアへの保存処理が完了した時に発火する
	EVENT_ABSORB_STORAGE_NODE_START        EventName = "ABSORB_STORAGE_NODE_START"        // ノードのベクトルストアへの保存処理が開始された時に発火する
	EVENT_ABSORB_STORAGE_NODE_END          EventName = "ABSORB_STORAGE_NODE_END"          // ノードのベクトルストアへの保存処理が完了した時に発火する
	EVENT_ABSORB_STORAGE_EDGE_START        EventName = "ABSORB_STORAGE_EDGE_START"        // エッジのベクトルストアへの保存処理が開始された時に発火する
	EVENT_ABSORB_STORAGE_EDGE_END          EventName = "ABSORB_STORAGE_EDGE_END"          // エッジのベクトルストアへの保存処理が完了した時に発火する
	EVENT_ABSORB_SUMMARIZATION_START       EventName = "ABSORB_SUMMARIZATION_START"       // 要約生成フェーズ全体が開始された時に発火する
	EVENT_ABSORB_SUMMARIZATION_CHUNK_START EventName = "ABSORB_SUMMARIZATION_CHUNK_START" // 個別のチャンクに対する要約生成処理が開始された時に発火する
	EVENT_ABSORB_SUMMARIZATION_REQ_START   EventName = "ABSORB_SUMMARIZATION_REQ_START"   // 要約生成のためのLLMリクエストが開始された時に発火する
	EVENT_ABSORB_SUMMARIZATION_REQ_END     EventName = "ABSORB_SUMMARIZATION_REQ_END"     // 要約生成のためのLLMリクエストが完了した時に発火する
	EVENT_ABSORB_SUMMARIZATION_SAVE_START  EventName = "ABSORB_SUMMARIZATION_SAVE_START"  // 生成された要約の保存処理が開始された時に発火する
	EVENT_ABSORB_SUMMARIZATION_SAVE_END    EventName = "ABSORB_SUMMARIZATION_SAVE_END"    // 生成された要約の保存処理が完了した時に発火する
	EVENT_ABSORB_SUMMARIZATION_CHUNK_END   EventName = "ABSORB_SUMMARIZATION_CHUNK_END"   // 個別のチャンクに対する要約生成処理が完了した時に発火する
	EVENT_ABSORB_SUMMARIZATION_END         EventName = "ABSORB_SUMMARIZATION_END"         // 要約生成フェーズ全体が完了した時に発火する
	EVENT_ABSORB_END                       EventName = "ABSORB_END"                       // Absorb処理全体が正常に完了した時に発火する
	EVENT_ABSORB_ERROR                     EventName = "ABSORB_ERROR"                     // Absorb処理中にエラーが発生した時に発火する
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

type AbsorbSummarizationChunkStartPayload struct {
	BasePayload
	ChunkID string
}

type AbsorbSummarizationReqStartPayload struct {
	BasePayload
	ChunkID string
}

type AbsorbSummarizationReqEndPayload struct {
	BasePayload
	ChunkID string
}

type AbsorbSummarizationSaveStartPayload struct {
	BasePayload
	ChunkID string
}

type AbsorbSummarizationSaveEndPayload struct {
	BasePayload
	ChunkID string
}

type AbsorbSummarizationChunkEndPayload struct {
	BasePayload
	ChunkID string
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
	eventbus.Subscribe(eb, string(EVENT_ABSORB_START), func(p AbsorbStartPayload) error {
		utils.LogInfo(l, "Event: Absorb Started", zap.Int("files", p.FileCount))
		return nil
	})
	eventbus.Subscribe(eb, string(EVENT_ABSORB_ADD_FILE_START), func(p AbsorbAddFileStartPayload) error {
		utils.LogDebug(l, "Event: Add File Start", zap.String("file", p.FileName))
		return nil
	})
	eventbus.Subscribe(eb, string(EVENT_ABSORB_ADD_FILE_END), func(p AbsorbAddFileEndPayload) error {
		utils.LogDebug(l, "Event: Add File End", zap.String("file", p.FileName), zap.Int64("size", p.Size))
		return nil
	})
	eventbus.Subscribe(eb, string(EVENT_ABSORB_CHUNKING_READ_START), func(p AbsorbChunkingReadStartPayload) error {
		utils.LogDebug(l, "Event: Chunking Read Start", zap.String("file", p.FileName))
		return nil
	})
	eventbus.Subscribe(eb, string(EVENT_ABSORB_CHUNKING_READ_END), func(p AbsorbChunkingReadEndPayload) error {
		utils.LogDebug(l, "Event: Chunking Read End", zap.String("file", p.FileName))
		return nil
	})
	eventbus.Subscribe(eb, string(EVENT_ABSORB_CHUNKING_SAVE_START), func(p AbsorbChunkingSaveStartPayload) error {
		utils.LogDebug(l, "Event: Chunking Save Start")
		return nil
	})
	eventbus.Subscribe(eb, string(EVENT_ABSORB_CHUNKING_SAVE_END), func(p AbsorbChunkingSaveEndPayload) error {
		utils.LogDebug(l, "Event: Chunking Save End")
		return nil
	})
	eventbus.Subscribe(eb, string(EVENT_ABSORB_CHUNKING_PROCESS_START), func(p AbsorbChunkingProcessStartPayload) error {
		utils.LogDebug(l, "Event: Chunking Process Start")
		return nil
	})
	eventbus.Subscribe(eb, string(EVENT_ABSORB_CHUNKING_PROCESS_END), func(p AbsorbChunkingProcessEndPayload) error {
		utils.LogDebug(l, "Event: Chunking Process End", zap.Int("chunks", p.ChunksCount))
		return nil
	})
	eventbus.Subscribe(eb, string(EVENT_ABSORB_GRAPH_REQUEST_START), func(p AbsorbGraphRequestStartPayload) error {
		utils.LogDebug(l, "Event: Graph Request Start", zap.String("chunk", p.ChunkID))
		return nil
	})
	eventbus.Subscribe(eb, string(EVENT_ABSORB_GRAPH_REQUEST_END), func(p AbsorbGraphRequestEndPayload) error {
		utils.LogDebug(l, "Event: Graph Request End", zap.String("chunk", p.ChunkID))
		return nil
	})
	eventbus.Subscribe(eb, string(EVENT_ABSORB_GRAPH_PARSE_START), func(p AbsorbGraphParseStartPayload) error {
		utils.LogDebug(l, "Event: Graph Parse Start", zap.String("chunk", p.ChunkID))
		return nil
	})
	eventbus.Subscribe(eb, string(EVENT_ABSORB_GRAPH_PARSE_END), func(p AbsorbGraphParseEndPayload) error {
		utils.LogDebug(l, "Event: Graph Parse End", zap.String("chunk", p.ChunkID), zap.Int("nodes", p.NodesExtracted), zap.Int("edges", p.EdgesExtracted))
		return nil
	})
	eventbus.Subscribe(eb, string(EVENT_ABSORB_STORAGE_CHUNK_START), func(p AbsorbStorageChunkStartPayload) error {
		utils.LogDebug(l, "Event: Storage Chunk Start", zap.String("chunk", p.ChunkID))
		return nil
	})
	eventbus.Subscribe(eb, string(EVENT_ABSORB_STORAGE_CHUNK_END), func(p AbsorbStorageChunkEndPayload) error {
		utils.LogDebug(l, "Event: Storage Chunk End", zap.String("chunk", p.ChunkID))
		return nil
	})
	eventbus.Subscribe(eb, string(EVENT_ABSORB_STORAGE_NODE_START), func(p AbsorbStorageNodeStartPayload) error {
		utils.LogDebug(l, "Event: Storage Node Start", zap.Int("count", p.NodeCount))
		return nil
	})
	eventbus.Subscribe(eb, string(EVENT_ABSORB_STORAGE_NODE_END), func(p AbsorbStorageNodeEndPayload) error {
		utils.LogDebug(l, "Event: Storage Node End", zap.Int("count", p.NodeCount))
		return nil
	})
	eventbus.Subscribe(eb, string(EVENT_ABSORB_STORAGE_EDGE_START), func(p AbsorbStorageEdgeStartPayload) error {
		utils.LogDebug(l, "Event: Storage Edge Start", zap.Int("count", p.EdgeCount))
		return nil
	})
	eventbus.Subscribe(eb, string(EVENT_ABSORB_STORAGE_EDGE_END), func(p AbsorbStorageEdgeEndPayload) error {
		utils.LogDebug(l, "Event: Storage Edge End", zap.Int("count", p.EdgeCount))
		return nil
	})
	eventbus.Subscribe(eb, string(EVENT_ABSORB_SUMMARIZATION_START), func(p AbsorbSummarizationStartPayload) error {
		utils.LogInfo(l, "Event: Summarization Started")
		return nil
	})
	eventbus.Subscribe(eb, string(EVENT_ABSORB_SUMMARIZATION_CHUNK_START), func(p AbsorbSummarizationChunkStartPayload) error {
		utils.LogDebug(l, "Event: Summarization Chunk Start", zap.String("chunk", p.ChunkID))
		return nil
	})
	eventbus.Subscribe(eb, string(EVENT_ABSORB_SUMMARIZATION_REQ_START), func(p AbsorbSummarizationReqStartPayload) error {
		utils.LogDebug(l, "Event: Summarization Req Start", zap.String("chunk", p.ChunkID))
		return nil
	})
	eventbus.Subscribe(eb, string(EVENT_ABSORB_SUMMARIZATION_REQ_END), func(p AbsorbSummarizationReqEndPayload) error {
		utils.LogDebug(l, "Event: Summarization Req End", zap.String("chunk", p.ChunkID))
		return nil
	})
	eventbus.Subscribe(eb, string(EVENT_ABSORB_SUMMARIZATION_SAVE_START), func(p AbsorbSummarizationSaveStartPayload) error {
		utils.LogDebug(l, "Event: Summarization Save Start", zap.String("chunk", p.ChunkID))
		return nil
	})
	eventbus.Subscribe(eb, string(EVENT_ABSORB_SUMMARIZATION_SAVE_END), func(p AbsorbSummarizationSaveEndPayload) error {
		utils.LogDebug(l, "Event: Summarization Save End", zap.String("chunk", p.ChunkID))
		return nil
	})
	eventbus.Subscribe(eb, string(EVENT_ABSORB_SUMMARIZATION_CHUNK_END), func(p AbsorbSummarizationChunkEndPayload) error {
		utils.LogDebug(l, "Event: Summarization Chunk End", zap.String("chunk", p.ChunkID))
		return nil
	})
	eventbus.Subscribe(eb, string(EVENT_ABSORB_SUMMARIZATION_END), func(p AbsorbSummarizationEndPayload) error {
		utils.LogInfo(l, "Event: Summarization Ended", zap.Int("summaries", p.SummariesCreated))
		return nil
	})
	eventbus.Subscribe(eb, string(EVENT_ABSORB_END), func(p AbsorbEndPayload) error {
		utils.LogInfo(l, "Event: Absorb Ended", zap.Int64("total_tokens", p.TotalTokens.InputTokens+p.TotalTokens.OutputTokens))
		return nil
	})
	eventbus.Subscribe(eb, string(EVENT_ABSORB_ERROR), func(p AbsorbErrorPayload) error {
		utils.LogWarn(l, "Event: Absorb Error", zap.Error(p.Error))
		return nil
	})
}
