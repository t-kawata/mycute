// Package cozodb は、CozoDBを使用したグラフストレージの実装を提供します。
// CozoDBは、Datalogクエリ言語を使用するグラフデータベースで、
// RocksDBバックエンドにより永続化をサポートします。
package cozodb

import (
	"context"
	"encoding/json"
	"fmt"
	"strings"
	"time"

	"mycute/pkg/cognee/storage"

	cozo "github.com/cozodb/cozo-lib-go"
)

// CozoStorage は、CozoDBを使用したGraphStorageの実装です。
// このストレージは以下のリレーション（テーブル）を管理します：
//   - nodes: グラフのノード（エンティティ）
//   - edges: グラフのエッジ（関係）
type CozoStorage struct {
	db *cozo.CozoDB // CozoDBへの接続
}

// NewCozoStorage は、CozoStorageの新しいインスタンスを作成します。
// 引数:
//   - db: 既に開かれたCozoDBへの接続
//
// 返り値:
//   - *CozoStorage: 新しいCozoStorageインスタンス
func NewCozoStorage(db *cozo.CozoDB) *CozoStorage {
	return &CozoStorage{db: db}
}

// インターフェース実装の確認
// コンパイル時に、CozoStorageがstorage.GraphStorageインターフェースを
// 正しく実装しているかをチェックします
var _ storage.GraphStorage = (*CozoStorage)(nil)

// EnsureSchema は、CozoDBにグラフスキーマを作成します。
// この関数は以下のリレーションを作成します：
//   - nodes: ノードを格納（id, group_id, type, properties）
//   - edges: エッジを格納（source_id, target_id, group_id, type, properties）
//
// 既にリレーションが存在する場合はエラーを無視します。
//
// 引数:
//   - ctx: コンテキスト
//
// 返り値:
//   - error: スキーマ作成に失敗した場合
func (s *CozoStorage) EnsureSchema(ctx context.Context) error {
	// スキーマ作成クエリのリスト
	// :create コマンドでリレーションを作成
	queries := []string{
		":create nodes { id: String, group_id: String, type: String, properties: Json }",
		":create edges { source_id: String, target_id: String, group_id: String, type: String, properties: Json }",
	}

	// 各クエリを実行
	for _, q := range queries {
		if _, err := s.db.Run(q, nil); err != nil {
			// エラーメッセージをチェック
			errMsg := err.Error()
			// リレーションが既に存在する場合はエラーを無視
			if !strings.Contains(errMsg, "already exists") && !strings.Contains(errMsg, "conflicts with an existing one") {
				return fmt.Errorf("failed to create schema: %w", err)
			}
		}
	}
	return nil
}

// AddNodes は、複数のノードをnodesリレーションに追加します。
// この関数はバッチ挿入を使用して効率的にノードを追加します。
//
// 引数:
//   - ctx: コンテキスト
//   - nodes: 追加するノードのリスト
//
// 返り値:
//   - error: ノードの追加に失敗した場合
func (s *CozoStorage) AddNodes(ctx context.Context, nodes []*storage.Node) error {
	// ノードが空の場合は何もしない
	if len(nodes) == 0 {
		return nil
	}

	// Datalogクエリ用のデータを構築
	// 各ノードを [id, group_id, type, properties] の配列に変換
	rows := make([][]any, len(nodes))
	for i, n := range nodes {
		rows[i] = []any{n.ID, n.GroupID, n.Type, n.Properties}
	}

	// Datalogクエリ
	// ?[id, group_id, type, properties] <- $data: データを$dataパラメータから取得
	// :put nodes {...}: nodesリレーションにデータを挿入（既存データは上書き）
	query := "?[id, group_id, type, properties] <- $data :put nodes {id, group_id, type, properties}"
	params := map[string]any{
		"data": rows,
	}

	// クエリを実行
	if _, err := s.db.Run(query, params); err != nil {
		return fmt.Errorf("failed to add nodes: %w", err)
	}

	return nil
}

// AddEdges は、複数のエッジをedgesリレーションに追加します。
// この関数はバッチ挿入を使用して効率的にエッジを追加します。
//
// 引数:
//   - ctx: コンテキスト
//   - edges: 追加するエッジのリスト
//
// 返り値:
//   - error: エッジの追加に失敗した場合
func (s *CozoStorage) AddEdges(ctx context.Context, edges []*storage.Edge) error {
	// エッジが空の場合は何もしない
	if len(edges) == 0 {
		return nil
	}

	// Datalogクエリ用のデータを構築
	// 各エッジを [source_id, target_id, group_id, type, properties] の配列に変換
	rows := make([][]any, len(edges))
	for i, e := range edges {
		// Propertiesがnilの場合は初期化
		if e.Properties == nil {
			e.Properties = make(map[string]any)
		}
		// WeightとConfidenceをPropertiesにマッピング
		e.Properties["weight"] = e.Weight
		e.Properties["confidence"] = e.Confidence

		rows[i] = []any{e.SourceID, e.TargetID, e.GroupID, e.Type, e.Properties}
	}

	// Datalogクエリ
	// :put edges {...}: edgesリレーションにデータを挿入（既存データは上書き）
	query := "?[source_id, target_id, group_id, type, properties] <- $data :put edges {source_id, target_id, group_id, type, properties}"
	params := map[string]any{
		"data": rows,
	}

	// クエリを実行
	if _, err := s.db.Run(query, params); err != nil {
		return fmt.Errorf("failed to add edges: %w", err)
	}

	return nil
}

// GetTriplets は、指定されたノードIDに関連するトリプレット（ノード-エッジ-ノード）を取得します。
// group_idによる厳格なフィルタリングを行います。
func (s *CozoStorage) GetTriplets(ctx context.Context, nodeIDs []string, groupID string) ([]*storage.Triplet, error) {
	if len(nodeIDs) == 0 {
		return nil, nil
	}

	// ========================================
	// 1. エッジを取得
	// ========================================
	// ノードIDをDatalogクエリ用にクォート
	quotedIDs := make([]string, len(nodeIDs))
	for i, id := range nodeIDs {
		quotedIDs[i] = fmt.Sprintf("'%s'", strings.ReplaceAll(id, "'", "\\'"))
	}
	idsList := fmt.Sprintf("[%s]", strings.Join(quotedIDs, ", "))
	quotedGroupID := fmt.Sprintf("'%s'", strings.ReplaceAll(groupID, "'", "\\'"))

	// Datalogクエリ
	// 厳格なパーティション分離: group_id = $group_id を強制
	query := fmt.Sprintf(`
		?[source_id, target_id, group_id, type, properties] := 
			*edges[source_id, target_id, group_id, type, properties],
			(source_id in %s or target_id in %s),
			group_id = %s
	`, idsList, idsList, quotedGroupID)

	// クエリを実行
	res, err := s.db.Run(query, nil)
	if err != nil {
		return nil, fmt.Errorf("failed to get triplets: %w", err)
	}

	edgeRows := res.Rows
	// エッジが見つからない場合は空のリストを返す
	if len(edgeRows) == 0 {
		return nil, nil
	}

	// エッジに関連するノードIDを収集
	relatedNodeIDs := make(map[string]bool)
	var edges []*storage.Edge

	// 各エッジ行を処理
	for _, row := range edgeRows {
		// row: [source_id, target_id, group_id, type, properties]
		sourceID := row[0].(string)
		targetID := row[1].(string)
		// groupID := row[2].(string) // 結果に含まれるが使用しない
		typ := row[3].(string)

		// propertiesをmap[string]anyに変換
		var props map[string]any
		if p, ok := row[4].(map[string]any); ok {
			// 既にmapの場合
			props = p
		} else if pStr, ok := row[4].(string); ok {
			// JSON文字列の場合はパース
			json.Unmarshal([]byte(pStr), &props)
		}

		// WeightとConfidenceを抽出
		var weight float64 = 1.0 // デフォルト値
		var confidence float64 = 1.0

		if w, ok := props["weight"].(float64); ok {
			weight = w
		}
		if c, ok := props["confidence"].(float64); ok {
			confidence = c
		}

		// Edgeオブジェクトを作成
		edges = append(edges, &storage.Edge{
			SourceID:   sourceID,
			TargetID:   targetID,
			Type:       typ,
			Properties: props,
			Weight:     weight,
			Confidence: confidence,
			GroupID:    groupID,
		})

		// 関連ノードIDを記録
		relatedNodeIDs[sourceID] = true
		relatedNodeIDs[targetID] = true
	}

	// ========================================
	// 2. 関連するすべてのノードを取得
	// ========================================
	// ノードIDをDatalogクエリ用にクォート
	allIDs := make([]string, 0, len(relatedNodeIDs))
	for id := range relatedNodeIDs {
		allIDs = append(allIDs, fmt.Sprintf("'%s'", strings.ReplaceAll(id, "'", "\\'")))
	}
	allIDsList := fmt.Sprintf("[%s]", strings.Join(allIDs, ", "))

	// Datalogクエリ
	// *nodes[...]: nodesリレーションから全データを取得
	// id in %s: 指定されたノードIDのみを取得
	nodeQuery := fmt.Sprintf(`
		?[id, group_id, type, properties] := 
			*nodes[id, group_id, type, properties],
			id in %s,
			group_id = %s
	`, allIDsList, quotedGroupID)

	// クエリを実行
	nodeRes, err := s.db.Run(nodeQuery, nil)
	if err != nil {
		return nil, fmt.Errorf("failed to fetch nodes: %w", err)
	}

	// ノードをマップに格納（IDで検索できるように）
	nodeMap := make(map[string]*storage.Node)
	for _, row := range nodeRes.Rows {
		id := row[0].(string)
		// groupID := row[1].(string) // 結果に含まれるが使用しない
		typ := row[2].(string)

		// propertiesをmap[string]anyに変換
		var props map[string]any
		if p, ok := row[3].(map[string]any); ok {
			props = p
		} else if pStr, ok := row[3].(string); ok {
			json.Unmarshal([]byte(pStr), &props)
		}

		// Nodeオブジェクトを作成してマップに格納
		nodeMap[id] = &storage.Node{
			ID:         id,
			Type:       typ,
			Properties: props,
			GroupID:    groupID,
		}
	}

	// ========================================
	// 3. トリプレットを構築
	// ========================================
	var triplets []*storage.Triplet
	// 各エッジについて、ソースノードとターゲットノードを取得してトリプレットを作成
	for _, edge := range edges {
		source, ok1 := nodeMap[edge.SourceID]
		target, ok2 := nodeMap[edge.TargetID]
		// 両方のノードが存在する場合のみトリプレットを作成
		if ok1 && ok2 {
			triplets = append(triplets, &storage.Triplet{
				Source: source,
				Edge:   edge,
				Target: target,
			})
		}
	}

	return triplets, nil
}

const (
	// chunkFetchBatchSize は、CozoDBから一度に取得するチャンク数です。
	// メモリ使用量と処理効率のバランスを考慮して設定します。
	// エッジデバイス向けに控えめな値を設定しています。
	chunkFetchBatchSize = 100
)

// StreamDocumentChunks は、DocumentChunk タイプのノードをストリーミングで取得します。
// CozoDBから LIMIT/OFFSET を使用してページネーションクエリを発行し、
// 1バッチずつデータを返します。これにより、大規模グラフでもメモリ使用量を一定に保ちます。
//
// 実装詳細:
//   - goroutine でバックグラウンドにデータをフェッチ
//   - chan でデータを送信（バッファなし: 消費されるまでブロック）
//   - コンテキストのキャンセルに対応
func (s *CozoStorage) StreamDocumentChunks(ctx context.Context, groupID string) (<-chan *storage.ChunkData, <-chan error) {
	chunkChan := make(chan *storage.ChunkData)
	errChan := make(chan error, 1) // バッファ1: エラーを1回だけ送信

	go func() {
		defer close(chunkChan)
		defer close(errChan)

		offset := 0

		for {
			// コンテキストのキャンセルをチェック
			select {
			case <-ctx.Done():
				errChan <- ctx.Err()
				return
			default:
			}

			// CozoDBクエリ: DocumentChunk タイプのノードを取得
			// LIMIT/OFFSET でページネーション
			query := `
				?[id, text, document_id] := 
					*nodes[id, group_id, type, properties],
					group_id = $group_id,
					type = "DocumentChunk",
					text = get(properties, "text", ""),
					document_id = get(properties, "document_id", "")
				:limit $limit
				:offset $offset
			`

			params := map[string]any{
				"group_id": groupID,
				"limit":    chunkFetchBatchSize,
				"offset":   offset,
			}

			result, err := s.db.Run(query, params)
			if err != nil {
				errChan <- fmt.Errorf("CozoDB StreamDocumentChunks query failed: %w", err)
				return
			}

			// 結果が空ならループ終了
			if len(result.Rows) == 0 {
				return
			}

			// 結果をパースしてチャネルに送信
			for _, row := range result.Rows {
				if len(row) < 3 {
					continue
				}

				id, _ := row[0].(string)
				text, _ := row[1].(string)
				documentID, _ := row[2].(string)

				// 空のテキストはスキップ
				if text == "" {
					continue
				}

				chunk := &storage.ChunkData{
					ID:         id,
					Text:       text,
					GroupID:    groupID,
					DocumentID: documentID,
				}

				// チャネルに送信（キャンセル対応）
				select {
				case chunkChan <- chunk:
				case <-ctx.Done():
					errChan <- ctx.Err()
					return
				}
			}

			// 次のページへ
			offset += chunkFetchBatchSize

			// 取得数がバッチサイズ未満なら終了
			if len(result.Rows) < chunkFetchBatchSize {
				return
			}
		}
	}()

	return chunkChan, errChan
}

// GetDocumentChunkCount は、指定されたグループIDの DocumentChunk 数を取得します。
// Memify の進捗表示や処理見積もりに使用されます。
func (s *CozoStorage) GetDocumentChunkCount(ctx context.Context, groupID string) (int, error) {
	query := `
		?[count(id)] := 
			*nodes[id, group_id, type, _],
			group_id = $group_id,
			type = "DocumentChunk"
	`

	params := map[string]any{
		"group_id": groupID,
	}

	result, err := s.db.Run(query, params)
	if err != nil {
		return 0, fmt.Errorf("CozoDB GetDocumentChunkCount query failed: %w", err)
	}

	if len(result.Rows) > 0 && len(result.Rows[0]) > 0 {
		if count, ok := result.Rows[0][0].(float64); ok {
			return int(count), nil
		}
	}

	return 0, nil
}

// GetNodesByType は、指定されたタイプのノードを取得します。
func (s *CozoStorage) GetNodesByType(ctx context.Context, nodeType string, groupID string) ([]*storage.Node, error) {
	query := `
		?[id, group_id, type, properties] := 
			*nodes[id, group_id, type, properties],
			group_id = $group_id,
			type = $type
	`
	params := map[string]any{"group_id": groupID, "type": nodeType}

	res, err := s.db.Run(query, params)
	if err != nil {
		return nil, fmt.Errorf("failed to get nodes by type: %w", err)
	}

	var nodes []*storage.Node
	for _, row := range res.Rows {
		id := row[0].(string)
		typ := row[2].(string)
		var props map[string]any
		if p, ok := row[3].(map[string]any); ok {
			props = p
		} else if pStr, ok := row[3].(string); ok {
			json.Unmarshal([]byte(pStr), &props)
		}

		nodes = append(nodes, &storage.Node{
			ID:         id,
			Type:       typ,
			Properties: props,
			GroupID:    groupID,
		})
	}
	return nodes, nil
}

// GetNodesByEdge は、指定されたエッジタイプでターゲットに接続されたノードを取得します。
func (s *CozoStorage) GetNodesByEdge(ctx context.Context, targetID string, edgeType string, groupID string) ([]*storage.Node, error) {
	query := `
		?[id, group_id, type, properties] := 
			*edges[source_id, target_id, group_id, edge_type, _],
			target_id = $target_id,
			edge_type = $edge_type,
			group_id = $group_id,
			*nodes[id, group_id, type, properties],
			id = source_id
	`
	params := map[string]any{
		"target_id": targetID,
		"edge_type": edgeType,
		"group_id":  groupID,
	}

	res, err := s.db.Run(query, params)
	if err != nil {
		return nil, fmt.Errorf("failed to get nodes by edge: %w", err)
	}

	var nodes []*storage.Node
	for _, row := range res.Rows {
		id := row[0].(string)
		typ := row[2].(string)
		var props map[string]any
		if p, ok := row[3].(map[string]any); ok {
			props = p
		} else if pStr, ok := row[3].(string); ok {
			json.Unmarshal([]byte(pStr), &props)
		}

		nodes = append(nodes, &storage.Node{
			ID:         id,
			Type:       typ,
			Properties: props,
			GroupID:    groupID,
		})
	}
	return nodes, nil
}

// UpdateEdgeWeight は、エッジの重みを更新します。
// Deprecated: Use UpdateEdgeMetrics instead.
func (s *CozoStorage) UpdateEdgeWeight(ctx context.Context, sourceID, targetID, groupID string, weight float64) error {
	return s.UpdateEdgeMetrics(ctx, sourceID, targetID, groupID, weight, 1.0) // Confidence defaults to 1.0 if not tracked
}

// UpdateEdgeMetrics は、エッジの重みと信頼度を更新します。
func (s *CozoStorage) UpdateEdgeMetrics(ctx context.Context, sourceID, targetID, groupID string, weight, confidence float64) error {
	// Step 1: 既存のエッジを取得
	getQuery := `
		?[source_id, target_id, group_id, type, properties] := 
			*edges[source_id, target_id, group_id, type, properties],
			source_id = $source_id,
			target_id = $target_id,
			group_id = $group_id
	`
	params := map[string]any{
		"source_id": sourceID,
		"target_id": targetID,
		"group_id":  groupID,
	}

	res, err := s.db.Run(getQuery, params)
	if err != nil {
		return fmt.Errorf("failed to get edge for update: %w", err)
	}

	if len(res.Rows) == 0 {
		return fmt.Errorf("edge not found: %s -> %s", sourceID, targetID)
	}

	// 既存のプロパティを取得
	row := res.Rows[0]
	edgeType := row[3].(string)

	var props map[string]any
	if p, ok := row[4].(map[string]any); ok {
		props = p
	} else if pStr, ok := row[4].(string); ok {
		json.Unmarshal([]byte(pStr), &props)
	}
	if props == nil {
		props = make(map[string]any)
	}

	// Step 2: プロパティを更新
	props["weight"] = weight
	props["confidence"] = confidence

	// Step 3: 更新されたエッジを書き戻す
	propsJSON, err := json.Marshal(props)
	if err != nil {
		return fmt.Errorf("failed to marshal properties: %w", err)
	}

	putQuery := `
		?[source_id, target_id, group_id, type, properties] <- [[$source_id, $target_id, $group_id, $type, $properties]]
		:put edges {source_id, target_id, group_id, type, properties}
	`
	putParams := map[string]any{
		"source_id":  sourceID,
		"target_id":  targetID,
		"group_id":   groupID,
		"type":       edgeType,
		"properties": string(propsJSON),
	}

	if _, err := s.db.Run(putQuery, putParams); err != nil {
		return fmt.Errorf("failed to update edge metrics: %w", err)
	}
	return nil
}

// DeleteEdge は、エッジを削除します。
func (s *CozoStorage) DeleteEdge(ctx context.Context, sourceID, targetID, groupID string) error {
	// 削除対象のエッジタイプを取得
	queryFind := `
		?[type] := *edges[source_id, target_id, group_id, type, _],
		source_id = $source_id,
		target_id = $target_id,
		group_id = $group_id
	`
	params := map[string]any{
		"source_id": sourceID,
		"target_id": targetID,
		"group_id":  groupID,
	}
	res, err := s.db.Run(queryFind, params)
	if err != nil {
		return fmt.Errorf("failed to find edge for deletion: %w", err)
	}

	for _, row := range res.Rows {
		typ := row[0].(string)

		rmQuery := "?[source_id, target_id, group_id, type] <- $data :rm edges {source_id, target_id, group_id, type}"
		rmData := [][]any{
			{sourceID, targetID, groupID, typ},
		}
		rmParams := map[string]any{"data": rmData}

		if _, err := s.db.Run(rmQuery, rmParams); err != nil {
			return fmt.Errorf("failed to delete edge: %w", err)
		}
	}
	return nil
}

// DeleteNode は、ノードを削除します。
func (s *CozoStorage) DeleteNode(ctx context.Context, nodeID, groupID string) error {
	// CozoDBの:rmはリレーションの全カラムを指定する必要がある
	// nodesリレーション: { id: String, group_id: String, type: String, properties: Json }
	// まず対象ノードを取得して、typeを含む完全なデータで削除
	query := `
		?[id, group_id, type, properties] := 
			*nodes[id, group_id, type, properties],
			id = $id,
			group_id = $group_id
		:rm nodes {id, group_id, type, properties}
	`
	params := map[string]any{
		"id":       nodeID,
		"group_id": groupID,
	}

	if _, err := s.db.Run(query, params); err != nil {
		return fmt.Errorf("failed to delete node: %w", err)
	}
	return nil
}

// GetEdgesByNode は、指定されたノードに接続されたエッジを取得します。
func (s *CozoStorage) GetEdgesByNode(ctx context.Context, nodeID string, groupID string) ([]*storage.Edge, error) {
	// ノードIDをエスケープ
	quotedID := fmt.Sprintf("'%s'", strings.ReplaceAll(nodeID, "'", "\\'"))
	quotedGroupID := fmt.Sprintf("'%s'", strings.ReplaceAll(groupID, "'", "\\'"))

	query := fmt.Sprintf(`
		?[source_id, target_id, group_id, type, properties] := 
			*edges[source_id, target_id, group_id, type, properties],
			(source_id = %s or target_id = %s),
			group_id = %s
	`, quotedID, quotedID, quotedGroupID)

	res, err := s.db.Run(query, nil)
	if err != nil {
		return nil, fmt.Errorf("failed to get edges by node: %w", err)
	}

	var edges []*storage.Edge
	for _, row := range res.Rows {
		sourceID := row[0].(string)
		targetID := row[1].(string)
		typ := row[3].(string)
		var props map[string]any
		if p, ok := row[4].(map[string]any); ok {
			props = p
		} else if pStr, ok := row[4].(string); ok {
			json.Unmarshal([]byte(pStr), &props)
		}

		var weight float64 = 1.0
		var confidence float64 = 1.0
		if w, ok := props["weight"].(float64); ok {
			weight = w
		}
		if c, ok := props["confidence"].(float64); ok {
			confidence = c
		}

		edges = append(edges, &storage.Edge{
			SourceID:   sourceID,
			TargetID:   targetID,
			Type:       typ,
			Properties: props,
			GroupID:    groupID,
			Weight:     weight,
			Confidence: confidence,
		})
	}
	return edges, nil
}

// ========================================
// 効率化API (Phase-09追加)
// ========================================

// GetOrphanNodes は、エッジを持たない孤立ノードを取得します。
//
// 実装詳細:
//   - Datalogの否定(not)を使用して、edgesリレーションに存在しないノードを検出
//   - gracePeriodを考慮し、作成直後のノードは除外
//   - 1回のクエリで全ての孤立ノードを取得（N+1問題の解消）
//
// Datalogクエリの解説:
//
//	?[id, type, props] :=
//	    *nodes[id, group_id, type, props],      <- nodesリレーションからすべてのノードを取得
//	    group_id = %s,                          <- 指定されたgroup_idでフィルタ
//	    created_at = get(props, "created_at", ""), <- propsからcreated_atを抽出
//	    created_at != "",                       <- created_atが存在するものだけ
//	    created_at < %s,                        <- 猶予期間外のもののみ対象
//	    not *edges[id, _, %s, _, _],            <- このIDがsource_idとして存在しない
//	    not *edges[_, id, %s, _, _]             <- このIDがtarget_idとして存在しない
//
// 引数:
//   - ctx: コンテキスト
//   - groupID: グループID
//   - gracePeriod: 猶予期間（この時間より前に作成されたノードのみ対象）
//
// 返り値:
//   - []*storage.Node: 孤立ノードのスライス
//   - error: クエリ実行エラーの場合
func (s *CozoStorage) GetOrphanNodes(ctx context.Context, groupID string, gracePeriod time.Duration) ([]*storage.Node, error) {
	// 猶予期間のカットオフ時刻を計算
	cutoffTime := time.Now().Add(-gracePeriod).Format(time.RFC3339)

	// グループIDとカットオフ時刻をクォート
	quotedGroupID := fmt.Sprintf("'%s'", strings.ReplaceAll(groupID, "'", "\\'"))
	quotedCutoffTime := fmt.Sprintf("'%s'", cutoffTime)

	// 孤立ノード検出クエリ
	// CozoDBの否定(not)演算子を使用して、エッジを持たないノードを検出
	query := fmt.Sprintf(`
		?[id, type, props] := 
			*nodes[id, group_id, type, props],
			group_id = %s,
			created_at = get(props, "created_at", ""),
			created_at != "",
			created_at < %s,
			not *edges[id, _, %s, _, _],
			not *edges[_, id, %s, _, _]
	`, quotedGroupID, quotedCutoffTime, quotedGroupID, quotedGroupID)

	res, err := s.db.Run(query, nil)
	if err != nil {
		return nil, fmt.Errorf("failed to query orphan nodes: %w", err)
	}

	var orphans []*storage.Node

	for _, row := range res.Rows {
		// row: [id, type, props]
		id := row[0].(string)
		typ := row[1].(string)

		// propsをmap[string]anyに変換
		var props map[string]any
		if p, ok := row[2].(map[string]any); ok {
			props = p
		} else if pStr, ok := row[2].(string); ok {
			json.Unmarshal([]byte(pStr), &props)
		}

		orphans = append(orphans, &storage.Node{
			ID:         id,
			GroupID:    groupID,
			Type:       typ,
			Properties: props,
		})
	}

	return orphans, nil
}

// Close は、CozoDBへの接続をクローズします。
// この関数は、CogneeService.Close() から呼び出されます。
//
// 返り値:
//   - error: 常にnilを返す（CozoDB.Close()はエラーを返さない）
func (s *CozoStorage) Close() error {
	s.db.Close()
	return nil
}
