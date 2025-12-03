package cognee

// Data represents the metadata of an ingested file.
type Data struct {
	ID                   string `json:"id"`
	Name                 string `json:"name"`
	RawDataLocation      string `json:"raw_data_location"`      // Path to the converted text file
	OriginalDataLocation string `json:"original_data_location"` // Path to the original file
	Extension            string `json:"extension"`
	MimeType             string `json:"mime_type"`
	ContentHash          string `json:"content_hash"`
	OwnerID              string `json:"owner_id"`
}

// Document represents a processed document associated with a Data item.
type Document struct {
	ID       string                 `json:"id"`
	DataID   string                 `json:"data_id"` // Foreign key to Data
	Text     string                 `json:"text"`
	MetaData map[string]interface{} `json:"metadata"`
}

// Chunk represents a segment of text from a Document, used for vector search.
type Chunk struct {
	ID         string    `json:"id"`
	DocumentID string    `json:"document_id"` // Foreign key to Document
	Text       string    `json:"text"`
	Embedding  []float32 `json:"embedding"` // Vector embedding
	TokenCount int       `json:"token_count"`
}

// Node represents an entity in the knowledge graph.
type Node struct {
	ID         string                 `json:"id"`
	Type       string                 `json:"type"`       // Node label (e.g., "Person", "Organization")
	Properties map[string]interface{} `json:"properties"` // Node attributes
}

// Edge represents a relationship between two Nodes in the knowledge graph.
type Edge struct {
	SourceID   string                 `json:"source_id"`
	TargetID   string                 `json:"target_id"`
	Type       string                 `json:"type"`       // Edge label (e.g., "WORKS_AT")
	Properties map[string]interface{} `json:"properties"` // Edge attributes
}

// SearchType defines the type of search operation.
type SearchType string

const (
	SearchTypeGraphCompletion SearchType = "GRAPH_COMPLETION"
	SearchTypeRAGCompletion   SearchType = "RAG_COMPLETION"
	SearchTypeCode            SearchType = "CODE"
	SearchTypeCypher          SearchType = "CYPHER"
)
