package event

import (
	"fmt"
	"math/rand/v2"
	"sync"
)

// StreamEvent represents a unified event structure for streaming
type StreamEvent struct {
	EventName EventName
	Payload   any
}

// roundRobinCounters holds the current index for each EventName
var roundRobinCounters = make(map[EventName]int)
var counterMutex sync.Mutex

// FormatEvent converts the event payload to a natural language string
// isEn: true for English, false for Japanese
func FormatEvent(e StreamEvent, isEn bool) (string, error) {
	templates, exists := templateMap[e.EventName]
	if !exists {
		return "", fmt.Errorf("Unknown event: %s", e.EventName)
	}
	// Get the next index for this event type using round-robin
	counterMutex.Lock()
	if _, exists := roundRobinCounters[e.EventName]; !exists {
		roundRobinCounters[e.EventName] = rand.IntN(25)
	}
	idx := roundRobinCounters[e.EventName]
	roundRobinCounters[e.EventName] = (idx + 1) % 25
	counterMutex.Unlock()
	var templateArray [25]string
	if isEn {
		templateArray = templates.En
	} else {
		templateArray = templates.Ja
	}
	template := templateArray[idx]
	switch p := e.Payload.(type) {
	// ================================
	// --- Absorb Events ---
	// ================================
	// Absorb処理全体が開始された時に発火する
	case AbsorbStartPayload:
		return fmt.Sprintf(template, p.FileCount), nil
	// 個別のファイルの取り込み（Add）処理が開始された時に発火する
	case AbsorbAddFileStartPayload:
		return template, nil
	// 個別のファイルの取り込み（Add）処理が完了した時に発火する
	case AbsorbAddFileEndPayload:
		return template, nil
	// ファイル読み込み処理が開始された時に発火する
	case AbsorbChunkingReadStartPayload:
		return template, nil
	// ファイル読み込み処理が完了した時に発火する
	case AbsorbChunkingReadEndPayload:
		return template, nil
	// チャンクデータの保存処理が開始された時に発火する
	case AbsorbChunkingSaveStartPayload:
		return template, nil
	// チャンクデータの保存処理が完了した時に発火する
	case AbsorbChunkingSaveEndPayload:
		return template, nil
	// チャンク分割処理（テキストスプリッターの実行）が開始された時に発火する
	case AbsorbChunkingProcessStartPayload:
		return template, nil
	// チャンク分割処理が完了した時に発火する
	case AbsorbChunkingProcessEndPayload:
		return fmt.Sprintf(template, p.ChunksCount), nil
	// 知識グラフ抽出のためのLLMリクエストが開始された時に発火する
	case AbsorbGraphRequestStartPayload:
		return fmt.Sprintf(template, p.ChunkNum), nil
	// 知識グラフ抽出のためのLLMリクエストが完了した時に発火する
	case AbsorbGraphRequestEndPayload:
		return fmt.Sprintf(template, p.ChunkNum), nil
	// LLMの応答からグラフ要素（ノード・エッジ）をパースする処理が開始された時に発火する
	case AbsorbGraphParseStartPayload:
		return fmt.Sprintf(template, p.ChunkNum), nil
	// グラフ要素のパース処理が完了した時に発火する
	case AbsorbGraphParseEndPayload:
		return fmt.Sprintf(template, p.ChunkNum), nil
	// グラフの解析と解釈が完了した時に発火する
	case AbsorbGraphInterpretedPayload:
		return fmt.Sprintf(template, p.InterpretedContent), nil
	// チャンクのベクトルストアへの保存処理が開始された時に発火する
	case AbsorbStorageChunkStartPayload:
		return fmt.Sprintf(template, p.ChunkNum), nil
	// チャンクのベクトルストアへの保存処理が完了した時に発火する
	case AbsorbStorageChunkEndPayload:
		return fmt.Sprintf(template, p.ChunkNum), nil
	// ノードのベクトルストアへの保存処理が開始された時に発火する
	case AbsorbStorageNodeStartPayload:
		return fmt.Sprintf(template, p.NodeCount), nil
	// ノードのベクトルストアへの保存処理が完了した時に発火する
	case AbsorbStorageNodeEndPayload:
		return fmt.Sprintf(template, p.NodeCount), nil
	// エッジのベクトルストアへの保存処理が開始された時に発火する
	case AbsorbStorageEdgeStartPayload:
		return fmt.Sprintf(template, p.EdgeCount), nil
	// エッジのベクトルストアへの保存処理が完了した時に発火する
	case AbsorbStorageEdgeEndPayload:
		return fmt.Sprintf(template, p.EdgeCount), nil
	// ノードのベクトルストアへのインデックス保管用保存処理が開始された時に発火する
	case AbsorbStorageNodeIndexStartPayload:
		return fmt.Sprintf(template, p.NodeCount, p.EdgeCount), nil
	// ノードのベクトルストアへのインデックス保管用保存処理が完了した時に発火する
	case AbsorbStorageNodeIndexEndPayload:
		return fmt.Sprintf(template, p.NodeCount, p.EdgeCount), nil
	// ノードのベクトルストアへのインデックス保管用保存処理が開始された時に発火する
	case AbsorbStorageNodeEmbeddingStartPayload:
		return fmt.Sprintf(template, p.EntityName), nil
	// ノードのベクトルストアへのインデックス保管用保存処理が完了した時に発火する
	case AbsorbStorageNodeEmbeddingEndPayload:
		return fmt.Sprintf(template, p.EntityName), nil
	// 要約生成フェーズ全体が開始された時に発火する
	case AbsorbSummarizationStartPayload:
		return template, nil
	// 要約生成のためのLLMリクエストが開始された時に発火する
	case AbsorbSummarizationReqStartPayload:
		return fmt.Sprintf(template, p.ChunkNum), nil
	// 要約生成のためのLLMリクエストが完了した時に発火する
	case AbsorbSummarizationReqEndPayload:
		return fmt.Sprintf(template, p.ChunkNum, p.SummaryText), nil
	// 生成された要約の保存処理が開始された時に発火する
	case AbsorbSummarizationSaveStartPayload:
		return fmt.Sprintf(template, p.ChunkNum), nil
	// 生成された要約の保存処理が完了した時に発火する
	case AbsorbSummarizationSaveEndPayload:
		return fmt.Sprintf(template, p.ChunkNum), nil
	// 要約生成フェーズ全体が完了した時に発火する
	case AbsorbSummarizationEndPayload:
		return template, nil
	// Absorb処理全体が正常に完了した時に発火する
	case AbsorbEndPayload:
		return template, nil
	// Absorb処理中にエラーが発生した時に発火する
	case AbsorbErrorPayload:
		return fmt.Sprintf(template, p.Error.Error()), nil
	// ================================
	// --- Query Events ---
	// ================================
	// クエリ処理全体が開始された時に発火する
	case QueryStartPayload:
		return fmt.Sprintf(template, p.QueryText), nil
	// クエリテキストの埋め込みベクトル生成処理が開始された時に発火する
	case QueryEmbeddingStartPayload:
		return template, nil
	// クエリテキストの埋め込みベクトル生成処理が完了した時に発火する
	case QueryEmbeddingEndPayload:
		return template, nil
	// ベクトル検索（チャンク、サマリー、エンティティ検索）が開始された時に発火する
	case QuerySearchVectorStartPayload:
		return template, nil
	// ベクトル検索が完了し、ヒットした件数が確定した時に発火する
	case QuerySearchVectorEndPayload:
		return template, nil
	// 知識グラフの探索処理が開始された時に発火する
	case QuerySearchGraphStartPayload:
		return template, nil
	// 知識グラフの探索処理が完了し、関連するトリプルが見つかった時に発火する
	case QuerySearchGraphEndPayload:
		return template, nil
	// LLMに渡すコンテキスト（検索結果の統合）の構築が開始された時に発火する
	case QueryContextStartPayload:
		return template, nil
	// LLMに渡すコンテキストの構築が完了した時に発火する
	case QueryContextEndPayload:
		return template, nil
	// 最終的な回答生成のためのLLMリクエストが開始された時に発火する
	case QueryGenerationStartPayload:
		return template, nil
	// 最終的な回答生成が完了した時に発火する
	case QueryGenerationEndPayload:
		return template, nil
	// クエリ処理全体が正常に完了した時に発火する
	case QueryEndPayload:
		return template, nil
	// クエリ処理中にエラーが発生した時に発火する
	case QueryErrorPayload:
		return fmt.Sprintf(template, p.ErrorMessage), nil
	// ================================
	// --- Memify Events ---
	// ================================
	// Memify処理全体が開始された時に発火する
	case MemifyStartPayload:
		return template, nil
	// 未解決のUnknownアイテムの検索フェーズが開始された時に発火する
	case MemifyUnknownSearchStartPayload:
		return template, nil
	// 未解決のUnknownアイテムの検索フェーズが完了し、処理対象が特定された時に発火する
	case MemifyUnknownSearchEndPayload:
		return template, nil
	// 個別のUnknownアイテムの解決処理が開始された時に発火する
	case MemifyUnknownItemStartPayload:
		return template, nil
	// 個別のUnknownアイテムに関する関連情報の検索が開始された時に発火する
	case MemifyUnknownItemSearchStartPayload:
		return template, nil
	// 個別のUnknownアイテムに関する関連情報の検索が完了した時に発火する
	case MemifyUnknownItemSearchEndPayload:
		return template, nil
	// LLMによるUnknown解決（Insight生成）処理が開始された時に発火する
	case MemifyUnknownItemSolveStartPayload:
		return template, nil
	// LLMによるUnknown解決処理が完了した時に発火する
	case MemifyUnknownItemSolveEndPayload:
		return template, nil
	// 個別のUnknownアイテムの解決処理が完了した時に発火する
	case MemifyUnknownItemEndPayload:
		return template, nil
	// 知識グラフの拡張ループ（1エポック分）が開始された時に発火する
	case MemifyExpansionLoopStartPayload:
		return template, nil
	// 知識グラフの拡張ループが完了した時に発火する
	case MemifyExpansionLoopEndPayload:
		return template, nil
	// 拡張処理のバッチ実行が開始された時に発火する
	case MemifyExpansionBatchStartPayload:
		return template, nil
	// バッチ内のノード処理が開始された時に発火する
	case MemifyExpansionBatchProcessStartPayload:
		return template, nil
	// バッチ内のノード処理が完了した時に発火する
	case MemifyExpansionBatchProcessEndPayload:
		return template, nil
	// 拡張処理のバッチ実行が完了した時に発火する
	case MemifyExpansionBatchEndPayload:
		return template, nil
	// Memify処理全体が正常に完了した時に発火する
	case MemifyEndPayload:
		return template, nil
	// Memify処理中にエラーが発生した時に発火する
	case MemifyErrorPayload:
		return fmt.Sprintf(template, p.Error.Error()), nil
	// Fallback for events with no specific fields other than BasePayload or if missed
	default:
		return template, nil
	}
}
