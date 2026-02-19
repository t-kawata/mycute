package metacognition

import (
	"context"
	"fmt"
	"time"

	"github.com/t-kawata/mycute/pkg/cuber/storage"
	"github.com/t-kawata/mycute/pkg/cuber/utils"
	"go.uber.org/zap"
)

// PruningTask は、孤立したノード（Orphan Nodes）を削除するタスクです。
// 孤立ノードとは、エッジを持たないノードのことを指します。
// ただし、作成直後のノードが誤って削除されないよう、GracePeriod（猶予期間）を設けます。
type PruningTask struct {
	GraphStorage storage.GraphStorage
	MemoryGroup  string
	GracePeriod  time.Duration // 削除猶予期間
	Logger       *zap.Logger
}

// NewPruningTask は、新しいPruningTaskを作成します。
func NewPruningTask(
	graphStorage storage.GraphStorage,
	memoryGroup string,
	gracePeriodMinutes int,
	l *zap.Logger,
) *PruningTask {
	return &PruningTask{
		GraphStorage: graphStorage,
		MemoryGroup:  memoryGroup,
		GracePeriod:  time.Duration(gracePeriodMinutes) * time.Minute,
		Logger:       l,
	}
}

// PruneOrphans は、孤立ノードを特定し、削除します。
//
// Phase-09最適化:
//   - GetOrphanNodesを使用して1クエリで孤立ノードを取得
//   - N+1問題を解消
//   - 処理時間を大幅に短縮
//
// 処理フロー:
//  1. GetOrphanNodesで孤立ノードを取得（GracePeriod考慮済み）
//  2. 取得したノードを順次削除
func (t *PruningTask) PruneOrphans(ctx context.Context) error {
	utils.LogDebug(t.Logger, "PruningTask: Starting pruning", zap.String("group", t.MemoryGroup), zap.Duration("grace_period", t.GracePeriod))

	// ========================================
	// 1クエリで全孤立ノードを取得
	// ========================================
	// GetOrphanNodesは、LadybugDB側で以下を実行:
	// - エッジを持たないノードを検出
	// - GracePeriod内のノードを除外
	orphans, err := t.GraphStorage.GetOrphanNodes(ctx, t.MemoryGroup, t.GracePeriod)
	if err != nil {
		return fmt.Errorf("PruningTask: failed to get orphan nodes: %w", err)
	}

	if len(orphans) == 0 {
		utils.LogDebug(t.Logger, "PruningTask: No orphan nodes found")
		return nil
	}

	utils.LogDebug(t.Logger, "PruningTask: Found orphans", zap.Int("count", len(orphans)))

	// ========================================
	// 孤立ノードを削除
	// ========================================
	deletedCount := 0
	failedCount := 0

	for _, node := range orphans {
		if err := t.GraphStorage.DeleteNode(ctx, node.ID, t.MemoryGroup); err != nil {
			utils.LogWarn(t.Logger, "PruningTask: Failed to delete node", zap.String("node_id", node.ID), zap.String("type", node.Type), zap.Error(err))
			failedCount++
			continue
		}
		deletedCount++
	}

	utils.LogDebug(t.Logger, "PruningTask: Completed", zap.Int("deleted", deletedCount), zap.Int("failed", failedCount))
	return nil
}
