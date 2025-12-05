package cozodb

import (
	"context"
	"encoding/json"
	"fmt"
	"strings"

	"mycute/pkg/cognee/storage"

	cozo "github.com/cozodb/cozo-lib-go"
)

type CozoStorage struct {
	db *cozo.CozoDB
}

func NewCozoStorage(db *cozo.CozoDB) *CozoStorage {
	return &CozoStorage{db: db}
}

// Ensure interface implementation
var _ storage.GraphStorage = (*CozoStorage)(nil)

func (s *CozoStorage) EnsureSchema(ctx context.Context) error {
	queries := []string{
		":create nodes { id: String, group_id: String, type: String, properties: Json }",
		":create edges { source_id: String, target_id: String, group_id: String, type: String, properties: Json }",
	}

	for _, q := range queries {
		if _, err := s.db.Run(q, nil); err != nil {
			// Ignore error if table already exists
			errMsg := err.Error()
			if !strings.Contains(errMsg, "already exists") && !strings.Contains(errMsg, "conflicts with an existing one") {
				return fmt.Errorf("failed to create schema: %w", err)
			}
		}
	}
	return nil
}

func (s *CozoStorage) AddNodes(ctx context.Context, nodes []*storage.Node) error {
	if len(nodes) == 0 {
		return nil
	}

	// Construct Datalog query for batch insert
	// :put nodes {id, type, properties} -> added group_id
	rows := make([][]interface{}, len(nodes))
	for i, n := range nodes {
		rows[i] = []interface{}{n.ID, n.GroupID, n.Type, n.Properties}
	}

	query := "?[id, group_id, type, properties] <- $data :put nodes {id, group_id, type, properties}"
	params := map[string]interface{}{
		"data": rows,
	}

	if _, err := s.db.Run(query, params); err != nil {
		return fmt.Errorf("failed to add nodes: %w", err)
	}

	return nil
}

func (s *CozoStorage) AddEdges(ctx context.Context, edges []*storage.Edge) error {
	if len(edges) == 0 {
		return nil
	}

	rows := make([][]interface{}, len(edges))
	for i, e := range edges {
		rows[i] = []interface{}{e.SourceID, e.TargetID, e.GroupID, e.Type, e.Properties}
	}

	query := "?[source_id, target_id, group_id, type, properties] <- $data :put edges {source_id, target_id, group_id, type, properties}"
	params := map[string]interface{}{
		"data": rows,
	}

	if _, err := s.db.Run(query, params); err != nil {
		return fmt.Errorf("failed to add edges: %w", err)
	}

	return nil
}

func (s *CozoStorage) GetTriplets(ctx context.Context, nodeIDs []string) ([]*storage.Triplet, error) {
	if len(nodeIDs) == 0 {
		return nil, nil
	}

	// Note on Partitioning:
	// This query retrieves edges where Source OR Target is in the provided (already filtered) nodeIDs list.
	// Since nodeIDs come from a VectorSearch that was ALREADY filtered by group_id,
	// we logically only traverse the subgraph belonging to that group's nodes.
	// Adding explicit group_id check here is redundant but harmless.
	// For now, relying on the input nodeIDs is sufficient and consistent with graph traversal logic.

	quotedIDs := make([]string, len(nodeIDs))
	for i, id := range nodeIDs {
		quotedIDs[i] = fmt.Sprintf("'%s'", strings.ReplaceAll(id, "'", "\\'"))
	}
	idsList := fmt.Sprintf("[%s]", strings.Join(quotedIDs, ", "))

	query := fmt.Sprintf(`
		?[source_id, target_id, group_id, type, properties] := 
			*edges[source_id, target_id, group_id, type, properties],
			(source_id in %s or target_id in %s)
	`, idsList, idsList)

	res, err := s.db.Run(query, nil)
	if err != nil {
		return nil, fmt.Errorf("failed to get triplets: %w", err)
	}

	edgeRows := res.Rows
	if len(edgeRows) == 0 {
		return nil, nil
	}

	relatedNodeIDs := make(map[string]bool)
	var edges []*storage.Edge

	for _, row := range edgeRows {
		// row: [source_id, target_id, group_id, type, properties]
		sourceID := row[0].(string)
		targetID := row[1].(string)
		// groupID := row[2].(string) -> unused but part of result
		typ := row[3].(string)

		var props map[string]any
		if p, ok := row[4].(map[string]any); ok {
			props = p
		} else if pStr, ok := row[4].(string); ok {
			json.Unmarshal([]byte(pStr), &props)
		}

		edges = append(edges, &storage.Edge{
			SourceID:   sourceID,
			TargetID:   targetID,
			Type:       typ,
			Properties: props,
			// GroupID: groupID, // Could populate if needed
		})
		relatedNodeIDs[sourceID] = true
		relatedNodeIDs[targetID] = true
	}

	// 2. Fetch all related nodes
	// ?[id, group_id, type, properties]
	allIDs := make([]string, 0, len(relatedNodeIDs))
	for id := range relatedNodeIDs {
		allIDs = append(allIDs, fmt.Sprintf("'%s'", strings.ReplaceAll(id, "'", "\\'")))
	}
	allIDsList := fmt.Sprintf("[%s]", strings.Join(allIDs, ", "))

	nodeQuery := fmt.Sprintf(`
		?[id, group_id, type, properties] := 
			*nodes[id, group_id, type, properties],
			id in %s
	`, allIDsList)

	nodeRes, err := s.db.Run(nodeQuery, nil)
	if err != nil {
		return nil, fmt.Errorf("failed to fetch nodes: %w", err)
	}

	nodeMap := make(map[string]*storage.Node)
	for _, row := range nodeRes.Rows {
		id := row[0].(string)
		// groupID := row[1].(string)
		typ := row[2].(string)
		var props map[string]any
		if p, ok := row[3].(map[string]any); ok {
			props = p
		} else if pStr, ok := row[3].(string); ok {
			json.Unmarshal([]byte(pStr), &props)
		}

		nodeMap[id] = &storage.Node{
			ID:         id,
			Type:       typ,
			Properties: props,
		}
	}

	// 3. Construct Triplets
	var triplets []*storage.Triplet
	for _, edge := range edges {
		source, ok1 := nodeMap[edge.SourceID]
		target, ok2 := nodeMap[edge.TargetID]
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

func (s *CozoStorage) Close() error {
	s.db.Close()
	return nil
}
