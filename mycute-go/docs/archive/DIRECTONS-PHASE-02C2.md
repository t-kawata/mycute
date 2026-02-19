# Phase-02C2: Search & Verification

## 0. 前提条件 (Prerequisites)

### Phase-02C1で実装済みであるべき項目
- [ ] `Cognify` パイプラインが正常に動作し、DBにデータが蓄積されていること。
- [ ] `VectorStorage` と `GraphStorage` が検索用メソッド（`Search`, `GetTriplets`）を実装していること。

## 1. 目的
Phase-02の最終段階として、蓄積された知識グラフを用いた **Search (検索)** 機能と、その精度を検証するための **Benchmark** ツールを実装します。

## 2. 実装詳細とPython解析

### 2.1. Search Functionality

**ファイル**: `src/pkg/cognee/tools/search/graph_completion.go`

#### Python実装の解析 (`cognee/modules/retrieval/utils/brute_force_triplet_search.py`)
Cogneeの検索は、単にノードを探すのではなく、**「関連するトリプレット（関係性）」**を探し出します。

```python
# cognee/modules/retrieval/utils/brute_force_triplet_search.py
# 1. 各コレクション（Entity名, Chunkテキスト等）からベクトル検索
results = await asyncio.gather(*[search_in_collection(c) for c in collections])
# 2. ベクトル距離をグラフ上のノード・エッジにマッピング
await memory_fragment.map_vector_distances_to_graph_nodes(node_distances=node_distances)
# 3. 重要度に基づいてトップのトリプレットを算出
results = await memory_fragment.calculate_top_triplet_importances(k=top_k)
```

**Why (設計根拠)**:
クエリに対して「最も関連性の高い関係性」を提示することで、LLMが文脈を理解しやすくなります。単に「近いノード」を返すだけでは、そのノードがどういう文脈で重要なのかが欠落します。

#### Go実装方針 (GraphCompletionTool)
Strategyパターンを採用し、ベクトル検索とグラフ探索を組み合わせます。

1.  **Vector Search**: クエリに関連するチャンクやノードをDuckDBから検索。
2.  **Graph Traversal**: 検索されたノードを起点に、CozoDBで1-hop/2-hop先の関連ノード（トリプレット）を取得。
3.  **Answer Generation**: 取得したトリプレットをコンテキストとしてLLMに渡し、回答を生成。

```go
func (t *GraphCompletionTool) Search(ctx context.Context, query string) (string, error) {
    // 1. Vector Search -> Node IDs
    // 2. CozoDB Traversal -> Triplets
    // 3. Generate Answer (using prompts)
}
```

### 2.2. Prompts Definition (Critical)

**ファイル**: `src/pkg/cognee/prompts.go`

以下のプロンプトを定数として定義してください。

#### AnswerSimpleQuestionPrompt
```go
const AnswerSimpleQuestionPrompt = `Answer the question using the provided context.
If the answer is not in the context, say "I don't know" or "Information not found".
Do not make up information.`
```

#### GraphContextForQuestionPrompt
```go
const GraphContextForQuestionPrompt = `The question is: %s

Context:
%s`
```

### 2.3. Scientific Verification (Benchmark)

**ファイル**: `src/cmd/benchmark/main.go`

`QA.json` を用いて、検索精度を自動評価するツールを実装します。

#### Scoring Logic
*   **Cosine Similarity**: 正解文と生成文のベクトル類似度（0.85以上で正解）。
*   **Unanswerable Detection**: "I don't know" 等の回答を検出し、Negative Sampleに対する正解とする。

```go
// 類似度計算
func calculateSimilarity(ctx context.Context, llm llms.Model, expected, actual string) (float64, error)
// 回答不能判定
func isUnanswerable(ctx context.Context, llm llms.Model, response string) bool
```

## 3. 開発ステップ & Checkpoints

### Checkpoint 6: Search機能の実装
- [ ] `GraphCompletionTool` が実装されていること。
- [ ] **検証コマンド**: `make run ARGS="search -q 'C言語の開発者は？'"`
    - [ ] 適切な回答（"Dennis Ritchie" 等）が返ってくること。

### Checkpoint 7: Benchmarkの実装と検証
- [ ] `benchmark` コマンドが実装されていること。
- [ ] **検証コマンド**: `make run ARGS="benchmark -j test_data/QA.json -n 10"`
    - [ ] 指定された数の質問に対してテストが実行されること。
    - [ ] 正解率（Accuracy）が表示されること。
    - [ ] **Goal**: Accuracy 80% 以上。

### Accuracy 80%未達時のトラブルシューティング

もしAccuracyが80%に到達しない場合、以下の順序でチェックしてください。

1. **ベクトル検索の確認**: `VectorStorage.Search`が正しく動作しているか、コサイン類似度の計算が正確か。
2. **グラフ探索の確認**: `GraphStorage.GetTriplets`が関連するノード・エッジを取得しているか。
3. **プロンプトの確認**: `GenerateGraphPrompt`、`AnswerSimpleQuestionPrompt`が正しく定義されているか（タイポなし）。
4. **LLMモデルの確認**: 使用しているLLMが十分な推論能力を持っているか（例: Gemini 1.5 Pro以上）。

最も可能性が高い原因は「グラフが疎（ノード間の接続が少ない）」です。この場合、`test_data/sample.txt`のCognify実行が正常に完了したか、CozoDBの`edges`テーブルにデータが存在するかを確認してください。グラフが疎となるのは、Python実装のオリジナルCogneeが主に英語を想定しており、日本語の特性を十分に考慮していない実装となっていることが根本的な原因になっていることがあります。グラフを作る処理の前に、日本語の特性に合わせたチャンク化処理が矛盾なく行われているかを十分に確認してください。

## 4. 実行コマンド

```bash
# 検索実行
make run ARGS="search -q '質問内容'"

# ベンチマーク実行
make run ARGS="benchmark -j test_data/QA.json -n 10"
```
