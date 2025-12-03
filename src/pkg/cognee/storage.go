package cognee

import (
	"context"
	"fmt"
	"math"
	"sort"
	"sync"
)

// MetadataStorage handles persistence for Data and Document objects.
type MetadataStorage interface {
	SaveData(ctx context.Context, data *Data) error
	GetData(ctx context.Context, id string) (*Data, error)
	// Add other methods as needed for Documents
}

// VectorStorage handles persistence and similarity search for Chunks.
type VectorStorage interface {
	SaveChunk(ctx context.Context, chunk *Chunk) error
	Search(ctx context.Context, queryVector []float32, topK int) ([]*Chunk, error)
}

// GraphStorage handles persistence and traversal for Nodes and Edges.
type GraphStorage interface {
	AddNode(ctx context.Context, node *Node) error
	AddEdge(ctx context.Context, edge *Edge) error
	GetContext(ctx context.Context, nodeIDs []string) ([]*Node, []*Edge, error)
}

// --- In-Memory Implementations ---

// inMemoryMetadataStorage implements MetadataStorage using a map.
type inMemoryMetadataStorage struct {
	data map[string]*Data
	mu   sync.RWMutex
}

func NewInMemoryMetadataStorage() MetadataStorage {
	return &inMemoryMetadataStorage{
		data: make(map[string]*Data),
	}
}

func (s *inMemoryMetadataStorage) SaveData(ctx context.Context, data *Data) error {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.data[data.ID] = data
	return nil
}

func (s *inMemoryMetadataStorage) GetData(ctx context.Context, id string) (*Data, error) {
	s.mu.RLock()
	defer s.mu.RUnlock()
	data, ok := s.data[id]
	if !ok {
		return nil, fmt.Errorf("data not found: %s", id)
	}
	return data, nil
}

// inMemoryVectorStorage implements VectorStorage using a slice.
type inMemoryVectorStorage struct {
	chunks []*Chunk
	mu     sync.RWMutex
}

func NewInMemoryVectorStorage() VectorStorage {
	return &inMemoryVectorStorage{
		chunks: make([]*Chunk, 0),
	}
}

func (s *inMemoryVectorStorage) SaveChunk(ctx context.Context, chunk *Chunk) error {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.chunks = append(s.chunks, chunk)
	return nil
}

func (s *inMemoryVectorStorage) Search(ctx context.Context, queryVector []float32, topK int) ([]*Chunk, error) {
	s.mu.RLock()
	defer s.mu.RUnlock()

	type scoredChunk struct {
		chunk *Chunk
		score float32
	}
	scores := make([]scoredChunk, len(s.chunks))

	for i, chunk := range s.chunks {
		scores[i] = scoredChunk{
			chunk: chunk,
			score: cosineSimilarity(queryVector, chunk.Embedding),
		}
	}

	// Sort by score descending
	sort.Slice(scores, func(i, j int) bool {
		return scores[i].score > scores[j].score
	})

	// Return top K
	result := make([]*Chunk, 0, topK)
	for i := 0; i < topK && i < len(scores); i++ {
		result = append(result, scores[i].chunk)
	}
	return result, nil
}

// cosineSimilarity calculates the cosine similarity between two vectors.
func cosineSimilarity(a, b []float32) float32 {
	if len(a) != len(b) {
		return 0
	}
	var dotProduct, normA, normB float32
	for i := range a {
		dotProduct += a[i] * b[i]
		normA += a[i] * a[i]
		normB += b[i] * b[i]
	}
	if normA == 0 || normB == 0 {
		return 0
	}
	return dotProduct / (float32(math.Sqrt(float64(normA))) * float32(math.Sqrt(float64(normB))))
}

// inMemoryGraphStorage implements GraphStorage using maps.
type inMemoryGraphStorage struct {
	nodes map[string]*Node
	edges []*Edge
	mu    sync.RWMutex
}

func NewInMemoryGraphStorage() GraphStorage {
	return &inMemoryGraphStorage{
		nodes: make(map[string]*Node),
		edges: make([]*Edge, 0),
	}
}

func (s *inMemoryGraphStorage) AddNode(ctx context.Context, node *Node) error {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.nodes[node.ID] = node
	return nil
}

func (s *inMemoryGraphStorage) AddEdge(ctx context.Context, edge *Edge) error {
	s.mu.Lock()
	defer s.mu.Unlock()
	s.edges = append(s.edges, edge)
	return nil
}

func (s *inMemoryGraphStorage) GetContext(ctx context.Context, nodeIDs []string) ([]*Node, []*Edge, error) {
	s.mu.RLock()
	defer s.mu.RUnlock()

	nodeSet := make(map[string]bool)
	for _, id := range nodeIDs {
		nodeSet[id] = true
	}

	relatedEdges := make([]*Edge, 0)
	expandedNodeSet := make(map[string]bool)

	// Find edges connected to the requested nodes
	for _, edge := range s.edges {
		if nodeSet[edge.SourceID] || nodeSet[edge.TargetID] {
			relatedEdges = append(relatedEdges, edge)
			expandedNodeSet[edge.SourceID] = true
			expandedNodeSet[edge.TargetID] = true
		}
	}

	// Retrieve all involved nodes
	relatedNodes := make([]*Node, 0)
	for nodeID := range expandedNodeSet {
		if node, ok := s.nodes[nodeID]; ok {
			relatedNodes = append(relatedNodes, node)
		}
	}

	return relatedNodes, relatedEdges, nil
}
