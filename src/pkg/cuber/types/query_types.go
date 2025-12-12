// QueryTypeは、Cuberシステムで利用可能な様々な検索方法を定義します。
package types

import "slices"

// QueryType は、検索操作のタイプを定義します。
// 各検索タイプは、異なる検索戦略とデータソースを使用します。
type QueryType uint8

const (
	QUERY_TYPE_GET_GRAPH                                         QueryType = iota + 1 // 知識グラフ自体を取得
	QUERY_TYPE_GET_CHUNKS                                                             // ベクトル検索によりチャンクを取得
	QUERY_TYPE_GET_PRE_MADE_SUMMARIES                                                 // 事前に作成された要約リストを取得
	QUERY_TYPE_GET_GRAPH_AND_CHUNKS                                                   // 知識グラフとベクトル検索によるチャンクを取得
	QUERY_TYPE_GET_GRAPH_AND_PRE_MADE_SUMMARIES                                       // 知識グラフと事前に作成された要約リストを取得
	QUERY_TYPE_GET_GRAPH_AND_CHUNKS_AND_PRE_MADE_SUMMARIES                            // 知識グラフとベクトル検索によるチャンクと事前に作成された要約リストを取得
	QUERY_TYPE_GET_GRAPH_EXPLANATION_EN                                               // 知識グラフを英語で構造文変換して取得
	QUERY_TYPE_GET_GRAPH_EXPLANATION_JA                                               // 知識グラフを日本語で構造文変換して取得
	QUERY_TYPE_GET_GRAPH_SUMMARY_EN                                                   // 知識グラフを英語で要約文変換して取得
	QUERY_TYPE_GET_GRAPH_SUMMARY_JA                                                   // 知識グラフを日本語で要約文変換して取得
	QUERY_TYPE_GET_GRAPH_SUMMARY_TO_ANSWER_EN                                         // 知識グラフを、クエリにダイレクトに答えられる形式の英語要約文で取得
	QUERY_TYPE_GET_GRAPH_SUMMARY_TO_ANSWER_JA                                         // 知識グラフを、クエリにダイレクトに答えられる形式の日本語要約文で取得
	QUERY_TYPE_ANSWER_BY_PRE_MADE_SUMMARIES_AND_GRAPH_SUMMARY_EN                      // 事前に作成された要約リストと、知識グラフ要約を用いて質問に英語で回答
	QUERY_TYPE_ANSWER_BY_PRE_MADE_SUMMARIES_AND_GRAPH_SUMMARY_JA                      // 事前に作成された要約リストと、知識グラフ要約を用いて質問に日本語で回答
	QUERY_TYPE_ANSWER_BY_CHUNKS_AND_GRAPH_SUMMARY_EN                                  // ベクトル検索によるチャンクと知識グラフ要約を用いて質問に英語で回答
	QUERY_TYPE_ANSWER_BY_CHUNKS_AND_GRAPH_SUMMARY_JA                                  // ベクトル検索によるチャンクと知識グラフ要約を用いて質問に日本語で回答

	// ========================================
	// 未実装（将来のフェーズ）
	// ========================================
	QUERY_TYPE_CHUNKS                       // チャンクのみを検索
	QUERY_TYPE_RAG_COMPLETION               // RAG（Retrieval-Augmented Generation）
	QUERY_TYPE_CODE                         // コード検索
	QUERY_TYPE_CYCLER                       // Cypherクエリ
	QUERY_TYPE_NATURAL_LANGUAGE             // 自然言語クエリ
	QUERY_TYPE_GRAPH_COMPLETION_COT         // Chain-of-Thought付きグラフ検索
	QUERY_TYPE_GRAPH_COMPLETION_CONTEXT_EXT // コンテキスト拡張付きグラフ検索
	QUERY_TYPE_FEELING_LUCKY                // ランダム検索
	QUERY_TYPE_FEEDBACK                     // フィードバックベース検索
	QUERY_TYPE_TEMPORAL                     // 時系列検索
	QUERY_TYPE_CODING_RULES                 // コーディングルール検索
	QUERY_TYPE_CHUNKS_LEXICAL               // 字句ベースチャンク検索
)

var VALID_QUERY_TYPES = []QueryType{
	QUERY_TYPE_GET_GRAPH,
	QUERY_TYPE_GET_CHUNKS,
	QUERY_TYPE_GET_PRE_MADE_SUMMARIES,
	QUERY_TYPE_GET_GRAPH_AND_CHUNKS,
	QUERY_TYPE_GET_GRAPH_AND_PRE_MADE_SUMMARIES,
	QUERY_TYPE_GET_GRAPH_AND_CHUNKS_AND_PRE_MADE_SUMMARIES,
	QUERY_TYPE_GET_GRAPH_EXPLANATION_EN,
	QUERY_TYPE_GET_GRAPH_EXPLANATION_JA,
	QUERY_TYPE_GET_GRAPH_SUMMARY_EN,
	QUERY_TYPE_GET_GRAPH_SUMMARY_JA,
	QUERY_TYPE_GET_GRAPH_SUMMARY_TO_ANSWER_EN,
	QUERY_TYPE_GET_GRAPH_SUMMARY_TO_ANSWER_JA,
	QUERY_TYPE_ANSWER_BY_PRE_MADE_SUMMARIES_AND_GRAPH_SUMMARY_EN,
	QUERY_TYPE_ANSWER_BY_PRE_MADE_SUMMARIES_AND_GRAPH_SUMMARY_JA,
	QUERY_TYPE_ANSWER_BY_CHUNKS_AND_GRAPH_SUMMARY_EN,
	QUERY_TYPE_ANSWER_BY_CHUNKS_AND_GRAPH_SUMMARY_JA,
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
func IsValidQueryType(queryType uint8) bool {
	return slices.Contains(VALID_QUERY_TYPES, QueryType(queryType))
}
