// Package query は、グラフベースの検索ツールを提供します。
// GraphCompletionToolは、ベクトル検索とグラフトラバーサルを組み合わせて
// 質問に対する回答を生成します。
package query

import (
	"context"
	"errors"
	"fmt"
	"math"
	"slices"
	"strings"
	"time"

	"github.com/cloudwego/eino/components/model"
	"github.com/ikawaha/kagome/v2/tokenizer"
	appconfig "github.com/t-kawata/mycute/config"
	"github.com/t-kawata/mycute/lib/eventbus"
	"github.com/t-kawata/mycute/pkg/cuber/event"
	"github.com/t-kawata/mycute/pkg/cuber/prompts"
	"github.com/t-kawata/mycute/pkg/cuber/storage"
	"github.com/t-kawata/mycute/pkg/cuber/types"
	"github.com/t-kawata/mycute/pkg/cuber/utils"
	"go.uber.org/zap"
)

// GraphCompletionTool は、グラフベースの検索ツールです。
// このツールは、以下の検索タイプをサポートします：
//   - SUMMARIES: 要約のみを検索
//   - GRAPH_SUMMARY_COMPLETION: グラフを検索して要約を生成
//   - GRAPH_COMPLETION: グラフとチャンクを組み合わせて回答を生成（デフォルト）
type GraphCompletionTool struct {
	VectorStorage storage.VectorStorage      // ベクトルストレージ（LadybugDB）
	GraphStorage  storage.GraphStorage       // グラフストレージ（LadybugDB）
	LLM           model.ToolCallingChatModel // テキスト生成LLM (Eino)
	Embedder      storage.Embedder           // Embedder
	Kagome        *tokenizer.Tokenizer       // 日本語形態素解析器（Kagome）- FTSキーワード抽出用
	memoryGroup   string                     // メモリーグループ（パーティション識別子）
	ModelName     string                     // 使用するモデル名（トークン集計用）
	Logger        *zap.Logger                // ロガー
	EventBus      *eventbus.EventBus
}

// NewGraphCompletionTool は、新しいGraphCompletionToolを作成します。
// 引数:
//   - vectorStorage: ベクトルストレージ
//   - graphStorage: グラフストレージ
//   - llm: テキスト生成LLM
//   - embedder: Embedder
//   - kagome: 日本語形態素解析器（Kagome）
//   - memoryGroup: メモリーグループ
//   - modelName: 使用するモデル名
//   - l: ロガー
//   - eb: EventBus
//
// 返り値:
//   - *GraphCompletionTool: 新しいGraphCompletionToolインスタンス
func NewGraphCompletionTool(vectorStorage storage.VectorStorage, graphStorage storage.GraphStorage, llm model.ToolCallingChatModel, embedder storage.Embedder, kagome *tokenizer.Tokenizer, memoryGroup string, modelName string, l *zap.Logger, eb *eventbus.EventBus) *GraphCompletionTool {
	return &GraphCompletionTool{
		VectorStorage: vectorStorage,
		GraphStorage:  graphStorage,
		LLM:           llm,
		Embedder:      embedder,
		Kagome:        kagome,
		memoryGroup:   memoryGroup,
		ModelName:     modelName,
		Logger:        l,
		EventBus:      eb,
	}
}

// Query は、指定されたタイプで検索を実行し、回答を生成します。
// 引数:
//   - ctx: コンテキスト
//   - query: 検索クエリ
//   - queryType: 検索タイプ
//
// 返り値:
//   - string: 検索結果（回答）
func (t *GraphCompletionTool) Query(ctx context.Context, query string, config types.QueryConfig) (answer *string, chunks *string, summaries *string, graph *[]*storage.Triple, embedding *[]float32, usage types.TokenUsage, err error) {
	if !types.IsValidQueryType(uint8(config.QueryType)) {
		err = fmt.Errorf("GraphCompletionTool: Unknown query type: %d", config.QueryType)
		return
	}

	// 検索クエリを正規化（FTS・ベクトル検索の整合性確保）
	query = utils.NormalizeForSearch(query)

	// Emit Query Start
	eventbus.Emit(t.EventBus, string(event.EVENT_QUERY_START), event.QueryStartPayload{
		BasePayload: event.NewBasePayload(t.memoryGroup),
		QueryType:   config.QueryType.String(),
		QueryText:   query,
	})

	defer func() {
		if err != nil {
			// Emit Query Error
			eventbus.Emit(t.EventBus, string(event.EVENT_QUERY_ERROR), event.QueryErrorPayload{
				BasePayload:  event.NewBasePayload(t.memoryGroup),
				QueryType:    config.QueryType.String(),
				ErrorMessage: err.Error(),
			})
		}
		// Emit Query End
		if err == nil {
			eventbus.EmitSync(t.EventBus, string(event.EVENT_QUERY_END), event.QueryEndPayload{
				BasePayload: event.NewBasePayload(t.memoryGroup),
				QueryType:   config.QueryType.String(),
				QueryText:   query,
			})
			time.Sleep(150 * time.Millisecond) // Ensure event is processed before function return
		}
	}()

	switch config.QueryType {
	case types.QUERY_TYPE_GET_GRAPH:
		if config.EntityTopk == 0 {
			err = fmt.Errorf("GraphCompletionTool: EntityTopk must be greater than 0")
			return
		}
		embedding, graph, usage, err = t.getGraph(ctx, config.EntityTopk, query, nil, config)
		return
	case types.QUERY_TYPE_GET_CHUNKS:
		if config.ChunkTopk == 0 {
			err = fmt.Errorf("GraphCompletionTool: ChunkTopk must be greater than 0")
			return
		}
		embedding, chunks, usage, err = t.getChunks(ctx, config.ChunkTopk, query, nil)
		return
	case types.QUERY_TYPE_GET_PRE_MADE_SUMMARIES:
		if config.SummaryTopk == 0 {
			err = fmt.Errorf("GraphCompletionTool: SummaryTopk must be greater than 0")
			return
		}
		embedding, summaries, usage, err = t.getSummaries(ctx, config.SummaryTopk, query, nil)
		return
	case types.QUERY_TYPE_GET_GRAPH_AND_CHUNKS:
		if config.EntityTopk == 0 || config.ChunkTopk == 0 {
			err = fmt.Errorf("GraphCompletionTool: EntityTopk and ChunkTopk must be greater than 0")
			return
		}
		embedding, graph, chunks, usage, err = t.getGraphAndChunks(ctx, config.EntityTopk, config.ChunkTopk, query, nil, config)
		return
	case types.QUERY_TYPE_GET_GRAPH_AND_PRE_MADE_SUMMARIES:
		if config.EntityTopk == 0 || config.SummaryTopk == 0 {
			err = fmt.Errorf("GraphCompletionTool: EntityTopk and SummaryTopk must be greater than 0")
			return
		}
		embedding, graph, summaries, usage, err = t.getGraphAndSummaries(ctx, config.EntityTopk, config.SummaryTopk, query, nil, config)
		return
	case types.QUERY_TYPE_GET_GRAPH_AND_CHUNKS_AND_PRE_MADE_SUMMARIES:
		if config.EntityTopk == 0 || config.ChunkTopk == 0 || config.SummaryTopk == 0 {
			err = fmt.Errorf("GraphCompletionTool: EntityTopk, ChunkTopk and SummaryTopk must be greater than 0")
			return
		}
		embedding, graph, chunks, summaries, usage, err = t.getGraphAndChunksAndSummaries(ctx, config.EntityTopk, config.ChunkTopk, config.SummaryTopk, query, nil, config)
		return
	case types.QUERY_TYPE_GET_GRAPH_EXPLANATION:
		if config.EntityTopk == 0 {
			err = fmt.Errorf("GraphCompletionTool: EntityTopk must be greater than 0")
			return
		}
		if config.IsEn {
			embedding, answer, usage, err = t.getEnglishGraphExplanation(ctx, config.EntityTopk, query, nil, config)
		} else {
			embedding, answer, usage, err = t.getJapaneseGraphExplanation(ctx, config.EntityTopk, query, nil, config)
		}
		return
	case types.QUERY_TYPE_GET_GRAPH_SUMMARY:
		if config.EntityTopk == 0 {
			err = fmt.Errorf("GraphCompletionTool: EntityTopk must be greater than 0")
			return
		}
		if config.IsEn {
			embedding, answer, usage, err = t.getEnglishGraphSummary(ctx, config.EntityTopk, query, nil, config)
		} else {
			embedding, answer, usage, err = t.getJapaneseGraphSummary(ctx, config.EntityTopk, query, nil, config)
		}
		return
	case types.QUERY_TYPE_GET_GRAPH_SUMMARY_TO_ANSWER:
		if config.EntityTopk == 0 {
			err = fmt.Errorf("GraphCompletionTool: EntityTopk must be greater than 0")
			return
		}
		if config.IsEn {
			embedding, answer, usage, err = t.getEnglishGraphSummaryToAnswer(ctx, config.EntityTopk, query, nil, config)
		} else {
			embedding, answer, usage, err = t.getJapaneseGraphSummaryToAnswer(ctx, config.EntityTopk, query, nil, config)
		}
		return
	case types.QUERY_TYPE_ANSWER_BY_PRE_MADE_SUMMARIES_AND_GRAPH_SUMMARY:
		if config.SummaryTopk == 0 || config.EntityTopk == 0 {
			err = fmt.Errorf("GraphCompletionTool: SummaryTopk and EntityTopk must be greater than 0")
			return
		}
		if config.IsEn {
			embedding, answer, usage, err = t.getGraphSummaryCompletionEN(ctx, config.SummaryTopk, config.EntityTopk, query, nil, config)
		} else {
			embedding, answer, usage, err = t.getGraphSummaryCompletionJA(ctx, config.SummaryTopk, config.EntityTopk, query, nil, config)
		}
		return
	case types.QUERY_TYPE_ANSWER_BY_CHUNKS_AND_GRAPH_SUMMARY:
		if config.ChunkTopk == 0 || config.EntityTopk == 0 {
			err = fmt.Errorf("GraphCompletionTool: ChunkTopk and EntityTopk must be greater than 0")
			return
		}
		if config.IsEn {
			embedding, answer, usage, err = t.getGraphCompletionEN(ctx, config.ChunkTopk, config.EntityTopk, query, nil, config)
		} else {
			embedding, answer, usage, err = t.getGraphCompletionJA(ctx, config.ChunkTopk, config.EntityTopk, query, nil, config)
		}
		return
	default:
		err = fmt.Errorf("GraphCompletionTool: Unknown query type: %d", config.QueryType)
		return
	}
}

func (t *GraphCompletionTool) getGraph(ctx context.Context, entityTopk int, query string, embeddingVecs *[]float32, config types.QueryConfig) (embedding *[]float32, graph *[]*storage.Triple, usage types.TokenUsage, err error) {
	// ========================================
	// 1. ノードを検索
	// ========================================
	// 1-1. クエリのベクトルを取得
	var embeddingVectors []float32
	if embeddingVecs != nil && len(*embeddingVecs) > 0 {
		embeddingVectors = *embeddingVecs
	} else {
		// Emit Embedding Start
		eventbus.Emit(t.EventBus, string(event.EVENT_QUERY_EMBEDDING_START), event.QueryEmbeddingStartPayload{
			BasePayload: event.NewBasePayload(t.memoryGroup),
			Text:        query,
		})

		tmpEmbeddingVectors, u, errr := t.Embedder.EmbedQuery(ctx, query)
		usage.Add(u)
		if errr != nil {
			err = fmt.Errorf("GraphCompletionTool: Failed to embed query: %w", errr)
			return
		}

		// Emit Embedding End
		eventbus.Emit(t.EventBus, string(event.EVENT_QUERY_EMBEDDING_END), event.QueryEmbeddingEndPayload{
			BasePayload: event.NewBasePayload(t.memoryGroup),
			Dimension:   len(tmpEmbeddingVectors),
		})

		embeddingVectors = tmpEmbeddingVectors
	}
	// 1-2. クエリのベクトルでエンティティを検索（グラフトラバーサルの種となるエンティティリストを作るため）

	// Emit Vector Search Start (Entity)
	eventbus.Emit(t.EventBus, string(event.EVENT_QUERY_SEARCH_VECTOR_START), event.QuerySearchVectorStartPayload{
		BasePayload: event.NewBasePayload(t.memoryGroup),
		TargetTable: string(types.TABLE_NAME_ENTITY),
	})

	entityResults, err := t.VectorStorage.Query(ctx, types.TABLE_NAME_ENTITY, embeddingVectors, entityTopk, t.memoryGroup)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Node query failed: %w", err)
		return
	}

	// Emit Vector Search End
	entities := []string{}
	entitiesLen := len(entityResults)
	for _, entity := range entityResults {
		entities = append(entities, utils.GetNameStrByGraphNodeID(entity.ID))
	}
	eventbus.Emit(t.EventBus, string(event.EVENT_QUERY_SEARCH_VECTOR_END), event.QuerySearchVectorEndPayload{
		BasePayload: event.NewBasePayload(t.memoryGroup),
		TargetTable: string(types.TABLE_NAME_ENTITY),
		TargetCount: entitiesLen,
		Targets:     strings.Join(entities, ", "),
	})

	// 1-3. FTSによるエンティティ拡張
	// 最終的な探索対象エンティティを管理（IDの重複をマップで完全に排除）
	graphNodeIDCandidatesDoneMap := make(map[string]bool)
	// ここには、EntityテーブルのIDや、FTSで得られたキーワードが、GraphNodeテーブルのIDにマッチを試みる候補として入る
	var graphNodeIDCandidates []string

	// ベクトル検索結果のエンティティIDを追加
	for _, res := range entityResults {
		if !graphNodeIDCandidatesDoneMap[res.ID] {
			graphNodeIDCandidatesDoneMap[res.ID] = true
			// ここで追加されるのは、エンティティのIDなので、
			// GraphNodeテーブルのIDにマッチするものが存在するはずである。
			// つまり、グラフとラバーサルの最有力な種候補。
			graphNodeIDCandidates = append(graphNodeIDCandidates, res.ID)
		}
	}
	// FtsTopk > 0 の場合のみFTS検索を実行
	if config.FtsTopk > 0 {
		// Emit FTS Start
		eventbus.Emit(t.EventBus, string(event.EVENT_QUERY_FTS_START), event.QueryFtsStartPayload{
			BasePayload: event.NewBasePayload(t.memoryGroup),
			EntityCount: entitiesLen,
			Entities:    strings.Join(entities, ", "),
		})

		expandedCount := 0

		// エンティティ増殖 Phase 1: クエリ自体から形態素解析でエンティティ候補を抽出し増殖を試みる
		queryKeywords := utils.ExtractKeywords(t.Kagome, query, config.IsEn)
		var queryTermsToAdd string
		switch config.FtsLayer {
		case types.FTS_LAYER_NOUNS:
			queryTermsToAdd = queryKeywords.Nouns
		case types.FTS_LAYER_NOUNS_VERBS:
			queryTermsToAdd = queryKeywords.NounsVerbs
		default:
			queryTermsToAdd = queryKeywords.AllContentWords
		}
		ftsTerms := []string{}
		for term := range strings.SplitSeq(queryTermsToAdd, " ") {
			if term != "" && len(term) > 1 && !graphNodeIDCandidatesDoneMap[term] {
				graphNodeIDCandidatesDoneMap[term] = true
				// ここで追加されるのは、クエリ文字列から抽出した内容語なので、
				// GraphNodeテーブルのIDにマッチするものがあればラッキーというもの。
				// つまり、グラフとラバーサルの種として機能できるかはわからないが、サブグラフ取得の可能性を高めるためのもの。
				graphNodeIDCandidates = append(graphNodeIDCandidates, term)
				expandedCount++
				ftsTerms = append(ftsTerms, term)
			}
		}
		// エンティティ増殖 Phase 2: Entityテーブルから得られたエンティティIDを基に、ChunkテーブルをFTS検索することでグラフトラバーサルの種を増加させる
		for _, res := range entityResults {
			// res.Text (エンティティ名、例: "テスラ") を検索クエリとして Chunk テーブルを FTS 検索
			ftsResults, ftsErr := t.VectorStorage.FullTextSearch(
				ctx,
				types.TABLE_NAME_CHUNK,
				utils.GetNameStrByGraphNodeID(res.ID), // エンティティIDで検索
				config.FtsTopk,                        // FTS の Top-K (例: 3)
				t.memoryGroup,
				config.IsEn,
				config.FtsLayer, // 例: types.FTS_LAYER_NOUNS_VERBS
			)
			if ftsErr != nil {
				// FTS エラーは致命的ではないためログのみ
				utils.LogWarn(t.Logger, fmt.Sprintf("FTS error for entity '%s': %v", res.Text, ftsErr))
				continue
			}

			// ヒットしたチャンクからキーワードを取り出し、エンティティ候補として追加
			for _, ftsRes := range ftsResults {
				// QueryResult.Nouns には、チャンクから抽出された名詞がスペース区切りで格納されている
				// config.FtsLayer の検索対象層指定に関わらず、エンティティ候補として追加するのは「名詞」だけとする
				candidateTerms := strings.SplitSeq(ftsRes.Nouns, " ")
				for term := range candidateTerms {
					// 1文字のノイズを除外し、既に追加済みでなければリストに追加
					if term != "" && len(term) > 1 && !graphNodeIDCandidatesDoneMap[term] {
						graphNodeIDCandidatesDoneMap[term] = true
						// ここで追加されるのは、エンティティIDにマッチする全文検索レコードが持っているチャンク内の名詞リストなので、
						// GraphNodeテーブルのIDにマッチするものがあればラッキーというもの。
						// つまり、グラフとラバーサルの種として機能できるかはわからないが、サブグラフ取得の可能性を高めるためのもの。
						graphNodeIDCandidates = append(graphNodeIDCandidates, term)
						expandedCount++
						ftsTerms = append(ftsTerms, term)
					}
				}
			}
		}

		// Emit FTS End
		eventbus.Emit(t.EventBus, string(event.EVENT_QUERY_FTS_END), event.QueryFtsEndPayload{
			BasePayload:   event.NewBasePayload(t.memoryGroup),
			EntityCount:   len(entityResults),
			Entities:      strings.Join(entities, ", "),
			ExpandedCount: expandedCount,
			TotalCount:    len(graphNodeIDCandidates),
			FtsTerms:      strings.Join(ftsTerms, ", "),
		})
	}

	// ========================================
	// 2. グラフトラバーサル
	// ========================================

	// Emit Graph Search Start
	graphNodeIDCandidatesForDisplay := []string{}
	for _, term := range graphNodeIDCandidates {
		graphNodeIDCandidatesForDisplay = append(graphNodeIDCandidatesForDisplay, utils.GetNameStrByGraphNodeID(term))
	}
	eventbus.Emit(t.EventBus, string(event.EVENT_QUERY_SEARCH_GRAPH_START), event.QuerySearchGraphStartPayload{
		BasePayload:           event.NewBasePayload(t.memoryGroup),
		NodeIDCandidatesCount: len(graphNodeIDCandidates),
		GraphNodeIDCandidates: strings.Join(graphNodeIDCandidatesForDisplay, ", "),
	})

	triples, err := t.GraphStorage.GetTriples(ctx, graphNodeIDCandidates, t.memoryGroup)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Graph traversal failed: %w", err)
		return
	}

	// ========================================
	// 3. Thickness スコアリングとフィルタリング
	// ========================================
	// ThicknessThreshold が未指定（0）の場合、デフォルト値を使用
	thicknessThreshold := config.ThicknessThreshold
	if thicknessThreshold == 0 {
		thicknessThreshold = appconfig.DEFAULT_THICKNESS_THRESHOLD
	}
	if len(triples) > 0 && thicknessThreshold > 0 {
		// 3-1. MaxUnix を取得して相対時間減衰を計算
		maxUnix, getMaxErr := t.GraphStorage.GetMaxUnix(ctx, t.memoryGroup)
		if getMaxErr != nil {
			utils.LogWarn(t.Logger, "Failed to get MaxUnix, skipping thickness filtering", zap.Error(getMaxErr))
		} else if maxUnix > 0 {
			// 3-2. MemoryGroupConfig を取得してλを計算
			groupConfig, _ := t.GraphStorage.GetMemoryGroupConfig(ctx, t.memoryGroup)
			halfLifeDays := appconfig.DEFAULT_HALF_LIFE_DAYS // デフォルトは settings.go から取得
			if groupConfig != nil && groupConfig.HalfLifeDays > 0 {
				halfLifeDays = groupConfig.HalfLifeDays
			}
			lambda := utils.CalculateLambda(halfLifeDays)

			// 3-3. 各エッジの Thickness を計算してフィルタリング + 矛盾解決
			scoredTriples := make([]utils.ScoredTriple, 0, len(triples))
			for _, triple := range triples {
				thickness := utils.CalculateThickness(triple.Edge.Weight, triple.Edge.Confidence, triple.Edge.Unix, maxUnix, lambda)

				// 閾値フィルタリング
				if thickness < thicknessThreshold {
					continue
				}

				scoredTriples = append(scoredTriples, utils.ScoredTriple{
					Triple:    triple,
					Thickness: thickness,
				})
			}

			// 3-4. 矛盾解決 (Conflict Resolution)
			var discardedEdges []utils.DiscardedTriple
			if config.ConflictResolutionStage >= 1 {
				stage1BeforeTriplesCount := len(scoredTriples)

				// Emit Conflict Resolution 1 Start
				eventbus.Emit(t.EventBus, string(event.EVENT_INFO_CONFLICT_RESOLUTION_1_START), event.InfoConflictResolution1StartPayload{
					BasePayload:        event.NewBasePayload(t.memoryGroup),
					BeforeTriplesCount: stage1BeforeTriplesCount,
				})

				// Stage 1: 決定論的解決
				resolved, stage1Discarded, remainingConflicts := utils.Stage1ConflictResolution(scoredTriples, t.Logger, config.IsEn)
				scoredTriples = resolved
				discardedEdges = append(discardedEdges, stage1Discarded...)

				// Emit conflict discarded events for Stage 1
				for _, st := range stage1Discarded {
					eventbus.Emit(t.EventBus, string(event.EVENT_INFO_CONFLICT_DISCARDED), event.InfoConflictDiscardedPayload{
						BasePayload:  event.NewBasePayload(t.memoryGroup),
						SourceID:     st.Triple.Edge.SourceID,
						RelationType: st.Triple.Edge.Type,
						TargetID:     st.Triple.Edge.TargetID,
						Stage:        1,
						Reason:       st.Reason,
					})
				}

				stage1AfterTriplesCount := len(scoredTriples)

				// Emit Conflict Resolution 1 End
				eventbus.Emit(t.EventBus, string(event.EVENT_INFO_CONFLICT_RESOLUTION_1_END), event.InfoConflictResolution1EndPayload{
					BasePayload:        event.NewBasePayload(t.memoryGroup),
					BeforeTriplesCount: stage1BeforeTriplesCount,
					AfterTriplesCount:  stage1AfterTriplesCount,
				})

				// Stage 2: LLM による最終仲裁（Stage 2 有効かつ未解決の矛盾が残っている場合）
				if config.ConflictResolutionStage >= 2 && len(remainingConflicts) > 0 {
					// Emit Conflict Resolution 2 Start
					eventbus.Emit(t.EventBus, string(event.EVENT_INFO_CONFLICT_RESOLUTION_2_START), event.InfoConflictResolution2StartPayload{
						BasePayload:        event.NewBasePayload(t.memoryGroup),
						BeforeTriplesCount: stage1AfterTriplesCount,
					})

					// Stage 2: LLM による最終仲裁
					stage2Discarded, stage2Usage, stage2Err := utils.Stage2ConflictResolution(
						ctx,
						t.LLM,
						t.ModelName,
						&scoredTriples,
						remainingConflicts,
						config.IsEn,
						t.Logger,
					)
					usage.Add(stage2Usage)
					if stage2Err != nil {
						utils.LogWarn(t.Logger, "Stage2 conflict resolution failed", zap.Error(stage2Err))
					} else {
						discardedEdges = append(discardedEdges, stage2Discarded...)

						// Emit conflict discarded events for Stage 2
						for _, st := range stage2Discarded {
							eventbus.Emit(t.EventBus, string(event.EVENT_INFO_CONFLICT_DISCARDED), event.InfoConflictDiscardedPayload{
								BasePayload:  event.NewBasePayload(t.memoryGroup),
								SourceID:     st.Triple.Edge.SourceID,
								RelationType: st.Triple.Edge.Type,
								TargetID:     st.Triple.Edge.TargetID,
								Stage:        2,
								Reason:       st.Reason,
							})
						}
					}

					stage2AfterTriplesCount := len(scoredTriples)

					// Emit Conflict Resolution 2 End
					eventbus.Emit(t.EventBus, string(event.EVENT_INFO_CONFLICT_RESOLUTION_2_END), event.InfoConflictResolution2EndPayload{
						BasePayload:        event.NewBasePayload(t.memoryGroup),
						BeforeTriplesCount: stage1AfterTriplesCount,
						AfterTriplesCount:  stage2AfterTriplesCount,
					})
				}
			}

			// 発見された矛盾データを非同期で物理削除
			if len(discardedEdges) > 0 {
				go func(edges []utils.DiscardedTriple, memoryGroup string) {
					// クエリのコンテキストは終了する可能性があるため、Background を使用
					cleanupCtx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
					defer cancel()

					for _, st := range edges {
						err := t.GraphStorage.DeleteEdge(cleanupCtx, st.Triple.Edge.SourceID, st.Triple.Edge.Type, st.Triple.Edge.TargetID, memoryGroup)
						if err != nil {
							utils.LogWarn(t.Logger, "Failed to delete conflicting edge in background",
								zap.String("source", st.Triple.Edge.SourceID),
								zap.String("target", st.Triple.Edge.TargetID),
								zap.Error(err))
						} else {
							utils.LogDebug(t.Logger, "Deleted conflicting edge in background",
								zap.String("source", st.Triple.Edge.SourceID),
								zap.String("target", st.Triple.Edge.TargetID))
						}
					}
				}(discardedEdges, t.memoryGroup)
			}

			// 最終的なトリプルリストを構築（Thickness値をEdgeに設定）
			var filteredTriples []*storage.Triple
			for _, st := range scoredTriples {
				st.Triple.Edge.Thickness = st.Thickness
				filteredTriples = append(filteredTriples, st.Triple)
			}
			triples = filteredTriples
		}
	}

	// Emit Graph Search End
	eventbus.Emit(t.EventBus, string(event.EVENT_QUERY_SEARCH_GRAPH_END), event.QuerySearchGraphEndPayload{
		BasePayload:           event.NewBasePayload(t.memoryGroup),
		NodeIDCandidatesCount: len(graphNodeIDCandidates),
		GraphNodeIDCandidates: strings.Join(graphNodeIDCandidatesForDisplay, ", "),
		TriplesCount:          len(triples),
	})
	graph = &triples
	embedding = &embeddingVectors
	return
}

// getChunks は、Chunkのみを検索して返します。
// この関数は以下の処理を行います：
//  1. クエリをベクトル化
//  2. "Chunk"テーブルから類似するChunkを検索
//  3. Chunkのリストを返す
//
// 引数:
//   - ctx: コンテキスト
//   - chunkTopk: 返す結果の最大数
//   - query: 検索クエリ
//
// 返り値:
//   - string: Chunkのリスト
//   - error: エラーが発生した場合
func (t *GraphCompletionTool) getChunks(ctx context.Context, chunkTopk int, query string, embeddingVecs *[]float32) (embedding *[]float32, chunks *string, usage types.TokenUsage, err error) {
	// クエリをベクトル化
	var embeddingVectors []float32
	if embeddingVecs != nil && len(*embeddingVecs) > 0 {
		embeddingVectors = *embeddingVecs
	} else {
		tmpEmbeddingVectors, u, errr := t.Embedder.EmbedQuery(ctx, query)
		usage.Add(u)
		if errr != nil {
			err = fmt.Errorf("GraphCompletionTool: Failed to embed query: %w", errr)
			return
		}
		embeddingVectors = tmpEmbeddingVectors
	}
	// Chunkテーブルを検索
	// Emit Vector Search Start (Chunk)
	eventbus.Emit(t.EventBus, string(event.EVENT_QUERY_SEARCH_VECTOR_START), event.QuerySearchVectorStartPayload{
		BasePayload: event.NewBasePayload(t.memoryGroup),
		TargetTable: string(types.TABLE_NAME_CHUNK),
	})

	results, err := t.VectorStorage.Query(ctx, types.TABLE_NAME_CHUNK, embeddingVectors, chunkTopk, t.memoryGroup)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to query chunks: %w", err)
		return
	}

	// Emit Vector Search End
	targets := []string{}
	for _, result := range results {
		targets = append(targets, utils.TruncateString(result.Text, 10))
	}
	eventbus.Emit(t.EventBus, string(event.EVENT_QUERY_SEARCH_VECTOR_END), event.QuerySearchVectorEndPayload{
		BasePayload: event.NewBasePayload(t.memoryGroup),
		TargetTable: string(types.TABLE_NAME_CHUNK),
		TargetCount: len(results),
		Targets:     strings.Join(targets, ", "),
	})
	// 結果が見つからない場合
	if len(results) == 0 {
		tmp := ""
		chunks = &tmp
		return
	}
	// 要約のリストを構築
	var sb strings.Builder
	for _, result := range results {
		sb.WriteString("- " + result.Text + "\n\n")
	}
	tmp := strings.TrimSpace(sb.String())
	chunks = &tmp
	embedding = &embeddingVectors
	return
}

// getSummaries は、要約のみを検索して返します。
// この関数は以下の処理を行います：
//  1. クエリをベクトル化
//  2. "Summary"テーブルから類似する要約を検索
//  3. 要約のリストを返す
//
// 引数:
//   - ctx: コンテキスト
//   - summaryTopk: 返す結果の最大数
//   - query: 検索クエリ
//
// 返り値:
//   - string: 要約のリスト
//   - error: エラーが発生した場合
func (t *GraphCompletionTool) getSummaries(ctx context.Context, summaryTopk int, query string, embeddingVecs *[]float32) (embedding *[]float32, summaries *string, usage types.TokenUsage, err error) {
	// クエリをベクトル化
	var embeddingVectors []float32
	if embeddingVecs != nil && len(*embeddingVecs) > 0 {
		embeddingVectors = *embeddingVecs
	} else {
		tmpEmbeddingVectors, u, errr := t.Embedder.EmbedQuery(ctx, query)
		usage.Add(u)
		if errr != nil {
			err = fmt.Errorf("GraphCompletionTool: Failed to embed query: %w", errr)
			return
		}
		embeddingVectors = tmpEmbeddingVectors
	}
	// Summaryテーブルを検索

	// Emit Vector Search Start (Summary)
	eventbus.Emit(t.EventBus, string(event.EVENT_QUERY_SEARCH_VECTOR_START), event.QuerySearchVectorStartPayload{
		BasePayload: event.NewBasePayload(t.memoryGroup),
		TargetTable: string(types.TABLE_NAME_SUMMARY),
	})

	results, err := t.VectorStorage.Query(ctx, types.TABLE_NAME_SUMMARY, embeddingVectors, summaryTopk, t.memoryGroup)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to query summaries: %w", err)
		return
	}

	// Emit Vector Search End
	targets := []string{}
	for _, result := range results {
		targets = append(targets, utils.TruncateString(result.Text, 10))
	}
	eventbus.Emit(t.EventBus, string(event.EVENT_QUERY_SEARCH_VECTOR_END), event.QuerySearchVectorEndPayload{
		BasePayload: event.NewBasePayload(t.memoryGroup),
		TargetTable: string(types.TABLE_NAME_SUMMARY),
		TargetCount: len(results),
		Targets:     strings.Join(targets, ", "),
	})
	// 結果が見つからない場合
	if len(results) == 0 {
		tmp := ""
		summaries = &tmp
		return
	}
	// 要約のリストを構築
	var sb strings.Builder
	for _, result := range results {
		sb.WriteString("- " + result.Text + "\n\n")
	}
	tmp := strings.TrimSpace(sb.String())
	summaries = &tmp
	embedding = &embeddingVectors
	return
}

func (t *GraphCompletionTool) getGraphAndChunks(ctx context.Context, entityTopk int, chunkTopk int, query string, embeddingVecs *[]float32, config types.QueryConfig) (embedding *[]float32, graph *[]*storage.Triple, chunks *string, usage types.TokenUsage, err error) {
	embeddingVectors, triples, u, err := t.getGraph(ctx, entityTopk, query, embeddingVecs, config)
	usage.Add(u)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to get graph: %w", err)
		return
	}
	_, tmpChunks, u, err := t.getChunks(ctx, chunkTopk, query, embeddingVectors)
	usage.Add(u)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to get chunks: %w", err)
		return
	}
	graph = triples
	chunks = tmpChunks
	embedding = embeddingVectors
	return
}

func (t *GraphCompletionTool) getGraphAndSummaries(ctx context.Context, entityTopk int, summaryTopk int, query string, embeddingVecs *[]float32, config types.QueryConfig) (embedding *[]float32, graph *[]*storage.Triple, summaries *string, usage types.TokenUsage, err error) {
	embeddingVectors, triples, u, err := t.getGraph(ctx, entityTopk, query, embeddingVecs, config)
	usage.Add(u)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to get graph: %w", err)
		return
	}
	_, tmpSummaries, u, err := t.getSummaries(ctx, summaryTopk, query, embeddingVectors)
	usage.Add(u)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to get summaries: %w", err)
		return
	}
	graph = triples
	summaries = tmpSummaries
	embedding = embeddingVectors
	return
}

func (t *GraphCompletionTool) getGraphAndChunksAndSummaries(ctx context.Context, entityTopk int, chunkTopk int, summaryTopk int, query string, embeddingVecs *[]float32, config types.QueryConfig) (embedding *[]float32, graph *[]*storage.Triple, chunks *string, summaries *string, usage types.TokenUsage, err error) {
	embeddingVectors, triples, u, err := t.getGraph(ctx, entityTopk, query, embeddingVecs, config)
	usage.Add(u)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to get graph: %w", err)
		return
	}
	_, tmpChunks, u, err := t.getChunks(ctx, chunkTopk, query, embeddingVectors)
	usage.Add(u)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to get chunks: %w", err)
		return
	}
	_, tmpSummaries, u, err := t.getSummaries(ctx, summaryTopk, query, embeddingVectors)
	usage.Add(u)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to get summaries: %w", err)
		return
	}
	graph = triples
	chunks = tmpChunks
	summaries = tmpSummaries
	embedding = embeddingVectors
	return
}

func (t *GraphCompletionTool) getEnglishGraphExplanation(ctx context.Context, entityTopk int, query string, embeddingVecs *[]float32, config types.QueryConfig) (embedding *[]float32, graphExplanation *string, usage types.TokenUsage, err error) {
	// 1. 関連するグラフを検索
	embeddingVectors, triples, u, err := t.getGraph(ctx, entityTopk, query, embeddingVecs, config)
	usage.Add(u)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to get graph: %w", err)
		return
	}
	// 2. トリプルをテキスト説明文に変換
	graphText := &strings.Builder{}
	graphText = GenerateNaturalEnglishGraphExplanationByTriples(triples, graphText)
	tmp := strings.TrimSpace(graphText.String())
	graphExplanation = &tmp
	embedding = embeddingVectors
	return
}

func (t *GraphCompletionTool) getJapaneseGraphExplanation(ctx context.Context, entityTopk int, query string, embeddingVecs *[]float32, config types.QueryConfig) (embedding *[]float32, graphExplanation *string, usage types.TokenUsage, err error) {
	// 1. 関連するグラフを検索
	embeddingVectors, triples, u, err := t.getGraph(ctx, entityTopk, query, embeddingVecs, config)
	usage.Add(u)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to get graph: %w", err)
		return
	}
	// 2. トリプルをテキスト説明文に変換
	graphText := &strings.Builder{}
	graphText = GenerateNaturalJapaneseGraphExplanationByTriples(triples, graphText)
	tmp := strings.TrimSpace(graphText.String())
	graphExplanation = &tmp
	embedding = embeddingVectors
	return
}

func (t *GraphCompletionTool) getEnglishGraphSummary(ctx context.Context, entityTopk int, query string, embeddingVecs *[]float32, config types.QueryConfig) (embedding *[]float32, graphSummary *string, usage types.TokenUsage, err error) {
	embeddingVectors, graphExplanation, u, err := t.getEnglishGraphExplanation(ctx, entityTopk, query, embeddingVecs, config)
	usage.Add(u)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to get graph explanation: %w", err)
		return
	}
	summarizePrompt := fmt.Sprintf("USER QUERY: %s\n\nKNOWLEDGE GRAPH INFORMATION:\n%s", query, *graphExplanation)

	// Emit Generation Start (Graph Summary EN)
	eventbus.Emit(t.EventBus, string(event.EVENT_QUERY_GENERATION_START), event.QueryGenerationStartPayload{
		BasePayload: event.NewBasePayload(t.memoryGroup),
		PromptName:  "SUMMARIZE_GRAPH_ITSELF_EN_PROMPT",
	})

	summaryContent, u, err := utils.GenerateWithUsage(ctx, t.LLM, t.ModelName, prompts.SUMMARIZE_GRAPH_ITSELF_EN_PROMPT, summarizePrompt)

	// Emit Generation End
	eventbus.Emit(t.EventBus, string(event.EVENT_QUERY_GENERATION_END), event.QueryGenerationEndPayload{
		BasePayload: event.NewBasePayload(t.memoryGroup),
		TokenUsage:  u,
		Response:    summaryContent,
	})

	usage.Add(u)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to generate summary of graph: %w", err)
		return
	}
	if summaryContent == "" {
		err = fmt.Errorf("GraphCompletionTool: Empty summary response from LLM.")
		return
	}
	graphSummary = &summaryContent
	embedding = embeddingVectors
	return
}

func (t *GraphCompletionTool) getJapaneseGraphSummary(ctx context.Context, entityTopk int, query string, embeddingVecs *[]float32, config types.QueryConfig) (embedding *[]float32, graphSummary *string, usage types.TokenUsage, err error) {
	embeddingVectors, graphExplanation, u, err := t.getEnglishGraphExplanation(ctx, entityTopk, query, embeddingVecs, config) // 要約時点で日本語にするので、ここは英語で良い
	usage.Add(u)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to get graph explanation: %w", err)
		return
	}
	summarizePrompt := fmt.Sprintf("USER QUERY: %s\n\nKNOWLEDGE GRAPH INFORMATION:\n%s", query, *graphExplanation)

	// Emit Generation Start (Graph Summary JA)
	eventbus.Emit(t.EventBus, string(event.EVENT_QUERY_GENERATION_START), event.QueryGenerationStartPayload{
		BasePayload: event.NewBasePayload(t.memoryGroup),
		PromptName:  "SUMMARIZE_GRAPH_ITSELF_JA_PROMPT",
	})

	summaryContent, u, err := utils.GenerateWithUsage(ctx, t.LLM, t.ModelName, prompts.SUMMARIZE_GRAPH_ITSELF_JA_PROMPT, summarizePrompt)

	// Emit Generation End
	eventbus.Emit(t.EventBus, string(event.EVENT_QUERY_GENERATION_END), event.QueryGenerationEndPayload{
		BasePayload: event.NewBasePayload(t.memoryGroup),
		TokenUsage:  u,
		Response:    summaryContent,
	})

	usage.Add(u)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to generate summary of graph: %w", err)
		return
	}
	if summaryContent == "" {
		err = fmt.Errorf("GraphCompletionTool: Empty summary response from LLM.")
		return
	}
	graphSummary = &summaryContent
	embedding = embeddingVectors
	return
}

func (t *GraphCompletionTool) getEnglishGraphSummaryToAnswer(ctx context.Context, entityTopk int, query string, embeddingVecs *[]float32, config types.QueryConfig) (embedding *[]float32, graphSummary *string, usage types.TokenUsage, err error) {
	embeddingVectors, graphExplanation, u, err := t.getEnglishGraphExplanation(ctx, entityTopk, query, embeddingVecs, config)
	usage.Add(u)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to get graph explanation: %w", err)
		return
	}
	summarizePrompt := fmt.Sprintf("USER QUERY: %s\n\nKNOWLEDGE GRAPH INFORMATION:\n%s", query, *graphExplanation)

	// Emit Generation Start (Graph Summary to Answer EN)
	eventbus.Emit(t.EventBus, string(event.EVENT_QUERY_GENERATION_START), event.QueryGenerationStartPayload{
		BasePayload: event.NewBasePayload(t.memoryGroup),
		PromptName:  "SUMMARIZE_GRAPH_EXPLANATION_TO_ANSWER_EN_PROMPT",
	})

	summaryContent, u, err := utils.GenerateWithUsage(ctx, t.LLM, t.ModelName, prompts.SUMMARIZE_GRAPH_EXPLANATION_TO_ANSWER_EN_PROMPT, summarizePrompt)

	// Emit Generation End
	eventbus.Emit(t.EventBus, string(event.EVENT_QUERY_GENERATION_END), event.QueryGenerationEndPayload{
		BasePayload: event.NewBasePayload(t.memoryGroup),
		TokenUsage:  u,
		Response:    summaryContent,
	})

	usage.Add(u)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to generate summary of graph: %w", err)
		return
	}
	if summaryContent == "" {
		err = fmt.Errorf("GraphCompletionTool: Empty summary response from LLM.")
		return
	}
	graphSummary = &summaryContent
	embedding = embeddingVectors
	return
}

func (t *GraphCompletionTool) getJapaneseGraphSummaryToAnswer(ctx context.Context, entityTopk int, query string, embeddingVecs *[]float32, config types.QueryConfig) (embedding *[]float32, graphSummary *string, usage types.TokenUsage, err error) {
	embeddingVectors, graphExplanation, u, err := t.getEnglishGraphExplanation(ctx, entityTopk, query, embeddingVecs, config) // 要約時点で日本語にするので、ここは英語で良い
	usage.Add(u)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to get graph explanation: %w", err)
		return
	}
	summarizePrompt := fmt.Sprintf("USER QUERY: %s\n\nKNOWLEDGE GRAPH INFORMATION:\n%s", query, *graphExplanation)

	// Emit Generation Start (Graph Summary to Answer JA)
	eventbus.Emit(t.EventBus, string(event.EVENT_QUERY_GENERATION_START), event.QueryGenerationStartPayload{
		BasePayload: event.NewBasePayload(t.memoryGroup),
		PromptName:  "SUMMARIZE_GRAPH_EXPLANATION_TO_ANSWER_JA_PROMPT",
	})

	summaryContent, u, err := utils.GenerateWithUsage(ctx, t.LLM, t.ModelName, prompts.SUMMARIZE_GRAPH_EXPLANATION_TO_ANSWER_JA_PROMPT, summarizePrompt)

	// Emit Generation End
	eventbus.Emit(t.EventBus, string(event.EVENT_QUERY_GENERATION_END), event.QueryGenerationEndPayload{
		BasePayload: event.NewBasePayload(t.memoryGroup),
		TokenUsage:  u,
		Response:    summaryContent,
	})

	usage.Add(u)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to generate summary of graph: %w", err)
		return
	}
	if summaryContent == "" {
		err = errors.New("GraphCompletionTool: Empty summary response from LLM")
		return
	}
	graphSummary = &summaryContent
	embedding = embeddingVectors
	return
}

// getGraphSummaryCompletion は、グラフ（要約）を検索して回答を生成します。（英語で回答）
// この関数は以下の処理を行います：
//  1. ノードを検索
//  2. グラフトラバーサルでトリプルを取得
//  3. トリプルをテキストに変換
//  4. LLMでグラフの要約を生成
//  5. 要約をコンテキストとして最終的な回答を生成
//
// 引数:
//   - ctx: コンテキスト
//   - summaryTopk: Summaryテーブルを検索する際のtopk
//   - entityTopk: Entityテーブルを検索する際のtopk
//   - query: 検索クエリ
//
// 返り値:
//   - string: 回答
//   - error: エラーが発生した場合
func (t *GraphCompletionTool) getGraphSummaryCompletionEN(ctx context.Context, summaryTopk int, entityTopk int, query string, embeddingVecs *[]float32, config types.QueryConfig) (embedding *[]float32, answer *string, usage types.TokenUsage, err error) {
	// 1. 関連するSummaryを検索
	embeddingVectors, summaries, u, err := t.getSummaries(ctx, summaryTopk, query, embeddingVecs)
	usage.Add(u)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to query summaries: %w", err)
		return
	}
	if *summaries == "" {
		answer = summaries
		return
	}
	// 2. グラフの「クエリ回答用要約」を生成
	_, graphSummaryText, u, err := t.getEnglishGraphSummary(ctx, entityTopk, query, embeddingVectors, config)
	usage.Add(u)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to get graph summary: %v", err)
		return
	}
	// 3. 取得した事前要約群とグラフ要約文をコンテキストとして最終的な回答を生成
	tmpAnswer, u, err := t.answerQueryByVectorAndGraphResultEN(ctx, summaries, graphSummaryText, query)
	usage.Add(u)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to answer query by vector and graph result: %w", err)
		return
	}
	answer = tmpAnswer
	embedding = embeddingVectors
	return
}

// getGraphSummaryCompletion は、グラフ（要約）を検索して回答を生成します。（日本語で回答）
// この関数は以下の処理を行います：
//  1. ノードを検索
//  2. グラフトラバーサルでトリプルを取得
//  3. トリプルをテキストに変換
//  4. LLMでグラフの要約を生成
//  5. 要約をコンテキストとして最終的な回答を生成
//
// 引数:
//   - ctx: コンテキスト
//   - summaryTopk: Summaryテーブルを検索する際のtopk
//   - entityTopk: Entityテーブルを検索する際のtopk
//   - query: 検索クエリ
//
// 返り値:
//   - string: 回答
//   - error: エラーが発生した場合
func (t *GraphCompletionTool) getGraphSummaryCompletionJA(ctx context.Context, summaryTopk int, entityTopk int, query string, embeddingVecs *[]float32, config types.QueryConfig) (embedding *[]float32, answer *string, usage types.TokenUsage, err error) {
	// 1. 関連するSummaryを検索
	embeddingVectors, summaries, u, err := t.getSummaries(ctx, summaryTopk, query, embeddingVecs)
	usage.Add(u)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to query summaries: %w", err)
		return
	}
	if *summaries == "" {
		answer = summaries
		return
	}
	// 2. グラフの「クエリ回答用要約」を生成
	_, graphSummaryText, u, err := t.getEnglishGraphSummary(ctx, entityTopk, query, embeddingVectors, config) // 最後の回答で日本語にするので、ここは英語で良い
	usage.Add(u)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to get graph summary: %v", err)
		return
	}
	// 3. 取得した事前要約群とグラフ要約文をコンテキストとして最終的な回答を生成
	tmpAnswer, u, err := t.answerQueryByVectorAndGraphResultJA(ctx, summaries, graphSummaryText, query)
	usage.Add(u)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to answer query by vector and graph result: %w", err)
		return
	}
	answer = tmpAnswer
	embedding = embeddingVectors
	return
}

// getGraphCompletionEN は、グラフとチャンクを組み合わせて回答を生成します（英語）。
// この関数は以下の処理を行います：
//  1. ベクトル検索（チャンクとノード）
//  2. グラフトラバーサル
//  3. コンテキストを構築（チャンク + グラフ）
//  4. LLMで回答を生成
//
// 引数:
//   - ctx: コンテキスト
//   - query: 検索クエリ
//
// 返り値:
//   - string: 回答
//   - error: エラーが発生した場合
func (t *GraphCompletionTool) getGraphCompletionEN(ctx context.Context, chunkTopk int, entityTopk int, query string, embeddingVecs *[]float32, config types.QueryConfig) (embedding *[]float32, answer *string, usage types.TokenUsage, err error) {
	// 1. 関連するSummaryを検索
	embeddingVectors, chunks, u, err := t.getChunks(ctx, chunkTopk, query, embeddingVecs)
	usage.Add(u)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to query chunks: %w", err)
		return
	}
	if *chunks == "" {
		answer = chunks
		return
	}
	// 2. グラフの「クエリ回答用要約」を生成
	_, graphSummaryText, u, err := t.getEnglishGraphSummary(ctx, entityTopk, query, embeddingVectors, config)
	usage.Add(u)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to get graph summary: %v", err)
		return
	}
	// 3. 取得した事前要約群とグラフ要約文をコンテキストとして最終的な回答を生成
	tmpAnswer, u, err := t.answerQueryByVectorAndGraphResultEN(ctx, chunks, graphSummaryText, query)
	usage.Add(u)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to answer query by vector and graph result: %w", err)
		return
	}
	answer = tmpAnswer
	embedding = embeddingVectors
	return
}

// getGraphCompletionJA は、グラフとチャンクを組み合わせて回答を生成します（日本語）。
// この関数は以下の処理を行います：
//  1. ベクトル検索（チャンクとノード）
//  2. グラフトラバーサル
//  3. コンテキストを構築（チャンク + グラフ）
//  4. LLMで回答を生成
//
// 引数:
//   - ctx: コンテキスト
//   - query: 検索クエリ
//
// 返り値:
//   - string: 回答
//   - error: エラーが発生した場合
func (t *GraphCompletionTool) getGraphCompletionJA(ctx context.Context, chunkTopk int, entityTopk int, query string, embeddingVecs *[]float32, config types.QueryConfig) (embedding *[]float32, answer *string, usage types.TokenUsage, err error) {
	// 1. 関連するSummaryを検索
	embeddingVectors, chunks, u, err := t.getChunks(ctx, chunkTopk, query, embeddingVecs)
	usage.Add(u)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to query chunks: %w", err)
		return
	}
	if *chunks == "" {
		answer = chunks
		return
	}
	// 2. グラフの「クエリ回答用要約」を生成
	_, graphSummaryText, u, err := t.getEnglishGraphSummary(ctx, entityTopk, query, embeddingVectors, config) // 最後の回答で日本語にするので、ここは英語で良い
	usage.Add(u)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to get graph summary: %v", err)
		return
	}
	// 3. 取得した事前要約群とグラフ要約文をコンテキストとして最終的な回答を生成
	tmpAnswer, u, err := t.answerQueryByVectorAndGraphResultJA(ctx, chunks, graphSummaryText, query)
	usage.Add(u)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to answer query by vector and graph result: %w", err)
		return
	}
	answer = tmpAnswer
	embedding = embeddingVectors
	return
}

// ベクトル検索結果とグラフ検索結果をコンテキストとして回答を生成する（英語で回答）
func (t *GraphCompletionTool) answerQueryByVectorAndGraphResultEN(ctx context.Context, vectorResult *string, graphResult *string, query string) (answer *string, usage types.TokenUsage, err error) {
	finalUserPrompt := fmt.Sprintf("User Question: %s\n\nVector Search Results:\n%s\n\nKnowledge Graph Summary:\n%s", query, *vectorResult, *graphResult)

	// Emit Generation Start (Final Answer EN)
	eventbus.Emit(t.EventBus, string(event.EVENT_QUERY_GENERATION_START), event.QueryGenerationStartPayload{
		BasePayload: event.NewBasePayload(t.memoryGroup),
		PromptName:  "ANSWER_QUERY_WITH_HYBRID_RAG_EN_PROMPT",
	})

	answerContent, u, err := utils.GenerateWithUsage(ctx, t.LLM, t.ModelName, prompts.ANSWER_QUERY_WITH_HYBRID_RAG_EN_PROMPT, finalUserPrompt)

	// Emit Generation End
	eventbus.Emit(t.EventBus, string(event.EVENT_QUERY_GENERATION_END), event.QueryGenerationEndPayload{
		BasePayload: event.NewBasePayload(t.memoryGroup),
		TokenUsage:  u,
		Response:    answerContent,
	})

	usage.Add(u)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to generate final answer: %w", err)
		return
	}
	if answerContent == "" {
		err = errors.New("GraphCompletionTool: No final answer generated.")
		return
	}
	answer = &answerContent
	return
}

// ベクトル検索結果とグラフ検索結果をコンテキストとして回答を生成する（日本語で回答）
func (t *GraphCompletionTool) answerQueryByVectorAndGraphResultJA(ctx context.Context, vectorResult *string, graphResult *string, query string) (answer *string, usage types.TokenUsage, err error) {
	finalUserPrompt := fmt.Sprintf("User Question: %s\n\nVector Search Results:\n%s\n\nKnowledge Graph Summary:\n%s", query, *vectorResult, *graphResult)

	// Emit Generation Start (Final Answer JA)
	eventbus.Emit(t.EventBus, string(event.EVENT_QUERY_GENERATION_START), event.QueryGenerationStartPayload{
		BasePayload: event.NewBasePayload(t.memoryGroup),
		PromptName:  "ANSWER_QUERY_WITH_HYBRID_RAG_JA_PROMPT",
	})

	answerContent, u, err := utils.GenerateWithUsage(ctx, t.LLM, t.ModelName, prompts.ANSWER_QUERY_WITH_HYBRID_RAG_JA_PROMPT, finalUserPrompt)

	// Emit Generation End
	eventbus.Emit(t.EventBus, string(event.EVENT_QUERY_GENERATION_END), event.QueryGenerationEndPayload{
		BasePayload: event.NewBasePayload(t.memoryGroup),
		TokenUsage:  u,
		Response:    answerContent,
	})

	usage.Add(u)
	if err != nil {
		err = fmt.Errorf("GraphCompletionTool: Failed to generate final answer: %w", err)
		return
	}
	if answerContent == "" {
		err = errors.New("GraphCompletionTool: No final answer generated.")
		return
	}
	answer = &answerContent
	return
}

/**
 * 与えられた知識グラフトリプルから、自然な英語の説明文を構成する（英語）
 */
func GenerateNaturalEnglishGraphExplanationByTriples(triples *[]*storage.Triple, graphText *strings.Builder) *strings.Builder {
	// =================================
	// Information about word entities
	// =================================
	graphText.WriteString("# Information about word entities\n")
	doneWords := []string{}
	for _, triple := range *triples {
		// 1. Source
		if !slices.Contains(doneWords, triple.Source.ID) {
			doneWords = append(doneWords, triple.Source.ID)
			fmt.Fprintf(graphText, "- '%s' is a type of '%s'.", triple.Source.ID, triple.Source.Type)
			if len(triple.Source.Properties) > 0 {
				fmt.Fprintf(graphText, " Additional information about '%s' is as follows:\n", triple.Source.ID)
				for k, prop := range triple.Source.Properties {
					fmt.Fprintf(graphText, "    * %s: %v\n", k, prop)
				}
			} else {
				graphText.WriteString("\n")
			}
		}
		// 2. Target
		if !slices.Contains(doneWords, triple.Target.ID) {
			doneWords = append(doneWords, triple.Target.ID)
			fmt.Fprintf(graphText, "- '%s' is a type of '%s'.", triple.Target.ID, triple.Target.Type)
			if len(triple.Target.Properties) > 0 {
				fmt.Fprintf(graphText, " Additional information about '%s' is as follows:\n", triple.Target.ID)
				for k, prop := range triple.Target.Properties {
					fmt.Fprintf(graphText, "    * %s: %v\n", k, prop)
				}
			} else {
				graphText.WriteString("\n")
			}
		}
	}
	// =================================
	// Relations between word entities
	// =================================
	graphText.WriteString("\n# Relations between word entities\n")
	for _, triple := range *triples {
		fmt.Fprintf(
			graphText,
			"- '%s' and '%s' are connected by the relation '%s', where '%s' is the source (from) and '%s' is the target (to).",
			triple.Edge.SourceID,
			triple.Edge.TargetID,
			triple.Edge.Type,
			triple.Edge.SourceID,
			triple.Edge.TargetID,
		)
		if len(triple.Edge.Properties) > 0 {
			graphText.WriteString(" Additional information about their relationship is as follows:\n")
			for k, prop := range triple.Edge.Properties {
				fmt.Fprintf(graphText, "    * %s: %v\n", k, prop)
			}
		} else {
			graphText.WriteString("\n")
		}
	}
	return graphText
}

/**
 * 与えられた知識グラフトリプルから、自然な日本語の説明文を構成する（日本語）
 */
func GenerateNaturalJapaneseGraphExplanationByTriples(triples *[]*storage.Triple, graphText *strings.Builder) *strings.Builder {
	// =================================
	// 単語エンティティの情報
	// =================================
	graphText.WriteString("# 単語エンティティの情報\n")
	doneWords := []string{}
	for _, triple := range *triples {
		// 1. Source
		if !slices.Contains(doneWords, triple.Source.ID) {
			doneWords = append(doneWords, triple.Source.ID)
			fmt.Fprintf(graphText, "- 「%s」は「%s」型のエンティティです。", triple.Source.ID, triple.Source.Type)
			if len(triple.Source.Properties) > 0 {
				fmt.Fprintf(graphText, "「%s」の追加情報は以下の通りです:\n", triple.Source.ID)
				for k, prop := range triple.Source.Properties {
					fmt.Fprintf(graphText, "    * %s: %v\n", k, prop)
				}
			} else {
				graphText.WriteString("\n")
			}
		}
		// 2. Target
		if !slices.Contains(doneWords, triple.Target.ID) {
			doneWords = append(doneWords, triple.Target.ID)
			fmt.Fprintf(graphText, "- 「%s」は「%s」型のエンティティです。", triple.Target.ID, triple.Target.Type)
			if len(triple.Target.Properties) > 0 {
				fmt.Fprintf(graphText, "「%s」の追加情報は以下の通りです:\n", triple.Target.ID)
				for k, prop := range triple.Target.Properties {
					fmt.Fprintf(graphText, "    * %s: %v\n", k, prop)
				}
			} else {
				graphText.WriteString("\n")
			}
		}
	}
	// =================================
	// 単語エンティティ間の関係
	// =================================
	graphText.WriteString("\n# 単語エンティティ間の関係\n")
	for _, triple := range *triples {
		fmt.Fprintf(
			graphText,
			"- 「%s」と「%s」は「%s」という関係で結ばれています。「%s」が始点、「%s」が終点です。",
			triple.Edge.SourceID,
			triple.Edge.TargetID,
			triple.Edge.Type,
			triple.Edge.SourceID,
			triple.Edge.TargetID,
		)
		if len(triple.Edge.Properties) > 0 {
			graphText.WriteString("この関係の追加情報は以下の通りです:\n")
			for k, prop := range triple.Edge.Properties {
				fmt.Fprintf(graphText, "    * %s: %v\n", k, prop)
			}
		} else {
			graphText.WriteString("\n")
		}
	}
	return graphText
}

// expDecay は、指数減衰を計算するヘルパー関数です。
// x: -λ × Δt の値（負の値が渡される）
func expDecay(x float64) float64 {
	return math.Exp(x)
}
