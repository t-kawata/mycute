// Package metacognition は、メタ認知処理に関するタスクを提供します。
package metacognition

import (
	"context"
	"fmt"
	"time"

	"github.com/cloudwego/eino/components/model"
	appconfig "github.com/t-kawata/mycute/config"
	"github.com/t-kawata/mycute/lib/common"
	"github.com/t-kawata/mycute/lib/eventbus"
	"github.com/t-kawata/mycute/pkg/cuber/event"
	"github.com/t-kawata/mycute/pkg/cuber/storage"
	"github.com/t-kawata/mycute/pkg/cuber/types"
	"github.com/t-kawata/mycute/pkg/cuber/utils"
	"go.uber.org/zap"
)

// MetabolismTask は、知識グラフの代謝（Pruning & Forgetting）を実行するタスクです。
// Memify パイプラインの末尾で実行され、以下の処理を行います：
//  1. Temporal Decay に基づく Thickness 計算
//  2. Thickness が閾値を下回るエッジの削除（Pruning）
//  3. MDL Principle に基づく孤立ノードの削除（Forgetting）
type MetabolismTask struct {
	VectorStorage           storage.VectorStorage
	GraphStorage            storage.GraphStorage
	Embedder                storage.Embedder
	LLM                     model.ToolCallingChatModel
	ModelName               string
	MemoryGroup             string
	ConflictResolutionStage int
	IsEn                    bool
	EventBus                *eventbus.EventBus
	Logger                  *zap.Logger
}

// NewMetabolismTask は、新しい MetabolismTask を作成します。
func NewMetabolismTask(
	vectorStorage storage.VectorStorage,
	graphStorage storage.GraphStorage,
	embedder storage.Embedder,
	llm model.ToolCallingChatModel,
	modelName string,
	memoryGroup string,
	conflictResolutionStage int,
	isEn bool,
	eventBus *eventbus.EventBus,
	logger *zap.Logger,
) *MetabolismTask {
	return &MetabolismTask{
		VectorStorage:           vectorStorage,
		GraphStorage:            graphStorage,
		Embedder:                embedder,
		LLM:                     llm,
		ModelName:               modelName,
		MemoryGroup:             memoryGroup,
		ConflictResolutionStage: conflictResolutionStage,
		IsEn:                    isEn,
		EventBus:                eventBus,
		Logger:                  logger,
	}
}

// Run は、代謝処理を実行します。
// 返り値:
//   - prunedEdges: 削除されたエッジ数
//   - deletedNodes: 削除されたノード数
//   - usage: トークン使用量（Embedding および LLM 用）
//   - error: エラー
func (t *MetabolismTask) Run(ctx context.Context) (prunedEdgesCount int, deletedNodesCount int, usage types.TokenUsage, err error) {
	utils.LogInfo(t.Logger, "MetabolismTask: Starting metabolism process", zap.String("memory_group", t.MemoryGroup))

	// ========================================
	// 1. MemoryGroup 設定の取得
	// ========================================
	memoryGroupConfig, err := t.GraphStorage.GetMemoryGroupConfig(ctx, t.MemoryGroup)
	if err != nil {
		utils.LogWarn(t.Logger, "MetabolismTask: Failed to get MemoryGroupConfig, using defaults", zap.Error(err))
	}

	// 設定値の決定（デフォルト値適用）
	halfLifeDays := appconfig.DEFAULT_HALF_LIFE_DAYS
	pruneThreshold := appconfig.DEFAULT_PRUNE_THRESHOLD
	minSurvivalProtectionHours := appconfig.DEFAULT_MIN_SURVIVAL_PROTECTION_HOURS
	mdlKNeighbors := appconfig.MDL_K_NEIGHBORS

	if memoryGroupConfig != nil {
		if memoryGroupConfig.HalfLifeDays > 0 {
			halfLifeDays = memoryGroupConfig.HalfLifeDays
		}
		if memoryGroupConfig.PruneThreshold > 0 {
			pruneThreshold = memoryGroupConfig.PruneThreshold
		}
		if memoryGroupConfig.MinSurvivalProtectionHours > 0 {
			minSurvivalProtectionHours = memoryGroupConfig.MinSurvivalProtectionHours
		}
		if memoryGroupConfig.MdlKNeighbors > 0 {
			mdlKNeighbors = memoryGroupConfig.MdlKNeighbors
		}
	}

	utils.LogDebug(t.Logger, "MetabolismTask: Config loaded",
		zap.Float64("half_life_days", halfLifeDays),
		zap.Float64("prune_threshold", pruneThreshold),
		zap.Float64("min_survival_hours", minSurvivalProtectionHours),
		zap.Int("mdl_k_neighbors", mdlKNeighbors))

	// ========================================
	// 2. エッジの削除（Pruning）
	// ========================================
	prunedEdgesCount, err = t.pruneEdges(ctx, halfLifeDays, pruneThreshold, minSurvivalProtectionHours)
	if err != nil {
		utils.LogWarn(t.Logger, "MetabolismTask: Edge pruning failed", zap.Error(err))
		return prunedEdgesCount, 0, usage, err
	}

	// ========================================
	// 3. MDL ベースのノード削除（Forgetting）
	// ========================================
	// 3-A. 完全孤立ノードの削除（MDL 判定なし）
	orphanDeletedCount, err := t.deleteOrphanedNodes(ctx, minSurvivalProtectionHours)
	if err != nil {
		utils.LogWarn(t.Logger, "MetabolismTask: deleteOrphanedNodes failed", zap.Error(err))
	}

	// 3-B. 弱接続ノードの削除（MDL 判定あり）
	weakDeletedCount, mdlUsage, err := t.deleteWeaklyConnectedNodes(
		ctx,
		pruneThreshold,
		minSurvivalProtectionHours,
		mdlKNeighbors,
	)
	usage.Add(mdlUsage)
	if err != nil {
		utils.LogWarn(t.Logger, "MetabolismTask: deleteWeaklyConnectedNodes failed", zap.Error(err))
	}

	deletedNodesCount = orphanDeletedCount + weakDeletedCount

	// ========================================
	// 4. 矛盾解決に基づくデータ洗練（独立した矛盾解決）
	// ========================================
	if t.ConflictResolutionStage >= 1 {
		refinedCount, refineUsage, err := t.refineConflicts(ctx)
		usage.Add(refineUsage)
		if err != nil {
			utils.LogWarn(t.Logger, "MetabolismTask: Conflict refinement failed", zap.Error(err))
		} else {
			prunedEdgesCount += refinedCount
		}
	}

	utils.LogInfo(t.Logger, "MetabolismTask: Metabolism completed",
		zap.Int("pruned_edges", prunedEdgesCount),
		zap.Int("deleted_nodes", deletedNodesCount))

	return prunedEdgesCount, deletedNodesCount, usage, nil
}

// pruneEdges は、Thickness が閾値を下回るエッジを削除します。
// ページング処理により、大規模グラフでもメモリ消費を抑えて処理できます。
func (t *MetabolismTask) pruneEdges(
	ctx context.Context,
	halfLifeDays float64,
	pruneThreshold float64,
	minSurvivalProtectionHours float64,
) (int, error) {
	// MaxUnix を取得
	maxUnix, err := t.GraphStorage.GetMaxUnix(ctx, t.MemoryGroup)
	if err != nil {
		return 0, err
	}
	if maxUnix == 0 {
		return 0, nil // エッジがない
	}

	// λ（減衰定数）を計算
	lambda := utils.CalculateLambda(halfLifeDays)

	// 最低生存保護期間（ミリ秒）
	minSurvivalMillis := utils.HoursToMillis(minSurvivalProtectionHours)
	nowUnix := common.GetNow().UnixMilli()

	// ページング処理
	pageSize := appconfig.METABOLISM_PAGE_SIZE
	offset := 0
	prunedCount := 0

	for {
		// ソースノードIDをページング取得
		nodeIDs, err := t.GraphStorage.GetSourceNodeIDs(ctx, t.MemoryGroup, offset, pageSize)
		if err != nil {
			return prunedCount, fmt.Errorf("failed to get source node IDs: %w", err)
		}
		if len(nodeIDs) == 0 {
			break // 最終ページ
		}

		// 該当ノードのトリプルを取得
		triples, err := t.GraphStorage.GetTriplesBySourceIDs(ctx, nodeIDs, t.MemoryGroup)
		if err != nil {
			return prunedCount, fmt.Errorf("failed to get triples: %w", err)
		}

		// 各エッジを評価して削除
		for _, triple := range triples {
			edge := triple.Edge

			// 1. 最低生存保護期間チェック
			ageMillis := float64(nowUnix - edge.Unix)
			if ageMillis < minSurvivalMillis {
				continue // まだ保護期間中
			}

			// 2. Thickness 計算
			thickness := utils.CalculateThickness(edge.Weight, edge.Confidence, edge.Unix, maxUnix, lambda)

			// 3. 閾値チェック
			if thickness < pruneThreshold {
				// エッジを削除
				if err := t.GraphStorage.DeleteEdge(ctx, edge.SourceID, edge.Type, edge.TargetID, t.MemoryGroup); err != nil {
					utils.LogWarn(t.Logger, "MetabolismTask: Failed to delete edge",
						zap.String("source", edge.SourceID),
						zap.String("target", edge.TargetID),
						zap.Error(err))
				} else {
					prunedCount++
					utils.LogDebug(t.Logger, "MetabolismTask: Pruned edge",
						zap.String("source", edge.SourceID),
						zap.String("target", edge.TargetID),
						zap.Float64("thickness", thickness))
				}
			}
		}

		// 次のページへ（シンプルなページングなのでオフセットをページサイズ分進める）
		offset += pageSize
	}

	return prunedCount, nil
}

// deleteOrphanedNodes は、完全に孤立したノード（エッジが 0 本）を削除します。
// 保護期間経過後の孤立ノードは、MDL 判定なしに即座に削除されます。
// MDL 判定が必要なのは「弱いエッジを持つノード」であり、これは deleteWeaklyConnectedNodes で処理します。
func (t *MetabolismTask) deleteOrphanedNodes(
	ctx context.Context,
	minSurvivalProtectionHours float64,
) (int, error) {
	gracePeriod := time.Duration(minSurvivalProtectionHours) * time.Hour
	orphanedNodes, err := t.GraphStorage.GetOrphanNodes(ctx, t.MemoryGroup, gracePeriod)
	if err != nil {
		return 0, err
	}

	deletedCount := 0
	for _, node := range orphanedNodes {
		if err := t.GraphStorage.DeleteNode(ctx, node.ID, t.MemoryGroup); err != nil {
			utils.LogWarn(t.Logger, "MetabolismTask: Failed to delete orphaned node",
				zap.String("node_id", node.ID),
				zap.Error(err))
		} else {
			deletedCount++
			utils.LogDebug(t.Logger, "MetabolismTask: Deleted orphaned node (no edges)",
				zap.String("node_id", node.ID))
		}
	}

	return deletedCount, nil
}

// deleteWeaklyConnectedNodes は、MDL Principle に基づいて「弱い接続のみを持つノード」を削除します。
// 全てのエッジの Thickness が閾値以下であり、かつ近傍ノードで情報を復元可能と判断されるノードを削除します。
func (t *MetabolismTask) deleteWeaklyConnectedNodes(
	ctx context.Context,
	pruneThreshold float64,
	minSurvivalProtectionHours float64,
	mdlKNeighbors int,
) (int, types.TokenUsage, error) {
	var usage types.TokenUsage

	gracePeriod := time.Duration(minSurvivalProtectionHours) * time.Hour
	weakNodes, err := t.GraphStorage.GetWeaklyConnectedNodes(ctx, t.MemoryGroup, pruneThreshold, gracePeriod)
	if err != nil {
		return 0, usage, err
	}

	if len(weakNodes) == 0 {
		return 0, usage, nil
	}

	deletedCount := 0
	for _, node := range weakNodes {
		// 1. ノードのテキスト表現を取得
		nodeText := node.ID
		if text, ok := node.Properties["text"].(string); ok && text != "" {
			nodeText = text
		}

		// 2. ノードのベクトルを生成
		nodeVec, embUsage, embErr := t.Embedder.EmbedQuery(ctx, nodeText)
		usage.Add(embUsage)
		if embErr != nil {
			utils.LogWarn(t.Logger, "MetabolismTask: Failed to embed node for MDL check", zap.Error(embErr))
			continue
		}

		// 3. 近傍ノードを検索
		neighbors, searchErr := t.VectorStorage.Query(ctx, types.TABLE_NAME_GRAPH_NODE, nodeVec, mdlKNeighbors+1, t.MemoryGroup)
		if searchErr != nil {
			utils.LogWarn(t.Logger, "MetabolismTask: Failed to search neighbors", zap.Error(searchErr))
			continue
		}

		// 4. 復元困難度を算出（最も近い近傍との類似度から計算）
		restorationDifficulty := 1.0 // デフォルト: 最大困難度（削除しない）
		for _, neighbor := range neighbors {
			if neighbor.ID == node.ID {
				continue // 自分自身を除外
			}
			// Distance は距離なので、類似度 = 1 - distance（distance が 0-1 の場合）
			// または Distance が小さいほど復元が容易
			similarity := 1.0 - neighbor.Distance
			if similarity > 0 {
				restorationDifficulty = 1.0 - similarity // 復元困難度 = 1 - 類似度
				break                                    // 最も近い近傍のみ使用
			}
		}

		// 5. MDL 判定：削除ベネフィット > 復元困難度 なら削除
		if appconfig.MDL_REDUCTION_BENEFIT > restorationDifficulty {
			// ノードに接続しているエッジも一緒に削除（DETACH DELETE 相当）
			if err := t.GraphStorage.DeleteNode(ctx, node.ID, t.MemoryGroup); err != nil {
				utils.LogWarn(t.Logger, "MetabolismTask: Failed to delete weakly connected node",
					zap.String("node_id", node.ID),
					zap.Error(err))
			} else {
				deletedCount++
				utils.LogDebug(t.Logger, "MetabolismTask: Deleted weakly connected node via MDL",
					zap.String("node_id", node.ID),
					zap.Float64("restoration_difficulty", restorationDifficulty),
					zap.Float64("mdl_benefit", appconfig.MDL_REDUCTION_BENEFIT))
			}
		}
	}

	return deletedCount, usage, nil
}

// refineConflicts は、Stage 1/2 の各排他ルール等に基づき、矛盾したデータを物理削除します。
// ページング＋オーバーラップ処理により、境界での矛盾見逃しを防ぎます。
func (t *MetabolismTask) refineConflicts(ctx context.Context) (int, types.TokenUsage, error) {
	var usage types.TokenUsage
	totalRefined := 0

	// ページング設定
	pageSize := appconfig.METABOLISM_PAGE_SIZE
	overlapSize := appconfig.METABOLISM_OVERLAP_SIZE
	offset := 0

	// 削除済みエッジを追跡（オーバーラップによる重複削除防止）
	deletedEdges := make(map[string]bool)
	edgeKey := func(sourceID, edgeType, targetID string) string {
		return sourceID + "|" + edgeType + "|" + targetID
	}

	for {
		// ソースノードIDをページング取得
		nodeIDs, err := t.GraphStorage.GetSourceNodeIDs(ctx, t.MemoryGroup, offset, pageSize)
		if err != nil {
			return totalRefined, usage, fmt.Errorf("failed to get source node IDs: %w", err)
		}
		if len(nodeIDs) == 0 {
			break // 最終ページ
		}

		// 該当ノードのトリプルを取得
		triples, err := t.GraphStorage.GetTriplesBySourceIDs(ctx, nodeIDs, t.MemoryGroup)
		if err != nil {
			return totalRefined, usage, fmt.Errorf("failed to get triples: %w", err)
		}
		if len(triples) == 0 {
			// 次のページへ（オーバーラップ付きなのでオフセットを (pageSize - overlapSize) 分進める）
			offset += pageSize - overlapSize
			continue
		}

		// スコア付きトリプルに変換
		scoredTriples := make([]utils.ScoredTriple, 0, len(triples))
		for _, triple := range triples {
			thickness := triple.Edge.Weight * triple.Edge.Confidence
			scoredTriples = append(scoredTriples, utils.ScoredTriple{
				Triple:    triple,
				Thickness: thickness,
			})
		}

		// Stage 1: 決定論的解決
		stage1BeforeTriplesCount := len(scoredTriples)
		eventbus.Emit(t.EventBus, string(event.EVENT_INFO_CONFLICT_RESOLUTION_1_START), event.InfoConflictResolution1StartPayload{
			BasePayload:        event.NewBasePayload(t.MemoryGroup),
			BeforeTriplesCount: stage1BeforeTriplesCount,
		})

		resolved, discarded1, remainingConflicts := utils.Stage1ConflictResolution(scoredTriples, t.Logger, t.IsEn)
		scoredTriples = resolved

		// Emit conflict discarded events for Stage 1
		for _, st := range discarded1 {
			eventbus.Emit(t.EventBus, string(event.EVENT_INFO_CONFLICT_DISCARDED), event.InfoConflictDiscardedPayload{
				BasePayload:  event.NewBasePayload(t.MemoryGroup),
				SourceID:     st.Triple.Edge.SourceID,
				RelationType: st.Triple.Edge.Type,
				TargetID:     st.Triple.Edge.TargetID,
				Stage:        1,
				Reason:       st.Reason,
			})
		}

		stage1AfterTriplesCount := len(scoredTriples)
		eventbus.Emit(t.EventBus, string(event.EVENT_INFO_CONFLICT_RESOLUTION_1_END), event.InfoConflictResolution1EndPayload{
			BasePayload:        event.NewBasePayload(t.MemoryGroup),
			BeforeTriplesCount: stage1BeforeTriplesCount,
			AfterTriplesCount:  stage1AfterTriplesCount,
		})

		// Stage 2: LLM による仲裁 (Stage 2 有効時)
		var discarded2 []utils.DiscardedTriple
		if t.ConflictResolutionStage >= 2 && len(remainingConflicts) > 0 && t.LLM != nil {
			eventbus.Emit(t.EventBus, string(event.EVENT_INFO_CONFLICT_RESOLUTION_2_START), event.InfoConflictResolution2StartPayload{
				BasePayload:        event.NewBasePayload(t.MemoryGroup),
				BeforeTriplesCount: stage1AfterTriplesCount,
			})

			d2, usage2, err := utils.Stage2ConflictResolution(ctx, t.LLM, t.ModelName, &scoredTriples, remainingConflicts, t.IsEn, t.Logger)
			usage.Add(usage2)
			if err != nil {
				utils.LogWarn(t.Logger, "MetabolismTask: Stage2 conflict resolution failed", zap.Error(err))
			} else {
				discarded2 = d2

				// Emit conflict discarded events for Stage 2
				for _, st := range discarded2 {
					eventbus.Emit(t.EventBus, string(event.EVENT_INFO_CONFLICT_DISCARDED), event.InfoConflictDiscardedPayload{
						BasePayload:  event.NewBasePayload(t.MemoryGroup),
						SourceID:     st.Triple.Edge.SourceID,
						RelationType: st.Triple.Edge.Type,
						TargetID:     st.Triple.Edge.TargetID,
						Stage:        2,
						Reason:       st.Reason,
					})
				}
			}

			stage2AfterTriplesCount := len(scoredTriples)
			eventbus.Emit(t.EventBus, string(event.EVENT_INFO_CONFLICT_RESOLUTION_2_END), event.InfoConflictResolution2EndPayload{
				BasePayload:        event.NewBasePayload(t.MemoryGroup),
				BeforeTriplesCount: stage1AfterTriplesCount,
				AfterTriplesCount:  stage2AfterTriplesCount,
			})
		}

		// 発見された discarded エッジを物理削除（重複チェック付き）
		allDiscarded := append(discarded1, discarded2...)
		for _, st := range allDiscarded {
			key := edgeKey(st.Triple.Edge.SourceID, st.Triple.Edge.Type, st.Triple.Edge.TargetID)
			if deletedEdges[key] {
				continue // 既に削除済み
			}
			err := t.GraphStorage.DeleteEdge(ctx, st.Triple.Edge.SourceID, st.Triple.Edge.Type, st.Triple.Edge.TargetID, t.MemoryGroup)
			if err != nil {
				utils.LogWarn(t.Logger, "MetabolismTask: Failed to delete conflicting edge",
					zap.String("source", st.Triple.Edge.SourceID),
					zap.String("edge_type", st.Triple.Edge.Type),
					zap.String("target", st.Triple.Edge.TargetID),
					zap.Error(err))
			} else {
				deletedEdges[key] = true
				totalRefined++
				utils.LogDebug(t.Logger, "MetabolismTask: Physically deleted conflicting edge",
					zap.String("source", st.Triple.Edge.SourceID),
					zap.String("edge_type", st.Triple.Edge.Type),
					zap.String("target", st.Triple.Edge.TargetID))
			}
		}

		// 次のページへ（オーバーラップ付きなのでオフセットを (pageSize - overlapSize) 分進める）
		offset += pageSize - overlapSize
	}

	return totalRefined, usage, nil
}
