// Package cozodb は、CozoDBを使用したグラフストレージの実装を提供します。
// CozoDBは、Datalogクエリ言語を使用するグラフデータベースで、
// RocksDBバックエンドにより永続化をサポートします。
package cozodb

import (
	"context"
	"encoding/json"
	"fmt"
	"strings"

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
	rows := make([][]interface{}, len(nodes))
	for i, n := range nodes {
		rows[i] = []interface{}{n.ID, n.GroupID, n.Type, n.Properties}
	}

	// Datalogクエリ
	// ?[id, group_id, type, properties] <- $data: データを$dataパラメータから取得
	// :put nodes {...}: nodesリレーションにデータを挿入（既存データは上書き）
	query := "?[id, group_id, type, properties] <- $data :put nodes {id, group_id, type, properties}"
	params := map[string]interface{}{
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
	rows := make([][]interface{}, len(edges))
	for i, e := range edges {
		rows[i] = []interface{}{e.SourceID, e.TargetID, e.GroupID, e.Type, e.Properties}
	}

	// Datalogクエリ
	// :put edges {...}: edgesリレーションにデータを挿入（既存データは上書き）
	query := "?[source_id, target_id, group_id, type, properties] <- $data :put edges {source_id, target_id, group_id, type, properties}"
	params := map[string]interface{}{
		"data": rows,
	}

	// クエリを実行
	if _, err := s.db.Run(query, params); err != nil {
		return fmt.Errorf("failed to add edges: %w", err)
	}

	return nil
}

// GetTriplets は、指定されたノードIDに関連するトリプレット（ノード-エッジ-ノード）を取得します。
// この関数は以下の処理を行います：
//  1. 指定されたノードIDに接続されているエッジを取得（group_idでフィルタリング）
//  2. エッジに関連するすべてのノードを取得
//  3. エッジとノードを組み合わせてトリプレットを構築
//
// group_idによる厳格なフィルタリング:
//   - nodeIDsは既にgroup_idでフィルタリングされたベクトル検索結果から来ている可能性が高いですが、
//     実装の一貫性と厳格なパーティション分離のため、ここでも明示的にgroup_idでフィルタリングします
//   - この冗長性は意図的で、クロスパーティションのデータ漏洩を確実に防ぎます
//
// 引数:
//   - ctx: コンテキスト
//   - nodeIDs: トリプレットを取得する起点となるノードIDのリスト
//   - groupID: グループID（パーティション分離のため）
//
// 返り値:
//   - []*storage.Triplet: トリプレットのリスト
//   - error: エラーが発生した場合
func (s *CozoStorage) GetTriplets(ctx context.Context, nodeIDs []string, groupID string) ([]*storage.Triplet, error) {
	// ノードIDが空の場合は空のリストを返す
	if len(nodeIDs) == 0 {
		return nil, nil
	}

	// ========================================
	// 1. エッジを取得
	// ========================================
	// ノードIDをDatalogクエリ用にクォート
	// シングルクォートをエスケープして、SQLインジェクション的な問題を防ぐ
	quotedIDs := make([]string, len(nodeIDs))
	for i, id := range nodeIDs {
		quotedIDs[i] = fmt.Sprintf("'%s'", strings.ReplaceAll(id, "'", "\\'"))
	}
	// ノードIDのリストを文字列化（例: ['id1', 'id2', 'id3']）
	idsList := fmt.Sprintf("[%s]", strings.Join(quotedIDs, ", "))

	// Datalogクエリ
	// *edges[...]: edgesリレーションから全データを取得
	// (source_id in %s or target_id in %s): 指定されたノードIDに接続されているエッジ
	// group_id = '%s': 厳格なパーティション分離のためのフィルタリング
	query := fmt.Sprintf(`
		?[source_id, target_id, group_id, type, properties] := 
			*edges[source_id, target_id, group_id, type, properties],
			(source_id in %s or target_id in %s),
			group_id = '%s'
	`, idsList, idsList, strings.ReplaceAll(groupID, "'", "\\'"))

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

		// Edgeオブジェクトを作成
		edges = append(edges, &storage.Edge{
			SourceID:   sourceID,
			TargetID:   targetID,
			Type:       typ,
			Properties: props,
			// GroupID: groupID, // 必要に応じて設定可能
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
			id in %s
	`, allIDsList)

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

// Close は、CozoDBへの接続をクローズします。
// この関数は、CogneeService.Close() から呼び出されます。
//
// 返り値:
//   - error: 常にnilを返す（CozoDB.Close()はエラーを返さない）
func (s *CozoStorage) Close() error {
	s.db.Close()
	return nil
}
