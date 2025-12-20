package storage

import "fmt"

// ConvertNodesAndEdgesToTriples は、ノードとエッジからトリプルを作成します。
// この関数は、ladybugdb_storage.go の GetTriples で取得される triples と
// 同じ構造のトリプルを手作業で作成します。
//
// 各Edgeについて、SourceIDとTargetIDに一致するノードを検索し、
// それらを組み合わせてTripleを作成します。
// 該当するノードが見つからないエッジはスキップされます。
//
// 引数:
//   - nodes: ノードのスライスへのポインタ
//   - edges: エッジのスライスへのポインタ
//
// 返り値:
//   - triples: 作成されたトリプルのスライスへのポインタ
//   - err: エラーが発生した場合
func ConvertNodesAndEdgesToTriples(nodes *[]*Node, edges *[]*Edge) (triples *[]*Triple, err error) {
	if nodes == nil || edges == nil {
		return nil, fmt.Errorf("nodes and edges must not be nil")
	}
	// ノードをIDでインデックス化してO(1)ルックアップを実現
	nodeMap := make(map[string]*Node)
	for _, node := range *nodes {
		if node != nil {
			nodeMap[node.ID] = node
		}
	}
	// 結果を格納するスライス
	result := make([]*Triple, 0, len(*edges))
	// 各エッジについて、対応するソースノードとターゲットノードを検索
	for _, edge := range *edges {
		if edge == nil {
			continue
		}
		// ソースノードを検索
		sourceNode, sourceExists := nodeMap[edge.SourceID]
		if !sourceExists {
			// ソースノードが見つからない場合はスキップ
			continue
		}
		// ターゲットノードを検索
		targetNode, targetExists := nodeMap[edge.TargetID]
		if !targetExists {
			// ターゲットノードが見つからない場合はスキップ
			continue
		}
		// GetTriplesと同じ構造でTripleを作成
		// GetTriplesでは各フィールドを個別にコピーしているが、
		// ここではポインタをそのまま使用（元データを変更しない前提）
		triple := &Triple{
			Source: &Node{
				ID:          sourceNode.ID,
				MemoryGroup: sourceNode.MemoryGroup,
				Type:        sourceNode.Type,
				Properties:  sourceNode.Properties,
			},
			Edge: &Edge{
				SourceID:    edge.SourceID,
				TargetID:    edge.TargetID,
				MemoryGroup: edge.MemoryGroup,
				Type:        edge.Type,
				Properties:  edge.Properties,
				Weight:      edge.Weight,
				Confidence:  edge.Confidence,
			},
			Target: &Node{
				ID:          targetNode.ID,
				MemoryGroup: targetNode.MemoryGroup,
				Type:        targetNode.Type,
				Properties:  targetNode.Properties,
			},
		}
		result = append(result, triple)
	}
	return &result, nil
}
