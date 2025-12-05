// Package search は、検索タイプの定義を提供します。
// SearchTypeは、Cogneeシステムで利用可能な様々な検索方法を定義します。
package search

// SearchType は、検索操作のタイプを定義します。
// 各検索タイプは、異なる検索戦略とデータソースを使用します。
type SearchType string

const (
	// ========================================
	// 実装済み（Phase 3）
	// ========================================

	// SearchTypeGraphCompletion は、グラフとチャンクを組み合わせた検索です（デフォルト）。
	// ベクトル検索でノードを見つけ、グラフトラバーサルで関連情報を取得し、
	// LLMで回答を生成します。
	SearchTypeGraphCompletion SearchType = "GRAPH_COMPLETION"

	// ========================================
	// 実装済み（Phase 4: 要約）
	// ========================================

	// SearchTypeSummaries は、要約のみを検索します。
	// "TextSummary_text"コレクションからベクトル検索を行います。
	SearchTypeSummaries SearchType = "SUMMARIES"

	// SearchTypeGraphSummaryCompletion は、グラフを検索して要約を生成します。
	// ノード検索→グラフトラバーサル→要約生成→回答生成の流れです。
	SearchTypeGraphSummaryCompletion SearchType = "GRAPH_SUMMARY_COMPLETION"

	// ========================================
	// 未実装（将来のフェーズ）
	// ========================================

	SearchTypeChunks                    SearchType = "CHUNKS"                             // チャンクのみを検索
	SearchTypeRAGCompletion             SearchType = "RAG_COMPLETION"                     // RAG（Retrieval-Augmented Generation）
	SearchTypeCode                      SearchType = "CODE"                               // コード検索
	SearchTypeCypher                    SearchType = "CYPHER"                             // Cypherクエリ
	SearchTypeNaturalLanguage           SearchType = "NATURAL_LANGUAGE"                   // 自然言語クエリ
	SearchTypeGraphCompletionCoT        SearchType = "GRAPH_COMPLETION_COT"               // Chain-of-Thought付きグラフ検索
	SearchTypeGraphCompletionContextExt SearchType = "GRAPH_COMPLETION_CONTEXT_EXTENSION" // コンテキスト拡張付きグラフ検索
	SearchTypeFeelingLucky              SearchType = "FEELING_LUCKY"                      // ランダム検索
	SearchTypeFeedback                  SearchType = "FEEDBACK"                           // フィードバックベース検索
	SearchTypeTemporal                  SearchType = "TEMPORAL"                           // 時系列検索
	SearchTypeCodingRules               SearchType = "CODING_RULES"                       // コーディングルール検索
	SearchTypeChunksLexical             SearchType = "CHUNKS_LEXICAL"                     // 字句ベースチャンク検索
)
