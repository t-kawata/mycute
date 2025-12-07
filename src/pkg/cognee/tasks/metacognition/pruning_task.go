package metacognition

import (
	"context"
	"fmt"
	"time"

	"mycute/pkg/cognee/storage"
)

// PruningTask は、孤立したノード（Orphan Nodes）を削除するタスクです。
// 孤立ノードとは、エッジを持たないノードのことを指します。
// ただし、作成直後のノードが誤って削除されないよう、GracePeriod（猶予期間）を設けます。
type PruningTask struct {
	GraphStorage storage.GraphStorage
	GroupID      string
	GracePeriod  time.Duration // 削除猶予期間
}

// NewPruningTask は、新しいPruningTaskを作成します。
func NewPruningTask(
	graphStorage storage.GraphStorage,
	groupID string,
	gracePeriodMinutes int,
) *PruningTask {
	return &PruningTask{
		GraphStorage: graphStorage,
		GroupID:      groupID,
		GracePeriod:  time.Duration(gracePeriodMinutes) * time.Minute,
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
	fmt.Printf("PruningTask: Starting pruning for group %s (GracePeriod: %v)\n", t.GroupID, t.GracePeriod)

	// ========================================
	// 1クエリで全孤立ノードを取得
	// ========================================
	// GetOrphanNodesは、KuzuDB側で以下を実行:
	// - エッジを持たないノードを検出
	// - GracePeriod内のノードを除外
	orphans, err := t.GraphStorage.GetOrphanNodes(ctx, t.GroupID, t.GracePeriod)
	if err != nil {
		return fmt.Errorf("PruningTask: failed to get orphan nodes: %w", err)
	}

	if len(orphans) == 0 {
		fmt.Println("PruningTask: No orphan nodes found")
		return nil
	}

	fmt.Printf("PruningTask: Found %d orphan nodes to delete\n", len(orphans))

	// ========================================
	// 孤立ノードを削除
	// ========================================
	deletedCount := 0
	failedCount := 0

	for _, node := range orphans {
		if err := t.GraphStorage.DeleteNode(ctx, node.ID, t.GroupID); err != nil {
			fmt.Printf("PruningTask: Warning - failed to delete node %s (type: %s): %v\n", node.ID, node.Type, err)
			failedCount++
			continue
		}
		deletedCount++
	}

	fmt.Printf("PruningTask: Completed. Deleted %d nodes, failed %d nodes\n", deletedCount, failedCount)
	return nil
}
