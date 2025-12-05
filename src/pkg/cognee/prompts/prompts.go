package prompts

// [WARNING] DO NOT MODIFY THIS FILE
// These prompts are exact copies of the original Python Cognee implementation.
// They must be kept in sync with the original version to ensure the quality of graph extraction and search.
// Any changes to these prompts must be verified against the original Python implementation.

// Source: cognee/infrastructure/llm/prompts/generate_graph_prompt.txt
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

// Source: cognee/infrastructure/llm/prompts/answer_simple_question.txt
const AnswerSimpleQuestionPrompt = `Answer the question using the provided context. Be as brief as possible.

IMPORTANT INSTRUCTION:
Answer in natural, professional JAPANESE.`

// Source: cognee/infrastructure/llm/prompts/graph_context_for_question.txt
// Note: Original uses jinja2 {{ question }} and {{ context }}. Mapped to %s for Go formatting.
const GraphContextForQuestionPrompt = `The question is: %s
and here is the context provided with a set of relationships from a knowledge graph separated by \n---\n each represented as node1 -- relation -- node2 triplet: %s`

// Source: cognee/infrastructure/llm/prompts/summarize_content.txt
const SummarizeContentPrompt = `Summarize the following text while strictly keeping the details that are essential for the understanding of the text.
The answer should be as detailed as possible.

IMPORTANT INSTRUCTION:
You must analyze the content in English to maintain accuracy, but your final OUTPUT MUST BE IN JAPANESE.
Translate your summary into natural, professional Japanese.

Text:
%s`

// Source: cognee/infrastructure/llm/prompts/summarize_search_results.txt
const SummarizeSearchResultsPrompt = `Summarize the search results to answer the query: %s

IMPORTANT INSTRUCTION:
You must analyze the content in English to maintain accuracy, but your final OUTPUT MUST BE IN JAPANESE.
Translate your summary into natural, professional Japanese.

Search Results:
%s`
