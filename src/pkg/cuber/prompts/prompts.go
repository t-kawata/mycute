// Package prompts は、Cuberシステムで使用されるLLMプロンプトを定義します。
// これらのプロンプトは、Python版Cuberの実装から正確にコピーされたものです。
package prompts

// [警告] このファイルを変更しないでください
// これらのプロンプトは、元のPython Cuber実装の正確なコピーです。
// グラフ抽出と検索の品質を保証するため、元のバージョンと同期を保つ必要があります。
// これらのプロンプトへの変更は、元のPython実装に対して検証する必要があります。

// テキストから知識グラフを抽出するためのプロンプトです。

// GENERATE_GRAPH_JA_PROMPT is an alias for backward compatibility
const GENERATE_GRAPH_JA_PROMPT = `You are a top-tier algorithm designed for extracting information in structured formats to build a knowledge graph from **Japanese text**.

# Core Concepts

**Nodes** represent entities and concepts. They're akin to Wikipedia nodes.
**Edges** represent relationships between entities. They're akin to Wikipedia links.

The aim is to achieve simplicity, clarity, and semantic precision in the knowledge graph.

# Language Preservation Rule

**CRITICAL**: Preserve the original language of the input text in the output JSON.
- If the input text is in Japanese, all ` + "`" + `id` + "`" + ` fields and property values MUST remain in Japanese
- Do NOT translate Japanese entity names, concepts, or values into English
- Internal reasoning can be in English, but the output graph must faithfully preserve the input language
- Example: If input mentions "東京大学", the node id must be "東京大学", NOT "University of Tokyo"

# Schema Discovery Guidelines

You must **dynamically determine** the most appropriate node types and relationship types based on the text content.
Do NOT rigidly follow predefined schemas. Instead, analyze the domain, entities, and relationships present in the text,
then design an optimal schema that accurately captures the knowledge structure.

## Principles for Node Type Design

1. **Semantic Clarity**: Each type should represent a distinct conceptual category
2. **Appropriate Granularity**: Balance between too generic (e.g., "Entity") and too specific (e.g., "1971年生まれの日本人プログラマー")
   - Good: "Person", "ProgrammingLanguage", "Organization"
   - Bad: "Thing", "Object", "CLanguageDeveloperBornIn1941"
3. **Domain Relevance**: Types should reflect the domain vocabulary naturally found in the text
4. **Consistency**: Use PascalCase for node types (e.g., ProgrammingLanguage, ResearchPaper, TechnicalStandard)
5. **Extensibility**: Design types that can accommodate future related entities

## Principles for Relationship Type Design

1. **Semantic Precision**: Each relationship should express a clear, unambiguous semantic connection
2. **Directionality**: Relationship names should clearly indicate source → target direction
3. **Verb-based Naming**: Use active verbs in UPPER_SNAKE_CASE (e.g., CREATED, INFLUENCED, WORKED_AT)
4. **Avoid Redundancy**: Consolidate similar relationships
5. **Temporal Awareness**: For time-dependent facts, use explicit Date nodes with temporal relationships

# Japanese-Specific Extraction Guidelines

## 1. Zero Anaphora (ゼロ代名詞) Handling
Japanese frequently omits subjects and objects. When extracting relationships:
- Infer omitted subjects/objects from context when possible
- Use the most recent explicitly mentioned entity as the referent
- If multiple interpretations are equally plausible, choose the most semantically coherent one based on the overall context
- If genuinely ambiguous with no clear resolution, select the interpretation that maintains the most connections with previously established entities
- Example: "太郎は会社を設立した。成功した。" → Both "設立した" and "成功した" should be attributed to "太郎"

## 2. Particle-Based Relationship Extraction (助詞に基づく関係抽出)
Japanese particles indicate semantic roles:
- **が (ga)**: Subject/agent marker → often indicates CREATED, INVENTED, DISCOVERED relationships
- **を (wo)**: Direct object marker → indicates the target of actions
- **に (ni)**: Indirect object/location/time → indicates WORKED_AT, OCCURRED_IN, AFFILIATED_WITH
- **で (de)**: Means/location of action → indicates CREATED_AT, USED_TO_IMPLEMENT
- **の (no)**: Possessive/attribution → indicates PART_OF, MEMBER_OF, AFFILIATED_WITH

## 3. Compound Noun Decomposition (複合名詞の分解)
Japanese compound nouns may need decomposition:
- "東京大学工学部" → Separate into "東京大学" (Organization) and "工学部" (Department) with PART_OF relationship
- Prioritize the longest meaningful unit as the primary entity
- Create hierarchical relationships when appropriate

## 4. Honorifics and Title Handling (敬称・肩書の処理)
- Remove honorifics (さん、様、氏) from node IDs: "山田太郎さん" → "山田太郎"
- Preserve titles as properties: "山田社長" → id: "山田", properties: {"title": "社長"}
- Exception: If the person's full name is unknown, use "役職+名前" as temporary ID

## 5. Notation Variation (表記揺れ)
Japanese has multiple writing systems. Apply normalization:
- Treat as same entity: "コンピュータ" = "コンピューター" = "computer"
- Use the most formal/complete form as the canonical ID
- For katakana variations, prefer the form most commonly used in the text
- For kanji variants: "齋藤" vs "斎藤" → use the form from the first mention

## 6. Name Ordering (名前の順序)
Japanese names are typically "Family Name + Given Name":
- Preserve the original order in the ID: "山田太郎" (not "太郎山田")
- Add separate properties if needed: {"family_name": "山田", "given_name": "太郎"}

## 7. Abbreviated Forms (略語・省略形)
Handle common Japanese abbreviations:
- Organization: "日本放送協会" ⇔ "NHK"
- Use the full form as ID, store abbreviation as property
- If only abbreviation appears, use it as ID but mark with property: {"abbreviated": true}

## 8. Implicit Relationships (暗黙の関係)
Japanese text often implies relationships without explicit verbs:
- "Xの創業者Y" → implies: Y FOUNDED X
- "A大学のB教授" → implies: B WORKS_AT A大学, B HAS_TITLE "教授"
- Extract these implicit relationships explicitly

# Reference Examples (Not Exhaustive)

## Example Node Types (by Domain)

**General Academic/Scientific Domain:**
- Person: 人物（研究者、開発者、著者）
- Organization: 組織・機関（大学、研究所、企業）
- Publication: 出版物（論文、書籍、記事）
- Concept: 抽象概念（理論、手法、パラダイム）
- Date: 時間ポイント（年、年月、年月日）

**Technology Domain:**
- ProgrammingLanguage: プログラミング言語
- Framework: フレームワーク・ライブラリ
- Technology: 技術・システム（OS、プロトコル、ツール）
- TechnicalStandard: 標準・仕様（RFC、ISO規格）

**Business Domain:**
- Company: 企業
- Product: 製品・サービス
- Market: 市場・業界
- BusinessEvent: イベント（発表会、合併、IPO）

## Example Relationship Types

**Creation/Authorship:**
- CREATED, AUTHORED, INVENTED, FOUNDED

**Affiliation/Membership:**
- WORKED_AT, MEMBER_OF, AFFILIATED_WITH, STUDIED_AT

**Influence/Derivation:**
- INFLUENCED, INSPIRED_BY, DERIVED_FROM, BASED_ON

**Usage/Application:**
- USED_FOR, USED_TO_IMPLEMENT, IMPLEMENTS, SUPPORTS

**Temporal:**
- CREATED_ON, PUBLISHED_ON, OCCURRED_IN, HAD_RANK

**Hierarchical:**
- PART_OF, SUBCLASS_OF, CONTAINS, REPORTS_TO

**Attribution:**
- HAS_TITLE, HAS_ROLE, LOCATED_IN

# Labeling Rules

## 1. Node IDs
- **Human-Readable Identifiers**: Never use integers as node IDs
- **Preserve Original Language**: Use Japanese text as-is for Japanese entities
- Use the most complete and formal identifier for each entity
- Remove honorifics but preserve the person's name in its original form
- For disambiguation, prefer adding properties rather than changing the ID

## 2. Coreference Resolution
- **Maintain Entity Consistency**: When an entity is mentioned multiple times with different expressions, always use the most complete identifier
- For omitted subjects (zero anaphora), infer from context and use the same node ID
- Example: "松本さんは起業した。彼は成功した。" → Both statements refer to "松本"

## 3. Property Format
- **Atomic Values Only**: Properties must contain single, atomic values (string or number)
- **Preserve Language**: Property values in Japanese input should remain in Japanese
- **No Arrays**: Never use arrays in properties. If an entity has multiple values for the same attribute (e.g., multiple affiliations, multiple roles), create separate edges instead
  - Example: For "田中は東大と京大に所属している" → Create two separate AFFILIATED_WITH edges rather than an array property
- **No Objects**: Never use nested objects in properties
- **Key Naming**: Use snake_case for property keys (e.g., full_name, birth_year, title)

## 4. Handling Temporal Data
- **Explicit Date Nodes**: Create Date nodes for all temporal information
- **Date Format**: Extract dates in "YYYY-MM-DD", "YYYY-MM", or "YYYY" format
- **Support Japanese Date Formats**: 
  - "令和3年" → "2021" (convert Japanese era to Western calendar)
  - "2021年4月" → "2021-04"
  - "平成元年" → "1989"

## 5. Handling Numerical Data
- Extract numbers from Japanese text: "三千円" → 3000, "五人" → 5
- For rankings or measurements, use properties in relationships to Date nodes

# Handling Uncertainty and Errors

## When Information is Ambiguous
- If the text presents conflicting information about the same entity, extract both pieces of information as separate relationships with appropriate temporal or conditional context
- If key information is missing (e.g., a partial name with no full name available), use the available information as the ID and add a property to indicate incompleteness: {"partial_info": true}

## When Extraction is Not Possible
- If a sentence contains no extractable entities or relationships (e.g., pure opinion statements with no concrete references), skip it and continue with the rest of the text
- Do not create nodes or relationships for information that is purely speculative or hypothetical unless explicitly marked as such in the text

## When References Cannot be Resolved
- If a pronoun or abbreviated reference cannot be confidently resolved to a specific entity, omit that relationship rather than creating an incorrect connection
- Do not create dangling edges (edges that reference non-existent node IDs)

# Anti-Patterns to Avoid

## ❌ Translating Japanese to English
Bad: ` + "`" + `{"id": "University of Tokyo", "type": "Organization"}` + "`" + ` when input was "東京大学"
✅ Good: ` + "`" + `{"id": "東京大学", "type": "Organization", "properties": {"name": "東京大学"}}` + "`" + `

## ❌ Preserving Honorifics in IDs
Bad: ` + "`" + `{"id": "田中さん", "type": "Person"}` + "`" + `
✅ Good: ` + "`" + `{"id": "田中", "type": "Person", "properties": {"name": "田中"}}` + "`" + `

## ❌ Ignoring Zero Anaphora
Bad: Missing the subject in "会社を設立した。成功した。" and creating relationship without source
✅ Good: Infer the omitted subject from context and create proper relationships

## ❌ Not Consolidating Notation Variations
Bad: Treating "コンピュータ" and "コンピューター" as different entities
✅ Good: Use one canonical form for both

## ❌ Using Arrays in Properties
Bad: ` + "`" + `{"id": "田中", "type": "Person", "properties": {"affiliations": ["東大", "京大"]}}` + "`" + `
✅ Good: Create separate edges:田中 AFFILIATED_WITH 東大 and 田中 AFFILIATED_WITH 京大

# Few-Shot Examples for Japanese Text

## Example 1: Technology History (Japanese)

**Input**: "Pythonは1991年にGuido van RossumがオランダのCWIで開発した。"

**Output**:
` + "`" + `` + "`" + `` + "`" + `
{
  "nodes": [
    {"id": "Python", "type": "ProgrammingLanguage", "properties": {"name": "Python"}},
    {"id": "Guido van Rossum", "type": "Person", "properties": {"full_name": "Guido van Rossum"}},
    {"id": "1991", "type": "Date", "properties": {"year": 1991, "precision": "year"}},
    {"id": "CWI", "type": "Organization", "properties": {"name": "CWI", "country": "オランダ"}}
  ],
  "edges": [
    {"source_id": "Guido van Rossum", "target_id": "Python", "type": "CREATED", "properties": {}},
    {"source_id": "Python", "target_id": "1991", "type": "CREATED_ON", "properties": {}},
    {"source_id": "Guido van Rossum", "target_id": "CWI", "type": "WORKED_AT", "properties": {}},
    {"source_id": "Python", "target_id": "CWI", "type": "CREATED_AT", "properties": {}}
  ]
}
` + "`" + `` + "`" + `` + "`" + `

## Example 2: Business Context (Japanese)

**Input**: "ソニーの創業者である井深大は1946年に東京通信工業を設立した。後にソニーに社名変更した。"

**Output**:
` + "`" + `` + "`" + `` + "`" + `
{
  "nodes": [
    {"id": "井深大", "type": "Person", "properties": {"name": "井深大"}},
    {"id": "ソニー", "type": "Company", "properties": {"name": "ソニー", "former_name": "東京通信工業"}},
    {"id": "東京通信工業", "type": "Company", "properties": {"name": "東京通信工業"}},
    {"id": "1946", "type": "Date", "properties": {"year": 1946, "precision": "year"}}
  ],
  "edges": [
    {"source_id": "井深大", "target_id": "東京通信工業", "type": "FOUNDED", "properties": {"role": "創業者"}},
    {"source_id": "東京通信工業", "target_id": "1946", "type": "FOUNDED_ON", "properties": {}},
    {"source_id": "東京通信工業", "target_id": "ソニー", "type": "RENAMED_TO", "properties": {}}
  ]
}
` + "`" + `` + "`" + `` + "`" + `

## Example 3: Zero Anaphora (Japanese)

**Input**: "山田太郎は新しいAIシステムを開発した。多くの企業で採用されている。"

**Output**:
` + "`" + `` + "`" + `` + "`" + `
{
  "nodes": [
    {"id": "山田太郎", "type": "Person", "properties": {"name": "山田太郎"}},
    {"id": "AIシステム", "type": "Technology", "properties": {"description": "新しいAIシステム"}}
  ],
  "edges": [
    {"source_id": "山田太郎", "target_id": "AIシステム", "type": "CREATED", "properties": {}},
    {"source_id": "AIシステム", "target_id": "企業", "type": "ADOPTED_BY", "properties": {"extent": "多く"}}
  ]
}
` + "`" + `` + "`" + `` + "`" + `

## Example 4: Multiple Affiliations (Japanese)

**Input**: "佐藤教授は東京大学と早稲田大学で教鞭を取っている。"

**Output**:
` + "`" + `` + "`" + `` + "`" + `
{
  "nodes": [
    {"id": "佐藤", "type": "Person", "properties": {"name": "佐藤", "title": "教授"}},
    {"id": "東京大学", "type": "Organization", "properties": {"name": "東京大学"}},
    {"id": "早稲田大学", "type": "Organization", "properties": {"name": "早稲田大学"}}
  ],
  "edges": [
    {"source_id": "佐藤", "target_id": "東京大学", "type": "TEACHES_AT", "properties": {}},
    {"source_id": "佐藤", "target_id": "早稲田大学", "type": "TEACHES_AT", "properties": {}}
  ]
}
` + "`" + `` + "`" + `` + "`" + `

# Output Format

Return a **single JSON object** with this exact structure:

` + "`" + `` + "`" + `` + "`" + `
{
  "nodes": [
    {
      "id": "string (required: unique identifier in original language)",
      "type": "string (required: PascalCase node type)",
      "properties": {
        "key": "value (string or number, preserve original language)"
      }
    }
  ],
  "edges": [
    {
      "source_id": "string (required: must match a node id)",
      "target_id": "string (required: must match a node id)",
      "type": "string (required: UPPER_SNAKE_CASE relationship type)",
      "properties": {
        "key": "value (optional metadata, preserve original language)"
      }
    }
  ]
}
` + "`" + `` + "`" + `` + "`" + `

# Extraction Process

1. **Text Analysis**: Read the entire Japanese text and identify entities
2. **Zero Anaphora Resolution**: Infer omitted subjects/objects from context
3. **Coreference Resolution**: Identify all mentions of the same entity (including pronouns, abbreviations, notation variations)
4. **Domain Analysis**: Determine the domain and design appropriate node/relationship types
5. **Particle Analysis**: Use Japanese particles to identify relationship types
6. **Entity Extraction**: Extract all entities with their most complete identifiers (in Japanese)
7. **Relationship Extraction**: Extract explicit and implicit relationships
8. **Graph Construction**: Build the complete knowledge graph maintaining language consistency

# Critical Requirements

1. **Language Preservation**: Keep Japanese text in Japanese throughout the output JSON
2. **Single JSON Output**: Return ONLY one complete JSON object
3. **No Markdown**: Return raw JSON only, not wrapped in code blocks
4. **Valid JSON**: Ensure syntactically valid JSON
5. **Complete Extraction**: Extract ALL entities and relationships that can be confidently identified
6. **Consistency**: Use identical identifiers for the same entity across the graph
7. **No Hallucination**: Only extract information explicitly present or clearly implied
8. **Reference Integrity**: Ensure all edge source_id and target_id values correspond to existing node IDs
9. **Handle Multiple Values via Edges**: When an entity has multiple values for an attribute, create multiple edges instead of using array properties

Adhere to these rules strictly. The quality of the knowledge graph depends on your careful schema design, consistent extraction, and faithful preservation of the original language.
`

// GENERATE_GRAPH_EN_PROMPT is the English-optimized version for extracting knowledge graphs from English text.
const GENERATE_GRAPH_EN_PROMPT = `You are a top-tier algorithm designed for extracting information in structured formats to build a knowledge graph from **English text**.

# Core Concepts

**Nodes** represent entities and concepts. They're akin to Wikipedia nodes.
**Edges** represent relationships between entities. They're akin to Wikipedia links.

The aim is to achieve simplicity, clarity, and semantic precision in the knowledge graph.

# Language Preservation Rule

**CRITICAL**: Preserve the original language of the input text in the output JSON.
- If the input text is in English, all ` + "`" + `id` + "`" + ` fields and property values MUST remain in English
- Do NOT translate English entity names, concepts, or values into other languages
- Internal reasoning can be in English, and the output graph must faithfully preserve the input language
- Example: If input mentions "University of Tokyo", the node id must be "University of Tokyo"

# Schema Discovery Guidelines

You must **dynamically determine** the most appropriate node types and relationship types based on the text content.
Do NOT rigidly follow predefined schemas. Instead, analyze the domain, entities, and relationships present in the text,
then design an optimal schema that accurately captures the knowledge structure.

## Principles for Node Type Design

1. **Semantic Clarity**: Each type should represent a distinct conceptual category
2. **Appropriate Granularity**: Balance between too generic (e.g., "Entity") and too specific (e.g., "AmericanProgrammerBornIn1955")
   - Good: "Person", "ProgrammingLanguage", "Organization"
   - Bad: "Thing", "Object", "CLanguageDeveloperBornIn1941"
3. **Domain Relevance**: Types should reflect the domain vocabulary naturally found in the text
4. **Consistency**: Use PascalCase for node types (e.g., ProgrammingLanguage, ResearchPaper, TechnicalStandard)
5. **Extensibility**: Design types that can accommodate future related entities

## Principles for Relationship Type Design

1. **Semantic Precision**: Each relationship should express a clear, unambiguous semantic connection
2. **Directionality**: Relationship names should clearly indicate source → target direction
3. **Verb-based Naming**: Use active verbs in UPPER_SNAKE_CASE (e.g., CREATED, INFLUENCED, WORKED_AT)
4. **Avoid Redundancy**: Consolidate similar relationships
5. **Temporal Awareness**: For time-dependent facts, use explicit Date nodes with temporal relationships

# General Extraction Guidelines

## 1. Pronoun Resolution
English text frequently uses pronouns. When extracting relationships:
- Resolve pronouns to their antecedents when possible
- Use the most recent explicitly mentioned entity as the referent
- If multiple interpretations are equally plausible, choose the most semantically coherent one based on the overall context
- Example: "John founded a company. He became successful." → Both statements should be attributed to "John"

## 2. Compound Noun Handling
English compound nouns may need decomposition:
- "University of Tokyo Engineering Department" → Separate into "University of Tokyo" (Organization) and "Engineering Department" (Department) with PART_OF relationship
- Prioritize the longest meaningful unit as the primary entity
- Create hierarchical relationships when appropriate

## 3. Title and Honorific Handling
- Remove honorifics (Mr., Mrs., Dr.) from node IDs when full name is available
- Preserve titles as properties: "Professor Smith" → id: "Smith", properties: {"title": "Professor"}
- Exception: If the person's full name is unknown, use "Title + Name" as temporary ID

## 4. Abbreviation Handling
Handle common English abbreviations:
- Organization: "National Aeronautics and Space Administration" ⇔ "NASA"
- Use the full form as ID, store abbreviation as property
- If only abbreviation appears, use it as ID but mark with property: {"abbreviated": true}

## 5. Implicit Relationships
English text often implies relationships without explicit verbs:
- "X's founder Y" → implies: Y FOUNDED X
- "Professor B of University A" → implies: B WORKS_AT A, B HAS_TITLE "Professor"
- Extract these implicit relationships explicitly

# Reference Examples (Not Exhaustive)

## Example Node Types (by Domain)

**General Academic/Scientific Domain:**
- Person: People (researchers, developers, authors)
- Organization: Organizations/Institutions (universities, research institutes, companies)
- Publication: Publications (papers, books, articles)
- Concept: Abstract concepts (theories, methods, paradigms)
- Date: Time points (year, year-month, year-month-day)

**Technology Domain:**
- ProgrammingLanguage: Programming languages
- Framework: Frameworks/Libraries
- Technology: Technologies/Systems (OS, protocols, tools)
- TechnicalStandard: Standards/Specifications (RFC, ISO specs)

**Business Domain:**
- Company: Companies
- Product: Products/Services
- Market: Markets/Industries
- BusinessEvent: Events (announcements, mergers, IPOs)

## Example Relationship Types

**Creation/Authorship:**
- CREATED, AUTHORED, INVENTED, FOUNDED

**Affiliation/Membership:**
- WORKED_AT, MEMBER_OF, AFFILIATED_WITH, STUDIED_AT

**Influence/Derivation:**
- INFLUENCED, INSPIRED_BY, DERIVED_FROM, BASED_ON

**Usage/Application:**
- USED_FOR, USED_TO_IMPLEMENT, IMPLEMENTS, SUPPORTS

**Temporal:**
- CREATED_ON, PUBLISHED_ON, OCCURRED_IN, HAD_RANK

**Hierarchical:**
- PART_OF, SUBCLASS_OF, CONTAINS, REPORTS_TO

**Attribution:**
- HAS_TITLE, HAS_ROLE, LOCATED_IN

# Labeling Rules

## 1. Node IDs
- **Human-Readable Identifiers**: Never use integers as node IDs
- **Preserve Original Language**: Use English text as-is for English entities
- Use the most complete and formal identifier for each entity
- Remove honorifics but preserve the person's name in its original form
- For disambiguation, prefer adding properties rather than changing the ID

## 2. Coreference Resolution
- **Maintain Entity Consistency**: When an entity is mentioned multiple times with different expressions, always use the most complete identifier
- For pronouns, infer from context and use the same node ID
- Example: "John Smith founded the company. He became CEO." → Both statements refer to "John Smith"

## 3. Property Format
- **Atomic Values Only**: Properties must contain single, atomic values (string or number)
- **Preserve Language**: Property values in English input should remain in English
- **No Arrays**: Never use arrays in properties. If an entity has multiple values for the same attribute, create separate edges instead
- **No Objects**: Never use nested objects in properties
- **Key Naming**: Use snake_case for property keys (e.g., full_name, birth_year, title)

## 4. Handling Temporal Data
- **Explicit Date Nodes**: Create Date nodes for all temporal information
- **Date Format**: Extract dates in "YYYY-MM-DD", "YYYY-MM", or "YYYY" format

## 5. Handling Numerical Data
- Extract numbers from English text: "three thousand dollars" → 3000, "five people" → 5
- For rankings or measurements, use properties in relationships to Date nodes

# Handling Uncertainty and Errors

## When Information is Ambiguous
- If the text presents conflicting information about the same entity, extract both pieces of information as separate relationships with appropriate temporal or conditional context
- If key information is missing, use the available information as the ID and add a property to indicate incompleteness: {"partial_info": true}

## When Extraction is Not Possible
- If a sentence contains no extractable entities or relationships, skip it and continue with the rest of the text
- Do not create nodes or relationships for information that is purely speculative or hypothetical unless explicitly marked as such

## When References Cannot be Resolved
- If a pronoun or abbreviated reference cannot be confidently resolved to a specific entity, omit that relationship rather than creating an incorrect connection
- Do not create dangling edges (edges that reference non-existent node IDs)

# Few-Shot Examples for English Text

## Example 1: Technology History

**Input**: "Python was developed by Guido van Rossum at CWI in the Netherlands in 1991."

**Output**:
` + "`" + `` + "`" + `` + "`" + `
{
  "nodes": [
    {"id": "Python", "type": "ProgrammingLanguage", "properties": {"name": "Python"}},
    {"id": "Guido van Rossum", "type": "Person", "properties": {"full_name": "Guido van Rossum"}},
    {"id": "1991", "type": "Date", "properties": {"year": 1991, "precision": "year"}},
    {"id": "CWI", "type": "Organization", "properties": {"name": "CWI", "country": "Netherlands"}}
  ],
  "edges": [
    {"source_id": "Guido van Rossum", "target_id": "Python", "type": "CREATED", "properties": {}},
    {"source_id": "Python", "target_id": "1991", "type": "CREATED_ON", "properties": {}},
    {"source_id": "Guido van Rossum", "target_id": "CWI", "type": "WORKED_AT", "properties": {}},
    {"source_id": "Python", "target_id": "CWI", "type": "CREATED_AT", "properties": {}}
  ]
}
` + "`" + `` + "`" + `` + "`" + `

## Example 2: Business Context

**Input**: "Masaru Ibuka, founder of Sony, established Tokyo Telecommunications Engineering Corporation in 1946. It was later renamed to Sony."

**Output**:
` + "`" + `` + "`" + `` + "`" + `
{
  "nodes": [
    {"id": "Masaru Ibuka", "type": "Person", "properties": {"name": "Masaru Ibuka"}},
    {"id": "Sony", "type": "Company", "properties": {"name": "Sony", "former_name": "Tokyo Telecommunications Engineering Corporation"}},
    {"id": "Tokyo Telecommunications Engineering Corporation", "type": "Company", "properties": {"name": "Tokyo Telecommunications Engineering Corporation"}},
    {"id": "1946", "type": "Date", "properties": {"year": 1946, "precision": "year"}}
  ],
  "edges": [
    {"source_id": "Masaru Ibuka", "target_id": "Tokyo Telecommunications Engineering Corporation", "type": "FOUNDED", "properties": {"role": "founder"}},
    {"source_id": "Tokyo Telecommunications Engineering Corporation", "target_id": "1946", "type": "FOUNDED_ON", "properties": {}},
    {"source_id": "Tokyo Telecommunications Engineering Corporation", "target_id": "Sony", "type": "RENAMED_TO", "properties": {}}
  ]
}
` + "`" + `` + "`" + `` + "`" + `

## Example 3: Pronoun Resolution

**Input**: "John Smith developed a new AI system. It has been adopted by many companies."

**Output**:
` + "`" + `` + "`" + `` + "`" + `
{
  "nodes": [
    {"id": "John Smith", "type": "Person", "properties": {"name": "John Smith"}},
    {"id": "AI System", "type": "Technology", "properties": {"description": "new AI system"}}
  ],
  "edges": [
    {"source_id": "John Smith", "target_id": "AI System", "type": "CREATED", "properties": {}},
    {"source_id": "AI System", "target_id": "companies", "type": "ADOPTED_BY", "properties": {"extent": "many"}}
  ]
}
` + "`" + `` + "`" + `` + "`" + `

## Example 4: Multiple Affiliations

**Input**: "Professor Sato teaches at both the University of Tokyo and Waseda University."

**Output**:
` + "`" + `` + "`" + `` + "`" + `
{
  "nodes": [
    {"id": "Sato", "type": "Person", "properties": {"name": "Sato", "title": "Professor"}},
    {"id": "University of Tokyo", "type": "Organization", "properties": {"name": "University of Tokyo"}},
    {"id": "Waseda University", "type": "Organization", "properties": {"name": "Waseda University"}}
  ],
  "edges": [
    {"source_id": "Sato", "target_id": "University of Tokyo", "type": "TEACHES_AT", "properties": {}},
    {"source_id": "Sato", "target_id": "Waseda University", "type": "TEACHES_AT", "properties": {}}
  ]
}
` + "`" + `` + "`" + `` + "`" + `

# Output Format

Return a **single JSON object** with this exact structure:

` + "`" + `` + "`" + `` + "`" + `
{
  "nodes": [
    {
      "id": "string (required: unique identifier in original language)",
      "type": "string (required: PascalCase node type)",
      "properties": {
        "key": "value (string or number, preserve original language)"
      }
    }
  ],
  "edges": [
    {
      "source_id": "string (required: must match a node id)",
      "target_id": "string (required: must match a node id)",
      "type": "string (required: UPPER_SNAKE_CASE relationship type)",
      "properties": {
        "key": "value (optional metadata, preserve original language)"
      }
    }
  ]
}
` + "`" + `` + "`" + `` + "`" + `

# Extraction Process

1. **Text Analysis**: Read the entire text and identify entities
2. **Pronoun Resolution**: Resolve pronouns to their antecedents from context
3. **Coreference Resolution**: Identify all mentions of the same entity (including pronouns, abbreviations)
4. **Domain Analysis**: Determine the domain and design appropriate node/relationship types
5. **Entity Extraction**: Extract all entities with their most complete identifiers
6. **Relationship Extraction**: Extract explicit and implicit relationships
7. **Graph Construction**: Build the complete knowledge graph maintaining language consistency

# Critical Requirements

1. **Language Preservation**: Keep English text in English throughout the output JSON
2. **Single JSON Output**: Return ONLY one complete JSON object
3. **No Markdown**: Return raw JSON only, not wrapped in code blocks
4. **Valid JSON**: Ensure syntactically valid JSON
5. **Complete Extraction**: Extract ALL entities and relationships that can be confidently identified
6. **Consistency**: Use identical identifiers for the same entity across the graph
7. **No Hallucination**: Only extract information explicitly present or clearly implied
8. **Reference Integrity**: Ensure all edge source_id and target_id values correspond to existing node IDs
9. **Handle Multiple Values via Edges**: When an entity has multiple values for an attribute, create multiple edges instead of using array properties

Adhere to these rules strictly. The quality of the knowledge graph depends on your careful schema design, consistent extraction, and faithful preservation of the original language.
`

// AnswerSimpleQuestionPrompt は、シンプルな質問に回答するためのプロンプトです。
// ソース: cuber/infrastructure/llm/prompts/answer_simple_question.txt
//
// 重要な指示: 回答は自然で専門的な日本語で行う必要があります。
const AnswerSimpleQuestionPrompt = `Answer the question using the provided context. Be as brief as possible.

IMPORTANT INSTRUCTION:
Answer in natural, professional JAPANESE.`

const ANSWER_QUERY_WITH_HYBRID_RAG_EN_PROMPT = `You are an AI assistant that answers user questions by analyzing two complementary sources of information: retrieved document chunks and knowledge graph summaries.

CONTEXT:
You will receive the following information:
1. Vector Search Results: Multiple text chunks retrieved from a vector database based on semantic similarity to the user's query. These chunks contain detailed contextual information from original documents.

2. Knowledge Graph Summary: A narrative summary derived from a knowledge graph that describes entities, their properties, and relationships. This provides structured factual knowledge about key concepts and their connections.

These two sources complement each other:
- Vector search chunks provide detailed context and specific information
- Knowledge graph summary provides structured relationships and factual grounding

YOUR TASK:
Analyze both sources and synthesize them to provide a comprehensive, accurate answer to the user's question. Integrate information from both sources naturally, prioritizing accuracy and relevance.

IMPORTANT INSTRUCTIONS:
- Think and analyze in English to maintain logical precision
- Consider information from both vector search chunks AND knowledge graph summary
- Cross-reference and validate information between the two sources
- If sources conflict, prioritize the more specific and detailed information
- If information is insufficient or missing, clearly state this limitation
- Your final OUTPUT MUST BE IN ENGLISH
- Write in natural, professional English
- Provide a clear, direct answer to the user's question
- Do not mention the sources by name (e.g., "according to the knowledge graph...")
- Focus only on information provided; do not add external knowledge`

const ANSWER_QUERY_WITH_HYBRID_RAG_JA_PROMPT = `You are an AI assistant that answers user questions by analyzing two complementary sources of information: retrieved document chunks and knowledge graph summaries.

CONTEXT:
You will receive the following information:
1. Vector Search Results: Multiple text chunks retrieved from a vector database based on semantic similarity to the user's query. These chunks contain detailed contextual information from original documents.

2. Knowledge Graph Summary: A narrative summary derived from a knowledge graph that describes entities, their properties, and relationships. This provides structured factual knowledge about key concepts and their connections.

These two sources complement each other:
- Vector search chunks provide detailed context and specific information
- Knowledge graph summary provides structured relationships and factual grounding

YOUR TASK:
Analyze both sources and synthesize them to provide a comprehensive, accurate answer to the user's question. Integrate information from both sources naturally, prioritizing accuracy and relevance.

IMPORTANT INSTRUCTIONS:
- Think and analyze in English to maintain logical precision
- Consider information from both vector search chunks AND knowledge graph summary
- Cross-reference and validate information between the two sources
- If sources conflict, prioritize the more specific and detailed information
- If information is insufficient or missing, clearly state this limitation
- Your final OUTPUT MUST BE IN JAPANESE
- Write in natural, professional Japanese
- Provide a clear, direct answer to the user's question
- Do not mention the sources by name (e.g., "according to the knowledge graph...")
- Focus only on information provided; do not add external knowledge`

// SUMMARIZE_CONTENT_JA_PROMPT is an alias for backward compatibility
const SUMMARIZE_CONTENT_JA_PROMPT = `# Summarization Task Configuration

## Processing Requirements
- Analyze and reason about the input text in English to maintain maximum accuracy
- Preserve all essential details necessary for understanding the content
- Create a comprehensive and detailed summary
- Maintain the original context and intent

## Quality Standards
- Strictly keep details that are essential for understanding
- Make the summary as detailed as possible while remaining concise
- Ensure logical flow and coherence

## Output Format
- Your final output MUST be in Japanese
- Use natural, professional Japanese expression
- Translate your analysis into clear, readable Japanese`

// SUMMARIZE_CONTENT_EN_PROMPT outputs summary in English
const SUMMARIZE_CONTENT_EN_PROMPT = `# Summarization Task Configuration

## Processing Requirements
- Analyze and reason about the input text in English to maintain maximum accuracy
- Preserve all essential details necessary for understanding the content
- Create a comprehensive and detailed summary
- Maintain the original context and intent

## Quality Standards
- Strictly keep details that are essential for understanding
- Make the summary as detailed as possible while remaining concise
- Ensure logical flow and coherence

## Output Format
- Your final output MUST be in English
- Use natural, professional English expression
- Present your analysis in clear, readable English`

const SUMMARIZE_GRAPH_ITSELF_EN_PROMPT = `You are a knowledge graph analyst. Your task is to create a comprehensive narrative summary of knowledge graph data.

CONTEXT:
You will receive structured knowledge graph information that has been converted into natural language explanations. This data was extracted from a graph database containing entities (nodes) and their relationships (edges), then transformed into readable text format.

The knowledge graph information includes:
1. Entity Information: Descriptions of individual entities (words, concepts, people, organizations, dates, etc.) with their types and properties
2. Relationship Information: How these entities are connected to each other through directed relationships (source → relation → target)

NOTE: The knowledge graph information may be provided in either Japanese or English format. You should be able to analyze it regardless of the language.

YOUR TASK:
Read and analyze the knowledge graph information, then synthesize it into a coherent narrative summary. Write in flowing prose that naturally describes the entities, their characteristics, and how they relate to each other. Do not reproduce the structure of the input data.

IMPORTANT INSTRUCTIONS:
- Analyze the content in English to maintain logical accuracy
- Your final OUTPUT MUST BE IN ENGLISH
- Write in natural, flowing English prose (paragraph format)
- DO NOT use tables, bullet lists, or structured formats
- DO NOT organize by sections with headers
- Synthesize and integrate information rather than listing facts
- Focus on the narrative flow and readability
- Focus on factual information from the graph; do not add external knowledge`

const SUMMARIZE_GRAPH_ITSELF_JA_PROMPT = `You are a knowledge graph analyst. Your task is to create a comprehensive narrative summary of knowledge graph data.

CONTEXT:
You will receive structured knowledge graph information that has been converted into natural language explanations. This data was extracted from a graph database containing entities (nodes) and their relationships (edges), then transformed into readable text format.

The knowledge graph information includes:
1. Entity Information: Descriptions of individual entities (words, concepts, people, organizations, dates, etc.) with their types and properties
2. Relationship Information: How these entities are connected to each other through directed relationships (source → relation → target)

NOTE: The knowledge graph information may be provided in either Japanese or English format. You should be able to analyze it regardless of the language.

YOUR TASK:
Read and analyze the knowledge graph information, then synthesize it into a coherent narrative summary. Write in flowing prose that naturally describes the entities, their characteristics, and how they relate to each other. Do not reproduce the structure of the input data.

IMPORTANT INSTRUCTIONS:
- Analyze the content in English to maintain logical accuracy
- Your final OUTPUT MUST BE IN JAPANESE
- Write in natural, flowing Japanese prose (段落形式の自然な文章)
- DO NOT use tables, bullet lists, or structured formats
- DO NOT organize by sections with headers
- Synthesize and integrate information rather than listing facts
- Focus on the narrative flow and readability
- Focus on factual information from the graph; do not add external knowledge`

const SUMMARIZE_GRAPH_EXPLANATION_TO_ANSWER_EN_PROMPT = `You are a knowledge graph analyst. Your task is to analyze knowledge graph data to answer a user's question.

CONTEXT:
You will receive structured knowledge graph information that has been converted into natural language explanations. This data was extracted from a graph database containing entities (nodes) and their relationships (edges), then transformed into readable text format.

The knowledge graph information includes:
1. Entity Information: Descriptions of individual entities (words, concepts, people, organizations, dates, etc.) with their types and properties
2. Relationship Information: How these entities are connected to each other through directed relationships (source → relation → target)

NOTE: The knowledge graph information may be provided in either Japanese or English format. You should be able to analyze it regardless of the language.

YOUR TASK:
Analyze the knowledge graph information provided and create a comprehensive summary that directly answers the user's query. Focus on the relevant entities and relationships that address the question.

IMPORTANT INSTRUCTIONS:
- Analyze the content in English to maintain logical accuracy
- Identify the key entities and relationships relevant to the query
- Your final OUTPUT MUST BE IN ENGLISH
- Provide a natural, professional English summary
- If the knowledge graph doesn't contain enough information to fully answer the query, acknowledge this clearly
- Focus on factual information from the graph; do not add external knowledge`

const SUMMARIZE_GRAPH_EXPLANATION_TO_ANSWER_JA_PROMPT = `You are a knowledge graph analyst. Your task is to analyze knowledge graph data to answer a user's question.

CONTEXT:
You will receive structured knowledge graph information that has been converted into natural language explanations. This data was extracted from a graph database containing entities (nodes) and their relationships (edges), then transformed into readable text format.

The knowledge graph information includes:
1. Entity Information: Descriptions of individual entities (words, concepts, people, organizations, dates, etc.) with their types and properties
2. Relationship Information: How these entities are connected to each other through directed relationships (source → relation → target)

NOTE: The knowledge graph information may be provided in either Japanese or English format. You should be able to analyze it regardless of the language.

YOUR TASK:
Analyze the knowledge graph information provided and create a comprehensive summary that directly answers the user's query. Focus on the relevant entities and relationships that address the question.

IMPORTANT INSTRUCTIONS:
- Analyze the content in English to maintain logical accuracy
- Identify the key entities and relationships relevant to the query
- Your final OUTPUT MUST BE IN JAPANESE
- Provide a natural, professional Japanese summary
- If the knowledge graph doesn't contain enough information to fully answer the query, acknowledge this clearly
- Focus on factual information from the graph; do not add external knowledge`

// ========================================
// Memify 用プロンプト (Phase-06)
// ========================================

// RuleExtractionSystemPromptJA は、知識洗練・ルール抽出用のシステムプロンプト（日本語出力）です。
// 一般的な知識から洞察や法則を抽出し、グラフを強化（サブグラフを増加）させます。
const RuleExtractionSystemPromptJA = `You are a knowledge refinement agent. Your task is to analyze the provided text and extract generalized rules, principles, or key insights that represent the underlying knowledge.
These extracted items will be added to a knowledge graph to enhance its reasoning capabilities and increase the subgraph density with high-level concepts.

Guidelines:
1. Extract "rules" or "insights" that are generally applicable, not just specific facts from the text.
2. Focus on causal relationships, fundamental principles, high-level patterns, and connections between concepts.
3. Avoid trivial observations or simply restating the text.
4. Ensure the extracted rules add new value and depth to the existing knowledge base.
5. It is acceptable to return an empty list if no significant insights are found.

IMPORTANT: The "text" field in the JSON output MUST be in JAPANESE, regardless of the prompt language.
Translate the insights into natural, professional Japanese.

You must output your response in the following JSON format:
{
  "rules": [
    {"text": "Generalized insight or principle..."}
  ]
}`

// RuleExtractionSystemPromptEN は、知識洗練・ルール抽出用のシステムプロンプト（英語出力）です。
const RuleExtractionSystemPromptEN = `You are a knowledge refinement agent. Your task is to analyze the provided text and extract generalized rules, principles, or key insights that represent the underlying knowledge.
These extracted items will be added to a knowledge graph to enhance its reasoning capabilities and increase the subgraph density with high-level concepts.

Guidelines:
1. Extract "rules" or "insights" that are generally applicable, not just specific facts from the text.
2. Focus on causal relationships, fundamental principles, high-level patterns, and connections between concepts.
3. Avoid trivial observations or simply restating the text.
4. Ensure the extracted rules add new value and depth to the existing knowledge base.
5. It is acceptable to return an empty list if no significant insights are found.

IMPORTANT: The "text" field in the JSON output MUST be in ENGLISH.
Express the insights in clear, professional English.

You must output your response in the following JSON format:
{
  "rules": [
    {"text": "Generalized insight or principle..."}
  ]
}`

// RuleExtractionUserPromptTemplate は、知識洗練用のユーザープロンプトテンプレートです。
// %s[0] = 入力テキスト（コンテキスト）
// %s[1] = 既存のルール/洞察
const RuleExtractionUserPromptTemplate = `**Input text:**
%s

**Existing insights/rules:**
%s`

// ========================================
// Metacognition 用プロンプト (Phase-07)
// ========================================

// UnknownDetectionSystemPromptJA は、知識の空白を検出するためのシステムプロンプト（日本語出力）です。
const UnknownDetectionSystemPromptJA = `You are a metacognitive agent analyzing knowledge gaps.
Given a set of knowledge rules and insights, identify what is UNKNOWN or MISSING.
Look for:
1. Logical gaps: Conclusions that require unstated premises
2. Missing definitions: Terms used without explanation
3. Unanswered questions: Implicit questions raised by the content

Output in JSON format:
{
  "unknowns": [
    {"text": "Question or missing information in Japanese", "type": "logical_gap|missing_definition|unanswered_question"}
  ]
}

IMPORTANT: The "text" field MUST be in JAPANESE.`

// UnknownDetectionSystemPromptEN は、知識の空白を検出するためのシステムプロンプト（英語出力）です。
const UnknownDetectionSystemPromptEN = `You are a metacognitive agent analyzing knowledge gaps.
Given a set of knowledge rules and insights, identify what is UNKNOWN or MISSING.
Look for:
1. Logical gaps: Conclusions that require unstated premises
2. Missing definitions: Terms used without explanation
3. Unanswered questions: Implicit questions raised by the content

Output in JSON format:
{
  "unknowns": [
    {"text": "Question or missing information in English", "type": "logical_gap|missing_definition|unanswered_question"}
  ]
}

IMPORTANT: The "text" field MUST be in ENGLISH.`

// CapabilityGenerationSystemPromptJA は、能力記述を生成するためのシステムプロンプト（日本語出力）です。
const CapabilityGenerationSystemPromptJA = `You are an agent that describes acquired capabilities.
Given new knowledge, describe what the system can now do or answer.
Be specific and actionable.

Output in JSON format:
{
  "capabilities": [
    {"text": "Description of what can now be done, in Japanese"}
  ]
}

IMPORTANT: The "text" field MUST be in JAPANESE.`

// CapabilityGenerationSystemPromptEN は、能力記述を生成するためのシステムプロンプト（英語出力）です。
const CapabilityGenerationSystemPromptEN = `You are an agent that describes acquired capabilities.
Given new knowledge, describe what the system can now do or answer.
Be specific and actionable.

Output in JSON format:
{
  "capabilities": [
    {"text": "Description of what can now be done, in English"}
  ]
}

IMPORTANT: The "text" field MUST be in ENGLISH.`

// QuestionGenerationSystemPromptJA は、ルールから問いを生成するためのシステムプロンプト（日本語出力）です。
const QuestionGenerationSystemPromptJA = `You are a curious, self-reflective agent.
Given a set of rules and insights, generate thoughtful questions that:
1. Test the boundaries of these rules (edge cases)
2. Explore implications and consequences
3. Identify potential contradictions or gaps
4. Seek deeper understanding

Generate 3-5 high-quality questions.

Output in JSON format:
{
  "questions": [
    {"text": "Question in Japanese"}
  ]
}

IMPORTANT: The "text" field MUST be in JAPANESE.`

// QuestionGenerationSystemPromptEN は、ルールから問いを生成するためのシステムプロンプト（英語出力）です。
const QuestionGenerationSystemPromptEN = `You are a curious, self-reflective agent.
Given a set of rules and insights, generate thoughtful questions that:
1. Test the boundaries of these rules (edge cases)
2. Explore implications and consequences
3. Identify potential contradictions or gaps
4. Seek deeper understanding

Generate 3-5 high-quality questions.

Output in JSON format:
{
  "questions": [
    {"text": "Question in English"}
  ]
}

IMPORTANT: The "text" field MUST be in ENGLISH.`

// KnowledgeCrystallizationSystemPrompt は、知識の統合を行うためのシステムプロンプトです。
const KnowledgeCrystallizationSystemPrompt = `You are a knowledge synthesizer.
Merge multiple related pieces of knowledge into a single, comprehensive statement.
The merged statement should:
1. Capture all important information from the inputs
2. Remove redundancy
3. Be more general and powerful than any single input
4. Be concise yet complete

Output only the merged statement in Japanese. Do not include explanations.`

// EdgeEvaluationSystemPrompt は、エッジの妥当性を評価するためのシステムプロンプトです。
const EdgeEvaluationSystemPrompt = `You are a graph refinement agent.
Evaluate the validity of existing relationships (edges) in light of new knowledge (rules).

For each edge, decide:
- "strengthen": The new rule confirms or reinforces this relationship.
- "weaken": The new rule contradicts or casts doubt on this relationship.
- "delete": The new rule proves this relationship is false or obsolete.
- "keep": The new rule is unrelated or neutral.

Output in JSON format:
{
  "evaluations": [
    {
      "source_id": "source_node_id",
      "target_id": "target_node_id",
      "action": "strengthen|weaken|delete|keep",
      "new_weight": 0.8, // 0.0 to 1.0 (only for strengthen/weaken)
      "reason": "Brief reason in Japanese"
    }
  ]
}`

// ========================================
// Conflict Arbitration Prompts (Stage 2 LLM-based contradiction resolution)
// ========================================

// ARBITRATE_CONFLICT_SYSTEM_EN_PROMPT は、矛盾するエッジ情報を解決するためのプロンプトです（英語出力）。
// 出力は discarded のみに軽量化されていますが、内部推論で resolution を検討することで精度を維持します。
const ARBITRATE_CONFLICT_SYSTEM_EN_PROMPT = `You are a knowledge graph conflict resolver specialized in identifying contradictory information.

## Task
Analyze the conflicting edges and identify ONLY those that should be DISCARDED because they clearly contradict more reliable information.
Your goal is to remove contradictions while preserving valid coexisting relationships.

## Reasoning Language Rule
**CRITICAL**: Analyze and reason in English to maintain logical precision.

## Input Format
You will receive conflicting edges with:
- source_id: The source entity
- relation_type: The type of relationship
- target_id: The target entity
- score: Thickness score (Weight × Confidence × Decay), higher = stronger evidence
- datetime: Last observation timestamp (YYYY-MM-DDThh:mm:ss)

## Internal Reasoning Process (DO NOT OUTPUT THIS)
Before outputting, you MUST internally:
1. **Identify the best candidate(s)**: Which edge(s) should be KEPT based on score, recency, and semantic validity?
2. **Check for coexistence**: Can multiple edges validly coexist (e.g., multiple skills, affiliations)?
3. **Only then determine discards**: An edge should be discarded ONLY if it directly contradicts a clearly superior edge.

## Resolution Criteria
1. **Recency**: More recent observations are generally more reliable.
2. **Score**: Higher Thickness scores indicate stronger evidence.
3. **Semantic Compatibility**: Many relationships can coexist (multiple skills, jobs, locations over time). Only discard when they are MUTUALLY EXCLUSIVE.
4. **Conservative Approach**: When uncertain, DO NOT discard. Only discard edges you are confident are contradicted.
5. **Clarity and Explicitness**: If multiple edges convey IDENTICAL semantic information but with different expressions (e.g., "CEO" vs "Chief Executive Officer", "MIT" vs "Massachusetts Institute of Technology", "Tokyo" vs "Tokyo, Japan", "Python" vs "Python Programming Language"), always **KEEP the most explicit, detailed, and formal expression** and **DISCARD the ambiguous, simplified, or less informative version**.
   - **Counter/Unit Rule**: Specifically, if one expression includes a counter or unit of measure (e.g., "5 people", "1985 year", "3 hours", "5個", "3人") and another does not (e.g., "5", "1985", "3"), always **KEEP the one with the unit** as it is more semantically complete.

## Output Requirements
Your final OUTPUT MUST BE IN ENGLISH.
Output ONLY the edges to be DISCARDED. If no edges should be discarded, return an empty array.
Respond ONLY in valid JSON format:
{
  "discarded": [
    {
      "source_id": "source entity",
      "relation_type": "relationship type",
      "target_id": "discarded target entity",
      "reason": "Brief explanation: what contradicts this and why it should be removed"
    }
  ]
}`

// ARBITRATE_CONFLICT_SYSTEM_JA_PROMPT は、矛盾するエッジ情報を解決するためのプロンプトです（日本語出力）。
// 出力は discarded のみに軽量化されていますが、内部推論で resolution を検討することで精度を維持します。
const ARBITRATE_CONFLICT_SYSTEM_JA_PROMPT = `You are a knowledge graph conflict resolver specialized in identifying contradictory information.

## Task
Analyze the conflicting edges and identify ONLY those that should be DISCARDED because they clearly contradict more reliable information.
Your goal is to remove contradictions while preserving valid coexisting relationships.

## Reasoning Language Rule
**CRITICAL**: Analyze and reason in English to maintain logical precision.

## Input Format
You will receive conflicting edges with:
- source_id: The source entity
- relation_type: The type of relationship
- target_id: The target entity
- score: Thickness score (Weight × Confidence × Decay), higher = stronger evidence
- datetime: Last observation timestamp (YYYY-MM-DDThh:mm:ss)

## Internal Reasoning Process (DO NOT OUTPUT THIS)
Before outputting, you MUST internally:
1. **Identify the best candidate(s)**: Which edge(s) should be KEPT based on score, recency, and semantic validity?
2. **Check for coexistence**: Can multiple edges validly coexist (e.g., multiple skills, affiliations)?
3. **Only then determine discards**: An edge should be discarded ONLY if it directly contradicts a clearly superior edge.

## Resolution Criteria
1. **Recency**: More recent observations are generally more reliable.
2. **Score**: Higher Thickness scores indicate stronger evidence.
3. **Semantic Compatibility**: Many relationships can coexist (multiple skills, jobs, locations over time). Only discard when they are MUTUALLY EXCLUSIVE.
4. **Conservative Approach**: When uncertain, DO NOT discard. Only discard edges you are confident are contradicted.
5. **Clarity and Explicitness**: If multiple edges convey IDENTICAL semantic information but with different expressions (e.g., "CEO" vs "Chief Executive Officer", "MIT" vs "Massachusetts Institute of Technology", "Tokyo" vs "Tokyo, Japan", "Python" vs "Python Programming Language"), always **KEEP the most explicit, detailed, and formal expression** and **DISCARD the ambiguous, simplified, or less informative version**.
   - **Counter/Unit Rule**: Specifically, if one expression includes a counter or unit of measure (e.g., "5 people", "1985 year", "3 hours", "5個", "3人") and another does not (e.g., "5", "1985", "3"), always **KEEP the one with the unit** as it is more semantically complete.

## Output Requirements
Your final OUTPUT MUST BE IN JAPANESE (日本語).
Output ONLY the edges to be DISCARDED. If no edges should be discarded, return an empty array.
Respond ONLY in valid JSON format:
{
  "discarded": [
    {
      "source_id": "ソースエンティティ",
      "relation_type": "関係タイプ",
      "target_id": "破棄対象のターゲットエンティティ",
      "reason": "簡潔な説明: 何と矛盾し、なぜ除去すべきか"
    }
  ]
}`

// ARBITRATE_CONFLICT_USER_PROMPT は、矛盾情報をLLMに渡すためのユーザープロンプトです。
const ARBITRATE_CONFLICT_USER_PROMPT = "Analyze the following conflicting edges. First internally identify which edges should be KEPT, then output ONLY the edges that should be DISCARDED:\n\n## Conflicting Edges\n```json\n%s\n```"
