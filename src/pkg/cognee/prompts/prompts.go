// Package prompts は、Cogneeシステムで使用されるLLMプロンプトを定義します。
// これらのプロンプトは、Python版Cogneeの実装から正確にコピーされたものです。
package prompts

// [警告] このファイルを変更しないでください
// これらのプロンプトは、元のPython Cognee実装の正確なコピーです。
// グラフ抽出と検索の品質を保証するため、元のバージョンと同期を保つ必要があります。
// これらのプロンプトへの変更は、元のPython実装に対して検証する必要があります。

// GenerateGraphPrompt は、テキストから知識グラフを抽出するためのプロンプトです。
// ソース: cognee/infrastructure/llm/prompts/generate_graph_prompt.txt
//
// このプロンプトは以下を指示します：
//   - ノード: エンティティと概念を表す（Wikipediaのノードに相当）
//   - エッジ: 概念間の関係を表す（Wikipediaのリンクに相当）
//   - ノードラベリング: 基本的なタイプを使用（例: "Person"、"Organization"）
//   - ノードID: 整数を使用せず、人間が読める識別子を使用
//   - 数値データと日付の取り扱い
//   - 共参照解決: エンティティの一貫性を維持
const GenerateGraphPrompt = `You are a top-tier algorithm designed for extracting information in structured formats to build a knowledge graph.
**Nodes** represent entities and concepts. They're akin to Wikipedia nodes.
**Edges** represent relationships between concepts. They're akin to Wikipedia links.

The aim is to achieve simplicity and clarity in the knowledge graph.
# 1. Labeling Nodes
**Consistency**: Ensure you use basic or elementary types for node labels.
  - For example, when you identify an entity representing a person, always label it as **"Person"**.
  - Avoid using more specific terms like "Mathematician" or "Scientist", keep those as "profession" property.
  - Don't use too generic terms like "Entity".
**Node IDs**: Never utilize integers as node IDs.
  - Node IDs should be names or human-readable identifiers found in the text.
# 2. Handling Numerical Data and Dates
  - For example, when you identify an entity representing a date, make sure it has type **"Date"**.
  - Extract the date in the format "YYYY-MM-DD"
  - If not possible to extract the whole date, extract month or year, or both if available.
  - **Property Format**: Properties must be in a key-value format.
  - **Quotation Marks**: Never use escaped single or double quotes within property values.
  - **Naming Convention**: Use snake_case for relationship names, e.g., acted_in.
# 3. Coreference Resolution
  - **Maintain Entity Consistency**: When extracting entities, it's vital to ensure consistency.
  If an entity, such as "John Doe", is mentioned multiple times in the text but is referred to by different names or pronouns (e.g., "Joe", "he"),
  always use the most complete identifier for that entity throughout the knowledge graph. In this example, use "John Doe" as the Persons ID.
Remember, the knowledge graph should be coherent and easily understandable, so maintaining consistency in entity references is crucial.
# 4. Strict Compliance
Adhere to the rules strictly. Non-compliance will result in termination`

// AnswerSimpleQuestionPrompt は、シンプルな質問に回答するためのプロンプトです。
// ソース: cognee/infrastructure/llm/prompts/answer_simple_question.txt
//
// 重要な指示: 回答は自然で専門的な日本語で行う必要があります。
const AnswerSimpleQuestionPrompt = `Answer the question using the provided context. Be as brief as possible.

IMPORTANT INSTRUCTION:
Answer in natural, professional JAPANESE.`

// GraphContextForQuestionPrompt は、質問に対するグラフコンテキストを提供するためのプロンプトです。
// ソース: cognee/infrastructure/llm/prompts/graph_context_for_question.txt
//
// 注意: 元のプロンプトはjinja2の {{ question }} と {{ context }} を使用しています。
// Goのフォーマットでは %s にマッピングされています。
const GraphContextForQuestionPrompt = `The question is: %s
and here is the context provided with a set of relationships from a knowledge graph separated by \n---\n each represented as node1 -- relation -- node2 triplet: %s`

// SummarizeContentPrompt は、テキストを要約するためのプロンプトです。
// ソース: cognee/infrastructure/llm/prompts/summarize_content.txt
//
// 重要な指示:
//   - 英語で内容を分析して正確性を維持
//   - 最終的な出力は日本語で行う
const SummarizeContentPrompt = `Summarize the following text while strictly keeping the details that are essential for the understanding of the text.
The answer should be as detailed as possible.

IMPORTANT INSTRUCTION:
You must analyze the content in English to maintain accuracy, but your final OUTPUT MUST BE IN JAPANESE.
Translate your summary into natural, professional Japanese.

Text:
%s`

// SummarizeSearchResultsPrompt は、検索結果を要約するためのプロンプトです。
// ソース: cognee/infrastructure/llm/prompts/summarize_search_results.txt
//
// 重要な指示:
//   - 英語で内容を分析して正確性を維持
//   - 最終的な出力は日本語で行う
const SummarizeSearchResultsPrompt = `Summarize the search results to answer the query: %s

IMPORTANT INSTRUCTION:
You must analyze the content in English to maintain accuracy, but your final OUTPUT MUST BE IN JAPANESE.
Translate your summary into natural, professional Japanese.

Search Results:
%s`
