# フェーズ 28: MDL Principle の正しい実装 ― 「弱いつながりのノード」の選別と削除

このディレクティブは、フェーズ 27 で導入された MDL (Minimum Description Length) Principle に基づくノード削除ロジックの設計ミスを是正し、本来の目的を正しく果たすための修正指針を詳細に記述します。

---

## 1. 問題の背景と本質

### 1.1 現状の実装の何が間違っているか

フェーズ 27 のドキュメント（DIRECTONS-PHASE-27.md）では、MDL Principle の目的を以下のように定義していました：

> **L114-L115**: 「**一次フィルタリング**: LadybugDBのインデックスを利用し、「一定期間アクセスがない」「**次数（エッジ数）が極めて低い**」「**太さが閾値以下のエッジのみを持つ**」といった、**削除候補ノード**を高速にリストアップします。」

しかし、現在の `metabolism_task.go` における `deleteOrphanedNodes` 関数の実装は：

1.  `GetOrphanNodes` を呼び出し、**「エッジが完全に 0 本のノード」のみ**を取得している。
2.  その「完全な孤立ノード」に対してのみ MDL 判定を実行している。

これは、ドキュメントの意図と根本的に乖離しています。

### 1.2 なぜこれが問題なのか

| 観点 | 現状の問題点 |
|---|---|
| **MDL の無駄遣い** | 完全に孤立したノード（エッジが 0 本）に対して MDL 判定を行うことは、計算コストの無駄です。孤立ノードは知識グラフの構造的価値を持たないため、保護期間経過後は MDL 判定なしに削除して良いはずです（唯一の例外：特異点保護。これは後述）。 |
| **本来の目的の不達成** | MDL で評価すべきは「**つながりはあるが、そのつながりが全て弱い（Thickness が低い）ノード**」です。これらのノードは、近傍のノードで情報を代替できるなら削除することで、知識グラフの「密度」を高められます。 |
| **2 段階処理の非効率** | 現状では、`pruneEdges` でエッジを削除 → 次回の `Memify` 実行時に `deleteOrphanedNodes` で孤立ノードを削除、という 2 段階が必要です。本来は 1 回の処理で「弱いノード」を評価・削除できるべきです。 |

### 1.3 本フェーズの目標

1.  **「弱いつながりのノード」を MDL 判定の対象とする**: エッジが存在するが、全てのエッジの Thickness が閾値を下回るノードを抽出し、MDL 判定にかける。
2.  **完全な孤立ノードは MDL 判定をスキップ**: 保護期間経過後の完全孤立ノードは、無条件で削除する（ただし、特異点保護オプションは検討）。
3.  **ストレージ層の新 API 追加**: 「弱いつながりのノード」を効率的に抽出するための新しいクエリを実装する。

---

## 2. 実装指示

### 2.1 ストレージインターフェースの拡張：`GetWeaklyConnectedNodes` の追加

「エッジは存在するが、全てのエッジの Thickness が閾値以下」であるノードを抽出する新しい API を追加します。

*   **対象ファイル**: `src/pkg/cuber/storage/interfaces.go`

*   **変更理由**: `GetOrphanNodes` は「エッジ 0 本」のノードのみを返しますが、MDL 判定には「弱いエッジのみを持つノード」が必要です。この新しいメソッドは、その候補ノードを効率的にリストアップします。

*   **実装コード**:
```go
// GetWeaklyConnectedNodes は、接続エッジの全てが「弱い」と判断されるノードを取得します。
// MDL Principle に基づくノード削除の候補を抽出する目的で使用されます。
//
// 引数:
//   - ctx: コンテキスト
//   - memoryGroup: メモリーグループ
//   - thicknessThreshold: Thickness 閾値（Weight × Confidence がこの値以下のエッジを「弱い」と判定）
//   - gracePeriod: この期間内に作成されたノードは除外（誤削除防止）
//
// 戻り値:
//   - []*Node: 「弱い接続のみを持つ」ノードのスライス
//   - error: エラー
//
// 対象ノードの条件:
//   1. 1 本以上のエッジを持つ（完全孤立ではない）
//   2. 接続している全てのエッジの Thickness (= Weight × Confidence) が thicknessThreshold 以下
//   3. ノードの created_at が gracePeriod より古い
GetWeaklyConnectedNodes(ctx context.Context, memoryGroup string, thicknessThreshold float64, gracePeriod time.Duration) ([]*Node, error)
```

---

### 2.2 LadybugDB ストレージ実装：`GetWeaklyConnectedNodes` の実装

*   **対象ファイル**: `src/pkg/cuber/db/ladybugdb/ladybugdb_storage.go`

*   **変更理由**: Cypher クエリを使用して、全エッジの Thickness が閾値以下であるノードを効率的に抽出します。

*   **実装コード**:
```go
// GetWeaklyConnectedNodes は、全エッジが弱い（Thickness ≤ threshold）ノードを取得します。
func (s *LadybugDBStorage) GetWeaklyConnectedNodes(
	ctx context.Context,
	memoryGroup string,
	thicknessThreshold float64,
	gracePeriod time.Duration,
) ([]*storage.Node, error) {
	// 猶予期間を計算
	cutoffTime := time.Now().Add(-gracePeriod).Format(time.RFC3339)

	// Cypher クエリ：
	// 1. 指定 memoryGroup のノードを全て取得
	// 2. そのノードに接続する全エッジの Thickness (weight × confidence) を計算
	// 3. 全エッジの Thickness が閾値以下であり、かつエッジが1本以上存在するノードのみを返す
	// 4. 猶予期間内に作成されたノードは除外
	query := fmt.Sprintf(`
		MATCH (n:%s {memory_group: '%s'})
		WHERE n.properties CONTAINS '"created_at":"'
		  AND timestamp(parse_json_get(n.properties, 'created_at')) < timestamp('%s')
		WITH n
		OPTIONAL MATCH (n)-[r:%s {memory_group: '%s'}]-()
		WITH n,
		     collect(r) AS edges,
		     count(r) AS edgeCount,
		     max(CASE WHEN r IS NOT NULL THEN r.weight * r.confidence ELSE 0.0 END) AS maxThickness
		WHERE edgeCount > 0 AND maxThickness <= %f
		RETURN n.id, n.type, n.properties
	`,
		types.TABLE_NAME_GRAPH_NODE,
		escapeString(memoryGroup),
		cutoffTime,
		types.TABLE_NAME_GRAPH_EDGE,
		escapeString(memoryGroup),
		thicknessThreshold,
	)

	result, err := s.getConn(ctx).Query(query)
	if err != nil {
		return nil, fmt.Errorf("GetWeaklyConnectedNodes query failed: %w", err)
	}
	defer result.Close()

	var nodes []*storage.Node
	for result.HasNext() {
		row, err := result.Next()
		if err != nil {
			return nil, err
		}

		node := &storage.Node{MemoryGroup: memoryGroup}
		if v, _ := row.GetValue(0); v != nil {
			tmpID := getString(v)
			node.ID = utils.GetNameStrByGraphNodeID(tmpID)
		}
		if v, _ := row.GetValue(1); v != nil {
			node.Type = getString(v)
		}
		if v, _ := row.GetValue(2); v != nil {
			node.Properties = parseJSONProperties(getString(v))
		}
		nodes = append(nodes, node)
		row.Close()
	}

	return nodes, nil
}
```

> [!NOTE]
> **Cypher クエリの解説**
> - `OPTIONAL MATCH` を使用することで、エッジが 0 本のノードも一時的に取得しますが、`WHERE edgeCount > 0` で孤立ノードを除外しています。
> - `max(r.weight * r.confidence)` で「最も強いエッジの Thickness」を計算し、それすらも閾値以下であれば「全エッジが弱い」と判定します。

---

### 2.3 MetabolismTask の修正：`deleteOrphanedNodes` の分割と再設計

現在の `deleteOrphanedNodes` を以下の 2 つのメソッドに分割します：

1.  **`deleteOrphanedNodes`**: 完全孤立ノード（エッジ 0 本）を削除（MDL 判定なし）
2.  **`deleteWeaklyConnectedNodes`**: 弱接続ノードを MDL 判定にかけて削除

*   **対象ファイル**: `src/pkg/cuber/tasks/metacognition/metabolism_task.go`

#### 2.3.1 新しい `deleteOrphanedNodes`（MDL 判定なし）

*   **変更理由**: 完全孤立ノードは知識グラフにおける構造的価値を持たないため、保護期間経過後は即座に削除して問題ありません。MDL 判定は計算コストがかかるため、不要な場合はスキップすべきです。

*   **実装コード**:
```go
// deleteOrphanedNodes は、完全に孤立したノード（エッジが 0 本）を削除します。
// 保護期間経過後の孤立ノードは、MDL 判定なしに即座に削除されます。
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
```

#### 2.3.2 新しい `deleteWeaklyConnectedNodes`（MDL 判定あり）

*   **変更理由**: これが MDL Principle の正しい適用対象です。「つながりはあるが弱い」ノードに対して、近傍ベクトル類似度による復元困難度を算出し、削除可否を判定します。

*   **実装コード**:
```go
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
			similarity := 1.0 - neighbor.Distance
			if similarity > 0 {
				restorationDifficulty = 1.0 - similarity
				break // 最も近い近傍のみ使用
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
```

---

### 2.4 MetabolismTask の `Run` メソッドの更新

新しいメソッドを正しい順序で呼び出すように `Run` メソッドを更新します。

*   **対象ファイル**: `src/pkg/cuber/tasks/metacognition/metabolism_task.go`

*   **変更理由**: `deleteOrphanedNodes` と `deleteWeaklyConnectedNodes` を順番に呼び出し、両方の削除件数を集計します。

*   **実装コード（Run メソッド内の該当部分）**:
```go
// 3. MDL Principle に基づくノード削除
// 3-A. 完全孤立ノードの削除（MDL 判定なし）
orphanDeletedCount, err := t.deleteOrphanedNodes(ctx, mgConfig.MinSurvivalProtectionHours)
if err != nil {
	utils.LogWarn(t.Logger, "MetabolismTask: deleteOrphanedNodes failed", zap.Error(err))
}

// 3-B. 弱接続ノードの削除（MDL 判定あり）
weakDeletedCount, mdlUsage, err := t.deleteWeaklyConnectedNodes(
	ctx,
	mgConfig.PruneThreshold,
	mgConfig.MinSurvivalProtectionHours,
	appconfig.MDL_K_NEIGHBORS,
)
usage.Add(mdlUsage)
if err != nil {
	utils.LogWarn(t.Logger, "MetabolismTask: deleteWeaklyConnectedNodes failed", zap.Error(err))
}

totalNodesDeleted := orphanDeletedCount + weakDeletedCount
```

---

### 2.5 `DeleteNode` の検証：DETACH DELETE の確認

ノードを削除する際、接続しているエッジも一緒に削除される必要があります（Cypher の `DETACH DELETE` 相当）。

*   **対象ファイル**: `src/pkg/cuber/db/ladybugdb/ladybugdb_storage.go`

*   **確認事項**: 既存の `DeleteNode` メソッドが `DETACH DELETE` を使用しているか確認し、必要であれば修正します。

*   **実装コード（確認・修正）**:
```go
func (s *LadybugDBStorage) DeleteNode(ctx context.Context, nodeID, memoryGroup string) error {
	// DETACH DELETE を使用して、ノードと接続エッジを一括削除
	query := fmt.Sprintf(`
		MATCH (n:%s {id: '%s', memory_group: '%s'})
		DETACH DELETE n
	`, types.TABLE_NAME_GRAPH_NODE, escapeString(nodeID), escapeString(memoryGroup))
	
	if result, err := s.conn.Query(query); err != nil {
		return fmt.Errorf("DeleteNode failed: %w", err)
	} else {
		result.Close()
	}
	return nil
}
```

---

## 3. 検証計画

### 3.1 ビルド検証
```bash
make build
make build-linux-amd64
```

### 3.2 ロジック検証（手動テスト）
1.  テスト用の Cube を作成し、複数のノードとエッジを Absorb する。
2.  一部のエッジの Weight/Confidence を意図的に低く設定する（例：0.05 × 0.05 = 0.0025）。
3.  Memify を実行し、`deleteWeaklyConnectedNodes` が対象ノードを正しく検出するか確認。
4.  MDL 判定により、近傍類似度が高いノードのみが削除されることを確認。

### 3.3 ログ確認
以下のログメッセージが適切に出力されることを確認：
- `"MetabolismTask: Deleted orphaned node (no edges)"`
- `"MetabolismTask: Deleted weakly connected node via MDL"`

---

## 4. 設計上の考慮事項

### 4.1 特異点保護（Singularity Protection）について

ドキュメント（L130）には以下の記述があります：

> 「**孤立していても周囲と全く異なるベクトル（特異点）を持つ場合**、「このノードを消すと、このユニークな情報は二度と復元できない」と判断し、復元困難度を極めて高く（残すべき知識として）設定します。」

本フェーズでは、**完全孤立ノードには MDL 判定を適用しない**設計としていますが、将来的に「孤立していても特異点として保護する」機能を追加する場合は、`deleteOrphanedNodes` にオプションの MDL 判定を追加することを検討してください。

### 4.2 パフォーマンスへの影響

`GetWeaklyConnectedNodes` は全ノードに対する集約クエリを実行するため、大規模グラフではコストがかかります。将来的には以下の最適化を検討：
- インデックスの追加（`memory_group` + `created_at`）
- バッチ処理（ページング）の導入

---

## 5. まとめ

| 項目 | 修正前 | 修正後 |
|---|---|---|
| MDL 判定の対象 | エッジ 0 本のノードのみ | **エッジがあるが全て Thickness ≤ 閾値のノード** |
| 孤立ノードの扱い | MDL 判定で削除を決定 | **保護期間後に即座に削除（MDL 判定なし）** |
| 新規ストレージ API | なし | `GetWeaklyConnectedNodes` を追加 |
| メソッド構成 | `deleteOrphanedNodes` 1 つ | `deleteOrphanedNodes` + `deleteWeaklyConnectedNodes` に分割 |

この修正により、MDL Principle の本来の目的である「**繋がりはあるが、その繋がりに価値がなく、かつ他のノードで情報を復元可能なノード**の選別と削除」が正しく実装されます。
