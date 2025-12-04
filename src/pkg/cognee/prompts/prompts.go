package prompts

const GenerateGraphPrompt = `
You are a helpful assistant that extracts a knowledge graph from the given text.
Extract entities (nodes) and relationships (edges) from the text.
Return the result as a JSON object with "nodes" and "edges" arrays.
Each node should have "id", "type", and "properties".
Each edge should have "source_id", "target_id", "type", and "properties".

Text:
%s
`

const AnswerSimpleQuestionPrompt = `Answer the question using the provided context.
If the answer is not in the context, say "I don't know" or "Information not found".
Do not make up information.`

const GraphContextForQuestionPrompt = `The question is: %s

Context:
%s`
