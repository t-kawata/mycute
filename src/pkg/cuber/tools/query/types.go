// Package search は、検索タイプの定義を提供します。
// QueryTypeは、Cuberシステムで利用可能な様々な検索方法を定義します。
package query

// QueryType は、検索操作のタイプを定義します。
// 各検索タイプは、異なる検索戦略とデータソースを使用します。
type QueryType string

const (
	// ========================================
	// 実装済み（Phase 3）
	// ========================================

	// QUERY_TYPE_GRAPH_COMPLETION は、グラフとチャンクを組み合わせた検索です（デフォルト）。
	// ベクトル検索でノードを見つけ、グラフトラバーサルで関連情報を取得し、
	// LLMで回答を生成します。
	QUERY_TYPE_GRAPH_COMPLETION QueryType = "GRAPH_COMPLETION"

	// ========================================
	// 実装済み（Phase 4: 要約）
	// ========================================

	// QUERY_TYPE_SUMMARIES は、要約のみを検索します。
	// "TextSummary_text"コレクションからベクトル検索を行います。
	QUERY_TYPE_SUMMARIES QueryType = "SUMMARIES"

	// QUERY_TYPE_GRAPH_SUMMARY_COMPLETION は、グラフを検索して要約を生成します。
	// ノード検索→グラフトラバーサル→要約生成→回答生成の流れです。
	QUERY_TYPE_GRAPH_SUMMARY_COMPLETION QueryType = "GRAPH_SUMMARY_COMPLETION"

	// ========================================
	// 未実装（将来のフェーズ）
	// ========================================

	QUERY_TYPE_CHUNKS                       QueryType = "CHUNKS"                             // チャンクのみを検索
	QUERY_TYPE_RAG_COMPLETION               QueryType = "RAG_COMPLETION"                     // RAG（Retrieval-Augmented Generation）
	QUERY_TYPE_CODE                         QueryType = "CODE"                               // コード検索
	QUERY_TYPE_CYCLER                       QueryType = "CYPHER"                             // Cypherクエリ
	QUERY_TYPE_NATURAL_LANGUAGE             QueryType = "NATURAL_LANGUAGE"                   // 自然言語クエリ
	QUERY_TYPE_GRAPH_COMPLETION_COT         QueryType = "GRAPH_COMPLETION_COT"               // Chain-of-Thought付きグラフ検索
	QUERY_TYPE_GRAPH_COMPLETION_CONTEXT_EXT QueryType = "GRAPH_COMPLETION_CONTEXT_EXTENSION" // コンテキスト拡張付きグラフ検索
	QUERY_TYPE_FEELING_LUCKY                QueryType = "FEELING_LUCKY"                      // ランダム検索
	QUERY_TYPE_FEEDBACK                     QueryType = "FEEDBACK"                           // フィードバックベース検索
	QUERY_TYPE_TEMPORAL                     QueryType = "TEMPORAL"                           // 時系列検索
	QUERY_TYPE_CODING_RULES                 QueryType = "CODING_RULES"                       // コーディングルール検索
	QUERY_TYPE_CHUNKS_LEXICAL               QueryType = "CHUNKS_LEXICAL"                     // 字句ベースチャンク検索
)

var VALID_QUERY_TYPES = []QueryType{
	QUERY_TYPE_GRAPH_COMPLETION,
	QUERY_TYPE_SUMMARIES,
	QUERY_TYPE_GRAPH_SUMMARY_COMPLETION,
	// QUERY_TYPE_CHUNKS,
	// QUERY_TYPE_RAG_COMPLETION,
	// QUERY_TYPE_CODE,
	// QUERY_TYPE_CYCLER,
	// QUERY_TYPE_NATURAL_LANGUAGE,
	// QUERY_TYPE_GRAPH_COMPLETION_COT,
	// QUERY_TYPE_GRAPH_COMPLETION_CONTEXT_EXT,
	// QUERY_TYPE_FEELING_LUCKY,
	// QUERY_TYPE_FEEDBACK,
	// QUERY_TYPE_TEMPORAL,
	// QUERY_TYPE_CODING_RULES,
	// QUERY_TYPE_CHUNKS_LEXICAL,
}

// 文字列を渡して有効なクエリタイプかどうか判定する関数
func IsValidQueryType(queryType string) bool {
	for _, validType := range VALID_QUERY_TYPES {
		if string(validType) == queryType {
			return true
		}
	}
	return false
}
