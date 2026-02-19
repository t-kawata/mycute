# Phase-02C1: Cognify Pipeline

## 0. 前提条件 (Prerequisites)

### Phase-02Bで実装済みであるべき項目
- [ ] `Task` インターフェースと `Pipeline` ランナーが実装されていること。
- [ ] `IngestTask` が実装され、`[]*Data` を返すこと。
- [ ] `VectorStorage` と `GraphStorage` が正常に動作すること。

## 1. 目的
Phase-02Bで実装したパイプライン基盤の上に、Cogneeの核心機能である **Cognify (グラフ構築)** パイプラインを実装します。
本ドキュメントでは、Chunking、Graph Extractionの各ロジックについて、Python版の実装意図を深く掘り下げ、Goでの再現方法を詳述します。

## 2. 実装詳細とPython解析

### 2.1. ChunkingTask (Text Segmentation)

**ファイル**: `src/pkg/cognee/modules/chunking/text_chunker.go`

#### Python実装の解析 (`cognee/modules/chunking/TextChunker.py`)
Cogneeは単純な文字数分割ではなく、階層的なチャンキングロジックを採用しています。

```python
# cognee/modules/chunking/TextChunker.py
def chunk_by_paragraph(data, max_chunk_size, batch_paragraphs=True):
    # ... (word -> sentence -> paragraph logic)
    yield {
        "text": current_chunk,
        "chunk_id": uuid5(NAMESPACE_OID, current_chunk),
        # ...
    }
```

**Why (設計根拠)**:
文脈の分断を防ぐためです。文や段落の途中で切れると、LLMが正しくエンティティ間の関係を抽出できなくなります。
また、ID生成には `uuid5(document_id + chunk_index)` を使用し、**決定論的（Deterministic）**なIDを生成しています。これにより、同じテキストからは常に同じIDのチャンクが生成され、再実行時の重複を防ぎます。

#### Go実装方針 (TextChunker)
このロジックを簡略化せず、忠実に移植します。

```go
func (c *TextChunker) Chunk(text string) ([]*Chunk, error) {
    // Word -> Sentence -> Paragraph -> Chunk の階層的分割
    // ID生成: uuid5(document_id + chunk_index)
}
```

#### 2.1.1. 日本語最適化チャンキング戦略 (Japanese-Optimized Chunking Strategy)

Deep Wikiの情報に基づき、日本語の特性（スペース区切りなし、句読点の違い）を考慮した実装を行います。
オリジナル実装の単純な移植では日本語で性能が出ないため、以下の戦略を必須とします。

**1. 形態素解析による単語分割 (Word Splitting)**
英語の `split(" ")` の代わりに、Go製の形態素解析器 `kagome` を使用して単語（トークン）単位に分割します。

*   **Library**: `github.com/ikawaha/kagome/v2`
*   **Logic**:
    ```go
    import (
        "github.com/ikawaha/kagome-dict/ipa"
        "github.com/ikawaha/kagome/v2/tokenizer"
    )

    func splitIntoWords(text string) []string {
        t, _ := tokenizer.New(ipa.Dict())
        tokens := t.Tokenize(text)
        var words []string
        for _, token := range tokens {
            if token.Class == tokenizer.DUMMY {
                continue
            }
            words = append(words, token.Surface)
        }
        return words
    }
    ```

**2. 日本語対応の文分割 (Sentence Splitting)**
正規表現を用いて、日本語の句読点（`。`, `！`, `？`）および英語の句読点（`.`, `!`, `?`）を正しく処理します。

*   **Regex**: `([。！？\.\!\?]+)` を区切り文字として使用し、区切り文字自体も前の文に含めます。
*   **Logic**:
    ```go
    var sentenceSplitter = regexp.MustCompile(`([^。！？\.\!\?]+[。！？\.\!\?]*)`)

    func splitIntoSentences(text string) []string {
        return sentenceSplitter.FindAllString(text, -1)
    }
    ```

**3. トークン数カウント (Token Counting)**
文字数（`len`）ではなく、LLMにとってのトークン数を基準にチャンクサイズを制御します。
`tiktoken-go` を使用し、使用するモデル（例: `cl100k_base` for GPT-4）に合わせたカウントを行います。

*   **Library**: `github.com/pkoukk/tiktoken-go`
*   **Logic**:
    ```go
    func countTokens(text string) int {
        tkm, _ := tiktoken.GetEncoding("cl100k_base") // for GPT-4
        token := tkm.Encode(text, nil, nil)
        return len(token)
    }
    ```

**4. チャンク再構築時の結合処理**
英語は単語間にスペースが必要ですが、日本語は不要です。
チャンクを結合する際は、言語判定を行うか、一律スペースなしで結合し、必要に応じて正規化するロジックを検討してください（今回は簡易的に「単純結合」とします）。

---

### 2.2. GraphExtractionTask (LLM Processing)

**ファイル**: `src/pkg/cognee/tasks/graph/extract_graph_task.go`

#### Python実装の解析 (`cognee/tasks/graph/extract_graph_from_data.py`)

```python
# cognee/tasks/graph/extract_graph_from_data.py
async def extract_graph_from_data(data_chunks, ...):
    chunk_graphs = await asyncio.gather(*[extract_content_graph(chunk) for chunk in data_chunks])
    return integrate_chunk_graphs(data_chunks, chunk_graphs)
```

**Why (設計根拠)**:
LLM呼び出しはI/Oバウンドな処理であるため、並列化が必須です。Pythonでは `asyncio.gather` を使用しています。

#### Go実装方針 (GraphExtractionTask)
`errgroup` を使用して、複数のチャンクを並列に処理します。

```go
g, ctx := errgroup.WithContext(ctx)
for _, chunk := range chunks {
    chunk := chunk
    g.Go(func() error {
        // LLM Call -> Extract Nodes/Edges
        return nil
    })
}
```

### 2.3. Prompts Definition (Critical)

**ファイル**: `src/pkg/cognee/prompts.go`

以下のプロンプトを定数として定義してください。これらはPython版から抽出したものです。**内容を変更せず、そのまま使用してください。**

#### GenerateGraphPrompt
```go
const GenerateGraphPrompt = `You are a top-tier algorithm designed for extracting information in structured formats to build a knowledge graph.
Your task is to identify the entities and relations requested with the user prompt, from a given text.
You must generate the output in a JSON format containing a list with JSON objects. Each JSON object must have the following keys:
- "nodes": A list of node objects with "id", "type", and "properties" keys.
- "edges": A list of edge objects with "source_id", "target_id", "type", and "properties" keys.

**Example Output**:
{
  "nodes": [
    {"id": "person_john_doe", "type": "Person", "properties": {"name": "John Doe", "age": 30}},
    {"id": "company_acme", "type": "Company", "properties": {"name": "Acme Corp"}}
  ],
  "edges": [
    {"source_id": "person_john_doe", "target_id": "company_acme", "type": "WORKS_AT", "properties": {}}
  ]
}

**Input Text**:
%s
`
```

### 2.4. StorageTask

**ファイル**: `src/pkg/cognee/tasks/storage/storage_task.go`

抽出されたデータを永続化します。
*   **Vectors**: DuckDB `vectors` テーブルへ保存（`collection_name` で分類）。
*   **Graph**: CozoDB `nodes`, `edges` リレーションへ保存（`:put` でUpsert）。

## 3. 開発ステップ & Checkpoints

### Checkpoint 5: Cognify機能の実装
- [ ] `ChunkingTask` が実装され、適切なサイズのチャンクが生成されること。
- [ ] `GraphExtractionTask` が実装され、LLMからグラフデータが抽出されること。
- [ ] **検証コマンド**: `make run ARGS="cognify -f test_data/sample.txt"`
    - [ ] DuckDB `chunks` テーブルにデータが存在すること。
    - [ ] CozoDB `nodes`, `edges` テーブルにデータが存在すること。

## 4. 実行コマンド

```bash
# Cognify実行
make run ARGS="cognify -f test_data/sample.txt"
```
