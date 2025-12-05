package search

// SearchType defines the type of search operation.
type SearchType string

const (
	// Implemented (Phase 3)
	SearchTypeGraphCompletion SearchType = "GRAPH_COMPLETION" // Default

	// Pending (Phase 4: Summarization)
	SearchTypeSummaries              SearchType = "SUMMARIES"
	SearchTypeGraphSummaryCompletion SearchType = "GRAPH_SUMMARY_COMPLETION"

	// Pending (Future Phases)
	SearchTypeChunks                    SearchType = "CHUNKS"
	SearchTypeRAGCompletion             SearchType = "RAG_COMPLETION"
	SearchTypeCode                      SearchType = "CODE"
	SearchTypeCypher                    SearchType = "CYPHER"
	SearchTypeNaturalLanguage           SearchType = "NATURAL_LANGUAGE"
	SearchTypeGraphCompletionCoT        SearchType = "GRAPH_COMPLETION_COT"
	SearchTypeGraphCompletionContextExt SearchType = "GRAPH_COMPLETION_CONTEXT_EXTENSION"
	SearchTypeFeelingLucky              SearchType = "FEELING_LUCKY"
	SearchTypeFeedback                  SearchType = "FEEDBACK"
	SearchTypeTemporal                  SearchType = "TEMPORAL"
	SearchTypeCodingRules               SearchType = "CODING_RULES"
	SearchTypeChunksLexical             SearchType = "CHUNKS_LEXICAL"
)
