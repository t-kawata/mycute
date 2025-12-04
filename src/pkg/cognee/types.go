package cognee

// SearchType defines the type of search operation.
type SearchType string

const (
	SearchTypeGraphCompletion SearchType = "GRAPH_COMPLETION"
	SearchTypeGraph           SearchType = "GRAPH"
	SearchTypeRAGCompletion   SearchType = "RAG_COMPLETION"
	SearchTypeCode            SearchType = "CODE"
	SearchTypeCypher          SearchType = "CYPHER"
)
