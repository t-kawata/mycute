package storage

import (
	"context"
	"time"
)

type Data struct {
	ID                   string    `json:"id"`
	GroupID              string    `json:"group_id"` // [NEW] Partition Identifier
	Name                 string    `json:"name"`
	RawDataLocation      string    `json:"raw_data_location"`      // Path to the converted text file
	OriginalDataLocation string    `json:"original_data_location"` // Path to the original file
	Extension            string    `json:"extension"`
	MimeType             string    `json:"mime_type"`
	ContentHash          string    `json:"content_hash"`
	OwnerID              string    `json:"owner_id"`
	CreatedAt            time.Time `json:"created_at"`
}

type Document struct {
	ID       string                 `json:"id"`
	GroupID  string                 `json:"group_id"` // [NEW]
	DataID   string                 `json:"data_id"`  // Foreign key to Data
	Text     string                 `json:"text"`
	MetaData map[string]interface{} `json:"metadata"`
}

type Chunk struct {
	ID         string    `json:"id"`
	GroupID    string    `json:"group_id"`    // [NEW]
	DocumentID string    `json:"document_id"` // Foreign key to Document
	Text       string    `json:"text"`
	Embedding  []float32 `json:"embedding"` // Vector embedding
	TokenCount int       `json:"token_count"`
	ChunkIndex int       `json:"chunk_index"`
}

type SearchResult struct {
	ID       string
	Text     string
	Distance float64
}

type VectorStorage interface {
	// Metadata operations
	SaveData(ctx context.Context, data *Data) error
	Exists(ctx context.Context, contentHash string, groupID string) bool
	GetDataByID(ctx context.Context, id string, groupID string) (*Data, error)
	GetDataList(ctx context.Context, groupID string) ([]*Data, error)

	// Vector operations
	SaveDocument(ctx context.Context, document *Document) error
	SaveChunk(ctx context.Context, chunk *Chunk) error
	SaveEmbedding(ctx context.Context, collectionName, id, text string, vector []float32, groupID string) error
	Search(ctx context.Context, collectionName string, vector []float32, k int, groupID string) ([]*SearchResult, error)

	Close() error
}

type Embedder interface {
	EmbedQuery(ctx context.Context, text string) ([]float32, error)
}

type Node struct {
	ID         string                 `json:"id"`
	GroupID    string                 `json:"group_id"`   // [NEW]
	Type       string                 `json:"type"`       // Node label (e.g., "Person", "Organization")
	Properties map[string]interface{} `json:"properties"` // Node attributes
}

type Edge struct {
	SourceID   string                 `json:"source_id"`
	TargetID   string                 `json:"target_id"`
	GroupID    string                 `json:"group_id"`   // [NEW]
	Type       string                 `json:"type"`       // Edge label (e.g., "WORKS_AT")
	Properties map[string]interface{} `json:"properties"` // Edge attributes
}

type Triplet struct {
	Source *Node
	Edge   *Edge
	Target *Node
}

type GraphStorage interface {
	AddNodes(ctx context.Context, nodes []*Node) error
	AddEdges(ctx context.Context, edges []*Edge) error
	// GetTriplets retrieves triplets for given node IDs, strictly filtered by group_id.
	// Even though nodeIDs may already come from group-filtered search results,
	// we enforce group_id filtering here for implementation consistency and strict partitioning.
	GetTriplets(ctx context.Context, nodeIDs []string, groupID string) ([]*Triplet, error)
	EnsureSchema(ctx context.Context) error

	Close() error
}

// GraphData represents a collection of nodes and edges.
type GraphData struct {
	Nodes []*Node `json:"nodes"`
	Edges []*Edge `json:"edges"`
}

// CognifyOutput represents the output of the Cognify pipeline steps.
type CognifyOutput struct {
	Chunks    []*Chunk   `json:"chunks"`
	GraphData *GraphData `json:"graph_data"`
}
