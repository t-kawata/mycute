package event

import (
	"github.com/t-kawata/mycute/lib/eventbus"
	"github.com/t-kawata/mycute/pkg/cuber/types"
	"github.com/t-kawata/mycute/pkg/cuber/utils"
	"go.uber.org/zap"
)

const (
	EVENT_MEMIFY_START                         EventName = "MEMIFY_START"                         // Memify処理全体が開始された時に発火する
	EVENT_MEMIFY_UNKNOWN_SEARCH_START          EventName = "MEMIFY_UNKNOWN_SEARCH_START"          // 未解決のUnknownアイテムの検索フェーズが開始された時に発火する
	EVENT_MEMIFY_UNKNOWN_SEARCH_END            EventName = "MEMIFY_UNKNOWN_SEARCH_END"            // 未解決のUnknownアイテムの検索フェーズが完了し、処理対象が特定された時に発火する
	EVENT_MEMIFY_UNKNOWN_ITEM_START            EventName = "MEMIFY_UNKNOWN_ITEM_START"            // 個別のUnknownアイテムの解決処理が開始された時に発火する
	EVENT_MEMIFY_UNKNOWN_ITEM_SEARCH_START     EventName = "MEMIFY_UNKNOWN_ITEM_SEARCH_START"     // 個別のUnknownアイテムに関する関連情報の検索が開始された時に発火する
	EVENT_MEMIFY_UNKNOWN_ITEM_SEARCH_END       EventName = "MEMIFY_UNKNOWN_ITEM_SEARCH_END"       // 個別のUnknownアイテムに関する関連情報の検索が完了した時に発火する
	EVENT_MEMIFY_UNKNOWN_ITEM_SOLVE_START      EventName = "MEMIFY_UNKNOWN_ITEM_SOLVE_START"      // LLMによるUnknown解決（Insight生成）処理が開始された時に発火する
	EVENT_MEMIFY_UNKNOWN_ITEM_SOLVE_END        EventName = "MEMIFY_UNKNOWN_ITEM_SOLVE_END"        // LLMによるUnknown解決処理が完了した時に発火する
	EVENT_MEMIFY_UNKNOWN_ITEM_END              EventName = "MEMIFY_UNKNOWN_ITEM_END"              // 個別のUnknownアイテムの解決処理が完了した時に発火する
	EVENT_MEMIFY_EXPANSION_LOOP_START          EventName = "MEMIFY_EXPANSION_LOOP_START"          // 知識グラフの拡張ループ（1エポック分）が開始された時に発火する
	EVENT_MEMIFY_EXPANSION_LOOP_END            EventName = "MEMIFY_EXPANSION_LOOP_END"            // 知識グラフの拡張ループが完了した時に発火する
	EVENT_MEMIFY_EXPANSION_BATCH_START         EventName = "MEMIFY_EXPANSION_BATCH_START"         // 拡張処理のバッチ実行が開始された時に発火する
	EVENT_MEMIFY_EXPANSION_BATCH_PROCESS_START EventName = "MEMIFY_EXPANSION_BATCH_PROCESS_START" // バッチ内のノード処理が開始された時に発火する
	EVENT_MEMIFY_EXPANSION_BATCH_PROCESS_END   EventName = "MEMIFY_EXPANSION_BATCH_PROCESS_END"   // バッチ内のノード処理が完了した時に発火する
	EVENT_MEMIFY_EXPANSION_BATCH_END           EventName = "MEMIFY_EXPANSION_BATCH_END"           // 拡張処理のバッチ実行が完了した時に発火する
	EVENT_MEMIFY_END                           EventName = "MEMIFY_END"                           // Memify処理全体が正常に完了した時に発火する
	EVENT_MEMIFY_ERROR                         EventName = "MEMIFY_ERROR"                         // Memify処理中にエラーが発生した時に発火する
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
	UnknownID   string
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
	eventbus.Subscribe(eb, string(EVENT_MEMIFY_START), func(p MemifyStartPayload) error {
		utils.LogInfo(l, "Event: Memify Started")
		return nil
	})
	eventbus.Subscribe(eb, string(EVENT_MEMIFY_UNKNOWN_SEARCH_START), func(p MemifyUnknownSearchStartPayload) error {
		utils.LogDebug(l, "Event: Unknown Search Start")
		return nil
	})
	eventbus.Subscribe(eb, string(EVENT_MEMIFY_UNKNOWN_SEARCH_END), func(p MemifyUnknownSearchEndPayload) error {
		utils.LogDebug(l, "Event: Unknown Search End", zap.Int("count", p.UnknownCount))
		return nil
	})
	eventbus.Subscribe(eb, string(EVENT_MEMIFY_UNKNOWN_ITEM_START), func(p MemifyUnknownItemStartPayload) error {
		utils.LogDebug(l, "Event: Unknown Item Start", zap.String("id", p.UnknownID))
		return nil
	})
	eventbus.Subscribe(eb, string(EVENT_MEMIFY_UNKNOWN_ITEM_SEARCH_START), func(p MemifyUnknownItemSearchStartPayload) error {
		utils.LogDebug(l, "Event: Unknown Item Search Start", zap.String("id", p.UnknownID))
		return nil
	})
	eventbus.Subscribe(eb, string(EVENT_MEMIFY_UNKNOWN_ITEM_SEARCH_END), func(p MemifyUnknownItemSearchEndPayload) error {
		utils.LogDebug(l, "Event: Unknown Item Search End", zap.String("id", p.UnknownID), zap.Int("results", p.ResultCount))
		return nil
	})
	eventbus.Subscribe(eb, string(EVENT_MEMIFY_UNKNOWN_ITEM_SOLVE_START), func(p MemifyUnknownItemSolveStartPayload) error {
		utils.LogDebug(l, "Event: Unknown Item Solve Start", zap.String("id", p.UnknownID))
		return nil
	})
	eventbus.Subscribe(eb, string(EVENT_MEMIFY_UNKNOWN_ITEM_SOLVE_END), func(p MemifyUnknownItemSolveEndPayload) error {
		utils.LogDebug(l, "Event: Unknown Item Solve End", zap.String("id", p.UnknownID)) // Insight might be too long to log by default
		return nil
	})
	eventbus.Subscribe(eb, string(EVENT_MEMIFY_UNKNOWN_ITEM_END), func(p MemifyUnknownItemEndPayload) error {
		utils.LogDebug(l, "Event: Unknown Item End", zap.String("id", p.UnknownID))
		return nil
	})
	eventbus.Subscribe(eb, string(EVENT_MEMIFY_EXPANSION_LOOP_START), func(p MemifyExpansionLoopStartPayload) error {
		utils.LogDebug(l, "Event: Expansion Loop Start", zap.Int("level", p.Level))
		return nil
	})
	eventbus.Subscribe(eb, string(EVENT_MEMIFY_EXPANSION_LOOP_END), func(p MemifyExpansionLoopEndPayload) error {
		utils.LogDebug(l, "Event: Expansion Loop End", zap.Int("level", p.Level))
		return nil
	})
	eventbus.Subscribe(eb, string(EVENT_MEMIFY_EXPANSION_BATCH_START), func(p MemifyExpansionBatchStartPayload) error {
		utils.LogDebug(l, "Event: Expansion Batch Start", zap.Int("index", p.BatchIndex))
		return nil
	})
	eventbus.Subscribe(eb, string(EVENT_MEMIFY_EXPANSION_BATCH_PROCESS_START), func(p MemifyExpansionBatchProcessStartPayload) error {
		utils.LogDebug(l, "Event: Expansion Batch Process Start")
		return nil
	})
	eventbus.Subscribe(eb, string(EVENT_MEMIFY_EXPANSION_BATCH_PROCESS_END), func(p MemifyExpansionBatchProcessEndPayload) error {
		utils.LogDebug(l, "Event: Expansion Batch Process End")
		return nil
	})
	eventbus.Subscribe(eb, string(EVENT_MEMIFY_EXPANSION_BATCH_END), func(p MemifyExpansionBatchEndPayload) error {
		utils.LogDebug(l, "Event: Expansion Batch End", zap.Int("index", p.BatchIndex))
		return nil
	})
	eventbus.Subscribe(eb, string(EVENT_MEMIFY_END), func(p MemifyEndPayload) error {
		utils.LogInfo(l, "Event: Memify Ended", zap.Int64("total_tokens", p.TotalTokens.InputTokens+p.TotalTokens.OutputTokens))
		return nil
	})
	eventbus.Subscribe(eb, string(EVENT_MEMIFY_ERROR), func(p MemifyErrorPayload) error {
		utils.LogWarn(l, "Event: Memify Error", zap.Error(p.Error))
		return nil
	})
}
